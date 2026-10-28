//! Evidence recorded by explicit station checks. Catalog inclusion is not validation.

use std::time::{Duration, SystemTime};

/// Maximum age for decoded-audio evidence before a station must be checked again.
pub const FEED_FRESHNESS_TTL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedHealth {
    Fresh,
    Stale,
    Unverified,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationStage {
    NotChecked,
    EndpointReached,
    StreamResponse,
    SupportedCodec,
    DecodedAudio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationStatus {
    Unverified,
    Unavailable,
    InvalidStream,
    Unsupported,
    Cancelled,
    Decoded,
}

/// A point-in-time check, not a guarantee of future availability or audible output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StationValidation {
    pub station_id: String,
    pub checked_at: Option<SystemTime>,
    pub stage: ValidationStage,
    pub status: ValidationStatus,
    pub resolved_url: Option<String>,
    pub content_type: Option<String>,
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<usize>,
    pub decoded_frames: usize,
    pub detail: String,
}

impl StationValidation {
    pub fn unverified(station_id: &str) -> Self {
        Self {
            station_id: station_id.into(),
            checked_at: None,
            stage: ValidationStage::NotChecked,
            status: ValidationStatus::Unverified,
            resolved_url: None,
            content_type: None,
            codec: None,
            sample_rate: None,
            channels: None,
            decoded_frames: 0,
            detail: "No decoded-audio validation has been performed".into(),
        }
    }

    /// Only recently decoded audio qualifies as fresh; a reachable socket alone never does.
    pub fn health_at(&self, now: SystemTime) -> FeedHealth {
        if self.is_decoded() {
            return match self.checked_at.and_then(|at| now.duration_since(at).ok()) {
                Some(age) if age <= FEED_FRESHNESS_TTL => FeedHealth::Fresh,
                _ => FeedHealth::Stale,
            };
        }
        match self.status {
            ValidationStatus::Unverified | ValidationStatus::Cancelled => FeedHealth::Unverified,
            _ => FeedHealth::Failed,
        }
    }

    pub fn is_decoded(&self) -> bool {
        self.status == ValidationStatus::Decoded
            && self.stage == ValidationStage::DecodedAudio
            && self.decoded_frames > 0
    }
}

#[cfg(test)]
mod freshness_tests {
    use super::*;
    #[test]
    fn decoded_audio_expires_and_failed_checks_never_qualify() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut report = StationValidation::unverified("test");
        assert_eq!(report.health_at(now), FeedHealth::Unverified);
        report.status = ValidationStatus::Decoded;
        report.stage = ValidationStage::DecodedAudio;
        report.decoded_frames = 256;
        report.checked_at = Some(now - FEED_FRESHNESS_TTL);
        assert_eq!(report.health_at(now), FeedHealth::Fresh);
        report.checked_at = Some(now - FEED_FRESHNESS_TTL - Duration::from_secs(1));
        assert_eq!(report.health_at(now), FeedHealth::Stale);
        report.status = ValidationStatus::Unavailable;
        assert_eq!(report.health_at(now), FeedHealth::Failed);
    }
}
