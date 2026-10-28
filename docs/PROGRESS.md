# Radio development progress

Updated 2026-10-27. Active priorities and ownership are in the
[roadmap](PORTING_PLAN.md); embedding commands are in [the module contract](EMBEDDING.md).

## Current module pass

- Fixed unconfirmed SDRconnect startup being treated as a transient network
  timeout. After one activation, missing confirmation now returns `StartTimeout`
  after three seconds and requires explicit Play again. Native output is opened
  only after startup succeeds. Transport disconnections keep bounded recovery.
- Reduced startup property traffic to one poll per 100 ms, retaining cancellation
  checks between polls. Tests cover successful startup, bounded failed startup,
  cancellation and socket closure without stopping shared reception.
- Reconciled the extraction notes, public API docs and roadmap with the completed
  host migration, real nRSP-ST audio evidence and remaining platform gates.
- Local all-feature workspace verification passed: formatting, check, strict
  Clippy for all targets, **131 tests** and build. The headless SDRconnect module
  also passed the AArch64 Linux compile check. These are software checks; this
  pass adds no new physical receiver, listening or Pi acceptance evidence.

## Recorded delivery milestones

| Commit | Verified result |
| --- | --- |
| Pip-Boy `2a9ad3d` | Headless module integrated through thin adapters; native IP playback and HUD regression checks |
| Rust Radio `6997dff` | Native output backpressure excluded from the next SDR audio read timeout |
| Rust Radio `9831f29` | Recorded operator-confirmed radio audibility on MacBook Air speakers using system default output |
| Rust Radio `f46034d`, Pip-Boy `1298bcf` | Explicit Play activation added and consumed by the host; stopped-receiver synthetic/native-output smoke passed |
| Rust Radio `3b38f8f` | Headless SDRconnect AArch64 Linux compile check passed locally and added to CI |

Real nRSP-ST IQ Lite PCM reached native callbacks in both players, including
concurrent subscribers, volume/output restart and local Stop leaving reception
running. See [dated receiver evidence](NRSP_ST.md). Synthetic activation tests
are recorded separately from real-receiver evidence.

## Open acceptance and known issues

- **Pi:** the headless module compiles for AArch64 Linux; physical native audio,
  the HUD/display, touch and resource/power behavior remain unverified.
- **Receiver:** real stopped-device activation/recovery still needs field
  acceptance. IQ Lite returned zero raw I/Q pairs; Full IQ startup showed an
  SDRconnect firmware update prerequisite. Firmware was not changed.
- **Outputs/UI:** physical Bluetooth/USB loss and recovery, Windows output and
  standalone egui GPU/touch acceptance remain open. The standalone panel's
  station controls currently support IP radio; Pip-Boy owns its SDR selector.
- **USB SDR:** the adapter compiles/links, but no supported device or driver was
  discovered on this Mac. RawIq ingress is implemented; local RF DSP audio is not.
- **CI:** the inspected [Rust Radio run at `3b38f8f`](https://github.com/pskeffington/rust_radio/actions/runs/40602802105)
  and [Pip-Boy run at `1298bcf`](https://github.com/pskeffington/pip_boy/actions/runs/40599722688)
  reported `startup_failure` with zero jobs. Earlier failed-run retry requests
  were refused by GitHub. No underlying cause or remote green result is established.
- **Host test reliability:** an existing dashboard role-activation test has
  intermittently returned 409 during parallel suites; subsequent complete runs
  passed. No dashboard implementation was changed by these radio passes.

Station history is currently kept for the CLI invocation or panel lifetime.
Persistent validation history, HLS transport and satellite adapters remain
deferred; they are not prerequisites for the accepted MP3/AAC/native PCM paths.
