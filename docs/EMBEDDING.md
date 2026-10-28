# Standalone radio module and host integration

`rust-radio` is the public entry point. It is independently runnable through
`rust-radio-cli` and independently embeddable as a Rust library. A host does not
launch the standalone GUI, browser or another audio engine.

```toml
[dependencies]
rust-radio = { git = "https://github.com/pskeffington/rust_radio", rev = "<reviewed-commit>" }
```

Default features include the headless IP transport, MP3/AAC decoder and native
output. `default-features = false` gives only pure control/catalog APIs; `sdr`
adds the receive-only tuner contract and explicit simulator. `gui` is optional
and adds `RadioPanel` for egui hosts. Hardware reception is a separate adapter
and acceptance gate; simulated IQ is never evidence of received RF.

`soapy-sdr` adds the optional SDK-backed receive adapter through
`rust_radio::receiver`, including explicit discovery, device open and bounded
I/Q capture. See [SDR receive limits and acceptance](SDR_RECEIVE.md). This feature
requires the native SDK and Rust 1.85; it adds no dependency to the default IP
or pure-control builds.

With default audio plus `soapy-sdr`, `NativeRadioEngine::start_soapy_wfm()`
starts the initial 240 ksample/s mono broadcast-FM path on a selected device.
Enable `gui` and `soapy-sdr` together for its optional panel controls. Synthetic
checks have passed; physical USB audio is not yet accepted.

`sdrconnect` adds the SDK-independent network API for the selected SDRplay
nRSP-ST through `rust_radio::receiver::sdrconnect`. It is independent of
`soapy-sdr` and the GUI, and requires Rust 1.85 or newer. See
[nRSP-ST setup and current limits](NRSP_ST.md).

The lighter `sdr` feature also exports `receiver::nrsp::Host` for explicit,
read-only checks of the nRSP-ST's direct web and device services. This path has
no SDRconnect or USB SDK dependency and does not yet receive audio or I/Q.

Enable `gui` together with `sdrconnect` to include the nRSP-ST source and local
API address field in the optional `RadioPanel`. Its phosphor skin is scoped to
the panel and settings window, leaving the host's global egui style intact.
Selecting a source or editing its address does not start a connection.

```rust
use rust_radio::NativeRadioEngine;

fn play_after_user_command(station_id: &str) -> Result<(), String> {
    let mut engine = NativeRadioEngine::start_internet(station_id, 35, "system")?;
    let snapshot = engine.status();
    engine.set_volume(40)?;
    engine.request_stop();
    // In a GUI, retain the engine and poll from the host event loop instead.
    while !engine.try_finish() {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    Ok(())
}
```

Hosts retain ownership of their UI, preferences and one engine at a time. The
standalone egui panel is not required by pip_boy's HTML HUD. Stable station IDs,
playback states and generation-tagged commands/events are shared through
`rust_radio::controller`; directory, parser, PCM and recovery APIs are also
available without loading audio dependencies. Host storage continues to use
station IDs, not arbitrary URLs.

Compatibility imports in `rust-radio-audio::compat_controller` refer to the
same core controller types. `rust-radio-core::SessionState` is the same playback
state type, so source selection and output-recovery policy cannot drift from the
native worker's state vocabulary.

With default IP/audio features plus `sdrconnect`, the same `NativeRadioEngine`
also accepts `start_sdrconnect(peer, volume, output)`. Its status uses
`InputKind::Sdr`; `receiver_status()` exposes the last server-reported tuning
snapshot without network I/O. Stop/reaping and output ownership follow the same
contract as IP playback. A control-only `sdrconnect` dependency still links no
native audio or USB SDR SDK.

Explicit `start_sdrconnect()` playback starts the selected receiver if stopped
and SDRconnect grants control. It waits for confirmation before opening native
output. A three-second startup confirmation failure is terminal (`StartTimeout`),
so network recovery does not repeat activation; the host can offer explicit Play
again after setup is corrected. Transport failures retain bounded recovery.
Status and capture remain passive, and local Stop leaves shared reception running.

For the Pi target, the headless receiver dependency can be checked without audio
or SDK libraries:

```sh
rustup target add aarch64-unknown-linux-gnu
cargo check --locked -p rust-radio --no-default-features --features sdrconnect \
  --target aarch64-unknown-linux-gnu
```

This checks compilation only. See the [roadmap](PORTING_PLAN.md) and
[progress notes](PROGRESS.md) for remaining physical acceptance.
