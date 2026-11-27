# Codex execution queue — Rust Radio (2026-11-05)

Status meanings: READY = scoped, not started; IN_PROGRESS = implementation underway; BLOCKED = missing prerequisite; IN_REVIEW = PR exists; DONE = implementation and evidence verified. These are proposed tasks, **not** reports of executed tests. Follow AGENTS.md.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-101 | P0 | Transport | READY | Reproduce current curated-stream validation/listening discrepancy with structured, redacted endpoint and stage diagnostics | Distinguish DNS/TLS/HTTP/codec/callback and reproduce with a fixture; no autoplay |
| RR-102 | P0 | Validation | READY | Catalog sweep with timestamped JSON/history review | Report each canonical ID, attempts, decoded stage, failures and known environmental limits; do not equate historical decoded feed results with current availability |
| RR-103 | P0 | Audio | BLOCKED: physical device | Mac audio loss, device-switch and stop-recovery evidence | Explicit speaker/Bluetooth/USB manual matrix; no overlapping sessions |
| RR-104 | P1 | GUI | READY | 480x320 control/state regression and a human operator checklist | Long text, arrows, keyboard, volume, mute, Stop/Retry, station switches tested; physical touch remains separate |
| RR-105 | P1 | SDR | BLOCKED: receiver access | Independent direct nRSP-ST live audio and control acceptance | Record receiver, handshake, tuning, PCM/callback, audible output, Stop cleanup |
| RR-106 | P1 | SDR | BLOCKED: USB hardware | SoapySDR 240ksample/s CS16 mono-FM field acceptance | Supported dongle/driver, actual RF/audio and unplug cleanup |
| RR-107 | P1 | Integration | BLOCKED: Pi 5 target | Pi runtime and Rust-Boy embedded host signoff | Pinned dependency, physical screen, native output, CPU/memory/thermal/power |
| RR-108 | P1 | Validation | READY | CI status audit and exact-SHA evidence reconciliation | Distinguish startup_failure from test failure and verify hosted jobs before completion |
| RR-109 | P0 | Research | BLOCKED: written publisher permission | Confirm GBH client and product-distribution rights for current stream candidates | Written authorization covering bundled preset use; attach source and scope |

## Current priority — streaming-only app (2026-11-27)

Product boundary: this repository builds an internet streaming radio app. nSDR, USB SDR, SDRconnect, SoapySDR, and satellite products are maintained in separate repositories. Their implementation and acceptance tasks are removed from this queue.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-119 | P0 | Integration | IN_PROGRESS | Remove receiver discovery, selectors, CLI paths, optional feature wiring, and receiver build dependencies from Rust Radio | GUI and CLI expose streaming only; workspace package graph contains no receiver crate |
| RR-120 | P1 | GUI | IN_PROGRESS | Finish and inspect streaming station browsing on the rebuilt Mac app | Browse curated/custom internet stations, play/stop, choose output; no receiver segments or scan actions |

## Current priority — curated station usability (2026-11-27)

SiriusXM is removed from the product roadmap because no supported development API is available. PR #18 was closed unmerged; origin/main did not contain a SiriusXM integration. The current implementation branch removes the SomaFM and Skylab presets and narrows the starter catalog to two GBH stations. This is not a production rights or feed-availability approval.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-116 | P0 | Validation | READY | Silent live validation of `gbh897` and `wcrb` on exact candidate SHA | Record HTTP/redirect/content type/codec/decode outcome per ID; no playback implied |
| RR-117 | P1 | Audio/GUI | BLOCKED: operator device | Mac operator playback of both candidates on exact reviewed build | User-initiated playback, stop and output state; callback and audible evidence distinguished |
| RR-118 | P1 | Release | BLOCKED: CI and rights | Reconcile CI and distribution gates before release | Exact-head fmt/tests/clippy, macOS bundle relaunch and rights decision |

The SiriusXM draft PR is closed and unmerged. Do not reopen the integration without a documented official development interface and applicable distribution rights.

## Current Codex order: RR-119, then RR-116
1. Finish the streaming-only source and package cleanup on the isolated review branch. Keep receiver code in its separate repository. Then inspect the curated station definition and stream path on the exact candidate SHA.
2. Add fixture-first tests around the reproducible failure; do not infer the cause from a website response, old status, or one failed network attempt.
3. Run silent `cargo run -p rust-radio-cli -- validate-all --json`; compare with explicit Mac `listen` or GUI playback only after rights review and operator authorization.
4. Capture redacted HTTP chain, final content type, codec probing and last verified stage without leaking URL query tokens. The transport must remain bounded and secure.
5. If a defect is proven, implement the minimum change, add a regression and run required gates. If outbound network is unavailable, mark blocked and provide deterministic fixtures, not a false failed station verdict.
6. Open a PR, update queue/roadmap/progress and stop at unresolved hardware or rights gates. Avoid changing other repos unless the host pin is specifically in scope.

## Parallel review
Validation agent independently checks the proposed reproduction, cancellation, stale-result handling, no autoplay, private endpoint filtering and evidence for claimed success. Integration agent reviews Rust-Boy API compatibility but does not update its dependency pin without explicit host validation.
