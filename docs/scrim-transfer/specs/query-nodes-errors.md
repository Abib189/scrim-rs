# Spec: `query_nodes` error messages

The companion to `query-nodes.md`. It holds every message the query, the `query_nodes` tool and the CLI's `query` / `find` commands can produce, verbatim. Placeholders are in `<angle brackets>`. Each example in §Examples was produced by running Scrim, and is a case in `../cases/query-nodes.json`.

## Position suffix

A message with a position gets ` (at character N)` appended, where N = 0-based offset + 1, counted in **UTF-16 code units**. The *Pos.* column below says which token's offset is used. Messages marked "no position" never carry the suffix.

Over MCP, every query error becomes a tool error (`isError: true`) whose text is `invalid query: ` followed by the full message, suffix included.

## Templates

**Before lexing (no position):**

| # | Message | Trigger |
|---|---|---|
| E1 | `a query is a path such as //FRAME or /FRAME//TEXT[@name="Title"]` | the query is empty, only whitespace, or not a string |
| E2 | `a query is at most 1000 characters` | longer than 1,000 UTF-16 code units |

**Lexer:**

| # | Message | Pos. | Trigger |
|---|---|---|---|
| E3 | `named axes (child::, ancestor:: …) are not supported: use / for a child and // for a descendant` | the `::` | `::` anywhere (the `…` is U+2026) |
| E4 | `'..' (parent) is not supported: queries walk down from the document with / and //` | the `..` | `..` anywhere |
| E5 | `unterminated string` | the opening quote | `"` or `'` with no matching closing quote |
| E6 | `unions ('\|') are not supported: run two queries` | the `\|` | `\|` (the message has a bare `\|`; it is escaped here only for the table) |
| E7 | `variables are not supported` | the `$` | `$` |
| E8 | `unexpected '<c>'` | the character | any other character, e.g. `#`, `€`, `!` alone, `-` not starting a number. `<c>` is one UTF-16 code unit. |

**Paths and node tests:**

| # | Message | Pos. | Trigger |
|---|---|---|---|
| E9 | `a query starts with / (top-level artboards) or // (anywhere): relative paths are not supported` | the first token | the first token isn't `/` or `//`: `FRAME`, `./TEXT`, `@name` |
| E10 | `'.' (self) is not supported` | the `.` | a `.` as a node test (`//.`) or after a step (`//TEXT.`) |
| E11 | `expected / or // or the end of the query` | the token | anything else after a complete step: `//TEXT[1]x`, `//TEXT[@a="x"]]` |
| E12 | `attributes are selected in a predicate: //FRAME[@name="Card"], not //FRAME/@name` | the `@` | `@` as a node test |
| E13 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` | the token | END, `/` or any other non-name after an axis: `/`, `//`, `///TEXT`, `//TEXT/` |
| E14 | `<name>() is not a node test: functions go inside [ ]` | the name | a node-test name followed by `(`: `//contains(@name, "x")` |
| E15 | `unknown node test '<name>' (did you mean <NAME>?): use FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` | the name | upper-casing `<name>` gives a valid test: `//frame`, `//Frame` |
| E16 | `unknown node test '<name>': use FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` | the name | any other name: `//GROUP` |

**Predicates:**

| # | Message | Pos. | Trigger |
|---|---|---|---|
| E17 | `a position is a whole number from 1` | the number | `[n]` where n isn't an integer ≥ 1: `[0]`, `[-1]`, `[1.5]` (`[1.0]` is valid) |
| E18 | `expected ']'` | the token found | a predicate's expression ends but no `]` follows: `[@a="x"`, `[@a='x' = 'y']`, `[@a=1e3]`, `[@w > 10px]`, `[@a="a" AND …]` |
| E19 | `expected ')'` | the token found | an unclosed `(` group or `not(`; `contains(@a, "x"` with no `)`; `starts-with(@a, "x", "y")` |
| E20 | `expected ',' then a quoted string` | the token found | `contains(@name)` |
| E21 | `<fn>() compares with a quoted string` | the token after `,` | `contains(@name, 3)` (`<fn>` is `contains` or `starts-with`) |
| E22 | `unknown function <name>(): the subset has contains(), starts-with(), not() and last()` | the name | a name followed by `(` that isn't one of the four, case-sensitively: `count(`, `position(`, `NOT(`, `Contains(` |
| E23 | `last() is a position on its own: [last()]` | `last` | `last(` anywhere except as the exact predicate `[last()]`: `[last() = 1]`, or `[last()` without its `]` |
| E24 | `'<and\|or>' needs a condition on each side` | the keyword | `and` or `or` where a condition should start: `[and @name]` |
| E25 | `'<name>' on its own is a child element test, which predicates don't support: compare an attribute (@<name>) or quote a string ('<name>')` | the name | a bare name where a condition should start: `[name]`, `[last]`, `[not @name]` |
| E26 | `expected a comparison (= != < <= > >=)` | the token found | a literal on the left with no op after it: `["x"]` |
| E27 | `expected an attribute (@name)` | the token found | no `@` where an attribute is required: `[]`, `[@a and]`, `[@a or]`, `[10 < 20]`, `[not()]`, `contains("x", @a)` |
| E28 | `compare an attribute with a literal, not with another attribute` | the second `@` | `[@name = @text]` |
| E29 | `'<name>' is not a value: quote strings ('<name>')` | the name | an unquoted word as the right-hand value: `[@name = Title]`, `[@name = false]` |
| E30 | `expected a quoted string or a number` | the token found | an op with nothing usable after it: `[@name =]`, `[@name == "x"]` |
| E31 | `unknown attribute @<name>: use @id, @name, @type, @text, @src, @alt, @width, @height, @fill, @color, @background, @layout, @direction, @position, @children, @component, @componentId, @lint, @lintCount, @style.<key> or @layout.<key>` | the `@` | `@colour`, `@visible`, any name not in the list. Here `<key>` is literal text. |
| E32 | `expected an attribute name after @` | the token found | `[@]`, `[@"x"]` |
| E33 | `@style needs a property: @style.fontSize` | `style` | `@style` with no `.` after it, including `@style.1x` (`.1` lexes as a number) |
| E34 | `@<style\|layout>. needs a CSS property in camelCase (@<style\|layout>.fontSize)` | the token after `.` | the key is missing or isn't `[a-zA-Z][a-zA-Z0-9]*`: `@style.font-size`, `@style.]`, `@layout.]` |
| E35 | `<op> compares numbers: write a number (@width > 240)` | the operator | `<`, `<=`, `>` or `>=` with a string literal. `<op>` is the operator **after** a left-hand literal is flipped (see Quirk 1). |

**Evaluator (no position):**

| # | Message | Trigger |
|---|---|---|
| E36 | `@lint and @lintCount need lint results: query through MCP (query_nodes) or the CLI` | the query reads `@lint` or `@lintCount` and the caller supplied no lint function. This is core-only: MCP and the CLI always supply one. |

**Unreachable:** `expected '('` exists in the source but can't fire, because a call is only entered when the next token is `(`.

## MCP

| Situation | Tool result |
|---|---|
| any E1–E35 | `isError: true`, text `invalid query: <message>` |
| `limit` not an integer in 1..1000, `query` missing or not a string | rejected by the MCP SDK's schema validation before the handler runs. The text is SDK-generated (`MCP error -32602: Input validation error: Invalid arguments for tool query_nodes: [ …zod issues… ]`); it is recorded in `toolCases` but is **not** part of the contract. |

## CLI

The CLI's own messages, verbatim. With `--json` they go on stdout as `{"error": {"code", "message"}}`, 2-space indented. Without it they go on stderr as `scrim: <message>`.

| Exit | Code | Message | Trigger |
|---|---|---|---|
| 1 | `invalid_query` | `invalid query: <message>` | any query error (the tool's text) |
| 2 | `usage` | `find needs --type and/or --name` | `find` with neither flag |
| 2 | `usage` | `--type is one of FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT` | `--type` not in the list after upper-casing (`*` too) |
| 2 | `usage` | `--name can hold single or double quotes, not both` | `--name` contains both `'` and `"` |
| 2 | `usage` | `--limit is a whole number from 1 to 1000` | `Number(raw)` is not an integer from 1 to 1000 |
| 2 | `usage` | `--<flag> needs a value` | a value flag at the end of argv |
| 2 | `usage` | `usage: scrim query <file> '<query>'` | `query` without exactly two positionals |
| 2 | `usage` | `usage: scrim find <file> [--type T] [--name text]` | `find` without exactly one positional |
| 2 | `usage` | `query has no --<flag> (it takes --limit, --json)` | an unknown flag on `query` |
| 2 | `usage` | `find has no --<flag> (it takes --type, --name, --limit, --json)` | an unknown flag on `find` |
| 2 | `usage` | `unknown command '<name>': tree, find, query, info, lint, export, analyze, import-fig` | an unknown command |
| 3 | `not_found` | `no such file: <path>` | `stat` fails |
| 3 | `not_a_file` | `<path> is not a file` | a directory |
| 3 | `too_large` | `“<basename>” is <MB> MB: Scrim opens .scrim files up to 200 MB` | over 200 MB (curly quotes; MB = bytes / 1048576, 0 decimals) |
| 3 | `unreadable` | `can't read <path>` | the read fails |
| 3 | `invalid_document` | `<basename>: <document validation message>` | not a Scrim document (that validator is out of scope here) |

## Quirks (reported, not fixed)

1. **E35 shows the flipped operator.** `["a" < @name]` reports `> compares numbers…`, not `<`, because the literal is moved to the right first.
2. **E25's suggestions can mislead.**
   - For `[last]` it suggests `@last`, which is not an attribute.
   - For `[not @name]` it calls `not` a child element test, when the user meant the function.
3. **E23 fires for a missing `]`.** `//TEXT[last()` says "last() is a position on its own: [last()]", which is what the user wrote.
4. **E34's suggestion is odd for layout.** For `@layout.` it suggests `@layout.fontSize`, which is not a layout property, although it does read the value.
5. **E33 vs E34 depends on the lexer.** `@style.1x` gives E33, because `.1` is a NUMBER.
6. **E8 can split a character.** On an astral character, `<c>` is half a surrogate pair. A Rust port printing the whole `char` is a reasonable divergence.

## Examples (generated)

Each row is a real output: the query, the reported character (or – for none), and the message without its ` (at character N)` suffix. The full strings, suffix included, are in the cases file. Long messages ending in E31's attribute list are shortened to "…(E31's list)".

| Query | Char | Message |
|---|---|---|
| `//*[@lint]` | – | `@lint and @lintCount need lint results: query through MCP (query_nodes) or the CLI` |
| `//*[not(@lintCount > 0)]` | – | `@lint and @lintCount need lint results: query through MCP (query_nodes) or the CLI` |
| `//*[@name = 'it's "quoted"']` | 27 | `unterminated string` |
| (empty) | – | `a query is a path such as //FRAME or /FRAME//TEXT[@name="Title"]` |
| (three spaces) | – | `a query is a path such as //FRAME or /FRAME//TEXT[@name="Title"]` |
| `FRAME` | 1 | `a query starts with / (top-level artboards) or // (anywhere): relative paths are not supported` |
| `./TEXT` | 1 | `a query starts with / (top-level artboards) or // (anywhere): relative paths are not supported` |
| `//frame` | 3 | `unknown node test 'frame' (did you mean FRAME?): use FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//text` | 3 | `unknown node test 'text' (did you mean TEXT?): use FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//Frame` | 3 | `unknown node test 'Frame' (did you mean FRAME?): use FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//GROUP` | 3 | `unknown node test 'GROUP': use FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//TEXT/..` | 8 | `'..' (parent) is not supported: queries walk down from the document with / and //` |
| `//child::TEXT` | 8 | `named axes (child::, ancestor:: …) are not supported: use / for a child and // for a descendant` |
| `//TEXT \| //BOX` | 8 | `unions ('\|') are not supported: run two queries` |
| `//TEXT[@colour="red"]` | 8 | `unknown attribute @colour: use …(E31's list)` |
| `//TEXT[@style]` | 9 | `@style needs a property: @style.fontSize` |
| `//TEXT[@name = Title]` | 16 | `'Title' is not a value: quote strings ('Title')` |
| `//TEXT[@name = false]` | 16 | `'false' is not a value: quote strings ('false')` |
| `//TEXT[@visible = "false"]` | 8 | `unknown attribute @visible: use …(E31's list)` |
| `//TEXT[name]` | 8 | `'name' on its own is a child element test, which predicates don't support: compare an attribute (@name) or quote a string ('name')` |
| `//TEXT[@name > "a"]` | 14 | `> compares numbers: write a number (@width > 240)` |
| `//TEXT[@name = @text]` | 16 | `compare an attribute with a literal, not with another attribute` |
| `//TEXT[count(*) > 1]` | 8 | `unknown function count(): the subset has contains(), starts-with(), not() and last()` |
| `//TEXT[last]` | 8 | `'last' on its own is a child element test, which predicates don't support: compare an attribute (@last) or quote a string ('last')` |
| `//TEXT[0]` | 8 | `a position is a whole number from 1` |
| `//TEXT[1.5]` | 8 | `a position is a whole number from 1` |
| `//TEXT[-1]` | 8 | `a position is a whole number from 1` |
| `//TEXT[@name="x"` | 17 | `expected ']'` |
| `//TEXT[contains(@name)]` | 22 | `expected ',' then a quoted string` |
| `//TEXT[contains(@name, 3)]` | 24 | `contains() compares with a quoted string` |
| `//TEXT[@name="x]` | 14 | `unterminated string` |
| `//TEXT/@name` | 8 | `attributes are selected in a predicate: //FRAME[@name="Card"], not //FRAME/@name` |
| `//TEXT[@name and]` | 17 | `expected an attribute (@name)` |
| `//TEXT[$x]` | 8 | `variables are not supported` |
| `//TEXT#` | 7 | `unexpected '#'` |
| `//TEXT[@colour]` | 8 | `unknown attribute @colour: use …(E31's list)` |
| `/` | 2 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//` | 3 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `///TEXT` | 3 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//TEXT/` | 8 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//TEXT//` | 9 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//.` | 3 | `'.' (self) is not supported` |
| `//TEXT/.` | 8 | `'.' (self) is not supported` |
| `//TEXT.` | 7 | `'.' (self) is not supported` |
| `//TEXT[1]x` | 10 | `expected / or // or the end of the query` |
| `//TEXT[@name="x"]]` | 18 | `expected / or // or the end of the query` |
| `//contains(@name, "x")` | 3 | `contains() is not a node test: functions go inside [ ]` |
| `//TEXT[@style.font-size = 12]` | 15 | `@style. needs a CSS property in camelCase (@style.fontSize)` |
| `//TEXT[@style.]` | 15 | `@style. needs a CSS property in camelCase (@style.fontSize)` |
| `//TEXT[@layout.]` | 16 | `@layout. needs a CSS property in camelCase (@layout.fontSize)` |
| `//TEXT[@style.1x]` | 9 | `@style needs a property: @style.fontSize` |
| `//TEXT[@]` | 9 | `expected an attribute name after @` |
| `//TEXT[@name =]` | 15 | `expected a quoted string or a number` |
| `//TEXT["a" < @name]` | 12 | `> compares numbers: write a number (@width > 240)` |
| `//TEXT[10 < 20]` | 13 | `expected an attribute (@name)` |
| `//TEXT["x"]` | 11 | `expected a comparison (= != < <= > >=)` |
| `//TEXT[last() = 1]` | 8 | `last() is a position on its own: [last()]` |
| `//TEXT[position() = 1]` | 8 | `unknown function position(): the subset has contains(), starts-with(), not() and last()` |
| `//TEXT[not @name]` | 8 | `'not' on its own is a child element test, which predicates don't support: compare an attribute (@not) or quote a string ('not')` |
| `//TEXT[and @name]` | 8 | `'and' needs a condition on each side` |
| `//TEXT[@name or]` | 16 | `expected an attribute (@name)` |
| `//TEXT[]` | 8 | `expected an attribute (@name)` |
| `//TEXT[(@name]` | 14 | `expected ')'` |
| `//TEXT[starts-with(@name, "x", "y")]` | 30 | `expected ')'` |
| `//TEXT[contains("x", @name)]` | 17 | `expected an attribute (@name)` |
| `//TEXT[@name = 'x' = 'y']` | 20 | `expected ']'` |
| `//TEXT€` | 7 | `unexpected '€'` |
| `//TEXT[@name=1e3]` | 15 | `expected ']'` |
| `//TEXT[@width > 10px]` | 19 | `expected ']'` |
| `//TEXT[@name="a" AND @name="b"]` | 18 | `expected ']'` |
| `//TEXT[NOT(@name)]` | 8 | `unknown function NOT(): the subset has contains(), starts-with(), not() and last()` |
| `//TEXT[Contains(@name, "x")]` | 8 | `unknown function Contains(): the subset has contains(), starts-with(), not() and last()` |
| `//TEXT[@name='it''s']` | 18 | `expected ']'` |
| `//ancestor::FRAME` | 11 | `named axes (child::, ancestor:: …) are not supported: use / for a child and // for a descendant` |
| `//TEXT[@name="x"] \| //BOX` | 19 | `unions ('\|') are not supported: run two queries` |
| `//TEXT[$1]` | 8 | `variables are not supported` |
| `//TEXT[@name = "x" or]` | 22 | `expected an attribute (@name)` |
| `//TEXT[not()]` | 12 | `expected an attribute (@name)` |
| `//TEXT[contains(@name, "x"]` | 27 | `expected ')'` |
| `//TEXT[@style.fontSize > "20"]` | 24 | `> compares numbers: write a number (@width > 240)` |
| `//TEXT[@width >= "1"]` | 15 | `>= compares numbers: write a number (@width > 240)` |
| `//TEXT[last()` | 8 | `last() is a position on its own: [last()]` |
| `@name` | 1 | `a query starts with / (top-level artboards) or // (anywhere): relative paths are not supported` |
| `//TEXT[@name ! "x"]` | 14 | `unexpected '!'` |
| `//TEXT[@name == "x"]` | 15 | `expected a quoted string or a number` |
| `//TEXT[@"x"]` | 9 | `expected an attribute name after @` |
| `//1` | 3 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//(` | 3 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//TEXT[@width > -x]` | 17 | `unexpected '-'` |
| `//*[@layout.5]` | 12 | `expected ']'` |
| `//TEXT[@name or and @id]` | 17 | `'and' needs a condition on each side` |
| `//TEXT[contains(@name, "x") = 1]` | 29 | `expected ']'` |
| `//TEXT[not(@name]` | 17 | `expected ')'` |
| `//TEXT[@name="x"]/` | 19 | `expected a node test: FRAME, BOX, TEXT, IMAGE, INSTANCE, COMPONENT or *` |
| `//BOX[@width >= 1 and]` | 22 | `expected an attribute (@name)` |
| `//*[@name="é"]#` | 15 | `unexpected '#'` |
| `//*[@name="😀"]#` | 16 | `unexpected '#'` |
