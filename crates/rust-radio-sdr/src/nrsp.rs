//! Read-only nRSP-ST service reachability without SDRconnect.
//!
//! The browser service and device service were identified in Fides Supra's
//! `docs/nsdr-network-bridge.md`. Reachability does not identify hardware or
//! provide a native audio, I/Q, authentication, tuning, or control protocol.
use std::fmt;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const WEB_PORT: u16 = 9001;
const DEVICE_PORT: u16 = 50000;
const SERVICE_TIMEOUT: Duration = Duration::from_millis(1500);
const READ_POLL: Duration = Duration::from_millis(50);
const MAX_RESPONSE_BYTES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Host(Ipv4Addr);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidHost,
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHost => f.write_str("Use a literal private-LAN or loopback IPv4 address"),
            Self::Cancelled => f.write_str("Direct receiver status check cancelled"),
        }
    }
}
impl std::error::Error for Error {}

/// An open port is reachability evidence, not proof of receiver identity or RF.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub web_connected: bool,
    pub web_http_status: Option<u16>,
    pub device_connected: bool,
}

impl Host {
    pub fn parse(value: &str) -> Result<Self, Error> {
        let ip: Ipv4Addr = value.parse().map_err(|_| Error::InvalidHost)?;
        if !ip.is_private() && !ip.is_loopback() {
            return Err(Error::InvalidHost);
        }
        Ok(Self(ip))
    }

    /// One bounded check of the explicitly supplied host; never scans a network,
    /// follows redirects, starts reception, or sends device-service payloads.
    pub fn status(self, cancelled: &AtomicBool) -> Result<Status, Error> {
        probe(
            SocketAddr::from((self.0, WEB_PORT)),
            SocketAddr::from((self.0, DEVICE_PORT)),
            cancelled,
        )
    }
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), Error> {
    if cancelled.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

fn probe(web: SocketAddr, device: SocketAddr, cancelled: &AtomicBool) -> Result<Status, Error> {
    check_cancelled(cancelled)?;
    let (web_connected, web_http_status) = probe_web(web, cancelled)?;
    check_cancelled(cancelled)?;
    let device_connected = TcpStream::connect_timeout(&device, SERVICE_TIMEOUT).is_ok();
    check_cancelled(cancelled)?;
    Ok(Status {
        web_connected,
        web_http_status,
        device_connected,
    })
}

fn probe_web(address: SocketAddr, cancelled: &AtomicBool) -> Result<(bool, Option<u16>), Error> {
    let deadline = Instant::now() + SERVICE_TIMEOUT;
    let mut stream = match TcpStream::connect_timeout(&address, SERVICE_TIMEOUT) {
        Ok(stream) => stream,
        Err(_) => return Ok((false, None)),
    };
    check_cancelled(cancelled)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Ok((true, None));
    }
    if stream.set_write_timeout(Some(remaining)).is_err() {
        return Ok((true, None));
    }
    let request = format!("GET / HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return Ok((true, None));
    }
    let mut bytes = [0u8; MAX_RESPONSE_BYTES];
    let mut used = 0;
    while used < bytes.len() && Instant::now() < deadline {
        check_cancelled(cancelled)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero()
            || stream
                .set_read_timeout(Some(remaining.min(READ_POLL)))
                .is_err()
        {
            break;
        }
        match stream.read(&mut bytes[used..]) {
            Ok(0) => break,
            Ok(count) => {
                used += count;
                if let Some(end) = bytes[..used].windows(2).position(|pair| pair == b"\r\n") {
                    let status = std::str::from_utf8(&bytes[..end]).ok().and_then(|line| {
                        let mut fields = line.split_whitespace();
                        let version = fields.next()?;
                        if !matches!(version, "HTTP/1.0" | "HTTP/1.1") {
                            return None;
                        }
                        fields
                            .next()?
                            .parse::<u16>()
                            .ok()
                            .filter(|status| (100..=599).contains(status))
                    });
                    return Ok((true, status));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue
            }
            Err(_) => break,
        }
    }
    check_cancelled(cancelled)?;
    Ok((true, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn accepts_only_explicit_local_ipv4_addresses() {
        for address in ["127.0.0.1", "10.2.3.4", "172.20.30.1", "192.168.1.1"] {
            assert!(Host::parse(address).is_ok());
        }
        for address in [
            "radio.local",
            "8.8.8.8",
            "169.254.169.254",
            "0.0.0.0",
            "127.0.0.1:9001",
            "::1",
        ] {
            assert_eq!(Host::parse(address), Err(Error::InvalidHost));
        }
    }

    #[test]
    fn probes_only_requested_services_without_device_commands_or_redirects() {
        let web = TcpListener::bind("127.0.0.1:0").unwrap();
        let device = TcpListener::bind("127.0.0.1:0").unwrap();
        let web_addr = web.local_addr().unwrap();
        let device_addr = device.local_addr().unwrap();
        let web_thread = thread::spawn(move || {
            let (mut stream, _) = web.accept().unwrap();
            let mut request = [0; 512];
            let len = stream.read(&mut request).unwrap();
            assert!(request[..len].starts_with(b"GET / HTTP/1.1\r\n"));
            stream
                .write_all(b"HTTP/1.1 302 Found\r\nLocation: http://example.invalid/\r\n\r\n")
                .unwrap();
        });
        let device_thread = thread::spawn(move || {
            let (mut stream, _) = device.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            assert_eq!(stream.read(&mut [0; 1]).unwrap(), 0);
        });
        let result = probe(web_addr, device_addr, &AtomicBool::new(false)).unwrap();
        web_thread.join().unwrap();
        device_thread.join().unwrap();
        assert_eq!(
            result,
            Status {
                web_connected: true,
                web_http_status: Some(302),
                device_connected: true
            }
        );
    }

    #[test]
    fn cancellation_prevents_connections() {
        let web = TcpListener::bind("127.0.0.1:0").unwrap();
        web.set_nonblocking(true).unwrap();
        assert_eq!(
            probe(
                web.local_addr().unwrap(),
                web.local_addr().unwrap(),
                &AtomicBool::new(true)
            ),
            Err(Error::Cancelled)
        );
        assert_eq!(
            web.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}
