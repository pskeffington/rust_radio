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
use std::time::Duration;

pub struct NativeRadioHttp {
    client: Client,
}

impl NativeRadioHttp {
    pub fn for_probe() -> Result<Self, StreamError> {
        let client = Client::builder()
            .https_only(true)
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            // Total deadline is appropriate ONLY for short stream probing.
            .timeout(Duration::from_secs(18))
            .user_agent("Rust-Boy-RADIO/0.1 (receiver probe)")
            .build()
            .map_err(|_| StreamError::TransportUnavailable)?;
        Ok(Self { client })
    }
}

impl RadioHttp for NativeRadioHttp {
    type Body = Response;

    fn get(&self, target: &ApprovedEndpoint) -> Result<HttpResponse<Self::Body>, StreamError> {
        // ApprovedEndpoint checks curated station ownership and HTTPS before
        // the request is built; reqwest performs normal certificate validation.
        let response = self
            .client
            .get(target.url())
            .header("Icy-MetaData", "1")
            .send()
            .map_err(|_| StreamError::TransportUnavailable)?;
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
            body: response,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_tls_probe_can_initialize_without_network() {
        assert!(NativeRadioHttp::for_probe().is_ok());
    }
}
