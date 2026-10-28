# Completed native audio extraction

Source extraction began on 2026-10-26. The shared engine is now consumed by
Pip-Boy; see [current progress](PROGRESS.md) and the [roadmap](PORTING_PLAN.md).

## Extracted sources

- `crates/rust-radio-audio/src/playback.rs` adapted from `pskeffington/pip_boy/crates/pipboy/src/radio_playback.rs` blob `859a5adf7d9e98d37ac8ce8e1fc674c5257aab0a`.
- `crates/rust-radio-audio/src/output.rs` copied from `pskeffington/pip_boy/crates/pipboy/src/radio_output.rs` blob `f3916c146e97b3adeb244b171810fbda9a235055`.
- `crates/rust-radio-audio/src/compat_controller.rs` adapted from `pskeffington/pip_boy/crates/pipboy-core/src/radio_controller.rs` blob `beaaee62ac9e1c621e17f52f89f41939cc117035`.

## Migration design

`rust-radio-audio` is enabled by the facade's default `ip-radio` feature.
`rust-radio-core` remains free of CPAL, Symphonia, Reqwest and hardware requirements.

`compat_controller` now re-exports `rust-radio-core::controller`; host and native
worker paths use the same types. The source-neutral `SessionController` remains
available separately through the facade.

`NativeRadioEngine::start_internet(station_id, volume_percent, output_id)` is the explicit action opening a selected audio output and station worker. `NativeRadioEngine::stop()` requests cancellation and joins its playback worker; GUI hosts use `request_stop()` and `try_finish()`. Offline tests cover receiver cleanup and bounded startup/stream retries; live DNS/TLS cancellation still needs field validation. Output selection is OS-managed, including already-paired Bluetooth. No microphone or RF transmission implementation.

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

## Current verification

Pip-Boy's pinned Git dependency, Mac IP/native PCM checks, real nRSP-ST PCM and
operator-confirmed MacBook Air speaker playback are recorded in
[native playback evidence](NATIVE_PLAYBACK_CONTRACT.md) and [nRSP-ST evidence](NRSP_ST.md).
The checklist above retains remaining platform/field requirements. Remote CI
success is still unestablished: observed GitHub runs fail before jobs start.
