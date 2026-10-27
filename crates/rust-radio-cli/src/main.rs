//! Operator-triggered standalone test harness. No automatic network or audio I/O.
use rust_radio_audio::playback::run_demo;
use rust_radio_audio::output::available_outputs;
use rust_radio_core::directory::curated_stations;
use std::{env, process};

fn usage() {
    eprintln!("Usage: rust-radio-cli stations | outputs | listen STATION --seconds 1..300");
}

fn parse_listen(args: &[String]) -> Result<(&str, u64), &'static str> {
    if args.len() != 4 || args[2] != "--seconds" {
        return Err("listen requires STATION --seconds 1..300");
    }
    let seconds = args[3].parse::<u64>().map_err(|_| "invalid duration")?;
    if !(1..=300).contains(&seconds) {
        return Err("duration must be 1..300 seconds");
    }
    if rust_radio_core::directory::station_by_id(&args[1]).is_none() {
        return Err("unknown curated station");
    }
    Ok((&args[1], seconds))
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
        Some("listen") => {
            let (station, seconds) = parse_listen(args)?;
            // This is the only subcommand that can start remote receiver I/O.
            run_demo(station, Some(seconds))
        }
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

    #[test]
    fn invalid_listen_args_fail_before_any_hardware_or_network_access() {
        assert!(parse_listen(&["skylab".into(), "--seconds".into(), "0".into()]).is_err());
        assert!(parse_listen(&["unknown".into(), "--seconds".into(), "30".into()]).is_err());
        assert!(parse_listen(&["skylab".into(), "--seconds".into(), "301".into()]).is_err());
        assert!(parse_listen(&["skylab".into(), "--seconds".into(), "30".into()]).is_ok());
    }

    #[test]
    fn station_listing_is_pure_and_available_offline() {
        assert!(run(&["stations".into()]).is_ok());
    }
}
