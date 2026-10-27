# rust_radio

Portable, **receive-only**, Rust radio subsystem designed to plug into [Rust-Boy / pip_boy](https://github.com/pskeffington/pip_boy) without coupling its engine to the HUD. Intended backends: curated HTTPS internet streams, optional USB SDR receivers and future satellite receive adapters.

## Status (2026-10-27)

This workspace contains portable control/parsing code, bounded HTTPS transport, native Symphonia/CPAL playback, a CLI and an embeddable 480×320 egui panel. Pip-Boy now embeds the headless `rust-radio` module through a pinned Git dependency. Its HUD runtime checks exercised Skylab and Groove Salad through native PCM output, explicit Stop, volume and saved selection. The selected nRSP-ST uses the optional SDRconnect API, including native PCM playback through the same engine as IP radio. SoapySDR remains a separate USB path. Physical native RF listening, raw I/Q and Pi acceptance remain open.

## Navigation

- [Standalone module and embedding API](docs/EMBEDDING.md)
- [Optional SDR receive adapter and hardware gates](docs/SDR_RECEIVE.md)
- [SDRplay nRSP-ST network receiver via SDRconnect](docs/NRSP_ST.md)
- [Literature and architecture matrix](docs/LITERATURE_MATRIX.md)
- [Component ownership and extraction gates](docs/PORTING_PLAN.md)
- [Core Rust interface](crates/rust-radio-core/src/lib.rs)
- [Native GUI and embedding](docs/GUI_INTEGRATION.md)
- [Standalone acceptance](docs/STANDALONE_ACCEPTANCE.md)
- [Existing Rust-Boy radio engineering roadmap](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_NATIVE_ROADMAP.md)
- [Existing Rust-Boy SDR contract](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_SDR.md)

## Develop

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For control-only testing, use `cargo test -p rust-radio-core`; it requires no network receiver, sound card, USB SDR or native audio libraries. Workspace builds include CPAL and eframe and need platform development libraries (on Linux, `libasound2-dev` and `pkg-config`). The local verification record is linked from the embedding documentation; remote CI and physical hardware acceptance are separate checks.

Cargo build output (`target/`) and macOS metadata (`.DS_Store`) are ignored so Git-aware source searches and indexes skip generated files.

## Required integration invariants

1. No external media-app launching, web browser audio or autoplay.
2. Only explicit Play or Validate opens network resources; validation never opens audio output or hardware receivers.
3. One active session; generation-tagged state and real worker cancellation.
4. Audible `Playing` requires proof of samples reaching audio output; successful HTTP/SDR tuning alone is insufficient.
5. Bounded streaming memory, TLS and publisher endpoint validation, controlled redirects and timeouts.
6. Keep audio output devices, Bluetooth pairing and touch UI under OS/host ownership.
7. No RF transmit path. Satellite reception is research-stage only.
8. Any curated broadcaster inclusion requires a redistribution/embedding rights review before released bundles.

## Near-term work

Verify live nRSP-ST native audio once its SDRconnect receiver can start; complete raw I/Q and firmware acceptance separately. USB receivers use the separate SoapySDR adapter. Complete physical Pi/ARM64 and Bluetooth recovery acceptance. Pip-Boy already consumes the standalone module through thin adapters; the default library is headless and the standalone GUI is optional.

## Run the native radio

```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
```

Playback starts only after PLAY. STOP requests cancellation and mutes queued audio immediately; the panel remains responsive while showing STOPPING. New playback and output changes stay locked until the previous playback worker releases its resources. SETTINGS contains output selection and CHECK STATION, which decodes silently on a background worker. Station badges distinguish unverified, decoded, unavailable, unsupported and invalid streams. Stop, Cancel Check or browsing another station cancels a check; PLAY and another check remain locked until its worker finishes. Closing/dropping the panel still performs synchronous final cleanup; embedded hosts can call `stop()` and poll `poll_shutdown()` before dismounting to avoid waiting during teardown.

The native transport resolves bounded M3U and PLS playlists using publisher-approved URLs. Legacy HTTP entries inside SomaFM playlists are requested over HTTPS after the same host/port/path checks; there is no plaintext fallback. HLS is detected and reported as unsupported; segmented playback needs a separate transport. Temporary connection and stream failures share a maximum of three retries with 1/2/4-second delays.

Check a station without opening an audio device:

```sh
cargo run -p rust-radio-cli -- validate skylab
```

Validation records its timestamp, resolved endpoint, furthest verified stage and MP3/AAC decode evidence. It shares one 18-second HTTP deadline across all playlist/redirect hops, reads at most 256 KiB of audio and examines at most 64 packets. Only decoded finite PCM produces a successful exit status; HTTP success and catalog inclusion alone do not. Checks are explicit and are not part of offline CI. CLI reports last for that invocation; GUI reports are retained per station for the panel's lifetime. Persistent validation history remains future work. Check cancellation prevents subsequent reads/hops and discards late results; an already-blocked request may wait for the remaining HTTP deadline while the GUI keeps repainting.
