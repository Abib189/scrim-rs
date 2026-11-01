# Penpot, FREE, closed formats and previews (Fix AH)

`docs/design/import-formats.md` › Built in Fix AH.

| Check | How | Covered by |
|---|---|---|
| A Penpot export imports with a report | Open `crates/format/tests/fixtures/penpot/sample.penpot`: a new unsaved document; the report lists the text, stroke and shadow left out and the instance imported as layers | `crates/format/src/penpot/tests.rs`; `crates/engine/src/importers_tests.rs` › `penpot_exports_open_by_their_contents_with_warnings` |
| Penpot is known by its contents | Rename the sample `.txt` and open it; a newer manifest version is refused, saying so | `importers_tests.rs` › `penpot_exports_open_by_their_contents_with_warnings` |
| A Lunacy FREE file imports with its auto layout | Open `crates/format/tests/fixtures/free/sample.free` | `crates/format/src/free/tests/`; `importers_tests.rs` › `free_and_sketch_files_go_to_their_own_importers` |
| Closed formats are explained, never opened | Drop or open a file named `x.afdesign` or `x.xd`: the dialog names the tool and says to export SVG; nothing opens | `crates/ui-egui/src/file_kinds_tests.rs`; `importers_tests.rs` › `closed_formats_are_refused_and_never_opened`; `apps/figcraft/src/first_screen_tests.rs` › `a_closed_format_named_at_launch_hands_back_its_refusal` |
| Home shows Sketch, Penpot and FREE previews | Put the samples in the recent files: their cards show their previews; a file without one shows the placeholder | `crates/ui-egui/src/scrim_format_ui_tests.rs` › `home_shows_a_sketch_files_preview_and_reads_it_again_only_when_it_changes`; `crates/format/src/thumbnail_tests.rs` › `design_files_give_the_preview_they_carry` |
