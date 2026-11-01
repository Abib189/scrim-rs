# Boards in upstream FigCraft: what they are, and what a port would cost

This note scopes upstream FigCraft's boards, its whiteboard file kind (Home card: "Sticky notes, shapes and connectors"), before we decide whether Scrim should have them. It is read-only: nothing in Scrim's source changed. It starts from [`upstream-sync.md`](upstream-sync.md) and its commit list, and the evidence is in [`upstream-board-scoping/`](upstream-board-scoping/).

**In short:**
- **Boards landed upstream on 24 Oct 2026**, six days after our fork point, so Scrim never had them. At the fork, FigCraft's own scope said FigJam was out of scope.
- **Size.** They took 36 upstream commits in 8 days:
  - 28 build or fix boards, adding about 13,900 lines: 7,200 of source, 4,200 of tests, 2,000 of translations and 500 of docs;
  - 8 only move board code.
  - Upstream rates its own boards at about 47% of FigJam's breadth.
- **Design.** A board is an ordinary document with a `kind` field. Its objects are ordinary layers carrying a small `board` field. The file format version doesn't change and there are no new dependencies. Upstream is MIT OR Apache-2.0 throughout.
- **Today, Scrim strips board files.** It opens a board saved by upstream without complaint, but treats it as a design file, and the next save drops the board marks. Checked on our tree.
- **A full port is large: about 15 to 21 working days for parity with upstream.**
  - It touches 9 of our frozen files.
  - The live-session wire format needs care.
  - Two gestures need reworking to follow Scrim's rules.
- **Recommendation: skip boards for now.** If they become a goal, port them in stages rather than rebuild. In the meantime, a half-day notice when Scrim opens a board file would stop the silent stripping.

## Contents

1. [When boards landed, and why Scrim doesn't have them](#1-when-boards-landed-and-why-scrim-doesnt-have-them)
2. [Every upstream board commit](#2-every-upstream-board-commit)
3. [A file kind, not a new format](#3-a-file-kind-not-a-new-format)
4. [What a port touches: crates, layers, egui, the renderer, frozen files](#4-what-a-port-touches)
5. [Licence and dependencies](#5-licence-and-dependencies)
6. [Cost, and a staged plan](#6-cost-and-a-staged-plan)
7. [Audit fixes that touch board code](#7-audit-fixes-that-touch-board-code)
8. [Conflicts with Scrim's rules](#8-conflicts-with-scrims-rules)
9. [Recommendation](#9-recommendation)
10. [What could not be determined](#10-what-could-not-be-determined)
11. [Evidence files](#11-evidence-files)

## 1. When boards landed, and why Scrim doesn't have them

| | Commit | Date | |
|---|---|---|---|
| Our fork point | `a0d1162` | 18 Oct 2026 | v0.7.0 |
| **Boards first land** | **`8167f13a`** | **24 Oct 2026** | M12: Boards: a collaborative whiteboard file kind (#1229), by BFlat |

- **Timing.** `a0d1162` is an ancestor of `8167f13a`, so boards came **after** the fork.
- **No earlier board code.** At the fork, upstream's tree had no board file, no `BoardItem` and no `FileKind`.
  - The only "connector" code at the fork was the `.fig` importer's, which turns FigJam connectors into plain layers. Scrim still has it.
- **Scope at the fork.** FigCraft's own scope then said: "Out of scope as separate products: FigJam, Figma Slides, …". This is `docs/target-app-parity.md:87`, still in our tree. `8167f13a` rewrote that line to bring whiteboarding in.

## 2. Every upstream board commit

Found by scanning all 1,064 non-merge commits since the fork (`scripts/find_commits.py`):
- **Signals scanned:**
  - board paths;
  - board words in the subject;
  - board identifiers (`BoardItem`, `FileKind::Board`, `board.*` command ids and so on) in changed lines.
- **Hand check.** Each hit was then checked by hand.
- **Dropped as noise.** Hits where "board" was part of another word (keyboard, clipboard, dashboard), Sketch's own "boards", and roadmap lines in unrelated commits.
- **Every row and its paths** are in [`commits.tsv`](upstream-board-scoping/commits.tsv).

Some commits sit in more than one group. In the lists below, "+n" is lines added and "n f" is the number of files touched.

**Data model and file format** (`crates/doc`, `crates/collab`):
- `8167f13a`: the model.
  - `Document.kind` (`FileKind::Design | Board`).
  - `Node.board: Option<BoardItem>`, which is Sticky, Shape, Connector, Label, Stamp or Ink.
  - The collab properties `Board` and `Kind`.
  - Doc +464, collab +6.
- `26291aff`: curved connectors, doc +18.
- `4fad29a6`: `facilitation.rs`, the timer and vote. Doc +125, collab +22 (`DocProp::Timer` and `DocProp::Vote`, `Entry::Ballot`).
- `9f4d1ce7`: vote history, doc +23, collab +3 (`DocProp::VoteHistory`).
- `4bcfd524`: 16 more shapes, doc +171.
- `b462a483`: a connector's saved path, doc +121.
- `12e9b409`: a connector end at any spot, doc +24.
- `9099f1d5`: `board_route.rs`, which routes connectors, doc +146.
- File-format changes are these serde fields only: no version change ([§3](#3-a-file-kind-not-a-new-format)).

**Engine commands** (22 `board.*` commands plus `file.newBoard`):
- `8167f13a`: sticky, shape, connector, label, stamp and ink; `nextSticky`, `setColor`, `setStyle` and `editText`; the tools and the normalize pass. Engine +1,468.
- `26291aff`: `board.reattach`, +266.
- `4fad29a6`: `board.timer`, `timerState`, `vote` and `voteResults`, +387.
- `d3782fdc`: `board.template`, `templates`, `table` and `tableEdit`, +572.
- `d4e6ad5d`: the connector tool preview, +116.
- `2d507d95`: `board.grid`, +147.
- `34fce9bf`: `board.addConnected`, +89.
- `4f5a25cc`: spreadsheet cells pasted as a table, +53.
- `e97961ba`: stamps stick to objects, +96.
- `9f4d1ce7`: vote history, +175.
- `b462a483`: `board.reshape`, +169.
- `59c76245`: side snapping, +19.
- `12e9b409`: an end at any spot, +58.

**UI** (`crates/ui-egui`): the Home card, board toolbar, contextual bar, sticky notes, shapes, connectors, text editing and the facilitation pill.
- `8167f13a`: Home card, toolbar, `board_bar`, tab and file-row marks, icons, the Design panel hidden. +688.
- `26291aff`: +102.
- `4fad29a6`: the timer and vote pill, +281.
- `d3782fdc`: the templates picker and table bar, +215.
- `2d507d95`: grid preferences, +104.
- `34fce9bf`: the quick-create ring, +125.
- `7e9d6ce8`: the connector's bar (ends, dashes, weight, colour), +166.
- `9f4d1ce7`: +120.
- `4bcfd524`: shape grid pickers, +28.
- Small fixes: `59af2bc4`, `9d86403f`, `b00cccb1`, `7f18940c`.
- Sticky and shape text editing reuse the ordinary text editor (`board.editText`, and a 7-line change to `textedit.rs`).

**File import** (`crates/format`):
- `8ee8ba6c`: a FigJam `.jam` opens as a board. +12 format, 32 f.
- `4e650a1c`: FigJam stickies and shapes keep their content. +77 format, 7 f.
- `01bc4741`: generated FigJam vectors and text are kept. +14 format, 5 f.
- `9099f1d5`: `.fig` connectors are routed. +68 format, 32 f.
- `838d3687`: `.fig` connector labels are kept. +146 format, 8 f.
- `8167f13a`: the thumbnail reader reports the kind, +14.

**Renderer** (`crates/render`): `26291aff` and `2d507d95` add the dot, line and graph-paper grid. That is `dots.rs` (171 lines) plus hooks in `lib.rs`, `retained.rs` and the UI's `render_worker.rs`, about +201. The board objects themselves render as ordinary layers.

**MCP and CLI:**
- No board commit touches `crates/mcp`, the CLI or the control channel.
- Board commands reach agents through the generic `run_command`, and `inspect` reports the kind. `docs/mcp.md` gains 51 lines in `8167f13a`.

**Tests:** 4,169 lines added across these commits. At upstream's head, 3,701 lines sit in 19 board-only test files ([`board-files.tsv`](upstream-board-scoping/board-files.tsv)), including:
- `board_tests`, `board_tools_tests`, `board_connector_tests` and `board_reshape_tests`;
- `facilitate_tests`, `board_ui_tests` and `board_import_tests`;
- `board_sync_tests` (two clients in a live session).

**Docs:**
- `docs/boards-parity.md`, 91 lines at head;
- rows in `ROADMAP.md`, `gaps.md`, `file-format-parity.md` and `target-app-parity.md`;
- README screenshots (`948358ee`, `a5f88f14`);
- credits (`6cc466a8`, `7081bbdd`).

**Refactors only:**
- `5d1b826f` and `688eeccc` move and split the engine's board code.
- `3e23a0f8` and `49ca56c3` move the UI's.
- Also `5e166839`, `60c671da`, `5f7aea91`, and `236f3185` (upstream's move to egui 0.37, which reaches board UI).
- About 9,200 lines change hands here; none of it is new behaviour.

| | Commits | Files touched (sum) | Lines added | Lines deleted |
|---|---|---|---|---|
| Features | 14 | 449 | 12,618 | 739 |
| FigJam and `.fig` import | 5 | 84 | 1,092 | 264 |
| Fixes | 5 | 13 | 135 | 32 |
| Docs only | 4 | 22 | 17 | 9 |
| **Board work** | **28** | **568** | **13,862** | **1,044** |
| Refactors and moves | 8 | 227 | 9,229 | 2,832 |

Of the 13,862 lines added, by area (3 more are in other files):

| Area | Lines added |
|---|---|
| engine | 3,633 |
| ui-egui | 1,895 |
| doc | 1,100 |
| format | 332 |
| render | 201 |
| collab | 31 |
| tests | 4,169 |
| translations | 1,987 |
| docs | 511 |

## 3. A file kind, not a new format

**A board is a mode of an ordinary document,** not its own document type:
- **Kind.** `Document` gains `kind: FileKind`, either `Design` (the default) or `Board`. Serde leaves it out for design files.
- **Board objects.** Each node may carry `board: Option<BoardItem>`. It is also left out when absent, so a design file's JSON doesn't change.
- **Ordinary layers.** Board objects are frames, vectors and text. Rendering, hit testing, undo, saving, export, copy and paste and live sessions treat them as layers.
- **Derived geometry.** A sticky's text size, a shape's text room and connector routes are recomputed in the engine's normalize pass, deterministically from the document.

**No format version change.** Upstream's `FORMAT` is still `"figcraft"` and `VERSION` is still `1`, the same as Scrim's (`crates/format/src/lib.rs:54-55`).

**What happens with our `.scrim` format:**
- **Same container.** A `.scrim` file is the same format, so a port adds the two optional fields with no version change, and design files stay byte-identical.
- **Scrim today (checked on `b790dea1`).** The test took a document saved by Scrim, added `"kind": "board"` and a sticky's `"board": {"type": "sticky", …}` to it, and ran our `load_bytes` and `save_bytes` ([`scripts/board_file_roundtrip.rs`](upstream-board-scoping/scripts/board_file_roundtrip.rs), [output](upstream-board-scoping/board-file-roundtrip-output.txt)):
  - Scrim opened it with no issues reported, as a design file.
  - Saving dropped both marks.
  - So a FigCraft user's board, opened in Scrim and saved, silently stops being a board: connectors no longer follow what they join, and stickies no longer fit their text. Its layers stay as they were.
- **Older Scrim builds.** After a port, an older Scrim would do the same to a board Scrim saved.

**What happens with the import registry:**
- **Boards need no importer.** The native importer reads the field.
- **`.jam` files.** Upstream's `8ee8ba6c` marks a canvas with FigJam's `fig-jam.` header as a board and adds `jam` to the `.fig` importer's extensions.
- **Scrim's `.fig` importer today:**
  - **Sniff.** It recognises any canvas starting `fig-` (`crates/format/src/fig/mod.rs:85`). The registry picks importers by content, so a `.jam` should already import as a design file.
  - **Open dialog.** It lists only `fig` (`crates/engine/src/importers/kinds.rs:115`), so the dialog filter doesn't offer `.jam`.
  - **Docs.** `docs/file-format-parity.md:21` still lists `.jam` as out of scope.
  - **Not tried.** We have no real `.jam` file.

**Live sessions: the wire format needs care.**
- **What upstream adds.** It appends wire variants:
  - `Prop::Board`, placed after `Prop::Measurements`;
  - `DocProp::Kind`, `Timer`, `Vote` and `VoteHistory`;
  - `Entry::Ballot`.
- **Why order matters.** Postcard encodes a variant by its position.
- **The catch.** `Measurements` comes from upstream's Dev Mode measurements (`e3e39bbc`), which Scrim doesn't have. To stay wire-compatible with upstream (Fix AI's COL-1 kept that), a port must add `Measurements`, or a reserved variant in its place, before `Board`.
- **Today.** A Scrim peer would fail to decode an upstream peer's board change. That is read from the code, not run.

## 4. What a port touches

**Crates and layers.** All board code sits in crates we already have:

| Crate | Layer |
|---|---|
| `doc` | L1 |
| `collab` | L2 |
| `render` | L3 |
| `format` | L3 |
| `engine` | L5 |
| `ui-egui` | L6 |

- **Dependencies.** Each crate's board code uses only crates below it, and none uses a network crate. No new edge between crates: `render` already depends on `format`.
- **Not checked by the tool.** `cargo xtask layers` wasn't run on board code, since none was ported.

**The renderer:**
- **New code.** One new module: the board grid's dots, lines and graph paper (`dots.rs`, 171 lines, plus 136 of tests).
- **Hooks.** In `render/src/lib.rs` (484 lines in Scrim) and `retained.rs` (415), plus the UI's `render_worker.rs`.
- **Board objects** need no renderer change.

**egui.** All the UI is egui:
- **Board-only panels:** the board toolbar, the contextual bar (`board_bar.rs`, 585 lines), the connect ring, the facilitation pill and the shape and template pickers.
- **Shared screens:** Home, tabs, icons, menus, shortcuts and Preferences.
- **egui version.** Upstream moved to egui 0.37, on a patched fork, on 25 Oct, the day after boards landed. Scrim is on 0.36.
  - `8167f13a` was written against 0.36.
  - Every later board UI commit targets 0.37, and needs translating back.

**Frozen files** (over 1,000 lines, `xtask/size-allowlist.txt`; they may shrink, never grow). The table maps upstream's shared files onto Scrim's layout. Upstream has since split several of these files; Scrim hasn't.

| Scrim file | Lines | What boards add there (from upstream's diffs) |
|---|---|---|
| `crates/engine/src/lib.rs` | 1,773 | Connector re-routing in `normalize` (upstream's later `derived.rs`); board preferences; prototyping and Dev Mode refused on boards |
| `crates/engine/src/tools.rs` | 2,276 | The board tool ids and pointer dispatch (upstream's `canvas/tools/*`) |
| `crates/engine/src/cmd/edit.rs` | 1,334 | Duplicate and paste remap connector ends; tables pasted from cells |
| `crates/engine/src/cmd/file.rs` | 1,673 | `file.newBoard`; the kind kept on open; `.jam` opens as a board |
| `crates/ui-egui/src/lib.rs` | 1,692 | The Design panel hidden on boards; board toolbar wiring |
| `crates/ui-egui/src/canvas.rs` | 1,509 | The board bar and grid in the canvas overlays |
| `crates/ui-egui/src/icons.rs` | 1,222 | The pen nib and note marks; shape outlines |
| `crates/ui-egui/src/menus.rs` | 1,371 | File ▸ New board; pasting a table |
| `crates/format/src/fig/map.rs` | 3,034 | The FigJam import changes and connector routing |

- **Near the cap.** `ui-egui/src/home_recents.rs` (985 lines) and `doc/src/lib.rs` (966) get small edits.
- **First step of each stage.** Each stage starts by moving the code it touches out of these files. That is how Fix AI's ports handled `doc/lib.rs` and `engine/library.rs`.
- **Upstream's own size.** Upstream's first engine `board.rs` was 1,307 lines, over our limit. A port takes upstream head's split layout (`boards/board/*.rs`, each under 400 lines).

## 5. Licence and dependencies

- **Licence.** Upstream is still MIT OR Apache-2.0 ([upstream-sync.md › Licence](upstream-sync.md#licence)).
  - **No third-party code.** No board file carries another copyright, licence, SPDX tag or "adapted from" note (scanned at head).
  - **Templates.** The commit messages say the templates are "all FigCraft's own".
  - **Icons.** The tab marks are "original icons drawn in code".
- **Dependencies.** No board commit touches `Cargo.toml` or `Cargo.lock`.
  - The egui 0.37 move is upstream's, separate from boards, and isn't needed.
- **Brand.** No board source file puts "FigCraft" in user-facing text, so Scrim's brand check would pass. Translated strings would need the usual review.

## 6. Cost, and a staged plan

**Verdict: large.** It is a port, not a rebuild:
- **The code is self-contained.** Board-only files make up 5,502 source lines and 3,701 test lines at head.
- **It fits our crates.** It is written in our crates' idioms, panic-free, licence-clean, and adds no dependencies.
- **Rebuilding would cost more.** It would mean redesigning the model, losing file compatibility with FigCraft's boards and losing wire compatibility with upstream peers.

The work beyond copying:
- moving code out of the 9 frozen files;
- back-porting the UI to egui 0.36;
- the wire-order decision;
- the two rule fixes in [§8](#8-conflicts-with-scrims-rules);
- a gesture parity entry and a checklist row for each capability;
- Scrim's 16 locales.

Effort is in working days for one person with an agent, including tests, `cargo xtask ci` and a live check.

| Stage | What it brings | Upstream source | Effort |
|---|---|---|---|
| **0. Stop the silent stripping** (optional, any time) | When Scrim opens a file whose `kind` is `board`, it says so: "This is a FigCraft board. Scrim opens it as a design file, and its board behaviour (connectors that follow, stickies that fit) is lost when you save." The alternative, keeping the two fields round-trip with no behaviour, needs the wire decision below. | none; a load note | **S, half a day** |
| **1. Boards with sticky notes and shapes** (smallest useful slice) | Everything this stage needs:<br>• `FileKind` and the `board` field, Sticky and Shape (7 outlines);<br>• `file.newBoard`, the Home card and the tab marks;<br>• `board.sticky`, `nextSticky`, `shape`, `setColor` and `editText`, with sticky text fitted in normalize;<br>• the board toolbar (Move, Hand, Sticky, Shape, Text, Section) and the contextual bar for colour;<br>• the dot grid;<br>• the Design panel hidden, prototyping and Dev Mode refused;<br>• collab `Kind` and `Board`, placed in upstream's order. | from `8167f13a` and `26291aff`, `2d507d95` (grid) | **M–L, 4–5 days** |
| **2. Connectors** | Straight, elbow and curved connectors that follow what they join:<br>• labels;<br>• re-attach by dragging an end;<br>• side snapping and ends at any spot;<br>• reshaping the path;<br>• the connector bar (ends, dashes, weight, colour);<br>• creating the next shape already connected.<br>The connector tool's preview moves to the overlay ([§8](#8-conflicts-with-scrims-rules)). | `8167f13a`, `26291aff`, `d4e6ad5d`, `34fce9bf`, `7e9d6ce8`, `b462a483`, `59c76245`, `12e9b409` | **L, 4–6 days** |
| **3a. Ink and stamps** | Marker, highlighter and eraser, the eraser committing through a new `board.erase`; stamps, sticking to what they land on. | `8167f13a`, `e97961ba` | **M, 1.5–2 days** |
| **3b. More shapes, grids, templates, tables** | 16 more shapes in grid pickers; dot, line or graph-paper grids; five templates; tables, including cells pasted from a spreadsheet. | `4bcfd524`, `2d507d95`, `d3782fdc`, `4f5a25cc` | **M, 2–3 days** |
| **3c. Facilitation** | A shared timer, and voting sessions with results and history. This adds more wire variants (`Timer`, `Vote`, `VoteHistory`, `Ballot`). | `4fad29a6`, `9f4d1ce7` | **M–L, 2–3 days** |
| **3d. FigJam files** | `.jam` opens as a board. The four FigJam and `.fig` import fixes can come earlier, independently ([§7](#7-audit-fixes-that-touch-board-code)). | `8ee8ba6c` (+ `4e650a1c`, `01bc4741`, `9099f1d5`, `838d3687`) | **S–M, 1–2 days** |

**Total for parity with upstream's head: about 15 to 21 days**, about 47% of FigJam's breadth by upstream's own estimate. The following are missing upstream too, and aren't in this plan:
- music;
- board widgets;
- mind maps.

**Decision needed before stage 1:** keep live sessions wire-compatible with upstream (add `Measurements`, or reserve its slot), or accept that board edits won't cross between Scrim and upstream peers.

## 7. Audit fixes that touch board code

Fix AI stage 1 found 300 upstream fixes that still apply to Scrim ([`scripts/audit_fixes.py`](upstream-board-scoping/scripts/audit_fixes.py), [output](upstream-board-scoping/audit-fixes-output.txt)):
- **Only one of the 300 touches board-named code.** `4e650a1c`, "preserve FigJam sticky and shape content on import" (verdict M, `fig/map.rs` frozen).
  - Its `board_snapshot` and `board_import_tests.rs` concern the `.fig` importer, not `BoardItem`. The test file uses no board type.
  - It doesn't need boards: port it with the import fixes, whatever is decided here.
- **Among the 143 that don't apply,** one is a board fix: `59c76245`, connector side snapping. It comes with stage 2 and is skipped otherwise.
- **The other board fixes** (`59af2bc4`, `9d86403f`, `b00cccb1`, `7f18940c`) weren't among the 451 bug, security, performance and dependency commits the audit read. They come with their stages.

So nothing in the fix backlog has to wait for boards, and nothing in it is wasted if we skip them.

## 8. Conflicts with Scrim's rules

Read from upstream's head (`boards/board/tools.rs` and the UI files), against `AGENTS.md` › Scrim's rules:

- **Every UI action is a command. Holds, with one gap.**
  - The board UI dispatches through `app.run("board.…")`: `setStyle`, `setColor`, `grid`, `addConnected`, `timer`, `vote` and `tableEdit`.
  - The tools call `execute("board.sticky" …)` and the like on release: 22 `board.*` commands plus `file.newBoard`.
  - **The gap is the eraser.** Its drag removes strokes through `begin_interaction` / `preview` and ends with `commit_interaction()`, with no command id. That breaks rule 1 (one command id per gesture, with an MCP route).
  - **Port:** add `board.erase {ids}`, and commit the gesture through it.
- **Previews never write to the document. One gesture conflicts.**
  - Edits of existing layers (re-attach, reshape, moving stamps) preview the way Scrim's own drags do: through an interaction on a snapshot, committed once on release, with no history before it. They fit.
  - **The conflict:** the connector tool previews a new connector by building it into the session's document during the drag (`connect_drag` calls `self.preview(… make(…))`), then cancels and executes `board.connector`. Scrim's R17 says a creation gesture previews on the overlay, drawn from the same function the release uses.
  - **Port:** draw the connector from `route()` on the overlay instead.
  - The marker and highlighter already preview on the overlay (`Gesture::Pencil`) and commit with `board.ink`.
- **Gesture parity and checklists.** Each board gesture needs an entry in the parity test (rule 1), and each capability a row in `docs/checklists/` (rule 4). Upstream has neither in this form.
- **No `unwrap`, `expect` or `panic`. Holds.** Upstream's board source has none. Its crates deny the same lints as ours: `clippy::unwrap_used`, `expect_used`, `panic`, `unimplemented`, `todo` and `unreachable`.
- **Layers enforced. Holds** ([§4](#4-what-a-port-touches)).
- **Files over 1,000 lines frozen.** Upstream's first `board.rs` broke this. A port takes head's split layout and moves code out of our frozen files first ([§4](#4-what-a-port-touches)).

## 9. Recommendation

**Skip boards for now. If they become a goal, port them in stages (§6) rather than rebuild. Do stage 0 whenever convenient.**

Why skip now:
- **Scope.** Scrim is a design tool with agents as collaborators. FigCraft itself had FigJam out of scope at our fork, and upstream still keeps boards out of its weighted parity sum ("a separate workflow").
- **Cost against the backlog.** 15 to 21 days is roughly what the rest of the audit's High and Medium fixes will take, and those apply to code Scrim users run today.
- **A moving target.** Upstream wrote boards in 8 days and was still changing them daily at its head: 14 feature commits, 5 on 31 Oct alone. Porting now means chasing it; porting later takes a settled version.

Why port rather than rebuild:
- **Ready to use.** The code is self-contained, licence-clean, panic-free and dependency-free, in our crates' own idioms.
- **Compatibility.** Porting keeps FigCraft's board files and upstream's live sessions working with Scrim. A rebuild would fork both.

Why stage 0: Scrim silently strips board files today (§3). A half-day notice on open makes that visible, without committing us to boards.

## 10. What could not be determined

- **`.jam` files in Scrim today.** No real `.jam` file was available. That they import as design files by content, but aren't offered by the Open dialog, is from reading the code.
- **Live sessions with upstream peers on a board.** That a Scrim peer fails to decode an upstream peer's board change is inferred from postcard's variant positions, not run.
- **Exactly how much each frozen file would grow.** Scrim's layout differs from upstream's split files. §4 lists what goes where, not line counts.
- **The egui 0.37 to 0.36 differences** in the board UI weren't catalogued; the estimates allow for them.
- **The effort figures are judgment**, calibrated on Fix AI's ports. Upstream wrote the code in 8 days with heavy agent help.
- **Whether Scrim's users want boards.** There is no usage or request data.
- **Translation quality.** Upstream's board translations for our 16 locales weren't reviewed.
- **Nothing upstream was built or run.** Upstream's tests weren't run, and no upstream binary was used.

## 11. Evidence files

In [`upstream-board-scoping/`](upstream-board-scoping/). The clone is `~/.cache/figcraft-upstream-audit`, a clone of Scrim with an `upstream` remote; it was read with `git show` and `git log` only.

| File | What it is |
|---|---|
| [`commits.tsv`](upstream-board-scoping/commits.tsv) | The 36 board commits: kind, files, lines added and deleted per area, and paths |
| [`board-files.tsv`](upstream-board-scoping/board-files.tsv) | Every board-only file at upstream's head, with its length, whether it is source or test, and the commit that added its path |
| [`board-file-roundtrip-output.txt`](upstream-board-scoping/board-file-roundtrip-output.txt) | Scrim at `b790dea1` opening and re-saving a board file: the marks are dropped |
| [`audit-fixes-output.txt`](upstream-board-scoping/audit-fixes-output.txt) | Which of the 300 applicable fixes touch board code: one, `4e650a1c` |
| [`scripts/find_commits.py`](upstream-board-scoping/scripts/find_commits.py) | Finds the candidate commits; the result was hand-checked into `commits.tsv` |
| [`scripts/commit_stats.py`](upstream-board-scoping/scripts/commit_stats.py) | The per-area line counts in `commits.tsv` |
| [`scripts/audit_fixes.py`](upstream-board-scoping/scripts/audit_fixes.py) | The §7 check against `upstream-sync/verdicts.tsv` |
| [`scripts/board_file_roundtrip.rs`](upstream-board-scoping/scripts/board_file_roundtrip.rs) | The §3 check: a scratch binary built outside the repo against `crates/format` and `crates/doc` |
