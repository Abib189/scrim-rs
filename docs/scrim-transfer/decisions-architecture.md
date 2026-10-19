# Scrim decisions digest, for the Rust fork of FigCraft (part 2 of 2)

Part 1 (`decisions.md`) has the purpose, the verdict legend, the FigCraft vocabulary, the summary table, the LOCKED decisions (D1–D4) and the hard rules (R1–R18). This part has every other principle and invariant from `ARCHITECTURE.md` and the design specs (A1–A60), then the **open questions for the human**.

Scrim paths are relative to the Scrim repo root. "The preview layer", "the registry" and "the one write path" mean A20, R17 and R1.

## Canvas and rendering

### A1. Measured geometry is ephemeral editor state
- **Source:** `ARCHITECTURE.md` › Decision 1 › "Note — measured rects are ephemeral editor state"; `packages/mcp/src/store.ts` (`RootGeometry`).
- **Verdict:** ADAPT. In Scrim the browser laid out and the studio reported rects to the server. In the fork the Rust layout pass produces them, so they are never "missing", but they stay outside the document: not in the `Document`, a command, the journal, a saved file or undo. They are a cache beside the document, invalidated by the same commits. The one exception is a root's canvas position, which agents write through a command.
- **Why it existed:** geometry is a function of the document, the fonts and the layout engine. Storing it makes two sources of truth that drift.
- **Test:** serialise a document before and after a layout pass and require identical bytes. Undo and redo never change the geometry cache directly.

### A2. Surviving large documents: culling, one camera transform, deferred commits
- **Source:** `ARCHITECTURE.md` › Decision 1 "How we survive" and "Note — stress results"; `docs/research/stress-test.md`, `docs/research/editor-frame-cost.md`.
- **Verdict:** ADAPT. Keep: off-screen nodes cost nothing, a camera move never re-runs layout, and a drag previews each frame and commits once on release.
  - Scrim's measured lesson: the cost was the editor's own work per frame, not drawing. Every layer row re-rendered, the whole document was linted, the document was deep-cloned per preview frame, and the whole tree re-rendered per camera step.
  - In egui, virtualise the layers panel and make lint incremental. `imbl` makes preview clones cheap.
  - Revisit trigger, re-stated: if real documents drop frames below about 2,000 visible nodes, add raster level of detail, as FigCraft already does.
- **Why it existed:** a design tool is judged by 60 fps pan, zoom and drag.
- **Test:** port the stress harness idea (`tools/stress/`): generated 2k–20k node documents, frame times per gesture, and a refusal to measure on an unfit machine (disk over 90%, swap over 4 GB, load over the core count).

### A3. A spatial index for hit-testing, marquee and snapping
- **Source:** `ARCHITECTURE.md` › Decision 1 bullets; `packages/engine/src/spatialIndex.ts`.
- **Verdict:** ADAPT. FigCraft hit-tests with a recursive walk and no index (`docs/research/figcraft-spike.md` §7). Add a flat grid or R-tree over resolved world bounds, rebuilt per layout pass, for hit, marquee, snap candidates and the distance chips (A39).
- **Why it existed:** a linear hit-test per pointer move does not scale to thousands of nodes.
- **Test:** the index answers equal a brute-force walk on random documents. A 20k-node hit-test benchmark.

### A4. Hit-testing follows what is visible
- **Source:** `ARCHITECTURE.md` › Decision 4 "Hand-drawn frames clip".
- **Verdict:** CARRY. Click, hover, marquee and drop targets use each node's rect cut by its clipping ancestors' content boxes. Selection outlines, resize handles (still grabbable outside the frame) and snap targets use the full rect.
- **Why it existed:** a clipped child that can be clicked where it isn't drawn feels broken.
- **Test:** a child half outside a clipping frame is hit inside and missed outside, while its handle outside the frame still grabs.

### A5. Export fidelity is measured against the canvas
- **Source:** `ARCHITECTURE.md` › Build sequence, benchmark obligation 5; `apps/desktop/scripts/export-fidelity.mjs`; `docs/research/openpencil-fork-decision.md` §1–2, §6.
- **Verdict:** ADAPT. With the DOM gone (D1), "the export looks like the canvas" is no longer free. Build a harness: render each fixture artboard with the fork's renderer, render its standalone HTML export headlessly in Chromium and WebKit with the network blocked, and diff pixels beyond anti-aliasing. Record the identical-artboard count and the worst difference, and fail CI when either gets worse. FigCraft matched 8 of 19 here, Scrim 15–16 of 20. Text (HarfRust against the browser's shaper) is the known drift.
- **Why it existed:** this was Scrim's thesis, that the design *is* the code. Without it the fork is "a Figma-model editor with a good exporter".
- **Test:** the harness, with committed thresholds that only tighten.

### A6. The export gate: pinned bytes
- **Source:** `ARCHITECTURE.md` › Motion foundation and Layout guides; `packages/mcp/src/__tests__/exportGate.test.ts` and `exportGate/*.txt`.
- **Verdict:** CARRY. A committed corpus covering every export feature has its exact export pinned. A new field must leave every document without it exporting byte for byte as before, and gets its own pin. Changing a pin needs a human's review (R2).
- **Why it existed:** additive features kept silently changing old exports.
- **Test:** the gate itself. Port the corpus, re-pin once in the fork, then pin.

## Document model

### A7. Additive optional fields; explicit migrations; refusals with reasons
- **Source:** `ARCHITECTURE.md` › Decision 2 "Note: optional `Node` fields" and "Lint configuration"; Divergence log item 6.
- **Verdict:** ADAPT. A new field is optional with a default (`#[serde(default, skip_serializing_if = …)]`), so older files load unchanged and nothing is migrated on load. A breaking change bumps the schema version with an explicit, forward-only migration. A malformed value is refused by the command with a sentence saying why, never clamped silently. A document validator runs on load and in tests.
- **Why it existed:** old documents must open forever, and an agent that writes a bad value needs the reason to fix it.
- **Test:** load every fixture from every past version. Per field, a malformed write is refused with its reason and leaves the history unchanged.

### A8. One home per value (shorthands, size in style, longhands)
- **Source:** `ARCHITECTURE.md` › Decision 2 "Note: shorthand and longhand on the canvas"; Decision 4 "Size lives in style", "One accessor for effective declarations", "Border and radius are stored as longhands only".
- **Verdict:** DROP. All four rules exist because the model held CSS: shorthands beside longhands, sizes in both `layout` and `style`, and React applying mixed style objects unreliably. A Figma-shaped Rust model has one typed field per value. The surviving principle: a value has exactly one home, and every reader goes through one accessor. If HTML import (A31) lands, its parser expands shorthands before writing.
- **Why it existed:** two homes for one value made edits silently shadowed, and the canvas fell out of step with `get_css` (20 of 28 agent edits to such pairs).

### A9. Lint configuration lives in the document
- **Source:** `ARCHITECTURE.md` › Decision 2 "Lint configuration"; MCP › "Lint profiles".
- **Verdict:** CARRY. Per-artboard profiles (`ui` by default, `expressive`) and reasoned ignores (rule, optional node, non-empty reason) are document data, changed only through one undoable command and never exported. Ignores naming deleted nodes are kept, so undoing the delete revives them. No profile may relax a contrast or geometry rule.
- **Why it existed:** a poster set in 300 px type on purpose is design intent. The next agent session has no memory of it unless the document says so.
- **Test:** an ignore without a reason is refused. A profile change is one undo step. The export gate is unchanged by lint data.

### A10. Duplication rule
- **Source:** `ARCHITECTURE.md` › Decision 2 "Duplication rule".
- **Verdict:** CARRY. Duplicating or pasting an instance makes another instance of the same component. Duplicating a master makes a new, independent component, never a silent alias. Pasting an instance into a document without its component brings the definition and master along. Every id in a copy is fresh, and the gesture is one undo step.
- **Why it existed:** a master aliased by copy would make edits to the "copy" change the original's instances.
- **Test:** duplicate a master, edit the copy, and the original's instances are unchanged. Every id in a pasted subtree is new.

### A11. Structural master edits refused while instances live
- **Source:** `ARCHITECTURE.md` › Decision 2 "Structural master edits"; `packages/core/src/components.ts` (`masterStructureRefusal`, `walkPairs`).
- **Verdict:** ADAPT. Scrim paired instance nodes with master nodes **by position**, so it refused inserts, deletes, reorders and reparents inside a master with live instances, on every write path, naming the count and how to detach. Its planned fix was keyed correspondence: each instance node records its master node's id. If FigCraft already keys instance children by source id, adopt that and drop the refusal; otherwise carry the refusal (open question 8).
- **Why it existed:** positional pairing desyncs on any structural edit, and the next propagation lands overrides on the wrong nodes.
- **Test:** insert a child into a master with two instances. Either it is refused with the reason, or both instances gain the child and every override stays on its node.

### A12. Images referenced by hash; portable files carry them
- **Source:** `ARCHITECTURE.md` › Decision 2 "Images in the document and in the file" and "Large portable files go through main".
- **Verdict:** ADAPT. The document holds a reference (`scrim-asset:<sha256>.<ext>` in Scrim), never image bytes. A file saved for sharing carries the images beside the document, and opening it restores them into the store with the import checks. In the fork that file is the `.figcraft` zip, with images as entries named by hash. "Large files go through main" drops (Electron); its cap stays (A13).
- **Why it existed:** image bytes in the document made every autosave rewrite megabytes and broke content-hash dedupe.
- **Test:** save-as then open elsewhere restores identical hashes. Autosave size does not grow with image count.

### A13. File caps checked from the size, before reading
- **Source:** `ARCHITECTURE.md` › Decision 2 "The cap: 200 MB"; Figma .fig import › Limits; Agents › The CLI.
- **Verdict:** CARRY. Every open (document, `.fig`, portable file, the CLI) checks the size from metadata before reading a byte, and the refusal names the limit. Scrim's caps: 200 MB per document file (about 1.2 GB of memory at worst); per image 25 MB, 16,384 px a side and 100 megapixels; 64 MB per import batch.
- **Why it existed:** reading first and refusing later is how a hostile or huge file takes the app down.
- **Test:** a file one byte over the cap is refused without being read (a sparse file or a fake file system).

### A14. Editor UI state is never document data
- **Source:** `ARCHITECTURE.md` › Editor input (modifiers); Build sequence (snapping as a UI preference); Vector tools (pen and path-edit stores); MCP › get_selection.
- **Verdict:** CARRY. The camera, selection, hover, active tool, modifiers, snapping options, guide visibility, a pen path in progress, path-edit mode and menu state are UI state: never in the document, a command, the journal or undo. Preferences persist in app settings, not the file. A modal dialog owns the keyboard: editor shortcuts skip keys aimed inside it.
- **Why it existed:** view state in the document makes undo move the camera, makes two people's files differ, and leaks into exports.
- **Test:** every UI action that changes only view state leaves the history length unchanged.

## Library, history and previews

### A15. One active document: switched, never merged
- **Source:** `ARCHITECTURE.md` › Decision 3 amendment of 2 Oct 2026; `packages/mcp/src/library/README.md`.
- **Verdict:** CARRY. The session holds exactly one open document; the library holds the rest on disk. Opening one flushes the current one, swaps document and history, and clears ephemeral state. Autosave writes the open document atomically on every commit, with `updatedAt`, `lastEditedBy` (the commit's author) and counts. Templates and renames are commands too (R1).
- **Why it existed:** people work on several designs, without giving up the one-authority rule (D3).
- **Test:** open A, edit, open B, and A's file holds the edit. Selection and geometry are cleared on the switch.

### A16. Per-document undo, capped
- **Source:** `ARCHITECTURE.md` › Decision 3 "Per-document undo, capped".
- **Verdict:** ADAPT. Each document keeps its history across switches, in an LRU of the 8 most recently used; the evicted one keeps its content on disk and loses its undo. FigCraft keeps 1,000 `Arc` snapshots for one document. Pick a memory budget across documents (open question 6).
- **Why it existed:** switching away and back should not lose undo, but memory is finite.
- **Test:** switch through nine documents: the first has no undo, the eighth still has its undo.

### A17. The epoch guard
- **Source:** `ARCHITECTURE.md` › Decision 3 "The epoch guard"; `packages/mcp/src/library/host.ts`, `packages/mcp/src/server.ts`.
- **Verdict:** ADAPT. The session holds a document epoch, bumped on every switch. Each MCP connection remembers the epoch of its last *content* read (outline, node, selection, code, screenshot, lint; listing documents does not count). Its next write, or `get_selection`, after a switch is refused **once** with "The open document changed to <name>. Re-read it with get_document_outline before writing." The retry proceeds. In the fork the check sits in the MCP layer, in front of `run_command`, `batch` and every write tool.
- **Why it existed:** an agent mid-task would otherwise write into a document it never read.
- **Test:** read, switch, write: refused once with that message, then the retry succeeds. A connection that never read is guarded the same way.

### A18. Library import dedupes by content hash
- **Source:** `ARCHITECTURE.md` › Decision 3 "Nothing is lost".
- **Verdict:** CARRY. Open, Open Recent, a file association and Import add a file only when its content hash (sha256 of the document minus its name) is new. Identical content opens the existing document, and a hint says so. Other content under a taken name becomes "Name (2)". Save As writes a copy. No file outside the library is mirrored.
- **Why it existed:** opening the same file twice made silent duplicates that drifted apart.
- **Test:** import a file twice and get one entry. A rename-only difference dedupes. Different content with the same name gets "(2)".

### A19. Thumbnails
- **Source:** `ARCHITECTURE.md` › Decision 3 "Thumbnails (desktop)".
- **Verdict:** ADAPT. The first artboard, at most 480 px on the long edge, rendered by the `get_screenshot` renderer (A46) without chrome. Made when a document is left and 8 s after its last commit, only if it changed, and never during an agent burst of more than 5 writes a second. In the fork, a tiny-skia render on a worker thread, not a hidden window.
- **Why it existed:** Home needs pictures, but rendering on every agent write would starve the editor.
- **Test:** 20 rapid commands make no thumbnail until the burst ends. An unchanged document is not re-rendered.

### A20. The preview layer: a local view, never a commit
- **Source:** `ARCHITECTURE.md` › Decision 3 "Live preview for inspector controls"; `apps/studio/src/editor/usePreview.ts`, `preview.ts`.
- **Verdict:** ADAPT. During a gesture or an inspector scrub, the UI shows the pending change applied to a scratch copy of the document (cheap with `imbl`).
  - The scratch copy never reaches history, the journal, MCP, a save, export, `get_screenshot` or thumbnails. MCP geometry comes from the committed document only.
  - The commit recomputes the change against the current document and executes it once. Another writer's commit rebases the preview; a preview whose node was deleted is dropped quietly; Escape cancels with no history entry.
  - Scrim's "hold the view until the server's echo" drops, because the fork's session is in-process.
  - Number fields scrub: drag the label, ⇧ ×10, ⌥ ×0.1.
- **Why it existed:** a 300 ms round trip made colour and number editing feel broken.
- **Test:** during a scrub, `inspect_document` over MCP returns the committed value. An agent command during a scrub rebases it. Escape leaves the history length unchanged.

### A21. Gesture previews use the commit's own function
- **Source:** `ARCHITECTURE.md` › Decision 3 "Canvas gestures preview through the same layer"; Editor input › "Draw tools preview the real shape".
- **Verdict:** CARRY. Each frame, the gesture computes the exact command the release will execute, with the same function (snapping, ⇧ ratio, rounding), and previews it. Duplicates take ids from a per-gesture stable source, so the previewed copy is the committed node. Snap, drop and hit inputs are frozen at the press, while chrome reads live geometry; otherwise a Hug parent that grows with the node becomes a moving snap target. A draw past 3 px previews the real node with its real defaults and clipping.
- **Why it existed:** a preview that differs from the commit makes the release jump.
- **Test:** for each registry entry, the last preview equals the committed document (R17's scenario test).

## Layout

### A22. Stored size is the visual size is the exported size
- **Source:** `ARCHITECTURE.md` › Decision 4 "Box-sizing: border-box".
- **Verdict:** ADAPT. Scrim rendered and exported `border-box`, so a padded frame's stored width was its visual and exported width. In the fork, define where strokes sit (inside, centre, outside) relative to the stored size, and make the exporter reproduce it (`box-sizing: border-box`, plus `outline` or `box-shadow` for outside strokes).
- **Why it existed:** CSS's content-box default made stored sizes diverge from what people saw for every padded element.
- **Test:** A5 harness fixtures with padded and stroked frames at each stroke alignment.

### A23. Movement never sizes, except the node that leaves
- **Source:** `ARCHITECTURE.md` › Decision 4 "Movement never sizes"; `apps/studio/src/editor/keepSize.ts`, `apps/studio/src/editor/__tests__/movementInvariant.test.ts`.
- **Verdict:** ADAPT. A move, nudge, reorder, align, distribute, group, ungroup, z-order change, tidy, paste or duplicate writes no sizing field on any existing node. A node that leaves its parent's layout (to the canvas, to absolute, or its parent switched to free) keeps the size it was drawn at: each axis its parent decided becomes Fixed at that size, and Fixed and Hug axes are untouched. Aligning a flow child sets its cross-axis self-alignment, not a size. In the fork, "sizing field" means the sizing modes and sizes of the layout intent (D4).
- **Why it existed:** a Fill card that collapsed when dragged out of a row visibly changed size because of a move.
- **Test:** port the invariant test: every movement command over Fill, stretched, Fixed and Hug children of Hug, Fixed and Fill frames.

### A24. Reparenting keeps the visual position, through one helper
- **Source:** `ARCHITECTURE.md` › Decision 4 "Reparenting keeps the visual position"; `packages/engine/src/layout/` (`reparentPatch`).
- **Verdict:** ADAPT. Every reparent (canvas drop, layers panel, paste into, group, ungroup, MCP `move_node`) is built by ONE engine function.
  - Into a free frame the node becomes absolute at its on-screen place; into an auto-layout frame it takes the slot nearest the drop; to the canvas it becomes a root at its place.
  - Between auto-layout frames a flow child keeps its sizing modes, and Fill is rewritten for the new axis. Leaving a grid drops grid placement (A25).
  - The pointer at release decides the parent, never the node's bounds. A root nests only when strictly smaller than the frame on both axes.
- **Why it existed:** two code paths gave two answers; a layers-panel drop once left a stale canvas position.
- **Test:** the same reparent through the UI and through MCP yields identical documents. Fill survives a row-to-column move.

### A25. Grid: placement, not order
- **Source:** `ARCHITECTURE.md` › Grid layout in the editor; `docs/research/grid-audit.md`.
- **Verdict:** ADAPT. Track geometry for the overlay comes from the layout pass, never recomputed by chrome. A cell drop writes the child's grid placement (start and span), never its size or its child order (R5). Templates are written back in idiomatic form (`repeat(n, x)`). Switching Flex to Grid keeps sizing modes. If the fork's model has no CSS-style grid, this waits on open question 2.
- **Why it existed:** grid placement and child order are different things in CSS, and conflating them broke paint and export order.
- **Test:** a cell drop leaves the children list unchanged. Leaving a grid clears placement in the same commit.

### A26. Layout guides: document data, never exported
- **Source:** `ARCHITECTURE.md` › Layout guides; `docs/research/layout-guides.md`.
- **Verdict:** CARRY. Column, row and square-grid guides on frames are saved in the document as one whole-list LWW value, mirrored from master to instances, edited through one command. No exporter reads them, and they never appear in screenshots or thumbnails. They draw from the frame's resolved bounds, under all other chrome, and capture no pointer. Direct children snap to band edges and grid lines, never band centres, with an option to turn that off.
- **Why it existed:** guides are part of the document a designer reopens, but not part of the product.
- **Test:** the export gate with guides added to every frame stays byte-identical.

## Vectors and import

### A27. Vector content is structured data behind an allow-list
- **Source:** `ARCHITECTURE.md` › Inline SVG on a node; `packages/core/src/svg.ts` (`svgIssues`).
- **Verdict:** ADAPT. FigCraft has its own vector model, so Scrim's field drops. What carries: vector content is numbers and enums, never stored markup; SVG markup is generated by one function for export and `get_svg`; every write path (commands, paste, import, plugins) checks the allow-list and caps (200,000 characters per path, 1,000,000 per node, 5,000 elements, depth 16, numbers within ±10⁷); gradient and clip ids are scoped per node on export; paints are hex, `rgb()`, `none`, `currentColor` or a local gradient.
- **Why it existed:** with nothing stored as markup there is nothing to sanitise on output, and two icons' gradients never collide on one page.
- **Test:** a refusal test per disallowed element, key and paint, on each write path. Export two nodes sharing a gradient name.

### A28. Drawn vectors: the box is derived from the geometry
- **Source:** `ARCHITECTURE.md` › Vector tools; `docs/design/vector-tools.md` D1–D4, D16, §3 "Movement and size".
- **Verdict:** ADAPT. A drawn line or path stores only its geometry. Its box is the geometry's bounds padded by half the stroke weight; a horizontal line's thin side is its weight, never 0. Every geometry edit (end point, anchor, handle, weight, closing) writes geometry and box in one command. Movement writes position only, and geometry edits are not movements. Strokes don't scale on resize (`non-scaling-stroke`). Nothing tool-specific is stored: "a line" is recognised by shape. The path math is in-house (parse, arcs to cubics, bounds, de Casteljau, RDP, a Schneider fit). FigCraft has vector networks, so Scrim's single-path restriction drops.
- **Why it existed:** one source of truth for geometry, rendered the same everywhere with no tool state.
- **Test:** every geometry-edit command leaves box = derived(geometry). Moving a vector leaves its geometry unchanged.

### A29. Vector hit band and pencil fit
- **Source:** `docs/design/vector-tools.md` D9, D10; `ARCHITECTURE.md` › Vector tools "Hit testing is by geometry".
- **Verdict:** CARRY. A line or path is hit within max(3 screen px, half its weight), so a thin line's band is at least 6 screen px at any zoom, and a diagonal line's empty corners fall through. The pencil smooths every frame with the commit's own function: RDP 0.8 screen px, a fit within 1 px, corners split over 75°, tolerances doubled until the path fits its cap. A stroke under 6 screen px creates nothing.
- **Why it existed:** a thin line must be easy to grab, and the release must not jump.
- **Test:** a hit 3 screen px from a 1 px line at 0.25× and 4×. The pencil preview equals the committed path.

### A30. `.fig` import: own decoder, limits first, one commit, a report
- **Source:** `ARCHITECTURE.md` › Figma .fig import; `packages/fig-import/README.md`; `docs/research/fig-fidelity.md`.
- **Verdict:** ADAPT. FigCraft already reads `.fig`; carry Scrim's guards into its reader.
  - The Kiwi schema the file embeds is interpreted, never compiled or evaluated.
  - Limits before the work they guard: file size, entry count, each entry's and the total decompressed size (output caps), entry paths (no `..`, absolute, backslash or NUL), nesting depth, array lengths against the bytes left, no trailing bytes.
  - Embedded images must match their declared hash, then go through the asset import (R18).
  - The import is ONE commit into a new document. A fidelity report (exact, approximated, skipped, with reasons) lives beside the document, never in it, and agents can read it. Pages follow the fork's model; Scrim stacked them as bands.
- **Why it existed:** a `.fig` is untrusted input, and an honest report is what lets a person trust a lossy import.
- **Test:** port `packages/fig-import/src/__tests__/security.test.ts` (bombs, traversal, lying headers). An import plus one undo leaves an empty document.

### A31. HTML import posture (designed, not built)
- **Source:** `docs/design/html-import.md`.
- **Verdict:** ADAPT. Scrim's design: parse inert, with no browsing context; walk an element and attribute allow-list, dropping everything else, never sanitising in place; never fetch (remote `url()`, `src`, `@import` are dropped and reported); images through the asset import; caps on input size, node count and depth before any work; one commit and a report with counts. In Rust the parser is a dependency (`html5ever` or similar), under R3. The model mapping depends on open question 2.
- **Why it existed:** HTML is a program, not a picture.
- **Test:** the spec's hostile corpus (scripts, handlers, `javascript:`, remote `url()`, a 6 MB paste, 100k nodes, depth 500), with zero network requests.

## Imported images (detail behind R18)

### A32. Decode and image-end checks before storing
- **Source:** `ARCHITECTURE.md` › Imported images "A real decode before storing"; `packages/mcp/src/assets/imageEnd.ts`, `importer.ts`.
- **Verdict:** ADAPT. The structure walk carries as-is: PNG IEND, JPEG EOI past every scan by segment lengths, GIF trailer, WebP RIFF size, never searching for a marker. A truncated file is refused; bytes after the end are cut off, and the trimmed bytes are what is stored and hashed. The decode check (decode to exactly the header's size, with a timeout and at most 2 at once) used Chromium's codecs; in Rust it needs decoder crates (open question 9), off the UI thread.
- **Why it existed:** a phone's motion photo is a valid JPEG with video appended, and a polyglot's ZIP must never reach the store.
- **Test:** port the encoder-made fixtures in `packages/mcp/src/__tests__/fixtures/trailing/` and `assetTrailing.test.ts`.

### A33. Background verification only records
- **Source:** `ARCHITECTURE.md` › Imported images "Images already in the library are checked too"; `packages/mcp/src/assets/verify.ts`.
- **Verdict:** CARRY. One file at a time, 50 ms apart, from 3 s after launch, the open document's images first. It writes only `verifiedAt` and a `damaged` reason to the index, and never modifies, moves or deletes a file. A timeout is not recorded and retries next launch. Damage shows as editor chrome, never in a render.
- **Why it existed:** older files were never checked, and a checker that "fixes" files destroys evidence and hashes.
- **Test:** after a pass every stored file keeps its bytes and mtime, and damaged files carry their reasons.

### A34. Display previews and decode-before-swap
- **Source:** `ARCHITECTURE.md` › Imported images "Display previews" and "Decode before swap"; `apps/studio/src/editor/decodeQueue.ts`.
- **Verdict:** ADAPT. A raster over 1024 px gets an upright 1024 px preview at import. The canvas draws the preview while it is big enough, else the original. The next image is decoded before it is swapped in, at most 2 at once, with a timeout, cancelled when the zoom no longer wants it. `get_screenshot`, export and save always use the original. In the fork: mip levels or a decode cache for tiny-skia.
- **Why it existed:** decoding 12-megapixel photos on every zoom step froze the editor (frames up to 1.5 s).
- **Test:** a screenshot uses the original's pixels. A broken original keeps its preview and retries at most twice.

### A35. Orientation follows what the renderers draw
- **Source:** `ARCHITECTURE.md` › Imported images "Orientation"; `packages/mcp/src/assets/exif.ts`, `heif.ts`.
- **Verdict:** ADAPT. Scrim matched Chromium exactly (JPEG APP1, PNG eXIf before IDAT, AVIF `irot`/`imir`, never WebP EXIF). The fork has two renderers, its canvas and the browser showing the export. The index's width and height, and the canvas, must match what the browser draws, or A5 fails.
- **Why it existed:** a phone photo on its side was placed landscape.
- **Test:** the eight EXIF orientations per format, canvas against exported page, in the A5 harness.

### A36. Location privacy
- **Source:** `ARCHITECTURE.md` › Imported images "Location privacy"; `packages/mcp/src/assets/location.ts`.
- **Verdict:** CARRY. By default an import removes the GPS IFD and EXIF thumbnail in place, at the same length, without re-encoding; orientation and other tags stay. What can't be stripped safely (XMP, for example) is imported unchanged and reported. A setting keeps location data. Exports of located photos warn with the count, Cancel is the default, and "remove location data and save" is offered. A command re-imports located images stripped and repoints the document in one commit.
- **Why it existed:** a design sent as a file or page would otherwise carry where each photo was taken.
- **Test:** port `location.ts`'s tests: stripped files sniff the same, offsets are unchanged, a PNG CRC is recomputed.

### A37. Unused images: mark, verify, grace, delete
- **Source:** `ARCHITECTURE.md` › Imported images "Unused images, with a safety net"; `packages/mcp/src/assets/references.ts`.
- **Verdict:** CARRY. *Mark:* a read-only scan collects references from every library document (trash included) and every undo and redo history in memory, cached by content hash. *Verify and move:* Clean up asks, scans again at that moment, and moves unreferenced files to an asset trash. *Grace:* anything that references a file again (open, import, undo, an agent) restores it. *Delete:* only Empty asset trash, or 30 days in the trash.
- **Why it existed:** a store that only grows fills the disk; one that deletes on a guess loses the photo an undo still needs.
- **Test:** a file referenced only by an undo step survives Clean up. A trashed file returns when undo references it.

## Editor input and chrome

### A38. Modifier conventions while dragging
- **Source:** `ARCHITECTURE.md` › Editor input: modifier keys while dragging.
- **Verdict:** CARRY. ⇧ locks to the dominant axis of the total delta, the other axis exact and never snapped. ⌥ held at release duplicates, the copy landing at the drop through the reparent helper. ⌘ inverts snapping for the gesture. ⌥ with no button down measures to the hovered layer. Modifiers are read live, so a change mid-drag updates the preview without moving the mouse. A pill names what is active, as chrome only.
- **Why it existed:** these are the conventions designers bring from Figma and Sketch. An earlier ⌥-inverts-snapping collided with ⌥-drag duplicate.
- **Test:** headless drags with each modifier, pressed and released mid-gesture.

### A39. Measurement chrome shows what will commit
- **Source:** `ARCHITECTURE.md` › Editor input "Distances while moving" and "Measurement beyond moves"; `packages/engine/src/measure/`.
- **Verdict:** CARRY. Port the pure measurement engine: distances to the nearest overlapping neighbours (else the parent's content edge), equal-gap detection and snapping limited to what is in view, and a resize measuring only the edges it moves. Chips measure the rect the release will commit, after snapping and rounding. Chrome only, never in the document or any output.
- **Why it existed:** a distance chip that differs from the result teaches people not to trust it.
- **Test:** for random moves, the chip values equal the committed geometry within 0.01.

### A40. One code path per action
- **Source:** `ARCHITECTURE.md` › Editor input "Right-click context menu".
- **Verdict:** ADAPT. FigCraft already generates menus from its catalogue and tests 212 of 212. Keep that: every menu row, palette entry and shortcut dispatches the same command id through `Session::execute`. Opening a menu writes nothing.
- **Why it existed:** two paths to one action drift until the menu and the shortcut do different things.
- **Test:** FigCraft's menu-catalogue parity test, extended to the shortcut table and the palette.

## Agents and MCP

### A41. Agents never manage documents or touch the file system
- **Source:** `ARCHITECTURE.md` › Decision 3 "The epoch guard" (last sentence); Imported images "Agents"; `docs/design/parity.md` › "Why one side is empty on purpose".
- **Verdict:** ADAPT. Agents can list documents and read the open one, but never open, create, switch, rename or delete one: switching files is the person's decision. No agent tool reads a path, fetches a URL or imports a file; agents use only images already in the store, by a listed hash. Agents never use the system clipboard. FigCraft's MCP has `open_file` and `save_file` (confined to automation roots) and `pointer`, `key` and `type_text`, which drive the UI as a person would: remove them or gate them off by default (open question 4).
- **Why it existed:** the person's files and screen are the person's. An agent that can open files or click can do anything the person can, unseen.
- **Test:** the MCP tool list has no path-taking or input-injecting tool. A write naming an unlisted asset hash is refused.

### A42. Agents fix forward; they do not undo
- **Source:** `docs/design/parity.md` › "Undo and redo"; `ARCHITECTURE.md` › Decision 2 Rules.
- **Verdict:** ADAPT. The history is shared, so an agent's undo could revert the person's edit. FigCraft's MCP exposes `undo` and `redo`: remove them, or restrict them to the caller's own last step (open question 4). An agent's writes stay undoable by the person.
- **Why it existed:** an agent cannot see what the person did between its own steps.
- **Test:** no MCP route reaches undo; or, if restricted, an agent undo after a person's edit is refused.

### A43. MCP write-tool rules
- **Source:** `ARCHITECTURE.md` › The MCP server › "Design rules".
- **Verdict:** ADAPT. Every write returns the resulting node state (read-after-write) and is one command on the shared history. Destructive batch operations (deleting a node with children) need `confirm: true`. Every refusal is a sentence saying why and what to do. Tool descriptions are versioned in the repo and carry the read-after-write and lint instructions, because a new session has no memory. Per-tool rate limits were planned and never built. FigCraft's `batch` with step references is the batch Scrim lacked.
- **Why it existed:** agents act on what tools return, and descriptions are their only memory.
- **Test:** every write tool's result includes the node. A destructive delete without `confirm` is refused.

### A44. Agents place by real geometry, never by guesses
- **Source:** `ARCHITECTURE.md` › The MCP server › "Planned, in order" (1); Decision 4 "Reparenting" (last sentences).
- **Verdict:** ADAPT. Outline and node reads carry resolved world rects. Create, move and instantiate take `place: { relativeTo, side, gap }`, resolved by the engine. `tidy_roots` lays out roots in one commit, and lint flags `root-overlap`. In Scrim a missing or stale rect made the tool answer "retry", never guess. In the fork layout is synchronous, but text measured in an unloaded font is still provisional (A50).
- **Why it existed:** the first gate run placed ten artboards blind and they overlapped.
- **Test:** `place` below a root yields no overlap. `root-overlap` fires on overlapping roots.

### A45. `get_selection`: the person's selection as shared context
- **Source:** `ARCHITECTURE.md` › The MCP server › "get_selection".
- **Verdict:** ADAPT. The UI's selection is ephemeral (A14) and readable by agents: each node's id, name, type, parent, artboard, rect and a one-line summary. "No UI attached" is an error and "nothing selected" says so; neither is an empty list posing as an answer. Every node-taking tool's description says to call it when the user says "this" or "it". FigCraft's `select` tool, which *sets* the selection, is a judgement call (open question 4).
- **Why it existed:** "make this tighter" only works if the agent can see what *this* is.
- **Test:** the three answers: no UI, nothing selected, a selection.

### A46. `get_screenshot` from the canvas renderer, without chrome
- **Source:** `ARCHITECTURE.md` › The MCP server › "get_screenshot".
- **Verdict:** ADAPT. One node alone, at true size, scale 1 or 2, capped at 1,600 px on the long edge, as PNG. Same renderer as the canvas, no chrome, no theme dependence. It never touches the person's camera, selection or document; renders are serialised with a timeout. In the fork, a tiny-skia render of the committed document, which also works headless (Scrim's was desktop-only). Scrim's known gap was ignoring colour tokens; don't repeat it.
- **Why it existed:** once agents could place by geometry, the blind spot was seeing the result. A headline lost in a same-colour shape passes every rect check.
- **Test:** the screenshot is identical in light and dark themes and with or without a selection. Token colours render.

### A47. Lint for agents
- **Source:** `ARCHITECTURE.md` › The MCP server › "Lint profiles"; Build sequence › Design intelligence; `packages/engine/design-rules/VENDORED.md`; `packages/engine/src/design/lint.ts`.
- **Verdict:** ADAPT. Port the 22 ported Impeccable rules and the motion rules to Rust as data plus pure functions, keeping the vendored files with their Apache-2.0 licence and NOTICE. `lint_design` returns the profile used, counted findings, informational findings and reasoned ignores separately. Every content-writing tool tells the agent to lint before reporting done. Lint is incremental or off the frame path (A2).
- **Why it existed:** agents produce generic "slop" unless something checks them. FigCraft has no lint.
- **Test:** port the rule tests and the lint gate (documents without motion lint unchanged).

### A48. Attribution and presence
- **Source:** `ARCHITECTURE.md` › The MCP server › "Attribution + presence".
- **Verdict:** ADAPT. Each MCP connection's writes are attributed to its `initialize` client name, never a session id. The journal entry and change notification carry `actor: agent(client)` or `person`. A presence list (client, connected at, last seen) lets the UI show who is connected. FigCraft's journal is the natural home for the actor.
- **Why it existed:** the person must see what their agent did, and tell its history entries from their own.
- **Test:** an MCP write produces a journal entry with that client's name and no session id.

### A49. MCP security baseline
- **Source:** `ARCHITECTURE.md` › The MCP server › "Security baseline"; Divergence log item 5.
- **Verdict:** ADAPT. Stdio MCP has no network surface, so loopback binding and Origin checks apply only to listeners. FigCraft's `--attach PORT` control channel is one: bind 127.0.0.1, require a per-launch token, check Origin and Host, off by default (open question 5). An in-app audit log of every agent mutation carries; Scrim never persisted its log.
- **Why it existed:** an unauthenticated localhost server is a remote-code-execution surface (the MCP Inspector CVE-2025-49596 lesson).
- **Test:** the attach port refuses a connection without the token or with a foreign Origin.

### A50. Agents are told when fonts are not loaded
- **Source:** `ARCHITECTURE.md` › Agent Dock › "Fonts and agents".
- **Verdict:** ADAPT. The outline carries `fontsLoaded` (bundled, cached, not loaded, with the families), and placement tools tell agents to check it: text is measured in a fallback font and sizes change once fonts load. In the fork, read it from the font registry the Rust shaper uses.
- **Why it existed:** agents placed artboards from sizes that changed seconds later.
- **Test:** a document using an uncached family reports it as not loaded.

### A51. The read-only query subset
- **Source:** `ARCHITECTURE.md` › Agents: query, analyze, the CLI and parity; `packages/core/src/query/README.md`.
- **Verdict:** CARRY. A hand-written XPath-style subset with no dependency: `/` and `//`, node-type tests, comparisons, `contains()`, `starts-with()`, `and`, `or`, `not()`, `[n]` and `[last()]`. Attributes are document fields as declared; rects come back in results, never in predicates. Anything outside the subset is an error naming what is supported, never an empty result. Queries are linear. Results are capped (100 by default, 1,000 at most) with the total.
- **Why it existed:** agents need to find nodes without reading the whole tree.
- **Test:** port the query and error-message tests, plus a 20k-node timing.

### A52. `analyze_design`
- **Source:** `ARCHITECTURE.md` › Agents; `packages/engine/src/design/analyze.ts`.
- **Verdict:** CARRY. Read-only. It reports against the document's own tokens: colours matched exactly or within CIE76 ΔE 3, with near-duplicate clusters; type styles; off-scale spacing (against spacing tokens, or a 4 px grid); and repeated subtree shapes as component candidates.
- **Why it existed:** consolidating a design is the step agents skipped.
- **Test:** port the analysis tests.

### A53. The CLI answers through the MCP read tools
- **Source:** `ARCHITECTURE.md` › Agents › "The CLI"; `packages/cli/README.md`.
- **Verdict:** ADAPT. FigCraft's CLI runs an in-process session. Keep Scrim's two locks for read commands: the store refuses every write, and only an allow-list of read tools can be called. `--json` output is the tool's output byte for byte. It never touches the library or app data and never fetches. Exit codes: 0 done, 1 refused, 2 usage, 3 the file. FigCraft's `run` of command scripts is a write path, kept apart from the read commands.
- **Why it existed:** one code path means the CLI and the agent never disagree.
- **Test:** each read command's `--json` equals the MCP tool's result. A write through the read commands is refused.

### A54. The parity rule
- **Source:** `ARCHITECTURE.md` › Agents › "The parity rule"; `packages/mcp/src/__tests__/parity.test.ts`; `docs/design/parity.md` (generated).
- **Verdict:** ADAPT. A test reads every surface from its source of truth (gesture registry, keymap, menus, inspector sections, file and library commands, the command catalogue, MCP tools, CLI commands) and maps each to a capability. It fails on anything unmapped or stale, a one-sided capability without a reason or a listed known gap, and a closed gap still listed, and it generates the parity document. In the fork the 562-command catalogue is the main input. Scrim's open gaps (tokens, duplicate, group, remove-definition, document properties, screenshot tokens) are a list to re-check.
- **Why it existed:** "agents are first-class" decays one feature at a time unless a test makes every new capability answer for agents.
- **Test:** the parity test itself.

## Motion

### A55. Motion: the settled state, an inert canvas, one cascade
- **Source:** `docs/design/motion.md` §2.1–2.3; `ARCHITECTURE.md` › Motion foundation.
- **Verdict:** ADAPT. The CSS forms drop (states as style deltas, motion as CSS longhands, export classes); FigCraft has its own prototyping. What carries:
  - the canvas, geometry, hit-testing, screenshots and thumbnails show a node's settled base appearance, never a frame mid-transition, and nothing a design declares moves in the editor except an explicit preview;
  - one fixed state order (disabled last), shared by the canvas preview and the export;
  - every movement the editor creates gets its reduced-motion variant in the same commit;
  - `as` exports a real `<button>` or `<a>`, never a `div` with `tabindex`; `href` allows https, mailto, `#id` or a relative path.
- **Why it existed:** geometry and screenshots must be deterministic, and exported states must be what a frontend developer would write.
- **Test:** a screenshot of a node with a transition equals the settled one. A state fixture's export is pinned (A6).

### A56. Editor-only data is fenced; export names are derived
- **Source:** `docs/design/motion.md` §2.3 points 8–9, §5.1.
- **Verdict:** CARRY. Data that doesn't export as plain markup or CSS (prototype flows) lives in one document-level field keyed by node id, never on the node, so stripping it from an export is one field. Export identifiers such as class names are derived from layer names in document order (`cta`, `cta-2`), never stored, so codegen stays deterministic.
- **Why it existed:** node data stays exportable, and the same document always produces the same output.
- **Test:** exporting with and without prototype data gives the same bytes. Exporting twice gives identical bytes.

## Process

### A57. LOCKED decisions stop a session; the Divergence log
- **Source:** `ARCHITECTURE.md` › header, Working agreement, Divergence log.
- **Verdict:** CARRY. The fork writes its own LOCKED decisions, starting from D2–D4 as adapted here. A session that would violate one stops and asks. Where code differs from a locked decision's wording, record it in a Divergence log for the human; never edit the decision to match the code. Scrim's standing interpretations were item 4 (DOM chrome outside the design subtree) and item 5 (unshipped MCP security pieces).
- **Why it existed:** most code was written by agent sessions, which otherwise rewrite the architecture one convenient change at a time.
- **Test:** none automated; it is a review rule.

### A58. Deterministic tests
- **Source:** `CLAUDE.md` › Workflow "Tests stay deterministic".
- **Verdict:** ADAPT. Bind port 0. Never sleep a fixed time: advance a fake clock or await a condition. Each test owns its state (a temp dir, a fresh session), so the suite passes in any order; run it shuffled (for example `cargo nextest`, if the dependency log accepts it). Make slow tests faster rather than raising timeouts, and never add retries. A UI test fails on a warning it doesn't expect.
- **Why it existed:** flaky tests teach agents to ignore red.
- **Test:** a periodic shuffled, repeated run (Scrim's `pnpm test:repeat`).

### A59. README stub first; manual checks name their test
- **Source:** `CLAUDE.md` › Workflow; `docs/checklists/README.md`.
- **Verdict:** CARRY. Before code for a new module, write its README: purpose, public API, what it must never do. Manual behaviour lands in an area checklist in the same change, each row naming its test. Work in small verifiable steps: types, tests, implementation, wire-up.
- **Why it existed:** the "never" list is what the next session needs and can't infer from code.
- **Test:** none automated; it is a review rule.

### A60. v1 non-goals
- **Source:** `ARCHITECTURE.md` › What Scrim is "Non-goals (v1)" and its amendments.
- **Verdict:** ADAPT. Scrim's v1 non-goals were real-time multiplayer, booleans and multi-branch vector networks, plugins, mobile, and prototype flows with Present mode. FigCraft already ships several (a wasmi plugin sandbox, vector networks, prototyping, branches and history). The human decides which stay off in the fork's first scope (open question 12).
- **Why it existed:** to keep Stage 1 on editor craft and MCP depth.
- **Test:** none; it is a scope decision.

---

## Open questions for the human

Places where the right Rust form is a judgement call. Each names the entries it affects.

1. **What replaces Decision 1's promise?** (D1, A5, A35) Is "the exported HTML renders like the canvas" a measured target with a CI threshold? Which renderer is the reference when they disagree: the fork's tiny-skia canvas, or Chromium rendering the export? Text shaping (HarfRust against the browser) is the known drift.
2. **How CSS-shaped should the fork's model be?** (D4, R4, A8, A25, A31) Keep FigCraft's Figma-shaped auto layout with an exporter that maps it to flex and grid, or move the layout intent towards CSS semantics (Fill = `flex: 1 1 0`, real CSS grid, percentages)? This decides whether rule 6 holds by construction or by mapping.
3. **The mutable accessor.** (R1) Confirm the real name and visibility of FigCraft's direct document write, and remove the one UI call site the spike found.
4. **FigCraft's agent tools that Scrim forbade.** (A41, A42, A45) `open_file`, `save_file`, `undo`, `redo`, `select`, `pointer`, `key` and `type_text`: remove each, gate it off by default, or keep it with a confirmation?
5. **The `--attach PORT` control channel.** (A49, R14) Remove it, or keep it behind a per-launch token, Origin and Host checks, off by default?
6. **Undo memory.** (A16) FigCraft keeps 1,000 snapshots per document; Scrim kept histories for 8 documents. What budget, in steps or bytes, across documents?
7. **The 500-line rule in Rust.** (R13) New and changed files only, with FigCraft's large files frozen in a shrinking allow-list? Or split them up front?
8. **Component correspondence.** (A11) Does FigCraft map instance children to master children by source id? If yes, drop the structural-edit refusal; if no, carry it.
9. **Image decoders.** (A32, A34, R3) Which Rust decoder crates are acceptable, in which crate layer? Must the decode check stay as strict (exact header size, lying headers refused)?
10. **Typed command parameters.** (R8) FigCraft's 562 commands take free-text JSON. Type them all up front, as they are touched, or only those MCP exposes?
11. **Network clients FigCraft ships.** (R12) The collab host (`collab.craftapps.dev`), `craft-fonts` and the release workflow: disabled until a cloud stage? Is the Google Fonts fetch still wanted, or are fonts bundled only?
12. **Scope of the first fork milestone.** (A60, R14, A55) Which of FigCraft's plugins, prototyping, branches and collab stay on? Is the Agent Dock (embedded terminal) ported?
