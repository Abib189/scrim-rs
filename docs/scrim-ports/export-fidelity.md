# Port 2: the export guarantee (19 Oct 2026)

**The question:** does the HTML export render like the canvas? This port measures it with Scrim's harness (`docs/scrim-transfer/html-fidelity/`), fixes the largest cheap causes, and adds `cargo xtask fidelity`, which fails when any artboard gets worse than `xtask/fidelity-baseline.json`.

**How it measures:** the 20 top-level frames of the five transfer fixtures.
- **Canvas:** `figcraft-cli export --node`.
- **Export:** `inspect.code {language: html}`, rendered in Chromium 156 and WebKit 27.2 with the network blocked.
- **The diff:** a channel off by more than 24 counts; antialiasing is a match within 1 px both ways; the outer 1 px is excluded.
- **Fixtures:** stage-a is read bare (`fixtures/fig/stage-a.bare.fig`), because its zip has CRC 0.
- **Timings:** none are claimed. The machine was loaded and swapping.

## Result

| | Chromium | WebKit |
|---|---|---|
| **Before** (FigCraft a0d1162) | 9 of 20 identical | 9 of 20 identical |
| **After** | **10 of 20** | **11 of 20** |
| In Scrim's 19-artboard set (no FigJam note): before | 8 of 19 | 8 of 19 |
| In Scrim's set: after | **9 of 19** | **10 of 19** |
| Scrim's own probe (DOM canvas, for reference) | 15 of 20 | 16 of 20 |

### Per artboard (real % / antialiasing %)

| Artboard | Before (Chromium) | After (Chromium) | After (WebKit) | What remains, and its cause |
|---|---|---|---|---|
| stage-a / Card | 12.21 / 0.22 | 1.36 / 0.26 | 1.32 / 0.25 | **Font:** the text is "Inter", installed in neither renderer, so each falls back to a different face. **Layout:** a 2 px centred stroke (CSS borders sit inside) |
| stage-a / Button, Button copy, FigJam note | 0 | 0 | 0 | — |
| stage-b / Row | 28.54 / 0.07 | 14.69 / 0.06 | 14.67 / 0.06 | **Fixture artifact.** The synthetic file's stored geometry (Grows 200 px at x 68, 12 px gaps) agrees with neither Figma's rules nor FigCraft's own layout (fill takes the rest; "space between" drops the gap), and FigCraft keeps stored geometry on import. A row FigCraft lays out itself is identical in both engines (checked) |
| stage-b / Column | 2.26 / 0 | 0.91 / 0.49 | 0.88 / 0.57 | **Font** (the label) |
| stage-b / Empty hug, Wrap, Grid, Free | 0 | 0 | 0 | — |
| stage-b / Effects | 27.44 / 0.48 | 20.80 / 0.48 | 20.81 / 0.45 | **Resampling:** a 2 × 2 image filling the frame, scaled up about 100×. The canvas smooths it to a blob; browsers show a flat square. **No CSS equivalent:** a diamond gradient (exported as radial) |
| stage-b / Masks | 53.66 / 0 | 0.005 / 0 | **0** / 0.014 | One antialiased corner pixel |
| stage-c / Texts | 1.39 / 0.09 | 1.06 / 0.22 | 1.08 / 0.23 | **Font** |
| stage-c / Icon | 0 | 0 | 0 | — |
| stage-c / Card, Board, Card B | 2.08 / 1.73 / 0.36 | 1.09 / 0.91 / 0.17 | 0.94 / 0.79 / 0.16 | **Font**, and antialiasing on round corners |
| stage-d / Icons | 3.63 / 0.04 | 1.35 / 0 | 1.35 / 0.00 | **Layout:** a star used as a vector mask (only rectangle and ellipse masks become `clip-path`), and a gradient-stroked rule |
| lines / Rules | 3.37 / 0.54 | **0** / 0.12 | **0** / 0.12 | — |
| lines / Stack | 0 | 0 | 0 | — |

## What was fixed

All the fixes are in `crates/engine/src/codegen.rs` and `codegen_css.rs`, each with a case in `codegen_export_tests.rs`.

1. **Box sizing and paragraph margins.** Every exported stylesheet starts with `*, *::before, *::after { box-sizing: border-box }` and `p { margin: 0 }`.
   - **Why:** the document's sizes are border-box, so padding was being added on top of them, and paragraphs added the browser's 1 em margin.
   - **Effect:** Row, Card, Column and the text artboards.
2. **Images inlined.** Image paints become `data:` URIs placed by their scale mode (Fill → cover, Fit → contain, Tile → repeat at the image's size). Before, the export referenced a missing `image.png`. Crop shows as Fill, and rotation is not exported.
3. **Paints layered.** Several fills become CSS background layers, top first; a solid paint among others becomes a flat gradient. Before, only the last fill survived.
4. **Shapes CSS can't draw become inline SVG.** Paths, polygons, stars, lines, booleans and ellipse arcs or rings are drawn by FigCraft's own SVG exporter, placed over the layer's box.
   - The SVG carries the layer's turn in page space, so the box itself isn't rotated.
   - **Before**, they were grey or filled boxes.
5. **Masks become `clip-path`.** A rectangle or ellipse mask clips the layers above it, which go in a wrapper; the mask itself isn't drawn. **Before**, the mask was drawn as a layer and nothing was clipped.
6. **Text shadows come from the glyphs.** A drop shadow on text is `text-shadow`, not `box-shadow`.
7. **Auto spacing replaces the gap; wrapped lines keep theirs.** "Space between" or "around" exports `gap: 0`, as FigCraft lays it out. Wrapped rows use `cross_gap` as the gap between lines. **Before**, CSS added the gap and wrapped lines used the wrong spacing.

**Tried and reverted** (the gate refused both):
- **Strokes as `outline` with an offset by alignment.** It was closer to the design, but the antialiasing at corners made three artboards worse.
- **A generic `sans-serif` fallback after the family.** The canvas's fallback face isn't the browser's default sans, so text got slightly worse.

## What remains, with estimates

| Cause | Artboards | Fix | Estimate |
|---|---|---|---|
| **Font:** "Inter" is in neither renderer | stage-a Card, Column, Texts, the stage-c cards (about 1% each) | Give both renderers the same font: the `craft-fonts` build input for the canvas, and `@font-face` from the same file in the export (`--fonts assets`, as OpenPencil does). This is the biggest remaining lever | 1–2 sessions |
| **Fixture artifact:** stored geometry against auto-layout rules | Row | None needed in the export. Either re-layout auto-layout frames on import (an importer decision, with Port 4), or measure a fixture saved by FigCraft | Decision, then about 1 session |
| **Vector masks** | Icons | `clip-path: path(…)` in the wrapper's coordinates, from the mask's outline | 1 session |
| **Gradient strokes** | Icons | Draw gradient-stroked boxes as inline SVG as well | 0.5 session |
| **Diamond gradient** | Effects | No CSS equivalent: draw the paint as inline SVG with a mesh, or as an image | 1 session, or accept as an approximation |
| **Upscaled tiny images** | Effects | A resampling difference: `image-rendering` can't match the canvas's filter. Accept, or rasterise the paint | Accept |
| **Centred and outside strokes** | stage-a Card (2 px) | `outline` regressed corners. Try `box-shadow` rings or an inner wrapper instead, gated by the xtask | 1 session |

## Running the check

```sh
export PWCORE=/path/to/node_modules/playwright-core/index.mjs     # npm i playwright-core
export PLAYWRIGHT_BROWSERS_PATH=/path/to/browsers                 # npx playwright-core install chromium webkit
cargo xtask fidelity                    # fails if any artboard is worse than the baseline
cargo xtask fidelity --update           # after an intended change: write the new baseline
```

**Output:** `target/fidelity/` holds the pairs, the rendered pages, the diff heat maps and `summary.json`.

**Why it isn't in `cargo xtask ci`:** CI machines may lack Node and browsers. Run it with every change to `codegen*.rs`, the renderer, text or layout.
