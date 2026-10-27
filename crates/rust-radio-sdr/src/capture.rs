use rust_radio_core::sdr::{RadioDevice, RadioError, RadioTuner, ReceiverKind, Tuning};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const MAX_CAPTURE_DURATION: Duration = Duration::from_secs(5);
pub const MAX_IQ_VALUES: usize = 16_384;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    pub receiver_kind: ReceiverKind,
    pub complex_samples: u64,
    pub elapsed: Duration,
    pub cancelled: bool,
}

/// Explicit bounded I/Q health check; samples are discarded, never stored or
/// transmitted. A sample count proves driver ingress, not signal demodulation.
/// Device reads must have a finite timeout; the Soapy adapter uses 100 ms.
pub fn capture<D: RadioDevice>(
    device: D,
    tuning: Tuning,
    duration: Duration,
    cancelled: &AtomicBool,
) -> Result<CaptureReport, RadioError> {
    if duration.is_zero() || duration > MAX_CAPTURE_DURATION {
        return Err(RadioError::Device(
            "Capture duration must be greater than zero and at most 5 seconds".into(),
        ));
    }
    let mut tuner = RadioTuner::new(device)?;
    let kind = tuner.receiver_kind();
    let start = Instant::now();
    if cancelled.load(Ordering::Acquire) {
        return Ok(CaptureReport {
            receiver_kind: kind,
            complex_samples: 0,
            elapsed: start.elapsed(),
            cancelled: true,
        });
    }
    tuner.tune(tuning)?;
    let result = (|| {
        let mut buffer = vec![0; MAX_IQ_VALUES];
        let mut count = 0u64;
        while start.elapsed() < duration && !cancelled.load(Ordering::Acquire) {
            let received = tuner.read_iq(&mut buffer)?;
            count = count.saturating_add((received / 2) as u64);
            if received == 0 {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Ok(CaptureReport {
            receiver_kind: kind,
            complex_samples: count,
            elapsed: start.elapsed(),
            cancelled: cancelled.load(Ordering::Acquire),
        })
    })();
    let stopped = tuner.stop();
    match (result, stopped) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(report), Ok(())) => Ok(report),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_radio_core::sdr::{Demodulation, RadioCapabilities, SimulatedRadio};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    #[derive(Default)]
    struct Calls {
        tuned: usize,
        read: usize,
        stopped: usize,
    }

    struct MockDevice {
        caps: RadioCapabilities,
        calls: Rc<RefCell<Calls>>,
        cancelled: Arc<AtomicBool>,
        fail_tune: bool,
        count: Option<usize>,
    }

    impl RadioDevice for MockDevice {
        fn capabilities(&self) -> &RadioCapabilities {
            &self.caps
        }
        fn apply_tuning(&mut self, _: Tuning) -> Result<(), RadioError> {
            self.calls.borrow_mut().tuned += 1;
            if self.fail_tune {
                Err(RadioError::Device("tuning failed".into()))
            } else {
                Ok(())
            }
        }
        fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError> {
            assert!(buffer.len() <= MAX_IQ_VALUES);
            self.calls.borrow_mut().read += 1;
            self.cancelled.store(true, Ordering::Release);
            self.count
                .ok_or_else(|| RadioError::Device("receiver disconnected".into()))
        }
        fn stop(&mut self) -> Result<(), RadioError> {
            self.calls.borrow_mut().stopped += 1;
            Ok(())
        }
        fn receiver_kind(&self) -> ReceiverKind {
            ReceiverKind::Simulated
        }
    }

    fn fixture(count: Option<usize>) -> (MockDevice, Rc<RefCell<Calls>>, Arc<AtomicBool>) {
        let calls = Rc::new(RefCell::new(Calls::default()));
        let cancelled = Arc::new(AtomicBool::new(false));
        (
            MockDevice {
                caps: SimulatedRadio::default().capabilities().clone(),
                calls: Rc::clone(&calls),
                cancelled: Arc::clone(&cancelled),
                fail_tune: false,
                count,
            },
            calls,
            cancelled,
        )
    }

    fn tuning() -> Tuning {
        Tuning {
            frequency_hz: 100_100_000,
            sample_rate_hz: 1_024_000,
            demodulation: Demodulation::RawIq,
            gain_db_tenths: None,
        }
    }

    #[test]
    fn bounded_capture_cancels_and_stops_without_claiming_real_rf() {
        let (device, calls, cancelled) = fixture(Some(32));
        let report = capture(device, tuning(), Duration::from_secs(1), &cancelled).unwrap();
        assert_eq!(report.receiver_kind, ReceiverKind::Simulated);
        assert_eq!(report.complex_samples, 16);
        assert!(report.cancelled);
        assert_eq!(calls.borrow().tuned, 1);
        assert_eq!(calls.borrow().read, 1);
        assert_eq!(calls.borrow().stopped, 1);
    }

    #[test]
    fn disconnected_or_malformed_driver_reads_always_stop_the_receiver() {
        for count in [None, Some(3), Some(MAX_IQ_VALUES + 2)] {
            let (device, calls, cancelled) = fixture(count);
            assert!(capture(device, tuning(), Duration::from_secs(1), &cancelled).is_err());
            assert_eq!(calls.borrow().read, 1);
            assert_eq!(calls.borrow().stopped, 1);
        }
    }

    #[test]
    fn failed_tuning_stops_and_never_reads() {
        let (mut device, calls, cancelled) = fixture(Some(32));
        device.fail_tune = true;
        assert!(capture(device, tuning(), Duration::from_secs(1), &cancelled).is_err());
        assert_eq!(calls.borrow().tuned, 1);
        assert_eq!(calls.borrow().read, 0);
        assert_eq!(calls.borrow().stopped, 1);
    }

    #[test]
    fn invalid_limits_and_pre_cancelled_capture_never_tune_or_read() {
        for duration in [Duration::ZERO, Duration::from_secs(6)] {
            let (device, calls, cancelled) = fixture(Some(32));
            assert!(capture(device, tuning(), duration, &cancelled).is_err());
            assert_eq!(calls.borrow().tuned, 0);
            assert_eq!(calls.borrow().read, 0);
        }
        let (device, calls, cancelled) = fixture(Some(32));
        cancelled.store(true, Ordering::Release);
        let report = capture(device, tuning(), Duration::from_secs(1), &cancelled).unwrap();
        assert!(report.cancelled);
        assert_eq!(report.complex_samples, 0);
        assert_eq!(calls.borrow().tuned, 0);
        assert_eq!(calls.borrow().read, 0);
    }

    #[test]
    fn simulator_and_stop_state_remain_explicit() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        assert_eq!(tuner.receiver_kind(), ReceiverKind::Simulated);
        tuner.tune(tuning()).unwrap();
        tuner.stop().unwrap();
        assert_eq!(tuner.active_tuning(), None);
        assert_eq!(tuner.read_iq(&mut [0; 8]), Err(RadioError::NotConfigured));
    }
}
