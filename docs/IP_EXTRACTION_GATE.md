# Completed IP-radio core extraction

Source extraction began on 2026-10-26. Host cutover is complete; use the
[current roadmap](PORTING_PLAN.md) and [progress notes](PROGRESS.md) for active
work and acceptance status.

## Source provenance
- `directory.rs` extracted from `pskeffington/pip_boy` path `crates/pipboy-core/src/radio_directory.rs`, blob `429a065440238dffd0a81fd0ff706d80f8201333`.
- `stream.rs` extracted from `pskeffington/pip_boy` path `crates/pipboy-core/src/radio_stream.rs`, blob `868e9fc2d7d9f2b77bdf50d70961330e8ac68ea4`.
- Only local module path changed in stream imports. The upstream tests are included.

## Current ownership

The catalog and parser live in `rust-radio-core`; IP transport and native audio
live in their own crates behind the standalone facade. Pip-Boy compatibility
imports re-export the shared implementation and preserve station/favorites IDs.
Network-capable `PublisherTcpCheck` and `station_tcp_reachable` remain explicit
operations, never invoked on construction.

Local workspace gates and Mac native playback have passed after extraction.
Remote CI startup, live cancellation/address-policy review and physical Bluetooth
parity remain separate acceptance items; source import alone did not prove them.
