# Port 5: `query_nodes`, Scrim's XPath-style layer query (19 Oct 2026)

**What it is:** a read-only query such as `//FRAME[@layout="flex"]//TEXT[contains(@name, "Price")]`. It returns the matching layers in document order. Anything outside the subset is an error that says what is supported, never an empty result.

**Sources:**
- the spec, `docs/scrim-transfer/specs/query-nodes.md`;
- the error messages, `query-nodes-errors.md`;
- the cases, `docs/scrim-transfer/cases/query-nodes.json`.

**Where it lives:**
- **The query:** `crates/engine/src/cmd/query/`. A hand-written lexer, a recursive-descent parser and an evaluator, with no new dependency.
- **The engine command:** `node.query`, read-only, reachable from every route (control channel, `run`, plugins).
- **MCP:** the tool `query_nodes`, annotated `readOnlyHint: true`.
- **CLI:** `figcraft-cli query` and `find`.

## Against Scrim's cases

The parser and evaluator take a vocabulary of node tests and attributes, and a document. The tests replay Scrim's cases with Scrim's vocabulary, on Scrim's own documents, through the same code that serves FigCraft (`cmd/query/scrim_cases_tests.rs`).

| Cases | Result |
|---|---|
| **Core** (evaluator, every error message and position) | **244 of 244** identical to Scrim's output, including UTF-16 positions, JavaScript number strings, the quirks and the lint cases |
| **MCP tool** | **18 of 19** identical (result JSON, or the `invalid query: …` text). The 19th depends on Scrim's design linter (Port 6). The 4 schema-rejected arguments are checked for refusal only, as the case file says. |
| **CLI** (exit code, stdout, stderr; `--json`, `--limit`, `find`'s quoting, usage messages) | **24 of 25** identical, through the same argument code `figcraft-cli` runs. The 25th needs Scrim's linter (Port 6). |

**Divergences, each documented in the code:**
- **`unexpected '<c>'` names the whole character.** Scrim names one UTF-16 unit, which can be half an emoji (its quirk 6). No case covers it.
- **The tool's `results` come back with sorted keys over MCP.** The engine returns JSON values, and this workspace's `serde_json` doesn't keep key order. The CLI's `--json` keeps the spec's order (pinned by the cases), and the values are identical.
- **"Never changes the revision"** (invariant 1) holds as: the document identical, nothing in the change feed, no undo step. FigCraft's `revision` counts every command, reads included, because it is the app's redraw counter (`figcraft_tests.rs` › `results_carry_the_document_box_and_never_change_the_document`).

## On FigCraft's documents: the [MAP] choices

| Spec | FigCraft |
|---|---|
| **The document and `/X`** | Every page, page by page. `/X` is a top-level layer of a page. Positions count per parent, and a page is a parent, so `/*[1]` is each page's first layer. `page` (MCP, `node.query`) limits it to one page. |
| **Node tests** | FigCraft's kinds by their Figma names: `FRAME GROUP SECTION RECTANGLE ELLIPSE POLYGON STAR LINE VECTOR TEXT BOOLEAN_OPERATION COMPONENT COMPONENT_SET INSTANCE SLICE`. Plus `IMAGE`, any layer with a visible image fill (it can also answer to `RECTANGLE`), and `*`. Scrim's `BOX` is not a kind here: the error lists the kinds, and `//rectangle` suggests `RECTANGLE`. |
| `@type` / `element` | `rectangle` / `RECTANGLE` …, from the kind |
| `@id`, `parentId` | Numbers. A top-level layer's `parentId` is its page. |
| `@width`, `@height` | The stored size, a number |
| `@fill` | The first visible solid fill as `#rrggbb` (`#rrggbbaa` when not opaque), for any layer |
| `@color` / `@background` | That fill, for text / for anything else |
| `@layout`, `@direction` | Auto layout: `flex` (or `grid`), and `row` or `column`. Absent without auto layout. |
| `@position` | `absolute` for a layer set to absolute position inside auto layout |
| `@src` | The first visible image fill's hash |
| `@alt` | **Not supported** (an error, not a silent absence): FigCraft has no alt text |
| `@lint`, `@lintCount` | **Not yet.** They come with the linter (Port 6). Until then they are errors. |
| **New: `@hidden`, `@locked`** | Present when the layer is hidden or locked (`[not(@hidden)]`). A query never sees booleans, so they are presence attributes. |
| `@style.<key>`, `@layout.<key>` | A fixed table: `width height display flexDirection flexWrap gap rowGap columnGap padding paddingTop/Right/Bottom/Left position flexGrow opacity borderRadius overflow color background backgroundColor fontSize fontFamily fontStyle textAlign`. Any other key is absent, as an undeclared property is in Scrim. |
| **Rects** | From the document (page-space bounds), so never stale: `stale: false`, no `measuredAt` |
| **CLI exit codes** | Scrim's: 1 invalid query, 2 usage, 3 the file (`not_found`, `not_a_file`, `too_large` over 200 MB, `invalid_document`) |

Tests: `cmd/query/figcraft_tests.rs` (5), `crates/mcp/tests/query_nodes.rs` (3) and `apps/figcraft-cli/tests/query.rs` (2).

## Timing (INVALID)

`figcraft_tests.rs` › `query_timing` (ignored; run it locally) uses a 20,201-layer document and 30 runs after 3 warm-ups. The machine had a load average of **240** and **24 GB of swap** in use, from other processes, so **the numbers are INVALID**. Read them as an upper bound:
- **`//TEXT`:** median 36 ms;
- **`//FRAME[@layout="flex"]//TEXT[contains(@name,"Title")]`:** 78 ms;
- **`/FRAME[3]//RECTANGLE[@width > 100]`:** 0.3 ms;
- **`//*[@position="absolute" or not(@type="rectangle")]`:** 47 ms;
- **`//*//*//TEXT`:** 68 ms. That `//` chain stays linear: each `//` step visits a layer once, the spec's design point.

For reference, Scrim's were 2.7–7.8 ms, on a machine that was also unfit (INVALID). Run it again on an idle machine before reading anything into the comparison.

## What remains

| Item | Estimate |
|---|---|
| `@lint` / `@lintCount` once the linter is in (Port 6) | Part of Port 6 |
| Keep the key order over MCP: `serde_json`'s `preserve_order` workspace-wide, or return the text from the engine | 0.5 session (a workspace-wide feature change needs care) |
| More `@style` keys as agents ask for them (effects, strokes, constraints) | Small each |
