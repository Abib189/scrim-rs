# Theme (Scrim's UI system, Port 1)

Tokens: `docs/scrim-transfer/ui/` (`tokens.json`, `README.md`). Screenshots, before and after: `docs/scrim-ports/theme/`.

| Check | How | Covered by |
|---|---|---|
| The dark and light ramps are Scrim's (canvas, panel, field, raised, selected, float) | View ▸ Theme ▸ Dark / Light; compare with `ui/README.md` §2 | `crates/ui-egui/src/theme_tokens_tests.rs` › `the_ramp_is_scrims` |
| One accent (orange `#ff5a1f`), derived per theme: the mark `#ff5a1f` (dark) / `#c53400` (light), the fill `#ff5a1f`, the label `#111214` | Share, Close and the active tool show dark labels on orange; the selected layer has an orange 2 px left edge | `theme_tokens_tests.rs` › `the_accent_is_the_one_derived_for_each_theme`; `accent_tests.rs` › `every_preset_derives_as_scrim_derived_it_in_both_themes` (all six presets, both themes) |
| Chrome text reads at AA (4.5:1) and marks at 3:1, in both themes | Check secondary and tertiary labels, menus and the primary button in both themes | `theme_tokens_tests.rs` › `chrome_text_reads_at_aa_in_both_themes` |
| Type scale 11 / 12 / 13; rows 28 px (24 px controls + 4 px gap); 120 ms motion | The layers list and inspector rows sit on a 28 px rhythm | `widgets::ROW_H`, `layers::ROW`; `theme::apply_with` (no automated check on rendered layout) |
| Selection is neutral; the accent appears only as an edge or a mark | Select a layer: a grey row with an orange edge, not a blue fill | manual only: the row painting is checked by eye (`layers.rs`) |
| With any accent, nothing interactive stays orange: Share, the active tool, the rail, the selection and handles, paths, marquee, drag lines, pins, menus; the brand "S" badges keep the brand orange | Settings ▸ General ▸ Accent: Green; look around | `crates/ui-egui/src/accent_ink_tests.rs` › `with_green_no_interactive_orange_is_painted` |
| Changing the accent repaints the canvas handles on the next frame, without input | Select a frame, change the accent | `changing_the_accent_repaints_the_handles_on_the_next_frame` |
| Canvas overlays keep the chosen colour where it reads at 3:1 on the page, else a lighter or darker shade of it | A light page and a dark page | `the_overlay_keeps_the_accent_where_it_reads_and_keeps_its_hue_where_it_doesnt` |
| Every accent role reads for every preset in both themes: indicators 3:1 on panel and canvas, labels on the accent 4.5:1 (also hovered and pressed), accent text 4.5:1 | Each preset, light and dark | `accent_contrast_tests.rs` › `every_preset_in_both_themes_meets_every_role`, `which_presets_were_adjusted` |
| No hard-coded brand orange outside the allow-list; `theme::ACCENT` only where tokens are defined | — | `accent_guard_tests.rs` › `no_brand_orange_literal_outside_the_allow_list`, `the_default_accent_constants_are_used_only_where_tokens_are_defined` |
