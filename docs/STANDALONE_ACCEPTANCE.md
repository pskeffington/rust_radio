# Standalone radio playback acceptance

The `rust-radio-cli` crate is a directly runnable host for the extracted radio engine. It is not a replacement for the existing Pip-Boy HUD.

## Commands

```sh
cargo run -p rust-radio-cli -- stations
cargo run -p rust-radio-cli -- outputs
cargo run -p rust-radio-cli -- listen skylab --seconds 30
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

`stations` enumerates the local curated catalog without networking. `outputs` asks the operating system for its currently visible audio devices but does not pair or start one. `listen` is the only command requesting a network stream and audible output; it validates the station and 1–300-second duration before engine startup.

## Acceptance record

On a Mac with actual audio output, record OS/audio device, UTC timestamp, commit SHA, station, output selection, playback status and errors, start/stop responsiveness, 30-second completed sample playback, and stream permissions. Then test invalid station/duration (no I/O), disconnected Bluetooth, interrupted HTTP, and stop during buffering. On Raspberry Pi, separately measure CPU, memory and power.

## Current evidence

The CLI source and Cargo workspace registration are committed, but this environment does not have Cargo and cannot run the native audio tests. The connected GitHub Actions lookup for the previous source commit returned no matching PR-triggered runs; this does not prove that push-triggered workflows are absent. No audible standalone playback is claimed.


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
