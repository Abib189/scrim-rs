# .fig import: Scrim's design, for a Rust port

**What this is:** the design of Scrim's Figma `.fig` importer (`packages/fig-import`, about 1,800 lines of TypeScript, no third-party code), written so that a Rust fork can rebuild it from this page and its companions without reading the TypeScript.

**What Scrim built, in one line:** a `.fig` is decoded with an *interpreting* Kiwi reader under hard limits. Its layers become plain HTML elements, CSS declarations and generated inline SVG, applied as ONE patch (one undo step), together with a fidelity report that names every approximation and skip. Nothing Figma-specific stays in the document.

**Companion files:**

| File | Holds |
|---|---|
| `specs/fig-import-mapping.md` | the Figma → output mapping table: exact, approximate, resize or skipped, the reason, and the CSS each one defines |
| `specs/fig-import-security.md` | every limit with its value and its refusal message verbatim, what is *not* limited, and the bugs found while probing |
| `cases/fig-import.json` | expectations produced by running Scrim's importer: the 5 fixtures (each node, the report, 140 checked asserts), 62 hostile inputs (56 embedded as base64), 3 image-report cases and 10 host-level refusals |
| `fixtures.md` | every fixture in the Scrim repo a port can use, with its provenance |

**Scrim sources (for reference only):**
- the package: `packages/fig-import/src/` (`container.ts`, `kiwi.ts`, `read.ts`, `tree.ts`, `build.ts`, `layout.ts`, `style.ts`, `paint.ts`, `text.ts`, `vector.ts`, `lines.ts`, `components.ts`, `report.ts`, `importFig.ts`);
- the docs: `docs/research/fig-fidelity.md`, `ARCHITECTURE.md` › *Figma .fig import*, `docs/design/vector-tools.md` §6.

Everything here was checked against Scrim at commit `f2b1bdb`.

## 1. The contract

| Entry point | Input | Output |
|---|---|---|
| `readFig(bytes, limits?)` | the file's bytes | `{ version, nodeChanges, blobs, images }`, Figma's raw records. Throws `FigError` |
| `importFig(bytes, baseDoc, { fileName, importImage, limits? })` | bytes, the document the patch applies to, and the host's image pipeline | `{ patch, report, roots, bounds }`. Throws `FigError` before any patch exists |
| `analyzeFig(bytes, fileName, limits?)` | bytes | the report alone. Images are hash-checked but never stored (the CLI's `import-fig`) |

- **The error type.** `FigError { code, message }`. The code is one of `too_large`, `not_fig`, `corrupt`, `limit`, `unsafe_entry`, `unsupported`, and the message is a user-facing sentence (`fig-import-security.md` lists all of them).
- **The image pipeline.** `importImage(layerName, bytes) → { src } | { error }` is the host's job (sniff, decode, EXIF / GPS stripping, content-addressed store); a refusal is reported, never thrown.
- **Measurement only.** `bounds` maps each created node to its Figma box within its artboard. It is used for measuring fidelity and never enters the document.

**It never:**
- writes a `.fig` or modifies the source file;
- evaluates anything from the file (the Kiwi schema is data, never compiled to code);
- follows a URL or reference;
- copies markup (every SVG is generated from numbers through an allow-list);
- puts Figma metadata (GUIDs, keys, plugin data, style ids) in the document.

**Where it runs in Scrim:**
- **The server's library host** imports into a new document and applies the result as one commit:
  - the desktop's File › Import passes the file's bytes;
  - a `.fig` dropped on Home arrives as base64 over the WebSocket, capped at 50 MB.
- **The CLI's `import-fig`** prints the report only.

## 2. The pipeline at a glance

| # | Stage | Input | Output | Scrim file |
|---|---|---|---|---|
| 1 | Container | file bytes | `canvas` (fig-kiwi bytes) + `images` (sha1 hex → bytes) | `container.ts › readArchive` |
| 2 | Chunks and decompression | `canvas` | `version`, `schema` bytes, `data` bytes | `container.ts › readChunks` |
| 3 | Kiwi decode | `schema`, `data` | the `Message` object: `nodeChanges[]`, `blobs[]` | `kiwi.ts`, `read.ts` |
| 4 | Tree | `nodeChanges` | `byId`, ordered `children`, `pages`, `internalPages` | `tree.ts` |
| 5 | Images | tree, blobs, zip images | sha1 → `src` or null, plus failures in the report | `build.ts › importImages` |
| 6 | Conversion | tree, images, blobs | per root, a list of plain nodes; `bounds`; master and instance records | `build.ts`, `layout.ts`, `style.ts`, `paint.ts`, `text.ts`, `vector.ts`, `lines.ts` |
| 7 | Components and patch | base document, built nodes | ONE patch: roots, components, instances, overrides, tokens | `components.ts` |
| 8 | Report | everything recorded on the way | `FigReport` | `report.ts` |

Stages 1–3 run before anything else, so every limit fails before any conversion and before any patch exists.

## 3. Stage 1: container

**Input:** the file bytes. **Output:** `canvas` (bytes starting `fig-kiwi`) and `images` (`Map<40-hex lowercase, bytes>`, not yet validated).

1. **Size first.** `file.length > fileBytes` (200 MiB) → `too_large`. In Scrim the hosts also check the size with `stat` before reading the file (`fig-import-security.md` §3).
2. **Bare fig-kiwi.** If the file is at least 12 bytes and starts with the 8 ASCII bytes `fig-kiwi`, it is the canvas itself, with no images. The 24.6 MB sample is this form; its images live in Kiwi blobs.
3. **Otherwise it must be a ZIP starting at offset 0.** The length must be ≥ 22 and the first u32 must be `0x04034b50`; otherwise `not_fig`. (A ZIP with leading data, such as a self-extractor, is refused.)
4. **The end-of-central-directory record.** It is found by scanning backwards from `len − 22` to `max(0, len − 22 − 65535)` for `0x06054b50`; none → `corrupt`. The reader takes the entry count (u16 at +10), the directory size (u32 at +12) and its offset (u32 at +16).
   - A count of `0xFFFF` or an offset of `0xFFFFFFFF` means ZIP64 → `unsupported`.
   - More entries than the cap (5000) → `limit`.
   - `offset + size > eocd` → `corrupt`.
5. **Each central-directory entry, in order.** The signature must be `0x02014b50` within bounds, else `corrupt`. The reader takes the flags (+8), method (+10), compressed size (+20), uncompressed size (+24), the name, extra and comment lengths (+28/+30/+32) and the local header offset (+42). The name is decoded as UTF-8, non-fatal.
   - **Every name is checked**, wanted or not, before any entry is read: unsafe → `unsafe_entry` (the rule is in `fig-import-security.md` §2).
   - Flag bit 0 (encrypted) → `unsupported`.
   - A name ending in `/` (a directory) is skipped.
   - **Wanted entries:** `canvas.fig`, `canvas`, and `images/` followed by 40 hex characters (case-insensitive). Every other entry (`meta.json`, `thumbnail.png`, …) is listed as ignored and **never inflated**, so its method is never checked.
   - **The local header** must have signature `0x04034b50` at the stated offset, else `corrupt`. The data starts at `local + 30 + localNameLen + localExtraLen` and runs for the *central* compressed size; past the end of the file → `corrupt` ("truncated").
6. **Inflate under a budget.** Each wanted entry is decompressed with `cap = min(entryBytes, budgetLeft)`, where the budget starts at `totalBytes` (768 MiB) for the archive.
   - A declared size over the cap → `limit`, before inflating.
   - Method 0 (stored) is taken as is; method 8 is raw deflate, with the output capped at `cap + 1`; any other method → `unsupported`.
   - Output over the cap → `limit`; a deflate error → `corrupt`.
   - An output length different from the declared size → `corrupt`.
   - The budget then drops by the output length.
7. **Results.** `images/<hex>` is stored under its lowercase hex. `canvas.fig` or `canvas` becomes the canvas (a later one wins). No canvas → `not_fig`; a canvas that doesn't start with `fig-kiwi` → `not_fig`.

**CRCs are never read**, in either header, so a CRC of 0 or a wrong CRC is accepted. The local header's sizes and name are ignored too: the central directory is authoritative. Integrity rests on three checks: the declared-size equality, the SHA-1 of each image, and the exact Kiwi decode. The committed zipped fixture (`stage-a.fig`) has CRC 0 in both headers of both entries; FigCraft rightly refuses it ("Invalid checksum"). See `cases/fig-import.json › zipCrc` for the real CRCs.

## 4. Stage 2: chunks and decompression

**Input:** `canvas`. **Output:** `version`, `schema`, `data` (decompressed).

- **Layout:** `fig-kiwi` (8 bytes), `version` (u32 LE at 8), then chunks, each a u32 LE length followed by that many bytes.
  - The version is read and returned but **never validated**; the fixtures and the sample are version 15.
- **Reading chunks:** up to 4 are read, from offset 12, while 4 more bytes remain.
  - A chunk running past the end → `corrupt` (*"canvas.fig is truncated (a chunk runs past the end)"*).
  - Fewer than 2 chunks → `corrupt`.
  - Chunk 0 is the schema and chunk 1 the data; chunks 2–3 and any bytes after the fourth are ignored.
- **Decompressing each chunk:** **zstd** if it starts `28 B5 2F FD`, else **raw deflate** (no zlib header). The output is capped at `min(entryBytes, budgetLeft) + 1`.
  - Over the cap → `limit`; a decoder error → `corrupt`.
  - **This stage starts a fresh 768 MiB budget**, separate from the archive's (`fig-import-security.md` §5).
- **On a cut zstd frame,** Node's zstd returns what it could decode without an error, so a truncated frame surfaces later as a Kiwi "ends early" error. A port's decoder may fail earlier, with the same code `corrupt`.

## 5. Stage 3: Kiwi decode (interpreting, never compiling)

**The format** is Evan Wallace's Kiwi (MIT). Scrim re-derived it and **interprets** the embedded schema as data; the reference runtimes compile it to code, which would run code derived from an untrusted file.

**The binary schema:**
- `varuint definitionCount` (cap 4096, `limit`);
- then for each definition: `cstring name`, `byte kind` (0 enum, 1 struct, 2 message; anything else `corrupt`), `varuint fieldCount` (cap 4096, `limit`);
- then for each field: `cstring name`, `varint type`, `byte isArray` (1 = array), `varuint value`.

**Types:**
- −1 bool, −2 byte, −3 int (zigzag varint), −4 uint (varint), −5 float, −6 string (NUL-terminated UTF-8), −7 int64 (zigzag varuint64), −8 uint64;
- `≥ 0` is a definition index.
- For struct and message definitions, a type below −8 or at least the definition count is refused (`corrupt`); enum "fields" (the values) aren't type-checked.

**The message decode:**
- **The root** is the definition named `Message`; it must exist (`corrupt`).
- **Primitives:**
  - a varuint is at most 5 bytes (more → `corrupt`); a varuint64 is at most 10;
  - int64 and uint64 become doubles (precision is lost beyond 2^53);
  - a float is Kiwi's form: one byte; if it is 0 the value is 0.0, else 3 more bytes, and the 32 bits are rotated (`bits << 23 | bits >>> 9`) into an IEEE-754 f32.
- **Enums** decode to the value's *name* (an unknown value → null).
- **Structs** read every field, in order.
- **Messages** read `(varuint fieldId, value)` pairs until id 0. An unknown id → `corrupt`; a **repeated id is not refused** (the last value wins).
- **Arrays:**
  - `varuint n`, then n items; a byte array (type −2) is a slice of n bytes (past the end → `corrupt`);
  - any other array needs `n ≤ bytesLeft` before reading (`corrupt`): one byte per item at least, so a lying count can't allocate.
- **Depth:** each nested definition is one level; deeper than **64** → `limit`.
- **Bounds:** every read is bounds-checked (*"the document data ends early (the file is truncated or corrupt)"*).
- **No trailing bytes:** any left after the root message → `corrupt`.
- **No prototypes:** decoded objects have none, so a field named `__proto__` is an ordinary key.

**The decoded message:**
- `Message.nodeChanges` must be an array (else `not_fig`, *"the file holds no document nodes"*);
- `Message.blobs[i].bytes` are the blobs (anything else becomes an empty blob).

## 6. Stage 4: the tree

**Input:** `nodeChanges` (Figma's flat records). **Output:** a tree.

- **Ids:**
  - A record is kept only if it has `guid { sessionID, localID }` (both numbers) and a string `type`. Its key is `"sessionID:localID"`; a later duplicate wins.
  - Records without them are dropped silently, and so are records whose parent isn't in the file (orphans).
- **Children:** each record is a child of `parentIndex.guid`. Siblings are sorted by `parentIndex.position` (Figma's fractional index) in plain UTF-16 code-unit order, ties broken by the id string. **Order is back to front**: the first child is painted first, so it is the first DOM child.
- **Pages:** `CANVAS` children of the `DOCUMENT`, in order (all `CANVAS` records if there is no `DOCUMENT`). A page with `internalOnly: true` is Figma's hidden style-preview page.
- **Masters that can become components:** `SYMBOL`s reachable from a visible page without passing through an `INSTANCE`.
- **Typed readers:**
  - every number must be finite, else it falls back (0, or 1 for scales and alpha);
  - a missing `transform` is the identity;
  - `size` is clamped to ≥ 0;
  - colours are 0–1 floats, written `#rrggbb` when alpha ≥ 0.999 and `rgba(r, g, b, a)` otherwise (a rounded to 3 decimals; channels `round(clamp(v) × 255)`);
  - px values are rounded to 2 decimals.

## 7. Stage 5: images

- **Collecting:** every `IMAGE` paint in `fillPaints` and `strokePaints` of **every** record (internal page included).
  - The key is `image.hash` (20 bytes) as lowercase hex.
  - The bytes come from the ZIP entry `images/<hex>`, else from `blobs[image.dataBlob]` if that blob is non-empty.
  - The first layer that uses an image names it.
- **Order:** each image is processed once, sequentially, in record order.
  - No bytes → failure *"the image is not in the file"*.
  - SHA-1 of the bytes ≠ the hash → failure *"the image bytes do not match their hash"*, and the pipeline is never called.
  - Otherwise `importImage(layerName or "image", bytes)`: `{ src }` counts as imported; `{ error }` is a failure with that reason, verbatim.
- **Failures** go to `report.images.failed` as `{ layer, reason }`. The layer stays a box, reported *"Image fill not imported (see images)"*.

## 8. Stage 6: conversion

**Output:** per top-level layer, a subtree of plain nodes:
- `frame` (a div that can hold children), `box` (a div, optionally holding generated SVG), `text`, `image` and `instance`;
- each with `layout` (how it is placed) and `style` (how it looks), both CSS-aligned property maps.

**Model-neutral reading:** `layout` is the node's position and flow role; `style` is size and appearance. The CSS values are the reference because they define "exact" (`fig-import-mapping.md`).

### 8.1 Pages become bands

Scrim has one canvas. For each visible page, in order:
- **Its top level** is its non-`CANVAS` children. A page with none is skipped and gets no `report.pages` row.
- **The band:** `minX`, `minY` and `maxY` come from the top-level layers' transforms (`m02`, `m12`) and heights. Each root is placed at `(m02 − minX, m12 − minY + y)`, then `y += (maxY − minY) + 400`.
- **The report row:** `report.pages` gets `{ name, roots }`, the roots that were actually created.
- **The internal page** is not converted. Its non-page layer count is one `skipped` item, *"Figma’s internal-only page (style previews)"*, unless it is empty.

### 8.2 One layer, in this order (`build.ts › convert`)

1. **Overrides in effect** (inside an expanded instance): the record merged with the override whose path is `instancePath/guid/…`.
2. **Kind:**
   - FRAME, GROUP, SECTION, SYMBOL, INSTANCE, COMPONENT_SET → `frame`;
   - RECTANGLE, ROUNDED_RECTANGLE, ELLIPSE, LINE → `box`;
   - TEXT → `text`;
   - VECTOR, STAR, REGULAR_POLYGON, BOOLEAN_OPERATION → `box` with SVG;
   - **anything else is skipped with its whole subtree**: *"\<type, lower case, `_` → space\> layer (no Scrim equivalent)"* (STICKY, SLICE, CONNECTOR, WIDGET, …).
3. **Instances.** An INSTANCE's master is `symbolData.symbolID`; a missing master skips the instance (*"Instance whose master is not in the file"*).
   - **A real instance:** a top-level-path instance (not inside a master or another instance) of a convertible master. It becomes a placeholder with its own placement and look; stage 7 replaces it with a real instance.
   - **A detached copy:** any other instance is expanded **inline from the master's children**, with `symbolOverrides` keyed by `guidPath`, and reported (§10).
4. **Placement,** decided by the parent:
   - **Root:** `width` and `height` from `size`; rotation is ignored and reported.
   - **Free parent** (a frame without auto layout), or `stackPositioning: ABSOLUTE` inside auto layout:
     - with a rotation, a flip, an absolute child or a mask wrapper: `position: absolute; left; top; width; height`, plus `transform: rotate() [scaleY(-1)]; transform-origin: 0 0` when transformed;
     - otherwise **constraints** (`layout.ts › constrained`; `fig-import-mapping.md` §2).
   - **Auto-layout parent:** a flow child (`layout.ts › flexChild`):
     - **grow:** `flex-grow: 1; flex-basis: 0`, or the fixed primary size;
     - **stretch:** `align-self: stretch`, or the fixed counter size;
     - always `flex-shrink: 0`;
     - text that hugs on an axis gets no size on that axis;
     - a grid child gets its fixed width and height.
5. **Look:**
   - opacity, blend mode, hidden (`display: none`) and effects;
   - then one of: the line rule (§8.5), text (§8.3), vector SVG (§8.4), a LINE the rule can't take, or fills / image / radius / ellipse / stroke / clipping for boxes and frames.
6. **Own layout:**
   - **Auto layout** becomes flex or grid. A hugging axis drops its size, unless the frame has no children or sits in an auto-layout parent.
   - **Every frame** not already placed absolutely gets `position: relative` (for its absolute children); one without auto layout also gets `display: block`.
7. **The node** is emitted, and its Figma box (the corners through the composed ancestor transforms, then the axis-aligned bounds) goes to `bounds`.
8. **An image fill on a frame** becomes an `image` child first (behind the content): `position: absolute; left: 0; top: 0; width: 100%; height: 100%; object-fit`.
9. **Children**, in order:
   - **A mask in a free parent** turns itself and every later sibling into a **clipping wrapper** frame placed at the mask's box. The wrapper takes the mask's radius, `border-radius: 50%` for an ellipse, `clip-path: path()` for a single-path vector, or its bounds. The masked layers keep their place through plain left / top offsets. The mask's own paint is not drawn.
   - **A mask inside auto layout** is not applied and is reported.
   - A vector's children (boolean operands) are never converted.

### 8.3 Text (`text.ts`)

**One text node holds one style:**
- **The base:** a layer linked to a shared text style (`inheritTextStyleID`) takes the style's `fontName`, `fontSize`, `letterSpacing`, `lineHeight`, `textCase`, `textDecoration` and `fontVariations` (the layer's own copies can be stale). Its fill stays its own.
- **Style runs:** with `characterStyleIDs` and a `styleOverrideTable`, every character votes for its run (unlisted characters count as run 0, the base). The run with the most characters wins (ties: the lower id). A run's values are its own over its shared style's over the base's.
- **Reported:** if the runs differ visually (family, style, size, tracking, line height, case, decoration, solid fills), *"Mixed text styles — …"*.

**Characters:**
- `textData.characters`, line breaks kept;
- `white-space: pre` for auto width and height, `pre-wrap` otherwise.

The full property mapping (weights from style names, fonts, line height, tracking, alignment, features and auto-resize) is `fig-import-mapping.md` §4.

### 8.4 Vectors (`vector.ts`)

The SVG is **generated from Figma's resolved geometry**: `fillGeometry` and `strokeGeometry`, each `{ windingRule, commandsBlob }`.
- **The command blob:** a sequence of `cmd` bytes, each followed by float32 LE arguments:
  - 0 Z (no arguments), 1 M (2), 2 L (2), 3 Q (4), 4 C (6);
  - any other byte, a truncation or a non-finite value → no path;
  - the path must start with M;
  - numbers are rounded to 3 decimals.
- **Paint:** each fill paint fills every fill path; each stroke paint fills every stroke path (strokes come pre-outlined).
  - With strokes but no stroke geometry, a solid stroke is drawn as an SVG `stroke` of `strokeWeight` along the fill paths (centred; inside or outside is reported).
  - `windingRule: ODD` → `fill-rule: evenodd`.
  - Gradients become `<linearGradient>` or `<radialGradient>` defs in the layer's px space, with ids `g0`, `g1`, … per node.
- **The box:** `viewBox 0 0 w h`. A 0-wide or 0-high layer is drawn 1:1 in a 1 px box (overflow visible).
- **Before it is kept:**
  - a path over 200,000 characters → not drawn;
  - the result must pass Scrim's SVG allow-list (`svgContentIssues`); anything refused is reported, never written.
  - Not drawn means the box stays empty.

### 8.5 Lines (`lines.ts`; the exact rule is `docs/design/vector-tools.md` §6)

**What becomes ONE `<line>`** (`vector-effect: non-scaling-stroke`, the shape Scrim's Line tool edits):
- **A LINE** with exactly one visible SOLID stroke, a weight over 0 and a length over 0.
  - Caps: NONE butt, ROUND, SQUARE; arrows are drawn as plain ends and reported.
  - A dash pattern of 1–16 values ≥ 0, not all 0, is kept; any other pattern is drawn solid and reported.
- **A RECTANGLE or ROUNDED_RECTANGLE that is only a rule:**
  - no children, exactly one visible SOLID fill, no radius, no visible stroke, no visible effect, not a mask;
  - its thin side over 0 and at most 2 px, and shorter than its long side.
  - The fill becomes the stroke and the thin side the weight.

**Where it is drawn:**
- **Unrotated:** the box the layer had (a LINE: its length × its weight, its top moved up by weight / 2), with the line centred along it.
- **Rotated or flipped** (always placed absolutely): the end points are taken into the parent's px, the node gets the Line tool's box (bounds ± half the weight), and no CSS transform.
- **A rotated line in auto-layout flow**, or one that fails the SVG check, keeps the box import.

### 8.6 Not in the conversion

Layout grids are not read: there is no guide and no report line. Stroke on text, per-paint image opacity, `backgroundPaints`, min / max sizes, and auto-layout `counterAxisAlignContent` are not read either, also with no report line. Prototypes, comments, plugin data, export settings, style ids and keys are dropped by design. See the divergence list in `fig-import-mapping.md` §7.

## 9. Stage 7: components and the one patch (`components.ts`)

On a working copy, each step applied as it is built, all collected into ONE patch:

1. **The roots:** each root's subtree is added as a new root (after the base's roots), in page order.
2. **Components:** every master converted at the top level of a visible page becomes a component (`createComponentFromSubtree`).
3. **Instances:**
   - **The swap:** each instance placeholder is removed and replaced by a real instance of its component, at the same parent and index, with the same name and canvas position.
   - **Placement:** the instance root keeps the placeholder's own placement keys (position, offsets, flex-child role; keys the placeholder lacks are cleared).
   - **The root's look:**
     - every style value that differs from the master root's is pinned as an instance override (its size included);
     - if the master has `background`, `opacity`, `boxShadow`, `filter`, `mixBlendMode` or `display` and the instance lacks it, it is reset (`transparent`, 1, `none`).
   - **Descendants** are paired master → instance **by position**.
   - **Each `symbolOverride`:**
     - a `guidPath` longer than 1 (nested) → reported, not applied;
     - an unknown target → reported;
     - `fillPaints` becomes `color` on text and `background` elsewhere (an image fill is reported, not applied);
     - `visible: false` → `display: none`;
     - `textData.characters` → the text;
     - any other overridden field is reported by name (the first three).
4. **Tokens:**
   - each `VARIABLE` with `variableResolvedType: COLOR` becomes a colour token: the first `{r, g, b}` found in its values (the first mode), searched at most 8 deep;
   - its name: lower case, non-alphanumeric runs → `-`, trimmed; a duplicate gets `-2`, `-3` …; an empty name becomes `color`;
   - other kinds, and colours without a readable value, are `skipped` items with count 0;
   - if any token was made, one `approximated` item (§10).
5. **The imported count is recounted:** nodes in the final document minus nodes in the base (instance content included).

## 10. Stage 8: the report (format and wording)

```text
FigReport {
  file: string                    // the file name given
  imported: number                // nodes created (final doc − base), instance content included
  approximated: number            // DISTINCT layers with ≥ 1 approximation (keyed by instance path + Figma id)
  skipped: number                 // sum of skipped counts (a subtree counts every layer in it)
  resize: number                  // distinct layers exact now whose resizing CSS approximates (constraints)
  pages: [{ name, roots }]        // visible pages with ≥ 1 top-level layer
  items: [{ kind: 'approximated' | 'skipped' | 'resize', feature, count, layers }]
  fonts: [{ family, as, layers, note }]
  images: { imported, failed: [{ layer, reason }] }
  components: { masters, instances, detached }
  tokens: { colors }
  summary: string
}
```

**The categories:**
- **approximated:** drawn, but something differs from Figma (named by feature);
- **skipped:** not imported at all;
- **resize:** exact at the imported size, but resizing follows CSS's version of a Figma constraint. It is **not** counted as approximated.

**The rules:**
- **One item per (kind, feature).** `count` adds 1 per occurrence (a skipped item adds its own count). `layers` keeps the first **8** distinct layer names.
- **Item order:** approximated, then skipped, then resize; within a kind, by count (descending), then by feature (code-unit ascending).
- **Fonts:** one row per family. `layers` counts text layers; `as` is what draws (the family, or the generic fallback for an unknown font). Sorted by layers (descending), then family.
- **Summary, exactly:**
  `` `${imported} layer${imported === 1 ? '' : 's'} imported, ${approximated} approximated, ${skipped} skipped${resize ? ` (${resize} resize by CSS rules)` : ''}` ``
  For example *"752 layers imported, 33 approximated, 27 skipped (249 resize by CSS rules)"* and *"1 layer imported, 1 approximated, 0 skipped"*.
- **Where it shows:**
  - **the hosts' notice:** *"Imported from Figma: \<summary\>."*;
  - **the studio card:** sections "Approximated", "Skipped", "Exact now, resize by CSS rules", "Fonts", "Images not imported";
  - **the CLI text:** `<file>: <summary>`, then `pages: Name (n roots), …`, one line per item `~ feature ×count layers` (`-` for skipped *and* resize), `font Family → as: note`, and `images: N ok[, M not imported (first reason)]`.

**Every feature string, verbatim:** the mapping doc, §6.

## 11. The fidelity numbers to beat

### 11.1 The fixtures (run by `cases/fig-import.json`'s scripts)

| Fixture | Container | Records | Report summary | Approximated items | Skipped items |
|---|---|---|---|---|---|
| `stage-a.fig` | ZIP (CRC 0); the bare form gives the same report | 20 | 15 layers imported, 0 approximated, 2 skipped | — | internal-only page ×1 (Internal Only Canvas); sticky layer ×1 (FigJam note) |
| `stage-b.fig` | fig-kiwi | 31 | 30 layers imported, 3 approximated, 0 skipped (4 resize by CSS rules) | Auto-layout grid (Grid); Background blur (Blurred); Diamond gradient (Diamond); Layer blur (Blurred) | — (resize: Right, Centred, Stretched, Scaled) |
| `stage-c.fig` | fig-kiwi | 26 | 27 layers imported, 6 approximated, 2 skipped | Mixed text styles ×2 (Runs, Shared runs); Colour variables became tokens (“1 tokens”); Instance inside a component (Nested icon); Instance of a component on Figma’s internal page (Internal icon copy); Paragraph spacing (System font) | internal-only page ×2; float variable ×0 (Spacing/sm) |
| `stage-d.fig` | fig-kiwi | 16 | 12 layers imported, 2 approximated, 2 skipped | Vector without geometry ×2 (No geometry, Image vector); image fill on a vector (Image vector) | Boolean operation operands ×2 (Union) |
| `lines.fig` | fig-kiwi | 18 | 16 layers imported, 1 approximated, 0 skipped | Arrowheads and other line ends (Arrow) | — |

These match FigCraft's spike table (`docs/research/figcraft-spike.md` §3: 15/0/2, 30/3/0, 27/6/2, 12/2/2, 16/1/0). Per-node expectations, 140 of them, are in the JSON; each was checked against the run.

**Measured in the built app** (`docs/research/fig-fidelity/lines/after-*.json`; the metric is in §11.3):

| | Box layers | Within 1 px | Within 4 px | Canvas vs HTML pixels |
|---|---|---|---|---|
| `lines.fig` (2 artboards) | 16 | 11 | 16 | 0% |
| `stage-a.fig` (3 artboards) | 14 | 11 | 12 | 0–0.05% |

The misses are by design of the metric: Figma's box for a line is 0 thick, a hidden layer renders 0 × 0, and an auto-height text renders taller than its stored height. `fig-fidelity.md` › *Lines* shows that the stroke positions are exact.

### 11.2 The 24.6 MB sample (not in the repo; `fixtures.md`)

| What | Value |
|---|---|
| File | the owner's 24.6 MB sample (a third-party template, name withheld), sha256 `5d08369d…`, in the owner's iCloud Drive (read in place, never copied) |
| Container | bare fig-kiwi, version 15; schema 31.7 KB, 193 definitions; data 25.1 MB zstd |
| Decoded | 572 records (568 non-page), 333 blobs, 22 of them images (6 PNG, 16 JPEG, largest 7.9 MB) |
| **Report** | **752 layers imported, 33 approximated, 27 skipped (249 resize by CSS rules)** |
| **Boxes (HTML vs Figma)** | **522 of 541 within 1 px (96.5%); 537 of 541 within 4 px (99.3%)**; canvas vs Figma identical |
| Canvas vs standalone HTML | 0.006% to 1.136% of pixels differ per artboard (20 artboards), all antialiasing |
| The 4 layers over 4 px | font metrics: Typeface › Subtitle ×2 (2 px taller), Style Guide › A4d ×2 and Logo - White (7.6 px wider) |
| Time | 64 ms median decode and convert in Node; 693 ms in the app with images stored (both marked INVALID: the machine failed its fitness check) |

**Per artboard** (`fig-fidelity/sample/summary.json`; box layers / within 1 px / within 4 px, worst Δ px, pixel diff %):

| Artboard | Layers | ≤1 | ≤4 | Worst | Pixels |
|---|---|---|---|---|---|
| Blank Page | 5 | 5 | 5 | 0 | 0.704 |
| Photography Extra | 4 | 4 | 4 | 0 | 0.085 |
| Photography | 9 | 9 | 9 | 0 | 0.353 |
| Pattern | 6 | 6 | 6 | 0 | 0.641 |
| Grids | 8 | 8 | 8 | 0 | 0.757 |
| Scalability | 5 | 5 | 5 | 0 | 0.43 |
| Typography Scale | 19 | 19 | 19 | 0.39 | 0.626 |
| Typeface | 17 | 15 | 17 | 2 | 0.925 |
| Colour Palettes | 71 | 71 | 71 | 0.04 | 0.679 |
| Logo Dont's | 50 | 50 | 50 | 0.02 | 0.694 |
| Logo Do's | 6 | 6 | 6 | 0 | 0.741 |
| Logo | 10 | 10 | 10 | 0 | 0.136 |
| Brand Personality | 67 | 67 | 67 | 1 | 0.381 |
| Quote | 6 | 6 | 6 | 0 | 1.059 |
| Brand Tone | 8 | 8 | 8 | 0 | 0.68 |
| Brand Audience | 7 | 7 | 7 | 0 | 0.675 |
| About Page | 6 | 6 | 6 | 0 | 0.31 |
| Brand Guidelines | 8 | 8 | 8 | 0.02 | 0.156 |
| Read me | 6 | 6 | 6 | 0 | 0.006 |
| Style Guide | 223 | 206 | 219 | 7.61 | 1.136 |
| **Total** | **541** | **522** | **537** | | |

**The itemised report for the sample is not recorded anywhere:** only the summary line, so the 33 and the 27 can't be broken down without re-reading the file. The owner chose a single read.

### 11.3 How the percentages are derived

**"94% exact" is a pre-build estimate, not an importer output.** It is `docs/research/fig-fidelity.md` › *Share of the sample's nodes by verdict*: the 568 non-page records, classified by hand from the decoded records against the mapping table.

| Bucket | Nodes | Share (of 568) |
|---|---|---|
| Exact | 534 | 94.0% |
| Approximate | 8 | 1.4% (6 layer blurs, 2 texts whose runs change size or font) |
| Not expressible | 0 | 0% |
| Skipped | 26 | 4.6% (the internal-only page; "its one master is kept") |
| Resize behaviour (separate) | 270 | 47.5% (Center, Scale or Max constraints) |

The quoted sentence: *"On this sample, 94% of nodes render exactly through plain HTML, CSS and generated SVG, nothing is inexpressible, and the approximations are small and nameable."*

**The importer's report counts differently, so its numbers don't reproduce those:**
- **752 imported** counts nodes *created*: instance content expanded per instance, image layers added for frame image fills, mask wrappers. It does not count records.
- **27 skipped** is the internal page's whole non-page subtree. The estimate's 26 kept its one master, which the importer doesn't do; instances of internal masters are detached copies instead.
- **33 approximated** counts distinct layers per instance path. Detached copies, every mixed-run text, blurs, tokens and so on each count, which is more than the estimate's 8.
- **249 resize** is the layers the importer actually placed with a non-MIN constraint in a free parent; the estimate's 270 was every node whose constraints weren't MIN.

**"96.5%" and "99.3%"** are 522 / 541 and 537 / 541 from the box measurement:
- `docs/research/fig-fidelity/measure.mjs` imports in the built app and exports standalone HTML;
- each created node's rendered box (relative to its artboard) is paired with the importer's `bounds` box **by tree position**;
- `d = max(|Δx|, |Δy|, |Δw|, |Δh|)`, so "within 1 px" means d ≤ 1;
- only nodes that have a `bounds` entry and a rendered element count (541 of 752): instance content and SVG internals have none.
- The pixel figure counts pixels where any channel differs by more than 24, after both images are flattened over white, at 1600 × 900.

**A port beats Scrim** when, on the same file:
- its report is at least as complete (no silent drops; Scrim has some, `fig-import-mapping.md` §7);
- more of the 541 boxes land within 1 px;
- every approximation stays nameable.
