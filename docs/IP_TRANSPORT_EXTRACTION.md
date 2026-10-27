# Native IP receiver extraction (2026-10-26)

The optional `rust-radio-ip` workspace crate contains source adapted from:
- `pskeffington/pip_boy/crates/pipboy/src/radio_net.rs` source blob `77c4e619277797ad29080b9ab97130cf8d8dcac2`
- `pskeffington/pip_boy/crates/pipboy/src/radio_http.rs` source blob `4acd0d9a27835716676c1186553e335d72bb224b`

Adaptations: swap `pipboy_core` import paths for `rust_radio_core`, expose transport entrypoints at library boundary, change the worker thread name. The crate retains publisher-approved HTTPS only, explicitly disabled automatic redirects/proxies, bounded compressed-byte queues, per-read inactivity timeouts, and cancellation.

The short probe and sustained stream paths are intentionally separate. `probe::NativeRadioHttp::for_probe()` sets a total deadline; `net::open_station()` starts a long-lived bounded worker only when called by a host. The default `rust-radio-core` crate remains free of network dependencies.

**Not yet established:** compilation, rustfmt, Clippy, CI, dependency MSRV compatibility, live stream playback, native sound output, Pi hardware behavior, or source-host parity. An attempted shell invocation returned no Cargo and could not resolve `github.com`.

Next mandatory steps:
1. Run `cargo fmt --all` and `cargo test --workspace`, then `cargo clippy --workspace --all-targets -- -D warnings` on a machine with Rust toolchain and registry access; commit lockfile.
2. Check Rust minimum supported version against pinned `reqwest` / `tokio` transitive dependencies; adjust the IP crate MSRV independently if necessary.
3. Audit cancellation/guard join behavior when startup deadline fires; ensure reader/session cancellation does not block indefinitely during join.
4. Audit DNS/address-based SSRF safeguards at connection time; validating host strings alone does not ensure resolved IP safety.
5. Extract CPAL/Symphonia player only after network compile and offline fixtures pass.
6. Prove Skylab Mac audio, Stop, volume and OS-managed Bluetooth parity before replacing Pip-Boy's native implementation.
