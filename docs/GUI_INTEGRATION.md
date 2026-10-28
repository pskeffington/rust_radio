# R8 — Standalone Rust Radio micro-GUI and embedded panel

## Goal
A standalone desktop GUI using exactly the same Rust `RadioPanel` instance API that another Rust app can mount. No browser, external media player, local web server, or new audio backend. Display target: **480 × 320 logical pixels**. An embedded host chooses the allocated region and owns the top-level event loop.

Source is split into `lib.rs` (panel lifecycle), `view.rs` (main controls), `settings.rs` (overlay), and `tests.rs`. Selection uses a direct catalog index while playback, validation and cached reports retain canonical station IDs. Catalog display order, including Skylab first, is preserved.

## Listening interface

The station selector fills the available width between previous/next controls;
station rows pair each curated station name with its category; `CLASSICAL` is
shortened to `CLASS` in the row, while the full genre remains on hover. The line below
the selected station contains genre only. Validation history
is kept out of this listening surface: a stale or missing silent check cannot
contradict actual Playing. The single primary button becomes Play, Stop or Retry
as appropriate. A separate listening label shows Ready to listen, Checking
station, Connecting, Buffering, Playing, Reconnecting, Stopping or Unable to play.
Queued station switches remain cancelable with Stop.

The volume slider uses the available width, rather than egui's default short
slider. Mute/Unmute preserves the last nonzero volume. While playing, the footer
identifies the audio output; otherwise it gives a short next-action hint. Errors
wrap/scroll within the remaining viewport height and keep Retry visible. Popup
and output settings retain the locally scoped green theme, with softer scanlines
and quieter secondary text. IP Radio setup contains the output selector,
stream editor, Save/Remove, check action and collapsible Connection details.
Live audio takes priority over historical silent-check results there too.

Construction, embedding, station storage and the playback engine are unchanged.
UI tests cover each playback state, stale validation during Playing, absence of
decoder diagnostics on the listening surface, volume restoration and long error
messages at 480×320. The station popup rows use up to 340 pixels of available
width. A measured 14-point monospace label A/B/C with 20 pixels total horizontal
button padding originally found full category labels fit 24/25 stations, names
alone 25/25, and compact category labels 25/25 at 295 pixels; all fit at 340
pixels. After the 2026-10-28 rights review first reduced the directory to five,
the regression reported A 4/5, B 5/5 and C 5/5 at 295 pixels. With twelve
publisher-supported stations, it reports A 11/12, B 12/12 and C 12/12 at 295
pixels; all fit at 340 pixels. Shortening `CLASSICAL` to `CLASS` preserves the
category cue and saves 128 aggregate text pixels across four classical
stations. With the current nineteen stations, full category labels fit 18/19;
names-only and compact category labels fit 19/19 at 295 pixels. All three
variants fit at 340 pixels. Compact categories save 128 aggregate text pixels
versus full categories. These are layout fit measurements, not listener
preference evidence.
Native
muted acceptance checks
Receiving audio now against actual playback, including switching/failure
responsiveness. Physical touch and Pi rendering still need acceptance.

## Launch on Mac
```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
```
The panel uses a green phosphor, monospaced skin with subtle scanlines. It does
not start playback at launch. Selecting an IP station queues a debounced silent
check of its actual HTTP audio endpoint. Tap PLAY to start native playback. STOP requests cancellation without joining on the UI
thread. Queued PCM is muted immediately. STOPPING remains visible until worker
cleanup finishes; PLAY and output changes remain locked during playback cleanup.
Drag volume while playing. The top input buttons open their setup panels; IP
Radio includes URL/station storage, silent checks and output selection. Stop
before changing outputs. Station selection stays enabled during IP playback.

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
- Curated stations plus up to 24 saved named public HTTPS or private IPv4 HTTP streams; custom URL/name entry is behind IP Radio. M3U/PLS and checked redirects are supported; HLS is reported as unsupported.
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

Check the radio window at 480 × 320: station arrows, play/stop, volume, error text, and input setup panels must be fully tappable without body scrolling. Opening the standalone GUI may perform read-only discovery, but must emit no audio or start a receiver or station stream. CHECK STREAM should update setup details without audible output. Cancel during connection, browse to another station, and confirm the old result cannot replace its status. PLAY should produce audible Skylab reception, and STOP should terminate it. Device changes require stopping first. These native GUI acceptance steps are pending operator testing; the prior CLI macOS acceptance does not certify the GUI.

## Stop responsiveness regression

The offline playback test holds a synthetic worker behind a channel while requesting Stop. It checks cancellation, immediate mute, stale-event rejection, idempotent Stop, and nonblocking completion polling; a volume update after Stop must not unmute queued PCM. This test opens no audio device or network connection.

On macOS, stop during connect, buffering, live playback and reconnection. Confirm the window continues repainting while STOPPING is shown and that PLAY/output selection cannot create a second session. Hardware timing and GUI rendering still require operator verification.

## Completed and failed workers

`poll_shutdown()` also reaps workers that finish without an explicit Stop. The panel retains the final failure snapshot after releasing the engine, so Unable to play and its error remain visible while station/output selection and Retry become available. RETRY explicitly restarts a failed target. Switching an active IP station queues only the latest replacement after cleanup; changing outputs never starts a session. Status/error text wraps in a bounded scrolling area, with a visible failure marker and consistent phosphor colors.

`NativeRadioEngine::try_finish()` converts a panic or an otherwise unexplained worker exit into a failed snapshot. It preserves existing device/stream errors and never overwrites an explicit Stop with a cleanup failure. Hosts using the engine directly should poll this method to observe terminal worker exits.

Offline regression coverage now includes unexpected normal exit, worker panic, repeated completion polling, preserved failure details, and Stop taking priority over a worker panic. GUI/native audio acceptance remains pending for this pass.

## Background station validation

Station selection while stopped schedules one silent check after 250 ms. It
uses `StationValidationTask::start_input` and the actual GET/redirect/playlist/
decode path used for playback. There is no ICMP gate, internet-availability gate,
or audio output enumeration. Public and private URLs use the same validation.
The shared parser caps M3U/PLS at 16 KiB/16 entries, rejects nested playlists and
HLS, validates every URL and limits resolution to five requests. Headers time out
at eight seconds, reads at three seconds; first decoded audio has a 35-second
budget. Silent checks additionally cap audio at 256 KiB/64 packets. Curated
reports remain cached for the panel's lifetime; custom reports omit URLs.

Station changes cancel obsolete checks. Play cancels a check and queues playback
until its worker releases. Stop clears all pending starts/checks. Active station
changes mute and cancel the old engine immediately; the GUI polls `try_finish`
before opening the latest selection, preventing overlapping outputs. Connecting,
Buffering, Playing, Recovering and Failed reflect the engine; Playing requires
actual decoded PCM consumption by the native output callback. Silent success
never reports Playing. Failed responses retain their detail and expose RETRY.

The station menu indicates the current selection, truncates long names with
full-name tooltips, and scrolls vertically. Tab focuses the selector, Enter opens
it, arrows and Home/End move the menu cursor, Enter commits, and Escape closes.
Left/Right on the closed focused selector step stations. The phosphor theme is
applied locally to controls, popup surfaces, selection and error text.

IP Radio setup offers Save/Remove for named custom URLs. Explicit Save persists
URLs, including query parameters; unsaved drafts remain in memory. During
playback, edit the URL and choose APPLY STREAM to reconnect. See
[storage and host APIs](EMBEDDING.md#ip-station-storage-and-selection).

Offline tests hold a fixture transport behind a channel while rendering, stopping and changing stations, then verify cleanup and cache identity. They also cover worker panic and the 480 × 320 layout. Receiver cancellation returns a terminal I/O error: `Interrupted` would be retried by `read_exact` and could loop forever during ICY metadata. A regression test exercises Stop at that exact boundary.

## Input buttons and discovery

The header uses **IP Radio | nSDR | USB**. Available inputs glow green;
unavailable/unknown inputs are dim. The selected input is underlined. Clicking
an input opens its own setup panel; the main view has selection, Play/Stop,
volume and a short status message. USB setup uses discovered device names.

Standalone startup opts into discovery, with idle refreshes every 30 seconds.
Scanning never starts RF or opens audio output. The first scan prefers a ready
network receiver, then USB, then IP. Explicit source choices are preserved even when discovery cannot verify them, and active playback is never switched by discovery. SCAN INPUTS is
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

## Native FM tuning panels

The direct nSDR setup uses Tune / FM Audio / Presets / Device tabs to stay within
480 × 320. Tune supports Hz, kHz and MHz with exact integer conversion, step
selection and live arrows; the main nSDR arrows use the same step. FM Audio
contains stereo, de-emphasis, filters and limiter plus receiver-reported PLL lock.
A settings-sent indicator distinguishes queued local edits from the last sent
choices. Presets recall full FM settings, and the standalone host saves them
locally on a dedicated file worker. Embedded panel users own import/export.
No preset read or stopped-state recall starts reception. Tests cover all tabs,
long preset names, full banks, invalid files and frequency-step boundaries.


## IP GUI acceptance (2026-10-28)

Native muted acceptance exercised HTTP MP3 and AAC through relative redirect +
PLS, active station switches, rapid selection changes, stalled headers, invalid
HTML/unsupported WAV, explicit Retry, refused connections and clean Stop. GUI
render calls remained below the 250 ms regression threshold throughout. Local
transport fixtures independently cover M3U, oversized/unsafe/nested playlists,
HLS rejection, HTTP 404/503, hop loops, timeout and cancellation. Headless GUI
tests cover long 24-entry banks, keyboard selection/closing and 480×320 layouts.
WCRB and Skylab live streams reached Playing through native output at volume 0.
The main panel, station popup and IP editor were visually inspected on macOS.
No external LAN broadcaster was supplied; private-stream playback was verified
with a local HTTP fixture. Physical touch, Pi graphics, audible quality and
extended network interruption tests remain open.
