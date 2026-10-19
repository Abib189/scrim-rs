// Resolve every --scrim-* token of apps/studio/src/theme.css in both themes,
// and every derived colour expression the chrome rules use, in headless Chrome.
// Output: resolved.json next to this script. Read-only on the repo.
import { chromium } from 'playwright-core';
import { readFileSync, writeFileSync } from 'node:fs';

const REPO = (process.env.SCRIM_REPO ?? new URL('../../../', import.meta.url).pathname.replace(/\/$/, ''));
const css = readFileSync(`${REPO}/apps/studio/src/theme.css`, 'utf8');
const lines = css.split('\n');

// the three token blocks: theme-independent :root (14-86), dark (95-165), light (168-228)
function block(from, to) {
  const out = {};
  const text = lines.slice(from - 1, to).join('\n').replace(/\/\*[\s\S]*?\*\//g, '');
  for (const m of text.matchAll(/(--scrim-[a-z0-9-]+)\s*:\s*([^;]+);/g)) out[m[1]] = m[2].trim().replace(/\s+/g, ' ');
  return out;
}
const base = block(14, 86);
const dark = block(95, 165);
const light = block(168, 228);

// derived expressions used by chrome rules (theme.css line numbers in the key)
const EXPRS = {
  'resize-handle-active (theme.css:425)': 'color-mix(in srgb, var(--scrim-accent-mark) 50%, transparent)',
  'chip-slop-border (theme.css:446)': 'color-mix(in srgb, var(--scrim-warning) 30%, transparent)',
  'lint-card-border (theme.css:697)': 'color-mix(in srgb, var(--scrim-warning) 25%, transparent)',
  'lint-group-border (theme.css:879)': 'color-mix(in srgb, var(--scrim-warning) 18%, transparent)',
  'hint-border (theme.css:681)': 'color-mix(in srgb, var(--scrim-text-2) 22%, transparent)',
  'lint-quality-border (theme.css:891)': 'color-mix(in srgb, var(--scrim-text-2) 16%, transparent)',
  'swatch-btn-edge (theme.css:2250)': 'color-mix(in srgb, var(--scrim-text-1) 12%, transparent)',
  'swatch-edge (theme.css:611)': 'rgb(var(--scrim-overlay-rgb) / 0.18)',
  'swatch-stack-edge (theme.css:626)': 'rgb(var(--scrim-overlay-rgb) / 0.2)',
};

const browser = await chromium.launch({ executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true });
const page = await browser.newPage({ deviceScaleFactor: 1 });
const result = { base: {}, dark: {}, light: {}, exprs: { dark: {}, light: {} }, filters: { dark: {}, light: {} }, probes: { dark: {}, light: {} }, chrome: browser.version() };

for (const theme of ['dark', 'light']) {
  await page.setContent(`<!doctype html><html data-theme="${theme}"><head><style>${css}</style></head><body>
    <div id="p"></div>
    <div id="f1" style="position:fixed;left:0;top:0;width:20px;height:20px;background:var(--scrim-accent-fill);filter:brightness(0.92)"></div>
    <div id="f2" style="position:fixed;left:20px;top:0;width:20px;height:20px;background:var(--scrim-accent-fill);filter:brightness(0.84)"></div>
    <div id="f3" style="position:fixed;left:40px;top:0;width:20px;height:20px;background:var(--scrim-danger-fill);filter:brightness(0.92)"></div>
    <div id="f4" style="position:fixed;left:60px;top:0;width:20px;height:20px;background:var(--scrim-accent-fill)"></div>
    <div class="scrim-app" style="position:fixed;left:0;top:40px">
      <div class="scrim-mod-pill" id="modpill">⇧ snap</div>
      <div class="scrim-arrange-tip" id="arrangetip">Align</div>
      <div class="scrim-tip" id="tip">Frame <span class="scrim-tip__key" id="tipkey">F</span></div>
      <div class="scrim-layer-row" data-selected="true" id="selrow"><span class="scrim-layer-name">Card</span><span class="scrim-layer-meta" id="selmeta">row · 28</span></div>
    </div>
  </body></html>`);
  const data = await page.evaluate(({ names, exprs }) => {
    const root = getComputedStyle(document.documentElement);
    const p = document.getElementById('p');
    const tokens = {};
    for (const n of names) {
      const raw = root.getPropertyValue(n).trim();
      let color = null;
      if (CSS.supports('color', raw)) {
        p.style.color = '';
        p.style.color = `var(${n})`;
        color = getComputedStyle(p).color;
      }
      tokens[n] = { computed: raw, color };
    }
    const ex = {};
    for (const [k, e] of Object.entries(exprs)) {
      p.style.color = '';
      p.style.color = e;
      ex[k] = getComputedStyle(p).color;
    }
    const probe = (id) => {
      const s = getComputedStyle(document.getElementById(id));
      return { color: s.color, background: s.backgroundColor, fontSize: s.fontSize, opacity: s.opacity };
    };
    return { tokens, ex, probes: { modPill: probe('modpill'), arrangeTip: probe('arrangetip'), tip: probe('tip'), tipKey: probe('tipkey'), selectedRow: probe('selrow'), selectedRowMeta: probe('selmeta') } };
  }, { names: [...new Set([...Object.keys(base), ...Object.keys(dark), ...Object.keys(light)])], exprs: EXPRS });
  result[theme] = data.tokens;
  result.exprs[theme] = data.ex;
  result.probes[theme] = data.probes;
  // sample the filtered fills from a real screenshot
  const png = await page.screenshot({ clip: { x: 0, y: 0, width: 80, height: 20 } });
  result.filters[theme] = await page.evaluate(async (b64) => {
    const img = new Image();
    img.src = `data:image/png;base64,${b64}`;
    await img.decode();
    const c = document.createElement('canvas');
    c.width = 80;
    c.height = 20;
    const ctx = c.getContext('2d');
    ctx.drawImage(img, 0, 0);
    const px = (x) => Array.from(ctx.getImageData(x, 10, 1, 1).data.slice(0, 3));
    return { accentHover92: px(10), accentPressed84: px(30), dangerHover92: px(50), accentFillUnfiltered: px(70) };
  }, png.toString('base64'));
}
await browser.close();

result.source = { base, dark, light, exprs: EXPRS };
writeFileSync(new URL('./resolved.json', import.meta.url), JSON.stringify(result, null, 2));
console.log('tokens', Object.keys(result.dark).length, 'chrome', result.chrome);
console.log(JSON.stringify(result.filters), JSON.stringify(result.probes.light.modPill), JSON.stringify(result.exprs.dark));
