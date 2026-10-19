// independent reference capture (not fidelity.mjs): Chromium, network blocked, fonts ready
import { readFileSync, writeFileSync } from 'node:fs';
const pw = await import(process.env.PWCORE);
const dir = process.argv[2], engine = process.argv[3] ?? 'chromium', suffix = process.argv[4] ?? '';
const b = await pw[engine].launch();
const shot = async (html, f) => {
  const ctx = await b.newContext({ viewport: { width: 1600, height: 1200 } });
  await ctx.route('**/*', (r) => (r.request().url().startsWith('data:') ? r.continue() : r.abort()));
  const p = await ctx.newPage(); await p.setContent(html, { waitUntil: 'load' }); await p.evaluate(() => document.fonts.ready);
  const buf = await f(p); await ctx.close(); return buf;
};
const rd = (n) => readFileSync(`${dir}/${n}`, 'utf8');
writeFileSync(`${dir}/card${suffix}.png`, await shot(rd('card.html'), (p) => p.screenshot({ clip: { x: 0, y: 0, width: 300, height: 180 } })));
writeFileSync(`${dir}/frag${suffix}.png`, await shot(`<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0">${rd('frag.html')}</body></html>`, (p) => p.screenshot({ clip: { x: 0, y: 0, width: 120, height: 40 } })));
writeFileSync(`${dir}/centred${suffix}.png`, await shot(rd('centred.html'), (p) => p.locator('main.ab > *').screenshot()));
writeFileSync(`${dir}/inline${suffix}.png`, await shot("<!doctype html><body style='margin:0'><p style='margin:0;font:20px serif'>Inline string</p></body>", (p) => p.screenshot({ clip: { x: 0, y: 0, width: 160, height: 30 } })));
await b.close();
