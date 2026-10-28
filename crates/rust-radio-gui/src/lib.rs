//! Embeddable 480x320 radio surface. The host owns the window and event loop.
//! Constructing a panel never opens an audio device or connects to a broadcaster.
use rust_radio_audio::compat_controller::RadioSession;
use rust_radio_audio::output::{available_outputs, OutputDeviceInfo};
use rust_radio_audio::playback::NativeRadioEngine;
use rust_radio_audio::validation::StationValidationTask;
use rust_radio_core::directory::{curated_stations, station_by_id, InternetStation};
use rust_radio_core::validation::{FeedHealth, StationValidation, ValidationStatus};
use std::time::SystemTime;
use std::collections::BTreeMap;

#[cfg(feature = "nrsp-native")]
mod fm_presets;
mod inputs;
#[cfg(feature = "nrsp-native")]
mod native_settings;
use inputs::Segment;
use rust_radio_audio::discovery::{InputReport, InputScan};
use std::time::Instant;
mod ip_settings;
mod ip_stations;
mod settings;
mod station_menu;
mod theme;
mod view;

pub const PANEL_WIDTH: f32 = 480.0;
pub const PANEL_HEIGHT: f32 = 320.0;

/// Owns one radio session. Embed with `panel.show(ui)` inside an existing HUD.
pub struct RadioPanel {
    segment: Segment,
    inputs: InputReport,
    input_scan: Option<InputScan>,
    scan_enabled: bool,
    source_user_selected: bool,
    last_scan: Option<Instant>,
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
    #[cfg(feature = "nrsp-native")]
    native_selected: bool,
    #[cfg(feature = "nrsp-native")]
    native_endpoint: String,
    #[cfg(feature = "nrsp-native")]
    native_frequency: String,
    #[cfg(feature = "nrsp-native")]
    native_mode: String,
    #[cfg(feature = "nrsp-native")]
    native_bandwidth: String,
    #[cfg(feature = "nrsp-native")]
    native_tab: u8,
    #[cfg(feature = "nrsp-native")]
    native_step: u64,
    #[cfg(feature = "nrsp-native")]
    native_fm: rust_radio_audio::playback::NativeFmSettings,
    #[cfg(feature = "nrsp-native")]
    native_presets: fm_presets::PresetBank,
    #[cfg(feature = "nrsp-native")]
    native_preset_name: String,
    #[cfg(feature = "nrsp-native")]
    native_presets_dirty: bool,
    #[cfg(feature = "sdrconnect")]
    sdr_selected: bool,
    #[cfg(feature = "sdrconnect")]
    sdr_endpoint: String,
    #[cfg(feature = "soapy-sdr")]
    usb_selected: bool,
    #[cfg(feature = "soapy-sdr")]
    usb_id: String,
    #[cfg(feature = "soapy-sdr")]
    usb_frequency: String,
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
    last_good_checks: BTreeMap<&'static str, SystemTime>,
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            segment: Segment::Ip,
            inputs: InputReport::default(),
            input_scan: None,
            scan_enabled: false,
            source_user_selected: false,
            last_scan: None,
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
            #[cfg(feature = "nrsp-native")]
            native_selected: false,
            #[cfg(feature = "nrsp-native")]
            native_endpoint: String::new(),
            #[cfg(feature = "nrsp-native")]
            native_frequency: "99.100000 MHz".into(),
            #[cfg(feature = "nrsp-native")]
            native_mode: "wfm".into(),
            #[cfg(feature = "nrsp-native")]
            native_bandwidth: "200000".into(),
            #[cfg(feature = "nrsp-native")]
            native_tab: 0,
            #[cfg(feature = "nrsp-native")]
            native_step: 200_000,
            #[cfg(feature = "nrsp-native")]
            native_fm: Default::default(),
            #[cfg(feature = "nrsp-native")]
            native_presets: Default::default(),
            #[cfg(feature = "nrsp-native")]
            native_preset_name: String::new(),
            #[cfg(feature = "nrsp-native")]
            native_presets_dirty: false,
            #[cfg(feature = "sdrconnect")]
            sdr_selected: false,
            #[cfg(feature = "sdrconnect")]
            sdr_endpoint: "127.0.0.1:5454".into(),
            #[cfg(feature = "soapy-sdr")]
            usb_selected: false,
            #[cfg(feature = "soapy-sdr")]
            usb_id: String::new(),
            #[cfg(feature = "soapy-sdr")]
            usb_frequency: "99100000".into(),
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
            last_good_checks: BTreeMap::new(),
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
        self.restart_pending = false;
        self.selection_check_pending = None;
        if let Some(scan) = &self.input_scan {
            scan.cancel();
            self.last_scan = Some(Instant::now());
        }
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
        self.poll_inputs();
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
                            if report.is_decoded() {
                                if let Some(at) = report.checked_at {
                                    self.last_good_checks.insert(station.id, at);
                                }
                            }
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
        if self.engine.is_none() && self.validation.is_none() && self.input_scan.is_none() {
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
        self.engine.is_some()
            || self.validation.is_some()
            || self.input_scan.is_some()
            || self.restart_pending
    }

    fn play(&mut self) {
        self.selection_check_pending = None;
        if self.engine.is_some() || self.restart_pending {
            return;
        }
        if self.validation.is_some() || self.input_scan.is_some() {
            if let Some(task) = &self.validation {
                task.request_cancel();
            }
            if let Some(scan) = &self.input_scan {
                scan.cancel();
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
        if !self.segment_supported() {
            return Err("This receiver type is not included in this build".into());
        }
        #[cfg(feature = "nrsp-native")]
        if self.native_selected {
            let frequency = fm_presets::frequency_hz(&self.native_frequency)?;
            let bandwidth = self
                .native_bandwidth
                .parse()
                .map_err(|_| "Enter a bandwidth in Hz")?;
            if self.native_mode == "wfm" {
                return NativeRadioEngine::start_nrsp_native_fm(
                    &self.native_endpoint,
                    frequency,
                    bandwidth,
                    self.native_fm,
                    self.volume,
                    &self.output,
                );
            }
            return NativeRadioEngine::start_nrsp_native(
                &self.native_endpoint,
                frequency,
                &self.native_mode,
                bandwidth,
                self.volume,
                &self.output,
            );
        }
        if self.direct_selected {
            return NativeRadioEngine::start_stream(&self.direct_url, self.volume, &self.output);
        }
        #[cfg(feature = "soapy-sdr")]
        if self.usb_selected {
            let frequency = self
                .usb_frequency
                .parse::<u64>()
                .map_err(|_| "Enter a positive FM frequency in Hz")?;
            return NativeRadioEngine::start_soapy_wfm(
                &self.usb_id,
                frequency,
                rust_radio_core::sdr::wfm::Deemphasis::Us75,
                self.volume,
                &self.output,
            );
        }
        #[cfg(feature = "sdrconnect")]
        if self.sdr_selected {
            return NativeRadioEngine::start_sdrconnect(
                &self.sdr_endpoint,
                self.volume,
                &self.output,
            );
        }
        NativeRadioEngine::start_internet(self.selected_station().id, self.volume, &self.output)
    }

    #[cfg(feature = "nrsp-native")]
    fn apply_native_tuning(&mut self) {
        let result = (|| {
            let frequency = fm_presets::frequency_hz(&self.native_frequency)?;
            let bandwidth = self
                .native_bandwidth
                .parse::<u32>()
                .map_err(|_| "Enter a bandwidth in Hz")?;
            let engine = self
                .engine
                .as_ref()
                .ok_or("Start the native receiver first")?;
            self.native_fm.validate().map_err(|e| e.to_string())?;
            if self.native_mode == "wfm"
                && self.native_fm.stereo
                && engine
                    .nrsp_native_status()
                    .is_some_and(|status| !status.stereo_allowed)
            {
                return Err("Receiver does not permit stereo FM; select mono".into());
            }
            engine.tune_nrsp_native(frequency, &self.native_mode, bandwidth)?;
            if self.native_mode == "wfm" {
                engine.set_nrsp_native_fm(self.native_fm)?;
            }
            Ok::<(), String>(())
        })();
        self.note = match result {
            Ok(()) => "TUNING REQUESTED".into(),
            Err(error) => error,
        };
    }

    fn ip_target(&self) -> &str {
        if self.direct_selected {
            &self.direct_url
        } else {
            self.selected_station().id
        }
    }

    fn check_ip_selection(&mut self) {
        if self.segment != Segment::Ip || self.engine.is_some() || self.validation.is_some() {
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
        if self.segment != Segment::Ip {
            return;
        }
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
        if self.is_active()
            || self.segment != Segment::Ip
            || self.is_receiver_selected()
            || self.direct_selected
        {
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
            if self.segment != Segment::Ip
                || self.is_receiver_selected()
                || target != self.ip_target()
            {
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
            Some(report) if report.health_at(SystemTime::now()) == FeedHealth::Fresh => "FRESH / DECODED",
            Some(report) if report.health_at(SystemTime::now()) == FeedHealth::Stale => "STALE / RECHECK",
            Some(_) if self.last_good_checks.get(id).is_some_and(|at| {
                SystemTime::now()
                    .duration_since(*at)
                    .is_ok_and(|age| age <= std::time::Duration::from_secs(24 * 60 * 60))
            }) => "DEGRADED / RETRY",
            Some(report) => match report.status {
                ValidationStatus::Unavailable => "CHECK FAILED / RETRY",
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

    #[cfg(test)]
    fn step_station(&mut self, direction: isize) {
        let count = curated_stations().len()
            + usize::from(cfg!(feature = "sdrconnect"))
            + usize::from(cfg!(feature = "soapy-sdr"))
            + 1
            + usize::from(cfg!(feature = "nrsp-native"));
        if count == 0 {
            return;
        }
        let offset = direction.rem_euclid(count as isize) as usize;
        let next = (self.source_index() + offset) % count;
        self.select_source(next);
    }

    fn source_index(&self) -> usize {
        if self.is_native_selected() {
            curated_stations().len()
                + usize::from(cfg!(feature = "sdrconnect"))
                + usize::from(cfg!(feature = "soapy-sdr"))
                + 1
        } else if self.direct_selected {
            curated_stations().len()
                + usize::from(cfg!(feature = "sdrconnect"))
                + usize::from(cfg!(feature = "soapy-sdr"))
        } else if self.is_sdr_selected() {
            curated_stations().len()
        } else if self.is_usb_selected() {
            curated_stations().len() + usize::from(cfg!(feature = "sdrconnect"))
        } else {
            self.station_index
        }
    }

    fn select_source(&mut self, index: usize) {
        let previous = self.source_index();
        self.source_user_selected = true;
        #[cfg(feature = "nrsp-native")]
        {
            self.native_selected = index
                == curated_stations().len()
                    + usize::from(cfg!(feature = "sdrconnect"))
                    + usize::from(cfg!(feature = "soapy-sdr"))
                    + 1;
        }
        self.direct_selected = index
            == curated_stations().len()
                + usize::from(cfg!(feature = "sdrconnect"))
                + usize::from(cfg!(feature = "soapy-sdr"));
        #[cfg(feature = "sdrconnect")]
        {
            self.sdr_selected = index == curated_stations().len();
        }
        #[cfg(feature = "soapy-sdr")]
        {
            self.usb_selected =
                index == curated_stations().len() + usize::from(cfg!(feature = "sdrconnect"));
        }
        if index < curated_stations().len() {
            self.station_index = index;
        }
        self.segment = if self.is_native_selected() || self.is_sdr_selected() {
            Segment::Network
        } else if self.is_usb_selected() {
            Segment::Usb
        } else {
            Segment::Ip
        };
        self.cancel_stale_validation();
        if self.segment == Segment::Ip && previous != index {
            self.restart_ip_if_active();
        }
    }

    fn is_usb_selected(&self) -> bool {
        #[cfg(feature = "soapy-sdr")]
        {
            self.usb_selected
        }
        #[cfg(not(feature = "soapy-sdr"))]
        {
            false
        }
    }

    fn is_native_selected(&self) -> bool {
        #[cfg(feature = "nrsp-native")]
        {
            self.native_selected
        }
        #[cfg(not(feature = "nrsp-native"))]
        {
            false
        }
    }

    fn is_receiver_selected(&self) -> bool {
        self.is_native_selected() || self.is_sdr_selected() || self.is_usb_selected()
    }

    fn is_sdr_selected(&self) -> bool {
        #[cfg(feature = "sdrconnect")]
        {
            self.sdr_selected
        }
        #[cfg(not(feature = "sdrconnect"))]
        {
            false
        }
    }

    fn source_detail(&self) -> String {
        if !self.segment_supported() {
            return format!(
                "{} / SUPPORT NOT INCLUDED IN THIS BUILD",
                self.segment.label()
            );
        }
        #[cfg(feature = "nrsp-native")]
        if self.native_selected {
            if let Some(status) = self
                .engine
                .as_ref()
                .and_then(NativeRadioEngine::nrsp_native_status)
            {
                if status.configured_mode != Some(rust_radio_audio::playback::NativeNrspMode::Wfm) {
                    return format!(
                        "nRSP-ST / {} / {}",
                        fm_presets::format_frequency(status.center_frequency_hz),
                        status
                            .configured_mode
                            .map(|mode| format!("{mode:?}").to_uppercase())
                            .unwrap_or_else(|| "NATIVE".into())
                    );
                }
                let lock = if status.fm_settings.is_some_and(|settings| !settings.stereo) {
                    "MONO"
                } else {
                    match status.stereo_locked {
                        Some(true) => "STEREO LOCK",
                        Some(false) => "NO STEREO LOCK",
                        None => "LOCK UNKNOWN",
                    }
                };
                return format!(
                    "nRSP-ST / {} / {lock}",
                    fm_presets::format_frequency(status.center_frequency_hz)
                );
            }
            return format!(
                "nRSP-ST / {} / {}",
                self.native_frequency,
                self.native_mode.to_uppercase()
            );
        }
        if self.direct_selected {
            return self
                .direct_validation
                .as_ref()
                .map(|report| report.detail.clone())
                .unwrap_or_else(|| "CUSTOM STREAM / PUBLIC HTTPS OR PRIVATE LAN".into());
        }
        #[cfg(feature = "soapy-sdr")]
        if self.usb_selected {
            return format!("USB FM / {} Hz / MONO", self.usb_frequency);
        }
        #[cfg(feature = "sdrconnect")]
        if self.sdr_selected {
            return format!("SDRCONNECT / {}", self.sdr_endpoint);
        }
        let station = self.selected_station();
        format!(
            "{} / {} / {}",
            self.validation_label(station.id),
            station.category,
            station.genre
        )
    }

    fn selected_station(&self) -> &'static InternetStation {
        &curated_stations()[self.station_index]
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod live_ip_tests;
