# Standalone radio module and host integration

`rust-radio` is the public entry point. It is independently runnable through
`rust-radio-cli` and independently embeddable as a Rust library. A host does not
launch the standalone GUI, browser or another audio engine.

```toml
[dependencies]
rust-radio = { git = "https://github.com/pskeffington/rust_radio", rev = "<reviewed-commit>" }
```

Default features include the headless IP transport, MP3/AAC decoder and native
output. `default-features = false` gives only pure control/catalog APIs; `sdr`
adds the receive-only tuner contract and explicit simulator. `gui` is optional
and adds `RadioPanel` for egui hosts. Hardware reception is a separate adapter
and acceptance gate; simulated IQ is never evidence of received RF.

`soapy-sdr` adds the optional SDK-backed receive adapter through
`rust_radio::receiver`, including explicit discovery, device open and bounded
I/Q capture. See [SDR receive limits and acceptance](SDR_RECEIVE.md). This feature
requires the native SDK and Rust 1.85; it adds no dependency to the default IP
or pure-control builds.

With default audio plus `soapy-sdr`, `NativeRadioEngine::start_soapy_wfm()`
starts the initial 240 ksample/s mono broadcast-FM path on a selected device.
Enable `gui` and `soapy-sdr` together for its optional panel controls. Synthetic
checks have passed; physical USB audio is not yet accepted.

`sdrconnect` adds the SDK-independent network API for the selected SDRplay
nRSP-ST through `rust_radio::receiver::sdrconnect`. It is independent of
`soapy-sdr` and the GUI, and requires Rust 1.85 or newer. See
[nRSP-ST setup and current limits](NRSP_ST.md).

The lighter `sdr` feature also exports `receiver::nrsp::Host` for explicit,
read-only checks of the nRSP-ST's direct web and device services. This path has
no SDRconnect or USB SDK dependency and does not yet receive audio or I/Q.

Enable `gui` together with `sdrconnect` to include the nRSP-ST source and local
API address field in the optional `RadioPanel`. Its phosphor skin is scoped to
the panel and settings window, leaving the host's global egui style intact.
Selecting a source or editing its address does not start a connection.

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

Hosts retain ownership of their UI, preferences and one engine at a time. The
standalone egui panel is not required by pip_boy's HTML HUD. Stable station IDs,
playback states and generation-tagged commands/events are shared through
`rust_radio::controller`; directory, parser, PCM and recovery APIs are also
available without loading audio dependencies. Host storage continues to use station IDs. Hosts can also explicitly call
`NativeRadioEngine::start_stream(url, volume, output)` for a direct MP3/AAC
URL; it is retained only for that engine session. Direct input accepts HTTPS
or private/loopback literal IPv4 HTTP, rejects credentials and fragments, and
supports bounded redirects and M3U/PLS (including relative entries), with URL
validation on every hop and no HTTPS downgrade. HLS remains unsupported.
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

With default IP/audio features plus `sdrconnect`, the same `NativeRadioEngine`
also accepts `start_sdrconnect(peer, volume, output)`. Its status uses
`InputKind::Sdr`; `receiver_status()` exposes the last server-reported tuning
snapshot without network I/O. Stop/reaping and output ownership follow the same
contract as IP playback. A control-only `sdrconnect` dependency still links no
native audio or USB SDR SDK.

Explicit `start_sdrconnect()` playback starts the selected receiver if stopped
and SDRconnect grants control. It waits for confirmation before opening native
output. A three-second startup confirmation failure is terminal (`StartTimeout`),
so network recovery does not repeat activation; the host can offer explicit Play
again after setup is corrected. Transport failures retain bounded recovery.
Status and capture remain passive, and local Stop leaves shared reception running.

For the Pi target, the headless receiver dependency can be checked without audio
or SDK libraries:

```sh
rustup target add aarch64-unknown-linux-gnu
cargo check --locked -p rust-radio --no-default-features --features sdrconnect \
  --target aarch64-unknown-linux-gnu
```

This checks compilation only. See the [roadmap](PORTING_PLAN.md) and
[progress notes](PROGRESS.md) for remaining physical acceptance.

## Direct nRSP-ST feature

Enable `nrsp-native` to use the receiver directly over its own port 9001. The
headless `receiver::nrsp_native::Client` has no audio or vendor SDK dependency.
With default IP/audio features, `NativeRadioEngine::start_nrsp_native(peer,
frequency_hz, mode, bandwidth_hz, volume, output)` starts native Opus playback.
`tune_nrsp_native` queues the latest validated live tuning request in a bounded
slot; `nrsp_native_status` returns observed receiver state and
`nrsp_native_audio_stats` reports decoded frames and signal peak. Neither read
performs I/O. Keep one engine and continue calling `try_finish` after Stop.

Enable `gui` as well for the optional direct receiver controls in `RadioPanel`.
The host owns address/preferences; runtime session state and keys are not saved.
See [receiver setup and limits](NRSP_ST.md#direct-native-control-and-audio).

## Input availability

`rust_radio::discovery::InputScan` runs explicit read-only internet, network
receiver and supported USB descriptor discovery on background workers. Poll
`try_finish()` and use the returned availability/device lists to enable input
controls. No receiver or output starts during discovery. `cancel()` discards
late results. For the embedded egui panel, use `scan_inputs()` explicitly or
opt into startup/idle scanning with `enable_input_discovery()`.

## FM tuning and presets

With `nrsp-native`, the native engine also exposes
`start_nrsp_native_fm(peer, frequency_hz, bandwidth_hz, settings, volume, output)`
and `set_nrsp_native_fm(settings)`. `NativeFmSettings` contains stereo,
`NativeFmDeemphasis`, audio filter cutoffs/enables and limiter. Updates are
validated and queued on the existing worker. `nrsp_native_status()` distinguishes
last-sent FM choices from the optional receiver-reported `stereo_locked` value.

The panel exposes `load_fm_presets(json)`, `take_fm_presets_update()` and
`report_fm_presets_saved(result)`. Hosts should persist exported updates away
from the render/audio thread. Construction and preference import remain offline;
recall prepares tuning when stopped and queues it only for active playback.
The standalone binary provides its own bounded atomic file writer. Preset JSON
is versioned, at most 16 KiB / 16 entries, and contains tuning/audio choices only.


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
