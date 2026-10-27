#[cfg(feature = "sdrconnect")]
mod enabled {
    use rust_radio::receiver::sdrconnect::{Client, Endpoint, StreamKind};
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    enum Command {
        Status,
        Receive(StreamKind, Duration),
    }
    fn parse(args: &[String]) -> Result<(Endpoint, Command), String> {
        let usage = "Use sdrconnect-status IP:PORT or sdrconnect-capture IP:PORT iq|audio --milliseconds 1..5000";
        let command = match args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["sdrconnect-status", _] => Command::Status,
            ["sdrconnect-capture", _, kind, "--milliseconds", ms] => {
                let kind = match *kind {
                    "iq" => StreamKind::Iq,
                    "audio" => StreamKind::Audio,
                    _ => return Err(usage.into()),
                };
                let ms: u64 = ms.parse().map_err(|_| usage)?;
                if !(1..=5000).contains(&ms) {
                    return Err(usage.into());
                }
                Command::Receive(kind, Duration::from_millis(ms))
            }
            _ => return Err(usage.into()),
        };
        Ok((
            Endpoint::parse(&args[1]).map_err(|e| e.to_string())?,
            command,
        ))
    }
    pub(super) fn run(args: &[String]) -> Result<(), String> {
        let (endpoint, command) = parse(args)?;
        let cancel = AtomicBool::new(false);
        let mut client = Client::connect(endpoint, &cancel).map_err(|e| e.to_string())?;
        match command {
            Command::Status => {
                let status = client.status(&cancel).map_err(|e| e.to_string())?;
                println!(
                    "SDRconnect API {} / {} / started: {} / control: {}",
                    status.api_version, status.active_device, status.started, status.can_control
                );
                println!("Devices: {:?}", status.valid_devices);
                println!(
                    "{} Hz / {} samples/s / {}",
                    status.center_frequency_hz, status.sample_rate_hz, status.demodulator
                );
                println!(
                    "Antennas: {:?} / selected: {}",
                    status.antennas, status.active_antenna
                );
                println!("Read-only status; no tuning or stream activation performed.");
            }
            Command::Receive(kind, duration) => {
                let report = client
                    .receive_for(kind, duration, &cancel, |_| {})
                    .map_err(|e| e.to_string())?;
                println!(
                    "SDRconnect {:?} / {} sample pairs / {:?} / unsubscribed",
                    kind, report.sample_pairs, report.elapsed
                );
                println!("Network ingress only; no audio output or physical signal verification performed.");
                if report.sample_pairs == 0 {
                    return Err("No sample pairs received before the deadline".into());
                }
            }
        }
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn invalid_network_commands_fail_before_connecting() {
            for raw in [
                vec!["sdrconnect-status"],
                vec!["sdrconnect-status", "8.8.8.8:5454"],
                vec![
                    "sdrconnect-capture",
                    "127.0.0.1:9",
                    "audio",
                    "--milliseconds",
                    "0",
                ],
                vec![
                    "sdrconnect-capture",
                    "127.0.0.1:9",
                    "iq",
                    "--milliseconds",
                    "5001",
                ],
                vec![
                    "sdrconnect-capture",
                    "127.0.0.1:9",
                    "unknown",
                    "--milliseconds",
                    "500",
                ],
            ] {
                let args: Vec<_> = raw.into_iter().map(str::to_owned).collect();
                assert!(parse(&args).is_err());
            }
        }
    }
}

pub(super) fn run(args: &[String]) -> Result<(), String> {
    #[cfg(feature = "sdrconnect")]
    {
        enabled::run(args)
    }
    #[cfg(not(feature = "sdrconnect"))]
    {
        let _ = args;
        Err("Enable the optional sdrconnect feature for nRSP-ST network reception (no SoapySDR SDK needed)".into())
    }
}
