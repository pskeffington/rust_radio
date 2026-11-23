# Rust Radio Mac operator checklist

Use this checklist on the intended macOS host after the local workspace gates
pass. Record interactive/native evidence separately from offline egui tests.
Physical touch, Pi rendering, output-loss recovery and human-audible acceptance
are separate gates.

## Record the session

- Date/time and operator:
- Rust Radio commit:
- macOS version and architecture:
- Display resolution and GUI logical size (target: 480 × 320):
- Audio output selected:
- Stream/station used:
- Volume at start (begin muted; raise only for the explicit listening step):

Mark each row **Pass**, **Fail**, or **Not run** and add a short observation.

| Check | Expected result | Result / notes |
| --- | --- | --- |
| Launch and idle | Window opens at 480 × 320; no station starts automatically. | Not run |
| Station selection | Previous/next controls wrap through the catalog; selected name and category stay legible. | Not run |
| Keyboard navigation | Tab focuses the selector; Enter opens it; arrows/Home/End move the cursor; Enter commits; Escape closes without changing selection. | Not run |
| Long station names | Long entries remain identifiable and controls stay reachable. | Not run |
| Silent station check | CHECK STREAM updates status/details without starting audible output or changing selection. | Not run |
| Stale check cancellation | Changing stations during a check prevents an older result replacing the selected station's state. | Not run |
| Play and status | Explicit PLAY reaches Playing only after callback-consumed PCM; cached validation is not shown as live playback. | Not run |
| Volume and mute | Slider changes level; MUTE silences promptly; UNMUTE restores the previous nonzero level. | Not run |
| Station switch during playback | The old session stops and only the latest selected station starts after cleanup; the window remains responsive. | Not run |
| Stop | STOP silences queued PCM, shows Stopping during cleanup, and returns to a retryable idle state. | Not run |
| Failure and Retry | A bounded stream/output failure remains legible within the viewport and RETRY is visible and actionable. | Not run |
| Output selection | Output changes are unavailable while a worker owns the device; after cleanup, selection does not autoplay. | Not run |
| Audible listening | With the intended output selected and volume deliberately raised, confirm audibility and record station and duration. | Not run |

## Offline coverage reference

The GUI suite covers no-autoplay construction, selection and stale-check
cancellation, long diagnostics and viewport fit, the station bank, keyboard
selection semantics, mute restoration, failure/retry state and saved-stream
handling. Relevant tests in `crates/rust-radio-gui/src/tests.rs` include
`station_status_and_long_diagnostics_fit_the_micro_viewport`,
`full_ip_station_bank_and_custom_setup_fit_the_small_viewport`,
`ip_menu_keyboard_navigation_keeps_selection_until_enter_and_scrolls_long_names`,
`mute_restores_the_last_listening_volume_without_starting_audio`, and
`stream_errors_offer_listener_actions_with_original_diagnostics_retained`.
These tests verify state and geometry, not physical controls, output hardware or
human audibility. Fill the table only after a human runs each interactive check.
