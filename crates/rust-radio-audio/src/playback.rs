//! In-process IP-radio playback worker using native Rust decoding and CPAL.
//! No subprocesses, browser playback, persistent audio files or RF transmit.

use crate::compat_controller::{
    PlaybackEvent, PlaybackState, RadioCommand, RadioController, RadioSession,
};
use crate::media::{AdtsCompat, LiveMediaSource};
use crate::output::NativeAudioOutput;
use rust_radio_core::directory::station_by_id;
use rust_radio_core::pcm::PcmConverter;
use rust_radio_core::recovery::{decide, FailureClass, RecoveryAction};
use rust_radio_ip::error::TransportError;
use rust_radio_ip::net as radio_net;
use std::io;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

mod ip_pcm;
#[cfg(feature = "nrsp-native")]
mod nrsp_native;
#[cfg(feature = "nrsp-native")]
pub use nrsp_native::AudioStats as NativeNrspAudioStats;
#[cfg(feature = "nrsp-native")]
pub use rust_radio_sdr::nrsp_native::{
    Deemphasis as NativeFmDeemphasis, FmSettings as NativeFmSettings, Mode as NativeNrspMode,
};
#[cfg(feature = "sdrconnect")]
mod sdrconnect;
#[cfg(feature = "soapy")]
mod soapy;

enum PlaybackSource {
    Internet(String),
    Direct(String),
    #[cfg(feature = "nrsp-native")]
    NrspNative(
        rust_radio_sdr::nrsp_native::Endpoint,
        rust_radio_sdr::nrsp_native::Config,
        Option<NativeFmSettings>,
    ),
    #[cfg(feature = "sdrconnect")]
    SdrConnect(rust_radio_sdr::sdrconnect::Endpoint),
    #[cfg(feature = "soapy")]
    SoapyWfm {
        id: String,
        frequency_hz: u64,
        deemphasis: rust_radio_core::sdr::wfm::Deemphasis,
    },
}

#[derive(Debug)]
enum PlaybackFailure {
    Network(String),
    Output(String),
    Permanent(String),
}

impl PlaybackFailure {
    fn class(&self) -> FailureClass {
        match self {
            Self::Network(_) => FailureClass::NetworkTransient,
            Self::Output(_) => FailureClass::OutputUnavailable,
            Self::Permanent(_) => FailureClass::Permanent,
        }
    }

    fn message(self) -> String {
        match self {
            Self::Network(message) | Self::Output(message) | Self::Permanent(message) => message,
        }
    }
}

impl From<String> for PlaybackFailure {
    fn from(message: String) -> Self {
        Self::Permanent(message)
    }
}

impl From<&str> for PlaybackFailure {
    fn from(message: &str) -> Self {
        Self::Permanent(message.to_owned())
    }
}

impl From<TransportError> for PlaybackFailure {
    fn from(error: TransportError) -> Self {
        match error {
            TransportError::Network(message) => Self::Network(message),
            TransportError::Permanent(message) => Self::Permanent(message),
            TransportError::Cancelled => Self::Permanent(error.to_string()),
        }
    }
}

fn probe_failure(error: DecodeError) -> PlaybackFailure {
    match error {
        DecodeError::IoError(error) => PlaybackFailure::Network(format!(
            "Radio stream interrupted during format detection: {error}"
        )),
        error => PlaybackFailure::Permanent(format!("Unsupported radio stream format: {error}")),
    }
}

/// No direct audio output or network access occurs until the caller issues
/// an explicit internet or SDRconnect Play command. One engine owns one active worker.
pub struct NativeRadioEngine {
    controller: Arc<Mutex<RadioController>>,
    cancelled: Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    output_failed: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    #[cfg(feature = "nrsp-native")]
    native_control: Option<Arc<nrsp_native::Control>>,
    #[cfg(feature = "sdrconnect")]
    receiver: Arc<Mutex<Option<rust_radio_sdr::sdrconnect::Status>>>,
}

impl NativeRadioEngine {
    pub fn start_internet(
        station_id: &str,
        volume_percent: u8,
        output_id: &str,
    ) -> Result<Self, String> {
        if station_by_id(station_id).is_none() {
            return Err("Unknown curated internet radio station".into());
        }
        Self::start(
            PlaybackSource::Internet(station_id.to_owned()),
            volume_percent,
            output_id,
        )
    }

    /// Play an explicitly selected direct MP3 or AAC stream URL.
    pub fn start_stream(url: &str, volume_percent: u8, output_id: &str) -> Result<Self, String> {
        radio_net::validate_direct_url(url)?;
        Self::start(
            PlaybackSource::Direct(url.to_owned()),
            volume_percent,
            output_id,
        )
    }

    /// Connect, tune and receive directly from the nRSP-ST's native service.
    #[cfg(feature = "nrsp-native")]
    pub fn start_nrsp_native(
        endpoint: &str,
        frequency_hz: u64,
        mode: &str,
        bandwidth_hz: u32,
        volume_percent: u8,
        output_id: &str,
    ) -> Result<Self, String> {
        use rust_radio_sdr::nrsp_native::{Config, Endpoint, Mode};
        let endpoint = Endpoint::parse(endpoint).map_err(|e| e.to_string())?;
        let mode = Mode::parse(mode).map_err(|e| e.to_string())?;
        let config = Config::new(frequency_hz, mode, bandwidth_hz).map_err(|e| e.to_string())?;
        Self::start(
            PlaybackSource::NrspNative(endpoint, config, None),
            volume_percent,
            output_id,
        )
    }

    #[cfg(feature = "nrsp-native")]
    pub fn start_nrsp_native_fm(
        endpoint: &str,
        frequency_hz: u64,
        bandwidth_hz: u32,
        settings: NativeFmSettings,
        volume_percent: u8,
        output_id: &str,
    ) -> Result<Self, String> {
        use rust_radio_sdr::nrsp_native::{Config, Endpoint, Mode};
        settings.validate().map_err(|e| e.to_string())?;
        let endpoint = Endpoint::parse(endpoint).map_err(|e| e.to_string())?;
        let config =
            Config::new(frequency_hz, Mode::Wfm, bandwidth_hz).map_err(|e| e.to_string())?;
        Self::start(
            PlaybackSource::NrspNative(endpoint, config, Some(settings)),
            volume_percent,
            output_id,
        )
    }

    /// Explicit native playback of SDRconnect's selected primary receiver.
    /// Starts reception if stopped and control is allowed. Missing startup
    /// confirmation requires explicit retry; Stop leaves shared reception running.
    /// Device selection, refresh, tuning and recording remain SDRconnect actions.
    #[cfg(feature = "sdrconnect")]
    pub fn start_sdrconnect(
        endpoint: &str,
        volume_percent: u8,
        output_id: &str,
    ) -> Result<Self, String> {
        let endpoint =
            rust_radio_sdr::sdrconnect::Endpoint::parse(endpoint).map_err(|e| e.to_string())?;
        Self::start(
            PlaybackSource::SdrConnect(endpoint),
            volume_percent,
            output_id,
        )
    }

    /// Start mono broadcast FM from an explicitly selected SoapySDR receiver.
    /// The driver must support channelized 240 ksample/s CS16 I/Q; no SDRconnect
    /// process is used. Stop releases the owned receive stream and audio output.
    #[cfg(feature = "soapy")]
    pub fn start_soapy_wfm(
        receiver_id: &str,
        frequency_hz: u64,
        deemphasis: rust_radio_core::sdr::wfm::Deemphasis,
        volume_percent: u8,
        output_id: &str,
    ) -> Result<Self, String> {
        if !receiver_id.starts_with("soapy-")
            || receiver_id.len() != 22
            || !receiver_id[6..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || frequency_hz == 0
        {
            return Err("Use a discovered SoapySDR receiver ID and positive frequency".into());
        }
        Self::start(
            PlaybackSource::SoapyWfm {
                id: receiver_id.to_owned(),
                frequency_hz,
                deemphasis,
            },
            volume_percent,
            output_id,
        )
    }

    #[cfg(feature = "sdrconnect")]
    pub fn receiver_status(&self) -> Option<rust_radio_sdr::sdrconnect::Status> {
        self.receiver
            .lock()
            .expect("receiver state mutex poisoned")
            .clone()
    }

    fn start(source: PlaybackSource, volume_percent: u8, output_id: &str) -> Result<Self, String> {
        if volume_percent > 100 {
            return Err("Volume must be between 0 and 100".into());
        }
        let mut controller = RadioController::default();
        controller
            .apply(RadioCommand::SetVolume(volume_percent))
            .map_err(|e| format!("{e:?}"))?;
        let command = match &source {
            PlaybackSource::Internet(station_id) => RadioCommand::PlayInternet {
                station_id: station_id.clone(),
            },
            PlaybackSource::Direct(_) => RadioCommand::PlayStream,
            #[cfg(feature = "nrsp-native")]
            PlaybackSource::NrspNative(..) => RadioCommand::PlaySdr,
            #[cfg(feature = "sdrconnect")]
            PlaybackSource::SdrConnect(_) => RadioCommand::PlaySdr,
            #[cfg(feature = "soapy")]
            PlaybackSource::SoapyWfm { .. } => RadioCommand::PlaySdr,
        };
        let generation = controller.apply(command).map_err(|e| format!("{e:?}"))?;
        let controller = Arc::new(Mutex::new(controller));
        let cancelled = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(AtomicU8::new(volume_percent));
        let worker_controller = Arc::clone(&controller);
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_volume = Arc::clone(&volume);
        let output_failed = Arc::new(AtomicBool::new(false));
        let worker_output_failed = Arc::clone(&output_failed);
        #[cfg(feature = "sdrconnect")]
        let receiver = Arc::new(Mutex::new(None));
        #[cfg(feature = "sdrconnect")]
        let worker_receiver = Arc::clone(&receiver);
        #[cfg(feature = "nrsp-native")]
        let native_control = if matches!(&source, PlaybackSource::NrspNative(..)) {
            Some(Arc::new(nrsp_native::Control::default()))
        } else {
            None
        };
        #[cfg(feature = "nrsp-native")]
        let worker_native = native_control.clone();
        let selected_output = output_id.to_owned();
        let worker = thread::Builder::new()
            .name("rust-radio-playback".into())
            .spawn(move || {
                let result = match source {
                    PlaybackSource::Internet(id) => play_with_recovery(
                        &id,
                        false,
                        generation,
                        &worker_controller,
                        &worker_cancelled,
                        worker_volume,
                        &selected_output,
                    ),
                    PlaybackSource::Direct(url) => play_with_recovery(
                        &url,
                        true,
                        generation,
                        &worker_controller,
                        &worker_cancelled,
                        worker_volume,
                        &selected_output,
                    ),
                    #[cfg(feature = "nrsp-native")]
                    PlaybackSource::NrspNative(endpoint, config, fm) => nrsp_native::play(
                        (endpoint, config, fm),
                        generation,
                        &worker_controller,
                        &worker_cancelled,
                        worker_volume,
                        &selected_output,
                        worker_native.as_ref().expect("native controls present"),
                    ),
                    #[cfg(feature = "sdrconnect")]
                    PlaybackSource::SdrConnect(endpoint) => recover_attempts(
                        generation,
                        &worker_controller,
                        &worker_cancelled,
                        || {
                            sdrconnect::play(
                                endpoint,
                                generation,
                                &worker_controller,
                                &worker_cancelled,
                                Arc::clone(&worker_volume),
                                &selected_output,
                                &worker_receiver,
                            )
                        },
                        |delay| interruptible_wait(&worker_cancelled, delay),
                    ),
                    #[cfg(feature = "soapy")]
                    PlaybackSource::SoapyWfm {
                        id,
                        frequency_hz,
                        deemphasis,
                    } => soapy::play(
                        (&id, frequency_hz, deemphasis),
                        generation,
                        &worker_controller,
                        &worker_cancelled,
                        worker_volume,
                        &selected_output,
                    ),
                };
                if let Err(error) = result {
                    if !worker_cancelled.load(Ordering::Acquire) {
                        if matches!(error, PlaybackFailure::Output(_)) {
                            worker_output_failed.store(true, Ordering::Release);
                        }
                        worker_controller
                            .lock()
                            .expect("radio state mutex poisoned")
                            .event(generation, PlaybackEvent::Error(error.message()));
                    }
                }
            })
            .map_err(|e| format!("Unable to start radio worker: {e}"))?;
        Ok(Self {
            controller,
            cancelled,
            volume,
            output_failed,
            worker: Some(worker),
            #[cfg(feature = "nrsp-native")]
            native_control,
            #[cfg(feature = "sdrconnect")]
            receiver,
        })
    }

    /// Queue an explicit tune on the current native session. The bounded slot
    /// keeps only the latest request; observed status updates after device response.
    #[cfg(feature = "nrsp-native")]
    pub fn tune_nrsp_native(
        &self,
        frequency_hz: u64,
        mode: &str,
        bandwidth_hz: u32,
    ) -> Result<(), String> {
        use rust_radio_sdr::nrsp_native::{Config, Mode};
        let mode = Mode::parse(mode).map_err(|e| e.to_string())?;
        let config = Config::new(frequency_hz, mode, bandwidth_hz).map_err(|e| e.to_string())?;
        if self.cancelled.load(Ordering::Acquire) || self.status().state != PlaybackState::Playing {
            return Err("Start the native receiver before tuning".into());
        }
        let control = self
            .native_control
            .as_ref()
            .ok_or("The active source is not a direct nRSP-ST")?;
        let mut pending = control
            .pending
            .lock()
            .expect("native controls mutex poisoned");
        control
            .desired_mode
            .store(config.mode as u32, Ordering::Release);
        *pending = Some(config);
        Ok(())
    }
    #[cfg(feature = "nrsp-native")]
    pub fn set_nrsp_native_fm(&self, settings: NativeFmSettings) -> Result<(), String> {
        settings.validate().map_err(|e| e.to_string())?;
        if self.cancelled.load(Ordering::Acquire) || self.status().state != PlaybackState::Playing {
            return Err("Start the native receiver before changing FM settings".into());
        }
        let control = self
            .native_control
            .as_ref()
            .ok_or("The active source is not a direct nRSP-ST")?;
        if control.desired_mode.load(Ordering::Acquire) != NativeNrspMode::Wfm as u32 {
            return Err("FM audio controls require WFM mode".into());
        }
        if settings.stereo
            && control
                .observed
                .lock()
                .expect("native status mutex poisoned")
                .as_ref()
                .is_some_and(|status| !status.stereo_allowed)
        {
            return Err("Receiver does not permit stereo FM".into());
        }
        *control
            .pending_fm
            .lock()
            .expect("native FM controls mutex poisoned") = Some(settings);
        Ok(())
    }

    #[cfg(feature = "nrsp-native")]
    pub fn nrsp_native_status(&self) -> Option<rust_radio_sdr::nrsp_native::Status> {
        self.native_control
            .as_ref()?
            .observed
            .lock()
            .expect("native status mutex poisoned")
            .clone()
    }

    #[cfg(feature = "nrsp-native")]
    pub fn nrsp_native_audio_stats(&self) -> Option<NativeNrspAudioStats> {
        Some(self.native_control.as_ref()?.stats())
    }

    /// An audio-device failure is never treated as transient network loss.
    /// The HUD can offer an explicit output selection/retry.
    pub fn output_failed(&self) -> bool {
        self.output_failed.load(Ordering::Acquire)
    }

    /// A Stop has muted PCM but the owned worker is still releasing resources.
    pub fn shutdown_pending(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
            && self
                .worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished())
    }

    pub fn status(&self) -> RadioSession {
        self.controller
            .lock()
            .expect("radio state mutex poisoned")
            .state()
            .clone()
    }

    pub fn set_volume(&self, percent: u8) -> Result<(), String> {
        self.controller
            .lock()
            .expect("radio state mutex poisoned")
            .apply(RadioCommand::SetVolume(percent))
            .map_err(|e| format!("{e:?}"))?;
        self.volume.store(percent, Ordering::Release);
        if self.cancelled.load(Ordering::Acquire) {
            self.volume.store(0, Ordering::Release);
        }
        Ok(())
    }

    /// Cancel immediately without waiting for network or device cleanup.
    /// Hosts must retain this engine and poll `try_finish` before starting another.
    /// Muting also silences queued PCM while the worker releases its device.
    pub fn request_stop(&self) {
        let already_cancelled = self.cancelled.swap(true, Ordering::AcqRel);
        self.volume.store(0, Ordering::Release);
        if !already_cancelled {
            let _ = self
                .controller
                .lock()
                .expect("radio state mutex poisoned")
                .apply(RadioCommand::Stop);
        }
    }

    /// Reap a completed worker without blocking. False means cleanup is pending.
    /// Unexpected worker termination becomes a visible failure, never stale Playing.
    pub fn try_finish(&mut self) -> bool {
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return false;
        }
        if let Some(worker) = self.worker.take() {
            let panicked = worker.join().is_err();
            if !self.cancelled.load(Ordering::Acquire) {
                let mut controller = self.controller.lock().expect("radio state mutex poisoned");
                let session = controller.state();
                let generation = session.generation;
                if !matches!(
                    session.state,
                    PlaybackState::Failed | PlaybackState::Stopped
                ) {
                    let message = if panicked {
                        "Radio playback worker failed unexpectedly"
                    } else {
                        "Radio playback worker ended unexpectedly"
                    };
                    controller.event(generation, PlaybackEvent::Error(message.into()));
                }
            }
        }
        true
    }

    /// Synchronous shutdown for CLI callers and final teardown.
    /// GUI hosts should use `request_stop` followed by `try_finish` instead.
    pub fn stop(&mut self) {
        self.request_stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for NativeRadioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

fn signal(controller: &Arc<Mutex<RadioController>>, generation: u64, event: PlaybackEvent) {
    controller
        .lock()
        .expect("radio state mutex poisoned")
        .event(generation, event);
}

/// Retry typed connection and stream I/O failures. Device loss and invalid
/// codecs remain terminal; session Stop overrides any scheduled retry.
fn play_with_recovery(
    station_id: &str,
    direct: bool,
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    output_id: &str,
) -> Result<(), PlaybackFailure> {
    recover_attempts(
        generation,
        controller,
        cancelled,
        || {
            decode_and_play(
                station_id,
                direct,
                generation,
                controller,
                cancelled,
                Arc::clone(&volume),
                output_id,
            )
        },
        |delay| interruptible_wait(cancelled, delay),
    )
}

fn recover_attempts(
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &AtomicBool,
    mut attempt: impl FnMut() -> Result<(), PlaybackFailure>,
    mut wait: impl FnMut(Duration) -> bool,
) -> Result<(), PlaybackFailure> {
    let mut attempted_retries = 0;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        let result = attempt();
        let Err(error) = result else {
            return Ok(());
        };
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        match decide(
            error.class(),
            attempted_retries,
            cancelled.load(Ordering::Acquire),
        ) {
            RecoveryAction::RetryAfter(delay) => {
                signal(controller, generation, PlaybackEvent::ConnectionLost);
                attempted_retries += 1;
                // No audio callback, network thread or old PCM queue survives
                // decode_and_play returning before we start this delay.
                if !wait(delay) {
                    return Ok(());
                }
            }
            RecoveryAction::Stop => return Ok(()),
            RecoveryAction::SelectOutput | RecoveryAction::Fail => return Err(error),
        }
    }
}

fn interruptible_wait(cancelled: &AtomicBool, delay: Duration) -> bool {
    let until = Instant::now() + delay;
    while Instant::now() < until {
        if cancelled.load(Ordering::Acquire) {
            return false;
        }
        thread::sleep(
            until
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(25)),
        );
    }
    !cancelled.load(Ordering::Acquire)
}

fn decode_and_play(
    station_id: &str,
    direct: bool,
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    output_id: &str,
) -> Result<(), PlaybackFailure> {
    // Establish an audio device before opening a remote broadcaster.
    let mut output = NativeAudioOutput::open(volume, output_id).map_err(PlaybackFailure::Output)?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    let (stream, _network_guard) = if direct {
        radio_net::open_direct_classified(station_id, Arc::clone(cancelled))?
    } else {
        radio_net::open_station_classified(station_id, Arc::clone(cancelled))?
    };
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    signal(controller, generation, PlaybackEvent::Connected);

    let startup = stream.startup.clone();
    let mut hint = Hint::new();
    let mime = stream.content_type.as_deref().unwrap_or_default();
    if mime.contains("aac") {
        hint.with_extension("aac");
    } else {
        hint.with_extension("mp3");
    }
    let media = if mime.contains("aac") {
        MediaSourceStream::new(
            Box::new(LiveMediaSource::new(AdtsCompat::new(stream.reader))),
            Default::default(),
        )
    } else {
        MediaSourceStream::new(
            Box::new(LiveMediaSource::new(stream.reader)),
            Default::default(),
        )
    };
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            media,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(probe_failure)?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No decodable audio track in stream")?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Unsupported radio codec: {e}"))?;
    let mut converter: Option<PcmConverter> = None;
    let mut previous_format: Option<(u32, usize)> = None;
    let mut sink_state = ip_pcm::SinkState::default();

    while !cancelled.load(Ordering::Acquire) {
        if output.has_failed() {
            return Err(PlaybackFailure::Output(
                "Native audio output device disconnected or failed".into(),
            ));
        }
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(DecodeError::IoError(err)) => {
                if cancelled.load(Ordering::Acquire) {
                    return Ok(());
                }
                if output.has_failed() {
                    return Err(PlaybackFailure::Output(
                        "Native audio output disconnected during radio stream".into(),
                    ));
                }
                return Err(PlaybackFailure::Network(format!(
                    "Broadcast stream ended or stalled: {err}"
                )));
            }
            Err(DecodeError::ResetRequired) => {
                return Err("Broadcast format changed; reconnect required".into());
            }
            Err(err) => {
                return Err(PlaybackFailure::Permanent(format!(
                    "Broadcast packet error: {err}"
                )));
            }
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(DecodeError::DecodeError(_)) => continue,
            Err(err) => {
                return Err(PlaybackFailure::Permanent(format!(
                    "Radio audio decode error: {err}"
                )));
            }
        };
        if decoded.frames() == 0 {
            continue;
        }
        startup.decoded();
        let spec = *decoded.spec();
        let source_channels = spec.channels.count();
        if previous_format != Some((spec.rate, source_channels)) {
            converter = Some(
                PcmConverter::new(
                    spec.rate,
                    output.sample_rate,
                    source_channels,
                    output.channels,
                )
                .map_err(|e| format!("Unsupported broadcast audio format: {e:?}"))?,
            );
            previous_format = Some((spec.rate, source_channels));
        }
        // The allocation and decode occur only on the worker, never in CPAL.
        // Reject unreasonably long decoded packets.
        if decoded.capacity() > 192_000 {
            return Err("Radio audio packet exceeds bounded frame budget".into());
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        let keep_going = sink_state.feed(
            converter.as_mut().expect("PCM converter is initialized"),
            samples.samples(),
            &mut output,
            cancelled,
            |event| signal(controller, generation, event),
        )?;
        if !keep_going {
            if output.has_failed() && !cancelled.load(Ordering::Acquire) {
                return Err(PlaybackFailure::Output(
                    "Native audio output stopped consuming samples".into(),
                ));
            }
            return Ok(());
        }
    }
    Ok(())
}

fn validate_demo_duration(seconds: Option<u64>) -> Result<(), String> {
    if seconds.is_some_and(|value| !(1..=300).contains(&value)) {
        return Err("Demo duration must be 1..300 seconds".into());
    }
    Ok(())
}

/// Interactive operator demo. A non-interactive caller should pass a duration,
/// otherwise standard input supports 'volume N', 'status' and 'stop'.
pub fn run_demo(station_id: &str, duration_seconds: Option<u64>) -> Result<(), String> {
    validate_demo_duration(duration_seconds)?;
    let mut player = NativeRadioEngine::start_internet(station_id, 60, "system")?;
    let mut played_audio = false;
    println!("RUST-BOY RADIO / {station_id} / native Rust audio / 60% volume");
    if let Some(seconds) = duration_seconds {
        let started_at = Instant::now();
        while started_at.elapsed() < Duration::from_secs(seconds) {
            thread::sleep(Duration::from_millis(100));
            let snapshot = player.status();
            if snapshot.state == PlaybackState::Playing {
                played_audio = true;
            }
            if snapshot.state == PlaybackState::Failed {
                let error = snapshot
                    .last_error
                    .unwrap_or_else(|| "Unknown failure".into());
                player.stop();
                return Err(error);
            }
        }
    } else {
        println!("Type: status | volume 0..100 | stop");
        let mut input = String::new();
        loop {
            input.clear();
            let len = io::stdin()
                .read_line(&mut input)
                .map_err(|e| format!("Radio console input error: {e}"))?;
            if len == 0 {
                break;
            }
            let command = input.trim();
            if command == "stop" || command == "q" {
                break;
            }
            if command == "status" {
                let state = player.status();
                println!("RADIO {:?} / {:?}", state.state, state.last_error);
            } else if let Some(value) = command.strip_prefix("volume ") {
                let percent = value
                    .parse::<u8>()
                    .map_err(|_| "Volume must be a number from 0 to 100")?;
                player.set_volume(percent)?;
                println!("RADIO VOLUME {percent}%");
            } else {
                println!("Available: status | volume N | stop");
            }
            if player.status().state == PlaybackState::Failed {
                let reason = player.status().last_error.unwrap_or_default();
                player.stop();
                return Err(reason);
            }
        }
    }
    let final_state = player.status();
    player.stop();
    if final_state.state == PlaybackState::Failed {
        return Err(final_state
            .last_error
            .unwrap_or_else(|| "Radio playback failed".into()));
    }
    if duration_seconds.is_some() && !played_audio {
        return Err("No decoded PCM reached the audio callback during the demo".into());
    }
    println!("RUST-BOY RADIO / STOPPED");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recovery_controller() -> (Arc<Mutex<RadioController>>, u64) {
        let mut controller = RadioController::default();
        let generation = controller
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        (Arc::new(Mutex::new(controller)), generation)
    }

    #[test]
    fn transient_startup_failures_exhaust_exactly_three_retries() {
        let (controller, generation) = recovery_controller();
        let cancelled = AtomicBool::new(false);
        let mut attempts = 0;
        let mut delays = Vec::new();
        let error = recover_attempts(
            generation,
            &controller,
            &cancelled,
            || {
                attempts += 1;
                Err(TransportError::Network(format!("connect timeout {attempts}")).into())
            },
            |delay| {
                assert_eq!(
                    controller.lock().unwrap().state().state,
                    PlaybackState::Recovering
                );
                delays.push(delay.as_secs());
                true
            },
        )
        .unwrap_err();
        assert_eq!(attempts, 4);
        assert_eq!(delays, [1, 2, 4]);
        assert_eq!(error.message(), "connect timeout 4");
    }

    #[test]
    fn temporary_startup_failure_can_recover_to_playing() {
        let (controller, generation) = recovery_controller();
        let cancelled = AtomicBool::new(false);
        let mut attempts = 0;
        recover_attempts(
            generation,
            &controller,
            &cancelled,
            || {
                attempts += 1;
                if attempts == 1 {
                    return Err(TransportError::Network("HTTP 503".into()).into());
                }
                let mut controller = controller.lock().unwrap();
                assert!(controller.event(generation, PlaybackEvent::Connected));
                assert!(controller.event(generation, PlaybackEvent::Buffered));
                assert!(controller.event(generation, PlaybackEvent::PlaybackStarted));
                Ok(())
            },
            |_| true,
        )
        .unwrap();
        assert_eq!(attempts, 2);
        assert_eq!(
            controller.lock().unwrap().state().state,
            PlaybackState::Playing
        );
    }

    #[test]
    fn stop_during_retry_prevents_another_attempt_and_stale_events() {
        let (controller, generation) = recovery_controller();
        let cancelled = AtomicBool::new(false);
        let mut attempts = 0;
        recover_attempts(
            generation,
            &controller,
            &cancelled,
            || {
                attempts += 1;
                Err(TransportError::Network("connection refused".into()).into())
            },
            |_| {
                cancelled.store(true, Ordering::Release);
                controller
                    .lock()
                    .unwrap()
                    .apply(RadioCommand::Stop)
                    .unwrap();
                false
            },
        )
        .unwrap();
        assert_eq!(attempts, 1);
        let mut controller = controller.lock().unwrap();
        assert_eq!(controller.state().state, PlaybackState::Stopped);
        assert!(!controller.event(generation, PlaybackEvent::Connected));
    }

    #[test]
    fn rejected_streams_and_device_failures_do_not_enter_recovery() {
        for error in [
            PlaybackFailure::from(TransportError::Permanent("HTTP 403".into())),
            PlaybackFailure::Output("speaker unavailable".into()),
            probe_failure(DecodeError::Unsupported("unsupported codec")),
        ] {
            let (controller, generation) = recovery_controller();
            let mut error = Some(error);
            let result = recover_attempts(
                generation,
                &controller,
                &AtomicBool::new(false),
                || Err(error.take().expect("Permanent failures must not retry")),
                |_| panic!("Permanent failures must not wait to retry"),
            );
            assert!(result.is_err());
        }
        assert_eq!(
            probe_failure(DecodeError::IoError(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "network disconnected during probe",
            )))
            .class(),
            FailureClass::NetworkTransient
        );
    }

    fn completed_test_engine(panics: bool) -> NativeRadioEngine {
        let mut controller = RadioController::default();
        controller
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        NativeRadioEngine {
            controller: Arc::new(Mutex::new(controller)),
            cancelled: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU8::new(60)),
            output_failed: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "nrsp-native")]
            native_control: None,
            #[cfg(feature = "sdrconnect")]
            receiver: Arc::new(Mutex::new(None)),
            worker: Some(thread::spawn(move || {
                assert!(!panics, "Synthetic playback worker panic");
            })),
        }
    }

    fn finish_test_engine(engine: &mut NativeRadioEngine) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !engine.try_finish() {
            assert!(Instant::now() < deadline, "Synthetic worker did not finish");
            thread::yield_now();
        }
    }

    #[test]
    fn unexpected_worker_exit_is_reported_and_reaping_is_idempotent() {
        for panics in [false, true] {
            let mut engine = completed_test_engine(panics);
            finish_test_engine(&mut engine);
            let status = engine.status();
            assert_eq!(status.state, PlaybackState::Failed);
            assert!(status.last_error.unwrap().contains("unexpectedly"));
            assert!(engine.try_finish());
            assert!(engine.worker.is_none());
        }
    }

    #[test]
    fn worker_cleanup_preserves_existing_failure_and_explicit_stop() {
        let mut engine = completed_test_engine(false);
        let generation = engine.status().generation;
        engine.controller.lock().unwrap().event(
            generation,
            PlaybackEvent::Error("Selected speaker disconnected".into()),
        );
        finish_test_engine(&mut engine);
        assert_eq!(
            engine.status().last_error.as_deref(),
            Some("Selected speaker disconnected")
        );
        let mut engine = completed_test_engine(true);
        engine.request_stop();
        finish_test_engine(&mut engine);
        assert_eq!(engine.status().state, PlaybackState::Stopped);
        assert!(engine.status().last_error.is_none());
    }

    #[test]
    fn stop_request_does_not_wait_for_worker_and_rejects_late_events() {
        let mut controller = RadioController::default();
        let generation = controller
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        let controller = Arc::new(Mutex::new(controller));
        let (release, pending) = std::sync::mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = pending.recv_timeout(Duration::from_secs(5));
        });
        let mut engine = NativeRadioEngine {
            controller: Arc::clone(&controller),
            cancelled: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU8::new(60)),
            output_failed: Arc::new(AtomicBool::new(false)),
            worker: Some(worker),
            #[cfg(feature = "nrsp-native")]
            native_control: None,
            #[cfg(feature = "sdrconnect")]
            receiver: Arc::new(Mutex::new(None)),
        };
        engine.request_stop();
        assert!(engine.cancelled.load(Ordering::Acquire));
        assert!(engine.shutdown_pending());
        assert_eq!(engine.volume.load(Ordering::Acquire), 0);
        engine.set_volume(80).unwrap();
        assert_eq!(engine.volume.load(Ordering::Acquire), 0);
        assert_eq!(engine.status().state, PlaybackState::Stopped);
        assert!(!engine.try_finish());
        assert!(!controller
            .lock()
            .unwrap()
            .event(generation, PlaybackEvent::Connected));
        let stopped_generation = engine.status().generation;
        engine.request_stop();
        assert_eq!(engine.status().generation, stopped_generation);
        release.send(()).unwrap();
        engine.stop();
        assert!(!engine.shutdown_pending());
        assert!(engine.try_finish());
        assert!(engine.worker.is_none());
    }

    #[test]
    fn device_failures_never_consume_the_network_retry_budget() {
        let output = PlaybackFailure::Output("Bluetooth output lost".into());
        assert_eq!(output.class(), FailureClass::OutputUnavailable);
        assert_eq!(
            decide(output.class(), 0, false),
            RecoveryAction::SelectOutput
        );
        let network = PlaybackFailure::Network("stream ended".into());
        assert_eq!(network.class(), FailureClass::NetworkTransient);
        assert!(matches!(
            decide(network.class(), 0, false),
            RecoveryAction::RetryAfter(_)
        ));
        assert_eq!(
            PlaybackFailure::Permanent("unknown codec".into()).class(),
            FailureClass::Permanent
        );
    }

    #[cfg(feature = "nrsp-native")]
    #[test]
    fn fm_controls_follow_the_latest_tuning_mode_without_stopping_audio() {
        let mut engine = completed_test_engine(false);
        let mut controller = RadioController::default();
        let generation = controller.apply(RadioCommand::PlaySdr).unwrap();
        for event in [
            PlaybackEvent::Connected,
            PlaybackEvent::Buffered,
            PlaybackEvent::PlaybackStarted,
        ] {
            assert!(controller.event(generation, event));
        }
        engine.controller = Arc::new(Mutex::new(controller));
        engine.native_control = Some(Arc::new(nrsp_native::Control::default()));
        engine.tune_nrsp_native(99_100_000, "nfm", 12_000).unwrap();
        assert!(engine
            .set_nrsp_native_fm(NativeFmSettings::default())
            .is_err());
        assert_eq!(engine.status().state, PlaybackState::Playing);
        engine.tune_nrsp_native(99_100_000, "wfm", 200_000).unwrap();
        engine
            .set_nrsp_native_fm(NativeFmSettings::default())
            .unwrap();
        engine.request_stop();
        assert!(engine
            .set_nrsp_native_fm(NativeFmSettings::default())
            .is_err());
    }

    #[test]
    fn refuses_unapproved_stream_without_opening_audio() {
        assert!(NativeRadioEngine::start_internet("untrusted", 50, "system").is_err());
        assert!(NativeRadioEngine::start_internet("skylab", 101, "system").is_err());
    }

    #[test]
    fn invalid_duration_is_rejected_before_device_or_network_access() {
        assert!(validate_demo_duration(Some(0)).is_err());
        assert!(validate_demo_duration(Some(301)).is_err());
        assert!(validate_demo_duration(Some(30)).is_ok());
        assert!(validate_demo_duration(None).is_ok());
        assert!(run_demo("skylab", Some(0)).is_err());
    }

    #[test]
    fn retry_wait_can_be_cancelled_before_next_connection() {
        let cancelled = AtomicBool::new(true);
        assert!(!interruptible_wait(&cancelled, Duration::from_secs(4)));
        let cancelled = AtomicBool::new(false);
        assert!(interruptible_wait(&cancelled, Duration::ZERO));
    }
}
