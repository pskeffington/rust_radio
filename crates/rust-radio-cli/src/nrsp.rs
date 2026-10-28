#[cfg(feature = "sdr")]
pub(super) fn status(args: &[String]) -> Result<(), String> {
    use rust_radio::receiver::nrsp::Host;
    use std::sync::atomic::AtomicBool;
    if args.len() != 2 {
        return Err("Use nrsp-status PRIVATE_IPV4".into());
    }
    let host = Host::parse(&args[1]).map_err(|error| error.to_string())?;
    let status = host
        .status(&AtomicBool::new(false))
        .map_err(|error| error.to_string())?;
    println!(
        "Direct receiver web service :9001 / connected={} / HTTP={}",
        status.web_connected,
        status
            .web_http_status
            .map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
    );
    println!(
        "Direct receiver device service :50000 / connected={}",
        status.device_connected
    );
    println!("Reachability only: no receiver identity, audio, I/Q or tuning was verified.");
    if !status.web_connected && !status.device_connected {
        return Err("No direct receiver services were reachable".into());
    }
    Ok(())
}

#[cfg(not(feature = "sdr"))]
pub(super) fn status(_: &[String]) -> Result<(), String> {
    Err("Direct receiver status is optional; rebuild with --features sdr".into())
}

#[cfg(all(test, feature = "sdr"))]
mod tests {
    use super::*;

    #[test]
    fn invalid_status_requests_are_rejected_without_connecting() {
        for args in [
            vec!["nrsp-status"],
            vec!["nrsp-status", "8.8.8.8"],
            vec!["nrsp-status", "radio.local"],
            vec!["nrsp-status", "127.0.0.1:9001"],
        ] {
            assert!(status(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
        }
    }
}
