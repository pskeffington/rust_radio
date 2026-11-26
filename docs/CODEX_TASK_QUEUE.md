# Codex execution queue — Rust Radio (2026-11-05)

Status meanings: READY = scoped, not started; BLOCKED = missing prerequisite; IN_REVIEW = PR exists; DONE = implementation and evidence verified. These are proposed tasks, **not** reports of executed tests. Follow AGENTS.md.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-101 | P0 | Transport | READY | Reproduce Skylab validation/listening discrepancy with structured, redacted endpoint and stage diagnostics | Distinguish DNS/TLS/HTTP/playlist/codec/callback and reproduce with a fixture; no autoplay |
| RR-102 | P0 | Validation | READY | Catalog sweep with timestamped JSON/history review | Report each canonical ID, attempts, decoded stage, failures and known environmental limits; do not equate 25/25 historical decode with current availability |
| RR-103 | P0 | Audio | BLOCKED: physical device | Mac audio loss, device-switch and stop-recovery evidence | Explicit speaker/Bluetooth/USB manual matrix; no overlapping sessions |
| RR-104 | P1 | GUI | READY | 480x320 control/state regression and a human operator checklist | Long text, arrows, keyboard, volume, mute, Stop/Retry, station switches tested; physical touch remains separate |
| RR-105 | P1 | SDR | BLOCKED: receiver access | Independent direct nRSP-ST live audio and control acceptance | Record receiver, handshake, tuning, PCM/callback, audible output, Stop cleanup |
| RR-106 | P1 | SDR | BLOCKED: USB hardware | SoapySDR 240ksample/s CS16 mono-FM field acceptance | Supported dongle/driver, actual RF/audio and unplug cleanup |
| RR-107 | P1 | Integration | BLOCKED: Pi 5 target | Pi runtime and Rust-Boy embedded host signoff | Pinned dependency, physical screen, native output, CPU/memory/thermal/power |
| RR-108 | P1 | Validation | READY | CI status audit and exact-SHA evidence reconciliation | Distinguish startup_failure from test failure and verify hosted jobs before completion |
| RR-109 | P2 | Research | BLOCKED: publisher permission | Broadcaster rights/approved stream audit, NPR candidate review | Source-linked provenance and license/embedding approvals before distributable preset changes |

## Priority override — SiriusXM and main-panel simplification (2026-11-26)

Owner: Linux Operations / Codex. Status: READY; implementation and runtime tests NOT RUN. Execute RR-110 first, then RR-111; preserve existing tasks.

| ID | Priority | Deliverable | Acceptance |
| --- | --- | --- | --- |
| RR-110 | P0 | Remove nSDR and USB buttons from standalone main listening panel; move existing receiver entry points into Settings > Receivers | Main panel has neither button nor empty hit targets; receiver functionality and feature flags remain intact; 480x320 screenshot and keyboard navigation pass |
| RR-111 | P0 | Review external-provider architecture and explicitly revise existing no-browser-launch invariant for user-initiated standalone provider only | Document rights/platform review, no autoplay, no embedded-host behavior changes, explicit user action; stop if provider terms prohibit integration |
| RR-112 | P0 | Add SiriusXM button opening official HTTPS player in system browser | Click launches https://player.siriusxm.com/; browser owns login/MFA/cookies; no credentials, tokens, browser profile reads, scraping or debugging interfaces; user can return manually |
| RR-113 | P1 | Generic external audio provider and PipeWire session controls on Arch | Explicitly associate correct media session; volume/mute/output controls only where supported; never control unrelated browser tabs; native playback and external playback do not overlap; fail closed if ambiguous |
| RR-114 | P1 | Optional MPRIS transport/metadata capability discovery | Only supported controls exposed; no private API or DRM bypass; no recording, capture, redistribution; distinguish external playback state from native PCM callback evidence |
| RR-115 | P1 | Regression and physical acceptance | fmt, workspace tests, clippy, feature builds, screenshot, real subscriber-driven playback and return-to-UI test; redacted evidence with exact SHA; update README, progress and roadmap |

### Execution contract
Inspect AGENTS.md, GUI source and current porting plan before changes. Keep IP Radio as the default; place SiriusXM alongside it without expanding the main panel footprint. Receiver settings remain reachable but must not autoplay. No automatic login or callback is assumed. Browser launch success is not authentication or playback success. If session isolation or supported controls cannot be verified, retain a launch-only integration and label limitations. Preserve native headless API and Pip-Boy dependency pin. Use mock launcher/session fixtures; test real login only by user interaction on target machine. No credentials in tests, logs, screenshots or Git. Roll back individual changes if existing playback regresses. Record each gate separately and open a PR before implementation merge.

## Previous Codex order: RR-101, then RR-102 (resume after priority override or when blocked)
1. Fetch latest main and inspect actual station definition and stream path. Document the exact commit being tested.
2. Add fixture-first tests around the reproducible failure; do not infer the cause from a website response, old status, or one failed network attempt.
3. Run silent `cargo run -p rust-radio-cli -- validate skylab`; compare with `cargo run -p rust-radio-cli -- listen skylab --seconds 30` only when audio testing is explicitly authorized.
4. Capture redacted HTTP chain, final content type, codec probing and last verified stage without leaking URL query tokens. The transport must remain bounded and secure.
5. If a defect is proven, implement the minimum change, add a regression and run required gates. If outbound network is unavailable, mark blocked and provide deterministic fixtures, not a false failed station verdict.
6. Open a PR, update queue/roadmap/progress and stop at unresolved hardware or rights gates. Avoid changing other repos unless the host pin is specifically in scope.

## Parallel review
Validation agent independently checks the proposed reproduction, cancellation, stale-result handling, no autoplay, private endpoint filtering and evidence for claimed success. Integration agent reviews Rust-Boy API compatibility but does not update its dependency pin without explicit host validation.
