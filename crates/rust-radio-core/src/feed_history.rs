//! Host-neutral reader for versioned, persisted curated-feed validation.
//!
//! This module performs no file, network, audio, or receiver I/O. A host owns
//! persistence and may display these records, but saved evidence never gates
//! playback.

use crate::directory::curated_stations;
use crate::validation::{FeedHealth, ValidationStage, ValidationStatus, FEED_FRESHNESS_TTL};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const FEED_HISTORY_SCHEMA_VERSION: u64 = 1;
pub const MAX_FEED_HISTORY_BYTES: usize = 64 * 1024;
const MAX_CONSECUTIVE_FAILURES: u64 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedHistory {
    pub updated_at: SystemTime,
    pub stations: BTreeMap<&'static str, FeedHistoryRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeedHistoryRecord {
    pub last_checked_at: SystemTime,
    pub status: ValidationStatus,
    pub stage: ValidationStage,
    pub attempts: u8,
    pub last_decoded_at: Option<SystemTime>,
    pub consecutive_failures: u32,
}

impl FeedHistoryRecord {
    /// Recompute freshness at display time; cached health labels are not trusted.
    pub fn health_at(self, now: SystemTime) -> FeedHealth {
        match self.status {
            ValidationStatus::Decoded => match self
                .last_decoded_at
                .and_then(|checked| now.duration_since(checked).ok())
            {
                Some(age) if age <= FEED_FRESHNESS_TTL => FeedHealth::Fresh,
                _ => FeedHealth::Stale,
            },
            ValidationStatus::Unverified | ValidationStatus::Cancelled => FeedHealth::Unverified,
            ValidationStatus::Unavailable
            | ValidationStatus::InvalidStream
            | ValidationStatus::Unsupported => FeedHealth::Failed,
        }
    }
}

/// Parse and validate the v1 CLI `--history` document for any Rust host.
pub fn parse_feed_history(json: &str) -> Result<FeedHistory, String> {
    if json.len() > MAX_FEED_HISTORY_BYTES {
        return Err("Feed validation history exceeds 64 KiB".into());
    }
    let value: Value = serde_json::from_str(json)
        .map_err(|error| format!("Feed validation history is invalid JSON: {error}"))?;
    if value["schema_version"].as_u64() != Some(FEED_HISTORY_SCHEMA_VERSION) {
        return Err("Unsupported feed validation history schema".into());
    }
    let updated_at = epoch(value["updated_at_unix"].as_u64(), "update")?;
    if updated_at > SystemTime::now() {
        return Err("Feed validation history has a future update timestamp".into());
    }
    let stations = value["stations"]
        .as_object()
        .ok_or_else(|| "Feed validation history has no stations object".to_owned())?;
    if stations.len() > curated_stations().len() {
        return Err("Feed validation history contains too many stations".into());
    }

    let mut parsed = BTreeMap::new();
    for (id, record) in stations {
        let station = curated_stations()
            .iter()
            .find(|station| station.id == id)
            .ok_or_else(|| format!("Unknown curated station in feed history: {id}"))?;
        let checked_at = epoch(record["last_checked_at_unix"].as_u64(), "last check")?;
        if checked_at > updated_at {
            return Err(format!("Future station check in feed history for {id}"));
        }
        let status = parse_status(record["last_status"].as_str())?;
        let stage = parse_stage(record["last_stage"].as_str())?;
        let attempts = record["attempts"]
            .as_u64()
            .filter(|attempts| (1..=2).contains(attempts))
            .ok_or_else(|| format!("Invalid attempt count in feed history for {id}"))?
            as u8;
        let failures = record["consecutive_failures"]
            .as_u64()
            .filter(|failures| *failures <= MAX_CONSECUTIVE_FAILURES)
            .ok_or_else(|| format!("Invalid failure count in feed history for {id}"))?
            as u32;
        let last_decoded_at = match record["last_decoded_at_unix"].as_u64() {
            Some(timestamp) => {
                let time = epoch(Some(timestamp), "last decoded")?;
                if time > checked_at || time > updated_at {
                    return Err(format!("Future decoded timestamp in feed history for {id}"));
                }
                Some(time)
            }
            None if record["last_decoded_at_unix"].is_null() => None,
            None => {
                return Err(format!(
                    "Invalid decoded timestamp in feed history for {id}"
                ))
            }
        };
        if status == ValidationStatus::Decoded {
            if last_decoded_at != Some(checked_at)
                || stage != ValidationStage::DecodedAudio
                || failures != 0
            {
                return Err(format!(
                    "Inconsistent decoded evidence in feed history for {id}"
                ));
            }
        } else if stage == ValidationStage::DecodedAudio {
            return Err(format!(
                "Inconsistent decoded stage in feed history for {id}"
            ));
        }

        let health = record["health"]
            .as_str()
            .ok_or_else(|| format!("Missing health in feed history for {id}"))?;
        let valid_health = match status {
            ValidationStatus::Decoded => {
                let age = updated_at.duration_since(checked_at).unwrap_or_default();
                health
                    == if age <= FEED_FRESHNESS_TTL {
                        "fresh"
                    } else {
                        "stale"
                    }
            }
            ValidationStatus::Cancelled | ValidationStatus::Unverified => health == "unverified",
            _ => health == "failed",
        };
        if !valid_health {
            return Err(format!("Inconsistent health in feed history for {id}"));
        }

        parsed.insert(
            station.id,
            FeedHistoryRecord {
                last_checked_at: checked_at,
                status,
                stage,
                attempts,
                last_decoded_at,
                consecutive_failures: failures,
            },
        );
    }
    Ok(FeedHistory {
        updated_at,
        stations: parsed,
    })
}

fn epoch(seconds: Option<u64>, label: &str) -> Result<SystemTime, String> {
    let seconds = seconds.ok_or_else(|| format!("Missing {label} timestamp in feed history"))?;
    UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .ok_or_else(|| format!("Invalid {label} timestamp in feed history"))
}

fn parse_status(status: Option<&str>) -> Result<ValidationStatus, String> {
    match status {
        Some("unverified") => Ok(ValidationStatus::Unverified),
        Some("unavailable" | "check_error") => Ok(ValidationStatus::Unavailable),
        Some("invalid_stream") => Ok(ValidationStatus::InvalidStream),
        Some("unsupported") => Ok(ValidationStatus::Unsupported),
        Some("cancelled") => Ok(ValidationStatus::Cancelled),
        Some("decoded") => Ok(ValidationStatus::Decoded),
        _ => Err("Unknown status in feed validation history".into()),
    }
}

fn parse_stage(stage: Option<&str>) -> Result<ValidationStage, String> {
    match stage {
        Some("not_checked") => Ok(ValidationStage::NotChecked),
        Some("endpoint_reached") => Ok(ValidationStage::EndpointReached),
        Some("stream_response") => Ok(ValidationStage::StreamResponse),
        Some("supported_codec") => Ok(ValidationStage::SupportedCodec),
        Some("decoded_audio") => Ok(ValidationStage::DecodedAudio),
        _ => Err("Unknown stage in feed validation history".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shared_contract_parses_without_io_and_recomputes_health() {
        let history =
            parse_feed_history(include_str!("../../../testdata/feed-health-v1.json")).unwrap();
        let station = history.stations["skylab"];
        assert_eq!(station.status, ValidationStatus::Decoded);
        assert_eq!(station.stage, ValidationStage::DecodedAudio);
        assert_eq!(station.attempts, 1);
        assert_eq!(station.consecutive_failures, 0);
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(station.health_at(now), FeedHealth::Fresh);
        assert_eq!(
            station.health_at(now + FEED_FRESHNESS_TTL),
            FeedHealth::Fresh
        );
        assert_eq!(
            station.health_at(now + FEED_FRESHNESS_TTL + Duration::from_secs(1)),
            FeedHealth::Stale
        );
    }

    #[test]
    fn rejects_unknown_and_inconsistent_saved_evidence() {
        let source = include_str!("../../../testdata/feed-health-v1.json");
        let mut invalid: Value = serde_json::from_str(source).unwrap();
        invalid["stations"]["skylab"]["last_stage"] = json!("endpoint_reached");
        assert!(parse_feed_history(&invalid.to_string()).is_err());
        invalid["stations"]["skylab"]["last_stage"] = json!("decoded_audio");
        invalid["stations"]["skylab"]["health"] = json!("failed");
        assert!(parse_feed_history(&invalid.to_string()).is_err());
        invalid["stations"]["not-a-station"] = invalid["stations"]["skylab"].clone();
        assert!(parse_feed_history(&invalid.to_string()).is_err());
        assert!(parse_feed_history(&" ".repeat(MAX_FEED_HISTORY_BYTES + 1)).is_err());

        let mut future_update: Value = serde_json::from_str(source).unwrap();
        let future = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60;
        future_update["updated_at_unix"] = json!(future);
        assert!(parse_feed_history(&future_update.to_string()).is_err());
    }
}
