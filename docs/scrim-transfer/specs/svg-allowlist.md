# SVG: the allow-list and its security rules

**For:** the Rust fork of FigCraft, re-implementing Scrim's behaviour from this spec and `../cases/svg.json` alone.
**Source of truth (Scrim, commit `f2b1bdb`):** `packages/core/src/svg.ts` (inline vector content), `packages/core/src/patch.ts` (`checkSvg`), `packages/core/src/validate.ts` (`bad_svg`), `packages/core/src/vector/path.ts` and `vector/box.ts` (path edit mode), `packages/mcp/src/assets/sniff.ts` (imported `.svg` files), `packages/mcp/src/assets/importer.ts`, `apps/desktop/src/main/protocol.ts` (serving), CLAUDE.md rule 18, ARCHITECTURE.md › "Inline SVG on a node" and › "Imported images".
**Cases:** `../cases/svg.json`, 178 cases, every expectation produced by running Scrim's code (generator kept outside the repo).

## 0. Two different things called "SVG"

Scrim has two SVG surfaces, and they follow different policies. Keep them apart in the fork.

| | **Inline vector content** (layer `inline`) | **Imported `.svg` files** (layer `file`) |
|---|---|---|
| What it is | The optional `svg` field on a `box` node (or an `instance` of one): **structured JSON data**, never markup. | An image file brought in through the picker, a drop or a paste, stored in the content-addressed asset store and shown by an `image` node whose `src` is `scrim-asset:<sha256>.svg`. |
| Who writes it | Agents (`create_node` / `update_node`), the `.fig` import, the vector tools (Line, Pencil, Pen), Convert to line, paste. | A person importing a file. |
| How it is checked | A **strict allow-list** over the JSON (`svgContentIssues`). Anything not on the list is refused. | A **deny-list text sniff** over the file's text (`svgRefusal` and `sniffSvg`). It looks for script, handlers, entities and external references. |
| How it reaches the screen | Markup is **generated** from the data (`svgTree` → `svgMarkup` / JSX / the canvas). Nothing is stored as markup, so there is nothing to sanitise on output. | The file's **exact bytes** are stored and drawn only as `<img src="scrim://app/__assets/<hash>.svg">`, which runs no script and loads nothing. If the file is opened directly, the protocol adds the CSP `default-src 'none'; style-src 'unsafe-inline'`. |
| Policy | **Refuse, never sanitise.** Nothing is repaired; the generated markup only normalises spelling (§3.4). | **Refuse, never sanitise** (rule 18). The stored file is byte-for-byte what was imported. |
| Is a file ever converted into inline content? | **No.** No code path parses SVG markup into the `svg` field. A `.svg` file always stays an image asset. | — |

Rule 18 (CLAUDE.md), verbatim: *"An SVG that could run script or reach outside itself is refused, never sanitised."* Rule 18 is about the file layer. The inline layer obeys the stronger form of it: only data, never markup.

---

## 1. Inline vector content: the data model

```
SvgContent = { viewBox: [x, y, width, height], children: Shape[], defs?: Def[] }

Shape = Common & one of
  { tag: "path",    d: string }
  { tag: "rect",    x, y, width, height: number, rx?, ry?: number }
  { tag: "ellipse", cx, cy, rx, ry: number }
  { tag: "circle",  cx, cy, r: number }
  { tag: "line",    x1, y1, x2, y2: number }
  { tag: "g",       children: Shape[] }

Common (all optional, on every shape including g):
  fill, stroke             : Paint
  fillOpacity, strokeOpacity, opacity : number in [0, 1]
  fillRule                 : "nonzero" | "evenodd"
  strokeWidth              : Num ≥ 0
  strokeLinecap            : "butt" | "round" | "square"
  strokeLinejoin           : "miter" | "round" | "bevel"
  strokeDasharray          : Num[] (1–16 items, each ≥ 0, not all 0)
  vectorEffect             : "non-scaling-stroke"     (the only value)
  transform                : [a, b, c, d, e, f]       (six Num; emitted as matrix())
  clipPath                 : id of a clipPath def     (bare id, NOT url(#id))
  mask                     : id of a mask def         (bare id, NOT url(#id))

Def = one of
  { tag: "linearGradient", id, x1, y1, x2, y2: Num, units?, transform?: [6 Num], stops: Stop[] }
  { tag: "radialGradient", id, cx, cy, r: Num, fx?, fy?: Num, units?, transform?, stops: Stop[] }
  { tag: "clipPath",       id, children: Shape[] }
  { tag: "mask",           id, children: Shape[] }
  units: "userSpaceOnUse" | "objectBoundingBox"   (emitted default: userSpaceOnUse)

Stop = { offset: number in [0, 1], color: Colour, opacity?: number in [0, 1] }
```

Key names are camelCase **data** names, never SVG attribute spellings: `strokeWidth` is allowed, `stroke-width` is refused. Shapes carry **no `id`**; only defs do.

### 1.1 Allowed elements (generated), exactly

`svg`, `path`, `rect`, `ellipse`, `circle`, `line`, `g`, `defs`, `linearGradient`, `radialGradient`, `stop`, `clipPath`, `mask`. That is 13 (`SVG_ELEMENTS`). Of these, only `path rect ellipse circle line g` may appear as a shape `tag`. `defs` and `stop` are implied by the structure, and `svg` is the generated root.

Everything else is refused as a shape tag, including `script`, `foreignObject`, `use`, `image`, `text`, `style`, `a`, `polyline`, `polygon`, `set`, `animate*`, a nested `svg`, `symbol`, `marker`, `pattern` and `filter`. A def tag other than the four is refused.

### 1.2 Allowed keys, exactly

| Object | Allowed keys (any other key is refused) | Required |
|---|---|---|
| top level | `viewBox`, `children`, `defs` | `viewBox`, `children` |
| path | `tag`, `d`, + Common | `d` |
| rect | `tag`, `x`, `y`, `width`, `height`, `rx`, `ry`, + Common | `x`, `y`, `width`, `height` |
| ellipse | `tag`, `cx`, `cy`, `rx`, `ry`, + Common | all four |
| circle | `tag`, `cx`, `cy`, `r`, + Common | all three |
| line | `tag`, `x1`, `y1`, `x2`, `y2`, + Common | all four |
| g | `tag`, `children`, + Common | `children` |
| linearGradient | `tag`, `id`, `x1`, `y1`, `x2`, `y2`, `units`, `transform`, `stops` | `x1 y1 x2 y2`, `id`, `stops` |
| radialGradient | `tag`, `id`, `cx`, `cy`, `r`, `fx`, `fy`, `units`, `transform`, `stops` | `cx cy r`, `id`, `stops` |
| clipPath / mask | `tag`, `id`, `children` | `id`, `children` |
| stop | `offset`, `color`, `opacity` | `offset`, `color` (an absent value fails its type check) |

Common = `fill fillOpacity fillRule stroke strokeWidth strokeOpacity strokeLinecap strokeLinejoin strokeDasharray vectorEffect opacity transform clipPath mask` (14 keys).

Unknown keys are matched by own enumerable property name, so `__proto__` that arrives as an own JSON property is refused like any other key (case "a __proto__ key"). There is no event-handler rule, `href` rule or `style` rule as such. An `onload`, `href`, `xlinkHref`, `style`, `class` or `id` key is simply not on the list.

### 1.3 Value rules

- **Num** (every geometry number, `transform` entries, dash entries, gradient geometry, viewBox entries): `typeof number`, finite, and `|v| ≤ 10_000_000`. A numeric string (`"4"`) is refused. NaN and ±Infinity are refused; JSON cannot carry them, so the cases leave them out. Signs are **not** checked, except that `strokeWidth ≥ 0`, dash entries are `≥ 0`, and viewBox width and height are `> 0`. A negative `r`, `rx` or gradient `r` is accepted.
- **Unit numbers** (`fillOpacity`, `strokeOpacity`, `opacity`, stop `offset`, stop `opacity`): `typeof number`, finite, `0 ≤ v ≤ 1`.
- **viewBox:** an array of exactly 4 Num with `[2] > 0` and `[3] > 0`.
- **Paint** (`fill`, `stroke`) is a string and one of:
  1. `url(#ID)`, matched against the **untrimmed** value by `^url\(#([A-Za-z][A-Za-z0-9_-]{0,63})\)$`. It must name a `linearGradient` or `radialGradient` def **in this same svg**. No quotes and no spaces are allowed: `url('#g')` and ` url(#g)` are refused.
  2. Otherwise, the **trimmed** value must match COLOUR (below).
- **COLOUR** (JS regex, case-sensitive, applied to the trimmed string):
  ```
  ^(#[0-9a-fA-F]{3}|#[0-9a-fA-F]{4}|#[0-9a-fA-F]{6}|#[0-9a-fA-F]{8}
   |rgba?\(\s*\d{1,3}(\.\d+)?\s*,\s*\d{1,3}(\.\d+)?\s*,\s*\d{1,3}(\.\d+)?\s*(,\s*(0|1|0?\.\d+)\s*)?\)
   |none|currentColor|transparent)$
  ```
  The consequences are exact and the fork must reproduce them:
  - Named colours (`red`), `hsl()`, `var()`, percentages and `NONE` / `CurrentColor` / `RGB(` are refused.
  - `rgb(999, 999, 999)` is accepted (no range check). `rgb()` with a 4th alpha argument is accepted, and so is `rgba()` with 3 arguments.
  - The alpha is `0`, `1` or `.d+` / `0.d+`, so `1.0` and `0.` are refused.
- **Stop colour:** a string that does not start with `url(` and whose trimmed value matches COLOUR (so `none`, `currentColor` and `transparent` pass).
- **clipPath / mask references:** a string equal to the id of a def whose tag is `clipPath` (or `mask`) in this svg.
- **Def ids:** `^[A-Za-z][A-Za-z0-9_-]{0,63}$` (a letter, then up to 63 of letters, digits, `_` and `-`). Ids are unique across all defs. A duplicate is reported at the **second** occurrence.
- **Enums** are exact strings: `fillRule`, `strokeLinecap`, `strokeLinejoin`, `vectorEffect`, `units`.
- **strokeDasharray:** an array, 1–16 items, every item Num and `≥ 0`, at least one `> 0`.
- **transform** (shape or gradient): an array of exactly 6 Num. A CSS string is refused.

### 1.4 Path data validation (write time)

`d` must satisfy all of the following, in this order. The first failure is the issue for that path:

1. It is a string, and it matches `^[MmLlHhVvCcSsQqTtAaZz0-9eE.,+\-\s]*$`, and it matches `^\s*[Mm]`. Otherwise: `<at>.d must be path data (M, L, H, V, C, S, Q, T, A, Z and numbers), starting with M`. `\s` is the **JavaScript** whitespace class: it includes U+0009–U+000D, U+0020, U+00A0, U+1680, U+2000–U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 **and U+FEFF**. Rust's `char::is_whitespace` lacks U+FEFF, so spell the set out.
2. `d.length ≤ 200_000`, counted in UTF-16 code units (all allowed characters are ASCII except the Unicode whitespace). Otherwise: `<at>.d is longer than 200000 characters`.
3. It does not match `/[eE][eE]|[+-]{2,}|[eE](?![+-]?\d)|[+-](?![\d.])/`. Otherwise: `<at>.d has a malformed number`.
4. A path that passes adds `d.length` (untrimmed) to the node total. If the total is `> 1_000_000`: `svg path data totals more than 1000000 characters`.

This is **not a parser**. It accepts data that has the right characters but the wrong grammar: `M0 0 1` (odd argument count), `M0 0A1 1 0 2 1 5 5` (arc flag 2), `M0 0L1e5e5 1` (a doubled exponent), and `M0 0L1 1Z 2 2`. Each is accepted on write and refused by the path-edit parser (§4). The `pathEdit` cases carry `writeIssues` to show both verdicts. A Rust port that validates with a real parser would be stricter than Scrim.

### 1.5 Caps

| Cap | Value | Constant | Counted how |
|---|---|---|---|
| Elements | 5,000 | `MAX_SVG_ELEMENTS` | Every entry of every shape list (top-level children, group children, clipPath and mask children), groups included, counted **before** the entry is type-checked. Defs and stops don't count. On the 5,001st, the issue `svg has more than 5000 elements` is pushed and that list stops being walked. |
| Nesting | 16 | `MAX_SVG_DEPTH` | The top-level `children` list is depth 0, a clipPath's or mask's `children` list is depth 1, and each `g` adds 1. A list at depth `> 16` gives `<at>: groups nest deeper than 16`, and its entries are not examined. So 16 nested `g` pass and 17 fail. |
| One path's `d` | 200,000 chars | `MAX_SVG_PATH_CHARS` | §1.4 step 2. |
| All paths of a node | 1,000,000 chars | `MAX_SVG_TOTAL_CHARS` | §1.4 step 4. Only paths that passed every check are summed. |
| Stops per gradient | 64 (and ≥ 1) | `MAX_SVG_STOPS` | |
| Dash values | 16 (and ≥ 1) | `MAX_SVG_DASHES` | |
| Any Num | ±10,000,000 | (`MAX_ABS`) | |
| Def id | 64 chars | — | |

There is no cap on the number of defs.

### 1.6 Which nodes, and when the checks run

- Only `type: "box"` or `type: "instance"` may hold `svg`. Any other type gives `only a box (or an instance of one) holds svg content (this is a <type>)` (no other issue is listed). An absent `svg` (`undefined`) is always fine.
- The checks run inside `applyPatch` on `add_node`, on every node of `add_subtree`, and on `update_node` **when `set.svg` is present**. `update_node` with `svg: null` clears the field (whole-value LWW, one undo step). An `add_node` whose node carries `svg: null` is refused (`svg must be an object …`).
- `validateDocument` (load, MCP, tests) runs the same checks and reports one `{ code: "bad_svg", message: "<nodeId>: <issue>", nodeId }` per issue.
- The checks are pure, and they **collect every issue** in a fixed order: top-level unknown keys, then viewBox, then `defs must be a list` and duplicate ids (a first pass over defs), then each def in order, then the children tree depth-first in document order, then the total-characters issue.

### 1.7 Refusal messages, verbatim

`<at>` is a path such as `svg.children[1].children[0]` or `svg.defs[0].stops[2]`. `<tag>` is the shape's tag, and `<k>` is the key.

```
svg must be an object { viewBox, children, defs? }
svg has no "<k>" (it takes viewBox, children, defs)
svg.viewBox must be [x, y, width, height] with a positive width and height
svg.defs must be a list
svg.defs[<i>]: the id "<id>" is used twice
svg.defs[<i>].tag must be linearGradient, radialGradient, clipPath or mask
svg.defs[<i>]: a <deftag> has no "<k>"
svg.defs[<i>].id must be a letter then letters, digits, _ or - (at most 64)
svg.defs[<i>].<k> must be a finite number
svg.defs[<i>]: a <deftag> needs <k>
svg.defs[<i>].units must be userSpaceOnUse or objectBoundingBox
svg.defs[<i>].transform must be six numbers [a, b, c, d, e, f]
svg.defs[<i>].stops must be a list of 1 to 64 stops
svg.defs[<i>].stops[<j>] is { offset, color, opacity? }
svg.defs[<i>].stops[<j>].offset must be a number from 0 to 1
svg.defs[<i>].stops[<j>].color must be a colour
svg.defs[<i>].stops[<j>].opacity must be a number from 0 to 1
<at> must be a list
<at>: groups nest deeper than 16
svg has more than 5000 elements
<at>[<i>] must be an object
<at>[<i>].tag "<tag, first 30 chars>" is not allowed: an svg holds path, rect, ellipse, circle, line and g
<at>[<i>]: a <tag> has no "<k>"
<at>[<i>]: a <tag> needs <k>
<at>[<i>].<k> must be a finite number
<at>[<i>].d must be path data (M, L, H, V, C, S, Q, T, A, Z and numbers), starting with M
<at>[<i>].d is longer than 200000 characters
<at>[<i>].d has a malformed number
<at>[<i>].<fill|stroke> must be a colour, none or url(#id)                       (not a string)
<at>[<i>].<fill|stroke> refers to "<id>", which is not a gradient defined in this svg
<at>[<i>].<fill|stroke> must be a hex or rgb() colour, none, currentColor or url(#id) (got "<value, first 40 chars>")
<at>[<i>].<fillOpacity|strokeOpacity|opacity> must be a number from 0 to 1
<at>[<i>].strokeWidth must be a number from 0
<at>[<i>].fillRule must be nonzero or evenodd
<at>[<i>].strokeLinecap must be butt, round or square
<at>[<i>].strokeLinejoin must be miter, round or bevel
<at>[<i>].strokeDasharray must be 1 to 16 numbers from 0, not all 0
<at>[<i>].vectorEffect must be non-scaling-stroke
<at>[<i>].transform must be six numbers [a, b, c, d, e, f]
<at>[<i>].clipPath must be the id of a clipPath defined in this svg
<at>[<i>].mask must be the id of a mask defined in this svg
svg path data totals more than 1000000 characters
only a box (or an instance of one) holds svg content (this is a <type>)
```

The two `<…> needs <k>` messages fire when the key is `undefined`. A shape with an unknown tag gets only the tag message; nothing else about it is checked. For a stop with an extra key, only `is { offset, color, opacity? }` is reported. `<value, first 40 chars>` and `<tag, first 30 chars>` are JS `slice` truncations (UTF-16 units).

**The message a writer sees** (PatchError, returned verbatim as an MCP tool error):
`<node.name, or node.id when the name is empty>: <the first 5 issues joined by "; ">` + (when there are more than 5) ` (and <n − 5> more)`.
Example, from case "seven problems at once": `Icon: svg has no "style" (it takes viewBox, children, defs); svg.viewBox must be …; … (and 2 more)`. Nothing is written.

---

## 2. URL and reference rules (both layers)

| Construct | Inline (data) | Imported file |
|---|---|---|
| `href` / `xlink:href` attribute | **No such key.** No shape or def takes one. | Refused when its value (after optional quote and whitespace) starts `http:`, `https:`, `file:`, `javascript:` or `//` (case-insensitive) → "SVG references an external file". A local `#id`, a **relative** path (`other.svg#a`) and a `data:` URI are accepted. |
| `src` attribute | No such key. | The same rule as `href`. |
| `url()` in paint | Only `url(#localId)` naming a gradient in the same svg. | In any text (`style` attributes, `<style>`, presentation attributes): refused when the url starts `http:` / `https:` / `file:` / `javascript:` / `//` after an optional quote and whitespace. Local, relative and `data:` urls are accepted. |
| `clipPath` / `mask` reference | The bare local id of a def of that kind. | Not inspected beyond the url rule. |
| External file (`<use href="x.svg#i">`) | `use` is not an element. | Refused only when absolute or protocol-relative, as above. |
| `data:` URI | Impossible: no key holds one. | **Accepted** (self-contained). |
| `javascript:` | Refused (a paint fails COLOUR). | Refused in `href` / `src` / `url()` when spelled literally. A character-reference-encoded form (`&#106;avascript:`) and a SMIL `<set to="javascript:…">` are **not** detected (see §6). |
| `@import` | n/a | Refused anywhere in the text, case-insensitive → "SVG imports a stylesheet". |

## 3. Style, script, events and generated markup

### 3.1 Inline

- There is no `style` key, no `class`, no `<style>` element and no CSS at all. Paint is attribute data only.
- There is no script element and no event-handler key (none is on the list). The rule is the allow-list, not a name pattern.
- The generated root always carries `style="display:block"`. That is the only `style` attribute Scrim ever emits, and it is a constant.

### 3.2 Imported file

The text sniff refuses (the first match wins, in this order, over the **whole** text including the prolog and comments):

| # | Test (JS regex) | Reason |
|---|---|---|
| 1 | `/<!ENTITY/i` | `SVG declares entities` |
| 2 | `/<(?:[a-z0-9_-]+:)?script[\s>/]/i` | `SVG contains a script element` |
| 3 | `/@import/i` | `SVG imports a stylesheet` |
| 4 | `/url\(\s*['"]?\s*(?:https?:\|file:\|javascript:\|\/\/)/i` | `SVG references an external file` |
| 5 | for each tag `/<[a-z][^>]*>/gi`: `/\son[a-z]+\s*=/i` | `SVG has an event handler attribute` |
| 6 | the same tag: `/\s(?:[a-z0-9_-]+:)?(?:href\|src)\s*=\s*["']?\s*(?:https?:\|file:\|javascript:\|\/\/)/i` | `SVG references an external file` |

The message is `<reason>: Scrim imports only self-contained SVGs`, with code `unsafe_svg`. Being a text match, a `<script>` inside a comment is still refused (case "a script element inside a comment").

`<style>` elements, `style` attributes, CSS `class`es, SMIL animation and `<foreignObject>` content are **allowed** in a file unless they trip one of the six tests. DOCTYPEs are allowed unless they declare an entity.

### 3.3 What makes an imported file safe to show anyway

The deny-list is one layer. The others are:

- Scrim only ever draws an imported SVG as an `<img>`. In image mode the browser runs no script and fetches nothing.
- The `scrim://app/__assets/` protocol serves only names matching `^[a-f0-9]{64}(\.preview)?\.(png|jpg|webp|gif|avif|svg)$`, with `Content-Type: image/svg+xml`, `X-Content-Type-Options: nosniff` and, for `.svg`, `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'`.
- The app's own CSP (`img-src 'self' data: blob: scrim:`) is never loosened.

A Rust port that renders imported SVGs itself, with its own SVG renderer rather than a browser's image mode, needs the equivalent: no script, no external fetch, no `foreignObject` HTML. Otherwise the gaps in §6 become real.

### 3.4 What is normalised (inline only; all of it is output spelling)

Nothing in the stored data is changed. The generator (`svgTree`) prints:

- **The root:** `<svg xmlns="http://www.w3.org/2000/svg" viewBox="x y w h" width="100%" height="100%" preserveAspectRatio="none" overflow="visible" style="display:block">`, in that order.
- **Element order:** `<defs>` first (only when there are defs), then the shapes in order.
- **Numbers:** `String(Math.round(v * 1000) / 1000)`, i.e. at most 3 decimals in JS shortest form (`0.333`, `2.667`). `-0` prints as `0`. Math.round rounds .5 toward +∞ (`-0.0005` → `-0` → `"0"`). Every number is within ±10⁷, so there is never an exponent.
- **Paint and stop colours:** printed trimmed. Path `d` is printed trimmed and otherwise verbatim (not re-serialised). Case: "whitespace … is trimmed".
- **Local ids:** scoped to the node as `<scope>-<id>`, with `scope = "svg-" + nodeId.replace(/[^A-Za-z0-9_-]/g, "_")`. `url(#g)` becomes `url(#svg-v-g)`, `clipPath: "c"` becomes `clip-path="url(#svg-v-c)"` and `mask: "m"` becomes `mask="url(#svg-v-m)"`.
- **Attribute order on a shape:**
  1. its geometry, in the key order of §1.2 (for a path, `d`);
  2. `fill`, then `stroke`;
  3. `fill-opacity`, `fill-rule`, `stroke-width`, `stroke-opacity`, `stroke-linecap`, `stroke-linejoin`, `opacity`;
  4. `stroke-dasharray` (space-joined numbers), `vector-effect`, `transform="matrix(a b c d e f)"`, `clip-path`, `mask`.

  An attribute is printed only when its value is set.
- **Gradients:** `id`, the geometry (`x1 y1 x2 y2`, or `cx cy r fx fy` when set), then `gradientUnits` (**always printed**, defaulting to `userSpaceOnUse`), then `gradientTransform="matrix(…)"`. Stops are `offset`, `stop-color`, then `stop-opacity` when set.
- **Escaping:** in attribute values, `&` → `&amp;`, `"` → `&quot;` and `<` → `&lt;`. Indentation is two spaces per level. An element without children self-closes as `<tag …/>`.
- **JSX spelling:** `class` becomes `className`, and kebab-case becomes camelCase (`fill-rule` → `fillRule`).

The same tree feeds the canvas, `get_jsx`, `get_svg`, standalone HTML and the exports, so they can't disagree. Every accepted inline case in `svg.json` carries its exact `markup`.

### 3.5 Imported-file normalisation

None. SVG files skip the steps that change raster files: there is no end-of-image trim (`imageEnd` returns `unchecked` for SVG), no location stripping and no decode check (`decodeChecker` returns SVGs untouched, because "SVGs are text"). The stored bytes are the imported bytes, and the hash is their sha256. The size comes from the header (§5).

---

## 4. Path edit mode (where path data *is* parsed)

Path edit mode (`parsePath`, `serializePath`) is used only when a person edits a single-path vector, never on write. It **never repairs**: data it can't parse returns `null`, and the path can't be edited. It shows no message; the cases record this as `"parsePath: null"`.

- **Who can enter:** `vectorEditRefusal(svg)` must be null. The svg needs exactly one child, no defs, and the child must be a `path` or `line` with no `transform`, `clipPath` or `mask`. The messages, verbatim:
  - `This layer has no vector content`
  - `Only a single path can be edited (this vector has <n> shapes)`
  - `Only a path or a line can be edited (this is a <tag>)`
  - `Only a plain path can be edited (this one has a transform, a clip, a mask or definitions)`

  A path with arcs says `<n> arc(s) will become curves when you edit this path (the shape stays within 0.01 px)` on entry.
- **Tokens:** separators are space, comma, `\n`, `\t` and `\r` (not the full JS `\s` set). Numbers match `[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?`. Arc flags (arc arguments 4 and 5) are a single `0` or `1` character, so `a1 1 0 011 1` parses. Any other character returns null.
- **Grammar:** each command takes N×arity numbers (M 2, L 2, H 1, V 1, C 6, S 4, Q 4, T 2, A 7, Z 0). After M, further pairs are L (l after m). Numbers after Z, a short argument list, a stray letter, or a first segment that isn't M all return null.
- **Normalisation:** relative commands become absolute; H and V become L; S becomes C (reflected c1); T becomes Q (reflected c1); A becomes cubics (SVG F.6.5 endpoint-to-centre conversion, split into pieces of at most 90°, each with `k = 4/3·tan(θ/4)`). An arc with a zero radius becomes L, and an arc to its own start point is dropped. Q is kept as Q by the parser. The output alphabet is `M L C Q Z`.
- **Serialisation:** absolute, no separators between segments, a space between numbers, each number rounded to 0.01 (`Math.round(v*100)/100`, `-0` → `0`). For example `M10 20L30 40C1 2 3 4 5 6Q7 8 9 10Z`.

---

## 5. Imported files: the full gate (`sniffImage`), in order

1. **Size:** `bytes > 25 × 1024 × 1024` gives `too_large`: `Images are limited to 25 MB (this one is <(n/1048576).toFixed(1)> MB)`.
2. **Raster magic bytes** (PNG, JPEG, GIF, WebP, AVIF) are tried first, so a file that starts like a PNG is never treated as SVG.
3. **SVG sniff:**
   - **Bytes:** a NUL byte in the first 1,024 bytes means not SVG, and so does UTF-8 that doesn't decode strictly. A leading U+FEFF is dropped.
   - **Entities first:** if the text contains `<svg[\s>/]` anywhere and `<!ENTITY` (case-insensitive) anywhere, the file is refused at once with `SVG declares entities: Scrim imports only self-contained SVGs`. This runs before the root is found, because an internal DTD subset would otherwise hide it.
   - **The root:** skip whitespace, `<?…?>`, `<!--…-->` and `<!doctype …>` (case-insensitive, up to the first `>`). An unterminated construct means no root. The next thing must match `<svg[\s>/]`, lower-case and unprefixed. Otherwise the file is not SVG.
   - **The deny-list:** §3.2's six tests.
   - **The size, from the root tag** (up to its first `>`). Attributes are read only when quoted, by `\s<name>\s*=\s*("…"|'…')`, case-insensitively. A length is `number[unit]`, where the unit is px, pt (4/3), pc (16), in (96), cm (96/2.54) or mm (96/25.4). `%`, `em` and anything else count as unknown. A viewBox is split on `[\s,]+` and must be 4 finite numbers with a positive width and height.
     - Width and height both unknown, and a viewBox: take the viewBox's size.
     - Exactly one of them known, and a viewBox: derive the other by the viewBox's ratio.
     - Each side is then `max(1, round(v))` when it is known and non-zero, or 0 otherwise.
4. **Neither raster nor SVG:** a video is `video`: `Video import is planned and not available yet`. Anything else is `unsupported`: `Not a supported image: Scrim imports PNG, JPEG, WebP, GIF, AVIF and SVG`.
5. **Pixel cap:** `width > 16384`, `height > 16384` or `width × height > 100_000_000` gives `too_many_pixels`: `The image is <w> × <h> px: the limit is 16384 px a side and 100 megapixels` (`×` is U+00D7). An SVG of unknown size (0 × 0) passes.
6. **The batch:** a picker batch whose files total over 64 MB imports nothing (`batch_too_large`; not exercised in `svg.json`).

There is **no element-count, depth or path-length cap on imported files**: 50,000 nested `<g>` import fine. The only size cap is 25 MB.

---

## 6. Gaps: accepted file inputs that arguably "reach outside" (report, not fixed)

Each of these is a `file (gap): …` case in `svg.json`, accepted by Scrim today. The `<img>` display and the CSP make them harmless in Scrim. A port that renders SVG another way, or exports the file into a page as inline markup, must refuse them or neutralise them by construction.

1. An `<?xml-stylesheet href="https://…"?>` processing instruction. Tags are matched by `<[a-z]…`, and PIs start `<?`.
2. A `<!DOCTYPE svg SYSTEM "https://…">` external DTD. The test file's own `PUBLIC … "http://www.w3.org/…dtd"` is meant to pass.
3. Character-reference-encoded schemes: `href="&#106;avascript:…"` and `href="&#x68;ttps://…"`.
4. SMIL `<set attributeName="href" to="javascript:…"/>` (and `values=` on `<animate>`).
5. Relative external references: `<use href="other.svg#a"/>` and `url(other.svg#g)`.
6. `<foreignObject>` holding XHTML. An `<iframe srcdoc="…">` is accepted, while an `<iframe src="https://…">` is refused.
7. `<svg/onload=…>`. The handler rule needs whitespace before `on`. The file is not well-formed XML, so browsers don't run it as SVG.
8. **There is no structural cap** (depth, element count) on files.

Two more consequences of the text sniff, both refusals:

- **Real-world false positive:** Adobe Illustrator exports declare namespace entities (`<!ENTITY ns_svg …>`). Every such file is refused with `SVG declares entities: …` (case "an Adobe Illustrator export …").
- **A DOCTYPE with an internal subset but no ENTITY** (for example `<!ATTLIST …>`) is refused as `unsupported`, not `unsafe_svg`: root-finding stops at the subset's first `>`.

## 7. Where the cases come from, and how to run them

`../cases/svg.json` › `conventions` explains every field. In short:

- **`inline`:** write `svg` (materialised from `generate` when present) on a node of `nodeType` (default `box`) with id `v` and name `Icon` in a new document. Expect `accept`, plus the exact `markup` for small cases. Otherwise the verbatim PatchError in `expect.refuse`, with the full unprefixed `issues` list. `normalised` is the generated markup.
- **`file`:** run the import gate on the UTF-8 bytes of `svg` (or the raw `bytes`). Expect `accept` with `sniffed` `{ ext, width, height, orientation }`, or `expect.refuse` with `code`. `svgRefusal` is the bare reason.
- **`pathEdit`:** parse and serialise `svg` (a `d` string). `writeIssues` is the write-time verdict on the same `d`.
- **`editRefusal`:** `vectorEditRefusal(svg)`.

Counts: 178 cases.

| Layer | Accepted | Refused | Normalised |
|---|---|---|---|
| `inline` | 15 | 82 | 2 |
| `file` | 23 (11 of them gaps) | 34 | — |
| `pathEdit` | — | 6 | 11 |
| `editRefusal` | 1 | 4 | — |
