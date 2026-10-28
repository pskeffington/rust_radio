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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Once,
    Daemon,
    Status,
    Eligible,
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    mode: Mode,
    output: PathBuf,
    interval: Duration,
}

fn help() {
    println!(
        "Usage: rust-radio-feed-validator [--once|--daemon|--status|--eligible] [--output PATH] [--interval-minutes 15..240]\\n\\
--once: scan curated stations; return nonzero if any fail\\n\\
--daemon: scan immediately and repeat every four hours\\n\\
--status: summarize health, applying the six-hour TTL\\n\\
--eligible: output JSON IDs with fresh decoded-audio evidence\\n\\
Default output: target/radio-feed-health.json"
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
            "--once" | "--daemon" | "--status" | "--eligible" => {
                if mode_supplied {
                    return Err("Supply only one mode".into());
                }
                mode = match args[i].as_str() {
                    "--daemon" => Mode::Daemon,
                    "--status" => Mode::Status,
                    "--eligible" => Mode::Eligible,
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

/// Recompute on every read; never trust the health string stored hours earlier.
fn current_health(validation: &Value, now_unix: u64) -> &'static str {
    let status = validation["status"].as_str().unwrap_or("Error");
    if status == "Decoded" {
        if validation["stage"] != "DecodedAudio"
            || validation["decoded_frames"].as_u64().unwrap_or(0) == 0
        {
            return "failed";
        }
        return match validation["checked_at_unix"].as_u64() {
            Some(at) if now_unix >= at && now_unix - at <= FEED_FRESHNESS_TTL.as_secs() => "fresh",
            _ => "stale",
        };
    }
    if status == "Unverified" || status == "Cancelled" {
        "unverified"
    } else {
        "failed"
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
    })
}

fn snapshot(stations: &[Value], checked: usize, complete: bool, started: u64, now: u64) -> Value {
    let mut counts = [0usize; 4];
    let live: Vec<Value> = stations
        .iter()
        .map(|station| {
            let mut station = station.clone();
            let health = current_health(&station["validation"], now);
            match health {
                "fresh" => counts[0] += 1,
                "stale" => counts[1] += 1,
                "failed" => counts[2] += 1,
                _ => counts[3] += 1,
            }
            station["health"] = json!(health);
            station
        })
        .collect();
    json!({
        "schema_version": 1,
        "generated_at_unix": now,
        "scan_started_at_unix": started,
        "complete": complete,
        "total": curated_stations().len(),
        "checked": checked,
        "fresh": counts[0],
        "stale": counts[1],
        "failed": counts[2],
        "unverified": counts[3],
        "freshness_ttl_seconds": FEED_FRESHNESS_TTL.as_secs(),
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
    if report["schema_version"] != 1
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

fn fresh_count(report: &Value, now: u64) -> usize {
    report["stations"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| current_health(&entry["validation"], now) == "fresh")
                .count()
        })
        .unwrap_or(0)
}

/// Generic bounded sweep; injectable validator gives deterministic offline tests.
fn sweep_with(
    path: &Path,
    pause: Duration,
    mut validate: impl FnMut(&str) -> Result<StationValidation, String>,
) -> io::Result<usize> {
    let now = SystemTime::now();
    let started = unix_seconds(now).unwrap_or(0);
    let mut stations: Vec<_> = curated_stations().iter().map(empty_station).collect();
    let total = stations.len();
    atomic_save(path, &snapshot(&stations, 0, false, started, started))?;
    for (index, station) in curated_stations().iter().enumerate() {
        eprintln!("[{}/{}] {} / {}", index + 1, total, station.id, station.genre);
        let validation = validation_value(validate(station.id), SystemTime::now());
        stations[index]["validation"] = validation;
        let at = unix_seconds(SystemTime::now()).unwrap_or(started);
        let partial = snapshot(&stations, index + 1, index + 1 == total, started, at);
        eprintln!(
            "  {}: {}",
            station.id,
            partial["stations"][index]["health"].as_str().unwrap_or("failed")
        );
        atomic_save(path, &partial)?;
        if index + 1 < total && !pause.is_zero() {
            thread::sleep(pause);
        }
    }
    let now = unix_seconds(SystemTime::now()).unwrap_or(started);
    Ok(fresh_count(
        &snapshot(&stations, total, true, started, now),
        now,
    ))
}

fn show_report(path: &Path, eligible_only: bool) -> Result<bool, String> {
    let report = read_snapshot(path)?;
    let now = unix_seconds(SystemTime::now()).ok_or("System clock precedes UNIX epoch")?;
    let entries = report["stations"].as_array().ok_or("Missing stations")?;
    if eligible_only {
        let ids: Vec<&str> = entries
            .iter()
            .filter(|s| current_health(&s["validation"], now) == "fresh")
            .filter_map(|s| s["id"].as_str())
            .collect();
        println!("{}", serde_json::to_string(&ids).map_err(|e| e.to_string())?);
        return Ok(ids.len() == entries.len());
    }
    let fresh = fresh_count(&report, now);
    println!(
        "Feed health: {fresh}/{} fresh | last report {} | complete={}",
        entries.len(),
        report["generated_at_unix"].as_u64().unwrap_or(0),
        report["complete"].as_bool().unwrap_or(false),
    );
    for station in entries {
        println!(
            "{}\t{}\t{}\t{}",
            station["id"].as_str().unwrap_or("?"),
            current_health(&station["validation"], now),
            station["category"].as_str().unwrap_or("?"),
            station["genre"].as_str().unwrap_or("?"),
        );
    }
    Ok(fresh == entries.len() && report["complete"].as_bool() == Some(true))
}

fn run(options: Options) -> Result<bool, String> {
    match options.mode {
        Mode::Status => show_report(&options.output, false),
        Mode::Eligible => show_report(&options.output, true),
        Mode::Once | Mode::Daemon => {
            loop {
                let cycle = Instant::now();
                match sweep_with(&options.output, INTER_STATION_PAUSE, validate_station) {
                    Ok(fresh) => {
                        eprintln!(
                            "Validated {fresh}/{} stations; report: {}",
                            curated_stations().len(),
                            options.output.display()
                        );
                        if options.mode == Mode::Once {
                            return Ok(fresh == curated_stations().len());
                        }
                    }
                    Err(error) => {
                        if options.mode == Mode::Once {
                            return Err(format!("Cannot write feed report: {error}"));
                        }
                        eprintln!("Feed report failed: {error}");
                    }
                }
                // One serial checker per process, no overlap, no host audio output.
                // A long scan reduces the sleep rather than spawning another scan.
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
    use rust_radio::validation_types::{ValidationStage, ValidationStatus};

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

    #[test]
    fn options_require_explicit_daemon_for_intervals() {
        assert_eq!(parse_args(&[]).unwrap().mode, Mode::Once);
        assert_eq!(parse_args(&args(&["--daemon"])).unwrap().interval.as_secs(), 14_400);
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
    fn health_requires_recent_real_decoded_frames() {
        let now = SystemTime::now();
        let fresh = validation_value(Ok(decoded("gbhnews", now)), now);
        let at = unix_seconds(now).unwrap();
        assert_eq!(current_health(&fresh, at), "fresh");
        assert_eq!(
            current_health(&fresh, at + FEED_FRESHNESS_TTL.as_secs() + 1),
            "stale"
        );
        let mut failed = fresh.clone();
        failed["status"] = json!("Unavailable");
        assert_eq!(current_health(&failed, at), "failed");
        let mut fake = fresh;
        fake["decoded_frames"] = json!(0);
        assert_eq!(current_health(&fake, at), "failed");
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
    fn sweep_persists_progress_and_eligibility_without_live_network() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = env::temp_dir().join(format!(
            "rust-radio-validator-{}-{timestamp}",
            process::id()
        ));
        let path = root.join("health.json");
        let fresh = sweep_with(&path, Duration::ZERO, |id| {
            if id == "gbhnews" {
                Ok(decoded(id, SystemTime::now()))
            } else {
                Err("synthetic unavailable".into())
            }
        })
        .unwrap();
        assert_eq!(fresh, 1);
        let report = read_snapshot(&path).unwrap();
        assert_eq!(report["complete"].as_bool(), Some(true));
        assert_eq!(report["checked"], 25);
        assert_eq!(report["fresh"], 1);
        assert_eq!(report["failed"], 24);
        assert_eq!(
            current_health(
                &report["stations"].as_array().unwrap().iter()
                    .find(|station| station["id"] == "gbhnews").unwrap()["validation"],
                unix_seconds(SystemTime::now()).unwrap()
            ),
            "fresh"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
