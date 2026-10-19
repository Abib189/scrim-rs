# `query-nodes.json`: the test cases for `query_nodes`

These are the cases for the spec in `../specs/query-nodes.md`. Every expectation in the file is a real output, produced by running Scrim's implementation:
- `queryNodes` from `packages/core/src`, cross-checked against the built `packages/core/dist`;
- the `query_nodes` MCP tool, through the CLI's in-process session;
- the CLI's `run()`.

None of them was typed by hand. The generator script lives outside the repo, in the session scratchpad (`gen-query-cases.ts`). Re-running it against a changed Scrim regenerates the file.

## Top level

```jsonc
{
  "format": "scrim-query-cases/1",
  "about": "…",
  "documents": { "<docName>": ScrimDoc, … },
  "cases":     [ CoreCase, … ],   // the evaluator: ids or an error
  "toolCases": [ ToolCase, … ],   // the MCP tool: result JSON or tool-error text
  "cliCases":  [ CliCase, … ]     // the CLI: exit code, stdout, stderr
}
```

## Documents

`documents` holds plain Scrim documents, exactly as `.scrim` files store them: `{ schemaVersion, id, name, rootIds, nodes, components, tokens }`. Each node is:

```jsonc
{ "id": "logo", "type": "text",            // frame | box | text | image | instance
  "name": "Logo", "parentId": "nav",       // null for an artboard (it is in rootIds)
  "childIds": [],                          // order = paint = DOM = query document order
  "layout": { … }, "style": { … },         // camelCase CSS: string | number values
  "text"?: "…", "src"?: "…", "alt"?: "…", "componentId"?: "…", "canvas"?: {x,y} }
```

A component is `components[id] = { id, name, rootId, props }`. Its `rootId` node is the master, the node that answers to `COMPONENT`. An instance's children are real nodes.

| Name | What it is | Used for |
|---|---|---|
| `query` | Scrim's core test document: three artboards (`page`, `poster`, `card`) and an instance `inst` of the `card` master (`cmp-card`), whose child is `inst-title`. The instance ids are renamed from generated ids so they stay stable. | most cases |
| `query-stray` | `query` plus box `stray` under `poster`, with `position` and `display` only in `style` | the "declared" (merged layout+style) cases |
| `edge` | values the core tests don't pin: px with spaces, `-.5px`, `240PX`, `1e2`, `+5`, `0`, `0.5`; empty-string CSS values; `inline-flex` and `row-reverse`; style beating layout; an empty name and an empty text; quotes and non-ASCII in names; a nested master `btn` (`cmp-btn`) with an instance `binst`; and three levels of nested frames | the edge cases |
| `landing` | the export gate's landing page (`packages/mcp/src/__tests__/exportGateDocs.ts`) | tool and CLI cases |
| `many` | frame `f` holding 105 text nodes, `t0`…`t104` | the limit and truncation cases |

## `cases` (core evaluator)

```jsonc
{ "name": "position: //TEXT[1] per parent",   // unique
  "doc": "query",                              // a key of documents
  "query": "//TEXT[1]",
  "lint"?: { "<nodeId>": ["rule-id", …] },     // the caller's lint results (see below)
  "expect": { "ids": ["logo", "h1", …] }       // document order, each once
         |  { "error": "<exact message>", "at": <1-based char> | null } }
```

**`lint`:**
- **Present:** the case supplies a lint function `id → lint[id] ?? []`. Duplicates in a list are meaningful: `@lintCount` counts them and `@lint` deduplicates them.
- **Absent:** no lint function is supplied, so a query that reads `@lint` or `@lintCount` must fail with E36. A Rust port that always supplies lint can skip the two cases named `lint: … without lint results is an error`, or model the "no lint" mode.

**`error`:** the full message, ` (at character N)` suffix included. `at` is that N, counted in UTF-16 code units, or null when the message has no position. The cases named `utf16: …` pin the UTF-16 counting.

## `toolCases` (the MCP tool)

```jsonc
{ "name": "tool: limit 5", "doc": "many",
  "args": { "query": "//TEXT", "limit": 5 },    // the tool arguments, verbatim
  "linterDependent"?: true,                    // the result depends on Scrim's linter
  "expect": { "isError": false, "json": { "query", "total", "returned", "truncated", "results": [ … ] } }
         |  { "isError": true, "text": "invalid query: …" } }
```

- **On success,** the tool's text is exactly `JSON.stringify(json, null, 2)`: 2-space indent, keys in the order shown, no trailing newline. The generator asserts it.
- **Results** carry `rect: null, measuredAt: null, stale: true`, because no editor measured anything.
- **Schema-rejected arguments** (`limit` 0, 1001 or 2.5; a missing `query`) keep the SDK's text for reference. That text is not part of the contract: assert only `isError`.
- **`linterDependent: true`:** the matches come from Scrim's own design linter. Assert only the shape, not the ids, unless the port has the same rules.

## `cliCases` (the CLI)

```jsonc
{ "name": "cli: query --limit, truncated, human", "doc": "query",
  "argv": ["query", "<file>", "//*", "--limit", "2"],   // "<file>" = the doc written as a .scrim file
  "expect": { "exit": 0, "stdout": "2 of 15 matches (--limit for more)\n  FRAME     Page  (page)\n…", "stderr": "" } }
```

The file's path is replaced by `<file>` in both `argv` and the outputs. Write `documents[doc]` to a temporary `.scrim` file (plain JSON) and substitute its path.

## Counts

| Section | Cases |
|---|---|
| `cases` | 244: 145 with `ids`, 99 with `error` |
| `toolCases` | 19 |
| `cliCases` | 25 |
