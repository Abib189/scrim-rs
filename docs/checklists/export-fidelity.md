# The export guarantee (Port 2)

The report, with the numbers and what remains: `docs/scrim-ports/export-fidelity.md`.

| Check | How | Covered by |
|---|---|---|
| No artboard of the 20 renders worse in the HTML export than the baseline (10 / 11 of 20 identical in Chromium / WebKit) | `cargo xtask fidelity` (needs Node, playwright-core, Chromium and WebKit) | `xtask/src/fidelity.rs` (gate: `regressions`); `xtask/fidelity-baseline.json` |
| The exported page sizes boxes as the canvas does: padding inside, no paragraph margins | Inspect ▸ Code ▸ HTML on a padded auto layout frame; open it in a browser | `crates/engine/src/codegen_export_tests.rs` › `sizes_include_padding_and_paragraphs_add_no_margin` |
| Image paints are in the page (data URIs), placed by their scale mode | Export a frame with an image fill; open the HTML offline | `codegen_export_tests.rs` › `image_paints_are_inlined_as_data_uris_with_their_scale_mode` |
| Several fills stack as the canvas stacks them | A frame with two fills | `codegen_export_tests.rs` › `paints_stack_as_background_layers_top_first` |
| Paths, polygons, stars, lines, booleans and arcs export as inline SVG, turned ones too | Export a frame holding each | `codegen_export_tests.rs` › `shapes_css_cant_draw_are_inline_svg_turned_or_not` |
| A rectangle or ellipse mask clips the layers above it | Export a masked group | `codegen_export_tests.rs` › `a_mask_clips_the_layers_above_it_and_is_not_drawn` |
| Text shadows are `text-shadow`; "space between" has no gap; wrapped lines are `cross_gap` apart | Export text with a drop shadow; a space-between row; a wrapped row | `codegen_export_tests.rs` › `text_shadows_come_from_the_glyphs`, `auto_spacing_replaces_the_gap_and_wrapped_lines_keep_theirs` |
