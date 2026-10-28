use rust_radio_audio::playback::{NativeFmDeemphasis, NativeFmSettings};
use serde::{Deserialize, Serialize};

pub const MAX_PRESET_BYTES: usize = 16 * 1024;
const MAX_PRESETS: usize = 16;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FmPreset {
    pub name: String,
    pub frequency_hz: u64,
    pub bandwidth_hz: u32,
    pub stereo: bool,
    pub deemphasis_us: u32,
    pub filter_enabled: bool,
    pub low_cut_hz: u32,
    pub high_cut_hz: u32,
    pub limiter: bool,
}
impl FmPreset {
    pub fn new(
        name: String,
        frequency_hz: u64,
        bandwidth_hz: u32,
        fm: NativeFmSettings,
    ) -> Result<Self, String> {
        let preset = Self {
            name: name.trim().into(),
            frequency_hz,
            bandwidth_hz,
            stereo: fm.stereo,
            deemphasis_us: fm.deemphasis as u32,
            filter_enabled: fm.filter_enabled,
            low_cut_hz: fm.low_cut_hz,
            high_cut_hz: fm.high_cut_hz,
            limiter: fm.limiter,
        };
        preset.validate()?;
        Ok(preset)
    }
    pub fn settings(&self) -> Result<NativeFmSettings, String> {
        let deemphasis = match self.deemphasis_us {
            0 => NativeFmDeemphasis::Off,
            50 => NativeFmDeemphasis::Us50,
            75 => NativeFmDeemphasis::Us75,
            _ => return Err("Invalid preset de-emphasis".into()),
        };
        let fm = NativeFmSettings {
            stereo: self.stereo,
            deemphasis,
            filter_enabled: self.filter_enabled,
            low_cut_hz: self.low_cut_hz,
            high_cut_hz: self.high_cut_hz,
            limiter: self.limiter,
        };
        fm.validate().map_err(|e| e.to_string())?;
        Ok(fm)
    }
    fn validate(&self) -> Result<(), String> {
        if self.name != self.name.trim()
            || self.name.trim().is_empty()
            || self.name.len() > 48
            || self.name.chars().any(char::is_control)
            || !(64_000_000..=108_000_000).contains(&self.frequency_hz)
            || !(100_000..=250_000).contains(&self.bandwidth_hz)
        {
            return Err(
                "Use a short preset name, broadcast FM frequency and 100–250 kHz bandwidth".into(),
            );
        }
        self.settings()?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PresetBank {
    version: u8,
    pub stations: Vec<FmPreset>,
}
impl Default for PresetBank {
    fn default() -> Self {
        Self {
            version: 1,
            stations: vec![FmPreset::new(
                "WCRB Boston".into(),
                99_500_000,
                200_000,
                NativeFmSettings::default(),
            )
            .expect("valid built-in preset")],
        }
    }
}
impl PresetBank {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_PRESET_BYTES {
            return Err("Preset file exceeds 16 KiB".into());
        }
        let bank: Self = serde_json::from_str(text).map_err(|_| "Invalid FM preset file")?;
        if bank.version != 1 || bank.stations.len() > MAX_PRESETS {
            return Err("Unsupported FM preset version or count".into());
        }
        let mut names = std::collections::BTreeSet::new();
        for station in &bank.stations {
            station.validate()?;
            if !names.insert(station.name.to_lowercase()) {
                return Err("Duplicate FM preset name".into());
            }
        }
        Ok(bank)
    }
    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).expect("validated presets serialize")
    }
    pub fn save(&mut self, preset: FmPreset) -> Result<(), String> {
        preset.validate()?;
        if let Some(index) = self
            .stations
            .iter()
            .position(|station| station.name.to_lowercase() == preset.name.to_lowercase())
        {
            self.stations[index] = preset;
        } else {
            if self.stations.len() == MAX_PRESETS {
                return Err("Maximum 16 presets; remove one first".into());
            }
            self.stations.push(preset);
        }
        Ok(())
    }
}

pub(super) fn frequency_hz(text: &str) -> Result<u64, String> {
    let normalized = text.trim().to_ascii_lowercase().replace(' ', "");
    let (digits, multiplier) = if let Some(number) = normalized.strip_suffix("mhz") {
        (number, 1_000_000u64)
    } else if let Some(number) = normalized.strip_suffix("khz") {
        (number, 1_000)
    } else {
        (normalized.strip_suffix("hz").unwrap_or(&normalized), 1)
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 6
    {
        return Err("Enter a frequency in Hz, kHz or MHz".into());
    }
    let whole = whole
        .parse::<u64>()
        .ok()
        .and_then(|value| value.checked_mul(multiplier))
        .ok_or("Invalid frequency")?;
    let scale = 10u64.pow(fraction.len() as u32);
    let part = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u64>()
            .map_err(|_| "Invalid frequency")?
            .checked_mul(multiplier)
            .ok_or("Invalid frequency")?
    };
    if part % scale != 0 {
        return Err("Frequency must resolve to whole Hz".into());
    }
    let frequency = whole.checked_add(part / scale).ok_or("Invalid frequency")?;
    if !(1_000..=2_000_000_000).contains(&frequency) {
        return Err("Frequency must be 1 kHz–2 GHz".into());
    }
    Ok(frequency)
}
pub(super) fn format_frequency(hz: u64) -> String {
    format!("{}.{:06} MHz", hz / 1_000_000, hz % 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frequency_units_are_exact_and_never_rounded() {
        for (text, hz) in [
            ("99.5 MHz", 99_500_000),
            ("99500 kHz", 99_500_000),
            ("99500000", 99_500_000),
            ("99.500001MHz", 99_500_001),
        ] {
            assert_eq!(frequency_hz(text).unwrap(), hz);
        }
        for text in [
            "99.5Hz",
            "1e8",
            "-99500000",
            "1.1234567MHz",
            "2000000001",
            "999999999999999999999999",
        ] {
            assert!(frequency_hz(text).is_err());
        }
    }
    #[test]
    fn preset_roundtrip_and_validation_preserve_audio_choices() {
        let mut bank = PresetBank::default();
        let fm = NativeFmSettings {
            deemphasis: NativeFmDeemphasis::Us50,
            stereo: false,
            ..Default::default()
        };
        bank.save(FmPreset::new("Local FM".into(), 99_100_000, 150_000, fm).unwrap())
            .unwrap();
        assert_eq!(PresetBank::parse(&bank.json()).unwrap(), bank);
        bank.save(FmPreset::new("local fm".into(), 99_300_000, 200_000, fm).unwrap())
            .unwrap();
        assert_eq!(bank.stations.len(), 2);
        let mut unknown = bank.clone();
        unknown.version = 2;
        assert!(PresetBank::parse(&unknown.json()).is_err());
        let mut invalid = bank.clone();
        invalid.stations[0].deemphasis_us = 1;
        assert!(PresetBank::parse(&invalid.json()).is_err());
        assert!(PresetBank::parse(&"x".repeat(MAX_PRESET_BYTES + 1)).is_err());
        assert!(FmPreset::new("".into(), 99_100_000, 200_000, fm).is_err());
    }
}
