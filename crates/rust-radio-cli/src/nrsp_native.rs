#[cfg(feature = "nrsp-native")]
pub(super) fn run(args: &[String]) -> Result<(), String> {
    use rust_radio::receiver::nrsp_native::{Client, Endpoint, Mode};
    use std::sync::atomic::AtomicBool;
    if args.first().map(String::as_str) == Some("nrsp-native-status") {
        if args.len() != 2 {
            return Err("Use nrsp-native-status PRIVATE_IP:9001".into());
        }
        let endpoint = Endpoint::parse(&args[1]).map_err(|e| e.to_string())?;
        let client =
            Client::connect(endpoint, &AtomicBool::new(false)).map_err(|e| e.to_string())?;
        println!("NATIVE nRSP-ST / {:?}", client.status());
        return Ok(());
    }
    if args.len() < 4 {
        return Err("Use nrsp-listen PRIVATE_IP:9001 FREQ_HZ am|usb|lsb|nfm|wfm|cw|sam [--seconds N] [--volume N] [--output ID]".into());
    }
    Endpoint::parse(&args[1]).map_err(|e| e.to_string())?;
    let frequency = args[2]
        .parse::<u64>()
        .map_err(|_| "Invalid receiver frequency")?;
    let mode = Mode::parse(&args[3]).map_err(|e| e.to_string())?;
    let mut options = vec!["stream-listen".to_owned(), args[1].clone()];
    options.extend_from_slice(&args[4..]);
    let listen = super::parse_listen(&options)?;
    let engine = rust_radio::NativeRadioEngine::start_nrsp_native(
        &args[1],
        frequency,
        &args[3],
        mode.default_bandwidth(),
        listen.volume,
        listen.output,
    )?;
    println!(
        "NATIVE nRSP-ST / {frequency} Hz / {mode:?} / bandwidth {} Hz",
        mode.default_bandwidth()
    );
    super::monitor(engine, listen.seconds)
}
#[cfg(not(feature = "nrsp-native"))]
pub(super) fn run(_: &[String]) -> Result<(), String> {
    Err("Rebuild with --features nrsp-native for direct nRSP-ST reception".into())
}
