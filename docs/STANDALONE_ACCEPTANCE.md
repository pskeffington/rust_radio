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

Validation outcomes distinguish unverified catalog records, unavailable endpoints, invalid stream responses, unsupported formats/transports, cancellation and decoded audio. The stage distinguishes endpoint reachability, an accepted audio response, a supported codec and actual PCM decoding. Unrecognized or truncated headers may report Unsupported because no format reader can identify them. A decoded result is a point-in-time codec check, not proof of audible playback or continued availability. Reports are returned through the Rust API, printed by the CLI, and retained in memory by the GUI. Persistence is opt-in: `validate-all --json --history PATH` writes bounded local history atomically; the GUI does not persist it automatically.

The validator reuses the publisher allowlist, redirect/playlist limits and ICY reader. One 18-second HTTP deadline covers the complete request chain; audio reads are capped at 256 KiB and 64 packets. Synthetic MP3/AAC fixtures exercise this path in offline CI. Live checks are separate and require network access; the synchronous Rust API belongs on a host worker thread.

## Acceptance record

On a Mac with actual audio output, record OS/audio device, UTC timestamp, commit SHA, station, output selection, playback status and errors, start/stop responsiveness, 30-second completed sample playback, and stream permissions. Then test invalid station/duration (no I/O), disconnected Bluetooth, interrupted HTTP, and stop during buffering. On Raspberry Pi, separately measure CPU, memory and power.

## Current evidence

Offline tests cover stream parsing, recovery and silent MP3/AAC validation. Audible playback, GUI interaction and device-loss behavior still require tests on the target hardware; the earlier operator-confirmed Mac Skylab pass does not certify later commits.

On 2026-10-27, explicit live `validate` checks decoded PCM from all ten curated stations without opening audio output. Skylab returned MP3 at 48 kHz stereo; Groove Salad, Secret Agent, Lush, Drone Zone, Deep Space One, Space Station Soma, Beat Blender, DEF CON Radio and Underground 80s returned MP3 at 44.1 kHz stereo. SomaFM required upgrading legacy HTTP playlist entries to their allowlisted HTTPS equivalents. These results establish point-in-time decoding only; rerun checks to establish current availability.

## Catalog rights and current live evidence (2026-10-28)

The earlier SomaFM sweep documents historical codec availability only. After
reviewing the publisher's current Terms of Service, SomaFM presets and their
HTTP-to-HTTPS playlist compatibility rule were removed pending written
permission. The directory initially retained five entries: Skylab Radio, WCRB
Classical, GBH 89.7 News Radio, KEXP 90.3 FM and Radio Paradise Main Mix. A
subsequent publisher-source review added WPR News and Music, Radio Swiss Jazz
and Classic, All Classical, WFMT and KFUO, bringing the directory to twelve.
WRVO's documented NPR News, PRX Remix and BBC World Service player streams
expanded it to fifteen; the first is NPR-member station programming, not a
national continuous NPR feed.
WFMU's main, Give the Drummer Radio, Rock'n'Soul and Sheena's Jungle Room MP3
streams, each linked from its official third-party-player listings, expanded
the current catalog to nineteen.
Classical California, KUNC, The Colorado Sound and three KUAF services brought
the directory to twenty-five. A bounded live sweep at 2026-10-28 22:14:21 UTC
decoded all twenty-five entries in one attempt each. This is point-in-time
decoder evidence only; rerun the sweep for current endpoint health. No physical
output or device acceptance follows from these source additions.
Each publisher documents listener playback in third-party radio players or
internet radios; this does not authorize redistribution or relay. Latest sweep
results, including stations that did not decode, are recorded in dated progress
notes. Decoder evidence is point-in-time only. Physical output and hardware
acceptance remain separate.

On 2026-10-28 at 22:43 UTC, a fresh bounded `validate-all --json` run decoded
all 25 catalog entries in 25 total attempts. This confirms point-in-time decode
health only; it does not verify audible output, ongoing availability or stream
redistribution rights.


On 2026-10-28 at 23:34:35 UTC, bounded `validate-all --json` decoded all 25
current catalog feeds in 25 attempts, with no retries. This is point-in-time
codec evidence only and does not certify audible playback, device behavior, or
redistribution permission.

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

## Native buffering A/B measurements

The CLI exposes three bounded IP-radio profiles: `low-latency` (200 ms),
`balanced` (350 ms) and `resilient` (500 ms). Existing engine start methods
continue to default to `resilient`. Timed playback prints one JSON diagnostic
record. Keep station, device, volume and duration fixed; `--volume 0` keeps the
CPAL callback active while avoiding audible output. Repeat an alternating
order (A/B/B/A) to reduce warm connection and device-start bias:

```sh
cargo run -p rust-radio-cli -- listen skylab --seconds 30 --volume 0 --output system --buffering low-latency
cargo run -p rust-radio-cli -- listen skylab --seconds 30 --volume 0 --output system --buffering resilient
cargo run -p rust-radio-cli -- listen skylab --seconds 30 --volume 0 --output system --buffering resilient
cargo run -p rust-radio-cli -- listen skylab --seconds 30 --volume 0 --output system --buffering low-latency
```

Save each `RUST RADIO DIAGNOSTICS` JSON line with the UTC time, commit, station,
and output device. Compare `buffer_ready_latency_observed_ms`,
`prebuffer_fill_latency_observed_ms`, `first_play_latency_observed_ms`, and
`underrun_samples`. Buffer-ready latency is measured from playback start;
prebuffer-fill latency isolates elapsed time from the first decoded PCM until
the selected threshold is met. Both are recorded on the decoder worker when
the output starts, outside the audio callback. First-play latency is measured
when the CLI polls session status after callback PCM consumption, so it
includes up to one polling interval plus stream/decode variance. Zero-volume
callback measurements establish neither audible quality nor listener
preference; perform a separate audible trial and record human listening
observations before selecting a new default.

### WPR News live muted buffering comparison

At 2026-10-29 04:05 UTC, bounded silent validation decoded the curated WPR News
feed as MP3 at 44.1 kHz mono. Eight 15-second muted native CPAL trials compared
balanced 350 ms and resilient 500 ms with counterbalanced ordering; all had zero
callback underruns. Buffer-ready/first-play means were 1574.757/1606.892 ms vs
1456.876/1496.955 ms, while medians were 1615.866/1655.414 ms vs
1421.652/1472.593 ms. Paired A-minus-B medians were +132.680 ms for
buffer-ready and +123.552 ms for first-play, but both paired ranges crossed
zero. This single muted feed shows no dropout difference or human preference
and does not justify changing the default. Raw data is in
`docs/AGENT_PROGRESS.json` under `wpr_news_live_balanced_resilient_ab`.

### Controlled loopback network-stall comparison

This optional stress test streams a generated 12-second MP3 over loopback HTTP
with three repeatable gaps of different lengths after output has had time to
start. It uses the actual decoder, ring buffer and native CPAL callback, but no broadcaster or
physical network; set volume to zero to avoid audible output. The fixture server
binds only to `127.0.0.1`:

```sh
python3 scripts/prebuffer_fixture_server.py --stall-schedule '20:350,38:500,54:250' --chunk-bytes 1600 --interval-ms 100
```

In a second terminal, run both profiles in A/B/B/A order twice, keeping the
duration, output and fixture URL the same:

```sh
cargo run -p rust-radio-cli -- stream-listen 'http://127.0.0.1:18743/live' --seconds 8 --volume 0 --output system --buffering low-latency
cargo run -p rust-radio-cli -- stream-listen 'http://127.0.0.1:18743/live' --seconds 8 --volume 0 --output system --buffering resilient
cargo run -p rust-radio-cli -- stream-listen 'http://127.0.0.1:18743/live' --seconds 8 --volume 0 --output system --buffering resilient
cargo run -p rust-radio-cli -- stream-listen 'http://127.0.0.1:18743/live' --seconds 8 --volume 0 --output system --buffering low-latency
# Repeat the four commands above once more.
```

Compare the same JSON fields as the live-feed A/B. Each 1600-byte chunk is
about 100ms of the generated 128 kbps stream and is sent every 100ms; the
server pauses after chunks 20, 38 and 54 for 350, 500 and 250ms respectively.
This deterministic multi-gap profile represents changing delivery jitter,
not a statistical model of any broadcaster. An earlier schedule paused after
chunk 10 while probing could still be buffering, and one 200ms run had zero underruns
because the gap landed before playback. The later pause makes the measured
event post-start on this host, but check `buffer_ready_latency_observed_ms`
against the gap position on each platform before interpreting results. This
test does not establish a live station's jitter profile or audible quality.

The CLI also reports `output_samples_consumed_current_attempt`, counted by the
native output callback. For a run without output recovery, estimate missing
sample share as `underrun_samples / (underrun_samples +
output_samples_consumed_current_attempt)`. This denominator includes the
profile-specific time the callback actually ran and is more useful than raw
underrun totals when startup times differ. After recovery, the consumed count
resets per output attempt while underruns remain session-cumulative, so do not
combine them into a ratio for a session that retried.

The A/B runner preserves each trial and reports per-profile mean, median,
minimum and maximum for buffer-ready, first-play and prebuffer-fill latency. It
also pairs adjacent counterbalanced trials and reports A-minus-B mean, median
and range; negative deltas mean A was faster. With small trial counts, these
descriptive ranges are not confidence intervals and do not by themselves prove
a profile advantage.

On 2026-10-29, a repeated combined captured-Skylab-trace and injected-stall run
compared balanced 350 ms (A) with resilient 500 ms (B) across eight muted native
trials. All trials had zero underruns. Buffer-ready means were 483.464/468.548
ms and medians were 467.319/462.181 ms. The paired A-minus-B median was
+5.137 ms, with a -147.847 to +197.234 ms range; first-play paired median was
+27.006 ms (-149.995 to +173.167 ms). This remains inconclusive startup evidence
and is not live-feed or listener acceptance.

For repeatable loopback trials, `scripts/run_prebuffer_trace_ab.py` runs two
profiles in A/B/B/A order twice and stores per-trial JSON. The default compares
200 ms against 350 ms. Pass `--profile-a balanced --profile-b resilient` to
compare 350 ms against 500 ms, or choose any two distinct supported profiles.

For a repeat with average payload headroom between the same gaps, restart the
fixture with `--interval-ms 90`. The 1600-byte chunk carries roughly 100 ms of
encoded audio, so 90 ms pacing provides about 11.1% average delivery headroom.
Run the same A/B/B/A sequence; this deterministic delivery schedule still is
not a measured broadcaster network trace.

On 2026-10-29, eight muted native trials compared 200 ms vs 350 ms and then
350 ms vs 500 ms under the 90 ms pacing condition. Aggregate missing-sample
shares were 8.020%, 6.142%, and 5.834%, with mean buffer-ready latencies of
533.010, 653.123, and 674.922 ms respectively. The second block's 500 ms
profile reached 2.348% missing share at 979.370 ms mean buffer-ready latency.
Each comparison had four trials per profile, and all trials had underruns.
These are synthetic single-host measurements; the balanced option offers a
measured middle point but does not prove listener preference or justify
changing the engine's 500 ms default.

On 2026-10-29, the combined captured-Skylab timing plus 350/500/250 ms
post-start stall replay completed eight counterbalanced muted native trials.
Balanced 350 ms and resilient 500 ms both had zero callback underruns; mean
buffer-ready/first-play was 491.986/522.645 ms and 432.237/471.774 ms
respectively. This startup direction opposes the combined WCRB replay, and neither
block demonstrates a consistent profile advantage. Audio was synthetic; live
stream, audible preference and physical output-loss gates remain open.

### Captured response-body timing replay

The ignored live test captures at most 256 response-body chunk intervals and
sizes per approved feed. These are observations from the transport pump,
including downstream backpressure; they are not packet arrival times. Two
bounded sanitized samples are stored in
`crates/rust-radio-audio/tests/fixtures/` for repeatable replay. To refresh a
trace when network access is available:

```sh
cargo test --locked -p rust-radio-ip --test live_timing -- --ignored --nocapture > /tmp/rust-radio-live-timing.log 2>&1
python3 scripts/extract_live_timing_trace.py /tmp/rust-radio-live-timing.log --station wcrb --output /tmp/rust-radio-wcrb-trace.json
```

Start the loopback server with a captured or refreshed timing file and the
generated MP3 data, then run the A/B/B/A procedure above against `/live`:

```sh
python3 scripts/prebuffer_fixture_server.py --timing-trace-file crates/rust-radio-audio/tests/fixtures/wcrb-body-timing.json --fixture crates/rust-radio-audio/tests/fixtures/prebuffer-trace.mp3
```

In another terminal, automate the two A/B/B/A blocks and save every native
diagnostic plus profile summaries as JSON:

```sh
python3 scripts/run_prebuffer_trace_ab.py --output-file /tmp/rust-radio-trace-ab.json
```

The runner is loopback-only by default, verifies that each diagnostic reports
the intended prebuffer, and prints each trial followed by the complete result.
Use `--seconds`, `--output` or `--repetitions` to adjust a run; the default
keeps output muted and runs eight 8-second trials.

The server replays observed intervals and response-body sizes for the captured
samples, then resumes default pacing until the fixture or client ends. This
preserves a measured transport timing pattern while keeping the audio payload
synthetic. It does not reproduce the broadcaster's compressed audio, the full
network path, decoded-buffer state, or an audible dropout. Keep this separate
from live-feed and physical-device acceptance.

On 2026-10-29, each checked-in trace was replayed for four 8-second muted
native CPAL runs per profile in A/B/B/A order twice (A=200 ms, B=500 ms).
All 16 runs reported zero callback underrun samples. WCRB-trace buffer-ready
means were 532.599 ms (A) and 409.206 ms (B); Skylab-trace means were
482.740 ms (A) and 433.046 ms (B). The run-to-run spread was substantial and
neither trace produced underruns, so these startup means do not identify a
reliability winner or listener preference. Keep the default provisional. Raw
trials and capture metadata are in `docs/AGENT_PROGRESS.json` under
`captured_trace_replay_prebuffer_ab`.

The automated WCRB-trace runner was repeated on the same host: A/B buffer-ready
means were 436.343/446.401 ms and first-play means were 489.356/477.779 ms.
All eight trials again had zero callback underruns. The mixed startup result
and absent dropout separation do not justify a default change; per-run data is
in `docs/AGENT_PROGRESS.json` under `captured_trace_ab_runner_followup`.

A subsequent Skylab-trace block produced A/B buffer-ready means of
497.759/433.065 ms and first-play means of 527.580/454.368 ms, again with zero
underruns in all eight trials. The opposite WCRB/Skylab startup direction and
zero misses in both profiles make the mean differences inconclusive for a
buffering reliability decision.

On 2026-10-29 03:01 UTC, the loopback test was repeated with 90 ms chunk
pacing and the same three post-start gaps. Eight muted CPAL trials completed
on macOS ARM64. A=200 ms produced 7.151% aggregate missing-sample share and
632.250 ms mean buffer-ready latency; B=500 ms produced 2.483% and 998.023 ms.
This synthetic one-host comparison shows a reliability/startup tradeoff only;
it does not establish live-feed behavior or listener preference. Per-trial
records are in `docs/AGENT_PROGRESS.json` under
`loopback_multi_gap_ab_90ms_followup`.

Curated-station runs now validate the selected station before starting the
A/B trials. The runner calls `rust-radio-cli validate STATION --json`, requires
exactly one decoded-audio result matching the selected ID, and stores the version-1
validation envelope beside the playback measurements. The result excludes
resolved URLs and free-form diagnostics to avoid retaining sensitive endpoint
values. A failed or malformed preflight aborts before opening audio output.

On 2026-10-29, the WPR News preflight decoded MP3 at 44.1 kHz mono. The updated
runner then completed eight 8-second muted native trials (350 ms vs 500 ms), all
without callback underruns. Paired A-minus-B buffer-ready median was +225.357 ms
with range -323.771 to +626.593 ms. This wide range crosses zero; startup and
reliability remain inconclusive. Raw preflight and trials are recorded under
`single_station_json_preflight_wpr_ab` in `docs/AGENT_PROGRESS.json`.

For a live but still muted comparison, pass a curated station ID instead of a
fixture URL, for example:

```sh
python3 scripts/run_prebuffer_trace_ab.py --station skylab --seconds 30 --profile-a balanced --profile-b resilient --output-file /tmp/skylab-buffer-ab.json
```

The harness accepts only IDs in the machine-readable curated catalog and calls
the CLI's `listen` command, which enforces its publisher allowlist. At 2026-10-29
03:19 UTC, Skylab decoded as MP3, 48 kHz stereo. Eight 30-second trials at
volume 0% produced no callback underruns for either 350 ms or 500 ms. Mean
buffer-ready latency was 753.643 ms vs 934.959 ms and first-play latency was
802.896 ms vs 965.990 ms. This is one stable-feed, single-host result; it does
not establish listener preference or broader network reliability.

At 2026-10-29 03:31 UTC, WCRB Classical (GBH) decoded as AAC, 48 kHz stereo
before playback. Eight muted 30-second native trials (A/B/B/A twice, balanced
350 ms vs resilient 500 ms) all completed with zero callback underruns. Mean
buffer-ready latency was 2227.812 ms vs 1427.275 ms; first-play was
2266.739 ms vs 1462.261 ms. The balanced set included a 3541.560 ms startup
outlier; medians were 1953.414/1428.868 ms (buffer-ready) and
2002.450/1457.970 ms (first-play). This timing spread and zero underruns in
both groups do not identify a reliability winner. Muted trials do not measure
human preference. Raw trials are recorded in `docs/AGENT_PROGRESS.json` under
`live_station_wcrb_balanced_resilient_ab`.

The A/B runner now counterbalances trial starts: two repetitions use A/B/B/A
followed by B/A/A/B, so both profiles take the first-trial position equally.
On 2026-10-29, a repeated WCRB comparison completed eight muted 30-second
trials with zero callback underruns. Balanced 350 ms vs resilient 500 ms mean
buffer-ready latency was 1607.326 ms vs 1549.155 ms; first-play latency was
1648.815 ms vs 1573.061 ms. Two earlier fixed-order blocks each showed a
roughly 3.5-second delay on the first A trial; that outlier did not recur when
the starting profile was counterbalanced. This suggests an order/cold-start
effect but does not establish its cause or a reliability advantage. Detailed
measurements are in `docs/AGENT_PROGRESS.json` under
`live_station_wcrb_balanced_resilient_ab_counterbalanced`.

For reliability under paced stalls, a second counterbalanced comparison used
90 ms chunks with 350/500/250 ms gaps after chunks 20/38/54. In eight muted
8-second native trials, balanced 350 ms had a 4.246% aggregate missing-sample
share (109568 missing samples); resilient 500 ms had 0.735% (18432 missing).
Both profiles had underruns, but 500 ms reduced the missing share by 82.7% and
added about 206 ms buffer-ready / 183 ms first-play latency. This favors 500 ms
for the engine default when stream gaps are expected.

A counterbalanced replay of the captured WCRB response-body timing trace used
the same synthetic MP3 payload and observed chunk sizes/intervals. No trial
underran; 350/500 ms buffer-ready means were 437.115/550.701 ms and first-play
means were 471.148/589.770 ms. This trace shows the startup cost of additional
buffering under that captured pattern but cannot verify real broadcaster audio
or audible preference. Per-trial data for both tests is in
`docs/AGENT_PROGRESS.json` under `counterbalanced_wcrb_timing_trace_ab` and
`counterbalanced_90ms_multi_gap_ab`.

The loopback replay server can also add scheduled stalls on top of captured
WCRB body pacing. Eight counterbalanced muted 8-second trials (350 ms vs
500 ms) under the combined pattern had zero underruns for both. Mean
buffer-ready/first-play was 480.865/527.455 ms vs 420.987/469.475 ms. This
single replay had no reliability separation and timing direction differed
between profiles; it does not outweigh the separate multi-gap comparison that
favored 500 ms for missing-sample share. The server combination is covered by
`scripts/test_prebuffer_fixture_server.py`; detailed trial data is in
`docs/AGENT_PROGRESS.json` under `combined_wcrb_trace_and_stalls_ab`.


The WPR News 30-second follow-up repeated the decode preflight and eight muted
native trials in counterbalanced ABBA/BAAB order. All eight had zero underruns.
Balanced 350 ms and resilient 500 ms buffer-ready medians were 1490.077 and
1480.564 ms; paired A-minus-B median was +20.399 ms with a -404.680 to
+156.185 ms range. First-play paired median was +4.967 ms with a -439.430 to
+108.563 ms range. The ranges cross zero; retain the resilient engine default
until reliability or listening evidence supports a change. Full raw evidence is
in `docs/AGENT_PROGRESS.json` under `wpr_news_live_ab_30s_followup`.


The A/B runner accepts only the six playback diagnostic fields needed for its
summary and preserves no unknown CLI fields. It rejects non-finite, negative or out-of-timeout
latencies and mistyped, negative or out-of-range prebuffer and sample counts. Failed trials do not
echo raw CLI stdout or stderr, so a loopback query token or future URL-bearing
diagnostic cannot be copied into an error report. Regression coverage is in
`scripts/test_prebuffer_trace_ab.py`.


Machine-readable station validation no longer retains raw Content-Type header
parameters. Both single-station and catalog JSON keep only a validated lowercase
MIME type of at most 127 ASCII characters; malformed values become null. A WPR
News validation and eight-trial A/B run passed through the normalized envelope.
The 350/500 ms comparison had zero underruns, while paired startup ranges
crossed zero; it does not establish a profile winner. Raw data is in
`docs/AGENT_PROGRESS.json` under `wpr_news_mime_sanitized_ab_8s`.
