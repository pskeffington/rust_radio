# RR-111 — Credential-isolated external provider contract

Date: 2026-11-26. Status: IN REVIEW (proposed policy and design, not runtime acceptance).
Main baseline: 03b000ffd330b053efda628bb9a3f8b40db7132a. GUI candidate baseline: 3d55494d6b7287719d07334c747a6e2f2a4d1e49 (draft PR #17). Owner: Linux Operations; Foundation/Mac validates macOS separately.

## Decision

Permit a narrowly scoped, explicitly clicked standalone link to the official SiriusXM web player after this design and its implementation are reviewed. This is a browser handoff, not a native streaming adapter, OAuth integration, authenticated Rust session, or permission to redistribute content. Use only the compile-time HTTPS URL https://player.siriusxm.com/ with no query, fragment, user info, or user-supplied launch arguments. Plain-text provider label, no bundled logo, streams, channel data or account data. Preserve the existing prohibition on browser-based playback inside the native engine, CLI and embedded host. No automatic launch, preflight network requests, browser automation, session interception or background login.

## Primary evidence (reviewed 2026-11-26)

- [Official web player troubleshooting](https://www.siriusxm.com/help/web-player-problems): identifies the official player and directs users to log in in their browser.
- [Official computer requirements](https://listenercare.siriusxm.com/prweb/autoredirect/app/ExternalKM/help/SupportCenter/article/KC-1957/What-are-the-computer-requirements-for-streaming-SiriusXM): lists Windows/macOS and mainstream browsers; does not establish Linux support. Arch playback requires manual verification; no compatibility guarantee.
- [Customer agreement](https://www.siriusxm.com/customer-agreement), last updated June 5, 2025 in the reviewed version: section 9 limits personal use, recording, account sharing and exploitation; subsections 9(j–l) restrict interference, reverse engineering and extraction, including AI use. Do not scrape, extract media URLs, impersonate a device, circumvent access controls or train models on service data.
- [Web terms](https://www.siriusxm.com/sxm-web-terms-of-use): separate website/content and technology restrictions. A plain external link does not establish partner approval, branding permission, or rights to integrate protected playback. Recheck terms before distributing a branded integration.
- [Streaming idle notice](https://www.siriusxm.com/help/streaming-listening): service may require listener confirmation; no automated keepalive.

No public documented subscriber playback API, OAuth callback, or authorized channel-control interface was verified. No credentials or authenticated endpoints were requested or inspected. These observations are engineering scope boundaries, not legal advice or publisher authorization.

## Trust boundary

```text
Standalone RadioPanel  -- typed, one-shot open intent --> standalone host
Standalone host       -- fixed official HTTPS URL ------> OS browser handler
Browser               <--- authentication and media ---> SiriusXM
User                  -- manual return / pause ---------> browser or Rust Radio
```

There is no reverse credential, cookie, callback, browser DOM, or media-data channel into Rust Radio. Browser and provider own subscription, session persistence, login/MFA, playback and sign-out. Rust Radio does not inspect browser files, OS credential stores, process arguments of browsers, network traffic, clipboard, devtools or TLS traffic. No local HTTP listener. Native engine and station catalog never receive the provider URL. Never log credentials, cookie/session values, visited URLs, provider titles or channel data. No account state in preset/config/evidence files.

## RR-112 implementation contract

- Inspect PR #17 and reconcile with draft PR #14 before source integration. Keep receiver backends and embedded defaults intact unless an explicitly approved scope change says otherwise.
- Add a standalone-only `SiriusXM` button to the 480×320 header next to IP Radio and Settings. Tooltips explain it opens an external browser and that browser manages audio. Keyboard activation has the same single action as pointer activation. No click handler on construction, repaint, discovery, restore or source browsing.
- Represent the action as a closed enum such as `ExternalProvider::SiriusXm`, with one compile-time URL mapping. Panel emits an in-memory one-shot intent, host consumes it once; do not add a generic arbitrary-URL executor. Only the standalone executable performs OS launch. Embedded panel emits no external-provider intent by default.
- Require native audio stopped and pending restart canceled before opening; avoid races with pending source validation/discovery. Keep native Play unavailable during the external handoff until an explicit user return/confirmation that browser playback is stopped. This is a local interaction guard, not proof of global audio exclusivity. Do not stop unrelated browser audio or terminate a browser process. Preserve normal native Stop/cancellation.
- On Arch, investigate the system default URL handler (xdg-open with a fixed argument, no shell; handle missing handler/nonzero status without blocking the UI or leaking subprocess output). On other platforms use a reviewed platform handler or leave feature disabled. Evaluate the pinned eframe 0.31 integration before selecting its URL API; do not assume a feature enabled because it is a default upstream feature. No added dependency until reviewed. Test a mock launcher; avoid browser profiles, custom arguments and untrusted URLs.
- On handoff display "External player — sign in / play in browser". Successful dispatch is not authentication, authorization, or playback; failures show a bounded generic message and explicit retry. Return to IP Radio must be manual and cannot imply browser sign-out. No "Connected", "Playing" or volume/mute controls for an unverified external session.
- No process or credential access beyond handing the single fixed URL to the OS. No automatic retry or session resurrection. The browser may continue playback after Rust Radio exits; tell the user to stop it in the browser.

## RR-113 / RR-114 — separate capability gates

Only research ordinary OS media controls for an explicitly selected, uniquely attributable external audio session after provider-policy review. PipeWire may expose application-level rather than tab-level audio. A browser-wide session cannot be assumed to represent SiriusXM; disable controls if attribution is ambiguous. MPRIS capability signals do not prove provider identity or subscription. Do not use private APIs, browser debugging, traffic inspection, recording, PCM capture, re-encoding, rebroadcast, or DRM bypass to obtain audio. Until a supported, isolated and permitted control path is established, remain launch-only; use the browser or OS audio controls. No native decoder/callback claims for external audio.

## Acceptance matrix

| Gate | Observable check | Status |
| --- | --- | --- |
| RR-111-D | Source-backed contract and explicit proposed invariant exception; owner/independent review | IN REVIEW |
| RR-112-A | Mock receives exactly one constant URL for explicit activation; one-shot intent and embedded isolation covered | PASS in local unit tests |
| RR-112-B | Block native active/pending work, poll handler exit without blocking UI, report spawn/nonzero failure, and require manual return without auto-playing | PASS in local tests; physical overlap unverified |
| RR-112-C | No credential fields/files, cookies, browser automation, arbitrary URL or shell; provider URL absent from catalog and native engine | PASS by code review/static inspection |
| RR-112-D | 480×320 render bounds, standalone-only button, no nSDR/USB standalone header buttons; keyboard/pointer and focus-return test | PASS for automated layout, pointer and keyboard activation; OS focus-return NOT RUN |
| RR-112-E | fmt, locked workspace tests, strict Clippy, applicable feature/platform builds | PASS locally on macOS (default and all features); hosted CI and target Arch build NOT RUN |
| RR-112-F | Human subscriber launches official browser, authenticates independently, selects/plays channel, returns to UI; stops in browser | NOT RUN |
| RR-113/114 | Permitted, unambiguous external-session controls; no unrelated media or media extraction | BLOCKED pending platform/policy review |
| Release | Publisher/branding review if distributed, required checks, independent review and human approval | BLOCKED |

Required Rust gates: `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`; run receiver-feature checks where supported. Record exact source SHA and redacted results. Do not capture login screens or account identifiers. Rollback: remove the standalone button and one-shot host dispatch; leave native sources, receiver settings and host API unchanged.

## Agent handoff and evidence

Canonical registry: e07de9c76ce5bf53a665945fa641b036f85ce1aa. Inspected: architecture-design 12c91c8040d13f618a209de24cbbf6e4f89830d1; security-assurance 4ad8219c8c8142a76a8ae0a33f6c39ce3214dc4a; engineering-research f3c8f3c4ae99829d045ad918667c8b125e8d344c; audio-ui-engineering 9245aff3f93768cd31da115d38b47a127f0b5487; audio-systems-engineering 20b5652c462ebc0032e9f282491a14f41adc90d2; test-verification c15b91ca42a279ccf92d55c1e41db11655ee0b4a; independent-code-review e44217c9f0c2492087f8b16da94f3ca80eae4555. Role catalog and resource chain inspected. These procedures informed this design; separate agent execution/independent acceptance BLOCKED-AGENT. The design author is not an independent verifier.

Next owner: Linux Operations implementation after design review. PR #17 remains draft and its previously observed Actions run 45801654813 failed to start, zero jobs. RR-110 runtime acceptance remains open; perform local checks on the actual target. No native launch, authentication, browser profile access, Rust code change, compilation or audio test occurred in this design pass.
