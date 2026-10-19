// Generates docs/transfer/cases/query-nodes.json by RUNNING Scrim's real
// implementation (core src for queryNodes; the MCP server through the CLI's
// in-process session for tool cases; the CLI's run() for CLI cases).
// Run: bun gen-query-cases.ts   (from this directory)

import { mkdirSync, writeFileSync, mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  addNode, applyPatch, createComponentFromSubtree, createDocument, createNode, instantiateComponent,
  parseQuery, queryNodes, QueryError, MAX_QUERY_LENGTH, type Node, type ScrimDoc,
} from '../../../packages/core/src/index.ts';
import * as coreDist from '../../../packages/core/dist/index.js';
import { openSession } from '../../../packages/cli/src/session.ts';
import { run } from '../../../packages/cli/src/cli.ts';
import { gateDocuments } from '../../../packages/mcp/src/__tests__/exportGateDocs.ts';

const OUT = '../../../docs/transfer/cases/query-nodes.json';

/** Rename generated ids to stable ones (rootIds, nodes, parentId, childIds). */
function remap(doc: ScrimDoc, map: Record<string, string>): ScrimDoc {
  const r = (id: string): string => map[id] ?? id;
  const nodes: Record<string, Node> = {};
  for (const n of Object.values(doc.nodes)) {
    nodes[r(n.id)] = { ...n, id: r(n.id), parentId: n.parentId === null ? null : r(n.parentId), childIds: n.childIds.map(r) };
  }
  return { ...doc, rootIds: doc.rootIds.map(r), nodes };
}

// ── documents ────────────────────────────────────────────────────────────────

/** The core test document (packages/core/src/__tests__/query.test.ts build()). */
function queryDoc(): ScrimDoc {
  let doc = createDocument({ id: 'q', name: 'Query' });
  const add = (node: Node, parent: string | null, index: number): void => {
    doc = applyPatch(doc, addNode(node, parent, index)).doc;
  };
  add(createNode('frame', { id: 'page', name: 'Page', canvas: { x: 0, y: 0 }, layout: { display: 'flex', flexDirection: 'column', gap: 32 }, style: { width: 1280, background: '#ffffff' } }), null, 0);
  add(createNode('frame', { id: 'nav', name: 'Nav', layout: { display: 'flex' }, style: { height: 'fit-content' } }), 'page', 0);
  add(createNode('text', { id: 'logo', name: 'Logo', text: 'Moth & Co', style: { color: '#111111', fontSize: 20 } }), 'nav', 0);
  add(createNode('box', { id: 'cta', name: 'CTA button', style: { width: '120px', backgroundColor: '#1a2b3c' } }), 'nav', 1);
  add(createNode('frame', { id: 'hero', name: 'Hero', layout: { display: 'grid' } }), 'page', 1);
  add(createNode('text', { id: 'h1', name: 'Headline', text: 'Night moths', style: { fontSize: 56 } }), 'hero', 0);
  add(createNode('text', { id: 'lede', name: 'Lede', text: 'Quiet light', style: { fontSize: 18 } }), 'hero', 1);
  add(createNode('image', { id: 'photo', name: 'Photo', src: `scrim-asset:${'a'.repeat(64)}.png`, alt: 'A moth', style: { width: 320, height: 200 } }), 'page', 2);
  add(createNode('frame', { id: 'poster', name: 'Poster', canvas: { x: 1400, y: 0 }, layout: { display: 'block', position: 'relative' }, style: { width: 600, height: 800 } }), null, 1);
  add(createNode('box', { id: 'shape', name: 'Shape', layout: { position: 'absolute' }, style: { width: '50%' } }), 'poster', 0);
  add(createNode('text', { id: 'title', name: 'Title', text: 'MOTH WATCH', style: { color: 'transparent' } }), 'poster', 1);
  add(createNode('frame', { id: 'card', name: 'Card', canvas: { x: 0, y: 1200 }, layout: { display: 'flex' } }), null, 2);
  add(createNode('text', { id: 'ctitle', name: 'Title', text: 'Pricing' }), 'card', 0);
  doc = applyPatch(doc, createComponentFromSubtree(doc, 'card', { id: 'cmp-card', name: 'Pricing Card', props: [] }).patch).doc;
  const inst = instantiateComponent(doc, 'cmp-card', { parentId: null, index: 3, canvas: { x: 400, y: 1200 }, name: 'Pro' });
  doc = applyPatch(doc, inst.patch).doc;
  const instId = doc.rootIds[3]!;
  return remap(doc, { [instId]: 'inst', [doc.nodes[instId]!.childIds[0]!]: 'inst-title' });
}

/** query + a box with position/display only in `style` (the "declared" test). */
function strayDoc(base: ScrimDoc): ScrimDoc {
  const doc = applyPatch(base, addNode(createNode('box', { id: 'stray', name: 'Stray', style: { position: 'absolute', display: 'flex' } as Record<string, string> }), 'poster', 2)).doc;
  return { ...doc, id: 'q-stray' };
}

/** Edge values the core tests don't pin. */
function edgeDoc(): ScrimDoc {
  let doc = createDocument({ id: 'edge', name: 'Edge' });
  const counts = new Map<string | null, number>();
  const add = (node: Node, parent: string | null): void => {
    const index = counts.get(parent) ?? 0;
    counts.set(parent, index + 1);
    doc = applyPatch(doc, addNode(node, parent, index)).doc;
  };
  add(createNode('frame', { id: 'a', name: 'Artboard', canvas: { x: 0, y: 0 }, layout: { display: 'inline-flex' }, style: { width: ' 240px ', paddingTop: '' } }), null);
  add(createNode('box', { id: 'neg', name: '', style: { width: '-.5px', height: '240PX', opacity: 0.5 } }), 'a');
  add(createNode('box', { id: 'sci', name: 'it\'s "quoted"', style: { width: '1e2', height: '+5', zIndex: 3 } }), 'a');
  add(createNode('text', { id: 'empty', name: 'Café ☕', text: '', style: { background: '#000000' } }), 'a');
  add(createNode('box', { id: 'rev', name: 'Reverse', layout: { display: 'flex', flexDirection: 'row-reverse', position: 'absolute' }, style: { position: 'relative', display: '', background: '', backgroundColor: '#ff0000' } as Record<string, string> }), 'a');
  add(createNode('frame', { id: 'outer', name: 'Outer', layout: { display: 'block' } }), 'a');
  add(createNode('frame', { id: 'inner', name: 'Inner', layout: { display: 'block' } }), 'outer');
  add(createNode('text', { id: 'deep1', name: 'Deep', text: 'one' }), 'inner');
  add(createNode('frame', { id: 'innermost', name: 'Innermost', layout: { display: 'block' } }), 'inner');
  add(createNode('text', { id: 'deep2', name: 'Deep', text: 'two' }), 'innermost');
  add(createNode('text', { id: 'deep3', name: 'Deep', text: 'three' }), 'outer');
  add(createNode('frame', { id: 'btn', name: 'Button', layout: { display: 'flex' }, style: { width: 100 } }), 'a');
  add(createNode('text', { id: 'btnlabel', name: 'Label', text: 'Go' }), 'btn');
  add(createNode('image', { id: 'img', name: 'Img', src: 'data:image/png;base64,AAAA', style: { width: 0 } }), 'a');
  doc = applyPatch(doc, createComponentFromSubtree(doc, 'btn', { id: 'cmp-btn', name: 'Button', props: [] }).patch).doc;
  const inst = instantiateComponent(doc, 'cmp-btn', { parentId: 'outer', index: 2, name: 'Button copy' });
  doc = applyPatch(doc, inst.patch).doc;
  const instId = doc.nodes.outer!.childIds[2]!;
  return remap(doc, { [instId]: 'binst', [doc.nodes[instId]!.childIds[0]!]: 'binst-label' });
}

/** n text nodes in one flex frame (the MCP limit test's many()). */
function manyDoc(n: number): ScrimDoc {
  let doc = createDocument({ id: 'many', name: 'Many' });
  doc = applyPatch(doc, addNode(createNode('frame', { id: 'f', name: 'F', canvas: { x: 0, y: 0 }, layout: { display: 'flex' } }), null, 0)).doc;
  doc = applyPatch(doc, Array.from({ length: n }, (_, i) => addNode(createNode('text', { id: `t${i}`, name: `T${i}`, text: 'x' }), 'f', i))).doc;
  return doc;
}

const DOCS: Record<string, ScrimDoc> = {};
DOCS.query = queryDoc();
DOCS['query-stray'] = strayDoc(DOCS.query);
DOCS.edge = edgeDoc();
DOCS.landing = gateDocuments().find(([n]) => n === 'landing')![1];
DOCS.many = manyDoc(105);

// ── core cases ───────────────────────────────────────────────────────────────

type Lint = Record<string, string[]>;
interface CaseIn { name: string; doc: string; query: string; lint?: Lint; note?: string }
const LINT: Lint = { h1: ['oversized-h1', 'flat-type-hierarchy'], lede: ['flat-type-hierarchy'] };
const LINT_DUP: Lint = { h1: ['flat-type-hierarchy', 'flat-type-hierarchy'] };

const C = (name: string, query: string, extra: Partial<CaseIn> = {}): CaseIn => ({ name, doc: 'query', query, ...extra });
const E = (name: string, query: string, extra: Partial<CaseIn> = {}): CaseIn => ({ name, doc: 'edge', query, ...extra });

const cases: CaseIn[] = [
  // paths and node tests (core tests)
  C('path: / is a top-level artboard', '/FRAME'),
  C('path: /* every artboard', '/*'),
  C('path: //TEXT anywhere, instance children included', '//TEXT'),
  C('path: /FRAME/FRAME children of artboards', '/FRAME/FRAME'),
  C('path: /FRAME//TEXT', '/FRAME//TEXT'),
  C('path: //FRAME//TEXT (a master is not a FRAME)', '//FRAME//TEXT'),
  C('path: //*//TEXT dedupes and keeps document order', '//*//TEXT'),
  C('path: //* every node once', '//*'),
  C('test: BOX', '//BOX'),
  C('test: IMAGE', '//IMAGE'),
  C('test: INSTANCE', '//INSTANCE'),
  C('test: COMPONENT (the master root)', '//COMPONENT'),
  C('test: /COMPONENT at top level', '/COMPONENT'),
  C('test: //COMPONENT/TEXT', '//COMPONENT/TEXT'),
  C('test: //INSTANCE/TEXT (materialised instance children)', '//INSTANCE/TEXT'),
  C('path: whitespace between tokens is ignored', '  // TEXT [ @name = "Logo" ]\n'),
  // attributes
  C('attr: @id', '//*[@id="cta"]'),
  C('attr: @name', '//*[@name="Title"]'),
  C('attr: @type', '//*[@type="box"]'),
  C('attr: @text', '//TEXT[@text="Quiet light"]'),
  C('attr: starts-with @src', '//*[starts-with(@src, "scrim-asset:")]'),
  C('attr: @alt', '//IMAGE[@alt="A moth"]'),
  C('attr: @alt exists', '//*[@alt]'),
  C('width: > 500 (numbers)', '//*[@width > 500]'),
  C('width: = 120 matches "120px"', '//*[@width = 120]'),
  C('width: < 200 ("50%" is never numeric)', '//*[@width < 200]'),
  C('width: = "50%" as text', '//*[@width = "50%"]'),
  C('width: = "120px" as text', '//*[@width = "120px"]'),
  C('width: = "120" as text does not match "120px"', '//*[@width = "120"]'),
  C('width: = "320" as text matches the number 320', '//*[@width = "320"]'),
  C('width: != 5 is false for non-numeric and absent widths', '//*[@width != 5]'),
  C('width: != "5" as text, absent still false', '//*[@width != "5"]'),
  C('width: > -1 negative literal', '//*[@width > -1]'),
  C('height: = "fit-content"', '//*[@height = "fit-content"]'),
  C('height: range with and', '//*[@height >= 200 and @height <= 800]'),
  C('height: > 199.5 decimal literal', '//*[@height > 199.5]'),
  C('fill: text colour', '//*[@fill="#111111"]'),
  C('fill: backgroundColor', '//*[@fill="#1a2b3c"]'),
  C('fill: exists', '//*[@fill]'),
  C('background', '//*[@background="#ffffff"]'),
  C('color on text', '//TEXT[@color="transparent"]'),
  C('layout = flex', '//*[@layout="flex"]'),
  C('layout = grid', '//*[@layout="grid"]'),
  C('layout.display reads the same as @layout', '//*[@layout.display="grid"]'),
  C('direction = column', '//*[@direction="column"]'),
  C('direction = row (flex default)', '//FRAME[@direction="row"]'),
  C('direction exists', '//*[@direction]'),
  C('position = absolute', '//*[@position="absolute"]'),
  C('children = 3', '/FRAME[@children=3]'),
  C('children > 1', '//*[@children > 1]'),
  C('children = "3" as text', '//*[@children = "3"]'),
  C('children always exists', '//*[@children]'),
  C('contains on a number stringifies it', '//*[contains(@children, "3")]'),
  C('component name (master and instance)', '//*[@component="Pricing Card"]'),
  C('componentId on an instance', '//INSTANCE[@componentId="cmp-card"]'),
  C('component exists', '//*[@component]'),
  C('style.fontSize >= 20', '//TEXT[@style.fontSize >= 20]'),
  C('style.fontSize exists', '//TEXT[@style.fontSize]'),
  C('style.fontSize = "56" as text', '//*[@style.fontSize = "56"]'),
  C('layout.gap = 32', '//*[@layout.gap = 32]'),
  C('style.width reads width', '//*[@style.width > 300]'),
  C('layout.width reads the same merged value', '//*[@layout.width > 300]'),
  C('style.flexDirection reads layout', '//*[@style.flexDirection="column"]'),
  { name: 'declared: style.position found by @position', doc: 'query-stray', query: '//*[@position="absolute"]' },
  { name: 'declared: style.display flex gives direction row', doc: 'query-stray', query: '//BOX[@direction="row"]' },
  { name: 'declared: @layout.position = @style.position', doc: 'query-stray', query: '//*[@layout.position="absolute"]' },
  { name: 'declared: @style.position', doc: 'query-stray', query: '//*[@style.position="absolute"]' },
  // lint (caller-supplied)
  C('lint: exists', '//*[@lint]', { lint: LINT }),
  C('lint: contains', '//*[contains(@lint, "oversized")]', { lint: LINT }),
  C('lint: lintCount > 1', '//*[@lintCount > 1]', { lint: LINT }),
  C('lint: sorted, comma-joined', '//*[@lint = "flat-type-hierarchy,oversized-h1"]', { lint: LINT }),
  C('lint: not(@lint)', '/FRAME/FRAME/TEXT[not(@lint)]', { lint: LINT }),
  C('lint: lintCount = 0', '/FRAME/*[@lintCount = 0]', { lint: LINT }),
  C('lint: duplicate ids are deduped in @lint', '//*[@lint = "flat-type-hierarchy"]', { lint: LINT_DUP }),
  C('lint: but counted twice in @lintCount', '//*[@lintCount = 2]', { lint: LINT_DUP }),
  C('lint: without lint results is an error', '//*[@lint]'),
  C('lint: lintCount without lint results is an error', '//*[not(@lintCount > 0)]'),
  // operators and functions
  C('!= as text', '//BOX[@name != "Shape"]'),
  C('!= on absent attribute is false', '//*[@text != "x"]'),
  C('!= position', '//*[@position != "absolute"]'),
  C('< numeric', '//TEXT[@style.fontSize < 20]'),
  C('<= numeric', '//TEXT[@style.fontSize <= 20]'),
  C('> numeric', '//TEXT[@style.fontSize > 20]'),
  C('literal on the left flips: 20 < @x', '//TEXT[20 < @style.fontSize]'),
  C('literal on the left: 56 = @x', '//TEXT[56 = @style.fontSize]'),
  C('literal on the left: string', '//*["Logo" = @name]'),
  C('literal on the left flips: 600 >= @width', '//*[600 >= @width]'),
  C('contains is case-sensitive (match)', '//*[contains(@name, "button")]'),
  C('contains is case-sensitive (no match)', '//*[contains(@name, "Button")]'),
  C('contains "" matches every present value', '//TEXT[contains(@name, "")]'),
  C('contains on @text', '//*[contains(@text, "moth")]'),
  C('contains % on width', '//*[contains(@width, "%")]'),
  C('starts-with', '//*[starts-with(@name, "He")]'),
  C('and binds tighter than or', '//TEXT[@name="Logo" or @name="Lede" and @text="nope"]'),
  C('parentheses', '//TEXT[(@name="Logo" or @name="Lede") and @text="Quiet light"]'),
  C('not(@layout)', '/FRAME/*[not(@layout)]'),
  C('not and not', '//TEXT[not(@style.fontSize) and not(@color)]'),
  C('not(not())', '//TEXT[not(not(@color))]'),
  C('several predicates chain', '//*[@layout][@children > 1][@name != "Page"]'),
  // positions
  C('position: //TEXT[1] per parent', '//TEXT[1]'),
  C('position: //TEXT[2] per parent', '//TEXT[2]'),
  C('position: /FRAME[2]', '/FRAME[2]'),
  C('position: /*[last()]', '/*[last()]'),
  C('position: //FRAME[@layout][2] filter then position', '//FRAME[@layout][2]'),
  C('position: out of range', '/FRAME[9]'),
  C('position: /FRAME[1]/FRAME[last()]', '/FRAME[1]/FRAME[last()]'),
  C('position: //*[1] first child of every parent', '//*[1]'),
  C('position then filter: //*[1][@type="text"]', '//*[1][@type="text"]'),
  C('filter then position: //*[@type="text"][1]', '//*[@type="text"][1]'),
  C('position: [1.0] is position 1', '/FRAME[1.0]'),
  C('position: [01] is position 1', '/FRAME[01]'),
  C('position: //*[@name="Title"][2] none (one per parent)', '//*[@name="Title"][2]'),
  C('position: [last()][1]', '//TEXT[last()][1]'),
  // edge document
  E('edge: " 240px " is numeric after trimming', '//*[@width = 240]'),
  E('edge: "-.5px" is numeric', '//*[@width < 0]'),
  E('edge: "240PX" is not numeric', '//*[@height = 240]'),
  E('edge: "240PX" as text', '//*[@height = "240PX"]'),
  E('edge: "1e2" is not numeric', '//*[@width = 100]'),
  E('edge: "+5" is not numeric', '//*[@height = 5]'),
  E('edge: number 0 is numeric and present', '//*[@width = 0]'),
  E('edge: number 0.5 = "0.5" as text', '//*[@style.opacity = "0.5"]'),
  E('edge: number 0.5 = 0.50 numerically', '//*[@style.opacity = 0.50]'),
  E('edge: empty-string style value is absent', '//*[@style.paddingTop]'),
  E('edge: inline-flex defaults direction to row', '//FRAME[@direction="row"]'),
  E('edge: row-reverse', '//*[@direction="row-reverse"]'),
  E('edge: style.position wins over layout.position', '//*[@position="relative"]'),
  E('edge: layout.position hidden by style', '//*[@position="absolute"]'),
  E('edge: style.display "" falls back to layout.display', '//BOX[@layout="flex"]'),
  E('edge: background "" falls back to backgroundColor', '//*[@fill="#ff0000"]'),
  E('edge: text fill is color, not background', '//TEXT[@fill]'),
  E('edge: text background still readable', '//TEXT[@background="#000000"]'),
  E('edge: empty name still exists', '//BOX[@name = ""]'),
  E('edge: empty text exists and equals ""', '//TEXT[@text = ""]'),
  E('edge: double quotes inside a single-quoted literal', `//*[contains(@name, '"quoted"')]`),
  E('edge: no escapes: a value with both quote kinds cannot be written', `//*[@name = 'it's "quoted"']`),
  E('edge: double-quote literal containing an apostrophe', `//*[contains(@name, "it's")]`),
  E('edge: unicode contains', '//*[contains(@name, "☕")]'),
  E('edge: nested master is COMPONENT', '//COMPONENT'),
  E('edge: nested instance', '//INSTANCE[@component="Button"]'),
  E('edge: //FRAME//TEXT from overlapping contexts, each once, document order', '//FRAME//TEXT'),
  E('edge: //FRAME/TEXT', '//FRAME/TEXT'),
  E('edge: //FRAME//FRAME', '//FRAME//FRAME'),
  E('edge: //FRAME[1]//TEXT[1]', '//FRAME[1]//TEXT[1]'),
  E('edge: /FRAME/*[last()]', '/FRAME/*[last()]'),
  E('edge: //*[@name="Deep"][last()]', '//*[@name="Deep"][last()]'),
  E('edge: zIndex number compared as text', '//*[@style.zIndex = "3"]'),
  E('edge: no lint function needed for @layout', '//*[@layout = "block"]'),
  // errors
  ...[
    '', '   ', 'FRAME', './TEXT', '//frame', '//text', '//Frame', '//GROUP', '//TEXT/..', '//child::TEXT', '//TEXT | //BOX',
    '//TEXT[@colour="red"]', '//TEXT[@style]', '//TEXT[@name = Title]', '//TEXT[@name = false]', '//TEXT[@visible = "false"]',
    '//TEXT[name]', '//TEXT[@name > "a"]', '//TEXT[@name = @text]', '//TEXT[count(*) > 1]', '//TEXT[last]', '//TEXT[0]', '//TEXT[1.5]',
    '//TEXT[-1]', '//TEXT[@name="x"', '//TEXT[contains(@name)]', '//TEXT[contains(@name, 3)]', '//TEXT[@name="x]', '//TEXT/@name',
    '//TEXT[@name and]', '//TEXT[$x]', '//TEXT#', '//TEXT[@colour]', '/', '//', '///TEXT', '//TEXT/', '//TEXT//', '//.', '//TEXT/.',
    '//TEXT.', '//TEXT[1]x', '//TEXT[@name="x"]]', '//contains(@name, "x")', '//TEXT[@style.font-size = 12]', '//TEXT[@style.]',
    '//TEXT[@layout.]', '//TEXT[@style.1x]', '//TEXT[@]', '//TEXT[@name =]', '//TEXT["a" < @name]', '//TEXT[10 < 20]', '//TEXT["x"]',
    '//TEXT[last() = 1]', '//TEXT[position() = 1]', '//TEXT[not @name]', '//TEXT[and @name]', '//TEXT[@name or]', '//TEXT[]',
    '//TEXT[(@name]', '//TEXT[starts-with(@name, "x", "y")]', '//TEXT[contains("x", @name)]', "//TEXT[@name = 'x' = 'y']",
    '//TEXT€', '//TEXT[@name=1e3]', '//TEXT[@width > 10px]', '//TEXT[@name="a" AND @name="b"]', '//TEXT[NOT(@name)]',
    '//TEXT[Contains(@name, "x")]', "//TEXT[@name='it''s']", '//ancestor::FRAME', '//TEXT[@name="x"] | //BOX', '//TEXT[$1]',
    '//TEXT[@name = "x" or]', '//TEXT[not()]', '//TEXT[contains(@name, "x"]', '//TEXT[@style.fontSize > "20"]', '//TEXT[@width >= "1"]',
    '//TEXT[last()', '@name', '//TEXT[@name ! "x"]', '//TEXT[@name == "x"]',
    '//TEXT[@"x"]', '//1', '//(', '//TEXT[@width > -x]', '//*[@layout.5]', '//TEXT[@name or and @id]', '//TEXT[contains(@name, "x") = 1]',
    '//TEXT[not(@name]', '//TEXT[@name="x"]/', '//BOX[@width >= 1 and]',
  ].map((query) => C(`error: ${JSON.stringify(query)}`, query)),
  C('error: 1001 characters', `//*[@name="${'x'.repeat(MAX_QUERY_LENGTH - 12)}"]`),
  C('utf16: error position counts UTF-16 code units (é is 1 unit)', '//*[@name="é"]#'),
  C('utf16: error position counts UTF-16 code units (an emoji is 2 units)', '//*[@name="😀"]#'),
  C('utf16: the length limit counts UTF-16 code units (507 chars, 1001 units)', `//*[@name="${'😀'.repeat(494)}"]`),
  C('limit: exactly 1000 characters is accepted', `//*[@name="${'x'.repeat(MAX_QUERY_LENGTH - 13)}"]`),
];

function runCore(c: CaseIn): { ids: string[] } | { error: string; at: number | null } {
  const doc = DOCS[c.doc]!;
  const lint = c.lint ? (id: string): string[] => c.lint![id] ?? [] : undefined;
  let src: { ids: string[] } | { error: string; at: number | null };
  try {
    src = { ids: queryNodes(doc, c.query, lint ? { lint } : {}) };
  } catch (e) {
    if (!(e instanceof QueryError)) throw e;
    src = { error: e.message, at: e.at === null ? null : e.at + 1 };
  }
  // cross-check the built dist gives the same answer
  let dist: unknown;
  try {
    dist = { ids: coreDist.queryNodes(doc, c.query, lint ? { lint } : {}) };
  } catch (e) {
    dist = { error: (e as Error).message, at: (e as { at: number | null }).at === null ? null : (e as { at: number }).at + 1 };
  }
  if (JSON.stringify(dist) !== JSON.stringify(src)) throw new Error(`dist and src disagree on ${c.name}`);
  return src;
}

const names = new Set<string>();
const outCases = cases.map((c) => {
  if (names.has(c.name)) throw new Error(`duplicate case name ${c.name}`);
  names.add(c.name);
  return { name: c.name, doc: c.doc, query: c.query, ...(c.lint ? { lint: c.lint } : {}), expect: runCore(c) };
});
// sanity: length cases
if (!('error' in outCases.find((c) => c.name === 'error: 1001 characters')!.expect)) throw new Error('length case');
if ('error' in outCases.find((c) => c.name === 'limit: exactly 1000 characters is accepted')!.expect) throw new Error('length case 2');
void parseQuery;

// ── tool cases (query_nodes over MCP, through the CLI's in-process session) ──

interface ToolCaseIn { name: string; doc: string; args: Record<string, unknown>; linterDependent?: boolean }
const toolCases: ToolCaseIn[] = [
  { name: 'tool: //TEXT on landing, document order, element and parentId', doc: 'landing', args: { query: '//TEXT' } },
  { name: 'tool: one node by id; unmeasured rect is null and stale', doc: 'landing', args: { query: '//*[@id="cta"]' } },
  { name: 'tool: COMPONENT element in results', doc: 'query', args: { query: '/*' } },
  { name: 'tool: no match', doc: 'landing', args: { query: '//INSTANCE' } },
  { name: 'tool: default limit 100, total counts all', doc: 'many', args: { query: '//TEXT' } },
  { name: 'tool: limit 5', doc: 'many', args: { query: '//TEXT', limit: 5 } },
  { name: 'tool: limit 1000 returns all 105', doc: 'many', args: { query: '//TEXT', limit: 1000 } },
  { name: 'tool: limit 105 exactly is not truncated', doc: 'many', args: { query: '//TEXT', limit: 105 } },
  { name: 'tool: limit 1 with //*', doc: 'many', args: { query: '//*', limit: 1 } },
  { name: 'tool: invalid query (..)', doc: 'landing', args: { query: '//TEXT/..' } },
  { name: 'tool: invalid query (//text)', doc: 'landing', args: { query: '//text' } },
  { name: 'tool: invalid query (empty)', doc: 'landing', args: { query: '' } },
  { name: 'tool: invalid query (too long)', doc: 'landing', args: { query: `//*[@name="${'x'.repeat(990)}"]` } },
  { name: 'tool: limit 0 rejected by the schema', doc: 'many', args: { query: '//TEXT', limit: 0 } },
  { name: 'tool: limit 1001 rejected by the schema', doc: 'many', args: { query: '//TEXT', limit: 1001 } },
  { name: 'tool: limit 2.5 rejected by the schema', doc: 'many', args: { query: '//TEXT', limit: 2.5 } },
  { name: 'tool: missing query rejected by the schema', doc: 'many', args: {} },
  { name: 'tool: @lintCount >= 0 matches every node (lint is always supplied over MCP)', doc: 'landing', args: { query: '//*[@lintCount >= 0]', limit: 1000 } },
  { name: 'tool: @lint from the real linter', doc: 'landing', args: { query: '//*[@lint]' }, linterDependent: true },
];

const outTool = [];
for (const t of toolCases) {
  const session = await openSession(DOCS[t.doc]!);
  try {
    const r = await session.call('query_nodes', t.args);
    let expect: unknown;
    if (r.isError) expect = { isError: true, text: r.text };
    else {
      const parsed = JSON.parse(r.text);
      if (r.text !== JSON.stringify(parsed, null, 2)) throw new Error('text is not 2-space JSON');
      expect = { isError: false, json: parsed };
    }
    outTool.push({ name: t.name, doc: t.doc, args: t.args, ...(t.linterDependent ? { linterDependent: true } : {}), expect });
  } finally {
    await session.close();
  }
}

// ── CLI cases ────────────────────────────────────────────────────────────────

const dir = mkdtempSync(join(tmpdir(), 'scrim-query-cli-'));
const files: Record<string, string> = {};
for (const d of ['query', 'landing', 'many']) {
  files[d] = join(dir, `${d}.scrim`);
  writeFileSync(files[d]!, JSON.stringify(DOCS[d]));
}
interface CliIn { name: string; doc: string; argv: string[] }
const cliCases: CliIn[] = [
  { name: 'cli: query, human', doc: 'query', argv: ['query', '<file>', '//TEXT'] },
  { name: 'cli: query, one match, human', doc: 'query', argv: ['query', '<file>', '//IMAGE'] },
  { name: 'cli: query, no match, human', doc: 'query', argv: ['query', '<file>', '//TEXT[@name="nope"]'] },
  { name: 'cli: query --limit, truncated, human', doc: 'query', argv: ['query', '<file>', '//*', '--limit', '2'] },
  { name: 'cli: query --limit=3 inline', doc: 'query', argv: ['query', '<file>', '//*', '--limit=3'] },
  { name: 'cli: query --json is the tool text', doc: 'query', argv: ['query', '<file>', '/*', '--json'] },
  { name: 'cli: find --type (lower case accepted)', doc: 'query', argv: ['find', '<file>', '--type', 'component'] },
  { name: 'cli: find --type --name', doc: 'query', argv: ['find', '<file>', '--type', 'text', '--name', 'Title'] },
  { name: 'cli: find --name only', doc: 'query', argv: ['find', '<file>', '--name', 'e'] },
  { name: 'cli: find --name with double quotes uses single-quoted literal', doc: 'query', argv: ['find', '<file>', '--name', 'CTA "primary"', '--json'] },
  { name: 'cli: find with neither flag', doc: 'query', argv: ['find', '<file>'] },
  { name: 'cli: find with neither flag, --json', doc: 'query', argv: ['find', '<file>', '--json'] },
  { name: 'cli: find --type GROUP', doc: 'query', argv: ['find', '<file>', '--type', 'GROUP'] },
  { name: 'cli: find --type * is refused', doc: 'query', argv: ['find', '<file>', '--type', '*'] },
  { name: 'cli: find --name with both quote kinds', doc: 'query', argv: ['find', '<file>', '--name', `a"b'c`] },
  { name: 'cli: --limit 0', doc: 'query', argv: ['query', '<file>', '//*', '--limit', '0'] },
  { name: 'cli: --limit 1001', doc: 'query', argv: ['query', '<file>', '//*', '--limit', '1001'] },
  { name: 'cli: --limit abc, --json', doc: 'query', argv: ['query', '<file>', '//*', '--limit', 'abc', '--json'] },
  { name: 'cli: --limit 5e0 is accepted (JS Number())', doc: 'many', argv: ['query', '<file>', '//TEXT', '--limit', '5e0'] },
  { name: 'cli: --limit without a value', doc: 'query', argv: ['query', '<file>', '//*', '--limit'] },
  { name: 'cli: query without the query', doc: 'query', argv: ['query', '<file>'] },
  { name: 'cli: query with an unknown flag', doc: 'query', argv: ['query', '<file>', '//*', '--type', 'TEXT'] },
  { name: 'cli: invalid query, human (stderr)', doc: 'query', argv: ['query', '<file>', '//TEXT/..'] },
  { name: 'cli: invalid query, --json', doc: 'query', argv: ['query', '<file>', '//TEXT/..', '--json'] },
  { name: 'cli: @lint works in the CLI (real linter)', doc: 'landing', argv: ['query', '<file>', '//*[@lintCount >= 0]', '--limit', '3'] },
];
const outCli = [];
for (const c of cliCases) {
  let stdout = '';
  let stderr = '';
  const argv = c.argv.map((a) => (a === '<file>' ? files[c.doc]! : a));
  const exit = await run(argv, { out: (t) => void (stdout += t), err: (t) => void (stderr += t), cwd: dir });
  const scrub = (s: string): string => s.split(files[c.doc]!).join('<file>');
  outCli.push({ name: c.name, doc: c.doc, argv: c.argv, expect: { exit, stdout: scrub(stdout), stderr: scrub(stderr) } });
}

// ── write ────────────────────────────────────────────────────────────────────

const out = {
  format: 'scrim-query-cases/1',
  about: 'Generated by running Scrim\'s implementation (packages/core/src/query, packages/mcp/src/queryTools.ts, packages/cli/src/cli.ts) on 2026-10-19. Schema: docs/transfer/cases/README-query-nodes.md.',
  documents: DOCS,
  cases: outCases,
  toolCases: outTool,
  cliCases: outCli,
};
mkdirSync('../../../docs/transfer/cases', { recursive: true });
writeFileSync(OUT, `${JSON.stringify(out, null, 2)}\n`);
console.log(`cases ${outCases.length}, tool ${outTool.length}, cli ${outCli.length}`);
