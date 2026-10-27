# R8 — Standalone Rust Radio micro-GUI and embedded panel

## Goal
A standalone desktop GUI using exactly the same Rust `RadioPanel` instance API that another Rust app can mount. No browser, external media player, local web server, or new audio backend. Display target: **480 × 320 logical pixels**. An embedded host chooses the allocated region and owns the top-level event loop.

## Launch on Mac
```sh
cargo run -p rust-radio-gui --bin rust-radio-gui
```
The panel does not start playback at launch. Station browse is offline. Tap PLAY to open approved native IP radio. STOP requests cancellation without joining on the UI thread. Queued PCM is muted immediately. STOPPING remains visible until worker cleanup finishes; PLAY and output changes remain locked during cleanup. Drag volume while playing. OUTPUT opens a selectable list of OS audio devices; stop first to change outputs.

## Embed in an egui host
Add the package (pinned to an approved commit for repeatability):
```toml
rust-radio-gui = { git = "https://github.com/pskeffington/rust_radio", rev = "<reviewed-commit>" }
```
In a struct owned by the host GUI, create `radio: rust_radio_gui::RadioPanel::new()`. Inside the host's existing `egui::Ui`, call `self.radio.show(ui)`. The host controls mount/unmount, navigation, and lifetime. Drop synchronously stops and joins the worker. To avoid blocking the host UI on dismount, call `radio.stop()`, retain the panel, and continue calling `radio.poll_shutdown()` until it returns true before dropping it. `is_active()` stays true during cleanup.

The current Rust-Boy desktop HUD is a same-origin HTML/JavaScript interface, **not** an egui host. Thus direct `show(ui)` embedding requires an egui shell; the existing HTML HUD needs a bridge or a separate native pane. Do not claim drop-in compatibility with the HTML screen until that adapter is built and validated. The playback/audio crates can still be reused independently.

## Acceptance and constraints
- 480 × 320 is a logical-pixel target; high-DPI and physical touch screenshots require verification.
- No intentional scroll in default view. OUTPUT now opens a centered fixed-size 380 × 165 logical-pixel overlay rather than extending the panel. Confirm the physical 480 × 320 viewport has no clipping on macOS and Pi.
- No direct station URL input. Only curated stations.
- Output selection is disabled while the worker still owns receiver resources, including failure cleanup. Completed failed sessions release automatically; failure details remain visible and PLAY requests a fresh session.
- Keep firmware/Pi builds optional: eframe's graphics/window stack and target platform must be validated separately; the audio engine is independent.
- Formatting, compile, native GPU rendering and physical mouse/touch acceptance require user Mac test after commit.

## Next operator Mac acceptance

```sh
cd ~/GitHub/rust_radio
git pull --ff-only origin main
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p rust-radio-gui --bin rust-radio-gui
```

Check the radio window at 480 × 320: station arrows, play/stop, volume, error text, and OUTPUT overlay must be fully tappable without body scrolling. Verify that simply opening the GUI emits no audio or radio HTTP traffic. PLAY should produce audible Skylab reception, and STOP should terminate it. Device changes require stopping first. These GUI acceptance steps are pending operator testing; the prior CLI macOS acceptance does not certify the GUI.

## Stop responsiveness regression

The offline playback test holds a synthetic worker behind a channel while requesting Stop. It checks cancellation, immediate mute, stale-event rejection, idempotent Stop, and nonblocking completion polling; a volume update after Stop must not unmute queued PCM. This test opens no audio device or network connection.

On macOS, stop during connect, buffering, live playback and reconnection. Confirm the window continues repainting while STOPPING is shown and that PLAY/output selection cannot create a second session. Hardware timing and GUI rendering still require operator verification.

## Completed and failed workers

`poll_shutdown()` also reaps workers that finish without an explicit Stop. The panel retains the final failure snapshot after releasing the engine, so FAILED and its error remain visible while station/output selection and PLAY become available. STOP clears a retained failure. No retry is started by polling or changing an output. Long status/error text is truncated to one line to preserve the micro-panel height; hovering reveals the full message.

`NativeRadioEngine::try_finish()` converts a panic or an otherwise unexplained worker exit into a failed snapshot. It preserves existing device/stream errors and never overwrites an explicit Stop with a cleanup failure. Hosts using the engine directly should poll this method to observe terminal worker exits.

Offline regression coverage now includes unexpected normal exit, worker panic, repeated completion polling, preserved failure details, and Stop taking priority over a worker panic. GUI/native audio acceptance remains pending for this pass.
