# Rust Radio

Rust Radio is a streaming radio app for curated and user-supplied internet stations. The app provides station browsing, explicit playback controls, local audio output selection, and bounded stream validation. It does not autoplay.

nSDR/network receivers, USB SDR, SDRconnect, SoapySDR, and satellite reception are separate products maintained in their own repositories. They are outside this repository's app, build, and release scope. Older receiver notes under `docs/` are historical records and do not define the current product.

## Current station catalog

The starter catalog contains two GBH streams: GBH 89.7 (`gbh897`) and WCRB Classical (`wcrb`). Inclusion in the catalog does not establish current feed availability or permission for third-party product distribution. Written publisher permission and fresh stream checks remain release gates. SiriusXM was removed from the roadmap because no supported development API was identified.

## Workspace

- `rust-radio-core`: station metadata, playback state, stream parsing, and validation types.
- `rust-radio-ip`: bounded internet stream and probe transports.
- `rust-radio-audio`: internet stream decoding and native audio output.
- `rust-radio-gui`: compact station browsing and playback interface.
- `rust-radio-cli`: operator-triggered catalog, validation, and stream-listen commands.
- `rust-radio`: host-facing facade.

## Build and run

```sh
cargo run -p rust-radio-gui
```

The CLI supports station listing, silent validation, output listing, and explicit stream listening. Run it without arguments for current usage. Workspace builds include CPAL and eframe and need the platform's audio and windowing development libraries.

## Product and release gates

See [the streaming roadmap](docs/PORTING_PLAN.md), [the task queue](docs/CODEX_TASK_QUEUE.md), [embedding](docs/EMBEDDING.md), and [native playback evidence](docs/NATIVE_PLAYBACK_CONTRACT.md). Silent validation, decoded audio, callback-consumed audio, human-audible playback, publisher authorization, and release approval are separate evidence gates.
