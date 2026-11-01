# The .scrim format and imports (Fix AG)

`docs/design/scrim-format.md`.

| Check | How | Covered by |
|---|---|---|
| Save writes .scrim | File ▸ Save as on a new document: the dialog suggests "Untitled.scrim", .scrim first; the file saved reopens the same | `crates/ui-egui/src/save_as_tests.rs` › `save_as_menu_and_first_save_open_the_platform_picker_directly_with_both_extensions`; `crates/engine/src/importers_tests.rs` › `save_writes_scrim_and_a_second_save_is_byte_stable` |
| A .figcraft opens and is never written | Open a .figcraft, edit, Save: Save as opens with "<name>.scrim" in its folder; the .figcraft is unchanged | `save_as_tests.rs` › `save_on_a_figcraft_document_asks_for_a_scrim_beside_it_and_leaves_the_original`; `importers_tests.rs` › `a_figcraft_opens_unchanged_and_save_asks_for_a_scrim_beside_it` |
| Files are read by their contents | Rename a .scrim to .txt and open it; open the earlier Scrim's JSON .scrim; open garbage | `importers_tests.rs` › `files_are_known_by_their_contents_not_their_names` |
| A Sketch file imports with a report | Open `crates/format/tests/fixtures/sketch/sample.sketch`: a new unsaved document and a report with counts by severity, each warning's layer, Copy report | `crates/ui-egui/src/scrim_format_ui_tests.rs` › `a_sketch_import_shows_its_report_and_a_scrim_document_shows_none`; `crates/format/src/sketch/tests/` |
| Home names the formats | Home's Open file card reads "Open .scrim, .fig, .sketch, .svg or images" | `scrim_format_ui_tests.rs` › `home_names_the_formats_from_the_registry` |
| Finder knows .scrim | `plutil -p Scrim.app/Contents/Info.plist`: the `design.scrim.document` UTI and document type; double-click a .scrim | `xtask/src/bundle_macos_tests.rs` › `the_written_plist_declares_the_scrim_document_type_and_keeps_figcraft`; double-click: manual only (needs an installed app) |
| Agents | MCP `save_file` and `open_file` with .scrim; a Sketch import's warnings in `open_file`'s result | `crates/mcp/tests/scrim_format.rs` |
