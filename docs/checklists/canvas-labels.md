# Canvas labels and status icons (Stage E)

The investigation and the plan: `docs/design/canvas-badges.md`.

| Check | How | Covered by |
|---|---|---|
| A frame's handoff status is a 14 px icon after its title, above the frame (never on it, never past its width): a ring with a check (Ready for dev), a half-filled ring (Changed), a solid disc with a check (Completed) | Mark frames in each state (Design ▸ status menu), zoom 10–100% | `crates/ui-egui/src/dev_icon_ui_tests.rs` › `each_status_is_an_icon_after_the_title_outside_the_frame`; `dev_icon_tests.rs` › `the_states_are_ready_changed_and_completed` |
| A long title is cut with an ellipsis before the icon | Rename a ready frame to a long name | `a_long_title_ends_before_the_icon_and_never_past_its_frame`; `dev_icon_tests.rs` › `a_title_is_cut_before_the_icon_and_the_icon_goes_first_then_the_label` |
| A frame too narrow on screen drops the icon first, then its label; an icon is never drawn on its own | Zoom out on a narrow ready frame | `a_narrow_frame_drops_the_icon_then_the_label` |
| A section's title and status icon sit above its top edge, outside it | Mark a section ready | `a_sections_title_and_icon_sit_above_it` |
| The icon's tooltip and screen-reader name say the status; pressing it selects and drags its own frame | Hover the icon; VoiceOver; press it | `the_icon_is_named_in_a_tooltip_and_pressing_it_selects_its_frame` |
| The icon colours read at 3:1 on both themes' canvases and both default page colours | Light and dark themes, light and dark pages | `dev_icon_tests.rs` › `every_status_icon_reads_on_both_canvases_and_both_default_pages` |
| Frame, component and section labels never overlap each other's text at any zoom; each stays within its frame's width just above it | Ridgeline app at 10%, 22%, 50%, 100%; zoom in and out | `crates/ui-egui/src/canvas_labels_ui_tests.rs` › `ridgeline_labels_never_overlap_at_10_22_50_and_100_percent`, `a_sweep_of_zooms_never_overlaps`; `canvas_labels_tests.rs` › `hundreds_of_scattered_labels_never_overlap` |
| Long titles, every status, right-to-left titles and narrow frames between others don't overlap | Rename, mark, add a narrow frame | `long_titles_every_state_rtl_and_narrow_frames_never_overlap` |
| A section's title wins its place; a child label at its corner moves along past it or hides (Components and Landscape) | Ridgeline at 22% | `canvas_labels_tests.rs` › `a_higher_priority_label_pushes_a_lower_one_along_or_hides_it`; `ridgeline_labels_never_overlap_at_10_22_50_and_100_percent` |
| A label's frame's width is its room: Home — Light's status no longer lands on Trail — Light's title | Ridgeline at 22% | `canvas_labels_tests.rs` › `ridgeline_at_22_percent_keeps_each_label_on_its_own_frame` |
| Pressing a label, wherever the layout put it, selects (and drags) its frame | Press each label | `pressing_a_moved_label_still_selects_its_frame` |
| Label text reads at 4.5:1 on the page behind it, in both themes, on light and dark pages | Light theme on Ridgeline's dark page | `label_text_reads_on_both_canvases_and_both_default_pages` |
| A label inside a filled section takes the ink that reads on the section's fill, not the page's | Ridgeline's Components section in both themes | `labels_inside_a_section_read_on_its_fill_not_the_page` |
| A frame's status round-trips unchanged through MCP, undo/redo and save/open (Changed kept); files saved before Stage E open with their status | Mark, edit inside, undo, redo, save, reopen | `crates/mcp/tests/dev_status.rs` › `the_status_round_trips_through_mcp_undo_redo_and_the_file`, `the_example_saved_before_stage_e_opens_with_its_status` |
| Live in the built app: before/after captures in both themes at 10–100%, 0 px changed outside the label regions | `docs/scrim-ports/canvas-badges/README.md` | E3 captures (manual) |
