//! Explicit background discovery; it never starts RF reception or audio output.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputDevice {
    pub id: String,
    pub label: String,
    pub ready: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputReport {
    pub internet: bool,
    pub network: Vec<InputDevice>,
    pub usb: Vec<InputDevice>,
    pub network_note: String,
    pub usb_note: String,
}
impl InputReport {
    pub fn network_available(&self) -> bool {
        self.network.iter().any(|device| device.ready)
    }
    pub fn usb_available(&self) -> bool {
        self.usb.iter().any(|device| device.ready)
    }
}

pub struct InputScan {
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<InputReport>>,
}
impl InputScan {
    /// Scan local receiver neighborhoods and driver descriptors on a worker.
    /// The optional endpoint is an operator-entered private peer, not persisted.
    pub fn start(endpoint: Option<String>) -> Result<Self, String> {
        Self::with_worker(move |cancel| scan(&cancel, endpoint.as_deref()))
    }
    fn with_worker(
        work: impl FnOnce(Arc<AtomicBool>) -> InputReport + Send + 'static,
    ) -> Result<Self, String> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker = thread::Builder::new()
            .name("rust-radio-discovery".into())
            .spawn(move || work(worker_cancelled))
            .map_err(|error| format!("Could not start input discovery: {error}"))?;
        Ok(Self {
            cancelled,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn try_finish(&mut self) -> Option<Result<Option<InputReport>, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .map_err(|_| "Input discovery worker failed".to_owned());
        Some(result.map(|report| {
            if self.cancelled.load(Ordering::Acquire) {
                None
            } else {
                Some(report)
            }
        }))
    }
}
impl Drop for InputScan {
    fn drop(&mut self) {
        self.cancel();
    } // Never block a UI on a driver enumeration.
}
fn scan(cancel: &Arc<AtomicBool>, endpoint: Option<&str>) -> InputReport {
    thread::scope(|scope| {
        let internet = scope.spawn(|| rust_radio_ip::probe::internet_available(cancel));
        let network = scope.spawn(|| network_scan(cancel, endpoint));
        let usb = scope.spawn(|| usb_scan(cancel));
        let (network, network_note) = network
            .join()
            .unwrap_or_else(|_| (Vec::new(), "Network discovery failed".into()));
        let (usb, usb_note) = usb
            .join()
            .unwrap_or_else(|_| (Vec::new(), "USB discovery failed".into()));
        InputReport {
            internet: internet.join().unwrap_or(false),
            network,
            usb,
            network_note,
            usb_note,
        }
    })
}
fn network_scan(cancel: &AtomicBool, endpoint: Option<&str>) -> (Vec<InputDevice>, String) {
    #[cfg(feature = "nrsp-native")]
    {
        match rust_radio_sdr::nrsp_native::discover(cancel, endpoint) {
            Ok(devices) => (
                devices
                    .into_iter()
                    .map(|device| InputDevice {
                        id: device.endpoint,
                        label: format!("nRSP-ST / model {}", device.status.model_number),
                        ready: device.status.audio_allowed
                            && !device.status.authentication_required,
                    })
                    .collect(),
                "Local receiver scan complete".into(),
            ),
            Err(error) => (Vec::new(), error.to_string()),
        }
    }
    #[cfg(not(feature = "nrsp-native"))]
    {
        let _ = (cancel, endpoint);
        (
            Vec::new(),
            "Network receiver support is not included in this build".into(),
        )
    }
}
fn usb_scan(cancel: &AtomicBool) -> (Vec<InputDevice>, String) {
    if cancel.load(Ordering::Acquire) {
        return (Vec::new(), "USB scan cancelled".into());
    }
    #[cfg(feature = "soapy")]
    {
        match rust_radio_sdr::discover() {
            Ok(devices) => (
                devices
                    .into_iter()
                    .filter(|device| {
                        matches!(
                            device.driver.as_str(),
                            "rtlsdr"
                                | "sdrplay"
                                | "airspy"
                                | "airspyhf"
                                | "hackrf"
                                | "lime"
                                | "bladerf"
                        )
                    })
                    .map(|device| InputDevice {
                        id: device.id,
                        label: device.label,
                        ready: true,
                    })
                    .collect(),
                "USB receiver scan complete".into(),
            ),
            Err(error) => (Vec::new(), format!("{error:?}")),
        }
    }
    #[cfg(not(feature = "soapy"))]
    {
        (
            Vec::new(),
            "USB receiver support is not included in this build".into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_scan_discards_late_results_without_blocking_poll() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut task = InputScan::with_worker(move |_| {
            rx.recv().unwrap();
            InputReport {
                internet: true,
                ..Default::default()
            }
        })
        .unwrap();
        assert!(task.try_finish().is_none());
        task.cancel();
        tx.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(result) = task.try_finish() {
                assert_eq!(result.unwrap(), None);
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
    }
}
