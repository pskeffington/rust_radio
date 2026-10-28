//! Silent, bounded station validation using the same MP3/AAC decoder as playback.
//! These synchronous checks belong on a host worker, never the GUI render thread.

use crate::media::{AdtsCompat, LiveMediaSource};
use rust_radio_core::directory::station_by_id;
use rust_radio_core::stream::{open_station, RadioHttp, StreamError};
use rust_radio_core::validation::{StationValidation, ValidationStage, ValidationStatus};
use rust_radio_ip::probe::{probe_was_cancelled, NativeRadioHttp};
use std::io::{self, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::SystemTime;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const MAX_VALIDATION_BYTES: u64 = 256 * 1024;
const MAX_VALIDATION_PACKETS: usize = 64;
const MAX_FRAME_CAPACITY: usize = 192_000;

/// Explicit network check, with one 18-second HTTP deadline and no retries.
/// No audio output device is enumerated or opened. Unknown station IDs fail first.
pub fn validate_station(station_id: &str) -> Result<StationValidation, String> {
    validate_station_cancellable(station_id, Arc::new(AtomicBool::new(false)))
}

fn validate_station_cancellable(
    station_id: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<StationValidation, String> {
    require_station(station_id)?;
    let client = match NativeRadioHttp::for_cancellable_probe(cancelled) {
        Ok(client) => client,
        Err(error) => return Ok(transport_failure(station_id, error)),
    };
    validate_with(&client, station_id)
}

/// One background check owned by a host. Poll `try_finish` to reap it without
/// blocking. After cancellation retain the task until that method returns Some;
/// an in-flight blocking request may need the remaining 18-second deadline.
/// Final Drop cancels and joins synchronously, as with the playback engine.
pub struct StationValidationTask {
    station_id: String,
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<StationValidation, String>>>,
}

impl StationValidationTask {
    pub fn start(station_id: &str) -> Result<Self, String> {
        require_station(station_id)?;
        let id = station_id.to_owned();
        Self::spawn(station_id, move |cancelled| {
            validate_station_cancellable(&id, cancelled)
        })
    }

    /// Check the actual playback transport without opening an audio output.
    /// Custom URLs are omitted from reports; cancellation polls pending IO.
    pub fn start_input(target: &str, direct: bool) -> Result<Self, String> {
        if direct {
            rust_radio_ip::net::validate_direct_url(target)?;
        } else {
            require_station(target)?;
        }
        let id = if direct { "custom" } else { target };
        let target = target.to_owned();
        Self::spawn(id, move |cancelled| {
            let input = if direct {
                rust_radio_ip::net::open_direct_classified(&target, cancelled)
            } else {
                rust_radio_ip::net::open_station_classified(&target, cancelled)
            };
            let mut report = StationValidation::unverified(if direct { "custom" } else { &target });
            match input {
                Ok((stream, _guard)) => {
                    report.stage = ValidationStage::StreamResponse;
                    report.content_type = stream.content_type;
                    Ok(decode_response(report, stream.reader))
                }
                Err(error) => {
                    let status = match &error {
                        rust_radio_ip::error::TransportError::Network(_) => {
                            ValidationStatus::Unavailable
                        }
                        rust_radio_ip::error::TransportError::Permanent(_) => {
                            ValidationStatus::InvalidStream
                        }
                        rust_radio_ip::error::TransportError::Cancelled => {
                            ValidationStatus::Cancelled
                        }
                    };
                    Ok(finish(report, status, error.to_string()))
                }
            }
        })
    }

    /// Background counterpart to `validate_with`. Custom transports must enforce
    /// finite I/O deadlines; cancelled results are discarded when reaped.
    pub fn with_transport<C: RadioHttp + Send + 'static>(
        client: C,
        station_id: &str,
    ) -> Result<Self, String>
    where
        C::Body: Send + 'static,
    {
        require_station(station_id)?;
        let id = station_id.to_owned();
        Self::spawn(station_id, move |_| validate_with(&client, &id))
    }

    fn spawn(
        station_id: &str,
        check: impl FnOnce(Arc<AtomicBool>) -> Result<StationValidation, String> + Send + 'static,
    ) -> Result<Self, String> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker = thread::Builder::new()
            .name("rust-radio-validation".into())
            .spawn(move || check(worker_cancelled))
            .map_err(|error| format!("Unable to start station validation: {error}"))?;
        Ok(Self {
            station_id: station_id.into(),
            cancelled,
            worker: Some(worker),
        })
    }

    pub fn station_id(&self) -> &str {
        &self.station_id
    }

    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn cancellation_requested(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Returns a result once; None means pending or already reaped. A panic is
    /// reported as an error, and a cancelled task never returns decoded success.
    pub fn try_finish(&mut self) -> Option<Result<StationValidation, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Station validation worker failed unexpectedly".into()));
        if self.cancellation_requested() {
            return Some(Ok(finish(
                StationValidation::unverified(&self.station_id),
                ValidationStatus::Cancelled,
                "Station check cancelled",
            )));
        }
        Some(result.and_then(|report| {
            if report.station_id == self.station_id {
                Ok(report)
            } else {
                Err("Station validation returned a different station".into())
            }
        }))
    }
}

impl Drop for StationValidationTask {
    fn drop(&mut self) {
        self.request_cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Validate through a supplied transport, useful for deterministic fixtures.
/// The transport must enforce I/O deadlines as required by `RadioHttp`.
pub fn validate_with<C: RadioHttp>(
    client: &C,
    station_id: &str,
) -> Result<StationValidation, String>
where
    C::Body: Send + 'static,
{
    require_station(station_id)?;
    let stream = match open_station(client, station_id) {
        Ok(stream) => stream,
        Err(error) => return Ok(transport_failure(station_id, error)),
    };
    let mut report = StationValidation::unverified(station_id);
    report.stage = ValidationStage::StreamResponse;
    report.resolved_url = Some(stream.endpoint.url().into());
    report.content_type = stream.content_type;
    Ok(decode_response(report, stream.reader))
}

fn require_station(station_id: &str) -> Result<(), String> {
    station_by_id(station_id)
        .map(|_| ())
        .ok_or_else(|| "Unknown curated internet radio station".into())
}

fn finish(
    mut report: StationValidation,
    status: ValidationStatus,
    detail: impl Into<String>,
) -> StationValidation {
    report.checked_at = Some(SystemTime::now());
    report.status = status;
    report.detail = detail.into();
    report
}

fn transport_failure(station_id: &str, error: StreamError) -> StationValidation {
    let mut report = StationValidation::unverified(station_id);
    let status = match &error {
        StreamError::Io(error) if probe_was_cancelled(error) => ValidationStatus::Cancelled,
        StreamError::TransportUnavailable | StreamError::Io(_) => ValidationStatus::Unavailable,
        StreamError::HttpStatus(status) => {
            report.stage = ValidationStage::EndpointReached;
            if matches!(status, 408 | 425 | 429 | 500..=599) {
                ValidationStatus::Unavailable
            } else {
                ValidationStatus::InvalidStream
            }
        }
        StreamError::UnsupportedHls => {
            report.stage = ValidationStage::EndpointReached;
            ValidationStatus::Unsupported
        }
        _ => {
            report.stage = ValidationStage::EndpointReached;
            ValidationStatus::InvalidStream
        }
    };
    finish(report, status, error.to_string())
}

fn decode_response<R: Read + Send + 'static>(
    mut report: StationValidation,
    mut reader: R,
) -> StationValidation {
    let mut first_byte = [0; 1];
    match reader.read(&mut first_byte) {
        Ok(0) => {
            return finish(
                report,
                ValidationStatus::InvalidStream,
                "Audio response was empty",
            )
        }
        Err(error) => return decode_failure(report, DecodeError::IoError(error)),
        Ok(_) => {}
    }
    let mut hint = Hint::new();
    if let Some(mime) = &report.content_type {
        hint.mime_type(mime.split(';').next().unwrap_or(mime).trim());
    }
    let reader = io::Cursor::new(first_byte)
        .chain(reader)
        .take(MAX_VALIDATION_BYTES);
    let media = if report
        .content_type
        .as_deref()
        .is_some_and(|mime| mime.contains("aac"))
    {
        MediaSourceStream::new(
            Box::new(LiveMediaSource::new(AdtsCompat::new(reader))),
            Default::default(),
        )
    } else {
        MediaSourceStream::new(Box::new(LiveMediaSource::new(reader)), Default::default())
    };
    let probed = match symphonia::default::get_probe().format(
        &hint,
        media,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(probed) => probed,
        Err(error) => return decode_failure(report, error),
    };
    let mut format = probed.format;
    let Some(track) = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
    else {
        return finish(
            report,
            ValidationStatus::Unsupported,
            "No audio track found",
        );
    };
    let track_id = track.id;
    let registry = symphonia::default::get_codecs();
    let mut decoder = match registry.make(&track.codec_params, &DecoderOptions::default()) {
        Ok(decoder) => decoder,
        Err(error) => return decode_failure(report, error),
    };
    report.stage = ValidationStage::SupportedCodec;
    report.codec = registry
        .get_codec(track.codec_params.codec)
        .map(|codec| codec.short_name.to_owned());
    for _ in 0..MAX_VALIDATION_PACKETS {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(error) => return decode_failure(report, error),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(DecodeError::DecodeError(_)) => continue,
            Err(error) => return decode_failure(report, error),
        };
        let spec = *decoded.spec();
        if decoded.capacity() > MAX_FRAME_CAPACITY
            || !(1..=192_000).contains(&spec.rate)
            || !(1..=8).contains(&spec.channels.count())
        {
            return finish(
                report,
                ValidationStatus::Unsupported,
                "Decoded audio exceeds supported limits",
            );
        }
        if decoded.frames() == 0 {
            continue;
        }
        report.decoded_frames = decoded.frames();
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        samples.copy_interleaved_ref(decoded);
        if !samples.samples().iter().all(|sample| sample.is_finite()) {
            report.decoded_frames = 0;
            return finish(
                report,
                ValidationStatus::InvalidStream,
                "Decoded audio contains invalid samples",
            );
        }
        report.stage = ValidationStage::DecodedAudio;
        report.sample_rate = Some(spec.rate);
        report.channels = Some(spec.channels.count());
        return finish(
            report,
            ValidationStatus::Decoded,
            "Decoded PCM verified; no audio output was opened",
        );
    }
    finish(
        report,
        ValidationStatus::InvalidStream,
        "No PCM decoded within the 64-packet budget",
    )
}

fn decode_failure(report: StationValidation, error: DecodeError) -> StationValidation {
    let status = match &error {
        DecodeError::IoError(error) if probe_was_cancelled(error) => ValidationStatus::Cancelled,
        DecodeError::Unsupported(_) => ValidationStatus::Unsupported,
        DecodeError::IoError(error) if error.kind() != io::ErrorKind::UnexpectedEof => {
            ValidationStatus::Unavailable
        }
        _ => ValidationStatus::InvalidStream,
    };
    finish(report, status, format!("Audio validation failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };
    use std::time::{Duration, Instant};

    fn finish_task(task: &mut StationValidationTask) -> Result<StationValidation, String> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(result) = task.try_finish() {
                return result;
            }
            assert!(Instant::now() < deadline, "Validation task did not finish");
            thread::yield_now();
        }
    }

    #[test]
    fn background_worker_panics_and_wrong_station_results_are_reported_once() {
        let mut panicked =
            StationValidationTask::spawn("skylab", |_| panic!("synthetic failure")).unwrap();
        assert!(finish_task(&mut panicked)
            .unwrap_err()
            .contains("unexpectedly"));
        assert!(panicked.try_finish().is_none());
        let mut wrong_station = StationValidationTask::spawn("skylab", |_| {
            Ok(StationValidation::unverified("groovesalad"))
        })
        .unwrap();
        assert!(finish_task(&mut wrong_station)
            .unwrap_err()
            .contains("different station"));
        assert!(wrong_station.try_finish().is_none());
        assert!(StationValidationTask::start("invalid-station").is_err());
    }

    #[test]
    fn cancellation_takes_priority_over_a_late_success_without_blocking_poll() {
        let (release, pending) = std::sync::mpsc::channel();
        let mut task = StationValidationTask::spawn("skylab", move |_| {
            pending.recv_timeout(Duration::from_secs(3)).unwrap();
            Ok(finish(
                StationValidation::unverified("skylab"),
                ValidationStatus::Decoded,
                "late success",
            ))
        })
        .unwrap();
        task.request_cancel();
        assert!(task.try_finish().is_none());
        task.request_cancel();
        release.send(()).unwrap();
        let report = finish_task(&mut task).unwrap();
        assert_eq!(report.status, ValidationStatus::Cancelled);
        assert!(!report.is_decoded());
        assert!(task.try_finish().is_none());
    }

    struct CountedReader {
        source: io::Cursor<Vec<u8>>,
        bytes: Arc<AtomicU64>,
    }

    impl Read for CountedReader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let count = self.source.read(out)?;
            self.bytes.fetch_add(count as u64, Ordering::Relaxed);
            Ok(count)
        }
    }

    #[test]
    fn oversized_metadata_cannot_exceed_the_validation_byte_budget() {
        let mut body = vec![0; 1024 * 1024];
        // ID3v2 declares 512 KiB of metadata, exceeding the entire probe budget.
        body[..10].copy_from_slice(b"ID3\x04\x00\x00\x00\x20\x00\x00");
        let bytes = Arc::new(AtomicU64::new(0));
        let report = decode_response(
            StationValidation::unverified("skylab"),
            CountedReader {
                source: io::Cursor::new(body),
                bytes: Arc::clone(&bytes),
            },
        );
        assert!(!report.is_decoded());
        assert!(bytes.load(Ordering::Relaxed) <= MAX_VALIDATION_BYTES);
        assert!(report.checked_at.is_some());
    }
}
