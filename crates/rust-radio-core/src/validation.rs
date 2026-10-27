//! Evidence recorded by explicit station checks. Catalog inclusion is not validation.

use std::time::SystemTime;

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

    pub fn is_decoded(&self) -> bool {
        self.status == ValidationStatus::Decoded
            && self.stage == ValidationStage::DecodedAudio
            && self.decoded_frames > 0
    }
}
