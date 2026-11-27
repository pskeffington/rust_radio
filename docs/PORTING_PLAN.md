# Radio module roadmap and ownership

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
| IP playback | Native MP3/AAC, WCRB/Skylab presets, explicit public HTTPS and private IPv4 HTTP direct streams, checked redirects and bounded curated/custom M3U/PLS and ICY, cancellation, native callback checks and serialized GUI station switching | Live DNS/TLS cancellation, long-running network and LAN hardware tests; HLS remains unsupported; playlist playback uses the first entry |
| Audio outputs | System/default and selected outputs, local volume, output-loss classification, asynchronous Stop | Physical Bluetooth and USB loss/reconnect; Windows output |
| Compact GUI | Optional 480×320 egui panel with phosphor skin, availability-driven IP/nSDR/USB setup buttons, passive input discovery, direct nRSP-ST and USB controls, asynchronous station checks, saved public/LAN streams, keyboard menus and layout/lifecycle coverage | Physical touch and Pi rendering; fresh real-receiver GUI audio |
| nRSP-ST | Direct native handshake, activation, live frequency/mode/bandwidth and FM stereo/de-emphasis/filter controls, real stereo-lock reports, presets, Opus playback and owned session Stop; optional SDRconnect adapter | Native authenticated login, extended link interruption/reconnect, other demodulator/antenna field checks, physical Pi client and raw I/Q |
| USB SDR | Optional SoapySDR discovery, bounded RawIq capture and initial 240 ksample/s mono-FM/native audio path; synthetic DSP and callback evidence | Supported physical receiver/driver, audible RF, unplug/replug and GUI hardware validation; stereo/AM/SSB remain unimplemented |
| Raspberry Pi | Headless SDRconnect AArch64 Linux compile check, also configured in CI | Physical audio/HUD, CPU/memory, thermal/power and touch acceptance |
| Remote CI | Linux core/native/receiver checks and ARM64 compile check configured | GitHub runs currently fail at startup before any jobs are created |

## November 2026 execution order

The [scoped task queue](CODEX_TASK_QUEUE.md) and [agent doctrine](../AGENTS.md) govern subsequent Codex passes. Start with RR-101 (Skylab root-cause reproduction), RR-102 (fresh silent catalog evidence), and RR-108 (exact-head CI review). Do not call historical 25/25 decoded streams a current live result. Mac device/output tests, physical Pi 5 and SDR hardware acceptance remain distinct blocked or operator-run gates. See [status checkpoint](STATUS_2026-11-05.md).

## Next development priorities

1. Keep recovery source-aware: terminal receiver setup failures require explicit
   retry; transient transport failures retain bounded recovery. Add regression
   evidence for demonstrated faults before expanding features.
2. Exercise output loss/recovery and the standalone micro-GUI on actual hardware,
   preserving the confirmed Mac speaker and Pip-Boy playback paths.
3. Run the native module and Pip-Boy on the physical Pi with its intended sound
   output and display. Cross-compilation alone does not complete this gate.
4. Develop a validated direct nRSP-ST audio/control path without depending on
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

## RR-110 standalone navigation (2026-11-26 candidate)

The standalone constructor opts into an IP-first layout; receiver selection moves behind Settings. Embedded construction keeps its existing presentation and discovery behavior. Receiver APIs and feature flags are unchanged. Source selection is blocked while another source owns active work. Compilation, keyboard/pointer verification, physical playback, and review are pending. External provider policy and the SiriusXM launch action remain separate gates RR-111/RR-112. Do not conflate this narrow change with draft PR #14's broader SDR removal.

## RR-111 external-provider design (2026-11-26)

See [credential-isolated external provider contract](EXTERNAL_PROVIDER_CONTRACT.md). Proposed narrow standalone fixed-URL browser handoff, no in-engine browser audio or credential path. RR-112 implementation follows design review; RR-113/114 remain blocked until permitted, unambiguous session controls are established. RR-110 and all runtime, independent-review and release gates remain open.

## RR-112 SiriusXM browser handoff (2026-11-27 candidate)

The standalone header now has a SiriusXM action that emits a closed, one-shot provider intent. Only the standalone executable dispatches the fixed official HTTPS URL through the OS handler (`xdg-open` on Linux, `open` on macOS); other platforms remain disabled. The app polls handler completion without blocking the UI and distinguishes dispatch failure from success. A successful handler exit means only that the URL was handed to the OS, not that login or playback succeeded. During the handoff, native Play and volume controls stay unavailable until the user confirms browser playback has stopped. This is an operator assertion, not session isolation. Embedded defaults and the native engine remain unchanged. Local Rust gates pass; hosted CI, pointer/keyboard review, target screenshot, and subscriber-driven playback remain open. See the contract acceptance matrix and dated progress entry.

## SiriusXM channel browsing and authenticated playback (2026-11-27)

Follow the gated SX-0–SX-4 roadmap in [the external provider contract](EXTERNAL_PROVIDER_CONTRACT.md#siriusxm-channel-browsing-roadmap--proposed-gates). The current RR-112 branch is a browser handoff only. Official-source review found a consumer channel guide but did not find a public developer API/OAuth registration path, playback SDK/API, channel deep-link contract, or listing-reuse permission. SX-1 catalog ingestion and SX-2 channel-specific handoff are BLOCKED pending provider documentation and written permission; SX-3 account context and SX-4 native playback are separately gated, with SX-4 outside the current approved scope. Authenticated subscriber status alone does not authorize app playback, data extraction or redistribution. Do not scrape the player, inspect private endpoints, access browser credentials, capture media, or route browser playback into the native engine. Resume implementation when provider supplies an authorized interface and rights; otherwise continue only with generic fixture-backed browsing clearly marked as fixtures, or the external-player handoff.
