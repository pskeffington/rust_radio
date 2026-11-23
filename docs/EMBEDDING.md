# Standalone radio module and host integration

`rust-radio` is the public entry point. It is independently runnable through
`rust-radio-cli` and independently embeddable as a Rust library. A host does not
launch the standalone GUI, browser or another audio engine.

```toml
[dependencies]
rust-radio = { git = "https://github.com/pskeffington/rust_radio", rev = "<reviewed-commit>" }
```

Default features include the headless IP transport, MP3/AAC decoder and native
output. `default-features = false` gives pure control/catalog APIs. `gui` is
optional and adds `RadioPanel` for egui hosts. This crate handles internet radio
streams only; RF receivers and USB/nSDR hardware belong to their own projects.

```rust
use rust_radio::NativeRadioEngine;

fn play_after_user_command(station_id: &str) -> Result<(), String> {
    let mut engine = NativeRadioEngine::start_internet(station_id, 35, "system")?;
    let snapshot = engine.status();
    engine.set_volume(40)?;
    engine.request_stop();
    // In a GUI, retain the engine and poll from the host event loop instead.
    while !engine.try_finish() {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    Ok(())
}
```

Hosts retain ownership of their UI and preferences. The audio crate enforces one
process-wide playback worker, even if a host accidentally creates multiple
engine instances. A new start returns an error until the prior worker exits.
The standalone egui panel is not required by pip_boy's HTML HUD. Stable station IDs,
playback states and generation-tagged commands/events are shared through
`rust_radio::controller`; directory, parser, PCM and recovery APIs are also
available without loading audio dependencies. Host storage continues to use station IDs. Hosts can also explicitly call
`NativeRadioEngine::start_stream(url, volume, output)` for a direct MP3/AAC
URL; it is retained only for that engine session. Direct input accepts HTTPS
or private/loopback literal IPv4 HTTP, rejects non-public HTTPS literal-IP
targets, credentials and fragments, and
supports bounded redirects and M3U/PLS (including relative entries), with URL
validation on every hop and no HTTPS downgrade. Private HTTP redirects remain
on the selected private origin, and DNS answers remain public-only. HLS remains
unsupported.
Hosts should protect query tokens and ask for an explicit Play action before
starting audio. `StationValidationTask::start_input(target, direct)` checks the
same transport silently without opening audio output.

`NativeRadioEngine::output_failed()` remains available until the engine is
dropped, including after its worker finishes. It is true for audio-device
failures and false for network failures or canceled sessions. Hosts can use it
to explain output loss and offer device selection followed by explicit Play;
changing the selected output alone must not start playback.

Compatibility imports in `rust-radio-audio::compat_controller` refer to the
same core controller types. `rust-radio-core::SessionState` is the same playback
state type, so source selection and output-recovery policy cannot drift from the
native worker's state vocabulary.

## Input availability

`rust_radio::discovery::InputScan` runs an explicit read-only internet
reachability check on a background worker. Poll `try_finish()` for its status.
No audio output starts during discovery. `cancel()` discards
late results. For the embedded egui panel, use `scan_inputs()` explicitly or
opt into startup/idle scanning with `enable_input_discovery()`.

### IP station storage and selection

`RadioPanel::new()` and `load_ip_stations(json)` perform no network or file IO
and never autoplay. Operator station changes queue a debounced silent endpoint
check; Play supersedes it. During active IP playback, selection immediately
mutes/cancels the old engine, then polls cleanup before creating the latest
engine. Retain and render/poll the panel during this transition. Stop also clears
queued playback and validation. Cross-input/output changes still require Stop.

Hosts may read their optional `validate-all --history PATH` state file off the
UI thread and pass its contents to `RadioPanel::load_validation_history(json)`.
The panel validates the bounded schema and station IDs, recomputes the six-hour
freshness window, and labels imported evidence `SAVED / …`. It performs no file
or network IO, never autoplays, and never gates Play. A live station check takes
precedence over saved history; the active listening surface continues to report
current playback rather than historical check state.

Headless hosts can consume the same versioned snapshot with
`rust_radio::feed_history::parse_feed_history(json)`; a Rust-Boy adapter can
adopt this API when it updates its pinned module revision. The parser lives in
the core crate and does not require the optional egui feature. It enforces the
64 KiB schema and returns typed per-station status, stage, attempts, last
decoded time and failure count. Call
`FeedHistoryRecord::health_at(now)` when displaying the result so freshness is
recomputed instead of trusting a cached label. The host owns file I/O and any
presentation; saved evidence is informational and never gates playback.

Use `take_ip_stations_update() -> Option<String>` to obtain the versioned JSON
when saved stations or selection change, and persist it off the render thread.
Call `report_ip_stations_saved(result)` for write failures. Import validates the
whole bank before replacing it (24 entries, 64 KiB, bounded names and URLs);
imports during playback/checks are rejected. Unsaved URL edits remain in memory.
The standalone host uses an atomic background writer at
`rust_radio/ip-stations.json` in the platform configuration directory (macOS:
`~/Library/Application Support`); `RUST_RADIO_STATIONS_PATH` overrides the path.
Explicit Save persists query parameters too. No station credentials in URL
userinfo are accepted. Embedded Pip-Boy hosts keep storage/window ownership and
can instead use the headless engine with their own station picker.
