# RR-118: stable macOS icon and presentation

## Task contract

- User request (2026-11-24): fix the icon changing when Rust Radio is engaged on macOS and clean up presentation.
- Source baseline: `cf40b5d05b9dbc5a459eb46d038f18a6b5e84f63` (`codex/rr118-macos-launch`). Work branch: `codex/macos-icon-identity`.
- Scope: standalone GUI launch options and title, targeted configuration regression, operator and progress documentation. Preserve the existing SVG/ICNS, bundle ID, installer, Dock tile, playback, and embedding APIs.
- Acceptance: the packaged app retains its native icon through launch/focus/Play/Stop; window and application-menu title are `Rust Radio`; 480×320 controls and no-autoplay remain intact. Keep unbundled `cargo run` distinct from a packaged-app icon acceptance.
- Original checkout has an unrelated audio-output edit; it is excluded from this isolated worktree and change set.

## Design and provenance

Registry: `pskeffington/agent_skills` at `42cf3196122feffa8658239fe7947c033bd39913`. Skills invoked from `.agents/skills/`: `architecture-design` (independent architecture agent), `code-implementation` (primary agent), `test-verification` (independent verifier), and `independent-code-review` (independent reviewer). Verification/review outcomes below must reflect completed invocations, not planned work.

Architecture agent `icon_architecture` confirmed in eframe 0.31.1 that `EpiIntegration::new` substitutes the framework icon for a missing viewport icon, and `AppTitleIconSetter` calls macOS `setApplicationIconImage`. The app currently leaves that option missing. The existing bundle correctly declares `RustRadio.icns` and `com.skeffington.rustradio`.

Use the documented `IconData::default()` sentinel on macOS: eframe interprets it as no runtime override, retaining the bundle's multi-resolution artwork. This avoids an extra raster/dependency and preserves the existing icon at all native sizes. Set one friendly app/window title. Configure these only in the standalone binary; playback and `RadioPanel` must not change the host application's identity.

## Evidence

- **PASS — implementation:** only standalone launch options changed. Tested `main.rs` SHA-256: `2ec74573fb4b98ae13410ed23defce0b453b4e022974947c535cc895170f8d5e`.
- **PASS — independent verification:** agent `icon_verification` reran the binary tests: 2 passed; format, plist lint, shell syntax and ICNS container checks passed. It confirmed no panel, playback, transport, core, SDR, asset, script or lockfile change.
- **PASS — independent review:** agent `icon_independent_review` reported no blocking findings after checking the eframe sentinel, macOS-only scope, title and package boundaries.
- **PASS — required local gates:** `cargo fmt --all -- --check`; `CARGO_TARGET_DIR=/private/tmp/rust-radio-target cargo test --workspace --locked` (176 passed, 1 hardware test ignored); default strict Clippy; all-feature workspace tests (232 passed, 2 manual/live tests ignored); all-feature strict Clippy and check. Tests that bind local sockets ran with the required local-network permission.
- **PASS — package:** `CARGO_TARGET_DIR=/private/tmp/rust-radio-target bash scripts/package_macos_app.sh app`; plist lint; source/executable and source/packaged-icon hashes match. Output: `/private/tmp/rust-radio-target/release/Rust Radio.app`.
- **PASS — artwork unchanged:** `RustRadio.icns` SHA-256 `917c2fca7340ee9c04730912b3dbc8768bb20bea8351e025eb7ddf539930ab00`.
- **BLOCKED — native visual acceptance:** both installed-app and Dock accessibility inspection timed out (`-10005: timeoutReached`). No visual launch/focus/Play/Stop pass is claimed.
- **PENDING — installation/relaunch:** the existing guarded installer reported `Quit Rust Radio before updating it.` The operator was asked to close the current session. The installed app and persistent Dock registration have not been changed by this pass.
- **NOT RUN — hosted CI, signing/notarization, physical audio/Pi/receiver acceptance.** Existing release gates remain open. No signing, new permissions, Dock restart, or Dock tile rewrite is required by this fix.
- **NOT RUN — public registry feedback:** no external issue message was sent; implementation evidence is retained in this repository and the review PR.
