# Native audio extraction gate — 2026-10-26

## Extracted sources

- `crates/rust-radio-audio/src/playback.rs` adapted from `pskeffington/pip_boy/crates/pipboy/src/radio_playback.rs` blob `859a5adf7d9e98d37ac8ce8e1fc674c5257aab0a`.
- `crates/rust-radio-audio/src/output.rs` copied from `pskeffington/pip_boy/crates/pipboy/src/radio_output.rs` blob `f3916c146e97b3adeb244b171810fbda9a235055`.
- `crates/rust-radio-audio/src/compat_controller.rs` adapted from `pskeffington/pip_boy/crates/pipboy-core/src/radio_controller.rs` blob `beaaee62ac9e1c621e17f52f89f41939cc117035`.

## Migration design

`rust-radio-audio` is an opt-in crate. The default `rust-radio-core` remains free of CPAL, Symphonia, Reqwest and hardware requirements. Source adaptations are limited to module/crate imports and native worker naming; current behavior is intentionally retained.

The compatibility controller is explicitly transitional: `rust-radio-core::RadioController` has a different event/command API and should **not** replace the original until event-state equivalence and failure/retry tests prove parity. For now, `NativeRadioEngine` continues to use `compat_controller`.

`NativeRadioEngine::start_internet(station_id, volume_percent, output_id)` is the explicit action opening a selected audio output and station worker. `NativeRadioEngine::stop()` requests cancellation and joins its playback worker. Stream receiver cancellation/join behavior still requires independent fault injection. Output selection is OS-managed, including already-paired Bluetooth. No microphone or RF transmission implementation.

## Verification requirements

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p rust-radio-audio` plus macOS CoreAudio and Linux ALSA device tests
- `cargo test -p rust-radio-ip` for bounded HTTP/ICY worker fixtures
- Build for a Raspberry Pi ARM64 target with documented ALSA packages and speaker device availability
- Operator-initiated Skylab 30-second playback; verify audio callback consumed PCM before marking Playing
- Stop during connect, empty playlist, stream stall, stream disconnect, device loss, and Bluetooth output swap; inspect worker and socket cleanup
- License review for station playback and distribution before packaging with a game or publicly redistributing
- Confirm minimum supported Rust version against transitive dependencies and commit Cargo.lock

## CI status

GitHub source changes and Linux ALSA header installation are committed. No claim is made that CI has passed, that the extracted player has actually produced sound, or that Rust-Boy is using this crate yet. Only after passing CI and Mac field parity should the Pip-Boy host switch to a path dependency.
