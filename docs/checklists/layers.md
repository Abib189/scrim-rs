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
| A Layers row click selects and flies the camera to the layer: 320 ms eased, the zoom kept, a 32 px margin, the least move (none when it's in view; a big layer's leading edge at the margin) | Click a row far off screen | `camera_flight_tests.rs` › `a_row_click_selects_and_flies_the_camera_there`; `cmd/camera_tests.rs` (ported from the old `cameraNav.test.tsx`) |
| A jump with Settings ▸ Animations off (or the system's reduced motion) | Animations: Off, click a row | `with_animations_off_it_jumps` |
| Any input mid-flight (scroll, pan, zoom, drag) stops it; the camera stays where the person put it | Click a row, scroll at once | `a_scroll_mid_flight_stops_it_where_the_person_put_it` |
| A canvas click never moves the camera | Click a layer on the canvas | `a_canvas_click_never_moves_the_camera` |
| A focus asked for before the canvas has its size lands after the first sizing (not the page fit) | Agent: `camera.focus-layer` as the file opens | `a_focus_before_the_canvas_has_its_size_lands_after_it` |
| Enter on a focused row focuses its layer; a multi-selection frames the union | F6 to Layers, arrows, Enter; Shift-click two rows | `enter_on_a_focused_row_focuses_its_layer`, `a_multi_selection_frames_the_union` |
| A hidden layer is focused, and a toast says it is hidden; a layer on another page shows that page first | Hide a layer, click its row; `camera.focus-layer` on another page's layer | `a_hidden_layer_is_focused_with_a_toast`; `another_page_is_shown_first_and_hidden_layers_are_focused_and_reported` |
