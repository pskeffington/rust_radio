use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
struct CaptureArgs<'a> {
    id: &'a str,
    frequency_hz: u64,
    sample_rate_hz: u32,
    duration: Duration,
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
