//! Import bounded host-persisted health summaries without performing file or network I/O.

use rust_radio_core::feed_history::parse_feed_history;
use rust_radio_core::validation::{ValidationStatus, FEED_FRESHNESS_TTL};
use std::collections::BTreeMap;
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SavedFeedValidation {
    pub status: ValidationStatus,
    pub last_decoded_at: Option<SystemTime>,
}

pub(super) fn parse_history(
    json: &str,
) -> Result<BTreeMap<&'static str, SavedFeedValidation>, String> {
    let history = parse_feed_history(json)?;
    Ok(history
        .stations
        .into_iter()
        .map(|(id, record)| {
            (
                id,
                SavedFeedValidation {
                    status: record.status,
                    last_decoded_at: record.last_decoded_at,
                },
            )
        })
        .collect())
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
    use serde_json::Value;
    use std::time::{Duration, UNIX_EPOCH};

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
        let mut wrong_stage: Value =
            serde_json::from_str(&history("decoded", Some(1_000_000), 1_000_000)).unwrap();
        wrong_stage["stations"]["skylab"]["last_stage"] = Value::String("endpoint_reached".into());
        assert!(parse_history(&wrong_stage.to_string()).is_err());
        let mut future_update: Value =
            serde_json::from_str(&history("decoded", Some(1_000_000), 1_000_000)).unwrap();
        future_update["updated_at_unix"] = json!(999_999);
        assert!(parse_history(&future_update.to_string()).is_err());
    }

    #[test]
    fn gui_imports_shared_cli_history_and_recomputes_stale_health() {
        let history = include_str!("../../../testdata/feed-health-v1.json");
        let parsed = parse_history(history).unwrap();
        assert_eq!(label(parsed["skylab"], SystemTime::now()), "SAVED / STALE");
    }
}
