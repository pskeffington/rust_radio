//! Explicit background internet reachability check; it never starts playback.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputReport {
    pub internet: bool,
}

pub struct InputScan {
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<InputReport>>,
}

impl InputScan {
    pub fn start() -> Result<Self, String> {
        Self::with_worker(|cancel| InputReport {
            internet: rust_radio_ip::probe::internet_available(&cancel),
        })
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
            .map_err(|_| "Input reachability worker failed".to_owned());
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
            InputReport { internet: true }
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
