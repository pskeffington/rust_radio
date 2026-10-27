//! Embeddable 480x320 radio surface. The host owns the window and event loop.
//! Constructing a panel never opens an audio device or connects to a broadcaster.
use rust_radio_audio::compat_controller::RadioSession;
use rust_radio_audio::output::{available_outputs, OutputDeviceInfo};
use rust_radio_audio::playback::NativeRadioEngine;
use rust_radio_audio::validation::StationValidationTask;
use rust_radio_core::directory::{curated_stations, station_by_id, InternetStation};
use rust_radio_core::validation::{StationValidation, ValidationStatus};
use std::collections::BTreeMap;

mod settings;
mod view;

pub const PANEL_WIDTH: f32 = 480.0;
pub const PANEL_HEIGHT: f32 = 320.0;

/// Owns one radio session. Embed with `panel.show(ui)` inside an existing HUD.
pub struct RadioPanel {
    station_index: usize,
    volume: u8,
    output: String,
    outputs: Vec<OutputDeviceInfo>,
    engine: Option<NativeRadioEngine>,
    note: String,
    settings_open: bool,
    stopping: bool,
    last_session: Option<RadioSession>,
    validation: Option<StationValidationTask>,
    validations: BTreeMap<&'static str, StationValidation>,
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            station_index: 0,
            volume: 60,
            output: "system".into(),
            outputs: Vec::new(),
            engine: None,
            note: "READY / SELECT STATION AND PRESS PLAY".into(),
            settings_open: false,
            stopping: false,
            last_session: None,
            validation: None,
            validations: BTreeMap::new(),
        }
    }
}

impl RadioPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin cancellation without blocking the host UI. Keep rendering the panel
    /// (or call `poll_shutdown`) until `is_active` becomes false before dismounting.
    pub fn stop(&mut self) {
        let active = self.is_active();
        if let Some(engine) = &self.engine {
            engine.request_stop();
            self.stopping = true;
            self.note = "STOPPING / RELEASING RECEIVER".into();
        }
        if let Some(task) = &self.validation {
            task.request_cancel();
            self.note = "CANCELLING CHECK / RELEASING NETWORK".into();
        }
        if !active {
            self.last_session = None;
            self.note = "RECEIVER STOPPED".into();
        }
        self.poll_shutdown();
    }

    /// Reap finished receivers even while this panel is hidden. False means
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
                    .unwrap_or_else(|| "RECEIVER STOPPED".into());
                self.last_session = Some(session);
            }
        }
        self.cancel_stale_validation();
        if let Some(task) = self.validation.as_mut() {
            if let Some(result) = task.try_finish() {
                self.validation = None;
                match result {
                    Ok(report) if report.status == ValidationStatus::Cancelled => {
                        self.note = "STATION CHECK CANCELLED".into();
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
        !self.is_active()
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_some() || self.validation.is_some()
    }

    fn play(&mut self) {
        if self.is_active() {
            return;
        }
        match NativeRadioEngine::start_internet(
            self.selected_station().id,
            self.volume,
            &self.output,
        ) {
            Ok(engine) => {
                self.engine = Some(engine);
                self.last_session = None;
                self.note = "CONNECTING / NATIVE AUDIO".into();
            }
            Err(error) => self.note = error,
        }
    }

    fn start_validation(&mut self) {
        if self.is_active() {
            return;
        }
        match StationValidationTask::start(self.selected_station().id) {
            Ok(task) => {
                self.validation = Some(task);
                self.note = "CHECKING STATION / NO AUDIO OUTPUT".into();
            }
            Err(error) => self.note = error,
        }
    }

    fn cancel_stale_validation(&mut self) {
        if let Some(task) = &self.validation {
            if task.station_id() != self.selected_station().id {
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
            Some(report) if report.is_decoded() => "DECODED",
            Some(report) => match report.status {
                ValidationStatus::Unavailable => "UNAVAILABLE",
                ValidationStatus::InvalidStream => "INVALID STREAM",
                ValidationStatus::Unsupported => "UNSUPPORTED",
                _ => "UNVERIFIED",
            },
            None => "UNVERIFIED",
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

    fn step_station(&mut self, direction: isize) {
        let count = curated_stations().len();
        if count == 0 {
            return;
        }
        let offset = direction.rem_euclid(count as isize) as usize;
        self.station_index = (self.station_index + offset) % count;
        self.cancel_stale_validation();
    }

    fn selected_station(&self) -> &'static InternetStation {
        &curated_stations()[self.station_index]
    }
}

#[cfg(test)]
mod tests;
