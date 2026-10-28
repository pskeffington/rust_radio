use super::*;
use rust_radio_audio::discovery::{InputReport, InputScan};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Segment {
    #[default]
    Ip,
    Network,
    Usb,
}
impl Segment {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ip => "IP Radio",
            Self::Network => "nSDR",
            Self::Usb => "USB",
        }
    }
}
impl RadioPanel {
    /// Opt into startup and idle input scans. Construction remains offline.
    /// Embedded hosts can instead call `scan_inputs` when the operator requests it.
    pub fn enable_input_discovery(&mut self) {
        self.scan_enabled = true;
    }
    pub fn scan_inputs(&mut self) {
        if self.input_scan.is_some() || self.engine.is_some() {
            return;
        }
        #[cfg(feature = "nrsp-native")]
        let endpoint = (!self.native_endpoint.is_empty()).then(|| self.native_endpoint.clone());
        #[cfg(not(feature = "nrsp-native"))]
        let endpoint = None;
        match InputScan::start(endpoint) {
            Ok(scan) => {
                self.input_scan = Some(scan);
                self.note = "SCANNING RADIO INPUTS".into();
            }
            Err(error) => self.note = error,
        }
    }
    pub(super) fn maybe_scan_inputs(&mut self) {
        if self.scan_enabled
            && !self.is_active()
            && !self.stopping
            && self
                .last_scan
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(30))
        {
            self.scan_inputs();
        }
    }
    pub(super) fn poll_inputs(&mut self) {
        let result = self.input_scan.as_mut().and_then(InputScan::try_finish);
        if let Some(result) = result {
            self.input_scan = None;
            self.last_scan = Some(Instant::now());
            match result {
                Ok(Some(report)) => self.apply_inputs(report),
                Ok(None) => self.note = "INPUT SCAN CANCELLED".into(),
                Err(error) => self.note = error,
            }
        }
    }
    pub(super) fn apply_inputs(&mut self, report: InputReport) {
        #[cfg(feature = "nrsp-native")]
        if let Some(device) = report.network.iter().find(|device| device.ready) {
            if !report
                .network
                .iter()
                .any(|device| device.id == self.native_endpoint)
            {
                self.native_endpoint = device.id.clone();
            }
        }
        #[cfg(feature = "soapy-sdr")]
        if let Some(device) = report.usb.iter().find(|device| device.ready) {
            if !report.usb.iter().any(|device| device.id == self.usb_id) {
                self.usb_id = device.id.clone();
            }
        }
        self.inputs = report;
        if self.engine.is_none()
            && self.validation.is_none()
            && !self.restart_pending
            && !self.source_user_selected
        {
            let selected = if self.inputs.network_available() {
                Segment::Network
            } else if self.inputs.usb_available() {
                Segment::Usb
            } else {
                Segment::Ip
            };
            let user_selected = self.source_user_selected;
            self.select_segment(selected, false);
            self.source_user_selected = user_selected;
        }
        if self.engine.is_none()
            && self.last_session.is_none()
            && !self.restart_pending
            && self.selection_check_pending.is_none()
        {
            self.note = format!(
                "INPUTS / IP {} / nSDR {} / USB {}",
                if self.inputs.internet {
                    "UP"
                } else {
                    "OFFLINE"
                },
                self.inputs.network.len(),
                self.inputs.usb.len()
            );
        }
    }
    pub(super) fn segment_available(&self, segment: Segment) -> bool {
        if self.engine.as_ref().is_some_and(|engine| {
            engine.status().state == rust_radio_audio::compat_controller::PlaybackState::Playing
        }) && self.segment == segment
        {
            return true;
        }
        match segment {
            Segment::Ip => self.inputs.internet,
            Segment::Network => self.inputs.network_available(),
            Segment::Usb => self.inputs.usb_available(),
        }
    }
    pub(super) fn segment_supported(&self) -> bool {
        match self.segment {
            Segment::Ip => true,
            Segment::Network => cfg!(feature = "nrsp-native") || cfg!(feature = "sdrconnect"),
            Segment::Usb => cfg!(feature = "soapy-sdr"),
        }
    }
    pub(super) fn step_segment_source(&mut self, direction: isize) {
        #[cfg(feature = "nrsp-native")]
        if self.is_native_selected() {
            self.step_native_frequency(direction > 0);
            return;
        }
        if self.segment == Segment::Ip {
            let count = self.ip_station_count();
            let current = self.ip_station_index();
            let next = (current + direction.rem_euclid(count as isize) as usize) % count;
            self.select_ip_station(next);
            self.ip_selection_changed();
        }
    }
    pub(super) fn select_segment(&mut self, segment: Segment, setup: bool) {
        if self.engine.is_some() && self.segment != segment {
            self.note = "STOP BEFORE SWITCHING INPUTS".into();
            return;
        }
        if self.engine.is_none() && self.segment != segment {
            let index = match segment {
                Segment::Ip => {
                    if self.saved_station_index.is_some() {
                        curated_stations().len()
                            + usize::from(cfg!(feature = "sdrconnect"))
                            + usize::from(cfg!(feature = "soapy-sdr"))
                    } else {
                        self.station_index
                    }
                }
                Segment::Network if cfg!(feature = "nrsp-native") => {
                    curated_stations().len()
                        + usize::from(cfg!(feature = "sdrconnect"))
                        + usize::from(cfg!(feature = "soapy-sdr"))
                        + 1
                }
                Segment::Network if cfg!(feature = "sdrconnect") => curated_stations().len(),
                Segment::Usb if cfg!(feature = "soapy-sdr") => {
                    curated_stations().len() + usize::from(cfg!(feature = "sdrconnect"))
                }
                _ => self.station_index,
            };
            self.select_source(index);
            self.segment = segment;
            self.cancel_stale_validation();
        }
        if setup {
            self.source_user_selected = true;
            self.settings_open = true;
            self.refresh_outputs();
        }
    }
}
