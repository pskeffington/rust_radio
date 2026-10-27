//! Operator-triggered standalone test harness. No automatic network or audio I/O.
use rust_radio_audio::output::available_outputs;
use rust_radio_audio::playback::NativeRadioEngine;
use rust_radio_core::directory::{curated_stations, station_by_id};
use std::io::{self, BufRead};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::{env, process};

#[derive(Debug, PartialEq, Eq)]
struct ListenArgs<'a> {
    station: &'a str,
    seconds: Option<u64>,
    output: &'a str,
    volume: u8,
}

fn usage() {
    eprintln!("Usage: rust-radio-cli stations | outputs | listen STATION [--seconds 1..300] [--output ID] [--volume 0..100]");
    eprintln!("Interactive listen commands: status | volume N | stop");
}

fn parse_listen(args: &[String]) -> Result<ListenArgs<'_>, &'static str> {
    if args.len() < 2 || args[0] != "listen" || station_by_id(&args[1]).is_none() {
        return Err("unknown curated station");
    }
    let mut parsed = ListenArgs {
        station: &args[1],
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
    let mut engine = NativeRadioEngine::start_internet(args.station, args.volume, args.output)?;
    println!(
        "RUST RADIO / {} / output {} / volume {}%",
        args.station, args.output, args.volume
    );

    // A separate stdin reader lets the main thread monitor duration and playback failures.
    let (tx, rx) = mpsc::channel::<String>();
    if args.seconds.is_none() {
        println!("Commands: status | volume N | stop");
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
        if snapshot.state == rust_radio_audio::compat_controller::PlaybackState::Playing {
            audible = true;
        }
        if snapshot.state == rust_radio_audio::compat_controller::PlaybackState::Failed {
            break Err(snapshot
                .last_error
                .unwrap_or_else(|| "playback failed".to_owned()));
        }
        if let Some(seconds) = args.seconds {
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
            } else if let Some(value) = command.strip_prefix("volume ") {
                match value.parse::<u8>() {
                    Ok(value) if value <= 100 => {
                        engine.set_volume(value)?;
                        println!("RADIO VOLUME {}%", value);
                    }
                    _ => eprintln!("volume must be 0..100"),
                }
            } else {
                eprintln!("Commands: status | volume N | stop");
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
        Some("listen") => listen(parse_listen(args)?),
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

    #[test]
    fn listing_is_network_free() {
        assert!(run(&options(&["stations"])).is_ok());
    }
}
