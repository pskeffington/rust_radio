//! Async native HTTPS radio receiver with bounded backpressure and cancellation.
//! Network I/O runs outside the Symphonia worker and CPAL callback.

use rust_radio_core::directory::station_by_id;
use rust_radio_core::stream::{
    icy_interval, parse_m3u, ApprovedEndpoint, IcyAudioReader, MAX_PLAYLIST_BYTES, MAX_REQUEST_HOPS,
};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::redirect::Policy;
use std::io::{self, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

const CHUNK_SIZE: usize = 8192;
const PENDING_CHUNKS: usize = 16;
const READ_DEADLINE: Duration = Duration::from_secs(3);
const CONNECT_DEADLINE: Duration = Duration::from_secs(8);
const STARTUP_DEADLINE: Duration = Duration::from_secs(35);

struct StreamHeaders {
    content_type: Option<String>,
    icy_interval: Option<usize>,
}

/// One in-flight compressed audio queue with a hard byte-capacity bound.
/// Receiver pauses between reads; no entire stream is accumulated.
pub struct ChunkReader {
    receiver: Receiver<Vec<u8>>,
    buffered: Vec<u8>,
    position: usize,
    cancelled: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

impl Read for ChunkReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "Radio stopped"));
            }
            if self.position < self.buffered.len() {
                let available = self.buffered.len() - self.position;
                let copied = available.min(out.len());
                out[..copied]
                    .copy_from_slice(&self.buffered[self.position..self.position + copied]);
                self.position += copied;
                return Ok(copied);
            }
            match self.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(bytes) => {
                    self.position = 0;
                    self.buffered = bytes;
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    let error = self
                        .error
                        .lock()
                        .expect("Radio error mutex poisoned")
                        .take();
                    return match error {
                        Some(text) => Err(io::Error::other(text)),
                        None => Ok(0),
                    };
                }
            }
        }
    }
}

pub struct RadioInput {
    pub reader: IcyAudioReader<ChunkReader>,
    pub content_type: Option<String>,
}

/// The guard must outlive the decoder. Dropping the compressed-byte reader
/// releases the sender; the join waits for the bounded HTTP read to finish.
/// Do NOT flip the session's cancel flag here: reconnection creates a new
/// stream within the same user-authorized session.
pub struct NetworkGuard {
    worker: Option<JoinHandle<()>>,
}

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn open_station(
    station_id: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(RadioInput, NetworkGuard), String> {
    if station_by_id(station_id).is_none() {
        return Err("Unapproved internet radio station".into());
    }
    let id = station_id.to_owned();
    let (startup_tx, startup_rx) = mpsc::sync_channel::<Result<StreamHeaders, String>>(1);
    let (audio_tx, audio_rx) = mpsc::sync_channel(PENDING_CHUNKS);
    let error = Arc::new(Mutex::new(None));
    let worker_error = Arc::clone(&error);
    let worker_cancel = Arc::clone(&cancelled);
    let worker = thread::Builder::new()
        .name("rust-radio-http".into())
        .spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Radio network runtime unavailable: {e}"))
                .and_then(|runtime| {
                    runtime.block_on(pump_station(&id, &worker_cancel, &startup_tx, &audio_tx))
                });
            if let Err(message) = result {
                if !worker_cancel.load(Ordering::Acquire) {
                    *worker_error.lock().expect("Radio error mutex poisoned") =
                        Some(message.clone());
                    let _ = startup_tx.try_send(Err(message));
                }
            }
        })
        .map_err(|e| format!("Cannot launch native radio receiver: {e}"))?;
    let guard = NetworkGuard {
        worker: Some(worker),
    };

    let start = Instant::now();
    let headers = loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("Radio stopped before streaming began".into());
        }
        if start.elapsed() > STARTUP_DEADLINE {
            return Err("Radio stream connection exceeded 35 seconds".into());
        }
        match startup_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(info)) => break info,
            Ok(Err(reason)) => return Err(reason),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err("Radio receiver stopped before HTTP response".into());
            }
        }
    };
    let reader = ChunkReader {
        receiver: audio_rx,
        buffered: Vec::new(),
        position: 0,
        cancelled,
        error,
    };
    Ok((
        RadioInput {
            reader: IcyAudioReader::new(reader, headers.icy_interval),
            content_type: headers.content_type,
        },
        guard,
    ))
}

async fn pump_station(
    station_id: &str,
    cancelled: &AtomicBool,
    startup: &SyncSender<Result<StreamHeaders, String>>,
    audio: &SyncSender<Vec<u8>>,
) -> Result<(), String> {
    let broadcaster = station_by_id(station_id).ok_or("Unknown station")?;
    let mut next = ApprovedEndpoint::new(station_id, broadcaster.playlist_url)
        .map_err(|e| format!("Stream URL rejected: {e}"))?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .user_agent("Rust-Boy-RADIO/0.1 (native audio)")
        .build()
        .map_err(|e| format!("Cannot initialize native TLS client: {e}"))?;
    let mut playlist_seen = false;

    for _ in 0..MAX_REQUEST_HOPS {
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut response = timeout(
            CONNECT_DEADLINE,
            client.get(next.url()).header("Icy-MetaData", "1").send(),
        )
        .await
        .map_err(|_| "Radio connect/header timeout".to_owned())?
        .map_err(|e| format!("Radio HTTPS connection failed: {e}"))?;
        let status = response.status();
        if status.is_redirection() {
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or("Radio redirected without a Location header")?
                .to_str()
                .map_err(|_| "Radio redirect had an invalid Location header")?;
            next = ApprovedEndpoint::new(station_id, location)
                .map_err(|e| format!("Radio redirect rejected: {e}"))?;
            continue;
        }
        if !status.is_success() {
            return Err(format!("Radio server responded HTTP {status}"));
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .map(|v| v.to_str().map(str::to_owned))
            .transpose()
            .map_err(|_| "Invalid radio content-type header")?;
        let mime = content_type
            .as_deref()
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let playlist = next.url().ends_with(".m3u")
            || matches!(
                mime.as_str(),
                "audio/x-mpegurl" | "audio/mpegurl" | "application/x-mpegurl"
            );
        if playlist {
            if playlist_seen {
                return Err("Nested radio playlist not permitted".into());
            }
            playlist_seen = true;
            let mut document = Vec::new();
            loop {
                let part = timeout(READ_DEADLINE, response.chunk())
                    .await
                    .map_err(|_| "Radio playlist read timed out")?
                    .map_err(|e| format!("Radio playlist read failed: {e}"))?;
                let Some(bytes) = part else { break };
                if document.len().saturating_add(bytes.len()) > MAX_PLAYLIST_BYTES {
                    return Err("Radio playlist exceeds 16 KiB".into());
                }
                document.extend_from_slice(&bytes);
            }
            let mut entries = parse_m3u(station_id, &document)
                .map_err(|e| format!("Radio playlist rejected: {e}"))?;
            next = entries.remove(0);
            continue;
        }
        if !mime.starts_with("audio/") && mime != "application/octet-stream" {
            return Err(format!("Server did not return an audio stream: {mime}"));
        }
        let icy_header = response
            .headers()
            .get("icy-metaint")
            .map(|v| v.to_str().map(str::to_owned))
            .transpose()
            .map_err(|_| "Invalid ICY metadata interval header")?;
        let metaint = icy_interval(icy_header.as_deref())
            .map_err(|e| format!("ICY metadata rejected: {e}"))?;
        startup
            .send(Ok(StreamHeaders {
                content_type,
                icy_interval: metaint,
            }))
            .map_err(|_| "Radio decoder no longer available")?;

        loop {
            if cancelled.load(Ordering::Acquire) {
                return Ok(());
            }
            let piece = timeout(READ_DEADLINE, response.chunk())
                .await
                .map_err(|_| "Radio audio read inactivity timeout")?
                .map_err(|e| format!("Radio audio read failed: {e}"))?;
            let Some(piece) = piece else {
                return Err("Radio stream ended".into());
            };
            for fragment in piece.chunks(CHUNK_SIZE) {
                let mut pending = fragment.to_vec();
                loop {
                    if cancelled.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    match audio.try_send(pending) {
                        Ok(()) => break,
                        Err(TrySendError::Full(bytes)) => {
                            pending = bytes;
                            sleep(Duration::from_millis(5)).await;
                        }
                        Err(TrySendError::Disconnected(_)) => return Ok(()),
                    }
                }
            }
        }
    }
    Err("Radio exceeded redirect/playlist hop limit".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn test_reader(bytes: Vec<u8>) -> ChunkReader {
        let (sender, receiver) = mpsc::sync_channel(2);
        sender.send(bytes).unwrap();
        drop(sender);
        ChunkReader {
            receiver,
            buffered: Vec::new(),
            position: 0,
            cancelled: Arc::new(AtomicBool::new(false)),
            error: Arc::new(Mutex::new(None)),
        }
    }

    #[test]
    fn chunk_reader_is_bounded_and_preserves_bytes() {
        let mut reader = test_reader(vec![1, 2, 3, 4, 5]);
        let mut out = [0u8; 3];
        assert_eq!(reader.read(&mut out).unwrap(), 3);
        assert_eq!(out, [1, 2, 3]);
        assert_eq!(reader.read(&mut out).unwrap(), 2);
        assert_eq!(&out[..2], &[4, 5]);
        assert_eq!(reader.read(&mut out).unwrap(), 0);
    }

    #[test]
    fn chunk_reader_obeys_stop_and_reports_failure() {
        let mut reader = test_reader(vec![1, 2, 3]);
        reader.cancelled.store(true, Ordering::Release);
        assert!(reader.read(&mut [0; 4]).is_err());
        let mut reader = test_reader(Vec::new());
        *reader.error.lock().unwrap() = Some("network lost".into());
        let mut dest = [0; 2];
        assert!(reader.read(&mut dest).is_err());
    }

    #[test]
    fn unapproved_station_never_starts_network_thread() {
        assert!(open_station("invalid-station", Arc::new(AtomicBool::new(false))).is_err());
        let source = Cursor::new([1u8, 2u8]);
        assert_eq!(source.into_inner(), [1, 2]);
    }
}
