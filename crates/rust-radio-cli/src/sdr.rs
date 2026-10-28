use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
struct CaptureArgs<'a> {
    id: &'a str,
    frequency_hz: u64,
    sample_rate_hz: u32,
    duration: Duration,
}

#[derive(Debug, PartialEq, Eq)]
struct WfmArgs<'a> {
    id: &'a str,
    frequency_hz: u64,
    seconds: Option<u64>,
    volume: u8,
    output: &'a str,
    deemphasis_us: u8,
}

fn parse_wfm(args: &[String]) -> Result<WfmArgs<'_>, String> {
    if args.len() < 3 || args[0] != "sdr-listen" {
        return Err("Use sdr-listen DEVICE_ID FREQ_HZ [--seconds 1..300] [--volume 0..100] [--output ID] [--deemphasis 50|75]".into());
    }
    let id = args[1].as_str();
    if !id.starts_with("soapy-")
        || id.len() != 22
        || !id[6..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Copy a receiver ID from sdr-devices".into());
    }
    let frequency_hz = args[2]
        .parse::<u64>()
        .map_err(|_| "Frequency must be positive integer Hz")?;
    if frequency_hz == 0 {
        return Err("Frequency must be greater than zero".into());
    }
    let mut result = WfmArgs {
        id,
        frequency_hz,
        seconds: None,
        volume: 60,
        output: "system",
        deemphasis_us: 75,
    };
    let mut seen = std::collections::BTreeSet::new();
    for pair in args[3..].chunks(2) {
        if pair.len() != 2 || !seen.insert(pair[0].as_str()) {
            return Err("Missing or repeated option".into());
        }
        match pair[0].as_str() {
            "--seconds" => {
                let value = pair[1].parse::<u64>().map_err(|_| "Invalid duration")?;
                if !(1..=300).contains(&value) {
                    return Err("Duration must be 1..300 seconds".into());
                }
                result.seconds = Some(value);
            }
            "--volume" => {
                let value = pair[1].parse::<u8>().map_err(|_| "Invalid volume")?;
                if value > 100 {
                    return Err("Volume must be 0..100".into());
                }
                result.volume = value;
            }
            "--output" => {
                let id = pair[1].as_str();
                if id != "system"
                    && (!id.starts_with("output-")
                        || !id
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
                {
                    return Err("Invalid output identifier".into());
                }
                result.output = id;
            }
            "--deemphasis" => {
                result.deemphasis_us = match pair[1].as_str() {
                    "50" => 50,
                    "75" => 75,
                    _ => return Err("Deemphasis must be 50 or 75 microseconds".into()),
                };
            }
            _ => return Err("Unknown listen option".into()),
        }
    }
    Ok(result)
}

pub(super) fn listen(args: &[String]) -> Result<(), String> {
    let request = parse_wfm(args)?;
    run_wfm(request)
}

#[cfg(not(feature = "soapy-sdr"))]
fn run_wfm(_: WfmArgs<'_>) -> Result<(), String> {
    devices()
}

#[cfg(feature = "soapy-sdr")]
fn run_wfm(request: WfmArgs<'_>) -> Result<(), String> {
    use rust_radio::sdr::wfm::Deemphasis;
    let deemphasis = if request.deemphasis_us == 50 {
        Deemphasis::Us50
    } else {
        Deemphasis::Us75
    };
    let engine = rust_radio::NativeRadioEngine::start_soapy_wfm(
        request.id,
        request.frequency_hz,
        deemphasis,
        request.volume,
        request.output,
    )?;
    println!(
        "RUST RADIO / USB MONO FM / {} Hz / output {} / volume {}%",
        request.frequency_hz, request.output, request.volume
    );
    super::monitor(engine, request.seconds)
}

fn parse(args: &[String]) -> Result<CaptureArgs<'_>, String> {
    if !matches!(args.len(), 4 | 6) || args[0] != "sdr-capture" {
        return Err("Use sdr-capture DEVICE_ID FREQ_HZ RATE_HZ [--milliseconds 1..5000]".into());
    }
    let id = args[1].as_str();
    if !id.starts_with("soapy-")
        || id.len() != 22
        || !id[6..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("Copy a receiver ID from sdr-devices".into());
    }
    let frequency_hz = args[2]
        .parse::<u64>()
        .map_err(|_| "Frequency must be positive integer Hz")?;
    let sample_rate_hz = args[3]
        .parse::<u32>()
        .map_err(|_| "Rate must be positive integer Hz")?;
    if frequency_hz == 0 || sample_rate_hz == 0 {
        return Err("Frequency and sample rate must be greater than zero".into());
    }
    let milliseconds = if args.len() == 6 {
        if args[4] != "--milliseconds" {
            return Err("Unknown capture option".into());
        }
        args[5]
            .parse::<u64>()
            .map_err(|_| "Invalid capture duration")?
    } else {
        500
    };
    if !(1..=5000).contains(&milliseconds) {
        return Err("Capture duration must be 1..5000 milliseconds".into());
    }
    Ok(CaptureArgs {
        id,
        frequency_hz,
        sample_rate_hz,
        duration: Duration::from_millis(milliseconds),
    })
}

#[cfg(feature = "soapy-sdr")]
pub(super) fn devices() -> Result<(), String> {
    let receivers = rust_radio::receiver::discover().map_err(|error| error.to_string())?;
    if receivers.is_empty() {
        println!("No physical SDR receivers found. IP radio remains available; nothing started.");
    }
    for receiver in receivers {
        println!("{}\t{}\t{}", receiver.id, receiver.driver, receiver.label);
    }
    Ok(())
}

#[cfg(not(feature = "soapy-sdr"))]
pub(super) fn devices() -> Result<(), String> {
    Err("Hardware SDR support is optional; rebuild with --features soapy-sdr".into())
}

pub(super) fn capture(args: &[String]) -> Result<(), String> {
    let request = parse(args)?;
    run_capture(request)
}

#[cfg(not(feature = "soapy-sdr"))]
fn run_capture(_: CaptureArgs<'_>) -> Result<(), String> {
    devices()
}

#[cfg(feature = "soapy-sdr")]
fn run_capture(request: CaptureArgs<'_>) -> Result<(), String> {
    use rust_radio::sdr::{Demodulation, RadioDevice, Tuning};
    let receiver = rust_radio::receiver::SoapyReceiver::open(request.id, 0, request.sample_rate_hz)
        .map_err(|error| error.to_string())?;
    println!(
        "RECEIVE ONLY / {} / {} Hz / {} samples/s",
        receiver.capabilities().name,
        request.frequency_hz,
        request.sample_rate_hz
    );
    let report = rust_radio::receiver::capture(
        receiver,
        Tuning {
            frequency_hz: request.frequency_hz,
            sample_rate_hz: request.sample_rate_hz,
            demodulation: Demodulation::RawIq,
            gain_db_tenths: None,
        },
        request.duration,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{:?} / {} complex samples / {:?} / stopped",
        report.receiver_kind, report.complex_samples, report.elapsed
    );
    println!("I/Q ingress only: no RF audio demodulation or signal verification was performed.");
    if report.complex_samples == 0 {
        return Err("No I/Q samples received before the deadline".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_listen_validates_receiver_frequency_and_options_before_io() {
        for args in [
            vec!["sdr-listen"],
            vec!["sdr-listen", "driver=remote", "100100000"],
            vec!["sdr-listen", "soapy-0000000000000000", "0"],
            vec![
                "sdr-listen",
                "soapy-0000000000000000",
                "100100000",
                "--deemphasis",
                "30",
            ],
            vec![
                "sdr-listen",
                "soapy-0000000000000000",
                "100100000",
                "--volume",
                "101",
            ],
            vec![
                "sdr-listen",
                "soapy-0000000000000000",
                "100100000",
                "--seconds",
                "1",
                "--seconds",
                "2",
            ],
        ] {
            assert!(parse_wfm(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
        let args = [
            "sdr-listen",
            "soapy-0000000000000000",
            "100100000",
            "--seconds",
            "5",
            "--volume",
            "0",
            "--deemphasis",
            "50",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let parsed = parse_wfm(&args).unwrap();
        assert_eq!(parsed.frequency_hz, 100_100_000);
        assert_eq!(parsed.seconds, Some(5));
        assert_eq!(parsed.volume, 0);
        assert_eq!(parsed.deemphasis_us, 50);
    }

    #[test]
    fn invalid_capture_requests_fail_before_discovery_or_open() {
        for raw in [
            vec!["sdr-capture"],
            vec!["sdr-capture", "driver=remote", "100100000", "1024000"],
            vec!["sdr-capture", "soapy-0000000000000000", "0", "1024000"],
            vec!["sdr-capture", "soapy-0000000000000000", "100100000", "0"],
            vec![
                "sdr-capture",
                "soapy-0000000000000000",
                "100100000",
                "1024000",
                "--milliseconds",
                "5001",
            ],
            vec![
                "sdr-capture",
                "soapy-0000000000000000",
                "100100000",
                "1024000",
                "--milliseconds",
                "0",
            ],
        ] {
            let args: Vec<String> = raw.into_iter().map(str::to_owned).collect();
            assert!(capture(&args).is_err());
        }
        let args: Vec<String> = [
            "sdr-capture",
            "soapy-0000000000000000",
            "100100000",
            "1024000",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(parse(&args).unwrap().duration, Duration::from_millis(500));
    }
}
