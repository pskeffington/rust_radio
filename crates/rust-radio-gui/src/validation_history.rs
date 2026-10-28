//! Import bounded host-persisted health summaries without performing file or network I/O.

use rust_radio_core::directory::curated_stations;
use rust_radio_core::validation::{ValidationStatus, FEED_FRESHNESS_TTL};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_HISTORY_BYTES: usize = 64 * 1024;
const MAX_FAILURES: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SavedFeedValidation {
    pub status: ValidationStatus,
    pub last_decoded_at: Option<SystemTime>,
}

pub(super) fn parse_history(
    json: &str,
) -> Result<BTreeMap<&'static str, SavedFeedValidation>, String> {
    if json.len() > MAX_HISTORY_BYTES {
        return Err("Feed validation history exceeds 64 KiB".into());
    }
    let value: Value = serde_json::from_str(json)
        .map_err(|error| format!("Feed validation history is invalid JSON: {error}"))?;
    if value["schema_version"].as_u64() != Some(1) {
        return Err("Unsupported feed validation history schema".into());
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
        let status = parse_status(record["last_status"].as_str())?;
        let attempts = record["attempts"]
            .as_u64()
            .filter(|attempts| (1..=2).contains(attempts))
            .ok_or_else(|| format!("Invalid attempt count in feed history for {id}"))?;
        let _ = attempts;
        let failures = record["consecutive_failures"]
            .as_u64()
            .filter(|failures| *failures <= MAX_FAILURES)
            .ok_or_else(|| format!("Invalid failure count in feed history for {id}"))?;
        let _ = failures;
        let last_decoded = match record["last_decoded_at_unix"].as_u64() {
            Some(timestamp) => {
                let time = epoch(Some(timestamp), "last decoded")?;
                if time > checked_at {
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
            if last_decoded != Some(checked_at)
                || record["last_stage"].as_str() != Some("decoded_audio")
                || failures != 0
            {
                return Err(format!(
                    "Inconsistent decoded evidence in feed history for {id}"
                ));
            }
        } else if last_decoded.is_some_and(|time| time > checked_at) {
            return Err(format!("Invalid decoded evidence in feed history for {id}"));
        }
        parsed.insert(
            station.id,
            SavedFeedValidation {
                status,
                last_decoded_at: last_decoded,
            },
        );
    }
    Ok(parsed)
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
        Some("unavailable") => Ok(ValidationStatus::Unavailable),
        Some("invalid_stream") => Ok(ValidationStatus::InvalidStream),
        Some("unsupported") => Ok(ValidationStatus::Unsupported),
        Some("cancelled") => Ok(ValidationStatus::Cancelled),
        Some("decoded") => Ok(ValidationStatus::Decoded),
        Some("check_error") => Ok(ValidationStatus::Unavailable),
        _ => Err("Unknown status in feed validation history".into()),
    }
}

pub(super) fn label(validation: SavedFeedValidation, now: SystemTime) -> &'static str {
    match validation.status {
        ValidationStatus::Decoded => match validation
            .last_decoded_at
            .and_then(|time| now.duration_since(time).ok())
        {
            Some(age) if age <= FEED_FRESHNESS_TTL => "SAVED / FRESH",
            _ => "SAVED / STALE",
        },
        ValidationStatus::Unavailable => "SAVED / UNAVAILABLE",
        ValidationStatus::InvalidStream => "SAVED / INVALID STREAM",
        ValidationStatus::Unsupported => "SAVED / UNSUPPORTED",
        ValidationStatus::Cancelled | ValidationStatus::Unverified => "SAVED / UNVERIFIED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn history(last_status: &str, decoded_at: Option<u64>, checked_at: u64) -> String {
        json!({
            "schema_version": 1,
            "updated_at_unix": checked_at,
            "stations": {
                "skylab": {
                    "last_checked_at_unix": checked_at,
                    "last_status": last_status,
                    "last_stage": if last_status == "decoded" { "decoded_audio" } else { "endpoint_reached" },
                    "attempts": 1,
                    "last_decoded_at_unix": decoded_at,
                    "consecutive_failures": if last_status == "decoded" { 0 } else { 1 },
                    "health": if last_status == "decoded" { "fresh" } else { "failed" }
                }
            }
        })
        .to_string()
    }

    #[test]
    fn saved_success_expires_and_saved_failure_never_claims_freshness() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000_000);
        let fresh = parse_history(&history("decoded", Some(999_999), 999_999)).unwrap();
        assert_eq!(label(fresh["skylab"], now), "SAVED / FRESH");
        let stale_at = 1_000_000 - 7 * 60 * 60;
        let stale = parse_history(&history("decoded", Some(stale_at), stale_at)).unwrap();
        assert_eq!(label(stale["skylab"], now), "SAVED / STALE");
        let failed = parse_history(&history("unavailable", Some(999_999), 1_000_000)).unwrap();
        assert_eq!(label(failed["skylab"], now), "SAVED / UNAVAILABLE");
        let check_error = parse_history(&history("check_error", Some(999_999), 1_000_000)).unwrap();
        assert_eq!(label(check_error["skylab"], now), "SAVED / UNAVAILABLE");
    }

    #[test]
    fn invalid_or_forged_station_records_are_rejected() {
        assert!(parse_history("{}").is_err());
        assert!(parse_history(&history("decoded", Some(1_000_001), 1_000_000)).is_err());
        assert!(parse_history(&history("decoded", None, 1_000_000)).is_err());
    }

    #[test]
    fn gui_imports_shared_cli_history_and_recomputes_stale_health() {
        let history = include_str!("../../../testdata/feed-health-v1.json");
        let parsed = parse_history(history).unwrap();
        assert_eq!(label(parsed["skylab"], SystemTime::now()), "SAVED / STALE");
    }
}
