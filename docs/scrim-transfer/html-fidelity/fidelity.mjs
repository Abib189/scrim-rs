#!/usr/bin/env node
/**
 * HTML fidelity: does a tool's exported HTML render the same as its canvas?
 *
 * Standalone (Node >= 20, ESM). Its only dependency is playwright-core, loaded
 * from $PWCORE (a path to playwright-core's index.mjs or its package folder) or
 * by a normal import. Browsers come from PLAYWRIGHT_BROWSERS_PATH or
 * Playwright's default cache. See README.md for the method and the baselines.
 *
 * The method (Scrim's docs/research/portability/probe.mjs, kept exactly):
 * - both images are composited on white and compared at their minimum common size;
 * - a pixel differs when any RGB channel differs by more than THRESHOLD (24);
 * - a differing pixel is "antialiasing" when a near-equal pixel exists within
 *   1 px in BOTH directions (A's pixel near B's 3x3 neighbourhood, and B's near
 *   A's), otherwise "real";
 * - the outer 1 px frame is excluded, and percentages are of (w-2) x (h-2);
 * - the HTML is rendered with the network blocked (only data: URLs), after
 *   document.fonts.ready, optionally shifted by a CSS `translate` so it shares
 *   the reference capture's sub-pixel phase.
 * The diff itself always runs in a Chromium page (canvas drawImage on white,
 * getImageData), whatever engine rendered the HTML.
 *
 *   node fidelity.mjs manifest.json [--engines chromium,webkit,firefox] [--out dir]
 *                     [--threshold 24] [--json] [--fail-over <pct>]
 *   node fidelity.mjs <dir>          (every <name>.png with a <name>.html beside it)
 *   node fidelity.mjs --compare a.png b.png [--out dir] [--json] [--fail-over <pct>]
 *
 * Exit code: 0, unless --fail-over is given and an artboard's real % exceeds it
 * in some engine (or could not be measured): then 1. Usage errors exit 2.
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

// ── arguments ──

const USAGE = 'usage: node fidelity.mjs <manifest.json | dir> [--engines chromium,webkit,firefox] [--out dir] [--threshold 24] [--json] [--fail-over pct]\n       node fidelity.mjs --compare a.png b.png [--out dir] [--threshold 24] [--json] [--fail-over pct]';

function parseArgs(argv) {
  const o = { engines: ['chromium', 'webkit'], threshold: 24, json: false, out: null, failOver: null, compare: null, input: null };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const val = () => { if (i + 1 >= argv.length) usage(`${a} needs a value`); return argv[++i]; };
    if (a === '--engines') o.engines = val().split(',').map((s) => s.trim()).filter(Boolean);
    else if (a === '--out') o.out = resolve(val());
    else if (a === '--threshold') o.threshold = Number(val());
    else if (a === '--json') o.json = true;
    else if (a === '--fail-over') o.failOver = Number(val());
    else if (a === '--compare') { o.compare = [resolve(val()), resolve(val())]; }
    else if (a === '--help' || a === '-h') { console.log(USAGE); process.exit(0); }
    else if (a.startsWith('--')) usage(`unknown option ${a}`);
    else if (!o.input) o.input = resolve(a);
    else usage(`unexpected argument ${a}`);
  }
  if (!o.compare && !o.input) usage('give a manifest, a directory or --compare a.png b.png');
  if (!Number.isFinite(o.threshold) || o.threshold < 0) usage('--threshold must be a number >= 0');
  if (o.failOver !== null && !Number.isFinite(o.failOver)) usage('--fail-over must be a number (a percentage)');
  for (const e of o.engines) if (!['chromium', 'webkit', 'firefox'].includes(e)) usage(`unknown engine ${e}`);
  return o;
}

function usage(msg) {
  console.error(`fidelity: ${msg}\n${USAGE}`);
  process.exit(2);
}

// ── playwright-core ──

async function loadPlaywright() {
  const env = process.env.PWCORE;
  const tries = [];
  if (env) {
    const p = resolve(env);
    tries.push(existsSync(p) && statSync(p).isDirectory() ? join(p, 'index.mjs') : p);
  }
  tries.push('playwright-core');
  try { tries.push(createRequire(join(process.cwd(), 'x.js')).resolve('playwright-core')); } catch { /* not in cwd */ }
  for (const t of tries) {
    try {
      const mod = await import(t.startsWith('/') ? pathToFileURL(t).href : t);
      const pw = mod.chromium ? mod : mod.default;
      if (pw?.chromium) return pw;
    } catch { /* next */ }
  }
  console.error('fidelity: playwright-core not found. Set PWCORE=/path/to/node_modules/playwright-core/index.mjs or run from a folder where `npm i playwright-core` was done.');
  process.exit(2);
}

// ── PNG size and inputs ──

function pngSize(buf, file) {
  const sig = '89504e470d0a1a0a';
  if (buf.length < 24 || buf.subarray(0, 8).toString('hex') !== sig) throw new Error(`${file} is not a PNG`);
  return [buf.readUInt32BE(16), buf.readUInt32BE(20)];
}

// The minimal page the FigCraft driver wrapped `inspect.code {language: html}` fragments in.
const wrapFragment = (code) => `<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0">${code}</body></html>`;

function loadArtboards(input) {
  if (statSync(input).isDirectory()) {
    return readdirSync(input).filter((f) => f.endsWith('.png') && existsSync(join(input, f.replace(/\.png$/, '.html'))))
      .sort().map((f) => ({ name: f.replace(/\.png$/, ''), reference: join(input, f), html: join(input, f.replace(/\.png$/, '.html')) }));
  }
  const raw = JSON.parse(readFileSync(input, 'utf8'));
  const list = Array.isArray(raw) ? raw : raw.artboards;
  if (!Array.isArray(list)) throw new Error('the manifest must be an array of artboards or { "artboards": [...] }');
  const base = dirname(input);
  return list.map((a, i) => {
    if (!a.name || !a.reference || (!a.html && a.htmlString == null)) throw new Error(`artboard ${i}: needs name, reference and html (or htmlString)`);
    return { ...a, reference: resolve(base, a.reference), html: a.html ? resolve(base, a.html) : undefined };
  });
}

const safe = (s) => String(s).replace(/[^a-z0-9._-]+/gi, '_');
const dataUrl = (buf) => `data:image/png;base64,${buf.toString('base64')}`;

// ── the diff (runs in a Chromium page) ──

async function makeDiffer(pw) {
  const browser = await pw.chromium.launch();
  const page = await browser.newPage();
  const diff = (a, b, threshold) => page.evaluate(async ({ a, b, T }) => {
    const load = (src) => new Promise((res, rej) => { const i = new Image(); i.onload = () => res(i); i.onerror = () => rej(new Error('image decode failed')); i.src = src; });
    const [ia, ib] = await Promise.all([load(a), load(b)]);
    const w = Math.min(ia.width, ib.width);
    const h = Math.min(ia.height, ib.height);
    // composite each image on white, at the minimum common size
    const px = (img) => {
      const c = document.createElement('canvas'); c.width = w; c.height = h;
      const x = c.getContext('2d'); x.fillStyle = '#fff'; x.fillRect(0, 0, w, h); x.drawImage(img, 0, 0);
      return x.getImageData(0, 0, w, h).data;
    };
    const A = px(ia), B = px(ib);
    const near = (P, i, Q, j) => Math.max(Math.abs(P[i] - Q[j]), Math.abs(P[i + 1] - Q[j + 1]), Math.abs(P[i + 2] - Q[j + 2])) <= T;
    // does P's pixel at i match any pixel of Q within 1 px of (x, y)?
    const found = (P, i, Q, x, y) => {
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const X = x + dx, Y = y + dy;
        if (X >= 0 && Y >= 0 && X < w && Y < h && near(P, i, Q, (Y * w + X) * 4)) return true;
      }
      return false;
    };
    let aa = 0, real = 0;
    const out = document.createElement('canvas'); out.width = w; out.height = h;
    const o = out.getContext('2d'); const od = o.createImageData(w, h);
    // heat map: A as a pale grey ground, antialiasing orange, real red
    for (let i = 0; i < w * h * 4; i += 4) { const g = (A[i] + A[i + 1] + A[i + 2]) / 12 + 190; od.data[i] = od.data[i + 1] = od.data[i + 2] = g; od.data[i + 3] = 255; }
    // the outer 1 px frame is the capture boundary: left out of the counts
    for (let y = 1; y < h - 1; y++) for (let x = 1; x < w - 1; x++) {
      const i = (y * w + x) * 4;
      if (near(A, i, B, i)) continue;
      if (found(A, i, B, x, y) && found(B, i, A, x, y)) { aa++; od.data[i] = 255; od.data[i + 1] = 170; od.data[i + 2] = 0; }
      else { real++; od.data[i] = 220; od.data[i + 1] = 0; od.data[i + 2] = 40; }
    }
    o.putImageData(od, 0, 0);
    const n = Math.max(1, (w - 2) * (h - 2));
    return {
      reference: [ia.width, ia.height], rendered: [ib.width, ib.height], compared: [w, h],
      sizeMismatch: ia.width !== ib.width || ia.height !== ib.height,
      realPx: real, aaPx: aa, realPct: +((real / n) * 100).toFixed(3), aaPct: +((aa / n) * 100).toFixed(3),
      identical: real === 0, png: out.toDataURL('image/png'),
    };
  }, { a, b, T: threshold });
  return { diff, close: () => browser.close() };
}

function keepHeatmap(r, outDir, file) {
  if (outDir) writeFileSync(join(outDir, file), Buffer.from(r.png.split(',')[1], 'base64'));
  delete r.png;
  return r;
}

// ── rendering one artboard's HTML in one engine ──

async function render(browser, art, refSize) {
  const scale = art.scale ?? 1;
  const vp = art.viewport ?? { width: 1600, height: 1200 };
  const ctx = await browser.newContext({ viewport: vp, deviceScaleFactor: scale, reducedMotion: 'reduce' });
  // like a design canvas (no remote content): only data: URLs load
  let blocked = 0;
  await ctx.route('**/*', (rt) => {
    if (rt.request().url().startsWith('data:')) return rt.continue();
    blocked++;
    return rt.abort();
  });
  try {
    const page = await ctx.newPage();
    let html = art.htmlString ?? readFileSync(art.html, 'utf8');
    if (art.wrap) html = wrapFragment(html);
    await page.setContent(html, { waitUntil: 'load', timeout: 30000 });
    // fonts ready; then, when the reference was captured at a sub-pixel position,
    // cancel the target's own fraction and apply the reference's (CSS translate:
    // no layout change), so both are rasterised at one sub-pixel phase
    await page.evaluate(async ({ sel, sub }) => {
      await document.fonts.ready;
      if (!sub) return;
      const n = sel ? document.querySelector(sel) : document.body.firstElementChild;
      if (!n) return;
      const q = n.getBoundingClientRect();
      n.style.translate = `${sub.x - (q.left - Math.floor(q.left))}px ${sub.y - (q.top - Math.floor(q.top))}px`;
    }, { sel: art.selector ?? null, sub: art.subpixel ?? null });
    let shot;
    if (art.selector) {
      shot = await page.locator(art.selector).first().screenshot({ animations: 'disabled' });
    } else {
      const clip = art.clip ?? { x: 0, y: 0, width: refSize[0] / scale, height: refSize[1] / scale };
      const fullPage = clip.x + clip.width > vp.width || clip.y + clip.height > vp.height;
      shot = await page.screenshot({ clip, fullPage, animations: 'disabled' });
    }
    return { shot, blocked };
  } finally {
    await ctx.close();
  }
}

// ── modes ──

async function runCompare(pw, opts) {
  const [a, b] = opts.compare;
  const bufA = readFileSync(a), bufB = readFileSync(b);
  pngSize(bufA, a); pngSize(bufB, b);
  const differ = await makeDiffer(pw);
  try {
    const r = keepHeatmap(await differ.diff(dataUrl(bufA), dataUrl(bufB), opts.threshold), opts.out, `${safe(basename(a, '.png'))}.vs.${safe(basename(b, '.png'))}.diff.png`);
    const summary = { mode: 'compare', a, b, threshold: opts.threshold, ...r };
    if (opts.out) writeFileSync(join(opts.out, 'summary.json'), JSON.stringify(summary, null, 2));
    if (opts.json) console.log(JSON.stringify(summary, null, 2));
    else console.log(`${basename(a)} vs ${basename(b)}: real ${r.realPct}% (${r.realPx} px)  aa ${r.aaPct}% (${r.aaPx} px)  compared ${r.compared.join('x')} (a ${r.reference.join('x')}, b ${r.rendered.join('x')})${r.sizeMismatch ? '  SIZE MISMATCH' : ''}  ${r.identical ? 'IDENTICAL' : ''}`);
    return opts.failOver !== null && r.realPct > opts.failOver ? 1 : 0;
  } finally {
    await differ.close();
  }
}

async function runManifest(pw, opts) {
  const artboards = loadArtboards(opts.input);
  if (!artboards.length) usage(`no artboards in ${opts.input}`);
  const differ = await makeDiffer(pw);
  const engines = {};
  for (const name of opts.engines) {
    try { const b = await pw[name].launch(); engines[name] = { browser: b, version: b.version() }; }
    catch (e) { engines[name] = { error: String(e.message ?? e).split('\n')[0] }; }
  }
  const rows = [];
  try {
    for (const art of artboards) {
      const row = { name: art.name, results: {} };
      rows.push(row);
      let ref;
      try { ref = readFileSync(art.reference); row.referenceSize = pngSize(ref, art.reference); }
      catch (e) { row.error = String(e.message ?? e); continue; }
      for (const [name, eng] of Object.entries(engines)) {
        if (!eng.browser) { row.results[name] = { error: `not launched: ${eng.error}` }; continue; }
        try {
          const { shot, blocked } = await render(eng.browser, art, row.referenceSize);
          if (opts.out) writeFileSync(join(opts.out, `${safe(art.name)}.${name}.png`), shot);
          const r = keepHeatmap(await differ.diff(dataUrl(ref), dataUrl(shot), opts.threshold), opts.out, `${safe(art.name)}.${name}.diff.png`);
          row.results[name] = { ...r, blockedRequests: blocked };
        } catch (e) {
          row.results[name] = { error: String(e.message ?? e).split('\n')[0] };
        }
      }
    }
  } finally {
    for (const e of Object.values(engines)) await e.browser?.close().catch(() => undefined);
    await differ.close();
  }

  const totals = {};
  for (const name of opts.engines) {
    const rs = rows.map((r) => r.results[name] ?? { error: r.error });
    totals[name] = {
      version: engines[name].version ?? null, launchError: engines[name].error,
      identical: rs.filter((r) => r.identical).length, artboards: rs.length, errors: rs.filter((r) => r.error).length,
    };
  }
  const summary = {
    tool: 'html-fidelity', date: new Date().toISOString(), threshold: opts.threshold, input: opts.input,
    method: 'composite on white; min common size; pixel differs when any RGB channel > threshold; antialiasing when a near-equal pixel exists within 1 px both ways; outer 1 px frame excluded; network blocked (data: only); fonts ready',
    totals, artboards: rows,
  };
  if (opts.out) writeFileSync(join(opts.out, 'summary.json'), JSON.stringify(summary, null, 2));
  if (opts.json) console.log(JSON.stringify(summary, null, 2));
  else printTable(rows, totals, opts.engines);

  if (opts.failOver === null) return 0;
  const over = rows.some((r) => opts.engines.some((e) => { const x = r.results[e]; return !x || x.error || x.realPct > opts.failOver; }));
  if (over && !opts.json) console.error(`fidelity: FAIL, an artboard's real % exceeds ${opts.failOver} (or could not be measured)`);
  return over ? 1 : 0;
}

function printTable(rows, totals, engines) {
  const pad = (s, n) => String(s).padEnd(n);
  const w = Math.max(8, ...rows.map((r) => r.name.length)) + 2;
  console.log(pad('artboard', w) + engines.map((e) => pad(`${e} real% / aa%`, 30)).join(''));
  for (const r of rows) {
    const cells = engines.map((e) => {
      const x = r.results[e];
      if (!x || x.error) return pad(`ERROR ${(x?.error ?? r.error ?? '').slice(0, 23)}`, 30);
      const size = x.sizeMismatch ? ` ${x.reference.join('x')}≠${x.rendered.join('x')}` : '';
      return pad(`${x.identical ? '= ' : '  '}${x.realPct} / ${x.aaPct}${size}`, 30);
    });
    console.log(pad(r.name, w) + cells.join(''));
    if (r.error) console.log(`  ${r.error}`);
  }
  console.log('');
  for (const e of engines) {
    const t = totals[e];
    const v = t.version ? ` ${t.version}` : '';
    console.log(`${e}${v}: ${t.identical} of ${t.artboards} identical${t.errors ? ` (${t.errors} could not be measured)` : ''}${t.launchError ? ` [launch failed: ${t.launchError}]` : ''}`);
  }
}

// ── main ──

const opts = parseArgs(process.argv.slice(2));
if (opts.out) mkdirSync(opts.out, { recursive: true });
const pw = await loadPlaywright();
process.exitCode = opts.compare ? await runCompare(pw, opts) : await runManifest(pw, opts);
