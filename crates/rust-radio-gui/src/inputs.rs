use super::*;
use rust_radio_audio::discovery::{InputReport, InputScan};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Segment {
    #[default]
    Ip,
}
impl RadioPanel {
    /// Opt into periodic internet checks. Construction remains offline.
    pub fn enable_input_discovery(&mut self) {
        self.scan_enabled = true;
    }

    pub fn scan_inputs(&mut self) {
        if self.input_scan.is_some() || self.engine.is_some() {
            return;
        }
        match InputScan::start() {
            Ok(scan) => {
                self.input_scan = Some(scan);
                self.note = "CHECKING INTERNET CONNECTION".into();
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
                Ok(None) => self.note = "INTERNET CHECK CANCELLED".into(),
                Err(error) => self.note = error,
            }
        }
    }

    pub(super) fn apply_inputs(&mut self, report: InputReport) {
        self.inputs = report;
        if self.engine.is_none()
            && self.last_session.is_none()
            && !self.restart_pending
            && self.selection_check_pending.is_none()
        {
            self.note = if self.inputs.internet {
                "INTERNET AVAILABLE".into()
            } else {
                "INTERNET NOT VERIFIED".into()
            };
        }
    }

    pub(super) fn segment_supported(&self) -> bool {
        true
    }

    pub(super) fn step_segment_source(&mut self, direction: isize) {
        let count = self.ip_station_count();
        if count == 0 {
            return;
        }
        let next =
            (self.ip_station_index() + direction.rem_euclid(count as isize) as usize) % count;
        self.select_ip_station(next);
        self.ip_selection_changed();
    }
}
