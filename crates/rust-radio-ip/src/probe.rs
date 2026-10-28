//! Native HTTP(S) radio source for explicit, user-requested receiver operations.
//! No external browser or player process. No automatic request on module load.
//!
//! The current CLI uses this adapter for short, bounded stream probes.
//! Continuous playback requires an independently cancellable worker and a
//! per-read inactivity timeout; it must not reuse the probe's total deadline.

use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::redirect::Policy;
use rust_radio_core::stream::{ApprovedEndpoint, HttpResponse, RadioHttp, StreamError};
use std::fmt;
use std::io::{self, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

/// One deadline covers every playlist/redirect hop and the final audio body.
pub const PROBE_DEADLINE: Duration = Duration::from_secs(18);

#[derive(Debug)]
struct ProbeCancelled;

impl fmt::Display for ProbeCancelled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Radio validation cancelled")
    }
}

impl std::error::Error for ProbeCancelled {}

pub fn probe_was_cancelled(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|cause| cause.is::<ProbeCancelled>())
}

pub struct NativeRadioHttp {
    client: Client,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

/// A bounded probe body. Cancellation is checked before and after each read;
/// an already-blocked request/read can wait until the shared HTTP deadline.
pub struct ProbeBody<R = Response> {
    inner: R,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

impl<R: Read> Read for ProbeBody<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        check_probe(self.deadline, &self.cancelled)?;
        let count = self.inner.read(out)?;
        check_probe(self.deadline, &self.cancelled)?;
        Ok(count)
    }
}

fn check_probe(deadline: Instant, cancelled: &AtomicBool) -> io::Result<()> {
    if cancelled.load(Ordering::Acquire) {
        // Interrupted would be retried indefinitely by Read::read_exact,
        // including the ICY metadata reader. Cancellation must terminate it.
        return Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            ProbeCancelled,
        ));
    }
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Radio validation deadline exceeded",
        ));
    }
    Ok(())
}

impl NativeRadioHttp {
    pub fn for_probe() -> Result<Self, StreamError> {
        Self::for_cancellable_probe(Arc::new(AtomicBool::new(false)))
    }

    pub fn for_cancellable_probe(cancelled: Arc<AtomicBool>) -> Result<Self, StreamError> {
        let deadline = Instant::now() + PROBE_DEADLINE;
        check_probe(deadline, &cancelled)?;
        let client = Client::builder()
            .https_only(true)
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            // Total deadline is appropriate ONLY for short stream probing.
            .timeout(PROBE_DEADLINE)
            .user_agent("Rust-Boy-RADIO/0.1 (receiver probe)")
            .build()
            .map_err(|_| StreamError::TransportUnavailable)?;
        Ok(Self {
            client,
            deadline,
            cancelled,
        })
    }
}

impl RadioHttp for NativeRadioHttp {
    type Body = ProbeBody;

    fn get(&self, target: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
        check_probe(self.deadline, &self.cancelled)?;
        let remaining = self
            .deadline
            .checked_duration_since(Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or(StreamError::TransportUnavailable)?;
        // ApprovedEndpoint checks curated station ownership and HTTPS before
        // the request is built; reqwest performs normal certificate validation.
        let response = self
            .client
            .get(target.url())
            .timeout(remaining)
            .header("Icy-MetaData", "1")
            .send()
            .map_err(|_| StreamError::TransportUnavailable)?;
        check_probe(self.deadline, &self.cancelled)?;
        let status = response.status().as_u16();
        let content_type = header_value(&response, CONTENT_TYPE)?;
        let redirect_location = header_value(&response, LOCATION)?;
        let icy_metaint = response
            .headers()
            .get("icy-metaint")
            .map(|value| {
                value
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|_| StreamError::InvalidIcyInterval)
            })
            .transpose()?;

        Ok(HttpResponse {
            status,
            content_type,
            redirect_location,
            icy_metaint,
            body: ProbeBody {
                inner: response,
                deadline: self.deadline,
                cancelled: Arc::clone(&self.cancelled),
            },
        })
    }
}

fn header_value(
    response: &Response,
    name: reqwest::header::HeaderName,
) -> Result<Option<String>, StreamError> {
    response
        .headers()
        .get(name)
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .map_err(|_| StreamError::TransportUnavailable)
        })
        .transpose()
}

/// Short HTTPS reachability check used by explicitly enabled input discovery.
/// HEAD fetches no radio audio and does not follow redirects or use a proxy.
pub fn internet_available(cancelled: &AtomicBool) -> bool {
    if cancelled.load(Ordering::Acquire) {
        return false;
    }
    let result = Client::builder()
        .https_only(true)
        .redirect(Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .and_then(|client| client.head("https://somafm.com/").send());
    !cancelled.load(Ordering::Acquire)
        && result.is_ok_and(|response| response.status().is_success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_tls_probe_can_initialize_without_network() {
        assert!(NativeRadioHttp::for_probe().is_ok());
    }

    #[test]
    fn expired_probe_cannot_start_another_redirect_or_playlist_request() {
        let mut client = NativeRadioHttp::for_probe().unwrap();
        client.deadline = Instant::now() - Duration::from_secs(1);
        let endpoint =
            ApprovedEndpoint::new("groovesalad", "https://somafm.com/groovesalad.pls").unwrap();
        assert!(matches!(
            client.get(&endpoint),
            Err(StreamError::Io(error)) if error.kind() == io::ErrorKind::TimedOut
        ));
    }

    #[test]
    fn cancelled_probe_never_initializes_or_sends_another_request() {
        let cancelled = Arc::new(AtomicBool::new(true));
        assert!(
            matches!(NativeRadioHttp::for_cancellable_probe(Arc::clone(&cancelled)), Err(StreamError::Io(error)) if probe_was_cancelled(&error))
        );
        cancelled.store(false, Ordering::Release);
        let client = NativeRadioHttp::for_cancellable_probe(Arc::clone(&cancelled)).unwrap();
        cancelled.store(true, Ordering::Release);
        let endpoint =
            ApprovedEndpoint::new("groovesalad", "https://somafm.com/groovesalad.pls").unwrap();
        assert!(
            matches!(client.get(&endpoint), Err(StreamError::Io(error)) if probe_was_cancelled(&error))
        );
    }

    #[test]
    fn cancelled_or_expired_bodies_cannot_deliver_buffered_audio() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut body = ProbeBody {
            inner: io::Cursor::new([1, 2, 3]),
            deadline: Instant::now() + PROBE_DEADLINE,
            cancelled: Arc::clone(&cancelled),
        };
        assert_eq!(body.read(&mut [0; 1]).unwrap(), 1);
        cancelled.store(true, Ordering::Release);
        assert_eq!(
            body.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::ConnectionAborted
        );
        cancelled.store(false, Ordering::Release);
        body.deadline = Instant::now() - Duration::from_secs(1);
        assert_eq!(
            body.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }
}
