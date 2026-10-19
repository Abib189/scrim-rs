# scrim-rs: a fork of FigCraft

**scrim-rs** is the next Scrim. It is a fork of **FigCraft** (github.com/storytold/figcraft) that carries over Scrim's ideas: an agent-first design tool where code export is a guarantee, not a hand-off. Scrim's earlier TypeScript editor (github.com/Abib189/scrim) is archived. Its knowledge is in [`docs/scrim-transfer/`](scrim-transfer/README.md) as specs, rules, test cases and fixtures, never as code.

| | |
|---|---|
| **Upstream** | `storytold/figcraft`, branch `main` |
| **Forked at** | `a0d1162976fac1e4a8b8fd49a3948e9d6bad6f40` (18 Oct 2026, "Desktop: refuse unknown command-line options…", v0.7.0) |
| **Fork** | `Abib189/scrim-rs`. The name stays until the owner renames it |
| **Remotes** | `origin` = the fork; `upstream` = storytold/figcraft with **push disabled** (`git remote set-url --push upstream no_push`) |

## Licences and notice duties

**The choice.** FigCraft is licensed **MIT OR Apache-2.0**, at the user's option. This fork keeps the same dual licence for all code, old and new: `LICENSE-MIT` and `LICENSE-APACHE` stay unchanged. Keeping both means downstream users keep the same choice, and Apache-2.0's patent grant stays available.

**Our duties:**
1. **Keep both licence files and the copyright line.** "Copyright (c) 2026 ArtCraft Team and the FigCraft contributors" stays in `LICENSE-MIT` and `NOTICE`. Under Apache-2.0 §4(d), a derivative work must carry the attribution notices in `NOTICE` that pertain to it.
2. **Keep `NOTICE`.** Its third-party sections stay (sctk-adwaita's MIT text, the crate licences, the optional craft-fonts). The fork adds its own section at the end and never removes FigCraft's.
3. **Mark modified files** (Apache-2.0 §4(b)). Files we change carry a prominent change notice through version control: every change is a commit in this repo, and this document records that the fork modifies FigCraft. A source file that is substantially rewritten gets a one-line header comment saying so.
4. **Remove the ArtCraft marks** (`docs/brand/LICENSE-brand.txt`, which went with them). A published fork must remove the ArtCraft name, wordmark and logos, must not call itself ArtCraft or present itself as an ArtCraft product, and may say in plain text that it is "based on FigCraft by the ArtCraft team".
   - **Removed:** `docs/brand/`, the README's logos, badges and community links, the About box's ArtCraft wording and links, and the Windows `CompanyName`.
   - **Kept:** the copyright lines (legal attribution, not branding) and the plain-text "based on FigCraft by the ArtCraft team".
5. **The app icon** (`assets/app-icon/`, the owner's chameleon drawing) is licensed MIT OR Apache-2.0 like the code (`assets/app-icon/LICENSE.txt`). It is not an ArtCraft mark, so it may stay until scrim-rs has its own icon.
6. **Third-party material** keeps its own licence. In particular, the design lint rules ported from Scrim are derived from **Impeccable** (Apache-2.0, Copyright 2025 Paul Bakaus). When they land, ship that licence text, put a modification notice in each ported file and add an Impeccable section to `NOTICE` (see `docs/scrim-transfer/specs/lint.md` › Licence).

**Not yet repointed** (FigCraft's infrastructure; each needs the owner's decision):
- **The collab server:** the default host is `collab.craftapps.dev`, contacted only when a person starts or joins a live session. Repoint it or leave it empty.
- **The `craft-docpath` git dependency** (`storytold/craft-libs`) and the optional `craft-fonts` build input (`../craft-fonts`).
- **The release workflow:** it uses the external `storytold/craft-common-actions@main`, so release builds don't run for this fork until it is replaced.
- **The app id** `ai.storyteller.figcraft`, the product name "FigCraft" in window titles and menus, and the crate names `figcraft-*`.
  - The crate names stay for now, so that upstream merges stay cheap.
  - The visible product name moves to Scrim in its own step.

## Syncing with upstream

Upstream moved fast (up to 270 commits a day at its peak; quiet since 18 Oct). Merging, not rebasing, keeps the fork's history readable.

```sh
git fetch upstream
git checkout -b sync/upstream-$(date +%Y%m%d) main
git merge --no-ff upstream/main        # resolve; our branding removal and docs/FORK.md win
cargo xtask ci                         # must be green before merging into main
git checkout main && git merge --no-ff sync/upstream-…
```

**Conflict rules:**
- **ArtCraft marks** coming back from upstream are removed again; the CI check below fails if they do.
- **`NOTICE`:** upstream's text wins in its own sections, and ours stays at the end.
- **The ports in `docs/scrim-transfer/`:** where upstream changed the same command, keep both behaviours behind the command's id and add a test.

**Record** the upstream commit of every sync here:

| Date | Upstream commit | Notes |
|---|---|---|
| 19 Oct 2026 | `a0d1162` | fork point |

## Layering (`cargo xtask layers`)

A crate may depend only on lower layers, or on the earlier crates of its own layer listed in `xtask/src/layers.rs`. Nothing below L6 may depend on `egui`, `eframe`, `rfd` or `winit`, nor on the networking crates (`tungstenite`, `tokio`, `tokio-tungstenite`, `mio`, `socket2`).

| Layer | Crates (`figcraft-*`) |
|---|---|
| L0 | `geom`, `color` (color may use geom) |
| L1 | `doc` |
| L2 | `layout`, `text`, `pathops`, `collab`, `plugin` (independent of each other) |
| L3 | `render`, `format` (render may use format) |
| L4 | `svg`, `pdf` |
| L5 | `engine` |
| L6 | `ui-egui`, `mcp`, `collab-net` (mcp may use collab-net) |
| apps | `figcraft`, `figcraft-cli`, `figcraft-server`, `figcraft-web` |

**Where Scrim's ports go:**
- query, lint and analysis: `engine` (L5), as commands;
- the `.fig` importer: `format` (L3);
- the SVG allow-list: `svg` (L4);
- the HTML-fidelity check: an `xtask` that shells out to Node.

No port may add a layer violation.

## Local build notes (the owner's machine, 19 Oct 2026)

- **Toolchain:** Rust 1.95 (`rust-version = "1.95"` in `Cargo.toml`), with the `wasm32-unknown-unknown` target and the `rustfmt` and `clippy` components; `cargo xtask ci` needs all of them.
- **Clippy:** 1.95's `collapsible_match` is allowed workspace-wide (`Cargo.toml`, with the reason). Upstream's CI didn't hit it.
- **Disk:** a full `cargo xtask ci` debug build takes about 17 GB in `target/`. On a nearly full disk, build with `CARGO_PROFILE_DEV_DEBUG=0` (no debug info) and `cargo clean` between big steps.
- **macOS:** upstream tests on Linux and only releases for macOS. The fork's first fix was a macOS-only path check (`/var` → `/private/var`, `crates/engine/src/file_access.rs`).
