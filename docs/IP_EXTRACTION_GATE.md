# IP-radio extraction gate (2026-10-26)

## Source provenance
- `directory.rs` extracted from `pskeffington/pip_boy` path `crates/pipboy-core/src/radio_directory.rs`, blob `429a065440238dffd0a81fd0ff706d80f8201333`.
- `stream.rs` extracted from `pskeffington/pip_boy` path `crates/pipboy-core/src/radio_stream.rs`, blob `868e9fc2d7d9f2b77bdf50d70961330e8ac68ea4`.
- Only local module path changed in stream imports. The upstream tests are included.

## What this pass proves
The source files are committed and exported by the crate. No claim of independently passing CI, live endpoint health, audible output or host migration. Network-capable `PublisherTcpCheck` and `station_tcp_reachable` are explicit operations, never invoked on construction.

## Review items before host cutover
1. Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` on the actual current commit.
2. Reject/resolve any compiler or strict Clippy warnings (CI is authoritative).
3. Audit DNS rebinding and redirect behavior at the eventual transport boundary, verify proxies are disabled and timeout/cancellation enforced.
4. Preserve the previous station catalog and favorites identifiers; no silent preference migration.
5. Add publisher-consent review before public distribution (SomaFM personal-listening-only restrictions).
6. Copy HTTPS and audio workers into feature-gated crates only after this pure-core gate is green.
7. Test Mac Skylab PCM and Bluetooth output parity before switching `pip_boy` dependency.
