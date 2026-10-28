//! SDRconnect WebSocket API 1.0.x. An nRSP-ST is reached through SDRconnect,
//! not through the USB SDRplay API. Construction is offline; all I/O is explicit.
//! This client observes an already selected receiver. Explicit host playback can
//! start shared reception when SDRconnect grants control. It never stops, retunes,
//! or selects hardware, records, or transmits.
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tungstenite::client::client_with_config;
use tungstenite::handshake::HandshakeError;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_CONTROL_BYTES: usize = 8192;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const START_STATUS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidEndpoint,
    Cancelled,
    Timeout,
    StartTimeout,
    NotStreaming,
    ControlUnavailable,
    Protocol(String),
    Transport(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint => {
                f.write_str("Use an explicit private-LAN or loopback IP:port for SDRconnect")
            }
            Self::Cancelled => f.write_str("SDRconnect operation cancelled"),
            Self::Timeout => f.write_str("SDRconnect API deadline expired"),
            Self::StartTimeout => f.write_str(
                "SDRconnect did not confirm receiver startup within three seconds. Use Refresh Device List, select the nRSP-ST, and retry Play",
            ),
            Self::NotStreaming => {
                f.write_str(
                    "SDRconnect receiver did not start. Use Refresh Device List and select the nRSP-ST; start it for passive capture or use explicit native Play",
                )
            }
            Self::ControlUnavailable => {
                f.write_str("SDRconnect does not allow starting the selected receiver")
            }
            Self::Protocol(message) => write!(f, "SDRconnect protocol: {message}"),
            Self::Transport(message) => write!(f, "SDRconnect connection: {message}"),
        }
    }
}
impl std::error::Error for Error {}

/// Explicit local API peer. No DNS lookup, redirects, credentials or public
/// endpoint fallback. The nRSP-ST's native network port is a different protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint(SocketAddr);
impl Endpoint {
    pub fn parse(value: &str) -> Result<Self, Error> {
        let address: SocketAddr = value.parse().map_err(|_| Error::InvalidEndpoint)?;
        let allowed = match address.ip() {
            IpAddr::V4(ip) => ip.is_private() || ip.is_loopback(),
            IpAddr::V6(ip) => ip.is_loopback() || (ip.segments()[0] & 0xfe00 == 0xfc00),
        };
        if !allowed || address.port() == 0 {
            return Err(Error::InvalidEndpoint);
        }
        Ok(Self(address))
    }
    pub fn address(self) -> SocketAddr {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub api_version: String,
    pub active_device: String,
    pub valid_devices: Vec<String>,
    pub started: bool,
    pub can_control: bool,
    pub center_frequency_hz: u64,
    pub sample_rate_hz: f64,
    pub demodulator: String,
    pub antennas: Vec<String>,
    pub active_antenna: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamKind {
    Iq,
    Audio,
}
impl StreamKind {
    fn event(self) -> &'static str {
        match self {
            Self::Iq => "iq_stream_enable",
            Self::Audio => "audio_stream_enable",
        }
    }
}

/// Borrowed, signed little-endian pairs. Audio is primary-device stereo PCM
/// at 48 kHz; I/Q rate is the receiver's current setting. Secondary/spectrum
/// packets are distinguished and never counted as primary audio or I/Q.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryFrame<'a> {
    Audio(&'a [u8]),
    Iq(&'a [u8]),
    Other,
}
pub fn decode_binary(bytes: &[u8]) -> Result<BinaryFrame<'_>, Error> {
    if !(2..=MAX_MESSAGE_BYTES).contains(&bytes.len()) {
        return Err(Error::Protocol("Invalid binary message length".into()));
    }
    let tag = u16::from_le_bytes([bytes[0], bytes[1]]);
    let payload = &bytes[2..];
    if matches!(tag, 1 | 2 | 4 | 5) && payload.len() % 4 != 0 {
        return Err(Error::Protocol(
            "Incomplete signed 16-bit sample pair".into(),
        ));
    }
    match tag {
        1 => Ok(BinaryFrame::Audio(payload)),
        2 => Ok(BinaryFrame::Iq(payload)),
        3..=6 => Ok(BinaryFrame::Other),
        _ => Err(Error::Protocol("Unknown binary message type".into())),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiveReport {
    pub kind: StreamKind,
    pub sample_pairs: u64,
    pub cancelled: bool,
    pub elapsed: Duration,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    event_type: String,
    property: String,
    value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<String>,
}

fn pending(error: &tungstenite::Error) -> bool {
    matches!(error, tungstenite::Error::Io(error) if error.kind() == io::ErrorKind::WouldBlock)
}
fn transport(error: impl fmt::Display) -> Error {
    Error::Transport(error.to_string())
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

/// Owned, bounded synchronous API client. Keep on a host worker, never a UI or
/// audio callback. Dropping closes this connection and its subscriptions.
pub struct Client {
    socket: WebSocket<TcpStream>,
}
impl Client {
    pub fn connect(endpoint: Endpoint, cancel: &AtomicBool) -> Result<Self, Error> {
        Self::connect_until(endpoint, cancel, Instant::now() + OPERATION_TIMEOUT)
    }
    fn connect_until(
        endpoint: Endpoint,
        cancel: &AtomicBool,
        deadline: Instant,
    ) -> Result<Self, Error> {
        check(deadline, cancel)?;
        let timeout = deadline.saturating_duration_since(Instant::now());
        let stream = TcpStream::connect_timeout(&endpoint.0, timeout).map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                Error::Timeout
            } else {
                transport(error)
            }
        })?;
        stream.set_nonblocking(true).map_err(transport)?;
        let config = WebSocketConfig::default()
            .read_buffer_size(16 * 1024)
            .write_buffer_size(0)
            .max_write_buffer_size(128 * 1024)
            .max_message_size(Some(MAX_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_MESSAGE_BYTES));
        let mut handshake =
            client_with_config(format!("ws://{}/", endpoint.0), stream, Some(config));
        loop {
            check(deadline, cancel)?;
            match handshake {
                Ok((socket, _)) => return Ok(Self { socket }),
                Err(HandshakeError::Interrupted(mid)) => {
                    std::thread::sleep(POLL_INTERVAL);
                    handshake = mid.handshake();
                }
                Err(HandshakeError::Failure(error)) => return Err(transport(error)),
            }
        }
    }

    fn send(
        &mut self,
        event: &str,
        property: &str,
        value: &str,
        deadline: Instant,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        check(deadline, cancel)?;
        let json = serde_json::to_string(&Envelope {
            event_type: event.into(),
            property: property.into(),
            value: value.into(),
            device: Some("primary".into()),
        })
        .map_err(transport)?;
        // A WouldBlock send has already queued the message. Flush it, never
        // resend it (which could duplicate a subscription or control request).
        match self.socket.send(Message::Text(json.into())) {
            Ok(()) => return Ok(()),
            Err(error) if pending(&error) => {}
            Err(error) => return Err(transport(error)),
        }
        loop {
            check(deadline, cancel)?;
            match self.socket.flush() {
                Ok(()) => return Ok(()),
                Err(error) if pending(&error) => std::thread::sleep(POLL_INTERVAL),
                Err(error) => return Err(transport(error)),
            }
        }
    }

    fn read(&mut self, deadline: Instant, cancel: &AtomicBool) -> Result<Message, Error> {
        loop {
            check(deadline, cancel)?;
            match self.socket.read() {
                Ok(Message::Close(_)) => return Err(transport("Peer closed the API connection")),
                Ok(message) => return Ok(message),
                Err(error) if pending(&error) => std::thread::sleep(POLL_INTERVAL),
                Err(error) => return Err(transport(error)),
            }
        }
    }

    fn query(
        &mut self,
        property: &str,
        deadline: Instant,
        cancel: &AtomicBool,
    ) -> Result<String, Error> {
        self.send("get_property", property, "", deadline, cancel)?;
        loop {
            if let Message::Text(json) = self.read(deadline, cancel)? {
                if json.len() > MAX_CONTROL_BYTES {
                    return Err(Error::Protocol("Oversized control message".into()));
                }
                let envelope: Envelope = serde_json::from_str(&json)
                    .map_err(|_| Error::Protocol("Malformed control envelope".into()))?;
                if envelope.value.len() > 4096 {
                    return Err(Error::Protocol("Oversized property value".into()));
                }
                if envelope.event_type == "get_property_response"
                    && envelope.property == property
                    && envelope
                        .device
                        .as_deref()
                        .map_or(true, |device| device == "primary")
                {
                    return Ok(envelope.value);
                }
            }
        }
    }

    /// Read-only snapshot with one shared three-second deadline. No device
    /// selection, tuning, stream-enable or recording request is sent.
    pub fn status(&mut self, cancel: &AtomicBool) -> Result<Status, Error> {
        let deadline = Instant::now() + OPERATION_TIMEOUT;
        let api_version = self.query("api_version", deadline, cancel)?;
        if !api_version.starts_with("1.0.") {
            return Err(Error::Protocol(
                "Unsupported API version; expected 1.0.x".into(),
            ));
        }
        let active_device = self.query("active_device", deadline, cancel)?;
        let valid_devices = split_list(&self.query("valid_devices", deadline, cancel)?);
        let started = parse_bool(&self.query("started", deadline, cancel)?)?;
        let can_control = parse_bool(&self.query("can_control", deadline, cancel)?)?;
        let center_frequency_hz = self
            .query("device_center_frequency", deadline, cancel)?
            .parse()
            .map_err(|_| Error::Protocol("Invalid center frequency".into()))?;
        let sample_rate_hz: f64 = self
            .query("device_sample_rate", deadline, cancel)?
            .parse()
            .map_err(|_| Error::Protocol("Invalid sample rate".into()))?;
        if !sample_rate_hz.is_finite() || sample_rate_hz < 0.0 {
            return Err(Error::Protocol("Invalid sample rate".into()));
        }
        let demodulator = self.query("demodulator", deadline, cancel)?;
        let antennas = split_list(&self.query("valid_antennas", deadline, cancel)?);
        let active_antenna = self.query("active_antenna", deadline, cancel)?;
        Ok(Status {
            api_version,
            active_device,
            valid_devices,
            started,
            can_control,
            center_frequency_hz,
            sample_rate_hz,
            demodulator,
            antennas,
            active_antenna,
        })
    }

    /// Start the already selected receiver after an explicit host Play action.
    /// Call only after status confirms a selected device and `can_control`.
    /// This never selects or refreshes devices and leaves the receiver running
    /// when this client later disconnects.
    /// Missing startup confirmation returns `StartTimeout`, requiring explicit
    /// retry rather than repeating device activation through network recovery.
    pub fn start_receiver(&mut self, cancel: &AtomicBool) -> Result<(), Error> {
        let deadline = Instant::now() + OPERATION_TIMEOUT;
        let result = (|| {
            self.send("device_stream_enable", "", "true", deadline, cancel)?;
            loop {
                if parse_bool(&self.query("started", deadline, cancel)?)? {
                    return Ok(());
                }
                // Receiver startup can take time. Limit status traffic while
                // retaining the same prompt cancellation as socket polling.
                let next_query = (Instant::now() + START_STATUS_INTERVAL).min(deadline);
                while Instant::now() < next_query {
                    check(deadline, cancel)?;
                    std::thread::sleep(POLL_INTERVAL);
                }
            }
        })();
        result.map_err(|error| match error {
            Error::Timeout => Error::StartTimeout,
            error => error,
        })
    }

    /// Subscribe to an already running primary receiver for at most five
    /// seconds. Consumes/closes the client so buffered data cannot enter a later
    /// session. The callback borrows one bounded packet; it must not block.
    /// Cleanup disables only this client's subscription, leaving shared RF
    /// streaming and other clients untouched. No samples are stored here.
    pub fn receive_for(
        self,
        kind: StreamKind,
        duration: Duration,
        cancel: &AtomicBool,
        mut on_frame: impl FnMut(BinaryFrame<'_>),
    ) -> Result<ReceiveReport, Error> {
        if duration.is_zero() || duration > super::MAX_CAPTURE_DURATION {
            return Err(Error::Protocol(
                "Receive duration must be 1..5000 ms".into(),
            ));
        }
        self.receive(kind, Some(duration), cancel, |frame| {
            on_frame(frame);
            true
        })
    }

    /// Continuous primary PCM for a native playback worker. Memory remains
    /// bounded; missing primary audio fails after three seconds even if the
    /// server continues sending unrelated events. Stop unsubscribes and closes.
    /// The read deadline excludes time spent delivering a packet to the native
    /// output worker. The callback must bound its work and observe cancellation.
    pub fn stream_audio(
        self,
        cancel: &AtomicBool,
        mut on_audio: impl FnMut(&[u8]) -> bool,
    ) -> Result<ReceiveReport, Error> {
        self.receive(StreamKind::Audio, None, cancel, |frame| {
            if let BinaryFrame::Audio(bytes) = frame {
                on_audio(bytes)
            } else {
                true
            }
        })
    }

    fn receive(
        mut self,
        kind: StreamKind,
        duration: Option<Duration>,
        cancel: &AtomicBool,
        mut on_frame: impl FnMut(BinaryFrame<'_>) -> bool,
    ) -> Result<ReceiveReport, Error> {
        let start = Instant::now();
        if cancel.load(Ordering::Acquire) {
            return Ok(ReceiveReport {
                kind,
                sample_pairs: 0,
                cancelled: true,
                elapsed: start.elapsed(),
            });
        }
        if !parse_bool(&self.query("started", start + OPERATION_TIMEOUT, cancel)?)? {
            return Err(Error::NotStreaming);
        }
        // Arm cleanup before sending: even a partial enable followed by an I/O
        // failure must attempt the matching disable and close the connection.
        let result = (|| {
            self.send(kind.event(), "", "true", start + OPERATION_TIMEOUT, cancel)?;
            let until = duration.map(|duration| Instant::now() + duration);
            let mut last_audio = Instant::now();
            let mut sample_pairs = 0u64;
            loop {
                let deadline = until.unwrap_or(last_audio + OPERATION_TIMEOUT);
                let message = match self.read(deadline, cancel) {
                    Ok(message) => message,
                    Err(Error::Timeout) if until.is_some() => break,
                    Err(Error::Cancelled) => break,
                    Err(error) => return Err(error),
                };
                if let Message::Binary(bytes) = message {
                    let frame = decode_binary(&bytes)?;
                    let pairs = match (kind, frame) {
                        (StreamKind::Audio, BinaryFrame::Audio(payload))
                        | (StreamKind::Iq, BinaryFrame::Iq(payload)) => payload.len() / 4,
                        _ => continue,
                    };
                    sample_pairs = sample_pairs.saturating_add(pairs as u64);
                    if pairs > 0 {
                        if !on_frame(frame) {
                            break;
                        }
                        // Native output backpressure can take longer than a
                        // network read timeout for one bounded PCM packet.
                        // Start the next read's idle budget after delivery;
                        // fixed-duration captures still use their own `until`.
                        last_audio = Instant::now();
                    }
                }
            }
            Ok(ReceiveReport {
                kind,
                sample_pairs,
                cancelled: cancel.load(Ordering::Acquire),
                elapsed: start.elapsed(),
            })
        })();
        let cleanup = self.send(
            kind.event(),
            "",
            "false",
            Instant::now() + Duration::from_millis(250),
            &AtomicBool::new(false),
        );
        // Closing is the final fallback if unsubscribe cannot reach the peer.
        if result.is_err() || cleanup.is_err() {
            let _ = self.socket.get_ref().shutdown(Shutdown::Both);
        }
        match (result, cleanup) {
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(report), Ok(())) => Ok(report),
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.socket.get_ref().shutdown(Shutdown::Both);
    }
}
fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
fn parse_bool(value: &str) -> Result<bool, Error> {
    match value {
        "true" | "True" => Ok(true),
        "false" | "False" => Ok(false),
        _ => Err(Error::Protocol("Invalid Boolean property".into())),
    }
}

#[cfg(test)]
mod tests;
