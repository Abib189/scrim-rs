/** Generates the fixture half of docs/transfer/cases/fig-import.json by RUNNING
 * Scrim's importer (packages/fig-import/dist) on the committed fixtures.
 * Run: node gen-fixtures.ts > out/fixtures.json  (from this directory). */

import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { crc32 } from 'node:zlib';
import { stageA } from '../../../packages/fig-import/src/__tests__/fixtures.ts';

const REPO = (process.env.SCRIM_REPO ?? new URL('../../../', import.meta.url).pathname.replace(/\/$/, ''));
const fi = await import(`${REPO}/packages/fig-import/dist/index.js`);
const core = await import(`${REPO}/packages/core/dist/index.js`);

const sha256 = (b: Uint8Array): string => createHash('sha256').update(b).digest('hex');
const u16 = (b: Uint8Array, i: number): number => b[i]! | (b[i + 1]! << 8);
const u32 = (b: Uint8Array, i: number): number => (b[i]! | (b[i + 1]! << 8) | (b[i + 2]! << 16) | (b[i + 3]! << 24)) >>> 0;

/** The image pipeline stand-in: every image accepted, src = its sha256 (as the real store would name it). */
const accept = async (_layer: string, bytes: Uint8Array) => ({ src: `scrim-asset:${sha256(bytes)}.png` });
const refuse = async () => ({ error: 'no images' });

/** ZIP facts (central directory), CRCs included. */
function zipFacts(file: Uint8Array) {
  let eocd = -1;
  for (let i = file.length - 22; i >= 0; i--) if (u32(file, i) === 0x06054b50) { eocd = i; break; }
  const count = u16(file, eocd + 10);
  let p = u32(file, eocd + 16);
  const out = [];
  for (let n = 0; n < count; n++) {
    const method = u16(file, p + 10);
    const crcCentral = u32(file, p + 16);
    const compSize = u32(file, p + 20);
    const size = u32(file, p + 24);
    const nameLen = u16(file, p + 28);
    const extraLen = u16(file, p + 30);
    const commentLen = u16(file, p + 32);
    const local = u32(file, p + 42);
    const name = new TextDecoder().decode(file.subarray(p + 46, p + 46 + nameLen));
    const crcLocal = u32(file, local + 14);
    const start = local + 30 + u16(file, local + 26) + u16(file, local + 28);
    const raw = file.subarray(start, start + compSize);
    const { inflateRawSync } = await_zlib;
    const data = method === 8 ? inflateRawSync(raw) : raw;
    out.push({ name, method, compressedSize: compSize, size, crc32Central: crcCentral, crc32Local: crcLocal, crc32Actual: crc32(data) >>> 0 });
    p += 46 + nameLen + extraLen + commentLen;
  }
  return out;
}
const await_zlib = await import('node:zlib');

/** Every node in tree order, with a stable path (names + sibling index), no ids. */
function dumpNodes(doc: any, bounds: Map<string, any>) {
  const out: any[] = [];
  const compName = (id: string) => doc.components?.[id]?.name ?? id;
  const walk = (id: string, path: string) => {
    const n = doc.nodes[id];
    const rec: any = { path, name: n.name, type: n.type };
    if (n.canvas) rec.canvas = n.canvas;
    if (Object.keys(n.layout).length) rec.layout = n.layout;
    if (Object.keys(n.style).length) rec.style = n.style;
    for (const k of ['text', 'src', 'alt', 'svg', 'overrides']) if (n[k] !== undefined) rec[k] = n[k];
    if (n.componentId) rec.instanceOf = compName(n.componentId);
    const masterOf = Object.values(doc.components ?? {}).find((c: any) => c.rootId === id) as any;
    if (masterOf) rec.componentMaster = masterOf.name;
    const extra = Object.keys(n).filter((k) => !['id', 'type', 'name', 'parentId', 'childIds', 'layout', 'style', 'canvas', 'text', 'src', 'alt', 'svg', 'overrides', 'componentId'].includes(k));
    for (const k of extra) rec[k] = n[k];
    const b = bounds.get(id);
    if (b) rec.figmaBox = b;
    if (n.childIds.length) rec.children = n.childIds.map((c: string) => doc.nodes[c].name);
    out.push(rec);
    n.childIds.forEach((c: string, i: number) => walk(c, `${path}/${i}:${doc.nodes[c].name}`));
  };
  doc.rootIds.forEach((id: string, i: number) => walk(id, `${i}:${doc.nodes[id].name}`));
  return out;
}

const FIXTURES: Array<{ file: string; pipeline: 'accept' | 'refuse'; test: string }> = [
  { file: 'stage-a.fig', pipeline: 'accept', test: 'stageA.test.ts (okImage)' },
  { file: 'stage-b.fig', pipeline: 'accept', test: 'stageB.test.ts (accepts every image)' },
  { file: 'stage-c.fig', pipeline: 'refuse', test: "stageC.test.ts ({ error: 'no images here' }; the fixture has no images)" },
  { file: 'stage-d.fig', pipeline: 'refuse', test: "stageD.test.ts ({ error: 'none' }; the one image paint has no bytes)" },
  { file: 'lines.fig', pipeline: 'refuse', test: "lines.test.ts ({ error: 'no images' }; no images)" },
];

const results: any[] = [];
for (const f of FIXTURES) {
  const bytes = new Uint8Array(readFileSync(`${REPO}/tests/fixtures/fig/${f.file}`));
  const isZip = u32(bytes, 0) === 0x04034b50;
  const fig = fi.readFig(bytes);
  const canvas = isZip ? fi.readArchive(bytes).canvas : bytes;
  const chunks = fi.readChunks(canvas);
  const zstd = (c: Uint8Array) => c[0] === 0x28 && c[1] === 0xb5 && c[2] === 0x2f && c[3] === 0xfd;
  const schemaLen = u32(canvas, 12);
  const dataChunk = canvas.subarray(16 + schemaLen + 4);
  const schema = fi.decodeSchema(chunks.schema);
  const base = core.createDocument({ id: 'd', name: 'Imported' });
  const r = await fi.importFig(bytes, base, { fileName: f.file, importImage: f.pipeline === 'accept' ? accept : refuse });
  const applied = core.applyPatch(base, r.patch);
  const doc = applied.doc;
  const analysis = await fi.analyzeFig(bytes, f.file);
  const strip = (x: any) => JSON.stringify(x);
  const typeCounts: Record<string, number> = {};
  for (const n of fig.nodeChanges) typeCounts[n.type] = (typeCounts[n.type] ?? 0) + 1;
  results.push({
    file: `tests/fixtures/fig/${f.file}`,
    bytes: bytes.length,
    sha256: sha256(bytes),
    container: isZip ? 'zip' : 'fig-kiwi',
    zipEntries: isZip ? zipFacts(bytes) : undefined,
    figKiwiVersion: fig.version,
    schemaChunk: { compressed: schemaLen, decompressed: chunks.schema.length, codec: zstd(canvas.subarray(16)) ? 'zstd' : 'deflate-raw', definitions: schema.definitions.length },
    dataChunk: { compressed: u32(canvas, 16 + schemaLen), decompressed: chunks.data.length, codec: zstd(dataChunk) ? 'zstd' : 'deflate-raw' },
    nodeRecords: fig.nodeChanges.length,
    recordTypes: typeCounts,
    blobs: fig.blobs.length,
    zipImages: fig.images.size,
    imagePipeline: f.pipeline === 'accept' ? 'accepts every image: { src: "scrim-asset:<sha256 of bytes>.png" }' : 'refuses every image: { error: "no images" }',
    scrimTest: f.test,
    patchOps: r.patch.length,
    patchOpKinds: [...new Set(r.patch.map((o: any) => o.op ?? o.type))],
    validateDocument: core.validateDocument(doc),
    analyzeFigReportEqualsImportReport: strip({ ...analysis, images: undefined }) === strip({ ...r.report, images: undefined }),
    report: r.report,
    rootNames: doc.rootIds.map((id: string) => doc.nodes[id].name),
    components: Object.values(doc.components ?? {}).map((c: any) => c.name),
    tokens: doc.tokens,
    nodes: dumpNodes(doc, r.bounds),
  });
}

// stage A variants the tests use
const bare = stageA().bytes;
const zipped = stageA({ zipped: true }).bytes;
const a = fi.readFig(bare);
const b = fi.readFig(zipped);
const variants = {
  stageA_bare_vs_zipped: {
    bareBytes: bare.length,
    bareSha256: sha256(bare),
    zippedBytes: zipped.length,
    nodeRecordsBare: a.nodeChanges.length,
    sameRecordNames: JSON.stringify(a.nodeChanges.map((n: any) => n.name)) === JSON.stringify(b.nodeChanges.map((n: any) => n.name)),
    blobs: a.blobs.length,
    reportBare: (await fi.analyzeFig(bare, 'stage-a.fig')).summary,
  },
};

console.log(JSON.stringify({ fixtures: results, variants }, null, 1));
