/** Generates the security half of docs/transfer/cases/fig-import.json by
 * RUNNING Scrim's importer (packages/fig-import/dist) on hostile inputs built
 * here. Every expectError is the thrown FigError's code and message, verbatim.
 * Run: node gen-security.ts > out/security.json  (from this directory). */

import { createHash } from 'node:crypto';
import { deflateRawSync, zstdCompressSync } from 'node:zlib';
import { extendSchema, field, figKiwi, SCHEMA, zip } from '../../../packages/fig-import/src/__tests__/figEncoder.ts';
import { stageA } from '../../../packages/fig-import/src/__tests__/fixtures.ts';

const REPO = (process.env.SCRIM_REPO ?? new URL('../../../', import.meta.url).pathname.replace(/\/$/, ''));
const fi = await import(`${REPO}/packages/fig-import/dist/index.js`);
const core = await import(`${REPO}/packages/core/dist/index.js`);
const D = fi.DEFAULT_LIMITS;
const MiB = 1024 * 1024;
const sha256 = (b: Uint8Array): string => createHash('sha256').update(b).digest('hex');
const B64_MAX = 8192; // inputs up to this size are embedded base64

type Call = 'readArchive' | 'readFig' | 'importFig' | 'decodeSchema' | 'decodeMessage';
interface Case {
  name: string;
  group: string;
  scrimTest: string | null;
  inputDescription: string;
  howToBuild: string;
  call: Call;
  limits?: Record<string, number>;
  build: () => Uint8Array | { schema: Uint8Array; data?: Uint8Array };
}

const enc = (s: string): Uint8Array => new TextEncoder().encode(s);
const u32le = (n: number): number[] => [n & 255, (n >>> 8) & 255, (n >>> 16) & 255, (n >>> 24) & 255];
const varUint = (v: number): number[] => {
  const out: number[] = [];
  do {
    let b = v & 127;
    v >>>= 7;
    if (v) b |= 128;
    out.push(b);
  } while (v);
  return out;
};
const cstr = (s: string): number[] => [...enc(s), 0];
/** fig-kiwi container from raw (uncompressed) schema and data bytes, both deflate-raw. */
const container = (schema: number[] | Uint8Array, data: number[] | Uint8Array): Uint8Array => {
  const s = deflateRawSync(Uint8Array.from(schema));
  const d = deflateRawSync(Uint8Array.from(data));
  return Uint8Array.from([...enc('fig-kiwi'), ...u32le(15), ...u32le(s.length), ...s, ...u32le(d.length), ...d]);
};
/** A binary Kiwi schema: [name, kind(0 enum,1 struct,2 message), fields[name, type, isArray, value]]. */
const schemaBytes = (defs: Array<[string, number, Array<[string, number, boolean, number]>]>, count = defs.length): number[] => {
  const out = [...varUint(count)];
  for (const [name, kind, fields] of defs) {
    out.push(...cstr(name), kind, ...varUint(fields.length));
    for (const [fname, type, arr, value] of fields) out.push(...cstr(fname), ...varUint(((type << 1) ^ (type >> 31)) >>> 0), arr ? 1 : 0, ...varUint(value));
  }
  return out;
};
// Kiwi builtin type ids: bool -1, byte -2, int -3, uint -4, float -5, string -6, int64 -7, uint64 -8
/** A minimal schema: Message { nodeChanges: NodeChange[] = 1 }, NodeChange { name: string = 1, bytes: byte[] = 2, kids: uint[] = 3 }. */
const MINI = schemaBytes([
  ['NodeChange', 2, [['name', -6, false, 1], ['bytes', -2, true, 2], ['kids', -4, true, 3]]],
  ['Message', 2, [['nodeChanges', 0, true, 1]]],
]);

/** Patch a little-endian u16/u32 inside a built zip. */
const patch = (b: Uint8Array, at: number, value: number, width: 2 | 4): Uint8Array => {
  const out = Uint8Array.from(b);
  for (let i = 0; i < width; i++) out[at + i] = (value >>> (8 * i)) & 255;
  return out;
};
const eocdOf = (b: Uint8Array): number => {
  for (let i = b.length - 22; i >= 0; i--) if (b[i] === 0x50 && b[i + 1] === 0x4b && b[i + 2] === 5 && b[i + 3] === 6) return i;
  return -1;
};
const cdOf = (b: Uint8Array): number => {
  const e = eocdOf(b);
  return b[e + 16]! | (b[e + 17]! << 8) | (b[e + 18]! << 16) | (b[e + 19]! << 24);
};

/** The test encoder's zip() layout byte for byte (CRC 0), built with Buffers so large inputs don't overflow the stack. */
const zipBuf = (entries: Array<{ name: string; data: Uint8Array; deflate?: boolean; declaredSize?: number }>): Uint8Array => {
  const le = (v: number, w: number) => { const b = Buffer.alloc(w); w === 2 ? b.writeUInt16LE(v & 0xffff) : b.writeUInt32LE(v >>> 0); return b; };
  const parts: Buffer[] = [];
  const central: Buffer[] = [];
  let offset = 0;
  for (const e of entries) {
    const n = Buffer.from(e.name);
    const body = e.deflate ? deflateRawSync(e.data) : Buffer.from(e.data);
    const method = e.deflate ? 8 : 0;
    const size = e.declaredSize ?? e.data.length;
    const local = Buffer.concat([le(0x04034b50, 4), Buffer.from([20, 0, 0, 0, method, 0, 0, 0, 0, 0]), le(0, 4), le(body.length, 4), le(size, 4), le(n.length, 2), le(0, 2), n]);
    central.push(Buffer.concat([le(0x02014b50, 4), Buffer.from([20, 0, 20, 0, 0, 0, method, 0, 0, 0, 0, 0]), le(0, 4), le(body.length, 4), le(size, 4), le(n.length, 2), Buffer.alloc(8), le(0, 4), le(offset, 4), n]));
    parts.push(local, body);
    offset += local.length + body.length;
  }
  const cd = Buffer.concat(central);
  const eocd = Buffer.concat([le(0x06054b50, 4), Buffer.alloc(4), le(entries.length, 2), le(entries.length, 2), le(cd.length, 4), le(offset, 4), Buffer.alloc(2)]);
  return new Uint8Array(Buffer.concat([...parts, cd, eocd]));
};

const good = (): Uint8Array => stageA().bytes; // bare fig-kiwi, 2564 bytes
const goodZip = (): Uint8Array => zip([{ name: 'canvas.fig', data: good() }]);
const schemaLenOf = (g: Uint8Array): number => g[12]! | (g[13]! << 8) | (g[14]! << 16) | (g[15]! << 24);

const cases: Case[] = [
  // ── size cap ──
  { name: 'file over the size cap (test limit)', group: 'size', scrimTest: 'security.test.ts › a file over the size cap is refused from its length', inputDescription: 'stage A (bare fig-kiwi, 2564 bytes) with limits.fileBytes = 100', howToBuild: 'stageA().bytes; call readFig(bytes, { ...DEFAULT_LIMITS, fileBytes: 100 })', call: 'readFig', limits: { fileBytes: 100 }, build: good },
  { name: 'file over the 200 MB cap (default limits)', group: 'size', scrimTest: null, inputDescription: '209,715,201 zero bytes (200 MiB + 1): refused from the length alone, before any parsing', howToBuild: 'new Uint8Array(200 * 1024 * 1024 + 1); readFig with DEFAULT_LIMITS', call: 'readFig', build: () => new Uint8Array(200 * MiB + 1) },
  // ── zip bombs ──
  { name: 'zip bomb, honest header (test limit)', group: 'zip', scrimTest: 'security.test.ts › a zip bomb stops at the entry cap (honest header) …', inputDescription: 'ZIP with canvas.fig = 4 MiB of zeros, deflated, declared size 4 MiB; limits.entryBytes = 1 MiB', howToBuild: "zip([{ name: 'canvas.fig', data: new Uint8Array(4 * 1024 * 1024), deflate: true }]); readArchive(file, { ...DEFAULT_LIMITS, entryBytes: 1024 * 1024 })", call: 'readArchive', limits: { entryBytes: MiB }, build: () => zip([{ name: 'canvas.fig', data: new Uint8Array(4 * MiB), deflate: true }]) },
  { name: 'zip bomb, honest header (default limits)', group: 'zip', scrimTest: null, inputDescription: 'ZIP whose canvas.fig entry DECLARES 300 MiB uncompressed (the data is one deflated byte): refused from the declared size before inflating', howToBuild: "zip([{ name: 'canvas.fig', data: new Uint8Array(1), deflate: true, declaredSize: 300 * 1024 * 1024 }])", call: 'readArchive', build: () => zip([{ name: 'canvas.fig', data: new Uint8Array(1), deflate: true, declaredSize: 300 * MiB }]) },
  { name: 'zip bomb, lying header (test limit)', group: 'zip', scrimTest: 'security.test.ts › … and at the cap again when the header lies', inputDescription: 'ZIP with canvas.fig = 4 MiB of zeros, deflated, but declared size 1000; limits.entryBytes = 1 MiB: inflate stops at the output cap', howToBuild: "zip([{ name: 'canvas.fig', data: new Uint8Array(4 * 1024 * 1024), deflate: true, declaredSize: 1000 }]); readArchive with entryBytes 1 MiB", call: 'readArchive', limits: { entryBytes: MiB }, build: () => zip([{ name: 'canvas.fig', data: new Uint8Array(4 * MiB), deflate: true, declaredSize: 1000 }]) },
  { name: 'zip bomb, lying header (default limits)', group: 'zip', scrimTest: null, inputDescription: 'ZIP with canvas.fig = 257 MiB of zeros, deflated (about 256 KB), declared size 1000: inflate stops at the 256 MiB output cap', howToBuild: "zip([{ name: 'canvas.fig', data: new Uint8Array(257 * 1024 * 1024), deflate: true, declaredSize: 1000 }]) — deflate-raw of zeros at Node's default level", call: 'readArchive', build: () => zipBuf([{ name: 'canvas.fig', data: new Uint8Array(257 * MiB), deflate: true, declaredSize: 1000 }]) },
  { name: 'total decompressed size over the cap (test limit)', group: 'zip', scrimTest: 'security.test.ts › the total across entries is capped', inputDescription: 'ZIP: canvas.fig (stage A, stored) + 5 image entries images/<40 hex> of 600 KiB zeros each, deflated; limits.totalBytes = 2 MiB. The 4th image hits the remaining budget', howToBuild: "zip([{ name: 'canvas.fig', data: stageA().bytes }, ...[0..4].map(i => ({ name: `images/${String(i).repeat(40)}`, data: new Uint8Array(600 * 1024), deflate: true }))])", call: 'readArchive', limits: { totalBytes: 2 * MiB }, build: () => zip([{ name: 'canvas.fig', data: good() }, ...Array.from({ length: 5 }, (_, i) => ({ name: `images/${String(i).repeat(40)}`, data: new Uint8Array(600 * 1024), deflate: true }))]) },
  { name: 'too many entries (test limit)', group: 'zip', scrimTest: 'security.test.ts › too many entries', inputDescription: '12 stored 1-byte entries x0..x11; limits.entries = 10', howToBuild: 'zip(Array.from({ length: 12 }, (_, i) => ({ name: `x${i}`, data: new Uint8Array(1) })))', call: 'readArchive', limits: { entries: 10 }, build: () => zip(Array.from({ length: 12 }, (_, i) => ({ name: `x${i}`, data: new Uint8Array(1) }))) },
  { name: 'too many entries (default limits)', group: 'zip', scrimTest: null, inputDescription: '5001 stored 1-byte entries x0..x5000 (the count is read from the end-of-central-directory record)', howToBuild: 'zip(Array.from({ length: 5001 }, (_, i) => ({ name: `x${i}`, data: new Uint8Array(1) })))', call: 'readArchive', build: () => zipBuf(Array.from({ length: 5001 }, (_, i) => ({ name: `x${i}`, data: new Uint8Array(1) }))) },
  // ── entry names ──
  ...['../evil.fig', '/etc/passwd', 'images/../../x', 'images\\..\\x', 'C:/x', 'a/./b', 'nul\0byte'].map((n): Case => ({ name: `unsafe entry path ${JSON.stringify(n)}`, group: 'entry-name', scrimTest: `security.test.ts › an unsafe entry path is refused: ${n}`, inputDescription: `ZIP: a stored 1-byte entry named ${JSON.stringify(n)}, then canvas.fig (stage A). Every name is checked, wanted or not, before any entry is read`, howToBuild: `zip([{ name: ${JSON.stringify(n)}, data: new Uint8Array(1) }, { name: 'canvas.fig', data: stageA().bytes }])`, call: 'readArchive', build: () => zip([{ name: n, data: new Uint8Array(1) }, { name: 'canvas.fig', data: good() }]) })),
  { name: 'entry name over 512 characters', group: 'entry-name', scrimTest: null, inputDescription: "ZIP: an entry named 'a' × 513, then canvas.fig", howToBuild: "zip([{ name: 'a'.repeat(513), data: new Uint8Array(1) }, { name: 'canvas.fig', data: stageA().bytes }])", call: 'readArchive', build: () => zip([{ name: 'a'.repeat(513), data: new Uint8Array(1) }, { name: 'canvas.fig', data: good() }]) },
  { name: 'empty entry name', group: 'entry-name', scrimTest: null, inputDescription: "ZIP: an entry with an empty name, then canvas.fig", howToBuild: "zip([{ name: '', data: new Uint8Array(1) }, { name: 'canvas.fig', data: stageA().bytes }])", call: 'readArchive', build: () => zip([{ name: '', data: new Uint8Array(1) }, { name: 'canvas.fig', data: good() }]) },
  // ── zip structure ──
  { name: 'encrypted entry', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP with canvas.fig (stage A, stored) whose central-directory general-purpose flag has bit 0 (encrypted) set', howToBuild: 'goodZip = zip([{ name: "canvas.fig", data: stageA().bytes }]); set the u16 at centralDirectoryOffset + 8 to 1', call: 'readArchive', build: () => { const z = goodZip(); return patch(z, cdOf(z) + 8, 1, 2); } },
  { name: 'ZIP64 archive', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP whose end-of-central-directory entry count is 0xFFFF (the ZIP64 marker)', howToBuild: 'goodZip; set the u16 at eocd + 10 to 0xFFFF', call: 'readArchive', build: () => { const z = goodZip(); return patch(z, eocdOf(z) + 10, 0xffff, 2); } },
  { name: 'unsupported compression method', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP with canvas.fig whose central-directory compression method is 12 (bzip2)', howToBuild: 'goodZip; set the u16 at centralDirectoryOffset + 10 to 12', call: 'readArchive', build: () => { const z = goodZip(); return patch(z, cdOf(z) + 10, 12, 2); } },
  { name: 'unsupported method on an ignored entry', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP: meta.json with method 12, then canvas.fig (stage A). Entries other than canvas.fig / canvas / images/<sha1> are never inflated, so this is ACCEPTED', howToBuild: "zip([{ name: 'meta.json', data: enc('{}') }, { name: 'canvas.fig', data: stageA().bytes }]); set the first central entry's method (cd + 10) to 12", call: 'readArchive', build: () => { const z = zip([{ name: 'meta.json', data: enc('{}') }, { name: 'canvas.fig', data: good() }]); return patch(z, cdOf(z) + 10, 12, 2); } },
  { name: 'no central directory', group: 'zip-structure', scrimTest: null, inputDescription: 'a local-file-header signature PK\\x03\\x04 followed by 60 zero bytes: no end-of-central-directory record', howToBuild: '[0x50,0x4b,0x03,0x04, ...60 zeros]', call: 'readArchive', build: () => Uint8Array.from([0x50, 0x4b, 3, 4, ...new Array(60).fill(0)]) },
  { name: 'truncated archive', group: 'zip-structure', scrimTest: "security.test.ts › corrupt Kiwi data, … a truncated archive …", inputDescription: 'the first 100 bytes of a ZIP holding canvas.fig (stage A, stored)', howToBuild: "zip([{ name: 'canvas.fig', data: stageA().bytes }]).subarray(0, 100)", call: 'readArchive', build: () => goodZip().subarray(0, 100) },
  { name: 'central directory runs past its end', group: 'zip-structure', scrimTest: null, inputDescription: 'goodZip with the EOCD central-directory size increased by 1000', howToBuild: 'goodZip; add 1000 to the u32 at eocd + 12', call: 'readArchive', build: () => { const z = goodZip(); const e = eocdOf(z); const size = z[e + 12]! | (z[e + 13]! << 8); return patch(z, e + 12, size + 1000, 4); } },
  { name: 'damaged central directory entry', group: 'zip-structure', scrimTest: null, inputDescription: "goodZip with the central-directory entry's signature zeroed", howToBuild: 'goodZip; set the u32 at centralDirectoryOffset to 0', call: 'readArchive', build: () => { const z = goodZip(); return patch(z, cdOf(z), 0, 4); } },
  { name: 'damaged local header', group: 'zip-structure', scrimTest: null, inputDescription: "goodZip with canvas.fig's local-header signature zeroed (the central entry points at it)", howToBuild: 'goodZip; set the u32 at offset 0 to 0 — but keep it a ZIP: the reader requires PK\\x03\\x04 at offset 0, so prepend a dummy stored entry first: zip([{ name: "meta.json", data: enc("{}") }, { name: "canvas.fig", data }]) and zero the second local header', call: 'readArchive', build: () => { const z = zip([{ name: 'meta.json', data: enc('{}') }, { name: 'canvas.fig', data: good() }]); const second = 30 + 'meta.json'.length + 2; return patch(z, second, 0, 4); } },
  { name: 'entry truncated (compressed size past the end of the file)', group: 'zip-structure', scrimTest: null, inputDescription: "goodZip with canvas.fig's central compressed size set to 10,000,000", howToBuild: 'goodZip; set the u32 at centralDirectoryOffset + 20 to 10000000', call: 'readArchive', build: () => { const z = goodZip(); return patch(z, cdOf(z) + 20, 10_000_000, 4); } },
  { name: 'stored entry whose size disagrees with its header', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP with canvas.fig (stage A, 2564 bytes, stored) declaring an uncompressed size of 2000', howToBuild: "zip([{ name: 'canvas.fig', data: stageA().bytes, declaredSize: 2000 }])", call: 'readArchive', build: () => zip([{ name: 'canvas.fig', data: good(), declaredSize: 2000 }]) },
  { name: 'corrupt deflate entry', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP with canvas.fig, method 8 (deflate), whose data is 64 bytes of 0xFF', howToBuild: "zip([{ name: 'canvas.fig', data: new Uint8Array(64).fill(255) }]) then set the central method (cd + 10) to 8", call: 'readArchive', build: () => { const z = zip([{ name: 'canvas.fig', data: new Uint8Array(64).fill(255) }]); return patch(z, cdOf(z) + 10, 8, 2); } },
  { name: 'ZIP without canvas.fig', group: 'zip-structure', scrimTest: null, inputDescription: 'ZIP holding only meta.json', howToBuild: "zip([{ name: 'meta.json', data: enc('{}') }])", call: 'readArchive', build: () => zip([{ name: 'meta.json', data: enc('{}') }]) },
  { name: 'canvas.fig that is not fig-kiwi', group: 'zip-structure', scrimTest: null, inputDescription: "ZIP whose canvas.fig is the text 'hello, not kiwi data'", howToBuild: "zip([{ name: 'canvas.fig', data: enc('hello, not kiwi data') }])", call: 'readArchive', build: () => zip([{ name: 'canvas.fig', data: enc('hello, not kiwi data') }]) },
  { name: 'not a Figma file (SVG text)', group: 'container', scrimTest: "security.test.ts › … and not a Figma file at all", inputDescription: "the text '<svg onload=alert(1)>'", howToBuild: "new TextEncoder().encode('<svg onload=alert(1)>')", call: 'readFig', build: () => enc('<svg onload=alert(1)>') },
  { name: 'not a Figma file (deflated fig-kiwi)', group: 'container', scrimTest: "security.test.ts › … and not a Figma file at all", inputDescription: 'stage A bare fig-kiwi compressed with deflate-raw (no container magic at offset 0)', howToBuild: 'deflateRawSync(stageA().bytes)', call: 'readFig', build: () => deflateRawSync(good()) },
  { name: 'three bytes', group: 'container', scrimTest: 'security.test.ts › importFig throws before returning anything (the document stays untouched)', inputDescription: 'the bytes 01 02 03, through importFig: it rejects before any patch exists', howToBuild: 'new Uint8Array([1, 2, 3])', call: 'importFig', build: () => new Uint8Array([1, 2, 3]) },
  // ── fig-kiwi chunks ──
  { name: 'data chunk truncated', group: 'chunks', scrimTest: "security.test.ts › … the data chunk cut short (its length field still claims the full size)", inputDescription: 'stage A bare fig-kiwi with its last 10 bytes removed', howToBuild: 'stageA().bytes.subarray(0, length - 10)', call: 'readFig', build: () => { const g = good(); return g.subarray(0, g.length - 10); } },
  { name: 'only one chunk', group: 'chunks', scrimTest: null, inputDescription: 'fig-kiwi magic, version 15, then a single chunk (the schema) and nothing else', howToBuild: 'stageA().bytes.subarray(0, 16 + schemaChunkLength)', call: 'readFig', build: () => { const g = good(); return g.subarray(0, 16 + schemaLenOf(g)); } },
  { name: 'corrupt zstd data chunk', group: 'chunks', scrimTest: "security.test.ts › corrupt Kiwi data (the data chunk replaced by zstd of garbage)", inputDescription: "stage A's schema chunk kept; the data chunk replaced by zstd of 64 garbage bytes (i * 37 mod 256): it decompresses, then the Kiwi message decode fails", howToBuild: 'head = stageA().bytes.subarray(0, 16 + schemaLen); garbage = zstdCompressSync(Uint8Array.from({ length: 64 }, (_, i) => (i * 37) % 256)); [...head, u32le(garbage.length), ...garbage]', call: 'readFig', build: () => { const g = good(); const head = g.subarray(0, 16 + schemaLenOf(g)); const garbage = zstdCompressSync(Uint8Array.from({ length: 64 }, (_, i) => (i * 37) % 256)); return Uint8Array.from([...head, ...u32le(garbage.length), ...garbage]); } },
  { name: 'truncated zstd frame', group: 'chunks', scrimTest: "security.test.ts › a zstd frame cut short, with an honest length", inputDescription: "stage A's schema chunk kept; the data chunk = the first 12 bytes of zstd(5000 × 0x07), with an honest length. Node's zstd returns what it could decode; the Kiwi reader then runs out", howToBuild: 'cut = zstdCompressSync(new Uint8Array(5000).fill(7)).subarray(0, 12); [...head, cut.length, 0, 0, 0, ...cut]', call: 'readFig', build: () => { const g = good(); const head = g.subarray(0, 16 + schemaLenOf(g)); const cut = zstdCompressSync(new Uint8Array(5000).fill(7)).subarray(0, 12); return Uint8Array.from([...head, cut.length, 0, 0, 0, ...cut]); } },
  { name: 'schema chunk that is not deflate or zstd', group: 'chunks', scrimTest: null, inputDescription: 'fig-kiwi magic, version 15, chunk 0 = 16 bytes of 0xFF, chunk 1 = deflate of 1 byte', howToBuild: "[...'fig-kiwi', u32le(15), u32le(16), ...16×0xFF, u32le(n), ...deflateRawSync([0])]", call: 'readFig', build: () => { const d = deflateRawSync(new Uint8Array(1)); return Uint8Array.from([...enc('fig-kiwi'), ...u32le(15), ...u32le(16), ...new Array(16).fill(255), ...u32le(d.length), ...d]); } },
  { name: 'data chunk over the decompression cap (test-style limit)', group: 'chunks', scrimTest: null, inputDescription: "stage A's schema chunk kept; the data chunk = deflate-raw of 2 MiB of zeros; limits.entryBytes = 1 MiB (the default cap is 256 MiB)", howToBuild: '[...head, u32le(d.length), ...d] with d = deflateRawSync(new Uint8Array(2 * 1024 * 1024))', call: 'readFig', limits: { entryBytes: MiB }, build: () => { const g = good(); const head = g.subarray(0, 16 + schemaLenOf(g)); const d = deflateRawSync(new Uint8Array(2 * MiB)); return Uint8Array.from([...head, ...u32le(d.length), ...d]); } },
  // ── Kiwi schema ──
  { name: 'too many Kiwi definitions', group: 'kiwi-schema', scrimTest: null, inputDescription: 'a schema whose definition count is 4097 (the cap is 4096); data 1 byte', howToBuild: 'container(schema = varUint(4097), data = [0])', call: 'readFig', build: () => container(varUint(4097), [0]) },
  { name: 'too many fields in a definition', group: 'kiwi-schema', scrimTest: null, inputDescription: "one MESSAGE definition 'X' declaring 4097 fields (the cap is 4096)", howToBuild: "container([...varUint(1), ...cstr('X'), 2, ...varUint(4097)], [0])", call: 'readFig', build: () => container([...varUint(1), ...cstr('X'), 2, ...varUint(4097)], [0]) },
  { name: 'unknown definition kind', group: 'kiwi-schema', scrimTest: null, inputDescription: "one definition 'X' of kind 3 (only 0 enum, 1 struct, 2 message exist)", howToBuild: "container([...varUint(1), ...cstr('X'), 3, 0], [0])", call: 'readFig', build: () => container([...varUint(1), ...cstr('X'), 3, 0], [0]) },
  { name: 'field of unknown type', group: 'kiwi-schema', scrimTest: null, inputDescription: "Message { nodeChanges: type 99 } in a 1-definition schema (types must be -8..-1 or a definition index)", howToBuild: "container(schemaBytes([['Message', 2, [['nodeChanges', 99, true, 1]]]]), [0])", call: 'readFig', build: () => container(schemaBytes([['Message', 2, [['nodeChanges', 99, true, 1]]]]), [0]) },
  { name: 'schema without a Message definition', group: 'kiwi-schema', scrimTest: null, inputDescription: "a schema holding only NodeChange (the root must be named 'Message')", howToBuild: "container(schemaBytes([['NodeChange', 2, [['name', -6, false, 1]]]]), [0])", call: 'readFig', build: () => container(schemaBytes([['NodeChange', 2, [['name', -6, false, 1]]]]), [0]) },
  { name: 'unterminated string in the schema', group: 'kiwi-schema', scrimTest: null, inputDescription: "a schema 01 'Message' with no NUL terminator", howToBuild: "container([1, ...enc('Message')], [0])", call: 'readFig', build: () => container([1, ...enc('Message')], [0]) },
  // ── Kiwi message ──
  { name: 'nesting deeper than 64', group: 'kiwi-message', scrimTest: null, inputDescription: 'schema N { child: N = 1 }, Message { nodeChanges: N[] = 1 }; data: one NodeChange nested 70 levels deep through child', howToBuild: 'schemaBytes([[N, 2, [[child, 0, false, 1]]], [Message, 2, [[nodeChanges, 0, true, 1]]]]); data = [1, 1, then 70 × (1), then 71 × (0), 0] (field 1 = child, 0 = end)', call: 'readFig', build: () => { const s = schemaBytes([['N', 2, [['child', 0, false, 1]]], ['Message', 2, [['nodeChanges', 0, true, 1]]]]); const data = [1, 1, ...new Array(70).fill(1), ...new Array(71).fill(0), 0]; return container(s, data); } },
  { name: 'array longer than the bytes left', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; data: Message.nodeChanges with a declared length of 1,000,000 and 3 bytes after it', howToBuild: 'container(MINI, [1, ...varUint(1000000), 0, 0, 0])', call: 'readFig', build: () => container(MINI, [1, ...varUint(1_000_000), 0, 0, 0]) },
  { name: 'byte array past the end', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; one NodeChange whose bytes field claims 50 bytes with 2 left', howToBuild: 'container(MINI, [1, 1, 2, 50, 0, 0])', call: 'readFig', build: () => container(MINI, [1, 1, 2, 50, 0, 0]) },
  { name: 'unknown field id in a message', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; one NodeChange carrying field id 9 (NodeChange has 1..3)', howToBuild: 'container(MINI, [1, 1, 9, 0, 0])', call: 'readFig', build: () => container(MINI, [1, 1, 9, 0, 0]) },
  { name: 'malformed varint', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; data: field 1 then a varint of six 0xFF bytes (over 5 continuation bytes)', howToBuild: 'container(MINI, [1, 255, 255, 255, 255, 255, 255, 0])', call: 'readFig', build: () => container(MINI, [1, 255, 255, 255, 255, 255, 255, 0]) },
  { name: 'unterminated string in the message', group: 'kiwi-message', scrimTest: null, inputDescription: "MINI schema; one NodeChange whose name is 'abc' with no NUL", howToBuild: "container(MINI, [1, 1, 1, ...enc('abc')])", call: 'readFig', build: () => container(MINI, [1, 1, 1, ...enc('abc')]) },
  { name: 'data ends early', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; data: nodeChanges with 1 item whose first field id is 3 (kids, uint[]), then nothing: the array length read runs off the end', howToBuild: 'container(MINI, [1, 1, 3])', call: 'readFig', build: () => container(MINI, [1, 1, 3]) },
  { name: 'bytes after the message', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; data: an empty Message (0) followed by 3 extra bytes', howToBuild: 'container(MINI, [0, 1, 2, 3])', call: 'readFig', build: () => container(MINI, [0, 1, 2, 3]) },
  { name: 'message without nodeChanges', group: 'kiwi-message', scrimTest: null, inputDescription: 'MINI schema; data: an empty Message (just the terminator 0)', howToBuild: 'container(MINI, [0])', call: 'readFig', build: () => container(MINI, [0]) },
];

/** Cases that are not FigErrors: the importer's own recursion over the tree. */
const P = (sessionID: number, localID: number) => ({ sessionID, localID });
const doc = P(1, 1);
const page = P(1, 2);
const instanceCycle = (): Uint8Array =>
  figKiwi({
    nodeChanges: [
      { guid: doc, type: 'DOCUMENT', name: 'Document' },
      { guid: page, parentIndex: { guid: doc, position: 'a' }, type: 'CANVAS', name: 'Page' },
      { guid: P(1, 10), parentIndex: { guid: page, position: 'a' }, type: 'SYMBOL', name: 'Loop', size: { x: 10, y: 10 }, transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 } },
      { guid: P(1, 11), parentIndex: { guid: P(1, 10), position: 'a' }, type: 'INSTANCE', name: 'Self', size: { x: 10, y: 10 }, transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 }, symbolData: { symbolID: P(1, 10), symbolOverrides: [] } },
    ],
  });
const deepTree = (depth: number): Uint8Array => {
  const nodes: object[] = [{ guid: doc, type: 'DOCUMENT', name: 'Document' }, { guid: page, parentIndex: { guid: doc, position: 'a' }, type: 'CANVAS', name: 'Page' }];
  let parent = page;
  for (let i = 0; i < depth; i++) {
    const g = P(1, 100 + i);
    nodes.push({ guid: g, parentIndex: { guid: parent, position: 'a' }, type: 'FRAME', name: `F${i}`, size: { x: 10, y: 10 }, transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 } });
    parent = g;
  }
  return figKiwi({ nodeChanges: nodes });
};
const manyNodes = (n: number): Uint8Array => {
  const nodes: object[] = [{ guid: doc, type: 'DOCUMENT', name: 'Document' }, { guid: page, parentIndex: { guid: doc, position: 'a' }, type: 'CANVAS', name: 'Page' }, { guid: P(1, 3), parentIndex: { guid: page, position: 'a' }, type: 'FRAME', name: 'Board', size: { x: 100, y: 100 }, transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 } }];
  for (let i = 0; i < n; i++) nodes.push({ guid: P(2, i + 1), parentIndex: { guid: P(1, 3), position: `a${String(i).padStart(7, '0')}` }, type: 'RECTANGLE', name: `R${i}`, size: { x: 1, y: 1 }, transform: { m00: 1, m01: 0, m02: i % 100, m10: 0, m11: 1, m12: 0 } });
  return figKiwi({ nodeChanges: nodes });
};
/** Masters M0..M(levels-1) on one page; M(i) holds `fan` instances of M(i+1); the last holds one rectangle. Instances inside masters are expanded inline (detached copies), so the output grows as fan^levels. */
const nestedInstances = (levels: number, fan: number): Uint8Array => {
  const I = { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 };
  const nodes: object[] = [{ guid: P(1, 1), type: 'DOCUMENT', name: 'D' }, { guid: P(1, 2), parentIndex: { guid: P(1, 1), position: 'a' }, type: 'CANVAS', name: 'P' }];
  let id = 1000;
  for (let i = 0; i < levels; i++) nodes.push({ guid: P(2, i), parentIndex: { guid: P(1, 2), position: 'a' + String(i).padStart(3, '0') }, type: 'SYMBOL', name: `M${i}`, size: { x: 10, y: 10 }, transform: { ...I, m02: i * 20 } });
  for (let i = 0; i < levels - 1; i++) for (let k = 0; k < fan; k++) nodes.push({ guid: P(3, id++), parentIndex: { guid: P(2, i), position: 'b' + k }, type: 'INSTANCE', name: `I${i}.${k}`, size: { x: 10, y: 10 }, transform: I, symbolData: { symbolID: P(2, i + 1), symbolOverrides: [] } });
  nodes.push({ guid: P(3, id++), parentIndex: { guid: P(2, levels - 1), position: 'a' }, type: 'RECTANGLE', name: 'Leaf', size: { x: 1, y: 1 }, transform: I });
  return figKiwi({ nodeChanges: nodes });
};
/** A schema field literally named __proto__ (objects are prototype-less). */
const protoField = (): Uint8Array => figKiwi(
  { nodeChanges: [{ guid: doc, type: 'DOCUMENT', name: 'Document', ['__proto__']: 1 } as object] },
  'zstd',
  extendSchema([], [field('__proto__', 'uint')]),
);

async function run(c: Case): Promise<unknown> {
  const limits = c.limits ? { ...D, ...c.limits } : D;
  const input = c.build() as Uint8Array;
  const t0 = performance.now();
  try {
    if (c.call === 'readArchive') fi.readArchive(input, limits);
    else if (c.call === 'readFig') fi.readFig(input, limits);
    else await fi.importFig(input, core.createDocument({ id: 'd', name: 'D' }), { fileName: 'x.fig', importImage: async () => ({ src: 'scrim-asset:x.png' }), limits });
    return { input, outcome: { accepted: true }, ms: Math.round(performance.now() - t0) };
  } catch (e) {
    const err = e as { name: string; code?: string; message: string };
    return { input, outcome: { errorClass: err.name, code: err.code, message: err.message }, ms: Math.round(performance.now() - t0) };
  }
}

const out: unknown[] = [];
for (const c of cases) {
  const r = (await run(c)) as { input: Uint8Array; outcome: { accepted?: boolean; errorClass?: string; code?: string; message?: string }; ms: number };
  const { input, outcome } = r;
  out.push({
    name: c.name,
    group: c.group,
    scrimTest: c.scrimTest,
    inputDescription: c.inputDescription,
    howToBuild: c.howToBuild,
    call: c.call,
    limits: c.limits ? { ...D, ...c.limits } : 'DEFAULT_LIMITS',
    inputBytes: input.length,
    inputSha256: sha256(input),
    ...(input.length <= B64_MAX ? { inputBase64: Buffer.from(input).toString('base64') } : {}),
    ...(outcome.accepted ? { expectAccepted: true } : { expectError: outcome.message, expectCode: outcome.code, errorClass: outcome.errorClass }),
  });
}

// the importer's own tree recursion (no FigError: observed behaviour)
const observe = async (name: string, inputDescription: string, howToBuild: string, build: () => Uint8Array): Promise<void> => {
  const input = build();
  const t0 = performance.now();
  let outcome: Record<string, unknown>;
  try {
    const r = await fi.importFig(input, core.createDocument({ id: 'd', name: 'D' }), { fileName: 'x.fig', importImage: async () => ({ src: 'scrim-asset:x.png' }) });
    outcome = { accepted: true, summary: r.report.summary, patchOps: r.patch.length };
  } catch (e) {
    const err = e as Error & { code?: string };
    outcome = { errorClass: err.name, code: err.code, message: err.message };
  }
  out.push({ name, group: 'importer-recursion (observed, not a Scrim limit)', scrimTest: null, inputDescription, howToBuild, call: 'importFig', limits: 'DEFAULT_LIMITS', inputBytes: input.length, inputSha256: sha256(input), ...(input.length <= B64_MAX ? { inputBase64: Buffer.from(input).toString('base64') } : {}), observed: outcome, ms: Math.round(performance.now() - t0) });
};
await observe('component containing an instance of itself', "SYMBOL 'Loop' whose child is an INSTANCE of 'Loop' (test encoder schema)", "figKiwi({ nodeChanges: [DOCUMENT 1:1, CANVAS 1:2, SYMBOL 'Loop' 1:10 under the page, INSTANCE 'Self' 1:11 under 1:10 with symbolData.symbolID = 1:10] })", instanceCycle);
await observe('frames nested 3,000 deep', 'a chain of 3,000 FRAMEs, each the only child of the one before (well under the Kiwi depth cap: records are flat)', 'figKiwi with records F0..F2999, F(i).parentIndex.guid = F(i-1)', () => deepTree(3000));
await observe('frames nested 20,000 deep', 'a chain of 20,000 FRAMEs, each the only child of the one before', 'figKiwi with records F0..F19999, F(i).parentIndex.guid = F(i-1)', () => deepTree(20000));
await observe('100,000 rectangles in one frame', 'one FRAME holding 100,000 RECTANGLE records (no node-count cap exists)', 'figKiwi with 100,000 RECTANGLE records parented to one FRAME', () => manyNodes(100_000));
await observe('nested instances expand exponentially', '8 masters M0..M7 on one page; each M(i) holds 4 instances of M(i+1); M7 holds one rectangle. Instances inside a master are expanded inline as detached copies, so the layers grow as 4^levels (levels 9: 203,886 layers from a 2,191-byte file in 11.6 s; each further level is about 4× the layers and 5× the time)', 'nestedInstances(8, 4): see gen-security.ts', () => nestedInstances(8, 4));
await observe('a Kiwi field named __proto__', "the test schema plus a NodeChange field named __proto__; decoded objects have no prototype", "figKiwi(..., extendSchema([], [field('__proto__', 'uint')]))", protoField);

// image paths: reported, never thrown
const sha1 = (b: Uint8Array) => createHash('sha1').update(b).digest('hex');
const reportCases: unknown[] = [];
const imgCase = async (name: string, scrimTest: string | null, description: string, bytes: Uint8Array, pipeline: (n: string, b: Uint8Array) => Promise<{ src: string } | { error: string }>): Promise<void> => {
  const seen: number[] = [];
  const r = await fi.importFig(bytes, core.createDocument({ id: 'd', name: 'D' }), { fileName: 'a.fig', importImage: async (n: string, b: Uint8Array) => (seen.push(b.length), pipeline(n, b)) });
  const doc = core.applyPatch(core.createDocument({ id: 'd', name: 'D' }), r.patch).doc;
  const photo = Object.values(doc.nodes).find((n: any) => n.name === 'Photo') as any;
  reportCases.push({ name, scrimTest, inputDescription: description, pipelineCalls: seen.length, reportImages: r.report.images, photoLayerType: photo?.type, imageItem: r.report.items.find((i: any) => i.feature.startsWith('Image fill')) ?? null, summary: r.report.summary });
};
await imgCase('image bytes do not match their SHA-1', 'security.test.ts › the pipeline receives the embedded bytes; bytes that do not match their hash never reach it', 'stageA({ tamperImage: true }): the blob is a black 4×3 PNG, the paint hash names the pink one', stageA({ tamperImage: true }).bytes, async () => ({ src: 'scrim-asset:x.png' }));
await imgCase('image refused by the pipeline', 'security.test.ts › an image the pipeline refuses (not an image, does not decode) is reported; the layer stays a box', "stageA(); the pipeline answers { error: 'the file is not an image Scrim can open' }", stageA().bytes, async () => ({ error: 'the file is not an image Scrim can open' }));
// image paint whose hash has no bytes anywhere (dataBlob out of range)
{
  const nodes = [
    { guid: doc, type: 'DOCUMENT', name: 'Document' },
    { guid: page, parentIndex: { guid: doc, position: 'a' }, type: 'CANVAS', name: 'Page' },
    { guid: P(1, 3), parentIndex: { guid: page, position: 'a' }, type: 'RECTANGLE', name: 'Photo', size: { x: 10, y: 10 }, transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 }, fillPaints: [{ type: 'IMAGE', visible: true, opacity: 1, blendMode: 'NORMAL', image: { hash: new Uint8Array(20).fill(0xab), name: 'gone', dataBlob: 7 }, imageScaleMode: 'FILL' }] },
  ];
  await imgCase('image not in the file', null, 'an IMAGE paint whose hash (20 × 0xAB) is neither a ZIP entry nor a blob (dataBlob 7, no blobs)', figKiwi({ nodeChanges: nodes }), async () => ({ src: 'scrim-asset:x.png' }));
}

console.log(JSON.stringify({ limits: D, internalCaps: { kiwiMaxDepth: 64, kiwiMaxDefinitions: 4096, kiwiMaxFields: 4096, entryNameMaxChars: 512, eocdSearchWindow: 65535 + 22, chunksRead: 4 }, security: out, reportCases }, null, 1));
void SCHEMA;
