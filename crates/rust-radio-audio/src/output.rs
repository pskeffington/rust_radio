//! CPAL native audio sink for Rust-Boy RADIO.
//! Network and codec operations must never run in the real-time callback.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use ringbuf::{traits::*, HeapCons, HeapProd, HeapRb};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc,
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
    failed: Arc<AtomicBool>,
}

impl NativeAudioOutput {
    pub fn open(volume: Arc<AtomicU8>, output_id: &str) -> Result<Self, String> {
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
        let played_samples = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match sample_format {
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
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &config,
                consumer,
                volume,
                Arc::clone(&played_samples),
                Arc::clone(&failed),
            )?,
            format => return Err(format!("Unsupported native sample format: {format:?}")),
        };
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
        self.failed.load(Ordering::Acquire)
    }

    pub fn start(&self) -> Result<(), String> {
        self.stream
            .play()
            .map_err(|e| format!("Cannot start native audio output: {e}"))
    }

    /// Writes a complete interleaved frame. If the PCM ring is full, the
    /// decoder worker yields; the audio callback never blocks.
    pub fn push_frame(&mut self, frame: &[f32], cancelled: &AtomicBool) -> bool {
        let mut last_progress = Instant::now();
        let mut last_consumed = self.played_samples();
        for &sample in frame {
            loop {
                if cancelled.load(Ordering::Acquire) || self.has_failed() {
                    return false;
                }
                if self.producer.try_push(sample).is_ok() {
                    break;
                }
                let consumed = self.played_samples();
                if consumed != last_consumed {
                    last_progress = Instant::now();
                    last_consumed = consumed;
                }
                // A disconnected audio device can stop calling CPAL without
                // sending an error. Do not block the decoder worker forever
                // on a full ring or pretend audio is still playing.
                if last_progress.elapsed() >= Duration::from_secs(5) {
                    self.failed.store(true, Ordering::Release);
                    return false;
                }
                thread::sleep(Duration::from_millis(3));
            }
        }
        true
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: HeapCons<f32>,
    volume: Arc<AtomicU8>,
    played: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let error_flag = Arc::clone(&failed);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let gain = volume.load(Ordering::Relaxed) as f32 / 100.0;
                let mut consumed = 0u64;
                for sample in data.iter_mut() {
                    if let Some(value) = consumer.try_pop() {
                        *sample = T::from_sample((value * gain).clamp(-1.0, 1.0));
                        consumed += 1;
                    } else {
                        *sample = T::from_sample(0.0);
                    }
                }
                played.fetch_add(consumed, Ordering::Relaxed);
            },
            move |_error| {
                error_flag.store(true, Ordering::Release);
            },
            None,
        )
        .map_err(|e| format!("Cannot configure native audio stream: {e}"))
}

#[cfg(test)]
mod output_identity_tests {
    use super::*;

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
