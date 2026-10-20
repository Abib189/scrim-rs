# Layers: collapse all, click to focus (Stage D)

The plan and the old app's behaviour: `docs/design/layers-collapse-and-focus.md`.

| Check | How | Covered by |
|---|---|---|
| Collapse all layers (the header button left of Search, ⌥L, Object ▸ Collapse layers) leaves only the top-level rows; a second press changes nothing; the camera doesn't move | Open nested frames, press the button twice | `layers_header_tests.rs` › `the_button_collapses_every_row_to_the_top_level_and_is_idempotent` |
| It keeps the selection, works with nothing selected or an empty file, and a partial fold folds the rest | Select a nested layer, collapse | `the_selection_survives_and_a_partial_fold_folds_the_rest`, `an_empty_document_and_nothing_selected_are_fine` |
| With a filter active the filtered list is unchanged; clearing it shows the collapsed tree | Filter "swatch", collapse, clear | `an_active_filter_keeps_its_list_and_the_collapse_shows_after` |
| Opening a row by hand afterwards opens only that row | Collapse, open one frame | `opening_a_row_by_hand_afterwards_opens_only_that_row` |
| The button is named "Collapse all layers" with ⌥L for screen readers and Tab reaches it | VoiceOver; F6 to Layers, Tab | `the_button_is_named_keyed_and_reachable_with_tab` |
| No undo step, no document write, no new revision, no "Agent: …" label (agents too) | Collapse, look at Edit ▸ Undo | `cmd/layers_tests.rs` › `collapse_all_is_panel_state_only` |
