# Spec: `query_nodes`, the XPath-style node query

A transfer spec, written so that the query can be re-implemented without reading Scrim's TypeScript. The test cases are in `../cases/query-nodes.json` (schema: `../cases/README-query-nodes.md`). Every error message, verbatim, is in `query-nodes-errors.md`.

Scrim's sources, for reference only: `packages/core/src/query/{parse,evaluate}.ts` and its README; `packages/mcp/src/queryTools.ts`; `packages/cli/src/{cli,format}.ts`.

## 1. What it is

`query_nodes` finds nodes in a document with a small, hand-written subset of XPath 1.0. For example, `//FRAME[@layout="flex"]//TEXT[contains(@name, "Price")]` returns the ids of the matching nodes in document order.

Three layers use one evaluator:

| Layer | Surface | Adds |
|---|---|---|
| core | `parseQuery(text)`, `queryNodes(doc, text, {lint?})` | the grammar and the evaluation |
| MCP | the tool `query_nodes { query, limit? }` | lint results, the result cap, names, types, parents, measured rects |
| CLI | `scrim query <file> '<q>'`, `scrim find <file> --type --name` | calls the MCP tool in-process; human or `--json` output |

**Invariants. A Rust port must keep all four:**
1. **Read-only.** A query never mutates the document, never commits a patch and never changes the revision. The MCP tool is annotated `readOnlyHint: true`.
2. **A query is data, not code.** It never evaluates code, never builds a regular expression from the query text, and never fetches or imports anything.
3. **Closed subset.** Input outside the subset is an error that says what *is* supported. It is never a silent empty result.
4. **No dependency.** Write the parser by hand, not with an XPath library. The parser is recursive descent over a token stream.

## 2. The document model the query reads

A Scrim document is `{ rootIds: NodeId[], nodes: { [id]: Node }, components: { [id]: ComponentDef }, … }`.
- **`rootIds`** are the top-level artboards, in order.
- **`Node`** is `{ id, type, name, parentId, childIds, layout, style, text?, src?, alt?, componentId?, canvas?, overrides?, … }`.
  - `type` is one of `frame | box | text | image | instance`.
  - `layout` and `style` are flat maps of CSS property names in camelCase. Each value is a string or a number.
- **`ComponentDef`** is `{ id, name, rootId, props }`. Its `rootId` is the master's root node, which lives in the tree like any other node.

**Instances are materialised:** an instance node has real child nodes in `nodes`, with the overrides already written into them. `overrides` is only a pin marker, and the query never reads it.

**FigCraft mapping:** FigCraft has 17 Figma-like node kinds, and its layout is baked into transforms. Each place below that is marked **[MAP]** needs a deliberate decision on the Rust side.

## 3. Lexing

The lexer scans left to right. At each position it tries the following, **in this order**:

| # | Input | Token / action |
|---|---|---|
| 1 | space, `\t`, `\n`, `\r` | skipped. Whitespace may sit between any two tokens: `  // TEXT [ @name = "Logo" ]\n` is valid. |
| 2 | `//` | DSLASH |
| 3 | `!=` `<=` `>=` | OP |
| 4 | `::` | error (named axes) |
| 5 | `..` | error (parent) |
| 6 | `/` `[` `]` `(` `)` `@` `,` `*` | SLASH LB RB LP RP AT COMMA STAR |
| 7 | `=` `<` `>` | OP |
| 8 | `"` or `'` | STRING up to the next identical quote. **There are no escapes.** No closing quote: error. |
| 9 | regex `-?(\d+(\.\d+)?\|\.\d+)` | NUMBER (the value is a JS `Number(text)`, so `01` is 1 and `1.0` is 1) |
| 10 | `.` | DOT. It is only ever an error token: `.` (self), or the separator in `@style.key`. |
| 11 | regex `[A-Za-z_][A-Za-z0-9_-]*` | NAME. **Hyphens are allowed inside a name**, so `starts-with` is one token. |
| 12 | `\|` | error (unions) |
| 13 | `$` | error (variables) |
| 14 | anything else | error `unexpected '<c>'` |

Then comes END, at offset = the length of the input.

**Consequences, all of them pinned in the cases:**
- **NUMBER is tried before DOT and NAME.**
  - `1e3` lexes as NUMBER `1` then NAME `e3`, so it is an error.
  - `10px` lexes as NUMBER then NAME, so it is an error. Write `10`, not `10px`, in a comparison.
  - `@style.1x` lexes `.1` as a NUMBER, so you get the "@style needs a property" error, not the camelCase one.
- **Keywords are case-sensitive.** `and`, `or`, `not`, `contains`, `starts-with` and `last` work; `AND` is a NAME, and `NOT(` is an unknown function.
- **A lone `-` or `!` falls through to `unexpected`.** A `-` that doesn't start a number is not a NAME start either.

**Offsets:** every token carries its 0-based offset, in **UTF-16 code units** (JavaScript string indices). An error reports `offset + 1` as "character N". So an emoji before the error shifts the reported position by 2. **[Rust]** Either count UTF-16 units for parity (the cases pin this), or count `char`s and document the divergence.

## 4. Grammar (EBNF)

The parse succeeds only if it consumes every token up to END.

```ebnf
query        = step , { step } ;                      (* the first token must be "/" or "//" *)
step         = axis , nodetest , { predicate } ;
axis         = "/"                                   (* child *)
             | "//" ;                                (* descendant: descendant-or-self, then child *)
nodetest     = "*" | "FRAME" | "BOX" | "TEXT" | "IMAGE" | "INSTANCE" | "COMPONENT" ;
predicate    = "[" , ( position | last | orexpr ) , "]" ;
position     = NUMBER ;                              (* only when NUMBER is directly followed by "]";
                                                        it must be an integer >= 1 *)
last         = "last" , "(" , ")" ;                  (* only when exactly these 4 tokens precede "]" *)
orexpr       = andexpr , { "or" , andexpr } ;         (* left-associative *)
andexpr      = unary , { "and" , unary } ;            (* "and" binds tighter than "or" *)
unary        = "(" , orexpr , ")"
             | call                                  (* a NAME directly followed by "(" *)
             | comparison ;
call         = "not" , "(" , orexpr , ")"
             | ( "contains" | "starts-with" ) , "(" , attribute , "," , STRING , ")" ;
comparison   = attribute , [ op , literal ]           (* without an op, the attribute's existence *)
             | literal , op , attribute ;             (* flipped: 10 < @w is @w > 10 *)
op           = "=" | "!=" | "<" | "<=" | ">" | ">=" ;
literal      = STRING | NUMBER ;
attribute    = "@" , ( "style" , "." , KEY
                     | "layout" , [ "." , KEY ]       (* bare @layout is display *)
                     | ATTRNAME ) ;
KEY          = NAME matching /^[a-zA-Z][a-zA-Z0-9]*$/ ;   (* camelCase CSS property: no hyphens *)
ATTRNAME     = "id" | "name" | "type" | "text" | "src" | "alt" | "width" | "height" | "fill"
             | "color" | "background" | "layout" | "direction" | "position" | "children"
             | "component" | "componentId" | "lint" | "lintCount" ;
```

**Rules on top of the grammar:**
- **`<`, `<=`, `>` and `>=` need a NUMBER literal.** A STRING is an error.
  - The check runs *after* a left-hand literal is flipped, so `["a" < @name]` reports `>` (see the errors file, §Quirks).
- **Both sides can't be attributes.** `@a = @b` is an error.
- **A bare name** (`[name]`, `[last]`, `[not @x]`) is rejected as a "child element test".
- **A literal on its own** (`["x"]`) is an error: it needs an op.
- **`last()` is a position only**, as the whole predicate `[last()]`. Anywhere else it is an error. That includes `[last()` with no `]`, because the four-token lookahead fails and the parser falls through to the call path.
- **Positions:** `[1.0]` and `[01]` are valid (position 1). `[0]`, `[-1]` and `[1.5]` are errors.
- **Empty input:** a query that is empty or only whitespace is an error. So is one longer than 1,000 UTF-16 code units, checked before lexing.

## 5. The parsed form

```
Query     = { text, steps: Step[] }
Step      = { axis: child | descendant, test: Element | "*", predicates: Predicate[] }
Predicate = Position(n) | Last | Expr(expr)
Expr      = Exists(attr) | Cmp(attr, op, string|number) | Fn(contains|starts-with, attr, string)
          | Not(expr) | And(l, r) | Or(l, r)
Attr      = { name, key? }        (* @style.fontSize → {name: "style", key: "fontSize"} *)
```

`queryUsesLint(q)` is true when any predicate expression, at any depth, reads `@lint` or `@lintCount`. Only expressions are checked, since positions can't read an attribute.

## 6. Node tests

| Test | Matches |
|---|---|
| `FRAME` | `type = frame`, and not a component master root |
| `BOX` | `type = box`, and not a component master root |
| `TEXT` | `type = text` (a master root of this type is `COMPONENT`, not `TEXT`) |
| `IMAGE` | `type = image` (likewise) |
| `INSTANCE` | `type = instance` (likewise) |
| `COMPONENT` | any node that is some `ComponentDef.rootId`, whatever its type |
| `*` | any node |

`elementName(node)` is `COMPONENT` when the node is a master root, and `upper(type)` otherwise. It is also the `element` field in MCP results. The children of a master are ordinary nodes (`//COMPONENT/TEXT` finds the master's text), and so are an instance's.

**[MAP]** FigCraft has 17 kinds (frame, group, rectangle, ellipse, vector, text, component, component set, instance …). Decide which names to accept, and keep the *closed-set* rule: an unknown test is an error, with a "did you mean" hint when the upper-cased input is a known test. Scrim has no GROUP. If the port adds tests, extend the error message's list (it is built from the list).

## 7. Attributes

**Absent:** an attribute is absent when its value is `undefined`. For CSS reads, an empty string `''` also counts as absent. `name`, `text`, `src` and `alt` are read raw, so `name: ""` and `text: ""` are **present** (pinned).

**`decl(key)`** is the declared CSS value: `style[key]`, if present and non-empty, or else `layout[key]`, if present and non-empty. In other words the two maps are merged with `style` last, as the canvas and the export merge them.

| Attribute | Value | [MAP] for FigCraft |
|---|---|---|
| `@id` | `node.id` | id as string |
| `@name` | `node.name` (raw; `""` is present) | layer name |
| `@type` | `node.type`, lower case: `frame box text image instance`. A master still says `frame`. | its own kind names. Document them. |
| `@text` | `node.text` (text nodes; raw) | characters |
| `@src` | `node.src` (image nodes) | image ref / hash |
| `@alt` | `node.alt` | none: absent, or a new field |
| `@width`, `@height` | `decl("width")`, `decl("height")` **as stored**, never measured: `240`, `"120px"`, `"50%"`, `"fit-content"` | **[MAP]** FigCraft stores resolved sizes. Decide: the stored size (a number), or the sizing mode (`hug` / `fill` / fixed). |
| `@fill` | text: `decl("color")`; other nodes: `decl("background") ?? decl("backgroundColor")` | **[MAP]** the first visible solid fill, as a CSS colour string |
| `@color` | `decl("color")` | **[MAP]** the text fill |
| `@background` | `decl("background") ?? decl("backgroundColor")`, for any type, text included | **[MAP]** |
| `@layout` (bare) | `decl("display")`: `flex`, `inline-flex`, `grid`, `block` … | **[MAP]** auto-layout on gives `flex`, and none gives absent or `block`? Decide. |
| `@direction` | `decl("flexDirection")`, or `"row"` when `decl("display")` is `flex` or `inline-flex` and no direction is set; otherwise absent | **[MAP]** horizontal is `row`, vertical is `column` |
| `@position` | `decl("position")` | **[MAP]** absolute-in-auto-layout gives `absolute` |
| `@children` | `childIds.length` (a number; **always present**) | child count |
| `@component` | for an instance, `components[node.componentId].name`; for a master root, its def's name; otherwise absent (also when the def is missing) | main component name |
| `@componentId` | the same lookup, giving the def's `id` | main component id |
| `@lint` | the caller's rule ids for the node: **deduplicated, sorted (code-unit order), joined with `,`**. Absent when there are none. | your linter's ids |
| `@lintCount` | the caller's list length, **not deduplicated** (always present when lint is supplied) | |
| `@style.<key>`, `@layout.<key>` | `decl(key)`. The two prefixes read **the same merged value** (`@layout.width` = `@style.width`). `@layout.display` = `@layout`. | **[MAP]** the biggest one: Scrim exposes arbitrary CSS keys. FigCraft needs a fixed table from CSS keys to its properties (`fontSize`, `opacity`, `gap`/itemSpacing, `padding*`, `borderRadius`/cornerRadius …), or a `@prop.<key>` namespace of its own. |

Node values are strings or numbers. The query never sees booleans: `[@x = false]` is an error ("quote strings").

**Measured geometry is never a predicate.** Width and height are the stored values, so the CLI (with no canvas) and the app give the same answer. Rects come back only in the MCP *results*.

## 8. Predicate semantics

**`Exists(a)`:** true when the value is present.

**`Cmp(a, op, lit)`:**
1. **Absent value:** the comparison is **false, for every op, `!=` included** (XPath).
2. **NUMBER literal:** the value is converted with `numeric(v)`:
   - a number is itself (if finite);
   - a string is trimmed, then must match `^(-?(?:\d+(?:\.\d+)?|\.\d+))(?:px)?$`. That accepts `"120px"`, `" 240px "` and `"-.5px"`, and rejects `"240PX"`, `"50%"`, `"1e2"`, `"+5"` and `"fit-content"`.

   If the value doesn't convert, the result is **false for every op, `!=` included**: `[@width != 5]` doesn't match `"50%"`. Otherwise it is an ordinary numeric comparison.
3. **STRING literal** (only `=` or `!=` reach here): `String(v) == lit` or `String(v) != lit`, exact and case-sensitive. A number is stringified the JavaScript way: `56` gives `"56"`, `0.5` gives `"0.5"` (and `1e21` gives `"1e+21"`). **[Rust]** Match JS `Number.prototype.toString` for parity on stored floats. So `[@children = "3"]` and `[@width = "320"]` match, and `[@width = "120"]` doesn't match `"120px"`.

**`Fn(contains|starts-with, a, s)`:** false when absent; otherwise a substring or prefix test on `String(v)`. It is case-sensitive, and `""` matches every present value.

**Logic:** `Not`, `And` and `Or` are the boolean operations (short-circuit is fine: there are no side effects).

**Positions:** see §9. XPath semantics: **positions count the matches under the same parent, not across the document.**

## 9. Evaluation

```
masters = { def.rootId for each def in components }
context = [DOCUMENT]                                  # a virtual root; children(DOCUMENT) = rootIds
for step in steps:
    out = ordered set
    if step.axis == child:
        for c in context: out += group(children(c), step)
    else:                                             # descendant
        seen = set
        for c in context:
            for d in preorder(c), including c, skipping nodes already in seen:
                out += group(children(d), step)
    context = out
return documentOrder(context)

group(childIds, step):
    ids = [id in childIds if nodes[id] exists and (step.test == "*" or elementName(id) == step.test)]
    for p in step.predicates (in order):
        Position(n): ids = ids.length >= n ? [ids[n-1]] : []
        Last:        ids = ids.length > 0 ? [ids[last]] : []
        Expr(e):     ids = [id in ids if test(id, e)]
    return ids
```

**What follows from it:**
- **Each node is visited at most once per `//` step,** however many contexts reach it (`seen`). The evaluation is linear in the number of nodes for each step.
- **Predicates apply in order to the per-parent list.**
  - `//*[1][@type="text"]` keeps first children that are text.
  - `//*[@type="text"][1]` keeps the first text child of each parent.
  - `//TEXT[1]` is every parent's first TEXT child.
  - `/FRAME[2]` is the second top-level FRAME.
- **A child id missing from `nodes` is skipped** (no error).
- **`documentOrder`** is a pre-order DFS from `rootIds` in order (a parent before its children, children in `childIds` order). It keeps only the hits, each once, and stops early once all are found. A node not reachable from `rootIds` is never returned, because it can never be matched.
- **Lint:** when `queryUsesLint(q)` is true and no lint function was supplied, the result is an error (in core only; MCP always supplies one). The lint function is called lazily, per node.

## 10. Result shape (core)

The result is `NodeId[]`: each match once, in document order, uncapped. It is empty when nothing matches. An error is a `QueryError { message, at }`: `at` is the 0-based offset or null, and `message` already ends with ` (at character N)` when `at` is set.

## 11. The MCP tool

**Name:** `query_nodes`.

**Description (verbatim, one line):**

```text
Find nodes with an XPath-style query and get back their ids, names, types, parent ids and measured border-box rects {x,y,w,h} (with measuredAt and stale, as get_node) — use it instead of walking get_document_outline when you know what you are looking for. Read-only. Returns { query, total, returned, truncated, results }; at most limit results (default 100, max 1000) in document order, total counts every match. Paths: /X is a top-level artboard, //X anywhere below; steps chain with / (child) and // (descendant). Node tests: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT (a component master), * (any). Predicates in [ ]: [@a] exists; [@a="x"] / [@a!="x"]; [@a > 10] (< <= > >= compare numbers or px lengths); contains(@a,"x"), starts-with(@a,"x"); and, or, not( ), ( ); [n] and [last()] are positions among the matches under the same parent. Attributes (document fields, the same in the CLI): @id @name @type @text @src @alt @width @height (as declared, e.g. 240 or "100%") @fill (colour for text, background otherwise) @color @background @layout (display) @direction @position @children (count) @component @componentId @lint (active lint rule ids, comma-joined) @lintCount, and any declared CSS property as @style.<key> or @layout.<key> (layout and style merged, as the canvas renders them). Anything else (.., named axes, unions, other functions) is an error that says so. Example: //FRAME[@layout="flex"]//TEXT[contains(@name,"Price")].
```

**Input schema (as listed by the server):**

```json
{ "type": "object",
  "properties": {
    "query": { "type": "string", "description": "e.g. //TEXT[@style.fontSize > 40] or /FRAME[@name=\"Pricing\"]//INSTANCE" },
    "limit": { "type": "integer", "minimum": 1, "maximum": 1000 } },
  "required": ["query"], "additionalProperties": false }
```

**Annotations:** `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`, `openWorldHint: false`.

**Behaviour (`runQuery`):**
1. `parseQuery(query)`.
2. If the query reads lint, run the document linter **once** for this call: the whole document, with the last-known measured rects, keeping only *active* findings (not reason-ignored, not informational, not relaxed by the artboard's lint profile). Group the rule ids by node id, keeping duplicates.
3. `ids = queryNodes(doc, parsed, lint?)`.
4. `cap = max(1, min(1000, floor(limit ?? 100)))`, then take `ids[0..cap]`.

**Success:** a single text content item holding `JSON.stringify(result, null, 2)` (2-space indent, no trailing newline). The fields are, in this order:

```
{ "query": <the input string>, "total": <every match>, "returned": <results.length>,
  "truncated": total > returned,
  "results": [ { "id", "name", "type", "element", "parentId",
                 "rect": {"x","y","w","h"} | null, "measuredAt": ISO-8601 | null,
                 "stale": bool, "culled"?: true } ] }
```

- **`type`** is the raw node type, and **`element`** is the node-test name (`COMPONENT` for a master).
- **`parentId`** is `null` for an artboard.
- **The rect fields** are the node's border-box rect as last measured by the editor, the same as `get_node` returns:
  - each value is rounded to 2 decimals;
  - `stale` is true when there is no rect, the artboard changed since it was measured, or the rect is last-known because the artboard is culled;
  - `culled: true` appears only in that last case.
  - **Never measured:** `rect: null, measuredAt: null, stale: true`. The CLI always gets this, because no editor is attached.
  - **[MAP]** FigCraft can compute rects from its own layout, so it may never be stale.

**Errors:**
- **A `QueryError`:** the result has `isError: true` and the text `invalid query: <message>`.
- **Schema violations** (`limit` 0, 1001 or 2.5; a missing `query`) are rejected by the SDK before the handler runs. The texts are SDK-generated (zod). They are recorded in the cases as informational and are not part of the contract.

## 12. The CLI

```
scrim query <file> '<query>' [--limit N] [--json]
scrim find  <file> [--type T] [--name text] [--limit N] [--json]
```

- **How it runs:** both commands call the `query_nodes` tool on an in-process MCP server, over a read-only store made from the file.
- **`--json`:** stdout is exactly the tool text, followed by `\n`.
- **`find`:** builds `//<T or *>` plus `[contains(@name, <lit>)]` when `--name` is given.
  - **`T`** is upper-cased and must be in `FRAME BOX TEXT IMAGE INSTANCE COMPONENT`. `*` is refused.
  - **`<lit>`** is `"name"`, or `'name'` when the name contains `"`. A name with both quote kinds is a usage error.
- **`--limit`:** `Number(raw)` must be an integer from 1 to 1000, checked before the tool is called. JS `Number` accepts `1e2`, `0x10` and ` 5 `. **[Rust]** Either accept the same, or document a stricter parse.
- **Flags:** `--flag value` and `--flag=value` both work. `--json` can go anywhere.
- **Positionals:** exactly one for `find` (the file) and two for `query`.

**Human output (stdout):**

```
<head>
  <ELEMENT padded to 9> <name>  (<id>)        # one line per returned result
```

`<head>` is `N match` / `N matches`, or `R of T matches (--limit for more)` when truncated. For example: `6 matches\n  TEXT      Logo  (logo)\n…`, `1 match\n  COMPONENT Card  (card)\n` and `0 matches\n`.

**Errors and exit codes:**

| Exit | When | `--json` stdout | otherwise stderr |
|---|---|---|---|
| 0 | done | the tool text | (none) |
| 1 | an invalid query (code `invalid_query`), or another tool error | `{"error":{"code","message"}}`, 2-space indented | `scrim: <message>` |
| 2 | usage (code `usage`) | the same | the same |
| 3 | the file (`not_found`, `not_a_file`, `too_large` over 200 MB, `unreadable`, `invalid_document`) | the same | the same |

**An invalid query's message is the tool text,** including the `invalid query: ` prefix. The CLI's own messages are in `query-nodes-errors.md` §CLI.

## 13. Limits and caps

| Limit | Value | Where |
|---|---|---|
| Query length | 1,000 UTF-16 code units; longer is an error | parser, before lexing |
| Results per call | default 100, min 1, max 1,000; `total` is uncapped | MCP |
| Depth | none: an explicit stack, no recursion over the tree | evaluator |
| Timeout | none: the evaluation is linear | evaluator |
| CLI file size | 200 MB, refused from `stat` before it is read | CLI |

The parser recurses only on parenthesis and `not()` nesting. That nesting is bounded by the 1,000-character limit (at most about 500 levels). **[Rust]** Recursion this deep is safe on the default stack, but keep the length check before parsing.

## 14. Performance (`docs/research/query-nodes-timing.md`)

- **Measured** with `pnpm stress:query`, through `runQuery` (parse, evaluate, cap at 100, format), on 20,000-node documents: 3 warm-ups, then 30 runs. The machine was unfit (disk 97% full, 16 GB of swap), so the numbers are **INVALID** and should be read as an upper bound.
- **The budget:** a typical query well under 100 ms.
- **Non-lint queries:** medians of 0.04–7.8 ms, worst run 18 ms. Examples:
  - `//TEXT`: 2.7–4.3 ms;
  - `//FRAME[@layout="flex"]//TEXT[contains(@name,"Title")]`: 3.4–7.8 ms;
  - `/FRAME[3]//BOX[@width > 100]`: 0.04–0.2 ms;
  - `//*[@position="absolute" and not(@type="box")]`: 3.7–7.8 ms.
- **`@lint` queries:** 69–79 ms median. The time goes to the whole-document lint pass, not to the query. A per-revision lint cache would remove it, but none is built.
- **The design point to keep:** a `//` step visits each node once, whatever the number of contexts. Without the `seen` set, `//*//*//TEXT` would be quadratic.

## 15. Quirks and gaps to decide (reported, not fixed in Scrim)

See `query-nodes-errors.md` §Quirks for the error-message quirks. The behavioural ones:
1. **`@lint` and `@lintCount` disagree on duplicates.** `@lint` deduplicates and `@lintCount` doesn't. The MCP layer passes one id per finding, so a rule firing twice on a node gives `@lint = "r"` but `@lintCount = 2`.
2. **Number stringification is JavaScript's,** for `=` and `!=` with a STRING literal, and for `contains()` on numbers.
3. **Offsets and the length limit count UTF-16 code units.** An `unexpected '<c>'` error on an astral character would show half a surrogate pair. The cases avoid it because it can't round-trip through JSON parsers like serde.
4. **`[1.0]` and `[01]` are accepted as positions,** and `1e3` and `10px` are lexing errors.
5. **`@layout.<key>` and `@style.<key>` are the same lookup.** The prefix doesn't select the map.
6. **No escapes in strings.** A value holding both `'` and `"` can't be compared with `=`. Use `contains()` on a part of it.
