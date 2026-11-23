# Codex execution queue — Rust Radio (2026-11-23)

Status meanings: READY = scoped, not started; BLOCKED = missing prerequisite; IN_REVIEW = PR exists; DONE = implementation and evidence verified. These are proposed tasks, **not** reports of executed tests. Follow AGENTS.md.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-101 | P0 | Transport | IN_REVIEW | Reproduce Skylab validation/listening discrepancy with structured, redacted endpoint and stage diagnostics | Distinguish DNS/TLS/HTTP/playlist/codec/callback and reproduce with a fixture; no autoplay |
| RR-102 | P0 | Validation | READY | Catalog sweep with timestamped JSON/history review | Report each canonical ID, attempts, decoded stage, failures and known environmental limits; do not equate 25/25 historical decode with current availability |
| RR-103 | P0 | Audio | BLOCKED: physical device | Mac IP-stream output loss, device-switch and stop-recovery evidence | Explicit speaker/Bluetooth/USB sound-output matrix; no overlapping sessions |
| RR-104 | P1 | GUI | READY | 480x320 control/state regression and a human operator checklist | Long text, arrows, keyboard, volume, mute, Stop/Retry, station switches tested; physical touch remains separate |
| RR-107 | P1 | Integration | BLOCKED: Pi 5 target | Pi runtime and Rust-Boy embedded host signoff | Pinned dependency, physical screen, native output, CPU/memory/thermal/power |
| RR-108 | P1 | Validation | BLOCKED: Actions runner startup | CI status audit and exact-SHA evidence reconciliation | PR #14 head ac23d9b run 45155794094 failed before job creation (0 jobs/logs); retry API returns 403; recheck on next new run |
| RR-109 | P2 | Research | BLOCKED: publisher permission | Broadcaster rights/approved stream audit, NPR candidate review | Source-linked provenance and license/embedding approvals before distributable preset changes |
| RR-120 | P0 | Scope | IN_REVIEW | Restrict Rust Radio to internet streaming; remove receiver code, features, docs and CI | Workspace gates pass; retained audio output and IP stream paths remain covered |
| RR-121 | P2 | Scope | DONE | Reconcile current scope and historical evidence after receiver migration | USB SDR and nSDR code/acceptance belong to separate projects; current Rust Radio gates remain IP streaming, audio, GUI, Pi, CI and rights |

## Immediate Codex order: RR-101, then RR-102
1. Fetch latest main and inspect actual station definition and stream path. Document the exact commit being tested.
2. Add fixture-first tests around the reproducible failure; do not infer the cause from a website response, old status, or one failed network attempt.
3. Run silent `cargo run -p rust-radio-cli -- validate skylab`; compare with `cargo run -p rust-radio-cli -- listen skylab --seconds 30` only when audio testing is explicitly authorized.
4. Capture redacted HTTP chain, final content type, codec probing and last verified stage without leaking URL query tokens. The transport must remain bounded and secure.
5. If a defect is proven, implement the minimum change, add a regression and run required gates. If outbound network is unavailable, mark blocked and provide deterministic fixtures, not a false failed station verdict.
6. Open a PR, update queue/roadmap/progress and stop at unresolved hardware or rights gates. Avoid changing other repos unless the host pin is specifically in scope.

## Parallel review
Validation agent independently checks the proposed reproduction, cancellation, stale-result handling, no autoplay, private endpoint filtering and evidence for claimed success. Integration agent reviews Rust-Boy API compatibility but does not update its dependency pin without explicit host validation.
