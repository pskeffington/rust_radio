# Import plan: pip_boy -> rust_radio -> pip_boy

**Baseline**: https://github.com/pskeffington/pip_boy (capture exact main SHA at import). Existing live Mac Skylab playback, transport/volume control, favorites and OS-managed Bluetooth routing must not regress.

## Ownership map

| Current pip_boy path | New ownership | Notes |
| --- | --- | --- |
| crates/pipboy-core/src/radio_controller.rs | rust-radio-core | Preserve status and generation semantics |
| crates/pipboy-core/src/radio_directory.rs | rust-radio-directory (planned) | Publisher references and allowlists |
| crates/pipboy-core/src/radio_stream.rs | rust-radio-ip (planned) | ICY/M3U/redirect parsing |
| crates/pipboy-core/src/radio_pcm.rs | rust-radio-audio (planned) | PCM conversion |
| crates/pipboy-core/src/radio_presets.rs | rust-radio-presets (planned) | Versioned and atomic storage |
| crates/pipboy-core/src/radio_recovery.rs | rust-radio-core (planned) | Deterministic reconnection policy |
| crates/pipboy/src/radio_net.rs, radio_http.rs | rust-radio-ip (planned) | Async streaming vs short probe separation |
| crates/pipboy/src/radio_playback.rs, radio_output.rs | rust-radio-audio (planned) | Native Symphonia and CPAL |
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

**Current state**: rust-radio-core is only an initial pure-control scaffold. Actual player and SDR implementations remain in pip_boy. Do not claim code migration or live functionality until ports and tests pass.

## Compatibility API
Hosts should drive commands (select/play/stop/volume/output) and observe session snapshots; keep network receivers, native device discovery, publishers, and GUI outside the core. Provide versioned API and avoid cyclic dependency from rust_radio back to pip_boy.
