//! In-process IP-radio playback worker using native Rust decoding and CPAL.
//! No subprocesses, browser playback, persistent audio files or RF transmit.

use crate::compat_controller::{
    PlaybackEvent, PlaybackState, RadioCommand, RadioController, RadioSession,
};
use crate::output::NativeAudioOutput;
use rust_radio_core::directory::station_by_id;
use rust_radio_core::pcm::PcmConverter;
use rust_radio_core::recovery::{decide, FailureClass, RecoveryAction};
use rust_radio_ip::net as radio_net;
use std::io::{self, Read, Seek, SeekFrom};
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
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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

/// No direct audio output or network access occurs until the caller issues
/// an explicit PlayInternet command. One engine owns one active worker.
pub struct NativeRadioEngine {
    controller: Arc<Mutex<RadioController>>,
    cancelled: Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    output_failed: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
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
        if volume_percent > 100 {
            return Err("Volume must be between 0 and 100".into());
        }
        let mut controller = RadioController::default();
        controller
            .apply(RadioCommand::SetVolume(volume_percent))
            .map_err(|e| format!("{e:?}"))?;
        let generation = controller
            .apply(RadioCommand::PlayInternet {
                station_id: station_id.to_owned(),
            })
            .map_err(|e| format!("{e:?}"))?;
        let controller = Arc::new(Mutex::new(controller));
        let cancelled = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(AtomicU8::new(volume_percent));
        let worker_controller = Arc::clone(&controller);
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_volume = Arc::clone(&volume);
        let output_failed = Arc::new(AtomicBool::new(false));
        let worker_output_failed = Arc::clone(&output_failed);
        let id = station_id.to_owned();
        let selected_output = output_id.to_owned();
        let worker = thread::Builder::new()
            .name("rust-radio-playback".into())
            .spawn(move || {
                let result = play_with_recovery(
                    &id,
                    generation,
                    &worker_controller,
                    &worker_cancelled,
                    worker_volume,
                    &selected_output,
                );
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
        })
    }

    /// An audio-device failure is never treated as transient network loss.
    /// The HUD can offer an explicit output selection/retry.
    pub fn output_failed(&self) -> bool {
        self.output_failed.load(Ordering::Acquire)
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
    pub fn try_finish(&mut self) -> bool {
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return false;
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
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

/// A non-seekable, thread-safe media source around the R2 ICY reader.
/// Only the decoder worker locks it, never the real-time CPAL callback.
struct LiveMediaSource<R: Read + Send> {
    reader: Mutex<R>,
}

impl<R: Read + Send> LiveMediaSource<R> {
    fn new(reader: R) -> Self {
        Self {
            reader: Mutex::new(reader),
        }
    }
}

impl<R: Read + Send> Read for LiveMediaSource<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.reader
            .get_mut()
            .map_err(|_| io::Error::other("Audio source lock poisoned"))?
            .read(bytes)
    }
}

impl<R: Read + Send> Seek for LiveMediaSource<R> {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Live broadcasts are not seekable",
        ))
    }
}

impl<R: Read + Send> MediaSource for LiveMediaSource<R> {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

fn signal(controller: &Arc<Mutex<RadioController>>, generation: u64, event: PlaybackEvent) {
    controller
        .lock()
        .expect("radio state mutex poisoned")
        .event(generation, event);
}

/// Retry only a typed in-stream I/O failure. Device loss and invalid
/// codecs remain terminal; session Stop overrides any scheduled retry.
fn play_with_recovery(
    station_id: &str,
    generation: u64,
    controller: &Arc<Mutex<RadioController>>,
    cancelled: &Arc<AtomicBool>,
    volume: Arc<AtomicU8>,
    output_id: &str,
) -> Result<(), PlaybackFailure> {
    let mut attempted_retries = 0;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        let result = decode_and_play(
            station_id,
            generation,
            controller,
            cancelled,
            Arc::clone(&volume),
            output_id,
        );
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
                if !interruptible_wait(cancelled, delay) {
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
    let (stream, _network_guard) = radio_net::open_station(station_id, Arc::clone(cancelled))?;
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    signal(controller, generation, PlaybackEvent::Connected);

    let mut hint = Hint::new();
    let mime = stream.content_type.as_deref().unwrap_or_default();
    if mime.contains("aac") {
        hint.with_extension("aac");
    } else {
        hint.with_extension("mp3");
    }
    let source = LiveMediaSource::new(stream.reader);
    let media = MediaSourceStream::new(Box::new(source), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            media,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Unsupported radio stream format: {e}"))?;
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
    let mut started = false;
    let mut announced = false;

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
        let keep_going = converter
            .as_mut()
            .expect("PCM converter is initialized")
            .convert(samples.samples(), |frame| {
                output.push_frame(frame, cancelled)
            })
            .map_err(|e| format!("Radio PCM conversion failed: {e:?}"))?;
        if !keep_going {
            if output.has_failed() && !cancelled.load(Ordering::Acquire) {
                return Err(PlaybackFailure::Output(
                    "Native audio output stopped consuming samples".into(),
                ));
            }
            return Ok(());
        }

        if !started && output.buffered_samples() >= output.minimum_prebuffer_samples() {
            output.start().map_err(PlaybackFailure::Output)?;
            started = true;
            signal(controller, generation, PlaybackEvent::Buffered);
        }
        if started && !announced && output.played_samples() > 0 {
            signal(controller, generation, PlaybackEvent::PlaybackStarted);
            announced = true;
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
        };
        engine.request_stop();
        assert!(engine.cancelled.load(Ordering::Acquire));
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

    #[test]
    fn live_media_source_does_not_offer_seeking() {
        let mut source = LiveMediaSource::new(io::Cursor::new(vec![1u8, 2, 3]));
        let mut bytes = [0u8; 2];
        assert_eq!(source.read(&mut bytes).unwrap(), 2);
        assert_eq!(bytes, [1, 2]);
        assert!(!source.is_seekable());
        assert!(source.seek(SeekFrom::Start(0)).is_err());
    }
}
