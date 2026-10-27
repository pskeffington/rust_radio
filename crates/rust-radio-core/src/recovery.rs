//! Radio recovery decisions, intentionally pure and deterministic.
//! The async transport owns timers and cancellation; this policy NEVER opens
//! network/device resources and cannot auto-start a stopped receiver.

use std::time::Duration;

pub const MAX_RECOVERY_ATTEMPTS: u8 = 3;
pub const RETRY_DELAYS_MS: [u64; 3] = [1_000, 2_000, 4_000];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureClass {
    /// Transient loss of approved broadcaster or connectivity.
    NetworkTransient,
    /// Device was unplugged, Bluetooth disconnected or OS audio failed.
    OutputUnavailable,
    /// Unsupported file/codec, unapproved endpoint or invalid transport state.
    Permanent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Timed retry permitted only while the current generation is active.
    RetryAfter(Duration),
    /// A changed output must be selected by the operator.
    SelectOutput,
    /// Exhausted budget or non-recoverable condition.
    Fail,
    /// Stop always takes priority over any retry or recovery.
    Stop,
}

pub fn decide(class: FailureClass, attempted_retries: u8, stop_requested: bool) -> RecoveryAction {
    if stop_requested {
        return RecoveryAction::Stop;
    }
    match class {
        FailureClass::NetworkTransient if attempted_retries < MAX_RECOVERY_ATTEMPTS => {
            RecoveryAction::RetryAfter(Duration::from_millis(
                RETRY_DELAYS_MS[usize::from(attempted_retries)],
            ))
        }
        FailureClass::OutputUnavailable => RecoveryAction::SelectOutput,
        _ => RecoveryAction::Fail,
    }
}

/// Classify only structured failures from the transport/codec layers;
/// do not guess whether arbitrary server text describes a temporary error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiverError {
    NetworkTimeout,
    NetworkDisconnected,
    HttpRejected,
    InvalidPlaylist,
    UnsupportedCodec,
    OutputDeviceLost,
    AudioFormatRejected,
}

impl ReceiverError {
    pub fn class(self) -> FailureClass {
        match self {
            Self::NetworkTimeout | Self::NetworkDisconnected => FailureClass::NetworkTransient,
            Self::OutputDeviceLost => FailureClass::OutputUnavailable,
            Self::HttpRejected
            | Self::InvalidPlaylist
            | Self::UnsupportedCodec
            | Self::AudioFormatRejected => FailureClass::Permanent,
        }
    }
}

/// Selecting another OS output is a user action. Running streams may
/// switch immediately; failed streams may only resume if their failure was
/// explicitly classified as an output-device failure. Never autoplay a
/// stopped session or retry unsupported media just from changing speakers.
pub fn restart_after_output_change(
    state: crate::SessionState,
    output_failed: bool,
) -> bool {
    use crate::SessionState;
    match state {
        SessionState::Stopped => false,
        SessionState::Failed => output_failed,
        SessionState::Connecting
        | SessionState::Buffering
        | SessionState::Playing
        | SessionState::Recovering => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_selection_never_autoplays_stopped_or_unrelated_failures() {
        use crate::SessionState;
        assert!(!restart_after_output_change(SessionState::Stopped, false));
        assert!(!restart_after_output_change(SessionState::Stopped, true));
        assert!(!restart_after_output_change(SessionState::Failed, false));
        assert!(restart_after_output_change(SessionState::Failed, true));
        assert!(restart_after_output_change(SessionState::Playing, false));
        assert!(restart_after_output_change(
            SessionState::Recovering,
            false
        ));
    }

    #[test]
    fn bounded_retry_delays() {
        assert_eq!(
            decide(FailureClass::NetworkTransient, 0, false),
            RecoveryAction::RetryAfter(Duration::from_secs(1))
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 1, false),
            RecoveryAction::RetryAfter(Duration::from_secs(2))
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 2, false),
            RecoveryAction::RetryAfter(Duration::from_secs(4))
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 3, false),
            RecoveryAction::Fail
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 255, false),
            RecoveryAction::Fail
        );
    }

    #[test]
    fn output_recovery_requires_explicit_selection() {
        for attempt in 0..=3 {
            assert_eq!(
                decide(FailureClass::OutputUnavailable, attempt, false),
                RecoveryAction::SelectOutput
            );
        }
    }

    #[test]
    fn no_retry_for_permanent_failures() {
        for error in [
            ReceiverError::HttpRejected,
            ReceiverError::InvalidPlaylist,
            ReceiverError::UnsupportedCodec,
            ReceiverError::AudioFormatRejected,
        ] {
            assert_eq!(error.class(), FailureClass::Permanent);
            assert_eq!(decide(error.class(), 0, false), RecoveryAction::Fail);
        }
    }

    #[test]
    fn stop_overrides_all_states_including_a_retry_slot() {
        for class in [
            FailureClass::NetworkTransient,
            FailureClass::OutputUnavailable,
            FailureClass::Permanent,
        ] {
            assert_eq!(decide(class, 0, true), RecoveryAction::Stop);
            assert_eq!(decide(class, 3, true), RecoveryAction::Stop);
        }
    }

    #[test]
    fn typed_classification_does_not_merge_device_and_network() {
        assert_eq!(
            ReceiverError::NetworkTimeout.class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            ReceiverError::NetworkDisconnected.class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            ReceiverError::OutputDeviceLost.class(),
            FailureClass::OutputUnavailable
        );
    }
}
