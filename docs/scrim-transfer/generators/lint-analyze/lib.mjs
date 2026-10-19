// Shared harness for the transfer-kit case generators. Runs Scrim's REAL
// code (the built packages/*/dist) through the MCP server in-process, over an
// in-memory transport — the same path the `scrim` CLI uses — so recorded
// outputs are exactly what the tools answer.
const R = '../../../../packages';
export const core = await import(R + '/core/dist/index.js');
export const engine = await import(R + '/engine/dist/index.js');
const mcp = await import(R + '/mcp/dist/index.js');
const { Client } = await import(R + '/mcp/node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js');
const { InMemoryTransport } = await import(R + '/mcp/node_modules/@modelcontextprotocol/sdk/dist/esm/inMemory.js');

/** Build a ScrimDoc from a nested spec. Every node gets explicit
 * id/type/name/parentId/childIds/layout/style (nothing injected). */
export function mk({ id = 'doc', name = 'Case', roots, tokens, lint }) {
  const doc = {
    schemaVersion: 1,
    id,
    name,
    rootIds: [],
    nodes: {},
    components: {},
    tokens: { colors: {}, spacing: {}, typography: {}, ...(tokens ?? {}) },
  };
  if (lint) doc.lint = lint;
  const add = (spec, parentId) => {
    const { children = [], ...rest } = spec;
    const node = { id: rest.id, type: rest.type, name: rest.name ?? rest.id, parentId, childIds: [], layout: rest.layout ?? {}, style: rest.style ?? {} };
    for (const k of ['canvas', 'text', 'src', 'alt', 'states', 'media', 'svg']) if (rest[k] !== undefined) node[k] = rest[k];
    if (node.type === 'text' && node.text === undefined) node.text = '';
    doc.nodes[node.id] = node;
    if (parentId === null) doc.rootIds.push(node.id);
    else doc.nodes[parentId].childIds.push(node.id);
    for (const c of children) add(c, node.id);
  };
  for (const r of roots) add(r, null);
  const issues = core.validateDocument(doc);
  if (issues.length) throw new Error(`invalid doc ${id}: ${JSON.stringify(issues)}`);
  return doc;
}

/** Run a list of tool calls against a fresh server holding `doc`, with
 * optional measured rects recorded at the current revision. */
export async function run(doc, steps, rects) {
  const store = new mcp.DocStore(structuredClone(doc));
  if (rects) store.recordMeasured(Object.entries(rects).map(([id, rect]) => ({ id, rect, rev: store.revision })));
  const server = mcp.createScrimServer(store, { cwd: '/nonexistent-transfer-kit', screenshot: null, appearance: null });
  const [a, b] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: 'transfer-kit', version: '0' });
  await Promise.all([server.connect(b), client.connect(a)]);
  const out = [];
  try {
    for (const s of steps) {
      const res = await client.callTool({ name: s.tool, arguments: s.args ?? {} });
      const text = res.content[0]?.text ?? '';
      const isError = res.isError === true;
      let result;
      try {
        result = JSON.parse(text);
      } catch {
        result = text;
      }
      out.push({ tool: s.tool, args: s.args ?? {}, expect: { isError, result } });
    }
    return { steps: out, docAfter: store.doc };
  } finally {
    await client.close();
    await server.close();
  }
}

export const RECT = (x, y, width, height) => ({ x, y, width, height });
