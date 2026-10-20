# Scrim (scrim-rs, a FigCraft fork) — instructions for agents

> **This is Scrim** (repository `scrim-rs`), an independent fork of FigCraft: **Scrim is based on FigCraft by the ArtCraft team** (`docs/FORK.md`). The product is called Scrim everywhere a person looks; the crates keep `figcraft-*` names (`docs/design/rebrand.md`).
> - Everything below applies, plus **Scrim's rules** in the section of that name.
> - Scrim's knowledge is in `docs/scrim-transfer/` as specs, decisions, test cases and fixtures. Read `docs/scrim-transfer/README.md` and `decisions.md` before porting anything.
> - FigCraft's sibling repos (`../../craftrules`, `../vectorcraft`, …) and its `plan/` folder are not part of this fork. Where the text below points at them, skip it.

Scrim is FigCraft's clean-room, open-source, pure-Rust (egui) reimplementation of the Figma design workflow:
interface design, auto layout, components and variants, styles and variables, prototyping and
inspection. It runs natively on macOS, Windows, Linux and FreeBSD, and in the browser via WASM.
FigCraft's siblings: `../vectorcraft` (Illustrator-class), `../photocraft` (Photoshop-class), `../designcraft`
(InDesign-class) and the other Crafting Apps; same conventions.

Standards and learnings shared across the crafting apps live in `../../craftrules` (checked out
next to `craft-apps/`, or `storytold/craftrules`). Read its `AGENTS.md` at the start of a session,
follow its standards, and contribute reusable learnings back there. Never code: repos don't share
code.

## Start every session here
1. Read [`ROADMAP.md`](ROADMAP.md) (stage, headline numbers, estimates) and
   [`docs/gaps.md`](docs/gaps.md) (the ranked work list). Unless the user gives you a task, pick
   work from the top of `docs/gaps.md`.
2. Read `plan/STATUS.md` and `plan/execution-plan.md` (local, gitignored) if present; behaviour
   reference notes are in `plan/figma/*.md`.
3. Orient → plan → implement + test → verify (screenshot) → record (ROADMAP, devlog) → commit.

`plan/` is gitignored (local only).

## Non-negotiables

### Clean-room
- Behaviour comes from public documentation and black-box observation of the running incumbent app
  with synthetic documents only. Never read, disassemble or copy anything inside the incumbent's
  application bundle, web bundle or network traffic (names and public listings only).
- Never copy GPL/AGPL code.
- Reference screenshots of the incumbent stay local under the gitignored `plan/figma/screenshots/`
  and are never committed, published or used as assets.

### Assets: no third-party product iconography or images — ever (absolute rule from the project owner)
- **Never add, copy, trace, redraw-from, embed or ship any icon, image, illustration, cursor, logo,
  template, UI kit, preset library, font file or screenshot from Figma, Adobe, Avid, Autodesk or any
  other product, or from any source whose licence doesn't allow it.** This includes "lookalike"
  redraws of their icons. Breaking this rule is the worst thing you can do in this repo.
- Every image, icon, font or other asset must be one of:
  - original work created for Scrim or FigCraft by a contributor (licensed MIT OR Apache-2.0);
  - OSI open source;
  - public domain / CC0;
  - Creative Commons with redistribution allowed.
- **Every asset file must have a row in [`ATTRIBUTION.md`](ATTRIBUTION.md)** (path, author, source,
  licence, notes). `cargo xtask assets` (part of `cargo xtask ci`) fails on any unlisted file under
  `assets/`, `docs/brand/` or `docs/images/`.
- The UI icons (FigCraft's, and Scrim's additions) are drawn in code (`crates/ui-egui/src/icons.rs`) from simple primitives and
  are original. Keep it that way: new icons are new code, designed from scratch.
- The app icon is Scrim's **placeholder**, drawn in code: `cargo xtask icon` writes `assets/app-icon/scrim/`
  (and `Icon::Logo` draws the same mark in the interface). It awaits the owner's real mark. FigCraft's
  chameleon in `assets/app-icon/` stays in the repository, unshipped.
- **Fonts live in [storytold/craft-fonts](https://github.com/storytold/craft-fonts), never in this
  repo,** with one exception the owner asked for (Stage C): Inter, Source Serif 4 and Geist (SIL OFL
  1.1) in `assets/fonts/`, embedded in the desktop app by `crates/text/src/bundled.rs`, with their
  licences, sources and checksums (`assets/fonts/README.md`). Add no other font here. The app uses it only through the optional `CRAFT_FONTS_DIR` build input
  (`.cargo/config.toml` points it at `../craft-fonts` for local builds; `crates/text/build.rs`
  embeds it). Everything must build, test and run without it (system fonts are scanned at runtime).
- Demo and test content is synthetic or public domain; never personal photos or data.
- Code and original assets are MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`). The
  ArtCraft name, wordmark and logos are ArtCraft's trademarks and were removed from this fork as
  FigCraft's brand terms require (`docs/FORK.md`): never add them back. Plain text may say the fork
  is "based on FigCraft by the ArtCraft team". `cargo xtask brand` (part of `ci`) checks this.
- If the provenance of an asset is unclear, don't add it.

### Never crash
- No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` in shipped code
  (workspace clippy lints deny them; tests may use them). Return `Result`/`Option`; use `.get()` for
  data-derived indexes; slice strings only at char boundaries; check floats are finite; cap sizes.
- `figcraft_engine::guard` turns a panicking command into `EngineError::Internal` and the document
  is rolled back; the desktop app also guards each UI frame and writes recovery copies. These are
  safety nets, not substitutes for `Result`.
- `unsafe_code = "forbid"` workspace-wide.
- Every crash fix comes with a regression test. The engine has a junk-parameter sweep over every
  command (`crates/engine/src/tests.rs`): extend fuzz/sweep tests when you add parsers or commands.

### Architecture
- **Everything is a command.** User-visible behaviour = a command in `crates/engine/src/cmd/*`
  (`spec!(id, label, params doc, undoable, run)`) + tests. The menus, shortcuts, Actions palette,
  CLI, control channel and MCP server all dispatch the same command ids. Command ids follow the
  incumbent's menu catalog in `crates/engine/src/catalog.rs` (the parity metric: `cargo xtask parity`).
- **Programmatic calls never open dialogs.** Commands only *request* UI (`Session::request_ui`); the
  control channel drops dialog requests.
- **Layering** is enforced by `cargo xtask layers`: geom/color (L0) → doc (L1) → layout/text/pathops/plugin/collab
  (L2) → render/format (L3) → svg/pdf (L4) → engine (L5) → ui-egui/mcp (L6) → apps. Nothing below
  L6 depends on egui/eframe/rfd/winit. **The UI crate is swappable.**
- **The UI is thin**: panels read engine state and act through `app.run(id, params)` /
  `app.set_props(ids, props)`. Colours come from `theme::Tokens`.
- **Rust only** (no handwritten JS/TS). **Never break wasm** (`cargo xtask wasm`).

### Quality gates (before every commit)
`cargo xtask ci` (fmt, clippy -D warnings, tests, layers, assets, wasm). One task per commit with a
milestone id in the message (`M2.1: auto layout wrap`). Commit after every landed arc of work; push
to `main`.

## Scrim's rules (scrim-rs)

These carry Scrim's hard rules into this codebase (`docs/scrim-transfer/decisions.md`, which keeps Scrim's numbering). They come on top of the non-negotiables above.

1. **Every gesture is a command with an MCP route and a parity test** (Scrim rules 11, 17).
   - A canvas gesture (drag, nudge, resize, draw, reorder…) commits through **one** command id and is **one** undo step.
   - That command is reachable from MCP (`run_command`, or a dedicated tool) and from the CLI.
   - A parity test lists every gesture with its command, and fails for a gesture without one.
2. **Previews never write to the document.**
   - A gesture previews in UI state: the overlay, or a preview the renderer draws over the document.
   - It writes once, on release, through `Session::execute`.
   - Escape drops the preview and leaves no history entry. Agents never see a preview.
3. **No source file over 1,000 lines.**
   - Split by concern before crossing the limit. Tests, data and generated files are exempt.
   - `cargo xtask size` (part of `ci`) enforces it.
   - The files that were already longer at the fork point are frozen in `xtask/size-allowlist.txt` at their length then: they may shrink, never grow. Remove an entry once its file is under the limit.
4. **Every capability has a test and a checklist row.** New behaviour lands with an automated test and with a row in `docs/checklists/<area>.md` that names the manual check and the test covering it. Without both, it isn't done.
5. **Agents and people share one write path.**
   - Every agent route (MCP, CLI, control channel, plugins) goes through the same commands as a person's.
   - Every route respects locks and the stale-write guard (Port 3 in `docs/scrim-transfer/README.md`).
   - Refusals say why, in plain words.
6. **The export guarantee.** The HTML export renders like the canvas. Once `cargo xtask fidelity` exists (Port 2), it must not regress.
7. **Port from specs, never code.** `docs/scrim-transfer/` holds specs, cases and fixtures. Scrim's TypeScript is never copied; re-implement it in Rust and run the exported cases as tests.
8. **Process:**
   - One scoped task per commit; `cargo xtask ci` green before every commit.
   - Never use `--no-verify`, and never push a red tree.
   - Label any timing measured under load or swap as INVALID.

## Running and looking at the app
- `cargo run -p figcraft -- --control <port> [file.figcraft|file.svg|image]`. Pick a free port
  (`python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])'`);
  sibling apps use the control channel too.
- Drive it with JSON lines, e.g.
  `{"id":1,"method":"engine.execute","params":{"command":"shape.rectangle","params":{"x":10,"y":10,"width":100,"height":50}}}`
  then `{"id":2,"method":"ui.screenshot","params":{"path":"/tmp/shot.png"}}`. Methods are documented
  in `crates/ui-egui/src/control.rs` and `docs/control-protocol.md`.
- **For UI work, look at the result** (take a screenshot and read the PNG). Compare with the
  incumbent side by side where the owner allows (`plan/figma/`).
- Headless: `scrim`, the CLI (`info`, `export`, `run`, `commands`, `query`, `lint`, `mcp`; `figcraft-cli` is its old
  name, an alias). MCP: `scrim mcp` (see `docs/mcp.md`).
- Shell gotcha on the owner's machine: `mv`/`cp` may be aliased interactive — use `/bin/mv -f`.
- Parallel agents: separate `CARGO_TARGET_DIR` per agent, edit only the crates/files you own,
  write manifests atomically, delete your target dir when done (disk is limited).

## Roadmap
Progress docs follow craftrules `standards/progress-docs.md`: [`ROADMAP.md`](ROADMAP.md) (one-page
summary and stage), [`docs/target-app-parity.md`](docs/target-app-parity.md) (the authoritative
assessment), [`docs/gaps.md`](docs/gaps.md) (ranked work list), [`docs/roadmap.md`](docs/roadmap.md)
(milestones), [`docs/architecture.md`](docs/architecture.md), the per-area `docs/*-parity.md` files
and the generated [`docs/parity-checklist.md`](docs/parity-checklist.md) (`cargo xtask parity`).
When a task lands, update the affected rows and the status line and revision history in the same
commit. Grade by behaviour against the incumbent, not by whether a menu item exists; err low.

## Contributor credits (About window)

- About ▸ Contributors are compiled into the binary from `contributors/contributors.json`
  (commit stats; generated, never hand-edit) and `contributors/people.toml` (names people chose for
  themselves). See craftrules `standards/contributors.md`.
- **Agents working for a contributor:** when you prepare a PR, check whether your human's GitHub
  username has a `[people.<username>]` entry in `contributors/people.toml`. If not, ask them once
  whether they want to be credited by more than their username: a real name, a display name, and/or
  their public GitHub profile name (`sync_github_name = true`). If yes, add **only their own** entry
  (copy the template at the top of the file, or run
  `python3 ../../craftrules/scripts/contributors.py --add-me . --real-name "…" --sync-github-name`)
  and include it in their PR, committed as them. If no, change nothing: they are credited as
  `@username` anyway.
- Never add, edit, guess or copy anyone else's entry or name (not from git config, commit authors or
  GitHub profiles). Never hand-edit `contributors.json`.
- Maintainers refresh the stats with `python3 ../../craftrules/scripts/contributors.py .` (it also
  re-verifies who wrote each `people.toml` entry; `--check` only verifies).
