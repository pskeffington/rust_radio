# Curated radio directory and feed health

The **authoritative runtime directory** is `crates/rust-radio-core/src/directory.rs`. `catalog/curated-stations.json` is a machine-readable mirror for external tools and indexing; regenerate it when the Rust directory changes. The two lists must have identical ordered IDs, names, broadcaster, genre, language, country, category, stream URLs, publisher references, codec hints and nominal bitrates. A CLI regression test enforces this parity.

## Operational rules

- 25 stations, stable IDs, compact grouping and descriptive genres. Publisher links are provenance, not a playback guarantee.
- All catalog entries are candidates until an **actual bounded HTTP → supported codec → decoded audio** check succeeds. DNS, TCP, HEAD, and playlist status alone do not qualify.
- Decoder evidence has a six-hour TTL via `StationValidation::health_at`: `Fresh`, `Stale`, `Unverified`, `Failed`. Keep failures visible and identifiable, not falsely marked healthy.
- The CLI validator uses an 18-second deadline. GUI selection checks use the cancellable playback transport with a 35-second first-decode budget, without opening audio output. Setup and station tooltips retain historical freshness. A station change triggers a selection probe; checks never run on the GUI render thread.
- A daily GitHub Actions workflow runs an opt-in bounded silent sweep and retains its JSON report for 30 days. This is point-in-time evidence only: it does not persist freshness into the runtime, block playback, or guarantee every station is fresh between sweeps. The CLI can additionally maintain a local history file with `--history PATH`; an egui host may import that snapshot through `RadioPanel::load_validation_history`.
- Freshness refers to the last silent check. The listening view and selected-station tooltip prioritize current playback, so stale or missing historical evidence cannot contradict live audio. Other tooltips identify freshness as the last silent check.
- The user may still add direct/custom URLs using the independent saved-stations bank. Never pass arbitrary URLs into curated allowlist checks.
- Streams are for receiver-side listening only; no rebroadcasting or relay.

## Validation

Run `cargo fmt --all -- --check` and `cargo test --workspace`. On a network-enabled host, recheck each selected channel using the built-in decoder validation path; record the check time and status. Work through failed feeds without silently swapping stations or removing their identities. The CLI `validate-all --json` command probes the curated 25 feeds with at most four concurrent checks. Each probe uses the existing 18-second deadline; `unavailable` results receive one retry after a one-second recovery delay, so each station has at most two attempts. Other HTTP rejections (except 408, 425, and 429) are `invalid_stream` and are not retried. Reports include attempts and whether a retry recovered. A failed feed remains in the catalog either way. Add `--history PATH` to atomically maintain an optional 64 KiB-bounded local state file with latest status, last decoded time, health and consecutive failure count; the JSON report remains on stdout. The state file is an audit aid; hosts can import it for visible, non-authoritative history, but it never gates playback. For remote 24/7 catalog monitoring, rotate through feeds with per-station backoff and jitter rather than creating 25 simultaneous connections.

The SomaFM [Left Coast 70s direct stream page](https://somafm.com/seventies/directstreamlinks.html) lists `https://somafm.com/m3u/seventies.m3u` as its permanent 128 kbps MP3 M3U endpoint and states that streams are for individual personal use. Keep receiver-side playback only; do not redistribute or relay the stream.

Publisher directory: https://somafm.com/listen/ (for SomaFM station provenance); publisher references for the other two stations are recorded individually.
