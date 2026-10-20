# The accent everywhere (Stage G)

The owner's capture, with the accent set to green in Settings ▸ General:
- **Follows green:** the AI tab's underline, the Write and Create collection buttons, the avatar.
- **Stays orange:** the Share button, the toolbar's active tool, the left rail's File icon.
- **Known since Stage B:** canvas selection outlines and handles ignore the accent.

This page is the inventory at `26770bd` (G0), and the plan for G1 and G2.

## How the accent reaches the interface today

- **The choice:** `pref.accent` (Settings ▸ General ▸ Accent: six presets or any colour, `crates/engine/src/cmd/appearance.rs`, `PRESETS`). It is stored in `Prefs.accent`; empty means the default orange.
- **The tokens:** every frame, `FigApp::ui` calls `theme::apply_with(ctx, Tokens::for_prefs(dark, prefs))` (`lib.rs:1256`). `Tokens::with_accent` derives the theme's accent tokens from the base (`theme/accent.rs`, Port 1's rule):
  - `accent`: the mark, for icons and edges; 4.5:1 on the panels, 3:1 on the canvas and selected surfaces;
  - `accent_fill`: buttons and chips; 3:1 against the panel, with some label at 4.5:1 on it;
  - `accent_hover`;
  - `accent_on`: the label on the fill, black or white by contrast;
  - `accent_tint`;
  - `accent_text`;
  - `focus`.
- **They are installed in egui's memory and read back with `Tokens::get(ctx)`.** Anything that reads them follows a change on the next frame. **Nothing caches the accent at startup**: no static, no copy kept between frames.
- **What misses it is code that doesn't read the tokens.** About 150 places paint with the constant `theme::ACCENT` (#FF5A1F) and `theme::ACCENT_ON` (its label) instead. That is everything below marked "interactive accent".
- **The four misses in the capture:**
  - **Share** fills with `ACCENT` (`right.rs:91`); only its hover reads `t.accent_hover`.
  - **The active tool** is `ACCENT` (`toolbar.rs:425`).
  - **Canvas overlays**, selection, handles, marquee, paths and builder regions, are `ACCENT` throughout `canvas.rs`, `handles.rs` and `proto_canvas.rs`.
  - **The left rail's active icon** already reads `t.accent_text` (`layers.rs:55`), and the tokens for green derive (light #207848, dark #2FB36B; checked for every preset). So in this code it follows the accent. The BEFORE captures (G3) check it in the window. The owner's capture may come from an older build.

## Every orange in the code

Uses of `theme::ACCENT` / `ACCENT_ON` and their definitions (test files left out). Imports are left out too. "Accent fill" means the token whose default *is* #FF5A1F in both themes, so the default orange looks exactly as before.

| File | Line | What it paints | Role | Fix |
|---|---|---|---|---|
| `ui-egui/src/right.rs` | 87 | the Share button's label | interactive accent | `t.accent_on` |
| `ui-egui/src/right.rs` | 91 | the Share button | interactive accent | `t.accent_fill` (hover already `t.accent_hover`) |
| `ui-egui/src/right.rs` | 92 | the Share button's label | interactive accent | `t.accent_on` |
| `ui-egui/src/right.rs` | 218 | a menu row under the pointer or keyboard | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/right.rs` | 220 | a menu row under the pointer or keyboard | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/right.rs` | 224 | a menu row under the pointer or keyboard | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/accent.rs` | 18 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/accent.rs` | 20 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/accent.rs` | 21 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/section_panel.rs` | 69 | the chosen section colour's ring | interactive accent | `t.accent_fill` |
| `ui-egui/src/present.rs` | 2170 | the inspected layer's outline in the prototype player | interactive accent | `t.accent_fill` |
| `ui-egui/src/present.rs` | 2226 | a player bar button that is on | interactive accent | `t.accent_fill` |
| `ui-egui/src/present.rs` | 2812 | the hotspot flash after a click on nothing | interactive accent | `t.accent_fill` |
| `ui-egui/src/present.rs` | 2813 | the hotspot flash after a click on nothing | interactive accent | `t.accent_fill` |
| `ui-egui/src/layout_panel.rs` | 319 | the auto-layout alignment grid: the chosen cell, the hovered dot | interactive accent | `t.accent_fill` |
| `ui-egui/src/layout_panel.rs` | 322 | the auto-layout alignment grid: the chosen cell, the hovered dot | interactive accent | `t.accent_fill` |
| `ui-egui/src/colorpicker.rs` | 597 | the selected gradient stop's ring | interactive accent | `t.accent_fill` |
| `ui-egui/src/colorpicker.rs` | 1016 | a colour preset row under the pointer | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/colorpicker.rs` | 1018 | a colour preset row under the pointer (label) | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/home.rs` | 205 | the logo "S" (sidebar, file rows) | brand mark | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/home.rs` | 328 | the action cards' icon tile (New design file, Open file…) | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/home.rs` | 329 | the action cards' icon tile (New design file, Open file…) | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/home.rs` | 393 | the logo "S" (sidebar, file rows) | brand mark | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/theme.rs` | 65 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 67 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 68 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 69 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 101 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 103 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 144 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 145 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 147 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 149 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 165 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/theme.rs` | 179 | the token definition | tokens definition | none (the one definition) |
| `ui-egui/src/tabs.rs` | 100 | the "S" badge on a file tab | brand mark (file badge) | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/proto_canvas.rs` | 143 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/proto_canvas.rs` | 145 | the mark on a prototype connection's + handle | interactive accent | on-colour for the overlay accent |
| `ui-egui/src/proto_canvas.rs` | 146 | the mark on a prototype connection's + handle | interactive accent | on-colour for the overlay accent |
| `ui-egui/src/proto_canvas.rs` | 164 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/proto_canvas.rs` | 180 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/proto_canvas.rs` | 193 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/proto_canvas.rs` | 197 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/proto_canvas.rs` | 198 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/handles.rs` | 296 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/handles.rs` | 304 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/handles.rs` | 314 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/handles.rs` | 315 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/handles.rs` | 323 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/handles.rs` | 327 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/dialogs.rs` | 128 | the logo tile in About | brand mark | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/dialogs.rs` | 129 | the logo tile in About | brand mark | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/canvas.rs` | 250 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 251 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 816 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 962 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 978 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 985 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 989 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 991 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1014 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1020 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1089 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1094 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1125 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1126 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1141 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1153 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1157 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1168 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1176 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1267 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1279 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1295 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1297 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1316 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1334 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1342 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1350 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1351 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1353 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1357 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1374 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1389 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1392 | canvas overlay | interactive accent | overlay accent (the chosen accent, adjusted for the page behind it) |
| `ui-egui/src/canvas.rs` | 1481 | the rulers' selection span (on the panel-coloured ruler) | interactive accent | `t.accent_fill` (tinted) |
| `ui-egui/src/canvas.rs` | 1482 | the rulers' selection span (on the panel-coloured ruler) | interactive accent | `t.accent_fill` (tinted) |
| `ui-egui/src/home_recents.rs` | 741 | the "S" badge on a recent file | brand mark (home file badge) | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/home_recents.rs` | 742 | the "S" badge on a recent file | brand mark (home file badge) | keep brand orange (`theme::BRAND`) |
| `ui-egui/src/home_recents.rs` | 859 | the selected recent file's ring | interactive accent | `t.accent_fill` |
| `ui-egui/src/appearance_prefs.rs` | 47 | "is this the default?" in Settings | logic | none |
| `ui-egui/src/layers.rs` | 186 | drop into a row (Layers) | interactive accent | `t.accent_fill` |
| `ui-egui/src/layers.rs` | 703 | a row's hover outline (Layers) | interactive accent | `t.accent_fill` (components keep purple) |
| `ui-egui/src/layers.rs` | 841 | drop into a row while dragging layers | interactive accent | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 195 | an @mention in a comment | interactive accent | `t.accent_text` |
| `ui-egui/src/comments.rs` | 354 | a comment pin that is hovered or open | interactive accent (see Decision 4) | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 358 | the label or dot on an accent comment pin | interactive accent (see Decision 4) | `t.accent_on` |
| `ui-egui/src/comments.rs` | 361 | the label or dot on an accent comment pin | interactive accent (see Decision 4) | `t.accent_on` |
| `ui-egui/src/comments.rs` | 362 | the unread dot | interactive accent (see Decision 4) | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 365 | the resolved check on a pin | interactive accent (see Decision 4) | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 377 | a comment pin that is hovered or open | interactive accent (see Decision 4) | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 381 | the label or dot on an accent comment pin | interactive accent (see Decision 4) | `t.accent_on` |
| `ui-egui/src/comments.rs` | 402 | the pin being placed | interactive accent | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 403 | the label or dot on an accent comment pin | interactive accent (see Decision 4) | `t.accent_on` |
| `ui-egui/src/comments.rs` | 544 | the Send button when there is text | interactive accent | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 546 | the label or dot on an accent comment pin | interactive accent (see Decision 4) | `t.accent_on` |
| `ui-egui/src/comments.rs` | 770 | the selected thread's outline | interactive accent | `t.accent_fill` |
| `ui-egui/src/comments.rs` | 1065 | the "Show open" link | interactive accent | `t.accent_text` |
| `ui-egui/src/comments.rs` | 1117 | the unread dot | interactive accent (see Decision 4) | `t.accent_fill` |
| `ui-egui/src/proto_panel.rs` | 608 | the controller-button glyph | interactive accent | `t.accent_fill` |
| `ui-egui/src/proto_panel.rs` | 609 | the controller-button glyph | interactive accent | `t.accent_fill` |
| `ui-egui/src/proto_panel.rs` | 1051 | a menu row under the pointer | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/proto_panel.rs` | 1053 | a menu row under the pointer (label) | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/proto_panel.rs` | 1107 | the easing curve preview | interactive accent | `t.accent_fill` |
| `ui-egui/src/dev_mode.rs` | 174 | an annotation's colour when it has no category | interactive accent | `t.accent_fill` |
| `ui-egui/src/dev_mode.rs` | 220 | the selected annotation callout's border | interactive accent | `t.accent_fill` |
| `ui-egui/src/dev_mode.rs` | 634 | a code link's name | interactive accent | `t.accent_text` |
| `ui-egui/src/dev_mode.rs` | 765 | the chosen category colour's ring | interactive accent | `t.accent_fill` |
| `ui-egui/src/dev_compare.rs` | 302 | the before/after slider | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/dev_compare.rs` | 304 | the before/after slider | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/dev_compare.rs` | 305 | the before/after slider (its icon) | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/live.rs` | 152 | a collaborator's colour when theirs is missing or unreadable | semantic (collaborator) | keep (a person's colour, not the accent; `theme::BRAND` as the fallback) |
| `ui-egui/src/live.rs` | 157 | a collaborator's colour when theirs is missing or unreadable | semantic (collaborator) | keep (a person's colour, not the accent; `theme::BRAND` as the fallback) |
| `ui-egui/src/live.rs` | 350 | a collaborator's colour when theirs is missing or unreadable | semantic (collaborator) | keep (a person's colour, not the accent; `theme::BRAND` as the fallback) |
| `ui-egui/src/toolbar.rs` | 189 | the Actions button while its menu is open | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/toolbar.rs` | 193 | the Actions button while its menu is open | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/toolbar.rs` | 425 | the active tool | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/toolbar.rs` | 432 | the active tool | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/toolbar.rs` | 448 | the Done button (vector edit) | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/toolbar.rs` | 449 | the Done button (vector edit) label | interactive accent | `t.accent_fill` / `t.accent_on` |
| `ui-egui/src/widgets.rs` | 872 | the insertion line while dragging rows | interactive accent | `t.accent_fill` |
| `ui-egui/src/widgets.rs` | 883 | the dragged row's ghost outline | interactive accent | `t.accent_fill` |
| `ui-egui/src/widgets.rs` | 977 | a check, radio dot or toggle label on an accent fill | interactive accent | `t.accent_on` |
| `ui-egui/src/widgets.rs` | 993 | a check, radio dot or toggle label on an accent fill | interactive accent | `t.accent_on` |
| `ui-egui/src/widgets.rs` | 1011 | a check, radio dot or toggle label on an accent fill | interactive accent | `t.accent_on` |
| `ui-egui/src/widgets.rs` | 1012 | a check, radio dot or toggle label on an accent fill | interactive accent | `t.accent_on` |
| `engine/src/cmd/appearance.rs` | 3 | the token definition | tokens definition | none (the one definition) |
| `engine/src/cmd/appearance.rs` | 15 | the token definition | tokens definition | none (the one definition) |
| `engine/src/cmd/appearance.rs` | 19 | the token definition | tokens definition | none (the one definition) |
| `engine/src/cmd/appearance.rs` | 61 | the token definition | tokens definition | none (the one definition) |
| `engine/src/cmd/appearance.rs` | 74 | the token definition | tokens definition | none (the one definition) |
| `engine/src/cmd/appearance.rs` | 94 | the token definition | tokens definition | none (the one definition) |
| `engine/src/cmd/appearance.rs` | 96 | the token definition | tokens definition | none (the one definition) |
| `engine/src/templates/kit.rs` | 310 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/kit.rs` | 350 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/kit.rs` | 392 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/dashboard.rs` | 34 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/social.rs` | 39 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/social.rs` | 42 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/social.rs` | 48 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/slides.rs` | 74 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |
| `engine/src/templates/slides.rs` | 128 | a colour name in a generated example document | document content | none: it is the example files' palette, not the interface |

**Literal oranges and near values:**

| File | What | Role | Fix |
|---|---|---|---|
| `engine/src/cmd/appearance.rs` (`DEFAULT_ACCENT`, `PRESETS`) | #FF5A1F, the default and the "orange" preset | tokens definition | none |
| `ui-egui/src/theme.rs` (#C53400, #EB531D, the tint) | the light theme's orange mark and hover, the derived tints | tokens definition | none |
| `ui-egui/src/guides.rs:19` (`GUIDE` #F24822) | ruler guides the person places | other: guides | keep (see Decision 5) |
| `ui-egui/src/theme.rs` (`guide` #D61F69 / #FF4D94), `measure_overlay.rs` | measurement and snapping lines and their labels | other: guides | keep (see Decision 5) |
| `ui-egui/src/handles.rs:17` (`PINK` #F2488C) | auto layout padding and gap handles | semantic (spacing) | keep |
| `ui-egui/src/theme.rs` (`DEV_CHANGED` #D97A06, `warning`) | "Changed", warnings | semantic | keep |
| `assets/app-icon/scrim/*.svg` | the app icon | brand mark | keep |
| `docs/scrim-transfer/ui/tokens.json`, `contrast.json`, `README.md`, `generators/*` | Scrim's design-system source | tokens definition | keep |
| `locales/en.toml`, `es.toml` | the preset's name ("Orange") | text | keep |
| tests (`appearance_tests.rs`, `accent_tests.rs`, `appearance_prefs_tests.rs`, `query/figcraft_tests.rs`, `mcp/tests/panel.rs`) | assert the default's value | tests | keep (allow-listed) |

## The colours that must not follow the accent (semantic)

| Colour | Where | Why it stays |
|---|---|---|
| `danger`, `warning`, `success` | errors, warnings, Connected, the MCP status dots | they mean something; green success must not turn into the accent |
| `DEV_READY` / `DEV_CHANGED` / `DEV_COMPLETED` and the status icons' `Ink` | handoff status (Stage E) | a status, one shape and colour each |
| lint severities | the AI tab's design lint | severity |
| `COMPONENT` purple | component outlines, labels, slots | the component colour, Figma's convention |
| collaborators' colours | cursors, name tags, follow frame, their avatars (`live.rs`) | each person's own colour |
| `PINK` | auto layout padding and gap | the spacing colour |
| `guide` and `GUIDE` | measurement, snapping and ruler guides | see Decision 5 |
| section and annotation category colours | the person's chosen colours | content |

**Comment pins** are listed as semantic in the brief, but their orange isn't a status. It is the accent marking the pin that is open or hovered, unread, or being placed, so it should follow (Decision 4). The authors' avatars on pins keep their own colours.

## Decisions (overrule any)

1. **The brand stays orange:** the logo "S" (home sidebar, file rows, file tabs, recent files' badges), the About tile and the app icon. They move to a named constant, `theme::BRAND`, so a guard test can tell a brand mark from an interactive orange. Agreed with the brief.
2. **Everything interactive follows the accent,** in four ways:
   - fills and edges with `t.accent_fill`;
   - labels on them with `t.accent_on`;
   - text with `t.accent_text`;
   - canvas overlays with an overlay accent: the chosen colour, kept where it reads at 3:1 on the page behind it, its lightness moved where it doesn't.
3. **The default orange is unchanged on screen** wherever it already read, so the pixel check holds. Two exceptions:
   - **Canvas overlays on a light page** (#F5F5F5 or the light canvas) use a darker orange: #FF5A1F is 2.64–2.86:1 there.
   - **Three accent texts** (an @mention, "Show open", a code link's name) use `accent_text` (#C53400 in light): #FF5A1F is 3.1:1 as text on white.

   Dark pages, including Ridgeline's in both themes, keep #FF5A1F exactly.
4. **Comment pins follow the accent:** their orange is a selection or unread mark.
5. **Snapping, measurement and ruler guides keep their own colour,** not the accent. Pink and red guides stay distinguishable from the selection, which is the convention people know from Figma. The brief lists snapping guides among the overlays to follow. Overruling it is one line (`t.guide` → overlay accent).
6. **Text selection stays neutral** (`selected_row`): "Selection is neutral; the accent is only an edge or a mark" (`ui/README.md` §4). The brief lists the text-selection colour; it has no orange to fix.
7. **New tokens:** `accent_pressed` (the brief's accent-pressed). `accent_base` keeps the chosen colour for the canvas overlay. The brief's accent-subtle is the existing `accent_tint`, and on-accent is `accent_on` (black or white by contrast).
8. **Frozen files stay at their size.** `canvas.rs`, `present.rs`, `right.rs`, `comments.rs`, `colorpicker.rs`, `widgets.rs`, `proto_panel.rs` and `live.rs` are at their caps. The lookup goes into a new module (`accent_ink.rs`) with one-word calls, so the swaps don't add lines.
