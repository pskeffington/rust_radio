//! Operator-triggered standalone test harness. No automatic network or audio I/O.
use rust_radio::directory::{curated_stations, station_by_id};
use rust_radio::output::available_outputs;
use rust_radio::validation::validate_station;
use rust_radio::validation_types::{StationValidation, ValidationStage, ValidationStatus};
use rust_radio::{IpBufferingProfile, NativeRadioEngine};
use std::collections::VecDeque;
use std::io::{self, BufRead};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, process};

mod feed_history;
mod nrsp;
mod nrsp_native;
mod sdr;
mod sdrconnect;

#[derive(Debug, PartialEq, Eq)]
struct ListenArgs<'a> {
    station: &'a str,
    sdrconnect: bool,
    direct: bool,
    seconds: Option<u64>,
    output: &'a str,
    volume: u8,
    buffering: IpBufferingProfile,
}

fn usage() {
    eprintln!("Usage: rust-radio-cli stations | outputs | validate STATION | validate-all --json [--history PATH] | listen STATION [--seconds 1..300] [--output ID] [--volume 0..100] [--buffering low-latency|resilient]");
    eprintln!(
        "Direct stream: stream-listen URL [--seconds 1..300] [--output ID] [--volume 0..100] [--buffering low-latency|resilient]"
    );
    eprintln!("Input discovery: inputs (enable nrsp-native and/or soapy-sdr for receiver scans)");
    eprintln!("Interactive listen commands: status | volume N | fm auto|mono 50|75|off LOW_HZ HIGH_HZ | stop");
    eprintln!("IP buffering profiles: low-latency=200ms | resilient=500ms; run with --volume 0 for muted callback A/B measurements");
    eprintln!("Optional SDR: sdr-devices | sdr-capture DEVICE_ID FREQ_HZ RATE_HZ [--milliseconds 1..5000] (enable soapy-sdr)");
    eprintln!("USB mono FM: sdr-listen DEVICE_ID FREQ_HZ [--seconds 1..300] [--volume 0..100] [--output ID] [--deemphasis 50|75] (enable soapy-sdr)");
    eprintln!("Direct nRSP-ST control/audio: nrsp-native-status IP:9001 | nrsp-listen IP:9001 FREQ_HZ MODE [--seconds N] [--volume N] [--output ID] (enable nrsp-native)");
    eprintln!("FM options: --stereo auto|mono --deemphasis 50|75|off --low-cut Hz --high-cut Hz --bandwidth Hz");
    eprintln!("Direct nRSP-ST reachability: nrsp-status PRIVATE_IPV4 (enable sdr)");
    eprintln!("nRSP-ST via SDRconnect: sdrconnect-status IP:PORT | sdrconnect-capture IP:PORT iq|audio --milliseconds 1..5000 (enable sdrconnect)");
    eprintln!("Native SDR audio: sdrconnect-listen IP:PORT [--seconds 1..300] [--output ID] [--volume 0..100]");
}

const VALIDATION_WORKERS: usize = 4;
const VALIDATION_RECOVERY_DELAY: Duration = Duration::from_secs(1);
const INTERACTIVE_COMMAND_MAX_BYTES: usize = 4096;
const INTERACTIVE_COMMAND_QUEUE: usize = 8;
type CatalogValidationResult = Result<StationValidation, String>;
struct CatalogValidationRecord {
    station_id: String,
    attempts: u8,
    result: CatalogValidationResult,
}
type CatalogValidationResults = Vec<CatalogValidationRecord>;

fn collect_catalog_validations(
    check: fn(&str) -> Result<StationValidation, String>,
) -> Result<CatalogValidationResults, String> {
    collect_catalog_validations_with_delay(check, VALIDATION_RECOVERY_DELAY)
}

fn collect_catalog_validations_with_delay(
    check: fn(&str) -> Result<StationValidation, String>,
    recovery_delay: Duration,
) -> Result<CatalogValidationResults, String> {
    let ids: Vec<_> = curated_stations()
        .iter()
        .map(|station| station.id.to_owned())
        .collect();
    let queue = Arc::new(Mutex::new(
        ids.iter()
            .enumerate()
            .map(|(i, id)| (i, id.clone()))
            .collect::<VecDeque<_>>(),
    ));
    let (tx, rx) = mpsc::channel();
    let mut workers = Vec::new();
    for _ in 0..VALIDATION_WORKERS.min(ids.len()) {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        workers.push(
            thread::Builder::new()
                .name("rust-radio-catalog-validation".into())
                .spawn(move || loop {
                    let next = queue.lock().expect("validation queue poisoned").pop_front();
                    let Some((index, id)) = next else { break };
                    let mut result = check(&id);
                    let mut attempts = 1;
                    if matches!(result, Ok(ref report) if report.status == ValidationStatus::Unavailable)
                    {
                        thread::sleep(recovery_delay);
                        attempts = 2;
                        result = check(&id);
                    }
                    let record = CatalogValidationRecord {
                        station_id: id,
                        attempts,
                        result,
                    };
                    if tx.send((index, record)).is_err() {
                        break;
                    }
                })
                .map_err(|error| format!("Unable to start catalog validation worker: {error}"))?,
        );
    }
    drop(tx);
    let mut results: Vec<_> = rx.into_iter().collect();
    for worker in workers {
        worker
            .join()
            .map_err(|_| "Catalog validation worker panicked".to_owned())?;
    }
    if results.len() != ids.len() {
        return Err("Catalog validation ended before every station was checked".into());
    }
    results.sort_by_key(|(index, _)| *index);
    Ok(results.into_iter().map(|(_, record)| record).collect())
}

fn batch_validation_json(results: CatalogValidationResults) -> Result<String, String> {
    let checked_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System clock predates Unix epoch".to_owned())?
        .as_secs();
    let stations: Vec<_> = results.into_iter().map(|record| match record.result {
        Ok(report) => serde_json::json!({
            "station_id": record.station_id,
            "attempts": record.attempts,
            "recovered": record.attempts > 1 && report.is_decoded(),
            "status": validation_status_name(report.status),
            "stage": validation_stage_name(report.stage),
            "checked_at_unix": report.checked_at.and_then(|time| time.duration_since(UNIX_EPOCH).ok()).map(|time| time.as_secs()),
            "content_type": report.content_type,
            "codec": report.codec,
            "sample_rate": report.sample_rate,
            "channels": report.channels,
            "decoded_frames": report.decoded_frames,
            "decoded": report.is_decoded(),
        }),
        Err(_) => serde_json::json!({"station_id": record.station_id, "attempts": record.attempts, "status": "check_error", "decoded": false}),
    }).collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": 1,
        "checked_at_unix": checked_at,
        "results": stations,
    }))
    .map_err(|error| format!("Unable to encode catalog validation report: {error}"))
}

fn validation_status_name(status: ValidationStatus) -> &'static str {
    match status {
        ValidationStatus::Unverified => "unverified",
        ValidationStatus::Unavailable => "unavailable",
        ValidationStatus::InvalidStream => "invalid_stream",
        ValidationStatus::Unsupported => "unsupported",
        ValidationStatus::Cancelled => "cancelled",
        ValidationStatus::Decoded => "decoded",
    }
}

fn validation_stage_name(stage: ValidationStage) -> &'static str {
    match stage {
        ValidationStage::NotChecked => "not_checked",
        ValidationStage::EndpointReached => "endpoint_reached",
        ValidationStage::StreamResponse => "stream_response",
        ValidationStage::SupportedCodec => "supported_codec",
        ValidationStage::DecodedAudio => "decoded_audio",
    }
}

fn valid_validate_all_args(args: &[String]) -> bool {
    (args.len() == 2 && args[1] == "--json")
        || (args.len() == 4 && args[1] == "--json" && args[2] == "--history" && !args[3].is_empty())
}

fn validation_summary(report: &StationValidation) -> String {
    let checked_at = report
        .checked_at
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|time| time.as_secs().to_string())
        .unwrap_or_else(|| "not checked".into());
    format!(
        "{}\t{:?}\tstage={:?}\tchecked_at_unix={}\nendpoint={}\ncodec={}\tsample_rate={}\tchannels={}\tdecoded_frames={}\n{}",
        report.station_id,
        report.status,
        report.stage,
        checked_at,
        report.resolved_url.as_deref().unwrap_or("unknown"),
        report.codec.as_deref().unwrap_or("unknown"),
        report.sample_rate.unwrap_or(0),
        report.channels.unwrap_or(0),
        report.decoded_frames,
        report.detail,
    )
}

fn parse_listen(args: &[String]) -> Result<ListenArgs<'_>, &'static str> {
    if args.len() < 2 {
        return Err("missing listen source");
    }
    let sdrconnect = args[0] == "sdrconnect-listen";
    let direct = args[0] == "stream-listen";
    if sdrconnect {
        #[cfg(feature = "sdrconnect")]
        rust_radio::receiver::sdrconnect::Endpoint::parse(&args[1])
            .map_err(|_| "invalid SDRconnect peer")?;
        #[cfg(not(feature = "sdrconnect"))]
        return Err("enable the sdrconnect feature for native nRSP-ST audio");
    } else if !direct && (args[0] != "listen" || station_by_id(&args[1]).is_none()) {
        return Err("unknown curated station");
    }
    let mut parsed = ListenArgs {
        station: &args[1],
        sdrconnect,
        direct,
        seconds: None,
        output: "system",
        volume: 60,
        buffering: IpBufferingProfile::Resilient,
    };
    let mut used_seconds = false;
    let mut used_output = false;
    let mut used_volume = false;
    let mut used_buffering = false;
    let mut i = 2;
    while i < args.len() {
        if i + 1 >= args.len() {
            return Err("missing option value");
        }
        match args[i].as_str() {
            "--seconds" if !used_seconds => {
                let n = args[i + 1].parse::<u64>().map_err(|_| "invalid duration")?;
                if !(1..=300).contains(&n) {
                    return Err("duration must be 1..300 seconds");
                }
                parsed.seconds = Some(n);
                used_seconds = true;
            }
            "--volume" if !used_volume => {
                let n = args[i + 1].parse::<u8>().map_err(|_| "invalid volume")?;
                if n > 100 {
                    return Err("volume must be 0..100");
                }
                parsed.volume = n;
                used_volume = true;
            }
            "--output" if !used_output => {
                let id = args[i + 1].as_str();
                if id.is_empty()
                    || (id != "system"
                        && (!id.starts_with("output-")
                            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')))
                {
                    return Err("invalid output identifier");
                }
                parsed.output = id;
                used_output = true;
            }
            "--buffering" if !used_buffering => {
                parsed.buffering = match args[i + 1].as_str() {
                    "low-latency" => IpBufferingProfile::LowLatency,
                    "resilient" => IpBufferingProfile::Resilient,
                    _ => return Err("buffering must be low-latency or resilient"),
                };
                used_buffering = true;
            }
            _ => return Err("unknown or repeated listen option"),
        }
        i += 2;
    }
    if sdrconnect && used_buffering {
        return Err("IP buffering profiles apply only to internet radio streams");
    }
    Ok(parsed)
}

fn listen(args: ListenArgs<'_>) -> Result<(), String> {
    let engine = if args.sdrconnect {
        #[cfg(feature = "sdrconnect")]
        {
            NativeRadioEngine::start_sdrconnect(args.station, args.volume, args.output)?
        }
        #[cfg(not(feature = "sdrconnect"))]
        {
            return Err("enable the sdrconnect feature".into());
        }
    } else if args.direct {
        NativeRadioEngine::start_stream_with_buffering(
            args.station,
            args.volume,
            args.output,
            args.buffering,
        )?
    } else {
        NativeRadioEngine::start_internet_with_buffering(
            args.station,
            args.volume,
            args.output,
            args.buffering,
        )?
    };
    println!(
        "RUST RADIO / {} / output {} / volume {}%",
        if args.direct {
            "DIRECT STREAM"
        } else {
            args.station
        },
        args.output,
        args.volume
    );
    let prebuffer_ms = if args.sdrconnect {
        200
    } else {
        args.buffering.prebuffer_ms()
    };
    println!("OUTPUT PREBUFFER / {prebuffer_ms} ms");
    monitor(engine, args.seconds, prebuffer_ms)
}

fn monitor(
    mut engine: NativeRadioEngine,
    seconds: Option<u64>,
    prebuffer_ms: u16,
) -> Result<(), String> {
    // A separate stdin reader lets the main thread monitor duration and playback failures.
    let (tx, rx) = mpsc::sync_channel::<String>(INTERACTIVE_COMMAND_QUEUE);
    if seconds.is_none() {
        println!("Commands: status | volume N | tune FREQ_HZ MODE [BW_HZ] | fm auto|mono 50|75|off LOW_HZ HIGH_HZ | stop");
        thread::spawn(move || {
            let stdin = io::stdin();
            let mut input = stdin.lock();
            loop {
                match read_interactive_command(&mut input, INTERACTIVE_COMMAND_MAX_BYTES) {
                    Ok(Some(command)) => {
                        if tx.send(command).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(InteractiveCommandReadError::TooLong) => {
                        eprintln!("Interactive command ignored: line exceeds {INTERACTIVE_COMMAND_MAX_BYTES} bytes");
                    }
                    Err(InteractiveCommandReadError::Io(error)) => {
                        eprintln!("Interactive input failed: {error}");
                        break;
                    }
                }
            }
            let _ = tx.send("stop".to_owned());
        });
    }

    let start = Instant::now();
    let mut audible = false;
    let result = loop {
        let snapshot = engine.status();
        if snapshot.state == rust_radio::controller::PlaybackState::Playing {
            audible = true;
        }
        if snapshot.state == rust_radio::controller::PlaybackState::Failed {
            break Err(snapshot
                .last_error
                .unwrap_or_else(|| "playback failed".to_owned()));
        }
        if let Some(seconds) = seconds {
            if start.elapsed() >= Duration::from_secs(seconds) {
                break if audible {
                    let diagnostics = engine.diagnostics();
                    println!(
                        "RUST RADIO DIAGNOSTICS {}",
                        diagnostics_json(prebuffer_ms, diagnostics)
                    );
                    Ok(())
                } else {
                    let diagnostics = engine.diagnostics();
                    println!(
                        "RUST RADIO DIAGNOSTICS {}",
                        diagnostics_json(prebuffer_ms, diagnostics)
                    );
                    Err("No decoded PCM reached the audio callback".into())
                };
            }
        } else if let Ok(command) = rx.try_recv() {
            let command = command.trim();
            if command == "stop" || command == "q" {
                break Ok(());
            } else if command == "status" {
                println!("RADIO {:?} / {:?}", snapshot.state, snapshot.last_error);
                println!(
                    "RUST RADIO DIAGNOSTICS {}",
                    diagnostics_json(prebuffer_ms, engine.diagnostics())
                );
                #[cfg(feature = "nrsp-native")]
                if let Some(status) = engine.nrsp_native_status() {
                    println!("NATIVE STATUS {status:?}");
                    println!("NATIVE AUDIO {:?}", engine.nrsp_native_audio_stats());
                }
            } else if let Some(value) = command.strip_prefix("tune ") {
                #[cfg(feature = "nrsp-native")]
                {
                    let result = (|| {
                        let values: Vec<_> = value.split_whitespace().collect();
                        if !(2..=3).contains(&values.len()) {
                            return Err("Use tune FREQ_HZ MODE [BANDWIDTH_HZ]".to_owned());
                        }
                        let frequency = values[0].parse().map_err(|_| "Invalid frequency")?;
                        let mode = rust_radio::receiver::nrsp_native::Mode::parse(values[1])
                            .map_err(|e| e.to_string())?;
                        let bandwidth = if values.len() == 3 {
                            values[2].parse().map_err(|_| "Invalid bandwidth")?
                        } else {
                            mode.default_bandwidth()
                        };
                        engine.tune_nrsp_native(frequency, values[1], bandwidth)
                    })();
                    match result {
                        Ok(()) => println!("NATIVE TUNING REQUESTED"),
                        Err(error) => eprintln!("{error}"),
                    }
                }
                #[cfg(not(feature = "nrsp-native"))]
                {
                    let _ = value;
                    eprintln!("Enable nrsp-native for tuning");
                }
            } else if let Some(value) = command.strip_prefix("fm ") {
                #[cfg(feature = "nrsp-native")]
                {
                    let result = (|| {
                        use rust_radio::playback::{NativeFmDeemphasis, NativeFmSettings};
                        let values: Vec<_> = value.split_whitespace().collect();
                        if values.len() != 4 {
                            return Err("Use fm auto|mono 50|75|off LOW_HZ HIGH_HZ".to_owned());
                        }
                        let stereo = match values[0] {
                            "auto" => true,
                            "mono" => false,
                            _ => return Err("Use auto|mono".into()),
                        };
                        let deemphasis = match values[1] {
                            "50" => NativeFmDeemphasis::Us50,
                            "75" => NativeFmDeemphasis::Us75,
                            "off" => NativeFmDeemphasis::Off,
                            _ => return Err("Use 50|75|off".into()),
                        };
                        engine.set_nrsp_native_fm(NativeFmSettings {
                            stereo,
                            deemphasis,
                            low_cut_hz: values[2].parse().map_err(|_| "Invalid low cut")?,
                            high_cut_hz: values[3].parse().map_err(|_| "Invalid high cut")?,
                            ..Default::default()
                        })
                    })();
                    match result {
                        Ok(()) => println!("FM AUDIO REQUESTED"),
                        Err(error) => eprintln!("{error}"),
                    }
                }
                #[cfg(not(feature = "nrsp-native"))]
                {
                    let _ = value;
                    eprintln!("Enable nrsp-native for FM controls");
                }
            } else if let Some(value) = command.strip_prefix("volume ") {
                match value.parse::<u8>() {
                    Ok(value) if value <= 100 => {
                        engine.set_volume(value)?;
                        println!("RADIO VOLUME {}%", value);
                    }
                    _ => eprintln!("volume must be 0..100"),
                }
            } else {
                eprintln!("Commands: status | volume N | tune FREQ_HZ MODE [BW_HZ] | fm auto|mono 50|75|off LOW_HZ HIGH_HZ | stop");
            }
        }
        thread::sleep(Duration::from_millis(50));
    };
    engine.stop();
    println!("RUST RADIO / STOPPED");
    result
}

fn diagnostics_json(
    prebuffer_ms: u16,
    diagnostics: rust_radio::NativePlaybackDiagnostics,
) -> String {
    serde_json::json!({
        "output_prebuffer_ms": prebuffer_ms,
        "buffer_ready_latency_observed_ms": diagnostics.buffer_ready_latency_observed.map(|latency| latency.as_secs_f64() * 1000.0),
        "prebuffer_fill_latency_observed_ms": diagnostics.prebuffer_fill_latency_observed.map(|latency| latency.as_secs_f64() * 1000.0),
        "first_play_latency_observed_ms": diagnostics.first_play_latency_observed.map(|latency| latency.as_secs_f64() * 1000.0),
        "underrun_samples": diagnostics.underrun_samples,
    })
    .to_string()
}

/// Read one bounded command without allowing a long line or a fast producer to
/// grow the interactive CLI's memory use without limit. Oversized lines are
/// drained through their newline so the next command can still be processed.
#[derive(Debug)]
enum InteractiveCommandReadError {
    TooLong,
    Io(io::Error),
}

impl From<io::Error> for InteractiveCommandReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn read_interactive_command<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<String>, InteractiveCommandReadError> {
    let mut command = Vec::with_capacity(max_bytes.min(256));
    let mut saw_input = false;
    let mut oversized = false;
    loop {
        let (consumed, newline, eof) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                (0, false, true)
            } else {
                saw_input = true;
                let newline = available.contains(&b'\n');
                let consumed = available
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map_or(available.len(), |index| index + 1);
                let content_len = consumed - usize::from(newline);
                if !oversized {
                    if command.len().saturating_add(content_len) <= max_bytes {
                        command.extend_from_slice(&available[..content_len]);
                    } else {
                        oversized = true;
                    }
                }
                (consumed, newline, false)
            }
        };
        if eof {
            if !saw_input {
                return Ok(None);
            }
            if oversized {
                return Err(InteractiveCommandReadError::TooLong);
            }
            break;
        }
        reader.consume(consumed);
        if newline {
            if oversized {
                return Err(InteractiveCommandReadError::TooLong);
            }
            break;
        }
    }
    if command.last() == Some(&b'\r') {
        command.pop();
    }
    Ok(Some(String::from_utf8_lossy(&command).into_owned()))
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("inputs") if args.len() == 1 => {
            let mut scan = rust_radio::discovery::InputScan::start(None)?;
            let deadline = Instant::now() + Duration::from_secs(12);
            loop {
                if let Some(result) = scan.try_finish() {
                    let report = result?.ok_or("Input scan cancelled")?;
                    println!(
                        "IP Radio / {}",
                        if report.internet {
                            "available"
                        } else {
                            "offline or not verified"
                        }
                    );
                    println!(
                        "nSDR / {} found / {}",
                        report.network.len(),
                        report.network_note
                    );
                    for device in report.network {
                        println!("{} / {} / ready={}", device.id, device.label, device.ready);
                    }
                    println!("USB / {} found / {}", report.usb.len(), report.usb_note);
                    for device in report.usb {
                        println!("{} / {}", device.id, device.label);
                    }
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    scan.cancel();
                    return Err("Input scan deadline expired".into());
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        Some("stations") if args.len() == 1 => {
            for station in curated_stations() {
                println!("{}\t{}\t{}", station.id, station.name, station.category);
            }
            Ok(())
        }
        Some("outputs") if args.len() == 1 => {
            for output in available_outputs()? {
                println!("{}\t{}\t{}", output.id, output.name, output.available);
            }
            Ok(())
        }
        Some("validate") if args.len() == 2 => {
            let report = validate_station(&args[1])?;
            println!("{}", validation_summary(&report));
            if report.is_decoded() {
                Ok(())
            } else {
                Err("Station did not pass decoded-audio validation".into())
            }
        }
        Some("validate-all") if valid_validate_all_args(args) => {
            let results = collect_catalog_validations(validate_station)?;
            let report = batch_validation_json(results)?;
            if args.len() == 4 {
                let value: serde_json::Value = serde_json::from_str(&report)
                    .map_err(|error| format!("Unable to parse validation report: {error}"))?;
                feed_history::update_file(std::path::Path::new(&args[3]), &value)?;
            }
            println!("{report}");
            Ok(())
        }
        Some("listen" | "stream-listen" | "sdrconnect-listen") => listen(parse_listen(args)?),
        Some("sdr-devices") if args.len() == 1 => sdr::devices(),
        Some("sdr-capture") => sdr::capture(args),
        Some("sdr-listen") => sdr::listen(args),
        Some("nrsp-status") => nrsp::status(args),
        Some("nrsp-native-status" | "nrsp-listen") => nrsp_native::run(args),
        Some("sdrconnect-status" | "sdrconnect-capture") => sdrconnect::run(args),
        _ => {
            usage();
            Err("invalid command".into())
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("rust-radio: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ACTIVE_CHECKS: AtomicUsize = AtomicUsize::new(0);
    static MAX_ACTIVE_CHECKS: AtomicUsize = AtomicUsize::new(0);
    static RECOVERY_CHECKS: AtomicUsize = AtomicUsize::new(0);

    fn synthetic_check(station_id: &str) -> Result<StationValidation, String> {
        let active = ACTIVE_CHECKS.fetch_add(1, Ordering::SeqCst) + 1;
        MAX_ACTIVE_CHECKS.fetch_max(active, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(5));
        ACTIVE_CHECKS.fetch_sub(1, Ordering::SeqCst);
        Ok(StationValidation::unverified(station_id))
    }

    fn recovery_check(station_id: &str) -> Result<StationValidation, String> {
        let mut report = StationValidation::unverified(station_id);
        report.checked_at = Some(SystemTime::now());
        if station_id == "skylab" {
            if RECOVERY_CHECKS.fetch_add(1, Ordering::SeqCst) == 0 {
                report.status = ValidationStatus::Unavailable;
                report.stage = ValidationStage::EndpointReached;
                report.detail = "temporary network failure".into();
            } else {
                report.status = ValidationStatus::Decoded;
                report.stage = ValidationStage::DecodedAudio;
                report.decoded_frames = 1152;
                report.codec = Some("mp3".into());
            }
        } else if station_id == "wcrb" {
            report.status = ValidationStatus::InvalidStream;
            report.stage = ValidationStage::EndpointReached;
            report.detail = "HTTP 404".into();
        }
        Ok(report)
    }

    fn options(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| (*s).to_owned()).collect()
    }

    struct InvalidDataReader;

    impl io::Read for InvalidDataReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::InvalidData, "stdin failure"))
        }
    }

    impl BufRead for InvalidDataReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Err(io::Error::new(io::ErrorKind::InvalidData, "stdin failure"))
        }

        fn consume(&mut self, _amount: usize) {}
    }

    #[test]
    fn interactive_input_bounds_lines_and_recovers_after_oversized_input() {
        let input = format!("{}\n{}\nok\r\n", "x".repeat(4097), "y".repeat(4096));
        let mut reader = io::Cursor::new(input.into_bytes());

        assert!(matches!(
            read_interactive_command(&mut reader, 4096),
            Err(InteractiveCommandReadError::TooLong)
        ));
        assert_eq!(
            read_interactive_command(&mut reader, 4096).unwrap(),
            Some("y".repeat(4096))
        );
        assert_eq!(
            read_interactive_command(&mut reader, 4096).unwrap(),
            Some("ok".into())
        );
        assert_eq!(read_interactive_command(&mut reader, 4096).unwrap(), None);
    }

    #[test]
    fn interactive_input_preserves_reader_errors_instead_of_treating_them_as_long_lines() {
        let mut reader = InvalidDataReader;
        assert!(matches!(
            read_interactive_command(&mut reader, 4096),
            Err(InteractiveCommandReadError::Io(error))
                if error.kind() == io::ErrorKind::InvalidData
        ));
    }

    #[test]
    fn invalid_listen_args_fail_before_device_or_network_access() {
        for args in [
            vec!["listen", "skylab", "--seconds", "0"],
            vec!["listen", "unknown", "--seconds", "30"],
            vec!["listen", "skylab", "--seconds", "301"],
            vec!["listen", "skylab", "--volume", "101"],
            vec!["listen", "skylab", "--output", "https://localhost"],
            vec!["listen", "skylab", "--volume", "60", "--volume", "70"],
            vec!["listen", "skylab", "--buffering", "fast"],
        ] {
            assert!(parse_listen(&options(&args)).is_err(), "{:?}", args);
        }
        let parsed = options(&[
            "listen",
            "skylab",
            "--seconds",
            "30",
            "--volume",
            "25",
            "--output",
            "system",
        ]);
        assert_eq!(parse_listen(&parsed).unwrap().volume, 25);
        assert_eq!(parse_listen(&parsed).unwrap().seconds, Some(30));
        assert_eq!(
            parse_listen(&parsed).unwrap().buffering,
            IpBufferingProfile::Resilient
        );
        assert_eq!(
            parse_listen(&options(&[
                "listen",
                "skylab",
                "--buffering",
                "low-latency"
            ]))
            .unwrap()
            .buffering,
            IpBufferingProfile::LowLatency
        );
        assert!(parse_listen(&options(&["listen", "skylab"])).is_ok());
    }

    #[test]
    fn buffering_profiles_are_bounded_and_report_their_configured_duration() {
        assert_eq!(IpBufferingProfile::LowLatency.prebuffer_ms(), 200);
        assert_eq!(IpBufferingProfile::Resilient.prebuffer_ms(), 500);
        let diagnostics: serde_json::Value = serde_json::from_str(&diagnostics_json(
            200,
            rust_radio::NativePlaybackDiagnostics {
                underrun_samples: 23,
                buffer_ready_latency_observed: Some(Duration::from_millis(500)),
                prebuffer_fill_latency_observed: Some(Duration::from_millis(300)),
                first_play_latency_observed: Some(Duration::from_millis(734)),
            },
        ))
        .unwrap();
        assert_eq!(diagnostics["output_prebuffer_ms"].as_u64(), Some(200));
        assert_eq!(
            diagnostics["buffer_ready_latency_observed_ms"].as_f64(),
            Some(500.0)
        );
        assert_eq!(
            diagnostics["prebuffer_fill_latency_observed_ms"].as_f64(),
            Some(300.0)
        );
        assert_eq!(
            diagnostics["first_play_latency_observed_ms"].as_f64(),
            Some(734.0)
        );
        assert_eq!(diagnostics["underrun_samples"].as_u64(), Some(23));
    }

    #[cfg(feature = "sdrconnect")]
    #[test]
    fn native_sdr_listen_validates_peer_and_limits_before_resources() {
        for raw in [
            vec!["sdrconnect-listen", "8.8.8.8:5454"],
            vec!["sdrconnect-listen", "127.0.0.1:9", "--volume", "101"],
            vec!["sdrconnect-listen", "127.0.0.1:9", "--seconds", "0"],
        ] {
            assert!(parse_listen(&options(&raw)).is_err());
        }
        let args = options(&[
            "sdrconnect-listen",
            "127.0.0.1:5454",
            "--seconds",
            "3",
            "--volume",
            "10",
        ]);
        assert!(parse_listen(&args).unwrap().sdrconnect);
    }

    #[test]
    fn listing_is_network_free() {
        assert!(run(&options(&["stations"])).is_ok());
    }

    #[test]
    fn catalog_validation_is_bounded_ordered_and_machine_readable() {
        ACTIVE_CHECKS.store(0, Ordering::SeqCst);
        MAX_ACTIVE_CHECKS.store(0, Ordering::SeqCst);
        let results =
            collect_catalog_validations_with_delay(synthetic_check, Duration::ZERO).unwrap();
        let expected: Vec<_> = curated_stations()
            .iter()
            .map(|station| station.id)
            .collect();
        let actual: Vec<_> = results
            .iter()
            .map(|record| record.station_id.as_str())
            .collect();
        assert_eq!(actual, expected);
        assert!(MAX_ACTIVE_CHECKS.load(Ordering::SeqCst) > 1);
        assert!(MAX_ACTIVE_CHECKS.load(Ordering::SeqCst) <= VALIDATION_WORKERS);

        let json: serde_json::Value =
            serde_json::from_str(&batch_validation_json(results).unwrap()).unwrap();
        assert_eq!(json["schema_version"].as_u64(), Some(1));
        let stations = json["results"].as_array().unwrap();
        assert_eq!(stations.len(), curated_stations().len());
        assert_eq!(stations[0]["station_id"].as_str(), Some(expected[0]));
        assert_eq!(stations[0]["status"].as_str(), Some("unverified"));
        assert_eq!(stations[0]["decoded"].as_bool(), Some(false));
        assert!(stations[0].get("resolved_url").is_none());
        assert_eq!(stations[0]["attempts"].as_u64(), Some(1));
    }

    #[test]
    fn catalog_validation_retries_only_transient_failures_once() {
        RECOVERY_CHECKS.store(0, Ordering::SeqCst);
        let results =
            collect_catalog_validations_with_delay(recovery_check, Duration::ZERO).unwrap();
        assert_eq!(RECOVERY_CHECKS.load(Ordering::SeqCst), 2);
        let json: serde_json::Value =
            serde_json::from_str(&batch_validation_json(results).unwrap()).unwrap();
        let stations = json["results"].as_array().unwrap();
        let recovered = stations
            .iter()
            .find(|station| station["station_id"] == "skylab")
            .unwrap();
        assert_eq!(recovered["attempts"].as_u64(), Some(2));
        assert_eq!(recovered["recovered"].as_bool(), Some(true));
        assert_eq!(recovered["status"].as_str(), Some("decoded"));
        let rejected = stations
            .iter()
            .find(|station| station["station_id"] == "wcrb")
            .unwrap();
        assert_eq!(rejected["attempts"].as_u64(), Some(1));
        assert_eq!(rejected["status"].as_str(), Some("invalid_stream"));
    }

    #[test]
    fn json_catalog_mirrors_runtime_station_metadata_in_order() {
        let mirror: serde_json::Value =
            serde_json::from_str(include_str!("../../../catalog/curated-stations.json"))
                .expect("curated station JSON must parse");
        assert_eq!(mirror["schema_version"].as_u64(), Some(1));
        assert_eq!(mirror["policy"]["freshness_ttl_hours"].as_u64(), Some(6));
        assert_eq!(
            mirror["policy"]["required_validation"].as_str(),
            Some("decoded_audio")
        );
        assert_eq!(
            mirror["policy"]["validation_state"].as_str(),
            Some("unverified_until_runtime_probe")
        );
        assert_eq!(
            mirror["policy"]["endpoint_policy"].as_str(),
            Some("publisher_directed")
        );
        assert_eq!(
            mirror["policy"]["redistribution"].as_str(),
            Some("not_authorized")
        );
        assert_eq!(
            mirror["policy"]["third_party_player_evidence"].as_str(),
            Some("publisher documents listener playback in third-party players")
        );
        assert_eq!(
            mirror["policy"]["rights_reviewed_at"].as_str(),
            Some("2026-10-28")
        );
        let stations = mirror["stations"]
            .as_array()
            .expect("catalog must contain a station array");
        let runtime = curated_stations();
        assert_eq!(stations.len(), runtime.len());
        for (json, station) in stations.iter().zip(runtime) {
            assert_eq!(json["id"].as_str(), Some(station.id));
            assert_eq!(json["name"].as_str(), Some(station.name));
            assert_eq!(json["broadcaster"].as_str(), Some(station.broadcaster));
            assert_eq!(json["genre"].as_str(), Some(station.genre));
            assert_eq!(json["language"].as_str(), Some(station.language));
            assert_eq!(json["country"].as_str(), Some(station.country));
            assert_eq!(json["category"].as_str(), Some(station.category));
            assert_eq!(json["playlist_url"].as_str(), Some(station.playlist_url));
            assert_eq!(
                json["publisher_reference_url"].as_str(),
                Some(station.publisher_reference_url)
            );
            assert_eq!(
                json["third_party_player_support"].as_bool(),
                Some(station.third_party_player_support)
            );
            assert_eq!(
                json["rights_reviewed_date"].as_str(),
                Some(station.rights_reviewed_date)
            );
            assert_eq!(
                json["nominal_kbps"].as_u64(),
                Some(u64::from(station.nominal_kbps))
            );
            assert_eq!(json["format"].as_str(), Some(station.format));
        }
    }

    #[test]
    fn invalid_validation_requests_fail_without_network_or_audio() {
        for args in [
            vec!["validate"],
            vec!["validate", "unknown-station"],
            vec!["validate", "https://localhost/admin"],
            vec!["validate", "skylab", "unexpected"],
        ] {
            assert!(run(&options(&args)).is_err());
        }
        assert!(!valid_validate_all_args(&options(&["validate-all"])));
        assert!(!valid_validate_all_args(&options(&[
            "validate-all",
            "--json",
            "--history",
            ""
        ])));
        assert!(valid_validate_all_args(&options(&[
            "validate-all",
            "--json",
            "--history",
            "health.json"
        ])));
    }
}
