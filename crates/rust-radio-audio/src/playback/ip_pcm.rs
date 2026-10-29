//! Deliver decoded IP audio while the bounded native output prebuffers.
use super::{NativeAudioOutput, PcmConverter, PlaybackEvent, PlaybackFailure};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) trait Sink {
    fn push(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool;
    fn buffered(&self) -> usize;
    fn prebuffer(&self) -> usize;
    fn played(&self) -> u64;
    fn start(&self) -> Result<(), String>;
}

impl Sink for NativeAudioOutput {
    fn push(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool {
        self.push_frame(frame, cancelled)
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
    fn start(&self) -> Result<(), String> {
        self.start()
    }
}

#[derive(Default)]
pub(super) struct SinkState {
    started: bool,
    announced: bool,
}

impl SinkState {
    pub(super) fn feed(
        &mut self,
        converter: &mut PcmConverter,
        samples: &[f32],
        output: &mut impl Sink,
        cancelled: &AtomicBool,
        mut event: impl FnMut(PlaybackEvent),
    ) -> Result<bool, PlaybackFailure> {
        let mut start_error = None;
        let keep_going = converter
            .convert(samples, |frame| {
                if !output.push(frame, cancelled) || cancelled.load(Ordering::Acquire) {
                    return false;
                }
                // A decoded packet may exceed the paused ring's capacity.
                // Start as soon as it is prebuffered, before the producer
                // waits for a callback that cannot run until the next packet.
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
                PlaybackFailure::Permanent(format!("Radio PCM conversion failed: {e:?}"))
            })?;
        if let Some(error) = start_error {
            return Err(PlaybackFailure::Output(error));
        }
        Ok(keep_going)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::Mutex;
    use std::thread;
    use std::time::{Duration, Instant};

    struct BoundedSink {
        buffered: usize,
        pushed: usize,
        played: u64,
        started: Cell<bool>,
        prebuffer_samples: usize,
        fail_start: bool,
        overflowed: bool,
    }

    impl Sink for BoundedSink {
        fn push(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool {
            if cancelled.load(Ordering::Acquire) {
                return false;
            }
            if !self.started.get() && self.buffered + frame.len() > 8 {
                self.overflowed = true;
                return false;
            }
            self.pushed += frame.len();
            if self.started.get() {
                self.played += frame.len() as u64;
            } else {
                self.buffered += frame.len();
            }
            true
        }
        fn buffered(&self) -> usize {
            self.buffered
        }
        fn prebuffer(&self) -> usize {
            self.prebuffer_samples
        }
        fn played(&self) -> u64 {
            self.played
        }
        fn start(&self) -> Result<(), String> {
            if self.fail_start {
                return Err("Output unavailable".into());
            }
            self.started.set(true);
            Ok(())
        }
    }

    fn sink(fail_start: bool) -> BoundedSink {
        BoundedSink {
            buffered: 0,
            pushed: 0,
            played: 0,
            started: Cell::new(false),
            prebuffer_samples: 4,
            fail_start,
            overflowed: false,
        }
    }

    #[test]
    fn large_decoded_packet_starts_output_before_filling_paused_ring() {
        let mut converter = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
        let mut output = sink(false);
        let mut state = SinkState::default();
        let mut events = Vec::new();
        let samples = vec![0.25; 40];
        assert!(state
            .feed(
                &mut converter,
                &samples,
                &mut output,
                &AtomicBool::new(false),
                |event| events.push(event)
            )
            .unwrap());
        assert!(!output.overflowed);
        assert!(output.started.get());
        assert_eq!(output.pushed, samples.len());
        assert_eq!(
            events,
            [PlaybackEvent::Buffered, PlaybackEvent::PlaybackStarted]
        );
    }

    #[test]
    fn measured_prebuffer_fill_tracks_paced_decoded_packets() {
        fn fill_latency(prebuffer_samples: usize) -> Duration {
            let mut converter = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
            let mut output = sink(false);
            output.prebuffer_samples = prebuffer_samples;
            let mut state = SinkState::default();
            let observed = Mutex::new(None);
            let first_decoded_audio_at = Instant::now();
            let cancelled = AtomicBool::new(false);

            for packet_index in 0..4 {
                if packet_index > 0 {
                    thread::sleep(Duration::from_millis(10));
                }
                let keep_going = state
                    .feed(
                        &mut converter,
                        &[0.25, -0.25],
                        &mut output,
                        &cancelled,
                        |event| {
                            if event == PlaybackEvent::Buffered {
                                super::super::record_prebuffer_fill_latency(
                                    &observed,
                                    first_decoded_audio_at,
                                );
                            }
                        },
                    )
                    .unwrap();
                assert!(keep_going);
                if output.started.get() {
                    break;
                }
            }

            assert!(
                output.started.get(),
                "prebuffer threshold was never reached"
            );
            let latency = observed.lock().unwrap().unwrap();
            latency
        }

        let low_latency_fill = fill_latency(4);
        let resilient_fill = fill_latency(8);
        assert!(
            resilient_fill > low_latency_fill,
            "larger prebuffer should cross its threshold after more paced packets: {low_latency_fill:?} vs {resilient_fill:?}"
        );
    }

    #[test]
    fn start_failure_and_cancellation_never_announce_playing() {
        let mut converter = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
        let mut output = sink(true);
        let mut state = SinkState::default();
        let mut events = Vec::new();
        let result = state.feed(
            &mut converter,
            &[0.5; 20],
            &mut output,
            &AtomicBool::new(false),
            |event| events.push(event),
        );
        assert!(matches!(result, Err(PlaybackFailure::Output(_))));
        assert!(!output.started.get());
        assert!(events.is_empty());

        let mut converter = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
        let mut output = sink(false);
        let mut state = SinkState::default();
        assert!(!state
            .feed(
                &mut converter,
                &[0.5; 20],
                &mut output,
                &AtomicBool::new(true),
                |_| panic!("cancelled playback emitted an event")
            )
            .unwrap());
        assert!(!output.started.get());
    }
}
