# Curated radio directory, redundant streams and health hysteresis

The authoritative runtime directory is `crates/rust-radio-core/src/directory.rs`. `catalog/curated-stations.json` is a machine-readable mirror, checked against the native catalog by offline unit tests. The list contains **25 stable station IDs**, including **GBH 89.7 / NPR News (Boston)**, which replaced PopTron. Publisher provenance: https://www.wgbh.org/help/how-to-access-live-radio-streams.

## Safety rules: never automatically remove a good station

**The curated 25-station list is permanent until an explicit, reviewed catalog change.** A bad network check is not a reason to remove, rename, or reorder a channel. Live availability and the right to *try* a station are separate concepts.

A radio stream is marked **fresh** only when actual decoded audio frames succeeded within the previous **six hours**. DNS, TCP, an HTTP 200, and successful playlist parsing are insufficient. Separate operational states are:

| State | Evidence and behavior | In default `--eligible`? |
| --- | --- | --- |
| `fresh` | Latest successful decoded audio, age at most 6 hours | Yes |
| `degraded` | Previous decoded audio within 24 hours, now failing or not rechecked within 6 hours; fewer than 3 independent failing sweeps | **Yes**, labeled degraded, not fresh |
| `stale` | Last success older than 24 hours, or clock evidence invalid | No; still present in catalog and manually selectable |
| `unverified` | No trustworthy successful decode yet | No; still present in catalog |
| `unavailable` | 3 completed failing sweeps with no provider-wide suppression | No; still present in catalog and manually selectable |

After success, both failure counter and degraded status clear immediately. Last-good timestamps are **never** renewed by failed requests. The 24-hour grace period is deliberately bounded: it maintains the listening experience during a temporary outage, but never claims an untested feed is fresh.

### Independent redundancy layers

1. **Within each completed sweep:** retry an unsuccessful decoder probe once after 1.5 seconds; do not issue a second request when the first succeeds. Retries do **not** increment the independent-sweep failure counter.
2. **Across sweeps:** require 3 *completed*, separately scheduled failed sweeps to classify a single station as `unavailable`. Interrupted sweeps leave the counter unchanged. The default daemon interval is 4 hours.
3. **Across publishers:** if every channel on SomaFM or GBH fails simultaneously, suspect a shared provider/CDN fault. Do not advance those station-level failure counters; retain individual error details. Single-channel publishers still use ordinary multi-sweep hysteresis.
4. **Across the whole host:** if all 25 streams fail to decode in one sweep, mark `network_incident_suspected` and do not advance any station-level failure counters.
5. **Across CDN mirrors:** for curated M3U/PLS stations only, the core stream resolver **and native playback transport** may try another listed publisher-approved HTTPS endpoint if a prior endpoint has a transport, HTTP, or invalid-content-type failure. This is serial, bounded by `MAX_REQUEST_HOPS`, and never permits arbitrary hosts. A decoder-level malformed-audio failure still requires a new validation attempt; a playlist mirror is not proof of decoded audio.
6. **Across restarts:** keep a second complete health snapshot at `<output>.bak`. The primary file is atomically replaced after each checked station; the backup is replaced only after an entire sweep completes. On corrupt/missing primary JSON, load the backup. Do not silently replace two corrupt reports with empty health history.

For health monitoring, automatic filtering should operate on the exported **ID allowlist**, never mutate `CURATED_STATIONS`. Even a station recorded as `unavailable` remains available for an explicit operator playback attempt and subsequent revalidation.

## Headless native validator

`crates/rust-radio-cli/src/bin/rust-radio-feed-validator.rs` performs receive-only, serial stream checks without opening a speaker, attaching SDR hardware, or occupying the GUI render thread. It uses the same native Rust stream decoder/transport as the radio system.

```sh
# Single complete sweep; exits nonzero if any station is not eligible:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --once

# Continuous operator-enabled monitor, beginning with an immediate sweep:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --daemon --interval-minutes 240 --output ./state/feed-health.json

# Recompute health from last-good evidence, without contacting broadcasters:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --status --output ./state/feed-health.json

# Use for ordinary radio browsing: fresh + bounded degraded grace:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --eligible --output ./state/feed-health.json

# Use for strict applications: decoded within 6h only:
cargo run --locked -p rust-radio-cli --bin rust-radio-feed-validator -- --strict-eligible --output ./state/feed-health.json
```

The default output is `target/radio-feed-health.json` (ignored by Git). Use a persistent `--output` directory for deployment. Serial scans pause 500 ms between stations. Each underlying validator enforces finite network/decode deadlines, so complete scans may take several minutes when streams are unavailable. `--interval-minutes` accepts 15–240, only in daemon mode; the interval includes the duration of the sweep. **Run one daemon per report path**: there is no cross-process lease manager yet.

### JSON schema and migration

Current report schema is **version 2**; version-1 reports are accepted and their successful decoded-audio evidence is migrated without wiping the last-good timestamp.

Each ordered station record contains its immutable `id`, `name`, `genre`, `category`, and publisher reference, plus:

- `validation`: latest bounded probe outcome, including codec, timestamp, decoded frames and concise error detail
- `last_good`: the most recent successful decoded-audio evidence, retained even after failure
- `probe_attempts`: 1 or 2 per checked station
- `consecutive_failed_sweeps`: capped at 3, excluding suspected whole-host and shared-publisher incidents
- `health`: point-in-time cached label, recomputed by `--status` and `--eligible`

Top-level fields include `checked`, `complete`, `fresh`, `degraded`, `stale`, `unverified`, `unavailable`, `network_incident_suspected` and, after a complete scan, `publisher_incidents_suspected`. Do not trust a stored health string beyond its timestamp; consumers must recompute expiry from the validation evidence. The reader rejects oversized or invalid JSON, mismatched catalog IDs, and unsupported schema versions. Raw custom stream URLs and query tokens are not persisted.

## GUI integration boundary

The standalone GUI still performs its own explicit selected-station checks. The background JSON report and `--eligible` ID set are **not yet wired into the GUI**. This pass prevents validator consumers from treating single failures as automatic station removal, but it does not yet implement a GUI filter. A future GUI integration should show `fresh` and `degraded` distinctly, retain manual override for troubleshooting, and never remove a curated entry on probe failure.

## Verification and operational acceptance

```sh
cargo fmt --all -- --check
cargo test --locked -p rust-radio-core
cargo test --locked -p rust-radio-cli --bin rust-radio-feed-validator
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Offline regression tests cover a transient first-probe error followed by success, triple-sweep failure thresholds, recovery, total-host outage suppression, shared-publisher outage suppression, 6h/24h boundaries, atomic backup restoration, schema-v1 migration, strict versus grace eligibility and allowlisted CDN failover. Passing offline tests never proves external stations are currently live. Run a **real network-enabled `--once` sweep** and inspect the results before calling the 25 feeds verified. Publisher links are for personal listening; they do not authorize rebroadcasting or redistribution.
