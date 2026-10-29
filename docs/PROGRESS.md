# Radio development progress

Updated 2026-10-28. Active priorities and ownership are in the
[roadmap](PORTING_PLAN.md); embedding commands are in [the module contract](EMBEDDING.md).

## Permanent classification for blocked DNS resolutions (2026-10-28 23:40 UTC)

- `SafeResolver` now emits a typed marker when a host resolves to a non-public
  address. The reqwest error chain preserves that marker, so transport returns
  a permanent endpoint-policy failure instead of spending network retries.
  Ordinary DNS timeouts remain transient and retain bounded recovery.
- Added a reqwest-backed fixture test proving both classifications. The focused
  regression and all six required gates passed: format, default/all-feature
  workspace tests, strict Clippy in both configurations and all-feature check.
- Repeated GUI station-label A/B/C: at 295 px A full category 24/25, B names-only
  25/25, C compact category 25/25; at 340 px all fit. C saves 192 px and A
  overflows `wcrb`. This is layout fit evidence, not preference data.
- Fresh bounded `validate-all --json` at 2026-10-28 23:40:40 UTC decoded 25/25
  feeds in 25 attempts with no retries/failures. Point-in-time codec evidence
  only; no audio output opened.
- Implementation commit `878c4ac` and checkpoint `3926f84` are pushed to open,
  mergeable PR #5. No workflow run or combined status was observed for this
  head at 23:41 UTC; physical output/Pi/receiver, publisher rights, and native
  Windows MSVC acceptance remain open.
- Next: continue tested playback lifecycle or transport recovery improvements;
  pursue actual device-loss acceptance when target hardware is available.

## Publisher-domain trailing-dot guard (2026-10-28 23:34 UTC)

- Direct URL validation previously blocked `somafm.com` and its subdomains but
  accepted absolute DNS spellings such as `somafm.com.`. The policy check now
  strips terminal dots before matching the publisher domain. Regression cases
  cover uppercase root and dotted subdomain forms; deceptive lookalike domains
  remain allowed by the specific domain rule.
- Focused URL-policy test passed. All six local gates passed: format, default
  and all-feature workspace tests, strict Clippy for both configurations, and
  all-feature check. One manual native-output test remains ignored.
- Bounded `validate-all --json` at 2026-10-28 23:34:35 UTC decoded 25/25
  catalog entries in 25 total attempts, with no retry or failure. This is
  point-in-time codec evidence only; no audio device opened.
- Repeated station-label A/B/C: at 295 px, A full category 24/25, B names-only
  25/25, C compact category 25/25; at 340 px all variants fit. C saves 192 px;
  A overflows `wcrb`. Layout fit only, not listener preference.
- Implementation commit `d019cbe` and checkpoint `261a8e0` are pushed to open,
  mergeable PR #5. Actions run 278 (`40788520012`) for checkpoint `261a8e0`
  failed before job creation (`startup_failure`); combined statuses are empty.
  Physical output/Pi/receiver, rights and native Windows MSVC acceptance remain
  open.
- Next: continue source-aware recovery or cancellation work; pursue target-device
  output recovery when hardware is available.

## Output startup error cleanup racing with Stop (2026-10-28 23:29 UTC)

- Fixed `start_resource_if_active`: output-factory errors now clear callback
  progress, and a concurrent Stop suppresses that stale startup error. If the
  session remains active, the original output error is preserved. Added focused
  regressions for both outcomes.
- Focused regressions and all six local gates passed: format, default and
  all-feature workspace tests, strict Clippy in both configurations, and
  all-feature check. Windows GNU test executables compiled for default,
  SDRconnect and direct nRSP-native; they were not run.
- Repeated GUI station-label A/B/C: at 295 px A full category fits 24/25, B
  names-only and C compact category fit 25/25; all fit at 340 px. C saves
  192 px vs A; A overflows `wcrb`. This is layout evidence, not preference data.
- Source commit `03e3025` and checkpoint `1a8c307` are pushed to open,
  mergeable PR #5. Actions run 274 (`40787809365`) for checkpoint `c3f2798` failed before job
  creation (`startup_failure`); combined statuses are empty. Native Windows
  MSVC CI remains unverified.
- Next: continue playback lifecycle race coverage; physical output/Pi/receiver
  operation and publisher-rights review remain open acceptance gates.

## Cancellation-safe native output startup (2026-10-28 23:25 UTC)

- Added `start_resource_if_active` around native output creation. Stop set before
  startup skips the output factory; Stop arriving while startup runs drops the
  newly created output before the worker proceeds, and clears the callback
  progress slot. Synthetic RAII/counter tests cover both boundaries.
- Focused tests passed. All six local gates passed for this implementation;
  after a final tuple simplification, formatting and both focused startup tests
  passed again. Windows GNU workspace test binaries compiled for default,
  SDRconnect and direct nRSP-native; they were not run.
- Repeated GUI label A/B/C: at 295 px A full category fit 24/25, B names-only
  and C compact category 25/25; all fit at 340 px; C saves 192 px. Layout fit
  only, not listener preference evidence.
- Source commit `6069543` is pushed to PR #5. GitHub's PR API currently reports
  the head mergeable/clean, although one normalized connector read briefly
  returned `mergeable=false`; no workflow run or combined status was observed.
- Next: exercise output-factory failure concurrent with Stop and verify the
  callback progress slot clears on failure; physical output/Pi/receiver and
  publisher-rights gates remain open.

## Decoder probe cancellation guard (2026-10-28 23:21 UTC)

- `decode_and_play` now exits before opening output if Stop is already set.
  Format probing checks cancellation before starting and after the synchronous
  probe returns, so a cancellation-triggered reader error is discarded instead
  of being surfaced/classified as a transient stream failure. Added focused
  tests for pre-probe Stop and Stop during a probe I/O error.
- Focused probe regressions passed. All six required local gates passed:
  formatting, default/all-feature tests, strict Clippy for both configurations
  and all-feature check. Windows GNU test binaries compiled for default,
  SDRconnect and direct nRSP-native; they were not run.
- Repeated GUI label A/B/C: at 295 px, A full-category 24/25, B names-only and
  C compact-category 25/25; all variants fit at 340 px; C saves 192 px. This is
  layout-fit evidence, not listener preference evidence.
- Source commit `2cad1cb` is pushed to open, mergeable PR #5. No workflow run or
  combined status was observed for this head; previous run 264 failed before
  job creation (`startup_failure`).
- Next: add cancellation coverage for shutdown during native output startup and
  check that stale progress cannot reappear; physical output, Pi/receiver and
  publisher-rights gates remain open.

## Cancellation after audio headers (2026-10-28 23:16 UTC)

- Added a loopback HTTP fixture that returns successful audio headers then
  stalls before sending body bytes. Canceling the session aborts the first
  reader wait and reaps the transport worker within the bounded 500 ms check.
  This complements pending-header cancellation coverage.
- Focused integration test passed; all six local gates passed: formatting,
  default/all-feature workspace tests, both strict Clippy runs and all-feature
  check. Windows GNU test binaries compiled for default, SDRconnect and direct
  nRSP-native; they were not run.
- Repeated GUI label A/B/C: at 295 px, A full category fit 24/25, B names-only
  and C compact category 25/25; all fit at 340 px; C saves 192 px. Layout fit
  evidence only, not listener preference.
- Source commit `3264bc7` is pushed to PR #5, which is open and mergeable. No
  run or combined status was observed for this head; previous run 264 failed
  before job creation (`startup_failure`).
- Next: inspect cancellation during decoder probing and output startup, then
  continue physical Bluetooth/USB, Pi, receiver and publisher-rights acceptance.

## Stop racing with playback retry (2026-10-28 23:14 UTC)

- Added a deterministic regression for Stop arriving as a transient network
  failure returns from an attempt. It asserts cancellation wins before the
  retry wait, no second connection is attempted, and no stale recovery/error
  event can replace Stopped.
- Focused regression and all required local gates passed: format, default and
  all-feature workspace tests, strict Clippy for both configurations, and
  all-feature check. Windows GNU workspace test binaries compiled for default,
  SDRconnect and direct nRSP-native; they were not run.
- Repeated GUI label A/B/C: at 295 px, A full category fit 24/25, B names-only
  and C compact category 25/25; all fit at 340 px; C saves 192 text pixels.
  This is deterministic layout evidence, not listener preference evidence.
- Source commit `0ede192` is pushed to open PR #5. Actions run 264 (`40785912226`) for this head failed with `startup_failure`;
  combined statuses are empty, so remote CI still ran no jobs.
- Highest next priority: continue playback lifecycle race coverage, then pursue
  physical Bluetooth/USB recovery, Pi and receiver acceptance, publisher rights,
  and healthy native Windows CI.

## Native output failure diagnostics (2026-10-28 23:10 UTC)

- CPAL asynchronous output errors now retain their backend diagnostic instead
  of collapsing to a generic device-failure message. Playback recovery reports
  that detail during format probing, stream decoding and sink backpressure;
  synthetic failure-path tests cover recording and propagation.
- Focused diagnostics and output recovery regressions passed. Formatting,
  default/all-feature workspace tests, strict Clippy in both configurations,
  and the all-feature check passed. Windows GNU workspace test binaries
  cross-compiled for default, SDRconnect and direct nRSP-native features;
  binaries were not run.
- Repeated station-label A/B/C: at 295 px, full categories fit 24/25,
  names-only and compact categories 25/25; at 340 px all fit. Compact labels
  save 192 text pixels. This measures layout fit, not listener preference.
- Source commit `e0eebfc` is pushed to open, mergeable PR #5. No workflow run or
  commit status was visible for this head when checked; earlier PR run 259
  failed before job creation. Local validation is not remote CI evidence.
- Physical Bluetooth/USB output loss, Pi runtime, receiver hardware,
  redistribution rights and healthy Windows MSVC CI remain open.

## Live volume changes across output formats (2026-10-28 23:05 UTC)

- Added a callback regression that changes volume between successive buffers
  for each of the eleven active CPAL 0.16 PCM formats. It verifies full volume,
  half volume, mute, output scaling and callback-consumed sample counts.
- Focused regression passed. Formatting, default/all-feature workspace tests,
  strict Clippy gates and all-feature check passed. Windows GNU workspace test
  binaries compiled for default, SDRconnect and direct nRSP-native features;
  they were not run.
- Repeated the station-label A/B/C comparison: at 295 px, full categories fit
  24/25 and names-only/compact labels fit 25/25; all fit at 340 px. Layout
  evidence only, not listener preference.
- Source commit `002c86f` is pushed to open, mergeable PR #5. Actions run 259
  for that source failed at startup before creating jobs, with no statuses.
  The final evidence checkpoint commit does not have an observed run yet.
- Next: continue synthetic playback/output-recovery testing. Physical
  Bluetooth/USB, Pi, receiver, publisher-rights and healthy Windows CI gates
  remain open.

## Native output gain coverage (2026-10-28 23:01 UTC)

- Extended the generic CPAL callback regression so all eleven active PCM
  formats are checked at 100%, 50%, and 0% volume, with full-scale and
  intermediate samples plus consumed-sample accounting. This guards mute and
  attenuation parity across signed, unsigned and floating device outputs.
- Focused test, both default/all-feature workspace suites, strict Clippy gates,
  all-feature check and formatting passed. Windows GNU test binaries compiled
  for default, SDRconnect and direct nRSP-native configurations; not executed.
- Repeated the GUI label A/B/C fit comparison: at 295 px, full categories fit
  24/25 while names-only and compact labels fit 25/25; at 340 px all fit.
  Layout fit only, not listener preference.
- Source commit `452f444` is pushed to open, mergeable PR #5. The previous
  checkpoint's Actions run 255 failed at startup before creating jobs; the
  source head's run 256 also failed at startup before jobs, with no statuses.
  The final evidence checkpoint commit does not have an observed run yet.
- Next: continue source-driven recovery coverage; physical audio/Pi/receiver
  acceptance, distribution rights and healthy Actions/MSVC validation remain.

## Native output sample-format coverage (2026-10-28 22:55 UTC)

- Native CPAL output now dispatches to render callbacks for all eleven sample
  formats exposed by CPAL 0.16, including 24-bit integer PCM and 64-bit
  integer/float formats. Previously unhandled device defaults failed as
  unsupported even when the backend could convert them.
- A focused conversion regression covers each supported type and confirms
  callback consumption accounting at full scale (-1.0 and 1.0), zero and
  intermediate values. CPAL 0.16 exposes eleven active PCM formats; U24 is
  present in its sample wrapper crate but excluded from CPAL's active format
  enum. The default and all-feature workspace suites, both strict Clippy
  configurations, formatting, all-feature check and diff validation passed.
  One manual physical audio test remains ignored.
- Repeated the 25-station station-label A/B/C fit test: at 295 px, full genre
  labels fit 24/25 and names-only/compact labels fit 25/25; all fit at 340 px.
  This is layout evidence only, not listener preference.
- Windows GNU workspace test binaries cross-compiled for default, SDRconnect
  and native nRSP feature combinations; compiled test programs were not
  executed. Physical device acceptance remains open.
- PR #5 is open and mergeable at `dae41ee`. Actions run 251 for this commit
  failed before job creation (`startup_failure`) and combined statuses are
  empty, so remote CI and native MSVC validation remain unverified.
- Expanded the callback conversion fixture to check full-scale -1.0 and 1.0,
  zero and intermediate values; repeated the full default/all-feature gates,
  Windows GNU test-binary cross-compiles and station-label A/B/C. PR #5 remains
  open and mergeable at `26e2066`; Actions run 253 also failed at startup with
  no combined statuses.
- A fresh bounded `validate-all --json` sweep at 2026-10-28 22:58:15 UTC
  decoded all 25 stations in 25 attempts, with no retries or failures. This is
  point-in-time codec evidence only. PR #5 remains open and mergeable at
  `a5df6ff`; Actions run 254 failed before jobs started, with no statuses.
- Next: continue physical output-loss recovery and Pi/receiver acceptance,
  publisher-rights review and remote Windows MSVC CI when Actions starts jobs.

## Interactive CLI input error handling (2026-10-28 22:48 UTC)

- Follow-up review found the bounded line reader classified every
  `InvalidData` I/O error as an oversized command. A persistent stdin failure
  could therefore be reported repeatedly instead of terminating the reader.
- The reader now distinguishes its overlong-line outcome from underlying I/O
  errors. Oversized input is still drained and reported; actual read failures
  now print the underlying diagnostic and stop the input worker.
- Focused regressions cover an over-limit line followed by valid commands and
  a reader that itself returns `InvalidData`. Default and all-feature workspace
  test suites, formatting, both strict Clippy gates and all-feature check pass.
- A/B/C layout fit was repeated: at 295 px, full-category labels fit 24/25,
  names-only and compact labels fit 25/25; at 340 px all fit. This remains
  layout evidence, not listener preference.
- Windows GNU `cargo test --workspace --no-run` passed for the default,
  SDRconnect and direct nRSP-native feature configurations after the reader
  fix; executables were cross-compiled but not run. Repeated the compile after
  adding stdin error diagnostics.
- Next: continue physical audio, Pi, receiver/USB and publisher-rights gates;
  rerun Windows MSVC CI when Actions creates jobs.

## Live catalog and label comparison recheck (2026-10-28 22:43 UTC)

- A fresh bounded `validate-all --json` run decoded all 25 catalog streams
  in 25 total attempts. This is point-in-time codec evidence, not audible
  playback or permission to redistribute or relay the streams.
- Repeated the station-label fit comparison on all 25 entries. At 295 px,
  full-category labels fit 24/25 while names-only and compact labels fit
  25/25; all variants fit at 340 px. The compact variant preserves a short
  category cue and saves 192 text pixels versus full category labels. This is
  a rendered-layout measurement, not listener preference evidence.
- Next: retain this freshness evidence as historical only; complete physical
  audio/output, Pi, receiver/USB and publisher-rights acceptance separately.

## Bounded interactive CLI input (2026-10-28 22:42 UTC)

- Interactive `listen` previously used `stdin.lines()` and an unbounded channel;
  a huge line or rapid piped command stream could grow memory without limit.
- Each command is now limited to 4 KiB and the reader-to-monitor queue holds at
  most eight commands. An overlong line is drained and reported, and later
  commands remain usable.
- Focused regression passed for oversized-line rejection, exact-limit command
  acceptance, CRLF handling and recovery to a following command. Full default
  and all-feature workspace tests passed, along with format, both strict
  Clippy gates and all-feature check. Loopback fixtures needed temporary
  elevated socket permission; one existing native audio test remains ignored.
- Station-label fit A/B/C repeated on all 25 entries: 24/25 full-category
  labels vs 25/25 names-only and compact labels at 295 px; all fit at 340 px.
  This measures fit only, not listener preference.
- The local branch is cleanly based on `main` and 21 commits ahead. PR #5 is
  open and mergeable; latest Actions run 245 ended in `startup_failure` before
  any job started, leaving remote CI and native MSVC execution unverified.
- Next: complete physical output, Pi, receiver and USB acceptance; review
  distribution rights; rerun native Windows CI when Actions starts jobs.

## Windows test-binary and station-label recheck (2026-10-28 22:37 UTC)

- Local Windows GNU `cargo test --workspace --locked --no-run` passed for default, SDRconnect and direct nRSP-native workspace feature configurations. All workspace, GUI, CLI, receiver and integration-test executables linked successfully, but they were not run. The native Windows MSVC job now runs those three suites.
- Re-ran the GUI label comparison: at 295 px A(full category) fits 24/25, B(names-only) 25/25, C(compact category) 25/25; at 340 px all variants fit. C saves 192 px; layout fit only, not listener preference.
- PR #5 was open and mergeable at `2936b84`. Actions run 244 failed before creating jobs and reported no commit status. Native Windows execution remains unverified.
- Next: obtain a healthy runner for the Windows tests; preserve physical output, Pi/GUI/touch, receiver/USB, and broadcaster-rights acceptance as open gates.

## Windows cross-target coverage (2026-10-28 22:33 UTC)

- Added a `windows-latest` CI job that compile-checks, strictly lints, and runs the workspace tests for the default, SDRconnect, and direct nRSP-native CLI/GUI feature combinations with Cargo.lock enforced. This uses the runner's native MSVC environment.
- Installed MinGW locally and passed all three equivalent locked `x86_64-pc-windows-gnu` workspace checks and strict Clippy runs. These are cross-compilation evidence only; no Windows executable was run and this does not substitute for the native MSVC CI job.
- Ruby parsed the workflow YAML and confirmed `jobs.windows` is present. `cargo fmt --all -- --check` and `git diff --check` passed.
- PR #5 was open and mergeable at pre-test head `da9301f`. Actions run 243 failed before job creation, with no combined status. The expanded Windows MSVC compile, Clippy and test job remains unverified until a runner starts.
- Next priority: get the new platform job through remote CI, then complete physical output/Pi/receiver acceptance and publisher distribution-rights review.

## Fresh 25-feed sweep and label-fit rerun (2026-10-28 22:25 UTC)

- The bounded CLI decoder validator checked all 25 curated stations at 22:25:38 UTC. All 25 decoded, in 25 total attempts, with no retry or failure. This is point-in-time decoder evidence only and does not establish physical output.
- Re-ran the GUI station label A/B/C regression: at 295 px, A full category 24/25, B names-only 25/25, C compact category 25/25; at 340 px all variants fit 25/25. C saves 192 text pixels; this is layout fit, not preference evidence.
- No catalog or playback code changes were warranted by this sweep. Current PR #5 was open and mergeable at code head `4e8c8c9`; its latest Actions run 240 failed before job creation with no commit status. The new checkpoint commit needs a fresh CI observation.
- Next priority: physical output/Pi/receiver acceptance when available and runner-side diagnosis; preserve the current 25-entry set absent new source or rights evidence.

## Input scan cancellation and evidence cleanup (2026-10-28 22:21 UTC)

- Added a bounded lifecycle regression for the discovery worker: the test blocks synthetic driver work, drops its `InputScan`, confirms Drop returns without waiting, and then verifies the worker observes cancellation. The audio crate test and both workspace suites pass with it.
- Full required local gates passed: format, default tests, strict default Clippy, all-feature tests, strict all-feature Clippy, and all-feature check. Headless AArch64 core-only, SDRconnect and nRSP-native checks also passed. One physical native audio test remains ignored.
- Corrected the standalone acceptance record to use the existing 25/25 bounded decoder sweep at 22:14:21 UTC, and corrected catalog policy wording to match the configured six-hour/manual workflow. The schedule remains inactive until merged to the default branch. Fixed duplicate roadmap numbering.
- PR #5 remains open and mergeable at `ae5028d`. Actions run 239 on that head failed at startup with zero jobs and no combined status. The remote runner failure has no repository-side job diagnostics; do not infer a code failure.
- The production goal remains active. Next priorities: physical output/device gates and a runner-side explanation for repeated startup failures; retain the 25-entry catalog unless new evidence warrants change.

## Twenty-five publisher-documented feeds (2026-10-28 22:14 UTC)

- Added Classical California/KUSC, KUNC News, The Colorado Sound, and KUAF main, Classical and Jazz services. Publisher listening pages explicitly support third-party radio players or internet radios and provide the direct endpoints. Runtime allowlists pin exact HTTPS host/path pairs for StreamTheWorld, StreamOn and StreamGuys; cross-feed paths and unsafe schemes are rejected. These publisher playback listings do not grant redistribution or relay rights.
- The runtime directory and JSON mirror now contain 25 entries. A bounded live sweep at 2026-10-28 22:14:21 UTC decoded 25/25 streams, one attempt each. This is point-in-time decoder evidence, not physical output acceptance.
- Focused exact-endpoint boundary tests passed. Catalog mirror parity and JSON parsing passed.
- Station label A/B/C at 295 px: full category 24/25; names-only 25/25; compact category 25/25. At 340 px all variants fit 25/25. Compact category saves 192 aggregate text pixels; `wcrb` is the full-category overflow. This evaluates layout fit, not listener preference.
- Full workspace tests, formatting, strict default and all-feature Clippy, all-feature tests/check, and headless AArch64 Linux checks (core-only, SDRconnect, nRSP-native) passed. The manual native-audio test remains ignored; no physical audio, receiver or Pi acceptance was performed.
- Highest priority: preserve the source/rights evidence and complete output/device acceptance; national continuous NPR feed rights and remote Actions startup remain unresolved. The latest observed PR run for commit `78ac1aa` (run 40778857320, attempt 1) ended in `startup_failure` before creating any jobs; commit status and check diagnostics were empty. The final checkpoint commit requires its own remote CI verification.

## Four WFMU feeds and catalog validation (2026-10-28 22:10 UTC)

- Added WFMU Freeform, Give the Drummer Radio, Rock'n'Soul Radio and Sheena's
  Jungle Room from the publisher's official 128 kbps player links. Inspected
  each official PLS file and selected only its fixed HTTPS MP3 endpoint; the
  legacy plaintext alternatives are not accepted. The allowlist binds every
  channel ID to `stream0.wfmu.org` and its exact path.
- The runtime and JSON catalog now contain nineteen documented feeds. A
  bounded sweep at 2026-10-28 22:08:28 UTC decoded all nineteen in one attempt
  each. All four WFMU feeds decoded stereo MP3 at 44.1 kHz. This is
  point-in-time decoder evidence only.
- Added a regression covering each approved WFMU host/path pair and rejecting
  plaintext, an alternate hostname, a cross-channel path and a deceptive path.
  The focused test and catalog mirror parity test passed.
- Station label A/B/C at 295 px: full category 18/19; names-only 19/19;
  compact category 19/19. At 340 px all variants fit 19/19. Compact category
  saves 128 aggregate text pixels. This is layout fit, not listener preference.
- All six Rust gates passed: format, default tests, strict default Clippy,
  all-feature tests, strict all-feature Clippy and all-feature check. Headless
  AArch64 Linux checks passed for core-only, SDRconnect and direct nRSP-native.
  One manual native audio output test remains ignored; no hardware acceptance
  was performed.
- Highest priority: add six more diverse, publisher-documented feeds toward
  the ~25 target; retain hardware acceptance and remote Actions startup as
  separate gates.

## WRVO public-radio catalog addition (2026-10-28 22:04 UTC)

- Added WRVO 1 NPR News, WRVO 2 PRX Remix and WRVO 3 BBC World Service. WRVO's
  official listening page publishes direct third-party-player MP3 links, and
  its stream page identifies each service. This adds a named NPR-member news
  stream; it does not establish a national continuous NPR feed or grant
  redistribution rights.
- WRVO uses numbered `live.streamtheworld.com` redirect shards. The endpoint
  validator now accepts only all-numeric shard names paired with the exact
  stream-specific MP3 path; cross-stream and deceptive-host regression cases
  are rejected.
- A bounded live decoder sweep at 2026-10-28 22:01 UTC decoded 15/15 entries in
  one attempt each. WRVO 1 decoded mono MP3/44.1 kHz; WRVO 2 and 3 mono MP3/
  22.05 kHz. This is point-in-time codec evidence only.
- Station label A/B/C at 295 px: A full category 14/15, B names-only 15/15,
  C compact category 15/15. At 340 px all variants fit 15/15. C saves 128
  aggregate text pixels against A. These are layout-fit measurements, not
  listener preference evidence.
- Focused StreamTheWorld redirect regression passed. Default and all-feature
  workspace suites passed (including 57 core tests; one manual audio output
  test ignored). Both strict Clippy runs, format check and all-feature check
  passed. Core-only, SDRconnect and direct nRSP-native AArch64 Linux headless
  compile checks also passed; this is not Pi runtime acceptance.
- Catalog label A/B/C passed at 295 px: A full category 14/15, B names-only
  15/15, C compact category 15/15; all variants fit at 340 px. No physical
  output or receiver hardware was exercised.
- Committed as `6f11777` with checkpoint `5211b18` and pushed to PR #5. The PR
  remains open and GitHub reports it mergeable. Latest Actions run
  `40777122410` failed before job creation (`startup_failure`); combined status
  is empty, so remote CI remains unverified.
- Highest priority: source and validate more authorized diverse stations
  toward 25; separately track hardware acceptance and the recurring Actions
  startup failure.

## Publisher-supported catalog expansion (2026-10-28)

- Added seven publisher-documented endpoints, growing the directory from five
  to twelve: WPR News and Music, Radio Swiss Jazz and Classic, All Classical,
  WFMT and KFUO. Their official stream pages and exact source endpoints are
  recorded in the literature matrix. WPR News is a named regional/member
  broadcaster news service; a national continuous NPR stream remains
  unestablished. SomaFM stays excluded pending written permission.
- The first live sweep decoded 10/12; both Swiss stations returned
  `UnsafeEndpoint` after publisher URLs redirected to numbered
  `livestreaming-node-*.srg-ssr.ch` hosts. A bounded header check confirmed the
  pattern. The allowlist now accepts only numeric node labels and the exact
  station-specific paths; unsafe suffixes, labels and cross-station paths have
  regression coverage.
- The repeated bounded live sweep decoded 12/12 stations in one attempt each
  at 2026-10-28 21:54 UTC. Decoded formats: Skylab MP3/48 kHz; WCRB AAC/48;
  GBH News AAC/32; KEXP AAC/44.1; Radio Paradise, WPR News, WPR Music, Radio
  Swiss Jazz, Radio Swiss Classic and All Classical MP3/44.1 or 48; WFMT
  AAC/48; KFUO AAC/44.1. This is point-in-time decoder evidence only.
- Repeated GUI label A/B/C with the expanded roster: at 295 px, A full category
  fit 11/12, B names-only and C compact category fit 12/12. At 340 px all three
  fit 12/12. C saves 128 aggregate text pixels across the four classical
  stations. This is layout-fit evidence, not listener preference.
- `cargo test -p rust-radio-core` passed (47 tests); the focused GUI A/B/C test
  passed; the catalog policy mirror was updated. The expanded sources were
  subsequently rechecked with the full workspace gates. No physical playback
  or hardware was exercised.
- Highest priority: complete full validation and PR update for this increment,
  then keep expanding toward 25 with first-party authorized diverse sources.

## Publisher rights review and safe catalog correction (2026-10-28 21:49 UTC)

- Rechecked the catalog after finding SomaFM's current Terms of Service reject
  new third-party radio clients and require written permission for embedding
  or restreaming. Removed the 23 SomaFM presets and the SomaFM-only legacy HTTP
  playlist rewrite. Direct SomaFM URLs are explicitly denied by custom input
  validation, preventing accidental use as a user-provided endpoint.
- Retained five endpoints whose publishers document third-party radio player
  playback: Skylab Radio, WCRB Classical, GBH 89.7 News Radio, KEXP 90.3 FM and
  Radio Paradise Main Mix. This evidence is limited to end-user playback; the
  catalog continues to state redistribution and relay are not authorized.
- Updated the Rust directory and JSON mirror with third-party-player guidance
  flags and review dates, plus policy, transport, GUI and literature docs. A
  regression rejects publishers lacking this evidence and confirms mirror
  parity. The longer-term target of approximately 25 diverse stations remains
  open; NPR News still needs a named authorized broadcaster stream.
- Focused audio validation fixtures passed (8/8). The layout A/B/C test passed:
  at 295 px A full category fit 4/5, B names-only and C compact category fit
  5/5; all three fit 5/5 at 340 px. C saves 32 px over A. These are fit
  measurements, not listener preference evidence.
- All six required gates passed: format, workspace tests, strict Clippy,
  all-feature tests, all-feature strict Clippy and all-feature check. Workspace
  tests include loopback stream tests; sandbox loopback denial was followed by
  a successful run with local-socket permission. One manual native-output test
  and one physical Soapy output test remain intentionally ignored.
- Fresh bounded silent validation at 2026-10-28 21:48 UTC decoded 5/5 entries,
  one attempt each, at `decoded_audio`: Skylab MP3 48 kHz, WCRB AAC 48 kHz,
  GBH News AAC 32 kHz, KEXP AAC 44.1 kHz and Radio Paradise MP3. This is
  point-in-time codec evidence only.
- Current branch is `codex/catalog-publisher-permissions`, based on merged
  source commit `5568fb3`. PR #5 is open and mergeable at `fcf7c51`. Actions
  run `40775321206` for its preceding head `43a0763` failed at startup before
  job creation, matching the existing runner failure; local required gates all
  passed and no run is visible yet for `fcf7c51`. PR #4 remains open
  with docs-only CI evidence, and PR #2 remains open with its earlier catalog
  proposal. Both were left unchanged.
- Highest priority: research and validate additional authorized station
  sources without restoring feeds whose terms disallow third-party clients;
  then revisit required physical output, Pi and receiver acceptance.

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
- No receiver hardware was exercised. The preceding PR head's Actions run
  `40762846108` failed at startup before creating jobs. Documentation head
  `fe8ae3c` also has an Actions run (`40763293734`) that failed at startup with
  zero jobs; its combined status has no checks. Remote CI remains unverified.
  Physical nRSP reconnect and output-device recovery acceptance remain open.

## nRSP WebSocket upgrade deadline and catalog recheck (2026-10-28)

- Added a loopback case where a peer accepts TCP but leaves the HTTP WebSocket
  upgrade unanswered. `Client::connect_until` returns `Timeout` at its 50 ms
  deadline and within a 500 ms test bound, covering the earlier handshake
  stage before the initial-Hello wait.
- The focused timeout test passed. All six local gates passed on implementation
  source `882e9c0`: formatting, default/all-features workspace tests, strict
  Clippy in both configurations and all-features check. The station-label A/B
  regression passed again: at 295 px the full-category variant fits 24/25,
  names-only and compact category fit 25/25; all fit at 340 px. This measures
  layout fit, not listener preference. Loopback fixtures used local socket
  permission.
- Fresh bounded silent catalog validation at 2026-10-28 20:28:20 UTC decoded
  all 25 stations in 25 attempts. This is point-in-time codec evidence only;
  it does not establish audible output or redistribution permission.
- No hardware was exercised. The pushed code head has no workflow run visible;
  the preceding documentation head's Actions run `40763293734` failed before
  creating jobs. Remote CI remains unverified. Physical nRSP reconnect and
  output-device recovery acceptance remain open.

## Headless nRSP Linux ARM64 portability check (2026-10-28)

- On the current implementation source, `cargo check --locked -p rust-radio
  --no-default-features --features nrsp-native --target
  aarch64-unknown-linux-gnu` passed. This verifies the headless direct nRSP
  module compiles for the Pi's Linux ARM64 target; it does not establish native
  audio, device interoperability, or physical Pi runtime behavior.
- Re-ran the station-label A/B regression: at 295 px full category fits
  24/25 rows while names-only and compact category fit 25/25; at 340 px all
  three variants fit 25/25. This remains layout-fit evidence, not a listener
  preference test.
- PR #3 is open and mergeable. No workflow run or combined check is visible
  for the current PR head; remote CI remains unverified after repeated startup
  failures with no jobs. Next: resolve the first-party station rights gate or
  perform receiver/output/Pi acceptance when the relevant hardware is available.
## NPR stream identity and permission gate (2026-10-28)

- Reviewed NPR's official SmartAudio guide: “play NPR” streams the listener's
  local member station live, while “play the news from NPR” is a personalized
  national/local mix powered by NPR One. This source does not identify a single
  continuous national NPR News stream or grant embedding/redistribution rights.
- Updated the literature matrix to require a named member station, its own
  first-party direct endpoint and applicable reuse terms before catalog entry.
  NPR CDS marks live broadcast audio with `isStreamable`; premium enclosures
  require filtering unless express permission applies. This does not itself
  establish permission for this product.
- The NPR News entry remains out of the curated catalog pending endpoint and
  rights review. Next: identify a member station with an official direct stream
  and terms that permit the intended integration, or keep this gate open.
- Re-ran the label A/B regression: at 295 px full category fits 24/25, while
  names-only and compact category fit 25/25; all variants fit 25/25 at 340 px.
  These results measure layout fit, not listener preference.

## Fresh bounded catalog sweep (2026-10-28)

- Re-ran `cargo run --locked -p rust-radio-cli -- validate-all --json` at
  2026-10-28 20:34:19 UTC. All 25 curated streams decoded supported MP3/AAC
  audio in one attempt each (25/25 at `decoded_audio`). The earlier sweep at
  20:28:20 UTC was also 25/25; both are point-in-time codec checks, not audible
  playback, long-term uptime, or distribution-rights evidence.
- Re-ran the station-label A/B test. At 295 px full category fits 24/25,
  names-only and compact category fit 25/25; all three fit 25/25 at 340 px.
  This is layout-fit evidence, not listener preference.
- No stream or authorization policy changed. Next: obtain written rights for a
  named NPR member station or preserve that gate, and continue physical device
  acceptance when hardware is available.

## SDRconnect WebSocket upgrade deadline (2026-10-28)

- Bound the TCP connect phase of SDRconnect's WebSocket setup by the same
  absolute connection deadline used for the HTTP upgrade handshake. Expired
  TCP connection attempts now report the typed timeout error.
- Added a loopback regression where a TCP peer accepts the connection but
  never answers the WebSocket upgrade. The client returned `Timeout` at the
  50 ms fixture deadline and completed within the 500 ms test bound.
- The focused regression and all six required workspace gates passed. The
  direct nRSP headless ARM64 check also passed; this is compile evidence only.
- The 25-station sweep and compact-label A/B were repeated earlier in this run;
  they establish point-in-time codec and layout-fit results, not audible or
  listener-preference acceptance.
- PR #3 remains open. No current-head workflow run was visible at the latest
  check; a prior Actions run failed before creating jobs, so remote CI is
  unverified. No receiver, audio-output, or Pi hardware was exercised.
- Next: perform receiver/output/Pi acceptance when hardware is available and
  resolve publisher reuse rights before adding an NPR member stream.

## CI status refresh (2026-10-28)

- After pushing checkpoint commit `88227d5`, PR #3 remained open and
  mergeable. Its combined status was empty.
- GitHub Actions run `40765092086` ended `startup_failure` with zero jobs.
  No job logs exist, so this provides no code-level CI result; remote CI remains
  unverified. The local six-gate run on implementation source `d7c9f12` passed.

## DNS address pinning for radio HTTP (2026-10-28)

- The shared radio HTTP transport contract requires DNS/address safety, but the
  blocking probe client and live playback client previously used reqwest's
  default resolver. A public station or HTTPS redirect could therefore resolve
  to loopback or a private target, and DNS could change between a check and its
  connection.
- Added a custom resolver that rejects private, loopback, link-local,
  documentation, multicast, reserved and other special-use addresses for
  curated stations and public HTTPS streams. Reqwest connects using the checked
  address set returned by that resolver. Direct user-entered private IPv4 HTTP
  remains supported by explicit choice, with only private/loopback addresses
  allowed in that mode. Public HTTPS literal-IP redirects to non-public ranges
  are refused before connection.
- Regressions cover IPv4/IPv6 special-use ranges, localhost resolution,
  private-source exceptions and redirect target checks. `rust-radio-ip` tests,
  including loopback HTTP fixtures, passed. All six workspace gates and the
  headless nRSP ARM64 check passed.
- Implementation is committed as `d7ee9ac` on the active PR branch.
- A fresh bounded live sweep at 2026-10-28 20:46 UTC decoded all 25/25 stations
  in 25 attempts through the new resolver. This remains point-in-time codec
  evidence only.
- Repeated the station-label A/B/C test: at 295 px full category fits 24/25;
  names-only and compact category fit 25/25; all variants fit at 340 px. The
  compact variant preserves genre cues and saves 32 px; this is layout-fit
  evidence, not listener preference.
- PR #3 remains open and mergeable. Run `40766775155` for head `5f30955` failed
  at startup with zero jobs; combined status is empty and no code-level CI
  result is available. Physical audio, receiver and Pi runtime acceptance
  remains open.

## IANA address coverage and documentation reconciliation (2026-10-28)

- Checked the current IANA IPv4 and IPv6 special-purpose registries (updated
  2025-10-09) and expanded regression cases for IPv4-mapped IPv6, NAT64,
  discard-only, transition, documentation, benchmarking and reserved ranges.
  The filter deliberately blocks broad protocol-assignment aggregates rather
  than making per-address exceptions.
- Corrected stale docs: direct streams do use manually validated redirect and
  playlist hops, and CLI validation history can be persisted only when the
  caller supplies `--history PATH`.
- The focused address tests and all six workspace gates passed. The compact
  station-label A/B/C measurement repeated: 295 px full categories fit 24/25,
  names-only and compact categories 25/25; all fit at 340 px. A/B is layout
  fit, not listener preference.
- The headless nRSP ARM64 cross-check passed. No hardware was exercised.
- Address-range regression expansion is committed as `610b4c8`.
- PR #3 remains open with no reviews or inline threads. Run `40766775155` on
  head `5f30955` failed at startup with zero jobs. The new head `610b4c8` has
  no visible run or combined checks yet; remote CI remains unverified.
- Rust-Boy's local workspace still pins revision `3d77d11`; update and validate
  that integration pin only after this module's PR is accepted. Next: check
  host compatibility, then run receiver/output/Pi field acceptance.

## HTTPS literal-IP target protection (2026-10-28)

- Review found that DNS filtering did not cover a direct HTTPS URL whose host
  was already an IP literal: HTTP clients can connect to that address without
  invoking the DNS resolver. Direct input validation previously accepted such
  non-public targets.
- HTTPS direct URLs now reject literal private, loopback, link-local, shared,
  reserved, documentation, and other non-public addresses before networking.
  Public HTTPS literal IPs remain valid; explicitly entered private/loopback
  IPv4 HTTP remains available. Redirect validation uses the same policy.
- Added regressions for loopback/private IPv4, loopback/mapped/link-local IPv6,
  shared IPv4, and public IPv4. Focused test passed.
- All six workspace gates passed. The first sandboxed workspace attempt could
  not bind loopback fixtures; rerun with local socket permission passed. The
  station-label A/B remains A 24/25, B/C 25/25 at 295 px and all 25/25 at
  340 px; layout-fit evidence only. Headless nRSP ARM64 cross-check passed.
- GitHub Actions run `40767806650` remained `startup_failure` with zero jobs;
  no logs or checks expose a code-level failure. Remote CI remains unverified.
- No physical radio, audio-device, USB SDR, or Pi hardware was exercised.
  Next: investigate Actions only if repository-level diagnostics appear;
  continue hardware acceptance and station-rights review when available.

## Private-stream redirect boundary (2026-10-28)

- A follow-up review found that the explicit private IPv4 HTTP exception also
  enabled private DNS answers and redirects to unrelated private addresses.
  That widened a user's explicit connection choice into a broader LAN request
  surface, including custom playlist entries.
- Private HTTP now remains on the exact selected IPv4 origin and port for
  redirects and playlist URLs. DNS remains public-only for every input; the
  selected private literal does not need DNS resolution. Public HTTP remains
  forbidden, and public HTTPS destinations remain available if referenced.
- Added regressions for same-origin private paths (allowed), different private
  hosts, different ports, link-local ranges, HTTPS to the private origin, and
  localhost DNS filtering. Loopback HTTP fixtures verify that redirects and
  PLS entries to another private port are refused. Focused tests and all six
  required gates passed.
- Repeated the station-label A/B: at 295 px, full category fits 24/25 and
  names-only/compact fit 25/25; all variants fit at 340 px. This measures layout
  fit only. Headless direct nRSP ARM64 compilation passed.
- Code fix is committed as `bce23e1` on the active PR branch; no hardware was
  exercised. Next:
  pursue physical output/receiver/Pi acceptance and publisher rights review;
  monitor remote CI only if a run or diagnostic appears.

## Current catalog and PR check (2026-10-28)

- Ran `cargo run --quiet --locked -p rust-radio-cli -- validate-all --json`
  at 2026-10-28 21:10 UTC through the checked-address resolver. All 25 curated
  streams decoded supported audio in one attempt each (`decoded_audio`). This
  is point-in-time codec evidence, not audible playback or permission to bundle
  or redistribute the stream.
- Repeated the station-label A/B/C regression: at 295 px, full category fits
  24/25 labels; names-only and compact category fit 25/25. At 340 px all
  variants fit 25/25. Layout-fit evidence only, not listener preference.
- PR #3 remains open and mergeable with no reviews. Current head `b63c388`
  Actions run `40769502404` failed at startup with zero jobs; combined status is
  empty and no job logs exist. Remote CI remains unverified.
- No hardware was exercised. Next: prioritize available physical audio/output,
  receiver, Pi and USB SDR acceptance, and complete station publisher-rights
  review before adding or redistributing more feeds.

## NPR member-station candidate review (2026-10-28)

- Reviewed WBUR's official [Ways to Listen](https://www.wbur.org/ways-to-listen)
  and [content licensing](https://www.wbur.org/inside/licensing-wbur-content)
  pages. WBUR documents live listening on its own site, apps and NPR One; the
  public page does not identify a stable direct stream endpoint.
- WBUR states its audio and other materials are copyrighted and that licensing
  approvals are rare and may be declined. No product-use grant was found, so
  WBUR is not added to the bundled station catalog. This is a rights/endpoint
  gate, not a codec or availability failure.
- Keep the NPR News request open for a named member station, publisher-provided
  direct endpoint, and written permission applicable to this client.
- Corrected the Literature Matrix's stale network-security statement to match
  the current exact-origin private HTTP exception and public-only DNS policy.
- At review time PR #3 was open and mergeable at head `eabf106`. Actions run
  `40769883867` ended `startup_failure` with zero jobs and no status checks or
  logs; remote CI remains unverified.

## Catalog policy contract regression (2026-10-28)

- Strengthened the machine-readable catalog parity test to assert its policy
  fields: six-hour freshness, decoded-audio requirement, initial unverified
  state, publisher-directed endpoints, and no redistribution authorization.
  This protects the release gates from silent changes in the JSON mirror.
- Metadata completeness/code-shape assertions already cover each runtime
  station, and parity checks each JSON record against those values.
- All six workspace gates passed, including default/all-feature tests and
  strict Clippy. The station-label A/B repeated at 295 px (24/25 full category;
  25/25 names-only and compact) and 340 px (25/25 all variants); this measures
  layout fit only. Headless nRSP ARM64 compilation passed.
- Code/test commit `fbeb476`; hardware and publisher-rights acceptance remain
  open. Next: seek a first-party NPR endpoint and permission, and continue
  physical platform acceptance when hardware is available.

## Catalog freshness cadence and validation refresh (2026-10-28)

- The catalog declares a six-hour freshness TTL, while its scheduled silent
  validation workflow previously ran once per day. Changed the schedule to
  run at 17 minutes every six hours; manual dispatch and 30-day reports remain.
- Fresh `validate-all --json` run at 2026-10-28 21:18 UTC decoded all 25
  stations in 25 attempts. This is point-in-time codec evidence only.
- Repeated the label A/B/C regression: at 295 px, A full category fits 24/25,
  while B names-only and C compact category fit 25/25; all variants fit at
  340 px. Compact `CLASS` saves 32 px. This measures layout fit, not listener
  preference.
- Catalog policy regression passed on `fbeb476`. All six required workspace
  gates passed again against the current tree after the cadence change; Ruby's
  YAML parser accepted the workflow file. The workflow schedule update awaits
  remote CI; Actions run `40770761885` for head `2bd4bfd` failed before creating
  jobs. PR #3 remains open and mergeable; remote CI is unverified.
- No audio, receiver, or Pi hardware was exercised. Next: validate the workflow
  configuration, run the full local gates on the final source, then continue
  physical acceptance and publisher-rights review when available.

## Scheduled catalog report summary (2026-10-28)

- The scheduled six-hour catalog job now adds an operator-visible Actions
  summary beside the retained JSON artifact: total decoded, outcome counts and
  per-station ID/status/stage/attempts/recovery. It intentionally omits stream
  endpoints and server-provided error text.
- Added label sanitization and tests for credential/error omission, Markdown
  injection, status counts and malformed reports. All three Python tests,
  workflow YAML parsing, Rust format check, progress JSON parse and `git diff
  --check` passed.
- No playback code changed in this increment; all six workspace gates passed
  immediately before it. Remote workflow execution remains unavailable for
  the current PR head, and no hardware acceptance is inferred.
- At checkpoint, PR #3 is open and mergeable at `fab6f63`. No workflow run or
  status checks are visible for this head; prior Actions run `40770761885`
  failed before creating jobs. Local `git merge-tree origin/main HEAD` found
  no conflicts.

## Headless AArch64 feature matrix (2026-10-28)

- On current head `3b57855`, all three Pi-target compile configurations passed:
  core-only (`--no-default-features`), SDRconnect, and direct `nrsp-native`.
- These checks verify the headless Linux ARM64 dependency/feature combinations
  compile; they do not verify native Pi audio, receiver interoperability, or
  runtime behavior.
- PR #3 remains open and mergeable at `3b57855`. No workflow runs or combined
  status checks are visible for this head; remote CI remains unverified.

## Live validation summary and layout A/B recheck (2026-10-28)

- Ran a fresh bounded `validate-all --json` at 2026-10-28 21:25 UTC: all 25
  stations decoded in 25 attempts. The new summary script consumed that actual
  CLI report and produced 25/25 decoded, `decoded_audio`, one attempt each,
  without outputting endpoints or server error details. This is point-in-time
  codec evidence only.
- Repeated the GUI label A/B/C test on current source: at 295 px, full category
  fit 24/25; names-only and compact category fit 25/25. At 340 px all three fit
  25/25; compact `CLASS` saves 32 px. This is layout fit, not listener
  preference.
- PR #3 was open and mergeable at `c9dc197`; no workflow run or status was
  visible. No new catalog, GUI or hardware regression was found.

## Catalog summary status contract (2026-10-28)

- Review against the CLI JSON encoder found the summary formatter omitted the
  `check_error` outcome emitted when a validator returns an error, and it
  accepted stage strings outside the core validation enum. Added `check_error`
  handling, a five-value stage allowlist, and strict schema-version checking.
- Four standard-library tests passed, covering decoded/unavailable/unverified/
  check-error outcomes, redaction, label injection, invalid stages, malformed
  reports and unsupported schema versions. Ruby YAML parsing, Rust formatting
  and `git diff --check` passed.
- The live-sweep JSON path is unchanged; all 25 stations decoded in the latest
  real sweep, and the summary was verified against that CLI output before this
  contract tightening. No Rust source changed.

## Strict summary integration and A/B recheck (2026-10-28)

- Re-ran the strict schema-aware summary formatter against a fresh live CLI
  report at 2026-10-28 21:28 UTC. All 25/25 stations decoded at
  `decoded_audio`; the formatter emitted the real report's schema, count and
  outcome summary successfully.
- Repeated the label A/B/C test: at 295 px, A full category fit 24/25 and B
  names-only/C compact category fit 25/25; at 340 px all variants fit 25/25.
  Compact `CLASS` saves 32 px; layout-fit evidence only.
- PR #3 was open and mergeable at `96e07cc`; no Actions run or status was
  visible for that head. The automation memory records the exact current head.

## Actions startup and scheduled-workflow activation audit (2026-10-28)

- PR #3 is open and mergeable at `62e35f0`, but the latest run
  `40772372085` ended `startup_failure` at `2026-10-28T21:29:32Z`. GitHub
  reports zero jobs and no run log exists. Repository Actions are enabled and
  the selected-action policy allows GitHub-owned and verified actions; no
  actionable repo allowlist issue was found.
- The new `station-validation.yml` does not exist on default branch `main` yet:
  `gh run list --workflow station-validation.yml` returns 404 there. Its cron
  cannot run until that workflow reaches the default branch. PR checks have
  not passed, so the merge gate remains unmet; no merge was attempted.
- The real CLI/report formatter and local synthetic tests remain validated.
  Remote execution and artifact creation are not verified; continue local
  checks and revisit after an eligible Actions run is available.
- The findings are recorded in commit `52be452`; PR #3 remains open and
  mergeable at that head. No workflow run is visible for the checkpoint commit.
- The subsequent push triggered run `40772645579` on `52be452`; it again ended
  `startup_failure` with zero jobs. The latest head `1642027` has no run visible
  yet. This reproduces the runner-startup problem but still exposes no cause.


## Temporary HTTP retry handling and validation (2026-10-28 23:44 UTC)

- `TransportError::http_status` now classifies HTTP 425 Too Early as transient,
  matching the catalog validation policy; the focused regression verifies 425
  retries with the existing 408/429/5xx statuses while rejection statuses stay
  permanent.
- Focused transport regression passed. All six required locked workspace gates
  passed on the working tree: format, default tests, strict default Clippy,
  all-feature tests, strict all-feature Clippy and all-feature check.
- Repeated station label A/B/C: at 295 px A full category fits 24/25, B names-only
  and C compact category fit 25/25; at 340 px all fit 25/25. C saves 192 px vs A;
  only `wcrb` overflows with A. This measures fit, not listener preference.
- The first bounded live sweep ran inside the restricted network sandbox and
  returned all 25 stations as `unavailable` at `not_checked`, two attempts
  each. Rerunning with authorized network access decoded all 25 stations in 25
  attempts at 2026-10-28 23:45:04 UTC, with no retries or failures. This is
  point-in-time codec evidence only.
- Implementation commit `23ee373` contains the retry fix. PR #5 remains open
  and mergeable. Actions run `40789529683` (run 282) for checkpoint `dd3def2`
  ended `startup_failure` before creating any jobs; combined statuses were
  empty. This repeats the runner-startup blocker and provides no code-level CI
  signal. Run 283 (`40789606164`) for checkpoint `3ef4b64` also ended
  `startup_failure` before jobs; no combined statuses were reported. Native
  MSVC and physical hardware acceptance gates remain open.


## Permanent server response retry classification (2026-10-28 23:51 UTC)

- Tightened `TransportError::http_status`: retry only 408, 425, 429, 500,
  502, 503 and 504. Other HTTP statuses now fail permanently, including
  unsupported/configuration server responses such as 501, 505, 506, 507, 508,
  510 and 511. This avoids spending the bounded retry window on server errors
  unlikely to recover without operator or publisher action.
- Focused status classification regression passed, including transient retry
  decisions and permanent 5xx rejection; all six locked workspace gates passed.
- GUI label A/B/C repeated: at 295 px A full-category fits 24/25, B names-only
  and C compact-category fit 25/25; all variants fit at 340 px. C saves 192 px
  and A overflows `wcrb`. Layout fit only, not listener preference.
- Bounded live silent validation at 2026-10-28 23:51:08 UTC decoded all 25 feeds
  in 25 attempts without retries or failures. This is point-in-time codec
  evidence only.
- PR #5 is open and mergeable at code/evidence head `49f5525`. At 23:51 UTC,
  no workflow run or combined status was visible for that head; the latest
  observed run 284 for prior head `36ef13c` failed before jobs (`startup_failure`,
  zero jobs, empty statuses). Physical hardware and native Windows MSVC
  acceptance remain open.


## Published retry-policy checkpoint (2026-10-28 23:51 UTC)

- Pushed implementation `f92f0fc` and documentation/evidence checkpoint
  `49f5525` to PR #5. The PR is open and mergeable. No run or combined status
  was visible for `49f5525` at observation time; run 284 for its prior head
  failed before jobs, so remote CI remains unverified.
- The bounded live sweep decoded 25/25 entries and the layout A/B/C comparison
  passed on this source. No physical hardware was exercised.


## Muted native integration and portability check (2026-10-28 23:55 UTC)

- Ran ignored GUI integration test `native_ip_playback_switching_failure_retry_and_responsiveness` on the actual host audio backend at volume 0. It opened CPAL output, reached Playing on fixture MP3/AAC streams through redirect/playlist paths, then passed rapid switching, stalled headers, invalid/unsupported streams, explicit retry, refused endpoint, responsive rendering and clean Stop. This proves muted callback/lifecycle integration only; no audible listening or device-loss recovery acceptance.
- Repeated station label A/B/C: at 295 px A full-category 24/25, B names-only and C compact-category 25/25; at 340 px all 25/25; C saves 192 px and A overflows `wcrb`. Fit only, not listener preference.
- Windows GNU test binaries compiled and strict Clippy passed for default, SDRconnect and direct nRSP-native/GUI feature sets. Tests were compiled, not executed on Windows.
- Linux ARM64 headless check did not complete: `ring` could not find `aarch64-linux-gnu-gcc`. The Rust target is installed, but the cross C compiler is absent; this is an environment limitation, not a Rust diagnostic.
- PR #5 is open and mergeable at `139c00c`. Actions run 286 (`40790287485`) for that head ended `startup_failure` before job creation; jobs and combined statuses are empty. No production code changed during this acceptance-only run.
- Remaining: audible output, Bluetooth/USB loss recovery, Raspberry Pi audio/display/touch and receiver acceptance; publisher rights, national NPR source/rights, native Windows MSVC CI.


- Evidence checkpoint `37a7463` was pushed at 23:55 UTC. PR #5 remains open and
  mergeable; no run or combined status was visible for that head.


## Latest catalog and CI recheck (2026-10-28 23:57 UTC)

- Bounded live `validate-all --json` decoded 25/25 feeds in 25 attempts with no
  retries or failures at 23:57:22 UTC. This is point-in-time codec evidence
  only.
- Repeated GUI A/B/C at 295 px: full category 24/25, names-only and compact
  category 25/25; all variants fit at 340 px. Compact category saves 192 px and
  full category overflows `wcrb`. Layout fit only.
- PR #5 remains open and mergeable at `fdefa21`. Actions run 288
  (`40790726207`) failed before job creation (`startup_failure`), with zero jobs
  and no combined statuses. Remote CI remains unverified.


## Catalog and compact-label revalidation (2026-10-28 23:59 UTC)

- Repeated station-label A/B/C on the current tree: at 295 px A full-category
  fits 24/25, B names-only and C compact-category fit 25/25; at 340 px all
  variants fit 25/25. C saves 192 px versus A; A clips `wcrb`. This measures
  rendered fit only, not listener preference.
- Bounded live `validate-all --json` at 23:59:21 UTC decoded all 25 feeds in
  25 attempts, with no retries or failures. This is point-in-time codec evidence
  only, not publisher-rights or long-term availability evidence.
- PR #5 remains open and mergeable at `fe9fbbd`. Actions run 289
  (`40790897058`) failed before job creation (`startup_failure`); it had zero
  jobs and no combined statuses. Remote CI remains unverified.
- No source-code change was warranted by these checks. Hardware acceptance,
  publisher-rights verification, Windows MSVC and AArch64 toolchain gates remain
  open.


## Workspace regression gates (2026-10-29 00:01 UTC)

- Re-ran all six required gates on the current source. Format, both strict Clippy
  configurations and all-feature check passed. The first default and all-feature
  test runs hit sandbox `PermissionDenied` while binding local loopback fixtures;
  focused output localized the failure to test setup at `stream_inputs.rs:21`.
- Re-ran both complete workspace test suites with local test-fixture networking
  enabled; both passed. No source or test changes were needed.
- The current docs head `3ad18a8` had no PR workflow run visible when checked;
  prior head `fe9fbbd` run 289 failed before jobs with `startup_failure` (zero
  jobs, no combined statuses). Remote CI remains unverified.


## Transport contract and latest PR check (2026-10-29 00:03 UTC)

- Corrected `docs/IP_TRANSPORT_EXTRACTION.md`: playback retries HTTP 408, 425,
  429, 500, 502, 503 and 504; other HTTP responses are permanent, matching
  `TransportError::http_status` and its focused regression. Replaced the stale
  pending DNS audit with the resolver regression-maintenance requirement;
  SafeResolver already rejects private DNS answers and preserves transient
  resolver timeouts.
- Repeated the GUI station-label A/B/C regression and HTTP status retry test;
  both passed. At 295 px, compact genre labels fit 25/25 and full labels 24/25;
  at 340 px all fit. This is layout fit, not listener preference.
- PR #5 remains open and mergeable at `b502d10`, with no reviews. Actions run
  291 (`40791274189`) ended `startup_failure` before job creation; jobs and
  combined statuses are empty. Local CI remains the available validation path.


## Station menu frame-allocation polish (2026-10-29 00:07 UTC)

- Moved station-picker row-label vector construction into egui's popup closure.
  Closed frames no longer build and discard the 25 curated station labels; row
  contents and keyboard selection behavior are unchanged. This reduces repeated
  UI work in the compact native panel.
- Focused GUI A/B and keyboard-navigation regressions passed. A/B at 295 px:
  full category 24/25, names-only and compact category 25/25; at 340 px all
  variants fit 25/25. Compact labels save 192 px versus full categories. Fit
  measurement only, not listener preference.
- All six required local gates passed on the source change: formatting, both
  workspace test suites (with loopback fixture access), default/all-feature
  strict Clippy and all-feature check. `git diff --check` passed.
- Source commit `e29b173` was pushed to PR #5. PR is open and mergeable, with no
  reviews. No workflow run or combined status is visible for `e29b173`; the last
  observed run 291 (`40791274189`) on `b502d10` failed before jobs with
  `startup_failure`.


## Native output stall diagnostic (2026-10-29 00:11 UTC)

- The five-second full-ring callback watchdog now records `Native audio output
  stopped consuming samples` when the OS callback stalls without an asynchronous
  device error. `OutputFailure` keeps the first diagnostic instead of allowing a
  later generic watchdog message to hide the backend error. The CPAL sample
  callback remains lock-free; recording happens on the decoder/error path.
- Focused watchdog test confirms failure is bounded and exposes the detail;
  output error-detail retention regression passed. The GUI station label A/B was
  repeated on the current tree and passed: at 295px A 24/25, B/C 25/25; at
  340px all fit 25/25; C saves 192px. Fit only, not preference evidence.
- All six workspace gates passed on the exact source tree: format, default and
  all-feature tests, strict Clippy in both configurations, and all-feature
  check. Loopback fixture tests ran with local test networking enabled.
- Source commit `4415ef0` and preceding GUI performance commit `e29b173` are on
  PR #5. PR is open and mergeable with no reviews. No Actions run or combined
  status is visible for `4415ef0`; run 291 on `b502d10` failed before job
  creation (`startup_failure`, zero jobs).
