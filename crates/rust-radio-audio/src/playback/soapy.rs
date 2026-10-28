use super::*;
use rust_radio_core::sdr::wfm::{Deemphasis, AUDIO_RATE_HZ, INPUT_RATE_HZ};
use rust_radio_core::sdr::RadioDevice;
use rust_radio_sdr::{stream_mono_wfm, SoapyReceiver};

pub(super) fn play(
    source: (&str, u64, Deemphasis),
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &AtomicBool,
    volume: Arc<AtomicU8>,
    output_id: &str,
) -> Result<(), PlaybackFailure> {
    let (id, frequency_hz, deemphasis) = source;
    let receiver = SoapyReceiver::open(id, 0, INPUT_RATE_HZ)
        .map_err(|error| PlaybackFailure::Permanent(format!("USB receiver: {error}")))?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    let mut output = NativeAudioOutput::open(volume, output_id).map_err(PlaybackFailure::Output)?;
    let mut converter = PcmConverter::new(AUDIO_RATE_HZ, output.sample_rate, 1, output.channels)
        .map_err(|error| {
            PlaybackFailure::Output(format!("Unsupported output format: {error:?}"))
        })?;
    let result = receive_to_sink(
        receiver,
        (frequency_hz, deemphasis),
        generation,
        controller,
        cancelled,
        &mut output,
        &mut converter,
    );
    if !cancelled.load(Ordering::Acquire) && output.has_failed() {
        return Err(PlaybackFailure::Output(
            "Native audio output disconnected or stopped consuming samples".into(),
        ));
    }
    result
}

fn receive_to_sink<D: RadioDevice>(
    receiver: D,
    tuning: (u64, Deemphasis),
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &AtomicBool,
    output: &mut impl ip_pcm::Sink,
    converter: &mut PcmConverter,
) -> Result<(), PlaybackFailure> {
    let (frequency_hz, deemphasis) = tuning;
    let mut sink = ip_pcm::SinkState::default();
    let mut connected = false;
    let mut output_error = None;
    let result = stream_mono_wfm(receiver, frequency_hz, deemphasis, cancelled, |sample| {
        if !connected {
            signal(controller, generation, PlaybackEvent::Connected);
            connected = true;
        }
        match sink.feed(converter, &[sample], output, cancelled, |event| {
            signal(controller, generation, event)
        }) {
            Ok(true) => true,
            Ok(false) => {
                if !cancelled.load(Ordering::Acquire) {
                    output_error = Some(PlaybackFailure::Output(
                        "Native audio output stopped consuming samples".into(),
                    ));
                }
                false
            }
            Err(error) => {
                output_error = Some(error);
                false
            }
        }
    });
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    if let Some(error) = output_error {
        return Err(error);
    }
    result.map_err(|error| PlaybackFailure::Permanent(format!("USB receiver: {error}")))?;
    Err(PlaybackFailure::Permanent(
        "USB FM receive stream ended unexpectedly".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_radio_core::sdr::{
        RadioCapabilities, RadioError, ReceiverKind, SimulatedRadio, Tuning,
    };
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    struct Fixture {
        caps: RadioCapabilities,
        calls: Rc<RefCell<Vec<&'static str>>>,
        phase: f32,
        sample_index: u64,
    }
    impl RadioDevice for Fixture {
        fn capabilities(&self) -> &RadioCapabilities {
            &self.caps
        }
        fn apply_tuning(&mut self, _: Tuning) -> Result<(), RadioError> {
            self.calls.borrow_mut().push("tune");
            Ok(())
        }
        fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError> {
            self.calls.borrow_mut().push("read");
            for pair in buffer.chunks_exact_mut(2) {
                let time = self.sample_index as f32 / INPUT_RATE_HZ as f32;
                let modulation = (2.0 * std::f32::consts::PI * 1000.0 * time).sin();
                self.phase = (self.phase
                    + 2.0 * std::f32::consts::PI * 45_000.0 * modulation / INPUT_RATE_HZ as f32)
                    .rem_euclid(2.0 * std::f32::consts::PI);
                pair.copy_from_slice(&[
                    (self.phase.cos() * 20_000.0) as i16,
                    (self.phase.sin() * 20_000.0) as i16,
                ]);
                self.sample_index += 1;
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
    struct FixtureSink {
        pushed: usize,
        started: Cell<bool>,
        fail_start: bool,
        peak: f32,
    }
    impl ip_pcm::Sink for FixtureSink {
        fn push(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool {
            for sample in frame {
                self.peak = self.peak.max(sample.abs());
            }
            self.pushed += 2;
            if self.pushed >= 100 {
                cancelled.store(true, Ordering::Release);
            }
            true
        }
        fn buffered(&self) -> usize {
            self.pushed
        }
        fn prebuffer(&self) -> usize {
            10
        }
        fn played(&self) -> u64 {
            if self.started.get() && self.pushed > 10 {
                2
            } else {
                0
            }
        }
        fn start(&self) -> Result<(), String> {
            if self.fail_start {
                return Err("Output unavailable".into());
            }
            self.started.set(true);
            Ok(())
        }
    }

    #[test]
    fn usb_source_reaches_callback_evidence_and_cleans_up_on_stop_or_output_failure() {
        for fail_start in [false, true] {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let receiver = Fixture {
                caps: SimulatedRadio::default().capabilities().clone(),
                calls: Rc::clone(&calls),
                phase: 0.0,
                sample_index: 0,
            };
            let mut control = RadioController::default();
            let generation = control.apply(RadioCommand::PlaySdr).unwrap();
            let control = Arc::new(Mutex::new(control));
            let cancelled = AtomicBool::new(false);
            let mut output = FixtureSink {
                pushed: 0,
                started: Cell::new(false),
                fail_start,
                peak: 0.0,
            };
            let mut converter = PcmConverter::new(AUDIO_RATE_HZ, AUDIO_RATE_HZ, 1, 2).unwrap();
            let result = receive_to_sink(
                receiver,
                (100_100_000, Deemphasis::Us75),
                generation,
                &control,
                &cancelled,
                &mut output,
                &mut converter,
            );
            assert_eq!(*calls.borrow(), ["tune", "read", "stop"]);
            if fail_start {
                assert!(matches!(result, Err(PlaybackFailure::Output(_))));
                assert_ne!(
                    control.lock().unwrap().state().state,
                    PlaybackState::Playing
                );
            } else {
                assert!(result.is_ok());
                assert!(
                    output.peak > 0.05,
                    "synthetic FM tone did not reach the output sink"
                );
                assert!(cancelled.load(Ordering::Acquire));
                assert_eq!(
                    control.lock().unwrap().state().state,
                    PlaybackState::Playing
                );
            }
        }
    }

    #[test]
    fn invalid_usb_selection_is_rejected_before_a_worker_is_started() {
        assert!(NativeRadioEngine::start_soapy_wfm(
            "driver=remote",
            100_100_000,
            Deemphasis::Us75,
            10,
            "system"
        )
        .is_err());
        assert!(NativeRadioEngine::start_soapy_wfm(
            "soapy-0000000000000000",
            0,
            Deemphasis::Us75,
            10,
            "system"
        )
        .is_err());
        assert!(NativeRadioEngine::start_soapy_wfm(
            "soapy-0000000000000000",
            100_100_000,
            Deemphasis::Us75,
            101,
            "system"
        )
        .is_err());
    }

    #[test]
    #[ignore = "manual native audio output check"]
    fn synthetic_usb_iq_reaches_native_audio_callback() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let receiver = Fixture {
            caps: SimulatedRadio::default().capabilities().clone(),
            calls: Rc::clone(&calls),
            phase: 0.0,
            sample_index: 0,
        };
        let mut control = RadioController::default();
        let generation = control.apply(RadioCommand::PlaySdr).unwrap();
        let control = Arc::new(Mutex::new(control));
        let cancelled = Arc::new(AtomicBool::new(false));
        let monitor_control = Arc::clone(&control);
        let monitor_cancelled = Arc::clone(&cancelled);
        let monitor = std::thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(8);
            let mut playing = false;
            while Instant::now() < until {
                if monitor_control.lock().unwrap().state().state == PlaybackState::Playing {
                    playing = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            monitor_cancelled.store(true, Ordering::Release);
            playing
        });
        let mut output = NativeAudioOutput::open(Arc::new(AtomicU8::new(0)), "system").unwrap();
        let mut converter =
            PcmConverter::new(AUDIO_RATE_HZ, output.sample_rate, 1, output.channels).unwrap();
        let result = receive_to_sink(
            receiver,
            (100_100_000, Deemphasis::Us75),
            generation,
            &control,
            &cancelled,
            &mut output,
            &mut converter,
        );
        assert!(
            monitor.join().unwrap(),
            "native output did not consume synthetic PCM"
        );
        assert!(result.is_ok());
        assert_eq!(calls.borrow().first(), Some(&"tune"));
        assert_eq!(calls.borrow().last(), Some(&"stop"));
    }
}
