//! Manual native-output acceptance: actual HTTP → decoder → CPAL → GUI state.
use super::*;
use eframe::egui;
use rust_radio_audio::compat_controller::PlaybackState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

struct Fixture {
    root: String,
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Fixture {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let root = format!("http://{}", listener.local_addr().unwrap());
        let cancel = Arc::new(AtomicBool::new(false));
        let stopped = cancel.clone();
        let worker = thread::spawn(move || {
            let mut clients = Vec::new();
            while !stopped.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let cancel = stopped.clone();
                        clients.push(thread::spawn(move || serve(stream, cancel)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            for client in clients {
                client.join().unwrap();
            }
        });
        Self {
            root,
            cancel,
            worker: Some(worker),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        let result = self.worker.take().unwrap().join();
        if !thread::panicking() {
            result.unwrap();
        }
    }
}
fn serve(mut stream: TcpStream, cancel: Arc<AtomicBool>) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut bytes = [0; 4096];
    let n = stream.read(&mut bytes).unwrap_or(0);
    let path = String::from_utf8_lossy(&bytes[..n])
        .split_whitespace()
        .nth(1)
        .unwrap_or("")
        .to_owned();
    if path == "/stall" {
        while !cancel.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(5));
        }
        return;
    }
    if path == "/redirect" {
        let _ = stream.write_all(
            b"HTTP/1.1 302 Found\r\nLocation: /station.pls\r\nContent-Length: 0\r\n\r\n",
        );
        return;
    }
    if path == "/station.pls" {
        let body = "[playlist]\nFile1=/aac\nNumberOfEntries=1\nVersion=2";
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: audio/x-scpls\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        return;
    }
    if path == "/invalid" {
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 7\r\n\r\ninvalid",
        );
        return;
    }
    if path == "/unsupported" {
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\n\r\nRIFF\x24\x10\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x44\xac\x00\x00\x88\x58\x01\x00\x02\x00\x10\x00data\x00\x10\x00\x00");
        let _ = stream.write_all(&[0; 4096]);
        return;
    }
    let (mime, data): (&str, &[u8]) = if path == "/aac" {
        (
            "audio/aac",
            include_bytes!("../../rust-radio-audio/tests/fixtures/tone.aac"),
        )
    } else {
        (
            "audio/mpeg",
            include_bytes!("../../rust-radio-audio/tests/fixtures/tone.mp3"),
        )
    };
    if write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nConnection: close\r\n\r\n"
    )
    .is_err()
    {
        return;
    }
    while !cancel.load(Ordering::Acquire) {
        if stream.write_all(data).is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
}
fn pump(panel: &mut RadioPanel, context: &egui::Context) -> Duration {
    let start = Instant::now();
    let _ = context.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(480.0, 320.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
        },
    );
    start.elapsed()
}
fn wait_for(panel: &mut RadioPanel, context: &egui::Context, state: PlaybackState) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        assert!(
            pump(panel, context) < Duration::from_millis(250),
            "GUI blocked"
        );
        let status = panel
            .engine
            .as_ref()
            .map(|e| e.status())
            .or_else(|| panel.last_session.clone());
        if status.as_ref().is_some_and(|s| s.state == state) {
            println!("Reached {state:?}: {}", panel.note);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Expected {state:?}, got {status:?}, note {}",
            panel.note
        );
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
#[ignore = "requires native audio output and local TCP; runs muted"]
fn native_ip_playback_switching_failure_retry_and_responsiveness() {
    let fixture = Fixture::new();
    let context = egui::Context::default();
    let mut panel = RadioPanel::new();
    panel.volume = 0;
    for path in ["/mp3", "/redirect", "/stall", "/invalid", "/unsupported"] {
        panel.direct_selected = true;
        panel.direct_url = format!("{}{path}", fixture.root);
        panel.ip_station_name = path.into();
        panel.save_ip_station().unwrap();
    }
    let base = curated_stations().len();
    panel.select_ip_station(base);
    panel.play();
    wait_for(&mut panel, &context, PlaybackState::Playing);
    assert_eq!(panel.stream_check_summary(), "Receiving audio now");
    assert!(!panel.source_detail().contains("UNVERIFIED"));
    panel.change_volume(0);
    assert_eq!(panel.volume, 0);
    assert_eq!(panel.stream_check_summary(), "Receiving audio now");
    // Keep this hardware acceptance muted; volume restoration is covered offline.
    panel.select_ip_station(base + 1);
    assert!(panel.restart_pending);
    assert_eq!(
        panel.engine.as_ref().unwrap().status().state,
        PlaybackState::Stopped
    );
    wait_for(&mut panel, &context, PlaybackState::Playing);
    for index in [2, 0, 1, 2, 0, 2] {
        panel.select_ip_station(base + index);
        panel.ip_selection_changed();
    }
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        assert!(pump(&mut panel, &context) < Duration::from_millis(250));
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        panel.engine.as_ref().unwrap().status().state,
        PlaybackState::Connecting
    );
    panel.select_ip_station(base);
    panel.ip_selection_changed();
    wait_for(&mut panel, &context, PlaybackState::Playing);
    for index in [3, 4] {
        panel.select_ip_station(base + index);
        panel.ip_selection_changed();
        wait_for(&mut panel, &context, PlaybackState::Failed);
        assert!(!panel.restart_pending);
        while panel.engine.is_some() {
            pump(&mut panel, &context);
            thread::sleep(Duration::from_millis(5));
        }
        panel.play(); // explicit Retry repeats the failed target, never stale audio
        wait_for(&mut panel, &context, PlaybackState::Failed);
    }
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/audio", closed.local_addr().unwrap());
    drop(closed);
    panel.stop();
    while !panel.poll_shutdown() {
        thread::sleep(Duration::from_millis(5));
    }
    panel.direct_url = url;
    panel.saved_station_index = None;
    panel.play();
    wait_for(&mut panel, &context, PlaybackState::Failed);
    panel.stop();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !panel.poll_shutdown() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    println!("Muted MP3, AAC through redirect/PLS, rapid switching, stalled headers, invalid/unsupported audio, Retry, refused endpoint, responsive rendering and clean Stop passed");
}
