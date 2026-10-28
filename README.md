# rust_radio

Portable, **receive-only**, Rust radio subsystem designed to plug into [Rust-Boy / pip_boy](https://github.com/pskeffington/pip_boy) without coupling its engine to the HUD. IP playback runs independently. Optional SoapySDR supports raw I/Q and a mono-FM audio path; direct nRSP-ST service checks are also available. Direct nRSP-ST control and Opus audio are available through `nrsp-native`; USB FM awaits physical hardware acceptance.

## Status (2026-10-28)

The standalone module provides bounded internet and private-LAN streaming, MP3/AAC decoding, native audio output, a CLI and an optional 480×320 egui panel. Pip-Boy embeds the headless library through a pinned Git dependency and owns its HUD and preferences.

IP radio and nRSP-ST PCM have reached native output callbacks in standalone and Pip-Boy runs. Direct native nRSP-ST control and Opus playback have also reached Mac native callbacks without using SDRconnect: live frequency/mode changes and session cleanup passed. The operator confirmed audible radio on MacBook Air speakers through the system default output. Explicit SDRconnect Play can start the selected receiver when control is allowed; missing startup confirmation fails after three seconds and requires explicit retry. The headless SDRconnect module cross-compiles for AArch64 Linux. Physical Pi runtime, Bluetooth recovery, raw I/Q and standalone GUI field acceptance remain open.

## Navigation

- [Standalone module and embedding API](docs/EMBEDDING.md)
- [Optional SDR receive adapter and hardware gates](docs/SDR_RECEIVE.md)
- [SDRplay nRSP-ST native and SDRconnect access](docs/NRSP_ST.md)
- [Direct nRSP-ST service status](docs/NRSP_ST.md#direct-device-status)
- [Literature and architecture matrix](docs/LITERATURE_MATRIX.md)
- [Roadmap and component ownership](docs/PORTING_PLAN.md)
- [Progress notes and verification evidence](docs/PROGRESS.md)
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
2. Resource access requires an explicit host command. Construction, browsing and preference reads do not autoplay; validation never opens audio output or activates a receiver.
3. One active session; generation-tagged state and real worker cancellation.
4. `Playing` requires samples consumed by the native output callback; audible listening is verified separately.
5. Bounded streaming memory, publisher endpoint validation for presets, explicit direct URLs, controlled redirects and timeouts.
6. Keep audio output devices, Bluetooth pairing and touch UI under OS/host ownership.
7. No RF transmit path. Satellite reception is research-stage only.
8. Any curated broadcaster inclusion requires a redistribution/embedding rights review before released bundles.

## Near-term work

Follow the [roadmap](docs/PORTING_PLAN.md) for platform and receiver acceptance. The next field checks cover output loss/recovery, standalone GUI controls and the actual Pi audio/HUD runtime. Raw I/Q needs the SDRconnect Full IQ firmware prerequisite resolved before it can be accepted. The separate SoapySDR USB adapter still needs a supported device and driver.

## Run the native radio

```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
# Direct nRSP-ST controls and audio:
cargo run -p rust-radio-gui --features nrsp-native --bin rust-radio-gui
# Optional nRSP-ST source through SDRconnect:
cargo run -p rust-radio-gui --features sdrconnect --bin rust-radio-gui
# Optional USB SDR mono-FM source through SoapySDR:
cargo run -p rust-radio-gui --features soapy-sdr --bin rust-radio-gui
```

With `nrsp-native`, click **nSDR** for the receiver address (`IP:9001`), frequency, mode and bandwidth. PLAY starts the session; APPLY TUNING changes the active receiver; STOP releases it. See [direct receiver setup and verification](docs/NRSP_ST.md#direct-native-control-and-audio).

The top-left **IP Radio | nSDR | USB** buttons are input selectors and setup entry points. Green glow means the input is available; dim means unavailable or not checked. The underline marks the selected input. For example, with no internet and only a supported USB receiver attached, only USB glows. More than one input can glow when several are available.

The standalone GUI scans on startup and every 30 seconds while idle. It checks HTTPS reachability, identifies native nRSP-ST receivers on local private IPv4 neighborhoods, and enumerates supported SoapySDR USB drivers. It selects detected hardware controls without starting reception; PLAY starts the radio. Click an input button for its output and source settings, detected-device selection, stream URL or tuning controls. SCAN INPUTS refreshes availability. Switching input types requires stopping playback first. Settings and runtime addresses stay in memory.

```sh
# Network and USB receiver discovery and controls (SoapySDR SDK required):
cargo run -p rust-radio-gui --features receivers --bin rust-radio-gui
```

IP Radio contains curated station selection, silent station checks and direct MP3/AAC stream URLs. Select **WCRB Classical (Boston)** or **Skylab Radio** and press PLAY. A station website may differ from its audio URL; GBH publishes the [WCRB direct stream](https://www.wgbh.org/help/how-to-access-live-radio-streams). Direct input accepts HTTPS and literal private/loopback IPv4 HTTP, rejects credentials and fragments, and does not follow redirects, playlists or HLS. Protect query tokens, including from shell history when using the CLI.

nSDR contains the discovered receiver address, frequency, mode, bandwidth and live APPLY TUNING. USB contains the detected device picker and mono-FM frequency. USB playback requires a compatible driver supporting 240 ksample/s CS16; physical USB acceptance is still open. Builds without an optional receiver feature show its button dim and explain the missing support in setup. The headless and IP-only defaults remain independent of native SDR SDKs.

```sh
cargo run -p rust-radio-cli --features nrsp-native,soapy-sdr -- inputs
cargo run -p rust-radio-cli -- listen wcrb --seconds 30
cargo run -p rust-radio-cli -- stream-listen 'https://example.org/live.mp3' --seconds 30
```

The native transport resolves bounded M3U and PLS playlists using publisher-approved URLs. Legacy HTTP entries inside SomaFM playlists are requested over HTTPS after the same host/port/path checks; there is no plaintext fallback. HLS is detected and reported as unsupported; segmented playback needs a separate transport. Temporary connection and stream failures share a maximum of three retries with 1/2/4-second delays.

Check a station without opening an audio device:

```sh
cargo run -p rust-radio-cli -- validate skylab
```

Validation records its timestamp, resolved endpoint, furthest verified stage and MP3/AAC decode evidence. It shares one 18-second HTTP deadline across all playlist/redirect hops, reads at most 256 KiB of audio and examines at most 64 packets. Only decoded finite PCM produces a successful exit status; HTTP success and catalog inclusion alone do not. Checks are explicit and are not part of offline CI. CLI reports last for that invocation; GUI reports are retained per station for the panel's lifetime. Persistent validation history remains future work. Check cancellation prevents subsequent reads/hops and discards late results; an already-blocked request may wait for the remaining HTTP deadline while the GUI keeps repainting.
