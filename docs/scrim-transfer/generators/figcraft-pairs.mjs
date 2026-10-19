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
