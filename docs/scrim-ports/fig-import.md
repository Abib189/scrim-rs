# Port 4: the `.fig` importer, Scrim's design on FigCraft's reader (19 Oct 2026)

FigCraft already had a `.fig` reader: a Kiwi decode, mapped onto its REST JSON importer (`crates/format/src/fig/`). It reads Figma's model into a Figma-shaped model, so most of what Scrim had to approximate in CSS is exact here.

This port therefore brings over Scrim's **security design** and **measures fidelity** against Scrim's numbers. It doesn't replace the reader.

**Sources:**
- `docs/scrim-transfer/specs/fig-import*.md`;
- `docs/scrim-transfer/cases/fig-import.json`, including 62 hostile inputs, 56 of them embedded.

**Timings are INVALID:** the machine was loaded and swapping.

## Security: Scrim's hostile inputs against this reader

`crates/format/src/fig/security_tests.rs` replays every embedded input and asserts three things:
- no panic;
- no run-away (under 30 s each);
- refusal of every input Scrim refuses at its default limits.

| Group (cases) | Before this port | After |
|---|---|---|
| **Size, zip bombs, entry count** (9) | Zip bombs: "canvas.fig is too large". No file-size cap; no entry-count cap | **200 MB file cap** and **at most 5,000 entries** added (Scrim's caps). Zip bombs refused as before |
| **Unsafe entry names** (9) | Refused only because Scrim's test zips have CRC 0. With **correct CRCs they were accepted.** Not exploitable (no entry name ever becomes a path), but a sign of a hostile file | **Refused before anything is read:** *"the archive has an entry with an unsafe path ("…") — refused"*. The rule: non-empty; at most 512 characters; relative; no `\` or NUL; no drive letter; no `.` or `..` part. Tested with correct CRCs |
| **Zip structure** (14) | Encrypted, unsupported method, no EOCD, corrupt deflate: refused. ZIP64, an odd central directory, and a size disagreement: refused for CRC 0; **accepted with correct CRCs** | **Unchanged.** The `zip` crate tolerates these safely. Scrim refused ZIP64 only as unsupported |
| **Container and chunks** (8) | **A corrupt zstd chunk was accepted** as an empty document | **Refused** (the trailing-bytes check below catches it) |
| **Kiwi schema** (6) | Refused | Refused |
| **Kiwi message** (9) | **Bytes after the message: accepted. A message without `nodeChanges`: an empty document** | **Both refused:** *"message: N bytes after the message"* and *"not a design file: the message has no layers (nodeChanges)"* |
| **Recursion and stress** (6) | A self-instancing component: 4 layers. Exponential instances: **capped at 50,970 layers by the existing layer budget, 3.7 s** (Scrim: 203,886 layers, 11.6 s, no cap). `__proto__`: harmless | Unchanged, and tested |
| **Built in the test, not embedded** | **Frames nested 3,000 and 20,000 deep** (in Scrim: a stack overflow, `RangeError`) and **100,000 rectangles in one frame** | **No overflow and no panic**, within the layer budget (`deep_nesting_never_overflows_the_stack`, `a_hundred_thousand_layers_import_within_the_budget`) |

**Count, of 56 embedded:** before the port, 4 were accepted where Scrim refuses (#37, #54, #55, plus the CRC-masked names). After it, every case Scrim refuses at its default limits is refused, and the rest stay within the budget.

**Not ported:**
- **Scrim's 768 MiB decompression budget.** This reader caps the canvas and each image at 256 MiB, and the decoded values by the layer budget, which follows the compressed size.
- **Scrim's per-limit test overrides,** since nothing here takes limits as parameters.

## Fidelity: the fixtures and the 24.6 MB sample

| File | Scrim (layers imported / approximated / skipped) | This importer (layers) | This importer's notes (what it didn't keep) |
|---|---|---|---|
| stage-a (read bare) | 15 / 0 / 2 | 18 | "sticky layers import as frames with their text" |
| stage-b | 30 / 3 / 0 | 30 | — |
| stage-c | 27 / 6 / 2 | 31 | "document children other than pages were skipped" |
| stage-d | 12 / 2 / 2 | 15 | "vectors without geometry are empty"; 1 image fill whose image isn't in the file (grey placeholder) |
| lines | 16 / 1 / 0 | 17 | — |
| **The sample** (24.6 MB, read in place, never copied, sha256 `5d08369d…`) | **752 / 33 / 27** ("94% exact", by Scrim's hand count: 534 of 568 records) | **753 layers, 11 images** (2.9 s, INVALID) | **one note:** image filters drawn by the approximate renderer (×3) |

**How the counts compare:**
- **Layers:** the counts agree within one.
- **Scrim's "skipped" 27** is Figma's internal page, which this importer keeps as its styles and variables, not as layers.
- **Scrim's 33 "approximated"** were CSS approximations: masks, blurs, gradients, sizing rules that have no CSS form. This model keeps those as Figma keeps them, so they aren't approximations here. Where CSS loses them is the export, measured in Port 2.
- **What this importer can't express** it lists as notes, per feature with a count, not per layer. On the sample that's one feature (3 image filters).

**Not measured here:** Scrim's 522 / 541 boxes within 1 px of Figma's. This importer keeps the file's stored geometry and doesn't re-run layout on import (see `export-fidelity.md` › Row), so its boxes are Figma's own by construction. A per-layer comparison against Figma's stored boxes would confirm it: about half a session.

## What remains

| Item | Estimate |
|---|---|
| A per-layer import report in Scrim's form ("N imported, A approximated, S skipped") alongside the notes | 1–2 sessions: the notes come from `map.rs` and `rest.rs` and would each need the layers they affect |
| Should the importer re-run auto layout after import? (Scrim did, through CSS; this keeps Figma's stored geometry) | A decision for the owner |
| One decompression budget across all stages (Scrim's spec recommends it) | 0.5 session |
