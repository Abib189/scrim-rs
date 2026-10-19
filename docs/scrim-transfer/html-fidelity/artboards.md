# The standard artboard set (20 artboards)

The baselines in `README.md` were all measured on these 20 artboards: one hand-drawn vector board, plus every top-level frame of Scrim's five synthetic `.fig` fixtures. To compare a tool with Scrim, FigCraft and OpenPencil, measure the same 20.

## The fixtures

The five files are `tests/fixtures/fig/{stage-a,stage-b,stage-c,stage-d,lines}.fig` in the Scrim repo.
- **Where they come from.** The test-only encoder in `packages/fig-import/src/__tests__/fixtures.ts` (`stageA`, `stageB`, `stageC`, `stageD`, `linesFixture`) builds them.
- **They are synthetic and permission-safe.** No real designer's file is involved.
- **They are pinned.** `fixtureFiles.test.ts` checks that the committed bytes equal the builders' output. `SCRIM_WRITE_FIG_FIXTURES=1 pnpm --filter @scrim/fig-import test` rewrites them.
- **Copying them.** The `.fig` files can be copied into the fork as test data; they carry no third-party content.

**`stage-a.fig` is a zip whose CRCs are 0.** FigCraft rightly refuses it ("Invalid checksum"). Use the bare fig-kiwi file inside it:

```sh
unzip -p tests/fixtures/fig/stage-a.fig canvas.fig > stage-a.bare.fig   # exits 2 ("bad CRC"); the bytes are right
head -c 8 stage-a.bare.fig    # fig-kiwi
```

The other four are bare fig-kiwi files already (zstd).

## The 20

The sizes are the Figma frame sizes stored in the file. The "Scrim capture" column is what Scrim's probe actually compared in Chromium. It is 1 px wider where the canvas held the artboard at a half-pixel x (sub-pixel `0.5, 0`), and smaller where the frame hugs content.

| # | Fixture | Page | Artboard | Size | Scrim capture | What it exercises |
|---|---|---|---|---|---|---|
| 1 | (none) | (none) | vectors / Board | 640 × 400 | 640 × 400 | A line, a pencil stroke and a pen path drawn with the editor's own tools (see below) |
| 2 | stage-a | Page One | Card | 400 × 300 | 401 × 300 | Fill and corner radius, per-layer opacity, inside and centre strokes, an ellipse, a rotated line, two-line bold text, an **image fill**, a hidden layer, a group |
| 3 | stage-a | Page Two | Button | 120 × 40 | 121 × 40 | A component master |
| 4 | stage-a | Page Two | Button copy | 120 × 40 | 121 × 40 | An instance with a fill override |
| 5 | stage-b | Layouts | Row | 400 × 60 | 400 × 60 | Horizontal auto layout: gap, padding, space-evenly, centre alignment, a grow child, a stretch child and an absolute child |
| 6 | stage-b | Layouts | Column | 120 × 100 (hugs) | 30 × 14 | Vertical auto layout that hugs one auto-width text ("Label", Inter 12) |
| 7 | stage-b | Layouts | Empty hug | 501 × 72 | 501 × 72 | An auto-layout frame with no children keeps its stored size |
| 8 | stage-b | Layouts | Wrap | 200 × 100 | 200 × 100 | Auto-layout wrap with a row gap |
| 9 | stage-b | Layouts | Grid | 300 × 200 | 300 × 200 | Auto-layout grid (3 × 2, gaps 10) |
| 10 | stage-b | Layouts | Free | 400 × 300 | 400 × 300 | Constraints: MAX, CENTER, STRETCH, SCALE |
| 11 | stage-b | Layouts | Effects | 600 × 200 | 600 × 200 | An **image fill (FIT)**, a drop and an inner shadow, a layer and a background blur, linear, radial, angular and diamond gradients, multiply blend, a text shadow |
| 12 | stage-b | Layouts | Masks | 300 × 200 | 300 × 200 | A rounded-rectangle mask over two layers, with a layer before the mask |
| 13 | stage-c | Type and components | Texts | 400 × 800 | 401 × 800 | Line height and tracking in % and px, centre and justified alignment, variable-font axes, mixed style runs, a shared text style, an unknown font, a system font, paragraph spacing |
| 14 | stage-c | Type and components | Icon | 16 × 16 | 17 × 16 | A tiny component |
| 15 | stage-c | Type and components | Card | 200 × 100 | 201 × 100 | A clipping component with a background, a bold title, a badge and a nested instance |
| 16 | stage-c | Type and components | Board | 600 × 300 | 601 × 300 | An instance with text, fill and visibility overrides, and an instance of a master on the internal page |
| 17 | stage-c | Type and components | Card B | 240 × 100 | 241 × 100 | A resized instance with its own fill |
| 18 | stage-d | Vectors | Icons | 400 × 300 | 401 × 300 | Vectors (nonzero and even-odd), a gradient stroke, a star with a shadow, a stroked polygon, a boolean, an arc, a vector with no geometry, an image-filled vector, a vector mask |
| 19 | lines | Lines | Rules | 480 × 360 | 481 × 360 | Lines: plain, round-capped and translucent, dashed, rotated 30° and 90° with square caps, an arrow, a gradient stroke. Also thin rectangles that should become lines, and two that shouldn't |
| 20 | lines | Lines | Stack | 200 × 97 | 201 × 97 | A vertical stack with a 1 px separator, on an odd height |

**Not in the set:**
- **`stage-a` › Page Two › FigJam note**, a `STICKY`. Scrim skips it on import. FigCraft and OpenPencil measured it (both identical), but it is left out of their "of 19" and "of 20" counts.
- **The internal-only pages** of stage-a and stage-c.

**Fonts.** The fixtures name Inter, Roboto Flex, Helvetica Neue and a made-up "Acme Grotesk". The canvas and the HTML must draw text with the **same faces**, or every text artboard differs. With the network blocked, a page's Google Fonts `<link>` fails, and both sides fell back to the same system faces in Scrim's run. A tool that bundles its own fonts must inline them into the HTML as `data:` URLs (or `@font-face` with data URLs) for the comparison to be fair.

## Artboard 1: the vector board

It is not a `.fig`. Scrim's probe drew it in the running editor with real pointer input. To reproduce it in the fork:

1. Create a frame named `Board` at (0, 0), 600 × 400, white, clipping (`overflow: hidden`). Then set its width to 640 through the inspector, as the probe did.
2. **Line tool:** drag from (60, 60) through (70, 66) to (260, 140).
3. **Pencil tool:** one stroke through the 40 points (60 + 5i, 260 + 40·sin(i/5)), for i = 0…39.
4. **Pen tool:**
   - click (340, 80);
   - press at (440, 160), drag through (444, 164) to (480, 160) and release (a smooth anchor);
   - click (540, 70);
   - press Enter.

Keep the tools' default stroke. All coordinates are board-local, at zoom 100%.

## Producing the pairs

For each artboard you need two things:
- **`reference`:** a PNG of what the canvas draws, at 1× (or `scale`×);
- **`html`:** the tool's HTML export of that node.

Then `fidelity.mjs` renders the HTML in each engine and compares.

**In Scrim, for reference.** The probe captured the canvas with an element screenshot of the artboard in the studio, with the dot grid and the overlay hidden. The HTML was `standaloneHtml(doc, id)` from `@scrim/codegen`. Because that page centres the artboard, the manifest entry used `"selector": ".scrim-artboard > *"` and `"subpixel"` set to the canvas's fraction.

**In FigCraft** (the commands `docs/research/openpencil/fcfidelity.mjs` ran):

```sh
figcraft-cli info stage-b.fig --json        # → { pages: [{ name, frames: [{ id, name, width, height }] }] }
figcraft-cli export stage-b.fig --node <id> -o Row.png                       # the canvas (tiny-skia)
echo '[{"command":"inspect.code","params":{"id":"<id>","language":"html"}}]' > row.json
figcraft-cli run row.json --in stage-b.fig  # last stdout line: {"ok":true,"result":{"code":"<fragment>"}}
```

Wrap the fragment in the minimal page (`"wrap": true` in the manifest does this) and the frame is drawn at (0, 0). No `selector` or `clip` is needed: the default clip is the reference PNG's size at the top left. `README.md` has a script that does all 19 `.fig` artboards in one go.

For the fork's own Rust export, the shape is the same:
- the canvas renderer's PNG of the node;
- the HTML exporter's page or fragment;
- one manifest entry per artboard.
