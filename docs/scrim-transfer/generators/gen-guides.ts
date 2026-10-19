// Generates docs/transfer/cases/guides.json by RUNNING Scrim's code (bun).
const R = (process.env.SCRIM_REPO ?? new URL('../../../', import.meta.url).pathname.replace(/\/$/, ''));
const OUT = process.argv[2] ?? `${R}/docs/transfer/cases/guides.json`;
const core = await import(`${R}/packages/core/src/index.ts`);
const eng = await import(`${R}/packages/engine/src/layoutGuides.ts`);
const ge = await import(`${R}/apps/studio/src/editor/guideEdits.ts`);
const { applyPatch, addNode, createDocument, createNode, updateNode, guideIssues, PatchError } = core;

const look = { color: '#003880', opacity: 0.1, visible: true };
const col = (o: Record<string, unknown> = {}) => ({ type: 'column', align: 'stretch', size: 40, count: 12, margin: 0, gutter: 8, ...look, ...o });
const row = (o: Record<string, unknown> = {}) => ({ type: 'row', align: 'stretch', size: 40, count: 12, margin: 0, gutter: 8, ...look, ...o });
const grid = (o: Record<string, unknown> = {}) => ({ type: 'grid', size: 50, ...look, ...o });

const r3 = (v: number): number => Math.round(v * 1e6) / 1e6;
const rr = (r: { x: number; y: number; width: number; height: number }) => ({ x: r3(r.x), y: r3(r.y), width: r3(r.width), height: r3(r.height) });

interface GeoCase { name: string; frame: { x: number; y: number; width: number; height: number }; guides: Record<string, unknown>[]; note?: string; [k: string]: unknown }

const F = { x: 0, y: 0, width: 1440, height: 1024 };
const geometry: GeoCase[] = [
  { name: 'Paper default column guide (+) on a 1440 × 1024 frame', frame: F, guides: [col()] },
  { name: 'Paper default row guide on a 1440 × 1024 frame', frame: F, guides: [row()] },
  { name: 'Paper default grid guide (50) on a 1440 × 1024 frame', frame: F, guides: [grid()] },
  { name: 'stretch with a margin: (1000 − 2·20 − 11·8) / 12; size ignored', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ margin: 20, size: 999 })] },
  { name: 'start (Left): first column at the margin, each size px', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ align: 'start', count: 4, margin: 24 })] },
  { name: 'end (Right): last column ends at width − margin', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ align: 'end', count: 4, margin: 24 })] },
  { name: 'center: the block is centred; margin ignored', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ align: 'center', count: 4, margin: 500 })] },
  { name: 'center with an odd remainder (fractional start)', frame: { x: 0, y: 0, width: 1001, height: 600 }, guides: [col({ align: 'center', count: 3, size: 100, gutter: 10 })] },
  { name: 'frame not at the origin: bands are in world coordinates', frame: { x: 100, y: 50, width: 1000, height: 600 }, guides: [col({ align: 'start', count: 2, margin: 10 })] },
  { name: 'row start on an offset frame: full-width bands', frame: { x: 100, y: 50, width: 1000, height: 600 }, guides: [row({ align: 'start', count: 1 })] },
  { name: 'row end (Bottom) with margin and gutter', frame: { x: 0, y: 0, width: 400, height: 800 }, guides: [row({ align: 'end', count: 3, size: 100, margin: 16, gutter: 24 })] },
  { name: 'row center', frame: { x: 0, y: 0, width: 400, height: 800 }, guides: [row({ align: 'center', count: 2, size: 120, gutter: 40 })] },
  { name: 'gutter 0: band edges become lines (no bands)', frame: { x: 100, y: 50, width: 1000, height: 600 }, guides: [col({ count: 4, gutter: 0 })] },
  { name: 'gutter 0 rows, start aligned', frame: { x: 0, y: 0, width: 300, height: 300 }, guides: [row({ align: 'start', count: 3, size: 50, gutter: 0 })] },
  { name: 'count 1 stretch with margins', frame: { x: 0, y: 0, width: 360, height: 640 }, guides: [col({ count: 1, margin: 16 })] },
  { name: 'stretch with no room left: no bands, no lines', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ margin: 600 })] },
  { name: 'stretch exactly filled by margins and gutters: size 0, nothing drawn', frame: { x: 0, y: 0, width: 100, height: 100 }, guides: [col({ count: 3, margin: 10, gutter: 40 })] },
  { name: 'fixed columns past the frame: bands run outside (drawn clipped), snap lines outside are dropped', frame: { x: 100, y: 50, width: 1000, height: 600 }, guides: [col({ align: 'start', count: 12, size: 100, gutter: 10 })] },
  { name: 'end alignment wider than the frame: start goes negative', frame: { x: 0, y: 0, width: 200, height: 200 }, guides: [col({ align: 'end', count: 3, size: 100, gutter: 10, margin: 0 })] },
  { name: 'square grid: interior lines only (frame edges not repeated)', frame: { x: 0, y: 0, width: 160, height: 100 }, guides: [grid()] },
  { name: 'square grid on an offset frame', frame: { x: 100, y: 50, width: 160, height: 100 }, guides: [grid()] },
  { name: 'square grid whose size divides the frame exactly (no line on the far edge)', frame: { x: 0, y: 0, width: 150, height: 100 }, guides: [grid()] },
  { name: 'square grid bigger than the frame: nothing', frame: { x: 0, y: 0, width: 40, height: 40 }, guides: [grid()] },
  { name: '8 px grid on a 64 × 24 frame', frame: { x: 0, y: 0, width: 64, height: 24 }, guides: [grid({ size: 8 })] },
  { name: 'fractional frame and stretch columns', frame: { x: 10.5, y: 0, width: 375.5, height: 812 }, guides: [col({ count: 4, margin: 16, gutter: 16 })] },
  { name: 'a hidden guide still has a shape, but no snap lines and is not drawn', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ count: 4, visible: false })] },
  { name: 'zero opacity: snaps (visible) but is not drawn', frame: { x: 0, y: 0, width: 1000, height: 600 }, guides: [col({ count: 2, opacity: 0, align: 'start', size: 100 })] },
  { name: 'several guides on one frame: snap lines are the de-duplicated union (columns, rows and a grid)', frame: { x: 0, y: 0, width: 200, height: 100 }, guides: [col({ align: 'start', count: 2, size: 50, gutter: 0 }), grid({ size: 50 }), row({ align: 'start', count: 1, size: 50, gutter: 10 })] },
  { name: 'Paper defaults stacked (12 columns, 12 rows, a 50 px grid) on a mobile frame', frame: { x: 0, y: 0, width: 390, height: 844 }, guides: [col(), row(), grid()] },
];

for (const c of geometry) {
  c.perGuide = c.guides.map((g) => {
    const shape = eng.guideShape(g, c.frame);
    const out: Record<string, unknown> = { bands: shape.bands.map(rr), xs: shape.xs.map(r3), ys: shape.ys.map(r3) };
    if (g.type !== 'grid') {
      const t = eng.trackLayout(g, g.type === 'column' ? c.frame.width : c.frame.height);
      out.track = { start: r3(t.start), size: r3(t.size), step: r3(t.step) };
    }
    return out;
  });
  c.snapTargets = eng.guideSnapTargets(c.guides, c.frame).map(rr);
  // the draw at zoom 1, DPR 1, camera at the origin, clip = frame
  const rects: number[][] = [];
  const ctx = { globalAlpha: 1, fillStyle: '#000', strokeStyle: '#000', lineWidth: 1, clearRect() {}, strokeRect() {}, fillRect(x: number, y: number, w: number, h: number) { rects.push([r3(x), r3(y), r3(w), r3(h), ctx.globalAlpha, ctx.fillStyle as unknown as number]); } };
  eng.drawLayoutGuides(ctx, { x: 0, y: 0, zoom: 1 }, [{ rect: c.frame, clip: c.frame, guides: c.guides }], 1);
  c.drawnAtZoom1 = rects.length;
}

// a draw case at a zoom where the grid is too small
const smallGrid: number[][] = [];
{
  const ctx = { globalAlpha: 1, fillStyle: '#000', strokeStyle: '#000', lineWidth: 1, clearRect() {}, strokeRect() {}, fillRect(x: number, y: number, w: number, h: number) { smallGrid.push([x, y, w, h]); } };
  eng.drawLayoutGuides(ctx, { x: 0, y: 0, zoom: 0.07 }, [{ rect: F, clip: F, guides: [grid()] }], 1);
}
const drawRules = { MIN_GRID_CELL_PX: eng.MIN_GRID_CELL_PX, 'grid size 50 at zoom 0.07 (3.5 screen px cells): rects drawn': smallGrid.length };

// ── validation (core guides.ts via applyPatch) ───────────────────────────────
function doc0() {
  return applyPatch(createDocument({ id: 'd' }), [
    addNode(createNode('frame', { id: 'f', name: 'Frame', canvas: { x: 0, y: 0 } }), null, 0),
    addNode(createNode('box', { id: 'b', name: 'Box' }), 'f', 0),
    addNode(createNode('text', { id: 't', name: 'Label', text: 'Hi' }), 'f', 1),
  ]).doc;
}
interface ValCase { name: string; nodeId?: string; guides: unknown; expect?: unknown; issues?: string[] }
const validation: ValCase[] = [
  { name: 'the Paper defaults, one of each type', guides: [col(), row(), grid()] },
  { name: 'an empty list (stored as absent)', guides: [] },
  { name: 'null (clears the field)', guides: null },
  { name: 'boundary values: size 1, count 100, margin and gutter 0 and 100000, opacity 0 and 1', guides: [col({ size: 1, count: 100, margin: 0, gutter: 100000, opacity: 0 }), row({ size: 100000, count: 1, margin: 100000, gutter: 0, opacity: 1 })] },
  { name: 'a fractional size, margin and gutter are allowed', guides: [col({ size: 40.5, margin: 12.25, gutter: 7.75 })] },
  { name: 'upper-case hex is allowed', guides: [grid({ color: '#AA0033' })] },
  { name: '20 guides (the cap)', guides: Array.from({ length: 20 }, () => grid()) },
  { name: 'on a box', nodeId: 'b', guides: [col()] },
  { name: 'on a text node', nodeId: 't', guides: [grid()] },
  { name: 'not a list', guides: { type: 'grid' } },
  { name: '21 guides', guides: Array.from({ length: 21 }, () => grid()) },
  { name: 'a guide that is not an object', guides: ['grid'] },
  { name: 'an unknown type', guides: [grid({ type: 'ruler' })] },
  { name: 'Figma spelling of the type (COLUMNS)', guides: [col({ type: 'COLUMNS' })] },
  { name: 'a grid with a count', guides: [grid({ count: 3 })] },
  { name: 'a column with an offset (Scrim has no offset)', guides: [col({ offset: 10 })] },
  { name: 'a 3-digit colour', guides: [grid({ color: '#038' })] },
  { name: 'an 8-digit colour with alpha', guides: [grid({ color: '#00388019' })] },
  { name: 'an rgba() colour', guides: [grid({ color: 'rgba(0,56,128,0.1)' })] },
  { name: 'opacity over 1 (a percent typed as a number)', guides: [grid({ opacity: 10 })] },
  { name: 'visible as a string', guides: [grid({ visible: 'true' })] },
  { name: 'a missing visible', guides: [{ type: 'grid', size: 50, color: '#003880', opacity: 0.1 }] },
  { name: 'a zero grid size', guides: [grid({ size: 0 })] },
  { name: 'a size over 100000', guides: [grid({ size: 100001 })] },
  { name: 'a fractional count', guides: [col({ count: 2.5 })] },
  { name: 'count 0', guides: [col({ count: 0 })] },
  { name: '101 columns', guides: [col({ count: 101 })] },
  { name: 'a negative gutter', guides: [col({ gutter: -1 })] },
  { name: 'a margin over 100000', guides: [row({ margin: 100001 })] },
  { name: 'alignment left (the UI word, not the stored value)', guides: [col({ align: 'left' })] },
  { name: 'Figma alignment MIN', guides: [col({ align: 'MIN' })] },
  { name: 'a column guide missing align, count, margin and gutter', guides: [{ type: 'column', size: 40, ...look }] },
  { name: 'a grid guide (bare object)', guides: [{ type: 'grid' }] },
];
for (const c of validation) {
  const id = c.nodeId ?? 'f';
  const node = doc0().nodes[id];
  c.issues = core.guidesIssues({ type: node.type, guides: c.guides === null || (Array.isArray(c.guides) && c.guides.length === 0) ? undefined : c.guides });
  try {
    const d = applyPatch(doc0(), updateNode(id, { guides: c.guides }));
    c.expect = 'accept';
    (c as Record<string, unknown>).stored = d.doc.nodes[id].guides ?? '(absent)';
    if (JSON.stringify((c as Record<string, unknown>).stored) === JSON.stringify(c.guides)) delete (c as Record<string, unknown>).stored;
  } catch (e) {
    if (!(e instanceof PatchError)) throw e;
    c.expect = { refuse: (e as Error).message };
  }
  if (c.issues.length === 0) delete c.issues;
}
void guideIssues;

// ── the inspector model (studio editor/guideEdits.ts) ───────────────────────
const typeSwitch = [
  { from: col({ align: 'end', size: 64, count: 6, margin: 24, gutter: 20, color: '#aa0033', opacity: 0.25, visible: false }), to: 'row' },
  { from: col({ align: 'end', size: 64, count: 6, margin: 24, gutter: 20, color: '#aa0033', opacity: 0.25, visible: false }), to: 'grid' },
  { from: grid({ size: 24, color: '#aa0033', opacity: 0.3 }), to: 'column' },
  { from: grid({ size: 24 }), to: 'row' },
  { from: row({ count: 3 }), to: 'row' },
].map((c) => ({ ...c, result: ge.switchType(c.from, c.to) }));

const numberInputs = ([
  ['opacity', '10'], ['opacity', '10%'], ['opacity', '12.34'], ['opacity', '101'], ['opacity', '-1'],
  ['count', '12'], ['count', '2.5'], ['count', '0'], ['count', '101'],
  ['size', '0'], ['size', '1'], ['size', '40.555'], ['size', '40px'], ['size', '100001'],
  ['margin', '0'], ['margin', '-4'], ['gutter', '8'], ['gutter', 'abc'], ['gutter', ' 16 '], ['gutter', '1e3'],
] as const).map(([field, raw]) => ({ field, raw, value: ge.parseGuideNumber(field, raw) }));

const colorInputs = ['#AA0033', 'aa0033', '#a03', 'rgba(170, 0, 51, 0.5)', 'red', 'nonsense', 'hsl(0 100% 50%)'].map((raw) => ({ raw, result: ge.parseGuideColor(raw, {}) }));

const shown = [col({ opacity: 0.1 }), col({ opacity: 0.125 }), grid({ opacity: 1 })].map((g) => ({ guide: g, opacity: ge.shownGuideNumber(g, 'opacity'), size: ge.shownGuideNumber(g, 'size') }));

// the patches the section writes
const twoFrames = applyPatch(doc0(), [addNode(createNode('frame', { id: 'g', name: 'Other', canvas: { x: 2000, y: 0 } }), null, 1), updateNode('g', { guides: [grid()] })]).doc;
const patches = {
  'addGuidePatch on frame f with no guides': ge.addGuidePatch(doc0(), ['f']),
  'addGuidePatch on frames f (none) and g ([grid]): each keeps its own list': ge.addGuidePatch(twoFrames, ['f', 'g']),
  'readGuides on f and g (lists differ: Mixed)': ge.readGuides(twoFrames, ['f', 'g']),
  'readGuides on box b (section does not apply)': ge.readGuides(doc0(), ['b']),
  'guidesPatch writing [] to g (clears: guides null)': ge.guidesPatch(twoFrames, ['g'], []),
  'guidesPatch writing g its own list again (no change: null patch)': ge.guidesPatch(twoFrames, ['g'], [grid()]),
  'guideFieldPatch count "6" on g\'s grid (a grid has no count: null)': ge.guideFieldPatch(twoFrames, ['g'], 0, 'count', '6'),
  'guideFieldPatch size "24" on g\'s grid': ge.guideFieldPatch(twoFrames, ['g'], 0, 'size', '24'),
};

const doc = {
  about: 'Scrim layout guide cases (docs/transfer/specs/layout-guides.md). Every expectation was produced by running Scrim\'s own code.',
  generatedAt: new Date().toISOString().slice(0, 10),
  scrimCommit: 'f2b1bdb',
  sources: {
    geometry: 'packages/engine/src/layoutGuides.ts: trackLayout(guide, length), guideShape(guide, frame), guideSnapTargets(guides, frame), drawLayoutGuides (count of fillRect calls at zoom 1, DPR 1, clip = frame)',
    validation: 'packages/core/src/guides.ts guidesIssues, and applyPatch(updateNode(id, { guides })) on a frame "Frame" (id f), a box "Box" (id b) and a text "Label" (id t)',
    inspector: 'apps/studio/src/editor/guideEdits.ts: switchType, parseGuideNumber, parseGuideColor, shownGuideNumber, addGuidePatch, readGuides, guidesPatch, guideFieldPatch',
  },
  conventions: {
    frame: 'the frame\'s measured border box in world px { x, y, width, height }',
    perGuide: 'one entry per guide, in list order: track = trackLayout along the guide\'s axis (columns: width, rows: height) — start is relative to the frame\'s edge; bands / xs / ys = guideShape, in world px (xs are constant-x lines, ys constant-y lines)',
    snapTargets: 'guideSnapTargets: zero-width rects (x lines, full frame height) then zero-height rects (y lines, full frame width), each de-duplicated in first-seen order; only visible guides; only lines inside the frame (±1e-6)',
    drawnAtZoom1: 'how many rectangles drawLayoutGuides fills for this frame at zoom 1, DPR 1, camera at the origin, clip = frame',
    numbers: 'rounded to 6 decimals in this file',
    expect: 'validation: "accept" | { refuse: "<verbatim PatchError message>" }; stored = what the document holds afterwards when it differs from the input',
  },
  drawRules,
  geometry,
  validation,
  inspector: { defaults: ge.GUIDE_DEFAULTS, alignWords: ge.ALIGN_WORDS, readOnlyReason: ge.READ_ONLY_REASON, typeSwitch, numberInputs, colorInputs, shown, patches },
};
await Bun.write(OUT, JSON.stringify(doc, null, 1) + '\n');
console.log('geometry', geometry.length, 'validation', validation.length, 'typeSwitch', typeSwitch.length, 'numberInputs', numberInputs.length, 'colorInputs', colorInputs.length);
