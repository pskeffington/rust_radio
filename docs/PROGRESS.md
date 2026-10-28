# Radio development progress

Updated 2026-10-28. Active priorities and ownership are in the
[roadmap](PORTING_PLAN.md); embedding commands are in [the module contract](EMBEDDING.md).

## Cross-source callback status reconciliation (2026-10-28)

- Implementation commit `df6a73d` shares the native callback-consumed sample
  counter with the session status reconciler for IP, direct nRSP-native,
  SDRconnect and Soapy playback. `Playing` can now be observed after delayed
  output callbacks even if the source is waiting for its next packet/read.
- Recovery clears the ended attempt's counter before a retry; each new output
  attempt receives its own counter. The regression checks delayed callback
  progress for SDR, IP retry isolation, and Stop precedence.
- The focused callback tests passed (2/2). All six required gates passed:
  formatting, default workspace tests, strict default Clippy, all-feature
  workspace tests, strict all-feature Clippy and all-feature check. Loopback
  fixtures required local socket permission. One manual native-output test
  remains intentionally ignored.
- Headless AArch64 SDRconnect compilation passed. The layout A/B repeated at
  295 px: category+name fit 24/25 labels and names-only fit 25/25; both fit at
  340 px. Names-only saves 1,776 aggregate text pixels. This is a fit metric,
  not listener preference evidence.
- A fresh bounded silent station sweep at 2026-10-28 19:32 UTC decoded all
  25/25 catalog entries in 25 attempts at `decoded_audio`. This is point-in-time
  decoder evidence only; it does not establish human audibility or rights.
- The change is callback evidence only. Human-audible output, physical output
  loss/recovery, Pi runtime, USB/receiver hardware and publisher rights remain
  separate acceptance gates.

## Compact station category A/B/C (2026-10-28)

- The 295 px A/B evidence showed only WCRB overflowing when the full station
  name and category were shown. Variant C shortens `CLASSICAL` to `CLASS` while
  preserving the category cue and full genre on hover.
- egui 14-point monospace fit: full category A 24/25, names-only B 25/25, and
  compact category C 25/25 at 295 px. All variants fit 25/25 at 340 px. C saves
  32 text pixels on WCRB compared with A.
- Implemented in `a417db6`; regression
  `station_label_ab_compares_full_compact_and_name_only_variants` measures all
  three alternatives and confirms WCRB is the only A overflow.
- Test-guard commit `3b8cf74` additionally asserts that every non-WCRB category
  remains unchanged. The full GUI crate passed (31 passed, one hardware test
  ignored) and strict GUI Clippy passed.
- This compares compact layout fit, not listener preference. Hover text keeps
  the station's full genre.

## Bounded native output stall handling (2026-10-28)

- Code commit `ca4cf00` extracts the PCM-ring backpressure loop behind an
  internal callback watchdog helper without changing the five-second runtime
  threshold.
- New synthetic tests fill a one-sample ring and verify a silent callback is
  classified as output failure within an injected short timeout. A separate
  cancellation test verifies Stop does not become a device failure.
- The two new regressions, all six workspace validation gates, the station
  label A/B and headless AArch64 SDRconnect compilation passed. Network fixture
  tests required local socket permission.
- This exercises a hardware failure path deterministically; it does not replace
  physical Bluetooth/USB loss and recovery acceptance.

## Delayed callback playback-state fix (2026-10-28)

- Implementation commit `94f0daa` gives each IP stream attempt its own
  callback-consumed sample counter. `NativeRadioEngine::status()` transitions
  Buffering to Playing from that counter, even when the decoder has finished a
  packet and is waiting for the network. A retry replaces the counter before
  the next connection; Stop and generation handling remain authoritative.
- Regression `status_reconciles_delayed_callback_progress_per_attempt` covers
  delayed callback progress, retry isolation and Stop. It passes.
- All six required gates pass on this implementation: formatting, default and
  all-feature workspace tests, strict default and all-feature Clippy, and
  all-feature check. Loopback tests passed with local socket permission; one
  physical audio test remains intentionally ignored.
- The repeatable station-label A/B also passed: 24/25 vs. 25/25 at 295 px,
  both 25/25 at 340 px, with 1,776 aggregate text-pixel savings for names-only.
  This is a fit metric, not a listener preference test.
- This callback counter is software evidence only. Human-audible output and
  Bluetooth/USB device-loss recovery remain hardware gates.
- On head `94f0daa`, GitHub Actions run `40753639896` failed during startup
  before creating jobs; the combined status is empty. No Rust job-step failure
  is evidenced.

## Portability and layout recheck (2026-10-28)

- On pushed head `66f9efd`, `cargo check --locked -p rust-radio
  --no-default-features --features sdrconnect --target
  aarch64-unknown-linux-gnu` passed locally using the installed ARM64 target.
  This checks headless compilation only; it is not Raspberry Pi runtime or
  audio/display acceptance.
- Repeated `name_plus_category_rows_fit_without_truncation`: category+name
  fit 24/25 rows at 295 px versus 25/25 for names-only; both fit 25/25 at
  340 px. Names-only saves 1,776 aggregate text pixels. This is a layout-fit
  metric, not listener preference evidence.
- PR #3 remains open and mergeable; current head `66f9efd` has empty combined
  status. Latest run `40752135344` still failed before creating jobs.
- Rust-Boy’s actual product surface is its HTML/HUD host adapter; adding only
  the new egui history API pin would not exercise it. Host adoption remains a
  separate integration task, along with physical hardware and rights gates.

## Current pushed-head verification (2026-10-28)

- Revalidated exact head `cd239e2`: formatting, default and all-feature workspace
  tests, strict default and all-feature Clippy, and all-feature check passed.
  Loopback HTTP fixtures passed when run with local socket permission; the
  ordinary sandbox denied binds. One hardware audio test remains intentionally
  ignored.
- Repeated the bounded, silent live catalog sweep: 25/25 stations reached
  `decoded_audio` in 25 attempts. This is point-in-time codec evidence only.
- Repeated the station-name layout comparison: at 295 px category+name fit
  24/25 versus names-only 25/25; at 340 px both fit 25/25. Names-only saves
  1,776 aggregate label pixels; this is a fit measure, not a listener study.
- PR #3 remains open and mergeable. Head `cd239e2` has no combined status;
  Actions run `40751673743` failed before creating any jobs. The workflow file
  defines standard GitHub-hosted jobs, so no repository step failure is
  evidenced and the startup cause remains external/unknown.
- Rust-Boy's local dependency pin has not adopted the new headless history API.
  Hardware, host adoption and broadcaster rights gates remain open.

## Current module pass: strict saved-history validation (2026-10-28)

- Implementation commit: `52b85cc` (shared host-neutral parser).
- Tightened both the CLI history writer and GUI host importer to reject unknown
  status/stage values, future record timestamps, inconsistent decoded evidence,
  invalid attempt/failure counts and mismatched health states. Invalid old files
  are left untouched; transport errors without stage data normalize to
  `not_checked` so the saved contract remains readable.
- Corrected stale README and feed-policy statements that described persistence
  and host import as future work.
- Focused CLI/GUI history tests pass. All six required gates pass: formatting,
  default workspace tests, strict default Clippy, all-feature tests, strict
  all-feature Clippy and all-feature check. Loopback fixture tests passed with
  local network permission. One native-audio GUI test remains intentionally
  ignored because it requires physical audio output.
- Repeated the compact station-row A/B: category+name 24/25 vs name-only 25/25
  at 295 px; both 25/25 at 340 px; name-only saves 1,776 text pixels. This is a
  fit metric, not a user preference measurement.
- Moved the versioned parser into `rust-radio-core` and re-exported it from the
  headless facade. The CLI rewriter and optional egui panel now share the same
  parser; an embedding-contract test proves a non-GUI host can read history
  without file/network I/O or enabling audio features.
- PR #3 is open and reports mergeable. GitHub reports no combined status for
  remote head `1cb43f1`; Actions run `40751586284` failed during startup and
  returned zero jobs. The headless parser passed all required local gates.
  Rust-Boy currently pins an older module revision, so consuming the new API
  requires a later host pin update. Physical output recovery, Pi runtime, USB
  SDR acceptance and publisher rights remain open.

## Current module pass: scheduled feed sweep and endpoint recovery

- Added `rust-radio-cli validate-all --json`, which runs at most four checks at
  once, preserves catalog order, uses each validator's bounded 18-second
  no-retry request, and emits a credential-safe report without resolved URLs.
  Individual feed failures remain report entries and do not remove stations or
  fail the completed sweep. Offline regression coverage verifies concurrency,
  ordered results, and JSON shape.
- Added daily scheduled and manually dispatchable GitHub Actions validation;
  the JSON report is retained for 30 days. This remains point-in-time evidence
  and does not itself persist runtime freshness or gate playback.
- The first live sweep decoded 24/25 feeds. `leftcoast70s` returned HTTP 404,
  exposing that the validator incorrectly classified every HTTP status as
  temporary. HTTP 408, 425, 429 and 5xx remain `unavailable`; other rejected
  responses now report `invalid_stream`. Synthetic tests cover 404 and 429.
- SomaFM's official [Left Coast 70s stream page](https://somafm.com/seventies/directstreamlinks.html)
  lists the permanent MP3 M3U URL `https://somafm.com/m3u/seventies.m3u` and
  personal-use-only terms. Updated the runtime catalog and JSON mirror to that
  authorized endpoint while preserving the station ID. The focused silent
  decode passed; a second full live sweep decoded all 25/25 feeds.
- Format, workspace tests, strict default/all-feature Clippy, all-feature tests
  and all-feature check pass. The loopback HTTP fixtures require local-network
  permission; both complete test suites passed after granting it. A live decode
  verifies stream decoding only, not physical audio output or redistribution
  rights.
- Existing station-label layout A/B evidence remains: at 295px category labels
  fit 24/25 vs names-only 25/25; at 340px both fit 25/25, and names-only saves
  1,776 aggregate text pixels. This is a compact-layout measurement, not a
  listener preference test.

## Current module pass: transient feed recovery

- The first scheduled sweep had no retry window, so added one retry after a
  one-second recovery delay for `Unavailable` results only. Permanent endpoint
  rejection, unsupported formats, and cancellation are not retried. The sweep
  remains bounded at four workers and at most two 18-second probes per station.
- The JSON result now includes attempt count and whether the retry recovered.
  Synthetic coverage proves a transient outage followed by decoded audio is
  marked recovered, while a simulated 404 is attempted once and remains a
  visible failure. Focused tests pass.
- This improves short-outage tolerance without admitting stale results; local
  persistence and host import were added in the subsequent feed-history pass.
- Final format, default/all-feature workspace tests, strict default/all-feature
  Clippy and all-feature check pass after the retry change. A bounded live sweep
  decoded 25/25 feeds in 25 attempts, so no retry was needed during that sample.
- GitHub Actions run `40746359995` on the previous head failed at startup before
  jobs; no current-head validation is available yet. Physical hardware and
  persistence gates remain open.

## Current module pass: opt-in feed-health history

- Added `validate-all --json --history PATH`. The bounded 64 KiB versioned
  state keeps each station's latest status/time, last successful decoded time,
  freshness, attempts and consecutive failures. Failed checks preserve the
  last decoded timestamp while making current health visibly failed.
- History writes use a synced temporary file and atomic rename; malformed,
  oversized, partial-catalog or unsupported-schema input is rejected before
  replacing existing state. On Unix, the state file is owner-readable and
  writable only. The feature is opt-in; it does not change GUI or playback
  startup and does not gate a station from playback.
- Unit coverage writes a successful check followed by a failure, verifies the
  last decoded timestamp is retained and failure count increments, then checks
  malformed history remains untouched. Full CLI tests pass.

## Current module pass: host import of saved feed evidence

- Added `RadioPanel::load_validation_history(json)` so the host can read its
  saved state off the UI thread and pass the bounded JSON text to the panel.
  Import validates schema, catalog IDs, statuses, attempts, failure counts and
  timestamp consistency before replacing the in-memory snapshot.
- Imported results render as `SAVED / ...`; freshness is recomputed against the
  shared six-hour TTL instead of trusting the file's cached health value. A
  current silent check takes precedence, and active playback remains the
  listening surface's source of truth. Import performs no file/network I/O and
  never starts or gates playback.
- GUI tests verify saved success expiry, malformed/future evidence rejection,
  live-result precedence and no autoplay. Full default/all-feature gates pass.
- CLI and GUI now both consume the same checked-in version-one history fixture;
  the CLI preserves its last decoded timestamp on failure, while the GUI
  ignores the fixture's historical `fresh` label and recomputes it as stale.
- Re-ran the font-metric A/B regression: category-plus-name fits 24/25 rows at
  295px versus 25/25 names-only; both fit 25/25 at 340px, and names-only saves
  1,776 aggregate text pixels.

## Current module pass: catalog mirror parity

- The metadata implementation exposed a stale `catalog/curated-stations.json`
  mirror. Added broadcaster, language and country fields to all 25 JSON records
  and added a CLI test comparing every mirrored metadata field and station order
  against the runtime Rust catalog.
- Updated `CURATED_FEED_POLICY.md` with the full parity contract. This prevents
  external tools from seeing older metadata than the embedded runtime.
- The parity test and all requested gates pass: format, default/all-feature
  workspace tests, both strict Clippy configurations, and all-feature check.
  Loopback HTTP fixtures passed with local-network permission.
- GitHub Actions run `40744821542` (run 141) for the tested catalog-mirror code
  head ended `startup_failure` at creation time with zero jobs and no status
  checks. The following documentation-only head also failed before jobs were
  created. Workflow YAML parses locally; there is no job/log evidence pointing
  to a repository fault, so no speculative workflow edit was made.

## Current module pass: catalog metadata and NPR research

- Added machine-readable broadcaster, ISO 639-1 language, and ISO 3166-1
  country fields to the 25 curated station records. Existing genre, codec hint,
  stream endpoint and publisher provenance remain on each record; runtime
  decoded-audio state remains in `StationValidation`, not a static catalog flag.
  Regression coverage checks required values and code shapes for every record.
- Reviewed NPR's first-party Content Distribution Service audio schema and
  premium-content guide. They distinguish live/on-air audio from other
  streamable files and identify premium enclosures that need express rights.
  WBUR documents a live NPR-affiliate stream, but no stable direct audio URL or
  redistribution terms were verified. NPR News therefore remains a candidate,
  not an added endpoint, pending first-party stream and permission evidence.
- Sources: [NPR audio profile](https://npr.github.io/content-distribution-service/api-reference/profiles/audio.html),
  [NPR premium-content guide](https://npr.github.io/content-distribution-service/legal/premium-content/index),
  [WBUR ways to listen](https://www.wbur.org/ways-to-listen).
- Next: validate metadata completeness; resolve a direct authorized NPR News
  endpoint; then review each publisher's bundling terms before release.

## Current module pass: catalog probe coverage

- Replaced the duplicate station-ID/host probe table with a target derived
  from the catalog's publisher HTTPS URL after exact station-ID lookup. This
  keeps reachability probes allowlisted while automatically covering WCRB and
  future catalog endpoint changes.
- Added regression checks for every catalog entry, WCRB's GBH stream host,
  Skylab's explicit port, and rejection of unknown or URL-shaped IDs.
- Passed formatting, default/all-feature workspace tests, strict Clippy in
  both configurations, and all-feature workspace check. Loopback HTTP fixtures
  needed local-network permission; all tests passed on rerun.
- This validates TCP-target coverage only; it does not establish endpoint
  availability, decoded audio, publisher rights, or physical output. Next:
  continue station metadata/rights review and complete physical output recovery.

## Current module pass: native output callback coverage

- Extracted the callback's bounded sample transfer into a private helper used
  directly by CPAL. Added a synthetic ring-buffer regression for gain, silence
  on underrun, and counting only consumed PCM. The callback remains lock-free,
  allocation-free and non-blocking; this is not physical output acceptance.
- Passed formatting, default workspace tests, default strict Clippy,
  all-feature workspace tests, all-feature strict Clippy and all-feature check.
  The workspace HTTP fixture tests bind loopback; their first sandboxed runs
  were denied by the environment and passed when rerun with local-network
  permission. The focused callback test also passed independently.
- No runtime defect or physical hardware result was found. Next: retain these
  deterministic checks and complete Bluetooth/USB output-loss and recovery
  acceptance on an actual device.

## Current module pass: output failure reporting

- Extracted worker-result handling so output-loss classification, host-visible
  `output_failed()` state and the terminal session error are recorded together.
- Added synthetic regression coverage: output failure sets the host flag and
  preserves the detail; network failure does not set the output flag; canceled
  output failure does not report a stale error. No physical device was opened.
- Passed formatting, default workspace tests (148 passed, one ignored), default
  strict Clippy, all-feature tests (197 passed, two ignored), all-feature strict
  Clippy and all-feature workspace check. Apple Silicon GUI checks passed for
  default, `nrsp-native` and `sdrconnect`.
- The current test-only refinement `6432501` reran the worker-reaping regression
  in default and all-feature builds; both passed, as did all-feature strict
  Clippy and formatting. The full workspace suites above passed on `75d7ea6`.
- Next output gate is device-loss/reconnect acceptance on physical Bluetooth or
  USB audio, including selecting a replacement output and explicit retry.

## Current module pass: station-list label A/B

- Re-ran the label comparison with the actual 14-point monospace button font
  and the station menu's explicit 20-pixel total horizontal button padding. At
  the existing 295-pixel row width (275 pixels for text),
  `station · category` labels fit 24/25 stations and names alone fit 25/25;
  names alone save 1,776 aggregate text pixels. Widening rows to 340 pixels
  (320 pixels for text) lets all 25 category labels fit. Kept the category cue
  and widened the popup rather than dropping useful scan context.
- Added a font-metric regression test for both label variants and both widths.
  This is an offline layout A/B, not a listener cohort study; no audience
  preference was measured.
- PR #3, `Polish compact radio UI and test station labels`, remains open on
  `codex/station-label-layout-ab`. Recent Actions runs `40739532314` and
  `40740893003` ended as `startup_failure` before creating jobs; combined status
  and diagnostics were empty. Remote CI remains unverified, and no workflow
  change is justified by those results.
- Additional locked target checks passed for the headless module on
  `aarch64-unknown-linux-gnu` with both `nrsp-native` and `sdrconnect`, and the
  default IP module on `aarch64-apple-darwin`. These are compile checks, not Pi
  runtime or native device acceptance.
- A bounded live `cargo run -p rust-radio-cli -- validate skylab` returned exit
  2 with `Unavailable`, `NotChecked`, `TransportUnavailable`, and no resolved
  endpoint. This environment did not reach the stream; the result is not
  evidence that Skylab is unavailable or healthy, so a full live sweep remains
  pending working outbound stream access.

- Follow-up UI polish: widened the station popup row cap to 340 pixels and
  corrected the A/B test to measure egui's monospace font. The focused regression
  measures 24/25 category rows at 295 pixels and 25/25 in the other three
  label/width combinations. Formatting,
  default workspace tests (147 passed, one ignored), all-feature tests (196
  passed, two ignored), both strict Clippy configurations, and the all-feature
  check passed. The default, `nrsp-native` and `sdrconnect` Apple Silicon GUI
  cross-checks passed. Initial loopback fixture runs were denied by the sandbox;
  both suites passed when rerun with local-network permission. No physical
  hardware acceptance was performed.

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

Station history can be persisted by the CLI to an opt-in bounded file and
imported by an egui host; saved history remains informational and never gates
playback. HLS transport and satellite adapters remain deferred; they are not
prerequisites for the accepted MP3/AAC/native PCM paths.

## Process-wide playback session admission (2026-10-28)

A reliability audit found that the single-worker invariant applied only within
one `NativeRadioEngine`; separate host instances could each spawn their own
native playback worker. The audio crate now acquires a process-wide atomic RAII
lease before starting the worker. A concurrent start fails before opening audio
or network resources, and worker exit or unwinding releases the lease. Focused
tests cover rejection, normal release and panic release. The all-features test
suite, strict Clippy and all-features check pass; local loopback permission is
needed for the HTTP fixture suite. This does not validate physical device-loss
recovery.

The process-wide lease change is committed as `bd48649` and confirmed on the
remote PR branch. At the post-push query, the GitHub connector returned no
combined status or PR-triggered Actions run for this SHA; its PR snapshot still
showed the prior head, so remote CI is pending/unverified.

## Public playback API admission regression (2026-10-28)

- Strengthened the process-wide session test to exercise both public start paths:
  curated internet playback and an explicit direct stream. With the active slot
  held, each returns the duplicate-session error before a worker can open audio
  or network resources. The focused test passes.
- Repeated the station-label A/B/C regression on current source. At 295 px,
  full category+name fits 24/25 rows while names-only and compact-category fit
  25/25; at 340 px all three fit 25/25. Compact `CLASS` saves 32 text pixels on
  WCRB. This remains a deterministic fit measurement, not listener preference.
- Passed formatting, default and all-feature workspace tests, strict Clippy in
  default and all-feature configurations, and all-feature workspace check.
  Loopback fixture tests passed with local socket permission; two hardware audio
  tests remain ignored where the feature/test configuration requires physical
  output.
- GitHub reports run `40757657155` (`Rust radio core`) as `startup_failure` with
  zero jobs for the preceding PR head. Combined commit status is empty. This is
  not evidence of a Rust job failure or a passing remote build; root cause is
  not available from the run metadata.

The public-entry-point regression is committed as `082b029` and confirmed on the
remote PR branch. The PR is open and mergeable. No status or PR-triggered run is
visible yet for `082b029`; the startup failure above belongs to the prior head.

## Probe-time audio output failure classification (2026-10-28)

- Fixed an output-recovery classification gap in IP playback. If the native
  audio stream reports failure while Symphonia is probing the network stream's
  format, the probe can return an I/O error. The worker now checks CPAL's
  output-failure flag and routes that case to operator output selection rather
  than spending the transient network retry budget. Probe I/O without device
  failure remains a network error.
- Regression coverage verifies both classifications and retains the existing
  assertion that output failures do not retry. Focused test passed.
- Passed formatting, default and all-feature workspace tests, strict default
  and all-feature Clippy, all-feature check, and the station-label A/B test.
  Loopback fixture tests required local socket permission; physical audio
  acceptance remains separate.
- Code commit `bf14d59` is pushed. PR #3 is open and reported mergeable. The
  connector showed no combined status or PR-triggered run for this head; prior
  run `40757657155` ended at startup with zero jobs.

## Direct nRSP-ST stream interruption recovery (2026-10-28)

- Direct native nRSP-ST audio now applies the bounded network recovery policy
  to transport failures and audio inactivity timeouts after an active stream
  has been established. Initial connection and receiver activation failures
  remain terminal, so startup does not silently repeat activation.
- Each retry reconnects and reapplies the latest requested frequency, mode,
  bandwidth and FM settings. A regression checks the retained desired settings;
  classification tests distinguish active-stream transport loss from startup
  and protocol failures. Native output converter setup errors are surfaced as
  output failures so hosts can offer device selection.
- Code commit `00576b1` passed formatting, default and all-feature workspace
  tests, strict default and all-feature Clippy, all-feature check and the station
  label A/B regression. Loopback fixtures used local socket permission. The
  tests exercise policy and state; physical receiver interruption acceptance
  remains open.
- PR #3 remains open. At query time GitHub returned no status checks or
  PR-triggered run for `00576b1`; remote CI remains unverified.

## Deterministic direct receiver reconnect fixture (2026-10-28)

- Extracted the direct nRSP retry sequencing into `recover_nrsp_attempts` so
  production playback and offline regression use the same code path.
- The synthetic fixture injects a stream timeout, checks the bounded first
  backoff, verifies exactly one retry, ensures the old output progress counter
  is cleared, and confirms reconnect uses updated frequency, bandwidth and FM
  settings with reconnect mode enabled.
- The focused fixture passes. Formatting, default/all-feature workspace tests,
  strict Clippy in both configurations, all-feature check and the compact-label
  A/B regression pass. Loopback stream fixtures required local socket permission.
- Code commit `724db2a` is pushed. PR #3 remains open; its current head has no
  visible combined status or PR-triggered workflow run, so remote CI is
  unverified. The fixture validates retry policy and state restoration, not a
  physical receiver's RF/network recovery.

## nRSP terminal-reconnect and disconnect-reason coverage (2026-10-28)

- Extended the deterministic retry fixture with a second-attempt permanent setup
  failure. It asserts only one reconnect is attempted after the initial stream
  loss, the 1-second delay occurs once, and output progress is cleared for both
  failed attempts.
- The nRSP classification test now asserts a receiver `Disconnected(reason)`
  remains terminal and preserves its numeric reason in the host error. The local
  contract has no mapping that explains reason codes, so no automatic retry
  semantics are inferred from them.
- Both focused regressions pass. Formatting, default/all-feature workspace tests,
  strict Clippy in both configurations, all-feature check and station-label A/B
  pass. Loopback fixtures used local socket permission.
- These checks validate policy and diagnostics only; actual receiver reason
  codes and reconnection behavior still need physical nRSP-ST acceptance.
- The test increment is committed as `fdfd607` and pushed to
  `codex/station-label-layout-ab`. The focused station-label A/B regression was
  rerun on this source and passed (1 test): at 295 px, full category fits 24/25
  rows, names-only and compact-category fit 25/25; at 340 px, all fit 25/25.
  Compact `CLASS` still saves 32 text pixels on WCRB. This is layout evidence,
  not listener preference evidence.
- PR #3 remains open and mergeable. Its current head has no combined status
  checks; Actions run `40760314846` ended in `startup_failure` before creating
  any jobs. Remote CI is unverified, not a code-test failure. Next, inspect the
  checked-in receiver wire contract for any safe handshake-stage fixture, then
  continue physical receiver/output/Pi acceptance.

## nRSP handshake cancellation during Hello wait (2026-10-28)

- Added a loopback regression that completes the native WebSocket handshake,
  withholds the receiver's initial Hello, then cancels the client while it is
  waiting for that response. The client returns `Cancelled` within a 500 ms
  bound and the server observes the connection close. This exercises the real
  `Client::connect` read loop, not only the pre-cancelled fast path.
- The focused test passed. The full default and all-features workspace suites,
  formatting, strict Clippy in both configurations, all-features check and the
  station-label A/B regression passed. Socket fixtures required local-network
  permission. No physical receiver behavior is inferred.
- Code commit `c66b883` is pushed to the PR branch. PR #3 is open and mergeable;
  current combined status is empty and no associated PR workflow run was
  visible at query time. Remote CI remains unverified. Next, continue receiver
  and device recovery acceptance, which requires physical hardware.

## IP output-loss retry cleanup fixture (2026-10-28)

- Extracted the IP playback retry wrapper so deterministic failure injection
  exercises the production cleanup path. A synthetic Bluetooth output failure
  returns as `OutputUnavailable`, makes one attempt, never invokes the retry
  wait, and clears the active output-progress handle. The retired callback
  counter remains detached from the session's current progress slot.
- The focused regression and all six engineering gates passed: formatting,
  default/all-feature workspace tests, strict Clippy in both configurations and
  all-feature check. The compact station-label A/B regression passed again.
  Loopback fixtures used local socket permission. No physical output device was
  involved.
- Code commit `4a8cc57` is pushed. PR #3 is open and mergeable; the current head
  has no status checks or associated workflow run visible at query time. The
  preceding head's Actions run failed before jobs were created; remote CI is
  unverified. Next: verify Bluetooth/USB loss and replacement-device recovery
  on real hardware while retaining these deterministic lifecycle checks.

## Bounded catalog revalidation and label A/B (2026-10-28)

- The first bounded `validate-all --json` run under the restricted sandbox
  returned `Unavailable / not_checked` for all 25 stations after two
  attempts each. A publisher playlist HEAD request succeeded when permitted,
  so those results were treated as sandbox network restrictions, not station
  failures.
- Repeated the same silent validator with outbound network permission. At
  2026-10-28 20:16:30 UTC, all 25 stations decoded supported audio in one
  attempt each (25/25 at `decoded_audio`). This is point-in-time codec
  evidence only; no audio device was opened and no redistribution permission
  was established.
- Re-ran the station-label A/B regression: at 295 px the full category labels
  fit 24/25 rows while names-only and compact labels fit 25/25; at 340 px all
  variants fit 25/25. `CLASS` saves 32 text pixels on WCRB. These are layout-fit
  measurements, not listener preference evidence.
- Current PR code head `9647b3e` has an empty combined status; Actions run
  `40761496510` failed at startup before creating jobs. Remote CI remains
  unverified. Next: run physical output-device recovery acceptance when
  Bluetooth/USB hardware is available.

## Output progress isolation across a network retry (2026-10-28)

- Expanded the synthetic IP recovery regression to sequence a network transport
  failure followed by output-device loss. It verifies the first callback
  progress handle is cleared before the one-second retry wait and remains
  detached before the second attempt starts.
- The retry creates a distinct progress counter. When that attempt reports
  Bluetooth output loss, recovery returns `OutputUnavailable` without another
  retry wait and clears the second handle. This covers cross-attempt cleanup
  through the same helper used by production IP playback.
- Focused coverage and all six local gates passed: formatting, default and
  all-feature workspace tests, strict Clippy in both configurations and
  all-feature check. The compact station-label A/B regression passed again.
- Code commit `73d7eb2` is pushed. PR #3 is open and mergeable; its current head
  has no status checks or associated workflow run visible at query time. Prior
  run `40761915140` failed before creating jobs. Remote CI remains unverified.

## Listener guidance for native output setup errors (2026-10-28)

- The listening screen already mapped common CPAL output errors to an action,
  but uncommon native setup messages for unsupported sample formats, receiver
  PCM conversion and CPAL stream construction fell through to raw diagnostics.
- Added these concrete native-output failure strings to the output-selection
  guidance. Exact worker error text remains available in setup for diagnosis.
- The focused presentation regression passed. Formatting, default/all-feature
  workspace tests, strict Clippy in both configurations, all-feature check and
  the compact station-label A/B test passed.
- Code commit `176e169` is pushed. PR #3 is open and mergeable; current combined
  status is empty with no workflow run visible at query time. Prior Actions run
  `40762398646` failed before job creation, so remote CI remains unverified.

## nRSP initial Hello validation and timeout coverage (2026-10-28)

- Added a loopback protocol regression proving connection setup rejects a
  non-Hello first frame and returns `Timeout` when the WebSocket opens but the
  receiver never sends its required initial Hello. The timeout fixture uses a
  50 ms deadline and completes within a 500 ms bound.
- The focused test passed. All six local gates passed: formatting, default and
  all-features workspace tests, strict Clippy in both configurations and
  all-features check. The station-label A/B test passed again: at 295 px full
  category labels fit 24/25 while names-only and compact labels fit 25/25; all
  variants fit at 340 px. These are layout-fit measurements, not listener
  preference evidence. Loopback fixtures required local socket permission.
- No receiver hardware was exercised. The current PR head's Actions run
  `40762846108` failed at startup before creating jobs; remote CI remains
  unverified. Physical nRSP reconnect and output-device recovery acceptance
  remain open.
