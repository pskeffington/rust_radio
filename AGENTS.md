# AGENTS.md — Rust Radio development contract

## Mission
Develop a portable, receive-only, host-neutral native Rust radio system with reliable IP audio, optional SDR reception and a compact operator interface. Integrate with Rust-Boy through a pinned API; never couple core audio to the HUD.

## Instructions for Codex and delegated agents
1. Read this file, README.md, docs/STATUS_2026-11-05.md, docs/PORTING_PLAN.md, docs/PROGRESS.md, docs/EMBEDDING.md and relevant subsystem contract before changing code.
2. Inspect current implementation and tests; verify historical claims against the exact checked-out SHA. Treat a plan or prior test result as evidence only for that recorded revision.
3. Use a bounded task from docs/CODEX_TASK_QUEUE.md. Implement the smallest independently testable change; avoid unrelated refactors, new dependencies, license changes or feature creep.
4. Run cargo fmt --all -- --check, cargo test --workspace and cargo clippy --workspace --all-targets -- -D warnings. Run optional-feature and platform checks when dependencies and hardware permit. Report commands, counts, failures and environment limits precisely.
5. Preserve no-autoplay, receive-only operation, one active session, cancelable Stop, checked public endpoints, bounded memory/timeouts, and the separation between source validation, decoded PCM, callback-consumed PCM and human-audible verification.
6. Never commit secret-bearing stream URLs, API tokens, private receiver addresses or raw operator data to logs/evidence. Preserve broadcaster/publisher provenance and require rights review before bundling streams.
7. Update docs/PROGRESS.md with dated, commit-specific evidence, docs/PORTING_PLAN.md with changed gates, and docs/CODEX_TASK_QUEUE.md with task state. Mark BLOCKED, NOT RUN and PASS separately.
8. Open a focused PR against main; do not self-certify physical audio, Pi, receiver or publisher rights acceptance from synthetic tests or cross-compilation. Do not merge if required checks have not run, unless explicitly authorized and clearly documented.

## Delegation lanes
- Transport: redirects, playlist parsing, timeouts, DNS rebinding and codec validation.
- Audio: native device selection, output callbacks, ring buffering, cancellation and recovery.
- GUI: egui 480x320 accessibility, station selection, clear state and input/output lifecycle.
- SDR: direct nRSP-ST, SDRconnect and USB SoapySDR, independently feature gated.
- Integration: pinned host API, Pi 5 and cross-platform portability.
- Validation: independent review of tests, evidence provenance, regression and CI.

## Definition of done
Changes are reviewable, tests and behavior gates are recorded by exact SHA, source boundaries stay intact, regression coverage matches the failure, and unresolved hardware/rights gates remain explicitly open. A passing silent validator never equals audible output.
