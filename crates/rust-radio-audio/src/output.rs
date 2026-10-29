//! CPAL native audio sink for Rust-Boy RADIO.
//! Network and codec operations must never run in the real-time callback.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use ringbuf::{traits::*, HeapCons, HeapProd, HeapRb};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

/// CPAL exposes OS-managed audio endpoints. Connected Bluetooth speakers
/// and headsets appear here after pairing in macOS/Windows/Linux settings.
#[derive(Clone, Debug)]
pub struct OutputDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub available: bool,
}

/// Bind index-based CPAL enumeration to the observed device name. CPAL 0.16
/// does not expose a cross-platform persistent hardware identifier, so a
/// reordered list fails closed rather than silently redirecting audio.
fn device_key(index: usize, name: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("output-{index}-{hash:016x}")
}

pub fn available_outputs() -> Result<Vec<OutputDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_device = host.default_output_device();
    let default_available = default_device.is_some();
    let default_name = default_device.and_then(|device| device.name().ok());
    let devices = host
        .output_devices()
        .map_err(|error| format!("Cannot list audio outputs: {error}"))?;
    let mut result = vec![OutputDeviceInfo {
        id: "system".into(),
        name: "System default".into(),
        is_default: true,
        available: default_available,
    }];
    for (index, device) in devices.enumerate() {
        let name = device
            .name()
            .unwrap_or_else(|_| "Unnamed audio output".into());
        let id = device_key(index, &name);
        let label = if default_name.as_deref() == Some(name.as_str()) {
            format!("{name} (default)")
        } else {
            name
        };
        result.push(OutputDeviceInfo {
            id,
            name: label,
            is_default: false,
            available: true,
        });
    }
    Ok(result)
}

fn resolve_output(host: &cpal::Host, output_id: &str) -> Result<cpal::Device, String> {
    if output_id == "system" {
        return host
            .default_output_device()
            .ok_or_else(|| "No default audio output".into());
    }
    let index = output_id
        .strip_prefix("output-")
        .and_then(|value| value.split_once('-'))
        .and_then(|(index, _)| index.parse::<usize>().ok())
        .ok_or("Invalid audio output identifier")?;
    let device = host
        .output_devices()
        .map_err(|error| format!("Cannot enumerate audio outputs: {error}"))?
        .nth(index)
        .ok_or_else(|| "Audio output disconnected; refresh the device list".to_owned())?;
    let current_name = device
        .name()
        .map_err(|error| format!("Cannot identify selected audio output: {error}"))?;
    if device_key(index, &current_name) != output_id {
        return Err("Audio devices changed; select the output again".into());
    }
    Ok(device)
}

pub struct NativeAudioOutput {
    producer: HeapProd<f32>,
    stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: usize,
    played_samples: Arc<AtomicU64>,
    failed: Arc<OutputFailure>,
}

#[derive(Default)]
struct OutputFailure {
    failed: AtomicBool,
    detail: Mutex<Option<String>>,
}

impl OutputFailure {
    fn record(&self, detail: String) {
        let mut recorded = self
            .detail
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if recorded.is_none() {
            *recorded = Some(detail);
        }
        self.failed.store(true, Ordering::Release);
    }

    fn detail(&self) -> Option<String> {
        if !self.failed.load(Ordering::Acquire) {
            return None;
        }
        self.detail
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl NativeAudioOutput {
    pub fn open(volume: Arc<AtomicU8>, output_id: &str) -> Result<Self, String> {
        Self::open_with_progress(volume, output_id, Arc::new(AtomicU64::new(0)))
    }

    pub(crate) fn open_with_progress(
        volume: Arc<AtomicU8>,
        output_id: &str,
        played_samples: Arc<AtomicU64>,
    ) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = resolve_output(&host, output_id)?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("Audio output unavailable: {e}"))?;
        let sample_format = supported.sample_format();
        let config = supported.config();
        let sample_rate = config.sample_rate.0;
        let channels = usize::from(config.channels);
        if !(8_000..=192_000).contains(&sample_rate) || !(1..=8).contains(&channels) {
            return Err("Unsupported audio output rate or channel configuration".into());
        }
        // Two seconds of interleaved device-rate PCM; bounded to at most ~12 MiB.
        let ring = HeapRb::<f32>::new(sample_rate as usize * channels * 2);
        let (producer, consumer) = ring.split();
        let failed = Arc::new(OutputFailure::default());
        let stream = match sample_format {
            SampleFormat::I8 => build_stream::<i8>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::I24 => build_stream::<cpal::I24>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::I32 => build_stream::<i32>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::I64 => build_stream::<i64>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::U8 => build_stream::<u8>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::U32 => build_stream::<u32>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::U64 => build_stream::<u64>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            SampleFormat::F64 => build_stream::<f64>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            format => return Err(format!("Unsupported native sample format: {format:?}")),
        };
        // CPAL's CoreAudio backend starts the AudioUnit during construction.
        // Pause before exposing the producer so callbacks cannot drain a live
        // low-latency source faster than it can reach the prebuffer threshold.
        #[cfg(target_os = "macos")]
        stream
            .pause()
            .map_err(|e| format!("Cannot prepare paused native audio output: {e}"))?;
        Ok(Self {
            producer,
            stream,
            sample_rate,
            channels,
            played_samples,
            failed,
        })
    }

    pub fn minimum_prebuffer_samples(&self) -> usize {
        self.sample_rate as usize * self.channels / 5
    }

    pub fn buffered_samples(&self) -> usize {
        self.producer.occupied_len()
    }

    pub fn played_samples(&self) -> u64 {
        self.played_samples.load(Ordering::Relaxed)
    }

    pub fn has_failed(&self) -> bool {
        self.failed.failed.load(Ordering::Acquire)
    }

    /// Preserve CPAL's asynchronous device diagnostic when it is available.
    pub fn failure_detail(&self) -> Option<String> {
        self.failed.detail()
    }

    pub fn start(&self) -> Result<(), String> {
        self.stream
            .play()
            .map_err(|e| format!("Cannot start native audio output: {e}"))
    }

    /// Writes a complete interleaved frame. If the PCM ring is full, the
    /// decoder worker yields; the audio callback never blocks.
    pub fn push_frame(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool {
        push_frame_with_callback_watchdog(
            &mut self.producer,
            frame,
            cancelled,
            &self.failed,
            &self.played_samples,
            Duration::from_secs(5),
        )
    }
}

fn push_frame_with_callback_watchdog(
    producer: &mut HeapProd<f32>,
    frame: &[f32],
    cancelled: &AtomicBool,
    failed: &OutputFailure,
    played_samples: &AtomicU64,
    stalled_after: Duration,
) -> bool {
    let mut last_progress = Instant::now();
    let mut last_consumed = played_samples.load(Ordering::Relaxed);
    for &sample in frame {
        loop {
            if cancelled.load(Ordering::Acquire) || failed.failed.load(Ordering::Acquire) {
                return false;
            }
            if producer.try_push(sample).is_ok() {
                break;
            }
            let consumed = played_samples.load(Ordering::Relaxed);
            if consumed != last_consumed {
                last_progress = Instant::now();
                last_consumed = consumed;
            }
            // A disconnected audio device can stop calling CPAL without
            // sending an error. Do not block the decoder worker forever
            // on a full ring or pretend audio is still playing.
            if last_progress.elapsed() >= stalled_after {
                failed.record("Native audio output stopped consuming samples".into());
                return false;
            }
            thread::sleep(Duration::from_millis(3));
        }
    }
    true
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: HeapCons<f32>,
    volume: Arc<AtomicU8>,
    played: Arc<AtomicU64>,
    failed: Arc<OutputFailure>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let error_state = Arc::clone(&failed);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                render_samples(data, &mut consumer, &volume, &played);
            },
            move |error| {
                error_state.record(format!("Native audio output stream error: {error:?}"));
            },
            None,
        )
        .map_err(|e| format!("Cannot configure native audio stream: {e}"))
}

/// Transfer queued PCM without blocking or allocating in CPAL's callback.
fn render_samples<T>(
    data: &mut [T],
    consumer: &mut HeapCons<f32>,
    volume: &AtomicU8,
    played: &AtomicU64,
) where
    T: SizedSample + FromSample<f32>,
{
    let gain = volume.load(Ordering::Relaxed) as f32 / 100.0;
    let mut consumed = 0u64;
    for sample in data {
        if let Some(value) = consumer.try_pop() {
            *sample = T::from_sample((value * gain).clamp(-1.0, 1.0));
            consumed += 1;
        } else {
            *sample = T::from_sample(0.0);
        }
    }
    played.fetch_add(consumed, Ordering::Relaxed);
}

#[cfg(test)]
mod output_identity_tests {
    use super::*;
    use cpal::Sample;

    #[test]
    fn asynchronous_device_error_retains_backend_diagnostic() {
        let failure = OutputFailure::default();
        assert!(!failure.failed.load(Ordering::Acquire));
        assert_eq!(failure.detail(), None);

        failure.record("device disappeared during stream playback".into());
        failure.record("less specific follow-up failure".into());

        assert!(failure.failed.load(Ordering::Acquire));
        assert_eq!(
            failure.detail().as_deref(),
            Some("device disappeared during stream playback")
        );
    }

    fn assert_rendered_samples_convert<T>(volume_percent: u8)
    where
        T: SizedSample + FromSample<f32>,
        f32: FromSample<T>,
    {
        let expected = [-1.0f32, -0.5, 0.0, 0.75, 1.0];
        let ring = HeapRb::<f32>::new(expected.len());
        let (mut producer, mut consumer) = ring.split();
        for sample in expected {
            producer.try_push(sample).unwrap();
        }
        let volume = AtomicU8::new(volume_percent);
        let played = AtomicU64::new(0);
        let mut output = vec![T::from_sample(0.0); expected.len()];

        render_samples(&mut output, &mut consumer, &volume, &played);

        let converted: Vec<f32> = output.into_iter().map(f32::from_sample).collect();
        let gain = f32::from(volume_percent) / 100.0;
        for (actual, input) in converted.into_iter().zip(expected) {
            let expected = input * gain;
            assert!(
                (actual - expected).abs() <= 0.02,
                "sample conversion error: {actual} vs {expected}"
            );
        }
        assert_eq!(played.load(Ordering::Relaxed), expected.len() as u64);
    }

    #[test]
    fn callback_converts_every_cpal_format_at_full_half_and_mute_gain() {
        macro_rules! check_format {
            ($sample_type:ty) => {
                assert_rendered_samples_convert::<$sample_type>(100);
                assert_rendered_samples_convert::<$sample_type>(50);
                assert_rendered_samples_convert::<$sample_type>(0);
            };
        }
        check_format!(i8);
        check_format!(i16);
        check_format!(cpal::I24);
        check_format!(i32);
        check_format!(i64);
        check_format!(u8);
        check_format!(u16);
        check_format!(u32);
        check_format!(u64);
        check_format!(f32);
        check_format!(f64);
    }

    fn assert_live_volume_changes<T>()
    where
        T: SizedSample + FromSample<f32>,
        f32: FromSample<T>,
    {
        let ring = HeapRb::<f32>::new(3);
        let (mut producer, mut consumer) = ring.split();
        for sample in [0.8f32, -0.8, 0.4] {
            producer.try_push(sample).unwrap();
        }
        let volume = AtomicU8::new(100);
        let played = AtomicU64::new(0);

        for (volume_percent, expected) in [(100, 0.8f32), (50, -0.4), (0, 0.0)] {
            volume.store(volume_percent, Ordering::Relaxed);
            let mut output = [T::from_sample(0.0)];
            render_samples(&mut output, &mut consumer, &volume, &played);
            let actual = f32::from_sample(output[0]);
            assert!(
                (actual - expected).abs() <= 0.02,
                "live volume conversion error: {actual} vs {expected} at {volume_percent}%"
            );
        }
        assert_eq!(played.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn callback_applies_volume_changes_between_buffers_for_every_format() {
        macro_rules! check_format {
            ($sample_type:ty) => {
                assert_live_volume_changes::<$sample_type>();
            };
        }
        check_format!(i8);
        check_format!(i16);
        check_format!(cpal::I24);
        check_format!(i32);
        check_format!(i64);
        check_format!(u8);
        check_format!(u16);
        check_format!(u32);
        check_format!(u64);
        check_format!(f32);
        check_format!(f64);
    }

    #[test]
    fn callback_applies_gain_and_silences_underrun_without_counting_silence() {
        let ring = HeapRb::<f32>::new(4);
        let (mut producer, mut consumer) = ring.split();
        producer.try_push(1.5).unwrap();
        producer.try_push(-0.5).unwrap();
        let volume = AtomicU8::new(50);
        let played = AtomicU64::new(0);
        let mut output = [1.0f32; 4];

        render_samples(&mut output, &mut consumer, &volume, &played);

        assert_eq!(output, [0.75, -0.25, 0.0, 0.0]);
        assert_eq!(played.load(Ordering::Relaxed), 2);
        assert_eq!(consumer.occupied_len(), 0);
    }

    #[test]
    fn stopped_callback_fails_bounded_when_pcm_ring_is_full() {
        let ring = HeapRb::<f32>::new(1);
        let (mut producer, _consumer) = ring.split();
        producer.try_push(0.25).unwrap();
        let cancelled = AtomicBool::new(false);
        let failed = OutputFailure::default();
        let played = AtomicU64::new(0);
        let started = Instant::now();

        let accepted = push_frame_with_callback_watchdog(
            &mut producer,
            &[0.5],
            &cancelled,
            &failed,
            &played,
            Duration::from_millis(12),
        );

        assert!(!accepted);
        assert!(failed.failed.load(Ordering::Acquire));
        assert_eq!(
            failed.detail().as_deref(),
            Some("Native audio output stopped consuming samples")
        );
        assert!(started.elapsed() < Duration::from_millis(250));
    }

    #[test]
    fn cancellation_does_not_reclassify_a_full_ring_as_device_failure() {
        let ring = HeapRb::<f32>::new(1);
        let (mut producer, _consumer) = ring.split();
        producer.try_push(0.25).unwrap();
        let cancelled = AtomicBool::new(true);
        let failed = OutputFailure::default();
        let played = AtomicU64::new(0);

        let accepted = push_frame_with_callback_watchdog(
            &mut producer,
            &[0.5],
            &cancelled,
            &failed,
            &played,
            Duration::from_millis(12),
        );

        assert!(!accepted);
        assert!(!failed.failed.load(Ordering::Acquire));
    }

    #[test]
    fn name_bound_ids_reject_reordered_or_replaced_outputs() {
        let speaker = device_key(0, "Built-in Output");
        let bluetooth = device_key(1, "Bluetooth Headphones");
        assert_ne!(speaker, bluetooth);
        assert_ne!(bluetooth, device_key(0, "Bluetooth Headphones"));
        assert_ne!(bluetooth, device_key(1, "USB Audio"));
        assert_eq!(bluetooth, device_key(1, "Bluetooth Headphones"));
    }

    #[test]
    fn output_identity_uses_full_utf8_name_and_is_deterministic() {
        assert_ne!(device_key(0, "Audio"), device_key(0, "audio"));
        assert_ne!(device_key(0, "Casque"), device_key(0, "Casqué"));
        assert!(device_key(0, "Speaker").starts_with("output-0-"));
    }
}
