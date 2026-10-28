//! Optional, bounded persistence for point-in-time curated feed validation.

use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_HISTORY_BYTES: u64 = 64 * 1024;
const FRESHNESS_TTL: u64 = 6 * 60 * 60;

pub fn update_file(path: &Path, report: &Value) -> Result<(), String> {
    let previous = read(path)?;
    let history = merge(previous.as_deref(), report)?;
    let bytes = serde_json::to_vec_pretty(&history)
        .map_err(|error| format!("Unable to encode feed history: {error}"))?;
    if bytes.len() as u64 > MAX_HISTORY_BYTES {
        return Err("Feed history exceeds 64 KiB".into());
    }
    write_atomic(path, &bytes)
}

fn read(path: &Path) -> Result<Option<String>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read feed history: {error}")),
    };
    if file
        .metadata()
        .map_err(|error| format!("Cannot inspect feed history: {error}"))?
        .len()
        > MAX_HISTORY_BYTES
    {
        return Err("Feed history exceeds 64 KiB".into());
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_HISTORY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read feed history: {error}"))?;
    if bytes.len() as u64 > MAX_HISTORY_BYTES {
        return Err("Feed history exceeds 64 KiB".into());
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "Feed history is not UTF-8".into())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create feed history directory: {error}"))?;
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("System clock predates Unix epoch: {error}"))?
        .as_nanos();
    let temp = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|error| format!("Cannot save feed history: {error}"))
}

fn merge(previous: Option<&str>, report: &Value) -> Result<Value, String> {
    if report["schema_version"].as_u64() != Some(1) {
        return Err("Unsupported validation report schema version".into());
    }
    let checked_at = report["checked_at_unix"]
        .as_u64()
        .ok_or_else(|| "Validation report has no valid checked_at_unix".to_owned())?;
    let results = report["results"]
        .as_array()
        .ok_or_else(|| "Validation report has no results array".to_owned())?;
    if results.len() != rust_radio::directory::curated_stations().len() {
        return Err("Validation report does not cover the complete catalog".into());
    }
    let mut stations = match previous {
        Some(json) => parse_previous(json)?,
        None => Map::new(),
    };
    let mut seen = BTreeSet::new();

    for result in results {
        let id = result["station_id"]
            .as_str()
            .ok_or_else(|| "Validation result has no station ID".to_owned())?;
        if !rust_radio::directory::curated_stations()
            .iter()
            .any(|station| station.id == id)
        {
            return Err(format!(
                "Validation report contains unknown station ID: {id}"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "Validation report contains duplicate station ID: {id}"
            ));
        }
        let status = result["status"]
            .as_str()
            .ok_or_else(|| format!("Validation result for {id} has no status"))?;
        let decoded = result["decoded"] == true;
        if decoded != (status == "decoded") {
            return Err(format!(
                "Validation result for {id} has inconsistent decode status"
            ));
        }
        let checked = result["checked_at_unix"].as_u64().unwrap_or(checked_at);
        let previous_record = stations.get(id);
        let previous_decoded =
            previous_record.and_then(|record| record["last_decoded_at_unix"].as_u64());
        let previous_failures = previous_record
            .and_then(|record| record["consecutive_failures"].as_u64())
            .unwrap_or(0);
        let last_decoded = if decoded {
            Some(checked)
        } else {
            previous_decoded
        };
        let health = if decoded {
            if checked <= checked_at && checked_at - checked <= FRESHNESS_TTL {
                "fresh"
            } else {
                "stale"
            }
        } else if matches!(status, "unverified" | "cancelled") {
            "unverified"
        } else {
            "failed"
        };
        let failures = if decoded {
            0
        } else {
            previous_failures.saturating_add(1)
        };
        stations.insert(
            id.to_owned(),
            json!({
                "last_checked_at_unix": checked,
                "last_status": status,
                "last_stage": result["stage"],
                "attempts": result["attempts"].as_u64().unwrap_or(1),
                "last_decoded_at_unix": last_decoded,
                "consecutive_failures": failures,
                "health": health,
            }),
        );
    }

    serde_json::to_value(json!({
        "schema_version": 1,
        "updated_at_unix": checked_at,
        "stations": stations,
    }))
    .map_err(|error| format!("Unable to build feed history: {error}"))
}

fn parse_previous(raw: &str) -> Result<Map<String, Value>, String> {
    let previous: Value = serde_json::from_str(raw)
        .map_err(|error| format!("Feed history is invalid JSON: {error}"))?;
    if previous["schema_version"].as_u64() != Some(1) {
        return Err("Unsupported feed history schema version".into());
    }
    let stations = previous["stations"]
        .as_object()
        .ok_or_else(|| "Feed history has no stations object".to_owned())?;
    let valid_ids: Vec<_> = rust_radio::directory::curated_stations()
        .iter()
        .map(|station| station.id)
        .collect();
    if stations.len() > valid_ids.len() {
        return Err("Feed history contains too many stations".into());
    }
    for (id, record) in stations {
        if !valid_ids.contains(&id.as_str())
            || record["last_checked_at_unix"].as_u64().is_none()
            || record["consecutive_failures"].as_u64().is_none()
            || record["consecutive_failures"].as_u64().unwrap_or(u64::MAX) > 1_000_000
        {
            return Err("Feed history contains an invalid station record".into());
        }
        if record["last_decoded_at_unix"].as_u64().is_none()
            && !record["last_decoded_at_unix"].is_null()
        {
            return Err("Feed history has an invalid last decoded timestamp".into());
        }
    }
    Ok(stations.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rust-radio-feed-history-{}-{}.json",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn report(time: u64, status: &str, decoded: bool) -> Value {
        let results: Vec<_> = rust_radio::directory::curated_stations()
            .iter()
            .map(|station| {
                if station.id == "skylab" {
                    json!({
                        "station_id": station.id,
                        "status": status,
                        "stage": if decoded { "decoded_audio" } else { "endpoint_reached" },
                        "checked_at_unix": time,
                        "attempts": 1,
                        "decoded": decoded
                    })
                } else {
                    json!({
                        "station_id": station.id,
                        "status": "unverified",
                        "stage": "not_checked",
                        "checked_at_unix": time,
                        "attempts": 1,
                        "decoded": false
                    })
                }
            })
            .collect();
        json!({
            "schema_version": 1,
            "checked_at_unix": time,
            "results": results
        })
    }

    #[test]
    fn history_preserves_last_decode_and_tracks_consecutive_failures_atomically() {
        let path = temp_path();
        update_file(&path, &report(1_000_000, "decoded", true)).unwrap();
        update_file(&path, &report(1_000_100, "unavailable", false)).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["schema_version"].as_u64(), Some(1));
        assert_eq!(saved["stations"]["skylab"]["last_status"], "unavailable");
        assert_eq!(
            saved["stations"]["skylab"]["last_decoded_at_unix"],
            1_000_000
        );
        assert_eq!(saved["stations"]["skylab"]["consecutive_failures"], 1);
        assert_eq!(saved["stations"]["skylab"]["health"], "failed");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_history_is_not_overwritten() {
        let path = temp_path();
        fs::write(&path, "not-json").unwrap();
        assert!(update_file(&path, &report(1_000_000, "decoded", true)).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "not-json");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn cli_merges_the_shared_v1_history_fixture() {
        let previous = include_str!("../../../testdata/feed-health-v1.json");
        let updated = merge(Some(previous), &report(1_700_000_100, "unavailable", false)).unwrap();
        assert_eq!(updated["stations"]["skylab"]["last_status"], "unavailable");
        assert_eq!(
            updated["stations"]["skylab"]["last_decoded_at_unix"],
            1_700_000_000
        );
        assert_eq!(updated["stations"]["skylab"]["consecutive_failures"], 1);
    }
}
