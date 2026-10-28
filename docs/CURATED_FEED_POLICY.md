# Curated radio directory and feed health

The **authoritative runtime directory** is `crates/rust-radio-core/src/directory.rs`. `catalog/curated-stations.json` is a machine-readable mirror for external tools and indexing; regenerate it when the Rust directory changes. The two lists must have identical ordered IDs, URLs, genres, and categories.

## Operational rules

- 25 stations, stable IDs, compact grouping and descriptive genres. Publisher links are provenance, not a playback guarantee.
- All catalog entries are candidates until an **actual bounded HTTP → supported codec → decoded audio** check succeeds. DNS, TCP, HEAD, and playlist status alone do not qualify.
- Decoder evidence has a six-hour TTL via `StationValidation::health_at`: `Fresh`, `Stale`, `Unverified`, `Failed`. Keep failures visible and identifiable, not falsely marked healthy.
- The existing 18-second, cancellable validation worker checks the selected stream silently, without opening the audio output. The UI labels checked stations with their freshness state. A station change triggers a selection probe; checks are not performed on the GUI render thread.
- **Current limitation:** this implementation has no persistent six-hour background sweep or admission gate blocking playback of unverified stations. It therefore does not yet guarantee that every listed station is fresh. Add an opt-in bounded background sweep with persisted validation evidence and playback gating in a subsequent implementation before claiming such a guarantee.
- The user may still add direct/custom URLs using the independent saved-stations bank. Never pass arbitrary URLs into curated allowlist checks.
- Streams are for receiver-side listening only; no rebroadcasting or relay.

## Validation

Run `cargo fmt --all -- --check` and `cargo test --workspace`. On a network-enabled host, recheck each selected channel using the built-in decoder validation path; record the check time and status. Work through failed feeds without silently swapping stations or removing their identities. For remote 24/7 catalog monitoring, rotate through the 25 feeds with bounded concurrency, per-station backoff and jitter rather than creating 25 simultaneous connections. Probe results are volatile by design until a persisted feed-health store is implemented.

Publisher directory: https://somafm.com/listen/ (for SomaFM station provenance); publisher references for the other two stations are recorded individually.
