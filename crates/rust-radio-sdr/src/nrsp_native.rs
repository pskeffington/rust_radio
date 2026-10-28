//! Direct nRSP-ST browser wire protocol. All commands require explicit calls.
//! Implements the device's session handshake, private virtual receiver and Opus
//! ingress; no browser, SDRconnect process, recording or firmware changes.
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tungstenite::client::client_with_config;
use tungstenite::handshake::HandshakeError;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

const INITIAL_MASK: u32 = 0x3f5e220a;
// Public web-client bootstrap constants, not operator credentials. Subsequent
// keys are negotiated per connection, kept private and never logged or stored.
const INITIAL_KEY: [u32; 4] = [0xfe45a343, 0x2ef57aff, 0xaa500913, 0xfec3326a];
const MAX_MESSAGE: usize = 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidEndpoint,
    InvalidConfig,
    Cancelled,
    Timeout,
    AuthenticationRequired,
    AudioUnavailable,
    Protocol(&'static str),
    Disconnected(u32),
    Transport(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint => {
                f.write_str("Use a private or loopback IPv4 address and port for the nRSP-ST")
            }
            Self::InvalidConfig => {
                f.write_str("Use 1 kHz–2 GHz and a supported demodulator/bandwidth")
            }
            Self::Cancelled => f.write_str("Direct nRSP-ST session cancelled"),
            Self::Timeout => f.write_str("Direct nRSP-ST response or audio deadline expired"),
            Self::AuthenticationRequired => {
                f.write_str("Receiver requires authentication; native login is not configured")
            }
            Self::AudioUnavailable => f.write_str("Receiver does not permit native audio sessions"),
            Self::Protocol(message) => write!(f, "Direct nRSP-ST protocol: {message}"),
            Self::Disconnected(reason) => {
                write!(f, "Receiver ended the native session (reason {reason})")
            }
            Self::Transport(message) => write!(f, "Direct nRSP-ST connection: {message}"),
        }
    }
}
impl std::error::Error for Error {}
fn transport(error: impl fmt::Display) -> Error {
    Error::Transport(error.to_string())
}
fn pending(error: &tungstenite::Error) -> bool {
    matches!(error, tungstenite::Error::Io(e) if e.kind() == io::ErrorKind::WouldBlock)
}
fn check(deadline: Instant, cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(Error::Timeout);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint(SocketAddr);
impl Endpoint {
    pub fn parse(value: &str) -> Result<Self, Error> {
        let address: SocketAddr = value.parse().map_err(|_| Error::InvalidEndpoint)?;
        let std::net::IpAddr::V4(ip) = address.ip() else {
            return Err(Error::InvalidEndpoint);
        };
        if (!ip.is_private() && !ip.is_loopback()) || address.port() == 0 {
            return Err(Error::InvalidEndpoint);
        }
        Ok(Self(address))
    }
    pub fn from_host(value: &str) -> Result<Self, Error> {
        let ip: Ipv4Addr = value.parse().map_err(|_| Error::InvalidEndpoint)?;
        Self::parse(&format!("{ip}:9001"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Mode {
    Am = 1,
    Usb = 2,
    Lsb = 3,
    Nfm = 4,
    Wfm = 5,
    Cw = 6,
    Sam = 7,
}
impl Mode {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value.to_ascii_lowercase().as_str() {
            "am" => Ok(Self::Am),
            "usb" => Ok(Self::Usb),
            "lsb" => Ok(Self::Lsb),
            "nfm" => Ok(Self::Nfm),
            "wfm" => Ok(Self::Wfm),
            "cw" => Ok(Self::Cw),
            "sam" => Ok(Self::Sam),
            _ => Err(Error::InvalidConfig),
        }
    }
    pub fn default_bandwidth(self) -> u32 {
        match self {
            Self::Wfm => 200_000,
            Self::Nfm => 12_000,
            Self::Am | Self::Sam => 10_000,
            Self::Usb | Self::Lsb => 3_000,
            Self::Cw => 500,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub frequency_hz: u64,
    pub mode: Mode,
    pub bandwidth_hz: u32,
}
impl Config {
    pub fn new(frequency_hz: u64, mode: Mode, bandwidth_hz: u32) -> Result<Self, Error> {
        let value = Self {
            frequency_hz,
            mode,
            bandwidth_hz,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(self) -> Result<(), Error> {
        if !(1_000..=2_000_000_000).contains(&self.frequency_hz)
            || !(100..=250_000).contains(&self.bandwidth_hz)
        {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub protocol_version: u32,
    pub center_frequency_hz: u64,
    pub sample_rate_hz: u32,
    pub model_number: u8,
    pub authentication_required: bool,
    pub audio_allowed: bool,
    pub wfm_allowed: bool,
    pub antenna: u32,
}

struct Session {
    mask: u32,
    key: [u32; 4],
}
impl Session {
    fn initial() -> Self {
        Self {
            mask: INITIAL_MASK,
            key: INITIAL_KEY,
        }
    }
    fn decode(&self, frame: &[u8]) -> Result<(u8, Vec<u8>), Error> {
        if !(4..=MAX_MESSAGE).contains(&frame.len()) {
            return Err(Error::Protocol("Invalid envelope size"));
        }
        let header = u32::from_be_bytes(frame[..4].try_into().unwrap()) ^ self.mask;
        let length = ((header >> 4) & 0xfffff) as usize;
        let padding = (header & 15) as usize;
        if length + 4 != frame.len() || length % 16 != 0 || padding > length {
            return Err(Error::Protocol("Invalid envelope length or padding"));
        }
        let kind = (header >> 24) as u8;
        if !(201..=223).contains(&kind) {
            return Err(Error::Protocol("Unknown message type"));
        }
        let mut payload = frame[4..].to_vec();
        if kind != 203 && kind != 204 {
            decipher(&mut payload, &self.key);
        }
        payload.truncate(length - padding);
        Ok((kind, payload))
    }
    fn command(&self, kind: u8, words: &[u32]) -> Vec<u8> {
        let length = words.len() * 4;
        let padded = length.div_ceil(16) * 16;
        let header =
            ((kind as u32) << 24 | ((padded as u32) << 4) | (padded - length) as u32) ^ self.mask;
        let mut bytes = vec![0; padded + 4];
        bytes[..4].copy_from_slice(&header.to_be_bytes());
        for (word, target) in words.iter().zip(bytes[4..].chunks_exact_mut(4)) {
            target.copy_from_slice(&word.to_be_bytes());
        }
        // The observed browser protocol sends command payloads without XTEA.
        bytes
    }
    fn hello(&mut self, payload: &[u8]) -> Result<Status, Error> {
        if payload.len() != 172 || &payload[..4] != b"SDRP" {
            return Err(Error::Protocol("Invalid receiver Hello"));
        }
        let options = word(payload, 23)?;
        let protocol = word(payload, 1)?;
        if protocol != 0 {
            return Err(Error::Protocol("Unsupported receiver protocol version"));
        }
        let status = Status {
            protocol_version: protocol,
            center_frequency_hz: u64::from(word(payload, 2)?)
                | (u64::from(word(payload, 3)?) << 32),
            sample_rate_hz: word(payload, 4)?,
            model_number: (word(payload, 10)? >> 24) as u8,
            authentication_required: options & 128 != 0,
            audio_allowed: options & 32 != 0,
            wfm_allowed: options & 1 != 0,
            antenna: word(payload, 8)?,
        };
        self.mask = word(payload, 22)?;
        for (index, key) in self.key.iter_mut().enumerate() {
            *key = word(payload, 18 + index)?;
        }
        Ok(status)
    }
}
fn word(bytes: &[u8], index: usize) -> Result<u32, Error> {
    let value = bytes
        .get(index * 4..index * 4 + 4)
        .ok_or(Error::Protocol("Truncated message"))?;
    Ok(u32::from_be_bytes(value.try_into().unwrap()))
}
fn decipher(bytes: &mut [u8], key: &[u32; 4]) {
    // Device protocol transforms one eight-byte block per sixteen payload bytes.
    let transformed = bytes.len() / 16 * 8;
    for block in bytes[..transformed].chunks_exact_mut(8) {
        let mut a = u32::from_le_bytes(block[..4].try_into().unwrap());
        let mut b = u32::from_le_bytes(block[4..].try_into().unwrap());
        let mut sum = 0xc6ef3720u32;
        for _ in 0..32 {
            b = b.wrapping_sub(
                (((a << 4) ^ (a >> 5)).wrapping_add(a))
                    ^ sum.wrapping_add(key[((sum >> 11) & 3) as usize]),
            );
            sum = sum.wrapping_sub(0x9e3779b9);
            a = a.wrapping_sub(
                (((b << 4) ^ (b >> 5)).wrapping_add(b)) ^ sum.wrapping_add(key[(sum & 3) as usize]),
            );
        }
        block[..4].copy_from_slice(&a.to_le_bytes());
        block[4..].copy_from_slice(&b.to_le_bytes());
    }
}

/// Owns one device WebSocket and at most one connection-local virtual receiver.
/// Session secrets have no Debug/Serialize implementation. Drop attempts to stop
/// only this virtual receiver before shutting down the owned socket.
pub struct Client {
    endpoint: Endpoint,
    socket: WebSocket<TcpStream>,
    session: Session,
    status: Status,
    receiver_owned: bool,
}
impl Client {
    pub fn connect(endpoint: Endpoint, cancel: &AtomicBool) -> Result<Self, Error> {
        Self::connect_until(endpoint, cancel, Instant::now() + DEADLINE)
    }
    fn connect_until(
        endpoint: Endpoint,
        cancel: &AtomicBool,
        deadline: Instant,
    ) -> Result<Self, Error> {
        check(deadline, cancel)?;
        let stream = TcpStream::connect_timeout(
            &endpoint.0,
            deadline.saturating_duration_since(Instant::now()),
        )
        .map_err(transport)?;
        stream.set_nonblocking(true).map_err(transport)?;
        let config = WebSocketConfig::default()
            .read_buffer_size(16 * 1024)
            .write_buffer_size(0)
            .max_write_buffer_size(128 * 1024)
            .max_message_size(Some(MAX_MESSAGE))
            .max_frame_size(Some(MAX_MESSAGE));
        let mut handshake =
            client_with_config(format!("ws://{}/", endpoint.0), stream, Some(config));
        let socket = loop {
            check(deadline, cancel)?;
            match handshake {
                Ok((socket, _)) => break socket,
                Err(HandshakeError::Interrupted(mid)) => {
                    std::thread::sleep(POLL);
                    handshake = mid.handshake();
                }
                Err(HandshakeError::Failure(e)) => return Err(transport(e)),
            }
        };
        let mut client = Self {
            endpoint,
            socket,
            session: Session::initial(),
            receiver_owned: false,
            status: Status {
                protocol_version: 0,
                center_frequency_hz: 0,
                sample_rate_hz: 0,
                model_number: 0,
                authentication_required: false,
                audio_allowed: false,
                wfm_allowed: false,
                antenna: 0,
            },
        };
        let (kind, payload) = client.read(deadline, cancel)?;
        if kind != 201 {
            return Err(Error::Protocol("Expected initial Hello"));
        }
        client.status = client.session.hello(&payload)?;
        Ok(client)
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
    fn read(&mut self, deadline: Instant, cancel: &AtomicBool) -> Result<(u8, Vec<u8>), Error> {
        loop {
            check(deadline, cancel)?;
            match self.socket.read() {
                Ok(Message::Binary(bytes)) => return self.session.decode(&bytes),
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                Ok(Message::Close(_)) => {
                    return Err(Error::Transport("Receiver closed the connection".into()))
                }
                Ok(_) => return Err(Error::Protocol("Expected binary message")),
                Err(e) if pending(&e) => std::thread::sleep(POLL),
                Err(e) => return Err(transport(e)),
            }
        }
    }
    fn send(
        &mut self,
        kind: u8,
        words: &[u32],
        deadline: Instant,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        check(deadline, cancel)?;
        match self
            .socket
            .send(Message::Binary(self.session.command(kind, words).into()))
        {
            Ok(()) => return Ok(()),
            Err(e) if pending(&e) => {}
            Err(e) => return Err(transport(e)),
        }
        loop {
            check(deadline, cancel)?;
            match self.socket.flush() {
                Ok(()) => return Ok(()),
                Err(e) if pending(&e) => std::thread::sleep(POLL),
                Err(e) => return Err(transport(e)),
            }
        }
    }
    /// Configure and start receiver 1 on this connection. Physical center
    /// frequency is shared by the appliance; the caller explicitly chooses it.
    pub fn start(&mut self, config: Config, cancel: &AtomicBool) -> Result<(), Error> {
        config.validate()?;
        if self.receiver_owned {
            return Err(Error::Protocol("Receiver already started"));
        }
        if self.status.authentication_required {
            return Err(Error::AuthenticationRequired);
        }
        if !self.status.audio_allowed || (config.mode == Mode::Wfm && !self.status.wfm_allowed) {
            return Err(Error::AudioUnavailable);
        }
        let deadline = Instant::now() + DEADLINE;
        self.send(28, &[1], deadline, cancel)?;
        self.receiver_owned = true;
        self.configure(config, true, cancel)
    }

    /// Retune the active connection without opening another receiver or output.
    pub fn tune(&mut self, config: Config, cancel: &AtomicBool) -> Result<(), Error> {
        config.validate()?;
        if !self.receiver_owned {
            return Err(Error::Protocol("Receiver is not started"));
        }
        if config.mode == Mode::Wfm && !self.status.wfm_allowed {
            return Err(Error::AudioUnavailable);
        }
        self.configure(config, false, cancel)
    }

    fn configure(
        &mut self,
        config: Config,
        activate: bool,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        let deadline = Instant::now() + DEADLINE;
        self.send(
            2,
            &[
                config.frequency_hz as u32,
                (config.frequency_hz >> 32) as u32,
            ],
            deadline,
            cancel,
        )?;
        self.send(32, &[1, 0, 0], deadline, cancel)?;
        self.send(34, &[1, config.mode as u32], deadline, cancel)?;
        self.send(33, &[1, config.bandwidth_hz], deadline, cancel)?;
        self.send(39, &[1, 100], deadline, cancel)?;
        if activate {
            self.send(30, &[1, 1], deadline, cancel)?;
        }
        self.send(59, &[0], deadline, cancel)?;
        if self.status.center_frequency_hz == config.frequency_hz {
            return Ok(());
        }
        // This firmware does not echo a command to its sending connection.
        // Confirm changed physical tuning through an independent read-only Hello.
        loop {
            check(deadline, cancel)?;
            let observer = Self::connect_until(self.endpoint, cancel, deadline)?;
            if observer.status.center_frequency_hz == config.frequency_hz {
                self.status = observer.status.clone();
                return Ok(());
            }
            drop(observer);
            // Device control is asynchronous across connections. Keep polling
            // bounded and cancellable rather than accepting the first stale Hello.
            for _ in 0..20 {
                check(deadline, cancel)?;
                std::thread::sleep(POLL);
            }
        }
    }

    fn update(&mut self, payload: &[u8]) -> Result<(), Error> {
        if payload.len() < 60 {
            return Err(Error::Protocol("Truncated device update"));
        }
        self.status.center_frequency_hz =
            u64::from(word(payload, 0)?) | (u64::from(word(payload, 1)?) << 32);
        self.status.sample_rate_hz = word(payload, 2)?;
        self.status.antenna = word(payload, 6)?;
        Ok(())
    }
    /// Borrow each Opus packet for the primary connection-local receiver.
    /// Non-audio traffic never extends the three-second audio inactivity limit.
    pub fn stream_audio(
        &mut self,
        cancel: &AtomicBool,
        consume: impl FnMut(&[u8]) -> bool,
    ) -> Result<(), Error> {
        self.stream_audio_with_control(cancel, |_| Ok(()), consume)
    }

    /// Poll host commands between packets on the same receiver worker.
    pub fn stream_audio_with_control(
        &mut self,
        cancel: &AtomicBool,
        mut control: impl FnMut(&mut Self) -> Result<(), Error>,
        mut consume: impl FnMut(&[u8]) -> bool,
    ) -> Result<(), Error> {
        if !self.receiver_owned {
            return Err(Error::Protocol("Start the receiver before subscribing"));
        }
        let mut audio_deadline = Instant::now() + DEADLINE;
        let mut ping_at = Instant::now();
        loop {
            if cancel.load(Ordering::Acquire) {
                return Ok(());
            }
            control(self)?;
            if Instant::now() >= ping_at {
                let seconds = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                self.send(
                    17,
                    &[seconds as u32, (seconds >> 32) as u32],
                    audio_deadline,
                    cancel,
                )?;
                ping_at = Instant::now() + Duration::from_secs(1);
            }
            let (kind, payload) = self.read(audio_deadline, cancel)?;
            match kind {
                202 => self.update(&payload)?,
                222 => return Err(Error::Disconnected(word(&payload, 0)?)),
                211 => {
                    if payload.len() < 5 {
                        return Err(Error::Protocol("Truncated Opus packet"));
                    }
                    if word(&payload, 0)? == 1 {
                        if !consume(&payload[4..]) {
                            return Ok(());
                        }
                        audio_deadline = Instant::now() + DEADLINE;
                    }
                }
                _ => {}
            }
        }
    }
    pub fn stop(&mut self) -> Result<(), Error> {
        if self.receiver_owned {
            self.receiver_owned = false;
            self.send(
                30,
                &[1, 0],
                Instant::now() + Duration::from_millis(250),
                &AtomicBool::new(false),
            )?;
        }
        Ok(())
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.stop();
        let _ = self.socket.get_ref().shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    const TEST_MASK: u32 = 0x12345678;
    const TEST_KEY: [u32; 4] = [11, 22, 33, 44];

    fn server_frame(kind: u8, payload: &[u8], session: &Session) -> Vec<u8> {
        let padded = payload.len().div_ceil(16) * 16;
        let mut bytes = vec![0; padded + 4];
        let header = ((kind as u32) << 24 | (padded as u32) << 4 | (padded - payload.len()) as u32)
            ^ session.mask;
        bytes[..4].copy_from_slice(&header.to_be_bytes());
        bytes[4..4 + payload.len()].copy_from_slice(payload);
        for block in bytes[4..4 + padded / 16 * 8].chunks_exact_mut(8) {
            let mut a = u32::from_le_bytes(block[..4].try_into().unwrap());
            let mut b = u32::from_le_bytes(block[4..].try_into().unwrap());
            let mut sum = 0u32;
            for _ in 0..32 {
                a = a.wrapping_add(
                    (((b << 4) ^ (b >> 5)).wrapping_add(b))
                        ^ sum.wrapping_add(session.key[(sum & 3) as usize]),
                );
                sum = sum.wrapping_add(0x9e3779b9);
                b = b.wrapping_add(
                    (((a << 4) ^ (a >> 5)).wrapping_add(a))
                        ^ sum.wrapping_add(session.key[((sum >> 11) & 3) as usize]),
                );
            }
            block[..4].copy_from_slice(&a.to_le_bytes());
            block[4..].copy_from_slice(&b.to_le_bytes());
        }
        bytes
    }
    fn hello(auth: bool) -> Vec<u8> {
        let mut payload = vec![0; 172];
        payload[..4].copy_from_slice(b"SDRP");
        for (index, value) in [
            (2, 99_100_000u32),
            (4, 2_000_000),
            (10, 5 << 24),
            (18, TEST_KEY[0]),
            (19, TEST_KEY[1]),
            (20, TEST_KEY[2]),
            (21, TEST_KEY[3]),
            (22, TEST_MASK),
            (23, 33 | if auth { 128 } else { 0 }),
        ] {
            payload[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
        }
        server_frame(201, &payload, &Session::initial())
    }
    fn fixture(
        auth: bool,
        server: impl FnOnce(WebSocket<TcpStream>, TcpListener) + Send + 'static,
    ) -> (Endpoint, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = Endpoint(listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut socket = tungstenite::accept(socket).unwrap();
            socket.send(Message::Binary(hello(auth).into())).unwrap();
            server(socket, listener);
        });
        (endpoint, worker)
    }
    fn command(socket: &mut WebSocket<TcpStream>) -> (u8, Vec<u32>) {
        let Message::Binary(bytes) = socket.read().unwrap() else {
            panic!("Expected command")
        };
        let header = u32::from_be_bytes(bytes[..4].try_into().unwrap()) ^ TEST_MASK;
        let length = ((header >> 4) & 0xfffff) as usize;
        assert_eq!(length + 4, bytes.len());
        assert_eq!(length % 16, 0);
        let actual = length - (header & 15) as usize;
        let words = bytes[4..4 + actual]
            .chunks_exact(4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
            .collect();
        ((header >> 24) as u8, words)
    }

    #[test]
    fn invalid_configuration_and_precancelled_connect_are_offline() {
        for host in [
            "example.com:9001",
            "8.8.8.8:9001",
            "169.254.169.254:9001",
            "127.0.0.1:0",
        ] {
            assert_eq!(Endpoint::parse(host), Err(Error::InvalidEndpoint));
        }
        assert!(Config::new(0, Mode::Wfm, 200_000).is_err());
        assert!(Config::new(99_100_000, Mode::Wfm, 0).is_err());
        assert!(Mode::parse("invalid").is_err());
        assert!(matches!(
            Client::connect(
                Endpoint::parse("127.0.0.1:9").unwrap(),
                &AtomicBool::new(true)
            ),
            Err(Error::Cancelled)
        ));
    }
    #[test]
    fn handshake_recovers_session_and_rejects_malformed_envelopes() {
        let mut session = Session::initial();
        let (kind, payload) = session.decode(&hello(false)).unwrap();
        assert_eq!(kind, 201);
        let status = session.hello(&payload).unwrap();
        assert_eq!(status.center_frequency_hz, 99_100_000);
        assert_eq!(status.model_number, 5);
        assert_eq!(session.key, TEST_KEY);
        assert_eq!(session.mask, TEST_MASK);
        for bytes in [
            vec![],
            vec![0; 3],
            vec![0; 20],
            hello(false)[..179].to_vec(),
        ] {
            assert!(session.decode(&bytes).is_err());
        }
        let mut unsupported = payload;
        unsupported[7] = 1;
        assert!(session.hello(&unsupported).is_err());
    }
    #[test]
    fn native_start_confirms_changed_frequency_delivers_opus_and_stops_owned_receiver() {
        let (endpoint, server) = fixture(false, |mut socket, listener| {
            assert_eq!(command(&mut socket), (28, vec![1]));
            assert_eq!(command(&mut socket), (2, vec![99_200_000, 0]));
            assert_eq!(command(&mut socket), (32, vec![1, 0, 0]));
            assert_eq!(command(&mut socket), (34, vec![1, 5]));
            assert_eq!(command(&mut socket), (33, vec![1, 200_000]));
            assert_eq!(command(&mut socket), (39, vec![1, 100]));
            assert_eq!(command(&mut socket), (30, vec![1, 1]));
            assert_eq!(command(&mut socket), (59, vec![0]));
            let session = Session {
                mask: TEST_MASK,
                key: TEST_KEY,
            };
            let mut update = vec![0; 60];
            update[..4].copy_from_slice(&99_200_000u32.to_be_bytes());
            update[8..12].copy_from_slice(&2_000_000u32.to_be_bytes());
            socket
                .send(Message::Binary(server_frame(202, &update, &session).into()))
                .unwrap();
            let (stale_observer, _) = listener.accept().unwrap();
            let mut stale_observer = tungstenite::accept(stale_observer).unwrap();
            stale_observer
                .send(Message::Binary(hello(false).into()))
                .unwrap();
            drop(stale_observer);
            let (observer, _) = listener.accept().unwrap();
            let mut observer = tungstenite::accept(observer).unwrap();
            let (_, mut confirmed_hello) = Session::initial().decode(&hello(false)).unwrap();
            confirmed_hello[8..12].copy_from_slice(&99_200_000u32.to_be_bytes());
            observer
                .send(Message::Binary(
                    server_frame(201, &confirmed_hello, &Session::initial()).into(),
                ))
                .unwrap();
            drop(observer);
            // Live mode/bandwidth change reuses receiver 1 without Add/Start.
            assert_eq!(command(&mut socket), (2, vec![99_200_000, 0]));
            assert_eq!(command(&mut socket), (32, vec![1, 0, 0]));
            assert_eq!(command(&mut socket), (34, vec![1, 4]));
            assert_eq!(command(&mut socket), (33, vec![1, 12_000]));
            assert_eq!(command(&mut socket), (39, vec![1, 100]));
            assert_eq!(command(&mut socket), (59, vec![0]));
            assert_eq!(command(&mut socket).0, 17);
            socket
                .send(Message::Binary(
                    server_frame(211, &[0, 0, 0, 1, 0xf8, 0xff, 0xfe], &session).into(),
                ))
                .unwrap();
            assert_eq!(command(&mut socket), (30, vec![1, 0]));
            let mut bytes = [0];
            use std::io::Read;
            assert_eq!(socket.get_mut().read(&mut bytes).unwrap(), 0);
        });
        let cancel = AtomicBool::new(false);
        let mut client = Client::connect(endpoint, &cancel).unwrap();
        client
            .start(
                Config::new(99_200_000, Mode::Wfm, 200_000).unwrap(),
                &cancel,
            )
            .unwrap();
        assert_eq!(client.status().center_frequency_hz, 99_200_000);
        client
            .tune(Config::new(99_200_000, Mode::Nfm, 12_000).unwrap(), &cancel)
            .unwrap();
        let mut packets = 0;
        client
            .stream_audio(&cancel, |packet| {
                assert_eq!(packet, &[0xf8, 0xff, 0xfe]);
                packets += 1;
                false
            })
            .unwrap();
        assert_eq!(packets, 1);
        client.stop().unwrap();
        client.stop().unwrap();
        drop(client);
        server.join().unwrap();
    }
    #[test]
    fn authentication_requirement_prevents_all_control_commands() {
        let (endpoint, server) = fixture(true, |mut socket, _listener| {
            use std::io::Read;
            assert_eq!(socket.get_mut().read(&mut [0]).unwrap(), 0);
        });
        let cancel = AtomicBool::new(false);
        let mut client = Client::connect(endpoint, &cancel).unwrap();
        assert!(client.status().authentication_required);
        assert_eq!(
            client.start(
                Config::new(99_100_000, Mode::Wfm, 200_000).unwrap(),
                &cancel
            ),
            Err(Error::AuthenticationRequired)
        );
        drop(client);
        server.join().unwrap();
    }
}
