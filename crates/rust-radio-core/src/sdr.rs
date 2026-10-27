//! Receive-only SDR tuner abstraction.
//!
//! Drivers implement `RadioDevice`; application code uses `RadioTuner<D>`
//! independently of the physical SDR vendor. No device is opened on import.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrequencyRange {
    pub min_hz: u64,
    pub max_hz: u64,
}

impl FrequencyRange {
    pub fn contains(self, hz: u64) -> bool {
        self.min_hz <= hz && hz <= self.max_hz
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demodulation {
    Am,
    Nfm,
    Wfm,
    Usb,
    Lsb,
    RawIq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadioCapabilities {
    pub name: String,
    pub frequency_ranges: Vec<FrequencyRange>,
    pub sample_rates_hz: Vec<u32>,
    pub gain_range_db_tenths: Option<(i16, i16)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    pub frequency_hz: u64,
    pub sample_rate_hz: u32,
    pub demodulation: Demodulation,
    /// None delegates gain selection to the hardware's automatic gain control.
    pub gain_db_tenths: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioError {
    UnsupportedFrequency(u64),
    UnsupportedSampleRate(u32),
    UnsupportedGain(i16),
    InvalidCapabilities,
    NotConfigured,
    Device(String),
}

impl fmt::Display for RadioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RadioError {}

/// Receive-only hardware contract. Implementations must not transmit.
pub trait RadioDevice {
    fn capabilities(&self) -> &RadioCapabilities;
    fn apply_tuning(&mut self, tuning: Tuning) -> Result<(), RadioError>;
    /// Return interleaved signed I/Q samples from a bounded driver read.
    /// A return value of zero means no samples currently available.
    fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError>;
}

pub struct RadioTuner<D: RadioDevice> {
    device: D,
    active: Option<Tuning>,
}

impl<D: RadioDevice> RadioTuner<D> {
    pub fn new(device: D) -> Result<Self, RadioError> {
        let c = device.capabilities();
        if c.frequency_ranges.is_empty()
            || c.sample_rates_hz.is_empty()
            || c.frequency_ranges.iter().any(|r| r.min_hz > r.max_hz)
            || c.sample_rates_hz.contains(&0)
            || c.gain_range_db_tenths.is_some_and(|(min, max)| min > max)
        {
            return Err(RadioError::InvalidCapabilities);
        }
        Ok(Self {
            device,
            active: None,
        })
    }

    pub fn capabilities(&self) -> &RadioCapabilities {
        self.device.capabilities()
    }

    pub fn active_tuning(&self) -> Option<Tuning> {
        self.active
    }

    /// State changes only after the driver acknowledges the configuration.
    pub fn tune(&mut self, settings: Tuning) -> Result<(), RadioError> {
        let c = self.device.capabilities();
        if !c
            .frequency_ranges
            .iter()
            .any(|r| r.contains(settings.frequency_hz))
        {
            return Err(RadioError::UnsupportedFrequency(settings.frequency_hz));
        }
        if !c.sample_rates_hz.contains(&settings.sample_rate_hz) {
            return Err(RadioError::UnsupportedSampleRate(settings.sample_rate_hz));
        }
        if let Some(gain) = settings.gain_db_tenths {
            if !c
                .gain_range_db_tenths
                .is_some_and(|(low, high)| (low..=high).contains(&gain))
            {
                return Err(RadioError::UnsupportedGain(gain));
            }
        }
        self.device.apply_tuning(settings)?;
        self.active = Some(settings);
        Ok(())
    }

    pub fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError> {
        if self.active.is_none() {
            return Err(RadioError::NotConfigured);
        }
        if buffer.len() % 2 != 0 {
            return Err(RadioError::Device("I/Q buffer length must be even".into()));
        }
        let count = self.device.read_iq(buffer)?;
        if count > buffer.len() || count % 2 != 0 {
            return Err(RadioError::Device(
                "driver returned invalid I/Q count".into(),
            ));
        }
        Ok(count)
    }
}

/// Software-only adapter used for development without SDR hardware.
pub struct SimulatedRadio {
    capabilities: RadioCapabilities,
    tuning: Option<Tuning>,
}

impl Default for SimulatedRadio {
    fn default() -> Self {
        Self {
            capabilities: RadioCapabilities {
                name: "SIMULATED SDR - NO RF INPUT".into(),
                frequency_ranges: vec![FrequencyRange {
                    min_hz: 24_000_000,
                    max_hz: 1_700_000_000,
                }],
                sample_rates_hz: vec![240_000, 1_024_000, 2_048_000],
                gain_range_db_tenths: Some((0, 500)),
            },
            tuning: None,
        }
    }
}

impl RadioDevice for SimulatedRadio {
    fn capabilities(&self) -> &RadioCapabilities {
        &self.capabilities
    }

    fn apply_tuning(&mut self, tuning: Tuning) -> Result<(), RadioError> {
        self.tuning = Some(tuning);
        Ok(())
    }

    fn read_iq(&mut self, buffer: &mut [i16]) -> Result<usize, RadioError> {
        // Explicit silence: never claim synthetic samples came from the antenna.
        buffer.fill(0);
        Ok(buffer.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Tuning {
        Tuning {
            frequency_hz: 100_000_000,
            sample_rate_hz: 1_024_000,
            demodulation: Demodulation::Wfm,
            gain_db_tenths: None,
        }
    }

    #[test]
    fn tune_and_read_simulated_iq() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        assert_eq!(tuner.read_iq(&mut [0; 4]), Err(RadioError::NotConfigured));
        tuner.tune(settings()).unwrap();
        let mut buffer = [4i16; 8];
        assert_eq!(tuner.read_iq(&mut buffer).unwrap(), 8);
        assert_eq!(buffer, [0; 8]);
    }

    #[test]
    fn rejects_frequency_and_sample_rate_without_changing_state() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        let mut invalid = settings();
        invalid.frequency_hz = 1;
        assert_eq!(
            tuner.tune(invalid),
            Err(RadioError::UnsupportedFrequency(1))
        );
        invalid = settings();
        invalid.sample_rate_hz = 1;
        assert_eq!(
            tuner.tune(invalid),
            Err(RadioError::UnsupportedSampleRate(1))
        );
        assert_eq!(tuner.active_tuning(), None);
    }

    #[test]
    fn rejects_unavailable_gain_and_odd_iq_buffer() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        let mut invalid = settings();
        invalid.gain_db_tenths = Some(600);
        assert_eq!(tuner.tune(invalid), Err(RadioError::UnsupportedGain(600)));
        tuner.tune(settings()).unwrap();
        assert!(tuner.read_iq(&mut [0; 3]).is_err());
    }
}
