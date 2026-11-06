# Codex execution queue — Rust Radio (2026-11-05)

Status meanings: READY = scoped, not started; BLOCKED = missing prerequisite; IN_REVIEW = PR exists; DONE = implementation and evidence verified. These are proposed tasks, **not** reports of executed tests. Follow AGENTS.md.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-101 | P0 | Transport | IN_REVIEW | Reproduce Skylab validation/listening discrepancy with structured, redacted endpoint and stage diagnostics | Report the safe stage boundary supported by the transport API, add fixture/redaction and CLI compatibility regressions, and distinguish validation failure from output initialization failure; no autoplay |
| RR-102 | P0 | Validation | DONE | Catalog sweep with timestamped JSON/history review | Report each canonical ID, attempts, decoded stage, failures and known environmental limits; do not equate 25/25 historical decode with current availability |
| RR-103 | P0 | Audio | BLOCKED: physical device | Mac audio loss, device-switch and stop-recovery evidence | Explicit speaker/Bluetooth/USB manual matrix; no overlapping sessions |
| RR-104 | P1 | GUI | DONE | 480x320 control/state regression and a human operator checklist | Offline controls/layout regressions pass and a repeatable Mac operator checklist is documented; physical touch remains separate |
| RR-105 | P1 | SDR | BLOCKED: receiver access | Independent direct nRSP-ST live audio and control acceptance | Record receiver, handshake, tuning, PCM/callback, audible output, Stop cleanup |
| RR-106 | P1 | SDR | BLOCKED: USB hardware | SoapySDR 240ksample/s CS16 mono-FM field acceptance | Supported dongle/driver, actual RF/audio and unplug cleanup |
| RR-107 | P1 | Integration | BLOCKED: Pi 5 target | Pi runtime and Rust-Boy embedded host signoff | Pinned dependency, physical screen, native output, CPU/memory/thermal/power |
| RR-108 | P1 | Validation | BLOCKED: exact-head startup_failure, zero jobs | CI status audit and exact-SHA evidence reconciliation | Distinguish startup_failure from test failure and verify hosted jobs before completion |
| RR-109 | P2 | Research | BLOCKED: publisher permission | Broadcaster rights/approved stream audit, NPR candidate review | Source-linked provenance and license/embedding approvals before distributable preset changes |
| RR-110 | P1 | Transport | DONE | Reject invalid ICY metadata intervals at the public reader boundary | Direct `IcyAudioReader::new` calls with zero or above-limit intervals fail with `InvalidInput`; valid intervals and `None` remain unchanged; focused and workspace gates pass |
| RR-111 | P1 | Storage | DONE | Preserve the prior validated catalog-history report as an atomic `.bak` checkpoint | On update, an existing valid primary is copied to `<path>.bak` before replacement; backup failures preserve the primary; invalid primary is never overwritten; both files remain bounded and private; focused/workspace gates pass |

## Current execution order: RR-108 hosted CI, then physical acceptance gates
1. Fetch latest main and inspect actual station definition and stream path. Document the exact commit being tested.
2. Add fixture-first tests around the reproducible failure; do not infer the cause from a website response, old status, or one failed network attempt.
3. Run silent `cargo run -p rust-radio-cli -- validate skylab`; compare with `cargo run -p rust-radio-cli -- listen skylab --seconds 30` only when audio testing is explicitly authorized.
4. Capture redacted HTTP chain, final content type, codec probing and last verified stage without leaking URL query tokens. The transport must remain bounded and secure. If the HTTP client does not expose a safe sub-classification, retain a generic pre-response transport category rather than parsing error text.
5. If a defect is proven, implement the minimum change, add a regression and run required gates. If outbound network is unavailable, mark blocked and provide deterministic fixtures, not a false failed station verdict.
6. Open a PR, update queue/roadmap/progress and stop at unresolved hardware or rights gates. Avoid changing other repos unless the host pin is specifically in scope.

RR-101 execution update (2026-11-06): `validate skylab` returned unavailable at
`not_checked` before receiving a response; this is environment-qualified
transport evidence, not a broadcaster verdict. The muted `listen` comparison
failed while initializing CoreAudio before stream/output callback comparison.
The CLI now has opt-in redacted `--diagnostics-json`, and terminal listener
failure prints callback/output diagnostics. Failures before a response use a
generic transport category; diagnostics do not distinguish DNS, connection
setup, TLS, timeout or other I/O. Failed validations continue to emit their JSON report while
exiting nonzero, preserving the existing `--json` success/failure convention.
Local workspace tests pass; exact-head hosted CI is still
blocked by GitHub Actions `startup_failure` with zero jobs. See `PROGRESS.md`.

RR-102 execution update (2026-11-06): a 25-ID report at
`2026-11-06T08:01:40Z` matches the catalog exactly and records 24 decoded, one
Skylab unavailable after two attempts, and 26 total attempts. A first sweep
within the same minute had 23 decoded, Skylab unavailable, and WFMU unsupported;
the following sweep decoded WFMU, so retain its preset and treat the earlier
unsupported result as transient. Full redacted JSON and matching local history
are stored under `/Volumes/Data/Codex/rr101-skylab/` as
`rr102-catalog-validation-report-2026-11-06.json` and
`rr102-catalog-validation-report-history-2026-11-06.json`. No URLs or free-form
details are present. This is client-environment codec evidence, not audible
playback or a permanent station-health verdict. Continue with RR-108 exact-head
CI reconciliation. Exact PR head `f8adc633f0b5ed1581277f7a4de015470e46c871`
had Actions run 392 (`42185406559`) fail with `startup_failure` before jobs;
jobs, statuses and logs were empty. A retry request was rejected by GitHub with
403 because the run cannot be retried. Local required gates and detailed skill
provenance are recorded in `PROGRESS.md`.

## Parallel review
Validation agent independently checks the proposed reproduction, cancellation, stale-result handling, no autoplay, private endpoint filtering and evidence for claimed success. Integration agent reviews Rust-Boy API compatibility but does not update its dependency pin without explicit host validation.

RR-101 follow-up and required FACTORY-002 report are pushed to PR #5 at `324c32c733e986ee2936d82302fd76f371dd8855`. The sanitized issue report is recorded at [agent_skills #14](https://github.com/pskeffington/agent_skills/issues/14) (comment `6680570338`). RR-101 implementation review and local gates are complete; hosted exact-head CI remains outstanding. Before RR-111, Actions run 396 on documentation head `4033aec18dfcaa7cfd3611d49822719600f62f18` ended `startup_failure` before jobs; combined statuses were empty.

RR-110 public ICY reader hardening was verified locally and is complete: direct constructors reject zero and above-limit metadata intervals with `InvalidInput`, while valid boundaries and `None` retain their behavior. See the dated evidence in `PROGRESS.md`; exact-head Actions startup failure remains a separate RR-108 blocker.

RR-111 history backup hardening is complete: subsequent valid history updates atomically checkpoint the previous validated bytes to `<path>.bak` before replacing the primary. Backup failure leaves the primary intact; invalid primary input is preserved and rejected. No interprocess lock or parent-directory sync was added.
