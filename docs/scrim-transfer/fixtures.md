# Fixtures a fork can use from the Scrim repo

**What this is:** every test fixture in the Scrim repo that a Rust fork of FigCraft can reuse, with its path, size, what it exercises, how it was made, and its licence or provenance.

**Ground rules:**
- **Paths** are relative to the Scrim repo root, and sizes are in bytes.
- **"Synthetic"** means made by Scrim's own code or tools: no third-party content.
- **Licence:** every Scrim package declares `"license": "MIT"` in its `package.json`, but the repo root has no `LICENSE` file (stated, not resolved here). Synthetic fixtures are Scrim's own work under that declaration.

## 1. Figma `.fig` fixtures (`tests/fixtures/fig/`)

**How they were made:**
- **Five synthetic files**, built by the test-only encoder `packages/fig-import/src/__tests__/figEncoder.ts` from the builders in `packages/fig-import/src/__tests__/fixtures.ts`.
- **Byte-for-byte reproducible:** `fixtureFiles.test.ts` fails if a committed file differs from its builder's output, and `SCRIM_WRITE_FIG_FIXTURES=1` rewrites them.
- **The schema is Scrim's own:** a minimal Kiwi schema of 46 or 47 definitions with **Figma's field names** but its own type layout. The decoder reads whatever schema a file embeds, so these exercise a schema-agnostic reader; they are **not** byte-compatible with Figma's real schema.
- **The format:** fig-kiwi version 15, the schema chunk raw-deflated, the data chunk zstd.
- **The expected output** of each file (every node, the report, 140 checked asserts) is in `docs/transfer/cases/fig-import.json › fixtures`.

| File | Size | sha256 (prefix) | Container | Builder | What it exercises | Report |
|---|---|---|---|---|---|---|
| `tests/fixtures/fig/stage-a.fig` | 2,783 | `6e800b7a1e07dd9d` | **ZIP** (`canvas.fig` deflated + `meta.json` stored), **CRC-32 = 0 in every header** | `stageA({ zipped: true })` | two pages, an internal-only page, a clipping frame and a non-clipping group, rectangles (radius, opacity, inside and centre strokes), an ellipse, a 90°-rotated line, text (`HEIGHT` auto-resize, a line break), an image fill (a 4×3 PNG **in a Kiwi blob**), a hidden layer, a component and an instance with a fill override, a FigJam sticky | 15 / 0 / 2 |
| `tests/fixtures/fig/stage-b.fig` | 2,902 | `b234b1773d0cd0ad` | bare fig-kiwi | `stageB()` | auto layout (a row with space-between, padding and gap; grow, stretch, fixed and absolute children), a hugging column, an empty hugging frame, wrap, grid, constraints (MAX, CENTER, STRETCH, SCALE), drop and inner shadows, layer and background blur, a text shadow, linear, radial, angular and diamond gradients, multiply, an image fill on a frame (FIT; a 2×2 PNG blob), a rounded-rectangle mask | 30 / 3 / 0, 4 resize |
| `tests/fixtures/fig/stage-c.fig` | 2,786 | `2488e8017a0c1375` | bare fig-kiwi | `stageC()` | text: line height in % and px, tracking in % and px, horizontal and vertical alignment, case, underline, variable-font axes (`wght` and `wdth` as u32 tags), OpenType features on and off, mixed runs, a run on a shared text style, a layer on a shared style with stale copies; fonts (Google, system, unknown); components (two masters, a nested instance inside a master, real instances with text, fill and visibility overrides, a top-level instance with its own size and fill, an instance of an internal-page master); variables (COLOR and FLOAT) | 27 / 6 / 2 |
| `tests/fixtures/fig/stage-d.fig` | 2,504 | `c4a8b16dd48b558d` | bare fig-kiwi | `stageD()` | vectors from command blobs: a filled triangle, an even-odd ring with a gradient, a stroke-only 0-high rule with a gradient stroke, a star with a drop shadow, a hexagon with fill and a centred stroke without stroke geometry, a boolean with two operands, an arc, a vector without geometry, a vector with an image fill, a star used as a vector mask | 12 / 2 / 2 |
| `tests/fixtures/fig/lines.fig` | 2,462 | `a13b86e4957033f8` | bare fig-kiwi (its schema adds a `StrokeCap` enum and `strokeCap`: 47 definitions) | `linesFixture()` | 7 LINEs (plain; round cap and translucent; dashed `[4, 2]`; rotated 30°; rotated 90° with a square cap; an arrow; a gradient stroke), thin rectangles (a 1 px divider, a 2 px column rule, a separator in auto layout) and two that don't qualify (rounded, stroked) | 16 / 1 / 0 |

**Variants the builders make that are not committed:**
- `stageA()` bare: 2,564 bytes, sha256 `f5ae4eb4…`, the same 20 records and report as the zipped file.
- `stageA({ tamperImage: true })`: the blob is a black PNG under the pink PNG's hash.

**Notes for the fork:**
- **CRC 0:** the encoder's `zip()` writes CRC-32 = 0 in local and central headers. Scrim never reads CRCs; FigCraft refuses the file ("Invalid checksum", `docs/research/figcraft-spike.md` §3). To use it under a CRC-checking reader, re-zip `canvas.fig` and `meta.json` with real CRCs (`docs/transfer/cases/fig-import.json › zipCrc` lists them: `835121921` and `2745614147`). Every hostile ZIP in the cases file has CRC 0 too.
- **Images in blobs, not `images/` entries:** both PNGs are generated by `png()` in `fixtures.ts`, a solid-colour RGB PNG built byte by byte.
- **Builders as data:** `fixtures.ts` is the readable source of every record. Its node objects are a direct description of each file (`guid`, `parentIndex`, `type`, `size`, `transform`, paints …).
- **Where else they are used:**
  - stage A: `packages/mcp/src/__tests__/figImport.test.ts` (the server import, the Home drop, undo) and `packages/cli/src/__tests__/cli.test.ts` (`import-fig`);
  - stage C and stage D: the built-app smoke `apps/desktop/scripts/fig-import-smoke.mjs`.

## 2. The 24.6 MB real sample (NOT in the repo)

| | |
|---|---|
| File | the owner's 24.6 MB sample (a third-party template, name withheld) |
| Where | "one template in the owner's iCloud Drive", read in place (`docs/research/fig-fidelity.md` › *The sample*). There is no path in the repo, and `~/Scrim-fig-samples` didn't exist |
| Identity | sha256 prefix `5d08369d…` (`docs/research/openpencil-spike.md` §3; FigCraft's spike says "its hash is unchanged") |
| Provenance | a third-party Figma template the owner holds. Read once per study, never modified and never copied; only aggregate counts are recorded, no text content |
| Licence | unknown (a commercial or community template): **never copy it into the fork** |
| Format | bare fig-kiwi v15; schema 31.7 KB (193 definitions); data 25.1 MB zstd; 572 records, 333 blobs (22 images: 6 PNG, 16 JPEG) |
| Scrim's result | "752 layers imported, 33 approximated, 27 skipped (249 resize by CSS rules)"; 522 / 541 boxes within 1 px |

To compare a port with Scrim on it, the owner must run both on the file in place. The itemised report was never recorded (`specs/fig-import.md` §11.2).

## 3. Fidelity measurements and tools (`docs/research/fig-fidelity/`)

| Path | Size | What | Provenance |
|---|---|---|---|
| `docs/research/fig-fidelity/sample/summary.json` | 17,246 | per artboard (20): canvas vs HTML pixel diff %, HTML vs Figma and canvas vs Figma boxes (layers, within 1 px, within 4 px, median, worst, worst layers) for the real sample | output of `measure.mjs` on the owner's file (12 Oct 2026). It holds the artboard and layer *names* of the template |
| `docs/research/fig-fidelity/sample/*.jpg` (12 files, about 324 KB) | — | the canvas, HTML and diff renders of 4 artboards (Typography Scale, Colour Palettes, Logo Dont's, Read me) | **renders of the third-party template**: treat them as the sample's licence (unknown); don't redistribute |
| `docs/research/fig-fidelity/lines/{before,after}-lines.json` | 3,355 each | `lines.fig` measured before and after the line import (16 layers; 11 / 16 within 1 px) | `measure.mjs` on a synthetic fixture |
| `docs/research/fig-fidelity/lines/{before,after}-stage-a.json` | 4,021 each | `stage-a.fig` measured (14 box layers; 11 within 1 px) | the same |
| `docs/research/fig-fidelity/measure.mjs` | 13,359 | the measurement: import in the built app, standalone HTML offscreen with the network blocked, pixel diff (any channel > 24), boxes paired by tree position, `d = max(|Δx|, |Δy|, |Δw|, |Δh|)` | Scrim (MIT declaration); needs the built Electron app |
| `apps/desktop/scripts/fig-import-smoke.mjs` | 8,078 | the built-app smoke: import, report card, undo, redo, Save As, no metadata, relaunch, a drop on Home | the same |

## 4. Hostile inputs (`docs/transfer/cases/fig-import.json › security`)

- **62 cases:** 55 refusals with the verbatim `FigError`, 1 accepted (an ignored entry with a bad method), and 6 observed behaviours outside any limit: 3 stack overflows, 2 unbounded growths and 1 inert `__proto__` field.
- **Inputs:** 56 are embedded as base64 (at most 8 KB each); the 6 larger ones have exact build steps and a sha256. All of them were made by the scratch scripts listed in the JSON's `generatedBy`, from the same `figEncoder.ts` and `fixtures.ts`. They are synthetic.

## 5. Image-import fixtures (the pipeline a `.fig`'s images go through)

| Path | Size | What it exercises | How it was made |
|---|---|---|---|
| `packages/mcp/src/__tests__/fixtures/trailing/*.jpg, *.png, *.gif, *.webp` (16 files) | 285 – 19,272 | where an image's own structure ends (`imageEnd`): valid files and files with trailing data. **`motion-photo.jpg`** (13,994; ends at 11,202; a 2,792-byte H.264 MP4 appended), `ultra-hdr-mpf.jpg` (an MPF gain map), `exif-thumbnail.jpg`, progressive and restart-marker JPEGs, stuffed bytes, PNG with ancillary chunks plus a ZIP polyglot, animated GIFs (trailing, no trailer + ZIP), an extended WebP with an odd-size EXIF chunk | `make.mjs` in the same folder: real encoder output (libjpeg-turbo `cjpeg`, ImageMagick, `cwebp`, `ffmpeg`) plus hand-built structure; `fixtures.json` records each file's `kind`, `size`, `end` and `about` |
| `packages/mcp/src/__tests__/fixtures/trailing/fixtures.json` | 3,242 | the manifest: expected `end` per file, known from construction, never from the code under test | hand-written with `make.mjs` |
| `packages/mcp/src/__tests__/exifFixtures.ts` | 136 lines | generated JPEG, PNG, WebP and AVIF headers with an orientation, an optional GPS IFD and an optional IFD1 thumbnail, in either byte order (for GPS stripping, rule 18) | built byte by byte in code; the pixel data is a stand-in |
| `packages/fig-import/src/__tests__/fixtures.ts › png(w, h, rgb)` | — | a real, minimal RGB PNG | code |

## 6. Document fixtures (useful for output and export checks, not `.fig`)

| Path | What | How it was made |
|---|---|---|
| `packages/mcp/src/fixtures.ts › makeFixtureDoc()` | the "seed" document: a pricing card (flex column, text, a button) | code, through the patch pipeline |
| `packages/cli/src/__tests__/fixtures.ts` | `landing()` (a landing page with a component and an instance), the seed, a portable file with an embedded image | code |
| `packages/mcp/src/__tests__/exportGateDocs.ts` | the export-gate documents: seed, landing, components, mixes, poster; and **motion** (`motionDocuments()`: states, media, conditions, `as`, `href`) | code |
| `packages/mcp/src/__tests__/exportGate/*.txt` (`seed` 2,648 · `landing` 16,569 · `components` 9,233 · `mixes` 3,376 · `poster` 7,835 · **`motion` 21,655**) | the exact `get_jsx`, `get_css`, inline CSS and standalone HTML pinned for each gate document. They are byte-for-byte expectations of Scrim's codegen | written by `exportGate.test.ts` with `SCRIM_WRITE_EXPORT_GATE=1`, after human review |
| `packages/mcp/src/__tests__/lintGate/*.json` (67 – 335 bytes) | the expected design-lint findings per gate document | `lintGate.test.ts` |

## 7. Stress-harness generators (`tools/stress/`)

**No files: deterministic generators** (`tools/stress/src/scenarios.ts`, 287 lines; `build.ts`, `png.ts`). Each scenario returns **exactly n nodes**, the probe artboard included, for n in {2,000, 5,000, 10,000, 20,000}. Measurement only, never shipped (`tools/stress/README.md`). They are useful to a fork for building large documents to import, export and render; they are not `.fig`.

| Id | Scenario | Definition (verbatim, short) |
|---|---|---|
| S1 | Flat sweep, fit-all | 20 free artboards (1440 × 1024), the rest as absolute boxes in a grid, one of 12 fills each |
| S2 | Zoomed-out overview | free artboards of 50 absolute boxes each; measured at 7% zoom |
| S3 | Edge churn | a dense grid of small free roots (240 × 160), 4 boxes each; a fast diagonal pan |
| S4 | Deep nesting | towers of 40 nested clipping frames, about 960 leaves in the deepest |
| S5 | Text-heavy | hug-height articles of 100 text nodes: Latin, Japanese, Chinese, Arabic, emoji, figures; six font stacks |
| S6 | Images | artboards of 40 image nodes cycling 24 imported 12 MP PNGs (4000 × 3000, `gradientPng` in `png.ts`) |
| S7 | Effects | 200 boxes per artboard: box-shadow; every 3rd blur(2px); every 5th backdrop-filter |
| S8 | Zoom extremes | S1's document, zoomed 2% → 3200% → 2% |
| S9 | Live drag in a crowd | a hug root (flex, wrap) of up to 500 boxes among S1-style boards |
| S10 | Many small shapes | 1,000 circles per artboard |
| S11 | Flex rows | 10 flex rows of 100 fixed children per board |
| S12 | Realistic mix | **the library's "Fixture" document** (18 artboards, 782 nodes) cloned until the budget is spent. The template comes from the person's userData library at run time: **not in the repo** |

`pnpm stress:grid` (`src/grid.ts`) builds a 200-cell grid in the same way.
