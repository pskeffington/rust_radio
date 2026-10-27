# rust_radio

Portable, **receive-only**, Rust radio subsystem designed to plug into [Rust-Boy / pip_boy](https://github.com/pskeffington/pip_boy) without coupling its engine to the HUD. Intended backends: curated HTTPS internet streams, optional USB SDR receivers and future satellite receive adapters.

## Status (2026-10-26)

**Initial scaffold only.** The existing native Mac-demonstrated player remains in `pip_boy`; it has **not** yet been ported or independently validated in this repository. This repo currently supplies a source-neutral radio control contract, offline unit tests, integration/porting plan and research matrix.

## Navigation

- [Literature and architecture matrix](docs/LITERATURE_MATRIX.md)
- [Component ownership and extraction gates](docs/PORTING_PLAN.md)
- [Core Rust interface](crates/rust-radio-core/src/lib.rs)
- [Existing Rust-Boy radio engineering roadmap](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_NATIVE_ROADMAP.md)
- [Existing Rust-Boy SDR contract](https://github.com/pskeffington/pip_boy/blob/main/docs/RADIO_SDR.md)

## Develop

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

No network, sound card, USB SDR or proprietary system libraries are required for the default pure-control crate. Cargo commands are **instructions**, not test results from this authoring pass.

## Required integration invariants

1. No external media-app launching, web browser audio or autoplay.
2. Only explicit user Play opens network/hardware receiver resources.
3. One active session; generation-tagged state and real worker cancellation.
4. Audible `Playing` requires proof of samples reaching audio output; successful HTTP/SDR tuning alone is insufficient.
5. Bounded streaming memory, TLS and publisher endpoint validation, controlled redirects and timeouts.
6. Keep audio output devices, Bluetooth pairing and touch UI under OS/host ownership.
7. No RF transmit path. Satellite reception is research-stage only.
8. Any curated broadcaster inclusion requires a redistribution/embedding rights review before released bundles.

## Near-term work

Extract existing production radio components into independently testable crates with pinned upstream source revision, then switch Rust-Boy to a tested path dependency. Preserve Mac Skylab playback, explicit playback controls, Bluetooth output routing, favorites storage and 480x320 HUD behavior.
