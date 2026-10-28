# R8 — Standalone Rust Radio micro-GUI and embedded panel

## Goal
A standalone desktop GUI using exactly the same Rust `RadioPanel` instance API that another Rust app can mount. No browser, external media player, local web server, or new audio backend. Display target: **480 × 320 logical pixels**. An embedded host chooses the allocated region and owns the top-level event loop.

Source is split into `lib.rs` (panel lifecycle), `view.rs` (main controls), `settings.rs` (overlay), and `tests.rs`. Selection uses a direct catalog index while playback, validation and cached reports retain canonical station IDs. Catalog display order, including Skylab first, is preserved.

## Launch on Mac
```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
```
The panel uses a green phosphor, monospaced skin with subtle scanlines. It does
not start playback at launch, and source browsing is offline. Tap PLAY to open an
approved native IP station. STOP requests cancellation without joining on the UI
thread. Queued PCM is muted immediately. STOPPING remains visible until worker
cleanup finishes; PLAY and output changes remain locked during playback cleanup.
Drag volume while playing. The top input buttons open their setup panels; IP
Radio includes silent station validation. Stop before changing outputs or
checking a station.

For direct nRSP-ST control and audio, use `--features nrsp-native`, click
**nSDR**, select a discovered receiver or enter `IP:9001`, and configure frequency,
mode and bandwidth. PLAY starts reception; APPLY TUNING changes it live.

For the optional SDRconnect bridge instead, enable:

```sh
cargo run -p rust-radio-gui --features sdrconnect --bin rust-radio-gui
```

Select **nRSP-ST / SDRconnect** in the source selector, then set SDRconnect's
private-LAN or loopback API `IP:port` behind the **nSDR** button. The default is
`127.0.0.1:5454`; the address is held in panel memory and is not saved. Refresh
and select the receiver in SDRconnect first. Explicit PLAY uses the shared
native engine, including its selected-receiver startup and timeout behavior;
STOP unsubscribes and leaves shared reception running. IP station validation is
unavailable for the SDR source. Switching sources while a check is running
cancels that check before another session can begin.

For the optional USB mono-FM source, launch with `--features soapy-sdr` and
click **USB**, run SCAN INPUTS and choose the detected device and frequency in
its setup panel. The GUI
uses 75 µs de-emphasis; the CLI also supports 50 µs. This path needs SoapySDR,
an installed device driver and a receiver supporting 240 ksample/s CS16. It has
synthetic coverage but has not yet been tested with a physical dongle.

## Embed in an egui host
Add the package (pinned to an approved commit for repeatability):
```toml
rust-radio-gui = { git = "https://github.com/pskeffington/rust_radio", rev = "<reviewed-commit>" }
```
In a struct owned by the host GUI, create `radio: rust_radio_gui::RadioPanel::new()`. Inside the host's existing `egui::Ui`, call `self.radio.show(ui)`. The host controls mount/unmount, navigation, and lifetime. Drop synchronously stops and joins owned workers. To avoid blocking the host UI on dismount, call `radio.stop()`, retain the panel, and continue calling `radio.poll_shutdown()` until it returns true before dropping it. `is_active()` includes both playback and station validation and stays true during cleanup.

Pip-Boy's HTML/JavaScript HUD already embeds the headless `rust-radio` library
through thin adapters and a pinned Git dependency. It owns its presentation,
preferences and optional nRSP-ST selector. The standalone egui panel uses the
same native engine and remains an optional interface for egui hosts. Enable both
facade features `gui` and `sdrconnect` to include the SDR source in the panel.
The panel applies its skin locally rather than changing the host's global egui
theme. See [host integration](EMBEDDING.md).

## Acceptance and constraints
- 480 × 320 is a logical-pixel target; high-DPI and physical touch screenshots require verification.
- No intentional scroll in default view. Each input button opens a centered setup overlay sized for the 480 × 320 viewport. Offline egui layout tests cover the 480 × 320 bounds, including long diagnostics; Pi rendering still requires verification.
- No direct station URL input. Only curated stations.
- Output selection is disabled while the worker still owns receiver resources, including failure cleanup. Completed failed sessions release automatically; failure details remain visible and PLAY requests a fresh session.
- Keep firmware/Pi builds optional: eframe's graphics/window stack and target platform must be validated separately; the audio engine is independent.
- Mac rendering and mouse controls were inspected; physical touch and Pi GPU acceptance remain open.

On 2026-10-27, the SDR-enabled panel and themed settings were visually inspected
at the Mac's 480×320 logical window size. A local synthetic SDRconnect receiver
reached Playing through the native output callback at zero volume, and STOP
completed cleanup. The real nRSP-ST reported started but its separate 500 ms
audio capture returned zero PCM pairs, so a fresh live SDR GUI audio check remains
open. This inspection does not establish physical touch or Pi acceptance.

## Next operator Mac acceptance

```sh
cd ~/GitHub/rust_radio
git pull --ff-only origin main
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p rust-radio-gui --bin rust-radio-gui
```

Check the radio window at 480 × 320: station arrows, play/stop, volume, error text, and input setup panels must be fully tappable without body scrolling. Opening the standalone GUI may perform read-only discovery, but must emit no audio or start a receiver or station stream. CHECK STATION should update its badge and details without audible output. Cancel during connection, browse to another station, and confirm the old result cannot replace its status. PLAY should produce audible Skylab reception, and STOP should terminate it. Device changes require stopping first. These native GUI acceptance steps are pending operator testing; the prior CLI macOS acceptance does not certify the GUI.

## Stop responsiveness regression

The offline playback test holds a synthetic worker behind a channel while requesting Stop. It checks cancellation, immediate mute, stale-event rejection, idempotent Stop, and nonblocking completion polling; a volume update after Stop must not unmute queued PCM. This test opens no audio device or network connection.

On macOS, stop during connect, buffering, live playback and reconnection. Confirm the window continues repainting while STOPPING is shown and that PLAY/output selection cannot create a second session. Hardware timing and GUI rendering still require operator verification.

## Completed and failed workers

`poll_shutdown()` also reaps workers that finish without an explicit Stop. The panel retains the final failure snapshot after releasing the engine, so FAILED and its error remain visible while station/output selection and PLAY become available. STOP clears a retained failure. No retry is started by polling or changing an output. Long status/error text is truncated to one line to preserve the micro-panel height; hovering reveals the full message.

`NativeRadioEngine::try_finish()` converts a panic or an otherwise unexplained worker exit into a failed snapshot. It preserves existing device/stream errors and never overwrites an explicit Stop with a cleanup failure. Hosts using the engine directly should poll this method to observe terminal worker exits.

Offline regression coverage now includes unexpected normal exit, worker panic, repeated completion polling, preserved failure details, and Stop taking priority over a worker panic. GUI/native audio acceptance remains pending for this pass.

## Background station validation

IP Radio → CHECK STATION starts one `StationValidationTask`, using the same bounded MP3/AAC validation path as the CLI. Per-station labels show UNVERIFIED, CHECKING, CANCELLING, DECODED, UNAVAILABLE, UNSUPPORTED or INVALID STREAM. Settings retains the last report's detail, check age, codec, rate and channel count. DECODED proves a point-in-time PCM decode, not audible output or future availability. Reports remain in memory for this panel's lifetime.

PLAY and another check are blocked while a check owns resources. Station browsing, CANCEL CHECK and STOP request cancellation; the panel retains and polls the worker until it finishes. Cancellation checks bracket network reads and prevent another request hop. A blocking request/read may need the remaining shared 18-second deadline, so the panel can show CANCELLING while remaining responsive. Cancelled results never overwrite previous evidence, and results for a different station are rejected. Closing Settings does not cancel a check.

Offline tests hold a fixture transport behind a channel while rendering, stopping and changing stations, then verify cleanup and cache identity. They also cover worker panic and the 480 × 320 layout. Receiver cancellation returns a terminal I/O error: `Interrupted` would be retried by `read_exact` and could loop forever during ICY metadata. A regression test exercises Stop at that exact boundary.

## Input buttons and discovery

The header uses **IP Radio | nSDR | USB**. Available inputs glow green;
unavailable/unknown inputs are dim. The selected input is underlined. Clicking
an input opens its own setup panel; the main view has selection, Play/Stop,
volume and a short status message. USB setup uses discovered device names.

Standalone startup opts into discovery, with idle refreshes every 30 seconds.
Scanning never starts RF or opens audio output. The first scan prefers a ready
network receiver, then USB, then IP. Explicit available source choices are
preserved, and active playback is never switched by discovery. SCAN INPUTS is
available in each setup panel. IP glow reflects a recent HTTPS reachability
check or current playback; station availability can still vary independently.

`RadioPanel::new()` remains offline. Embedded hosts can call `scan_inputs()`
explicitly or `enable_input_discovery()` to opt into the standalone behavior.
`is_active()` includes an in-flight scan; Stop cancels it, and `poll_shutdown()`
reaps/discards cancelled results. GUI rendering never waits on discovery.
The reusable worker is `rust_radio::discovery::InputScan`.

The `receivers` GUI feature enables both direct nRSP-ST and SoapySDR USB paths.
USB detection needs a supported installed driver; it does not validate RF or
open a receive stream. Native network scanning validates the Hello protocol on
port 9001, with 16 concurrent probes, up to 512 addresses and a ten-second LAN
deadline. It uses up to two local private /24-sized neighborhoods, excluding
network/broadcast/self; larger networks are capped to the local /24. A manually
entered private receiver endpoint is also checked. Enter an address manually
when the receiver is across a routed subnet or VPN beyond those neighborhoods.
