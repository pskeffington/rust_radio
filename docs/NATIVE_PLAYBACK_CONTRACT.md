# Native playback migration contract

Status: implemented in the standalone module and consumed by Pip-Boy through a pinned Git dependency. Human-audible acceptance and native callback evidence are distinct; see the dated records below.

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

On 2026-10-27 Pip-Boy main integrated the standalone facade and thin compatibility adapters (commit `2a9ad3d0e14a03db8bffbb95541cae9266b32421`). The host retains the HTML HUD and preferences schema; it does not link the optional standalone GUI. All 328 host tests, formatting, check, strict Clippy and build passed locally. Browser regression fixtures passed at 480×320, 320×568 and 800×480.

A live loopback HUD run with temporary preferences exercised Skylab and Groove Salad. Both reached Playing after decoded samples were consumed by the native output callback. Volume, duplicate Play rejection, Stop-before-switch, favorites/selection and no autoplay on reads/output changes were checked. This run is not a new human listening confirmation, Bluetooth recovery test or Pi acceptance.

The optional receive-only SoapySDR adapter compiles and links on this Mac; explicit discovery found no receivers. It provides I/Q ingress only, without AM/FM/SSB audio demodulation. Physical reception, unplug/replug and ARM64 remain open. GitHub Actions has returned startup failures on prior pushes; local green checks do not establish remote CI success.
