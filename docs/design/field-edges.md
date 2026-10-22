# Input edges at 3:1 (Fix S2, 22 Oct 2026)

**The problem:** the border token inputs drew (`border_strong` / `field_hover_border`, `#D4D4D1` light, `#313338` dark) measured about 1.5:1 on the panels, under the 3:1 a control's boundary needs (WCAG 1.4.11). Most inputs drew no edge at all at rest, only their fill, which is about 1.1:1 against the panel.

## The audit (before)

`field_edges_tests.rs` draws real screens and measures every input AccessKit reports, in both themes with each of the six accent presets:
- **The inputs it reads:** text inputs, spin buttons and combo boxes.
- **The screens:**
  - the properties panel (number fields, paint rows, dropdowns);
  - the Layers and Assets searches;
  - the AI tab's composer;
  - Settings ▸ General and ▸ Tool access;
  - the batch rename dialog;
  - the shortcuts panel.

**Before the fix:** 1,080 failing measurements. Every input except the composer's field (which already used the secondary text colour) had no edge at rest:
- X, Y, W, H, rotation, opacity, corner radius, spacing;
- the zoom pill;
- the paint rows' hex fields;
- the searches;
- Settings' text fields, dropdowns and nudge amounts;
- the batch rename fields.

## What changed

- **A token, `field_edge`,** for an input's edge at rest: `#80838A` light, `#74777E` dark. At least 3:1 against the field, the panels, the floating surface (dialogs, menus), the pressed fill and the canvas.
- **The hover edge, `field_hover_border`,** is now the tertiary text colours: `#5F6268` light, `#91959C` dark.
- **One helper decides every edge** (`theme::field_stroke(tokens, hovered, focused)`): the accent's mark with the keyboard, the hover edge under the pointer, else `field_edge`. `theme::paint_field` draws a field's fill and edge together.
- **egui's own inputs** (a framed `TextEdit`, `ComboBox`) take it from the visuals: `widgets.inactive.bg_stroke` is now `field_edge` (it was none). egui's menus clear it for their items.
  - **`DragValue`** is framed through egui's button style instead, so `theme::add_input` draws the edge over it.
  - **Stock egui buttons** outside menus also get this edge now (Find, Replace all, Remove all, Copy …).
- **The hand-drawn inputs:**
  - `widgets.rs`: the number field, the bound field, text fields, `text_input`, dropdowns and the multiline field;
  - `typography.rs`;
  - the Layers, Assets and Variables searches;
  - the paint rows (`right.rs`, `colorpicker.rs`);
  - the zoom pill.
  - All of them now draw that edge at rest, not only on hover. The frozen files got shorter (`widgets.rs` 1138 → 1120, `typography.rs` 1426 → 1421), and their caps in `xtask/size-allowlist.txt` are lowered to match.

## The tests

- `every_input_edge_reads_at_3_to_1_in_both_themes_and_every_accent`: 1,188 input edges measured. The lowest are 3.45:1 (light) and 3.46:1 (dark).
  - **The edge counted** is the smallest stroked rectangle around the input, on its field box; translucent edges are composited over what is under them.
  - **Exempt:** an input drawn as plain text with no box at all, which is a label clicked to rename (the file's name, page names, frame labels).
  - **Mutation checks:** with the old colours it fails 1,068 times; with egui's inactive stroke removed, 36 times.
- `rest_hover_and_focus_edges_read_at_3_to_1_on_every_surface_in_every_theme_and_accent`: the three states on five surfaces, two themes, six accents.

## Captures

`docs/scrim-ports/field-edges/` holds the real window (debug build, scratch `HOME` and settings, `s2.py`) in light and dark, at 2×:
- the properties panel with the Layers search open;
- Settings ▸ General;
- the AI tab's composer.

`docs/scrim-ports/chat/`'s offscreen captures are redrawn with the new edges. All of them were checked for personal data with `scripts/ocr-captures.swift`.

## Not verified

- **Inputs on screens the audit doesn't open:** the style picker, the variables table's cells, comments, versions, the live-session forms, the plugin forms. They use the same widgets and visuals, but they aren't measured.
- **The focus edge** is checked through the tokens, not by drawing each field focused.
- **Older captures** under `docs/scrim-ports/` (Stages B–Q) still show the old edges. They record their stages and weren't retaken.
