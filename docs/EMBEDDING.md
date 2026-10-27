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
