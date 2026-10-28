//! Headless, receive-only auto-validator for the bounded curated radio directory.
//! No GUI, audio output, or SDR device is opened. All network I/O is explicit.
use rust_radio::directory::{curated_stations, InternetStation};
use rust_radio::validation::validate_station;
use rust_radio::validation_types::{StationValidation, FEED_FRESHNESS_TTL};
use serde_json::{json, Value};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_INTERVAL_MINUTES: u64 = 240;
const MIN_INTERVAL_MINUTES: u64 = 15;
const MAX_INTERVAL_MINUTES: u64 = 240;
const INTER_STATION_PAUSE: Duration = Duration::from_millis(500);
const MAX_REPORT_BYTES: u64 = 512 * 1024;
const REDUNDANT_PROBE_ATTEMPTS: usize = 2;
const PROBE_RETRY_DELAY: Duration = Duration::from_millis(1_500);
const FAILURE_SWEEPS_TO_UNAVAILABLE: u64 = 3;
const LAST_GOOD_GRACE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Once,
    Daemon,
    Status,
    Eligible,
    StrictEligible,
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    mode: Mode,
    output: PathBuf,
    interval: Duration,
}

fn help() {
    println!(
        r#"Usage: rust-radio-feed-validator [--once|--daemon|--status|--eligible|--strict-eligible] [--output PATH] [--interval-minutes 15..240]
--once: scan curated stations; return nonzero for ineligible stations
--daemon: scan immediately and repeat every four hours
--status: summarize health, applying the six-hour TTL
--eligible: output JSON IDs for fresh or temporarily degraded stations
--strict-eligible: output only IDs with fresh decoded audio
Default output: target/radio-feed-health.json"#
    );
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut mode = Mode::Once;
    let mut mode_supplied = false;
    let mut output = PathBuf::from("target/radio-feed-health.json");
    let mut interval_minutes = DEFAULT_INTERVAL_MINUTES;
    let mut interval_supplied = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--once" | "--daemon" | "--status" | "--eligible" | "--strict-eligible" => {
                if mode_supplied {
                    return Err("Supply only one mode".into());
                }
                mode = match args[i].as_str() {
                    "--daemon" => Mode::Daemon,
                    "--status" => Mode::Status,
                    "--eligible" => Mode::Eligible,
                    "--strict-eligible" => Mode::StrictEligible,
                    _ => Mode::Once,
                };
                mode_supplied = true;
            }
            "--output" => {
                i += 1;
                let path = args.get(i).ok_or("Missing --output path")?;
                if path.is_empty() || path.starts_with('-') {
                    return Err("Invalid output path".into());
                }
                output = PathBuf::from(path);
            }
            "--interval-minutes" => {
                i += 1;
                interval_minutes = args
                    .get(i)
                    .ok_or("Missing --interval-minutes value")?
                    .parse::<u64>()
                    .map_err(|_| "Invalid scan interval")?;
                if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&interval_minutes) {
                    return Err("Interval must be 15..240 minutes".into());
                }
                interval_supplied = true;
            }
            other => return Err(format!("Unknown option: {other}")),
        }
        i += 1;
    }
    if interval_supplied && mode != Mode::Daemon {
        return Err("--interval-minutes requires --daemon".into());
    }
    Ok(Options {
        mode,
        output,
        interval: Duration::from_secs(interval_minutes * 60),
    })
}

fn unix_seconds(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn validation_value(result: Result<StationValidation, String>, now: SystemTime) -> Value {
    match result {
        Ok(report) => json!({
            "status": format!("{:?}", report.status),
            "stage": format!("{:?}", report.stage),
            "checked_at_unix": report.checked_at.and_then(unix_seconds),
            "codec": report.codec,
            "sample_rate": report.sample_rate,
            "channels": report.channels,
            "decoded_frames": report.decoded_frames,
            "detail": safe_detail(&report.detail),
        }),
        Err(error) => json!({
            "status": "Error",
            "stage": "NotChecked",
            "checked_at_unix": unix_seconds(now),
            "codec": null,
            "sample_rate": null,
            "channels": null,
            "decoded_frames": 0,
            "detail": safe_detail(&error),
        }),
    }
}

// Never emit unbounded network/decoder error text into machine-readable reports.
fn safe_detail(detail: &str) -> String {
    detail
        .chars()
        .filter(|ch| !ch.is_control())
        .take(256)
        .collect()
}

fn unverified() -> Value {
    json!({
        "status": "Unverified",
        "stage": "NotChecked",
        "checked_at_unix": null,
        "codec": null,
        "sample_rate": null,
        "channels": null,
        "decoded_frames": 0,
        "detail": "Not checked by this sweep",
    })
}

/// Successful decoded audio is the only positive evidence. A socket/HTTP result
/// cannot create or refresh this timestamp.
fn decoded_at(validation: &Value) -> Option<u64> {
    if validation["status"] == "Decoded"
        && validation["stage"] == "DecodedAudio"
        && validation["decoded_frames"].as_u64().unwrap_or(0) > 0
    {
        validation["checked_at_unix"].as_u64()
    } else {
        None
    }
}

/// Fresh means the latest probe decoded audio recently; degraded means a prior
/// success is still within its bounded grace window after transient failures.
/// Failed probes never refresh the last-good timestamp.
fn current_health(station: &Value, now_unix: u64) -> &'static str {
    if let Some(last) = decoded_at(&station["validation"]) {
        return if now_unix >= last && now_unix - last <= FEED_FRESHNESS_TTL.as_secs() {
            "fresh"
        } else if now_unix >= last && now_unix - last <= LAST_GOOD_GRACE.as_secs() {
            "degraded"
        } else {
            "stale"
        };
    }
    if station["consecutive_failed_sweeps"].as_u64().unwrap_or(0)
        >= FAILURE_SWEEPS_TO_UNAVAILABLE
    {
        return "unavailable";
    }
    if let Some(last_good) = decoded_at(&station["last_good"]) {
        return if now_unix >= last_good
            && now_unix - last_good <= LAST_GOOD_GRACE.as_secs()
        {
            "degraded"
        } else {
            "stale"
        };
    }
    "unverified"
}

fn is_eligible(station: &Value, now_unix: u64, strict: bool) -> bool {
    match current_health(station, now_unix) {
        "fresh" => true,
        "degraded" => !strict,
        _ => false,
    }
}

fn empty_station(station: &InternetStation) -> Value {
    json!({
        "id": station.id,
        "name": station.name,
        "genre": station.genre,
        "category": station.category,
        "publisher_reference_url": station.publisher_reference_url,
        "health": "unverified",
        "validation": unverified(),
        "last_good": null,
        "consecutive_failed_sweeps": 0,
        "probe_attempts": 0,
    })
}

/// Accept the previous snapshot, including schema-v1 reports from the original
/// validator, without letting stale metadata replace the canonical station.
fn restore_station(station: &InternetStation, old: Option<&Value>) -> Value {
    let mut entry = empty_station(station);
    if let Some(old) = old {
        if old["validation"].is_object() {
            entry["validation"] = old["validation"].clone();
        }
        if decoded_at(&old["last_good"]).is_some() {
            entry["last_good"] = old["last_good"].clone();
        } else if decoded_at(&old["validation"]).is_some() {
            entry["last_good"] = old["validation"].clone();
        }
        entry["consecutive_failed_sweeps"] =
            json!(old["consecutive_failed_sweeps"]
                .as_u64()
                .unwrap_or(0)
                .min(FAILURE_SWEEPS_TO_UNAVAILABLE));
        entry["probe_attempts"] = json!(old["probe_attempts"].as_u64().unwrap_or(0).min(2));
    }
    entry
}

fn snapshot(
    stations: &[Value],
    checked: usize,
    complete: bool,
    started: u64,
    now: u64,
    network_incident: bool,
) -> Value {
    let mut counts = [0usize; 5];
    let live: Vec<Value> = stations
        .iter()
        .map(|station| {
            let mut station = station.clone();
            let health = current_health(&station, now);
            match health {
                "fresh" => counts[0] += 1,
                "degraded" => counts[1] += 1,
                "stale" => counts[2] += 1,
                "unavailable" => counts[3] += 1,
                _ => counts[4] += 1,
            }
            station["health"] = json!(health);
            station
        })
        .collect();
    json!({
        "schema_version": 2,
        "generated_at_unix": now,
        "scan_started_at_unix": started,
        "complete": complete,
        "total": curated_stations().len(),
        "checked": checked,
        "fresh": counts[0],
        "degraded": counts[1],
        "stale": counts[2],
        "unavailable": counts[3],
        "unverified": counts[4],
        "network_incident_suspected": network_incident,
        "freshness_ttl_seconds": FEED_FRESHNESS_TTL.as_secs(),
        "last_good_grace_seconds": LAST_GOOD_GRACE.as_secs(),
        "failed_sweeps_to_unavailable": FAILURE_SWEEPS_TO_UNAVAILABLE,
        "stations": live,
    })
}

/// Update via a temporary file in the same directory; protect the previous
/// report from partial writes on Unix/macOS. Reports contain no stream tokens.
fn atomic_save(path: &Path, report: &Value) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".rust-radio-health-{}-{}.tmp",
        process::id(),
        stamp
    ));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        serde_json::to_writer_pretty(&mut file, report).map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_snapshot(path: &Path) -> Result<Value, String> {
    let file = File::open(path).map_err(|error| format!("Cannot read report: {error}"))?;
    if file
        .metadata()
        .map_err(|error| error.to_string())?
        .len()
        > MAX_REPORT_BYTES
    {
        return Err("Feed report exceeds maximum size".into());
    }
    let report: Value = serde_json::from_reader(file)
        .map_err(|error| format!("Invalid feed report: {error}"))?;
    if !matches!(report["schema_version"].as_u64(), Some(1 | 2))
        || report["total"].as_u64() != Some(curated_stations().len() as u64)
    {
        return Err("Report has an unsupported version or station count".into());
    }
    let entries = report["stations"]
        .as_array()
        .ok_or("Missing report station array")?;
    if entries.len() != curated_stations().len()
        || !entries
            .iter()
            .zip(curated_stations().iter())
            .all(|(entry, station)| entry["id"].as_str() == Some(station.id))
    {
        return Err("Report catalog differs from the currently installed stations".into());
    }
    Ok(report)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".bak");
    PathBuf::from(value)
}

/// Maintain a second completed snapshot. An interrupted sweep leaves a valid
/// partial primary and the last fully completed backup; corrupt primary data
/// never silently resets a station's last-good history.
fn read_with_redundancy(path: &Path) -> Result<Option<Value>, String> {
    match read_snapshot(path) {
        Ok(report) => Ok(Some(report)),
        Err(primary_error) => {
            let backup = backup_path(path);
            match read_snapshot(&backup) {
                Ok(report) => {
                    eprintln!(
                        "Feed report unavailable ({primary_error}); recovered {}",
                        backup.display()
                    );
                    Ok(Some(report))
                }
                Err(backup_error) => {
                    if !path.exists() && !backup.exists() {
                        Ok(None)
                    } else {
                        Err(format!(
                            "Neither primary nor backup report is valid: {primary_error}; {backup_error}"
                        ))
                    }
                }
            }
        }
    }
}

fn fresh_count(report: &Value, now: u64) -> usize {
    report["stations"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| current_health(entry, now) == "fresh")
                .count()
        })
        .unwrap_or(0)
}

fn eligible_count(report: &Value, now: u64, strict: bool) -> usize {
    report["stations"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| is_eligible(entry, now, strict))
                .count()
        })
        .unwrap_or(0)
}

/// The second decoder probe is only invoked after the first fails. This
/// protects a working station against transient DNS, TLS, CDN and stream errors
/// without multiplying requests to a healthy broadcaster.
fn probe_with_retry(
    station_id: &str,
    retry_pause: Duration,
    validate: &mut impl FnMut(&str) -> Result<StationValidation, String>,
) -> (Value, usize) {
    for attempt in 1..=REDUNDANT_PROBE_ATTEMPTS {
        let report = validation_value(validate(station_id), SystemTime::now());
        if decoded_at(&report).is_some() {
            return (report, attempt);
        }
        if attempt == REDUNDANT_PROBE_ATTEMPTS {
            return (report, attempt);
        }
        if !retry_pause.is_zero() {
            thread::sleep(retry_pause);
        }
    }
    unreachable!("at least one probe attempt is required")
}

/// Resumable bounded sweep. Failed attempts do not advance failure hysteresis
/// until an entire sweep completes. A zero-success cross-publisher sweep is
/// classified as a possible *host-wide* outage, not 25 broken stations.
fn sweep_with(
    path: &Path,
    pause: Duration,
    retry_pause: Duration,
    mut validate: impl FnMut(&str) -> Result<StationValidation, String>,
) -> io::Result<usize> {
    let previous = read_with_redundancy(path).map_err(io::Error::other)?;
    let old = previous
        .as_ref()
        .and_then(|report| report["stations"].as_array());
    let now = SystemTime::now();
    let started = unix_seconds(now).unwrap_or(0);
    let mut stations: Vec<_> = curated_stations()
        .iter()
        .enumerate()
        .map(|(index, station)| restore_station(station, old.and_then(|entries| entries.get(index))))
        .collect();
    let total = stations.len();
    let mut successes = 0usize;
    atomic_save(path, &snapshot(&stations, 0, false, started, started, false))?;
    for (index, station) in curated_stations().iter().enumerate() {
        eprintln!("[{}/{}] {} / {}", index + 1, total, station.id, station.genre);
        let (report, attempts) = probe_with_retry(station.id, retry_pause, &mut validate);
        let passed = decoded_at(&report).is_some();
        stations[index]["validation"] = report.clone();
        stations[index]["probe_attempts"] = json!(attempts);
        if passed {
            successes += 1;
            stations[index]["last_good"] = report;
            stations[index]["consecutive_failed_sweeps"] = json!(0);
        }
        let at = unix_seconds(SystemTime::now()).unwrap_or(started);
        let partial = snapshot(&stations, index + 1, false, started, at, false);
        eprintln!(
            "  {}: {} ({} probe{})",
            station.id,
            partial["stations"][index]["health"].as_str().unwrap_or("unverified"),
            attempts,
            if attempts == 1 { "" } else { "s" },
        );
        atomic_save(path, &partial)?;
        if index + 1 < total && !pause.is_zero() {
            thread::sleep(pause);
        }
    }
    let network_incident = successes == 0 && total > 1;
    if !network_incident {
        for station in &mut stations {
            if decoded_at(&station["validation"]).is_none() {
                let prior = station["consecutive_failed_sweeps"]
                    .as_u64()
                    .unwrap_or(0)
                    .min(FAILURE_SWEEPS_TO_UNAVAILABLE);
                station["consecutive_failed_sweeps"] =
                    json!((prior + 1).min(FAILURE_SWEEPS_TO_UNAVAILABLE));
            }
        }
    } else {
        eprintln!(
            "WARNING: every publisher probe failed; treating this as a possible network/host incident"
        );
    }
    let at = unix_seconds(SystemTime::now()).unwrap_or(started);
    let completed = snapshot(&stations, total, true, started, at, network_incident);
    atomic_save(path, &completed)?;
    // Backup is updated *only* after a completed sweep. A crash or partial
    // scan never destroys the last complete copy.
    atomic_save(&backup_path(path), &completed)?;
    Ok(fresh_count(&completed, at))
}

fn show_report(path: &Path, eligible_mode: Option<bool>) -> Result<bool, String> {
    let report = read_with_redundancy(path)?.ok_or("No saved feed-health report")?;
    let now = unix_seconds(SystemTime::now()).ok_or("System clock precedes UNIX epoch")?;
    let old = report["stations"].as_array().ok_or("Missing stations")?;
    let entries: Vec<Value> = curated_stations()
        .iter()
        .zip(old)
        .map(|(station, entry)| restore_station(station, Some(entry)))
        .collect();
    if let Some(strict) = eligible_mode {
        let ids: Vec<&str> = entries
            .iter()
            .filter(|entry| is_eligible(entry, now, strict))
            .filter_map(|entry| entry["id"].as_str())
            .collect();
        println!("{}", serde_json::to_string(&ids).map_err(|error| error.to_string())?);
        return Ok(true);
    }
    let fresh = entries
        .iter()
        .filter(|entry| current_health(entry, now) == "fresh")
        .count();
    let degraded = entries
        .iter()
        .filter(|entry| current_health(entry, now) == "degraded")
        .count();
    println!(
        "Feed health: {fresh} fresh / {degraded} degraded / {} total | complete={} | network incident suspected={}",
        entries.len(),
        report["complete"].as_bool().unwrap_or(false),
        report["network_incident_suspected"].as_bool().unwrap_or(false),
    );
    for station in &entries {
        println!(
            "{}\t{}\tfailed_sweeps={}\t{}\t{}",
            station["id"].as_str().unwrap_or("?"),
            current_health(station, now),
            station["consecutive_failed_sweeps"].as_u64().unwrap_or(0),
            station["category"].as_str().unwrap_or("?"),
            station["genre"].as_str().unwrap_or("?"),
        );
    }
    Ok(true)
}

fn run(options: Options) -> Result<bool, String> {
    match options.mode {
        Mode::Status => show_report(&options.output, None),
        Mode::Eligible => show_report(&options.output, Some(false)),
        Mode::StrictEligible => show_report(&options.output, Some(true)),
        Mode::Once | Mode::Daemon => {
            loop {
                let cycle = Instant::now();
                match sweep_with(
                    &options.output,
                    INTER_STATION_PAUSE,
                    PROBE_RETRY_DELAY,
                    validate_station,
                ) {
                    Ok(fresh) => {
                        let now = unix_seconds(SystemTime::now()).unwrap_or(0);
                        let report = read_with_redundancy(&options.output)?
                            .ok_or("Completed feed report is missing")?;
                        let eligible = eligible_count(&report, now, false);
                        eprintln!(
                            "{fresh} fresh, {} degraded, {eligible} eligible / {} total; report: {}",
                            eligible.saturating_sub(fresh),
                            curated_stations().len(),
                            options.output.display(),
                        );
                        if options.mode == Mode::Once {
                            return Ok(eligible == curated_stations().len());
                        }
                    }
                    Err(error) => {
                        if options.mode == Mode::Once {
                            return Err(format!("Cannot save feed report: {error}"));
                        }
                        eprintln!("Feed report failed: {error}");
                    }
                }
                // One serial sweep per process, never overlapping. A slow sweep
                // reduces sleep instead of creating a second concurrent scan.
                thread::sleep(options.interval.saturating_sub(cycle.elapsed()));
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        help();
        return;
    }
    match parse_args(&args).and_then(run) {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(error) => {
            eprintln!("rust-radio-feed-validator: {error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_radio::directory::station_by_id;
    use rust_radio::validation_types::{ValidationStage, ValidationStatus};
    use std::collections::BTreeMap;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).into()).collect()
    }

    fn decoded(id: &str, time: SystemTime) -> StationValidation {
        let mut report = StationValidation::unverified(id);
        report.status = ValidationStatus::Decoded;
        report.stage = ValidationStage::DecodedAudio;
        report.checked_at = Some(time);
        report.decoded_frames = 1024;
        report
    }

    fn temporary_report(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir()
            .join(format!("rust-radio-validator-{}-{label}-{stamp}", process::id()))
            .join("health.json")
    }

    fn cleanup(path: &Path) {
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    fn good_sweep(path: &Path) -> usize {
        sweep_with(path, Duration::ZERO, Duration::ZERO, |id| {
            Ok(decoded(id, SystemTime::now()))
        })
        .unwrap()
    }

    fn station<'a>(report: &'a Value, id: &str) -> &'a Value {
        report["stations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"].as_str() == Some(id))
            .unwrap()
    }

    #[test]
    fn options_require_explicit_daemon_for_intervals() {
        assert_eq!(parse_args(&[]).unwrap().mode, Mode::Once);
        assert_eq!(parse_args(&args(&["--daemon"])).unwrap().interval.as_secs(), 14_400);
        assert_eq!(
            parse_args(&args(&["--strict-eligible"])).unwrap().mode,
            Mode::StrictEligible
        );
        assert!(parse_args(&args(&["--eligible", "--strict-eligible"])).is_err());
        assert!(parse_args(&args(&["--once", "--daemon"])).is_err());
        assert!(parse_args(&args(&["--interval-minutes", "15"])).is_err());
        assert!(parse_args(&args(&["--daemon", "--interval-minutes", "0"])).is_err());
        assert!(parse_args(&args(&["--daemon", "--interval-minutes", "241"])).is_err());
        assert_eq!(
            parse_args(&args(&["--daemon", "--interval-minutes", "60"]))
                .unwrap()
                .interval
                .as_secs(),
            3600
        );
    }

    #[test]
    fn last_good_grace_is_bounded_and_does_not_claim_freshness() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000_000);
        let at = unix_seconds(now).unwrap();
        let mut entry = empty_station(station_by_id("gbhnews").unwrap());
        let good = validation_value(Ok(decoded("gbhnews", now)), now);
        entry["last_good"] = good.clone();
        entry["validation"] = good;
        assert_eq!(current_health(&entry, at), "fresh");
        assert_eq!(
            current_health(&entry, at + FEED_FRESHNESS_TTL.as_secs() + 1),
            "degraded"
        );
        entry["validation"] = validation_value(Err("timeout".into()), now);
        entry["consecutive_failed_sweeps"] = json!(1);
        assert_eq!(current_health(&entry, at), "degraded");
        assert!(is_eligible(&entry, at, false));
        assert!(!is_eligible(&entry, at, true));
        assert_eq!(
            current_health(&entry, at + LAST_GOOD_GRACE.as_secs() + 1),
            "stale"
        );
        assert!(!is_eligible(
            &entry,
            at + LAST_GOOD_GRACE.as_secs() + 1,
            false
        ));
        entry["consecutive_failed_sweeps"] = json!(FAILURE_SWEEPS_TO_UNAVAILABLE);
        assert_eq!(current_health(&entry, at), "unavailable");
        assert!(!is_eligible(&entry, at, false));
        let mut invalid = entry;
        invalid["last_good"]["decoded_frames"] = json!(0);
        invalid["consecutive_failed_sweeps"] = json!(0);
        assert_eq!(current_health(&invalid, at), "unverified");
    }

    #[test]
    fn directory_and_index_include_npr_and_preserve_25_station_limit() {
        let directory = curated_stations();
        let index: Value =
            serde_json::from_str(include_str!("../../../../catalog/curated-stations.json"))
                .unwrap();
        let listed = index["stations"].as_array().unwrap();
        assert_eq!(directory.len(), 25);
        assert_eq!(directory.len(), listed.len());
        for (station, entry) in directory.iter().zip(listed) {
            assert_eq!(entry["id"], station.id);
            assert_eq!(entry["name"], station.name);
            assert_eq!(entry["genre"], station.genre);
            assert_eq!(entry["category"], station.category);
            assert_eq!(entry["playlist_url"], station.playlist_url);
            assert_eq!(entry["publisher_reference_url"], station.publisher_reference_url);
        }
        assert!(directory.iter().any(|station| station.id == "gbhnews"));
        assert!(!directory.iter().any(|station| station.id == "poptron"));
    }

    #[test]
    fn one_failed_probe_gets_a_second_attempt_and_can_recover() {
        let path = temporary_report("retry");
        let mut attempts = BTreeMap::<String, usize>::new();
        let fresh = sweep_with(&path, Duration::ZERO, Duration::ZERO, |id| {
            let attempt = attempts.entry(id.to_owned()).or_default();
            *attempt += 1;
            if id == "gbhnews" && *attempt == 1 {
                Err("transient CDN timeout".into())
            } else {
                Ok(decoded(id, SystemTime::now()))
            }
        })
        .unwrap();
        assert_eq!(fresh, 25);
        assert_eq!(attempts.get("gbhnews"), Some(&2));
        assert_eq!(attempts.get("skylab"), Some(&1));
        let report = read_snapshot(&path).unwrap();
        assert_eq!(station(&report, "gbhnews")["probe_attempts"], 2);
        assert_eq!(
            station(&report, "gbhnews")["consecutive_failed_sweeps"],
            0
        );
        assert_eq!(report["fresh"], 25);
        cleanup(&path);
    }

    #[test]
    fn three_distinct_completed_failures_required_and_recovery_is_immediate() {
        let path = temporary_report("hysteresis");
        assert_eq!(good_sweep(&path), 25);
        for cycle in 1..=FAILURE_SWEEPS_TO_UNAVAILABLE {
            sweep_with(&path, Duration::ZERO, Duration::ZERO, |id| {
                if id == "gbhnews" {
                    Err("temporary station outage".into())
                } else {
                    Ok(decoded(id, SystemTime::now()))
                }
            })
            .unwrap();
            let report = read_snapshot(&path).unwrap();
            let news = station(&report, "gbhnews");
            assert_eq!(news["consecutive_failed_sweeps"], cycle);
            assert!(decoded_at(&news["last_good"]).is_some());
            assert_eq!(report["total"], 25);
            assert_eq!(
                current_health(news, unix_seconds(SystemTime::now()).unwrap()),
                if cycle < FAILURE_SWEEPS_TO_UNAVAILABLE {
                    "degraded"
                } else {
                    "unavailable"
                }
            );
        }
        assert_eq!(good_sweep(&path), 25);
        let recovered = read_snapshot(&path).unwrap();
        assert_eq!(station(&recovered, "gbhnews")["consecutive_failed_sweeps"], 0);
        assert_eq!(
            current_health(station(&recovered, "gbhnews"), unix_seconds(SystemTime::now()).unwrap()),
            "fresh"
        );
        cleanup(&path);
    }

    #[test]
    fn all_providers_failing_does_not_remove_previously_good_stations() {
        let path = temporary_report("host-outage");
        assert_eq!(good_sweep(&path), 25);
        assert_eq!(
            sweep_with(&path, Duration::ZERO, Duration::ZERO, |_id| {
                Err("no network on host".into())
            })
            .unwrap(),
            0
        );
        let report = read_snapshot(&path).unwrap();
        let now = unix_seconds(SystemTime::now()).unwrap();
        assert_eq!(report["network_incident_suspected"], true);
        assert_eq!(report["degraded"], 25);
        assert_eq!(report["total"], 25);
        assert_eq!(eligible_count(&report, now, false), 25);
        assert_eq!(eligible_count(&report, now, true), 0);
        for item in report["stations"].as_array().unwrap() {
            assert_eq!(item["consecutive_failed_sweeps"], 0);
            assert!(decoded_at(&item["last_good"]).is_some());
        }
        cleanup(&path);
    }

    #[test]
    fn corrupted_primary_restores_last_good_from_completed_backup() {
        let path = temporary_report("backup");
        assert_eq!(good_sweep(&path), 25);
        assert!(backup_path(&path).exists());
        fs::write(&path, b"bad-json").unwrap();
        let backup = read_with_redundancy(&path).unwrap().unwrap();
        assert_eq!(backup["fresh"], 25);
        sweep_with(&path, Duration::ZERO, Duration::ZERO, |id| {
            if id == "gbhnews" {
                Err("temporarily offline".into())
            } else {
                Ok(decoded(id, SystemTime::now()))
            }
        })
        .unwrap();
        let recovered = read_snapshot(&path).unwrap();
        let news = station(&recovered, "gbhnews");
        assert_eq!(news["consecutive_failed_sweeps"], 1);
        assert!(decoded_at(&news["last_good"]).is_some());
        assert_eq!(current_health(news, unix_seconds(SystemTime::now()).unwrap()), "degraded");
        cleanup(&path);
    }

    #[test]
    fn sweep_retains_25_entries_and_publishes_partial_and_complete_evidence() {
        let path = temporary_report("partial");
        let fresh = sweep_with(&path, Duration::ZERO, Duration::ZERO, |id| {
            if id == "gbhnews" {
                Ok(decoded(id, SystemTime::now()))
            } else {
                Err("synthetic failure".into())
            }
        })
        .unwrap();
        assert_eq!(fresh, 1);
        let report = read_snapshot(&path).unwrap();
        assert_eq!(report["complete"], true);
        assert_eq!(report["checked"], 25);
        assert_eq!(report["fresh"], 1);
        assert_eq!(report["unverified"], 24);
        assert_eq!(report["network_incident_suspected"], false);
        assert_eq!(station(&report, "gbhnews")["health"], "fresh");
        assert_eq!(report["schema_version"], 2);
        assert!(backup_path(&path).exists());
        cleanup(&path);
    }

    #[test]
    fn old_schema_decoded_evidence_is_migrated_without_resetting_health() {
        let path = temporary_report("legacy");
        let now = unix_seconds(SystemTime::now()).unwrap();
        let old: Vec<Value> = curated_stations()
            .iter()
            .map(|station| {
                let mut entry = empty_station(station);
                entry["validation"] =
                    validation_value(Ok(decoded(station.id, SystemTime::now())), SystemTime::now());
                entry.as_object_mut().unwrap().remove("last_good");
                entry.as_object_mut().unwrap().remove("consecutive_failed_sweeps");
                entry
            })
            .collect();
        let mut legacy = snapshot(&old, 25, true, now, now, false);
        legacy["schema_version"] = json!(1);
        atomic_save(&path, &legacy).unwrap();
        sweep_with(&path, Duration::ZERO, Duration::ZERO, |id| {
            if id == "gbhnews" {
                Err("network timeout".into())
            } else {
                Ok(decoded(id, SystemTime::now()))
            }
        })
        .unwrap();
        let after = read_snapshot(&path).unwrap();
        let news = station(&after, "gbhnews");
        assert_eq!(after["schema_version"], 2);
        assert_eq!(news["consecutive_failed_sweeps"], 1);
        assert!(decoded_at(&news["last_good"]).is_some());
        assert_eq!(current_health(news, unix_seconds(SystemTime::now()).unwrap()), "degraded");
        cleanup(&path);
    }
}
