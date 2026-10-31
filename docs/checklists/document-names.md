# Document names (Fix AF)

One name for a document everywhere (`docs/design/doc-name.md`).

| Check | How | Covered by |
|---|---|---|
| A file saved with the name "Untitled" goes by its file name | Open `zeichen-26.figcraft` whose stored name is "Untitled": Home's card, the tab, the left panel's header and the window title all read "zeichen-26" | `crates/ui-egui/src/doc_name_ui_tests.rs` › `a_file_saved_as_untitled_goes_by_its_file_name_everywhere`; `crates/engine/src/doc_name_tests.rs` › `a_file_saved_with_the_placeholder_goes_by_its_file_name` |
| A name someone gave wins | A file storing "Poster": Home (before opening), the tab, the header and the title read "Poster" | `doc_name_ui_tests.rs` › `a_name_someone_gave_shows_everywhere_instead_of_the_file_name` |
| Unsaved documents are numbered | New design file twice: "Untitled", then "Untitled 2" ("Sin título 2" in Spanish) | `doc_name_ui_tests.rs` › `unsaved_documents_are_untitled_then_untitled_2_in_english_and_spanish`; `doc_name_tests.rs` › `unsaved_documents_are_untitled_then_numbered` |
| Save as names a new document at once | Save a new document as "Spring poster": the tab, the header, the title and Home read "Spring poster" without reopening | `doc_name_ui_tests.rs` › `save_as_names_a_new_document_after_its_file_at_once` |
| Renaming leaves the file alone | File ▸ Rename, type a name, Enter: every place shows it; the file on disk keeps its name and bytes until saved | `doc_name_ui_tests.rs` › `rename_in_the_panel_names_the_document_and_leaves_the_file_alone` |
| Home's search and sort by name use the name shown | Sort recent files by name; search for a stored name that isn't in the path | `doc_name_ui_tests.rs` › `home_search_and_sort_by_name_use_the_name_shown` |
| Long names | A long name ends in "…" in the tab and the header; hovering shows it whole | `doc_name_ui_tests.rs` › `long_names_are_cut_with_an_ellipsis_and_shown_whole_in_a_tooltip` |
| Opening never writes | Open a file and close it without changes: its bytes are unchanged | `doc_name_tests.rs` › `a_file_saved_with_the_placeholder_goes_by_its_file_name` |
