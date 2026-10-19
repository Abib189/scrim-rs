# HTML fidelity: does the export render like the canvas?

`fidelity.mjs` is Scrim's canvas-versus-HTML harness packaged as a standalone tool. It renders a tool's exported HTML in real browser engines and compares it, pixel for pixel, with a PNG of what the tool's canvas draws. It works for any tool's export: Scrim's, FigCraft's, OpenPencil's, or the Rust fork's.

## Why: the export guarantee

Scrim promises that **the exported HTML renders the same as the canvas**. What a designer sees is what ships. Scrim keeps that promise cheaply because its canvas *is* the DOM. A fork with a non-DOM canvas (FigCraft's tiny-skia renderer) has to earn it: layout, text shaping, effects and masks must come out the same in the renderer and in the browser.

This tool is the measurement that says whether they do, artboard by artboard and engine by engine. Run it in CI and the guarantee becomes a gate instead of a hope.

## What it measures (the method, unchanged from Scrim's probe)

The method is that of Scrim's `docs/research/portability/probe.mjs`. The OpenPencil and FigCraft drivers used the same one (`docs/research/openpencil/opfidelity.mjs`, `fcfidelity.mjs`).

**How the two images are prepared:**
1. **The HTML is rendered with the network blocked.** Only `data:` URLs load, as on a design canvas that never fetches remote content. A Google Fonts `<link>` fails, and remote images don't load. Inline what must render. The number of blocked requests is reported per artboard.
2. **Fonts are ready** (`document.fonts.ready`) before the capture.
3. **The sub-pixel phase is matched.** If the reference was captured with the artboard at a fractional screen position, give that fraction as `subpixel`. The page's target element is then shifted with CSS `translate` (no layout change) after cancelling its own fraction, so both rasterise at one phase.
4. **Both images are composited on white** and compared at their **minimum common size**. A size mismatch is flagged (`sizeMismatch`), never hidden.

**How a pixel is classified:**
5. **A pixel differs** when any RGB channel differs by more than **24** (`--threshold`).
6. **A differing pixel is "antialiasing"** when a near-equal pixel exists within 1 px in **both** directions: A's pixel matches something in B's 3×3 neighbourhood, and B's matches something in A's. That is an edge shifted by at most one pixel. Otherwise it is **"real"**.
7. **The outer 1 px frame is excluded.** It is the capture boundary, and at a half-pixel phase it mixes in what lies outside the artboard. Percentages are of `(w − 2) × (h − 2)`.

**Across engines:**
8. Results are per engine: **Chromium and WebKit** by default, **Firefox** when it can launch.
9. The pixel diff itself always runs in a Chromium page (canvas `drawImage` on white, then `getImageData`), whatever engine rendered the HTML. So Chromium must be installed even for a WebKit-only run.

**An artboard is identical when `realPx === 0`.** Antialiasing alone doesn't break it.

## Install

The tool needs Node ≥ 20 and **one** dependency, `playwright-core`. Install it anywhere: it doesn't belong in the project's dependencies.

```sh
mkdir -p /tmp/pw && cd /tmp/pw && npm init -y >/dev/null && npm i playwright-core@1.64.1
PLAYWRIGHT_BROWSERS_PATH=/tmp/pw/browsers npx playwright-core install chromium webkit   # add firefox if wanted
export PWCORE=/tmp/pw/node_modules/playwright-core/index.mjs
export PLAYWRIGHT_BROWSERS_PATH=/tmp/pw/browsers
```

- **Disk:** Chromium, WebKit and Firefox take about 1.2 GB.
- **Without `PLAYWRIGHT_BROWSERS_PATH`,** Playwright uses its default cache (`~/Library/Caches/ms-playwright` on macOS).
- **Without `PWCORE`,** the script tries a normal `import('playwright-core')`, then `playwright-core` resolved from the current directory.
- **Tested with:** playwright-core 1.64.1 (Chromium 156, WebKit 27.2) on Node 24.

## Run it

```sh
node fidelity.mjs manifest.json [--engines chromium,webkit,firefox] [--out dir] [--threshold 24] [--json] [--fail-over <pct>]
node fidelity.mjs <dir>                    # every <name>.png that has a <name>.html beside it
node fidelity.mjs --compare a.png b.png    # PNG against PNG, no browser render (same options)
```

| Option | Meaning |
|---|---|
| `--engines` | The engines that render the HTML. Default `chromium,webkit`. An engine that fails to launch is reported, and its artboards count as not measured |
| `--out dir` | Writes `<name>.<engine>.png` (the rendered HTML), `<name>.<engine>.diff.png` (the heat map) and `summary.json` |
| `--threshold n` | The per-channel tolerance. Default 24; keep it at 24 to compare with the baselines below |
| `--json` | Prints the summary JSON instead of the table |
| `--fail-over pct` | The CI gate. Exits 1 when any artboard's real % exceeds `pct` in any engine, or could not be measured. Without it the exit code is always 0. Usage and setup errors exit 2 |

### The manifest

A JSON array, or `{ "artboards": [...] }`. Paths are relative to the manifest.

```json
[
  { "name": "stage-b-Row", "reference": "pairs/stage-b-Row.png", "html": "pairs/stage-b-Row.html" },
  { "name": "frag", "reference": "frag.png", "html": "frag.html", "wrap": true },
  { "name": "centred", "reference": "c.png", "html": "c.html", "selector": ".scrim-artboard > *", "subpixel": { "x": 0.5, "y": 0 } },
  { "name": "inline", "reference": "i.png", "htmlString": "<!doctype html><body style='margin:0'>…</body>", "clip": { "x": 0, "y": 0, "width": 160, "height": 30 } }
]
```

| Field | Meaning |
|---|---|
| `name` | The row label and the output file stem |
| `reference` | A PNG of what the canvas draws: the truth |
| `html` / `htmlString` | The export: a file path, or the HTML itself |
| `wrap` | `true` wraps a fragment in the minimal page the FigCraft driver used: `<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0">CODE</body></html>` |
| `selector` | Capture this element with an element screenshot, for a page that centres or pads its artboard. Also the element `subpixel` shifts |
| `clip` | Capture this page rectangle in CSS px. **The default, with neither `selector` nor `clip`,** is the reference's size at (0, 0), divided by `scale` |
| `scale` | The device scale factor, for a reference rendered at 2× and so on. Default 1 |
| `subpixel` | `{x, y}`: the fractional screen position the reference was captured at. The target (the `selector`, or `body`'s first element) is translated to that phase. Omit it to render as is |
| `viewport` | The page viewport. Default `{ "width": 1600, "height": 1200 }`, as in the probes |

## Against FigCraft (and the fork)

The two halves, as `fcfidelity.mjs` produced them:
- **Canvas:** `figcraft-cli export <file> --node <id> -o <name>.png`, the tiny-skia renderer the app draws with.
- **HTML:** `inspect.code {language: html}`, run headless with `figcraft-cli run <script.json> --in <file>`. The last stdout line is `{"ok":true,"result":{"code":"<fragment>"}}`. The fragment is wrapped in the minimal page and the frame sits at (0, 0), so the default clip is right.

The commands, one frame by hand:

```sh
FC=path/to/figcraft-cli
$FC info stage-b.fig --json                               # pages[].frames[]: { id, name, width, height }
$FC export stage-b.fig --node 12 -o pairs/stage-b-Row.png
echo '[{"command":"inspect.code","params":{"id":"12","language":"html"}}]' > row.json
$FC run row.json --in stage-b.fig | tail -n 1 | node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>process.stdout.write(JSON.parse(s).result.code))' > pairs/stage-b-Row.html
# then either "wrap": true in the manifest, or wrap it yourself:
printf '<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0">%s</body></html>' "$(cat pairs/stage-b-Row.html)" > pairs/stage-b-Row.page.html
```

All 19 `.fig` artboards in one go. This is a small wrapper script. Save it beside `fidelity.mjs` as `figcraft-pairs.mjs`:

```js
// FC=path/to/figcraft-cli node figcraft-pairs.mjs <outDir> stage-a.bare.fig stage-b.fig stage-c.fig stage-d.fig lines.fig
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';

const [out, ...files] = process.argv.slice(2);
const SKIP = new Set(['FigJam note']); // not in the standard set (artboards.md)
mkdirSync(out, { recursive: true });
const fc = (...a) => execFileSync(process.env.FC ?? 'figcraft-cli', a, { encoding: 'utf8', maxBuffer: 64 << 20 });
const manifest = [];
for (const file of files) {
  const doc = basename(file).replace(/(\.bare)?\.fig$/, '');
  for (const page of JSON.parse(fc('info', file, '--json')).pages) {
    for (const fr of page.frames) {
      if (SKIP.has(fr.name)) continue;
      const name = `${doc}-${fr.name}`.replace(/[^a-z0-9._-]+/gi, '_');
      fc('export', file, '--node', String(fr.id), '-o', join(out, `${name}.png`));
      const script = join(out, `${name}.script.json`);
      writeFileSync(script, JSON.stringify([{ command: 'inspect.code', params: { id: String(fr.id), language: 'html' } }]));
      const step = JSON.parse(fc('run', script, '--in', file).trim().split('\n').pop());
      if (!step.ok) { console.error(`${name}: ${JSON.stringify(step).slice(0, 200)}`); continue; }
      writeFileSync(join(out, `${name}.html`), step.result.code);
      manifest.push({ name, reference: `${name}.png`, html: `${name}.html`, wrap: true });
    }
  }
}
writeFileSync(join(out, 'manifest.json'), JSON.stringify(manifest, null, 2));
```

```sh
unzip -p Scrim/tests/fixtures/fig/stage-a.fig canvas.fig > stage-a.bare.fig    # FigCraft refuses the CRC-0 zip (artboards.md)
FC=target/release/figcraft-cli node figcraft-pairs.mjs pairs stage-a.bare.fig Scrim/tests/fixtures/fig/{stage-b,stage-c,stage-d,lines}.fig
node fidelity.mjs pairs/manifest.json --engines chromium,webkit --out out
```

**For the fork,** swap the two commands for the fork's own canvas PNG export and its HTML exporter. If the exporter writes a whole page that centres the artboard, give `selector` instead of `wrap`. If the canvas capture comes from a live screen at a fractional position, give `subpixel`. The 20th artboard (the vector board) is drawn by hand: see `artboards.md`.

## Reading the results

An excerpt of the self-test. The references were captured in Chromium, so the WebKit column is a cross-engine comparison there:

```
artboard           chromium real% / aa%          webkit real% / aa%
card               = 0 / 0                         0.373 / 0.603
centred-phase-0      0.215 / 0.065 122x42≠121x41   0.151 / 0.108 122x42≠121x41
shift6               4.166 / 0.17                  4.528 / 0.767
tint20             = 0 / 0                         0.379 / 0.603

chromium 156.0.8078.4: 5 of 10 identical (1 could not be measured)
webkit 27.2: 0 of 10 identical (1 could not be measured)
```

- **`=`** marks an identical artboard (`realPx === 0`). The totals line is the headline: **N of M identical**, per engine. Errors count in M, so a tool that can't export an artboard doesn't look better for it.
- **real %** is content that is in one image and not the other: a missing image, a dropped mask, wrong layout, different text shaping. **Any real pixel is a bug** in the export or in the canvas, or a measurement condition to fix (fonts, sub-pixel phase).
- **aa %** is edges that moved by at most 1 px. It is noise from rasterisers and sub-pixel positions, and doesn't count against the guarantee.
- **`A≠B`** after the numbers means the sizes differ (reference ≠ rendered). Only the common area was compared, so a size mismatch is a finding even when real % is small.
- **The heat map** (`--out`) shows the reference as a pale grey ground, with antialiasing in **orange** and real differences in **red**.
- **In `summary.json`,** each artboard and engine has `reference`, `rendered`, `compared` (sizes), `realPx`, `aaPx`, `realPct`, `aaPct`, `identical`, `sizeMismatch` and `blockedRequests` (remote loads the page tried). `totals` has `identical`, `artboards`, `errors` and the engine `version`.
- **`blockedRequests > 0`** means the export depends on the network (web fonts, images by URL). In the app it would render differently online and offline.

## Baselines to beat

These numbers come from the standard 20 artboards (`artboards.md`), with this method, at threshold 24, in Chromium 156 and WebKit 27.2 (Oct 2026).

| Tool | Chromium identical | WebKit identical | Source |
|---|---|---|---|
| **Scrim** | **15 of 20** | **16 of 20** | `docs/research/portability-audit.md`, `portability/summary.json` |
| FigCraft v0.7.0 | 8 of 19 | 8 of 19 | `docs/research/figcraft-spike.md` §4, `openpencil/fc-fidelity.json` (no vector board: it isn't a `.fig`) |
| OpenPencil | 5 of 20 | 5 of 20 | `docs/research/openpencil-spike.md` §4, `openpencil/op-fidelity.json` |

**What "canvas" meant for each tool:**
- **Scrim:** an element screenshot of the artboard in the studio, a DOM canvas rendered by the same engine as the HTML, against `@scrim/codegen`'s standalone HTML.
- **FigCraft:** `figcraft-cli export` (tiny-skia) against `inspect.code` HTML.
- **OpenPencil:** the app's PNG export (CanvasKit), or a live-canvas capture where that failed, against its standalone HTML exporter.

Per artboard, real % (Scrim also shows aa %). A dash means not measurable; the reason is noted.

| # | Artboard | Scrim Cr real / aa | Scrim WK real / aa | FigCraft Cr | FigCraft WK | OpenPencil Cr | OpenPencil WK |
|---|---|---|---|---|---|---|---|
| 1 | vectors / Board | 0 / 0 | 0 / 0 | — (not a .fig) | — | — (not a .fig) | — |
| 2 | stage-a / Card | 1.031 / 0.015 | 0.845 / 0.181 | 12.205 | 12.198 | 2.32 | 2.354 |
| 3 | stage-a / Button | 0 / 0 | 0 / 0 | 0 | 0 | 0 | 0 |
| 4 | stage-a / Button copy | 0 / 0 | 0 / 0 | 0 | 0 | 0 | 0 |
| 5 | stage-b / Row | 0 / 0 | 0 / 0 | 28.544 | 28.535 | 5.714 | 5.71 |
| 6 | stage-b / Column | 0 / 0 | 0 / 0 | 2.257 | 2.292 | 25.995 (31×15 vs 60×16) | 25.995 |
| 7 | stage-b / Empty hug | 0.003 / 0 | 0.003 / 0 | 0 | 0 | — (exports 0 high) | — |
| 8 | stage-b / Wrap | 0 / 0 | 0 / 0 | 0 | 0 | — (exports 0 wide) | — |
| 9 | stage-b / Grid | 0 / 0 | 0 / 0 | 0 | 0 | 0 | 0 |
| 10 | stage-b / Free | 0 / 0 | 0 / 0 | 0 | 0 | 0 | 0 |
| 11 | stage-b / Effects | 0.924 / 0 | 0.687 / 0.18 | 27.444 | 27.448 | 42.833 (610×210 vs 600×200) | 42.839 |
| 12 | stage-b / Masks | 0 / 0 | 0 / 0 | 53.661 | 53.661 | 53.664 | 53.664 |
| 13 | stage-c / Texts | 0 / 0 | 0 / 0 | 1.401 | 1.408 | — (file refused) | — |
| 14 | stage-c / Icon | 0 / 0 | 0 / 0 | 0 | 0 | — | — |
| 15 | stage-c / Card | 0 / 0 | 0 / 0 | 2.082 | 2.031 | — | — |
| 16 | stage-c / Board | 0 / 0 | 0 / 0 | 0.363 | 0.364 | — | — |
| 17 | stage-c / Card B | 0 / 0 | 0 / 0 | 1.732 | 1.689 | — | — |
| 18 | stage-d / Icons | 0 / 0.001 | 0 / 0 | 3.625 | 3.597 | — (render failure) | — |
| 19 | lines / Rules | 0.005 / 0 | 0 / 0 | 3.37 | 3.371 | — (render failure) | — |
| 20 | lines / Stack | 2.105 / 0 | 2.105 / 0 | 0 | 0 | 0 | 0 |

**What Scrim's remaining differences are** (`portability-audit.md`):
- **Card and Effects:** the image fill. The web build had no asset store, so the canvas drew "Image file missing" and the page a broken `<img>`.
- **Empty hug:** one corner pixel.
- **Rules:** a few pixels at a line end.
- **Stack:** a measurement artifact. The standalone page centres the 97 px artboard on a half pixel, and `translate` snaps box edges but not SVG strokes, so the 1 px rule spreads over two rows.

**What FigCraft's are** (`figcraft-spike.md`):
- vectors exported as grey boxes;
- images not emitted;
- grid exported as flex;
- masks dropped;
- text shaped by HarfRust rather than the browser, so text artboards differ by 1–4% even when the layout is right.

**The target for the fork:** at least 15 of 20 in Chromium and 16 of 20 in WebKit, with no artboard's real % above Scrim's on the same row. In practice that means the fork's canvas and its HTML must draw text with the same shaping, images from the same bytes, and masks, gradients and effects the same way.

## Wiring it into `cargo xtask`

FigCraft already runs `cargo xtask layers`, `assets` and `wasm` in CI. Add `cargo xtask fidelity [max-real-pct]`. It generates the pairs, then shells out to Node, with the tool kept in the fork at `tools/html-fidelity/`. Node and `playwright-core` stay outside Cargo's dependency graph. This is written without `unwrap` or `expect`, to fit FigCraft's lints:

```rust
// xtask/src/fidelity.rs
use std::path::Path;
use std::process::Command;

/// `cargo xtask fidelity [max-real-pct]`: canvas vs exported HTML on the standard
/// artboards, in Chromium and WebKit. Needs Node >= 20, PWCORE and PLAYWRIGHT_BROWSERS_PATH.
pub fn fidelity(root: &Path, fail_over: &str) -> Result<(), String> {
    let tool = root.join("tools/html-fidelity");
    let pairs = root.join("target/fidelity/pairs");
    let out = root.join("target/fidelity/out");
    let fixtures = ["stage-a.bare.fig", "stage-b.fig", "stage-c.fig", "stage-d.fig", "lines.fig"]
        .map(|f| root.join("tests/fixtures/fig").join(f));

    // 1. the pairs: the canvas PNG and the HTML export of every artboard
    let made = Command::new("node")
        .arg(tool.join("figcraft-pairs.mjs"))
        .arg(&pairs)
        .args(&fixtures)
        .env("FC", root.join("target/release/figcraft-cli"))
        .status()
        .map_err(|e| format!("could not run node: {e}"))?;
    if !made.success() {
        return Err("fidelity: making the canvas/HTML pairs failed".into());
    }

    // 2. the comparison, as a gate: exit 0 pass, 1 over the limit, 2 setup error
    let gate = Command::new("node")
        .arg(tool.join("fidelity.mjs"))
        .arg(pairs.join("manifest.json"))
        .args(["--engines", "chromium,webkit", "--out"])
        .arg(&out)
        .args(["--fail-over", fail_over])
        .status()
        .map_err(|e| format!("could not run node: {e}"))?;
    match gate.code() {
        Some(0) => Ok(()),
        Some(1) => Err(format!("fidelity: an artboard's real % is over {fail_over}; see {}", out.join("summary.json").display())),
        _ => Err("fidelity: setup error (playwright-core or its browsers missing?)".into()),
    }
}
```

```rust
// xtask/src/main.rs, in the command match
Some("fidelity") => fidelity::fidelity(&root, args.next().as_deref().unwrap_or("0")),
```

CI setup, once per runner (cache `~/pw`):

```sh
cargo build --release -p figcraft-cli          # or the fork's CLI crate
npm i --prefix ~/pw playwright-core@1.64.1
PLAYWRIGHT_BROWSERS_PATH=~/pw/browsers npx --prefix ~/pw playwright-core install --with-deps chromium webkit
PWCORE=~/pw/node_modules/playwright-core/index.mjs PLAYWRIGHT_BROWSERS_PATH=~/pw/browsers cargo xtask fidelity 0
```

**Choosing the limit.**
- **The strict gate.** The guarantee says any real pixel is a bug, so `--fail-over 0` is the strict gate. Use it once the set passes.
- **Until then, ratchet.** Start at today's worst real %, and lower it as artboards are fixed. Or keep `summary.json` from the main branch, and fail when an artboard's `realPx` grows or an `identical` artboard stops being identical.
- **`sizeMismatch` isn't part of `--fail-over`.** To gate on it, check it in `summary.json` (`jq '[.artboards[].results[] | select(.sizeMismatch)] | length'`).

## Caveats

- **Firefox.** It is supported, but it did not launch inside the sandboxed session that tested this tool ("Could not find profile folder", the same limit `portability-audit.md` hit). So its results are unverified. On a normal machine or a CI runner it should launch like the others.
- **The diff runs in Chromium** for every engine. That keeps one decoder and one compositing path, as in the probes. PNG colour management is Chromium's.
- **The reference is the truth.** If the canvas capture includes chrome (selection, grid, rulers) or is at the wrong zoom, every number is off. Capture at 100%, chrome hidden, at the same `scale`.
- **Fonts must match.** With the network blocked, the canvas and the HTML must have the same faces available. Inline web fonts as `data:` URLs, or make sure both fall back to the same system faces. Otherwise text artboards show real differences that are about the setup, not the export.
- **The minimum common size** hides content outside the overlap. Read `sizeMismatch`.
- **1× only** in every baseline. Other scales are supported (`scale`) but have no baseline.
