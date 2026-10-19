# .fig import: limits, refusals and what is not limited

**What this is:** every limit and refusal in Scrim's `.fig` importer, with its exact value and its message verbatim. It is the companion of `specs/fig-import.md`. Every message below was produced by running the importer: `cases/fig-import.json › security` holds the inputs (most as base64), the build steps and `expectError`.

**What "verbatim" means here:**
- `<…>` is a value filled in at run time.
- `mb(n)` is `Math.round(n / 1048576) + " MB"`, so small test limits print as "0 MB" (§8).
- Every refusal is a `FigError { code, message }`, thrown before any patch exists, so the document is untouched.

## 1. Order of checks

1. **The size of the file,** from its length (the hosts check it again with `stat` before reading; §3).
2. **The container:** bare fig-kiwi or a ZIP at offset 0. For a ZIP:
   - the end-of-central-directory record;
   - ZIP64;
   - the entry count;
   - the directory's bounds;
   - then, per entry, in directory order: its name, encryption, whether it is wanted, its local header, truncation, and the inflate under the cap and the budget.
3. **The chunks:** truncation, the count, then each decompress under the cap and a new budget.
4. **The Kiwi schema:** the definition and field caps, each kind, each type.
5. **The Kiwi message:** bounds, the depth, array lengths, unknown fields, trailing bytes, then `nodeChanges`.
6. **Images,** each SHA-1 checked, then the host pipeline. These are reported, never thrown.
7. **Conversion and composition.** These stages have **no limits**; the gaps are in §6.

## 2. Limits inside the importer

**The defaults** (`DEFAULT_LIMITS`; a caller may pass others, and the tests do):

| Limit | Value | Code | Message (verbatim) | Case in the JSON |
|---|---|---|---|---|
| `fileBytes`: the file | **200 MiB** (209,715,200 bytes) | `too_large` | `the file is <mb(len)>: Scrim imports .fig files up to <mb(limit)>` (at 200 MiB + 1: *"the file is 200 MB: Scrim imports .fig files up to 200 MB"*) | file over the 200 MB cap (default limits) |
| `entries`: ZIP entries | **5000** | `limit` | `the archive has <count> entries — over the <limit> limit` | too many entries (default limits) |
| `entryBytes`: any one decompressed entry **or chunk** | **256 MiB** | `limit` | declared over the cap (checked before inflating): `<name> would decompress to <mb(declared)> — over the <mb(cap)> limit`; actual output over the cap (a lying header): `<name> decompresses to more than <mb(cap)>` | zip bomb, honest / lying header (default limits) |
| `totalBytes`: everything decompressed, together | **768 MiB per stage** (the archive and the chunks each start a new budget, §5) | `limit` | the same two messages; the cap is `min(entryBytes, budget left)` | total decompressed size over the cap |

**Internal caps** (not configurable):

| Cap | Value | Code | Message (verbatim) |
|---|---|---|---|
| Kiwi definitions | **4096** | `limit` | *the file declares too many Kiwi definitions* |
| Fields per definition | **4096** | `limit` | *a Kiwi definition has too many fields* |
| Kiwi nesting depth | **64** levels of nested definitions | `limit` | *the document nests too deeply* |
| Array length | `n ≤ bytes left` (one byte per item at least), checked before allocating | `corrupt` | `the array <Definition>.<field> claims more items than there are bytes` |
| Byte array | `n ≤ bytes left` | `corrupt` | *a Kiwi byte array runs past the end* |
| varuint | at most 5 bytes (a varuint64 at most 10) | `corrupt` | *a Kiwi number is malformed* / *a Kiwi 64-bit number is malformed* |
| Trailing bytes after the root message | none allowed | `corrupt` | *the document data has bytes after its end (corrupt)* |
| ZIP entry name length | **512** characters | `unsafe_entry` | (the unsafe-path message below) |
| End-of-central-directory search | the last 65,535 + 22 bytes | `corrupt` | *the archive has no central directory (truncated?)* |
| Chunks read | **4** (only 0 and 1 are used) | — | — |
| A generated SVG path | **200,000** characters (core `MAX_SVG_PATH_CHARS`) | reported | *Vector path too long for Scrim — not drawn* |
| A generated SVG node | core `svgContentIssues`: 5000 elements, 1,000,000 characters in total, depth 16, 64 stops, 16 dash values, the element and attribute allow-list | reported | *Vector Scrim's SVG check refused — not drawn (\<issue\>)* |

**Entry names (path traversal).** A name is **safe** only if all of these hold:
- it is non-empty and at most 512 characters;
- it doesn't start with `/`;
- it contains no `\` and no NUL;
- it doesn't start with a drive letter (`/^[a-z]:/i`);
- no `/`-separated part is `..` or `.`.

**Every** entry is checked, wanted or not, before any is read. The rule: `unsafe_entry`, *`the archive has an entry with an unsafe path ("<first 80 characters of the name>") — refused`*. The tested names are `../evil.fig`, `/etc/passwd`, `images/../../x`, `images\..\x`, `C:/x`, `a/./b` and `nul\0byte`; the cases add a 513-character name and an empty name. The name never becomes a path: Scrim writes nothing, and images are keyed by their hash.

**Every other refusal, verbatim** (all thrown):

| Code | Message | Trigger |
|---|---|---|
| `not_fig` | *this is not a Figma file (neither a .fig archive nor fig-kiwi data)* | not `fig-kiwi` at offset 0, and not a ZIP local header at offset 0 (or shorter than 22 bytes) |
| `not_fig` | *the archive has no canvas.fig: not a Figma file* | no `canvas.fig` or `canvas` entry |
| `not_fig` | *canvas.fig is not fig-kiwi data* | the canvas entry doesn't start with `fig-kiwi` |
| `not_fig` | *not fig-kiwi data* | `readChunks` called on non-fig-kiwi bytes (not reachable through `readFig`) |
| `not_fig` | *the file holds no document nodes* | `Message.nodeChanges` is not an array |
| `unsupported` | *ZIP64 archives are not read* | EOCD count `0xFFFF` or offset `0xFFFFFFFF` |
| `unsupported` | *encrypted archive entries are not read* | flag bit 0 on any entry |
| `unsupported` | `<name> uses compression method <n>; only stored and deflate are read` | a wanted entry with a method other than 0 or 8 (ignored entries are never inflated, so their method is never checked) |
| `corrupt` | *the archive has no central directory (truncated?)* | no EOCD in the window |
| `corrupt` | *the central directory runs past its end* | `cdOffset + cdSize > eocd` |
| `corrupt` | *a central directory entry is damaged* | a bad signature or out of bounds |
| `corrupt` | `the entry <name> is damaged` | a bad local-header signature or offset |
| `corrupt` | `the entry <name> is truncated` | data runs past the end of the file |
| `corrupt` | `<name> is corrupt (deflate)` | an inflate error |
| `corrupt` | `<name> is <n> bytes, not the <declared> the archive declares` | the output length differs from the declared size |
| `corrupt` | *canvas.fig is truncated (a chunk runs past the end)* | a chunk length past the end |
| `corrupt` | *canvas.fig has no schema or no data chunk* | fewer than 2 chunks |
| `corrupt` | `the schema is corrupt or truncated (<zstd\|deflate>)` / `the document data is corrupt or truncated (<zstd\|deflate>)` | a decompressor error |
| `limit` | `the schema decompresses to more than <mb>` / `the document data decompresses to more than <mb>` | a chunk over the cap |
| `corrupt` | `the Kiwi schema has an unknown kind <k>` | kind > 2 |
| `corrupt` | `the Kiwi field <Def>.<field> has an unknown type` | type < −8 or ≥ the definition count (struct and message fields) |
| `corrupt` | `the Kiwi schema has no <root>` | no `Message` definition |
| `corrupt` | `the Kiwi message <Def> has an unknown field <id>` | a field id not in the definition |
| `corrupt` | *a Kiwi string is unterminated* | no NUL before the end |
| `corrupt` | *the document data ends early (the file is truncated or corrupt)* | a read past the end |
| `corrupt` | *the document data ends early (Kiwi float)* | a float's 3 trailing bytes missing |

## 3. Limits in the hosts (outside the importer)

| Where | Check | Message (verbatim) |
|---|---|---|
| Desktop File › Import (`documents.ts › openPath`) | `stat` size > 200 MiB, **before** reading | `“<basename>” is <size/1048576, 0 decimals> MB: Scrim opens .scrim files up to 200 MB`. **A wording bug:** it says ".scrim" for a `.fig`, because `scrimTooLarge` is shared |
| CLI `scrim import-fig` (`open.ts › readFigFile`) | `stat` size > 200 MiB, before reading; exit code 3 | `“<basename>” is <n> MB: Scrim imports .fig files up to 200 MB` (code `too_large`) |
| Home drop, studio (`useHomeActions.ts`) | `file.size > 50 MiB`, before reading | `“<file name>” is larger than 50 MB.` |
| Home drop, server (`library/messages.ts`) | `floor(base64Length × 3 / 4) > 50 MiB` | `.fig files dropped on Home are limited to 50 MB: use Import for larger ones` (`too_large`) |
| Home drop, server | name not a string, empty, or over 255 characters | `a .fig import needs the file name` (`invalid_name`) |
| Home drop, server | data not a string or not `^[A-Za-z0-9+/]*={0,2}$` | `a .fig import needs the file bytes (base64)` (`invalid_document`) |
| WebSocket | no explicit `maxPayload`: the `ws` default (100 MiB) applies, above a 50 MB file's base64 (about 67 MB) | — |
| Each embedded image (`assets/sniff.ts`, `importer.ts`) | over 25 MiB; over 16,384 px a side or 100 megapixels (from the header, or from the decode when the header has none); a decoded size different from the header's; not PNG, JPEG, WebP, GIF, AVIF or SVG; truncated; an SVG that could run script or reach outside | `Images are limited to 25 MB (this one is <n> MB)`; `The image is <w> × <h> px: the limit is 16384 px a side and 100 megapixels`; `This <TYPE> says it is <w> × <h> px but decodes to <W> × <H>: refused as a possible decompression bomb`; `Not a supported image: Scrim imports PNG, JPEG, WebP, GIF, AVIF and SVG`. Each is **reported** in `report.images.failed`, never thrown |

**How errors reach the person:**
- **The server** replies `library:result { ok: false, code, error }`: the code is `FigError.code`, or `error` for anything else.
- **The desktop** shows the thrown message.
- **The CLI** prints `scrim: <message>` and exits 3; `--json` prints `{ "error": { code, message } }` instead.

**On the images, as rule 18 applies to the import:**
- An image's type comes from its bytes.
- Its file name comes from a validated sha256.
- GPS EXIF is stripped unless the person keeps it.
- A truncated image is refused.
- The importer itself never writes: it hands the bytes to the host.
- Figma's SHA-1 must match before the pipeline sees the bytes; otherwise the failure reads *the image bytes do not match their hash*.
- Scrim's per-batch 64 MB image cap does **not** apply to a `.fig`: images go one by one through `importChecked`.

## 4. No code execution, no network, no metadata

- **The Kiwi schema the file embeds is interpreted as data.** The reference runtime, and OpenPencil's adaptation, compile it to JavaScript with `new Function`: code derived from an untrusted file. A Rust port that generates code from the schema at build time is fine. Generating or evaluating code at *run time* from the file's schema is not.
- **Decoded objects have no prototype,** so field names from the file (`__proto__`, `constructor`) are inert keys. The case *a Kiwi field named \_\_proto\_\_* imports cleanly.
- **Nothing in the file is followed:** no URL, no font fetch, no image reference outside the file.
- **Every SVG is generated** from numeric path data and colours, then checked by the allow-list. Markup from the file is never copied.
- **No Figma metadata** (GUIDs, session or local ids, symbol data, plugin data, keys, style ids) reaches the document or its exports. The stage A test checks the document JSON against `/sessionID|localID|guid|symbolData|figma|fig-kiwi|"1:\d+"/i` and the exports against `/sessionID|guid|figma|data-fig/i`.
- **The source file is only read,** and Scrim never writes `.fig`.

## 5. CRCs and integrity

- **No CRC is read**, from either the local header or the central directory. CRC 0 (as in every Scrim test zip and `tests/fixtures/fig/stage-a.fig`), or a wrong CRC, is accepted. Its real CRCs are in `cases/fig-import.json › zipCrc`: canvas.fig `835121921`, meta.json `2745614147`.
- **Integrity comes from elsewhere:**
  - the inflated length must equal the declared size;
  - every image must hash to its SHA-1 name;
  - the Kiwi decode must consume the data exactly, with no trailing bytes.
- **For a port:**
  - verifying CRCs is stricter and reasonable;
  - then the fixtures must be re-zipped with real CRCs, or CRC 0 accepted as "unset";
  - FigCraft today refuses them with "Invalid checksum".

**The two budgets.** The archive's 768 MiB budget and the chunk stage's are separate: `readChunks` starts afresh. One file can therefore decompress about 768 MiB of entries, then up to 2 × 256 MiB of chunks (more than 1 GiB at peak, with the canvas held twice). The README's "the total" reads as one budget. A port should keep one budget across both stages.

## 6. What is NOT limited (probed; `cases/fig-import.json`, group "importer-recursion")

None of these is a `FigError`. Each was run against Scrim's importer:

| Input | File size | What happens in Scrim | What a port should do |
|---|---|---|---|
| A component containing an instance of itself | 1,921 B | **`RangeError: Maximum call stack size exceeded`**: unbounded recursion in `convert`, since the detached copy re-expands the master forever | detect cycles in master → instance expansion (a visited set on the path) and skip with a report line |
| Frames nested 3,000 deep (records are flat, so the Kiwi depth cap doesn't apply) | 14.8 KB | **`RangeError`**. The threshold on Node's default stack is between 2,000 (imports) and 2,500 (overflows) | an explicit tree-depth cap, or iterative conversion |
| Nested instances: 8 masters, each holding 4 instances of the next | 2.2 KB | **accepted: 50,969 layers in about 2 s**. Instances inside masters are expanded inline, so the output grows as fan^levels: 9 levels gave 203,886 layers in 11.6 s from 2,191 bytes, and each level multiplies it by about 4 | a cap on created nodes (and on expansion depth), checked as nodes are created |
| 100,000 rectangles in one frame | 378 KB | accepted: 100,001 layers in 1.7–3.8 s | a record-count and created-node cap with a clear message |

**Also unlimited:**
- the number of images;
- the number of instances or overrides;
- wall-clock time (there is no timeout or cancellation);
- the number of decoded Kiwi objects: a 256 MiB data chunk may hold tens of millions of small objects, since arrays are bounded only by bytes left.

These failures are not crashes, but not user-facing sentences either:
- **through the server** they surface as `code: 'error'` with the RangeError's message;
- **through the CLI** they exit 1 with code `internal`.

`docs/research/fig-import-assessment.md` proposed "a node-count limit"; it was not built.

## 7. Test coverage in Scrim

**Tested** (`security.test.ts`): 19 of the JSON's cases carry `scrimTest`:
- the size cap;
- both zip-bomb forms (with test limits);
- the total cap;
- the entry count;
- 7 unsafe names;
- the corrupt zstd data, a truncated chunk, a truncated zstd frame, a truncated archive and two non-Figma inputs;
- three bytes through `importFig`;
- plus the image hash mismatch and the pipeline refusal.

**Untested in Scrim but in the code:**
- encryption, ZIP64 and an unsupported method;
- the directory bounds, a damaged entry, a truncated entry, a size mismatch and a corrupt deflate entry;
- a missing canvas, or a canvas that isn't fig-kiwi;
- one chunk only;
- every Kiwi schema and message error except the ones the truncation cases reach.

The JSON covers all of them.

## 8. Quirks in the messages

- **`mb()` rounds to whole MB,** so the test limits print oddly. The 100-byte size cap reads *"the file is 0 MB: Scrim imports .fig files up to 0 MB"*, and the 2 MiB total cap gives *"images/3333…3333 would decompress to 1 MB — over the 0 MB limit"* (600 KiB declared against the 245 KiB left after three images). A port could print exact sizes below 1 MB.
- **A ZIP cut to 100 bytes** reports *the archive has no central directory (truncated?)*, not a truncation of the entry.
- **The "corrupt Kiwi data" test** (zstd of garbage) actually fails on trailing bytes: *the document data has bytes after its end (corrupt)*. Only the code is asserted.
- **The NUL name** is echoed into the message with its NUL character.
