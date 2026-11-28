# Curated publisher rights audit

Audit date: 2026-11-27
Repository revision reviewed: `2253fe7b7a98a5a5b1359affd441546ade8a3c1c`  
Scope: current curated station names, publisher references and stream endpoints
in `crates/rust-radio-core/src/directory.rs` and
`catalog/curated-stations.json`. This is a source review, not legal advice or a
license grant.

## Release decision

**BLOCKED: do not ship the current curated directory in a Rust Radio application
release.** The machine-readable catalog already sets
`policy.redistribution` to `not_authorized`; this audit adds publisher-specific
evidence supporting that fail-closed state. A publisher's public stream URL or
support for named listening platforms is not, by itself, permission to include
its stream, station metadata, or branding in this standalone client.

| Publisher | Current IDs | First-party evidence reviewed | Evidence status for Rust Radio bundle |
| --- | --- | --- | --- |
| SomaFM | `groovesalad`, `secretagent`, `lush`, `dronezone`, `deepspaceone`, `spacestation`, `beatblender`, `defcon`, `u80s`, `indiepop`, `folkfwd`, `metal`, `bootliquor`, `sonicuniverse`, `bossa`, `reggae`, `thistle`, `fluid`, `cliqhop`, `thetrip`, `poptron`, `leftcoast70s`, `illstreet` (23) | [Terms of Service](https://somafm.com/legal/tos.html), updated 2026-09-04; [station directory](https://somafm.com/listen/) and per-station pages linked from the catalog | **Blocked.** The terms state SomaFM cannot grant permission for new third-party clients, including noncommercial apps, and prohibit app embedding without prior written permission. Do not ship these 23 curated entries in a Rust Radio app unless a documented applicable rights basis is established. The terms describe a small number of longstanding exceptions but say no new ones are being added. |
| GBH | `wcrb` (1) | [Direct stream and listening options](https://www.wgbh.org/help/how-to-access-live-radio-streams); [Terms of Use](https://www.wgbh.org/privacy/terms-of-use) | **Unresolved for bundle.** GBH documents direct URLs and listening through Apple Music and iHeart. Its site terms limit site/content use to personal, noncommercial use and restrict copying, distribution and transmission. Those sources do not grant this Rust Radio package permission to bundle the station entry or stream. |
| Skylab Radio | `skylab` (1) | [Listen page](https://www.skylabradio.com/listen); [competition terms and site copyright notice](https://www.skylabradio.com/terms-conditions) | **Unresolved for bundle.** Skylab publishes a high-quality stream link and lists third-party listening services. The cited terms govern competitions; their copyright notice reserves reproduction of website material absent written consent. No stream or app reuse terms were found in the reviewed pages, and none grants Rust Radio permission to bundle the stream or station metadata. |

## Required next actions

1. Keep the curated release gate closed. The current 20-entry catalog is
   development data, not a cleared release manifest.
2. Exclude all 23 SomaFM entries from any Rust Radio application release unless
   an applicable written authorization or other documented rights basis is
   available. Their published policy expressly rejects new third-party clients.
3. Obtain and archive scope-specific written permission for GBH and Skylab before
   including their station data in a distributable Rust Radio app. If permission
   is not obtained, exclude those entries from the release manifest as well.
4. Preserve source URLs and station identities for auditability; never replace
   a blocked station silently. Re-run this review if terms, catalog membership,
   bundle behavior or release plans change.

## Current 20-station listener catalog

The current development catalog removes the prior SomaFM set and the five
additional entries `kusc`, `colorado-sound`, `kuaf`, `kuaf-classical` and
`kuaf-jazz`; it restores the previously selected 19 and adds `kunc`. Selection
is product curation, not a claim that removed feeds failed. Publisher guidance
supports listener use in third-party players, but this audit still does not
establish permission to bundle each station or stream. The release policy
therefore remains `not_authorized` until station-specific bundle rights are
recorded.

## NPR candidate

NPR is not present in the current curated catalog. The historical NPR research
record in `PROGRESS.md` remains a separate candidate gate: it found no approved
continuous national NPR News endpoint or permission applicable to this product.
Do not represent NPR as part of the current bundled inventory.

## Evidence limits

- Publisher pages document what each publisher says about its own streams and
  listening routes. This audit does not decide the legal treatment of any
  particular Rust Radio distribution or listener use.
- No publisher was contacted, and no written permission was received or inferred.
- Stream decoding, public accessibility, or inclusion in another listening
  service does not resolve this release gate.
