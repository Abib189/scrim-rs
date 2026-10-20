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
