use crate::MAX_IQ_VALUES;
use rust_radio_core::sdr::wfm::{Deemphasis, MonoWfm, INPUT_RATE_HZ};
use rust_radio_core::sdr::{
    Demodulation, RadioDevice, RadioError, RadioTuner, ReceiverKind, Tuning,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const NO_IQ_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WfmReport {
    pub receiver_kind: ReceiverKind,
    pub iq_pairs: u64,
    pub audio_frames: u64,
    pub cancelled: bool,
}

/// Explicit continuous mono FM receive operation. The device must support
/// 240 ksample/s RawIq and bounded reads. The callback must be brief and return
/// false when its consumer is stopping. All normal and error exits stop the tuner.
pub fn stream_mono_wfm<D: RadioDevice>(
    device: D,
    frequency_hz: u64,
    deemphasis: Deemphasis,
    cancelled: &AtomicBool,
    mut on_audio: impl FnMut(f32) -> bool,
) -> Result<WfmReport, RadioError> {
    let mut tuner = RadioTuner::new(device)?;
    let kind = tuner.receiver_kind();
    let mut report = WfmReport {
        receiver_kind: kind,
        iq_pairs: 0,
        audio_frames: 0,
        cancelled: cancelled.load(Ordering::Acquire),
    };
    if report.cancelled {
        return Ok(report);
    }
    tuner.tune(Tuning {
        frequency_hz,
        sample_rate_hz: INPUT_RATE_HZ,
        demodulation: Demodulation::RawIq,
        gain_db_tenths: None,
    })?;
    let result = (|| {
        let mut iq = vec![0i16; MAX_IQ_VALUES];
        let mut demod = MonoWfm::new(deemphasis);
        let mut last_iq = Instant::now();
        while !cancelled.load(Ordering::Acquire) {
            let count = tuner.read_iq(&mut iq)?;
            if count == 0 {
                if last_iq.elapsed() >= NO_IQ_TIMEOUT {
                    return Err(RadioError::Device(
                        "SDR receiver stopped delivering I/Q samples".into(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            report.iq_pairs = report.iq_pairs.saturating_add((count / 2) as u64);
            let keep_going = demod
                .process(&iq[..count], |sample| {
                    if cancelled.load(Ordering::Acquire) {
                        return false;
                    }
                    report.audio_frames = report.audio_frames.saturating_add(1);
                    on_audio(sample)
                })
                .map_err(|error| RadioError::Device(format!("Invalid I/Q stream: {error:?}")))?;
            if !keep_going {
                break;
            }
            // Output backpressure is not a receiver read timeout.
            last_iq = Instant::now();
        }
        report.cancelled = cancelled.load(Ordering::Acquire);
        Ok(report)
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
    use rust_radio_core::sdr::{RadioCapabilities, SimulatedRadio};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Fixture {
        caps: RadioCapabilities,
        calls: Rc<RefCell<Vec<&'static str>>>,
        malformed: bool,
        fail_read: bool,
    }
    impl RadioDevice for Fixture {
        fn capabilities(&self) -> &RadioCapabilities {
            &self.caps
        }
        fn apply_tuning(&mut self, tuning: Tuning) -> Result<(), RadioError> {
            assert_eq!(tuning.demodulation, Demodulation::RawIq);
            assert_eq!(tuning.sample_rate_hz, INPUT_RATE_HZ);
            self.calls.borrow_mut().push("tune");
            Ok(())
        }
        fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError> {
            self.calls.borrow_mut().push("read");
            if self.fail_read {
                return Err(RadioError::Device("receiver disconnected".into()));
            }
            if self.malformed {
                return Ok(3);
            }
            for pair in buffer.chunks_exact_mut(2) {
                pair.copy_from_slice(&[20_000, 0]);
            }
            Ok(buffer.len())
        }
        fn stop(&mut self) -> Result<(), RadioError> {
            self.calls.borrow_mut().push("stop");
            Ok(())
        }
        fn receiver_kind(&self) -> ReceiverKind {
            ReceiverKind::Simulated
        }
    }

    fn fixture(malformed: bool) -> (Fixture, Rc<RefCell<Vec<&'static str>>>) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        (
            Fixture {
                caps: SimulatedRadio::default().capabilities().clone(),
                calls: Rc::clone(&calls),
                malformed,
                fail_read: false,
            },
            calls,
        )
    }

    #[test]
    fn explicit_receive_delivers_pcm_and_stops_on_consumer_completion() {
        let (device, calls) = fixture(false);
        let mut frames = 0;
        let report = stream_mono_wfm(
            device,
            100_100_000,
            Deemphasis::Us75,
            &AtomicBool::new(false),
            |_| {
                frames += 1;
                frames < 32
            },
        )
        .unwrap();
        assert_eq!(frames, 32);
        assert_eq!(report.audio_frames, 32);
        assert_eq!(report.receiver_kind, ReceiverKind::Simulated);
        assert_eq!(*calls.borrow(), ["tune", "read", "stop"]);
    }

    #[test]
    fn malformed_driver_data_stops_and_never_emits_audio() {
        let (device, calls) = fixture(true);
        assert!(stream_mono_wfm(
            device,
            100_100_000,
            Deemphasis::Us75,
            &AtomicBool::new(false),
            |_| panic!("malformed input emitted audio")
        )
        .is_err());
        assert_eq!(*calls.borrow(), ["tune", "read", "stop"]);
    }

    #[test]
    fn driver_read_failure_stops_the_receiver_and_preserves_the_error() {
        let (mut device, calls) = fixture(false);
        device.fail_read = true;
        let result = stream_mono_wfm(
            device,
            100_100_000,
            Deemphasis::Us75,
            &AtomicBool::new(false),
            |_| panic!("failed read emitted audio"),
        );
        assert_eq!(
            result,
            Err(RadioError::Device("receiver disconnected".into()))
        );
        assert_eq!(*calls.borrow(), ["tune", "read", "stop"]);
    }

    #[test]
    fn pre_cancelled_receive_does_not_activate_the_device() {
        let (device, calls) = fixture(false);
        let report = stream_mono_wfm(
            device,
            100_100_000,
            Deemphasis::Us75,
            &AtomicBool::new(true),
            |_| panic!("cancelled input emitted audio"),
        )
        .unwrap();
        assert!(report.cancelled);
        assert!(calls.borrow().is_empty());
    }
}
