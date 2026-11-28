//! Embeddable 480x320 radio surface. The host owns the window and event loop.
//! Constructing a panel never opens an audio device or connects to a broadcaster.
use rust_radio_audio::compat_controller::RadioSession;
use rust_radio_audio::output::{available_outputs, OutputDeviceInfo};
use rust_radio_audio::playback::NativeRadioEngine;
use rust_radio_audio::validation::StationValidationTask;
use rust_radio_core::directory::{curated_stations, station_by_id, InternetStation};
use rust_radio_core::validation::{FeedHealth, StationValidation, ValidationStatus};
use std::collections::BTreeMap;
use std::time::{Instant, SystemTime};

mod ip_settings;
mod ip_stations;
mod presentation;
mod settings;
mod station_menu;
mod theme;
mod validation_history;
use validation_history::SavedFeedValidation;
mod view;

pub const PANEL_WIDTH: f32 = 480.0;
pub const PANEL_HEIGHT: f32 = 320.0;

/// Owns one radio session. Embed with `panel.show(ui)` inside an existing HUD.
pub struct RadioPanel {
    station_index: usize,
    direct_selected: bool,
    direct_url: String,
    ip_stations: Vec<ip_stations::SavedStation>,
    saved_station_index: Option<usize>,
    ip_station_name: String,
    ip_stations_dirty: bool,
    restart_pending: bool,
    selection_check_pending: Option<Instant>,
    validation_selection: Option<String>,
    direct_validation: Option<StationValidation>,
    volume: u8,
    unmuted_volume: u8,
    output: String,
    outputs: Vec<OutputDeviceInfo>,
    engine: Option<NativeRadioEngine>,
    note: String,
    settings_open: bool,
    stopping: bool,
    last_session: Option<RadioSession>,
    validation: Option<StationValidationTask>,
    validations: BTreeMap<&'static str, StationValidation>,
    saved_validations: BTreeMap<&'static str, SavedFeedValidation>,
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            station_index: 0,
            direct_selected: false,
            direct_url: String::new(),
            ip_stations: Vec::new(),
            saved_station_index: None,
            ip_station_name: String::new(),
            ip_stations_dirty: false,
            restart_pending: false,
            selection_check_pending: None,
            validation_selection: None,
            direct_validation: None,
            volume: 60,
            unmuted_volume: 60,
            output: "system".into(),
            outputs: Vec::new(),
            engine: None,
            note: "READY / SELECT STATION AND PRESS PLAY".into(),
            settings_open: false,
            stopping: false,
            last_session: None,
            validation: None,
            validations: BTreeMap::new(),
            saved_validations: BTreeMap::new(),
        }
    }
}

impl RadioPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a bounded, host-persisted silent-check snapshot without file or
    /// network I/O. The host should read its file off the UI thread. Saved
    /// checks are labeled separately, expire after the shared feed TTL, and
    /// never gate playback; live checks and active playback take precedence.
    pub fn load_validation_history(&mut self, json: &str) -> Result<usize, String> {
        let history = validation_history::parse_history(json)?;
        let count = history.len();
        self.saved_validations = history;
        Ok(count)
    }

    /// Begin cancellation without blocking the host UI. Keep rendering the panel
    /// (or call `poll_shutdown`) until `is_active` becomes false before dismounting.
    pub fn stop(&mut self) {
        self.restart_pending = false;
        self.selection_check_pending = None;
        let active = self.is_active();
        if let Some(engine) = &self.engine {
            engine.request_stop();
            self.stopping = true;
            self.note = "STOPPING / RELEASING AUDIO".into();
        }
        if let Some(task) = &self.validation {
            task.request_cancel();
            self.note = "CANCELLING CHECK / RELEASING NETWORK".into();
        }
        if !active {
            self.last_session = None;
            self.note = "STOPPED".into();
        }
        self.poll_shutdown();
    }

    /// Reap finished workers even while this panel is hidden. False means
    /// a worker still owns resources. A completed failure remains visible until
    /// the operator explicitly starts another session or clears it with Stop.
    pub fn poll_shutdown(&mut self) -> bool {
        if let Some(engine) = self.engine.as_mut() {
            if engine.try_finish() {
                let session = engine.status();
                self.engine = None;
                self.stopping = false;
                self.note = session
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "STOPPED".into());
                self.last_session = Some(session);
            }
        }
        self.cancel_stale_validation();
        if let Some(task) = self.validation.as_mut() {
            if let Some(result) = task.try_finish() {
                self.validation = None;
                self.validation_selection = None;
                if let Ok(report) = &result {
                    if !matches!(
                        report.status,
                        ValidationStatus::Decoded | ValidationStatus::Cancelled
                    ) {
                        self.last_session = Some(RadioSession {
                            state: rust_radio_audio::compat_controller::PlaybackState::Failed,
                            last_error: Some(report.detail.clone()),
                            ..Default::default()
                        });
                    }
                }
                match result {
                    Ok(report) if report.status == ValidationStatus::Cancelled => {
                        self.note = "STATION CHECK CANCELLED".into();
                    }
                    Ok(report) if report.station_id == "custom" => {
                        self.note = report.detail.clone();
                        self.direct_validation = Some(report);
                    }
                    Ok(report) => match station_by_id(&report.station_id) {
                        Some(station) => {
                            self.validations.insert(station.id, report);
                            self.note =
                                format!("STATION CHECK / {}", self.validation_label(station.id));
                        }
                        None => self.note = "UNKNOWN STATION CHECK RESULT".into(),
                    },
                    Err(error) => self.note = error,
                }
            }
        }
        if self.engine.is_none() && self.validation.is_none() {
            if self.restart_pending {
                self.restart_pending = false;
                self.stopping = false;
                self.play();
            } else if self
                .selection_check_pending
                .is_some_and(|at| Instant::now() >= at)
            {
                self.selection_check_pending = None;
                self.check_ip_selection();
            }
        }
        !self.is_active()
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_some() || self.validation.is_some() || self.restart_pending
    }

    fn play(&mut self) {
        self.selection_check_pending = None;
        if self.engine.is_some() || self.restart_pending {
            return;
        }
        if self.validation.is_some() {
            if let Some(task) = &self.validation {
                task.request_cancel();
            }
            self.restart_pending = true;
            self.note = "CONNECTING / RELEASING PREVIOUS CHECK".into();
            return;
        }
        match self.start_selected() {
            Ok(engine) => {
                self.engine = Some(engine);
                self.last_session = None;
                self.note = "CONNECTING / NATIVE AUDIO".into();
            }
            Err(error) => {
                self.last_session = Some(RadioSession {
                    state: rust_radio_audio::compat_controller::PlaybackState::Failed,
                    last_error: Some(error.clone()),
                    ..Default::default()
                });
                self.note = error;
            }
        }
    }

    fn start_selected(&self) -> Result<NativeRadioEngine, String> {
        if self.direct_selected {
            return NativeRadioEngine::start_stream(&self.direct_url, self.volume, &self.output);
        }
        NativeRadioEngine::start_internet(self.selected_station().id, self.volume, &self.output)
    }

    fn ip_target(&self) -> &str {
        if self.direct_selected {
            &self.direct_url
        } else {
            self.selected_station().id
        }
    }

    fn check_ip_selection(&mut self) {
        if self.engine.is_some() || self.validation.is_some() {
            return;
        }
        let target = self.ip_target().to_owned();
        match StationValidationTask::start_input(&target, self.direct_selected) {
            Ok(task) => {
                self.validation_selection = Some(target);
                self.validation = Some(task);
                self.note = "CONNECTING / CHECKING SELECTED STREAM SILENTLY".into();
            }
            Err(error) => {
                self.last_session = Some(RadioSession {
                    state: rust_radio_audio::compat_controller::PlaybackState::Failed,
                    last_error: Some(error.clone()),
                    ..Default::default()
                });
                self.note = error;
            }
        }
    }

    fn ip_selection_changed(&mut self) {
        self.last_session = None;
        self.direct_validation = None;
        self.restart_ip_if_active();
        if !self.restart_pending {
            self.selection_check_pending =
                Some(Instant::now() + std::time::Duration::from_millis(250));
            self.note = "SELECTED / WAITING TO CHECK STREAM".into();
        }
    }

    fn restart_ip_if_active(&mut self) {
        if let Some(engine) = &self.engine {
            engine.request_stop();
            self.restart_pending = true;
            self.stopping = true;
            self.note = "CONNECTING / SWITCHING STATION".into();
        }
        if let Some(task) = &self.validation {
            task.request_cancel();
        }
    }

    #[cfg(test)]
    fn start_validation(&mut self) {
        if self.is_active() || self.direct_selected {
            return;
        }
        match StationValidationTask::start(self.selected_station().id) {
            Ok(task) => {
                self.validation_selection = Some(self.selected_station().id.into());
                self.validation = Some(task);
                self.note = "CHECKING STATION / NO AUDIO OUTPUT".into();
            }
            Err(error) => self.note = error,
        }
    }

    fn cancel_stale_validation(&mut self) {
        if let Some(task) = &self.validation {
            let target = self
                .validation_selection
                .as_deref()
                .unwrap_or_else(|| task.station_id());
            if target != self.ip_target() {
                task.request_cancel();
            }
        }
    }

    fn validation_label(&self, id: &str) -> &'static str {
        if let Some(task) = &self.validation {
            if task.station_id() == id {
                return if task.cancellation_requested() {
                    "CANCELLING"
                } else {
                    "CHECKING"
                };
            }
        }
        match self.validations.get(id) {
            Some(report) if report.health_at(SystemTime::now()) == FeedHealth::Fresh => {
                "FRESH / DECODED"
            }
            Some(report) if report.health_at(SystemTime::now()) == FeedHealth::Stale => {
                "STALE / RECHECK"
            }
            Some(report) => match report.status {
                ValidationStatus::Unavailable => "UNAVAILABLE",
                ValidationStatus::InvalidStream => "INVALID STREAM",
                ValidationStatus::Unsupported => "UNSUPPORTED",
                _ => "UNVERIFIED",
            },
            None => self
                .saved_validations
                .get(id)
                .map(|validation| validation_history::label(*validation, SystemTime::now()))
                .unwrap_or("UNVERIFIED"),
        }
    }

    fn refresh_outputs(&mut self) {
        match available_outputs() {
            Ok(outputs) => {
                self.outputs = outputs;
                self.note = "SELECT OUTPUT / STOP BEFORE SWITCHING".into();
            }
            Err(error) => self.note = error,
        }
    }

    #[cfg(test)]
    fn step_station(&mut self, direction: isize) {
        let count = self.ip_station_count();
        if count == 0 {
            return;
        }
        let offset = direction.rem_euclid(count as isize) as usize;
        let next = (self.ip_station_index() + offset) % count;
        self.select_ip_station(next);
    }

    fn source_index(&self) -> usize {
        self.ip_station_index()
    }

    fn step_station_source(&mut self, direction: isize) {
        let count = self.ip_station_count();
        if count == 0 {
            return;
        }
        let next =
            (self.ip_station_index() + direction.rem_euclid(count as isize) as usize) % count;
        self.select_ip_station(next);
        self.ip_selection_changed();
    }

    fn select_source(&mut self, index: usize) {
        let previous = self.source_index();
        self.direct_selected = index >= curated_stations().len();
        if index < curated_stations().len() {
            self.station_index = index;
        }
        self.cancel_stale_validation();
        if previous != index {
            self.restart_ip_if_active();
        }
    }

    fn source_detail(&self) -> String {
        if self.direct_selected {
            return "Custom radio stream".into();
        }
        self.selected_station().genre.replace(" / ", " · ")
    }

    fn selected_station(&self) -> &'static InternetStation {
        &curated_stations()[self.station_index]
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod live_ip_tests;
