//! Pure Rust PCM channel mapping and streaming linear sample-rate conversion.
//! Does not open devices or allocate for each output sample.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcmError {
    UnsupportedRate,
    InvalidChannelCount,
    IncompleteFrame,
}

pub struct PcmConverter {
    source_rate: u32,
    output_rate: u32,
    source_channels: usize,
    output_channels: usize,
    previous: Option<[f32; 2]>,
    input_frame: u64,
    next_output_frame: f64,
}

impl PcmConverter {
    pub fn new(
        source_rate: u32,
        output_rate: u32,
        source_channels: usize,
        output_channels: usize,
    ) -> Result<Self, PcmError> {
        if !(8_000..=192_000).contains(&source_rate) || !(8_000..=192_000).contains(&output_rate) {
            return Err(PcmError::UnsupportedRate);
        }
        if !(1..=8).contains(&source_channels) || !(1..=8).contains(&output_channels) {
            return Err(PcmError::InvalidChannelCount);
        }
        Ok(Self {
            source_rate,
            output_rate,
            source_channels,
            output_channels,
            previous: None,
            input_frame: 0,
            next_output_frame: 0.0,
        })
    }

    /// Emit fixed-size interleaved output frames; returning false cancels
    /// processing immediately. Only the first two source channels are used,
    /// and extra device channels receive silence.
    pub fn convert(
        &mut self,
        input: &[f32],
        mut emit: impl FnMut(&[f32]) -> bool,
    ) -> Result<bool, PcmError> {
        if !input.len().is_multiple_of(self.source_channels) {
            return Err(PcmError::IncompleteFrame);
        }
        let step = self.source_rate as f64 / self.output_rate as f64;
        for frame in input.chunks_exact(self.source_channels) {
            let left = sanitize(frame[0]);
            let right = if self.source_channels == 1 {
                left
            } else {
                sanitize(frame[1])
            };
            let current = [left, right];
            if let Some(previous) = self.previous {
                let begin = self.input_frame.saturating_sub(1) as f64;
                let end = self.input_frame as f64;
                while self.next_output_frame <= end {
                    let fraction = (self.next_output_frame - begin).clamp(0.0, 1.0) as f32;
                    let l = previous[0] + (current[0] - previous[0]) * fraction;
                    let r = previous[1] + (current[1] - previous[1]) * fraction;
                    let mut samples = [0.0f32; 8];
                    if self.output_channels == 1 {
                        samples[0] = (l + r) * 0.5;
                    } else {
                        samples[0] = l;
                        samples[1] = r;
                    }
                    if !emit(&samples[..self.output_channels]) {
                        return Ok(false);
                    }
                    self.next_output_frame += step;
                }
            }
            self.previous = Some(current);
            self.input_frame = self.input_frame.saturating_add(1);
        }
        Ok(true)
    }
}

fn sanitize(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_mapping_preserves_expected_samples() {
        let mut resampler = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
        let mut out = Vec::new();
        assert!(resampler
            .convert(&[0.2, 0.8, 0.4, 0.6], |frame| {
                out.extend_from_slice(frame);
                true
            })
            .unwrap());
        assert_eq!(out.len(), 4);
        assert!((out[0] - 0.2).abs() < 0.0001);
        assert!((out[3] - 0.6).abs() < 0.0001);
    }

    #[test]
    fn mono_to_stereo_and_surround_silence() {
        let mut resampler = PcmConverter::new(48_000, 48_000, 1, 4).unwrap();
        let mut out = Vec::new();
        resampler
            .convert(&[0.5, 0.5], |frame| {
                out.extend_from_slice(frame);
                true
            })
            .unwrap();
        assert_eq!(out, [0.5, 0.5, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn common_radio_rate_resamples_to_device_rate() {
        let mut resampler = PcmConverter::new(44_100, 48_000, 1, 2).unwrap();
        let mut frames = 0usize;
        let samples = vec![0.4; 44_100];
        assert!(resampler
            .convert(&samples, |_| {
                frames += 1;
                true
            })
            .unwrap());
        assert!((47_998..=48_001).contains(&frames));
    }

    #[test]
    fn preserves_phase_across_packet_boundaries() {
        let mut one = PcmConverter::new(44_100, 48_000, 1, 1).unwrap();
        let mut two = PcmConverter::new(44_100, 48_000, 1, 1).unwrap();
        let signal: Vec<_> = (0..100).map(|n| n as f32 / 100.0).collect();
        let mut single = Vec::new();
        let mut split = Vec::new();
        one.convert(&signal, |v| {
            single.extend_from_slice(v);
            true
        })
        .unwrap();
        for chunk in signal.chunks(17) {
            two.convert(chunk, |v| {
                split.extend_from_slice(v);
                true
            })
            .unwrap();
        }
        assert_eq!(single, split);
    }

    #[test]
    fn invalid_format_and_cancellation() {
        assert!(matches!(
            PcmConverter::new(0, 48_000, 2, 2),
            Err(PcmError::UnsupportedRate)
        ));
        assert!(matches!(
            PcmConverter::new(44_100, 48_000, 0, 2),
            Err(PcmError::InvalidChannelCount)
        ));
        let mut r = PcmConverter::new(44_100, 48_000, 1, 2).unwrap();
        let mut calls = 0;
        assert!(!r
            .convert(&[0.2; 32], |_| {
                calls += 1;
                calls < 3
            })
            .unwrap());
    }

    #[test]
    fn non_finite_inputs_do_not_leak() {
        let mut r = PcmConverter::new(48_000, 48_000, 1, 1).unwrap();
        let mut out = Vec::new();
        r.convert(&[f32::NAN, f32::INFINITY], |v| {
            out.extend_from_slice(v);
            true
        })
        .unwrap();
        assert_eq!(out, vec![0.0; 2]);
    }
}
