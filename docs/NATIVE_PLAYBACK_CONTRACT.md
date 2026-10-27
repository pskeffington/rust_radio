# Native playback migration contract

Status: design gate only. Pip-Boy remains the only confirmed audible player.

## Worker ownership

- A single coordinator owns current session generation, network receiver, decoder worker, PCM queue and CPAL output.
- Starting a station is a deliberate operator command. Directory listing, favorites, startup, navigation and output enumeration are non-playing operations.
- Receiver worker resolves only `ApprovedEndpoint` values. The HTTP client must disable implicit redirects and proxies, validate TLS, impose connect and inactivity deadlines, and implement cancellation.
- The decoder produces bounded PCM frames; the CPAL callback consumes those frames without network requests, locks or per-callback allocation.
- PlaybackStarted may be emitted only after a live sink has actually consumed non-silent/nonempty decoded samples. TCP availability is not playback evidence.
- Stop must invalidate generation **and** cancel/join receiver, decoder and output worker resources; generation filtering alone does not release sockets or devices.
- Output-device changes are operator-driven. Bluetooth pairing remains OS-owned. Explicit failure must be surfaced instead of silently switching output.
- Recovery uses the existing bounded policy and must not restart a stopped generation.
- Receiver hardware support remains receive-only and behind optional feature flags.

## Port sequence

1. Verify pure `rust-radio-core` compilation, formatting, Clippy and offline fixtures in GitHub Actions.
2. Extract `radio_net.rs` into optional IP transport crate with preserved SSRF protections and streaming timeouts.
3. Extract `radio_playback.rs` and `radio_output.rs` into an audio crate with Symphonia/CPAL platform gating.
4. Prove short operator-triggered Skylab playback on Mac with volume, stop, and OS output selection.
5. Add an opt-in `rust_radio` host adapter in `pip_boy`; preserve existing UI and API response shape.
6. Switch the host only after parity and regression gates, then test Pi ARM64 separately.

## Current evidence and caveats

The remote GitHub files were written successfully; this environment cannot reach github.com via shell and does not have Cargo installed. The GitHub combined commit status query returned no reported statuses for the most recently checked revision; it is not evidence of green CI. No source-to-host cutover has been attempted.
