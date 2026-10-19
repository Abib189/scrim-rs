# Design lint and `analyze_design` (Port 6)

The report: `docs/scrim-ports/lint-analyze.md`.

| Check | How | Covered by |
|---|---|---|
| Every rule, profile, ignore and message matches Scrim's (88 cases, with measured rects where Scrim had them) | Not manual | `crates/engine/src/cmd/lint/scrim_cases_tests.rs` › `every_lint_case_matches_scrim` |
| Every reported rule is in the catalog, and the 11 unported ones never fire | Not manual | `scrim_cases_tests.rs` › `every_rule_a_check_names_is_in_the_catalog` |
| `analyze_design` matches Scrim's (8 cases) | Not manual | `crates/engine/src/cmd/analyze/scrim_cases_tests.rs` › `every_analyze_case_matches_scrim` |
| A FigCraft file's findings come from what its export would carry and from its boxes (tiny text, contrast, overlapping artboards) | `figcraft-cli lint <file>` on a file you know | `crates/engine/src/cmd/lint/figcraft_tests.rs` › `findings_come_from_the_layers_css_and_their_boxes` |
| Hidden layers are not linted | Hide a flagged layer and lint again | `figcraft_tests.rs` › `hidden_layers_are_not_linted` |
| An ignore is stored with the file and undone in one step; a reason is required | Ignore a finding through an agent, then Edit ▸ Undo | `figcraft_tests.rs` › `an_ignore_is_stored_in_the_document_and_undone_in_one_step` |
| An expressive artboard relaxes display rules, never contrast; only top-level layers take a profile | Not manual | `figcraft_tests.rs` › `an_expressive_artboard_relaxes_display_rules_but_never_contrast` |
| `query_nodes` reads active findings as `@lint` / `@lintCount` | `figcraft-cli query <file> '//*[@lint]'` | `figcraft_tests.rs` › `the_query_reads_active_findings` |
| `analyze_design` reads colours, type, spacing and repeats against variables and styles | `figcraft-cli analyze <file>` | `crates/engine/src/cmd/analyze/figcraft_tests.rs` |
| MCP lists the four tools; writing tools carry the lint hint; analysis never writes | Ask an agent to lint what it built | `crates/mcp/tests/lint_design.rs` |
| `figcraft-cli lint` / `analyze` print the tools' JSON | Run them on a file | `apps/figcraft-cli/tests/query.rs` › `lint_and_analyze_print_the_tools_json` |
| The Impeccable licence, NOTICE and attribution ship with the rules | Open `crates/engine/src/cmd/lint/design-rules/` | Manual only (files, not behaviour) |
