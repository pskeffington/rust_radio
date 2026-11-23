# Radio module roadmap and ownership

Implementation roadmap reconciled 2026-11-23. The standalone library is integrated into
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
| IP playback | Native MP3/AAC, WCRB/Skylab presets, explicit public HTTPS and private IPv4 HTTP direct streams, checked redirects and bounded curated/custom M3U/PLS and ICY, cancellation, native callback checks and serialized GUI station switching | Live DNS/TLS cancellation, long-running network and LAN hardware tests; HLS remains unsupported; playlist playback uses the first entry |
| Audio outputs | System/default and selected outputs, local volume, output-loss classification, asynchronous Stop | Physical Bluetooth and USB loss/reconnect; Windows output |
| Compact GUI | Optional 480×320 egui panel with phosphor skin, passive internet reachability check, asynchronous station checks, saved public/LAN streams, keyboard menus and layout/lifecycle coverage | Physical touch and Pi rendering |
| Raspberry Pi | Portable IP streaming library and native audio output | Physical audio/HUD, CPU/memory, thermal/power and touch acceptance |
| Remote CI | Linux workspace formatting, tests and strict clippy configured | Hosted runs must be checked at the tested commit |

## November 2026 execution order

The [scoped task queue](CODEX_TASK_QUEUE.md) and [agent doctrine](../AGENTS.md) govern subsequent Codex passes. Do not call historical decoded streams a current live result. Mac device/output tests and physical Pi 5 acceptance remain distinct blocked or operator-run gates. See [status checkpoint](STATUS_2026-11-05.md).

RR-120 commit `36e603152e1d770014e3487c056c0cd7664acd5f` removes SDR/nSDR/satellite support from this repository's code, build features, CLI/GUI, CI and current-facing guides. IP streaming, generic audio device selection and saved IP stations remain. Local locked workspace, all-feature and no-default checks pass; hosted CI and physical audio/Pi acceptance remain separate gates. Historical receiver evidence stays in dated progress entries.

## Next development priorities

1. Keep recovery source-aware: terminal stream setup failures require explicit
   retry; transient transport failures retain bounded recovery. Add regression
   evidence for demonstrated faults before expanding features.
2. Exercise output loss/recovery and the standalone micro-GUI on actual hardware,
   preserving the confirmed Mac speaker and Pip-Boy playback paths.
3. Run the native module and Pip-Boy on the physical Pi with its intended sound
   output and display. Cross-compilation alone does not complete this gate.
4. Keep the scope focused on IP station discovery, transport, decoding, output
   recovery and host integration. RF receiver development is maintained in
   separate projects. Publisher distribution review remains required before
   public station bundles.

See [embedding](EMBEDDING.md) and [native playback evidence](NATIVE_PLAYBACK_CONTRACT.md)
for implementation contracts and reproducible commands.

## Compatibility API
Hosts should drive commands (select/play/stop/volume/output) and observe session snapshots; keep network transport, publishers, and GUI outside the core. Provide versioned API and avoid cyclic dependency from rust_radio back to pip_boy.
