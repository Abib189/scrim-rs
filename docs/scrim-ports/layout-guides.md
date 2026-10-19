# Port 7: layout guides, mapped onto FigCraft's layout grids (19 Oct 2026)

**The brief's rule:** port Scrim's layout guides only if FigCraft's layout grids don't already cover the model; otherwise map and report.

**They cover it.** FigCraft has Figma's layout grids (`node.grids`, `figcraft_doc::LayoutGrid`, `figcraft_layout::grid_lines`). Scrim's guides were modelled on them, so this port **maps and reports** and writes no new feature. The mapping is tested, not assumed: `crates/engine/tests/scrim_guides.rs` runs Scrim's 29 geometry cases (25 column and row guides, 8 square grids) through FigCraft's grid code.

**Sources:**
- the spec, `docs/scrim-transfer/specs/layout-guides.md`;
- the cases, `docs/scrim-transfer/cases/guides.json`.

## What the test shows

| | Result |
|---|---|
| **Track geometry** (column and row: stretch, start, end, center; margins, gutters, fractional sizes, offset frames, tracks past the frame, a negative start) | **25 of 25 identical**: every track's position and size, to 1e-5. FigCraft uses the same formulas, Figma's. |
| **Square grids** | **8 of 8 identical** on the interior lines. FigCraft also draws the frame's own left and top edges, and the far edges when the size divides the frame; Scrim draws interior lines only. |
| **Gutter 0** | The same edges. Scrim draws them as lines; FigCraft draws touching bands whose edges are those lines. |
| **No room** (stretch with margins and gutters that fill the frame) | Scrim draws nothing; FigCraft keeps zero-width tracks (nothing visible). |
| **Snap lines** | **Every snap line Scrim has, FigCraft has too** (29 of 29 cases). Both snap only to visible guides. FigCraft also keeps lines that fall outside the frame, from tracks past it; Scrim drops them. |

## The mapping (spec §6, now verified against FigCraft's code)

| Scrim | FigCraft | Relation |
|---|---|---|
| `type` column / row / grid | `kind` `columns` / `rows` / `grid` | 1 : 1 |
| `align` stretch / start / end / center | `align` `stretch` / `min` / `max` / `center` | 1 : 1, the same maths |
| `size` (always stored, 1–100,000) | `size` (fixed tracks; a grid's cell) | 1 : 1. FigCraft falls back to 60 for a fixed track of size 0 and to 10 for a grid under 1. |
| `count` 1–100 | `count`, **0 = Auto** (as many as fit), clamped to 500 | FigCraft has Auto; Scrim doesn't |
| `margin` | `offset` | 1 : 1 in meaning |
| `gutter` | `gutter` | 1 : 1 |
| `color` `#rrggbb` + `opacity` | `color` with alpha | Split vs combined |
| `visible`: hidden neither draws nor snaps | `visible`: the same (`grid_snap_lines_in` filters on it) | 1 : 1. The spec had guessed FigCraft would snap to hidden grids, as Figma does; it doesn't |
| — | variable bindings for size, count, gutter and offset; grid styles | FigCraft only |
| at most 20 guides; count ≤ 100; lengths ≤ 100,000; refused, never clamped | no list cap; count clamped to 500; non-finite values fall back | Scrim's caps are stricter. A FigCraft list can exceed them |
| drawn on **every frame** with a visible guide, nested frames included | drawn on **top-level frames** only (`canvas.rs`, boards) | Differs |
| drawing: not below 4 screen px per cell; bands clipped to the frame and its clipping ancestors; 1 device px lines | no minimum cell size (a cap of 4,000 lines per axis); bands not clipped to the frame | Differs |
| creation default: a 12-column stretch guide, `#003880` at 10% (Paper's) | a 10 px grid, red at 10% (Figma's); columns 5 × 20 gutter | Differs (a product decision) |
| instances follow the master, no overrides | as Figma (overrides allowed) | Differs |

## What to adopt from Scrim, with estimates

| Item | Why | Estimate |
|---|---|---|
| Grids and guides on nested frames | Scrim draws and snaps them on any frame; FigCraft only on top-level frames | 0.5 session (`canvas.rs` loop, snapping already per parent) |
| Clip bands to the frame, and skip grids under 4 px per cell | Fixed tracks past the frame draw outside it; a fine grid at low zoom becomes a solid wash | 0.5 session |
| Interior grid lines only; lines (not touching bands) at gutter 0 | Cleaner drawing, as Scrim's research found; geometry is unchanged | 0.5 session |
| Drop snap lines outside the frame | Scrim's rule; avoids snapping to invisible lines | Under 0.5 session |
| Scrim's caps on writes (20 per frame, count ≤ 100, lengths ≤ 100,000, refused with a reason) | Agents writing grids get the same clear refusals as in Scrim | 0.5 session |
| Scrim's default (12 stretch columns, gutter 8) | A product choice: Paper's defaults suit UI work | A decision for the owner |

None of these is needed for parity of the model, which is complete.
