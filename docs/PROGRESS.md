# Radio development progress

Updated 2026-10-28. Active priorities and ownership are in the
[roadmap](PORTING_PLAN.md); embedding commands are in [the module contract](EMBEDDING.md).

## Current module pass: station-list label A/B

- Compared catalog rows that show `station · category` (A) with station-name-only
  rows (B), using egui's proportional 14-point font and the popup's 274-pixel
  text budget. Both variants fit all 25 curated stations without truncation;
  B saves about 1,496 pixels across all labels but does not fit any additional
  station names. Kept A because its category cue aids scanning without clipping.
- Added a font-metric regression test to keep the current catalog labels within
  the compact popup row width. This is an offline layout A/B, not a listener
  cohort study; no audience preference was measured.
- Pushed commit `af06e14` on `codex/station-label-layout-ab` and opened PR #3.
  The local formatting, default/all-feature test, strict Clippy and all-feature
  check gates passed. GitHub Actions run `40738637556` ended as
  `startup_failure` before creating any jobs; the connector returned no job logs
  or step diagnostics. Remote CI remains unverified, and no workflow change is
  justified by the available evidence.

## Current module pass: listener-focused interface

- Replaced the confusing Playing + Unverified combination with a presentation
  driven by live playback state. Genre remains beneath the station; validation
  history and technical details are available in setup, with live audio taking
  priority. No decode/callback diagnostics appear on the listening screen.
- Enlarged the station picker to use available width, removed duplicate status
  lines, softened scanlines and secondary text, and replaced the two playback
  buttons with a single Play/Stop/Retry action. Queued switching remains cancelable.
- Fixed the short volume-slider sizing and added Mute/Unmute with previous-volume
  restoration. Footer hints identify the next action or actual selected output.
  Common stream errors explain a next action in plain language; exact errors
  remain in setup. Long errors use remaining viewport space with Retry visible.
- Simplified IP setup labels, exposed save/remove feedback, and moved connection
  details into a collapsible section with a bounded scroll body. Playback,
  persistence and embeddable host APIs remain intact.
- Visually inspected the redesigned macOS app. Added regression coverage for
  all listening states, stale check results, diagnostic-free Playing, mute
  restoration and long-error bounds. Final verification passed **195 all-feature
  tests**, **146 default tests**, the separate native muted acceptance, formatting,
  strict Clippy in both configurations, workspace check and all-feature build.
  Integrated concurrent main changes: the 25-station directory, machine-readable
  catalog and six-hour feed freshness policy are preserved. Live playback takes
  priority; historical freshness remains visible in setup and qualified tooltips.
  The rebuilt standalone bundle is in `target/debug/Rust Radio.app`. Physical
  touch/Pi rendering remain open.

## Previous module pass: IP streaming GUI and station lifecycle

- Kept routine controls visible with locally scoped phosphor popup/menu/button
  styling, current-selection marker, long-name tooltips, scrolling and keyboard
  navigation. Failures wrap with a visible marker and RETRY. IP connection/output
  settings and named Save/Remove controls live behind IP Radio.
- Selection while stopped silently validates the actual endpoint after a short
  debounce; Play cancels that check and queues audio. Active station switching
  mutes/cancels the previous worker and waits for release before starting only
  the latest selection. Stop cancels queued work; discovery respects explicit
  source choices even when internet/receiver availability is unverified.
- Custom public HTTPS and private/loopback IPv4 HTTP now resolve checked relative
  redirects and M3U/PLS, sharing bounded parsers with curated streams. HLS,
  unsafe/nested/oversized playlists and hop loops produce useful failures.
  Fixed cancellation-versus-disconnected-channel error classification. A shared
  35-second first-decode budget also bounds continuous invalid startup input.
- Added bounded, atomic IP station import/export and standalone background
  persistence, without changing embedded host storage ownership. Up to 24 named
  streams and last saved/curated selection survive restart. Explicit Save stores
  URLs including query parameters; unsaved edits remain in memory.
- Native muted GUI acceptance passed MP3/AAC, redirect/PLS, rapid switches,
  stalled headers, invalid/unsupported streams, Retry, refused connections and
  clean Stop. Render calls stayed below 250 ms. WCRB and Skylab both reached
  native Playing live. macOS main/menu/editor layouts were visually inspected.
- External LAN broadcaster, audible quality, physical touch and Raspberry Pi
  rendering remain unverified. HLS and alternate-playlist-entry failover remain
  unsupported. Final gates passed: **189 all-feature tests**, **140 default
  tests**, the separate muted native GUI acceptance, formatting, strict Clippy
  in both configurations, all-feature workspace build and the headless native
  AArch64 compile check. The normal all-feature suite leaves two hardware/manual
  tests ignored; one was executed explicitly for this pass.

## Previous module pass: advanced FM broadcast tuning

- Added Tune / FM Audio / Presets / Device tabs behind nSDR, with exact unit-aware
  frequency entry and live 1/10/50/100/200 kHz steps. The main nSDR arrows also
  tune the current worker; no receiver/output replacement occurs.
- Added native FM stereo/mono, 50/75 µs or disabled de-emphasis, audio low/high
  cuts and limiter controls. Stereo permission and filters are validated.
  Stereo lock is parsed from receiver type-214 reports; requested stereo and
  two-channel PCM never count as lock evidence.
- Added 16 bounded named FM presets, including WCRB Boston 99.5 MHz, with full
  FM audio choices. Standalone persistence uses an atomic background file writer;
  embedded hosts retain preference ownership. No addresses or keys are stored.
- Live muted test kept the existing physical frequency unchanged, switched to
  mono / 50 µs / speech filtering and back to stereo / 75 µs / full-band audio,
  and remained Playing through at least 71,040 decoded frames. Real PLL reports
  showed lock and unlock; clean Stop released only the owned session.
- Fixture tests cover exact native FM properties, routing, filters, preset JSON,
  file replacement, all four layouts, long names and safe band-edge stepping.
  All-feature acceptance passed **179 tests** (one manual output test ignored);
  the default suite passed **129 tests**. Formatting, strict Clippy, receiver
  GUI/CLI builds and the headless AArch64 compile check passed.
- Opened the updated standalone app; the preset setup view was visually checked
  at the compact viewport. Further audible FM quality checks remain operator work.

## Previous module pass: input discovery and header setup

- Replaced the header label with IP Radio | nSDR | USB source buttons. Availability
  drives green glow; selection has its own underline. Each input button opens
  its setup panel, replacing the general Settings button.
- Added explicit reusable background discovery and standalone startup/30-second
  idle scans. Internet, native network receivers and supported USB descriptors
  are checked independently. Discovery selects available hardware controls but
  never starts reception or output, and cannot switch an active engine.
- Live discovery found the real nRSP-ST automatically and reported internet
  available with no USB receiver attached. The native scan checks private local
  neighborhoods and confirms device Hello identity, rather than assuming that
  an open port means a working receiver.
- Visually checked the running GUI: IP Radio and nSDR glowed, USB stayed dim,
  and the discovered nRSP-ST controls were selected with reception stopped.
- Fixture coverage includes the offline USB-only glow case, multiple inputs,
  unavailable sources, setup bounds, cancelled scans and receiver discovery
  without control commands. All-feature acceptance passed **169 tests** (one
  manual output test ignored); the default suite, formatting, strict Clippy and
  headless AArch64 compile checks passed. Physical USB reception is still unverified.

## Previous module pass: direct native nRSP-ST

- Established a direct Rust receiver session on the device's port 9001 without
  SDRconnect or a browser. Implemented native handshake/session framing, explicit
  virtual receiver creation/start/stop, shared physical tuning, mode/bandwidth
  configuration, Opus decode and native callback playback.
- Added `nrsp-native` features for the headless client, native engine, CLI and
  compact GUI. Live tuning uses one bounded request slot on the existing worker;
  the endpoint remains locked while active. Native login requirements are
  detected before controls; authenticated login remains unimplemented.
- Real device: direct status identified protocol 0 and model 5; WFM at 99.1 MHz
  reached the Mac audio callback at zero volume. A single session then tuned to
  99.2 MHz NFM and back to 99.1 MHz WFM, independently confirmed both frequencies,
  decoded 72,960 stereo frames with nonzero signal, and stopped cleanly. The
  device was restored to the original tuning. No samples, addresses or session
  keys were committed. Current checks establish live control and native audio;
  fresh speaker listening and the physical Pi client are separate acceptance.
- All-feature workspace tests passed **163 tests** (one manual output test
  ignored), with strict Clippy, formatting and build checks. The default feature
  suite and headless native receiver AArch64 Linux compile check also passed.
  Native GUI offline tests cover invalid input, source navigation and the
  480×320 settings bounds. A CI job now covers this optional native feature.
- Native protocol implementation notes: [wire contract](NRSP_NATIVE_PROTOCOL.md).

## Previous module pass: internet and LAN streams

- Added WCRB Classical as a publisher-documented preset and explicit direct
  HTTPS/private IPv4 HTTP stream entry in the standalone GUI, CLI and embedding
  API. Stream URLs are session-only. Direct input refuses redirects, playlists,
  credentials and fragments; MP3 and AAC use the existing bounded player.
- WCRB sends MPEG-2 ADTS AAC. A frame-aware compatibility reader normalizes
  the ADTS ID bit for the existing decoder without modifying AAC payloads;
  generated MPEG-2 ADTS fixture and live native callback tests cover the path.
  A zero ICY metadata interval is treated as disabled. WCRB preset and direct
  URL and Skylab preset each reached the Mac native output callback at zero
  volume; these runs do not establish audible listening.
- All-feature workspace tests (156 passed, one manual test ignored), strict
  Clippy, formatting and build passed. The direct transport also has a local
  HTTP audio and redirect fixture; a real LAN broadcaster remains to be tested.
- Direct nRSP-ST audio and tuning are still unimplemented. Previous direct
  service checks established reachability only; they did not establish a usable
  audio protocol. SDRconnect remains required for the existing nRSP-ST audio path.

## Previous module pass: USB mono FM

- Added initial standalone USB mono-FM playback via the existing SoapySDR
  adapter, a 240 ksample/s I/Q demodulator, shared native output and explicit
  CLI/GUI source selection. Synthetic FM tone/filter tests, a mock-device
  receive/cleanup test and a zero-volume Mac native callback check passed.
  No physical USB receiver was found during explicit discovery; RF and audible
  hardware acceptance remain open.
- The all-feature workspace passed formatting, check, strict Clippy, **152
  tests** and build; IP-only and USB-enabled GUI feature tests and the headless
  AArch64 compile check also passed. The separate ignored native-output check
  was run manually with synthetic I/Q and zero volume. The USB-enabled CLI
  also completed a six-second live Skylab playback check at zero volume.

## Previous module pass: direct nRSP status

- Cloned and reviewed Fides Supra main `2447fdc` in a temporary checkout. Its
  direct nRSP-ST work establishes web/device service checks and a passive
  browser-observed spectrum path; it does not provide direct audio or tuning.
- Added `nrsp-status` and a reusable direct status API without SDRconnect. A
  check of the operator-identified NETRADIO host returned HTTP 200 on port 9001
  and a TCP connection on port 50000. This is service reachability evidence,
  not receiver identity or audio/IQ evidence. No device address was saved. The
  Mac's SDRconnect app was running during the live check, but the command only
  contacted the device's two direct services.
- All-feature workspace formatting, check, strict Clippy, **140 tests** and build
  passed, along with the headless `sdr` AArch64 Linux compile check. Direct
  loopback fixtures cover local-address restrictions, cancellation, redirects
  and closing the device-service connection without sending a command.

## Previous module pass: GUI

- Added a locally scoped, classic phosphor-green retro skin to the 480×320
  standalone panel and settings. The optional `sdrconnect` feature adds an
  nRSP-ST source and in-memory API address field using the shared native engine.
  Source selection and address editing remain offline until explicit PLAY.
- On macOS, visually inspected the IP/SDR views and settings, then used a local
  synthetic receiver to reach Playing and Stop through the GUI at zero volume.
  The real nRSP-ST reported started but a 500 ms audio capture returned zero
  pairs; a fresh real-receiver GUI audio check is still open.
- Local all-feature formatting, check, strict Clippy, **136 workspace tests** and
  build passed. The IP-only panel's 11 tests, facade `gui,sdrconnect` check and
  headless AArch64 SDRconnect compile check also passed.

## Previous module pass: IP prebuffer

- Corrected IP playback when one decoded packet is larger than the paused
  native output ring. The worker now starts output while delivering the packet,
  once prebuffered, rather than waiting until the entire packet is queued.
  Bounded-sink regressions cover large packets, start failure and cancellation;
  Playing still requires native callback consumption.
- All-feature workspace format, check, strict Clippy, **133 tests** and build
  passed. The headless SDRconnect AArch64 Linux compile check passed. An explicit
  eight-second Skylab session at zero volume exited successfully after native
  callback playback on the Mac system output. This is callback evidence, not a
  fresh audible listening or physical Pi acceptance.
- Pip-Boy `81de2d5` pins module `3d77d11`; the host passed 344 tests serially,
  formatting, strict Clippy, build and UI/security checks. Its zero-volume smoke
  switched from synthetic SDR PCM to live Skylab and reached native callback
  playback. The primary host checkout was updated and rebuilt. Upstream sync
  formatting and explicit lock-file open mode were repaired in separate host
  commit `3e2488d` before the checks.

## Previous module pass

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
- Pip-Boy `beaa719` consumes module `21045b3`. Its extended HUD smoke confirmed
  exactly one activation and no audio subscription on unconfirmed startup, then
  native callback playback after corrected fixture setup and explicit Play.
  Standalone PCM, output restart and socket cleanup also passed at zero volume.
- After incorporating intervening upstream sync commits, host formatting, check,
  strict Clippy, build and UI/security validators passed; all **341 tests passed
  serially**. Upstream sync formatting/lint repairs were isolated in host commit
  `3b11bce`. The primary Pip-Boy checkout was fast-forwarded and rebuilt with SDR
  enabled; its pre-existing untracked `rust_radio/` directory was preserved.

## Recorded delivery milestones

| Commit | Verified result |
| --- | --- |
| Pip-Boy `2a9ad3d` | Headless module integrated through thin adapters; native IP playback and HUD regression checks |
| Rust Radio `6997dff` | Native output backpressure excluded from the next SDR audio read timeout |
| Rust Radio `9831f29` | Recorded operator-confirmed radio audibility on MacBook Air speakers using system default output |
| Rust Radio `f46034d`, Pip-Boy `1298bcf` | Explicit Play activation added and consumed by the host; stopped-receiver synthetic/native-output smoke passed |
| Rust Radio `3b38f8f` | Headless SDRconnect AArch64 Linux compile check passed locally and added to CI |
| Rust Radio `21045b3`, Pip-Boy `beaa719` | Unconfirmed receiver startup fails without automatic activation retries; explicit retry and native output integration passed |

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
- **Outputs/UI:** physical Bluetooth/USB loss and recovery, Windows output,
  standalone egui touch and Pi rendering remain open. The SDR-enabled standalone
  panel and Pip-Boy each expose their own UI over the shared engine.
- **USB SDR:** the adapter and initial mono-FM path compile/link, but no supported
  device or driver was discovered on this Mac. Physical RF and audio remain open.
- **CI:** the inspected [Rust Radio run at `3d77d11`](https://github.com/pskeffington/rust_radio/actions/runs/40624628207)
  and [Pip-Boy run at `81de2d5`](https://github.com/pskeffington/pip_boy/actions/runs/40624801806)
  reported `startup_failure` with zero jobs. Earlier failed-run retry requests
  were refused by GitHub. No underlying cause or remote green result is established.
- **Host test reliability:** an existing dashboard role-activation test again
  returned 409 in the parallel suite on this pass. The complete serial suite
  passed; the parallel failure remains unresolved. No dashboard implementation
  was changed by these radio passes.

Station history is currently kept for the CLI invocation or panel lifetime.
Persistent validation history, HLS transport and satellite adapters remain
deferred; they are not prerequisites for the accepted MP3/AAC/native PCM paths.
