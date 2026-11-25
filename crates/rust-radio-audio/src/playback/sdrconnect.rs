use super::*;
use rust_radio_sdr::sdrconnect::{Client, Endpoint, Error, Status};

fn api_failure(error: Error) -> PlaybackFailure {
    match error {
        Error::Transport(_) | Error::Timeout => PlaybackFailure::Network(error.to_string()),
        _ => PlaybackFailure::Permanent(error.to_string()),
    }
}

pub(super) fn play(
    endpoint: Endpoint,
    context: &PlaybackContext<'_>,
    receiver: &Arc<Mutex<Option<Status>>>,
) -> Result<(), PlaybackFailure> {
    *receiver.lock().expect("receiver state mutex poisoned") = None;
    let mut client = Client::connect(endpoint, context.cancelled).map_err(api_failure)?;
    let mut state = client.status(context.cancelled).map_err(api_failure)?;
    if state.active_device.is_empty() {
        return Err(api_failure(Error::NotStreaming));
    }
    if !state.started {
        if !state.can_control {
            return Err(api_failure(Error::ControlUnavailable));
        }
        client
            .start_receiver(context.cancelled)
            .map_err(api_failure)?;
        state.started = true;
    }
    *receiver.lock().expect("receiver state mutex poisoned") = Some(state);
    if context.cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    let played_samples = begin_output_attempt(context.output_progress);
    let mut output = NativeAudioOutput::open_with_progress(
        Arc::clone(&context.volume),
        context.output_id,
        played_samples,
        Arc::clone(&context.output_underruns),
    )
    .map_err(PlaybackFailure::Output)?;
    let mut pump = StereoPump::new(&output)?;
    signal(
        context.controller,
        context.generation,
        PlaybackEvent::Connected,
    );
    let mut render_error = None;
    let result = client.stream_audio(context.cancelled, |bytes| {
        if context.cancelled.load(Ordering::Acquire) {
            return false;
        }
        match pump.feed(bytes, &mut output, context.cancelled, |event| {
            signal(context.controller, context.generation, event)
        }) {
            Ok(()) => !context.cancelled.load(Ordering::Acquire),
            Err(error) => {
                render_error = Some(error);
                false
            }
        }
    });
    if context.cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    if output.has_failed() {
        return Err(PlaybackFailure::Output(
            "Native SDR audio output disconnected or stopped consuming PCM".into(),
        ));
    }
    if let Some(error) = render_error {
        return Err(error);
    }
    result.map_err(api_failure)?;
    Err(PlaybackFailure::Permanent(
        "SDR audio subscription ended unexpectedly".into(),
    ))
}

trait Sink {
    fn rate(&self) -> u32;
    fn channels(&self) -> usize;
    fn buffered(&self) -> usize;
    fn prebuffer(&self) -> usize;
    fn played(&self) -> u64;
    fn push(&mut self, frame: &[f32], cancel: &AtomicBool) -> bool;
    fn start(&self) -> Result<(), String>;
}
impl Sink for NativeAudioOutput {
    fn rate(&self) -> u32 {
        self.sample_rate
    }
    fn channels(&self) -> usize {
        self.channels
    }
    fn buffered(&self) -> usize {
        self.buffered_samples()
    }
    fn prebuffer(&self) -> usize {
        self.minimum_prebuffer_samples()
    }
    fn played(&self) -> u64 {
        self.played_samples()
    }
    fn push(&mut self, frame: &[f32], cancel: &AtomicBool) -> bool {
        self.push_frame(frame, cancel)
    }
    fn start(&self) -> Result<(), String> {
        self.start()
    }
}

struct StereoPump {
    converter: PcmConverter,
    started: bool,
    announced: bool,
}
impl StereoPump {
    fn new(output: &impl Sink) -> Result<Self, PlaybackFailure> {
        Ok(Self {
            converter: PcmConverter::new(48_000, output.rate(), 2, output.channels()).map_err(
                |e| PlaybackFailure::Output(format!("Unsupported SDR PCM output: {e:?}")),
            )?,
            started: false,
            announced: false,
        })
    }
    fn feed(
        &mut self,
        bytes: &[u8],
        output: &mut impl Sink,
        cancelled: &AtomicBool,
        mut event: impl FnMut(PlaybackEvent),
    ) -> Result<(), PlaybackFailure> {
        if bytes.len() % 4 != 0 {
            return Err(PlaybackFailure::Permanent(
                "Incomplete SDR stereo PCM frame".into(),
            ));
        }
        for pair in bytes.chunks_exact(4) {
            if cancelled.load(Ordering::Acquire) {
                break;
            }
            let frame = [
                f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32768.0,
                f32::from(i16::from_le_bytes([pair[2], pair[3]])) / 32768.0,
            ];
            let mut start_error = None;
            let keep_going = self
                .converter
                .convert(&frame, |frame| {
                    if !output.push(frame, cancelled) {
                        return false;
                    }
                    // Start while processing a large packet, before a stopped
                    // output's bounded ring can fill and deadlock the producer.
                    if !self.started && output.buffered() >= output.prebuffer() {
                        if let Err(error) = output.start() {
                            start_error = Some(error);
                            return false;
                        }
                        self.started = true;
                        event(PlaybackEvent::Buffered);
                    }
                    if self.started && !self.announced && output.played() > 0 {
                        self.announced = true;
                        event(PlaybackEvent::PlaybackStarted);
                    }
                    true
                })
                .map_err(|e| {
                    PlaybackFailure::Permanent(format!("SDR PCM conversion failed: {e:?}"))
                })?;
            if let Some(error) = start_error {
                return Err(PlaybackFailure::Output(error));
            }
            if !keep_going && !cancelled.load(Ordering::Acquire) {
                return Err(PlaybackFailure::Output(
                    "SDR output stopped accepting PCM".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    struct FakeSink {
        samples: Vec<f32>,
        started: Cell<bool>,
        played: u64,
        fail_start: bool,
        reject: bool,
    }
    impl Sink for FakeSink {
        fn rate(&self) -> u32 {
            48_000
        }
        fn channels(&self) -> usize {
            2
        }
        fn buffered(&self) -> usize {
            self.samples.len()
        }
        fn prebuffer(&self) -> usize {
            4
        }
        fn played(&self) -> u64 {
            self.played
        }
        fn push(&mut self, frame: &[f32], _: &AtomicBool) -> bool {
            if self.reject {
                return false;
            }
            self.samples.extend_from_slice(frame);
            true
        }
        fn start(&self) -> Result<(), String> {
            self.started.set(true);
            if self.fail_start {
                Err("Output disconnected".into())
            } else {
                Ok(())
            }
        }
    }
    fn sink() -> FakeSink {
        FakeSink {
            samples: Vec::new(),
            started: Cell::new(false),
            played: 0,
            fail_start: false,
            reject: false,
        }
    }
    fn packet() -> Vec<u8> {
        [i16::MIN, i16::MAX, 16384, -16384]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect()
    }
    #[test]
    fn playing_requires_native_callback_evidence_and_pcm_preserves_endianness() {
        let mut output = sink();
        let mut pump = StereoPump::new(&output).unwrap();
        let mut events = Vec::new();
        pump.feed(&packet(), &mut output, &AtomicBool::new(false), |e| {
            events.push(e)
        })
        .unwrap();
        assert!(output.started.get());
        assert_eq!(events, [PlaybackEvent::Buffered]);
        assert_eq!(output.samples[0], -1.0);
        assert!(output.samples[1] > 0.99);
        assert_eq!(output.samples[2..], [0.5, -0.5]);
        output.played = 2;
        pump.feed(&packet(), &mut output, &AtomicBool::new(false), |e| {
            events.push(e)
        })
        .unwrap();
        assert_eq!(
            events,
            [PlaybackEvent::Buffered, PlaybackEvent::PlaybackStarted]
        );
    }
    #[test]
    fn failed_output_and_cancellation_never_claim_playing() {
        for (fail_start, reject) in [(true, false), (false, true)] {
            let mut output = sink();
            output.fail_start = fail_start;
            output.reject = reject;
            let mut pump = StereoPump::new(&output).unwrap();
            assert!(matches!(
                pump.feed(&packet(), &mut output, &AtomicBool::new(false), |_| panic!(
                    "failed sink emitted playback"
                )),
                Err(PlaybackFailure::Output(_))
            ));
        }
        let mut output = sink();
        let mut pump = StereoPump::new(&output).unwrap();
        pump.feed(&packet(), &mut output, &AtomicBool::new(true), |_| {
            panic!("cancelled sink emitted playback")
        })
        .unwrap();
        assert!(output.samples.is_empty());
    }
    #[test]
    fn malformed_packets_and_invalid_start_parameters_fail_before_io() {
        let mut output = sink();
        let mut pump = StereoPump::new(&output).unwrap();
        assert!(pump
            .feed(&[0; 3], &mut output, &AtomicBool::new(false), |_| {})
            .is_err());
        assert!(output.samples.is_empty());
        assert!(NativeRadioEngine::start_sdrconnect("8.8.8.8:5454", 10, "system").is_err());
        assert!(NativeRadioEngine::start_sdrconnect("127.0.0.1:9", 101, "system").is_err());
    }
    #[test]
    fn sdr_transport_errors_reuse_typed_recovery_without_retrying_output_loss() {
        assert_eq!(
            api_failure(Error::StartTimeout).class(),
            FailureClass::Permanent
        );
        assert_eq!(
            api_failure(Error::NotStreaming).class(),
            FailureClass::Permanent
        );
        assert_eq!(
            api_failure(Error::Timeout).class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            api_failure(Error::Protocol("Bad frame".into())).class(),
            FailureClass::Permanent
        );
    }
}
