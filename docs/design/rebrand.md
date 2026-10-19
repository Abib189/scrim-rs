# Rebrand: FigCraft → Scrim (decisions)

**Stage A of the rename task, 19 Oct 2026.** This file records what changes name, what keeps it and why. Later steps (A1–A6) follow it, and A6 checks the remaining occurrences against it.

**The principle:** rename everything a person sees; keep the engine's internals stable so merges from upstream FigCraft stay cheap. Scrim is based on FigCraft by the ArtCraft team, and the attribution stays (`NOTICE`, `docs/FORK.md`, About, README). ArtCraft's name, wordmark and logos stay removed (`cargo xtask brand`).

## The search (repo-wide, `git grep`, at caad1bb)

| Pattern | Hits | Files | Mostly |
|---|---|---|---|
| `figcraft` | 5,282 | 841 | crate and module paths (`figcraft_engine::…`, `figcraft-*` package names), the `.figcraft` extension, packaging file names |
| `FigCraft` | 897 | 225 | locale catalogs (12–17 per locale, 16 locales), 69 string literals in shipped Rust, docs prose, packaging metadata, type names (`FileKind::FigCraft`) |
| `FIGCRAFT` | 112 | 52 | environment variables (`FIGCRAFT_DEFAULT_SERVER`, `_NO_SYSTEM_FONTS`, `_CONTROL_PORT`, `_SERVER_KEY`, `_CA_FILE`, `_BUILD_SHA`, test-only sample paths) |
| `ArtCraft` | 39 | 12 | attribution ("based on FigCraft by the ArtCraft team"), the brand check's deny list, `NOTICE`, `docs/FORK.md` |
| `storyteller` (any case) | 49 | 21 | the app id `ai.storyteller.figcraft` (window app id, packaging, icon file names, UTIs), maintainer and contact addresses |
| `storytold` | 39 | 26 | upstream repository URLs, the `craft-libs` and `craft-fonts` build inputs, CI workflows, docs |

## Decisions by group

| # | Group | Where | Decision | Why |
|---|---|---|---|---|
| 1 | **User-visible strings** | 16 locale catalogs (`crates/ui-egui/locales/*.toml`); 69 literals in shipped Rust (window titles, About, home, dialogs, toasts, MCP prompt and tool text, PDF creator metadata, the server's and CLI's help); README and docs prose | **Rename now (A1)** | A person sees them. A test refuses `FigCraft` and `ArtCraft` in catalog strings outside an allow-list of attribution lines. |
| 2 | **App and package identity** | the window app id; `packaging/macos/Info.plist.in` (`CFBundleIdentifier`, `CFBundleName`, the bundle `FigCraft.app`); Linux `.desktop`, metainfo, MIME and Flatpak manifests (all `ai.storyteller.figcraft.*`); `nfpm.yaml` (deb, rpm); the Windows `.wxs` and `.wxl`; FreeBSD; artifact names `figcraft-<version>-<platform>.<ext>`; `apps/figcraft/build.rs` (Windows `ProductName`) | **Rename now (A2)**: bundle identifier `design.scrim.app`, `Scrim.app`, the desktop entry, Flatpak id, package names and installers `scrim-…` | An installed app called FigCraft would be the wrong product. The Cargo packages keep their names (group 5); only the installed identity changes. |
| 3 | **Config, data and cache directories** | `apps/figcraft/src/services.rs` › `config_dir()`: `~/Library/Application Support/FigCraft`, `%APPDATA%\FigCraft`, `$XDG_CONFIG_HOME/FigCraft`. Recovery copies and installed plugins live under it. | **Rename now (A2), with a migration.** The first start of Scrim **copies** (never moves or deletes) the FigCraft directory into a new Scrim one, only when the Scrim one doesn't exist yet. **The new directory is `design.scrim.app`** (A6): `Scrim` is the TypeScript Scrim's Electron data directory on the same machines, which would have mixed both apps' files and skipped the copy. | Settings and recent files carry over; a FigCraft install keeps working beside Scrim. |
| 4 | **The CLI binary and the MCP server name** | `figcraft-cli` (help, version, usage); MCP `serverInfo` `figcraft` / `FigCraft`; the copyable "connect an agent" commands in the app and `docs/mcp.md`, `docs/cli.md` | **Rename now (A3):** the binary people type is `scrim`; `figcraft-cli` stays as a thin alias that prints a one-line deprecation note; MCP announces `scrim`. Tool names and parameters don't change. | What people type and what agents see. Existing MCP configs keep working through the alias until people update them. |
| 4b | **The self-hosted server** (`figcraft-server`, its systemd units, Docker image, install script) | `apps/figcraft-server`, `packaging/server/` | **Rename later** | Operators' configs and units name it. Renaming needs an upgrade path for running servers, which is out of this task's scope. Its user-facing pages get the new name in A1 where they're strings. |
| 5 | **Crate and module names** | `figcraft-*` packages, `figcraft_*` paths, `FigApp`, `FigTree`, `FileKind::FigCraft`; the plugin API's import module `figcraft` (`crates/plugin/src/sandbox.rs`) | **Keep** | Engine internals: renaming them would conflict with every upstream merge. The plugin module name is a public API that existing plugins import. |
| 6 | **The file extension and format identifiers** | `.figcraft`, `format::EXTENSION` / `FORMAT` = `figcraft`, the MIME type `application/x-figcraft`, the document UTI `ai.storyteller.figcraft.document` | **Keep** | Every existing file, including `examples/`, must keep opening, and files must stay exchangeable with FigCraft. The UTI stays the type's name; the Scrim bundle declares it as an imported type, not exported (A2). |
| 7 | **URLs and endpoints** | the repository URL in `Cargo.toml` and the metainfo (`storytold/figcraft`); issue and help links; the default share server (`collab.craftapps.dev`, `FIGCRAFT_DEFAULT_SERVER`); the invite URL scheme `figcraft:`; the build inputs `storytold/craft-libs` and `storytold/craft-fonts` | **Rename now:** repository, issue and help links point at `Abib189/scrim-rs` (A1, A4). **A5:** the default share server becomes empty, with `SCRIM_DEFAULT_SERVER` (the old name still honoured). **Keep:** the build inputs (they are upstream's libraries, used as they are) and the `figcraft:` invite scheme (**rename later**: the server writes those links, so renaming needs both ends). | Nothing should send people, or their files, to upstream's services by default. |
| 8 | **Licence and attribution text** | `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `ATTRIBUTION.md`, `docs/FORK.md`, About | **Keep everything the licences require, and add the Scrim line:** "Scrim is based on FigCraft by the ArtCraft team" in About, README and `docs/FORK.md`. | Required attribution, and credit where it's due. |
| 9 | **Icons and brand assets** | `assets/app-icon/` (FigCraft's chameleon: `figcraft*.png`, `.svg`, `.ico`, `hicolor/…/ai.storyteller.figcraft.*`); `xtask icon`; the window icon (`services.rs`) | **Rename now with a placeholder (A2):** an original placeholder icon drawn in code, as upstream draws its UI icons. **Recorded as a placeholder awaiting the owner's real mark.** FigCraft's icon files stay in the repository (MIT OR Apache-2.0, listed in `ATTRIBUTION.md`), but Scrim no longer ships them. | Scrim must not ship FigCraft's mark as its own, and the real mark is the owner's to make. |
| 10 | **Tests and fixtures** | tests asserting the old strings; fixture files (`examples/*.figcraft`, `docs/scrim-transfer/fixtures/`), test sample paths (`FIGCRAFT_FIG_SAMPLES`, `FIGCRAFT_PDF_SAMPLE`) | **Rename now:** assertions on user-visible text. **Keep:** fixture contents, and test-only environment variables. | Fixtures are data, and test-only variables don't reach users. |
| 11 | **Other environment variables** (`FIGCRAFT_NO_SYSTEM_FONTS`, `_CONTROL_PORT`, `_SERVER_KEY`, `_CA_FILE`, `_BUILD_SHA`, `_EMBED_CJK`) | runtime and build configuration | **Rename later**, honouring both spellings | Scripts and CI set them. Except the share-server default, which A5 changes. |

## What A2 left for later

- **The app binary** keeps its name: `figcraft` on Linux and FreeBSD (the desktop entry's `Exec`, the Flatpak `command`), `figcraft.exe` on Windows. Only the macOS bundle's executable is `Scrim`. **Rename later**, together with the CLI's `scrim` (A3) so the two don't collide.
- **The deb and rpm maintainer address** defaults to a placeholder (`Scrim maintainers <scrim-rs@users.noreply.github.com>`, `FIGCRAFT_MAINTAINER`): the owner sets a real one before publishing.
- **The server's packaging** (`packaging/server/`, group 4b).

## The placeholder icon (A2)

Scrim ships a **placeholder** app icon drawn in code: an original, simple mark, not FigCraft's chameleon and nothing of ArtCraft's. **It awaits the owner's real mark.** When it arrives, it replaces the placeholder in one place (the generator), and the packaged icons regenerate.

## GitHub-facing text (A4; the owner pastes these into the repository settings)

- **Description:** Scrim: a design tool where people and AI agents design together on one canvas. Native Rust; based on FigCraft.
- **Website:** leave empty until Scrim has a site. (Use `https://github.com/Abib189/scrim-rs` if a value is required.)

## A6: verification (19 Oct 2026, at 9d7fdc6)

**CI:** `cargo xtask ci` exit 0 (fmt, clippy, tests, layers, assets, brand, size, wasm). The locale checks are part of it: the i18n tests refuse missing and unused keys, and Spanish, the one complete catalog, has every string these steps added.

**The macOS app** (`packaging/macos/package.sh --arch aarch64`, ad-hoc signed; `verify.sh` passes, with the expected warnings that it isn't notarized):
- `scrim-0.7.0-macos-aarch64.dmg` holds `Scrim.app`: `CFBundleName` and `CFBundleDisplayName` Scrim, `CFBundleIdentifier` `design.scrim.app`, executable `Contents/MacOS/Scrim`, icon `Scrim.icns`. Its version line reads `Scrim 0.7.0 (9d7fdc6…)`.
- Started from the DMG's copy with a temporary home, it opened `examples/ridgeline-app.figcraft` (window title "Ridgeline app – Scrim"; `rebrand/a6/ridgeline-packaged.png`). The Missing fonts notice is expected in a local build without `../craft-fonts`.
- It opened the owner's `.fig` sample, read in place and never copied into the repository: two pages, every frame, and the file's text and colour styles. No screenshot is committed, because the file is the owner's.
- Share ▸ Online with no server shows the A5 prompt (`rebrand/a6/share-online-no-server.png`).

**Migrated settings:** the temporary home held a copy of the owner's real FigCraft directory, plus a stand-in for the TypeScript Scrim's `Scrim` directory. On first start:
- `design.scrim.app/` was made, with the light theme, all 6 recent files and `Recovery/`;
- `publish_server` changed from `wss://collab.craftapps.dev/` to empty, the A5 rule;
- FigCraft's `state.json` stayed byte-identical to the original;
- the `Scrim` directory still held only its own file.

**Verification found and fixed** (commit 9d7fdc6, before this record):
- **The settings directory.** It is now `design.scrim.app`, not `Scrim` (group 3 above). On this machine `Scrim` is the TypeScript Scrim's data directory, so A2's migration would have skipped, and the app would have written its files among the other app's.
- **The repository URL** in `Cargo.toml`.
- **The AppImage's update glob** (`figcraft-*` → `scrim-*`).
- **Two stale lines** in `docs/releasing.md`.

**The agent path, using the new names:**
- **The control channel** (`--control`): `ui.screenshot`, `ui.inspect`, `window.list` (title "… – Scrim"), and `engine.execute` with `view.zoom50` and `view.zoomFit`. `collab.share {via: online}` is refused with the A5 message; `pref.publishServer {}` reports `{"default": "", "url": ""}`.
- **MCP** (the packaged `scrim mcp --attach PORT`): `serverInfo` is `{"name": "scrim", "title": "Scrim", "version": "0.7.0"}`, with 24 tools. `screenshot`, `inspect_ui` and `run_command` (`view.zoom50`) all succeed.
- **The old name:** `figcraft-cli --version` prints the deprecation note, then `scrim 0.7.0`, and exits 0.

### What still says FigCraft, ArtCraft, storyteller or storytold, and why

The search was re-run at 9d7fdc6 (`git grep`):

| Pattern | Hits now (at caad1bb) |
|---|---|
| `figcraft` | 5,165 (5,282) |
| `FigCraft` | 449 (897) |
| `FIGCRAFT` | 114 (112) |
| `ArtCraft` | 53 (39) |
| `storyteller` | 10 (49) |
| `storytold` | 38 (39) |

| Where | What | Why it stays |
|---|---|---|
| Crate and module names, `figcraft_*` paths, `FigApp`, `FileKind::FigCraft`, the thumbnail enum, the plugin import module (most of the 5,165 `figcraft`) | Engine internals | Group 5: renaming would conflict with every upstream merge, and the plugin module is a public API |
| `.figcraft`, `application/x-figcraft`, the UTI `ai.storyteller.figcraft.document` (`Info.plist.in`, `docs/releasing.md`) | The file format | Group 6: every existing file keeps opening, and stays exchangeable with FigCraft. The UTI is imported, not exported |
| The `description` lines of the crates' `Cargo.toml` files (about 20) | Package metadata nobody installs or sees | Group 5, kept with the package names. **Rename later** with them if the owner wants |
| 91 Rust comments | They describe upstream behaviour, the format, or where code came from | Not shipped text |
| `migrate.rs` (`OLD`, `UPSTREAM_SERVER`), and `main.rs`'s log line "couldn't copy FigCraft's settings" | The migration from FigCraft | Names the thing being migrated from |
| About, README, `docs/FORK.md`, `NOTICE`, `ATTRIBUTION.md`, `LICENSE-MIT`, `AGENTS.md`, `CLAUDE.md`, the en and es attribution string, `xtask/src/brand.rs` (all 53 `ArtCraft`) | The attribution, the licences' copyright lines, and the brand check that guards them | Required, or asked for: "Scrim is based on FigCraft by the ArtCraft team" |
| `FIGCRAFT_*` environment variables (114) | Runtime, build and test switches; `FIGCRAFT_DEFAULT_SERVER` is honoured beside `SCRIM_DEFAULT_SERVER` (that is why the count went up by 2) | Group 11: **rename later**, honouring both spellings |
| `figcraft-server`, its units, Docker image and install script; the `storytold/figcraft` download URLs in `docs/server.md` | The self-hosted server | Group 4b: **rename later**. Operators' configurations name it, and this fork publishes no server releases yet |
| `storytold/craft-libs`, `storytold/craft-fonts`, `storytold/craft-common-actions` (Cargo.toml, `Cargo.lock`, workflows, Flatpak manifest) | Upstream's build inputs | Group 7: used as they are. The release workflow can't run for this fork until `craft-common-actions` is replaced (`docs/FORK.md`) |
| `docs/scrim-transfer/` (100), `docs/scrim-ports/` (38), the port checklists | Records of the transfer, written against FigCraft as it was | History: describes what was done, when |
| `docs/online-sharing-terms.md` (and `hello@storyteller.ai` in it) | FigCraft's terms for its own server | Kept unchanged as upstream's, under a note that Scrim runs no such service |
| `assets/app-icon/README.md`, `figcraft*.png`, `hicolor/…/ai.storyteller.figcraft.*` | FigCraft's icon files, still in the repository under their licence | Group 9: not shipped. The README describes both sets |
| `the figcraft:` invite scheme, `contributors/contributors.json`, `docs/contributors.md`, `release-playbook.md`, `file-format-parity.md`, ROADMAP history rows | Upstream's link scheme, credits and history | Group 7 (rename later, needs both ends); credit; history |
| `packaging/windows/sign.ps1` (`figcraft-sign-$PID` temp folder), test fixtures and assertions (`tests.rs` find/replace of the word) | Not user-visible | Test data and a temporary name |
