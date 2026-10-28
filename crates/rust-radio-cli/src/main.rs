//! Operator-triggered standalone test harness. No automatic network or audio I/O.
use rust_radio::directory::{curated_stations, station_by_id};
use rust_radio::output::available_outputs;
use rust_radio::validation::validate_station;
use rust_radio::validation_types::StationValidation;
use rust_radio::NativeRadioEngine;
use std::io::{self, BufRead};
use std::sync::mpsc;
use std::thread;
use std::time::UNIX_EPOCH;
use std::time::{Duration, Instant};
use std::{env, process};

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
}

fn usage() {
    eprintln!("Usage: rust-radio-cli stations | outputs | validate STATION | listen STATION [--seconds 1..300] [--output ID] [--volume 0..100]");
    eprintln!(
        "Direct stream: stream-listen URL [--seconds 1..300] [--output ID] [--volume 0..100]"
    );
    eprintln!("Input discovery: inputs (enable nrsp-native and/or soapy-sdr for receiver scans)");
    eprintln!("Interactive listen commands: status | volume N | stop");
    eprintln!("Optional SDR: sdr-devices | sdr-capture DEVICE_ID FREQ_HZ RATE_HZ [--milliseconds 1..5000] (enable soapy-sdr)");
    eprintln!("USB mono FM: sdr-listen DEVICE_ID FREQ_HZ [--seconds 1..300] [--volume 0..100] [--output ID] [--deemphasis 50|75] (enable soapy-sdr)");
    eprintln!("Direct nRSP-ST control/audio: nrsp-native-status IP:9001 | nrsp-listen IP:9001 FREQ_HZ MODE [--seconds N] [--volume N] [--output ID] (enable nrsp-native)");
    eprintln!("Direct nRSP-ST reachability: nrsp-status PRIVATE_IPV4 (enable sdr)");
    eprintln!("nRSP-ST via SDRconnect: sdrconnect-status IP:PORT | sdrconnect-capture IP:PORT iq|audio --milliseconds 1..5000 (enable sdrconnect)");
    eprintln!("Native SDR audio: sdrconnect-listen IP:PORT [--seconds 1..300] [--output ID] [--volume 0..100]");
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
    };
    let mut used_seconds = false;
    let mut used_output = false;
    let mut used_volume = false;
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
            _ => return Err("unknown or repeated listen option"),
        }
        i += 2;
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
        NativeRadioEngine::start_stream(args.station, args.volume, args.output)?
    } else {
        NativeRadioEngine::start_internet(args.station, args.volume, args.output)?
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
    monitor(engine, args.seconds)
}

fn monitor(mut engine: NativeRadioEngine, seconds: Option<u64>) -> Result<(), String> {
    // A separate stdin reader lets the main thread monitor duration and playback failures.
    let (tx, rx) = mpsc::channel::<String>();
    if seconds.is_none() {
        println!("Commands: status | volume N | tune FREQ_HZ MODE [BW_HZ] | stop");
        thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                match line {
                    Ok(command) => {
                        if tx.send(command).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
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
                    Ok(())
                } else {
                    Err("No decoded PCM reached the audio callback".into())
                };
            }
        } else if let Ok(command) = rx.try_recv() {
            let command = command.trim();
            if command == "stop" || command == "q" {
                break Ok(());
            } else if command == "status" {
                println!("RADIO {:?} / {:?}", snapshot.state, snapshot.last_error);
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
            } else if let Some(value) = command.strip_prefix("volume ") {
                match value.parse::<u8>() {
                    Ok(value) if value <= 100 => {
                        engine.set_volume(value)?;
                        println!("RADIO VOLUME {}%", value);
                    }
                    _ => eprintln!("volume must be 0..100"),
                }
            } else {
                eprintln!("Commands: status | volume N | tune FREQ_HZ MODE [BW_HZ] | stop");
            }
        }
        thread::sleep(Duration::from_millis(50));
    };
    engine.stop();
    println!("RUST RADIO / STOPPED");
    result
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

    fn options(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| (*s).to_owned()).collect()
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
        assert!(parse_listen(&options(&["listen", "skylab"])).is_ok());
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
    fn invalid_validation_requests_fail_without_network_or_audio() {
        for args in [
            vec!["validate"],
            vec!["validate", "unknown-station"],
            vec!["validate", "https://localhost/admin"],
            vec!["validate", "skylab", "unexpected"],
        ] {
            assert!(run(&options(&args)).is_err());
        }
    }
}
