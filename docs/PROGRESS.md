# Radio development progress

## Current-head verification and A/B review (2026-10-30 00:27 UTC)

- Starting and ending source commit: `11b9508f8b931f56c05a7a28521b3791a081b220` on `codex/catalog-publisher-permissions`; checkout clean and aligned with its configured origin branch. No source change was justified in this pass.
- Rechecked PR #5 through the GitHub connector: open, mergeable, no review threads or submitted reviews. Actions run 375 (`40979183899`) for the exact head ended `startup_failure`; GitHub returned zero usable code jobs, so remote CI and native Windows MSVC execution remain unverified.
- All six locked local gates passed after enabling local loopback sockets: format, default workspace tests, default strict Clippy, all-feature workspace tests, all-feature strict Clippy, and all-feature check. Default suite: 31 GUI tests passed and one hardware integration test ignored; all-feature GUI suite: 40 passed and one hardware integration test ignored. Two reqwest-backed probe tests first failed under the restricted sandbox because localhost binds were denied; both passed in the permitted full rerun. Public-network live timing remains intentionally ignored.
- A/B decision review: repeated WFMU live muted native buffering trials (350 ms vs 500 ms) in `live_station_wfmu_balanced_resilient_ab_repeat_2026_10_30` had zero underruns in both variants and paired latency ranges crossing zero. The earlier synthetic 90 ms multi-gap test showed lower missing-sample share with 500 ms at the cost of startup latency. Neither result establishes listener preference; retain the 500 ms resilient default. Do not repeat the same experiment without a distinct hypothesis or new environment/feed evidence.
- UI polish review against the 480×320 egui render tests found that listener-facing errors already use actionable summaries while retaining raw diagnostics on hover; long diagnostics and settings-window bounds are covered by `station_status_and_long_diagnostics_fit_the_micro_viewport` and `playing_surface_has_no_stale_badge_or_decode_diagnostics_and_errors_fit`. Existing station-label A/B/C is geometric fit evidence, not preference data, and already selects the compact-category treatment. No unmeasured variant was promoted.
- Remaining gates: repair/restore actionable GitHub Actions execution, native Windows MSVC CI, physical Bluetooth/USB output-loss recovery, Pi runtime/display/touch, receiver hardware acceptance, publisher redistribution-rights review, and ongoing Rust-Boy integration.
- Next action: obtain a real Windows MSVC workflow result and continue physical acceptance; schedule the next bounded Skylab validation after a meaningful recovery interval, preserving the catalog entry until repeated publisher/endpoint evidence supports a change.

## All-feature test rerun and overnight WPR A/B (2026-10-29 23:35 UTC)

- Reran `cargo test --workspace --all-features --locked` with local socket
  access. The full suite passed; the earlier sandbox-only run had failed to bind
  six existing loopback HTTP fixtures. No code assertion failed.
- Repeated the WPR News balanced 350 ms vs resilient 500 ms comparison after a
  longer time interval: silent JSON preflight decoded MP3, 44.1 kHz mono, 1152
  frames. Eight counterbalanced 30-second muted native trials completed with
  zero underruns. Buffer-ready means/medians were 1380.880/1418.910 ms vs
  1347.831/1365.802 ms. Paired A-minus-B median was +93.042 ms (range
  -300.417 to +246.531 ms); first-play median +84.789 ms (range -304.425 to
  +264.875 ms). Both paired ranges cross zero, and neither profile underruns.
  This does not support a default change or listening-preference claim; the
  timing direction differs from the earlier WPR block. Raw results are under
  `wpr_news_live_ab_30s_overnight_followup` in `docs/AGENT_PROGRESS.json`.
- PR #5 remains open and mergeable at `ec53bf1`. Actions run 364
  (`40819022289`) failed before job creation; jobs and combined statuses are
  empty, with no remote log to diagnose.

## Validation MIME metadata sanitization (2026-10-29 04:37 UTC)

- Both `validate STATION --json` and `validate-all --json` now retain only a
  bounded, valid lowercase MIME type from publisher-controlled Content-Type
  headers. Parameters, which can carry arbitrary text or token-like values, are
  dropped. Invalid, non-ASCII, malformed or overlong values become null.
- The CLI suite passed 18/18, including regression coverage for parameter
  stripping and invalid/oversized media types. The A/B Python suite passed 12/12.
  All six locked Rust workspace gates passed.
- End-to-end WPR News preflight returned normalized `audio/mpeg`, omitted URL
  and detail fields, and the 8-trial muted A/B completed with zero underruns.
  Balanced 350ms vs resilient 500ms buffer-ready means were
  1522.834/1655.351ms; paired A-minus-B median was -128.792ms (range -671.608
  to +399.125ms). First-play paired median was -114.943ms (range -676.902 to
  +420.357ms). Ranges cross zero; no profile/default or listening-preference
  conclusion is supported. Full raw trials are in
  `wpr_news_mime_sanitized_ab_8s` in `docs/AGENT_PROGRESS.json`.
- Actions run 362 (`40818606116`) on PR head `e1182ac` failed before job
  creation; GitHub returned zero jobs and no combined statuses or logs. The
  MIME sanitizer is pushed as `ba881be`; PR #5 is open and mergeable, with no
  workflow run or combined status visible for that exact head at 04:39 UTC.

## A/B diagnostics privacy hardening (2026-10-29 04:31 UTC)

- `scripts/run_prebuffer_trace_ab.py` now accepts only the six playback diagnostic
  fields used by the summary. Unexpected additions, including URL/detail fields,
  abort before being copied into the trial record. Latencies must be finite, non-negative and within the trial timeout; prebuffer
  values and sample counters require exact integer types, with counters bounded
  by supported output rates/channels and the trial timeout.
- Trial failures no longer echo CLI stdout/stderr or arbitrary diagnostic values,
  preventing query tokens or future URL-bearing fields from leaking through error
  reports. Twelve Python regressions passed, including token-bearing CLI output,
  unexpected URL fields, non-finite values, booleans and oversized numbers. Python
  compile and `git diff --check` passed.
- All six locked Rust workspace gates passed. The first default test run was
  blocked by sandbox-denied loopback sockets in six existing integration fixtures;
  rerunning with local socket access passed the workspace and all-feature suites.
- Actions run 360 (`40818001252`) on parent head `76d4d8d` failed before job
  creation; GitHub returned zero jobs and no combined statuses, so it has no
  diagnostic logs. The privacy hardening is pushed as `8d68310`; PR #5 remains
  open and mergeable, with no run or combined status visible for that exact head
  at 04:35 UTC.

## WPR News longer live buffering A/B (2026-10-29 04:22 UTC)

- The runner performed its single-station JSON preflight first; WPR News decoded
  as MP3, 44.1 kHz mono with 1152 frames. Eight 30-second muted native CPAL
  trials compared balanced 350 ms and resilient 500 ms in ABBA then BAAB order.
  All eight had zero callback underruns.
- Buffer-ready means were 1438.958/1490.883 ms and medians 1490.077/1480.564
  ms. Paired A-minus-B median was +20.399 ms (range -404.680 to +156.185 ms);
  first-play paired median was +4.967 ms (range -439.430 to +108.563 ms). The
  broad paired ranges cross zero. This adds no startup winner, reliability
  difference, or listener-preference evidence; retain the 500 ms resilient
  default. Full preflight and per-trial data are in
  `wpr_news_live_ab_30s_followup` in `docs/AGENT_PROGRESS.json`.
- The A/B harness regression passed 9/9 before the live run. No Rust source
  changed. GitHub Actions run 356 (`40817262098`) for exact PR head `4e74011`
  failed before job creation (`startup_failure`); GitHub returned zero jobs and
  no combined statuses, so there are no remote job logs to diagnose. PR #5
  remains open and mergeable. Evidence commit `47e7169` is pushed. The follow-up checkpoint `d62bc2f` was open and mergeable with a clean local merge-tree against fetched `main`. Actions run 359 (`40817891412`) on the subsequent head `c9bd359` ended `startup_failure` before job creation; the jobs endpoint returned zero jobs and combined statuses are empty. The PR remains open and mergeable.

## Curated A/B validation preflight (2026-10-29 04:16 UTC; checkpoint 04:20 UTC)

- Added `rust-radio-cli validate STATION --json` as a version-1 report using the
  existing `validate-all --json` envelope with one station result. The record
  includes decode status/stage, timestamp, codec, format and sample facts, but
  omits resolved URLs and free-form details.
- Curated mode in `scripts/run_prebuffer_trace_ab.py` now runs that bounded
  silent decode check before playback, requires one decoded PCM result for the
  requested station, and stores the validation envelope in the A/B artifact.
  Invalid, failed, malformed or sensitive-field preflights abort before any
  native output trial. Loopback URL mode remains a synthetic fixture path.
- End-to-end WPR News preflight decoded MP3, 44.1 kHz mono, 1152 frames. Eight
  muted 8-second trials compared balanced 350 ms (A) and resilient 500 ms (B);
  all eight had zero underruns. Buffer-ready means/medians were
  1557.398/1603.513 ms vs 1369.014/1389.799 ms. Paired A-minus-B median was
  +225.357 ms with a wide -323.771 to +626.593 ms range; first-play median
  +208.675 ms (-382.985 to +603.337 ms). This is inconclusive startup evidence
  and does not establish a reliability advantage or listener preference. Full
  machine record is under `single_station_json_preflight_wpr_ab` in
  `docs/AGENT_PROGRESS.json`.
- All six locked Rust workspace gates passed. CLI JSON/privacy regressions passed;
  Python runner suite passed 9/9, along with compile/help, workflow YAML, JSON and
  diff checks. Actions run 354 (`40816126511`) on current pushed head `288c81b`
  failed before job creation with zero jobs and no combined statuses. Implementation commit `1204391` is now pushed to PR #5, which remains open and mergeable. For exact head `1204391e0f029c069a29c7d5589653a7643fe1e7`, GitHub currently reports no workflow run and no combined status. The preceding run 354 on parent `288c81b` failed at startup before any jobs were created.
- Next: physical Bluetooth/USB output-loss and reconnect tests, blinded audible
  comparison, native Windows MSVC and Raspberry Pi runtime acceptance.

## WPR News live-feed buffering A/B (2026-10-29 04:05 UTC)

- Silent CLI validation decoded Wisconsin Public Radio News at
  `https://wpr-ice.streamguys1.com/wpr-ideas-mp3-64`: MP3, 44.1 kHz mono,
  1152 decoded frames. Validation did not open audio output.
- Ran eight 15-second counterbalanced muted native trials on macOS ARM64/system
  output, balanced 350 ms (A) vs resilient 500 ms (B), A/B/B/A then B/A/A/B.
  All eight had zero callback underruns. Buffer-ready/first-play means were
  1574.757/1606.892 ms and 1456.876/1496.955 ms; medians were
  1615.866/1655.414 ms and 1421.652/1472.593 ms.
- Paired A-minus-B buffer-ready median was +132.680 ms, range -102.329 to
  +308.492 ms; first-play median +123.552 ms, range -123.519 to +316.165 ms.
  Although the aggregate startup values favored 500 ms, paired ranges crossed
  zero and neither profile underrun. This does not establish a reliability
  winner or listener preference; retain the current default. Raw trials and
  validation evidence are in `docs/AGENT_PROGRESS.json` under
  `wpr_news_live_balanced_resilient_ab`.
- Actions run 352 (`40815715224`) on the then-current head `69ed85a` also
  ended `startup_failure` before job creation; there were zero jobs and no
  combined statuses. The follow-up evidence commit `184742c` is pushed; GitHub
  has not exposed a run or combined status for that exact head yet.
- Next: prioritize physical output-loss/reconnect and audible listening tests,
  followed by native Windows MSVC and Raspberry Pi runtime acceptance.

## A/B spread and paired latency reporting (2026-10-29 04:03 UTC)

- Extended `scripts/run_prebuffer_trace_ab.py` to report median/min/max for
  buffer-ready, first-play and prebuffer-fill latency while retaining existing
  means and underrun share. It also reports adjacent A-minus-B deltas for each
  pair in the counterbalanced run. Added tests for spread/delta values and
  rejection of malformed pair ordering.
- End-to-end replay used the captured Skylab response timing plus deterministic
  350/500/250 ms stalls after chunks 20/38/54. All eight muted native CPAL
  trials had zero underruns. Balanced 350 ms vs resilient 500 ms buffer-ready
  means were 483.464/468.548 ms; medians 467.319/462.181 ms. Paired A-minus-B
  median was +5.137 ms, with a wide -147.847 to +197.234 ms range. First-play
  paired median was +27.006 ms (-149.995 to +173.167 ms). This does not establish
  a profile winner or listener preference. Raw trials are under
  `ab_paired_summary_followup` in `docs/AGENT_PROGRESS.json`.
- All six locked Rust workspace gates passed. Python harness tests passed 6/6,
  fixture-server process test 1/1, Python compilation/help, workflow YAML, JSON
  parse and `git diff --check` passed. Actions run 350 (`40815638352`) on the pushed head `55883c9` also failed
  before job creation; GitHub returned zero jobs and no combined statuses. The
  follow-up evidence commit `a86ccac` is pushed; GitHub has not yet exposed a
  run or combined status for that exact documentation head.
- Next: complete physical output-loss/reconnect and blinded audible acceptance;
  native Windows MSVC and Raspberry Pi runtime gates remain open.

## Combined captured Skylab pacing and stall A/B (2026-10-29 03:57 UTC)

- Replayed the captured Skylab response-body sizes/intervals with bundled synthetic
  MP3 audio, adding 350/500/250 ms stalls after chunks 20/38/54. Eight
  counterbalanced muted native CPAL trials completed on macOS ARM64/system output.
- Both profiles had zero callback underruns. Mean buffer-ready/first-play was
  491.986/522.645 ms at balanced 350 ms and 432.237/471.774 ms at resilient
  500 ms. This direction opposes the combined WCRB replay; trial-to-trial spread
  is material, so neither replay establishes a consistent startup winner or
  reliability difference. Synthetic audio does not establish live-feed or human
  listening acceptance. Raw measurements are under
  `combined_skylab_trace_and_stalls_ab` in `docs/AGENT_PROGRESS.json`.
- Runner tests passed 4/4 and fixture-server regression passed 1/1; YAML parsed.
  Rust source is unchanged from the latest six-gate pass on `317a16f`. PR #5 is
  open. Actions run 347 (`40814722306`) on base head `0ab73bc` ended
  `startup_failure` before jobs; the jobs list and commit statuses were empty.
  The evidence checkpoint was pushed as `340daf9`; GitHub has no run or combined
  status visible yet for that new head.
- Next: continue cross-feed measurements and complete physical output-loss and
  audible acceptance; investigate remote Actions startup, Windows MSVC and Pi.

## Combined captured pacing and stall A/B (2026-10-29 03:52 UTC)

- Updated `scripts/prebuffer_fixture_server.py` to allow a captured timing trace
  and additional scheduled stalls together. Added a process-level regression
  that starts the server with both inputs, and added it to the portable-core
  workflow.
- Replayed the captured WCRB body-size/interval trace with synthetic MP3 audio
  and additive 350/500/250 ms stalls after chunks 20/38/54. Eight counterbalanced
  muted native 8-second trials produced zero callback underruns at both 350 ms
  and 500 ms. Mean buffer-ready/first-play: 480.865/527.455 ms at 350 ms and
  420.987/469.475 ms at 500 ms. Startup direction varied with feed timing, so
  this block shows no consistent latency winner or reliability difference.
- The separate paced multi-gap block (90 ms pacing without the captured trace)
  showed 4.246% missing samples at 350 ms vs 0.735% at 500 ms. Keep 500 ms as
  the engine default for its stronger result under repeatable stall stress;
  neither replay is a claim about live-stream dropout or human preference.
- Both Python suites passed (4 A/B runner tests and 1 fixture-server test),
  workflow YAML parsing passed, and the combined trace/stall server and native
  A/B run completed. Rust source is unchanged from the latest six-gate pass.
- Raw measurements are in `docs/AGENT_PROGRESS.json` under
  `combined_wcrb_trace_and_stalls_ab`. Run 345 (`40814017194`) on prior head
  `d84ed5e` had `startup_failure`, zero jobs and no combined statuses. Next:
  repeat using the Skylab trace and keep real output-loss and audible acceptance
  open.

## Counterbalanced WCRB trace and stall A/B (2026-10-29 03:50 UTC)

- Applied the updated A/B/B/A then B/A/A/B runner to two replay conditions,
  comparing 350 ms balanced with 500 ms resilient. Both used eight muted,
  8-second native CPAL trials on macOS ARM64/system output.
- Captured WCRB body-timing trace replay used only observed response chunk sizes
  and intervals with the bundled synthetic MP3. All trials had zero underruns.
  Mean buffer-ready/first-play: 437.115/471.148 ms at 350 ms and
  550.701/589.770 ms at 500 ms. This favors 350 ms for startup on that replay
  and shows no dropout difference.
- A separate loopback stream paced chunks every 90 ms and inserted stalls of
  350/500/250 ms after chunks 20/38/54. All trials had callback underruns.
  The 350 ms profile averaged 710.625 ms buffer-ready, 763.567 ms first-play
  and 4.246% missing samples; 500 ms averaged 916.482 ms, 947.009 ms and
  0.735%. Thus 500 ms reduced missing-sample share by 82.7% at a cost of
  205.858 ms buffer-ready and 183.442 ms first-play latency. Keep 500 ms as
  default for reliability; expose 350 ms only as the opt-in startup choice.
- Raw trials are under `counterbalanced_wcrb_timing_trace_ab` and
  `counterbalanced_90ms_multi_gap_ab` in `docs/AGENT_PROGRESS.json`.
  The trace replay is synthetic audio paced by a captured timing pattern; the
  multi-gap replay is fully synthetic. Neither establishes live-stream or
  listener acceptance.
- All six Rust gates were passing on source commit `317a16f`; the latest changes
  are evidence only. PR #5 is open/mergeable. Actions run 345 (`40814017194`)
  on `d84ed5e` failed before job creation (`startup_failure`, zero jobs and no
  combined statuses). Next: repeat using another captured feed trace and get
  physical audio/output-loss acceptance.

## Counterbalanced WCRB buffering A/B (2026-10-29 03:44 UTC)

- Repeated the muted native WCRB 350/500 ms comparison after two consecutive
  fixed-order A/B/B/A/A/B/B/A blocks both placed a 3.5 s outlier in their first
  A trial. The new harness alternates A/B/B/A and B/A/A/B blocks, balancing
  which profile runs first; regression coverage verifies both sequences.
- Revalidated WCRB as AAC, 48 kHz stereo, 1024 decoded frames before playback.
  Eight 30-second trials on macOS ARM64/system output had zero callback
  underruns. Buffer-ready means were 1607.326 ms (350 ms) and 1549.155 ms
  (500 ms); first-play means were 1648.815 ms and 1573.061 ms. This closer
  58 ms/76 ms timing gap is still startup-only evidence, with no dropout or
  listener-preference difference. The 500 ms default remains provisional.
- All four Python harness regressions passed; Python byte-compilation, CLI
  help, workflow YAML parsing and `git diff --check` passed. All six local
  Rust gates passed on the unchanged Rust source at `317a16f` this run.
- PR #5 remains open and mergeable. Actions run 343 (`40812675091`) for
  `317a16f` again failed with `startup_failure` before job creation; the job
  list is empty and combined statuses are empty. Cross-target Windows GNU
  check, strict Clippy and test-binary compilation passed for default,
  SDRconnect and direct nRSP-native; headless AArch64 Linux/macOS checks passed.
- Raw data: `docs/AGENT_PROGRESS.json` keys
  `live_station_wcrb_balanced_resilient_ab_repeat2` and
  `live_station_wcrb_balanced_resilient_ab_counterbalanced`. The fixed-order
  first-trial delay appeared twice but disappeared in the counterbalanced
  block, consistent with an order effect without identifying its mechanism.
  Next: use the counterbalanced runner for intermittent trace/feed tests and
  obtain audible preference and physical output-loss evidence.
- After pushing this counterbalanced update, Actions run 344
  (`40813939126`) for `fa4bce7` also ended `startup_failure` before job
  creation. GitHub returned zero jobs and no combined statuses, so this
  remains an external runner startup blocker rather than a code-test failure.

## Curated WCRB live-feed 350/500 ms A/B (2026-10-29 03:31 UTC)

- Before playback, validated curated station `wcrb`: GBH's approved endpoint
  decoded as AAC, 48 kHz stereo, 1024 frames; validation opened no output.
- Ran eight 30-second muted native-CPAL trials on macOS ARM64/system output,
  350 ms balanced vs 500 ms resilient, A/B/B/A twice. All eight completed
  without callback underruns. Buffer-ready means were 2227.812 ms vs
  1427.275 ms; first-play means were 2266.739 ms vs 1462.261 ms. Medians
  were 1953.414/1428.868 ms and 2002.450/1457.970 ms respectively. The
  balanced set included a 3541.560 ms startup outlier; the broad spread and
  zero underruns in both profiles make this inconclusive for reliability.
  It shows no audible preference because playback was muted.
- Raw measurements are retained at `/private/tmp/rust-radio-live-wcrb-balanced-resilient.json`
  and in `docs/AGENT_PROGRESS.json` under `live_station_wcrb_balanced_resilient_ab`.
  Together with the Skylab live block, both profiles had zero underruns on
  these stable feeds. Keep the 500 ms engine default provisional; do not infer
  a reliability advantage or listener preference from startup means alone.
- PR #5 was open and mergeable at source head `7d7da97`; no combined statuses
  were visible for that head. Earlier Actions run 340 failed before job
  creation (`startup_failure`, zero jobs). This evidence-only update does not
  change Rust source; next acceptance is blinded audible preference and
  physical output-loss/recovery testing, plus runner CI startup recovery.

## Curated live-feed 350/500 ms A/B (2026-10-29 03:19 UTC)

- Added a curated-station mode to `scripts/run_prebuffer_trace_ab.py`. It
  accepts only station IDs from `catalog/curated-stations.json` and invokes
  `rust-radio-cli listen`, preserving the runtime publisher allowlist; the
  existing custom URL mode remains loopback-only. Added three regressions and
  added the test command to the portable-core Actions job.
- Validated Skylab immediately before playback: decoded MP3, 48 kHz stereo,
  1152 frames; no output device was opened by validation. Ran eight muted
  30-second native CPAL trials in A/B/B/A order twice, 350 ms balanced vs
  500 ms resilient. All eight completed with zero callback underruns. Mean
  buffer-ready was 753.643 ms vs 934.959 ms; mean first-play was 802.896 ms
  vs 965.990 ms. The 350 ms profile reached buffer-ready 181.316 ms sooner
  and first-play 163.094 ms sooner. This stable single-feed block showed no
  dropout difference and no listener preference; it does not outweigh the
  synthetic jitter result or justify a default change.
- Harness tests (3/3), Python byte-compilation, help output, guarded URL
  rejection, workflow YAML parsing and `git diff --check` passed. The six
  workspace gates remain passing on the unchanged Rust source from the prior
  checkpoint; this turn changed the Python harness and its workflow test step.
- PR #5 remains open and mergeable at `28af928`; exact-head workflow runs
  and combined statuses were empty at 03:25 UTC. Previous docs head `5d11d08`
  had Actions run 340 (`40811389507`) fail at startup with zero jobs.
- Next: repeat 350/500 ms on another curated feed, then obtain audible and
  physical output-loss acceptance before considering a default change.


## Balanced 350 ms profile and three-setting A/B (2026-10-29 03:07 UTC)

- Added `IpBufferingPreset::Balanced` (350 ms), CLI `--buffering balanced`,
  and selectable A/B profiles in `scripts/run_prebuffer_trace_ab.py`. Existing
  200 ms and 500 ms profiles remain available; the engine default remains
  resilient at 500 ms. Added CLI parsing/duration coverage and updated README,
  native playback contract, and acceptance procedure.
- Repeated the same loopback MP3 fixture at 90 ms chunk pacing with post-start
  350/500/250 ms gaps. Both muted native-CPAL blocks used A/B/B/A twice,
  eight trials total per comparison, on macOS 26.5.2 ARM64/system output.
  Against 200 ms, 350 ms reduced missing-sample share 8.020% → 6.142%
  (23.4% relative) and added 120.113 ms mean buffer-ready latency. Against
  350 ms, 500 ms reduced share 5.834% → 2.348% (59.8% relative) and added
  304.447 ms. All 24 trials across the three profile blocks had underruns.
  These are synthetic one-host tradeoff measurements, not listener preference
  or live-broadcaster reliability evidence; no default change is justified.
- All six workspace gates passed. Windows GNU workspace check and strict
  Clippy passed, as did the headless AArch64 macOS IP check. CLI focused tests,
  runner byte-compilation/help, JSON and diff checks passed. No physical
  playback/device-loss acceptance was performed. PR #5 remains open and mergeable at head `67e35f1`. Actions run 339
  (`40811281044`) on that head failed before job creation (`startup_failure`);
  the jobs endpoint returned zero jobs and combined statuses were empty. The
  PR body now summarizes the balanced-profile implementation and both A/B
  blocks.
- Code commit `98235f5` adds the profile and flexible runner; `2fa930a` keeps
  the original public enum source-compatible. Next: obtain blinded audible
  preference and physical output-loss evidence, then repeat on another host or
  a measured live trace before selecting a default.

## 90 ms pacing multi-gap buffering A/B (2026-10-29 03:01 UTC)

- Repeated the loopback A/B on `dde25b6` with the generated 12-second MP3,
  1600-byte payloads every 90 ms and post-start gaps of 350/500/250 ms after
  chunks 20/38/54. Eight muted native CPAL trials ran in A/B/B/A order twice
  on macOS 26.5.2 ARM64 with system output; both used the same decoder and
  callback, with no output recovery.
- At A=200 ms, mean buffer-ready / first-play latency was 632.250/667.820 ms
  and aggregate missing-sample share was 7.151%. At B=500 ms, it was
  998.023/1033.327 ms and 2.483%. B reduced missing share by 65.3% relative
  (4.668 percentage points) and added 365.773 ms to buffer-ready latency.
  All eight trials completed; every trial had callback underruns. This is one
  synthetic single-host delivery condition, not live broadcaster behavior,
  audible quality or listener preference. Keep the buffering default
  provisional.
- The immediately preceding bounded catalog sweeps at 2026-10-29 02:58:42 and
  02:59:42 UTC decoded 25/25 entries in 25 attempts each, with no retries.
- All six Rust gates passed on this source tree with local-socket permission:
  formatting, default tests, strict default Clippy, all-feature tests, strict
  all-feature Clippy and all-feature check. The initial sandboxed test attempt
  could not bind loopback for six existing IP fixtures; the rerun passed.
- GitHub PR #5 remains open and mergeable at `dde25b6`. Actions run 335
  (`40809389372`) failed before creating jobs (`startup_failure`); the exact
  head has no combined-status checks. No source or regression change was
  indicated by this measurement.
- Next: obtain an audible comparison and physical output/device-loss evidence;
  hardware, Rust-Boy adoption, publisher-rights and native Windows MSVC gates
  remain open.

Updated 2026-10-29. Active priorities and ownership are in the
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


## IP stream prebuffer A/B (2026-10-29 00:15 UTC)

- Compared the current 200 ms prebuffer (A) with a 500 ms candidate (B) in a
  deterministic steady-state buffer model. With injected 100/300/800 ms network
  stalls, modeled silence was A: 0/100/600 ms and B: 0/0/300 ms. B adds 300 ms
  before initial playback. This is a synthetic jitter comparison, not live
  network or hardware measurement.
- Selected 500 ms for Internet IP streams to absorb stalls up to 500 ms; direct
  nRSP, SDRconnect and Soapy sources retain the 200 ms default to preserve
  tuning responsiveness. Buffer capacity remains two seconds. The implementation
  and reproducible comparison are in `output.rs` and `playback.rs`.
- Focused prebuffer A/B and station-label A/B regressions passed. All six
  required workspace gates passed on source commit `3a121df`; tests used local
  fixture network access.
- PR #5 remains open and mergeable at `3a121df`, with no reviews. No Actions run
  or combined status is visible for this head. The last observed run 291 on
  `b502d10` failed before jobs (`startup_failure`, zero jobs).
- Next: exercise the 500 ms policy with live network jitter and audible output,
  verify Raspberry Pi CPU/memory/start latency, and continue Bluetooth/output
  loss recovery acceptance.


## Live catalog and native startup recheck (2026-10-29 00:17 UTC)

- Bounded live `validate-all --json` at 00:16:38 UTC decoded all 25 feeds in
  25 attempts, one per feed, without failures or retries. This is point-in-time
  codec evidence; it does not measure inter-chunk jitter or audible quality.
- Existing ignored integration test
  `native_ip_playback_switching_failure_retry_and_responsiveness` passed on the
  host CPAL output at 0% volume with loopback MP3/AAC fixtures. The 500 ms IP
  prebuffer reached callback-confirmed Playing and the test exercised playlist
  and redirect streams, switching, failure, retry, responsiveness and Stop.
  This is muted callback/lifecycle evidence, not audible or device-loss
  acceptance.
- Repeated the jitter A/B: 200 ms vs 500 ms under 100/300/800 ms synthetic
  stalls yields 0/100/600 ms vs 0/0/300 ms modeled silence; B adds 300 ms
  startup. Repeated GUI label A/B: at 295 px, full category fits 24/25 while
  names-only and compact fit 25/25; all fit at 340 px. Both are repeatable
  synthetic/layout tests, not audience or field measurements.
- PR #5 remains open and mergeable at `577dc21`, with no reviews. No workflow
  run or combined status is visible for the current head; run 291 on `b502d10`
  failed before job creation (`startup_failure`, zero jobs).
- Next: measure chunk-arrival jitter on approved live streams and perform
  audible/target-device checks before claiming field benefit from the longer
  buffer. Raspberry Pi runtime, Bluetooth/USB loss, receiver and rights gates
  remain open.


## Buffer change live revalidation (2026-10-29 00:18 UTC)

- Rechecked PR #5 at `577dc21`: open and mergeable, no reviews. No Actions run
  or combined status was visible for that head; prior run 291 on `b502d10`
  failed before job creation (`startup_failure`, zero jobs).
- Live `validate-all --json` at 00:16:38 UTC decoded 25/25 feeds in 25 attempts
  with no retries or failures. Point-in-time codec check only.
- The existing muted native CPAL integration passed with the 500 ms IP prebuffer
  and loopback MP3/AAC fixtures. It verifies callback startup and lifecycle on
  this host at 0% volume, not audible quality, real stream jitter, or physical
  device loss.
- Repeated the synthetic jitter A/B and station-menu layout A/B. Results match
  the prior checkpoint. No source defect or reason to change either selected
  variant surfaced; no new source code changed in this revalidation.
- Next: collect actual inter-chunk timing for approved feeds and do audible and
  target-device acceptance before claiming field benefit from the prebuffer.


## Approved live response-chunk timing (2026-10-29 00:23 UTC)

- Added a bounded aggregate timing snapshot to the native HTTP transport.
  `NetworkGuard::timing_snapshot()` reports body chunks, mean and maximum
  intervals, and counts at or above 200 ms and 500 ms. The timing point is when
  reqwest yields an audio response-body chunk to the transport pump; it is not
  raw packet timing, decoded-buffer occupancy, callback underruns, or audible
  acceptance. Playlist chunks are excluded.
- Deterministic unit coverage checked three samples at 100 ms and 600 ms,
  producing a 350 ms mean, 600 ms maximum, one gap at each threshold.
- The ignored bounded live timing test ran for Skylab and WCRB using the
  publisher-approved catalog transport. Skylab: 182 chunks / 181 intervals in
  8.02 s, 44 ms mean, 653 ms max, 15 intervals >=200 ms and 7 >=500 ms;
  370,944 audio bytes. WCRB: 76 chunks / 75 intervals in 6.78 s, 85 ms mean,
  406 ms max, 16 intervals >=200 ms and none >=500 ms; 524,288 audio bytes.
  Each station had one short sample. The counts support retaining the 500 ms
  IP prebuffer as a candidate but cannot establish that it prevents audible
  dropouts; 300 ms more startup latency remains the known cost.
- Repeated the deterministic buffer A/B in the full default and all-feature
  suites: 200 ms gives modeled silence 0/100/600 ms and 500 ms gives 0/0/300 ms
  for 100/300/800 ms injected stalls. No end-user preference or actual
  playout-underrun comparison was performed.
- All six required gates passed on the implementation: format, default
  workspace tests, strict default Clippy, all-feature workspace tests, strict
  all-feature Clippy, and all-feature workspace check. The live timing test
  remains ignored in normal CI and passed when explicitly run.
- PR #5 is open and mergeable at implementation commit `0f82299`, with no
  reviews. At 00:24 UTC, GitHub showed no workflow runs or combined statuses for
  the new head. No physical playback, target-device, or device-loss acceptance
  was performed. Next: compare callback underruns and startup latency during
  repeated audible A/B playback on Mac, then repeat on Raspberry Pi if
  available.


## HTTP timing fixture and repeated prebuffer sample (2026-10-29 00:28 UTC)

- Renamed timing thresholds to `gaps_at_or_over_200ms` and
  `gaps_at_or_over_500ms`, matching the inclusive boundary used by the
  implementation. Added a loopback HTTP response that flushes three audio
  chunks with 80 ms and 350 ms gaps. The integration test confirmed the
  `NetworkGuard` snapshot reports three chunks, two intervals, and a gap at or
  above 200 ms. The exact threshold counters remain covered by deterministic
  aggregation tests; the loopback assertion allows for host scheduling delays.
- Repeated the buffer A/B: for 100/300/800 ms synthetic stalls, 200 ms yielded
  0/100/600 ms modeled silence; 500 ms yielded 0/0/300 ms, at 300 ms more
  startup. This still is a simple model, not a listener trial or native
  underrun measurement.
- Repeated approved live timing samples: Skylab 8.09 s, 376,704 bytes, 138
  chunks / 137 intervals, 59.0 ms mean, 655.6 ms max, 15 intervals >=200 ms and
  7 >=500 ms. WCRB 6.79 s, 524,288 bytes, 76 chunks / 75 intervals, 85.3 ms
  mean, 413.2 ms max, 16 intervals >=200 ms and none >=500 ms. The broadly
  similar gap counts across the two samples support keeping 500 ms as the
  candidate for a real playout A/B, but do not prove audio underruns or benefit.
- All six required local gates passed on this follow-up, including default and
  all-feature workspace tests with the paced loopback fixture, strict Clippy
  both ways, formatting, and all-feature check. Focused live timing fixture,
  prebuffer A/B and ignored public-feed timing test passed.
- At 00:24 UTC, PR #5 was open and mergeable at `aa523fa`. GitHub Actions run
  302 (`40793876301`) for that checkpoint ended `startup_failure` before job
  creation; there were zero jobs and no combined statuses. No audible or
  hardware test was performed. Next: record native callback underruns and
  startup latency during a repeated audible 200/500 ms Mac comparison, then
  run the same acceptance on Raspberry Pi if available.

- Final post-push check on `be8b76e`: all six workspace gates passed again,
  including the exact committed loopback timing fixture. PR #5 is open and
  mergeable at that head. No workflow run or combined status was visible for
  `be8b76e`; run 302 for its parent evidence checkpoint `aa523fa` is still the
  latest observed and failed before job creation. No audible or hardware
  acceptance was performed.


## Native callback underrun counter and A/B (2026-10-29 00:38 UTC)

- Added a device-rate `underrun_samples` counter to `NativeAudioOutput`. The
  CPAL callback adds missing samples to a relaxed atomic only when it emitted
  silence; it keeps the callback lock-free and allocation-free. The output
  exposes a read-only snapshot getter for diagnostics. This does not yet feed
  host session status or listener-facing UI.
- Reworked the 200 ms vs 500 ms regression to call the same `render_samples`
  callback used by CPAL, with one synthetic sample representing one millisecond
  of output. Across injected 100/300/800 ms stalls, callback underrun counts
  were A: 0/100/600 samples and B: 0/0/300 samples; B still requires 300 ms
  more initial fill in this model. This is repeatable callback-level evidence,
  not physical playback or listener preference.
- Focused callback tests passed: silence output is counted as underrun while
  consumed samples are not; prebuffer A/B reports the expected counts. The
  muted native GUI/CPAL integration passed on this commit with loopback MP3/AAC,
  switching, failure/retry and Stop paths. It does not measure audible quality
  or report actual live underrun totals.
- All six workspace gates passed on source commit `10eea53`: formatting,
  default and all-feature tests, strict Clippy in both feature configurations,
  and all-feature check. The A/B test exercises the production callback renderer
  but remains a synthetic input schedule.
- Before this change, PR #5 was open and mergeable at `604dd42`. Actions run 304
  (`40794577869`) failed before creating jobs (`startup_failure`, zero jobs);
  there were no combined status checks. Remote CI remains unavailable as code
  validation.
- After pushing these commits, GitHub reported PR #5 open and mergeable at
  `bcfadde`. No workflow run or combined status is visible for that head.
- Next: wire callback counters into per-session diagnostic telemetry and use
  repeated live-feed runs to compare actual initial-play latency and underruns
  for both prebuffer variants. Keep the 500 ms choice provisional pending
  audible Mac and target-device acceptance.


## Session output diagnostics and validation (2026-10-29 00:44 UTC)

- Wired the callback underrun sample counter through internet, direct nRSP-ST,
  SDRconnect and Soapy playback into one session-scoped atomic.
  `NativeRadioEngine::diagnostics()` returns the accumulated missing output
  samples and first-play latency observed at the first host status poll after
  callback PCM consumption. The latency is a polling-time upper estimate, not
  a precise device timestamp.
- Re-exported `NativePlaybackDiagnostics` from the stable `rust_radio` facade.
  The regression checks no latency before callback progress, then checks the
  observed latency and a known underrun count.
- Re-ran production-renderer 200 ms vs 500 ms prebuffer A/B in the
  all-feature audio unit suite: modeled callback underruns for 100/300/800 ms
  stalls remain 0/100/600 vs 0/0/300 samples; B adds 300 ms initial fill. This
  is synthetic scheduling evidence, not a live output A/B or listener trial.
- Passed: `cargo fmt --all -- --check`; default `cargo test --workspace` (after
  the sandbox initially denied fixture TCP binds, a permission retry passed);
  strict default Clippy; strict all-feature Clippy; all-feature workspace
  check; all-feature rust-radio-audio library tests (53 passed, 1 ignored); and
  the focused diagnostics test.
- The full `cargo test --workspace --all-features` reached test bodies but
  loopback TCP fixture binds were denied (`Operation not permitted`) in six IP
  integration tests and socket-driven SDR unit tests. The same occurred on the
  permission retry; those checks are environment-blocked, not code assertion
  failures. Default feature loopback tests did pass on retry.
- Implementation commit `13997d7` is pushed to the existing branch. PR #5 remains
  open and mergeable; GitHub reports no workflow run or combined status for this head. No physical playback, listener comparison,
  Pi runtime, Bluetooth, USB or Windows MSVC acceptance was performed.
- Next: expose a bounded operator-facing diagnostic snapshot during actual
  playback and collect repeatable audible 200 ms/500 ms A/B measurements on
  Mac; repeat on Pi when available. Keep modeled A/B results separate from
  hardware evidence.


## Operator-facing buffer profiles and live muted A/B (2026-10-29 00:53 UTC)

- Added `IpBufferingProfile::{LowLatency, Resilient}` to the stable headless API
  and `--buffering low-latency|resilient` to curated/direct CLI playback. The
  existing start methods remain on the 500 ms resilient default; native SDR
  sources remain at 200 ms. Timed and interactive CLI output now reports one
  JSON diagnostic snapshot with configured output prebuffer, observed
  first-play latency, and callback underrun samples. Invalid profiles fail
  before output or network resources open.
- Added an alternating four-run muted live callback A/B on Skylab, same system
  output, 30 seconds per run, at 0% volume: A/B/B/A. Results (observed
  first-play latency; callback underrun samples): A 200 ms = 674.615 ms / 0; B
  500 ms = 992.154 ms / 0; B = 738.498 ms / 0; A = 827.602 ms / 0. Mean: A
  751.109 ms, B 865.326 ms; B was 114.218 ms slower in this small sample. All
  four real native callback sessions consumed PCM and ended cleanly. Zero
  underruns across these four 30-second windows does not establish audible
  quality or prove the larger buffer reduces dropouts. Stream/decode timing
  varies more than this 2-trial-per-variant comparison can characterize; keep
  500 ms default provisional and collect more runs before changing it.
- Fresh bounded validation immediately before the A/B decoded Skylab MP3 at
  48 kHz stereo. `cargo run -p rust-radio-cli -- outputs` found the system
  default available. Output remained muted, so no listener preference was
  measured.
- Passed all six local workspace gates on the source changes: format; default
  and all-feature workspace tests (all local IP/SDR fixture tests passed with
  local-network permission); strict default/all-feature Clippy; all-feature
  check. Also passed focused CLI tests (16 default, 20 with all features),
  audio unit tests (53 passed, 1 ignored with all features), and diff checks.
- PR #5 was open and mergeable at `816b5e6`; Actions run 308
  (`40796119804`) failed with `startup_failure` before job creation, with zero
  jobs and no combined status on the base head before this implementation increment.
- Next: collect more muted live runs across at least two approved feeds and
  then arrange a separate human audible comparison; preserve the 500 ms default
  until stronger latency/underrun and listening evidence supports a change.

- Implementation commit `7434ce9` is pushed. At 00:54 UTC PR #5 remained open
  and mergeable at that source head; GitHub showed no workflow run or combined
  status for `7434ce9`. The previous base run 308 failed at startup with zero
  jobs.

- Follow-up PR check after the documentation push: PR #5 is still open and
  mergeable. One immediate snapshot temporarily reported not-mergeable; the
  next snapshot returned mergeable, and a read-only merge-tree check found no
  conflicts against the fetched `main`. Actions run 310
  (`40797089319`) failed before creating jobs (`startup_failure`, zero jobs);
  no combined statuses were reported for the checked checkpoint.

## Buffer-ready diagnostics checkpoint (2026-10-29)

- Added `NativePlaybackDiagnostics::buffer_ready_latency_observed` and the CLI
  JSON field `buffer_ready_latency_observed_ms`. The playback worker records the
  first threshold-crossing `PlaybackEvent::Buffered` elapsed time, when it
  starts the output after the selected prebuffer is full. This happens outside
  the CPAL callback. Existing first-play timing remains host-polled after PCM
  consumption, so both stages can now be compared.
- Focused validation passed: all-feature `rust-radio-audio` library tests
  (54 passed, 1 ignored manual native-output check) and CLI tests (20 passed).
  The worker timing test verifies only the first observation is retained, and
  the CLI schema test verifies the new field.
- The prior muted WCRB native CPAL A/B used four 30-second runs per profile in
  alternating A/B/B/A blocks. At 200 ms, first-play observations were
  1294.653/1486.159/1567.604/1876.953 ms (mean 1556.342 ms); at 500 ms they
  were 1553.546/1808.120/1503.609/1432.657 ms (mean 1574.483 ms). The 500 ms
  mean was 18.141 ms slower, small compared with per-run variation. All eight
  windows reported zero callback underruns. Trials predate the new timing field
  and do not establish audible quality or listener preference.
- Fresh WCRB and Skylab validation attempts returned `TransportUnavailable` at
  `NotChecked`; network access prevented repeating live trials with the new
  field in this execution. No station catalog state was changed. The synthetic
  production-renderer buffer A/B still passed: injected 100/300/800 ms stalls
  yielded 0/100/600 vs 0/0/300 missing callback samples for 200/500 ms buffers.
- Next: repeat live alternating runs when transport is reachable and compare
  buffer-ready latency against first-play latency. Then repeat the full six
  workspace gates, and keep physical audible/Pi/device-loss acceptance open.

## Buffer-ready A/B with native output (2026-10-29)

- Fresh approved Skylab validation decoded MP3 at 48 kHz stereo; system output
  was present. Ran eight 30-second muted CPAL trials at 0% volume, in two
  A/B/B/A blocks. All callbacks consumed audio and every trial reported zero
  underrun samples.
- A (200 ms) buffer-ready latency: 823.548/793.718/748.176/760.733 ms,
  mean 781.544 ms; first-play latency: 847.891/845.760/788.943/775.387 ms,
  mean 814.495 ms.
- B (500 ms) buffer-ready latency: 709.117/640.956/947.020/592.612 ms,
  mean 722.426 ms; first-play latency: 749.045/711.646/997.585/618.923 ms,
  mean 769.300 ms. In this one small block, B averaged 59.118 ms faster to
  buffer-ready and 45.196 ms faster to first play. Per-run ranges were broad,
  with zero underruns for both variants; this is inconclusive, muted evidence,
  not a robust latency win or listening preference. Keep the 500 ms default
  provisional and collect more runs across feeds plus human audible evidence.
- All six required gates passed after the diagnostics implementation: formatting,
  default workspace tests, strict default Clippy, all-feature workspace tests,
  strict all-feature Clippy and all-feature check. The first default test run
  could not bind loopback fixtures under the sandbox; the authorized retry
  passed, as did the all-feature test run with local fixture socket access.
- No physical device, audible, Pi or receiver acceptance is implied by the muted
  callback trials. A checked external probe now succeeds again, but network
  availability was intermittent during the earlier unprivileged probes.
- Pushed source commit `7eaccfd39f051cb6f1a8b683d0f967abcc2ce6a1` and evidence
  checkpoint `d12782975f4dec32c4fed3c3f65231d219bdbca0` to the existing PR branch.
  PR #5 remains open. Actions run 312 (`40798866432`) for the evidence head
  failed before creating jobs (`startup_failure`); no combined status checks
  were reported. All local required gates passed on the pushed code.

## Second-feed buffer-ready A/B (2026-10-29)

- After refreshing the pushed branch and PR state, bounded validation decoded
  WCRB Classical as AAC, 48 kHz stereo. Ran eight 30-second muted native CPAL
  trials on system output at 0% volume in two A/B/B/A blocks. All eight
  completed and reported zero callback underrun samples.
- A (200 ms) buffer-ready latency: 1411.588/1692.799/1641.195/1647.477 ms,
  mean 1598.265 ms; first-play latency: 1452.390/1714.706/1698.628/1661.843
  ms, mean 1631.892 ms.
- B (500 ms) buffer-ready latency: 1345.564/1793.285/1566.869/1485.453 ms,
  mean 1547.793 ms; first-play latency: 1366.842/1824.567/1625.744/1498.031
  ms, mean 1578.796 ms. B averaged 50.472 ms faster to buffer-ready and
  53.096 ms faster to first play. Per-trial latency varied by hundreds of
  milliseconds, so this is a small muted sample, not proof of a 500 ms profile
  advantage or evidence about audibility.
- Across Skylab and WCRB, B averaged 45–59 ms faster on these measurements and
  all 16 trials had zero callback underruns. The direction repeats across two
  feeds, but both are single eight-run blocks and the sample remains too small
  to change defaults. Keep 500 ms provisional; repeat on separate days and
  gather a human audible comparison.
- GitHub Actions run 313 (`40798920739`) for pushed checkpoint `c24bd5d` failed
  before job creation (`startup_failure`); no combined statuses were present.
  The PR is open with no reviews. This checkpoint records the second-feed
  measurements and retains the hardware, audible and CI gates.

## Decoded-audio buffer-fill comparison (2026-10-29)

- Added `prebuffer_fill_latency_observed_ms`, measured on the decode worker from
  the first decoded PCM packet until the output reaches the configured
  prebuffer threshold and starts. This separates buffer fill from connection,
  probing and decoder startup. No timing or mutex work was added to the CPAL
  callback. CLI JSON and the standalone acceptance procedure include the field.
- Both approved feeds decoded immediately before testing: Skylab MP3 48 kHz
  stereo and WCRB AAC 48 kHz stereo. Each feed received eight 30-second muted
  CPAL trials on system output at 0% volume, A/B/B/A repeated twice. All 16
  trials completed and reported zero callback underrun samples.
- Skylab A=200 ms fill samples: 68.142/57.965/56.153/68.780 ms (mean 62.760);
  B=500 ms: 70.150/308.547/162.379/67.332 ms (mean 152.102). B took 89.342 ms
  longer to fill on average. Total buffer-ready means were 706.213 ms (A) and
  886.484 ms (B); first-play means were 761.332 ms (A) and 921.154 ms (B).
- WCRB A fill samples: 176.892/144.593/64.507/99.943 ms (mean 121.484); B:
  230.141/221.973/288.302/77.832 ms (mean 204.562). B took 83.078 ms longer
  to fill on average. Total buffer-ready means were 1408.225 ms (A) and
  1585.672 ms (B); first-play means were 1426.773 ms (A) and 1618.475 ms (B).
- In these runs, the 500 ms profile took about 83–89 ms longer from first
  decoded PCM to threshold and 160–192 ms longer to first play. Skylab's bursty
  response delivery let three of four B runs fill much faster than 500 ms; one
  took 309 ms. Network and decoder startup still varied substantially. Zero
  underruns in both profiles means this sample did not show a drop-out benefit
  under real stalls. Keep 500 ms provisional because synthetic stall evidence
  favors it under delayed chunks, but collect jittered live tests and audible
  user comparisons before choosing a final default.
- All six workspace gates passed on the implementation: format, default tests,
  strict default Clippy, all-feature tests, strict all-feature Clippy and
  all-feature check. Audio all-feature library suite: 55 passed, 1 ignored
  manual hardware test. CLI all-feature suite: 20 passed. Local socket fixtures
  passed with network permission. Physical/human listening acceptance remains
  untested.

## Paced prebuffer timing regression (2026-10-29)

- Added `measured_prebuffer_fill_tracks_paced_decoded_packets` to the IP sink
  tests. It sends one decoded stereo packet every 10 ms through production
  `SinkState`, uses 4-sample and 8-sample thresholds, and calls the same
  prebuffer-fill timing recorder when the actual `Buffered` event fires.
  The larger threshold must report a later fill time. Focused test passed.
- Repeated the full six required gates with this fixture included: formatting,
  default workspace tests, strict default Clippy, all-feature tests, strict
  all-feature Clippy and all-feature check all passed. The all-feature audio
  suite reports 56 passed and one ignored manual hardware test; all-feature
  CLI suite reports 20 passed.
- Test commit: `e60f6e8e427a7d13f01cb364f5d69db89c3deb0d`; measured A/B values
  and interpretation remain in the preceding `Decoded-audio buffer-fill
  comparison` section and machine-readable `buffer_fill_ab_latest` record.

## Paced packet-stall prebuffer A/B (2026-10-29)

- Added `prebuffer_ab_measures_fill_and_underruns_under_injected_packet_stalls` in `crates/rust-radio-audio/src/playback/ip_pcm.rs`. It feeds five paced 100ms decoded PCM packets through production `SinkState`, records the actual `Buffered` fill event, and injects 100/300/800ms delays after packet 2. A=200ms starts after packet 2; B=500ms starts after packet 5.
- Measured fill latency for A was 0.372/0.023/0.065ms; B was 317.574/523.663/1006.891ms. The synthetic mock callback modeled missing samples A=0/100/600 and B=0/0/0 across the 100/300/800ms stall profiles. The injected delay occurs before B reaches its start threshold, so B has not started callback consumption during that gap.
- This fixture models callback consumption and is not a physical CPAL callback or audible acceptance. The separate production `render_samples` regression continues to report missing samples A=0/100/600 and B=0/0/300 for those same stall durations.
- Focused regression passed. All six required gates passed: format, default workspace tests, strict default Clippy, all-feature workspace tests, strict all-feature Clippy, and all-feature check. All-feature audio suite: 57 passed, 1 ignored manual hardware test; CLI suite: 20 passed. Local loopback fixture access was granted for workspace test runs.
- Test commit `f56f8c680b5278ab6297554825a5d0b6c63cdb83`; documentation checkpoint follows. PR #5 remains open. The prior pushed head had no visible current Actions run/status; older run 313 failed before job creation with `startup_failure`. Hardware and listener A/B evidence remain outstanding.
- Pushed code test commit `f56f8c680b5278ab6297554825a5d0b6c63cdb83` and evidence checkpoint `4f5b3b7c9b7b1e30ecc47ee6024e21087d2ecb92` to the existing PR #5 branch. PR remains open and mergeable with no reviews. GitHub showed no Actions run or combined status for the new head at 01:39:40 UTC; older run 313 on `c24bd5d` remains `startup_failure` with zero jobs. Local/remote branch heads match.
- Final refreshed PR lookup briefly returned `mergeable=false`, then `mergeable=true` on repeat for head `bae21d6`; the PR is open. GitHub still showed no workflow run or combined status for this head at 01:39:57 UTC.
- GitHub PR metadata for final code/documentation head `cdac5f2` alternated between mergeable true and false; latest lookup at 01:40:15 UTC said false. A read-only `git merge-tree` against freshly fetched `origin/main` (`e67760e`) completed cleanly, so no textual merge conflict was found. GitHub showed no run or combined status for this head.
- Rechecked the exact pushed PR head `31b5256` at 01:40:40 UTC: PR #5 open and mergeable, with no Actions run or combined status. This final documentation update changes only the progress checkpoint.

## Native live prebuffer A/B, second cross-feed block (2026-10-29)

- Starting head `a58a6c40cf13f35bd1e71058c5110f733f85f56d`, validated both feeds immediately before playback: Skylab decoded MP3 at 48 kHz stereo; WCRB decoded AAC at 48 kHz stereo. Each feed then ran four 30-second muted native CPAL callbacks at 0% volume on system output, A/B/B/A repeated twice. A=200ms low-latency; B=500ms resilient. All 16 runs completed and reported zero callback underrun samples.
- Skylab A fill ms: 65.865/66.802/143.723/284.395; buffer-ready: 718.433/735.800/1014.258/886.306; first-play: 730.811/783.195/1046.484/944.865. B fill: 74.595/86.665/88.052/89.247; buffer-ready: 621.522/608.743/751.864/709.097; first-play: 674.292/654.314/796.981/771.128. Mean B-minus-A was -55.556ms fill, -165.893ms buffer-ready, -152.160ms first-play.
- WCRB A fill ms: 206.467/182.456/224.911/239.270; buffer-ready: 1664.682/1596.908/1755.775/1759.257; first-play: 1721.074/1617.695/1768.678/1774.921. B fill: 134.977/127.689/184.996/81.466; buffer-ready: 1467.701/1390.125/1518.589/1463.662; first-play: 1494.259/1445.931/1553.514/1524.243. Mean B-minus-A was -80.994ms fill, -234.136ms buffer-ready, -216.105ms first-play.
- In this one 4-trial/profile block, B averaged faster for all three metrics on both feeds. The ranges still show substantial run variance (Skylab A fill ranged 65.865–284.395ms); all profiles had zero reported underruns. Thus the live sample does not show a dropout benefit from the larger buffer. Volume was zero, so it gives no human audible preference evidence. Keep the 500ms profile/default decision provisional and repeat under measured jitter plus separate listening.
- Both station validations and all timed playback commands completed. No source code changed in this run; the existing six local gates remain the latest code validation from the preceding test commit. PR #5 was open and mergeable at checked head `a58a6c4`, with no workflow run or combined status visible. Exact raw records and deltas are in `docs/AGENT_PROGRESS.json` under `live_prebuffer_ab_native_latest`.
- Current PR/CI refresh at 01:53 UTC: PR #5 open at `bb6ea7b`; repeated GitHub lookups alternated `mergeable=false` and `true` (latest true). `git merge-tree --write-tree HEAD origin/main` completed cleanly. GitHub showed no Actions run or combined status for the live-measurement head.

## Paced A/B wired to the production output renderer (2026-10-29)

- Strengthened `prebuffer_ab_measures_fill_and_underruns_under_injected_packet_stalls`: the test now feeds paced PCM through `SinkState` into an actual ring buffer and calls production `output::render_samples` to simulate callback consumption and update the same played/underrun atomics used by CPAL. `render_samples` is crate-visible for this internal integration; it remains absent from the external API. No CPAL hardware stream is opened in the unit test.
- Under 100/300/800ms gaps before the resilient profile starts, the measured A=200ms fill values were 0.470/0.218/0.210ms and prebuffer-gap underruns were 0/100/600 samples. B=500ms fill was 323.513/514.324/1018.320ms with 0/0/0 prebuffer-gap underruns. The synthetic cadence has five packets of 100 ring samples and models one ring sample as one millisecond.
- Then the test injects an 800ms callback consumption after playback starts. The production renderer reported A missing 600/700/700 samples and B missing 300/300/300, depending on how much A had left after the earlier prebuffer gap. The focused test and both full default/all-feature workspace suites pass.
- All six required workspace gates passed on this change: format, default tests, strict default Clippy, all-feature tests, strict all-feature Clippy and all-feature check. The manual physical hardware test remains ignored. This combines the production ring and callback algorithm but does not simulate device scheduling, a physical CPAL stream, real network timing, or audible output.
- PR #5 was open/mergeable at prior pushed head `05f3512`; GitHub showed no workflow run or combined status there. Historical run 313 (`40798920739`) ended `startup_failure` with zero jobs; an API detail lookup and jobs query confirmed no job logs exist. No root cause can be derived from this run.
- Regression commit `e55cbca59b7d95cf56266c8bb982edae80213e5b` was pushed. At 01:58:04 UTC GitHub reported PR #5 open and mergeable at this head, with no workflow run or combined status. A fresh merge-tree against `origin/main` completed cleanly. This following evidence commit records that status.
- Final PR refresh at 01:58:38 UTC: PR #5 remains open at `757f768`; GitHub's mergeability field again returned false after a true result on the same head. No current workflow run/status was visible. A fresh `git merge-tree` against refreshed `origin/main` completed cleanly, so there is no textual merge conflict in the local merge computation.

## Controlled loopback native prebuffer A/B (2026-10-29)

- Added `scripts/prebuffer_fixture_server.py`, bound only to `127.0.0.1`, and
  a generated 12-second, 44.1 kHz stereo 440 Hz MP3. The server paces 1600-byte
  chunks every 100 ms and injects one 500 ms gap after chunk 20. The test uses
  the real MP3 decoder, ring buffer and native CPAL callback at 0% volume; it
  does not contact a broadcaster or use a physical network.
- On macOS 26.5.2 ARM64, ran eight six-second trials on system output in two
  A/B/B/A blocks: A=200 ms low-latency and B=500 ms resilient. A underrun
  samples were 34048/48384/32000/28928 (mean 35840); B were
  4352/9472/8448/6400 (mean 7168), an 80% lower mean. B still missed samples
  in all four runs.
- A buffer-fill, buffer-ready and first-play means were 398.873, 673.248 and
  704.208 ms; B means were 717.800, 964.152 and 1004.130 ms. In this sample B
  added 318.926 ms to fill, 290.904 ms to buffer-ready and 299.922 ms to first
  play. This is a measurable synthetic resilience/startup tradeoff, not a
  default decision, broadcaster jitter result, audible comparison or hardware
  acceptance.
- An earlier schedule placed the pause after chunk 10, while probing could
  still be buffering; one A trial had zero misses. Those results were excluded.
  The procedure now places the pause after chunk 20 and directs operators to
  compare the measured buffer-ready time with the pause position on each host.
- Python syntax/help, fixture duration (12.042438 s), JSON parsing and
  `git diff --check` passed. No Rust source changed, so Cargo gates were not
  rerun; the last six-gate evidence remains on the preceding code commit.
- PR #5 was open and reported mergeable at head `e6818ef`. Latest Actions run
  326 (`40803639508`) failed with `startup_failure` before job creation; no jobs
  or combined status checks were present. Remote CI remains unverified.

- Final checkpoint at 2026-10-29 02:07:57 UTC: pushed fixture/evidence commit
  `66d2b9ec95b940a834b31d699108b477f7f38478` to PR #5. GitHub reports the PR
  open but currently returns `mergeable=false`; local `git merge-tree` against
  refreshed `origin/main` completes cleanly. No Actions run or combined status
  is visible for this head. The previous head `e6818ef` had run 326
  (`40803639508`), which failed with `startup_failure` before job creation and
  created zero jobs.

## Denominator-aware native prebuffer A/B under deterministic jitter (2026-10-29)

- Extended `scripts/prebuffer_fixture_server.py` with a deterministic schedule
  of gaps: 350 ms after chunk 20, 500 ms after chunk 38, and 250 ms after chunk
  54. The same generated MP3 is delivered in 1600-byte chunks paced at 100 ms.
- Added `NativeRadioEngine::output_samples_consumed()` and CLI JSON field
  `output_samples_consumed_current_attempt`, measured from samples consumed by
  the CPAL callback. For uninterrupted trials, missing-sample share is
  `underrun_samples / (underrun_samples + output_samples_consumed_current_attempt)`.
  Output consumption is per attempt, while underruns are session-cumulative, so
  do not compute the ratio after an output recovery.
- Ran eight muted eight-second native CPAL trials on macOS 26.5.2 ARM64/system
  output at 0%, A/B/B/A twice. A=200 ms fill/buffer-ready/first-play means were
  465.716/681.857/728.336 ms. B=500 ms means were 717.970/934.037/972.242 ms.
  B added 252.254 ms to fill, 252.179 ms to buffer-ready and 243.906 ms to
  first play on average.
- A consumed [557312,558336,549120,567552] output samples and had
  [88832,90880,88832,87808] underrun samples, for a 13.7658% missing share.
  B consumed [552192,553216,561408,560384] and had [69376,65280,70400,70400]
  underrun samples, for an 11.0065% share (20.05% lower relative, 2.76 points
  lower). Both profiles still had substantial silent samples; this is a single
  synthetic muted block, not audibility, listener preference or hardware proof.
- All six required Cargo gates passed after adding the metric: fmt, default
  tests, strict Clippy, all-feature tests, strict all-feature Clippy and
  all-feature check. Focused CLI serialization and callback attempt/reset tests,
  fmt, Python syntax/help and `git diff --check` passed after the final JSON
  field clarification.
- Latest Actions run 328 (`40804563359`) on preceding head `14eb1ed` failed with
  `startup_failure` before job creation. The jobs query returned no jobs; PR
  diagnostics returned empty checks, annotations and logs. No root cause is
  available from GitHub. Hardware, audible, Windows MSVC, rights and Rust-Boy
  gates remain open.

- Pushed implementation `83fc64cecaed823b0bef98c617707cf614b00824` and refreshed
  this checkpoint at 2026-10-29 02:18:45 UTC. PR #5 remains open; GitHub currently
  says `mergeable=false`, while local merge-tree against fetched `origin/main`
  is clean (tree `e48aa17a46dcb4c969eb3c10a7f92cd658dc5700`). No workflow run or
  combined status is visible on this exact head. Prior head `14eb1ed` run 328
  failed before job creation; diagnostics returned no jobs, checks, annotations
  or logs.

## Live-feed buffering comparison across MP3 and AAC (2026-10-29)

- Revalidated publisher-approved Skylab as MP3 48 kHz stereo and WCRB Classical
  as AAC 48 kHz stereo. Each feed received eight 30-second muted native CPAL
  trials on system output, A/B/B/A repeated twice (A=200 ms, B=500 ms). All 16
  trials completed and every run reported zero callback underrun samples.
- Skylab A fill/buffer-ready/first-play means were 69.927/624.650/658.095 ms;
  B means were 156.235/841.651/869.374 ms. B averaged 217.001 ms slower to
  buffer-ready, while B run-to-run latency spread was substantial.
- WCRB A means were 156.777/1551.381/1590.584 ms; B means were
  174.520/1546.533/1586.083 ms. The buffer-ready means differed by only
  -4.848 ms (B faster); latency variance exceeded this difference.
- Both feeds showed zero missing samples in both profiles, so this trial did
  not measure a real-feed dropout advantage. Volume was 0%, so it provides no
  audible-quality or listener-preference evidence. Keep the profile decision
  provisional. Exact per-trial samples and callback counts are in
  `docs/AGENT_PROGRESS.json` under `live_cross_feed_native_ab_followup`.
- Current PR #5 head `137f56a` is open and GitHub reports mergeable. Actions run
  330 (`40805620409`) failed with `startup_failure` before job creation; the job
  and PR diagnostics queries returned no jobs, checks or logs.

## Multi-gap A/B with delivery headroom (2026-10-29)

- Repeated the loopback MP3 A/B at 90 ms chunk intervals with the same gaps
  (350 ms after chunk 20, 500 ms after 38, 250 ms after 54), eight muted
  eight-second native CPAL trials in A/B/B/A order twice. This provides about
  11.1% average delivery headroom for 100 ms audio chunks between gaps.
- A=200 ms means: buffer fill 209.877 ms, buffer-ready 437.336 ms, first play
  473.234 ms, consumed 607598 samples, 62099 underruns, 9.2726% missing share.
  B=500 ms means: 716.541 ms, 974.650 ms, 1005.239 ms, consumed 605184,
  17152 underruns, 2.7561% missing share. B reduced missing share 70.28%
  relative (6.52 points) while adding 537.314 ms to buffer-ready latency.
- This synthetic condition illustrates that extra delivery capacity between
  gaps lets the larger buffer refill; the exact-real-time 100 ms schedule had
  a smaller 20% relative reduction and 252 ms startup cost. Neither schedule
  is a broadcaster network model. Keep the default provisional; results are
  muted and do not measure listener preference.
- PR #5 remained open/mergeable at `9d8181b`. Actions run 331
  (`40806730055`) failed with `startup_failure` before jobs; jobs/diagnostics
  returned no jobs, statuses, annotations or logs.

## Captured transport-timing replay A/B (2026-10-29)

- Added bounded response-body timing capture to the IP transport guard. Each
  guard retains at most 256 samples (body size and interval); aggregate timing
  counters remain unchanged. The samples describe `reqwest` body chunks
  observed by the transport pump, including downstream backpressure, rather
  than packet-arrival times.
- Captured two approved-feed traces and committed sanitized size/interval-only
  fixtures: WCRB has 76 samples/525,211 bytes over 6.402 seconds; Skylab has
  206 samples/369,847 bytes over 8.051 seconds. No station URL or audio is in
  these timing fixtures. Added a helper to extract future traces from the
  ignored live timing test and extended the loopback server to replay the
  measured intervals and response-body sizes into generated synthetic audio.
- Replayed each trace against the same generated 40-second MP3 on this Mac at
  0% volume using native CPAL system output. Each profile ran four 8-second
  trials in A/B/B/A order twice (A=200 ms, B=500 ms). All 16 runs succeeded;
  every run had zero callback underrun samples.
- WCRB means A/B: decoded fill 212.010/215.908 ms, buffer-ready
  532.599/409.206 ms, first-play 568.280/449.448 ms. Skylab means A/B: fill
  213.190/218.439 ms, buffer-ready 482.740/433.046 ms, first-play
  522.626/467.579 ms. B's lower startup means are not a causal buffer benefit:
  there were no underruns to reduce and substantial run-to-run timing spread.
  Keep the default provisional; this replay is neither audible preference nor
  full-network or hardware acceptance. Per-run values are in
  `docs/AGENT_PROGRESS.json` under `captured_trace_replay_prebuffer_ab`.
- All six required locked workspace gates passed after the trace API/server
  implementation: formatting, default tests, default strict Clippy,
  all-feature tests, all-feature strict Clippy and all-feature check. The
  ignored public-feed test and physical hardware tests are separate.
- At source head `c6aac53`, PR #5 was open and GitHub reported mergeable. The
  latest workflow run (332, `40807044594`) ended `startup_failure` before any
  jobs; combined status remained empty. Remote CI is still unverified.

## Automated captured-trace A/B runner (2026-10-29)

- Added `scripts/run_prebuffer_trace_ab.py` to automate the eight-trial
  A/B/B/A-twice loop, validate that the CLI reports the intended 200/500 ms
  profiles, and save all diagnostics and aggregate missing-sample share as
  JSON. It accepts only HTTP URLs on `127.0.0.1` and defaults to 0% volume.
- Ran it against the committed WCRB body-timing replay on the Mac system
  output. All eight 8-second runs succeeded with zero callback underruns.
  A=200 ms means: fill 210.018 ms, buffer-ready 436.343 ms, first-play
  489.356 ms. B=500 ms: fill 216.527 ms, buffer-ready 446.401 ms, first-play
  477.779 ms. The startup measures are mixed and close relative to run spread;
  no reliability difference was measured. Keep the default provisional.
- The machine-readable run, including all individual measurements, is in
  `docs/AGENT_PROGRESS.json` under `captured_trace_ab_runner_followup`.
  Python syntax/help checks and rejected-URL checks passed. The full Cargo gate
  run from the prior checkpoint still applies to the unchanged Rust sources;
  no Rust code changed in this runner-only increment.
- Exact pushed head `f40c0b5` remains open/mergeable on PR #5. Actions run 333
  (`40808442029`) failed before job creation; the jobs endpoint and PR
  diagnostics returned no jobs, checks, statuses, annotations or logs. The
  recurring remote CI issue remains without actionable root-cause evidence.
- Next: use the saved runner output for repeat comparisons and move to
  cross-target compilation plus physical audio/device-loss acceptance when
  hardware is available. Human audible preference remains untested.

## Cross-target validation and Skylab trace A/B (2026-10-29)

- On source head `e1bb54d`, Windows GNU cross-target validation passed for the
  default, SDRconnect and direct nRSP-native workspace configurations:
  `cargo check`, strict all-target Clippy and `cargo test --no-run` passed for
  all three. The Windows test binaries were compiled but not run; GNU
  cross-compilation does not replace the Windows MSVC CI job.
- Headless `aarch64-unknown-linux-gnu` checks passed for direct nRSP-native and
  SDRconnect. The `aarch64-apple-darwin` IP-radio check also passed. SoapySDR
  host-feature check and strict Clippy passed; the feature test suite passed
  all 15 tests after rerunning with local TCP permission (the first attempt was
  denied loopback sockets by the sandbox).
- Repeated the muted native A/B using the captured Skylab timing trace: four
  eight-second trials per profile, A/B/B/A twice. All eight runs completed
  with zero callback underruns. A=200 ms means: fill 210.626 ms, buffer-ready
  497.759 ms, first-play 527.580 ms. B=500 ms: 213.526 ms, 433.065 ms,
  454.368 ms. B started faster on average in this block, while the prior WCRB
  block was mixed. With zero underruns on both, neither block establishes a
  reliability win or human preference; keep the default provisional.
- Raw cross-target commands, per-trial values and caveats are in
  `docs/AGENT_PROGRESS.json` under `cross_target_validation_2026_10_29` and
  `skylab_trace_ab_cross_target_followup`.
- PR #5 remained open/mergeable at `e1bb54d`. Actions run 334
  (`40808863100`) failed with `startup_failure` before jobs. The job endpoint
  returned zero jobs and PR diagnostics contained no checks, statuses,
  annotations or logs. The current remote CI failure still has no actionable
  root cause; no physical hardware was exercised.
- Next: pursue native MSVC CI and physical output/device-loss and SDR hardware
  acceptance; keep the A/B profile default provisional until audible and
  dropout evidence supports a change.
# Portability validation checkpoint (2026-10-29)

- On source head `93cf016`, the six required locked workspace gates passed
  after rerunning with local loopback socket access: format, default workspace
  tests, default strict Clippy, all-feature workspace tests, all-feature strict
  Clippy and all-feature check. The first restricted test attempt was denied
  socket binds for six existing local HTTP fixture cases; all six passed in the
  permitted rerun. One public-network timing test remains intentionally
  ignored by the normal suite.
- Headless/IP/audio/CLI `cargo check` passed on both installed cross targets:
  `aarch64-apple-darwin` and `x86_64-pc-windows-gnu`.
- Broader cross-target requests were limited by missing native toolchains:
  all-feature AArch64 Linux stopped in `ring` because
  `aarch64-linux-gnu-gcc` is absent; all-feature Windows GNU stopped in
  `soapysdr-sys` because no Windows pkg-config/sysroot is configured. These
  are toolchain gaps, not Rust diagnostics. Windows MSVC remains the required
  native CI target.
- PR #5 remains open and mergeable at `93cf016`. Actions run 365
  (`40974691523`) on parent `93cf016` ended `startup_failure` before job
  creation; GitHub returned no jobs or combined statuses. No Actions run or
  combined status is visible yet for checkpoint `60925e4`. No source code
  changed in this checkpoint.
- The WPR 350/500 ms A/B from the prior checkpoint remains inconclusive, with
  paired latency ranges crossing zero. Keep the 500 ms default; no new live A/B
  was run because no distinct experimental condition was identified.
- Next: restore actionable Windows MSVC CI, then use physical Bluetooth/USB,
  Pi and receiver acceptance; publisher distribution review and Rust-Boy
  adoption also remain open.


## WFMU Freeform buffering A/B (2026-10-29 23:45 UTC)

- Silent curated-station preflight decoded WFMU Freeform as MP3, 44.1 kHz stereo (1152 frames), without opening audio output.
- Eight counterbalanced native trials compared balanced 350 ms and resilient 500 ms for 30 seconds each on macOS ARM64/system output at 0% volume. All eight completed with zero callback underruns.
- Buffer-ready means/medians were 574.294/567.796 ms vs 569.996/558.109 ms. Paired A-minus-B median was -14.088 ms (range -47.260 to +92.626 ms); first-play median was -18.951 ms (range -58.663 to +66.713 ms). Both ranges cross zero, so this second-feed comparison does not establish a latency or reliability winner, and muted trials measure no listener preference. Keep the 500 ms default.
- Full decoded preflight and individual measurements are in `docs/AGENT_PROGRESS.json` under `live_station_wfmu_balanced_resilient_ab`; the original harness artifact is `/private/tmp/rust-radio-wfmu-balanced-resilient-ab-2026-10-29.json`.
- The A/B runner regression suite passed 12/12 and the CLI built successfully.

PR/CI refresh at 2026-10-29 23:49 UTC: PR #5 is open at `52e3bb4`. GitHub returned `mergeable: false`, while a fresh local merge-tree against `origin/main` was clean. No exact-head Actions run or combined status was visible. Treat GitHub’s mergeability field as an unresolved snapshot, not a confirmed conflict.


## Curated station silent-validation sweep (2026-10-29 23:50 UTC)

- `validate-all --json` checked all 25 curated feeds without opening an audio output. Twenty-four decoded successfully: 19 MP3 and 5 AAC. Skylab was the only unavailable result; it had no decoded format evidence and returned `TransportUnavailable` before endpoint/header validation. An immediate single-station check returned the same unavailable stage with one attempt and endpoint unknown.
- This is a point-in-time transport failure, not evidence that the publisher removed or permanently changed the feed. The catalog entry is preserved; retry in a later validation window before any endpoint or availability decision.
- The full sanitized JSON results and recheck are in `docs/AGENT_PROGRESS.json` under `live_catalog_sweep_2026_10_29`. The sweep artifact is also saved at `/private/tmp/rust-radio-catalog-validation-2026-10-29.json`.


## Sanitized silent-probe transport diagnostics (2026-10-30 00:03 UTC)

Implementation commit: `bb74393`.

- Added separate `TransportTimeout` and `TransportConnectionFailed` errors for bounded native probes. Reqwest error values and URLs are never surfaced; user-facing details come from fixed messages. All transport failures retain the existing `Unavailable` status and `NotChecked` stage until an HTTP response is received.
- After the earlier outage window, the Skylab validator again returned `Unavailable`/`NotChecked`; it now reports `Radio stream connection failed before a response`. Bounded HEAD and rate-limited GET requests to the publisher-listed host both received a connection reset and zero bytes. The publisher’s [official listening page](https://www.skylabradio.com/listen) still lists a high-quality online stream, so this is not evidence to remove the station or replace its endpoint. Recheck after a later recovery window.
- Added tests for safe timeout/connect category selection and validation-report output. All six required locked workspace gates passed; one live-timing suite remains intentionally ignored. The prior WFMU A/B remains inconclusive and the 500 ms default stays unchanged.
- The point-in-time transport result and validation are recorded in `docs/AGENT_PROGRESS.json` under `probe_transport_diagnostic_2026_10_30`.

- PR #5 is open and mergeable at `0384534`; Actions run 371 (`40977280806`) on this exact head failed with `startup_failure` before job creation. GitHub returned zero jobs and no combined statuses.


## Cross-target validation of probe diagnostics (2026-10-30 00:06 UTC)

- `cargo check --workspace --locked --target x86_64-pc-windows-gnu --exclude rust-radio-sdr` passed.
- `cargo check --workspace --locked --target aarch64-apple-darwin --exclude rust-radio-sdr` passed.
- Both checks compiled core, IP, audio, GUI and CLI for the default feature path. Optional receiver crate was explicitly excluded; these do not replace native Windows MSVC CI, receiver-feature builds, or physical hardware acceptance.
- Results are in `docs/AGENT_PROGRESS.json` under `cross_target_probe_diagnostics_2026_10_30`.


## Probe timeout/connect regression and WFMU A/B repeat (2026-10-30 00:12 UTC)

- Added bounded loopback tests using real `reqwest` errors: an accepted connection that withholds a response reaches the request timeout category; a closed local port reaches the connection-failed category. Both are converted to fixed, sanitized messages. Focused probe suite passed 7/7 and station-validation suite passed 9/9.
- Reran the six locked workspace gates with local fixture socket access. Formatting, default/all-feature tests, strict Clippy for both configurations, and all-feature check passed. One public live-timing test remains intentionally ignored.
- Retried Skylab at Unix time `1793319164`: still `Unavailable`, `NotChecked`, one attempt, and no codec evidence. Keep its publisher-documented feed listed; recheck in the next scheduled validation window.
- Ran another counterbalanced WFMU 30-second A/B block (balanced 350 ms vs resilient 500 ms, eight muted trials at 0% volume). All eight had zero underruns. Means were 491.996/524.810 ms. Paired A-minus-B buffer-ready median was -56.759 ms (range -116.540 to +98.801 ms); first-play median was -55.726 ms (range -157.109 to +105.583 ms). Ranges cross zero. No reliable winner or listening preference follows; keep the 500 ms default.
- Raw WFMU records and the recheck are in `docs/AGENT_PROGRESS.json` under `live_station_wfmu_balanced_resilient_ab_repeat_2026_10_30` and `latest_probe_failure_regression_validation_2026_10_30`.


## URL-redaction assertions and later catalog repeat (2026-10-30 00:21 UTC)

- Strengthened the real loopback timeout/refusal tests to assert that the safe classified message omits the original request URL. Focused tests passed 2/2 after this test-only change. The full six workspace gates passed immediately before it on implementation `bb74393` plus the initial fixture tests (`6ed5474`).
- A repeat `validate-all --json` sweep checked all 25 curated feeds: 24 decoded (19 MP3, 5 AAC); Skylab remained unavailable at `NotChecked` after two attempts, with no response or codec evidence. No other station failed. Preserve Skylab and recheck in the next scheduled validation window.
- Full sanitized results are in `docs/AGENT_PROGRESS.json` under `live_catalog_sweep_repeat_2026_10_30`; raw sweep output is `/private/tmp/rust-radio-catalog-validation-repeat-2026-10-30.json`.
