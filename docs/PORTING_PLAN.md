# Import plan: pip_boy -> rust_radio -> pip_boy

**Baseline**: https://github.com/pskeffington/pip_boy (capture exact main SHA at import). Existing live Mac Skylab playback, transport/volume control, favorites and OS-managed Bluetooth routing must not regress.

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

## Migration gates
- P0: Freeze baseline commit, record lockfile dependencies and exact radio tests, verify source licenses.
- P1: Extract pure controller, PCM, directory and parser, retaining compatibility tests and data schema.
- P2: Lift native network and playback workers behind explicit features; compare live Mac behavior to baseline.
- P3: Add local path dependency in pip_boy then switch to a Git tagged dependency after CI.
- P4: Feature-gated physical SDR backend + DSP, receive-only; no simulator portrayed as RF.
- P5: Satellite receiver adapter design, protocol-specific rights review and test hardware.
- P6: Pi ARM64 CPU/memory/battery and 480x320 UI acceptance; Windows output test.
  CI cross-checks the headless SDRconnect module for AArch64 Linux, but physical
  Pi runtime and UI acceptance remain separate requirements.

**Current state (2026-10-27)**: the standalone `rust-radio` facade owns the shared controller, directory, parsing, PCM, recovery, IP transport, native playback/output and optional receiver adapter. Pip-Boy main consumes a pinned Git revision through thin re-exports; host preferences and HTML UI remain host-owned. The compatibility controller now lives in core. The source-neutral `SessionController` is also available for future backend coordination.

Host integration passed 328 tests, strict Clippy and build locally; browser fixtures and a live Skylab/Groove Salad native PCM callback run also passed. SoapySDR provides optional bounded I/Q ingress, with no installed device found here. RF DSP/audio, physical SDR/ARM64, Bluetooth recovery and native GUI visual acceptance remain open. Remote CI success has not been established. See [embedding](EMBEDDING.md), [native playback evidence](NATIVE_PLAYBACK_CONTRACT.md) and [SDR boundaries](SDR_RECEIVE.md).

## Compatibility API
Hosts should drive commands (select/play/stop/volume/output) and observe session snapshots; keep network receivers, native device discovery, publishers, and GUI outside the core. Provide versioned API and avoid cyclic dependency from rust_radio back to pip_boy.
