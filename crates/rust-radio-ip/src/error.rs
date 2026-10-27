//! Structured transport failures for bounded playback recovery.

use rust_radio_core::recovery::FailureClass;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// Connection/read failures or a temporary HTTP response.
    Network(String),
    /// Rejected endpoint, malformed playlist, unsupported content or setup error.
    Permanent(String),
    /// The host explicitly stopped this session before streaming began.
    Cancelled,
}

impl TransportError {
    pub fn class(&self) -> FailureClass {
        match self {
            Self::Network(_) => FailureClass::NetworkTransient,
            Self::Permanent(_) | Self::Cancelled => FailureClass::Permanent,
        }
    }

    pub(crate) fn http_status(status: reqwest::StatusCode) -> Self {
        let message = format!("Radio server responded HTTP {status}");
        if status.is_server_error()
            || matches!(
                status,
                reqwest::StatusCode::REQUEST_TIMEOUT | reqwest::StatusCode::TOO_MANY_REQUESTS
            )
        {
            Self::Network(message)
        } else {
            Self::Permanent(message)
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(message) | Self::Permanent(message) => formatter.write_str(message),
            Self::Cancelled => formatter.write_str("Radio stopped before streaming began"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<String> for TransportError {
    fn from(message: String) -> Self {
        Self::Permanent(message)
    }
}

impl From<&str> for TransportError {
    fn from(message: &str) -> Self {
        Self::Permanent(message.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_radio_core::recovery::{decide, RecoveryAction};

    #[test]
    fn temporary_http_responses_retry_but_rejections_do_not() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error = TransportError::http_status(reqwest::StatusCode::from_u16(status).unwrap());
            assert_eq!(error.class(), FailureClass::NetworkTransient);
            assert!(matches!(
                decide(error.class(), 0, false),
                RecoveryAction::RetryAfter(_)
            ));
            assert_eq!(decide(error.class(), 3, false), RecoveryAction::Fail);
        }
        for status in [400, 401, 403, 404, 410, 451] {
            let error = TransportError::http_status(reqwest::StatusCode::from_u16(status).unwrap());
            assert_eq!(error.class(), FailureClass::Permanent);
            assert_eq!(decide(error.class(), 0, false), RecoveryAction::Fail);
        }
    }

    #[test]
    fn message_text_never_decides_whether_to_retry() {
        let rejected = TransportError::from("network timeout in untrusted playlist");
        assert_eq!(rejected.class(), FailureClass::Permanent);
        assert_eq!(
            rejected.to_string(),
            "network timeout in untrusted playlist"
        );
        assert_eq!(
            decide(TransportError::Cancelled.class(), 0, true),
            RecoveryAction::Stop
        );
    }
}
