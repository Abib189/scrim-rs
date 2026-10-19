// Generates docs/transfer/cases/svg.json by RUNNING Scrim's code (bun).
// Every expectation below comes from the functions' actual results.
const R = (process.env.SCRIM_REPO ?? new URL('../../../', import.meta.url).pathname.replace(/\/$/, ''));
const OUT = process.argv[2] ?? `${R}/docs/transfer/cases/svg.json`;
const core = await import(`${R}/packages/core/src/index.ts`);
const sniff = await import(`${R}/packages/mcp/src/assets/sniff.ts`);
const { applyPatch, addNode, createDocument, createNode, svgContentIssues, svgMarkup, svgTree, parsePath, serializePath, vectorEditRefusal, PatchError } = core;

type Gen =
  | { kind: 'repeat-string'; target: string; prefix: string; unit: string; count: number; suffix: string }
  | { kind: 'repeat-element'; target: string; element: unknown; count: number; elementString?: { target: string; prefix: string; unit: string; count: number; suffix: string } }
  | { kind: 'nest'; target: string; open: string; close: string; count: number; inner: string; prefix: string; suffix: string }
  | { kind: 'nest-g'; target: string; depth: number; leaf: unknown };

interface Case {
  name: string;
  layer: 'inline' | 'file' | 'pathEdit' | 'editRefusal';
  note?: string;
  nodeType?: string;
  svg: unknown;
  generate?: Gen;
  expect?: unknown;
  [k: string]: unknown;
}

// ── materialise: the generator conventions documented in the JSON header ─────
function setAt(root: unknown, path: string, value: unknown): unknown {
  if (path === '') return value;
  const keys = path.split('.');
  let o = root as Record<string, unknown>;
  for (const k of keys.slice(0, -1)) o = o[k] as Record<string, unknown>;
  o[keys.at(-1)!] = value;
  return root;
}
function materialise(c: Case): unknown {
  const svg = structuredClone(c.svg);
  const g = c.generate;
  if (!g) return svg;
  if (g.kind === 'repeat-string') return setAt(svg, g.target, g.prefix + g.unit.repeat(g.count) + g.suffix);
  if (g.kind === 'nest') return setAt(svg, g.target, g.prefix + g.open.repeat(g.count) + g.inner + g.close.repeat(g.count) + g.suffix);
  if (g.kind === 'repeat-element') {
    const one = (): unknown => {
      const e = structuredClone(g.element);
      const s = g.elementString;
      return s ? setAt(e, s.target, s.prefix + s.unit.repeat(s.count) + s.suffix) : e;
    };
    return setAt(svg, g.target, Array.from({ length: g.count }, one));
  }
  if (g.kind === 'nest-g') {
    let list: unknown[] = [structuredClone(g.leaf)];
    for (let i = 0; i < g.depth; i++) list = [{ tag: 'g', children: list }];
    return setAt(svg, g.target, list);
  }
  throw new Error('unknown generator');
}

// ── runners ─────────────────────────────────────────────────────────────────
function runInline(c: Case): void {
  const svg = materialise(c);
  const type = c.nodeType ?? 'box';
  const issues = type === 'box' || type === 'instance' ? svgContentIssues(svg) : null;
  try {
    applyPatch(createDocument({ id: 'd', name: 'Svg' }), addNode({ ...createNode(type, { id: 'v', name: 'Icon' }), svg }, null, 0));
    const markup = svgMarkup(svgTree(svg, 'svg-v'));
    if (c.expectNormalised) {
      c.expect = { normalised: markup };
      delete c.expectNormalised;
    } else {
      c.expect = 'accept';
      if (JSON.stringify(svg).length < 4000) c.markup = markup;
    }
  } catch (e) {
    if (!(e instanceof PatchError)) throw e;
    c.expect = { refuse: (e as Error).message };
    c.issues = issues ?? [(e as Error).message.replace(/^Icon: /, '')];
    if ((issues?.length ?? 0) > 20) c.issues = [...issues!.slice(0, 20), `… ${issues!.length - 20} more`];
  }
  delete c.expectNormalised;
}

function runFile(c: Case): void {
  const text = materialise(c) as string | { bytes: number[] };
  const bytes = typeof text === 'string' ? new TextEncoder().encode(text) : new Uint8Array(text.bytes);
  c.bytes = bytes.length;
  try {
    const s = sniff.sniffImage(bytes);
    c.expect = 'accept';
    c.sniffed = s;
  } catch (e) {
    if (!(e instanceof sniff.AssetError)) throw e;
    c.expect = { refuse: (e as Error).message };
    c.code = (e as { code: string }).code;
  }
  if (typeof text === 'string') c.svgRefusal = sniff.svgRefusal(text);
}

function runPath(c: Case): void {
  const d = c.svg as string;
  const p = parsePath(d);
  const write = svgContentIssues({ viewBox: [0, 0, 10, 10], children: [{ tag: 'path', d }] });
  c.writeIssues = write;
  if (!p) {
    c.expect = { refuse: 'parsePath: null' };
    return;
  }
  c.expect = { normalised: serializePath(p.segs) };
  c.arcs = p.arcs;
  c.segments = p.segs.length;
}

function runEdit(c: Case): void {
  const msg = vectorEditRefusal(c.svg);
  c.expect = msg === null ? 'accept' : { refuse: msg };
}

// ── the cases ───────────────────────────────────────────────────────────────
const ICON = {
  viewBox: [0, 0, 24, 24],
  defs: [{ tag: 'linearGradient', id: 'g', x1: 0, y1: 0, x2: 24, y2: 0, stops: [{ offset: 0, color: '#ff0000' }, { offset: 1, color: 'rgba(0, 0, 255, 0.5)', opacity: 0.8 }] }],
  children: [
    { tag: 'path', d: 'M12 0C18.6 0 24 5.4 24 12S18.6 24 12 24 0 18.6 0 12 5.4 0 12 0Z', fill: 'url(#g)', fillRule: 'evenodd' },
    { tag: 'g', opacity: 0.5, transform: [1, 0, 0, 1, 2, 2], children: [{ tag: 'rect', x: 0, y: 0, width: 4, height: 4, rx: 1, fill: '#00ff00' }, { tag: 'circle', cx: 2, cy: 2, r: 1, stroke: '#000', strokeWidth: 0.5, fill: 'none' }] },
  ],
};
const P = (extra: Record<string, unknown>) => ({ viewBox: [0, 0, 24, 24], children: [{ tag: 'path', d: 'M0 0L24 24', stroke: '#000000', ...extra }] });
const withChild = (child: unknown) => ({ viewBox: [0, 0, 24, 24], children: [child] });
const withFill = (fill: unknown) => withChild({ tag: 'rect', x: 0, y: 0, width: 24, height: 24, fill });
const grad = { tag: 'linearGradient', id: 'g', x1: 0, y1: 0, x2: 1, y2: 0, stops: [{ offset: 0, color: '#000000' }] };
const withDefs = (defs: unknown[], child: unknown = { tag: 'rect', x: 0, y: 0, width: 24, height: 24, fill: '#000000' }) => ({ viewBox: [0, 0, 24, 24], defs, children: [child] });
const NS = 'xmlns="http://www.w3.org/2000/svg"';
const NSX = 'xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"';
const path200k = (n: number) => ({ kind: 'repeat-string' as const, target: 'children.0.d', prefix: 'M0 0', unit: 'L1 1', count: n, suffix: '' });

const cases: Case[] = [
  // ── INLINE: benign ─────────────────────────────────────────────────────────
  { name: 'inline: gradient icon with a group, transform and evenodd fill (core test fixture)', layer: 'inline', svg: ICON },
  { name: 'inline: Lucide "circle-check" as structured data (stroke currentColor, relative m)', layer: 'inline', svg: { viewBox: [0, 0, 24, 24], children: [{ tag: 'g', fill: 'none', stroke: 'currentColor', strokeWidth: 2, strokeLinecap: 'round', strokeLinejoin: 'round', children: [{ tag: 'circle', cx: 12, cy: 12, r: 10 }, { tag: 'path', d: 'm9 12 2 2 4-4' }] }] } },
  { name: 'inline: Heroicons solid "check-circle" (compact arc flags, evenodd)', layer: 'inline', svg: { viewBox: [0, 0, 24, 24], children: [{ tag: 'path', fill: 'currentColor', fillRule: 'evenodd', d: 'M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75 9.75S2.25 17.385 2.25 12Zm13.36-1.814a.75.75 0 1 0-1.22-.872l-3.236 4.53L9.53 12.22a.75.75 0 0 0-1.06 1.06l2.25 2.25a.75.75 0 0 0 1.14-.094l3.75-5.25Z' }] } },
  { name: 'inline: a Line-tool line (vectorEffect non-scaling-stroke, dashed)', layer: 'inline', svg: { viewBox: [0, 0, 121, 1], children: [{ tag: 'line', x1: 0.5, y1: 0.5, x2: 120.5, y2: 0.5, stroke: '#000000', strokeWidth: 1, fill: 'none', vectorEffect: 'non-scaling-stroke', strokeDasharray: [4, 2] }] } },
  { name: 'inline: clipPath, mask and a radial gradient with focus, units and transform', layer: 'inline', svg: { viewBox: [0, 0, 100, 100], defs: [{ tag: 'radialGradient', id: 'r', cx: 0.5, cy: 0.5, r: 0.5, fx: 0.3, fy: 0.3, units: 'objectBoundingBox', transform: [1, 0, 0, 1, 0, 0], stops: [{ offset: 0, color: '#ffffff' }, { offset: 1, color: '#000000', opacity: 0 }] }, { tag: 'clipPath', id: 'c', children: [{ tag: 'circle', cx: 50, cy: 50, r: 40 }] }, { tag: 'mask', id: 'm', children: [{ tag: 'rect', x: 0, y: 0, width: 100, height: 100, fill: '#ffffff' }] }], children: [{ tag: 'rect', x: 0, y: 0, width: 100, height: 100, fill: 'url(#r)', clipPath: 'c', mask: 'm' }] } },
  { name: 'inline: every colour form the allow-list takes (#rgb, #rgba, #rrggbb, #rrggbbaa, rgb(), rgba(), none, currentColor, transparent)', layer: 'inline', svg: { viewBox: [0, 0, 9, 1], children: ['#f00', '#f008', '#ff0000', '#ff000080', 'rgb(255, 0, 0)', 'rgba(255,0,0,.5)', 'none', 'currentColor', 'transparent'].map((fill, i) => ({ tag: 'rect', x: i, y: 0, width: 1, height: 1, fill })) } },
  { name: 'inline: a number exactly at the cap (10^7) is accepted', layer: 'inline', svg: withChild({ tag: 'rect', x: 0, y: 0, width: 10_000_000, height: 1 }) },
  { name: 'inline: a negative radius is accepted (only finiteness and ±10^7 are checked)', layer: 'inline', note: 'Behaviour, not a recommendation: the checks never test geometry signs except strokeWidth and viewBox size.', svg: withChild({ tag: 'circle', cx: 0, cy: 0, r: -5 }) },
  { name: 'inline: path data the write check accepts but the editor parser rejects (odd argument count)', layer: 'inline', note: 'Write-time path validation is a character class plus number-shape regexes, not a parser; see the pathEdit case with the same d.', svg: withChild({ tag: 'path', d: 'M0 0 1' }) },
  { name: 'inline: a path exactly 200000 characters long is accepted', layer: 'inline', svg: withChild({ tag: 'path', d: '' }), generate: path200k(49_999) },
  { name: 'inline: 5 paths of 200000 characters (1000000 in total, the cap) are accepted', layer: 'inline', svg: { viewBox: [0, 0, 1, 1], children: [] }, generate: { kind: 'repeat-element', target: 'children', count: 5, element: { tag: 'path', d: '' }, elementString: { target: 'd', prefix: 'M0 0', unit: 'L1 1', count: 49_999, suffix: '' } } },
  { name: 'inline: 5000 elements (the cap) are accepted', layer: 'inline', svg: { viewBox: [0, 0, 1, 1], children: [] }, generate: { kind: 'repeat-element', target: 'children', count: 5000, element: { tag: 'circle', cx: 0, cy: 0, r: 1 } } },
  { name: 'inline: groups nested 16 deep (the cap) are accepted', layer: 'inline', svg: { viewBox: [0, 0, 1, 1], children: [] }, generate: { kind: 'nest-g', target: 'children', depth: 16, leaf: { tag: 'circle', cx: 0, cy: 0, r: 1 } } },
  { name: 'inline: 64 gradient stops (the cap) are accepted', layer: 'inline', svg: withDefs([{ tag: 'linearGradient', id: 'g', x1: 0, y1: 0, x2: 1, y2: 0, stops: Array.from({ length: 64 }, (_, i) => ({ offset: i / 63, color: '#000000' })) }]) },
  { name: 'inline: 16 dash values (the cap) are accepted', layer: 'inline', svg: P({ strokeDasharray: Array.from({ length: 16 }, () => 1) }) },
  // ── INLINE: normalised on output ──────────────────────────────────────────
  { name: 'inline: whitespace around a paint and around path data is trimmed in the generated markup', layer: 'inline', expectNormalised: true, svg: withChild({ tag: 'path', d: '  M0 0L10 10  ', fill: '  #FF0000  ' }) },
  { name: 'inline: numbers print to at most 3 decimals; local ids are scoped to the node', layer: 'inline', expectNormalised: true, svg: { viewBox: [0, 0, 3, 3], defs: [{ tag: 'linearGradient', id: 'shine', x1: 0.33333, y1: 0, x2: 2.6666666, y2: 0, stops: [{ offset: 0.123456, color: '#ffffff' }] }], children: [{ tag: 'circle', cx: 1.00049, cy: 1.0005, r: 0.66666, fill: 'url(#shine)', strokeWidth: 0.0004 }] } },
  // ── INLINE: refused (elements) ────────────────────────────────────────────
  { name: 'inline: a script element', layer: 'inline', svg: withChild({ tag: 'script', d: 'M0 0' }) },
  { name: 'inline: foreignObject', layer: 'inline', svg: withChild({ tag: 'foreignObject' }) },
  { name: 'inline: use (even with an internal href)', layer: 'inline', svg: withChild({ tag: 'use', href: '#g' }) },
  { name: 'inline: image with a data URI', layer: 'inline', svg: withChild({ tag: 'image', href: 'data:image/png;base64,AAAA' }) },
  { name: 'inline: text', layer: 'inline', svg: withChild({ tag: 'text', x: 0, y: 0 }) },
  { name: 'inline: a style element', layer: 'inline', svg: withChild({ tag: 'style' }) },
  { name: 'inline: an anchor (a)', layer: 'inline', svg: withChild({ tag: 'a', href: 'javascript:alert(1)', children: [] }) },
  { name: 'inline: polyline (real-world icons must be converted to path)', layer: 'inline', svg: withChild({ tag: 'polyline', points: '0,0 1,1' }) },
  { name: 'inline: animate / set', layer: 'inline', svg: withChild({ tag: 'set', attributeName: 'href', to: 'javascript:alert(1)' }) },
  { name: 'inline: a nested svg element', layer: 'inline', svg: withChild({ tag: 'svg', children: [] }) },
  { name: 'inline: a def that is not a gradient, clipPath or mask (filter)', layer: 'inline', svg: withDefs([{ tag: 'filter', id: 'f' }]) },
  { name: 'inline: a def that is a pattern', layer: 'inline', svg: withDefs([{ tag: 'pattern', id: 'p', children: [] }]) },
  // ── INLINE: refused (keys) ────────────────────────────────────────────────
  { name: 'inline: an onload handler key on a path', layer: 'inline', svg: P({ onload: 'alert(1)' }) },
  { name: 'inline: an onclick key on the top level', layer: 'inline', svg: { ...ICON, onclick: 'x' } },
  { name: 'inline: an href key on a path', layer: 'inline', svg: P({ href: 'https://x.test/a.svg' }) },
  { name: 'inline: an xlinkHref key on a path', layer: 'inline', svg: P({ xlinkHref: 'https://x.test/a.svg' }) },
  { name: 'inline: a style key', layer: 'inline', svg: P({ style: 'fill:url(https://evil.example/p.svg#g)' }) },
  { name: 'inline: a class key', layer: 'inline', svg: P({ class: 'icon' }) },
  { name: 'inline: an id key on a shape (only defs carry ids)', layer: 'inline', svg: P({ id: 'p1' }) },
  { name: 'inline: kebab-case attribute names are not the data spelling (stroke-width)', layer: 'inline', svg: P({ 'stroke-width': 2 }) },
  { name: 'inline: a __proto__ key (an own JSON property)', layer: 'inline', svg: JSON.parse('{"viewBox":[0,0,24,24],"children":[{"tag":"path","d":"M0 0","__proto__":{"polluted":true}}]}') },
  { name: 'inline: an extra key on a gradient (href)', layer: 'inline', svg: withDefs([{ ...grad, href: '#other' }]) },
  { name: 'inline: an extra key on a stop (href)', layer: 'inline', svg: withDefs([{ ...grad, stops: [{ offset: 0, color: '#000000', href: 'x' }] }]) },
  { name: 'inline: a top-level width key', layer: 'inline', svg: { ...withChild({ tag: 'circle', cx: 1, cy: 1, r: 1 }), width: 24 } },
  // ── INLINE: refused (paint and references) ────────────────────────────────
  { name: 'inline: a javascript: paint', layer: 'inline', svg: withFill('javascript:alert(1)') },
  { name: 'inline: an external url() paint', layer: 'inline', svg: withFill('url(https://x.test/p.svg#g)') },
  { name: 'inline: a quoted local url() paint (only the bare form is allowed)', layer: 'inline', svg: withDefs([grad], { tag: 'rect', x: 0, y: 0, width: 1, height: 1, fill: "url('#g')" }) },
  { name: 'inline: a paint referring to a missing gradient', layer: 'inline', svg: withFill('url(#nope)') },
  { name: 'inline: a paint referring to a clipPath instead of a gradient', layer: 'inline', svg: withDefs([{ tag: 'clipPath', id: 'c', children: [] }], { tag: 'rect', x: 0, y: 0, width: 1, height: 1, fill: 'url(#c)' }) },
  { name: 'inline: a clipPath reference that names a gradient', layer: 'inline', svg: withDefs([grad], { tag: 'rect', x: 0, y: 0, width: 1, height: 1, clipPath: 'g' }) },
  { name: 'inline: a mask reference in url() form (it takes the bare id)', layer: 'inline', svg: withDefs([{ tag: 'mask', id: 'm', children: [] }], { tag: 'rect', x: 0, y: 0, width: 1, height: 1, mask: 'url(#m)' }) },
  { name: 'inline: a named colour (red)', layer: 'inline', svg: withFill('red') },
  { name: 'inline: a CSS variable paint', layer: 'inline', svg: withFill('var(--stroke)') },
  { name: 'inline: an hsl() paint', layer: 'inline', svg: withFill('hsl(0, 100%, 50%)') },
  { name: 'inline: an rgb() paint with percentages', layer: 'inline', svg: withFill('rgb(100%, 0%, 0%)') },
  { name: 'inline: a stop colour that is a url()', layer: 'inline', svg: withDefs([{ ...grad, stops: [{ offset: 0, color: 'url(#g)' }] }]) },
  { name: 'inline: a paint that is not a string', layer: 'inline', svg: withFill(16711680) },
  // ── INLINE: refused (path data) ───────────────────────────────────────────
  { name: 'inline: markup smuggled into path data', layer: 'inline', svg: withChild({ tag: 'path', d: 'M0 0"/><script>alert(1)</script>' }) },
  { name: 'inline: path data not starting with M', layer: 'inline', svg: withChild({ tag: 'path', d: 'L0 0' }) },
  { name: 'inline: an exponent with no digits (1e)', layer: 'inline', svg: withChild({ tag: 'path', d: 'M0 0L1e 2' }) },
  { name: 'inline: a doubled exponent (1ee2)', layer: 'inline', svg: withChild({ tag: 'path', d: 'M0 0L1ee2 2' }) },
  { name: 'inline: a doubled sign (--1)', layer: 'inline', svg: withChild({ tag: 'path', d: 'M0 0L--1 2' }) },
  { name: 'inline: a sign followed by a space', layer: 'inline', svg: withChild({ tag: 'path', d: 'M0 0L- 1 2' }) },
  { name: 'inline: path data that is not a string', layer: 'inline', svg: withChild({ tag: 'path', d: ['M', 0, 0] }) },
  { name: 'inline: empty path data', layer: 'inline', svg: withChild({ tag: 'path', d: '' }) },
  { name: 'inline: a path one character over 200000', layer: 'inline', svg: withChild({ tag: 'path', d: '' }), generate: { ...path200k(49_999), suffix: ' ' } },
  { name: 'inline: path data over 1000000 characters in total (6 paths of 200000)', layer: 'inline', svg: { viewBox: [0, 0, 1, 1], children: [] }, generate: { kind: 'repeat-element', target: 'children', count: 6, element: { tag: 'path', d: '' }, elementString: { target: 'd', prefix: 'M0 0', unit: 'L1 1', count: 49_999, suffix: '' } } },
  // ── INLINE: refused (numbers, structure, caps) ────────────────────────────
  { name: 'inline: a viewBox with a zero width', layer: 'inline', svg: { viewBox: [0, 0, 0, 24], children: [] } },
  { name: 'inline: a viewBox of three numbers', layer: 'inline', svg: { viewBox: [0, 0, 24], children: [] } },
  { name: 'inline: a viewBox given as a string', layer: 'inline', svg: { viewBox: '0 0 24 24', children: [] } },
  { name: 'inline: a number one over the cap (10^7 + 1)', layer: 'inline', svg: withChild({ tag: 'rect', x: 0, y: 0, width: 10_000_001, height: 1 }) },
  { name: 'inline: a huge number (1e12)', layer: 'inline', svg: withChild({ tag: 'rect', x: 0, y: 0, width: 1e12, height: 1 }) },
  { name: 'inline: a number given as a string', layer: 'inline', svg: withChild({ tag: 'rect', x: 0, y: 0, width: '4', height: 4 }) },
  { name: 'inline: a rect missing its height', layer: 'inline', svg: withChild({ tag: 'rect', x: 0, y: 0, width: 4 }) },
  { name: 'inline: opacity over 1', layer: 'inline', svg: P({ opacity: 1.5 }) },
  { name: 'inline: a negative stroke width', layer: 'inline', svg: P({ strokeWidth: -1 }) },
  { name: 'inline: fillRule inherit', layer: 'inline', svg: P({ fillRule: 'inherit' }) },
  { name: 'inline: strokeLinejoin arcs (an SVG 2 value not on the list)', layer: 'inline', svg: P({ strokeLinejoin: 'arcs' }) },
  { name: 'inline: a dash array of all zeros', layer: 'inline', svg: P({ strokeDasharray: [0, 0] }) },
  { name: 'inline: 17 dash values', layer: 'inline', svg: P({ strokeDasharray: Array.from({ length: 17 }, () => 1) }) },
  { name: 'inline: a dash array given as a string', layer: 'inline', svg: P({ strokeDasharray: '4 2' }) },
  { name: 'inline: vectorEffect with another value', layer: 'inline', svg: P({ vectorEffect: 'non-scaling-size' }) },
  { name: 'inline: a transform of five numbers', layer: 'inline', svg: P({ transform: [1, 0, 0, 1, 0] }) },
  { name: 'inline: a transform given as a CSS string', layer: 'inline', svg: P({ transform: 'rotate(45)' }) },
  { name: 'inline: a group without children', layer: 'inline', svg: withChild({ tag: 'g' }) },
  { name: 'inline: groups nested 17 deep (one over the cap)', layer: 'inline', svg: { viewBox: [0, 0, 1, 1], children: [] }, generate: { kind: 'nest-g', target: 'children', depth: 17, leaf: { tag: 'circle', cx: 0, cy: 0, r: 1 } } },
  { name: 'inline: 5001 elements (one over the cap)', layer: 'inline', svg: { viewBox: [0, 0, 1, 1], children: [] }, generate: { kind: 'repeat-element', target: 'children', count: 5001, element: { tag: 'circle', cx: 0, cy: 0, r: 1 } } },
  { name: 'inline: 65 gradient stops (one over the cap)', layer: 'inline', svg: withDefs([{ tag: 'linearGradient', id: 'g', x1: 0, y1: 0, x2: 1, y2: 0, stops: Array.from({ length: 65 }, () => ({ offset: 0, color: '#000000' })) }]) },
  { name: 'inline: a stop offset over 1', layer: 'inline', svg: withDefs([{ ...grad, stops: [{ offset: 2, color: '#000000' }] }]) },
  { name: 'inline: a gradient without stops', layer: 'inline', svg: withDefs([{ ...grad, stops: [] }]) },
  { name: 'inline: a gradient missing x1', layer: 'inline', svg: withDefs([{ tag: 'linearGradient', id: 'g', y1: 0, x2: 1, y2: 0, stops: [{ offset: 0, color: '#000000' }] }]) },
  { name: 'inline: gradient units not on the list', layer: 'inline', svg: withDefs([{ ...grad, units: 'strokeWidth' }]) },
  { name: 'inline: a def id carrying markup', layer: 'inline', svg: withDefs([{ ...grad, id: 'g"><script>' }]) },
  { name: 'inline: a def id starting with a digit', layer: 'inline', svg: withDefs([{ ...grad, id: '1g' }]) },
  { name: 'inline: a def id of 65 characters', layer: 'inline', svg: withDefs([{ ...grad, id: 'g'.repeat(65) }]) },
  { name: 'inline: a def id used twice', layer: 'inline', svg: withDefs([grad, { ...grad }]) },
  { name: 'inline: markup instead of data', layer: 'inline', svg: '<svg onload="alert(1)"></svg>' },
  { name: 'inline: a list instead of an object', layer: 'inline', svg: [] },
  { name: 'inline: null on add (a write of null through update_node clears instead)', layer: 'inline', svg: null },
  { name: 'inline: svg content on a frame', layer: 'inline', nodeType: 'frame', svg: withChild({ tag: 'circle', cx: 1, cy: 1, r: 1 }) },
  { name: 'inline: svg content on a text node', layer: 'inline', nodeType: 'text', svg: withChild({ tag: 'circle', cx: 1, cy: 1, r: 1 }) },
  { name: 'inline: seven problems at once: the message names five and counts the rest', layer: 'inline', svg: { viewBox: [0, 0, 0, 0], style: 'x', children: [{ tag: 'script' }, { tag: 'path', d: 'L0', onload: 'x', fill: 'red', opacity: 2 }] } },
  // ── FILE (imported .svg files: sniffImage) — benign ──────────────────────
  { name: 'file: minimal self-contained SVG', layer: 'file', svg: `<svg ${NS}><rect/></svg>` },
  { name: 'file: Lucide icon as shipped (width/height 24, stroke attributes)', layer: 'file', svg: `<svg ${NS} width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-circle-check"><circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/></svg>` },
  { name: 'file: Material icon (viewBox only)', layer: 'file', svg: `<svg ${NS} height="24px" viewBox="0 -960 960 960" width="24px" fill="#1f1f1f"><path d="M382-240 154-468l57-57 171 171 367-367 57 57-424 424Z"/></svg>` },
  { name: 'file: XML prolog, a comment and a public DOCTYPE (with an http URL) before the root', layer: 'file', svg: `<?xml version="1.0"?>\n<!-- logo -->\n<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">\n<svg ${NSX} viewBox="0 0 120 80"><rect width="120" height="80"/></svg>` },
  { name: 'file: a BOM, percentage sizes and a viewBox (size from the viewBox)', layer: 'file', svg: `﻿<svg ${NSX} width="100%" height="100%" viewBox="0 0 10 20"/>` },
  { name: 'file: width in pt (48pt = 64 px)', layer: 'file', svg: `<svg ${NS} width="48pt" height="24"/>` },
  { name: 'file: width only, height from the viewBox ratio (comma-separated viewBox)', layer: 'file', svg: `<svg ${NS} width="300" viewBox="0,0,120,80"></svg>` },
  { name: 'file: em sizes are unknown (0 × 0; the studio measures by decoding)', layer: 'file', svg: `<svg ${NS} width="100%" height="2em"/>` },
  { name: 'file: internal references (url(#g) and use href="#g") are fine', layer: 'file', svg: `<svg ${NSX}><defs><linearGradient id="g"/></defs><rect fill="url(#g)"/><use href="#g"/><use xlink:href="#g"/></svg>` },
  { name: 'file: a style element without @import or external url()', layer: 'file', svg: `<svg ${NS}><style>.a{fill:#f00}</style><rect class="a" width="1" height="1"/></svg>` },
  { name: 'file: an embedded data: URI image is accepted (self-contained)', layer: 'file', svg: `<svg ${NSX}><image href="data:image/png;base64,iVBORw0KGgo=" width="1" height="1"/></svg>` },
  { name: 'file: SMIL animation without handlers or external references', layer: 'file', svg: `<svg ${NS} viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"><animate attributeName="r" values="4;2;4" dur="1s" repeatCount="indefinite"/></circle></svg>` },
  // ── FILE — refused ────────────────────────────────────────────────────────
  { name: 'file: a script element', layer: 'file', svg: `<svg ${NS}><script>alert(1)</script></svg>` },
  { name: 'file: a namespaced script element (svg:script)', layer: 'file', svg: `<svg ${NS}><svg:script href="x.js"/></svg>` },
  { name: 'file: an upper-case SCRIPT element', layer: 'file', svg: `<svg ${NS}><SCRIPT>alert(1)</SCRIPT></svg>` },
  { name: 'file: a script element inside a comment (refused: no parsing, a text match)', layer: 'file', svg: `<svg ${NS}><!-- <script> --><rect/></svg>` },
  { name: 'file: an onload handler on the root', layer: 'file', svg: `<svg ${NS} onload="alert(1)"><rect/></svg>` },
  { name: 'file: an ONCLICK handler with spaces around =', layer: 'file', svg: `<svg ${NS}><rect ONCLICK = "x()"/></svg>` },
  { name: 'file: an onbegin handler on an animation', layer: 'file', svg: `<svg ${NS}><rect><animate attributeName="x" onbegin="alert(1)"/></rect></svg>` },
  { name: 'file: an https image href', layer: 'file', svg: `<svg ${NSX}><image href="https://evil.example/a.png"/></svg>` },
  { name: 'file: an http xlink:href', layer: 'file', svg: `<svg ${NSX}><image xlink:href="http://evil.example/a.png"/></svg>` },
  { name: 'file: use pointing at a file: URL', layer: 'file', svg: `<svg ${NSX}><use href="file:///etc/passwd#x"/></svg>` },
  { name: 'file: use pointing at an external https SVG', layer: 'file', svg: `<svg ${NSX}><use href="https://evil.example/sprite.svg#icon"/></svg>` },
  { name: 'file: a protocol-relative href (//host)', layer: 'file', svg: `<svg ${NSX}><use href="//evil.example/s.svg#i"/></svg>` },
  { name: 'file: a javascript: link', layer: 'file', svg: `<svg ${NSX}><a href="javascript:alert(1)"><rect/></a></svg>` },
  { name: 'file: an unquoted javascript: href with a leading space inside', layer: 'file', svg: `<svg ${NSX}><a xlink:href = ' JavaScript:alert(1)'><rect/></a></svg>` },
  { name: 'file: an iframe src inside foreignObject', layer: 'file', svg: `<svg ${NS}><foreignObject width="10" height="10"><iframe xmlns="http://www.w3.org/1999/xhtml" src="https://evil.example/"/></foreignObject></svg>` },
  { name: 'file: an external url() in a style attribute', layer: 'file', svg: `<svg ${NS}><rect style="fill:url(https://evil.example/p.svg#g)"/></svg>` },
  { name: "file: a javascript: url() in a style element", layer: 'file', svg: `<svg ${NS}><style>rect{fill:url( 'javascript:alert(1)')}</style><rect/></svg>` },
  { name: 'file: a CSS @import', layer: 'file', svg: `<svg ${NS}><style>@import "x.css";</style></svg>` },
  { name: 'file: a CSS @import of a font URL', layer: 'file', svg: `<svg ${NS}><style>@IMPORT url(https://fonts.example/x.css);</style></svg>` },
  { name: 'file: an internal entity (billion-laughs shape)', layer: 'file', svg: `<!DOCTYPE svg [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">]><svg ${NS}><text>&b;</text></svg>` },
  { name: 'file: an external entity (XXE)', layer: 'file', svg: `<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><svg ${NS}><text>&xxe;</text></svg>` },
  { name: 'file: a parameter entity', layer: 'file', svg: `<!DOCTYPE svg [<!ENTITY % p SYSTEM "https://evil.example/x.dtd"> %p;]><svg ${NS}/>` },
  { name: 'file: a DOCTYPE internal subset with ATTLIST defaults (root not found: unsupported, not unsafe)', layer: 'file', svg: `<!DOCTYPE svg [<!ATTLIST svg onload CDATA "alert(1)">]><svg ${NS}/>` },
  { name: 'file: declared width 40000 (over 16384 a side)', layer: 'file', svg: `<svg ${NS} width="40000" height="10"/>` },
  { name: 'file: a viewBox of 20000 × 20000 with no width/height', layer: 'file', svg: `<svg ${NS} viewBox="0 0 20000 20000"/>` },
  { name: 'file: 10001 × 10001 (over 100 megapixels, each side under 16384)', layer: 'file', svg: `<svg ${NS} width="10001" height="10001"/>` },
  { name: 'file: width 1e9 in exponent notation', layer: 'file', svg: `<svg ${NS} width="1e9" height="1"/>` },
  { name: 'file: over 25 MB', layer: 'file', svg: '', generate: { kind: 'repeat-string', target: '', prefix: `<svg ${NS}>`, unit: '<rect width="1" height="1"/>', count: 1_000_000, suffix: '</svg>' } },
  { name: 'file: an Adobe Illustrator export with its usual namespace entities (a real-world file, refused)', layer: 'file', svg: `<?xml version="1.0" encoding="utf-8"?>\n<!-- Generator: Adobe Illustrator 16.0.0, SVG Export Plug-In . SVG Version: 6.00 Build 0)  -->\n<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd" [\n\t<!ENTITY ns_extend "http://ns.adobe.com/Extensibility/1.0/">\n\t<!ENTITY ns_svg "http://www.w3.org/2000/svg">\n]>\n<svg version="1.1" xmlns="&ns_svg;" width="32px" height="32px" viewBox="0 0 32 32"><rect width="32" height="32"/></svg>` },
  { name: 'file: an HTML document containing an svg (the root must be svg)', layer: 'file', svg: `<html><svg ${NS}></svg></html>` },
  { name: 'file: text before the root', layer: 'file', svg: `hello <svg ${NS}/>` },
  { name: 'file: a NUL byte in the first 1024 bytes', layer: 'file', svg: `<svg ${NS}>\u0000</svg>` },
  { name: 'file: invalid UTF-8', layer: 'file', svg: { bytes: [...new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg">'), 0xff, 0xfe, ...new TextEncoder().encode('</svg>')] } },
  { name: 'file: an unterminated comment before the root', layer: 'file', svg: `<!-- never closed <svg ${NS}/>` },
  // ── FILE — accepted, but arguably "reaching outside" (see the spec's Gaps) ─
  { name: 'file (gap): deep nesting has no cap (50000 nested g)', layer: 'file', svg: '', generate: { kind: 'nest', target: '', prefix: `<svg ${NS}>`, open: '<g>', close: '</g>', count: 50_000, inner: '<rect/>', suffix: '</svg>' } },
  { name: 'file (gap): an xml-stylesheet processing instruction with an https URL', layer: 'file', svg: `<?xml-stylesheet href="https://evil.example/x.css"?><svg ${NS}/>` },
  { name: 'file (gap): a SYSTEM DOCTYPE pointing at an https DTD', layer: 'file', svg: `<!DOCTYPE svg SYSTEM "https://evil.example/x.dtd"><svg ${NS}/>` },
  { name: 'file (gap): a character-reference-encoded javascript: href', layer: 'file', svg: `<svg ${NSX}><a href="&#106;avascript:alert(1)"><rect/></a></svg>` },
  { name: 'file (gap): a character-reference-encoded https href', layer: 'file', svg: `<svg ${NSX}><image href="&#x68;ttps://evil.example/a.png"/></svg>` },
  { name: 'file (gap): SMIL set writing a javascript: href', layer: 'file', svg: `<svg ${NSX}><a><set attributeName="href" to="javascript:alert(1)"/><rect/></a></svg>` },
  { name: 'file (gap): a relative external reference (use href="other.svg#a")', layer: 'file', svg: `<svg ${NSX}><use href="other.svg#a"/></svg>` },
  { name: 'file (gap): a relative url() in CSS', layer: 'file', svg: `<svg ${NS}><rect style="fill:url(other.svg#g)"/></svg>` },
  { name: 'file (gap): foreignObject with XHTML content (no script, no URL)', layer: 'file', svg: `<svg ${NS}><foreignObject width="10" height="10"><div xmlns="http://www.w3.org/1999/xhtml">hi</div></foreignObject></svg>` },
  { name: 'file (gap): an iframe srcdoc inside foreignObject', layer: 'file', svg: `<svg ${NS}><foreignObject width="10" height="10"><iframe xmlns="http://www.w3.org/1999/xhtml" srcdoc="&lt;script&gt;alert(1)&lt;/script&gt;"/></foreignObject></svg>` },
  { name: 'file (gap): a handler after a slash (<svg/onload=…>, malformed XML)', layer: 'file', svg: `<svg/onload=alert(1)>` },
  // ── PATH EDIT (core vector/path.ts parsePath → serializePath) ─────────────
  { name: 'pathEdit: absolute M L C Q Z round-trips unchanged', layer: 'pathEdit', svg: 'M10 20L30 40C1 2 3 4 5 6Q7 8 9 10Z' },
  { name: 'pathEdit: relative commands become absolute, H / V become L', layer: 'pathEdit', svg: 'm10 10 l5 0 h5 v5 H0 V0 z' },
  { name: 'pathEdit: implicit lines after M (absolute and relative)', layer: 'pathEdit', svg: 'm1 1 2 2' },
  { name: 'pathEdit: S and T reflect the previous control point', layer: 'pathEdit', svg: 'M0 0C0 10 10 10 10 0S20 -10 20 0' },
  { name: 'pathEdit: T after Q', layer: 'pathEdit', svg: 'M0 0Q5 10 10 0T20 0' },
  { name: 'pathEdit: compact numbers (signs and dots separate; exponents)', layer: 'pathEdit', svg: 'M.5.5L-1-1L1e2 2E-1' },
  { name: 'pathEdit: a half-circle arc becomes 2 cubics', layer: 'pathEdit', svg: 'M0 50A50 50 0 0 1 100 50' },
  { name: 'pathEdit: compact arc flags ("a1 1 0 011 1")', layer: 'pathEdit', svg: 'M0 0a1 1 0 011 1' },
  { name: 'pathEdit: an arc with a zero radius is a line', layer: 'pathEdit', svg: 'M0 0A0 5 0 0 1 10 0' },
  { name: 'pathEdit: an arc to its own start point vanishes', layer: 'pathEdit', svg: 'M5 5A3 3 0 0 1 5 5' },
  { name: 'pathEdit: numbers serialise to 0.01', layer: 'pathEdit', svg: 'M0.004 1.235L3.33333 -0.004' },
  { name: 'pathEdit: an odd argument count is refused, never repaired', layer: 'pathEdit', svg: 'M0 0 1' },
  { name: 'pathEdit: a dangling command is refused', layer: 'pathEdit', svg: 'M1 1 L' },
  { name: 'pathEdit: an arc flag other than 0 or 1 is refused', layer: 'pathEdit', svg: 'M0 0A1 1 0 2 1 5 5' },
  { name: 'pathEdit: an unknown command letter is refused', layer: 'pathEdit', svg: 'M1 1 X2 2' },
  { name: 'pathEdit: Z alone is refused', layer: 'pathEdit', svg: 'Z' },
  { name: 'pathEdit: numbers after Z are refused', layer: 'pathEdit', svg: 'M0 0L1 1Z 2 2' },
  // ── EDIT REFUSAL (core vector/box.ts vectorEditRefusal) ───────────────────
  { name: 'editRefusal: one path is editable', layer: 'editRefusal', svg: withChild({ tag: 'path', d: 'M0 0L1 1' }) },
  { name: 'editRefusal: two shapes', layer: 'editRefusal', svg: { viewBox: [0, 0, 1, 1], children: [{ tag: 'path', d: 'M0 0' }, { tag: 'path', d: 'M1 1' }] } },
  { name: 'editRefusal: a rect', layer: 'editRefusal', svg: withChild({ tag: 'rect', x: 0, y: 0, width: 1, height: 1 }) },
  { name: 'editRefusal: a path with a transform', layer: 'editRefusal', svg: withChild({ tag: 'path', d: 'M0 0L1 1', transform: [1, 0, 0, 1, 0, 0] }) },
  { name: 'editRefusal: a path with defs', layer: 'editRefusal', svg: withDefs([grad], { tag: 'path', d: 'M0 0L1 1', fill: 'url(#g)' }) },
];

// round-trip through JSON first: what is tested is exactly what is written
const parsed: Case[] = JSON.parse(JSON.stringify(cases));
// the __proto__ case must stay an own property after the round trip
for (const c of parsed) {
  if (c.layer === 'inline') runInline(c);
  else if (c.layer === 'file') runFile(c);
  else if (c.layer === 'pathEdit') runPath(c);
  else runEdit(c);
}

const doc = {
  about: 'Scrim SVG allow-list cases (docs/transfer/specs/svg-allowlist.md). Every expectation was produced by running Scrim\'s own code, not written by hand.',
  generatedAt: new Date().toISOString().slice(0, 10),
  scrimCommit: 'f2b1bdb',
  sources: {
    inline: 'packages/core/src/svg.ts svgContentIssues via applyPatch(addNode(box "Icon" id "v" with svg)) (core patch.ts checkSvg); markup = svgMarkup(svgTree(svg, "svg-v"))',
    file: 'packages/mcp/src/assets/sniff.ts sniffImage(bytes) (the import gate for .svg files); svgRefusal(text) shown separately',
    pathEdit: 'packages/core/src/vector/path.ts parsePath(d) then serializePath(segs); writeIssues = svgContentIssues of a one-path svg with that d',
    editRefusal: 'packages/core/src/vector/box.ts vectorEditRefusal(svg)',
  },
  conventions: {
    layer: '"inline": structured vector data on a node (JSON; the svg field). "file": the text (or bytes) of an imported .svg file. "pathEdit": path data parsed for path edit mode. "editRefusal": whether path edit mode can open the vector.',
    expect: '"accept" | { refuse: "<verbatim message>" } | { normalised: "<verbatim output>" }. inline refuse = the PatchError message (node name "Icon" prefixed, at most 5 issues joined by "; ", then " (and N more)"); file refuse = the AssetError message; pathEdit refuse is the literal "parsePath: null" (the parser returns null and shows no message); pathEdit normalised = serializePath output; inline normalised = the generated markup.',
    issues: 'inline only: every issue svgContentIssues returns, unprefixed (the first 20 when there are more).',
    nodeType: 'inline only: the node type the svg is written on (default "box").',
    markup: 'inline accept only (small cases): the markup Scrim generates for that svg on node id "v".',
    code: 'file refuse only: the AssetError code.',
    bytes: 'file only: the byte length of the materialised input.',
    sniffed: 'file accept only: { ext, width, height, orientation } as sniffImage reports.',
    svgRefusal: 'file only: svgRefusal(text) — the bare reason before ": Scrim imports only self-contained SVGs" is appended (null: nothing found).',
    writeIssues: 'pathEdit only: what the write-time check says about a one-path svg with this d (empty: accepted on write).',
    generate: {
      what: 'Large inputs are stored as a generator; materialise them before running. target is a dot path into svg ("" = svg itself; "children.0.d" = svg.children[0].d).',
      'repeat-string': 'set target to prefix + unit repeated count times + suffix',
      'repeat-element': 'set target to an array of count deep copies of element; with elementString, each copy\'s elementString.target is first set to its prefix + unit×count + suffix',
      nest: 'set target to prefix + open×count + inner + close×count + suffix',
      'nest-g': 'set target to [ g ] nested depth times around [ leaf ]: depth 1 = [{ tag: "g", children: [leaf] }]',
    },
    'file svg as bytes': 'a file case whose svg is { bytes: [...] } is raw bytes (not valid UTF-8).',
  },
  cases: parsed,
};
await Bun.write(OUT, JSON.stringify(doc, null, 1) + '\n');
const tally: Record<string, number> = {};
for (const c of parsed) {
  const k = `${c.layer}:${typeof c.expect === 'string' ? c.expect : Object.keys(c.expect as object)[0]}`;
  tally[k] = (tally[k] ?? 0) + 1;
}
console.log(parsed.length, tally);
