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
    use crate::output::render_samples;
    use ringbuf::{traits::*, HeapCons, HeapProd, HeapRb};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicU64, AtomicU8};
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

    struct PacedSink {
        producer: HeapProd<f32>,
        consumer: HeapCons<f32>,
        volume: AtomicU8,
        played_samples: AtomicU64,
        underrun_samples: AtomicU64,
        prebuffer_samples: usize,
        started: Cell<bool>,
    }

    impl Sink for PacedSink {
        fn push(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool {
            if cancelled.load(Ordering::Acquire) {
                return false;
            }
            frame
                .iter()
                .all(|sample| self.producer.try_push(*sample).is_ok())
        }

        fn buffered(&self) -> usize {
            self.producer.occupied_len()
        }

        fn prebuffer(&self) -> usize {
            self.prebuffer_samples
        }

        fn played(&self) -> u64 {
            self.played_samples.load(Ordering::Relaxed)
        }

        fn start(&self) -> Result<(), String> {
            // `Sink::start` takes `&self`, so mutate the test flag through Cell.
            self.started.set(true);
            Ok(())
        }
    }

    impl PacedSink {
        fn consume_during_stall(&mut self, samples: usize) {
            if !self.started.get() {
                return;
            }
            let mut callback_buffer = vec![0.0; samples];
            render_samples(
                &mut callback_buffer,
                &mut self.consumer,
                &self.volume,
                &self.played_samples,
                &self.underrun_samples,
            );
        }
    }

    #[test]
    fn prebuffer_ab_measures_fill_and_underruns_under_injected_packet_stalls() {
        fn run(prebuffer_ms: usize, stall_ms: usize) -> (Duration, u64, u64, usize) {
            // Test scale: one interleaved sample represents one ms of queued
            // audio. Feed and callback use real production ring-buffer and
            // render_samples paths; packet gaps are injected on this worker.
            let mut converter = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
            let ring = HeapRb::<f32>::new(2_000);
            let (producer, consumer) = ring.split();
            let mut output = PacedSink {
                producer,
                consumer,
                volume: AtomicU8::new(0),
                played_samples: AtomicU64::new(0),
                underrun_samples: AtomicU64::new(0),
                prebuffer_samples: prebuffer_ms,
                started: Cell::new(false),
            };
            let mut state = SinkState::default();
            let first_decoded_audio_at = Instant::now();
            let observed = Mutex::new(None);
            let cancelled = AtomicBool::new(false);
            let mut packet_count_at_start = None;
            let mut prebuffer_gap_underruns = Vec::new();

            for packet_index in 0..5 {
                let gap_ms = match packet_index {
                    2 => stall_ms,
                    3 | 4 => 100,
                    _ => 0,
                };
                if gap_ms > 0 {
                    thread::sleep(Duration::from_millis(gap_ms as u64));
                    let underruns_before = output.underrun_samples.load(Ordering::Relaxed);
                    output.consume_during_stall(gap_ms);
                    prebuffer_gap_underruns
                        .push(output.underrun_samples.load(Ordering::Relaxed) - underruns_before);
                }

                state
                    .feed(
                        &mut converter,
                        &[0.25; 100],
                        &mut output,
                        &cancelled,
                        |event| {
                            if event == PlaybackEvent::Buffered {
                                super::super::record_prebuffer_fill_latency(
                                    &observed,
                                    first_decoded_audio_at,
                                );
                                packet_count_at_start = Some(packet_index + 1);
                            }
                        },
                    )
                    .unwrap();
            }

            let fill_latency = observed
                .lock()
                .unwrap()
                .expect("output never reached the prebuffer threshold");
            let post_start_underruns_before = output.underrun_samples.load(Ordering::Relaxed);
            output.consume_during_stall(800);
            (
                fill_latency,
                prebuffer_gap_underruns.into_iter().sum(),
                output.underrun_samples.load(Ordering::Relaxed) - post_start_underruns_before,
                packet_count_at_start.expect("prebuffer event did not record its packet"),
            )
        }

        let stalls_ms = [100, 300, 800];
        let mut low_latency_fill_ms = Vec::new();
        let mut resilient_fill_ms = Vec::new();
        let mut low_latency_underruns = Vec::new();
        let mut resilient_underruns = Vec::new();
        let mut low_latency_post_start_underruns = Vec::new();
        let mut resilient_post_start_underruns = Vec::new();
        for stall_ms in stalls_ms {
            let (
                low_latency_fill,
                low_latency_missing,
                low_latency_post_missing,
                low_latency_packet,
            ) = run(200, stall_ms);
            let (resilient_fill, resilient_missing, resilient_post_missing, resilient_packet) =
                run(500, stall_ms);
            assert!(
                resilient_fill >= Duration::from_millis(stall_ms as u64 + 150),
                "500ms threshold should include the injected stall and later packets: {resilient_fill:?} under {stall_ms}ms gap"
            );
            assert_eq!(low_latency_packet, 2);
            assert_eq!(resilient_packet, 5);
            low_latency_fill_ms.push(low_latency_fill.as_secs_f64() * 1000.0);
            resilient_fill_ms.push(resilient_fill.as_secs_f64() * 1000.0);
            low_latency_underruns.push(low_latency_missing);
            resilient_underruns.push(resilient_missing);
            low_latency_post_start_underruns.push(low_latency_post_missing);
            resilient_post_start_underruns.push(resilient_post_missing);
        }

        assert_eq!(low_latency_underruns, [0, 100, 600]);
        assert_eq!(resilient_underruns, [0, 0, 0]);
        assert_eq!(low_latency_post_start_underruns, [600, 700, 700]);
        assert_eq!(resilient_post_start_underruns, [300, 300, 300]);
        println!(
            "Paced IP prebuffer A/B (prebuffer_stall_ms={stalls_ms:?}, post_start_stall_ms=800): A 200ms fill={low_latency_fill_ms:?}, prebuffer_gap_missing={low_latency_underruns:?}, post_start_missing={low_latency_post_start_underruns:?}; B 500ms fill={resilient_fill_ms:?}, prebuffer_gap_missing={resilient_underruns:?}, post_start_missing={resilient_post_start_underruns:?}"
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
