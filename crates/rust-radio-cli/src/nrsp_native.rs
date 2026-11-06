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
    let (options, fm, bandwidth, fm_used) = fm_options(args, mode.default_bandwidth())?;
    if fm_used && mode != Mode::Wfm {
        return Err("FM audio options require wfm mode".into());
    }
    let listen = super::parse_listen(&options)?;
    let engine = if mode == Mode::Wfm {
        rust_radio::NativeRadioEngine::start_nrsp_native_fm(
            &args[1],
            frequency,
            bandwidth,
            fm,
            listen.volume,
            listen.output,
        )?
    } else {
        rust_radio::NativeRadioEngine::start_nrsp_native(
            &args[1],
            frequency,
            &args[3],
            bandwidth,
            listen.volume,
            listen.output,
        )?
    };
    println!("NATIVE nRSP-ST / {frequency} Hz / {mode:?} / bandwidth {bandwidth} Hz");
    super::monitor(engine, listen.seconds, 200)
}
#[cfg(not(feature = "nrsp-native"))]
pub(super) fn run(_: &[String]) -> Result<(), String> {
    Err("Rebuild with --features nrsp-native for direct nRSP-ST reception".into())
}

#[cfg(feature = "nrsp-native")]
fn fm_options(
    args: &[String],
    default_bandwidth: u32,
) -> Result<
    (
        Vec<String>,
        rust_radio::playback::NativeFmSettings,
        u32,
        bool,
    ),
    String,
> {
    use rust_radio::playback::{NativeFmDeemphasis, NativeFmSettings};
    let mut options = vec!["stream-listen".into(), args[1].clone()];
    let mut fm = NativeFmSettings::default();
    let mut bandwidth = default_bandwidth;
    let mut used = std::collections::BTreeSet::new();
    let mut fm_used = false;
    let mut index = 4;
    while index < args.len() {
        let value = args.get(index + 1).ok_or("Missing option value")?;
        match args[index].as_str() {
            "--stereo" | "--deemphasis" | "--low-cut" | "--high-cut" | "--bandwidth" => {
                if !used.insert(args[index].as_str()) {
                    return Err("Repeated receiver option".into());
                }
                match args[index].as_str() {
                    "--stereo" => {
                        fm_used = true;
                        fm.stereo = match value.as_str() {
                            "auto" => true,
                            "mono" => false,
                            _ => return Err("Use --stereo auto|mono".into()),
                        };
                    }
                    "--deemphasis" => {
                        fm_used = true;
                        fm.deemphasis = match value.as_str() {
                            "50" => NativeFmDeemphasis::Us50,
                            "75" => NativeFmDeemphasis::Us75,
                            "off" => NativeFmDeemphasis::Off,
                            _ => return Err("Use --deemphasis 50|75|off".into()),
                        };
                    }
                    "--low-cut" => {
                        fm_used = true;
                        fm.low_cut_hz = value.parse().map_err(|_| "Invalid low cut")?;
                    }
                    "--high-cut" => {
                        fm_used = true;
                        fm.high_cut_hz = value.parse().map_err(|_| "Invalid high cut")?;
                    }
                    "--bandwidth" => {
                        bandwidth = value.parse().map_err(|_| "Invalid RF bandwidth")?
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                options.push(args[index].clone());
                options.push(value.clone());
            }
        }
        index += 2;
    }
    fm.validate().map_err(|e| e.to_string())?;
    if !(100..=250_000).contains(&bandwidth) {
        return Err("Invalid RF bandwidth".into());
    }
    Ok((options, fm, bandwidth, fm_used))
}
#[cfg(all(test, feature = "nrsp-native"))]
mod tests {
    use super::*;
    #[test]
    fn fm_options_validate_before_resources() {
        let base = ["nrsp-listen", "127.0.0.1:9001", "99500000", "wfm"];
        for extra in [
            vec!["--stereo", "invalid"],
            vec!["--deemphasis", "1"],
            vec!["--low-cut", "5000"],
            vec!["--high-cut", "16000"],
            vec!["--bandwidth", "0"],
            vec!["--stereo", "auto", "--stereo", "mono"],
        ] {
            let args: Vec<_> = base
                .iter()
                .copied()
                .chain(extra)
                .map(str::to_owned)
                .collect();
            assert!(fm_options(&args, 200000).is_err());
        }
        let args: Vec<_> = base
            .iter()
            .copied()
            .chain(["--stereo", "mono", "--deemphasis", "50", "--seconds", "5"])
            .map(str::to_owned)
            .collect();
        let (options, fm, _, _) = fm_options(&args, 200000).unwrap();
        assert!(!fm.stereo);
        assert_eq!(
            fm.deemphasis,
            rust_radio::playback::NativeFmDeemphasis::Us50
        );
        assert!(super::super::parse_listen(&options).is_ok());
    }
}
