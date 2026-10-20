# Layers: collapse all, and click a layer to focus it (Stage D)

**D0, 20 Oct 2026.** What the old TypeScript Scrim (`~/Scrim`) does, read from its code, not guessed. Then what scrim-rs has today, and what D1 and D2 will do. Port from behaviour, never code (AGENTS.md rule 7): nothing below is copied.

Old paths are under `~/Scrim/apps/studio/src/`.

## Collapse all, in the old Scrim

**The button:**
- `Layers.tsx:270`: an icon button, `ListIcon` (`Icons.tsx:81`: two long lines and a short one, `M2 3.5h10M2 7h10M2 10.5h6`).
- Its tooltip and `aria-label` read "Collapse all", or "Expand all" once everything is folded.
- **The icon to its right is not a code toggle.** It is `CollapseIcon`, "Hide layers (⌘⌥L)" (`Layers.tsx:271`), which hides the whole panel. The code view is ⌘/ (`App.tsx:329`).

**The state:** a component-local `useState<Set<NodeId>>` of folded nodes (`Layers.tsx:82`). It writes nothing to the document. The logic is `editor/shell.ts:162-187`:
- `collapsibleIds` is every node with children, at any depth;
- `toggleCollapseAll` folds all of them, or unfolds all when all are folded.

**Tests:**
- `editor/__tests__/shell.test.ts:44-62`:
  - lists only nodes with children, in tree order;
  - folds everything, then unfolds on the next toggle;
  - a partial fold counts as "not all", so the toggle folds the rest;
  - an empty document is never "all collapsed".
- `__tests__/layersVirtual.dom.test.tsx:258-274`: clicking it leaves only the root rows.
- `editor/__tests__/layers.test.ts:60`: a collapsed node hides its subtree.

**Spec:** `~/Scrim/docs/checklists/canvas-chrome.md:39`: "Collapse-all folds every parent, then becomes expand-all … the state persists while you work (UI state, not the document)."

| Question | Old behaviour | Where |
|---|---|---|
| Second press | Toggles: expands everything ("Expand all"). After a partial fold, it folds the rest. | `shell.ts:185-187`, `shell.test.ts:55-59` |
| Selection's ancestors | Collapsed too: the selection isn't read; a selected row inside a fold is hidden and not scrolled to. | `Layers.tsx:128-129` |
| Scroll | Not reset. A layout effect re-reveals the selected row only if it is still listed. | `Layers.tsx:138-141` |
| Pages | The old document had none (`schema.ts:297-314`): the whole document. | — |
| Persisted | No: React state. Hiding and re-showing the panel resets it to expanded. New parents appear expanded. | `Layers.tsx:82`, `App.tsx:386-406` |
| Shortcut | None. | `App.tsx:301-336`, `keymap.ts` |
| Filter active | The filter runs on the already-collapsed rows, so descendants of folded parents aren't searched. Untested. | `Layers.tsx:90-91` |
| Edge cases | Nothing selected: the same. An empty document: no change. Locked or hidden: the old model had no such fields. | `schema.ts:188-231` |

**The filter and badges, for comparison:**
- **The filter:** "Filter layers" with a boxed `/` hint (`Layers.tsx:275-291`); the `/` shortcut is `App.tsx:332-335`.
- **The badges** ("col", "col · 72") are a frame's layout direction and its **gap in px**, not a child count (`editor/layers.ts:140-171`). They show on hover or on the selected row only.

## Click a layer to focus it, in the old Scrim

**The handler:** `Layers.tsx:236-240` selects, then sends `requestCamera({kind: 'reveal', id})`. That reaches `useCameraNav.ts:148-160` and `revealCamera` (`cameraNav.ts:45-62`).

| Question | Old behaviour | Where |
|---|---|---|
| Animated or jump | Animated, 320 ms, ease-in-out cubic; zoom interpolated geometrically. Reduced motion jumps. | `cameraNav.ts:11-17, 71-86`; `useCameraNav.ts:33-35, 72-75` |
| Target zoom | Unchanged. | `cameraNav.ts:60` |
| Padding | 32 screen px. | `cameraNav.ts:12` |
| Bigger than the view | No zoom-out: on each axis where it doesn't fit, its leading (left or top) edge goes to the margin. | `cameraNav.ts:52`; `cameraNav.test.tsx:40-43` |
| Already visible | Fully inside the margin: no move. Partly: the least pan per axis. | `useCameraNav.ts:151-152`; `cameraNav.test.tsx:30-39` |
| Multi-select | Only the clicked row is framed. Shift extends; ⌘ isn't read. | `selection.ts:10-21` |
| Double-click | Renames inline. Its two clicks each reveal, so the camera moves too. | `LayerRowView.tsx:76` |
| Canvas clicks | Never move the camera. | `useCanvasPointer.ts:93` |
| Input during the flight | Not cancelled: the flight overwrites wheel and pan until it ends. | `useCameraNav.ts:77-82`; `CanvasView.tsx:283-302` |
| Hidden / other page | No such things in the old model. An unmeasured (culled) layer falls back to its nearest measured ancestor, then reveals again once measured. | `useCameraNav.ts:37-46, 153-158` |
| Keyboard | Rows aren't focusable; no key moves the camera. | `LayerRowView.tsx:57-121` |

No old test asserts that a row click reveals. The spec is `~/Scrim/docs/checklists/selection-movement.md:16`: "A layers-tree click brings the node on screen with the least camera move and no zoom change … with macOS 'Reduce motion' on it jumps instead."

## scrim-rs today

**The panel:** `crates/ui-egui/src/layers.rs` is 1051 lines, frozen at that length (`xtask/size-allowlist.txt`), so new code goes in new modules.

| | scrim-rs | Where |
|---|---|---|
| Collapse all | **Already there** as `layers.collapseAll` ("Collapse layers", Object menu, ⌥L). It closes every row except those leading to the selection. No header button. | `layers.rs:517-522`, `cmd/edit.rs:204-207`, `keymap.rs:217`, `catalog.rs:183` |
| Its side effects | No undo step and no agent change. **But it bumps the document revision**: `Session::after_command` bumps it for every command. | `lib.rs:980` |
| Expanded rows | `app.ui.expanded`: one set for every page, UI state only, not persisted. A new selection re-expands its ancestors once (`layers_seen_sel`). | `layers.rs:624-636` |
| Filter | Behind the header's search button (magnifier), live, Escape closes. It lists matches from the **whole page**, whatever is collapsed. No "/" shortcut, no boxed hint. | `layers.rs:570-596, 413-430` |
| Row badges | None. | — |
| Row click | Selects (Shift range, Shift/⌘ toggle, else `select.set`); double-click renames. **Nothing moves the camera.** `select.set` switches page without a fit. | `layers.rs:1012-1049`, `cmd/edit.rs:367-386` |
| Camera | `ViewState {cx, cy, zoom, vw, vh}` per document. No animation anywhere: every setter jumps. `view.zoomSelection` fits with a 64 px margin. | `engine/src/lib.rs:185-310`, `cmd/view.rs` |
| The fit race | The first canvas sizing re-fits the page (`canvas.rs:173-184`), overwriting any camera set before the canvas had its size. | — |
| Hidden layers | `Node.visible`; `world_bounds` ignores it, so a hidden layer still has bounds. | `doc/src/lib.rs:678-684` |
| Reduced motion | `FigApp::reduce_motion()` (Settings ▸ General ▸ Animations, or the system's setting). | `ui-egui/src/lib.rs:1161` |

## What D1 and D2 do

**D1, collapse all:**
- **The command:** `layers.collapse-all`, renamed from `layers.collapseAll`; still ⌥L, still in the Object menu, and an MCP route.
- **The button:** an icon button in the Layers header, left of the search button. The icon is our own: three lines, the last one short. Its tooltip is "Collapse all layers  ⌥L" and its AccessKit name "Collapse all layers"; Tab reaches it.
- **What it changes:** it closes every row, on every page. The selection doesn't change, and selected rows inside closed parents are simply not listed until opened by hand or selected again.
- **No document effects:** no undo step, no document write, no revision bump (a view-only command), and no "Agent: …" label.
- **The filter:** while a filter is active, the filtered list is unchanged (it lists matches from the whole page). Clearing the filter shows the collapsed tree.
- **Opening rows by hand** afterwards opens only the row asked for (Alt: its subtree), as today.
- **The header code** (label, search, the new button) moves out of the frozen `layers.rs` into `layers_header.rs`.

**D2, click a layer to focus it:**
- **The command:** `camera.focus-layer {id | ids}`, an MCP route. It moves the camera with the old reveal:
  - the zoom is kept and the margin is 32 px;
  - the camera doesn't move when the layer is fully in view, and otherwise moves the least on each axis;
  - a layer bigger than the view gets its leading edge at the margin.
- **The engine** sets the target at once, so agents and headless runs are deterministic. In the window, a 320 ms ease-in-out takes the camera from where it was to the target. It is a jump when Animations is off or the system asks for reduced motion.
- **Input cancels it:** if anything else moves the camera during the flight (scroll, pan, zoom, a drag), the flight stops where the person put it.
- **Never before the canvas has its size:** a focus asked for before the first sizing is applied after it.
- **Panel clicks only:** a row click (and Enter on a focused row) selects, then focuses the selection, the union when several are selected. Canvas clicks never move the camera.
- **Hidden layers** are focused anyway, and a toast says the layer is hidden.
- **A layer on another page:** that page is shown first.

## Where I disagree with the old behaviour (overrule any)

1. **The second press doesn't expand everything again.** The old button toggled; the brief asks for an idempotent collapse-all. Rows are opened by hand (Alt-click opens a subtree).
2. **The filter keeps searching collapsed rows.** In the old app a filter couldn't find a layer inside a folded parent, which reads as a bug. scrim-rs's filter already searches the whole page, and stays that way.
3. **Collapse all now closes the selection's ancestors too,** as in the old app and as the brief's "only the top-level frames" asks. scrim-rs used to keep them open; that was its own choice.
4. **Multi-select focuses the union of the selection.** The old app framed only the clicked row; the brief asks for the union.
5. **Input cancels the flight.** The old flight overwrote the person's own scroll and pan for 320 ms.
6. **The double-click that renames** also focuses, because its first click is a click. That is the same as the old app.
7. **Every page.** The expanded set is one set for all pages, so collapse-all closes rows on every page. The old document had no pages, so there was nothing to follow.

## D1 as built

- **The command:** `layers.collapse-all` (`crates/engine/src/cmd/layers.rs`), renamed from `layers.collapseAll` everywhere: keymap ⌥L, Object ▸ Collapse layers, every locale's command name, the access list. It is moved out of the frozen `cmd/edit.rs` (whose cap drops from 1390 to 1386).
- **No new revision:** `Session::after_command` now skips the revision bump for view-only commands (`cmd::view_only`, `layers::VIEW_ONLY`). It never made an undo step or a change-log entry, so agents get no "Agent: …" label.
- **The header** (`layers_header.rs`): the title, **Collapse all layers** (24 × 24, left of Search; the tooltip "Collapse all layers  ⌥L" names it and its key for screen readers; Tab reaches it, Space presses it; a focus ring), and Search with its row.
  - Moving it, the search field and `collapse_all` out took `layers.rs` from 1051 lines to 999, under the limit: its frozen entry is gone.
- **The icon** is drawn in the module (three lines, the last short): `icons.rs` is at its cap.
- **`collapse_all`** clears every open row (every page) and marks the current selection as seen, so it isn't revealed again until it changes.
- **The tests port the old suite where it still applies:**
  - `shell.test.ts`: a partial fold folds the rest; an empty document is fine.
  - `layersVirtual.dom.test.tsx`: only the top-level rows remain.
  - "Unfolds on the next toggle" is deliberately not ported: collapse-all is idempotent.

## D2 as built

- **The engine** (`cmd/camera.rs`): `camera.focus-layer {id | ids}`, which defaults to the selection, an MCP route.
  - **`reveal`** is the old `revealCamera`: the zoom is kept, a 32 px margin is left, the least move is made on each axis, a layer bigger than the view gets its leading edge at the margin, and a layer already in view moves nothing.
  - **The rest of the command:** the union of several layers; another page's layer shows its page first; hidden layers are focused and listed in `hidden`.
  - **The camera is set at once** and the window is asked to fly (`camera.fly` with where it came from). It is classified Write for Tool access.
- **The window** (`camera_flight.rs`):
  - **The flight:** 320 ms, ease-in-out cubic, the zoom eased geometrically, from where the camera was. It jumps with Animations off, reduced motion, or a page switch.
  - **Cancel on input:** each frame compares the view with what the flight last wrote. Any other change (wheel, pinch, space-drag, the hand tool, a zoom key) stops the flight there.
  - **The fit race:** a focus asked for before the canvas has its real size (when its first sizing re-fits the page) is kept and applied after that sizing.
- **The hook:** the Layers row click (`layers.rs`, one line, at 1000 of 1000) calls `focus_selection` after selecting. Enter on a keyboard-focused row is a click to egui, so it focuses too: `a11y.rs`, at its cap, needed no change. A hidden layer gets the toast "“Name” is hidden".
- **Canvas clicks** don't go through it: they never move the camera (tested).

## D3 results

Live in the release build; the captures, numbers and method are in `docs/scrim-ports/layers/README.md`.

- **Collapse all**, on Brand colour system mockups and the Fixture (rebuilt, as scrim-rs can't open the old `.scrim` file): every changed pixel is in the Layers rows, 0 outside them. A second press changes 0 px and the camera never moves.
- **A row click** (the nested Nav Log) flies from (2000, 1000) to (1159, 3196) at 50%. The Layers panel matches the same state set by commands (0 px). On the canvas, 29 anti-aliased edge pixels differ by at most 16/255. Moving the camera there in steps by command, without the flight, gives the same kind of difference. It comes from the renderer's frame-to-frame state, predates Stage D, and is out of its scope.
- **A scripted scroll** 0.127 s into the flight stops it at once (x stays at 1896.6, short of 1159). A second focus answers `moved: false` with identical pixels. Canvas clicks never moved the camera.
- **By keyboard only:** F6, 9 Tabs to the button, Space collapses, then Tab to the rows, arrows, and Enter flies.
- **Contrast:** the icon 9.33:1 (light) and 8.05:1 (dark) on the panel; the focus ring 5.43:1 and 6.01:1; the toast 16.7:1.

## H0: the old second press, checked again

- **The old Scrim's second press expanded everything; it did not restore.** `toggleCollapseAll` (`~/Scrim/apps/studio/src/editor/shell.ts:183-187`) returns an empty folded set once everything is folded, and `shell.test.ts:48-54` pins "folds everything, then unfolds everything on the next toggle". Rows folded by hand before the collapse came back open.
- **Stage H disagrees on purpose:** the second press restores the rows that were open before the collapse (a snapshot), as the brief asks, rather than opening every row. Expanding everything would undo the person's own folds; restoring returns exactly what they had.
