# Standalone radio playback acceptance

The `rust-radio-cli` crate is a directly runnable host for the extracted radio engine. It is not a replacement for the existing Pip-Boy HUD.

## Commands

```sh
cargo run -p rust-radio-cli -- stations
cargo run -p rust-radio-cli -- outputs
cargo run -p rust-radio-cli -- validate skylab
cargo run -p rust-radio-cli -- listen skylab --seconds 30
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

`stations` enumerates the local curated catalog without networking. `outputs` asks the operating system for its currently visible audio devices but does not pair or start one. `listen` requests a network stream and audible output; it validates the station and 1–300-second duration before engine startup. `validate STATION` explicitly checks an approved stream without opening an audio device. It prints the completion timestamp (Unix seconds), outcome, furthest verified stage, resolved endpoint and decoded format. Exit status is 0 only for verified PCM, otherwise 2.

Validation outcomes distinguish unverified catalog records, unavailable endpoints, invalid stream responses, unsupported formats/transports and decoded audio. The stage distinguishes endpoint reachability, an accepted audio response, a supported codec and actual PCM decoding. Unrecognized or truncated headers may report Unsupported because no format reader can identify them. A decoded result is a point-in-time codec check, not proof of audible playback or continued availability. Reports are currently returned through the Rust API or printed by the CLI, not persisted.

The validator reuses the publisher allowlist, redirect/playlist limits and ICY reader. One 18-second HTTP deadline covers the complete request chain; audio reads are capped at 256 KiB and 64 packets. Synthetic MP3/AAC fixtures exercise this path in offline CI. Live checks are separate and require network access; the synchronous Rust API belongs on a host worker thread.

## Acceptance record

On a Mac with actual audio output, record OS/audio device, UTC timestamp, commit SHA, station, output selection, playback status and errors, start/stop responsiveness, 30-second completed sample playback, and stream permissions. Then test invalid station/duration (no I/O), disconnected Bluetooth, interrupted HTTP, and stop during buffering. On Raspberry Pi, separately measure CPU, memory and power.

## Current evidence

Offline tests cover stream parsing, recovery and silent MP3/AAC validation. Audible playback, GUI interaction and device-loss behavior still require tests on the target hardware; the earlier operator-confirmed Mac Skylab pass does not certify later commits.

On 2026-10-27, explicit live `validate` checks decoded PCM from all ten curated stations without opening audio output. Skylab returned MP3 at 48 kHz stereo; Groove Salad, Secret Agent, Lush, Drone Zone, Deep Space One, Space Station Soma, Beat Blender, DEF CON Radio and Underground 80s returned MP3 at 44.1 kHz stereo. SomaFM required upgrading legacy HTTP playlist entries to their allowlisted HTTPS equivalents. These results establish point-in-time decoding only; rerun checks to establish current availability.


## Reliability pass: manual output and session control

After the prior Mac 30-second Skylab audible pass, the CLI now supports:

```sh
cargo run -p rust-radio-cli -- listen skylab --seconds 30 --volume 25
cargo run -p rust-radio-cli -- outputs
cargo run -p rust-radio-cli -- listen skylab --seconds 30 --output system
cargo run -p rust-radio-cli -- listen skylab --volume 50
```

For a named `--output`, copy an ID from the most recent `outputs` listing. Index/name-bound identifiers can change when devices reconnect and must be refreshed. Interactive mode accepts `status`, `volume N`, `stop` or `q`. Output selection is made at start; mid-session output hot-switching is a separate gate.

These CLI additions have **not** been re-tested on hardware in this pass. The earlier operator-confirmed 30-second Skylab pass applies to the prior CLI implementation. Run formatting, tests, Clippy and playback regression on the exact new commit before release. Test stop during connecting and buffering, and verify that all worker resources close. In interactive mode the stdin thread is detached and is not itself a hardware resource; graceful cancellation of a pending stdin read is still a future cleanup.
