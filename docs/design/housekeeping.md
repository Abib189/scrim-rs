# Housekeeping: texts to paste, and what still needs you (R6, 22 Oct 2026)

Nothing here was pushed to another repository, and no GitHub setting was changed: these are the texts, ready for you to paste.

## (a) The scrim-rs repository's description

On github.com/Abib189/scrim-rs, the gear beside **About**:

- **Description** (from `docs/design/rebrand.md` › GitHub-facing text):

  ```
  Scrim: a design tool where people and AI agents design together on one canvas. Native Rust; based on FigCraft.
  ```

- **Website:** leave empty until Scrim has a site (or `https://github.com/Abib189/scrim-rs` if a value is required).

## (b) `ARCHIVED.md` for the old TypeScript Scrim (github.com/Abib189/scrim)

Add this file at the repository's root, then archive the repository (Settings ▸ General ▸ Danger Zone ▸ Archive this repository). Archiving makes it read-only; it isn't deleted.

```markdown
# This repository is archived

Scrim continues as **scrim-rs**, a native Rust app: https://github.com/Abib189/scrim-rs

This TypeScript editor (React, Electron and an MCP server) was Scrim's first version. It stays here, read-only, for its history. It is no longer maintained: no fixes, no releases, no answers to issues.

- **What carried over:** Scrim's ideas, rules, test cases and fixtures went to scrim-rs as specifications (`docs/scrim-transfer/` there), never as code. scrim-rs is a fork of FigCraft by the ArtCraft team.
- **Your documents:** scrim-rs doesn't open this editor's `.scrim` files. Keep a copy of this version if you need to open them.
- **Where to go now:** issues and ideas belong in scrim-rs.
```

## (c) What still needs you

1. **The real app icon.**
   - **Where it goes:** `assets/app-icon/scrim/` is the one place every package reads its icon from:
     - `cargo xtask bundle-macos` and the release DMG (`scrim-<size>.png`, made into `.icns`);
     - Linux's hicolor theme;
     - Windows' `scrim.ico`.
   - **How to put it there:** today those files are written by `cargo xtask icon` from the placeholder drawn in `xtask/src/icon.rs` (`svg()`). Either draw the new mark there and run `cargo xtask icon`, or replace the files directly at the same names and sizes.
   - **Also:** update its row in `ATTRIBUTION.md` (your own artwork, its licence). The small mark inside the app (`Icon::Logo` in `crates/ui-egui/src/icons.rs`) is drawn in code, so it changes separately if the new mark looks different.
2. **The maintainer address for the Linux packages.** The deb and rpm packages name `Scrim maintainers <scrim-rs@users.noreply.github.com>` until `FIGCRAFT_MAINTAINER` is set (`docs/releasing.md`). Choose the real address before publishing them.
3. **Notarisation.** Today the Mac app is signed ad hoc and not notarised, so on another Mac Gatekeeper asks once (`docs/design/macos-bundle.md` › Opening it).
   - **To notarise:** a paid Apple Developer Program membership gives a Developer ID certificate. The release workflow already reads `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` (`packaging/macos/package.sh`).
   - **The choice:** notarise, or keep shipping ad-hoc builds with the "Open Anyway" step documented.

**Also from tonight's report, for you to decide:**
- whether to rewrite pushed history to drop the five captures and the relay log that showed personal data (`docs/design/pii-scan.md`; a force push);
- whether agent runs may get `USER` in their environment, if Claude Code still says "Not logged in" when you try Describe a change (`docs/scrim-ports/chat/README.md`).
