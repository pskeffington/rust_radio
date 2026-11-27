# Curated radio directory and feed health

The **authoritative runtime directory** is `crates/rust-radio-core/src/directory.rs`. `catalog/curated-stations.json` is a machine-readable mirror. Both lists must have identical ordered IDs, names, broadcaster, genre, language, country, category, stream URLs, publisher references, codec hints and nominal bitrates. A CLI regression test enforces parity.

## Operational rules

- The starter catalog contains two GBH streams: `gbh897` and `wcrb`. GBH lists both for use through third-party radio apps in its [live-stream help page](https://www.wgbh.org/help/how-to-access-live-radio-streams). This is source provenance, not a feed guarantee or a grant of commercial bundling, embedding, or redistribution rights.
- Both streams remain `unverified_until_runtime_probe` until an actual bounded HTTP → supported codec → decoded audio check succeeds. DNS, TCP, HEAD, and HTTP status alone do not qualify.
- Decoder evidence has a six-hour TTL via `StationValidation::health_at`: `Fresh`, `Stale`, `Unverified`, `Failed`. Keep failures visible and identifiable, not falsely marked healthy.
- The CLI validator uses an 18-second deadline. GUI selection checks use the cancellable playback transport with a 35-second first-decode budget, without opening audio output. Setup and station tooltips retain historical freshness. A station change triggers a selection probe; checks never run on the GUI render thread.
- A daily GitHub Actions workflow runs an opt-in bounded silent sweep and retains its JSON report for 30 days. This is point-in-time evidence only and does not authorize distribution.
- Existing v1 history can contain retired IDs from older catalogs. Imports ignore retired station records, preserve only current curated IDs in memory and write only current IDs on the next successful update; oversized history still fails closed at the existing byte bound.
- The user may still add direct/custom URLs using the independent saved-stations bank. Never pass arbitrary URLs into curated allowlist checks.
- Do not rebroadcast, relay, or bundle streams without written rights approval for the intended product and distribution model.

## Validation and release gates

Run `cargo fmt --all -- --check` and `cargo test --workspace` when implementation verification is authorized. On a network-enabled host, recheck each selected channel using the built-in decoder validation path; record check time, exact commit and status. The CLI `validate-all --json` command probes the current two feeds with at most four concurrent checks. Each probe uses the existing 18-second deadline; `unavailable` results receive one retry after a one-second recovery delay. Reports include attempts and retry outcome. A failed feed remains visible until a reviewed catalog decision changes it. `--history PATH` atomically maintains an optional 64 KiB-bounded local state file; this is informational and never gates playback.

Before production distribution, obtain and record GBH's written permission or other explicit terms covering the Rust Radio client and its intended distribution. A listing in app directories alone does not clear this gate. Do not claim production-ready curated channels before rights, live decode and target-device playback gates pass.

## Excluded sources

SomaFM is removed from the current catalog because its [Terms of Service](https://somafm.com/legal/tos.html) say it cannot authorize third-party clients and prohibit stream embedding without prior written permission. Skylab is not in the starter set because explicit client permission has not been established. These exclusions can be revisited only with affirmative publisher authorization.
