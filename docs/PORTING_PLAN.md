# Radio module roadmap and ownership

Updated 2026-10-28. The standalone library is integrated into
[Pip-Boy](https://github.com/pskeffington/pip_boy) through a pinned Git dependency.
The next work strengthens playback and completes platform acceptance. Source
provenance remains in the extraction notes; dated evidence is in
[progress notes](PROGRESS.md).

## Ownership map

| Current pip_boy path | New ownership | Notes |
| --- | --- | --- |
| crates/pipboy-core/src/radio_controller.rs | rust-radio-core | Preserve status and generation semantics |
| crates/pipboy-core/src/radio_directory.rs | rust-radio-core | Publisher references and allowlists |
| crates/pipboy-core/src/radio_stream.rs | rust-radio-core | ICY/M3U/redirect parsing |
| crates/pipboy-core/src/radio_pcm.rs | rust-radio-core | PCM conversion |
| crates/pipboy-core/src/radio_presets.rs | host (pip_boy) | Versioned and atomic storage |
| crates/pipboy-core/src/radio_recovery.rs | rust-radio-core | Deterministic reconnection policy |
| crates/pipboy/src/radio_net.rs, radio_http.rs | rust-radio-ip | Async streaming vs short probe separation |
| crates/pipboy/src/radio_playback.rs, radio_output.rs | rust-radio-audio | Native Symphonia and CPAL |
| crates/pipboy/src/radio.rs | host adapter (pip_boy) | CLI commands stay in Rust-Boy |
| crates/pipboy/assets/radio-ui.js, radio-ui.css | host UI (pip_boy) | Micro and desktop view stay in Rust-Boy |

## Delivery and acceptance

| Area | Delivered | Remaining acceptance |
| --- | --- | --- |
| Standalone library and Pip-Boy | Headless facade, shared controller, thin host adapters and pinned dependency; preferences/HUD stay in the host; core exposes bounded typed feed-history parsing | Release/tag decision after CI and platform gates |
| IP playback and station catalog | Native MP3/AAC, nineteen publisher-documented third-party-player presets, explicit public HTTPS and private IPv4 HTTP direct streams, checked redirects and bounded curated/custom M3U/PLS and ICY, cancellation, native callback checks and serialized GUI station switching | Grow the catalog toward ~25 diverse stations with publisher guidance and direct endpoints; national NPR continuous-feed rights remain unestablished; live DNS/TLS cancellation, long-running network and LAN hardware tests; HLS remains unsupported |
| Audio outputs | System/default and selected outputs, local volume, output-loss classification, asynchronous Stop | Physical Bluetooth and USB loss/reconnect; Windows output |
| Compact GUI | Optional 480×320 egui panel with phosphor skin, availability-driven IP/nSDR/USB setup buttons, passive input discovery, direct nRSP-ST and USB controls, asynchronous station checks, saved public/LAN streams, keyboard menus and layout/lifecycle coverage | Physical touch and Pi rendering; fresh real-receiver GUI audio |
| nRSP-ST | Direct native handshake, activation, live frequency/mode/bandwidth and FM stereo/de-emphasis/filter controls, real stereo-lock reports, presets, Opus playback and owned session Stop; optional SDRconnect adapter | Native authenticated login, extended link interruption/reconnect, other demodulator/antenna field checks, physical Pi client and raw I/Q |
| USB SDR | Optional SoapySDR discovery, bounded RawIq capture and initial 240 ksample/s mono-FM/native audio path; synthetic DSP and callback evidence | Supported physical receiver/driver, audible RF, unplug/replug and GUI hardware validation; stereo/AM/SSB remain unimplemented |
| Raspberry Pi | Headless SDRconnect AArch64 Linux compile check, also configured in CI | Physical audio/HUD, CPU/memory, thermal/power and touch acceptance |
| Remote CI | Linux core/native/receiver checks and ARM64 compile check configured | GitHub runs currently fail at startup before any jobs are created |

## Next development priorities

1. Expand the safe station catalog from 19 toward approximately 25 diverse
   stations, with first-party direct endpoints and third-party radio-player
   guidance. SomaFM remains excluded pending written permission. WPR News is a
   named NPR-member news service; no national continuous NPR feed has been
   established. Temporary network outages must not remove authorized stations.
2. Keep recovery source-aware: terminal receiver setup failures require explicit
   retry; transient transport failures retain bounded recovery. Add regression
   evidence for demonstrated faults before expanding features.
3. Exercise output loss/recovery and the standalone micro-GUI on actual hardware,
   preserving the confirmed Mac speaker and Pip-Boy playback paths.
4. Run the native module and Pip-Boy on the physical Pi with its intended sound
   output and display. Cross-compilation alone does not complete this gate.
5. Develop a validated direct nRSP-ST audio/control path without depending on
   SDRconnect. Fides Supra's proven direct work currently covers reachability
   and display frames, not audio or tuning. Resolve the Full IQ firmware
   prerequisite before accepting raw I/Q. Keep USB/Soapy acceptance separate.
5. Defer satellite adapters and additional DSP until the playback and platform
   gates above are stable. Publisher distribution review remains required before
   public station bundles.

See [embedding](EMBEDDING.md), [native playback evidence](NATIVE_PLAYBACK_CONTRACT.md),
[nRSP-ST](NRSP_ST.md) and [USB SDR boundaries](SDR_RECEIVE.md) for implementation
contracts and reproducible commands.

## Compatibility API
Hosts should drive commands (select/play/stop/volume/output) and observe session snapshots; keep network receivers, native device discovery, publishers, and GUI outside the core. Provide versioned API and avoid cyclic dependency from rust_radio back to pip_boy.
