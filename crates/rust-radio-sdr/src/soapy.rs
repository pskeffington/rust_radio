use num_complex::Complex;
use rust_radio_core::sdr::{
    Demodulation, FrequencyRange, RadioCapabilities, RadioDevice, RadioError, ReceiverKind, Tuning,
};
use soapysdr::{Args, Device, Direction::Rx, ErrorCode, Format, Range, RxStream};
use std::collections::BTreeSet;

const MAX_RECEIVERS: usize = 32;
const MAX_COMPLEX_SAMPLES: usize = super::MAX_IQ_VALUES / 2;
const READ_TIMEOUT_US: i64 = 100_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiverInfo {
    pub id: String,
    pub label: String,
    pub driver: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRateRange {
    pub min_hz: u32,
    pub max_hz: u32,
    /// Zero denotes a continuous range.
    pub step_hz: u32,
}

impl SampleRateRange {
    pub fn contains(&self, rate: u32) -> bool {
        rate >= self.min_hz
            && rate <= self.max_hz
            && (self.step_hz == 0 || (rate - self.min_hz) % self.step_hz == 0)
    }
}

fn device_error(error: impl std::fmt::Display) -> RadioError {
    RadioError::Device(error.to_string())
}

fn receiver_id(args: &Args) -> String {
    let mut fields: Vec<_> = args.iter().collect();
    fields.sort_unstable();
    let mut hash = 0xcbf29ce484222325u64;
    for (key, value) in fields {
        for bytes in [
            (key.len() as u64).to_le_bytes().as_slice(),
            key.as_bytes(),
            (value.len() as u64).to_le_bytes().as_slice(),
            value.as_bytes(),
        ] {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("soapy-{hash:016x}")
}

fn enumerated() -> Result<Vec<(ReceiverInfo, Args)>, RadioError> {
    let devices = soapysdr::enumerate(()).map_err(device_error)?;
    if devices.len() > MAX_RECEIVERS {
        return Err(device_error("SDR enumeration exceeded 32 devices"));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for args in devices {
        let driver = args
            .get("driver")
            .ok_or(RadioError::InvalidCapabilities)?
            .to_owned();
        // A null test driver is not a physical radio receiver.
        if driver == "null" {
            continue;
        }
        let id = receiver_id(&args);
        if !ids.insert(id.clone()) {
            return Err(device_error(
                "Ambiguous SDR identifiers; refresh attached devices",
            ));
        }
        let label = args.get("label").unwrap_or(&driver).to_owned();
        result.push((ReceiverInfo { id, label, driver }, args));
    }
    Ok(result)
}

/// Explicit SDK enumeration only. No receive stream is opened or activated.
pub fn discover() -> Result<Vec<ReceiverInfo>, RadioError> {
    Ok(enumerated()?.into_iter().map(|(info, _)| info).collect())
}

fn frequency_range(range: Range) -> Result<FrequencyRange, RadioError> {
    const MAX_EXACT_HZ: f64 = 9_007_199_254_740_991.0;
    if !range.minimum.is_finite()
        || !range.maximum.is_finite()
        || range.minimum < 0.0
        || range.maximum > MAX_EXACT_HZ
        || range.minimum.ceil() > range.maximum.floor()
    {
        return Err(RadioError::InvalidCapabilities);
    }
    Ok(FrequencyRange {
        min_hz: range.minimum.ceil() as u64,
        max_hz: range.maximum.floor() as u64,
    })
}

fn sample_rate_range(range: Range) -> Result<SampleRateRange, RadioError> {
    if !range.minimum.is_finite()
        || !range.maximum.is_finite()
        || !range.step.is_finite()
        || range.minimum <= 0.0
        || range.maximum > u32::MAX as f64
        || range.step < 0.0
        || range.step > u32::MAX as f64
        || range.step.fract() != 0.0
        || (range.step > 0.0 && range.minimum.fract() != 0.0)
        || range.minimum.ceil() > range.maximum.floor()
    {
        return Err(RadioError::InvalidCapabilities);
    }
    Ok(SampleRateRange {
        min_hz: range.minimum.ceil() as u32,
        max_hz: range.maximum.floor() as u32,
        step_hz: range.step as u32,
    })
}

/// Owned receive-only handle. Device selection must use a fresh discovery ID;
/// arbitrary driver arguments and transmit interfaces are not exposed.
pub struct SoapyReceiver {
    device: Device,
    channel: usize,
    capabilities: RadioCapabilities,
    sample_rate_ranges: Vec<SampleRateRange>,
    stream: Option<RxStream<Complex<i16>>>,
    samples: Vec<Complex<i16>>,
}

impl SoapyReceiver {
    /// Open/query the selected device without activating a stream. The requested
    /// rate is checked against real driver ranges and becomes this handle's rate.
    pub fn open(id: &str, channel: usize, sample_rate_hz: u32) -> Result<Self, RadioError> {
        if !id.starts_with("soapy-")
            || id.len() != 22
            || !id[6..].bytes().all(|byte| byte.is_ascii_hexdigit())
            || sample_rate_hz == 0
        {
            return Err(device_error("Invalid SDR identifier or sample rate"));
        }
        let (info, args) = enumerated()?
            .into_iter()
            .find(|(info, _)| info.id == id)
            .ok_or_else(|| device_error("Selected SDR is unavailable; discover devices again"))?;
        let device = Device::new(args).map_err(device_error)?;
        if channel >= device.num_channels(Rx).map_err(device_error)? {
            return Err(device_error("SDR receive channel unavailable"));
        }
        if !device
            .stream_formats(Rx, channel)
            .map_err(device_error)?
            .contains(&Format::CS16)
        {
            return Err(device_error(
                "SDR driver does not support signed 16-bit I/Q",
            ));
        }
        let frequency_ranges = device
            .frequency_range(Rx, channel)
            .map_err(device_error)?
            .into_iter()
            .map(frequency_range)
            .collect::<Result<Vec<_>, _>>()?;
        let sample_rate_ranges = device
            .get_sample_rate_range(Rx, channel)
            .map_err(device_error)?
            .into_iter()
            .map(sample_rate_range)
            .collect::<Result<Vec<_>, _>>()?;
        if frequency_ranges.is_empty() || sample_rate_ranges.is_empty() {
            return Err(RadioError::InvalidCapabilities);
        }
        if !sample_rate_ranges
            .iter()
            .any(|range| range.contains(sample_rate_hz))
        {
            return Err(RadioError::UnsupportedSampleRate(sample_rate_hz));
        }
        let gain_range_db_tenths = match device.gain_range(Rx, channel) {
            Ok(range)
                if range.minimum.is_finite()
                    && range.maximum.is_finite()
                    && range.minimum * 10.0 >= i16::MIN as f64
                    && range.maximum * 10.0 <= i16::MAX as f64
                    && (range.minimum * 10.0).ceil() <= (range.maximum * 10.0).floor() =>
            {
                Some((
                    (range.minimum * 10.0).ceil() as i16,
                    (range.maximum * 10.0).floor() as i16,
                ))
            }
            _ => None,
        };
        Ok(Self {
            capabilities: RadioCapabilities {
                name: info.label,
                frequency_ranges,
                sample_rates_hz: vec![sample_rate_hz],
                gain_range_db_tenths,
            },
            device,
            channel,
            sample_rate_ranges,
            stream: None,
            samples: vec![Complex::new(0, 0); MAX_COMPLEX_SAMPLES],
        })
    }

    pub fn sample_rate_ranges(&self) -> &[SampleRateRange] {
        &self.sample_rate_ranges
    }
}

impl RadioDevice for SoapyReceiver {
    fn capabilities(&self) -> &RadioCapabilities {
        &self.capabilities
    }

    fn receiver_kind(&self) -> ReceiverKind {
        ReceiverKind::Physical
    }

    fn apply_tuning(&mut self, tuning: Tuning) -> Result<(), RadioError> {
        if tuning.demodulation != Demodulation::RawIq {
            return Err(device_error(
                "RF audio demodulation is not implemented; select raw I/Q",
            ));
        }
        if !self
            .capabilities
            .frequency_ranges
            .iter()
            .any(|range| range.contains(tuning.frequency_hz))
        {
            return Err(RadioError::UnsupportedFrequency(tuning.frequency_hz));
        }
        if !self
            .capabilities
            .sample_rates_hz
            .contains(&tuning.sample_rate_hz)
        {
            return Err(RadioError::UnsupportedSampleRate(tuning.sample_rate_hz));
        }
        if let Some(gain) = tuning.gain_db_tenths {
            if !self
                .capabilities
                .gain_range_db_tenths
                .is_some_and(|(low, high)| (low..=high).contains(&gain))
            {
                return Err(RadioError::UnsupportedGain(gain));
            }
        }
        self.stop()?;
        self.device
            .set_sample_rate(Rx, self.channel, f64::from(tuning.sample_rate_hz))
            .map_err(device_error)?;
        self.device
            .set_frequency(Rx, self.channel, tuning.frequency_hz as f64, ())
            .map_err(device_error)?;
        if self
            .device
            .has_gain_mode(Rx, self.channel)
            .map_err(device_error)?
        {
            self.device
                .set_gain_mode(Rx, self.channel, tuning.gain_db_tenths.is_none())
                .map_err(device_error)?;
        }
        if let Some(gain) = tuning.gain_db_tenths {
            self.device
                .set_gain(Rx, self.channel, f64::from(gain) / 10.0)
                .map_err(device_error)?;
        }
        let actual_rate = self
            .device
            .sample_rate(Rx, self.channel)
            .map_err(device_error)?;
        if !actual_rate.is_finite() || (actual_rate - f64::from(tuning.sample_rate_hz)).abs() > 1.0
        {
            return Err(device_error("SDR driver selected a different sample rate"));
        }
        let mut stream = self
            .device
            .rx_stream(&[self.channel])
            .map_err(device_error)?;
        stream.activate(None).map_err(device_error)?;
        self.stream = Some(stream);
        Ok(())
    }

    fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError> {
        if buffer.len() % 2 != 0 {
            return Err(device_error("I/Q buffer length must be even"));
        }
        let stream = self.stream.as_mut().ok_or(RadioError::NotConfigured)?;
        let requested = (buffer.len() / 2).min(self.samples.len());
        if requested == 0 {
            return Ok(0);
        }
        let received = match stream.read(&mut [&mut self.samples[..requested]], READ_TIMEOUT_US) {
            Ok(count) if count <= requested => count,
            Err(error) if error.code == ErrorCode::Timeout => return Ok(0),
            Err(error) => {
                self.stream = None;
                return Err(device_error(error));
            }
            _ => {
                self.stream = None;
                return Err(device_error("SDR driver returned an invalid sample count"));
            }
        };
        for (values, sample) in buffer.chunks_exact_mut(2).zip(&self.samples[..received]) {
            values.copy_from_slice(&[sample.re, sample.im]);
        }
        Ok(received * 2)
    }

    fn stop(&mut self) -> Result<(), RadioError> {
        if let Some(mut stream) = self.stream.take() {
            if stream.active() {
                stream.deactivate(None).map_err(device_error)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_identifiers_are_order_independent_and_device_bound() {
        let first: Args = "driver=rtlsdr,serial=abc,label=Receiver".into();
        let reordered: Args = "label=Receiver,serial=abc,driver=rtlsdr".into();
        let other: Args = "driver=rtlsdr,serial=def,label=Receiver".into();
        assert_eq!(receiver_id(&first), receiver_id(&reordered));
        assert_ne!(receiver_id(&first), receiver_id(&other));
        assert_eq!(receiver_id(&first).len(), 22);
    }

    #[test]
    fn sample_rate_constraints_follow_the_reported_range_and_step() {
        let stepped = sample_rate_range(Range {
            minimum: 240_000.0,
            maximum: 2_048_000.0,
            step: 256_000.0,
        })
        .unwrap();
        assert!(stepped.contains(240_000));
        assert!(stepped.contains(496_000));
        assert!(!stepped.contains(1_024_000));
        assert!(!stepped.contains(1));
        let continuous = sample_rate_range(Range {
            minimum: 240_000.0,
            maximum: 2_048_000.0,
            step: 0.0,
        })
        .unwrap();
        assert!(continuous.contains(1_024_000));
        for range in [
            Range {
                minimum: f64::NAN,
                maximum: 1.0,
                step: 0.0,
            },
            Range {
                minimum: 1.0,
                maximum: f64::INFINITY,
                step: 0.0,
            },
            Range {
                minimum: 2.0,
                maximum: 1.0,
                step: 0.0,
            },
            Range {
                minimum: 1.0,
                maximum: 2.0,
                step: -1.0,
            },
        ] {
            assert!(sample_rate_range(range).is_err());
        }
    }

    #[test]
    fn invalid_frequency_metadata_and_selection_fail_before_device_open() {
        for range in [
            Range {
                minimum: -1.0,
                maximum: 1.0,
                step: 0.0,
            },
            Range {
                minimum: f64::NAN,
                maximum: 1.0,
                step: 0.0,
            },
            Range {
                minimum: 2.0,
                maximum: 1.0,
                step: 0.0,
            },
            Range {
                minimum: 1.0,
                maximum: f64::INFINITY,
                step: 0.0,
            },
        ] {
            assert!(frequency_range(range).is_err());
        }
        for id in ["", "driver=remote", "soapy-zzzzzzzzzzzzzzzz"] {
            assert!(SoapyReceiver::open(id, 0, 1_024_000).is_err());
        }
        assert!(SoapyReceiver::open("soapy-0000000000000000", 0, 0).is_err());
    }
}
