// Generates docs/transfer/cases/analyze-design.json by running Scrim's real
// analyze_design tool. Run: node cases-analyze.mjs <outPath>
import { writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { core, mk, run } from './lib.mjs';

const OUT = process.argv[2];
const t = (id, text, style = {}, name) => ({ id, type: 'text', name: name ?? id, text, style });
const f = (id, layout, style, children = [], extra = {}) => ({ id, type: 'frame', name: extra.name ?? id, layout, style, children, ...extra });
const b = (id, style = {}, extra = {}) => ({ id, type: 'box', name: extra.name ?? id, style, ...extra });
const img = (id, extra = {}) => ({ id, type: 'image', name: extra.name ?? 'Image', src: 'data:image/png;base64,AA==', style: { width: 320, height: 180 }, ...extra });

/** Turn root `masterId` into a component and add `copies` instances as roots. */
function componentize(doc, masterId, compId, copies) {
  doc = core.applyPatch(doc, core.createComponentFromSubtree(doc, masterId, { id: compId, name: compId, props: [] }).patch).doc;
  for (let i = 0; i < copies; i++) {
    doc = core.applyPatch(doc, core.instantiateComponent(doc, compId, { parentId: null, index: doc.rootIds.length, canvas: { x: 3000 + i * 400, y: 0 }, name: `${compId} copy ${i + 1}` }).patch).doc;
  }
  return doc;
}

// ── A1: a landing page with a full token set ─────────────────────────────
function landing() {
  const card = (i) => f(`card${i}`, { display: 'flex', flexDirection: 'column', gap: 12, padding: 24 }, { background: '#fafaf7', borderRadius: 12, border: '1px solid #e7e5e4' }, [
    { id: `icon${i}`, type: 'box', name: 'Icon', style: { width: 24, height: 24 }, svg: { viewBox: [0, 0, 24, 24], children: [{ tag: 'circle', cx: 12, cy: 12, r: 10, fill: '#FF7A59', stroke: 'none' }] } },
    t(`ct${i}`, `Feature ${i}`, { fontSize: 24, fontWeight: 600 }, 'Title'),
    t(`cb${i}`, 'Short explanation of the feature.', { color: '#555555' }, 'Body'),
  ], { name: `Card ${i}` });
  let doc = mk({
    id: 'landing', name: 'Landing',
    tokens: {
      colors: { accent: '#ff7a59', brand: '#1a2b3c', ink: '#111111', surface: '#ffffff', 'ink-50': '#00000080' },
      spacing: { xs: 4, sm: 8, md: 16, lg: '24px', xl: '48px' },
      typography: { body: { fontFamily: 'Söhne', fontSize: 16, fontWeight: 400, lineHeight: 1.5 }, h1: { fontFamily: 'Canela', fontSize: 56, fontWeight: 700 } },
    },
    roots: [
      f('page', { display: 'flex', flexDirection: 'column', gap: 48, padding: 48 }, { background: '#ffffff', fontFamily: 'Söhne', fontSize: 16, fontWeight: 400, lineHeight: 1.5, color: '#111111', width: 1280 }, [
        f('nav', { display: 'flex', gap: 16, padding: '12px 24px' }, { borderBottom: '1px solid #e7e5e4' }, [
          t('logo', 'Moth', { fontWeight: 700, color: 'var(--ink, #111111)' }),
          t('link1', 'Pricing', { color: '#111112' }),
          t('link2', 'Docs', { color: '#555555' }),
          f('cta', { display: 'flex', padding: '8px 16px' }, { background: '#1a2b3d', borderRadius: 999 }, [t('ctaLabel', 'Start free', { color: '#ffffff', fontWeight: 600 })]),
        ]),
        f('hero', { display: 'flex', flexDirection: 'column', gap: 16 }, {}, [
          t('h1', 'Invoices your accountant will like', { fontFamily: 'Canela', fontSize: 56, fontWeight: 700, lineHeight: 1.1 }),
          t('lede', 'Send, chase and reconcile without a spreadsheet.', { fontSize: 20, color: 'rgba(0, 0, 0, 0.5)' }),
          b('heroImage', { backgroundImage: 'linear-gradient(90deg, #ffd27a, #ff7a59)', height: 360, borderRadius: 16, boxShadow: '0 12px 40px rgba(26, 43, 60, 0.15)' }),
        ]),
        f('features', { display: 'flex', gap: 24 }, {}, [card(1), card(2), card(3)]),
        f('footer', { display: 'flex', gap: 8, padding: 24 }, { background: 'var(--brand)', color: '#fafaf9' }, [t('legal', '© Moth Ltd', { fontSize: 12 }), t('legal2', 'Privacy', { fontSize: 12 })]),
      ], { canvas: { x: 0, y: 0 }, name: 'Landing' }),
      f('m', { display: 'flex', flexDirection: 'column', gap: 12, padding: 24 }, { background: '#fafaf7' }, [
        { id: 'mi', type: 'box', name: 'Icon', style: { width: 24, height: 24 } }, t('mt', 'Title', { fontSize: 24 }), t('mb', 'Body'),
      ], { canvas: { x: 2000, y: 0 }, name: 'Card master' }),
    ],
  });
  return componentize(doc, 'm', 'featureCard', 1);
}

// ── A3: a marketing page without tokens: the 4px grid and the unparsed counts ──
function noTokens() {
  return mk({
    id: 'promo', name: 'Promo (no tokens)',
    roots: [
      f('promo', { display: 'flex', flexDirection: 'column', gap: 10, padding: 18 }, { background: 'white', width: 960 }, [
        t('kicker', 'Limited offer', { color: 'red', fontSize: 14, letterSpacing: '0.1em' }),
        t('title', 'Half price for a year', { color: 'black', fontSize: 40 }),
        t('copy', 'Ends Friday.', { color: 'rebeccapurple' }),
        t('note', 'Terms apply.', { color: 'hsl(210, 40%, 40%)' }),
        t('ghost', 'Inherits the link colour', { color: 'currentColor' }),
        f('row', { display: 'flex', gap: 'var(--space-md)', padding: '0 12px', margin: '-8px auto 5%' }, { background: 'transparent' }, [
          b('pill', { background: 'rgba(37, 99, 235, 0.1)', borderColor: 'rgb(37,99,235)', borderRadius: 999, padding: 'calc(4px + 2px) 6px' }),
          b('swatch', { backgroundColor: '#2563eb', margin: 2, width: 40, height: 40 }),
          b('swatch2', { backgroundColor: '#2564eb', marginTop: 1, width: 40, height: 40 }),
          b('fade', { backgroundImage: 'linear-gradient(hsl(0, 0%, 100%), hsl(0, 0%, 90%))', paddingLeft: '7px', width: 40, height: 40 }),
        ]),
      ], { canvas: { x: 0, y: 0 }, name: 'Promo' }),
    ],
  });
}

// ── A4: an editorial article: type styles with inheritance and token matching ──
function editorial() {
  return mk({
    id: 'article', name: 'Article',
    tokens: {
      colors: { paper: '#f7f3ea', ink: '#1c1917' },
      spacing: { s: 8, m: 24, l: 64 },
      typography: {
        display: { fontFamily: 'Canela', fontSize: '72px', lineHeight: '1' },
        body: { fontFamily: 'Tiempos', fontSize: 18, lineHeight: 1.6 },
        caption: { fontFamily: 'Söhne Mono', fontSize: 12, letterSpacing: '0.04em' },
        empty: {},
      },
    },
    roots: [
      f('article', { display: 'flex', flexDirection: 'column', gap: 24, padding: 64 }, { background: '#f7f3ea', color: '#1c1917', fontFamily: 'Tiempos', fontSize: 18, lineHeight: 1.6, width: 760 }, [
        t('head', 'The quiet return of the moth', { fontFamily: 'Canela', fontSize: 72, lineHeight: '1' }),
        t('deck', 'Why night-flying insects are back in the city', { fontSize: 24, fontStyle: 'italic' }),
        f('byline', { display: 'flex', gap: 8 }, { fontFamily: 'Söhne Mono', fontSize: 12, letterSpacing: '0.04em' }, [t('author', 'By R. Okafor'), t('date', '12 Oct 2026')]),
        t('p1', 'First paragraph of the body copy.'),
        t('p2', 'Second paragraph of the body copy.'),
        t('p3', 'Third paragraph, set in px.', { fontSize: '18px' }),
        f('figure', { display: 'flex', flexDirection: 'column', gap: 8 }, {}, [img('photo'), t('cap', 'A moth on a lamp post.', { fontFamily: 'Söhne Mono', fontSize: 12, letterSpacing: '0.04em' })]),
        t('pull', 'Pull quote', { fontFamily: 'Canela', fontSize: '72px', lineHeight: 1 }),
      ], { canvas: { x: 0, y: 0 }, name: 'Article' }),
    ],
  });
}

// ── A5: a dashboard: repeated structures, nesting and components ─────────
function dashboard() {
  const stat = (i) => f(`stat${i}`, { display: 'flex', flexDirection: 'column', gap: 4, padding: 16 }, { background: '#ffffff', borderRadius: 8 }, [t(`sl${i}`, 'Revenue', { fontSize: 12, color: '#6b7280' }), t(`sv${i}`, '£12,400', { fontSize: 28, fontWeight: 600 })], { name: i <= 3 ? `Stat ${i}` : 'Stat' });
  const row = (i) => f(`row${i}`, { display: 'flex', gap: 12, padding: '8px 12px' }, { borderBottom: '1px solid #f3f4f6' }, [f(`cell${i}`, {}, {}, [t(`rn${i}`, `Invoice ${i}`), t(`rd${i}`, 'Due 1 Nov', { fontSize: 12 })], { name: 'Cell' }), t(`ra${i}`, '£120'), b(`rs${i}`, { background: '#dcfce7', borderRadius: 999, width: 8, height: 8 })], { name: `Row ${i}` });
  let doc = mk({
    id: 'dash', name: 'Dashboard',
    roots: [
      f('dash', { display: 'flex', flexDirection: 'column', gap: 24, padding: 32 }, { background: '#f9fafb', fontFamily: 'Söhne', width: 1440 }, [
        f('stats', { display: 'grid', gap: 16 }, { gridTemplateColumns: 'repeat(4, 1fr)' }, [1, 2, 3, 4].map(stat)),
        f('table', { display: 'flex', flexDirection: 'column' }, { background: '#ffffff', borderRadius: 8 }, Array.from({ length: 12 }, (_, i) => row(i + 1))),
        f('side', { display: 'flex', flexDirection: 'column', gap: 4, padding: 16 }, {}, [t('sideLabel', 'Total', { fontSize: 12 }), t('sideValue', '£3,100', { fontSize: 28 })]),
      ], { canvas: { x: 0, y: 0 }, name: 'Dashboard' }),
      f('btn', { display: 'flex', gap: 8, padding: '8px 16px' }, { background: '#111827', borderRadius: 6 }, [b('btnIcon', { width: 16, height: 16 }), t('btnLabel', 'Export', { color: '#ffffff' })], { canvas: { x: 2000, y: 0 }, name: 'Button' }),
    ],
  });
  return componentize(doc, 'btn', 'button', 2);
}

const L = landing();
const cases = [
  { name: 'landing/document', description: 'A landing page with colour, spacing and typography tokens; var() references (with and without fallbacks); near-duplicate colours; inline svg paints; three repeated feature cards; a component master and one instance (never component candidates).', doc: L, steps: [{ tool: 'analyze_design' }] },
  { name: 'landing/scoped-features', description: 'The same document scoped to one subtree (id): every report covers that subtree only; inherited type values come from the scope\'s ancestors.', doc: L, steps: [{ tool: 'analyze_design', args: { id: 'features' } }] },
  { name: 'landing/scoped-leaf', description: 'Scoped to a single text node: one node, one inherited type style, nothing repeated.', doc: L, steps: [{ tool: 'analyze_design', args: { id: 'ct2' } }] },
  { name: 'promo/no-tokens', description: 'No tokens: spacing is judged against the 4 px grid (1 and 2 allowed); named colours, rgba alpha, transparent; unparsed colours (rebeccapurple, hsl()) and unparsed spacing (var(), 5%, calc() parts); negative margins; auto ignored.', doc: noTokens(), steps: [{ tool: 'analyze_design' }] },
  { name: 'article/typography', description: 'Type styles with inheritance from the article frame and the byline, matched to typography tokens (a token with no keys never matches; a token leaves unset keys free; QUIRK: values compare numerically only when BOTH are px-suffixed strings — numeric 72 does not match a token "72px", and "18px" does not match a token 18; the same spelling always matches).', doc: editorial(), steps: [{ tool: 'analyze_design' }] },
  { name: 'dashboard/repeats', description: 'Twelve table rows (ids capped at 10, names at 3 distinct); the 3-node cells inside every row form a group whose members all sit inside another reported group, so it is dropped; four stat cards plus one same-shape subtree outside the grid (5 copies); a component master and its instances are excluded.', doc: dashboard(), steps: [{ tool: 'analyze_design' }] },
  { name: 'errors/not-found', description: 'An unknown id is refused.', doc: noTokens(), steps: [{ tool: 'analyze_design', args: { id: 'missing' } }] },
  { name: 'empty/document', description: 'An empty document: every list empty, the 4 px grid scale string.', doc: mk({ id: 'empty', name: 'Empty', roots: [] }), steps: [{ tool: 'analyze_design' }] },
];

const out = [];
for (const c of cases) {
  const r = await run(c.doc, c.steps);
  out.push({ name: c.name, description: c.description, doc: c.doc, steps: r.steps });
}
const commit = execSync(`git -C ${process.env.SCRIM_REPO ?? new URL('../../../../', import.meta.url).pathname} rev-parse --short HEAD`).toString().trim();
writeFileSync(OUT, JSON.stringify({ schema: 'scrim-transfer/analyze-design-cases@1', generatedFrom: `Scrim ${commit} (packages/*/dist), via the MCP tool in-process`, spec: '../specs/analyze-design.md', cases: out }, null, 2) + '\n');
console.log(`${out.length} cases`);
