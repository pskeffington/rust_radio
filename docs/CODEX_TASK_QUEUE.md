# Codex execution queue — Rust Radio (2026-11-27)

Status meanings: READY = scoped, not started; BLOCKED = missing prerequisite; IN_REVIEW = PR exists; DONE = implementation and evidence verified.

| ID | Priority | Owner lane | Status | Deliverable | Acceptance |
| --- | --- | --- | --- | --- | --- |
| RR-109 | P0 | Research | BLOCKED: app distribution rights | Rights review for the active 20-station listener catalog | Keep `redistribution: not_authorized`; publisher player guidance does not grant bundle rights |
| RR-120 | P0 | Scope | DONE | Keep Rust Radio streaming-only; receiver code and hardware acceptance live in separate repositories | GUI input discovery, segment state, startup scan and receiver status wording removed; fresh build and relaunched UI show only IP radio controls |
| RR-127 | P0 | Catalog/Transport | IN_REVIEW | Restore a 20-station curated IP channel list, preserve history compatibility, and pin each redirect/playlist pipeline to its publisher-approved station targets | Exact 20 ordered IDs in runtime/JSON; 20/20 silent decoded-audio sweep; core, CLI mirror, packaging preflight, and history migration checks pass; release rights remain blocked |
| RR-103 | P1 | Audio | BLOCKED: physical device | Mac IP-stream output loss, device-switch and stop-recovery evidence | Current-source audible output, device selection and reconnect evidence remain operator gates |
| RR-107 | P1 | Integration | BLOCKED: Pi 5 target | Pi runtime and Rust-Boy embedded host signoff | Physical screen, native output, CPU/memory/thermal/power still required |
| RR-108 | P1 | Validation | BLOCKED: Actions runner startup | Hosted exact-SHA CI evidence | Prior Actions startup failure had zero jobs; confirm a successful run on the current exact head |
| RR-118 | P1 | Packaging | DONE: local unsigned development app | Build, install and relaunch the current macOS app | Fresh release build installed and signature-verified; relaunch visually confirms IP-only UI and curated list; notarization and public distribution remain separate |
