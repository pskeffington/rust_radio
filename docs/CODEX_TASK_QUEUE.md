# Codex execution queue — Rust Radio (2026-11-23)

Status meanings: READY = scoped, not started; BLOCKED = missing prerequisite; IN_REVIEW = PR exists; DONE = implementation and evidence verified. These are proposed tasks, **not** reports of executed tests. Follow AGENTS.md.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-101 | P0 | Transport | IN_REVIEW | Reproduce Skylab validation/listening discrepancy with structured, redacted endpoint and stage diagnostics | Distinguish DNS/TLS/HTTP/playlist/codec/callback and reproduce with a fixture; no autoplay |
| RR-102 | P0 | Validation | DONE | Catalog sweep with timestamped JSON/history review | Current 25-station silent sweep and endpoint-free history are recorded by exact source SHA; callback/audible output and publisher rights remain separate |
| RR-103 | P0 | Audio | BLOCKED: physical device | Mac IP-stream output loss, device-switch and stop-recovery evidence | Explicit speaker/Bluetooth/external sound-output matrix; no overlapping sessions |
| RR-104 | P1 | GUI | DONE | 480x320 control/state regression and a human operator checklist | Offline controls/layout suite and IP-only Mac checklist are recorded; interactive, audible and physical touch acceptance remain separate |
| RR-107 | P1 | Integration | BLOCKED: Pi 5 target | Pi runtime and Rust-Boy embedded host signoff | Pinned dependency, physical screen, native output, CPU/memory/thermal/power |
| RR-108 | P1 | Validation | BLOCKED: Actions runner startup | CI status audit and exact-SHA evidence reconciliation | PR #14 head 241976c run 45214437722 ended `startup_failure` before job creation (0 jobs); recheck on next new run |
| RR-109 | P0 | Research | BLOCKED: app distribution rights | Current three-publisher audit recorded in `docs/PUBLISHER_RIGHTS_AUDIT.md`; SomaFM's 23 presets are expressly blocked for new third-party clients; GBH and Skylab permission remains unresolved | No curated station data in a Rust Radio release without recorded applicable rights; exclude blocked entries or establish written permission; NPR remains a separate excluded candidate |
| RR-120 | P0 | Scope | IN_REVIEW | Restrict Rust Radio to internet streaming; remove receiver code, features, docs and CI | Workspace gates pass; retained audio output and IP stream paths remain covered |
| RR-121 | P2 | Scope | DONE | Reconcile current scope and historical evidence after receiver migration | Radio-frequency receiver code/acceptance belongs to separate projects; current Rust Radio gates remain IP streaming, audio, GUI, Pi, CI and rights |
| RR-118 | P1 | Packaging | IN_REVIEW | Build and install a macOS app with a Rust Radio icon and one Dock launcher | Bundle metadata and staged install verified; signing/notarization and clean-session interactive launch remain separate gates |

## Immediate Codex order: RR-108 hosted CI startup diagnosis; continue local portable-platform and API compatibility gates while physical device/Pi gates await hardware; RR-109 release gate stays closed pending rights clearance or a product decision to exclude affected presets
1. Fetch latest main and inspect actual station definition and stream path. Document the exact commit being tested.
2. Add fixture-first tests around the reproducible failure; do not infer the cause from a website response, old status, or one failed network attempt.
3. Run silent `cargo run -p rust-radio-cli -- validate skylab`; compare with `cargo run -p rust-radio-cli -- listen skylab --seconds 30` only when audio testing is explicitly authorized.
4. Capture redacted HTTP chain, final content type, codec probing and last verified stage without leaking URL query tokens. The transport must remain bounded and secure.
5. If a defect is proven, implement the minimum change, add a regression and run required gates. If outbound network is unavailable, mark blocked and provide deterministic fixtures, not a false failed station verdict.
6. Open a PR, update queue/roadmap/progress and stop at unresolved hardware or rights gates. Avoid changing other repos unless the host pin is specifically in scope.

## Parallel review
Validation agent independently checks the proposed reproduction, cancellation, stale-result handling, no autoplay, private endpoint filtering and evidence for claimed success. Integration agent reviews Rust-Boy API compatibility but does not update its dependency pin without explicit host validation.
