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
mod theme;
mod view;

pub const PANEL_WIDTH: f32 = 480.0;
pub const PANEL_HEIGHT: f32 = 320.0;

/// Owns one radio session. Embed with `panel.show(ui)` inside an existing HUD.
pub struct RadioPanel {
    station_index: usize,
    direct_selected: bool,
    direct_url: String,
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
}

impl Default for RadioPanel {
    fn default() -> Self {
        Self {
            station_index: 0,
            direct_selected: false,
            direct_url: String::new(),
            #[cfg(feature = "nrsp-native")]
            native_selected: false,
            #[cfg(feature = "nrsp-native")]
            native_endpoint: String::new(),
            #[cfg(feature = "nrsp-native")]
            native_frequency: "99100000".into(),
            #[cfg(feature = "nrsp-native")]
            native_mode: "wfm".into(),
            #[cfg(feature = "nrsp-native")]
            native_bandwidth: "200000".into(),
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
        match self.start_selected() {
            Ok(engine) => {
                self.engine = Some(engine);
                self.last_session = None;
                self.note = "CONNECTING / NATIVE AUDIO".into();
            }
            Err(error) => {
                self.last_session = None;
                self.note = error;
            }
        }
    }

    fn start_selected(&self) -> Result<NativeRadioEngine, String> {
        #[cfg(feature = "nrsp-native")]
        if self.native_selected {
            let frequency = self
                .native_frequency
                .parse()
                .map_err(|_| "Enter a frequency in Hz")?;
            let bandwidth = self
                .native_bandwidth
                .parse()
                .map_err(|_| "Enter a bandwidth in Hz")?;
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
            let frequency = self
                .native_frequency
                .parse::<u64>()
                .map_err(|_| "Enter a frequency in Hz")?;
            let bandwidth = self
                .native_bandwidth
                .parse::<u32>()
                .map_err(|_| "Enter a bandwidth in Hz")?;
            let engine = self
                .engine
                .as_ref()
                .ok_or("Start the native receiver first")?;
            engine.tune_nrsp_native(frequency, &self.native_mode, bandwidth)
        })();
        self.note = match result {
            Ok(()) => "TUNING REQUESTED".into(),
            Err(error) => error,
        };
    }

    fn start_validation(&mut self) {
        if self.is_active() || self.is_receiver_selected() || self.direct_selected {
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
            if self.is_receiver_selected()
                || self.direct_selected
                || task.station_id() != self.selected_station().id
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
        self.cancel_stale_validation();
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
        #[cfg(feature = "nrsp-native")]
        if self.native_selected {
            if let Some(status) = self
                .engine
                .as_ref()
                .and_then(NativeRadioEngine::nrsp_native_status)
            {
                return format!(
                    "NATIVE nRSP-ST / {} Hz / OPUS 48 kHz",
                    status.center_frequency_hz
                );
            }
            return format!(
                "NATIVE nRSP-ST / {} Hz / {}",
                self.native_frequency,
                self.native_mode.to_uppercase()
            );
        }
        if self.direct_selected {
            return "DIRECT STREAM / SET URL IN SETTINGS".into();
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
