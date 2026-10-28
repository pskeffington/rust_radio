//! Async native HTTPS radio receiver with bounded backpressure and cancellation.
//! Network I/O runs outside the Symphonia worker and CPAL callback.

use crate::error::TransportError;
use crate::safe_dns::{is_public_unicast, SafeResolver};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::redirect::Policy;
use rust_radio_core::directory::station_by_id;
use rust_radio_core::stream::{
    icy_interval, parse_m3u_with, parse_playlist, parse_pls_with, playlist_format,
    ApprovedEndpoint, IcyAudioReader, PlaylistFormat, StreamError, MAX_PLAYLIST_BYTES,
    MAX_REQUEST_HOPS,
};
use std::future::Future;
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
const CANCEL_INTERVAL: Duration = Duration::from_millis(25);

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
    stopped: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    startup: StartupDeadline,
}

impl Read for ChunkReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            if !self.startup.complete.load(Ordering::Acquire)
                && Instant::now() >= self.startup.until
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "No decoded audio within 35 seconds",
                ));
            }
            if self.cancelled.load(Ordering::Acquire) || self.stopped.load(Ordering::Acquire) {
                // read_exact retries Interrupted, which can spin forever while
                // parsing an ICY metadata block after Stop.
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Radio stopped",
                ));
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

impl Drop for ChunkReader {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}

/// Shared startup budget. The decoder completes it only after valid PCM exists.
#[derive(Clone)]
pub struct StartupDeadline {
    until: Instant,
    complete: Arc<AtomicBool>,
}
impl StartupDeadline {
    pub fn decoded(&self) {
        self.complete.store(true, Ordering::Release);
    }
}

pub struct RadioInput {
    pub reader: IcyAudioReader<ChunkReader>,
    pub content_type: Option<String>,
    pub startup: StartupDeadline,
}

/// Dropping the guard cancels this connection before joining its worker, even
/// if the decoder still holds its reader or the audio queue is full. This never
/// changes the session's cancel flag: recovery can open another connection.
pub struct NetworkGuard {
    worker: Option<JoinHandle<()>>,
    stopped: Arc<AtomicBool>,
}

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Keep the in-flight future across polling deadlines. Cancelling drops that
/// future, releasing pending HTTP requests, responses and backpressure waits.
async fn until_cancelled<E>(
    work: impl Future<Output = Result<(), E>>,
    cancelled: &AtomicBool,
    stopped: &AtomicBool,
) -> Result<(), E> {
    let mut work = std::pin::pin!(work);
    loop {
        if cancelled.load(Ordering::Acquire) || stopped.load(Ordering::Acquire) {
            return Ok(());
        }
        if let Ok(result) = timeout(CANCEL_INTERVAL, work.as_mut()).await {
            return result;
        }
    }
}

async fn send_fragment(
    audio: &SyncSender<Vec<u8>>,
    mut pending: Vec<u8>,
    cancelled: &AtomicBool,
) -> bool {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return false;
        }
        match audio.try_send(pending) {
            Ok(()) => return true,
            Err(TrySendError::Full(bytes)) => {
                pending = bytes;
                sleep(Duration::from_millis(5)).await;
            }
            Err(TrySendError::Disconnected(_)) => return false,
        }
    }
}

pub fn open_station(
    station_id: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(RadioInput, NetworkGuard), String> {
    open_station_classified(station_id, cancelled).map_err(|error| error.to_string())
}

/// Validate a user-entered direct audio URL without making a network request.
/// Plain HTTP is limited to literal private or loopback IPv4 addresses.
pub fn validate_direct_url(value: &str) -> Result<(), String> {
    if value.len() > 2048
        || value
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
    {
        return Err("Invalid stream URL".into());
    }
    let url = reqwest::Url::parse(value).map_err(|_| "Invalid stream URL")?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err("Stream URL must not contain credentials or a fragment".into());
    }
    match url.scheme() {
        "https" if url.host().is_some() => {
            let literal_ip = url.host_str().and_then(|host| {
                host.trim_start_matches('[')
                    .trim_end_matches(']')
                    .parse::<std::net::IpAddr>()
                    .ok()
            });
            if literal_ip.is_some_and(|address| !is_public_unicast(address)) {
                Err("HTTPS streams cannot target a non-public IP address".into())
            } else {
                Ok(())
            }
        }
        "http" => match url
            .host_str()
            .and_then(|host| host.parse::<std::net::Ipv4Addr>().ok())
        {
            Some(ip) if ip.is_private() || ip.is_loopback() => Ok(()),
            _ => Err("HTTP streams require a private or loopback IPv4 address".into()),
        },
        _ => Err("Use an HTTPS stream URL or a private IPv4 HTTP URL".into()),
    }
}

/// Start an explicitly entered audio stream or bounded M3U/PLS playlist.
pub fn open_direct_classified(
    url: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(RadioInput, NetworkGuard), TransportError> {
    validate_direct_url(url).map_err(TransportError::Permanent)?;
    open_input(url.to_owned(), true, cancelled)
}

/// Start an approved stream, preserving structured errors for recovery policy.
/// `open_station` remains available for hosts using its original string errors.
pub fn open_station_classified(
    station_id: &str,
    cancelled: Arc<AtomicBool>,
) -> Result<(RadioInput, NetworkGuard), TransportError> {
    if station_by_id(station_id).is_none() {
        return Err("Unapproved internet radio station".into());
    }
    open_input(station_id.to_owned(), false, cancelled)
}

fn open_input(
    id: String,
    direct: bool,
    cancelled: Arc<AtomicBool>,
) -> Result<(RadioInput, NetworkGuard), TransportError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(TransportError::Cancelled);
    }
    let (startup_tx, startup_rx) = mpsc::sync_channel::<Result<StreamHeaders, TransportError>>(1);
    let (audio_tx, audio_rx) = mpsc::sync_channel(PENDING_CHUNKS);
    let error = Arc::new(Mutex::new(None));
    let worker_error = Arc::clone(&error);
    let worker_cancel = Arc::clone(&cancelled);
    let stopped = Arc::new(AtomicBool::new(false));
    let worker_stopped = Arc::clone(&stopped);
    let worker = thread::Builder::new()
        .name("rust-radio-http".into())
        .spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    TransportError::Permanent(format!("Radio network runtime unavailable: {e}"))
                })
                .and_then(|runtime| {
                    runtime.block_on(until_cancelled(
                        pump_input(&id, direct, &worker_cancel, &startup_tx, &audio_tx),
                        &worker_cancel,
                        &worker_stopped,
                    ))
                });
            if let Err(message) = result {
                if !worker_cancel.load(Ordering::Acquire) && !worker_stopped.load(Ordering::Acquire)
                {
                    *worker_error.lock().expect("Radio error mutex poisoned") =
                        Some(message.to_string());
                    let _ = startup_tx.try_send(Err(message));
                }
            }
        })
        .map_err(|e| format!("Cannot launch native radio receiver: {e}"))?;
    let guard = NetworkGuard {
        worker: Some(worker),
        stopped: Arc::clone(&stopped),
    };

    let start = Instant::now();
    let startup = StartupDeadline {
        until: start + STARTUP_DEADLINE,
        complete: Arc::new(AtomicBool::new(false)),
    };
    let headers = loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(TransportError::Cancelled);
        }
        if start.elapsed() > STARTUP_DEADLINE {
            return Err(TransportError::Network(
                "Radio stream connection exceeded 35 seconds".into(),
            ));
        }
        match startup_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(info)) => break info,
            Ok(Err(reason)) => return Err(reason),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err(if cancelled.load(Ordering::Acquire) {
                    TransportError::Cancelled
                } else {
                    TransportError::Network("Radio receiver stopped before HTTP response".into())
                });
            }
        }
    };
    let reader = ChunkReader {
        receiver: audio_rx,
        buffered: Vec::new(),
        position: 0,
        cancelled,
        stopped,
        error,
        startup: startup.clone(),
    };
    Ok((
        RadioInput {
            reader: IcyAudioReader::new(reader, headers.icy_interval),
            content_type: headers.content_type,
            startup,
        },
        guard,
    ))
}

async fn pump_input(
    station_id: &str,
    direct: bool,
    cancelled: &AtomicBool,
    startup: &SyncSender<Result<StreamHeaders, TransportError>>,
    audio: &SyncSender<Vec<u8>>,
) -> Result<(), TransportError> {
    let initial = if direct {
        station_id
    } else {
        station_by_id(station_id)
            .ok_or("Unknown station")?
            .playlist_url
    };
    let mut next = reqwest::Url::parse(initial).map_err(|_| "Invalid stream URL")?;
    check_hop(initial, &next, direct, station_id)?;
    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        // The private-LAN exception uses a user-entered IPv4 literal, which
        // bypasses DNS by definition. Every DNS lookup, including redirects
        // and playlist targets, must still return only public addresses.
        .dns_resolver(SafeResolver::public_only())
        .connect_timeout(Duration::from_secs(5))
        .user_agent("Rust-Radio/0.1 (native audio)")
        .build()
        .map_err(|_| "Cannot initialize radio HTTP client")?;
    let mut playlist_seen = false;

    for _ in 0..MAX_REQUEST_HOPS {
        if cancelled.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut response = timeout(
            CONNECT_DEADLINE,
            client.get(next.clone()).header("Icy-MetaData", "1").send(),
        )
        .await
        .map_err(|_| TransportError::Network("Radio connect/header timeout".to_owned()))?
        .map_err(|_| {
            TransportError::Network(
                "Radio stream connection failed (check address and network)".into(),
            )
        })?;
        let status = response.status();
        if status.is_redirection() {
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or("Radio redirected without a Location header")?
                .to_str()
                .map_err(|_| "Radio redirect had an invalid Location header")?;
            let resolved = next.join(location).map_err(|_| "Invalid redirect URL")?;
            check_hop(initial, &resolved, direct, station_id)?;
            next = resolved;
            continue;
        }
        if !status.is_success() {
            return Err(TransportError::http_status(status));
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
        if let Some(format) = playlist_format(next.as_str(), content_type.as_deref()) {
            if format == PlaylistFormat::Hls {
                return Err(TransportError::Permanent(
                    StreamError::UnsupportedHls.to_string(),
                ));
            }
            if playlist_seen {
                return Err("Nested radio playlist not permitted".into());
            }
            playlist_seen = true;
            let mut document = Vec::new();
            loop {
                let part = timeout(READ_DEADLINE, response.chunk())
                    .await
                    .map_err(|_| TransportError::Network("Radio playlist read timed out".into()))?
                    .map_err(|_| TransportError::Network("Radio playlist read failed".into()))?;
                let Some(bytes) = part else { break };
                if document.len().saturating_add(bytes.len()) > MAX_PLAYLIST_BYTES {
                    return Err("Radio playlist exceeds 16 KiB".into());
                }
                document.extend_from_slice(&bytes);
            }
            next = if direct {
                let resolve = |entry: &str| {
                    let url = next.join(entry).map_err(|_| StreamError::InvalidPlaylist)?;
                    check_hop(initial, &url, true, station_id)
                        .map_err(|_| StreamError::UnsafeEndpoint)?;
                    Ok(url)
                };
                let entries = match format {
                    PlaylistFormat::M3u => parse_m3u_with(&document, resolve),
                    PlaylistFormat::Pls => parse_pls_with(&document, resolve),
                    PlaylistFormat::Hls => unreachable!(),
                }
                .map_err(|e| format!("Radio playlist rejected: {e}"))?;
                entries.into_iter().next().ok_or("Empty radio playlist")?
            } else {
                let endpoint = parse_playlist(station_id, &document, format)
                    .map_err(|e| format!("Radio playlist rejected: {e}"))?
                    .into_iter()
                    .next()
                    .ok_or("Empty radio playlist")?;
                reqwest::Url::parse(endpoint.url()).map_err(|_| "Invalid playlist URL")?
            };
            continue;
        }
        if !mime.starts_with("audio/") && mime != "application/octet-stream" {
            return Err(TransportError::Permanent(format!(
                "Server did not return an audio stream: {mime}"
            )));
        }
        return pump_audio(response, content_type, cancelled, startup, audio).await;
    }
    Err("Radio exceeded redirect/playlist hop limit".into())
}

// Each indirection is checked again. Public inputs cannot downgrade to HTTP;
// the private HTTP exception remains on the exact user-selected origin.
fn check_hop(
    initial: &str,
    next: &reqwest::Url,
    direct: bool,
    station: &str,
) -> Result<(), String> {
    if direct {
        validate_direct_url(next.as_str())?;
        let initial_url = reqwest::Url::parse(initial).map_err(|_| "Invalid stream URL")?;
        let explicit_private_ip = if initial_url.scheme() == "http" {
            initial_url
                .host_str()
                .and_then(|host| host.parse::<std::net::Ipv4Addr>().ok())
                .filter(|ip| ip.is_private() || ip.is_loopback())
                .map(std::net::IpAddr::V4)
        } else {
            None
        };
        let target_ip = next.host_str().and_then(|host| {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .ok()
        });
        if target_ip.is_some_and(|address| !is_public_unicast(address)) {
            let same_private_http_origin = explicit_private_ip.is_some_and(|address| {
                Some(address) == target_ip
                    && next.scheme() == "http"
                    && next.port_or_known_default() == initial_url.port_or_known_default()
            });
            if !same_private_http_origin {
                return Err("Radio URL cannot target a non-routable address".into());
            }
        }
        if initial_url.scheme() == "https" && next.scheme() != "https" {
            return Err("Radio redirect cannot downgrade HTTPS to HTTP".into());
        }
    } else {
        ApprovedEndpoint::new(station, next.as_str())
            .map_err(|_| "Radio redirect is outside the broadcaster's approved endpoints")?;
    }
    Ok(())
}

async fn pump_audio(
    mut response: reqwest::Response,
    content_type: Option<String>,
    cancelled: &AtomicBool,
    startup: &SyncSender<Result<StreamHeaders, TransportError>>,
    audio: &SyncSender<Vec<u8>>,
) -> Result<(), TransportError> {
    let icy_header = response
        .headers()
        .get("icy-metaint")
        .map(|v| v.to_str().map(str::to_owned))
        .transpose()
        .map_err(|_| "Invalid ICY metadata interval header")?;
    let metaint =
        icy_interval(icy_header.as_deref()).map_err(|e| format!("ICY metadata rejected: {e}"))?;
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
            .map_err(|_| TransportError::Network("Radio audio read inactivity timeout".into()))?
            .map_err(|_| TransportError::Network("Radio audio read failed".into()))?;
        let Some(piece) = piece else {
            return Err(TransportError::Network("Radio stream ended".into()));
        };
        for fragment in piece.chunks(CHUNK_SIZE) {
            if !send_fragment(audio, fragment.to_vec(), cancelled).await {
                return Ok(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn startup_budget_rejects_even_buffered_bytes_until_pcm_is_decoded() {
        let mut reader = test_reader(vec![1, 2, 3]);
        reader.startup.until = Instant::now();
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        reader.startup.decoded();
        let mut byte = [0];
        assert_eq!(reader.read(&mut byte).unwrap(), 1);
        assert_eq!(byte, [1]);
    }

    #[test]
    fn redirect_policy_rechecks_credentials_and_blocks_https_downgrades() {
        for url in [
            "http://127.0.0.1/audio",
            "https://user:secret@example.com/audio",
            "file:///tmp/radio",
        ] {
            assert!(check_hop(
                "https://example.com/start",
                &reqwest::Url::parse(url).unwrap(),
                true,
                ""
            )
            .is_err());
        }
        assert!(check_hop(
            "http://192.168.1.12/listen",
            &reqwest::Url::parse("http://192.168.1.12/audio").unwrap(),
            true,
            ""
        )
        .is_ok());
        for target in [
            "http://192.168.1.13/audio",
            "http://192.168.1.12:8001/audio",
            "http://169.254.1.2/audio",
            "https://192.168.1.12/audio",
        ] {
            assert!(
                check_hop(
                    "http://192.168.1.12:80/listen",
                    &reqwest::Url::parse(target).unwrap(),
                    true,
                    ""
                )
                .is_err(),
                "{target}"
            );
        }
        for address in ["127.0.0.1", "192.168.1.5", "169.254.1.2", "[::1]"] {
            let target = reqwest::Url::parse(&format!("https://{address}/audio")).unwrap();
            assert!(
                check_hop("https://public.example/audio", &target, true, "").is_err(),
                "{address}"
            );
        }
        let public_ip = reqwest::Url::parse("https://8.8.8.8/audio").unwrap();
        assert!(check_hop("https://public.example/audio", &public_ip, true, "").is_ok());
    }

    #[test]
    fn direct_urls_require_explicit_safe_schemes_and_no_credentials() {
        for url in [
            "https://example.com/live?token=abc",
            "https://8.8.8.8/live",
            "http://127.0.0.1:8080/live",
            "http://192.168.1.5:8000/live",
        ] {
            assert!(validate_direct_url(url).is_ok());
        }
        for url in [
            "http://example.com/live",
            "http://169.254.169.254/",
            "https://127.0.0.1/live",
            "https://192.168.1.5/live",
            "https://[::1]/live",
            "https://[::ffff:127.0.0.1]/live",
            "https://100.64.0.1/live",
            "https://[fe80::1]/live",
            "https://user:pass@example.com/",
            "https://example.com/#fragment",
            "file:///tmp/audio",
            "not a url",
        ] {
            assert!(validate_direct_url(url).is_err());
        }
    }

    struct DropNotice(mpsc::Sender<()>);

    impl Drop for DropNotice {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    fn stalled_connection(
        cancelled: Arc<AtomicBool>,
        stopped: Arc<AtomicBool>,
    ) -> (NetworkGuard, Receiver<()>) {
        test_connection(std::future::pending(), cancelled, stopped)
    }

    fn test_connection(
        work: impl Future<Output = Result<(), String>> + Send + 'static,
        cancelled: Arc<AtomicBool>,
        stopped: Arc<AtomicBool>,
    ) -> (NetworkGuard, Receiver<()>) {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let worker_stopped = Arc::clone(&stopped);
        let worker = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime
                .block_on(until_cancelled(
                    async move {
                        let _notice = DropNotice(dropped_tx);
                        ready_tx.send(()).unwrap();
                        work.await
                    },
                    &cancelled,
                    &worker_stopped,
                ))
                .unwrap();
        });
        ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        (
            NetworkGuard {
                worker: Some(worker),
                stopped,
            },
            dropped_rx,
        )
    }

    #[test]
    fn guard_drop_releases_a_full_queue_even_while_the_receiver_is_retained() {
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.send(vec![1; CHUNK_SIZE]).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let (guard, dropped) = test_connection(
            async move {
                send_fragment(&sender, vec![2; CHUNK_SIZE], &worker_cancelled).await;
                Ok(())
            },
            Arc::clone(&cancelled),
            Arc::new(AtomicBool::new(false)),
        );
        let (joined_tx, joined_rx) = mpsc::channel();
        thread::spawn(move || {
            drop(guard);
            joined_tx.send(()).unwrap();
        });
        dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        joined_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(receiver.recv().unwrap(), vec![1; CHUNK_SIZE]);
        assert!(receiver.recv().is_err());
        assert!(!cancelled.load(Ordering::Acquire));
    }

    #[test]
    fn dropping_guard_cancels_pending_io_without_stopping_the_session() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        let (guard, dropped) = stalled_connection(Arc::clone(&cancelled), Arc::clone(&stopped));
        // Bound the regression test even if joining a stalled worker regresses.
        let (joined_tx, joined_rx) = mpsc::channel();
        thread::spawn(move || {
            drop(guard);
            joined_tx.send(()).unwrap();
        });
        dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        joined_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(stopped.load(Ordering::Acquire));
        assert!(!cancelled.load(Ordering::Acquire));

        // Recovery gets a fresh connection in the still-authorized session.
        let (guard, dropped) =
            stalled_connection(Arc::clone(&cancelled), Arc::new(AtomicBool::new(false)));
        cancelled.store(true, Ordering::Release);
        dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        drop(guard);
    }

    #[test]
    fn dropping_reader_cancels_connection_even_with_a_retained_guard() {
        let mut reader = test_reader(vec![1, 2, 3]);
        let (guard, dropped) =
            stalled_connection(Arc::clone(&reader.cancelled), Arc::clone(&reader.stopped));
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 1);
        drop(reader);
        dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        drop(guard);
    }

    #[test]
    fn guard_cancellation_rejects_buffered_bytes_with_the_reader_still_alive() {
        let mut reader = test_reader(vec![1, 2, 3]);
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 1);
        let (guard, dropped) =
            stalled_connection(Arc::clone(&reader.cancelled), Arc::clone(&reader.stopped));
        drop(guard);
        dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::ConnectionAborted
        );
    }

    #[test]
    fn stop_during_icy_metadata_does_not_retry_cancelled_reads_forever() {
        let reader = test_reader(b"a\x01StreamTitle='X';".to_vec());
        let cancelled = Arc::clone(&reader.cancelled);
        let mut icy = IcyAudioReader::new(reader, Some(1));
        assert_eq!(icy.read(&mut [0; 1]).unwrap(), 1);
        cancelled.store(true, Ordering::Release);
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            sender
                .send(icy.read(&mut [0; 1]).unwrap_err().kind())
                .unwrap();
        });
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(3)).unwrap(),
            io::ErrorKind::ConnectionAborted
        );
    }

    #[test]
    fn cancellation_polling_preserves_work_and_its_error() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cancelled = AtomicBool::new(false);
        let stopped = AtomicBool::new(false);
        let result = runtime.block_on(async {
            timeout(
                Duration::from_secs(3),
                until_cancelled(
                    async {
                        sleep(CANCEL_INTERVAL * 3).await;
                        Err("synthetic read timeout".to_owned())
                    },
                    &cancelled,
                    &stopped,
                ),
            )
            .await
            .expect("Cancellation polling must not restart in-flight work")
        });
        assert_eq!(result.unwrap_err(), "synthetic read timeout");
    }

    fn test_reader(bytes: Vec<u8>) -> ChunkReader {
        let (sender, receiver) = mpsc::sync_channel(2);
        sender.send(bytes).unwrap();
        drop(sender);
        ChunkReader {
            receiver,
            buffered: Vec::new(),
            position: 0,
            cancelled: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
            error: Arc::new(Mutex::new(None)),
            startup: StartupDeadline {
                until: Instant::now() + STARTUP_DEADLINE,
                complete: Arc::new(AtomicBool::new(false)),
            },
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
        assert!(matches!(
            open_station_classified("invalid-station", Arc::new(AtomicBool::new(false))),
            Err(TransportError::Permanent(_))
        ));
        assert!(matches!(
            open_station_classified("skylab", Arc::new(AtomicBool::new(true))),
            Err(TransportError::Cancelled)
        ));
        let source = Cursor::new([1u8, 2u8]);
        assert_eq!(source.into_inner(), [1, 2]);
    }
}
