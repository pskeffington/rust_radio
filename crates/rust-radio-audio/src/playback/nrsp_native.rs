use super::*;
use rust_radio_sdr::nrsp_native::{Client, Config, Endpoint, Status};
use std::sync::atomic::{AtomicU32, AtomicU64};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioStats {
    pub decoded_frames: u64,
    pub peak: f32,
}

#[derive(Default)]
pub(super) struct Control {
    frames: AtomicU64,
    peak: AtomicU32,
    pub(super) pending: Mutex<Option<Config>>,
    pub(super) observed: Mutex<Option<Status>>,
}

impl Control {
    pub(super) fn stats(&self) -> AudioStats {
        AudioStats {
            decoded_frames: self.frames.load(Ordering::Acquire),
            peak: f32::from_bits(self.peak.load(Ordering::Acquire)),
        }
    }
}

pub(super) fn play(
    (endpoint, config): (Endpoint, Config),
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &AtomicBool,
    volume: Arc<AtomicU8>,
    output_id: &str,
    control: &Control,
) -> Result<(), PlaybackFailure> {
    let mut decoder = opus::Decoder::new(48_000, opus::Channels::Stereo)
        .map_err(|e| format!("Native receiver Opus decoder unavailable: {e}"))?;
    let mut client = Client::connect(endpoint, cancelled).map_err(|e| e.to_string())?;
    let mut output = NativeAudioOutput::open(volume, output_id).map_err(PlaybackFailure::Output)?;
    let mut converter = PcmConverter::new(48_000, output.sample_rate, 2, output.channels)
        .map_err(|e| format!("Unsupported native receiver output: {e:?}"))?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    client.start(config, cancelled).map_err(|e| e.to_string())?;
    *control
        .observed
        .lock()
        .expect("native status mutex poisoned") = Some(client.status().clone());
    signal(controller, generation, PlaybackEvent::Connected);
    // Opus permits up to 120 ms (5760 frames) per packet at 48 kHz.
    let mut pcm = vec![0.0; 5760 * 2];
    let mut sink = ip_pcm::SinkState::default();
    let mut render_error = None;
    let result = client.stream_audio_with_control(
        cancelled,
        |client| {
            let pending = control
                .pending
                .lock()
                .expect("native controls mutex poisoned")
                .take();
            if let Some(config) = pending {
                client.tune(config, cancelled)?;
            }
            *control
                .observed
                .lock()
                .expect("native status mutex poisoned") = Some(client.status().clone());
            Ok(())
        },
        |packet| {
            let frames = match decoder.decode_float(packet, &mut pcm, false) {
                Ok(frames) => frames,
                Err(error) => {
                    render_error = Some(PlaybackFailure::Permanent(format!(
                        "Invalid native receiver Opus packet: {error}"
                    )));
                    return false;
                }
            };
            if frames == 0 || pcm[..frames * 2].iter().any(|sample| !sample.is_finite()) {
                render_error = Some(PlaybackFailure::Permanent(
                    "Invalid native receiver PCM".into(),
                ));
                return false;
            }
            control.frames.fetch_add(frames as u64, Ordering::Relaxed);
            let peak = pcm[..frames * 2]
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            control.peak.fetch_max(peak.to_bits(), Ordering::Relaxed);
            match sink.feed(
                &mut converter,
                &pcm[..frames * 2],
                &mut output,
                cancelled,
                |event| signal(controller, generation, event),
            ) {
                Ok(keep_going) => keep_going && !output.has_failed(),
                Err(error) => {
                    render_error = Some(error);
                    false
                }
            }
        },
    );
    // Drop closes the owned connection even when the stop write cannot complete.
    let stop_result = client.stop();
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    if let Some(error) = render_error {
        return Err(error);
    }
    if output.has_failed() {
        return Err(PlaybackFailure::Output(
            "Native receiver audio output failed".into(),
        ));
    }
    result.map_err(|e| e.to_string())?;
    stop_result.map_err(|e| e.to_string())?;
    Err(PlaybackFailure::Permanent(
        "Native receiver stream ended".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opus_silence_packet_decodes_finite_stereo_without_output_or_network() {
        let mut decoder = opus::Decoder::new(48_000, opus::Channels::Stereo).unwrap();
        let mut pcm = vec![0.0; 5760 * 2];
        let frames = decoder
            .decode_float(&[0xf8, 0xff, 0xfe], &mut pcm, false)
            .unwrap();
        assert_eq!(frames, 960);
        assert!(pcm[..frames * 2].iter().all(|sample| sample.is_finite()));
    }
    #[test]
    fn native_engine_rejects_bad_parameters_before_starting_resources() {
        for (endpoint, frequency, mode, bandwidth) in [
            ("8.8.8.8:9001", 99_100_000, "wfm", 200_000),
            ("127.0.0.1:9001", 0, "wfm", 200_000),
            ("127.0.0.1:9001", 99_100_000, "invalid", 200_000),
        ] {
            assert!(NativeRadioEngine::start_nrsp_native(
                endpoint, frequency, mode, bandwidth, 0, "system"
            )
            .is_err());
        }
    }
}
