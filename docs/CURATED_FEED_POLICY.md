# Curated radio directory and automatic feed health

The **authoritative runtime directory** is `crates/rust-radio-core/src/directory.rs`. `catalog/curated-stations.json` is its machine-readable mirror, checked for parity by the native validator's offline tests. Station IDs and their ordered positions must remain stable unless a station is deliberately replaced.

## Station policy

- There are exactly 25 first-party publisher-referenced curated channels, including **GBH 89.7 — NPR News (Boston)** (`gbhnews`) with the publisher-documented HTTPS direct stream. It replaces the former PopTron entry. GBH official source: https://www.wgbh.org/help/how-to-access-live-radio-streams
- A published address is a *candidate*, not proof of a live stream. Only **an actual HTTP → supported codec → decoded audio** probe can mark it fresh. TCP, HEAD, a successful playlist download or endpoint reachability never suffices.
- Successful decoder evidence has a strict six-hour TTL (`FEED_FRESHNESS_TTL`). The recognized states are `fresh`, `stale`, `unverified`, and `failed`. Subsequent failure supersedes earlier success. Hosts must recalculate freshness using `validation.checked_at_unix`, not assume a JSON status string remains current.
- Station-specific HTTPS host and path allowlists remain enforced by `ApprovedEndpoint`; a direct/user-saved URL is outside the curated catalog and cannot masquerade as a validated station.
- Listening is receive-only; the application does not rebroadcast audio or convey a redistribution license.

## Native automatic validator

The headless Rust CLI at `crates/rust-radio-cli/src/bin/rust-radio-feed-validator.rs` consumes the same 25-entry directory and the existing bounded decoder validation path. It does not open audio output, attach SDR hardware, start playback or block the radio GUI.

```sh
# One complete scan; nonzero exit if any station is not verified:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --once

# Operator-enabled periodic validator (immediate scan then every four hours):
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --daemon

# Use an explicit persistent location instead of target/:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --daemon --output ./state/feed-health.json

# Read the saved report and recompute the TTL (no network access):
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --status --output ./state/feed-health.json

# Emit only currently eligible station IDs for a host to consume (JSON array):
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --eligible --output ./state/feed-health.json
```

The default report is `target/radio-feed-health.json` (ignored by Git). The daemon scans serially (one remote connection at a time) with 500-ms spacing and no overlapping scans. Each probe inherits the existing cancellable, bounded native decoder validation deadline. A scan takes up to several minutes if every endpoint times out; the default four-hour interval gives margin before the six-hour TTL. Use `--daemon --interval-minutes N` with `15 <= N <= 240` to change cadence. Run only one daemon per report path; the current implementation has no multi-process lease manager.

Each station gets an ordered entry with stable ID, name, genre, category, publisher provenance, current `health`, and nested `validation` evidence (status, stage, UTC Unix check timestamp, codec, sample rate, channels, decoded frames and bounded error detail). Report schema version is **1**. After each station the complete 25-entry JSON structure is atomically replaced on Unix/macOS; `checked` and `complete` distinguish interrupted scans. The validator never persists user-custom streaming tokens or raw resolved URLs.

The `--eligible` output is an allowlist of fresh station **IDs**, not raw user-supplied URLs. `--status` recomputes expiry on read without contacting broadcasters. Invalid reports, mismatched catalog IDs, oversized reports and unsupported schema versions fail closed. For unattended services, supply an output path in a persistent application state directory (the default `target/` may be cleaned during development).

## Playback / GUI integration boundary

The existing GUI still independently validates a newly selected station before playback and reports its check state. **The auto-validator's JSON is currently external to the GUI:** an integrator must consume `--eligible` or parse and time-check the JSON to filter recommendations or enforce a strict fresh-only automatic mode. It does not silently block an operator's explicit Play request. A future integration pass may import signed/permission-controlled local snapshots and provide a `Verified only` toggle without blocking manual diagnostic tuning.

## Verification

```sh
cargo fmt --all -- --check
cargo test --locked -p rust-radio-core
cargo test --locked -p rust-radio-cli --bin rust-radio-feed-validator
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The validator unit tests use **offline injected fake reports**, never external stream probes. No channel is asserted live merely by a passing build. Run `--once` on a network-enabled host for real live acceptance, check the per-channel errors, and then evaluate `--eligible` before promoting the station list.
