# Rust Radio Mac operator checklist

Use this checklist on the intended macOS host after the local workspace gates
pass. It records interactive/native evidence; offline egui tests do not replace
these checks. Physical touch, Pi rendering and receiver hardware remain separate
acceptance gates.

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
| Launch and idle | Window opens at 480 × 320; no station playback or receiver session starts automatically. | |
| Station selection | Previous/next controls wrap through the catalog; the selected row and category remain legible. | |
| Keyboard navigation | Tab focuses the selector; Enter opens it; arrows/Home/End move the menu cursor; Enter commits; Escape closes without selecting. | |
| Long station names | Long entries scroll in the popup or show a useful tooltip; controls remain reachable. | |
| Silent station check | CHECK STREAM updates status/details without opening audible output or changing the selected station. | |
| Stale check cancellation | Change stations while a check is pending; an older completion cannot replace the new station's status. | |
| Play and status | Explicit PLAY reaches Connecting/Buffering and then Playing only after callback-consumed PCM; no cached validation badge is presented as live playback. | |
| Volume and mute | Slider changes level; MUTE silences promptly; UNMUTE restores the previous nonzero level. | |
| Station switch during playback | The prior session stops and only the latest selected station starts after cleanup; the window continues repainting. | |
| Stop | STOP silences queued PCM, shows Stopping while cleanup runs, and returns to a retryable idle state. | |
| Failure and Retry | Force or wait for a bounded stream/output failure; the message remains inside the viewport and RETRY is visible and actionable. | |
| Output selection | Output changes are unavailable while a worker owns the device; after cleanup, a selected output can be chosen without autoplay. | |
| Input setup panels | IP Radio, nSDR and USB setup overlays stay within the viewport and preserve explicit source choices. | |
| Audible listening | With the intended output selected and volume deliberately raised, confirm the station is audible; record the station and duration. | |

## Automated coverage reference

The workspace GUI suite includes offline regressions for no-autoplay
construction, selection and stale-check cancellation, long diagnostics and
viewport fit, full station-bank layout, keyboard selection semantics, mute
restoration, failure/retry state and saved-stream handling. Relevant tests are
in `crates/rust-radio-gui/src/tests.rs`, including
`station_status_and_long_diagnostics_fit_the_micro_viewport`,
`full_ip_station_bank_and_custom_setup_fit_the_small_viewport`,
`ip_menu_keyboard_navigation_keeps_selection_until_enter_and_scrolls_long_names`,
`mute_restores_the_last_listening_volume_without_starting_audio`, and
`stream_errors_offer_listener_actions_with_original_diagnostics_retained`.
These tests verify state and geometry, not physical controls or audible output.
