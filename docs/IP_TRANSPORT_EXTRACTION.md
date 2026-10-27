# Native IP receiver extraction (2026-10-26)

The optional `rust-radio-ip` workspace crate contains source adapted from:
- `pskeffington/pip_boy/crates/pipboy/src/radio_net.rs` source blob `77c4e619277797ad29080b9ab97130cf8d8dcac2`
- `pskeffington/pip_boy/crates/pipboy/src/radio_http.rs` source blob `4acd0d9a27835716676c1186553e335d72bb224b`

Adaptations: swap `pipboy_core` import paths for `rust_radio_core`, expose transport entrypoints at library boundary, change the worker thread name. The crate retains publisher-approved HTTPS only, explicitly disabled automatic redirects/proxies, bounded compressed-byte queues, per-read inactivity timeouts, and cancellation.

The short probe and sustained stream paths are intentionally separate. `probe::NativeRadioHttp::for_probe()` sets a total deadline; `net::open_station()` starts a long-lived bounded worker only when called by a host. The default `rust-radio-core` crate remains free of network dependencies.

Each connection has its own cancellation flag. Dropping its guard or byte reader cancels pending HTTP and queue work before joining the receiver, without stopping the user-authorized session needed for recovery. Pending async work is checked every 25 ms; existing connect and inactivity deadlines still apply. Offline regression tests cover stalled work, a full retained queue, reader/guard drop order, stale buffered bytes, fresh connections and session cancellation.

Hosts that need recovery policy can use `net::open_station_classified()` and `error::TransportError`; the original `open_station()` keeps its string-error API. Connection/header and read failures, HTTP 408/429 and 5xx responses are transient. Invalid endpoints, playlists, content and other HTTP rejections are permanent. Playback shares a three-retry budget across startup and in-stream failures, waiting 1, 2 and 4 seconds; Stop interrupts those waits. Stream I/O failures during decoder probing also use that budget, while unsupported formats and output-device failures remain terminal.

Core validation and native playback share M3U/PLS format detection and parsing. Both enforce 16 KiB/16-entry limits and validate every stream URL against the station's publisher allowlist. PLS requires a matching `NumberOfEntries`, contiguous `FileN` indices and `Version=2` when present; the transport selects the first entry in numeric order. Metadata is inert. HLS MIME types, `.m3u8` URLs and `#EXT-X-` manifest tags return an explicit unsupported-transport error without fetching segments. Playlist nesting remains disallowed.

SomaFM's official HTTPS playlists currently include legacy HTTP stream links. The parser upgrades these entries to HTTPS before allowlist validation, consistent with the publisher's documented TLS direct links. It preserves the original host, port and path, rejects unsafe variants and never falls back to plaintext. This compatibility rule is limited to SomaFM playlist entries; direct endpoints and redirects still require explicit HTTPS.

Live cancellation during DNS/TLS, device behavior, Pi hardware, dependency MSRV compatibility and source-host parity still require separate validation. These offline tests do not certify hardware playback.

Remaining validation:
1. Check dependency MSRV compatibility and adopt a reviewed application lockfile.
2. Exercise cancellation and startup expiry against live DNS/TLS and stalled servers.
3. Audit DNS/address-based SSRF safeguards at connection time; validating host strings alone does not ensure resolved IP safety.
4. Prove Skylab Mac audio, Stop, volume and OS-managed Bluetooth parity before replacing Pip-Boy's native implementation.
