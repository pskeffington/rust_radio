# Radio module roadmap and ownership

Implementation roadmap updated 2026-10-28; documentation reconciliation 2026-11-06. The standalone library is integrated into
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
| IP playback and station catalog | Native MP3/AAC, twenty-five publisher-documented third-party-player presets, explicit public HTTPS and private IPv4 HTTP direct streams, checked redirects and bounded curated/custom M3U/PLS and ICY, cancellation, native callback checks and serialized GUI station switching | Maintain diversity, publisher guidance and endpoint ownership across the 25-entry catalog; national NPR continuous-feed rights remain unestablished; live DNS/TLS cancellation, long-running network and LAN hardware tests; HLS remains unsupported |
| Audio outputs | System/default and selected outputs, local volume, output-loss classification, asynchronous Stop | Physical Bluetooth and USB loss/reconnect; Windows output |
| Compact GUI | Optional 480×320 egui panel with phosphor skin, availability-driven IP/nSDR/USB setup buttons, passive input discovery, direct nRSP-ST and USB controls, asynchronous station checks, saved public/LAN streams, keyboard menus and layout/lifecycle coverage | Physical touch and Pi rendering; fresh real-receiver GUI audio |
| nRSP-ST | Direct native handshake, activation, live frequency/mode/bandwidth and FM stereo/de-emphasis/filter controls, real stereo-lock reports, presets, Opus playback and owned session Stop; optional SDRconnect adapter | Native authenticated login, extended link interruption/reconnect, other demodulator/antenna field checks, physical Pi client and raw I/Q |
| USB SDR | Optional SoapySDR discovery, bounded RawIq capture and initial 240 ksample/s mono-FM/native audio path; synthetic DSP and callback evidence | Supported physical receiver/driver, audible RF, unplug/replug and GUI hardware validation; stereo/AM/SSB remain unimplemented |
| Raspberry Pi | Headless SDRconnect AArch64 Linux compile check, also configured in CI | Physical audio/HUD, CPU/memory, thermal/power and touch acceptance |
| Remote CI | Linux core/native/receiver checks, ARM64 headless receiver checks, and Windows MSVC workspace/receiver compile, strict Clippy and test suites configured | GitHub runs currently fail at startup before any jobs are created; Windows MSVC workflow execution remains unverified |

## November 2026 execution order

RR-117 closes the independent review finding that future-dated reports or saved history could freeze the outage-history merge order. It rejects timestamps later than one wall-clock sample captured after the history lock, including legacy `updated_at_unix`, without changing the schema or persistence protocol. Local default/all-feature tests and strict Clippy pass; hosted Actions, Windows lock runtime and hardware acceptance remain separate gates. See the RR-117 evidence in [progress](PROGRESS.md).

The [scoped task queue](CODEX_TASK_QUEUE.md) and [agent doctrine](../AGENTS.md) govern subsequent Codex passes. RR-101 has opt-in redacted CLI diagnostics and a terminal playback failure report; provenance and independent validation are recorded in [progress](PROGRESS.md). RR-110 hardens the public ICY reader against invalid metadata intervals while preserving its existing constructor API. RR-111 preserves the prior validated catalog-history file in an atomically written `.bak` before replacing the primary; this supports recovery from an incomplete update but does not claim interprocess serialization or power-loss durability. RR-112 keeps the existing host-facing `FeedHealth` API stable, reads schema v1 and v2, writes v2, and exposes additive 6-hour Fresh/24-hour Degraded last-good health to the CLI and GUI; saved status remains advisory and never gates playback. RR-113 adds CLI-local completed-sweep identities, three-sweep station confirmation, and whole-host/shared-publisher outage guards in history/report metadata; it preserves schema v2, last-good proof, and playback independence. RR-114 adds CLI `--eligible` and `--strict-eligible` report fields from validated persisted history, plus `--once` failure for a completed current host/publisher-wide outage; missing history is explicitly unverified, and the lists never alter catalog membership or gate playback. RR-115 sets the bounded catalog validation retry delay to 1.5 seconds after a transient `Unavailable` result; decoded and terminal results do not retry. RR-116 serializes same-path catalog-history updates behind a persistent sibling lock, with a bounded five-second wait, and prunes per-station failure references outside the retained 64-sweep window. Local default/all-feature gates and Windows GNU compile/link checks pass; Windows runtime locking remains unverified. Exact-head Actions runs 412–415 ended `startup_failure` before job creation with no jobs or combined statuses. RR-108 remains blocked without actionable logs. Do not call historical 25/25 decoded streams a current live result. Mac device/output tests, physical Pi 5 and SDR hardware acceptance remain distinct blocked or operator-run gates. See [status checkpoint](STATUS_2026-11-05.md).

## Next development priorities

1. Maintain the 25-entry safe catalog with diverse broadcasters, publisher
   guidance and exact direct endpoints. SomaFM remains excluded pending written
   permission. NPR member stations provide local NPR programming; no national
   continuous NPR feed has been established. Temporary network outages must
   not remove authorized stations.
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
6. Defer satellite adapters and additional DSP until the playback and platform
   gates above are stable. Publisher distribution review remains required before
   public station bundles.

See [embedding](EMBEDDING.md), [native playback evidence](NATIVE_PLAYBACK_CONTRACT.md),
[nRSP-ST](NRSP_ST.md) and [USB SDR boundaries](SDR_RECEIVE.md) for implementation
contracts and reproducible commands.

## Compatibility API
Hosts should drive commands (select/play/stop/volume/output) and observe session snapshots; keep network receivers, native device discovery, publishers, and GUI outside the core. Provide versioned API and avoid cyclic dependency from rust_radio back to pip_boy.
