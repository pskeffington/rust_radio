//! Mono broadcast-FM demodulation for 240 ksample/s channelized I/Q.
//! This is a streaming DSP component; it opens no device and proves no RF reception.
use std::f32::consts::PI;

pub const INPUT_RATE_HZ: u32 = 240_000;
pub const AUDIO_RATE_HZ: u32 = 48_000;
const DECIMATION: usize = 5;
const TAPS: usize = 121;
const CUTOFF_HZ: f32 = 19_000.0;
const FM_DEVIATION_HZ: f32 = 75_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deemphasis {
    Us50,
    Us75,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    IncompleteIqPair,
}

/// Converts signed interleaved I/Q to mono 48 kHz samples. Input must already
/// be channelized around the desired broadcast station at 240 ksample/s.
/// State is retained across chunks; a false callback result stops immediately.
pub struct MonoWfm {
    taps: [f32; TAPS],
    history: [f32; TAPS],
    cursor: usize,
    phase: usize,
    previous: Option<(f32, f32)>,
    dc_input: f32,
    dc_output: f32,
    deemphasis: f32,
    deemphasis_alpha: f32,
}

impl MonoWfm {
    pub fn new(deemphasis: Deemphasis) -> Self {
        // Hamming-windowed low pass reduces high-frequency composite content
        // and limits aliasing before 5:1 audio decimation.
        let mut taps = [0.0; TAPS];
        let cutoff = CUTOFF_HZ / INPUT_RATE_HZ as f32;
        let middle = (TAPS - 1) as f32 / 2.0;
        for (index, tap) in taps.iter_mut().enumerate() {
            let x = index as f32 - middle;
            let sinc = if x == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * PI * cutoff * x).sin() / (PI * x)
            };
            let window = 0.54 - 0.46 * (2.0 * PI * index as f32 / (TAPS - 1) as f32).cos();
            *tap = sinc * window;
        }
        let sum: f32 = taps.iter().sum();
        for tap in &mut taps {
            *tap /= sum;
        }
        let tau = match deemphasis {
            Deemphasis::Us50 => 50e-6,
            Deemphasis::Us75 => 75e-6,
        };
        let deemphasis_alpha = 1.0 - (-1.0 / (AUDIO_RATE_HZ as f32 * tau)).exp();
        Self {
            taps,
            history: [0.0; TAPS],
            cursor: 0,
            phase: 0,
            previous: None,
            dc_input: 0.0,
            dc_output: 0.0,
            deemphasis: 0.0,
            deemphasis_alpha,
        }
    }

    pub fn process(
        &mut self,
        iq: &[i16],
        mut on_audio: impl FnMut(f32) -> bool,
    ) -> Result<bool, Error> {
        if iq.len() % 2 != 0 {
            return Err(Error::IncompleteIqPair);
        }
        for pair in iq.chunks_exact(2) {
            let i = f32::from(pair[0]) / 32768.0;
            let q = f32::from(pair[1]) / 32768.0;
            let current = if i == 0.0 && q == 0.0 {
                None
            } else {
                Some((i, q))
            };
            let discriminator = match (self.previous, current) {
                (Some((pi, pq)), Some((ci, cq))) => {
                    let cross = pi * cq - pq * ci;
                    let dot = pi * ci + pq * cq;
                    cross.atan2(dot) * INPUT_RATE_HZ as f32 / (2.0 * PI * FM_DEVIATION_HZ)
                }
                _ => 0.0,
            };
            self.previous = current;
            self.history[self.cursor] = discriminator;
            self.cursor = (self.cursor + 1) % TAPS;
            self.phase = (self.phase + 1) % DECIMATION;
            if self.phase == 0 {
                let mut value = 0.0;
                for (offset, tap) in self.taps.iter().enumerate() {
                    value += tap * self.history[(self.cursor + TAPS - 1 - offset) % TAPS];
                }
                // Remove tuner offset/DC before applying standard FM de-emphasis.
                let highpass = value - self.dc_input + 0.9961 * self.dc_output;
                self.dc_input = value;
                self.dc_output = highpass;
                self.deemphasis += self.deemphasis_alpha * (highpass - self.deemphasis);
                if !on_audio(self.deemphasis.clamp(-1.0, 1.0)) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fm_tone(frequency_hz: f32) -> Vec<i16> {
        let mut iq = Vec::with_capacity(INPUT_RATE_HZ as usize / 2);
        let mut phase = 0.0_f32;
        for n in 0..INPUT_RATE_HZ / 4 {
            let audio = (2.0 * PI * frequency_hz * n as f32 / INPUT_RATE_HZ as f32).sin();
            phase += 2.0 * PI * 45_000.0 * audio / INPUT_RATE_HZ as f32;
            iq.push((phase.cos() * 24_000.0) as i16);
            iq.push((phase.sin() * 24_000.0) as i16);
        }
        iq
    }

    #[test]
    fn demodulates_synthetic_fm_tone_across_chunk_boundaries() {
        let iq = fm_tone(1000.0);
        let mut whole = MonoWfm::new(Deemphasis::Us75);
        let mut chunked = MonoWfm::new(Deemphasis::Us75);
        let mut a = Vec::new();
        let mut b = Vec::new();
        whole
            .process(&iq, |sample| {
                a.push(sample);
                true
            })
            .unwrap();
        for chunk in iq.chunks(514) {
            chunked
                .process(chunk, |sample| {
                    b.push(sample);
                    true
                })
                .unwrap();
        }
        assert_eq!(a, b);
        assert_eq!(a.len(), AUDIO_RATE_HZ as usize / 4);
        let steady = &a[1000..];
        let rms = (steady.iter().map(|s| s * s).sum::<f32>() / steady.len() as f32).sqrt();
        assert!((0.25..0.55).contains(&rms), "tone RMS: {rms}");
        let crossings = steady
            .windows(2)
            .filter(|p| p[0] <= 0.0 && p[1] > 0.0)
            .count();
        let hz = crossings as f32 * AUDIO_RATE_HZ as f32 / steady.len() as f32;
        assert!((980.0..1020.0).contains(&hz), "tone frequency: {hz}");
    }

    #[test]
    fn rejects_incomplete_pairs_and_silence_stays_silent() {
        let mut demod = MonoWfm::new(Deemphasis::Us50);
        assert_eq!(demod.process(&[0], |_| true), Err(Error::IncompleteIqPair));
        let mut audio = Vec::new();
        assert!(demod
            .process(&[0; 2000], |sample| {
                audio.push(sample);
                true
            })
            .unwrap());
        assert_eq!(audio.len(), 200);
        assert!(audio.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn callback_can_stop_processing_immediately() {
        let mut demod = MonoWfm::new(Deemphasis::Us75);
        let mut frames = 0;
        assert!(!demod
            .process(&fm_tone(1000.0), |_| {
                frames += 1;
                false
            })
            .unwrap());
        assert_eq!(frames, 1);
    }

    #[test]
    fn filters_high_frequency_composite_before_decimation() {
        let mut demod = MonoWfm::new(Deemphasis::Us75);
        let mut audio = Vec::new();
        demod
            .process(&fm_tone(30_000.0), |sample| {
                audio.push(sample);
                true
            })
            .unwrap();
        let steady = &audio[1000..];
        let rms = (steady.iter().map(|s| s * s).sum::<f32>() / steady.len() as f32).sqrt();
        assert!(rms < 0.02, "out-of-band RMS: {rms}");
    }
}
