//! Real HTTP fixtures exercise the same asynchronous transport as native playback.
use rust_radio_ip::error::TransportError;
use rust_radio_ip::net::open_direct_classified;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct Server {
    url: String,
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<String>>>,
    worker: Option<JoinHandle<()>>,
}
impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (cancel, seen) = (stop.clone(), requests.clone());
        let worker = thread::spawn(move || {
            while !cancel.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut request = Vec::new();
                while request.len() < 8192 && !request.windows(4).any(|bytes| bytes == b"\r\n\r\n")
                {
                    let mut buffer = [0; 1024];
                    let n = stream.read(&mut buffer).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..n]);
                }
                if request.is_empty() {
                    continue;
                }
                let path = String::from_utf8_lossy(&request)
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_owned();
                seen.lock().unwrap().push(path.clone());
                let (status, headers, body) = match path.split('?').next().unwrap_or("") {
                    "/redirect" => (
                        "302 Found",
                        "Location: /station.m3u?token=secret\r\n",
                        "".into(),
                    ),
                    "/station.m3u" => (
                        "200 OK",
                        "Content-Type: audio/x-mpegurl\r\n",
                        "#EXTM3U\n#EXTINF:-1,Local radio\n/audio\n".into(),
                    ),
                    "/station.pls" => (
                        "200 OK",
                        "Content-Type: audio/x-scpls\r\n",
                        "[playlist]\nFile1=/audio\nNumberOfEntries=1\nVersion=2\n".into(),
                    ),
                    "/audio" => (
                        "200 OK",
                        "Content-Type: audio/mpeg\r\nicy-metaint: 0\r\n",
                        "TEST".into(),
                    ),
                    "/html" => (
                        "200 OK",
                        "Content-Type: text/html\r\n",
                        "<html>No radio</html>".into(),
                    ),
                    "/missing" => ("404 Not Found", "", "".into()),
                    "/server-error" => ("503 Unavailable", "", "".into()),
                    "/loop" => ("302 Found", "Location: /loop\r\n", "".into()),
                    "/hls.m3u" => (
                        "200 OK",
                        "Content-Type: audio/x-mpegurl\r\n",
                        "#EXTM3U\n#EXT-X-VERSION:3\n/audio".into(),
                    ),
                    "/nested.m3u" => (
                        "200 OK",
                        "Content-Type: audio/x-mpegurl\r\n",
                        "/station.pls".into(),
                    ),
                    "/unsafe.pls" => (
                        "200 OK",
                        "Content-Type: audio/x-scpls\r\n",
                        "[playlist]\nFile1=/audio\nFile2=file:///tmp/audio\nNumberOfEntries=2"
                            .into(),
                    ),
                    "/oversized.m3u" => (
                        "200 OK",
                        "Content-Type: audio/x-mpegurl\r\n",
                        "x".repeat(16385),
                    ),
                    "/stall" => {
                        while !cancel.load(Ordering::Acquire) {
                            thread::sleep(Duration::from_millis(5));
                        }
                        continue;
                    }
                    other => panic!("Unexpected request {other}"),
                };
                let _ = write!(stream, "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        Self {
            url,
            stop,
            requests,
            worker: Some(worker),
        }
    }
    fn request(&self, path: &str) -> Result<Vec<u8>, TransportError> {
        let (mut input, _guard) = open_direct_classified(
            &format!("{}{path}", self.url),
            Arc::new(AtomicBool::new(false)),
        )?;
        let mut bytes = vec![0; 4];
        input.reader.read_exact(&mut bytes).unwrap();
        Ok(bytes)
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let result = self.worker.take().unwrap().join();
        if !thread::panicking() {
            result.unwrap();
        }
    }
}
#[test]
fn relative_redirect_and_m3u_pls_resolve_to_actual_audio() {
    let server = Server::new();
    for path in ["/audio", "/redirect", "/station.pls?download=1"] {
        assert_eq!(server.request(path).unwrap(), b"TEST");
    }
    assert_eq!(
        server.requests.lock().unwrap().as_slice(),
        [
            "/audio",
            "/redirect",
            "/station.m3u?token=secret",
            "/audio",
            "/station.pls?download=1",
            "/audio"
        ]
    );
}
#[test]
fn http_errors_bad_content_playlists_and_hop_loops_are_bounded() {
    let server = Server::new();
    for (path, expected) in [
        ("/html", "audio stream"),
        ("/missing", "404"),
        ("/hls.m3u", "HLS"),
        ("/nested.m3u", "Nested"),
        ("/unsafe.pls", "UnsafeEndpoint"),
        ("/oversized.m3u", "16 KiB"),
        ("/loop", "hop limit"),
    ] {
        let start = Instant::now();
        let error = server.request(path).unwrap_err();
        assert!(matches!(error, TransportError::Permanent(_)), "{error:?}");
        assert!(error.to_string().contains(expected), "{path}: {error}");
        assert!(start.elapsed() < Duration::from_secs(3));
    }
    assert!(matches!(
        server.request("/server-error"),
        Err(TransportError::Network(_))
    ));
    assert_eq!(
        server
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.as_str() == "/loop")
            .count(),
        5
    );
}
#[test]
fn cancellation_interrupts_pending_headers_and_unreachable_endpoint_is_classified() {
    let server = Server::new();
    let url = format!("{}/stall", server.url);
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let worker = thread::spawn(move || open_direct_classified(&url, worker_cancel).err().unwrap());
    let deadline = Instant::now() + Duration::from_secs(3);
    while server.requests.lock().unwrap().is_empty() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let start = Instant::now();
    cancel.store(true, Ordering::Release);
    assert_eq!(worker.join().unwrap(), TransportError::Cancelled);
    assert!(start.elapsed() < Duration::from_millis(500));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let closed = listener.local_addr().unwrap();
    drop(listener);
    assert!(matches!(
        open_direct_classified(
            &format!("http://{closed}/audio"),
            Arc::new(AtomicBool::new(false))
        ),
        Err(TransportError::Network(_))
    ));
}
#[test]
fn stalled_endpoint_hits_header_timeout_without_an_icmp_probe() {
    let server = Server::new();
    let start = Instant::now();
    let error = server.request("/stall").unwrap_err();
    assert!(error.to_string().contains("timeout"), "{error}");
    assert!((Duration::from_secs(7)..Duration::from_secs(11)).contains(&start.elapsed()));
}
