# Scrim decisions digest, for the Rust fork of FigCraft (part 1 of 2)

**What this is.** Scrim was a TypeScript/React/Electron design tool whose canvas was the browser DOM. It is being archived. Its knowledge moves into a Rust fork of FigCraft: egui chrome, a tiny-skia CPU renderer, a persistent `imbl` document, one command entry point `Session::execute(id, json)`, snapshot undo, a 562-command catalogue, an MCP server over stdio, and layered crates checked by `cargo xtask layers`. In the fork **the canvas is not the DOM**.

This digest lists every Scrim decision, hard rule and architectural invariant, and says what happens to each in the fork.

- **This file:** the summary table, the four LOCKED decisions (D1–D4) and the hard rules (R1–R18, Scrim's own numbers, which other Scrim docs cite).
- **`decisions-architecture.md`:** every other principle (A1–A60), then the **open questions for the human**.

**Verdicts.**
- **CARRY:** applies as-is in Rust.
- **ADAPT:** the reason survives, the form changes. The entry gives the Rust or FigCraft form.
- **DROP:** specific to the DOM, Electron, React, Vite, CSS naming in the model, `applyPatch` naming or the `scrim://` protocol. The entry says why, and what (if anything) replaces it.

**FigCraft names used below** come from `docs/research/figcraft-spike.md` §7–8. Verify them against the fork before you write a check:
- `Session::execute` (`crates/engine/src/lib.rs`): panic guard, validation, rollback on error, one undo step, a journal entry.
- `CommandSpec` and the command modules (`crates/engine/src/cmd/`).
- The document (`crates/doc/src/lib.rs`): `imbl::OrdMap<NodeId, Arc<Node>>`, 17 node kinds, layout baked into transforms and sizes.
- Undo: one `Arc<Document>` snapshot per step, limit 1,000.
- `cargo xtask layers` (`xtask/src/layers.rs`): crates in layers L0–L6, lower-only dependencies, no egui or network below L6.
- `doc_mut()` is FigCraft's real mutable accessor: `Session::doc_mut()` (`crates/engine/src/lib.rs:886`, returns `Result<&mut Document>`) and `DocState::doc_mut()` (`lib.rs:558`). Checked against the fork at `a0d1162`.

**Scrim paths** are relative to the Scrim repo root (github.com/Abib189/scrim, archived after the fork).

## Summary

| Id | Title | Verdict |
|---|---|---|
| D1 | The DOM is the canvas | DROP |
| D2 | Document model: flat ID-keyed tree, one write path, LWW | ADAPT |
| D3 | Single-player first, one authority, sync-ready semantics | ADAPT |
| D4 | Layout: auto layout first, absolute as a per-child opt-in | ADAPT |
| R1 | Every mutation through the one write path | ADAPT |
| R2 | Model changes ship with tests | ADAPT |
| R3 | No new dependencies in the core crates; a dependency log | ADAPT |
| R4 | CSS-aligned camelCase property names in the model | DROP |
| R5 | Child order is paint order is export order | CARRY |
| R6 | Exported code looks hand-written | CARRY |
| R7 | Editor chrome never enters the design | ADAPT |
| R8 | Strict types, no escape hatch | ADAPT |
| R9 | Chrome geometry from resolved layout, never stored sizes | ADAPT |
| R10 | Creation defaults live in the tools, not the factory | CARRY |
| R11 | One gesture, one commit, one undo step | ADAPT |
| R12 | No network except the user-initiated font fetch | ADAPT |
| R13 | Source files under 500 lines | ADAPT |
| R14 | The terminal is never a network surface; nothing runs on the user's behalf | ADAPT |
| R15 | Never push a red tree | ADAPT |
| R16 | Library files are never lost | CARRY |
| R17 | Every canvas gesture is registered with its preview and commit | ADAPT |
| R18 | Imported files: type from bytes, content-addressed, never lost | ADAPT |
| A1 | Measured geometry is ephemeral editor state | ADAPT |
| A2 | Surviving large documents: culling, one camera transform, deferred commits | ADAPT |
| A3 | A spatial index for hit-testing, marquee and snapping | ADAPT |
| A4 | Hit-testing follows what is visible | CARRY |
| A5 | Export fidelity is measured against the canvas | ADAPT |
| A6 | The export gate: pinned bytes | CARRY |
| A7 | Additive optional fields; explicit migrations; refusals with reasons | ADAPT |
| A8 | One home per value (shorthands, size in style, longhands) | DROP |
| A9 | Lint configuration lives in the document | CARRY |
| A10 | Duplication rule | CARRY |
| A11 | Structural master edits refused while instances live | ADAPT |
| A12 | Images referenced by hash; portable files carry them | ADAPT |
| A13 | File caps checked from the size, before reading | CARRY |
| A14 | Editor UI state is never document data | CARRY |
| A15 | One active document: switched, never merged | CARRY |
| A16 | Per-document undo, capped | ADAPT |
| A17 | The epoch guard | ADAPT |
| A18 | Library import dedupes by content hash | CARRY |
| A19 | Thumbnails | ADAPT |
| A20 | The preview layer: a local view, never a commit | ADAPT |
| A21 | Gesture previews use the commit's own function | CARRY |
| A22 | Stored size is the visual size is the exported size | ADAPT |
| A23 | Movement never sizes, except the node that leaves | ADAPT |
| A24 | Reparenting keeps the visual position, through one helper | ADAPT |
| A25 | Grid: placement, not order | ADAPT |
| A26 | Layout guides: document data, never exported | CARRY |
| A27 | Vector content is structured data behind an allow-list | ADAPT |
| A28 | Drawn vectors: the box is derived from the geometry | ADAPT |
| A29 | Vector hit band and pencil fit | CARRY |
| A30 | `.fig` import: own decoder, limits first, one commit, a report | ADAPT |
| A31 | HTML import posture (designed, not built) | ADAPT |
| A32 | Decode and image-end checks before storing | ADAPT |
| A33 | Background verification only records | CARRY |
| A34 | Display previews and decode-before-swap | ADAPT |
| A35 | Orientation follows what the renderers draw | ADAPT |
| A36 | Location privacy | CARRY |
| A37 | Unused images: mark, verify, grace, delete | CARRY |
| A38 | Modifier conventions while dragging | CARRY |
| A39 | Measurement chrome shows what will commit | CARRY |
| A40 | One code path per action | ADAPT |
| A41 | Agents never manage documents or touch the file system | ADAPT |
| A42 | Agents fix forward; they do not undo | ADAPT |
| A43 | MCP write-tool rules | ADAPT |
| A44 | Agents place by real geometry, never by guesses | ADAPT |
| A45 | `get_selection`: the person's selection as shared context | ADAPT |
| A46 | `get_screenshot` from the canvas renderer, without chrome | ADAPT |
| A47 | Lint for agents | ADAPT |
| A48 | Attribution and presence | ADAPT |
| A49 | MCP security baseline | ADAPT |
| A50 | Agents are told when fonts are not loaded | ADAPT |
| A51 | The read-only query subset | CARRY |
| A52 | `analyze_design` | CARRY |
| A53 | The CLI answers through the MCP read tools | ADAPT |
| A54 | The parity rule | ADAPT |
| A55 | Motion: the settled state, an inert canvas, one cascade | ADAPT |
| A56 | Editor-only data is fenced; export names are derived | CARRY |
| A57 | LOCKED decisions stop a session; the Divergence log | CARRY |
| A58 | Deterministic tests | ADAPT |
| A59 | README stub first; manual checks name their test | CARRY |
| A60 | v1 non-goals | ADAPT |

**Counts across both files (82 entries):** CARRY 25, ADAPT 54, DROP 3. The open questions for the human are at the end of `decisions-architecture.md`.

---

## The four LOCKED decisions

Source for all four: `ARCHITECTURE.md` › the Decision sections. "LOCKED" meant a session could not change one without the human (A57).

### D1. The DOM is the canvas
- **Source:** `ARCHITECTURE.md` › Decision 1 (and its notes: performance, stress results, measured rects).
- **Verdict:** DROP. The fork's canvas is FigCraft's tiny-skia renderer, so the mechanism cannot survive. The pieces of it that still apply are carried separately: culling and deferred commits (A2), the spatial index (A3), chrome kept out of the design (R7), and measured geometry as editor state (A1). The promise the DOM gave for free, that the exported HTML looks exactly like the canvas, becomes a measured obligation (A5).
- **Why it existed:** the design *was* the markup, so `get_jsx` was lossless and the browser supplied layout, fonts, text shaping and accessibility. Scrim matched its canvas pixel for pixel on 15–16 of 20 artboards (`docs/research/openpencil-fork-decision.md` §1), where FigCraft matched 8 of 19.

### D2. Document model: flat ID-keyed tree, one write path, LWW
- **Source:** `ARCHITECTURE.md` › Decision 2; `packages/core/src/schema.ts`, `packages/core/src/ids.ts`.
- **Verdict:** ADAPT.
  - **Already true in FigCraft:** nodes live in a flat map keyed by id (`imbl::OrdMap<NodeId, Arc<Node>>`).
  - **Carry:** ids stay stable for a node's lifetime. Hierarchy is parent and child-id fields, never nesting. `schemaVersion` migrations are explicit and forward-only (A7).
  - **The single write path** is `Session::execute` (R1), not `applyPatch`.
  - **Conflict semantics** stay last-writer-wins per property, even in single player. FigCraft's collab is already property-granular, server-ordered LWW.
  - **Undo** may stay as `Arc` snapshots instead of Scrim's inverse patches. What carries is that agent writes are undoable like a person's and share the same history.
  - **Drop:** "flat, CSS-named style that serialises to CSS with zero translation" (R4). FigCraft's model is Figma-shaped, so CSS naming lives only in the exporter.
- **Why it existed:** the schema was the contract between the editor, MCP, codegen and the future sync server, and the most irreversible thing in the project. Flat maps diff, patch and sync cleanly. LWW from day one meant multiplayer would change the transport, not the semantics.
- **Test:** the R1 check. A serde round-trip test over every fixture (load, save, load, compare). A test that a command sent through MCP and one sent by the UI land on the same history and both undo.

### D3. Single-player first, one authority, sync-ready semantics
- **Source:** `ARCHITECTURE.md` › Decision 3 (with the status note and the amendment of 2 Oct 2026); `packages/mcp/src/store.ts`.
- **Verdict:** ADAPT.
  - **Scrim's form:** the MCP server's `DocStore` was the one authority, and the studio rendered only from the server's broadcasts.
  - **The fork's form:** the `Session` is the one authority. The egui UI, MCP, the CLI and plugins are all clients of it. No client keeps its own document copy that could diverge: the UI reads the session's current `Arc<Document>`.
  - **Collaboration:** Yjs and Hocuspocus drop. If multiplayer comes, use FigCraft's own collab (server-ordered LWW), which it rates 13% ready.
  - **Human and agent** are interleaved writers on one shared undo stack.
  - **Forbidden, carried verbatim:** any feature that writes to the document outside the one write path.
- **Why it existed:** multiplayer was the biggest cost and the target user (one designer-developer plus an agent) didn't need it. The agent was already a second concurrent writer, and the one-authority, LWW discipline made that safe.
- **Test:** the R1 check. A test that interleaves UI and MCP commands, then undoes all of them in order, and gets back the starting document.

### D4. Layout: auto layout first, absolute as a per-child opt-in
- **Source:** `ARCHITECTURE.md` › Decision 4 (and its amendments: hand-drawn frames are free and clip; size lives in style; sizing modes).
- **Verdict:** ADAPT.
  - **Drop:** "the browser is the layout engine". FigCraft solves layout in Rust and stores the result as absolute transforms and sizes.
  - **Carry:** the model keeps the layout *intent*: direction, gap, padding, alignment, wrap, grid tracks, and per-child sizing modes (Fixed, Hug, Fill) plus absolute-or-in-flow. Transforms are derived from that intent and never become the source of truth for an auto-layout child.
  - **Defaults:** agent-created frames default to auto layout. Hand-drawn frames are free (children absolute) and clip (R10). Absolute stays a per-child opt-in inside auto-layout frames. Text hugs by default. Root artboards sit at canvas positions that are never exported.
  - **Export:** maps the intent to idiomatic flex or grid (R6), never to the baked transforms. Scrim's sizing mapping was:
    - Fixed: a px size plus `flex-shrink: 0` on the parent's main axis;
    - Hug: `fit-content`;
    - Fill: `flex: 1 1 0` on the main axis, `align-self: stretch` across it, never `width: 100%`.
- **Why it existed:** Scrim's thesis was that exported code is idiomatic flexbox, not a pile of absolute positions, so auto layout had to be the default mental model. FigCraft stores layout as absolute transforms, which is exactly the "position soup" this decision exists to prevent (`docs/research/openpencil-fork-decision.md` §6).
- **Test:**
  - An export test over auto-layout fixtures: every flow child exports without `position: absolute`, and every frame exports `display: flex` or `grid`.
  - A test that moving a sibling and re-running layout never rewrites an auto-layout child's stored intent.

---

## Hard rules 1–18

Source for all: `CLAUDE.md` › Hard rules (the numbers are Scrim's own). The detail behind rules 12–18 is in `ARCHITECTURE.md`.

### R1. Every mutation through the one write path
- **Source:** `CLAUDE.md` rule 1; `ARCHITECTURE.md` › Decision 2 Rules and Decision 3 "Forbidden".
- **Verdict:** ADAPT.
  - Every mutation, from the UI, MCP, the CLI, plugins, tests, templates and file import, goes through `Session::execute` (or a `batch`, which is one execute).
  - No `doc_mut()` outside `crates/engine/src/cmd/`. The spike counted one direct document write in FigCraft's UI (`docs/research/figcraft-spike.md` §7); route it through a command.
  - Make the mutable accessor `pub(crate)` in the engine, so the compiler refuses a write from the UI crate.
- **Why it existed:** "the most important invariant in the codebase". One write path with three writers (UI, MCP, undo) made agents safe as concurrent writers and kept the door to multiplayer open.
- **Test:**
  - Visibility: the accessor is `pub(crate)`.
  - An xtask check that greps for `doc_mut(` outside `crates/engine/src/cmd/` and fails on a hit.
  - A test that a failing command leaves the document and the history unchanged (FigCraft's rollback).

### R2. Model changes ship with tests
- **Source:** `CLAUDE.md` rule 2; `ARCHITECTURE.md` › Working agreement.
- **Verdict:** ADAPT.
  - A change to `crates/doc` or to a command in `crates/engine/src/cmd/` updates its tests in the same change.
  - Run `cargo test --workspace` before calling a task done.
  - Scrim also had a human review every codegen snapshot change; keep that for the exporter.
- **Why it existed:** the model was the contract for everything else, and every other package trusted it blindly.
- **Test:** a catalogue-coverage test: every `CommandSpec` id is executed by at least one test, with an allow-list that may only shrink.

### R3. No new dependencies in the core crates; a dependency log
- **Source:** `CLAUDE.md` rule 3; `ARCHITECTURE.md` › Stack › Dependency log; `packages/mcp/src/__tests__/repoRules.test.ts` ("rule 3").
- **Verdict:** ADAPT.
  - Freeze the dependency sets of the low-layer crates (geom, color, doc, engine) at fork time. Adding to them needs the human.
  - Anywhere else, a new crate needs a one-line justification in the fork's dependency log.
  - Vendored data (Scrim's Impeccable design rules) keeps its licence, NOTICE and pinned version.
- **Why it existed:** a zero-dependency core stayed pure, testable and safe to embed. The log made every dependency a decision with a reason.
- **Test:** an xtask `deps` check that compares each crate's `[dependencies]` with a committed allow-list and fails on anything new. It runs beside `cargo xtask layers`.

### R4. CSS-aligned camelCase property names in the model
- **Source:** `CLAUDE.md` rule 4; `ARCHITECTURE.md` › Decision 2 Rules (`reactStyleKey`).
- **Verdict:** DROP. The fork's model is Figma-shaped: fills, strokes, effects and auto-layout fields. CSS names exist only inside the HTML/CSS exporter, and one mapping module there owns every name. Whether the model should move towards CSS semantics is open question 2.
- **Why it existed:** with the DOM as the canvas, the style map *was* CSS. Zero translation meant the canvas, `get_css` and the export could never disagree.

### R5. Child order is paint order is export order
- **Source:** `CLAUDE.md` rule 5; `ARCHITECTURE.md` › Decision 2 schema; `apps/studio/src/__tests__/childOrder.dom.test.tsx`.
- **Verdict:** CARRY. A parent's children list is the only ordering. It is paint order (later paints on top), layer-panel order and export order. There is no z-index field and no second list. If FigCraft's collab orders children by fractional indices, the index *is* that one ordering, never a second one beside it.
- **Why it existed:** two ordering sources drift, and then the canvas, the layers panel and the export disagree about what is on top.
- **Test:** render overlapping siblings and assert the top pixel belongs to the last child. Export the same frame and assert the elements appear in children order. Reorder, then assert both change together.

### R6. Exported code looks hand-written
- **Source:** `CLAUDE.md` rule 6; `ARCHITECTURE.md` › Decision 4 "Consequence".
- **Verdict:** CARRY. Idiomatic flexbox and grid, Tailwind classes, CSS defaults stripped, equal sides collapsed to shorthands, and no absolute-position soup. Write the code you would want in a PR, then make the exporter produce it. A free frame may export absolute children, because the designer chose it per frame (D4).
- **Why it existed:** "Scrim's export should look like a good frontend dev wrote it." It was the primary quality bar and the reason to choose Scrim over Figma.
- **Test:** the export gate (A6). A lint over exported fixtures that fails on `position: absolute` under an auto-layout parent.

### R7. Editor chrome never enters the design
- **Source:** `CLAUDE.md` rule 7; `ARCHITECTURE.md` › Decision 1 bullets and Divergence log item 4.
- **Verdict:** ADAPT.
  - Selection, handles, guides, marquee, snap lines, labels and measurement chips are drawn in a separate chrome pass, never into the document raster.
  - The node raster used by the renderer, screenshots, thumbnails and export never contains chrome.
  - All egui chrome is styled from one theme module, with no inline colours or sizes elsewhere. In Scrim that module was `apps/studio/src/theme.css`.
- **Why it existed:** a pure design layer is what gets exported, screenshotted and measured. Chrome mixed into it leaks into every output.
- **Test:**
  - A screenshot of a node with a selection, a hover and guides showing equals the screenshot with none of them.
  - An xtask grep that fails on colour or size literals (e.g. `Color32::from_rgb`) in `ui-egui` outside the theme module.

### R8. Strict types, no escape hatch
- **Source:** `CLAUDE.md` rule 8; `repoRules.test.ts` ("rule 8").
- **Verdict:** ADAPT.
  - Keep FigCraft's workspace lints: deny `unwrap`, `expect`, `panic`, `todo`, `unimplemented` and `unreachable` in shipped code; forbid `unsafe` outside the plugin SDK; run clippy with `-D warnings`.
  - Rust's equivalent of TypeScript's `any` is an untyped `serde_json::Value`. Do not add one to the document crate.
  - Command parameters are "free text" in FigCraft's `CommandSpec`. Type them as you touch them (open question 10).
- **Why it existed:** an untyped hole in the model or the MCP layer let malformed agent input reach the document.
- **Test:** the workspace lints and clippy in CI. An xtask grep that fails on `serde_json::Value` in `crates/doc`.

### R9. Chrome geometry from resolved layout, never stored sizes
- **Source:** `CLAUDE.md` rule 9; `ARCHITECTURE.md` › Decision 1 note "measured rects are ephemeral editor state".
- **Verdict:** ADAPT. Handles, outlines, snapping and hit-testing read the world bounds the layout pass resolved, the same boxes the renderer draws. They never read a node's stored width and height, which a Hug or Fill mode, a min or max, or a parent's layout can override.
- **Why it existed:** stored sizes lied for every node whose size its layout decided. The outline then disagreed with what was drawn.
- **Test:** for a Fill child, a Hug parent and a min-width-clamped node, the selection outline's rect equals the renderer's bounds for that node.

### R10. Creation defaults live in the tools, not the factory
- **Source:** `CLAUDE.md` rule 10; `ARCHITECTURE.md` › Decision 4 amendment "hand-drawn frames are free".
- **Verdict:** CARRY.
  - The engine's create command stays faithful to what its caller asked for: an agent's frame is auto layout.
  - The editor's Frame tool supplies its own defaults: free, clipping, children placed absolutely where drawn.
  - Other tool defaults (ellipse radius, line height, the Pen's stroke) also live in the tool code in `ui-egui`.
- **Why it existed:** people place things directly and expect them to stay where drawn, while agents author structure and their output is what the export bar judges. One factory default could not serve both.
- **Test:** the create command with minimal params yields an auto-layout frame. Driving the Frame tool headless yields a free, clipping frame.

### R11. One gesture, one commit, one undo step
- **Source:** `CLAUDE.md` rule 11; `ARCHITECTURE.md` › Decision 2 "Duplication rule".
- **Verdict:** ADAPT. A drag, nudge, duplicate, paste, group, align, z-order change or delete commits exactly one `Session::execute` call: one command, or one `batch`, which FigCraft records as one undo step.
- **Why it existed:** a gesture that undoes in pieces leaves the document in states the person never saw.
- **Test:** for every registered gesture (R17), the history length grows by exactly 1 on release and by 0 on Escape.

### R12. No network except the user-initiated font fetch
- **Source:** `CLAUDE.md` rule 12; `ARCHITECTURE.md` › Agent Dock › "Web font cache" and "The inspector's font picker".
- **Verdict:** ADAPT.
  - **Drop:** the renderer process, the CSP and `scrim://`, which are Electron forms.
  - **Carry:** the editor never loads remote content. The one network fetch is a font fetch, and it is:
    - started only by a click;
    - https only, to the Google Fonts CSS and font hosts, with redirects off that allowlist rejected;
    - rebuilt from the app's own font catalogue, never from free input;
    - checked by the file's own bytes and size caps, then cached under a content-hash name.
  - **Offline:** a font that fails to load leaves the fallback with a "not loaded" status and never blocks the editor.
  - **FigCraft:** its collab host and any other network client stay off until the human decides (open question 11).
- **Why it existed:** a design tool that phones home, or renders remote content, leaks what people work on and opens an attack surface. A click is consent.
- **Test:** `cargo xtask layers` already forbids network below L6. Add a check that exactly one module in the app crate depends on an HTTP client. Unit-test the font request builder's allowlist, redirect refusal and caps against a local fake server on port 0.

### R13. Source files under 500 lines
- **Source:** `CLAUDE.md` rule 13; `repoRules.test.ts` ("rule 13").
- **Verdict:** ADAPT. Rust source files stay under 500 lines; tests and generated data are exempt. Split by concern before crossing the limit. FigCraft likely has many larger files, so freeze them in an allow-list on the day of the fork. An entry may not grow and leaves the list once it is under the limit (open question 7).
- **Why it existed:** small files keep each concern reviewable, and agent sessions read whole files.
- **Test:** an xtask check with the allow-list. Scrim's test also tested its own logic: an unlisted big file, a grown entry, an entry back under the limit and a vanished entry.

### R14. The terminal is never a network surface; nothing runs on the user's behalf
- **Source:** `CLAUDE.md` rule 14; `ARCHITECTURE.md` › Agent Dock › "Security baseline — SUPERSEDED".
- **Verdict:** ADAPT.
  - **Drop:** Electron IPC and sender-frame validation.
  - **Carry if the Agent Dock is ported:**
    - the PTY lives in the app process and only the dock's UI code can reach it;
    - never through MCP, the CLI, a plugin, FigCraft's `--attach` control port, or any socket;
    - a session belongs to the window that opened it, at most 4 at once;
    - every PTY is killed on quit;
    - writes and resizes are bounded;
    - opens and closes are audited without keystrokes or output.
  - **Carry always:** nothing is ever executed on the user's behalf. "Connect Claude Code" types the `claude mcp add` command and does not press Enter.
- **Why it existed:** a loopback WebSocket terminal is reachable by every process on the machine and, through DNS rebinding, by any web page. That is remote code execution.
- **Test:** an xtask scan that fails if the PTY type is named outside the dock module, or if any MCP tool or command id reaches it. A unit test that the dock never sends a newline it did not receive from the keyboard.

### R15. Never push a red tree
- **Source:** `CLAUDE.md` rule 15.
- **Verdict:** ADAPT.
  - **Pre-commit hook:** `cargo fmt --check` and `cargo clippy --workspace -- -D warnings`.
  - **Pre-push hook:** `cargo test --workspace` and the xtask checks.
  - Hooks are installed by a documented setup step. Never `--no-verify`. If a red commit lands anyway, fix it in a follow-up and say so.
- **Why it existed:** most code was written by agents. A red main breaks every session after it.
- **Test:** the hooks themselves, and CI running the same commands.

### R16. Library files are never lost
- **Source:** `CLAUDE.md` rule 16; `ARCHITECTURE.md` › Decision 3 amendment "Nothing is lost"; `packages/mcp/src/library/README.md` › Never.
- **Verdict:** CARRY.
  - A path is built only from a validated UUID (`Uuid::parse_str`), never from a name.
  - A file that fails to parse is listed as unreadable with its path, and never deleted or overwritten. Migration copies legacy files and never writes them again.
  - Every write is atomic: a temp file, then a rename.
  - Trashing the open document is refused.
  - Every destructive action is confirmed and recoverable from the Trash, except Delete forever and Empty trash, which need an explicit `confirm: true`.
- **Why it existed:** a person's files are the one thing a design tool must never lose, and an agent-driven app has more ways to lose them.
- **Test:** port Scrim's library tests (`packages/mcp/src/__tests__/library*.test.ts`): a corrupt file is listed and kept, a path-traversal id is refused, trashing the open document is refused, and the irreversible actions without `confirm` are refused.

### R17. Every canvas gesture is registered with its preview and commit
- **Source:** `CLAUDE.md` rule 17; `ARCHITECTURE.md` › Decision 3 › "The gesture registry" and "Note: creation previews on the overlay"; `apps/studio/src/editor/gestureRegistry.ts`.
- **Verdict:** ADAPT.
  - Every canvas gesture in `ui-egui` is an entry in one registry, with its preview channel, its preview builder and its commit builder.
  - **Edits of an existing node** preview on the real node before release, through the preview layer (A20), never as an outline only.
  - **Creation gestures** (a line, a pencil stroke, a pen path) preview in the chrome pass, drawing exactly the geometry the release will create, from the same function.
  - Nothing is committed before release, and the release is one execute (R11).
- **Why it existed:** live preview was being added gesture by gesture, and an outline-only gesture went unnoticed until someone resized a child.
- **Test:**
  - A registry test scans `ui-egui` for gesture kinds the pointer code starts and fails on one that isn't listed.
  - A headless scenario per entry asserts the previewed document differs before release, the history is unchanged, and the committed document equals the last preview.

### R18. Imported files: type from bytes, content-addressed, never lost
- **Source:** `CLAUDE.md` rule 18; `ARCHITECTURE.md` › Imported images; `packages/mcp/src/assets/README.md`.
- **Verdict:** ADAPT.
  - **Carry:**
    - the type comes from the bytes, never a name or extension;
    - the store path is built only from a validated sha256 and a sniffed extension;
    - an SVG that could run script or reach outside itself is refused, never sanitised;
    - an existing file's hash never changes;
    - a referenced asset is never moved or deleted (A37);
    - GPS is stripped on import unless the person keeps it (A36);
    - a truncated file is refused, and bytes after the image's end are cut off before hashing (A32);
    - the background check only records (A33);
    - exports of photos that still carry location data warn first, and Cancel is the default.
  - **Drop:** "the renderer never receives a path", `webUtils.getPathForFile` and the CSP line. These were Electron process-boundary rules. Their Rust form: no command, MCP tool or plugin API takes a file system path for an asset; only the person's own file dialog or drop does (A41).
- **Why it existed:** imported files are untrusted input, the store must stay valid across documents and undo, and a photo can carry where it was taken.
- **Test:** port the fixtures in `packages/mcp/src/__tests__/fixtures/trailing/` and their tests (`assetTrailing.test.ts` and the asset tests). Add a test that no command parameter schema accepts a path, except the file commands the person drives.

---

Continue with `decisions-architecture.md` (A1–A60, then the open questions).
