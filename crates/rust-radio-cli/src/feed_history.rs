//! Optional, bounded persistence for point-in-time curated feed validation.

use rust_radio::feed_history::{feed_history_health_at, parse_feed_history, FeedHistoryHealth};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_HISTORY_BYTES: u64 = 64 * 1024;
const REQUIRED_FAILED_SWEEPS: usize = 3;
const MAX_PROCESSED_SWEEP_IDS: usize = 64;
const MAX_SWEEP_ID_BYTES: usize = 80;
const HISTORY_LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const HISTORY_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

pub fn update_file(path: &Path, report: &Value) -> Result<Value, String> {
    let _lock = HistoryLock::acquire(path)?;
    let now = unix_time_seconds()?;
    update_file_locked(path, report, now)
}

fn update_file_locked(path: &Path, report: &Value, now: u64) -> Result<Value, String> {
    let previous = read(path)?;
    let history = merge_at(previous.as_deref(), report, now)?;
    let serialized = serde_json::to_vec_pretty(&history)
        .map_err(|error| format!("Unable to encode feed history: {error}"))?;
    let validated_text = std::str::from_utf8(&serialized)
        .map_err(|error| format!("Encoded feed history is not UTF-8: {error}"))?;
    parse_feed_history(validated_text)?;
    parse_outage_guard_at(&history, now)?;
    if previous.as_deref().is_some_and(|previous| {
        serde_json::from_str::<Value>(previous).is_ok_and(|value| value == history)
    }) {
        return Ok(history);
    }
    if serialized.len() as u64 > MAX_HISTORY_BYTES {
        return Err("Feed history exceeds 64 KiB".into());
    }
    if let Some(previous) = previous.as_deref() {
        write_atomic(&backup_path(path), previous.as_bytes())?;
    }
    write_atomic(path, &serialized)?;
    Ok(history)
}

fn unix_time_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("System clock predates Unix epoch: {error}"))
}

fn lock_path(path: &Path) -> PathBuf {
    let mut lock = path.as_os_str().to_owned();
    lock.push(".lock");
    PathBuf::from(lock)
}

struct HistoryLock {
    file: File,
    state: LockState,
}

#[cfg(unix)]
type LockState = ();

#[cfg(windows)]
type LockState = WindowsOverlapped;

impl HistoryLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        Self::acquire_with_timeout(path, HISTORY_LOCK_TIMEOUT)
    }

    fn acquire_with_timeout(path: &Path, timeout: Duration) -> Result<Self, String> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Cannot create feed history directory: {error}"))?;
        }

        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(lock_path(path))
            .map_err(|error| format!("Cannot open feed history lock: {error}"))?;
        let deadline = Instant::now() + timeout;
        let mut first_attempt = true;

        loop {
            if !first_attempt && Instant::now() >= deadline {
                return Err("Timed out waiting for feed history update lock".into());
            }
            first_attempt = false;
            match try_lock(&file) {
                Ok(state) => {
                    return Ok(Self { file, state });
                }
                Err(error) if is_lock_contention(&error) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Err("Timed out waiting for feed history update lock".into());
                    }
                    thread::sleep(HISTORY_LOCK_RETRY_INTERVAL.min(deadline - now));
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(format!("Cannot lock feed history: {error}")),
            }
        }
    }
}

impl Drop for HistoryLock {
    fn drop(&mut self) {
        unlock(&self.file, &mut self.state);
    }
}

#[cfg(unix)]
fn try_lock(file: &File) -> std::io::Result<LockState> {
    use std::os::fd::AsRawFd;

    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    // SAFETY: `flock` uses the valid file descriptor held by `file` and does
    // not retain pointers after the call.
    let result = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn unlock(file: &File, _state: &mut LockState) {
    use std::os::fd::AsRawFd;

    const LOCK_UN: i32 = 8;
    // SAFETY: `flock` uses the valid file descriptor held by `file`.
    let _ = unsafe { flock(file.as_raw_fd(), LOCK_UN) };
}

#[cfg(unix)]
fn is_lock_contention(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock
}

#[cfg(unix)]
extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

#[cfg(windows)]
#[repr(C)]
struct WindowsOverlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: *mut std::ffi::c_void,
}

#[cfg(windows)]
impl WindowsOverlapped {
    fn new() -> Self {
        Self {
            internal: 0,
            internal_high: 0,
            offset: 0,
            offset_high: 0,
            event: std::ptr::null_mut(),
        }
    }
}

#[cfg(windows)]
fn try_lock(file: &File) -> std::io::Result<LockState> {
    use std::os::windows::io::AsRawHandle;

    const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x0000_0002;
    const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x0000_0001;
    let mut overlapped = WindowsOverlapped::new();
    // SAFETY: The file handle is valid for the duration of the call, and the
    // initialized OVERLAPPED remains alive until the matching unlock.
    let result = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if result != 0 {
        Ok(overlapped)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn unlock(file: &File, state: &mut LockState) {
    use std::os::windows::io::AsRawHandle;

    // SAFETY: This is the same handle, byte range, and OVERLAPPED used to
    // acquire the lock, all of which remain valid until `HistoryLock` drops.
    let _ = unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, state) };
}

#[cfg(windows)]
fn is_lock_contention(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock || error.raw_os_error() == Some(33)
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LockFileEx(
        file: *mut std::ffi::c_void,
        flags: u32,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut WindowsOverlapped,
    ) -> i32;
    fn UnlockFileEx(
        file: *mut std::ffi::c_void,
        reserved: u32,
        bytes_low: u32,
        bytes_high: u32,
        overlapped: *mut WindowsOverlapped,
    ) -> i32;
}

fn backup_path(path: &Path) -> PathBuf {
    let mut backup = path.as_os_str().to_owned();
    backup.push(".bak");
    PathBuf::from(backup)
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

#[cfg(test)]
fn merge(previous_text: Option<&str>, report: &Value) -> Result<Value, String> {
    merge_at(previous_text, report, unix_time_seconds()?)
}

fn merge_at(previous_text: Option<&str>, report: &Value, now: u64) -> Result<Value, String> {
    if report["schema_version"].as_u64() != Some(1) {
        return Err("Unsupported validation report schema version".into());
    }
    let checked_at = report["checked_at_unix"]
        .as_u64()
        .ok_or_else(|| "Validation report has no valid checked_at_unix".to_owned())?;
    if checked_at > now {
        return Err("Validation report timestamp is in the future".into());
    }
    let sweep_id = report["sweep_id"]
        .as_str()
        .filter(|id| valid_sweep_id(id))
        .ok_or_else(|| "Validation report has no valid sweep_id".to_owned())?;
    if sweep_time_seconds(sweep_id) != checked_at {
        return Err("Validation report sweep ID timestamp is inconsistent".into());
    }
    let results = report["results"]
        .as_array()
        .ok_or_else(|| "Validation report has no results array".to_owned())?;
    if results.len() != rust_radio::directory::curated_stations().len() {
        return Err("Validation report does not cover the complete catalog".into());
    }
    let previous_history = previous_text.map(parse_feed_history).transpose()?;
    let previous_json = previous_history
        .as_ref()
        .map(|_| serde_json::from_str::<Value>(previous_text.unwrap_or_default()))
        .transpose()
        .map_err(|_| "Feed history is invalid JSON".to_owned())?;
    let previous_guard = previous_json
        .as_ref()
        .map(|history| parse_outage_guard_at(history, now))
        .transpose()?
        .unwrap_or_default();
    if previous_json.is_some()
        && (previous_guard
            .processed_sweep_ids
            .iter()
            .any(|id| id == sweep_id)
            || previous_guard
                .last_completed_sweep_id
                .as_deref()
                .is_some_and(|last| sweep_id <= last)
            || previous_guard
                .last_completed_at
                .is_some_and(|last| sweep_time_seconds(sweep_id) < last))
    {
        return previous_json.ok_or_else(|| "Missing prior feed history".to_owned());
    }
    let mut stations = Map::new();
    let mut seen = BTreeSet::new();
    let mut outcomes = BTreeMap::new();

    for result in results {
        let id = result["station_id"]
            .as_str()
            .ok_or_else(|| "Validation result has no station ID".to_owned())?;
        if !rust_radio::directory::curated_stations()
            .iter()
            .any(|station| station.id == id)
            || !seen.insert(id.to_owned())
        {
            return Err("Validation report has an unknown or duplicate station ID".into());
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
        if !matches!(
            status,
            "decoded"
                | "unavailable"
                | "invalid_stream"
                | "unsupported"
                | "check_error"
                | "cancelled"
                | "unverified"
        ) {
            return Err(format!("Validation result for {id} has unknown status"));
        }
        outcomes.insert(id.to_owned(), status.to_owned());
    }
    if seen.len() != rust_radio::directory::curated_stations().len() {
        return Err("Validation report does not cover the complete catalog".into());
    }

    let mut processed_sweep_ids = previous_guard.processed_sweep_ids.clone();
    processed_sweep_ids.push(sweep_id.to_owned());
    if processed_sweep_ids.len() > MAX_PROCESSED_SWEEP_IDS {
        processed_sweep_ids.remove(0);
    }
    let retained_sweeps: BTreeSet<_> = processed_sweep_ids.iter().cloned().collect();

    let decoded_count = outcomes
        .values()
        .filter(|status| *status == "decoded")
        .count();
    let host_outage = decoded_count == 0;
    let mut publisher_outages = BTreeSet::new();
    for station in rust_radio::directory::curated_stations() {
        let group: Vec<_> = rust_radio::directory::curated_stations()
            .iter()
            .filter(|candidate| candidate.broadcaster == station.broadcaster)
            .collect();
        if group.len() > 1
            && group.iter().all(|member| {
                outcomes
                    .get(member.id)
                    .is_some_and(|status| is_definitive_failure(status))
            })
        {
            publisher_outages.insert(station.broadcaster.to_owned());
        }
    }

    for result in results {
        let id = result["station_id"]
            .as_str()
            .ok_or_else(|| "Validation result has no station ID".to_owned())?;
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
        let previous_record = previous_history
            .as_ref()
            .and_then(|history| history.stations.get(id));
        let previous_decoded = previous_record
            .and_then(|record| record.last_decoded_at)
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs());
        let last_decoded = if decoded {
            Some(checked)
        } else {
            previous_decoded
        };
        let status_kind = match status {
            "decoded" => rust_radio::validation_types::ValidationStatus::Decoded,
            "unavailable" | "check_error" => {
                rust_radio::validation_types::ValidationStatus::Unavailable
            }
            "invalid_stream" => rust_radio::validation_types::ValidationStatus::InvalidStream,
            "unsupported" => rust_radio::validation_types::ValidationStatus::Unsupported,
            "cancelled" => rust_radio::validation_types::ValidationStatus::Cancelled,
            "unverified" => rust_radio::validation_types::ValidationStatus::Unverified,
            _ => return Err(format!("Validation result for {id} has unknown status")),
        };
        let decoded_at =
            last_decoded.map(|seconds| UNIX_EPOCH + std::time::Duration::from_secs(seconds));
        let health = match feed_history_health_at(
            status_kind,
            decoded_at,
            UNIX_EPOCH + std::time::Duration::from_secs(checked_at),
        ) {
            FeedHistoryHealth::Fresh => "fresh",
            FeedHistoryHealth::Degraded => "degraded",
            FeedHistoryHealth::Stale => "stale",
            FeedHistoryHealth::Unverified => "unverified",
            FeedHistoryHealth::Failed => "failed",
        };
        let mut failed_sweeps = previous_guard.stations.get(id).cloned().unwrap_or_default();
        failed_sweeps.retain(|failure| retained_sweeps.contains(&failure.sweep_id));
        let publisher = rust_radio::directory::curated_stations()
            .iter()
            .find(|station| station.id == id)
            .expect("validated station exists")
            .broadcaster;
        let publisher_outage = publisher_outages.contains(publisher);
        if decoded {
            failed_sweeps.clear();
        } else if !host_outage
            && !publisher_outage
            && is_definitive_failure(status)
            && failed_sweeps
                .last()
                .map(|failure| failure.sweep_id.as_str())
                != Some(sweep_id)
        {
            failed_sweeps.push(FailedSweep {
                sweep_id: sweep_id.to_owned(),
                checked_at,
            });
            if failed_sweeps.len() > REQUIRED_FAILED_SWEEPS {
                failed_sweeps.remove(0);
            }
        }
        let failures = if decoded {
            0
        } else {
            failed_sweeps.len() as u64
        };
        let classification = if host_outage || publisher_outage {
            "outage_watch"
        } else if failed_sweeps.len() >= REQUIRED_FAILED_SWEEPS {
            "confirmed_unavailable"
        } else if !failed_sweeps.is_empty() {
            "outage_watch"
        } else {
            "not_confirmed"
        };
        let stage = result["stage"].as_str().unwrap_or("not_checked");
        stations.insert(
            id.to_owned(),
            json!({
                "last_checked_at_unix": checked,
                "last_status": status,
                "last_stage": stage,
                "attempts": result["attempts"].as_u64().unwrap_or(1),
                "last_decoded_at_unix": last_decoded,
                "consecutive_failures": failures,
                "independent_failure_sweeps": failed_sweeps.iter().map(|failure| json!({
                    "sweep_id": failure.sweep_id,
                    "checked_at_unix": failure.checked_at,
                })).collect::<Vec<_>>(),
                "outage_classification": classification,
                "health": health,
            }),
        );
    }

    let station_classifications: Map<String, Value> = rust_radio::directory::curated_stations()
        .iter()
        .map(|station| {
            let record = stations
                .get(station.id)
                .expect("all curated stations have an output record");
            (
                station.id.to_owned(),
                json!({
                    "classification": record["outage_classification"],
                    "failed_sweeps": record["independent_failure_sweeps"],
                }),
            )
        })
        .collect();
    let outage_guard = json!({
        "state": if host_outage {
            "host_outage_watch"
        } else if !publisher_outages.is_empty() {
            "publisher_outage_watch"
        } else {
            "station_checks"
        },
        "required_failed_sweeps": REQUIRED_FAILED_SWEEPS,
        "last_completed_sweep_id": sweep_id,
        "last_completed_at_unix": checked_at,
        "processed_sweep_ids": processed_sweep_ids,
        "host_outage": host_outage,
        "publisher_outages": publisher_outages,
        "stations": station_classifications,
    });

    serde_json::to_value(json!({
        "schema_version": 2,
        "updated_at_unix": checked_at,
        "stations": stations,
        "outage_guard": outage_guard,
    }))
    .map_err(|error| format!("Unable to build feed history: {error}"))
}

#[derive(Clone, Debug, Default)]
struct OutageGuard {
    last_completed_sweep_id: Option<String>,
    last_completed_at: Option<u64>,
    processed_sweep_ids: Vec<String>,
    stations: BTreeMap<String, Vec<FailedSweep>>,
}

#[derive(Clone, Debug)]
struct FailedSweep {
    sweep_id: String,
    checked_at: u64,
}

fn valid_sweep_id(id: &str) -> bool {
    let mut parts = id.split('-');
    let (Some(time), Some(process), Some(counter), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    time.len() == 32
        && process.len() == 8
        && counter.len() == 16
        && id.len() <= MAX_SWEEP_ID_BYTES
        && [time, process, counter].iter().all(|part| {
            part.bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
}

fn sweep_time_seconds(id: &str) -> u64 {
    let timestamp = id
        .split('-')
        .next()
        .and_then(|time| u128::from_str_radix(time, 16).ok())
        .unwrap_or_default();
    (timestamp / 1_000_000_000).min(u128::from(u64::MAX)) as u64
}

fn is_definitive_failure(status: &str) -> bool {
    matches!(status, "unavailable" | "invalid_stream" | "unsupported")
}

#[cfg(test)]
fn parse_outage_guard(history: &Value) -> Result<OutageGuard, String> {
    parse_outage_guard_at(history, unix_time_seconds()?)
}

fn parse_outage_guard_at(history: &Value, now: u64) -> Result<OutageGuard, String> {
    let Some(value) = history.get("outage_guard") else {
        let last_completed_at = history["updated_at_unix"].as_u64();
        if last_completed_at.is_some_and(|timestamp| timestamp > now) {
            return Err("Feed history update timestamp is in the future".into());
        }
        return Ok(OutageGuard {
            last_completed_at,
            ..OutageGuard::default()
        });
    };
    let object = value
        .as_object()
        .ok_or_else(|| "Invalid outage guard in feed history".to_owned())?;
    let last_completed_sweep_id = object
        .get("last_completed_sweep_id")
        .and_then(Value::as_str)
        .filter(|id| valid_sweep_id(id))
        .ok_or_else(|| "Invalid last sweep ID in feed history".to_owned())?
        .to_owned();
    let last_completed_at = object
        .get("last_completed_at_unix")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Invalid last sweep time in feed history".to_owned())?;
    if last_completed_at > now {
        return Err("Last feed history sweep timestamp is in the future".into());
    }
    if last_completed_at != history["updated_at_unix"].as_u64().unwrap_or_default()
        || sweep_time_seconds(&last_completed_sweep_id) != last_completed_at
    {
        return Err("Outage sweep time does not match history update".into());
    }
    let host_outage = object
        .get("host_outage")
        .and_then(Value::as_bool)
        .ok_or_else(|| "Invalid host outage flag in feed history".to_owned())?;
    let state = object
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| "Invalid outage guard state in feed history".to_owned())?;
    if object.get("required_failed_sweeps").and_then(Value::as_u64)
        != Some(REQUIRED_FAILED_SWEEPS as u64)
    {
        return Err("Invalid outage confirmation threshold in feed history".into());
    }
    let publishers = object
        .get("publisher_outages")
        .and_then(Value::as_array)
        .ok_or_else(|| "Invalid publisher outage list in feed history".to_owned())?;
    let mut publisher_names = BTreeSet::new();
    if publishers.len() > rust_radio::directory::curated_stations().len()
        || publishers.iter().any(|publisher| {
            publisher.as_str().map_or(true, |name| {
                !publisher_names.insert(name.to_owned())
                    || !rust_radio::directory::curated_stations()
                        .iter()
                        .any(|station| station.broadcaster == name)
            })
        })
    {
        return Err("Unknown publisher outage in feed history".into());
    }
    let expected_state = if host_outage {
        "host_outage_watch"
    } else if !publishers.is_empty() {
        "publisher_outage_watch"
    } else {
        "station_checks"
    };
    if state != expected_state {
        return Err("Inconsistent outage guard state in feed history".into());
    }
    let processed = object
        .get("processed_sweep_ids")
        .and_then(Value::as_array)
        .filter(|ids| ids.len() <= MAX_PROCESSED_SWEEP_IDS)
        .ok_or_else(|| "Invalid processed sweep list in feed history".to_owned())?;
    let mut processed_sweep_ids = Vec::with_capacity(processed.len());
    let mut seen_processed = BTreeSet::new();
    let mut previous_processed: Option<&str> = None;
    for id in processed {
        let id = id
            .as_str()
            .filter(|id| valid_sweep_id(id))
            .ok_or_else(|| "Invalid processed sweep ID in feed history".to_owned())?;
        if sweep_time_seconds(id) > last_completed_at
            || !seen_processed.insert(id.to_owned())
            || previous_processed.is_some_and(|previous| id <= previous)
        {
            return Err("Processed sweep IDs are duplicated or out of order".into());
        }
        previous_processed = Some(id);
        processed_sweep_ids.push(id.to_owned());
    }
    if processed_sweep_ids.last().map(String::as_str)
        != object
            .get("last_completed_sweep_id")
            .and_then(Value::as_str)
    {
        return Err("Last completed sweep is not the newest processed sweep".into());
    }
    let stations = object
        .get("stations")
        .and_then(Value::as_object)
        .ok_or_else(|| "Invalid station outage map in feed history".to_owned())?;
    if stations.len() != rust_radio::directory::curated_stations().len() {
        return Err("Incomplete station outage metadata in feed history".into());
    }
    let history_stations = history["stations"]
        .as_object()
        .ok_or_else(|| "Missing station history records".to_owned())?;
    if history_stations.len() != rust_radio::directory::curated_stations().len() {
        return Err("Incomplete station history records".into());
    }
    let decoded_count = history_stations
        .values()
        .filter(|station| station["last_status"].as_str() == Some("decoded"))
        .count();
    if host_outage != (decoded_count == 0) {
        return Err("Host outage flag disagrees with station results".into());
    }
    let mut expected_publishers = BTreeSet::new();
    for station in rust_radio::directory::curated_stations() {
        let group: Vec<_> = rust_radio::directory::curated_stations()
            .iter()
            .filter(|member| member.broadcaster == station.broadcaster)
            .collect();
        if group.len() > 1
            && group.iter().all(|member| {
                history_stations
                    .get(member.id)
                    .and_then(|record| record["last_status"].as_str())
                    .is_some_and(is_definitive_failure)
            })
        {
            expected_publishers.insert(station.broadcaster);
        }
    }
    if publisher_names
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != expected_publishers
    {
        return Err("Publisher outage flags disagree with station results".into());
    }
    let mut parsed = BTreeMap::new();
    for (id, station) in stations {
        if !rust_radio::directory::curated_stations()
            .iter()
            .any(|known| known.id == id)
        {
            return Err(format!("Unknown outage station in feed history: {id}"));
        }
        let failed_sweeps = station["failed_sweeps"]
            .as_array()
            .filter(|sweeps| sweeps.len() <= REQUIRED_FAILED_SWEEPS)
            .ok_or_else(|| format!("Invalid failure sweep list for {id}"))?;
        let classification = station["classification"]
            .as_str()
            .filter(|value| {
                matches!(
                    *value,
                    "not_confirmed" | "outage_watch" | "confirmed_unavailable"
                )
            })
            .ok_or_else(|| format!("Invalid outage classification for {id}"))?;
        let mut seen = BTreeSet::new();
        let mut records = Vec::new();
        let mut previous_checked_at = None;
        let mut previous_sweep_id: Option<&str> = None;
        for sweep in failed_sweeps {
            let sweep_id = sweep["sweep_id"]
                .as_str()
                .filter(|value| valid_sweep_id(value))
                .ok_or_else(|| format!("Invalid failure sweep ID for {id}"))?;
            let checked_at = sweep["checked_at_unix"]
                .as_u64()
                .ok_or_else(|| format!("Invalid failure sweep time for {id}"))?;
            if checked_at > last_completed_at
                || sweep_time_seconds(sweep_id) != checked_at
                || previous_checked_at.is_some_and(|previous| checked_at < previous)
                || previous_sweep_id.is_some_and(|previous| sweep_id <= previous)
                || !seen.insert(sweep_id.to_owned())
                || !seen_processed.contains(sweep_id)
            {
                return Err(format!("Invalid or duplicate failure sweep ID for {id}"));
            }
            previous_checked_at = Some(checked_at);
            previous_sweep_id = Some(sweep_id);
            records.push(FailedSweep {
                sweep_id: sweep_id.to_owned(),
                checked_at,
            });
        }
        let station_record = history_stations
            .get(id)
            .ok_or_else(|| format!("Missing station history for {id}"))?;
        let current_status = station_record["last_status"]
            .as_str()
            .ok_or_else(|| format!("Missing station status for {id}"))?;
        if current_status == "decoded" && !records.is_empty() {
            return Err(format!(
                "Successful station check did not reset failures for {id}"
            ));
        }
        let station_publisher_outage = publishers
            .iter()
            .any(|publisher| publisher.as_str() == Some(history_station_publisher(id)));
        if is_definitive_failure(current_status)
            && !host_outage
            && !station_publisher_outage
            && records.last().map(|record| record.sweep_id.as_str())
                != Some(last_completed_sweep_id.as_str())
        {
            return Err(format!(
                "Current station failure is missing from its streak for {id}"
            ));
        }
        let expected_classification = if host_outage || station_publisher_outage {
            "outage_watch"
        } else if records.len() >= REQUIRED_FAILED_SWEEPS {
            "confirmed_unavailable"
        } else if records.is_empty() {
            "not_confirmed"
        } else {
            "outage_watch"
        };
        let record_failures = station_record["consecutive_failures"].as_u64();
        let record_sweeps = station_record["independent_failure_sweeps"].as_array();
        if classification != expected_classification
            || record_failures != Some(records.len() as u64)
            || record_sweeps != Some(failed_sweeps)
            || station_record["outage_classification"].as_str() != Some(classification)
        {
            return Err(format!("Inconsistent outage metadata for {id}"));
        }
        parsed.insert(id.clone(), records);
    }
    Ok(OutageGuard {
        last_completed_sweep_id: Some(last_completed_sweep_id),
        last_completed_at: Some(last_completed_at),
        processed_sweep_ids,
        stations: parsed,
    })
}

fn history_station_publisher(id: &str) -> &str {
    rust_radio::directory::curated_stations()
        .iter()
        .find(|station| station.id == id)
        .map(|station| station.broadcaster)
        .unwrap_or("")
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

    fn remove_test_files(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(backup_path(path));
        let _ = fs::remove_file(lock_path(path));
    }

    fn update_file_at(path: &Path, report: &Value, now: u64) -> Result<Value, String> {
        let _lock = HistoryLock::acquire(path)?;
        update_file_locked(path, report, now)
    }

    #[test]
    fn history_lock_waits_for_the_current_writer_and_releases() {
        use std::sync::mpsc;

        assert_eq!(HISTORY_LOCK_TIMEOUT, Duration::from_secs(5));
        let path = temp_path();
        let held = HistoryLock::acquire(&path).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let worker_path = path.clone();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let _lock = HistoryLock::acquire(&worker_path).unwrap();
            acquired_tx.send(()).unwrap();
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second writer should start");
        assert!(acquired_rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(held);
        acquired_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second writer should acquire after release");
        worker.join().unwrap();
        remove_test_files(&path);
    }

    #[test]
    fn history_lock_times_out_and_can_be_acquired_after_release() {
        let path = temp_path();
        let held = HistoryLock::acquire(&path).unwrap();
        let timeout = HistoryLock::acquire_with_timeout(&path, Duration::from_millis(25));
        assert!(matches!(timeout, Err(error) if error.contains("Timed out")));
        drop(held);

        let acquired = HistoryLock::acquire_with_timeout(&path, Duration::from_secs(1)).unwrap();
        drop(acquired);
        remove_test_files(&path);
    }

    #[test]
    fn concurrent_success_and_failure_writers_merge_after_the_previous_reset() {
        use std::sync::mpsc;

        let path = temp_path();
        update_file(&path, &report(1_700_000_100, "unavailable", false)).unwrap();
        update_file(&path, &report(1_700_000_101, "unavailable", false)).unwrap();
        assert_eq!(
            read(&path).unwrap().unwrap().parse::<Value>().unwrap()["stations"]["skylab"]
                ["consecutive_failures"],
            2
        );

        let held = HistoryLock::acquire(&path).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let mut workers = Vec::new();
        for (time, status, decoded) in [
            (1_700_000_102, "unavailable", false),
            (1_700_000_103, "decoded", true),
        ] {
            let path = path.clone();
            let started = started_tx.clone();
            let done = done_tx.clone();
            workers.push(std::thread::spawn(move || {
                started.send(()).unwrap();
                done.send(update_file(&path, &report(time, status, decoded)).map(|_| ()))
                    .unwrap();
            }));
        }
        drop(started_tx);
        drop(done_tx);
        for _ in 0..2 {
            started_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("both writer threads should start");
        }
        assert!(done_rx.recv_timeout(Duration::from_millis(50)).is_err());
        drop(held);
        for _ in 0..2 {
            done_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("writer should finish after lock release")
                .unwrap();
        }
        for worker in workers {
            worker.join().unwrap();
        }

        let saved: Value = serde_json::from_str(&read(&path).unwrap().unwrap()).unwrap();
        parse_outage_guard(&saved).unwrap();
        assert_eq!(saved["stations"]["skylab"]["consecutive_failures"], 0);
        assert_ne!(
            saved["outage_guard"]["stations"]["skylab"]["classification"],
            "confirmed_unavailable"
        );
        remove_test_files(&path);
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
                        "status": "decoded",
                        "stage": "decoded_audio",
                        "checked_at_unix": time,
                        "attempts": 1,
                        "decoded": true
                    })
                }
            })
            .collect();
        json!({
            "schema_version": 1,
            "sweep_id": format!("{:032x}-00000001-{:016x}", u128::from(time) * 1_000_000_000, time),
            "checked_at_unix": time,
            "results": results
        })
    }

    fn report_with_failures(time: u64, failed_ids: &[&str]) -> Value {
        let mut value = report(time, "decoded", true);
        for result in value["results"].as_array_mut().unwrap() {
            let id = result["station_id"].as_str().unwrap();
            if failed_ids.contains(&id) {
                result["status"] = json!("unavailable");
                result["stage"] = json!("not_checked");
                result["decoded"] = json!(false);
                result["checked_at_unix"] = json!(time);
            }
        }
        value
    }

    fn full_outage_report(time: u64) -> Value {
        let mut value = report_with_failures(time, &[]);
        for result in value["results"].as_array_mut().unwrap() {
            result["status"] = json!("unavailable");
            result["stage"] = json!("not_checked");
            result["decoded"] = json!(false);
        }
        value
    }

    #[test]
    fn history_preserves_last_decode_and_tracks_consecutive_failures_atomically() {
        let path = temp_path();
        let backup = backup_path(&path);
        update_file(&path, &report(1_000_000, "decoded", true)).unwrap();
        let first_report = fs::read(&path).unwrap();
        assert!(!backup.exists());
        update_file(&path, &report(1_000_100, "unavailable", false)).unwrap();
        assert_eq!(fs::read(&backup).unwrap(), first_report);
        assert!(fs::metadata(&backup).unwrap().len() <= MAX_HISTORY_BYTES);
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let previous: Value = serde_json::from_slice(&fs::read(&backup).unwrap()).unwrap();
        assert_eq!(saved["schema_version"].as_u64(), Some(2));
        assert_eq!(saved["stations"]["skylab"]["last_status"], "unavailable");
        assert_eq!(previous["updated_at_unix"], 1_000_000);
        assert_eq!(
            saved["stations"]["skylab"]["last_decoded_at_unix"],
            1_000_000
        );
        assert_eq!(saved["stations"]["skylab"]["consecutive_failures"], 1);
        assert_eq!(saved["stations"]["skylab"]["health"], "degraded");
        remove_test_files(&path);
    }

    #[test]
    fn invalid_history_is_not_overwritten() {
        let path = temp_path();
        let backup = backup_path(&path);
        fs::write(&path, "not-json").unwrap();
        fs::write(&backup, "last-known-backup").unwrap();
        assert!(update_file(&path, &report(1_000_000, "decoded", true)).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "not-json");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "last-known-backup");
        remove_test_files(&path);
    }

    #[test]
    fn invalid_merged_history_is_rejected_before_creating_a_file() {
        let path = temp_path();
        let mut report = report(1_700_000_100, "unavailable", false);
        report["results"][0]["stage"] = json!("unknown_stage");
        assert!(update_file(&path, &report).is_err());
        assert!(!path.exists());
        remove_test_files(&path);
    }

    #[test]
    fn backup_failure_leaves_the_primary_history_unchanged() {
        let path = temp_path();
        update_file(&path, &report(1_000_000, "decoded", true)).unwrap();
        let primary = fs::read(&path).unwrap();
        let backup = backup_path(&path);
        fs::create_dir(&backup).unwrap();

        assert!(update_file(&path, &report(1_000_100, "unavailable", false)).is_err());
        assert_eq!(fs::read(&path).unwrap(), primary);

        let _ = fs::remove_dir(backup);
        remove_test_files(&path);
    }

    #[cfg(unix)]
    #[test]
    fn backup_history_is_private_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_path();
        let backup = backup_path(&path);
        update_file(&path, &report(1_000_000, "decoded", true)).unwrap();
        update_file(&path, &report(1_000_100, "unavailable", false)).unwrap();
        let mode = fs::metadata(&backup).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        remove_test_files(&path);
    }

    #[test]
    fn inconsistent_history_is_rejected_before_rewrite() {
        let path = temp_path();
        let mut invalid: Value =
            serde_json::from_str(include_str!("../../../testdata/feed-health-v1.json")).unwrap();
        invalid["stations"]["skylab"]["last_stage"] = json!("endpoint_reached");
        fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(update_file(&path, &report(1_700_000_100, "unavailable", false)).is_err());
        assert_eq!(
            fs::read(&path).unwrap(),
            serde_json::to_vec(&invalid).unwrap()
        );
        remove_test_files(&path);
    }

    #[test]
    fn transport_errors_without_stage_produce_valid_history_records() {
        let mut report = report(1_700_000_100, "check_error", false);
        let result = report["results"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|result| result["station_id"] == "skylab")
            .unwrap();
        result.as_object_mut().unwrap().remove("stage");
        result.as_object_mut().unwrap().remove("checked_at_unix");

        let updated = merge(None, &report).unwrap();
        assert_eq!(updated["stations"]["skylab"]["last_status"], "check_error");
        assert_eq!(updated["stations"]["skylab"]["last_stage"], "not_checked");
        assert_eq!(updated["stations"]["skylab"]["health"], "failed");
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
        assert_eq!(updated["schema_version"], 2);
        assert_eq!(updated["stations"]["skylab"]["health"], "degraded");
    }

    #[test]
    fn outage_confirmation_requires_three_distinct_completed_failed_sweeps_and_success_resets() {
        let mut previous: Option<Value> = None;
        for (index, time) in [1_700_000_100, 1_700_000_200, 1_700_000_300]
            .into_iter()
            .enumerate()
        {
            let report = report(time, "unavailable", false);
            let previous_text = previous.as_ref().map(Value::to_string);
            let merged = merge(previous_text.as_deref(), &report).unwrap();
            let expected = if index == 2 {
                "confirmed_unavailable"
            } else {
                "outage_watch"
            };
            assert_eq!(
                merged["outage_guard"]["stations"]["skylab"]["classification"],
                expected
            );
            assert_eq!(
                merged["stations"]["skylab"]["consecutive_failures"],
                index as u64 + 1
            );
            previous = Some(merged);
        }

        let previous_text = previous.as_ref().unwrap().to_string();
        let recovered = merge(
            Some(&previous_text),
            &report(1_700_000_400, "decoded", true),
        )
        .unwrap();
        assert_eq!(
            recovered["outage_guard"]["stations"]["skylab"]["classification"],
            "not_confirmed"
        );
        assert_eq!(recovered["stations"]["skylab"]["consecutive_failures"], 0);
        assert_eq!(
            recovered["stations"]["skylab"]["last_decoded_at_unix"],
            1_700_000_400
        );
    }

    #[test]
    fn partial_or_duplicate_catalog_sweeps_are_rejected() {
        let mut partial = report(1_700_000_100, "unavailable", false);
        partial["results"].as_array_mut().unwrap().pop();
        assert!(merge(None, &partial).is_err());

        let mut duplicate = report(1_700_000_100, "unavailable", false);
        let first = duplicate["results"][0].clone();
        let results = duplicate["results"].as_array_mut().unwrap();
        let last = results.len() - 1;
        results[last] = first;
        assert!(merge(None, &duplicate).is_err());
    }

    #[test]
    fn replay_after_an_intervening_sweep_is_idempotent() {
        let first = report(1_700_000_100, "unavailable", false);
        let after_first = merge(None, &first).unwrap();
        let after_first_text = after_first.to_string();
        let second = report(1_700_000_200, "unavailable", false);
        let after_second = merge(Some(&after_first_text), &second).unwrap();
        let after_second_text = after_second.to_string();
        let replayed = merge(Some(&after_second_text), &first).unwrap();
        assert_eq!(replayed, after_second);
        assert_eq!(replayed["stations"]["skylab"]["consecutive_failures"], 2);
    }

    #[test]
    fn distinct_completed_sweeps_in_the_same_second_are_counted() {
        let time = 1_700_000_100;
        let first = report(time, "unavailable", false);
        let after_first = merge(None, &first).unwrap();
        let previous_text = after_first.to_string();
        let mut second = report(time, "unavailable", false);
        second["sweep_id"] = json!(format!(
            "{:032x}-00000001-0000000000000002",
            u128::from(time) * 1_000_000_000 + 1
        ));
        let after_second = merge(Some(&previous_text), &second).unwrap();
        assert_eq!(
            after_second["stations"]["skylab"]["consecutive_failures"],
            2
        );
    }

    #[test]
    fn complete_zero_decode_sweep_is_recorded_as_host_outage_and_suppresses_station_counts() {
        let merged = merge(None, &full_outage_report(1_700_000_100)).unwrap();
        assert_eq!(merged["outage_guard"]["host_outage"], true);
        for station in rust_radio::directory::curated_stations() {
            assert_eq!(merged["stations"][station.id]["consecutive_failures"], 0);
            assert_eq!(
                merged["outage_guard"]["stations"][station.id]["classification"],
                "outage_watch"
            );
        }
    }

    #[test]
    fn suppressed_sweeps_expire_failure_references_with_the_processed_window() {
        let mut history = merge(None, &report(1_700_000_100, "decoded", true)).unwrap();
        for time in [1_700_000_101, 1_700_000_102] {
            history = merge(
                Some(&history.to_string()),
                &report(time, "unavailable", false),
            )
            .unwrap();
        }
        assert_eq!(history["stations"]["skylab"]["consecutive_failures"], 2);

        for offset in 0..=MAX_PROCESSED_SWEEP_IDS as u64 {
            let sweep = full_outage_report(1_700_000_103 + offset);
            history = merge(Some(&history.to_string()), &sweep).unwrap();
            parse_outage_guard(&history).expect("merged history must stay internally valid");
        }

        assert_eq!(
            history["outage_guard"]["processed_sweep_ids"]
                .as_array()
                .unwrap()
                .len(),
            MAX_PROCESSED_SWEEP_IDS
        );
        assert_eq!(
            history["outage_guard"]["stations"]["skylab"]["failed_sweeps"],
            json!([])
        );
        assert_eq!(history["stations"]["skylab"]["consecutive_failures"], 0);
        parse_feed_history(&history.to_string()).unwrap();
    }

    #[test]
    fn shared_publisher_outage_suppresses_member_counters_but_mixed_results_do_not() {
        let shared = merge(
            None,
            &report_with_failures(1_700_000_100, &["wcrb", "gbh-news"]),
        )
        .unwrap();
        assert_eq!(shared["outage_guard"]["host_outage"], false);
        assert_eq!(shared["outage_guard"]["publisher_outages"][0], "GBH");
        assert_eq!(shared["stations"]["wcrb"]["consecutive_failures"], 0);
        assert_eq!(shared["stations"]["gbh-news"]["consecutive_failures"], 0);

        let mixed = merge(None, &report_with_failures(1_700_000_100, &["wcrb"])).unwrap();
        assert_eq!(
            mixed["outage_guard"]["publisher_outages"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(mixed["stations"]["wcrb"]["consecutive_failures"], 1);
    }

    #[test]
    fn malformed_or_oversized_outage_metadata_is_rejected() {
        let mut history = merge(None, &report(1_700_000_100, "unavailable", false)).unwrap();
        history["outage_guard"]["publisher_outages"] = json!(["unknown publisher"]);
        assert!(parse_outage_guard(&history).is_err());

        let mut history = merge(None, &report(1_700_000_100, "unavailable", false)).unwrap();
        history["outage_guard"]["stations"]["skylab"]["failed_sweeps"][0]["checked_at_unix"] =
            json!(1_700_000_101);
        assert!(parse_outage_guard(&history).is_err());

        let mut history = merge(None, &report(1_700_000_100, "unavailable", false)).unwrap();
        history["outage_guard"]["processed_sweep_ids"] = json!(vec![
            history["outage_guard"]
                ["last_completed_sweep_id"]
                .clone();
            MAX_PROCESSED_SWEEP_IDS + 1
        ]);
        assert!(parse_outage_guard(&history).is_err());
    }

    #[test]
    fn outage_flags_must_match_the_complete_sweep_results() {
        let mut history = merge(None, &report(1_700_000_100, "unavailable", false)).unwrap();
        history["outage_guard"]["host_outage"] = json!(true);
        history["outage_guard"]["state"] = json!("host_outage_watch");
        assert!(parse_outage_guard(&history).is_err());

        let mut history = merge(None, &report(1_700_000_100, "unavailable", false)).unwrap();
        history["outage_guard"]["publisher_outages"] = json!(["GBH"]);
        history["outage_guard"]["state"] = json!("publisher_outage_watch");
        assert!(parse_outage_guard(&history).is_err());
    }

    #[test]
    fn future_sweep_id_timestamp_cannot_poison_outage_history() {
        let mut history = merge(None, &report(1_700_000_100, "decoded", true)).unwrap();
        let future_id = format!(
            "{:032x}-00000001-0000000000000002",
            u128::from(1_700_000_101u64) * 1_000_000_000
        );
        history["outage_guard"]["last_completed_sweep_id"] = json!(&future_id);
        history["outage_guard"]["processed_sweep_ids"][0] = json!(future_id);
        assert!(parse_outage_guard(&history).is_err());
    }

    #[test]
    fn incoming_future_sweep_is_rejected_before_history_ordering_checks() {
        let now = 1_700_000_100;
        let future_report = report(now + 1, "decoded", true);
        assert!(merge_at(None, &future_report, now)
            .unwrap_err()
            .contains("future"));

        let valid_past_history = merge_at(None, &report(now - 1, "decoded", true), now).unwrap();
        let history_text = serde_json::to_string(&valid_past_history).unwrap();
        assert!(merge_at(Some(&history_text), &future_report, now)
            .unwrap_err()
            .contains("future"));
    }

    #[test]
    fn future_persisted_guard_and_legacy_timestamp_are_rejected() {
        let now = 1_700_000_100;
        let mut history = merge_at(None, &report(now - 1, "decoded", true), now).unwrap();
        let future_time = now + 1;
        let future_id = format!(
            "{:032x}-00000001-0000000000000002",
            u128::from(future_time) * 1_000_000_000
        );
        history["updated_at_unix"] = json!(future_time);
        history["outage_guard"]["last_completed_at_unix"] = json!(future_time);
        history["outage_guard"]["last_completed_sweep_id"] = json!(&future_id);
        history["outage_guard"]["processed_sweep_ids"] = json!([future_id]);
        assert!(parse_outage_guard_at(&history, now)
            .unwrap_err()
            .contains("future"));
        let future_history_text = serde_json::to_string(&history).unwrap();
        assert!(merge_at(
            Some(&future_history_text),
            &report(now, "decoded", true),
            now
        )
        .unwrap_err()
        .contains("future"));

        let legacy: Value =
            serde_json::from_str(include_str!("../../../testdata/feed-health-v1.json")).unwrap();
        let mut future_legacy = legacy;
        future_legacy["updated_at_unix"] = json!(future_time);
        assert!(parse_outage_guard_at(&future_legacy, now)
            .unwrap_err()
            .contains("future"));
        let future_legacy_text = serde_json::to_string(&future_legacy).unwrap();
        assert!(merge_at(
            Some(&future_legacy_text),
            &report(now, "decoded", true),
            now
        )
        .unwrap_err()
        .contains("future"));
    }

    #[test]
    fn rejected_future_report_preserves_primary_and_backup_bytes() {
        let path = temp_path();
        let original =
            serde_json::to_vec(&merge(None, &report(1_700_000_100, "decoded", true)).unwrap())
                .unwrap();
        let backup = backup_path(&path);
        let backup_bytes = b"prior-backup-checkpoint";
        fs::write(&path, &original).unwrap();
        fs::write(&backup, backup_bytes).unwrap();

        let result = update_file_at(
            &path,
            &report(1_700_000_102, "decoded", true),
            1_700_000_101,
        );
        assert!(result.unwrap_err().contains("future"));
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(&backup).unwrap(), backup_bytes);
        remove_test_files(&path);
    }

    #[test]
    fn current_and_past_sweeps_are_accepted_by_injected_clock() {
        let now = 1_700_000_100;
        assert!(merge_at(None, &report(now, "decoded", true), now).is_ok());
        assert!(merge_at(None, &report(now - 1, "decoded", true), now).is_ok());
    }

    #[test]
    fn cancelled_and_unverified_checks_preserve_good_proof_without_counting_failures() {
        let previous = include_str!("../../../testdata/feed-health-v1.json");
        for status in ["cancelled", "unverified"] {
            let updated = merge(Some(previous), &report(1_700_000_100, status, false)).unwrap();
            assert_eq!(
                updated["stations"]["skylab"]["last_decoded_at_unix"],
                1_700_000_000
            );
            assert_eq!(updated["stations"]["skylab"]["consecutive_failures"], 0);
            assert_eq!(updated["stations"]["skylab"]["health"], "unverified");
        }
    }

    #[test]
    fn updating_a_v1_file_backs_it_up_and_writes_v2() {
        let path = temp_path();
        let backup = backup_path(&path);
        let original = include_str!("../../../testdata/feed-health-v1.json");
        fs::write(&path, original).unwrap();
        update_file(&path, &report(1_700_000_100, "unavailable", false)).unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved["schema_version"], 2);
        assert_eq!(fs::read_to_string(&backup).unwrap(), original);
        remove_test_files(&path);
    }
}
