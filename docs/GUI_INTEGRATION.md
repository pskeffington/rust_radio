# R8 — Standalone Rust Radio micro-GUI and embedded panel

## Goal
A standalone desktop GUI using exactly the same Rust `RadioPanel` instance API that another Rust app can mount. No browser, external media player, local web server, or new audio backend. Display target: **480 × 320 logical pixels**. An embedded host chooses the allocated region and owns the top-level event loop.

Source is split into `lib.rs` (panel lifecycle), `view.rs` (main controls), `settings.rs` (overlay), and `tests.rs`. Selection uses a direct catalog index while playback, validation and cached reports retain canonical station IDs. Catalog display order, including Skylab first, is preserved.

## Launch on Mac
```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
```
The panel does not start playback at launch. Station browse is offline. Tap PLAY to open approved native IP radio. STOP requests cancellation without joining on the UI thread. Queued PCM is muted immediately. STOPPING remains visible until worker cleanup finishes; PLAY and output changes remain locked during playback cleanup. Drag volume while playing. SETTINGS opens output selection and silent station validation; stop playback before changing outputs or checking a station.

## Embed in an egui host
Add the package (pinned to an approved commit for repeatability):
```toml
rust-radio-gui = { git = "https://github.com/pskeffington/rust_radio", rev = "<reviewed-commit>" }
```
In a struct owned by the host GUI, create `radio: rust_radio_gui::RadioPanel::new()`. Inside the host's existing `egui::Ui`, call `self.radio.show(ui)`. The host controls mount/unmount, navigation, and lifetime. Drop synchronously stops and joins owned workers. To avoid blocking the host UI on dismount, call `radio.stop()`, retain the panel, and continue calling `radio.poll_shutdown()` until it returns true before dropping it. `is_active()` includes both playback and station validation and stays true during cleanup.

Pip-Boy's HTML/JavaScript HUD already embeds the headless `rust-radio` library
through thin adapters and a pinned Git dependency. It owns its presentation,
preferences and optional nRSP-ST selector. The standalone egui panel uses the
same native IP engine and remains an optional interface for egui hosts; its
current station controls are IP-only. See [host integration](EMBEDDING.md).

## Acceptance and constraints
- 480 × 320 is a logical-pixel target; high-DPI and physical touch screenshots require verification.
- No intentional scroll in default view. SETTINGS opens a centered fixed-size 380 × 230 logical-pixel overlay. Offline egui layout tests cover the 480 × 320 bounds, including long diagnostics; physical macOS/Pi rendering still requires verification.
- No direct station URL input. Only curated stations.
- Output selection is disabled while the worker still owns receiver resources, including failure cleanup. Completed failed sessions release automatically; failure details remain visible and PLAY requests a fresh session.
- Keep firmware/Pi builds optional: eframe's graphics/window stack and target platform must be validated separately; the audio engine is independent.
- Native GPU rendering and physical mouse/touch acceptance require a Mac/Pi test after commit.

## Next operator Mac acceptance

```sh
cd ~/GitHub/rust_radio
git pull --ff-only origin main
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p rust-radio-gui --bin rust-radio-gui
```

Check the radio window at 480 × 320: station arrows, play/stop, volume, error text, and SETTINGS overlay must be fully tappable without body scrolling. Verify that simply opening the GUI emits no audio or radio HTTP traffic. CHECK STATION should update its badge and details without audible output. Cancel during connection, browse to another station, and confirm the old result cannot replace its status. PLAY should produce audible Skylab reception, and STOP should terminate it. Device changes require stopping first. These native GUI acceptance steps are pending operator testing; the prior CLI macOS acceptance does not certify the GUI.

## Stop responsiveness regression

The offline playback test holds a synthetic worker behind a channel while requesting Stop. It checks cancellation, immediate mute, stale-event rejection, idempotent Stop, and nonblocking completion polling; a volume update after Stop must not unmute queued PCM. This test opens no audio device or network connection.

On macOS, stop during connect, buffering, live playback and reconnection. Confirm the window continues repainting while STOPPING is shown and that PLAY/output selection cannot create a second session. Hardware timing and GUI rendering still require operator verification.

## Completed and failed workers

`poll_shutdown()` also reaps workers that finish without an explicit Stop. The panel retains the final failure snapshot after releasing the engine, so FAILED and its error remain visible while station/output selection and PLAY become available. STOP clears a retained failure. No retry is started by polling or changing an output. Long status/error text is truncated to one line to preserve the micro-panel height; hovering reveals the full message.

`NativeRadioEngine::try_finish()` converts a panic or an otherwise unexplained worker exit into a failed snapshot. It preserves existing device/stream errors and never overwrites an explicit Stop with a cleanup failure. Hosts using the engine directly should poll this method to observe terminal worker exits.

Offline regression coverage now includes unexpected normal exit, worker panic, repeated completion polling, preserved failure details, and Stop taking priority over a worker panic. GUI/native audio acceptance remains pending for this pass.

## Background station validation

SETTINGS → CHECK STATION starts one `StationValidationTask`, using the same bounded MP3/AAC validation path as the CLI. Per-station labels show UNVERIFIED, CHECKING, CANCELLING, DECODED, UNAVAILABLE, UNSUPPORTED or INVALID STREAM. Settings retains the last report's detail, check age, codec, rate and channel count. DECODED proves a point-in-time PCM decode, not audible output or future availability. Reports remain in memory for this panel's lifetime.

PLAY and another check are blocked while a check owns resources. Station browsing, CANCEL CHECK and STOP request cancellation; the panel retains and polls the worker until it finishes. Cancellation checks bracket network reads and prevent another request hop. A blocking request/read may need the remaining shared 18-second deadline, so the panel can show CANCELLING while remaining responsive. Cancelled results never overwrite previous evidence, and results for a different station are rejected. Closing Settings does not cancel a check.

Offline tests hold a fixture transport behind a channel while rendering, stopping and changing stations, then verify cleanup and cache identity. They also cover worker panic and the 480 × 320 layout. Receiver cancellation returns a terminal I/O error: `Interrupted` would be retried by `read_exact` and could loop forever during ICY metadata. A regression test exercises Stop at that exact boundary.
