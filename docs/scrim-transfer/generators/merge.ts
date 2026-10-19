/** Merges out/fixtures.json and out/security.json into
 * docs/transfer/cases/fig-import.json, adding the per-node expectations the
 * Scrim tests assert — each one CHECKED here against the importer's actual
 * output (the script fails if any does not hold).
 * Run: node gen-fixtures.ts > out/fixtures.json && node gen-security.ts > out/security.json && node merge.ts */

import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';

const REPO = (process.env.SCRIM_REPO ?? new URL('../../../', import.meta.url).pathname.replace(/\/$/, ''));
const OUT = `${REPO}/docs/transfer/cases/fig-import.json`;
const fx = JSON.parse(readFileSync(new URL('./out/fixtures.json', import.meta.url), 'utf8'));
const sec = JSON.parse(readFileSync(new URL('./out/security.json', import.meta.url), 'utf8'));

type Sel = string | { name: string; under: string };
type Check =
  | { test: string; node: Sel; equals: [string, unknown] }
  | { test: string; node: Sel; subset: [string, Record<string, unknown>] }
  | { test: string; node: Sel; absent: string }
  | { test: string; node: Sel; matches: [string, string] }
  | { test: string; report: [string, unknown] }
  | { test: string; reportMatches: [string, string] }
  | { test: string; doc: [string, unknown] };

const get = (o: any, path: string): unknown => (path === '' ? o : path.split('.').reduce((a, k) => (a == null ? undefined : a[k]), o));
const canon = (v: any): any => (Array.isArray(v) ? v.map(canon) : v && typeof v === 'object' ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])])) : v);
/** Deep equality, key order ignored (vitest toEqual). */
const same = (a: unknown, b: unknown): boolean => JSON.stringify(canon(a)) === JSON.stringify(canon(b));
const subset = (o: any, s: Record<string, unknown>): boolean => o != null && Object.entries(s).every(([k, v]) => same(o[k], v));

const ASSERTS: Record<string, Check[]> = {
  'stage-a.fig': [
    { test: 'one patch: a valid document, pages as bands of roots', doc: ['rootNames', ['Card', 'Button', 'Button copy']] },
    { test: 'one patch: … each page reported', report: ['pages', [{ name: 'Page One', roots: 1 }, { name: 'Page Two', roots: 2 }]] },
    { test: 'one patch: … pages as bands', node: 'Card', equals: ['canvas', { x: 0, y: 0 }] },
    { test: "page two's band sits below page one's (≥ 300 + 400)", node: 'Button', equals: ['canvas', { x: 0, y: 700 }] },
    { test: 'frames are free frames that clip as Figma frames do', node: 'Card', subset: ['layout', { display: 'block', position: 'relative' }] },
    { test: 'frames are free frames that clip as Figma frames do', node: 'Card', subset: ['style', { width: 400, height: 300, background: '#ffffff', borderRadius: 12, overflow: 'hidden' }] },
    { test: 'groups do not clip', node: 'Group', absent: 'style.overflow' },
    { test: 'groups do not clip', node: 'In group', subset: ['layout', { position: 'absolute', left: 0, top: 0 }] },
    { test: 'shapes: fills, radius, opacity', node: 'Swatch', subset: ['style', { width: 80, height: 40, background: '#336699', borderRadius: 6, opacity: 0.5 }] },
    { test: 'shapes: placement', node: 'Swatch', subset: ['layout', { position: 'absolute', left: 16, top: 16 }] },
    { test: 'strokes by alignment: inside → border', node: 'Bordered', equals: ['style.border', '2px solid #000000'] },
    { test: 'strokes by alignment: centre → outline pulled in half its width', node: 'Outlined', subset: ['style', { outline: '4px solid #ff0000', outlineOffset: '-2px' }] },
    { test: 'ellipse', node: 'Dot', subset: ['style', { borderRadius: '50%', background: '#ff8000' }] },
    { test: 'a 90°-rotated LINE is a real <svg> line in the Line tool box', node: 'Rule', subset: ['layout', { position: 'absolute', left: 15, top: 139 }] },
    { test: 'a 90°-rotated LINE …', node: 'Rule', subset: ['style', { width: 2, height: 302 }] },
    { test: 'a 90°-rotated LINE … no fill, no CSS transform', node: 'Rule', absent: 'style.background' },
    { test: 'a 90°-rotated LINE … no fill, no CSS transform', node: 'Rule', absent: 'style.transform' },
    { test: 'a 90°-rotated LINE …', node: 'Rule', equals: ['svg', { viewBox: [0, 0, 2, 302], children: [{ tag: 'line', stroke: '#808080', vectorEffect: 'non-scaling-stroke', strokeWidth: 2, x1: 1, y1: 1, x2: 1, y2: 301 }] }] },
    { test: 'hidden', node: 'Hidden', equals: ['style.display', 'none'] },
    { test: 'text: characters with line breaks', node: 'Title', equals: ['text', 'Hello\nWorld'] },
    { test: 'text: family, weight, size, colour, auto height', node: 'Title', subset: ['style', { fontFamily: 'Inter, sans-serif', fontWeight: 700, fontSize: 20, color: '#1a1a1a', width: 200, whiteSpace: 'pre-wrap' }] },
    { test: 'text: auto height (no height)', node: 'Title', absent: 'style.height' },
    { test: 'an image fill becomes an image layer', node: 'Photo', subset: ['', { type: 'image', alt: 'Photo' }] },
    { test: 'an image fill becomes an image layer', node: 'Photo', equals: ['style.objectFit', 'cover'] },
    { test: 'an image fill …', report: ['images', { imported: 1, failed: [] }] },
    { test: 'an instance is the master with its overrides', node: { name: 'Button bg', under: 'Button copy' }, equals: ['style.background', '#ff0000'] },
    { test: 'the master is untouched', node: { name: 'Button bg', under: 'Button' }, equals: ['style.background', '#000000'] },
    { test: 'reports what it skipped', reportMatches: ['summary', '^\\d+ layers imported, \\d+ approximated, 2 skipped$'] },
  ],
  'stage-b.fig': [
    { test: 'auto layout → flex', node: 'Row', subset: ['layout', { display: 'flex', flexDirection: 'row', gap: 12, padding: '8px 16px 8px 16px', justifyContent: 'space-between', alignItems: 'center', position: 'relative' }] },
    { test: 'auto layout → flex', node: 'Row', subset: ['style', { width: 400, height: 60 }] },
    { test: 'fixed child', node: 'Fixed', subset: ['layout', { flexShrink: 0 }] },
    { test: 'fixed child', node: 'Fixed', subset: ['style', { width: 40, height: 40 }] },
    { test: 'fixed child is in flow', node: 'Fixed', absent: 'layout.position' },
    { test: 'grow', node: 'Grows', subset: ['layout', { flexGrow: 1, flexBasis: 0 }] },
    { test: 'grow', node: 'Grows', equals: ['style.height', 20] },
    { test: 'grow', node: 'Grows', absent: 'style.width' },
    { test: 'stretch', node: 'Stretches', equals: ['layout.alignSelf', 'stretch'] },
    { test: 'stretch', node: 'Stretches', absent: 'style.height' },
    { test: 'an absolute child', node: 'Badge', subset: ['layout', { position: 'absolute', left: 385, top: 5 }] },
    { test: 'an absolute child', node: 'Badge', subset: ['style', { width: 10, height: 10 }] },
    { test: 'a hugging column leaves its size to its content', node: 'Column', subset: ['layout', { display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'flex-start' }] },
    { test: 'a hugging column …', node: 'Column', absent: 'style.width' },
    { test: 'a hugging column …', node: 'Column', absent: 'style.height' },
    { test: 'auto-width text', node: 'Label', absent: 'style.width' },
    { test: 'empty hug keeps its implicit size', node: 'Empty hug', subset: ['style', { width: 501, height: 72 }] },
    { test: 'wrap', node: 'Wrap', subset: ['layout', { flexWrap: 'wrap', gap: 8, rowGap: 16 }] },
    { test: 'grid', node: 'Grid', subset: ['layout', { display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gridTemplateRows: 'repeat(2, auto)', columnGap: 10, rowGap: 10 }] },
    { test: 'constraints: MAX → right', node: 'Right', subset: ['layout', { position: 'absolute', right: 20, top: 10 }] },
    { test: 'constraints: MAX → right', node: 'Right', absent: 'layout.left' },
    { test: 'constraints: CENTER → calc', node: 'Centred', subset: ['layout', { left: 'calc(50% - 50px)', top: 'calc(50% - 25px)' }] },
    { test: 'constraints: STRETCH → both sides', node: 'Stretched', subset: ['layout', { left: 20, right: 20, bottom: 30 }] },
    { test: 'constraints: STRETCH → both sides', node: 'Stretched', absent: 'style.width' },
    { test: 'constraints: SCALE → %', node: 'Scaled', subset: ['layout', { left: '25%', top: '50%' }] },
    { test: 'constraints: SCALE → %', node: 'Scaled', subset: ['style', { width: '25%', height: '25%' }] },
    { test: 'constraints are "resize", not approximated', report: ['resize', 4] },
    { test: 'shadows', node: 'Shadowed', equals: ['style.boxShadow', '0px 4px 8px 2px rgba(0, 0, 0, 0.25), inset 1px 1px 0px 0px #000000'] },
    { test: 'blurs (half the radius)', node: 'Blurred', subset: ['style', { filter: 'blur(4px)', backdropFilter: 'blur(10px)' }] },
    { test: 'text shadow', node: 'Shadow text', equals: ['style.textShadow', '1px 2px 3px #000000'] },
    { test: 'blend mode', node: 'Multiply', equals: ['style.mixBlendMode', 'multiply'] },
    { test: 'linear exact', node: 'Linear', equals: ['style.background', 'linear-gradient(180deg, #ff0000 0%, #0000ff 100%)'] },
    { test: 'radial', node: 'Radial', equals: ['style.background', 'radial-gradient(ellipse 50px 50px at 50px 50px, #ffffff 0%, #000000 100%)'] },
    { test: 'angular → conic', node: 'Angular', equals: ['style.background', 'conic-gradient(from 90deg at 50px 50px, #ffffff 0%, #000000 100%)'] },
    { test: 'diamond → radial (reported)', node: 'Diamond', matches: ['style.background', '^radial-gradient\\('] },
    { test: 'an image fill on a frame: an image layer behind its content', node: 'Effects', equals: ['children.0', 'Effects image'] },
    { test: 'an image fill on a frame …', node: 'Effects image', subset: ['', { type: 'image' }] },
    { test: 'an image fill on a frame …', node: 'Effects image', subset: ['layout', { position: 'absolute', left: 0, top: 0 }] },
    { test: 'an image fill on a frame …', node: 'Effects image', subset: ['style', { width: '100%', height: '100%', objectFit: 'contain' }] },
    { test: 'a mask: a clipping wrapper', node: 'Masks', equals: ['children', ['Before mask', 'Mask']] },
    { test: 'a mask: a clipping wrapper', node: 'Mask', subset: ['', { type: 'frame' }] },
    { test: 'a mask: a clipping wrapper', node: 'Mask', subset: ['layout', { position: 'absolute', left: 50, top: 40 }] },
    { test: 'a mask: a clipping wrapper', node: 'Mask', subset: ['style', { width: 100, height: 80, overflow: 'hidden', borderRadius: 16 }] },
    { test: 'a mask holds the layers above it', node: 'Mask', equals: ['children', ['Masked A', 'Masked B']] },
    { test: 'masked layers keep their place', node: 'Masked B', subset: ['layout', { position: 'absolute', left: 10, top: 10 }] },
    { test: 'masked layers: plain offsets inside the wrapper', node: 'Masked A', subset: ['layout', { left: -50, top: -40 }] },
    { test: "the mask's own paint isn't drawn", node: 'Mask', absent: 'style.background' },
  ],
  'stage-c.fig': [
    { test: 'line height, letter spacing, alignment, case, decoration', node: 'Spaced', subset: ['style', { lineHeight: 1.5, letterSpacing: '0.05em', textAlign: 'center', textTransform: 'capitalize', textDecoration: 'underline' }] },
    { test: 'vertical alignment: a centring column', node: 'Spaced', subset: ['layout', { display: 'flex', flexDirection: 'column', justifyContent: 'center' }] },
    { test: 'px line height and tracking; justify', node: 'Pixels', subset: ['style', { lineHeight: '24px', letterSpacing: '-0.5px', textAlign: 'justify' }] },
    { test: 'variable-font axes and OpenType features', node: 'Variable', subset: ['style', { fontVariationSettings: '"wght" 650, "wdth" 80', fontFeatureSettings: '"tnum" 1, "ss01" 1, "liga" 0' }] },
    { test: 'mixed runs: 6 of 9 characters are the base style', node: 'Runs', equals: ['style.fontSize', 16] },
    { test: 'mixed runs: 3 of 4 use the shared Heading style', node: 'Shared runs', subset: ['style', { fontSize: 32, fontWeight: 700 }] },
    { test: "a layer on a shared style draws with the style's values", node: 'Styled', subset: ['style', { fontSize: 32, fontWeight: 700 }] },
    { test: 'unknown font kept by name with a generic fallback', node: 'Unknown font', equals: ['style.fontFamily', '"Acme Grotesk", sans-serif'] },
    { test: 'system font', node: 'System font', subset: ['style', { fontFamily: '"Helvetica Neue", sans-serif', fontWeight: 300 }] },
    { test: 'masters become components', doc: ['components', ['Icon', 'Card']] },
    { test: 'instances become real instances, placed where Figma had them', node: 'Card A', subset: ['', { type: 'instance', instanceOf: 'Card' }] },
    { test: 'instances … placed', node: 'Card A', subset: ['layout', { position: 'absolute', left: 40, top: 50 }] },
    { test: 'an instance at the top level keeps its own size and fill (pinned overrides)', node: 'Card B', subset: ['style', { width: 240, background: '#333333' }] },
    { test: 'components counted', report: ['components', { masters: 2, instances: 2, detached: 2 }] },
    { test: 'text override pinned', node: { name: 'Card title', under: 'Card A' }, equals: ['text', 'Pricing'] },
    { test: 'fill override pinned (float32 0.9 × 255 = 229.49997 → e5)', node: { name: 'Card bg', under: 'Card A' }, equals: ['style.background', '#ffe599'] },
    { test: 'visibility override pinned', node: { name: 'Card badge', under: 'Card A' }, equals: ['style.display', 'none'] },
    { test: 'the master is untouched', node: 'Card title', equals: ['text', 'Title'] },
    { test: 'a nested instance inside a master is a detached copy', node: 'Nested icon', subset: ['', { type: 'frame' }] },
    { test: 'an instance of an internal-page master is a detached copy', node: 'Internal icon copy', subset: ['', { type: 'frame' }] },
    { test: 'colour variables become tokens', doc: ['tokens.colors', { 'brand-primary': '#1a33e5' }] },
    { test: 'colour variables become tokens', report: ['tokens', { colors: 1 }] },
  ],
  'stage-d.fig': [
    { test: 'a filled vector: a box whose svg draws the geometry', node: 'Triangle', subset: ['', { type: 'box' }] },
    { test: 'a filled vector …', node: 'Triangle', subset: ['layout', { position: 'absolute', left: 10, top: 10 }] },
    { test: 'a filled vector …', node: 'Triangle', subset: ['style', { width: 24, height: 24 }] },
    { test: 'a filled vector …', node: 'Triangle', equals: ['svg', { viewBox: [0, 0, 24, 24], children: [{ tag: 'path', d: 'M0 0L24 0L12 24Z', fill: '#000000' }] }] },
    { test: 'a filled vector … no background', node: 'Triangle', absent: 'style.background' },
    { test: 'even-odd winding and a gradient fill as a def', node: 'Ring', equals: ['svg.defs', [{ tag: 'linearGradient', id: 'g0', x1: 0, y1: 12, x2: 24, y2: 12, units: 'userSpaceOnUse', stops: [{ offset: 0, color: '#ff0000' }, { offset: 1, color: '#0000ff' }] }]] },
    { test: 'even-odd winding …', node: 'Ring', subset: ['svg.children.0', { tag: 'path', fill: 'url(#g0)', fillRule: 'evenodd' }] },
    { test: 'a 0-high vector is drawn 1:1 in a 1 px box', node: 'Gradient rule', subset: ['style', { width: 100, height: 1 }] },
    { test: 'a 0-high vector …', node: 'Gradient rule', equals: ['svg.viewBox', [0, 0, 100, 1]] },
    { test: 'strokes are filled outlines', node: 'Gradient rule', subset: ['svg.children.0', { tag: 'path', d: 'M0 -1L100 -1L100 1L0 1Z', fill: 'url(#g0)' }] },
    { test: 'a star, with a drop shadow as filter', node: 'Star', equals: ['style.filter', 'drop-shadow(0px 2px 2px rgba(0, 0, 0, 0.5))'] },
    { test: 'polygon: fill + centred stroke without stroke geometry', node: 'Hexagon', subset: ['svg.children.1', { fill: 'none', stroke: '#ff0000' }] },
    { test: 'booleans flattened (no operands)', node: 'Union', absent: 'children' },
    { test: 'arcs exact (even-odd)', node: 'Arc', subset: ['svg.children.0', { fillRule: 'evenodd' }] },
    { test: 'arcs exact: no border-radius', node: 'Arc', absent: 'style.borderRadius' },
    { test: 'no geometry: not drawn', node: 'No geometry', absent: 'svg' },
    { test: 'a vector mask clips with its exact shape', node: 'Star mask', equals: ['style.clipPath', "path('M12 0L15 9L24 9L17 15L20 24L12 18L4 24L7 15L0 9L9 9Z')"] },
    { test: 'a vector mask …', node: 'Star mask', equals: ['children', ['Starred']] },
  ],
  'lines.fig': [
    { test: 'plain LINE: the box import’s own box', node: 'Rule', subset: ['layout', { position: 'absolute', left: 20, top: 39 }] },
    { test: 'plain LINE …', node: 'Rule', subset: ['style', { width: 300, height: 2 }] },
    { test: 'plain LINE …', node: 'Rule', absent: 'style.background' },
    { test: 'plain LINE …', node: 'Rule', equals: ['svg', { viewBox: [0, 0, 300, 2], children: [{ tag: 'line', stroke: '#808080', vectorEffect: 'non-scaling-stroke', strokeWidth: 2, x1: 0, y1: 1, x2: 300, y2: 1 }] }] },
    { test: 'caps and a translucent paint', node: 'Hairline', subset: ['svg.children.0', { stroke: 'rgba(0, 0, 0, 0.5)', strokeWidth: 1, strokeLinecap: 'round' }] },
    { test: 'a dash', node: 'Dashed', subset: ['svg.children.0', { strokeWidth: 1, strokeDasharray: [4, 2] }] },
    { test: 'square cap', node: 'Upright', subset: ['svg.children.0', { strokeWidth: 4, strokeLinecap: 'square' }] },
    { test: 'rotated: no CSS transform', node: 'Diagonal', absent: 'style.transform' },
    { test: 'an arrowhead is drawn as a plain end', node: 'Arrow', equals: ['svg.children.0.tag', 'line'] },
    { test: 'a gradient stroke stays the box import', node: 'Gradient rule', absent: 'svg' },
    { test: 'a gradient stroke stays the box import', node: 'Gradient rule', matches: ['style.background', 'linear-gradient'] },
    { test: 'a 1 px divider', node: 'Divider', subset: ['layout', { position: 'absolute', left: 20, top: 320 }] },
    { test: 'a 1 px divider', node: 'Divider', subset: ['style', { width: 440, height: 1 }] },
    { test: 'a 1 px divider', node: 'Divider', equals: ['svg.children.0', { tag: 'line', stroke: '#1a1a1a', vectorEffect: 'non-scaling-stroke', strokeWidth: 1, x1: 0, y1: 0.5, x2: 440, y2: 0.5 }] },
    { test: 'a 2 px column rule', node: 'Column rule', subset: ['svg.children.0', { x1: 1, y1: 0, x2: 1, y2: 120, strokeWidth: 2 }] },
    { test: 'in auto layout: its flow box unchanged', node: 'Separator', absent: 'layout.position' },
    { test: 'in auto layout …', node: 'Separator', subset: ['style', { width: 200, height: 1 }] },
    { test: 'in auto layout …', node: 'Separator', equals: ['svg.children.0.tag', 'line'] },
    { test: 'a radius: stays a box', node: 'Pill', absent: 'svg' },
    { test: 'a radius: stays a box', node: 'Pill', subset: ['style', { background: '#1a1a1a', borderRadius: 1 }] },
    { test: 'a stroke: stays a box', node: 'Outlined rule', absent: 'svg' },
    { test: 'only the arrowhead is approximated', report: ['approximated', 1] },
  ],
};

/** The rotated lines' end points (lines.test.ts): left/top + the line's x1,y1 / x2,y2. */
const lineEnds = (n: any) => { const c = n.svg.children[0]; return [[n.layout.left + c.x1, n.layout.top + c.y1], [n.layout.left + c.x2, n.layout.top + c.y2]].map((p) => p.map((v: number) => Math.round(v * 1000) / 1000)); };

const failures: string[] = [];
for (const f of fx.fixtures) {
  const short = f.file.split('/').pop();
  const find = (sel: Sel): any => {
    if (typeof sel === 'string') return f.nodes.find((n: any) => n.name === sel);
    const anc = f.nodes.find((n: any) => n.name === sel.under);
    return f.nodes.find((n: any) => n.name === sel.name && anc && n.path.startsWith(`${anc.path}/`));
  };
  const checks = ASSERTS[short] ?? [];
  const recorded: any[] = [];
  for (const c of checks) {
    let ok = false;
    let rec: any;
    if ('report' in c) { ok = same(get(f.report, c.report[0]), c.report[1]); rec = { test: c.test, report: c.report[0], equals: c.report[1] }; }
    else if ('reportMatches' in c) { ok = new RegExp(c.reportMatches[1]).test(String(get(f.report, c.reportMatches[0]))); rec = { test: c.test, report: c.reportMatches[0], matches: c.reportMatches[1] }; }
    else if ('doc' in c) {
      const v = c.doc[0] === 'rootNames' ? f.rootNames : c.doc[0] === 'components' ? f.components : get({ tokens: f.tokens }, c.doc[0]);
      ok = c.doc[0] === 'components' ? same([...(v as string[])].sort(), [...(c.doc[1] as string[])].sort()) : same(v, c.doc[1]);
      rec = { test: c.test, document: c.doc[0], equals: c.doc[1] };
    } else {
      const n = find(c.node);
      if ('equals' in c) { ok = same(get(n, c.equals[0]), c.equals[1]); rec = { test: c.test, node: c.node, field: c.equals[0], equals: c.equals[1] }; }
      else if ('subset' in c) { ok = subset(get(n, c.subset[0]), c.subset[1]); rec = { test: c.test, node: c.node, field: c.subset[0] || '(node)', includes: c.subset[1] }; }
      else if ('absent' in c) { ok = n !== undefined && get(n, c.absent) === undefined; rec = { test: c.test, node: c.node, absent: c.absent }; }
      else { ok = new RegExp(c.matches[1]).test(String(get(n, c.matches[0]))); rec = { test: c.test, node: c.node, field: c.matches[0], matches: c.matches[1] }; }
    }
    if (!ok) failures.push(`${short}: ${JSON.stringify(rec)}`);
    recorded.push(rec);
  }
  if (short === 'lines.fig') {
    const by = (n: string) => f.nodes.find((x: any) => x.name === n);
    const d = lineEnds(by('Diagonal'));
    const u = lineEnds(by('Upright'));
    const exp = [[40, 140], [40 + 200 * Math.cos(Math.PI / 6), 140 + 200 * Math.sin(Math.PI / 6)]];
    const close = d.every((p, i) => p.every((v, k) => Math.abs(v - exp[i]![k]!) < 0.05));
    if (!close) failures.push(`lines.fig Diagonal ends ${JSON.stringify(d)}`);
    if (!(u[0]![0] === 400 && u[0]![1] === 20 && u[1]![1] === 120)) failures.push(`lines.fig Upright ends ${JSON.stringify(u)}`);
    recorded.push({ test: 'rotated 30°: end points (layout.left/top + x1,y1 / x2,y2) where Figma draws them, within 0.05 px', node: 'Diagonal', endPoints: [[40, 140], [Math.round(exp[1]![0]! * 1000) / 1000, Math.round(exp[1]![1]! * 1000) / 1000]], tolerancePx: 0.05 });
    recorded.push({ test: '90°, 4 px: centred on x = 400, from y 20 to 120 (exact)', node: 'Upright', endPoints: u });
  }
  if (short === 'stage-a.fig') recorded.push({ test: 'nothing Figma-specific reaches the document or its exports', documentJsonMustNotMatch: '/sessionID|localID|guid|symbolData|figma|fig-kiwi|"1:\\d+"/i', exportsMustNotMatch: '/sessionID|guid|figma|data-fig/i' });
  f.asserts = recorded;
}
if (failures.length) {
  console.error('ASSERTS FAILED:\n' + failures.join('\n'));
  process.exit(1);
}

const head = execSync('git rev-parse --short HEAD', { cwd: REPO }).toString().trim();
const LIM = sec.limits;
const final = {
  $about: [
    "Scrim's .fig importer, run on its own fixtures and on hostile inputs: the expectations a Rust port must meet.",
    'Every value here was produced by RUNNING packages/fig-import/dist (Scrim ' + head + ') from scripts outside the repo; nothing is hand-typed except the asserts, and those were checked against the run.',
    'Spec: docs/transfer/specs/fig-import.md (+ -security.md, -mapping.md). Fixture inventory: docs/transfer/fixtures.md.',
  ],
  generatedBy: {
    scrimCommit: head,
    node: process.version,
    scripts: ['gen-fixtures.ts', 'gen-security.ts', 'merge.ts'].map((s) => `./${s}`),
    builders: 'the fixture builders and test encoder are packages/fig-import/src/__tests__/fixtures.ts and figEncoder.ts, copied verbatim (one import path changed); they reproduce tests/fixtures/fig/*.fig byte for byte',
  },
  conventions: {
    nodeAddressing: "Nodes have no ids here (Scrim's are random). `path` is the tree path: '<rootIndex>:<name>' then '/<childIndex>:<name>' per level. `node` selectors in asserts are a layer name (first match in tree order) or { name, under } (the first match inside the named ancestor).",
    units: 'Numbers in style/layout are CSS px unless the value is a string with its own unit. Colours are #rrggbb when opaque, else rgba(r, g, b, a) with a rounded to 3 decimals.',
    figmaBox: "Each node's box in Figma, relative to its top-level artboard (its transform composed through its ancestors; rotated layers as their axis-aligned bounds). Measurement only: never in the document. Instance content has no figmaBox (only the instance root).",
    patchOps: 'Scrim op names; a port need not reproduce them. One import = ONE patch = one undo step.',
    imagePipeline: "importFig takes the host's image pipeline. stage-a and stage-b ran with one that accepts every image (src = scrim-asset:<sha256>.png); the others with one that refuses (they carry no image bytes).",
  },
  limits: { defaults: LIM, internal: sec.internalCaps },
  zipCrc: {
    fact: 'The test encoder (figEncoder.ts › zip) writes CRC-32 = 0 in every local header and central-directory entry. The one committed zipped fixture is tests/fixtures/fig/stage-a.fig; every ZIP-based security case below has CRC 0 too.',
    scrimReader: "Scrim's reader (container.ts › readArchive) never reads either CRC field: CRC 0, or any wrong CRC, is accepted. It also ignores the local header's sizes and name (the central directory's are used), and data-descriptor flag bit 3 needs no handling for the same reason. Integrity rests on: the declared uncompressed size must equal the inflated length, the image entries must hash to their SHA-1 name, and the Kiwi decode must consume the data exactly.",
    figcraft: 'FigCraft refuses these zips ("Invalid checksum"; docs/research/figcraft-spike.md §3). A port that verifies CRCs must either accept CRC 0 for the fixtures or re-zip them with real CRCs; crc32Actual below is the value a correct writer would store.',
    stageAEntries: fx.fixtures.find((f: any) => f.container === 'zip')?.zipEntries,
  },
  fixtures: fx.fixtures,
  variants: fx.variants,
  security: sec.security,
  imageReportCases: sec.reportCases,
  hostLevelRefusals: [
    { name: 'Home drop over 50 MB (server)', where: 'packages/mcp/src/library/messages.ts › parseLibraryMessage (library:importFig)', check: 'floor(base64.length × 3 / 4) > 50 MiB (MAX_IMPORT_BYTES, library.ts)', expectCode: 'too_large', expectError: '.fig files dropped on Home are limited to 50 MB: use Import for larger ones', fromSource: true },
    { name: 'Home drop over 50 MB (studio, before sending)', where: 'apps/studio/src/home/useHomeActions.ts › importFile', check: 'file.size > 50 MiB', expectError: '“<file name>” is larger than 50 MB.', fromSource: true },
    { name: 'Home drop without a usable name', where: 'messages.ts', check: 'name not a string, empty, or over 255 characters', expectCode: 'invalid_name', expectError: 'a .fig import needs the file name', fromSource: true },
    { name: 'Home drop whose data is not base64', where: 'messages.ts', check: 'data not a string or not /^[A-Za-z0-9+/]*={0,2}$/', expectCode: 'invalid_document', expectError: 'a .fig import needs the file bytes (base64)', fromSource: true },
    { name: 'CLI import-fig over 200 MB', where: 'packages/cli/src/open.ts › readFigFile (stat before reading; exit 3)', check: 'stat size > 200 MiB (MAX_SCRIM_FILE_BYTES)', expectCode: 'too_large', expectError: '“<basename>” is <size / 1048576, toFixed(0)> MB: Scrim imports .fig files up to 200 MB', fromSource: true },
    { name: 'Desktop File › Import over 200 MB', where: 'apps/desktop/src/main/documents.ts › openPath (stat before reading)', check: 'stat size > 200 MiB', expectError: '“<basename>” is <size / 1048576, toFixed(0)> MB: Scrim opens .scrim files up to 200 MB', note: 'says ".scrim files" for a .fig (scrimTooLarge is shared): reported as a wording bug, not something to copy', fromSource: true },
    { name: 'An embedded image over 25 MB', where: 'packages/mcp/src/assets/sniff.ts (the image pipeline; reported per image, never thrown)', check: 'bytes > 25 MiB (MAX_IMAGE_BYTES)', expectReportReason: 'Images are limited to 25 MB (this one is <n> MB)', fromSource: true },
    { name: 'An embedded image over 16384 px a side or 100 megapixels', where: 'sniff.ts / importer.ts', check: 'header (or decoded) size', expectReportReason: 'The image is <w> × <h> px: the limit is 16384 px a side and 100 megapixels', fromSource: true },
    { name: 'An embedded image whose decoded size differs from its header', where: 'importer.ts › decodeChecker', expectReportReason: 'This <TYPE> says it is <w> × <h> px but decodes to <W> × <H>: refused as a possible decompression bomb', fromSource: true },
    { name: 'An embedded file that is not a supported image', where: 'sniff.ts', expectReportReason: 'Not a supported image: Scrim imports PNG, JPEG, WebP, GIF, AVIF and SVG', fromSource: true },
  ],
  counts: {
    fixtures: fx.fixtures.length,
    fixtureAsserts: fx.fixtures.reduce((s: number, f: any) => s + f.asserts.length, 0),
    securityCases: sec.security.length,
    securityCasesWithFigError: sec.security.filter((c: any) => c.expectError !== undefined).length,
    securityCasesInScrimTests: sec.security.filter((c: any) => c.scrimTest).length,
    securityCasesEmbeddedBase64: sec.security.filter((c: any) => c.inputBase64).length,
    imageReportCases: sec.reportCases.length,
    hostLevelRefusals: 10,
  },
};
mkdirSync(`${REPO}/docs/transfer/cases`, { recursive: true });
writeFileSync(OUT, JSON.stringify(final, null, 1) + '\n');
console.log(JSON.stringify(final.counts));
