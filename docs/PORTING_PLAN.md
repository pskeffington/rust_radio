# Radio module roadmap and ownership

## Current station and provider decision (2026-11-27)

## Product scope update (2026-11-27)

Rust Radio is the internet streaming app only. nSDR/network receivers, USB SDR, SoapySDR, SDRconnect, satellite reception, and their discovery/tuning workflows have moved to their own repositories. They are not delivery targets or roadmap gates for this repository. Remove their selectors, commands, dependencies, and app integration as the streaming-only cleanup proceeds; keep any historical evidence clearly labeled as history. Native audio output device selection remains in scope because it serves internet streams.

The SiriusXM browser-launch draft is closed unmerged because no supported development API was identified; `origin/main` never carried that draft into production. The active starter catalog is limited to GBH 89.7 (`gbh897`) and WCRB Classical (`wcrb`). Both need fresh silent stream validation, Mac operator playback and written publisher permission for intended product distribution. SomaFM was removed because its current terms bar new third-party clients without prior written permission. Skylab remains excluded pending affirmative app-client permission. Production channel distribution is gated on publisher rights.

Implementation roadmap originally updated 2026-10-27; documentation reconciliation 2026-11-05. The standalone library is integrated into
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
| IP playback | Native MP3/AAC, GBH 89.7/WCRB presets, explicit public HTTPS and private IPv4 HTTP direct streams, checked redirects and bounded curated/custom M3U/PLS and ICY, cancellation, native callback checks and serialized GUI station switching | Live DNS/TLS cancellation, long-running network and LAN hardware tests; HLS remains unsupported; playlist playback uses the first entry |
| Audio outputs | System/default and selected outputs, local volume, output-loss classification, asynchronous Stop | Physical Bluetooth and USB loss/reconnect; Windows output |
| Compact GUI | Optional 480×320 egui streaming panel, curated and custom IP stations, asynchronous station checks, saved streams, keyboard menus and native output controls | Physical touch and platform rendering; operator playback acceptance |
| Receiver products | nSDR, SDRconnect, USB SDR, SoapySDR, and satellite workflows belong to separate repositories | No implementation, release, or acceptance scope in Rust Radio |
| Raspberry Pi | Headless SDRconnect AArch64 Linux compile check, also configured in CI | Physical audio/HUD, CPU/memory, thermal/power and touch acceptance |
| Remote CI | Linux core/native/receiver checks and ARM64 compile check configured | GitHub runs currently fail at startup before any jobs are created |

## November 2026 execution order

The [scoped task queue](CODEX_TASK_QUEUE.md) and [agent doctrine](../AGENTS.md) govern subsequent Codex passes. Start with RR-109 (written publisher permission), RR-110 (fresh silent validation of both current candidates), and RR-108 (exact-head CI review). Historical feed results do not establish current availability or distribution rights. Mac device/output tests, physical Pi 5 and SDR hardware acceptance remain distinct blocked or operator-run gates. See [status checkpoint](STATUS_2026-11-05.md).

## Next development priorities

1. Complete the streaming-only repository cleanup: remove receiver selectors,
   discovery, CLI routes, feature flags, and build dependencies from the app.
2. Keep the starter catalog small and rights-gated; refresh silent feed validation
   and obtain written permission for product distribution before release.
3. Exercise station browsing, stream playback, Stop/retry, and output selection on
   the rebuilt Mac app. Keep callback evidence distinct from audible acceptance.
4. Reconcile exact-head CI and remaining supported platform checks before release.

See [embedding](EMBEDDING.md) and [native playback evidence](NATIVE_PLAYBACK_CONTRACT.md) for the active streaming contracts. Receiver documents are historical migration records only and do not define this app’s scope.

## Compatibility API
Hosts should drive commands (select/play/stop/volume/output) and observe session snapshots; keep network receivers, native device discovery, publishers, and GUI outside the core. Provide versioned API and avoid cyclic dependency from rust_radio back to pip_boy.
