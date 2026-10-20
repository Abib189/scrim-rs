# Stage D verification (D3, 20 Oct 2026)

Real window captures from the release build (`ui.screenshot`, the window's own pixels at 2×: 1710 points wide), a scratch HOME, two files open: **Brand colour system mockups** and **Fixture**. The plan and what was built: `docs/design/layers-collapse-and-focus.md`; the checklist: `docs/checklists/layers.md`.

**The Fixture file is a rebuild.** The owner's Fixture is an old TypeScript Scrim `.scrim` document (782 layers, 18 top-level frames: Poster, Quorum, Rootstock, Kiln Ledger and the rest), which scrim-rs can't open. A script rebuilt it as a scrim-rs file with the same tree, names and positions (782 layers), without its styling. The Layers panel is what's under test, and it shows the same rows.

Pixel diffs compare whole captures. The Layers panel ends at x = 592 px (296 points); "outside" means everything to the right of it, the canvas included. The pointer was parked over the Inspector for every comparison, because a pointer over the canvas draws a hover outline.

## Collapse all layers

| File | Captures | Before → after | After → a second press | Camera |
|---|---|---|---|---|
| Brand colour system mockups | `brand-01-before-collapse.png`, `brand-02-after-collapse.png`, `brand-03-after-second-press.png` | 23,029 px differ, all in the Layers rows (box 146–511 × 548–2119 px); **0 outside** | **0 px** | unchanged (4570, 1222, 8.49%) |
| Fixture | `fixture-01-before-collapse.png`, `fixture-02-after-collapse.png`, `fixture-03-after-second-press.png` | 35,309 px differ, all in the Layers rows (box 143–458 × 494–2119 px); **0 outside** | **0 px** | unchanged (2901, 6867, 7.97%) |

The header itself doesn't change: the button has no pressed state.

## Click a layer: the camera flies to it

On the Fixture, a click on the nested row **Nav Log** (Kiln Ledger — Landing ▸ Header ▸ Nav ▸ Nav Log), from a camera at (2000, 1000), 50%:

| Capture | Camera |
|---|---|
| `camera-01-before-click.png` | (2000, 1000), 50% |
| `camera-02-mid-flight.png` (taken during the 320 ms flight) | (1422.6, 2507.8), 50% |
| `camera-03-after-click.png`: Nav Log selected, its row highlighted | (1159, 3196), 50%: the zoom is kept |

**The only canvas change is the camera move.** The same state reached by commands (`select.set` Nav Log, `view.set` to the same camera) gives the same Layers panel: 0 px differ left of x = 592. On the canvas, 29 px differ, each an anti-aliased edge pixel at most 16/255 off. That isn't the flight: moving the camera to the same place in 20 `view.set` steps, with no flight, gives 33 such pixels. A single `view.set` jump gives 0. The renderer keeps state from frame to frame (`render_worker`'s `Retained`), so the edge pixels depend on the path the camera took. That predates Stage D and is out of its scope.

**Input cancels the flight** (`camera-04-cancelled-by-scroll.png`). A scripted wheel scroll over the canvas 0.127 s into the flight, the camera sampled every 0.1 s:

| Time after the click | Camera |
|---|---|
| 0.111 s, before the scroll | (1969.7, 1079.2) |
| 0.232 s | (1896.6, 1486.2) |
| 0.344 s | (1896.6, 1508.4) |
| 0.466 s to 1.424 s | (1896.6, 1510.0), unchanged |

x stops at 1896.6, short of 1159, the moment the scroll lands. y moves only by the scroll, which egui eases over a few frames. The camera stays where the scroll put it.

**Idempotent.** `camera.focus-layer {id: Nav Log}` twice: the first answers `moved: true`, the second `moved: false`. The camera is unchanged and the two captures are identical (0 px).

**Canvas clicks don't move the camera.** A click on empty canvas and a click on a layer left the camera at (1159, 3196).

## By keyboard only

From the canvas, with nothing selected and the camera at (2000, 1000):

| Step | Capture | Result |
|---|---|---|
| Start | `kb-01-start.png` | the tree open (Kiln Ledger — Landing to Nav) |
| F6 until the Layers region | `kb-02-layers-region.png` | |
| Tab (9 presses) to **Collapse all layers** | `kb-03-collapse-button-focused.png` | the focus ring on the button |
| Space | `kb-04-collapsed.png` | only the top-level rows; the canvas unchanged (0 px right of the panel); the camera unchanged |
| Tab, Tab: past Search to the first row | `kb-05-tab-to-first-row.png` | |
| ↓ ↓ | `kb-06-row-selected-by-arrows.png` | the second row's frame selected; the arrows select and don't move the camera |
| Enter | `kb-07-after-enter.png` | the camera flies to it: (2000, 1000) → (2000, 3338), its top edge at the 32 px margin (it is taller than the view) |

`ui.inspect` names the focused control only while a screen reader reads the tree. That's why the script found the button by its focus-ring pixels. The unit test `the_button_is_named_keyed_and_reachable_with_tab` checks the name ("Collapse all layers", ⌥L).

## Contrast

| What | Light | Dark |
|---|---|---|
| The button's icon (`icon`) on the panel | 9.33:1 | 8.05:1 |
| The icon on the hover fill | 8.48:1 | 7.32:1 |
| The focus ring on the panel | 5.43:1 | 6.01:1 |
| The hidden-layer toast (white on #1E1E1E, both themes) | 16.7:1 | 16.7:1 |

The tooltip "Collapse all layers  ⌥L" uses egui's tooltip colours, as every other tooltip does.
