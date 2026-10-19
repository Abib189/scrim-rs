# `query_nodes`: the XPath-style layer query (Port 5)

The report: `docs/scrim-ports/query-nodes.md`.

| Check | How | Covered by |
|---|---|---|
| The grammar, every error message and position, and the evaluation match Scrim's (244 core cases) | Not manual | `crates/engine/src/cmd/query/scrim_cases_tests.rs` › `every_core_case_matches_scrim` |
| The tool's result and errors match Scrim's (18 of 19; the 19th needs the linter) | Not manual | `scrim_cases_tests.rs` › `every_tool_case_matches_scrim` |
| The CLI's output, exit codes and argument rules match Scrim's (24 of 25) | Not manual | `scrim_cases_tests.rs` › `every_cli_case_matches_scrim` |
| Node tests are FigCraft's kinds; positions count per page | `scrim query <file> '/*[1]'`: each page's first layer | `crates/engine/src/cmd/query/figcraft_tests.rs` › `node_tests_are_figcrafts_kinds_across_pages_in_document_order` |
| Attributes read the document (layout, fill, text, sizes, `@style.*`); `@alt` is an error | Query a file you know | `figcraft_tests.rs` › `attributes_read_the_document`, `hidden_and_locked_are_presence_attributes` |
| Read-only: the document is unchanged, with no change-feed entry and no undo step | Run a query from an agent, then check Edit ▸ Undo and the change feed | `figcraft_tests.rs` › `results_carry_the_document_box_and_never_change_the_document` |
| `limit`, truncation and `page` scope; bad parameters refused | Not manual | `figcraft_tests.rs` › `limit_and_page_scope` |
| MCP lists `query_nodes` as read-only; errors say what is supported | Ask an agent to find layers | `crates/mcp/tests/query_nodes.rs` |
| `scrim query` / `find` print, `--json`, and exit 1/2/3 | Run them on a file | `apps/figcraft-cli/tests/query.rs` |
