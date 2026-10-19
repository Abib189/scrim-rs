# .fig import: the mapping from Figma to output

**What this is:** how each Figma concept becomes output in Scrim's importer, and how faithfully. It is the companion of `specs/fig-import.md`, which covers the pipeline and the report.

**What the verdicts mean:**
- **exact:** renders as Figma does at the imported size;
- **approx:** drawn, but differs from Figma, and the report names it;
- **resize:** exact now, but resizing follows CSS's version of the Figma behaviour (a separate report category);
- **skipped:** not imported, and the report counts it;
- **silent:** dropped or changed with **no report line**, a gap the port should close (§7).

**About the columns:**
- **Output (model-neutral)** says what the result must *be*, in any model.
- **CSS (Scrim's reference)** is the exact output Scrim writes. It defines "exact": a port that renders differently from this CSS in a browser is not exact by Scrim's measure.
- **Field names** are Figma's Kiwi field names: `stackMode`, `fillPaints` and so on.

## 1. Structure

| Figma | Output (model-neutral) | CSS (Scrim's reference) | Verdict |
|---|---|---|---|
| Document, pages (CANVAS) | one canvas; each visible page's top-level layers become roots, pages stacked as bands top to bottom (§8.1 of the spec) | roots with canvas positions; `report.pages` lists roots per page | approx (no pages); not a report item |
| Figma's internal-only page | not imported; its layer count is skipped | — | skipped: *Figma’s internal-only page (style previews)* |
| FRAME, SYMBOL, INSTANCE, COMPONENT_SET | a container | `frame` | exact |
| GROUP, SECTION | a container that doesn't clip | `frame`, no `overflow` | exact |
| Clip content (`frameMaskDisabled !== true` on a non-group, non-section frame) | the container clips its children | `overflow: hidden` | exact |
| RECTANGLE, ROUNDED_RECTANGLE, ELLIPSE, LINE | a box | `box` | exact (the details below) |
| TEXT | a text node | `text` | §4 |
| VECTOR, STAR, REGULAR_POLYGON, BOOLEAN_OPERATION | a box holding generated vector paths | `box` + inline `<svg>` | §5 |
| Boolean operands | not imported: the flattened result is drawn | — | skipped: *Boolean operation operands — the shape is drawn from its flattened result* |
| STICKY, SLICE, CONNECTOR, WIDGET, SHAPE_WITH_TEXT, … (any other type) | not imported, with its subtree | — | skipped: *\<type\> layer (no Scrim equivalent)* |
| Hidden (`visible: false`) | kept and hidden | `display: none` | exact |
| Opacity < 1 | layer opacity | `opacity: round(o, 3)` | exact |

## 2. Layout and placement

### 2.1 Auto layout (`stackMode` HORIZONTAL, VERTICAL or GRID)

| Figma | CSS | Verdict |
|---|---|---|
| HORIZONTAL / VERTICAL | `display: flex; flex-direction: row / column` | exact |
| `stackSpacing` | `gap: <px>` (omitted when 0) | exact |
| Padding: `stackHorizontalPadding` = left, `stackVerticalPadding` = top, `stackPaddingRight` (default left), `stackPaddingBottom` (default top) | `padding: <n>` when all four are equal, else `padding: "<t>px <r>px <b>px <l>px"`; omitted when all are 0 | exact |
| `stackPrimaryAlignItems`: MIN, CENTER, MAX, SPACE_EVENLY, SPACE_BETWEEN | `justify-content`: (none), `center`, `flex-end`, **`space-between`**, `space-between`. Figma stores its "space between" mode as SPACE_EVENLY | exact |
| `stackCounterAlignItems`: MIN, CENTER, MAX, BASELINE, STRETCH (default MIN) | `align-items`: `flex-start`, `center`, `flex-end`, `baseline`, `stretch` (always written) | exact |
| `stackWrap: WRAP` | `flex-wrap: wrap`; `row-gap: stackCounterSpacing` when it differs from the gap | exact |
| Hug (`stackPrimarySizing` / `stackCounterSizing` = RESIZE_TO_FIT or RESIZE_TO_FIT_WITH_IMPLICIT_SIZE) | that axis's `width` / `height` omitted, unless the frame has no children (it keeps its stored size) or its parent is auto layout | exact |
| `stackReverseZIndex` (first on top) | the usual order | approx: *Auto layout “first on top” — drawn in the usual order* |
| GRID | `display: grid; grid-template-columns: repeat(<gridColumnCount>, 1fr); grid-template-rows: repeat(<gridRowCount>, auto)`; `column-gap` / `row-gap`; children at their fixed width and height | approx: *Auto-layout grid — equal tracks (Figma’s track sizes and cell spans not read)* |
| A frame with no auto layout | free: `display: block; position: relative`, children absolute | exact |

### 2.2 A child of auto layout

| Figma | CSS | Verdict |
|---|---|---|
| Any flow child | `flex-shrink: 0` | exact |
| `stackChildPrimaryGrow > 0` | `flex-grow: 1; flex-basis: 0`, no primary size | exact |
| Not growing | its primary size in px (none for text that hugs on that axis) | exact |
| `stackChildAlignSelf: STRETCH` | `align-self: stretch`, no counter size | exact |
| Not stretching | its counter size in px (none for text that hugs on that axis) | exact |
| `stackPositioning: ABSOLUTE` | `position: absolute; left; top; width; height` from its transform | exact |
| **Rotated or flipped flow child** | drawn unrotated, at its flow slot | **silent** (§7) |
| A mask inside auto layout | not applied: the masked layers are drawn whole | approx: *Mask inside auto layout — not applied (the masked layers are drawn whole)* |

### 2.3 A child of a free frame: constraints

`x, y` are the child's offset in the parent (`transform.m02`, `m12`), `w, h` its size, `pw, ph` the parent's size. Every case reproduces the Figma position at the imported size exactly.

| `horizontalConstraint` / `verticalConstraint` | CSS on that axis | Verdict |
|---|---|---|
| MIN (or missing, or anything else) | `left: x` / `top: y`, `width` / `height` in px | exact |
| MAX | `right: pw − x − w` / `bottom: ph − y − h`, size in px | resize |
| CENTER | `left: calc(50% - <pw/2 − x>px)` / `top: calc(50% - <ph/2 − y>px)`, size in px | resize |
| STRETCH | `left: x; right: pw − x − w` (no width) / `top; bottom` (no height) | resize |
| SCALE (parent size > 0) | `left: <x/pw × 100>%`, `width: <w/pw × 100>%` (3 decimals); same vertically | resize |

Always `position: absolute`. Any non-MIN constraint adds the layer to the resize item, *Constraints (centre, right, stretch, scale) — exact at this size; resizing follows CSS’s approximation*.

**Rotation and flips** come from the transform matrix `[m00 m01 m02; m10 m11 m12]`:
- `θ = atan2(m10, m00)`, and the layer is flipped when `det < 0`.
- A rotated or flipped child of a free frame is placed with `left: m02; top: m12; width; height; transform: rotate(<θ°, 3 decimals>deg)[ scaleY(-1)]; transform-origin: 0 0`, with no constraints: exact.
- At the top level, rotation is ignored: approx, *Rotated top-level frame — drawn unrotated*. A flipped root is **silent**.

**A mask** (`mask: true`) in a free frame turns into a wrapper `frame` placed at the mask's box:
- `display: block; position: absolute; left; top; width; height; overflow: hidden`;
- its shape:
  - an ellipse: `border-radius: 50%`;
  - a rectangle or frame: its radius;
  - a vector with exactly one path: `clip-path: path([evenodd, ]'<d>')`;
  - any other vector: its bounds only (approx: *Vector mask — clipped to its bounds*);
- the mask's own paint is not drawn;
- every later sibling moves inside it, at `left: x − maskX`, `top: y − maskY`, with no constraints;
- a rotated mask is drawn unrotated (approx: *Rotated mask — clipped unrotated*).

## 3. Paint, strokes and effects

### 3.1 Fills on boxes and frames

| Figma | CSS | Verdict |
|---|---|---|
| Visible paints | `visible !== false` and `opacity > 0`; the rest are ignored | exact |
| One SOLID | `background: <colour × paint.opacity>` | exact |
| Several paints (Figma's last is on top) | `background:` the layers top first, comma-separated; a solid that isn't the bottom layer becomes `linear-gradient(c, c)` | exact |
| GRADIENT_LINEAR | `linear-gradient(<angle>deg, <colour> <pos>%, …)` (the formula is below) | exact |
| GRADIENT_RADIAL, axis-aligned | `radial-gradient(ellipse <rx>px <ry>px at <cx>px <cy>px, <colour> <t×100>%, …)` | exact |
| GRADIENT_RADIAL, rotated | the same, upright | approx: *Rotated radial gradient — drawn upright* |
| GRADIENT_ANGULAR | `conic-gradient(from <a>deg at <cx>px <cy>px, <colour> <t×100>%, …)` | exact |
| GRADIENT_DIAMOND | radial, as above | approx: *Diamond gradient — drawn as a radial gradient* |
| EMOJI, VIDEO, other paint types | not drawn | approx: *\<type\> fill — not imported* |
| A paint `blendMode` other than NORMAL or PASS_THROUGH | drawn normally | approx: *Fill blend mode — fills drawn normally* |

**The gradient geometry (`paint.ts › handles`):**
- **The handles.** Take the inverse of the paint's `transform`, map gradient-space points into the layer's unit box, then scale by `w × h`:
  - start = `(0, ½)`, end = `(1, ½)`, centre = `(½, ½)`;
  - `u = map(1, ½) − centre`, `v = map(½, 1) − centre`.
- **Linear:**
  - `d = end − start`, `û = d / |d|`;
  - angle = `(atan2(ûx, −ûy) in degrees + 360) mod 360`;
  - line length `L = |w·ûx| + |h·ûy|`;
  - a stop at `t` sits at `(((start + d·t − (w/2, h/2)) · û) / L + ½) × 100%`.
- **Radial:** `rx = |u|`, `ry = |v|`; it counts as rotated when `|u.y| > 0.001 × rx`.
- **Angular:** start angle = `atan2(u.x, −u.y)` in degrees, mod 360.
- **Stops:** the colour × the paint's opacity; positions clamped to 0–1.
- **Rounding:** every number to 2 decimals.

### 3.2 Image fills

| Figma | Output | CSS | Verdict |
|---|---|---|---|
| IMAGE fill on a rectangle or ellipse, image imported | the layer becomes an image | `image` node, `src` from the pipeline, `alt` = the layer name; `object-fit: cover` (FILL) or `contain` (FIT) | exact |
| `imageScaleMode` TILE, STRETCH, CROP | cover | `object-fit: cover` | approx: *Image \<mode\> mode — drawn as cover* |
| Other fills above the image | kept as `background` under the image | — | approx: *Fills over an image — drawn under it* |
| IMAGE fill on a frame | an image child first, behind the content | `position: absolute; left: 0; top: 0; width: 100%; height: 100%; object-fit` | exact |
| Image not imported (missing, hash mismatch, refused) | the layer stays a box | — | approx: *Image fill not imported (see images)*, plus `report.images.failed` |
| The image paint's own opacity, transform (crop window) and filters | not read | — | **silent** |

### 3.3 Corners and strokes on boxes

| Figma | CSS | Verdict |
|---|---|---|
| `cornerRadius` | `border-radius: <px>` (omitted at 0) | exact |
| `rectangleCornerRadiiIndependent` | `border-top-left-radius` and the other three, or one value when all are equal | exact |
| ELLIPSE | `border-radius: 50%` | exact |
| Ellipse arc or ring (`arcData`) **with** `fillGeometry` | SVG from the geometry (§5) | exact |
| Ellipse arc **without** geometry | full ellipse | approx: *Ellipse arc or ring without geometry in the file — drawn as a full ellipse* |
| `cornerSmoothing > 0` | plain radius | approx: *Corner smoothing — CSS border-radius has no squircle* |
| Stroke INSIDE | `border: <w>px solid <c>` (border-box) | exact for the stroke; **see §7: children of a stroked frame shift by w** |
| Stroke OUTSIDE | `outline: <w>px solid <c>` | exact |
| Stroke CENTER (the default) | `outline: <w>px solid <c>; outline-offset: <−w/2>px` | exact on boxes |
| `borderStrokeWeightsIndependent` + INSIDE | `border-<side>-width / -style / -color` for each side with weight > 0 | exact |
| Per-side weights, outside or centred | one width | approx: *Per-side stroke weights outside or centred — one width used* |
| `dashPattern` | `dashed` | approx: *Dashed stroke — CSS dashes can’t take the exact pattern* |
| Several strokes | the **first SOLID in Figma's order** is kept, which is the *bottom* one; the note says "top" (§7.3) | approx: *Several strokes — the top one kept* |
| A gradient or image stroke | its first stop's colour | approx: *Gradient or image stroke on a box — drawn in its first colour* |

### 3.4 Effects and blend modes

| Figma | Box | Text | Vector | Verdict |
|---|---|---|---|---|
| DROP_SHADOW | `box-shadow: x y r spread c` | `text-shadow: x y r c` | `filter: drop-shadow(x y r/2 c)` | exact; spread on text or a vector is approx (*Text shadow spread — not in CSS*, *Shadow spread on a vector — not in CSS*) |
| INNER_SHADOW | `box-shadow: inset x y r spread c` | not drawn | not drawn | exact on boxes; else approx: *Inner shadow on text or a vector — not drawn* |
| FOREGROUND_BLUR / LAYER_BLUR | `filter: blur(r/2)` | same | same | approx: *Layer blur — CSS blur is half Figma’s radius (approximate)* |
| BACKGROUND_BLUR | `backdrop-filter: blur(r/2)` | same | same | approx: *Background blur — CSS backdrop blur is half Figma’s radius (approximate)* |
| Other effect types | — | — | — | approx: *\<type\> effect — not imported* |
| Layer `blendMode` | `mix-blend-mode`: MULTIPLY multiply, SCREEN screen, OVERLAY overlay, DARKEN darken, LIGHTEN lighten, COLOR_DODGE color-dodge, COLOR_BURN color-burn, HARD_LIGHT hard-light, SOFT_LIGHT soft-light, DIFFERENCE difference, EXCLUSION exclusion, HUE hue, SATURATION saturation, COLOR color, LUMINOSITY luminosity, LINEAR_DODGE plus-lighter; PASS_THROUGH and NORMAL write nothing | | | exact |
| Any other blend mode (LINEAR_BURN, …) | normal | | | approx: *Blend mode \<mode\> — no CSS equivalent, normal used* |

Several shadows are comma-joined and several filters space-joined. Hidden effects (`visible: false`) are ignored. Offsets, radius and spread are px rounded to 2 decimals. The shadow colour carries its own alpha.

## 4. Text

| Figma | CSS | Verdict |
|---|---|---|
| `textData.characters` | the text, line breaks kept | exact |
| `fontName.family` (default `Inter`) | `font-family: <family or "family">, <generic>`. It is unquoted when it matches `[A-Za-z0-9-]+`, else quoted with `"` and `\` removed. The generic is `monospace` for /mono\|code\|courier/i; `serif` for /serif\|garamond\|times\|georgia\|baskerville\|didot\|playfair\|merriweather\|lora/i without "sans"; else `sans-serif`. A CSS generic family is written alone | exact where the font is present |
| Font availability | recorded per family in `report.fonts`, with these notes: Google Fonts *"Google Font: the canvas draws it once loaded (Document › Load web fonts); exports link it"*; system fonts *"system font: exact where it is installed, \<generic\> elsewhere"*; generic *"a CSS generic family"*; anything else *"not a font Scrim knows: kept by name (drawn if installed), else \<generic\>"* (`as` = the generic). The lists are `packages/codegen/src/fontCatalog.ts` | not an item |
| `fontName.style` → weight (the first match wins) | thin or hairline 100; extra or ultra light 200; light 300; semi or demi bold 600; extra or ultra bold 800; black or heavy 900; bold 700; medium 500; else 400 (`font-weight` only when ≠ 400) | exact |
| italic or oblique in the style name | `font-style: italic` | exact |
| `fontSize` (default 12) | `font-size: <px>` | exact |
| Fill colour | the topmost visible SOLID → `color`. If a gradient or image fill is present: the first stop of the top fill, approx (*Gradient or image text fill — drawn in a solid colour*). No visible fill → `color: transparent` | exact for solids |
| `lineHeight` PIXELS / PERCENT / RAW | `"<v>px"` / unitless `v/100` / unitless `v` (3 decimals); AUTO and others write nothing | exact |
| `letterSpacing` PERCENT / PIXELS | `"<v/100>em"` / `"<v>px"` (omitted at 0) | exact |
| `textAlignHorizontal` CENTER, RIGHT, JUSTIFIED | `text-align: center / right / justify` | exact |
| `textAlignVertical` CENTER or BOTTOM on fixed-size text | `display: flex; flex-direction: column; justify-content: center / flex-end` | exact |
| `textCase` UPPER, LOWER, TITLE | `text-transform: uppercase / lowercase / capitalize` | exact |
| `textCase` SMALL_CAPS, SMALL_CAPS_FORCED | `font-variant-caps: small-caps` | exact |
| `textDecoration` UNDERLINE, STRIKETHROUGH | `text-decoration: underline / line-through` | exact |
| `fontVariations` | `font-variation-settings: "wght" 650, …`. The tag is `axisTag` (a u32 read as 4 characters, or a 4-alphanumeric string, lower-cased), else the first 4 characters of `axisName`; the value has 2 decimals | exact |
| `toggledOnOTFeatures` / `toggledOffOTFeatures` | `font-feature-settings: "tnum" 1, …, "liga" 0` (tags lower-cased) | exact |
| `textAutoResize` WIDTH_AND_HEIGHT | no width or height; `white-space: pre` | exact |
| `textAutoResize` HEIGHT | width fixed, no height; `white-space: pre-wrap` | exact |
| NONE (fixed) | width and height fixed; `white-space: pre-wrap` | exact |
| TRUNCATE, or `maxLines > 0` | not truncated | approx: *Text truncation — not kept* |
| `paragraphSpacing > 0` in multi-line text | dropped | approx: *Paragraph spacing — not kept (one text node, no paragraphs)* |
| Mixed style runs | the dominant run's style (spec §8.3) | approx: *Mixed text styles — the style of most characters is used (Scrim text has one style)* |
| Shared text style (`inheritTextStyleID`) | its values win over the layer's stale copies; the fill stays the layer's | exact |
| A stroke on text | not read | **silent** |

## 5. Vectors, components and variables

| Figma | Output | Verdict |
|---|---|---|
| `fillGeometry` / `strokeGeometry` command blobs | `<path d>` generated from float32 commands (M L Q C Z, 3 decimals); `windingRule: ODD` → `fill-rule="evenodd"`; strokes pre-outlined, so filled | exact |
| A gradient on a vector | `<linearGradient x1 y1 x2 y2 gradientUnits="userSpaceOnUse">`, or `<radialGradient cx=0 cy=0 r=1 gradientTransform="matrix(u.x u.y v.x v.y cx cy)">` | exact; angular approx (*Angular gradient on a vector — drawn as a radial gradient (SVG has no conic gradient)*); diamond approx |
| An IMAGE or other paint on a vector | not drawn | approx: *\<type\> fill on a vector — not drawn* (and *… stroke on a vector — not drawn*) |
| Strokes without stroke geometry | an SVG `stroke` along the fill paths, centred | exact for centre; approx: *Vector stroke inside / outside — drawn centred* |
| No usable geometry | an empty box | approx: *Vector without geometry in the file — not drawn* |
| A path over 200,000 characters | not drawn | approx: *Vector path too long for Scrim — not drawn* |
| Refused by Scrim's SVG allow-list | not drawn | approx: *Vector Scrim's SVG check refused — not drawn (\<first issue\>)* |
| A 0-wide or 0-high vector | drawn 1:1 in a 1 px box | exact |
| A LINE, or a thin rectangle that is only a rule | ONE `<line>` (spec §8.5) | exact; *Arrowheads and other line ends — drawn as plain ends*, *Dash pattern Scrim can’t hold — drawn solid* |
| A LINE the rule can't take (a gradient stroke, several strokes, weight 0) | a filled box, weight high, top moved up by half the weight, `background` from the top stroke paint | as the fill rules (§3.1) |
| Master (SYMBOL) on a visible page | a component made from its converted subtree | exact |
| An instance of such a master, not inside a master or instance | a real instance: own placement, look differences pinned as overrides, size included | exact |
| An instance override of `fillPaints`, `visible` or `textData.characters` | an instance override (`background` or text `color`, `display`, text) | exact |
| An override of an image fill | not applied | approx: *Image fill override on an instance — not applied* |
| An override of any other field | not applied | approx: *Instance override of \<first 3 fields\> — not applied* |
| An override through a nested instance (`guidPath` longer than 1) | not applied | approx: *Override inside a nested instance — not applied* |
| An override of a layer the master doesn't have | not applied | approx: *Override of a layer the component no longer has — not applied* |
| An instance inside a master or inside another instance | a detached copy (the master's layers, overrides applied inline) | approx: *Instance inside a component — kept as a detached copy* |
| An instance of a master on the internal page | a detached copy | approx: *Instance of a component on Figma’s internal page — kept as a detached copy* |
| An instance whose master is missing | not imported | skipped (count 1): *Instance whose master is not in the file* |
| Variants, component properties, instance swap, slots | COMPONENT_SET is a plain frame and each variant a separate component | **silent** |
| COLOR variable | a colour token from its first mode; layers keep their literal colours | approx (one item for all): *Colour variables became tokens — layers keep their colours (not linked to the tokens); modes: the first only* |
| FLOAT, STRING or BOOLEAN variable | none | skipped (count 0): *\<kind\> variable (only colour variables become tokens)* |
| A colour variable without a readable value | none | skipped (count 0): *Colour variable without a value Scrim can read (an alias?)* |

## 6. Every report feature string

Verbatim, typographic quotes and dashes included. `<…>` is filled in at run time.

**approximated:**
- *Auto-layout grid — equal tracks (Figma’s track sizes and cell spans not read)* · *Auto layout “first on top” — drawn in the usual order*
- *Rotated top-level frame — drawn unrotated* · *Mask inside auto layout — not applied (the masked layers are drawn whole)* · *Vector mask — clipped to its bounds* · *Rotated mask — clipped unrotated*
- *Fills over an image — drawn under it* · *Image fill not imported (see images)* · *Image \<tile\|stretch\|crop\> mode — drawn as cover* · *Ellipse arc or ring without geometry in the file — drawn as a full ellipse*
- *Corner smoothing — CSS border-radius has no squircle* · *Gradient or image stroke on a box — drawn in its first colour* · *Several strokes — the top one kept* · *Dashed stroke — CSS dashes can’t take the exact pattern* · *Per-side stroke weights outside or centred — one width used* · *Blend mode \<mode, lower case, spaces\> — no CSS equivalent, normal used*
- *Diamond gradient — drawn as a radial gradient* · *Rotated radial gradient — drawn upright* · *\<type\> fill — not imported* · *Fill blend mode — fills drawn normally* · *Angular gradient on a vector — drawn as a radial gradient (SVG has no conic gradient)*
- *Inner shadow on text or a vector — not drawn* · *Text shadow spread — not in CSS* · *Shadow spread on a vector — not in CSS* · *Layer blur — CSS blur is half Figma’s radius (approximate)* · *Background blur — CSS backdrop blur is half Figma’s radius (approximate)* · *\<type\> effect — not imported*
- *\<type\> \<fill\|stroke\> on a vector — not drawn* · *Vector stroke inside / outside — drawn centred* · *Vector without geometry in the file — not drawn* · *Vector path too long for Scrim — not drawn* · *Vector Scrim's SVG check refused — not drawn (\<issue\>)*
- *Arrowheads and other line ends — drawn as plain ends* · *Dash pattern Scrim can’t hold — drawn solid*
- *Mixed text styles — the style of most characters is used (Scrim text has one style)* · *Gradient or image text fill — drawn in a solid colour* · *Paragraph spacing — not kept (one text node, no paragraphs)* · *Text truncation — not kept*
- *Instance inside a component — kept as a detached copy* · *Instance of a component on Figma’s internal page — kept as a detached copy* · *Override inside a nested instance — not applied* · *Override of a layer the component no longer has — not applied* · *Image fill override on an instance — not applied* · *Instance override of \<fields\> — not applied*
- *Colour variables became tokens — layers keep their colours (not linked to the tokens); modes: the first only* (its "layer" is *"\<n\> tokens"*)

**skipped:**
- *Figma’s internal-only page (style previews)* · *\<type\> layer (no Scrim equivalent)* · *Instance whose master is not in the file* · *Boolean operation operands — the shape is drawn from its flattened result*
- *\<kind\> variable (only colour variables become tokens)* · *Colour variable without a value Scrim can read (an alias?)* (both count 0)

**resize:** *Constraints (centre, right, stretch, scale) — exact at this size; resizing follows CSS’s approximation*

**Image failure reasons** (`report.images.failed[].reason`): *the image is not in the file* · *the image bytes do not match their hash* · otherwise the pipeline's own message.

`style.ts` also holds the string *Non-solid fills (gradients) — not yet mapped*, in `solidFills`, which nothing calls (dead code).

## 7. Divergences, silent drops and quirks

### 7.1 The design doc and the code disagree

`docs/research/fig-fidelity.md`'s mapping table predates the build. The code is what ships, and it is what `cases/fig-import.json` records.

| Feature | The doc says | The code does |
|---|---|---|
| Layout grids | Scrim layout guides, exact | **not read, and not reported**: the sample's 57 grids were dropped silently |
| Image CROP / TILE | `background-size` / `-position` / `-repeat`, approx | `object-fit: cover`, approx |
| An image with other fills | `background-image` + `background-size` | an image node, other fills as its background, approx |
| Masks | SVG `<clipPath>` / `<mask>`, or `mask-image` with a data URI | a clipping wrapper frame: `overflow: hidden` + radius or `clip-path: path()` |
| Flips | `scale(-1, 1)` | `rotate(θ) scaleY(-1)` (equivalent) |
| Auto-layout grid | CSS grid tracks, exact | equal tracks, approx |
| Paint blend mode | `background-blend-mode`, exact | not applied, approx |
| Text auto-resize | `fit-content` | the size omitted |
| Internal page | skipped "except a master an instance needs" | skipped whole; such instances become detached copies |

### 7.2 Silent drops and fidelity gaps (no report line)

**Probed by running the importer:**
- **A rotated or flipped child of auto layout** is drawn unrotated in its flow slot. Probed: a 45° rectangle in a HORIZONTAL frame came out as `{ flexShrink: 0 }`, `{ width: 20, height: 20 }`, with an empty report.
- **The children of a frame with an INSIDE stroke shift by the stroke width.** The stroke is a CSS `border`, and absolute children are placed from the padding edge, inside the border. Probed: a 4 px inside stroke on a free frame, with its child at `left: 0; top: 0`; rendered, that child sits 4 px in. The same goes for flex content (the border eats the content box). Figma's inside stroke overlaps the content without moving it. This wasn't measured in a browser for this doc.
- **A stroke on TEXT** is ignored.

**From reading the code:**
- A flipped root (rotation 0, det < 0) is drawn unflipped.
- An image paint's opacity, crop transform and image filters are ignored.
- Variants, component properties, instance swap and slots are flattened silently.
- `backgroundPaints`, min / max width and height, `counterAxisAlignContent`, hyperlinks, text on a path, prototypes, comments, plugin data, export settings and style ids are not read (the last group by design).
- A record without a GUID or type, or whose parent is missing, disappears without a count.

### 7.3 Report quirks a port may fix, or must match if it compares against Scrim's numbers

- **The colour-token item adds 1 to `approximated`:** it is keyed as a pseudo-layer, `"tokens"`. Stage C's 6 includes it.
- **Skipped variable items have count 0,** so the summary's skipped total excludes variables.
- **"Vector without geometry" also fires for a vector that has geometry but only non-drawable paints:** stage D's "Image vector" appears under both items.
- **With several strokes on a box, the kept colour is the first SOLID in the array**, Figma's bottom-most, while the note says *the top one kept* (`style.ts › boxStroke`).
- **The CLI marks resize items with `-`,** the same as skipped.
- **Roboto Flex is in Scrim's system-font list,** although it is a Google Font, so its note says "system font".
