use super::*;
use rust_radio_sdr::nrsp_native::{Client, Config, Endpoint, Status};
use std::sync::atomic::{AtomicU32, AtomicU64};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioStats {
    pub decoded_frames: u64,
    pub peak: f32,
}

pub(super) struct Control {
    frames: AtomicU64,
    peak: AtomicU32,
    pub(super) desired_mode: AtomicU32,
    pub(super) pending: Mutex<Option<Config>>,
    pub(super) pending_fm: Mutex<Option<NativeFmSettings>>,
    pub(super) observed: Mutex<Option<Status>>,
    desired_settings: Mutex<(Config, Option<NativeFmSettings>)>,
}

impl Control {
    pub(super) fn new(config: Config, fm: Option<NativeFmSettings>) -> Self {
        Self {
            frames: AtomicU64::new(0),
            peak: AtomicU32::new(0),
            desired_mode: AtomicU32::new(config.mode as u32),
            pending: Mutex::new(None),
            pending_fm: Mutex::new(None),
            observed: Mutex::new(None),
            desired_settings: Mutex::new((config, fm)),
        }
    }

    pub(super) fn desired_settings(&self) -> (Config, Option<NativeFmSettings>) {
        *self
            .desired_settings
            .lock()
            .expect("native settings mutex poisoned")
    }

    pub(super) fn set_desired_config(&self, config: Config) {
        self.desired_settings
            .lock()
            .expect("native settings mutex poisoned")
            .0 = config;
    }

    pub(super) fn set_desired_fm(&self, settings: NativeFmSettings) {
        self.desired_settings
            .lock()
            .expect("native settings mutex poisoned")
            .1 = Some(settings);
    }

    pub(super) fn stats(&self) -> AudioStats {
        AudioStats {
            decoded_frames: self.frames.load(Ordering::Acquire),
            peak: f32::from_bits(self.peak.load(Ordering::Acquire)),
        }
    }
}

fn startup_failure(
    error: rust_radio_sdr::nrsp_native::Error,
    reconnecting: bool,
) -> PlaybackFailure {
    match error {
        rust_radio_sdr::nrsp_native::Error::Transport(_)
        | rust_radio_sdr::nrsp_native::Error::Timeout
            if reconnecting =>
        {
            PlaybackFailure::Network(error.to_string())
        }
        _ => PlaybackFailure::Permanent(error.to_string()),
    }
}

fn stream_failure(error: rust_radio_sdr::nrsp_native::Error) -> PlaybackFailure {
    match error {
        rust_radio_sdr::nrsp_native::Error::Transport(_)
        | rust_radio_sdr::nrsp_native::Error::Timeout => {
            PlaybackFailure::Network(error.to_string())
        }
        _ => PlaybackFailure::Permanent(error.to_string()),
    }
}

pub(super) fn play(
    (endpoint, config, fm): (Endpoint, Config, Option<NativeFmSettings>),
    context: PlaybackContext<'_>,
    control: &Control,
    reconnecting: bool,
) -> Result<(), PlaybackFailure> {
    control
        .desired_mode
        .store(config.mode as u32, Ordering::Release);
    let mut decoder = opus::Decoder::new(48_000, opus::Channels::Stereo)
        .map_err(|e| format!("Native receiver Opus decoder unavailable: {e}"))?;
    let mut client = Client::connect(endpoint, context.cancelled)
        .map_err(|error| startup_failure(error, reconnecting))?;
    if fm.is_some_and(|settings| settings.stereo) && !client.status().stereo_allowed {
        return Err("Receiver does not permit stereo FM; select mono".into());
    }
    let played_samples = begin_output_attempt(context.output_progress);
    let mut output = NativeAudioOutput::open_with_progress(
        Arc::clone(&context.volume),
        context.output_id,
        played_samples,
        Arc::clone(&context.output_underruns),
    )
    .map_err(PlaybackFailure::Output)?;
    let mut converter =
        PcmConverter::new(48_000, output.sample_rate, 2, output.channels).map_err(|e| {
            PlaybackFailure::Output(format!("Unsupported native receiver output: {e:?}"))
        })?;
    if context.cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    client
        .start(config, context.cancelled)
        .map_err(|e| e.to_string())?;
    if let Some(fm) = fm {
        client
            .set_fm(fm, context.cancelled)
            .map_err(|e| e.to_string())?;
    }
    *control
        .observed
        .lock()
        .expect("native status mutex poisoned") = Some(client.status().clone());
    signal(
        context.controller,
        context.generation,
        PlaybackEvent::Connected,
    );
    // Opus permits up to 120 ms (5760 frames) per packet at 48 kHz.
    let mut pcm = vec![0.0; 5760 * 2];
    let mut sink = ip_pcm::SinkState::default();
    let mut render_error = None;
    let result = client.stream_audio_with_control(
        context.cancelled,
        |client| {
            let pending = control
                .pending
                .lock()
                .expect("native controls mutex poisoned")
                .take();
            if let Some(config) = pending {
                client.tune(config, context.cancelled)?;
            }
            let fm = control
                .pending_fm
                .lock()
                .expect("native FM controls mutex poisoned")
                .take();
            if let Some(fm) =
                fm.filter(|_| client.status().configured_mode == Some(NativeNrspMode::Wfm))
            {
                client.set_fm(fm, context.cancelled)?;
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
                context.cancelled,
                |event| signal(context.controller, context.generation, event),
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
    if context.cancelled.load(Ordering::Acquire) {
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
    result.map_err(stream_failure)?;
    stop_result.map_err(|e| e.to_string())?;
    Err(PlaybackFailure::Permanent(
        "Native receiver stream ended".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_radio_sdr::nrsp_native::Error;

    #[test]
    fn active_stream_transport_errors_recover_but_initial_activation_errors_do_not() {
        assert_eq!(
            stream_failure(Error::Timeout).class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            stream_failure(Error::Transport("socket closed".into())).class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            stream_failure(Error::Protocol("invalid frame")).class(),
            FailureClass::Permanent
        );
        let disconnected = stream_failure(Error::Disconnected(3));
        assert_eq!(disconnected.class(), FailureClass::Permanent);
        assert_eq!(
            disconnected.message(),
            "Receiver ended the native session (reason 3)"
        );
        assert_eq!(
            startup_failure(Error::Timeout, false).class(),
            FailureClass::Permanent
        );
        assert_eq!(
            startup_failure(Error::Transport("receiver unavailable".into()), false).class(),
            FailureClass::Permanent
        );
        assert_eq!(
            startup_failure(Error::Protocol("invalid reconnect hello"), true).class(),
            FailureClass::Permanent
        );
        assert_eq!(
            startup_failure(
                Error::Transport("receiver temporarily offline".into()),
                true
            )
            .class(),
            FailureClass::NetworkTransient
        );
    }

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
