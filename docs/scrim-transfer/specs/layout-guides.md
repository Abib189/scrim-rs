# Layout guides

**For:** the Rust fork of FigCraft, re-implementing Scrim's layout guides from this spec and `../cases/guides.json` alone.

**Source of truth** (Scrim, commit `f2b1bdb`):
- `packages/core/src/schema.ts` (`LayoutGuide`) and `packages/core/src/guides.ts` (validation);
- `packages/engine/src/layoutGuides.ts` (geometry, snap lines, drawing);
- `apps/studio/src/editor/guideEdits.ts` (the inspector's model and the creation defaults), `editor/guidePrefs.ts` (⇧G), `editor/layoutGuides.ts` (inputs from the document) and `inspector/GuidesSection.tsx`;
- `packages/mcp/src/guideTools.ts` (agents);
- `docs/research/layout-guides.md` (decisions and reasons) and `docs/checklists/layout-guides.md`;
- ARCHITECTURE.md › "Layout guides".

**Cases:** `../cases/guides.json`, produced by running Scrim's code. It holds:
- 29 geometry cases;
- 33 validation cases;
- inspector cases: 5 type switches, 20 number inputs, 7 colour inputs, 3 displayed values and 8 patches/readings;
- the draw rule.

Layout guides are **editor chrome**: Paper-style column, row and square-grid overlays on a frame, drawn and snapped to, and **never** part of the design. They have nothing to do with CSS Grid layout (`display: grid`).

## 1. The model

A frame has an optional field `guides?: LayoutGuide[]`. The field is additive and needs no `schemaVersion` bump.

```
LayoutGuide = GridGuide | TrackGuide

GridGuide  = { type: "grid",            size, color, opacity, visible }
TrackGuide = { type: "column" | "row",  align, size, count, margin, gutter, color, opacity, visible }
```

| Field | Types | Unit | Allowed (enforced by core) | Meaning |
|---|---|---|---|---|
| `type` | all | — | `"column"`, `"row"`, `"grid"` | Columns divide the width, rows divide the height, and a grid is square cells from the frame's top-left corner. |
| `size` | all | px | finite, `1 ≤ size ≤ 100000` (fractions allowed) | Grid: the cell size. Column / row: each track's width (or height), **ignored when `align` is `stretch`** but still required and validated. |
| `align` | column, row | — | `"stretch"`, `"start"`, `"center"`, `"end"` | Shown as stretch / left / center / right for columns and stretch / top / center / bottom for rows. The stored values are never the UI words. |
| `count` | column, row | tracks | an integer, `1 ≤ count ≤ 100` | There is no "Auto". |
| `margin` | column, row | px | finite, `0 ≤ margin ≤ 100000` | The space before the first track and after the last (stretch); the offset from the start edge (start) or the end edge (end). **Ignored by `center`.** |
| `gutter` | column, row | px | finite, `0 ≤ gutter ≤ 100000` | The space between tracks. |
| `color` | all | — | exactly `#` followed by 6 hex digits (either case) | `#rgb`, `#rrggbbaa` and `rgba()` are refused. |
| `opacity` | all | — | finite, `0 ≤ opacity ≤ 1` | Applied as the fill alpha. |
| `visible` | all | — | a boolean | The eye. A hidden guide neither draws nor snaps. |

A grid guide takes **only** `type size color opacity visible`; any other key is refused (`count`, `align`…). A track guide takes only its nine keys. **There is no `offset`**: Scrim's `margin` plays Figma's `offset` role (§6).

The list holds at most **20 guides** per node. Their order is the draw order; overlaps simply add their tints.

### 1.1 Validation (core `guidesIssues`, run by `applyPatch`)

Checks run on `add_node`, on each node of `add_subtree`, and on `update_node` when `set.guides` is present. `validateDocument` reports each issue as `{ code: "bad_guides", message: "<nodeId>: <issue>" }`.

**Order** (the first two steps short-circuit; the rest collect):

1. Absent → fine. Not an array → `guides must be a list` (this is checked **before** the node type).
2. The node type is not `frame` or `instance` → `only frames hold guides (this is a <type>)`.
3. More than 20 → `a frame holds at most 20 guides (got <n>)`.
4. Per guide `i`, with `<at>` = `guides[<i>]`:
   - not an object → `<at> must be an object` (stop);
   - a bad `type` → `<at>.type must be one of column, row, grid` (stop);
   - each unknown key, in the object's key order → `<at>: a <type> guide has no "<k>" (it takes <allowed keys joined by ", ">)`. The allowed keys print as `type, color, opacity, visible, size` for a grid, and `type, color, opacity, visible, align, size, count, margin, gutter` for column and row;
   - `<at>.color must be a 6-digit hex colour like #003880`;
   - `<at>.opacity must be a number from 0 to 1`;
   - `<at>.visible must be true or false`;
   - `<at>.size must be a number of px from 1 to 100000`;
   - track guides only:
     - `<at>.align must be one of stretch, start, center, end`;
     - `<at>.count must be a whole number from 1 to 100`;
     - `<at>.margin must be a number of px from 0 to 100000`, then the same for `gutter`.

**The message a writer sees** (PatchError): `<node.name or node.id>: <every issue joined by "; ">`. This is not capped at 5 the way svg is. Example: `Frame: guides[0].count must be a whole number from 1 to 100`.

**Agents** (MCP `create_node` / `update_node`) are refused before core with these messages:
- on an instance or anything inside one: `this frame is part of a component instance, which follows its master's guides: set them on the master (or detach the instance first)`;
- on a non-frame: `guides go on frames only (this is a <type>)`.

Core itself also accepts `instance` nodes: propagation writes the master's list there.

### 1.2 Serialisation and semantics

- **Storage:** the node JSON field `guides`, holding the list with exactly the keys above. Whole-list LWW (last write wins), like `canvas`.
  - `update_node { guides: null }` and `{ guides: [] }` both remove the field (an empty list is **stored as absent**), so an undo round trip is exact.
  - The stored value is a deep copy.
- **Undo:** each write is one patch and one undo step. The inverse restores the previous list, or `null` when there was none.
- **Files:** kept in the `.scrim` file (library and Save As), like `canvas` positions.
- **Never exported:** not in JSX, CSS, Tailwind, standalone HTML, screenshots, thumbnails or lint. Scrim's `exportGateGuides.test.ts` proves the export bytes are unchanged with guides present.
- **Copy, paste, duplicate:** they keep a frame's guides and its nested frames'. A pasted payload with a malformed list is rejected whole.
- **Components:** an instance mirrors its master's whole list in the same patch as the master edit (`propagateForNode` / `syncNode`). There are no per-instance overrides, and detaching keeps the list.
- **Agents:** `get_node` returns `guides`. `get_document_outline` marks a guided frame `guides: <count>`. There are no guide-specific tools.

## 2. Geometry (engine `layoutGuides.ts`)

Everything is computed from the frame's **measured border box** `F = { x, y, width, height }` in world px on every draw, never from stored sizes (rule 9). Guides therefore follow a resize: stretch tracks grow, and fixed tracks keep their size and move with the alignment. Padding is ignored.

### 2.1 Track layout (`trackLayout(g, L)`)

`L` is `F.width` for columns and `F.height` for rows. `n = count`, `s = size`, `m = margin`, `g = gutter`.

| align | track size | start (from the frame's edge) | step |
|---|---|---|---|
| `stretch` | `max(0, (L − 2m − (n−1)·g) / n)` | `m` | `size + g` |
| `start` | `s` | `m` | `s + g` |
| `end` | `s` | `L − m − (n·s + (n−1)·g)` | `s + g` |
| `center` | `s` | `(L − (n·s + (n−1)·g)) / 2` (`m` unused) | `s + g` |

These are Figma's formulas. Penpot's stretch-with-fixed-size, which spreads the gutters instead, is deliberately **not** used.

### 2.2 Shape (`guideShape(g, F)`): bands and lines in world px

- **Grid:**
  - `xs` = `F.x + size`, `F.x + 2·size`, … while `x < F.x + F.width − 1e-6`;
  - `ys` likewise from `F.y`.
  - These are **interior lines only**: the frame's own edges are never included, and a line exactly on the far edge is not added.
  - The loop accumulates by repeated addition (`x += size`). Use the same accumulation to match fractional sizes bit for bit.
  - No bands.
- **Column / row:**
  - If the track size is `≤ 0`, nothing is produced: no bands and no lines.
  - Otherwise, with `origin = F.x` (columns) or `F.y` (rows), for `i = 0 … n−1`: `a = origin + start + i·step`.
  - **`gutter > 0`:** a band per track.
    - Columns: `{ x: a, y: F.y, width: size, height: F.height }`.
    - Rows: `{ x: F.x, y: a, width: F.width, height: size }`.
  - **`gutter === 0`:** no bands; touching bands would merge into one flat tint. The band edges become lines instead: `a₀`, then `a + size` for every `i`, so `n + 1` constant-x lines for columns or constant-y lines for rows.
- **Bands are not clipped** to the frame here. Fixed tracks can run past it (`start` with a large count) or start before it (`end` wider than the frame gives a negative start). Clipping happens when drawing and when collecting snap lines.

**Worked example** (geometry case 1). Paper's default column guide on a 1440-wide frame:
- size = (1440 − 0 − 11·8) / 12 = 112.666667, step 120.666667;
- 12 bands at x = 0, 120.67, 241.33, …

### 2.3 Snap lines (`guideSnapTargets(guides, F)`)

Only guides with `visible: true` contribute. A guide with `opacity: 0` but `visible: true` **still snaps** (geometry case "zero opacity").

Two sets are collected, de-duplicated in first-seen order:
- **xs:** a grid's `xs`; a gutter-0 track's `xs` / `ys`; for column bands, both band edges `b.x` and `b.x + b.width`;
- **ys:** likewise, from row bands (`b.y` and `b.y + b.height`).

A value is kept only if it lies inside the frame, `F.x − 1e-6 ≤ x ≤ F.x + F.width + 1e-6` (likewise for y). So a gutter-0 track's lines **on the frame's edges are kept**, unlike a grid's.

The output is the x lines as zero-width rects `{ x, y: F.y, width: 0, height: F.height }`, then the y lines as `{ x: F.x, y, width: F.width, height: 0 }`. Band centres are never snap targets; the snap engine aligns a rect's start, centre and end to these zero-width lines.

**Who snaps** (studio):
- only the **direct children** of the guided frame;
- when moving an absolute child, resizing any child, and drawing a new layer inside the frame;
- governed by the master snapping switch, ⌘ (which inverts it for the gesture), the option "Snap to layout guides" (on by default) and View › Layout guides (⇧G);
- flow children of a flex or grid frame snap only while being resized;
- a move still writes no size.

### 2.4 Drawing (`drawLayoutGuides`)

- **Placement:** on the Canvas2D chrome overlay, under all other chrome, never in the design DOM. Guides capture no pointer.
- **Which frames:** every frame with a visible guide, **selected or not**, unless View › Layout guides is off.
- **The clip:** the frame's border box, intersected with the padding box of every clipping ancestor (overflow clipping, per axis), then with the viewport. Frames off screen are culled.
- **Each guide in list order:**
  - skip it when `!visible`, when `opacity ≤ 0`, or when it is a grid with `size × zoom < 4` (`MIN_GRID_CELL_PX`) screen px;
  - otherwise fill with `color` at `globalAlpha = opacity`:
    - bands as rects clipped to the frame's clip;
    - lines as **1 CSS px** rects (`max(1, round(dpr)) / dpr` wide) spanning the clip.
  - A line is drawn only when its screen position satisfies `clipStart ≤ p < clipEnd`, so a line exactly on the far edge is snappable but **not drawn**.
- **Pixel alignment:** every edge is snapped to the device pixel grid with `round(v·dpr)/dpr`, so guides are crisp at DPR 1 and 2.
- **Afterwards:** `globalAlpha` is reset to 1.

`drawnAtZoom1` in each geometry case counts the rectangles filled at zoom 1, DPR 1, with the clip equal to the frame.

## 3. Editor creation defaults (rule 10: they live in the editor, never in core)

| Action | Result |
|---|---|
| **+** in the Guides section, and the only way to add | A **Column** guide: `{ type: "column", align: "stretch", size: 40, count: 12, margin: 0, gutter: 8, color: "#003880", opacity: 0.1, visible: true }`. These are Paper's values. Figma and Penpot add a square grid; Scrim deliberately adds columns. |
| Type → Row | `{ align: "stretch", size: 40, count: 12, margin: 0, gutter: 8 }` plus the look values. |
| Type → Grid | `{ size: 50 }` plus the look values. |
| Look values for every type | `#003880` at 10%, visible. |

**Type switch (`switchType`)** keeps every value the new type also has. The shared values are `color`, `opacity`, `visible` and `size`; column ↔ row keeps everything. The new type's other fields take their defaults. So a grid becomes a column with `size` = the grid's size, `count` 12, stretch, margin 0, gutter 8. The cases are in `inspector.typeSwitch`.

Core's `createNode` never adds guides: a new frame has no `guides` field.

## 4. The inspector (GuidesSection)

**When the section shows:** only when every selected node is a frame or an instance.
- **Mixed:** if the selected frames' lists differ (compared as JSON), it shows "Different guides on each frame" and one − that removes every guide from the selection.
- **Instances:** on an instance (or inside one) it is **read-only**, showing the note `Guides on a component instance follow its master: edit them on the master (or detach the instance).` and the count. It offers no fields and no +.

**Each guide row has:**
- the type select (`column`, `row`, `grid`), an eye button (`aria-pressed` = visible; its title is "Hide this guide (it stops snapping too)") and − to remove it;
- for a track guide:
  - the alignment select (the axis words);
  - **w** (columns) or **h** (rows) for size, **disabled while stretch** with the reason "Stretch sizes the columns / rows to fill the frame: size is used by the other alignments";
  - **#** for count, **Margin** and **Gutter**;
- for a grid: **Size**;
- for every guide: a colour field and an opacity field in %.

**Number entry (`parseGuideNumber`):**
- a trailing `px` or `%` is accepted, and the rest must match `^-?\d*\.?\d+$`, so there are no exponents;
- **opacity** is typed in percent, 0–100, and stored as `round(v·10)/1000`;
- **count** is a whole number, 1–100;
- **size** is from 1, **margin** and **gutter** from 0, all up to 100000, rounded to 0.01;
- anything else is rejected and nothing changes.

The displayed opacity is `round(opacity·1000)/10`. See `inspector.numberInputs` and `inspector.shown`.

**Colour entry (`parseGuideColor`):**
- accepts `#rgb`, `#rrggbb`, bare `rrggbb`, `rgb()` / `rgba()`, `hsl()` / `hsla()` and token references;
- stores the colour as lowercase `#rrggbb`;
- a picked alpha below 1 becomes the guide's opacity (rounded to 0.001);
- named colours (`red`) are rejected.

See `inspector.colorInputs`.

**Editing behaviour:**
- Typing and ↑ / ↓ preview live through the preview layer, and nothing is sent.
- Enter or blur commits, and Escape restores.
- Dragging a number label scrubs (gesture registry entry `scrub:guides`).

**Patches:** every change is **one patch** that writes the whole list to each selected frame, which is one undo step.
- `+` on several frames appends a default column to **each frame's own list**.
- Writing a list equal to the current one makes no patch.
- `guideFieldPatch` refuses fields a grid doesn't have (`count`, `margin`, `gutter`).

See `inspector.patches`.

**View preference:** View › Layout guides (⇧G) is a UI preference in `localStorage` key `scrim.guides.shown` (`"0"` = hidden; default shown). It is never document data, and it hides all guides and their snapping.

## 5. Not built in Scrim

- count Auto;
- guide styles;
- "set as default";
- ruler guides;
- copying one guide between frames;
- reordering guides in the UI (the list order can only be changed by writing a new list);
- variable or token bindings;
- per-instance overrides;
- importing Figma `layoutGrids` (`packages/fig-import` ignores them).

## 6. Mapping to FigCraft's layout grids

**Assumption, stated once:** FigCraft's code was not readable here. This mapping assumes FigCraft mirrors Figma's Plugin API `LayoutGrid`:
- `{ pattern: "COLUMNS" | "ROWS", alignment: "MIN" | "MAX" | "STRETCH" | "CENTER", gutterSize, count, sectionSize?, offset?, visible?, color: RGBA }`;
- `{ pattern: "GRID", sectionSize, visible?, color }`.

FigCraft exposes this as `node.set { grids }`, the `grid.add` / `grid.remove` / `grid.move` commands, and the variable bindings `layoutGrids/<i>/<size|count|gutter|offset>`. Rows marked *(Figma, unverified)* come from Figma's documentation or memory, not from running anything.

### 6.1 Field by field

| Scrim | FigCraft / Figma | Relation | Conversion and notes |
|---|---|---|---|
| `type: "column"` / `"row"` / `"grid"` | `pattern: "COLUMNS"` / `"ROWS"` / `"GRID"` | **1 : 1** (renamed) | Upper-case, plural. |
| `align: "start"` / `"end"` / `"center"` / `"stretch"` | `alignment: "MIN"` / `"MAX"` / `"CENTER"` / `"STRETCH"` | **1 : 1** (renamed) | The same math (§2.1). Scrim's UI words left/right and top/bottom are Figma's too. |
| `size` | `sectionSize` | **1 : 1**, with a difference | Both ignore it for stretch. Scrim **always stores** it (1–100000, even for stretch); Figma may omit it. FigCraft → Scrim, a missing `sectionSize`: use 40 (Scrim's default) *(suggestion)*. |
| `count` | `count` | **differs** | Scrim: an integer, 1–100. Figma: also `Infinity` = **Auto** (as many as fit) *(Figma)*. Scrim has no Auto. FigCraft → Scrim: compute a count, or refuse *(Scrim has no rule; a port must choose)*. |
| `margin` | `offset` | **1 : 1 in meaning, renamed** | Both: both sides for stretch, distance from the start edge for MIN, from the end edge for MAX, ignored for CENTER. Scrim: 0–100000. |
| `gutter` | `gutterSize` | **1 : 1** | Scrim: 0–100000. A gutter of 0 draws edge lines instead of bands in Scrim; in Figma it draws touching bands *(Figma, unverified)*. |
| `color` (`#rrggbb`) + `opacity` (0–1) | `color: { r, g, b, a }` (0–1 each) | **differs** (split vs combined) | Scrim → Figma: `r = parseInt(hex[1..3], 16) / 255`, and so on; `a = opacity`. Figma → Scrim: `color = "#" + hex2(round(r·255)) + hex2(round(g·255)) + hex2(round(b·255))` (lower-case), `opacity = a`. The hex rounds to 1/255 per channel. |
| `visible` | `visible` | **1 : 1 field, different semantics** | Scrim: hidden = **neither drawn nor snapped**. Figma: a hidden guide "will still work" for snapping *(Figma help)*. Figma's default when absent is `true` *(Figma)*; Scrim requires the key. |
| — | `boundVariables` / FigCraft `layoutGrids/<i>/<size\|count\|gutter\|offset>` | **FigCraft only** | Scrim cannot bind a guide field to a variable or token. The binding paths map to `size`, `count`, `gutter` and `margin`. |
| — | `gridStyleId` (layout guide styles) | **FigCraft only** | Scrim has no guide styles. |
| — | per-instance overrides | **FigCraft only** *(Figma)* | Scrim instances follow the master and refuse writes. |
| list limits: 20 guides, count ≤ 100, lengths ≤ 100000 px, size ≥ 1 | none documented *(Figma)* | **Scrim only** | A FigCraft list can exceed Scrim's caps. Converting into Scrim must refuse or split; Scrim never clamps. |
| `margin` ignored by center; `size` ignored by stretch, but both validated | `offset` / `sectionSize` ignored the same way | — | Keep the stored value on round trips, so switching alignment back restores it. |

### 6.2 Commands

| FigCraft | Scrim equivalent |
|---|---|
| `node.set { grids: [...] }` | `update_node { id, guides: [...] }`: the whole list, one commit and one undo step. `null` or `[]` clears it. Refused on non-frames and instances (§1.1). |
| `grid.add` | Write the list with one guide appended. The studio's + appends Scrim's default **column** guide (§3). Figma's + adds a GRID of 10 px, `#FF0000` at 10% *(Figma)*. |
| `grid.remove <i>` | Write the list without index `i`. Removing the last guide removes the field. |
| `grid.move <from> <to>` | Write the reordered list. Order = draw order; nothing else depends on it, and snap lines are a de-duplicated union. Scrim has no UI command for this. |
| `layoutGrids/<i>/size` | `guides[i].size` |
| `layoutGrids/<i>/count` | `guides[i].count` (whole, 1–100) |
| `layoutGrids/<i>/gutter` | `guides[i].gutter` |
| `layoutGrids/<i>/offset` | `guides[i].margin` |

### 6.3 Defaults side by side

| | Scrim (Paper's) | Figma *(unverified where marked in `docs/research/layout-guides.md`)* |
|---|---|---|
| What + adds | Column guide | Square grid |
| Grid | size 50, `#003880` at 10% | sectionSize 10, `#FF0000` at 10% |
| Columns / rows | 12, stretch, size 40, margin 0, gutter 8, `#003880` at 10% | count 5, stretch, offset 0, gutter 20 *(unverified)* |
| Hidden guides snap | no | yes |
| Shown on unselected frames | yes | yes |

## 7. What's in `guides.json`

- **`geometry[]`:** `{ name, frame, guides, perGuide[], snapTargets[], drawnAtZoom1 }`. Each `perGuide` entry has `track` (from `trackLayout`, track guides only) and `bands`, `xs` and `ys` (from `guideShape`, world px). Numbers are rounded to 6 decimals.
- **`validation[]`:** `{ name, nodeId?, guides, expect, issues?, stored? }`, written on frame `f` ("Frame"), box `b` ("Box") or text `t` ("Label").
  - `expect` is `"accept"` or `{ refuse: "<verbatim PatchError>" }`.
  - `stored` is what the document holds when it differs from the input (`"(absent)"` for `[]` and `null`).
- **`drawRules`:** `MIN_GRID_CELL_PX` and a below-threshold draw.
- **`inspector`:** `defaults`, `alignWords`, `readOnlyReason`, `typeSwitch`, `numberInputs`, `colorInputs`, `shown`, and `patches` (the exact patches the section builds).
