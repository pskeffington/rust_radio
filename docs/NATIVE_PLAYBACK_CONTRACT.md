# Native playback migration contract

Status: IP-streaming implementation is consumed by Pip-Boy through a pinned Git dependency. Human-audible acceptance and native callback evidence are distinct; see dated records below. This project no longer contains RF receiver paths.

## Worker ownership

- A single coordinator owns current session generation, network receiver, decoder worker, PCM queue and CPAL output.
- A process-wide atomic session lease admits one playback worker across all `NativeRadioEngine` instances. A second start fails before output or network resources open; the lease releases when the worker exits or unwinds.
- Starting a station is a deliberate operator command. Directory listing, favorites, startup, navigation and output enumeration are non-playing operations.
- Receiver worker resolves only `ApprovedEndpoint` values. The HTTP client must disable implicit redirects and proxies, validate TLS, impose connect and inactivity deadlines, and implement cancellation.
- The decoder produces bounded PCM frames; the CPAL callback consumes those frames without network requests, locks or per-callback allocation.
- IP playback starts the paused output as soon as the prebuffer threshold is
  reached while delivering a decoded packet. A packet larger than the ring can
  therefore drain without falsely timing out on a full, still-paused output.
- PlaybackStarted may be emitted only after a live sink has actually consumed nonempty PCM. TCP availability is not playback evidence; local mute does not erase callback evidence.
- `NativeRadioEngine::status()` reconciles the callback's consumed-sample counter before returning a `Playing` snapshot for IP streams. This covers delayed callbacks while a stream awaits another packet/read; each output attempt gets a fresh counter so stale progress cannot advance recovery state.
- `NativeRadioEngine::underrun_callbacks()` reports session-lifetime callback buffers that needed zero-fill because the PCM ring ran empty. It accumulates across output retries, excludes empty buffers and queued zero-valued PCM, and is gated until output startup. It is diagnostic only: callback consumption and underrun counts do not establish audible output.
- Stop must invalidate generation **and** cancel/join receiver, decoder and output worker resources; generation filtering alone does not release sockets or devices.
- Output-device changes are operator-driven. Bluetooth pairing remains OS-owned. Explicit failure must be surfaced instead of silently switching output.
- Recovery uses the existing bounded policy and must not restart a stopped generation.
- RF hardware support is outside this project's scope.

## Completed port sequence

1. Verify pure `rust-radio-core` compilation, formatting, Clippy and offline fixtures in GitHub Actions.
2. Extract `radio_net.rs` into optional IP transport crate with preserved SSRF protections and streaming timeouts.
3. Extract `radio_playback.rs` and `radio_output.rs` into an audio crate with Symphonia/CPAL platform gating.
4. Prove short operator-triggered Skylab playback on Mac with volume, stop, and OS output selection.
5. Add an opt-in `rust_radio` host adapter in `pip_boy`; preserve existing UI and API response shape.
6. Switch the host only after parity and regression gates, then test Pi ARM64 separately.

Extraction and host cutover are complete. Physical Pi runtime and remote CI
success remain acceptance requirements in the [current roadmap](PORTING_PLAN.md).

## Current evidence and caveats

On 2026-10-27 Pip-Boy main integrated the standalone facade and thin compatibility adapters (commit `2a9ad3d0e14a03db8bffbb95541cae9266b32421`). The host retains the HTML HUD and preferences schema; it does not link the optional standalone GUI. All 328 host tests, formatting, check, strict Clippy and build passed locally. Browser regression fixtures passed at 480×320, 320×568 and 800×480.

A live loopback HUD run with temporary preferences exercised Skylab and Groove Salad. Both reached Playing after decoded samples were consumed by the native output callback. Volume, duplicate Play rejection, Stop-before-switch, favorites/selection and no autoplay on reads/output changes were checked. This run is not a new human listening confirmation, Bluetooth recovery test or Pi acceptance.

Earlier receiver implementation evidence has been retained in dated progress
records for historical reference. Current validation applies to IP streaming
only; see [progress notes](PROGRESS.md) and the [current roadmap](PORTING_PLAN.md).
