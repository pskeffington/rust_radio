# rust_radio

Portable, **receive-only** internet radio designed to plug into [Rust-Boy / pip_boy](https://github.com/pskeffington/pip_boy) without coupling its engine to the HUD. It supports curated and custom IP audio streams, MP3/AAC decoding, native audio output, a CLI and an optional 480×320 egui panel. USB and nSDR reception are maintained in separate projects.

## Status (2026-11-23)

The standalone module provides bounded internet and private-LAN streaming. Construction is offline and playback starts only after an explicit operator action. Physical audio, platform and publisher-rights acceptance remain separate gates; see the current [scope note](docs/PROGRESS.md).

## Navigation

- [Standalone module and embedding API](docs/EMBEDDING.md)
- [Literature and architecture matrix](docs/LITERATURE_MATRIX.md)
- [Roadmap and component ownership](docs/PORTING_PLAN.md)
- [Progress notes and verification evidence](docs/PROGRESS.md)
- [November 2026 status checkpoint](docs/STATUS_2026-11-05.md)
- [Codex task queue](docs/CODEX_TASK_QUEUE.md)
- [Development agent instructions](AGENTS.md)
- [Core Rust interface](crates/rust-radio-core/src/lib.rs)
- [Native GUI and embedding](docs/GUI_INTEGRATION.md)
- [Standalone acceptance](docs/STANDALONE_ACCEPTANCE.md)
- [Existing Rust-Boy radio engineering roadmap](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_NATIVE_ROADMAP.md)

## Develop

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For control-only testing, use `cargo test -p rust-radio-core`; it requires no network access, sound card or native audio libraries. Workspace builds include CPAL and eframe and need platform development libraries (on Linux, `libasound2-dev` and `pkg-config`). Remote CI and physical hardware acceptance are separate checks.

Cargo build output (`target/`) and macOS metadata (`.DS_Store`) are ignored so Git-aware source searches and indexes skip generated files.

## Required integration invariants

1. No external media-app launching, web browser audio or autoplay.
2. Resource access requires an explicit host command. Construction, browsing and preference reads do not autoplay; validation never opens audio output.
3. One active session; generation-tagged state and real worker cancellation.
4. `Playing` requires samples consumed by the native output callback; audible listening is verified separately.
5. Bounded streaming memory, publisher endpoint validation for presets, explicit direct URLs, controlled redirects and timeouts.
6. Keep audio output devices, Bluetooth pairing and touch UI under OS/host ownership.
7. No RF transmit or receive path is part of this project.
8. Any curated broadcaster inclusion requires a redistribution/embedding rights review before released bundles.

## Near-term work

Follow the [roadmap](docs/PORTING_PLAN.md) for streaming-radio platform acceptance. Next field checks cover output loss/recovery, standalone GUI controls and the actual Pi audio/HUD runtime.

## Run the native radio

```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
```

The listening screen gives the station name priority, with a short genre line,
one Play/Stop/Retry control, the current listening state, a wide volume slider
and Mute (which restores the previous volume). Live playback never shows an
unverified badge. Audio-output names replace decoder diagnostics; connection
checks and technical details stay in setup. Errors remain beside Retry in a
bounded, readable area. The phosphor theme uses softer scanlines and quieter
secondary text.

IP Radio contains curated and saved stations, silent endpoint checks and custom MP3/AAC stream URLs. Station selection while stopped checks the actual stream asynchronously; selection during playback cancels and mutes the old session, waits for cleanup, then starts only the latest selection. PLAY reports Connecting → Buffering → Playing only after decoded samples reach native output. Failures show a readable message and RETRY. The dropdown supports scrolling, long-name tooltips and keyboard navigation (Tab/Enter, arrows, Home/End, Escape). Connection/output settings stay behind **IP Radio**. Save up to 24 named public or LAN stations there; Save stores the URL, including query parameters. Select **WCRB Classical (Boston)** or **Skylab Radio** and press PLAY. A station website may differ from its audio URL; GBH publishes the [WCRB direct stream](https://www.wgbh.org/help/how-to-access-live-radio-streams). Direct input accepts HTTPS and literal private/loopback IPv4 HTTP, rejects non-public HTTPS literal-IP targets, credentials and fragments, and follows up to five checked request hops through redirects and M3U/PLS playlists. Relative locations and entries are supported; HTTPS cannot downgrade to HTTP. HLS remains unsupported. Protect query tokens, including from shell history when using the CLI.

Curated stations and public HTTPS inputs resolve through an address filter that rejects private, loopback, link-local, reserved and documentation ranges, then returns the checked addresses to the HTTP client so a second DNS lookup cannot rebind the target. A user-entered private IPv4 HTTP stream may use that exact private origin by explicit choice; redirects and playlist entries to other private addresses and private DNS answers remain blocked.

```sh
cargo run -p rust-radio-cli -- listen wcrb --seconds 30
cargo run -p rust-radio-cli -- stream-listen 'https://example.org/live.mp3' --seconds 30
```

Curated transport resolves bounded M3U and PLS playlists using publisher-approved URLs. Explicit custom streams use the same bounded parser, with HTTPS or private/loopback IPv4 HTTP validation at every hop. Legacy HTTP entries inside SomaFM playlists are requested over HTTPS after the same host/port/path checks; there is no plaintext fallback. HLS is detected and reported as unsupported; segmented playback needs a separate transport. Temporary connection and stream failures share a maximum of three retries with 1/2/4-second delays.

Check a station without opening an audio device:

```sh
cargo run -p rust-radio-cli -- validate skylab
```

Validation records its timestamp, resolved endpoint, furthest verified stage and MP3/AAC decode evidence. It shares one 18-second HTTP deadline across all playlist/redirect hops, reads at most 256 KiB of audio and examines at most 64 packets. Only decoded finite PCM produces a successful exit status; HTTP success and catalog inclusion alone do not. CLI checks are explicit; the GUI also checks operator station selections silently. Public network checks are separate from fixture CI. The CLI uses its 18-second probe path. GUI selection checks use the cancellable playback transport with a 35-second startup budget, 8-second header timeout, 3-second read timeout, 256 KiB decode limit and no automatic retry. CLI reports last for that invocation; GUI reports are retained per station for the panel's lifetime. `validate-all --json --history PATH` optionally persists bounded local evidence, and egui hosts can import it through `RadioPanel::load_validation_history`; runtime checks and playback remain authoritative. Check cancellation prevents subsequent reads/hops and discards late results; an already-blocked request may wait for the remaining HTTP deadline while the GUI keeps repainting.
