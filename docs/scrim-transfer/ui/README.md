# Scrim UI system: transfer kit for an egui theme

This folder carries the Scrim editor's chrome design system into a Rust fork whose UI is egui. It has three files:

| File | What it holds |
|---|---|
| `tokens.json` | Every token, DTCG-style (`$value`, `$type`). Colours are resolved to concrete sRGB for **dark** and **light** under `color.dark.*` / `color.light.*`. The original CSS expression is kept in `$extensions.scrim.css`, and the `--scrim-*` variable in `$extensions.scrim.var`. |
| `contrast.json` | 105 text, mark and background pairs per theme, each with its WCAG 2.x ratio and an AA verdict. It also holds the guarantees for the six accent presets. |
| `README.md` | This document: the principles, which token goes where, the states, the accent rule, and the egui mapping. |

**Source of truth.** The tokens come from `apps/studio/src/theme.css` (its three `:root` blocks at lines 14–228), `apps/studio/src/appearance/derive.ts` (the accent rule), `apps/studio/src/appearance/presets.ts`, `apps/studio/src/editor/useOverlayDraw.ts` with `packages/engine/src/overlay.ts` (the canvas chrome palette and geometry), and `docs/design/ui-system.md`.

`ui-system.md` is headed "proposal only", but most of it shipped. Where the two differ, **theme.css wins**, and the differences are listed at the end of this document.

**How the values were resolved.**
- `theme.css` was loaded into headless Chrome 155 (playwright-core, installed in a scratch folder outside the repo) with `html[data-theme=dark|light]`. Every `--scrim-*` property was read with `getComputedStyle`.
- Each `color-mix(in srgb, X p%, transparent)` and `rgb(var(--scrim-overlay-rgb) / a)` was resolved by assigning it to a probe element. `color-mix` with `transparent` comes out as X at alpha p.
- Values the chrome makes from opacity, Canvas2D `globalAlpha` or `filter: brightness()` were computed by hand in sRGB. For the brightness filter, a screenshot sample is kept beside the value; it reads a few levels darker in dark, which is colour management.

**Formats.** A colour `$value` is `#rrggbb`, or `#rrggbbaa` when translucent, with straight (unpremultiplied) alpha. In egui that is `Color32::from_rgba_unmultiplied`. Dimensions are DTCG 2025 objects `{value, unit}`, where 1 px = 1 egui point.

---

## 1. Principles

**Grey does the work; colour means something.** The chrome is a neutral ramp. A hue appears only when it carries a meaning:

| Hue | Meaning, and only this |
|---|---|
| **Accent** (default orange `#ff5a1f`, user-selectable) | You: the selection on the canvas and in layers, the focus ring, the primary action, checked boxes, snap guides and distance chips. |
| **Agent** (default violet `#8e7bff`, user-selectable) | The AI agent: its outline and chip on the canvas, the dock's status dot, agent-edited layer markers. |
| **Danger** | Destructive actions, inline errors, a closed connection. |
| **Warning** (amber) | Lint findings only: the count, the badge, the lint tool. |
| **Measure** (pink) | Distances while moving. This is the one exception to "one accent", so that a distance is never confused with the selection. |

**The rest of the system:**
- **Type:** one family (IBM Plex Sans) at three sizes (11 / 12 / 13 px) and two weights (400 / 500). There is no uppercase, no letter-spacing and no bold, and numerals are tabular. Plex Mono is used only for code (the Code tab, the terminal) and for the canvas measure digits.
- **Space:** a 4 px grid, with one optical 2 px step.
- **Rows:** a **28 px row** with **24 px controls**, separated by a 4 px gap.
- **Radii:** two of them, 4 px for controls and 8 px for anything floating.
- **Panels:** flat, separated by 1 px hairlines. There is **one shadow**, used only on things that float, and no gradients, glows or blur.
- **Selection:** a selected item is **neutral** (`bg-4` with `text-1`). The accent appears only as an *edge*: the 2 px left edge of a selected layer row, and the canvas outline.
- **Motion:** one duration (120 ms) and one easing (`cubic-bezier(0.2, 0, 0, 1)`), both set to 0 under reduced motion. Nothing in the chrome loops forever, and canvas content never animates.
- **Chrome only:** none of these tokens may reach a design node, an export or a screenshot.
- **Copy:** sentence case. No exclamation marks and no "please".

---

## 2. The roles: the neutral ramp

| Role (tokens.json) | Var | Dark | Light | Used for |
|---|---|---|---|---|
| `surface.canvas` | `--scrim-bg-0` | `#17181a` | `#ececea` | The canvas field behind artboards. |
| `surface.panel` | `--scrim-bg-1` | `#111214` | `#ffffff` | Panels, the top bar, the shell, Home, the terminal background. |
| `surface.field` | `--scrim-bg-2` | `#1b1c1f` | `#f4f4f3` | Fields, buttons at rest, hovered rows, chips, the layers band. |
| `surface.raised` | `--scrim-bg-3` | `#232428` | `#ebebea` | Pressed fields, hovered secondary buttons, the floating tool bar. |
| `surface.selected` | `--scrim-bg-4` | `#2c2d32` | `#e1e1df` | Selected rows, segments and toggles, keycaps, a pressed secondary button, the disabled primary. |
| `surface.float` | `--scrim-bg-float` | `#232428` | `#ffffff` | Menus, popovers, dialogs, the palette, toasts and canvas hints. |
| `surface.floatHover` | `--scrim-float-hover` | `#2c2d32` | `#ebebea` | The highlighted row on a floating surface. |
| `surface.tooltip` | `--scrim-tooltip-bg` | `#232428` | `#18191b` | Tooltips and canvas readout chips. **In light this surface is dark.** |
| `surface.backdrop` | `--scrim-backdrop` | `#00000073` | `#0000002e` | Behind a modal dialog or the palette. |
| `text.primary` | `--scrim-text-1` | `#ececee` | `#18191b` | Names, values, the active tab, menu rows, button labels. |
| `text.secondary` | `--scrim-text-2` | `#a7aab0` | `#44474c` | Labels, section heads, icon buttons at rest, inactive segments. |
| `text.tertiary` | `--scrim-text-3` | `#91959c` | `#5f6268` | Meta, prefixes, units, shortcuts, placeholders, layer glyphs, carets, "Mixed". |
| `text.disabled` | `--scrim-text-off` | `#70747b` | `#8e9095` | Disabled text. |
| `text.onTooltip` | `--scrim-tooltip-text` | `#ececee` | `#ececee` | Text on a tooltip. |
| `border.hairline` | `--scrim-line-1` | `#232428` | `#e8e8e6` | Panel and section separators, header rules, indent guides. |
| `border.strong` | `--scrim-line-2` | `#313338` | `#d4d4d1` | The floating-surface edge, a field on hover or while editing, menu separators, tool bar dividers, the checkbox edge. |
| `feedback.danger` / `dangerFill` / `onDanger` | | `#ff5c5c` / `#d42f2f` / `#fff` | `#cf2a2a` / `#d42f2f` / `#fff` | Error text and the destructive button. |
| `feedback.warning` / `warningTint` / `onWarning` | | `#e6a23a` / `#e6a23a24` / `#111214` | `#985f08` / `#985f081f` / `#fff` | Lint text, the lint wash, the lint badge. |
| `canvasChrome.measure` / `onMeasure` | | `#ff4d94` / `#1f0a14` | `#d61f69` / `#fff` | Distance lines and chips. |

The accent and agent roles are covered in section 5.

---

## 3. Which token goes where

**Shell and panels**
- **Window and shell:** `surface.panel` background, `text.primary` text, 12 px / 16 px. Panels are separated by a 1 px `border.hairline`. Panels are never shadowed.
- **Top bar:** 40 px tall on `surface.panel`, with a `border.hairline` under it.
  - Breadcrumb: the dim parts in `text.tertiary`, the title in `text.primary` at 12 px / 500.
  - "Saved" in `text.tertiary` at 11 px; it turns `feedback.danger` when the connection is closed.
  - Agent status: an 8 px dot in `text.tertiary`, which becomes `agent.mark` when connected and `danger` when closed, followed by a word in `text.secondary`. It hovers to `surface.field` with `text.primary`.
  - Zoom and the undo/redo icon buttons are 24 px.
- **Panel header** (Layers, Lint) and the **inspector tabs:** 40 px tall, `surface.panel`, with a `border.hairline` under them. The title is 13 px / 18 px / 500 in `text.primary`.
  - Tabs are 13 px in `text.tertiary`, `text.secondary` on hover, and `text.primary` at 500 when active. There is no underline and no fill.
  - The lint count is `feedback.warning` at 11 px.
- **Inspector:**
  - **Sections** are flat. The first has no border; each later one has a `border.hairline` on top.
  - **The section head** is 32 px tall, with its label at 11 px / 500 in `text.secondary` (`text.primary` on hover). A collapse caret in `text.tertiary` appears only on hover or focus.
  - **Rows** are on a **28 px** rhythm: 24 px controls with a 4 px gap. The label column is 72 px, and labels are 12 px in `text.secondary`.
  - **Panel padding** is 12 px on each side.
- **Layers panel:**
  - The filter row is 32 px; its placeholder is in `text.tertiary`.
  - **Rows are 28 px**, radius 4, with `text.primary` names. Each tree level indents 16 px, with a `border.hairline` guide line at 10 px.
  - Row states: hover `surface.field`; **selected** `surface.selected` with a 2 px inset **left edge** in `accent.mark` and the name at 500. The selected row's descendants sit on a `surface.field` band with square corners.
  - Glyphs and carets are 14 px in `text.tertiary`. Layer meta is 11 px in `text.tertiary`, shown only on hover or selection. The agent-edited marker is 11 px in `agent.mark`.
  - Dragging draws a 2 px `accent.mark` insert line above or below a row. Dropping inside a row outlines it 1 px in `accent.mark` on `accent.tint`.

**Inputs**
- **Fields, inputs and selects:**
  - 24 px tall, `surface.field`, radius 4, 8 px side padding, 12 px text in `text.primary`.
  - **Borderless at rest:** the 1 px border is transparent. It becomes `border.strong` on hover and while editing.
  - Units and prefixes are 11 px in `text.tertiary`; the dropdown chevron is 14 px in `text.tertiary`.
  - The keyboard focus ring is drawn on the field wrapper.
  - "Mixed" is shown in `text.tertiary`.
- **Buttons:** 24 px tall, 8 px side padding, radius 4, 12 px / 500 labels.
  - **Secondary:** `surface.field` with `text.primary`.
  - **Primary:** `accent.fill` with `accent.onFill`.
  - **Danger:** `feedback.dangerFill` with `onDanger`.
  - The small variant is 20 px tall with 11 px text.
- **Icon buttons and tools:** 24 px (tools are 28 px, inside a 40 px bar), transparent, icons in `text.secondary`. See section 4 for their states.
- **Segmented controls:** a 24 px `surface.field` track with a 2 px inset. Segments are 20 px, transparent, in `text.secondary`; the selected segment is `surface.selected` with `text.primary` at 500.
- **Checkbox:** 14 px, `surface.field`, with a 1 px inset `border.strong` edge. Checked, it fills with `accent.fill`, and a 10 px check mark is drawn in `accent.onFill`.
- **Slider:** the fill is `text.tertiary` (neutral, not the accent) and the thumb is a 12 px circle in `text.primary`.
- **Keycap:** 16 px tall, minimum 16 px wide, 4 px padding, radius 4, `surface.selected` with `text.secondary` at 11 px. An inline shortcut is plain `text.tertiary` at 11 px.

**Floating surfaces**
- **Shared style** (menus, popovers, the palette, dialogs, the colour picker, toasts and canvas chips): `surface.float`, radius 8, `shadow.float`, which includes a 1 px `border.strong` ring. Text is `text.primary` at 12 px.
  - **Menus:** at least 200 px wide with 4 px padding. Rows are 28 px with 8 px padding and radius 4, and highlight to `surface.floatHover`. Shortcuts are right-aligned in `text.tertiary` at 11 px. Separators are a 1 px `border.strong`. Disabled rows are `text.disabled` and still show their reason.
  - **Popover group heads** are 11 px / 500 in `text.secondary`.
  - **Dialogs:** 400 px wide, 24 px padding, a 12 px gap. The title is 13 / 18 / 500, the body `text.secondary` on 18 px lines. The modal backdrop is `surface.backdrop`. Cancel comes before the action.
  - **Command palette:** 560 px wide, at most 60 vh tall. Its 40 px search row uses 13 px text, and the active row is `surface.floatHover`.
- **Tool bar** (floating, bottom centre): `surface.raised`, **not** `surface.float`. It has radius 8, `shadow.float` and 4 px padding.
  - Tools are 28 px. They hover to `surface.field` with `text.primary`; the active tool is `surface.selected` with `text.primary`.
  - The lint tool is in `feedback.warning`. Its badge is a round 16 px `warning` fill with an `onWarning` 11 px count.
  - Dividers are 1 × 20 px in `border.strong`.
- **Tooltips:** at least 24 px tall, 8 px side padding, radius 8, `surface.tooltip` with `text.onTooltip` at 12 px, and `shadow.float`.

**Canvas overlay (Canvas2D)**

The overlay is drawn on a canvas, not in the DOM, and all of it is screen-space and zoom-independent.

| Element | Colour token (`canvasChrome.*`) | Geometry |
|---|---|---|
| Selection outline, multi-select union, draft | `selection` (= accent.mark) | 1.5 px stroke |
| Hover outline | `hover` (accent.mark at alpha 0.45) | 1 px |
| Resize handles | fill `handleFill` `#ffffff`, stroke `handleStroke` (accent.mark) | 7 px square, 1.5 px stroke, 4 px hit slop. Drawn at alpha 0.35 when the selection can't be resized. |
| Marquee | fill `marqueeFill` (alpha 0.08), stroke accent.mark | 1.5 px |
| Snap guides | `snapGuide` (alpha 0.75) | 1 px, 4 px dash / 4 px gap |
| Spacing ticks | `spacingTick` | 1 px bar, 6 px end caps |
| Distance chip | `chipFill` (accent.fill) + `chipText` (accent.onFill) | 14 px tall, 5 px side padding, 11 px Plex Sans |
| Drop container / ghost / insert line | `dropContainer` (alpha 0.6) / `dropGhostFill` (alpha 0.1, edge 0.9) / `dropLine` | 1 px / 1 px / 2 px |
| Agent presence | `agentOutline`, `agentChipFill`, `agentChipText` | 1 px outline. The chip is 15 px tall, 4 px under the node. Both fade out. |
| Measurement | `measure`, `onMeasure` | 1 px lines (3 px dashes, 3 px caps), a band at alpha 0.16, 10 px Plex Mono digits |
| Position / size readout | `readout` / `readoutText` (the tooltip pair) | |
| Artboard label | `artboardLabel` (text.tertiary) | 11 px |
| Dot grid | `canvasDot` | 1 px dots; the spacing adapts to zoom |
| Transparency checker | `checkerA` / `checkerB` | 8 px squares |

**Agent dock and terminal.** The terminal host sits on `surface.canvas`. xterm itself paints on `surface.panel` with `text.primary`, an `accent.mark` cursor and the `terminal.*` ANSI palette. The dock's status dot uses `agent.mark`, `warning` or `danger`.

---

## 4. States

Every control has the same six states, and every one is built from the ramp.

| State | Fields, segments | Secondary button | Primary button | Rows (layers, menus) | Icon buttons, tools |
|---|---|---|---|---|---|
| **Default** | `field` bg, transparent border | `field` bg, `text.primary` | `accent.fill`, `accent.onFill` | transparent, `text.primary` | transparent, `text.secondary` |
| **Hover** | + 1 px `border.strong` | `raised` bg | `state.primaryHover` (fill × brightness 0.92) | `field` bg (menus: `floatHover`) | `field` bg, `text.primary` |
| **Pressed** | `raised` bg | `selected` bg | `state.primaryPressed` (× 0.84) | `raised` bg | `raised` bg (tools: `field`) |
| **Focus-visible** | 2 px `accent.mark` ring, 1 px outside | same | same, but the ring is `text.primary` | same | same |
| **Disabled** | opacity 0.5, `text.disabled`; the reason goes in a tooltip | `text.disabled` | `selected` bg, `text.disabled` | `text.disabled` (the reason shows on hover) | `text.disabled`, transparent |
| **Selected** | segment: `selected` bg, `text.primary`, weight 500 | n/a | n/a | `selected` bg, plus a 2 px `accent.mark` left edge (layers only), weight 500 | `selected` bg, `text.primary` |

Rules:
- **Keyboard focus is always visible** (`:focus-visible`). Mouse focus on a field shows only the hover border. A ring is never removed without a replacement.
- **Each kind of item has one selected style.** It is neutral everywhere, with the accent only as an edge.
- **Transitions:** colour, background and border change over 120 ms with the one easing.

---

## 5. Accent, agent and the AA contrast rule

The user chooses **one base colour per role** (accent and agent) in Settings › Appearance. Six presets are offered: orange `#ff5a1f` (the accent default), blue `#4a86ff`, violet `#8e7bff` (the agent default), green `#2fb36b`, pink `#ec4f98` and teal `#1fb5a6`. A custom hex is also accepted.

One pure function (`derive.ts`) turns a base colour into four tokens for each theme. A port must reimplement it; `tokens.json › accentRule.presets` holds the results for all six presets.

| Token | Use | Contract (WCAG 2.x) |
|---|---|---|
| `mark` | Icons, the selected edge, the focus ring, the canvas selection and guides, coloured text | **4.5:1** on panel, field, raised and float; **3:1** on canvas and selected |
| `fill` | Primary buttons, checked boxes, canvas chips | Some label reaches 4.5:1 on it, **and** it reaches **3:1** against the panel |
| `onFill` | The label on the fill | `#ffffff` or `#111214`, whichever has the higher ratio; it must reach **4.5:1** |
| `tint` | A low-alpha wash | the **mark's** RGB at alpha **0.16** (dark) or **0.12** (light) |

**The algorithm,** run separately for each theme against that theme's ramp:

1. **Parse the hex.** Anything else is refused.
2. **Convert to HSL.** Refuse a colour whose saturation is under 0.18, or whose lightness is above 0.97 or below 0.03; it would read as grey.
3. **Derive the mark.** If the base fails the mark contract, change **lightness only**, keeping hue and saturation. Step by 0.005, at most 200 times: lighter on the dark theme, darker on the light theme. The first value that passes wins; if none passes, the colour is refused.
4. **Derive the fill.** Use the base if it passes the fill contract. Otherwise try lighter HSL steps, then darker ones.
5. **Derive onFill.** Compare white and `#111214` on the fill and take the better one (a tie goes to white).
6. **Derive the tint** from the mark (see the table above).
7. **Keep the roles apart.** The agent colour is refused when it is "too close" to the accent: a hue difference under 20° **and** a lightness difference under 0.2.

The result for the defaults:
- **Dark:** the accent mark and fill are both `#ff5a1f` with an `#111214` label (white fails on that orange).
- **Light:** the mark darkens to `#c53400` and the fill stays `#ff5a1f`.
- **Agent:** `#8e7bff` in dark; `#6248ff` mark with a `#8e7bff` fill in light.

**Fixed contrast floors for the rest of the system.** All chrome text is small (under 18 px), so text needs 4.5:1. Marks (dots, edges, rings, icons, the primary button's shape) need 3:1. Disabled text is exempt, but the repo holds it to 3:1 on the panel. `contrast.json` lists every pair.

---

## 6. Type, space, size, radii, icons, elevation, motion, layers

- **Type styles** (`typography.*`; every one has tabular numerals and no letter-spacing):
  - `body`: 12 / 16 / 400
  - `bodyStrong`: 12 / 16 / 500, for buttons and the selected row
  - `small`: 11 / 16 / 400
  - `label`: 11 / 16 / 500, for section heads
  - `title`: 13 / 18 / 500
  - `tab`: 13 / 18 / 400
  - `code`: Plex Mono 12, line height 1.7
  - `canvasLabel`: 11 px sans
  - `canvasMeasure`: 10 px mono
- **Space:** 2, 4, 8, 12, 16, 20, 24 and 32 px. Chrome padding, margins and gaps use only these values, or 0, 1 or 2 px.
- **Sizes:**
  - Rows: row **28**, control **24**, small control 20, section head 32.
  - Bars: top bar 40, tool bar 40, tool 28.
  - Panel metrics: panel padding 12, label column 72, indent 16.
  - Panels: layers 264 (min 200, max 420), inspector 300, lint 400, dock 340.
  - Floating surfaces: dialog 400, Settings 480, palette 560, picker 272, menu minimum 200.
- **Radii:** 4 (control), 8 (float), 9999 (dots, avatars, the badge, the modifier pill).
- **Icons:** 14 px in rows and fields, 16 px in bars and section actions. Outline only, 1.25 px stroke, round caps.
- **Borders:** 1 px hairline, a 2 px focus ring at 1 px offset, a 2 px selected edge. On the canvas: 1.5 px for the selection, 1 px for everything else.
- **Shadow:** `shadow.<theme>.float` is the only chrome shadow:
  - dark: a 1 px `#313338` ring + `0 8 24 #00000066`;
  - light: a 1 px `#d4d4d1` ring + `0 8 24 #0000001a`.

  The artboard shadow is canvas-only and has a negative spread.
- **Motion:** 120 ms, `cubic-bezier(0.2, 0, 0, 1)`; 0 ms under reduced motion. Two exceptions exist: the dock resize uses `cubic-bezier(0.22, 1, 0.36, 1)`, and the 1.6 s "reveal" flash is a fading 2 px accent ring.
- **Z-layers:** 1 sticky panel header · 2 resize handle · 3 dock resize · 8 place chip · 9 lint panel, connection banner, import report · 10 tool bar · 11 tool flyout · 30 canvas hint · 50 Home · 1000 menus and popovers · 1100 dialog and palette scrims, the modifier pill. In egui these become `Order::{Background, Middle, Foreground, Tooltip}` plus the order in which things are painted.

---

## 7. egui mapping

The field names follow **egui 0.31 / 0.32**: `CornerRadius` (u8), `Margin` (i8), `Shadow { offset: [i8; 2], blur: u8, spread: u8, color }`, and `Visuals::weak_text_color`. On an older egui, `corner_radius` was `rounding` and `window_corner_radius` was `window_rounding`. Build one `Visuals` per theme from `color.dark` and `color.light`.

| Scrim token | egui field | Dark | Light | Note |
|---|---|---|---|---|
| (theme) | `Visuals::dark_mode` | `true` | `false` | Start from `Visuals::dark()` / `Visuals::light()` and override the fields below. |
| `surface.panel` | `panel_fill` | `#111214` | `#ffffff` | Side and top panels, the top bar. |
| `surface.float` | `window_fill` | `#232428` | `#ffffff` | Windows, menus, popups and tooltips all use this one fill. |
| `border.strong` | `window_stroke` = `Stroke(1.0, …)` | `#313338` | `#d4d4d1` | Stands in for the first layer of `shadow.float`. |
| `shadow.float` (2nd layer) | `window_shadow`, `popup_shadow` = `Shadow{offset:[0,8], blur:24, spread:0, color}` | `#00000066` | `#0000001a` | |
| `radius.float` | `window_corner_radius`, `menu_corner_radius` | 8 | 8 | |
| `surface.field` | `extreme_bg_color` | `#1b1c1f` | `#f4f4f3` | TextEdit background and the scrollbar track. Also set `text_edit_bg_color = Some(field)` where it exists. |
| `surface.field` | `faint_bg_color` | `#1b1c1f` | `#f4f4f3` | Striped rows and the layers band. |
| `surface.field` | `code_bg_color` | `#1b1c1f` | `#f4f4f3` | |
| `text.secondary` | `hyperlink_color` | `#a7aab0` | `#44474c` | Scrim links are neutral `text-2` with a `line-2` underline. Using the accent would break "colour means something". |
| `feedback.warning` | `warn_fg_color` | `#e6a23a` | `#985f08` | |
| `feedback.danger` | `error_fg_color` | `#ff5c5c` | `#cf2a2a` | |
| `text.tertiary` | `weak_text_color` (0.32+) | `#91959c` | `#5f6268` | On older versions, use `RichText::color` (egui fades weak text by alpha instead). |
| none | `override_text_color` | `None` | `None` | |
| **noninteractive** (labels, separators, frames) | `widgets.noninteractive.bg_fill` / `weak_bg_fill` | `#111214` | `#ffffff` | |
| | `.bg_stroke` | `Stroke(1, #232428)` | `Stroke(1, #e8e8e6)` | Separators, the collapsing-header vline, indent guides (`border.hairline`). |
| | `.fg_stroke` | `Stroke(1, #ececee)` | `Stroke(1, #18191b)` | The default label colour (`text.primary`). Paint labels in `text.secondary` explicitly. |
| | `.corner_radius` / `.expansion` | 4 / 0 | 4 / 0 | |
| **inactive** (at rest) | `widgets.inactive.weak_bg_fill` | `#1b1c1f` | `#f4f4f3` | The button background (`surface.field`). |
| | `.bg_fill` | `#1b1c1f` | `#f4f4f3` | Checkbox, radio and slider rail. |
| | `.bg_stroke` | `Stroke::NONE` | `Stroke::NONE` | Fields are borderless at rest. The Scrim checkbox has a `line-2` edge that egui can't give it here. |
| | `.fg_stroke` | `Stroke(1, #ececee)` | `Stroke(1, #18191b)` | Button labels and TextEdit text (egui uses `inactive.text_color()` for both). Tint icon buttons `text.secondary` yourself. |
| | `.corner_radius` / `.expansion` | 4 / 0 | 4 / 0 | No hover growth anywhere. |
| **hovered** | `widgets.hovered.weak_bg_fill` / `.bg_fill` | `#232428` | `#ebebea` | Secondary button hover (`raised`). Icon buttons hover to `field` `#1b1c1f` / `#f4f4f3`: paint them yourself. |
| | `.bg_stroke` | `Stroke(1, #313338)` | `Stroke(1, #d4d4d1)` | The field hover border. Buttons also pick it up; Scrim buttons have none. |
| | `.fg_stroke` | `Stroke(1, #ececee)` | `Stroke(1, #18191b)` | |
| | `.corner_radius` / `.expansion` | 4 / **0** | 4 / **0** | egui's default expansion is 1.0; set it to 0. |
| **active** (pressed, dragged, **focused**) | `widgets.active.weak_bg_fill` | `#2c2d32` | `#e1e1df` | Secondary button pressed (`selected`). |
| | `.bg_fill` | `#232428` | `#ebebea` | Field pressed (`raised`). |
| | `.bg_stroke` | `Stroke(1, #313338)` | `Stroke(1, #d4d4d1)` | |
| | `.fg_stroke` | `Stroke(1, #ececee)` | `Stroke(1, #18191b)` | |
| | `.corner_radius` / `.expansion` | 4 / 0 | 4 / 0 | |
| **open** (open combo box or menu button) | `widgets.open.weak_bg_fill` / `.bg_fill` | `#232428` | `#ebebea` | |
| | `.bg_stroke` / `.fg_stroke` | `line-2` / `text-1` | `line-2` / `text-1` | |
| selected (neutral) | `selection.bg_fill` | `#2c2d32` | `#e1e1df` | Selectable labels, `Button::selected` and toggles (`surface.selected`). egui also uses it for **text** selection, which becomes neutral grey. |
| | `selection.stroke` | `Stroke(1, #ececee)` | `Stroke(1, #18191b)` | The selected label's text colour **and** the focused TextEdit's frame. The colour must be `text.primary`, or selected labels become unreadable. |
| `state.focusRing` | none | `#ff5a1f` | `#c53400` | **Paint it:** when `response.has_focus()`, draw a 2 px stroke 1 px outside the rect (`text.primary` on a primary button). See below. |
| `accent.fill` / `accent.onFill` | none (custom `Button::fill`) | `#ff5a1f` / `#111214` | `#ff5a1f` / `#111214` | Primary button: `Button::new(RichText::new(..).color(onFill)).fill(fill)`. Hover and pressed use `state.primaryHover` / `primaryPressed`. |
| `slider fill` | `slider_trailing_fill = true` uses `selection.bg_fill` | | | Scrim's slider fill is neutral `text.tertiary`; set `slider_trailing_fill = false`, or paint it. |
| `radius.control` | every `widgets.*.corner_radius` | 4 | 4 | |
| text caret | `text_cursor.stroke` | `Stroke(1.5, #ececee)` | `Stroke(1.5, #18191b)` | The browser default is `currentColor`. The terminal cursor is `accent.mark`. |
| `space`: 8 h, 4 v | `spacing.item_spacing` | `vec2(8, 4)` | | 24 px controls + 4 = the 28 px row rhythm. |
| button padding | `spacing.button_padding` | `vec2(8, 4)` | | 8 px sides; (24 − 16) / 2 = 4 vertically. |
| `size.control` | `spacing.interact_size` | `vec2(24, 24)` | | Minimum interactive size. Force a 28 px row with `ui.allocate_ui(vec2(w, 28.0), ..)` where a row has no control. |
| `size.indent` | `spacing.indent` | 16 | | Also set `indent_has_left_vline = true` (`line-1`). |
| menu padding | `spacing.menu_margin` | `Margin::same(4)` | | |
| dialog padding | `spacing.window_margin` | `Margin::same(24)` dialogs; `Margin::symmetric(8, 4)` popovers | | One value per `Style`; override it per `Frame`. |
| `size.menuMinWidth` | `spacing.menu_width` | 200 | | |
| checkbox | `spacing.icon_width` / `icon_width_inner` / `icon_spacing` | 14 / 10 / 8 | | |
| scrollbars | `spacing.scroll` = `ScrollStyle::thin()`; `bar_width` ≈ 6 | | | The thumb is `scrollbar.thumb` (`#313338` / `#d4d4d1`), hover `thumbHover`, and the track is transparent. egui derives the handle colour from the widget fills, so paint it to match exactly. |
| `motion.duration` | `Style::animation_time` | 0.12 | | Use 0.0 when the OS asks for reduced motion. |
| type | `Style::text_styles` | `Small` → 11 sans · `Body` → 12 sans · `Button` → 12 **medium** · `Heading` → 13 **medium** · `Monospace` → 12 mono · `Name("label")` → 11 medium · `Name("title")` → 13 medium | | Register IBM Plex Sans Regular, **IBM Plex Sans Medium as its own `FontFamily::Name("medium")`** and IBM Plex Mono. |
| cursor | `interact_cursor` | `Some(CursorIcon::PointingHand)` | | Scrim buttons use `cursor: pointer`. |
| misc | `button_frame = true`, `collapsing_header_frame = false`, `striped = false`, `window_highlight_topmost = false`, `handle_shape = HandleShape::Circle` | | | |

**Code sketch** for the dark theme; light is the same shape with `color.light.*`:

```rust
fn hex(s: &str) -> Color32 { /* parse #rrggbb / #rrggbbaa → from_rgba_unmultiplied */ }

let mut v = Visuals::dark();
v.panel_fill = hex("#111214");
v.window_fill = hex("#232428");
v.window_stroke = Stroke::new(1.0, hex("#313338"));
v.window_shadow = Shadow { offset: [0, 8], blur: 24, spread: 0, color: hex("#00000066") };
v.popup_shadow = v.window_shadow;
v.window_corner_radius = CornerRadius::same(8);
v.menu_corner_radius = CornerRadius::same(8);
v.extreme_bg_color = hex("#1b1c1f");
v.faint_bg_color = hex("#1b1c1f");
v.code_bg_color = hex("#1b1c1f");
v.hyperlink_color = hex("#a7aab0");
v.warn_fg_color = hex("#e6a23a");
v.error_fg_color = hex("#ff5c5c");
v.selection.bg_fill = hex("#2c2d32");
v.selection.stroke = Stroke::new(1.0, hex("#ececee"));
v.slider_trailing_fill = false;
v.interact_cursor = Some(CursorIcon::PointingHand);
let w = |bg: &str, weak: &str, stroke: Stroke, fg: &str| WidgetVisuals {
    bg_fill: hex(bg), weak_bg_fill: hex(weak), bg_stroke: stroke,
    fg_stroke: Stroke::new(1.0, hex(fg)), corner_radius: CornerRadius::same(4), expansion: 0.0,
};
let line1 = Stroke::new(1.0, hex("#232428"));
let line2 = Stroke::new(1.0, hex("#313338"));
v.widgets.noninteractive = w("#111214", "#111214", line1, "#ececee");
v.widgets.inactive = w("#1b1c1f", "#1b1c1f", Stroke::NONE, "#ececee");
v.widgets.hovered = w("#232428", "#232428", line2, "#ececee");
v.widgets.active = w("#232428", "#2c2d32", line2, "#ececee");
v.widgets.open = w("#232428", "#232428", line2, "#ececee");

style.spacing.item_spacing = vec2(8.0, 4.0);
style.spacing.button_padding = vec2(8.0, 4.0);
style.spacing.interact_size = vec2(24.0, 24.0);
style.spacing.indent = 16.0;
style.spacing.menu_margin = Margin::same(4);
style.animation_time = 0.12;

// the focus ring egui doesn't draw:
if resp.has_focus() {
    ui.painter().rect_stroke(resp.rect.expand(2.0), CornerRadius::same(5), Stroke::new(2.0, accent_mark), StrokeKind::Middle);
}
```

### What egui can't express (and the workaround)

| Scrim feature | Why it doesn't map | Workaround |
|---|---|---|
| A separate **focus-visible ring** (2 px accent, 1 px offset) | egui folds `has_focus()` into the **active** style and doesn't tell keyboard focus from pointer focus. | Paint the ring after the widget when it has focus. Track whether the last input was a key (Tab or arrows) so a mouse click doesn't show it. |
| **Neutral selection vs accent text selection** | `selection.bg_fill` serves selected widgets **and** selected text; `selection.stroke` serves selected-label text **and** the focused TextEdit border. | Keep the neutral values above. If you want an accent text highlight, set it per TextEdit via its layouter or `TextFormat`. |
| **Selected layer row's 2 px left accent edge** and the descendant band | There is no per-row edge in `Visuals`. | Paint it: fill `rect` with `bg-4`, then a 2 × h rect in `accent.mark` at the left. |
| **Icon buttons vs text buttons** use different rest colours (`text-2` vs `text-1`) and hover fills (`field` vs `raised`) | One `inactive` / `hovered` pair covers every `Button`. | Make an `icon_button()` helper that paints its own background and tint. |
| **Field border only on hover**, while buttons never get one | One `hovered.bg_stroke` covers buttons and fields. | Accept a hairline on button hover, or use `Button::frame(false)` and paint the background. |
| **Checkbox edge at rest** (`line-2`) while fields are borderless | Both use `inactive.bg_stroke`. | Paint the checkbox, or accept no edge. |
| **Primary / danger buttons** | Not a visual state. | Use `Button::fill()` with explicit hover and pressed colours from `state.*` (egui has no `filter: brightness`). |
| **Font weight** | `FontId` has no weight. | Register Plex Sans Medium as its own family (`FontFamily::Name("medium")`). |
| **Line height** 16 / 18 px | Text styles have no line height. | Set `TextFormat::line_height = Some(16.0)` in `LayoutJob`s. For single-line widgets, the row and control heights matter more than the line box. |
| **Tabular numerals** (`font-variant-numeric: tabular-nums`) | egui doesn't apply OpenType features. | Check whether the bundled Plex Sans's default figures are tabular. If they aren't, ship a build with `tnum` frozen in (`pyftfeatfreeze -f tnum`). |
| **Two-layer shadow** (1 px ring + drop) | `Shadow` is a single layer. | `window_stroke` stands in for the ring. |
| **Negative shadow spread** (the artboard `-20` / `-24`) | `Shadow.spread` is `u8`. | Canvas-only; draw the artboard shadow yourself, or use spread 0 with less blur. |
| **One easing curve** `cubic-bezier(0.2, 0, 0, 1)` | `animation_time` is global and the built-in animations use their own curve. | Use `ctx.animate_bool_with_time_and_easing` (or equivalent) with a cubic-bezier function where it matters. |
| **Reduced motion** | egui can't read the OS setting. | Read it on the platform side and set `animation_time = 0`. |
| **Translucent colours** (tints, `color-mix`) | egui `Color32` is premultiplied. | Build them with `from_rgba_unmultiplied` (the values here are straight alpha), or paint the composited opaque colour; `contrast.json` gives the composite over the panel. |
| **Disabled = colour change** (`text-off`), not fading | `add_enabled(false)` fades the whole widget by `disabled_alpha` / `fade_out_to_color`. | Close enough for fields (Scrim also uses opacity 0.5). For buttons, menu rows and icons, set the colour to `text.disabled`. |
| **Tooltip surface** differs from window surface (dark `#18191b` tooltip in the light theme) | Tooltips use `window_fill`. | Wrap tooltip content in a `Frame` with `surface.tooltip` and `text.onTooltip`. |
| **Tool bar** is `surface.raised`, not `surface.float` | | Give the tool bar its own `Frame`. |
| **Canvas overlay** (selection, handles, guides, chips, measure) | Not a widget concern. | Paint it with `Painter` from `color.<theme>.canvasChrome.*` and the geometry in section 3. |
| **Accent derivation** | | Port `derive.ts` (about 200 lines of pure maths, no dependencies) or ship only the six presets from `accentRule.presets`. |

---

## 8. Contrast findings

The full list is in `contrast.json`. Every pair the repo enforces (`themeContrast.test.ts`) passes in both themes, at the orange and violet defaults. Reading `theme.css` turned up these additional pairs that **fail** their floor:

| Theme | Pair | Ratio | Needs | Where |
|---|---|---|---|---|
| light | `text-1` on `tooltip-bg` | **1.00** | 4.5 | **The modifier pill** (`.scrim-mod-pill`, theme.css:1569) sets `color: var(--scrim-text-1)` on the tooltip surface. In light that is near-black on near-black, so the pill is unreadable. Confirmed by computing the styles in Chrome. It should use `--scrim-tooltip-text`. Don't copy this. |
| both | `text-off` on `bg-1` | 3.99 / 3.19 | 4.5 | Code tab line numbers (`.scrim-code-line__n`). They aren't disabled, so the exemption doesn't apply; use `text-3`. |
| both | `agent-mark` on `bg-4` | 4.20 / 4.12 | 4.5 | The "agent" word (11 px) on a **selected** layer row. The derive rule only guarantees 3:1 for marks on `bg-4`. |
| both | `accent-on-fill` on the pressed primary button | 4.38 / 4.38 | 4.5 | `brightness(0.84)` darkens the orange under its near-black label. This holds only for the pressed instant. |
| light | `warning` on `warning-tint` over `bg-1` | 4.49 | 4.5 | Lint chips and counts on the warning wash; a rounding hair under the floor. |
| light | `accent-fill` on `bg-0` | 2.64 | 3 | A canvas distance chip's orange block against the light canvas field. Its label still reads at 6.0:1. |

Two pairs pass but sit at the floor:
- `text-3` on `bg-4` (4.57 dark / 4.67 light) is used for the 11 px layer meta on a selected row, although ui-system.md says that pair is never used under 12 px.
- `accent-mark` on a white artboard is 3.12 in dark.

The six accent presets all meet the derive contract (`contrast.json › accentPresets`).

---

## 9. Underspecified, and where docs and code differ

- **`ui-system.md` is headed "proposal only",** but most of it shipped. These differences are resolved in favour of theme.css:
  - The default accent is **orange `#ff5a1f`**, with a near-black label, not the recommended blue.
  - The accent is user-selectable and *derived*; the mark and fill split comes from `derive.ts`, not from fixed hex pairs.
  - The light theme's danger (`#cf2a2a`) and warning (`#985f08`) were darkened from the proposal to pass on `bg-2`.
  - The agent default is `#8e7bff`, not `#9b8cff`.
- **The comment at the top of theme.css** still describes the older spec: "Plex Mono for every value", "teal = bound", "green = live" and weights up to 600. The tokens and the rules below it don't do any of that. `.scrim-mono` maps to the sans family, and only 400 and 500 are used.
- **Tool bar surface:** the tool bar and its flyouts use `bg-3`, while the Image tool's menu uses `bg-float`. In light those are `#ebebea` and `#ffffff`, but ui-system.md puts every floating surface on `bg-float`.
- **Icon set:** the 1.25 px stroke and the 16 px grid are specified in ui-system.md but aren't tokens. The icons are SVGs in `apps/studio` and aren't part of this kit.
- **Scrollbar width:** CSS `scrollbar-width: thin` is browser-defined, about 6–8 px on macOS. No px value is specified.
- **Text caret colour** isn't specified; the browser uses `currentColor`.
- **Dead CSS:** `.scrim-tip__key` (an accent-on-fill key on a `text-2` block, about 2.1:1 in light) isn't used by any component. It is left out of the pairs.
- **The light theme's `--scrim-on-warning` is white,** while the dark one is `#111214`. Both pass.
- **The brightness filter** (primary and danger hover and pressed) is computed in sRGB. Chrome's screenshot of it reads a few levels darker in dark (for example, pressed `#cd4919` measured against `#d64c1a` computed), which is display colour management. Treat the computed values as the design intent.
- **Unspecified, so values here are recommendations:** the egui `window_margin` for popovers, the scrollbar `bar_width`, and the caret stroke width.
