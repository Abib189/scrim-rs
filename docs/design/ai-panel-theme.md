# The AI panel's colours (Fix X1, 23 Oct 2026)

**Why:** the AI panel is rebuilt to Scrim's AI panel design, in dark and light (Fix X). Every colour it draws comes from the theme's tokens (`Tokens::get(ctx)`), so the light or dark theme and the person's accent (`pref.accent`) both apply. The roles are code, in `crates/ui-egui/src/ai_panel_theme.rs` › `roles`.

## Roles

| Part | Token |
|---|---|
| Panel background | `t.panel` |
| Header rule | `t.border` |
| Header buttons (Follow, New chat) at rest | fill `t.pressed` |
| Follow on | fill `t.accent_tint`, icon `t.accent` |
| Follow off | icon `t.icon` |
| Follow paused while an agent works (Fix N) | the dot `t.warning` |
| New chat disabled during a run (Fix U3) | `t.disabled` |
| Title and body text | `t.text` |
| Secondary text: notes, times, status, the Selection label, the grace hint | `t.text_secondary` |
| Placeholder | `t.text_tertiary` |
| Accent-coloured words (a link) | `t.accent_text` |
| Composer | fill `t.field`; edge `theme::field_stroke(t, hovered, focused)` (rest `t.field_edge`, focused `t.accent`) |
| Chips (agent, selection, image) | fill `t.selected_row` |
| Chevrons | `t.icon` |
| Your message's bubble | fill `t.selected_row` |
| Working banner | fill `t.field` |
| Run card | fill `t.field`, 1 px edge `t.border_strong`; its buttons a 1 px `t.field_edge` edge, label `t.text` |
| Send and Stop | circle `t.accent_fill`; the arrow and the stop square `t.accent_on` (white or near-black, whichever reads on the fill; never fixed) |
| Marks: the working dots, the active step's ring and dot, the progress, the empty state's core | `t.accent` |
| The empty state's disc behind the core | `t.pressed` |
| The status dot while following; a done step's circle | `t.success` |
| The check in a done circle | `readable_on(t.success)`: white or the near-black ink (`accent::INK`), whichever reads at 4.5:1 |
| A pending step's ring | `t.field_edge` (no pending steps are drawn: Fix X4) |
| The progress track | `t.border_strong` |
| The status dot while following is off | `t.text_secondary` |

## The guard

`ai_panel_theme_tests.rs` › `no_ai_panel_file_writes_a_colour` reads the AI panel's sources and fails on a colour written in code: `Color32::from_rgb…`, `from_rgba_unmultiplied`, `from_rgba_premultiplied`, `from_gray`, a `hex(` call, or the fixed `ACCENT`, `BRAND`, `ACCENT_ON` and `BRAND_ON` constants. Comments don't count.
- **Its files:** `ai_panel.rs`, `ai_composer.rs`, `ai_icons.rs`, `ai_panel_theme.rs`, `chat_ui.rs`, `chat_store.rs`, and the run's banner and card when Fix X adds them.
- **It also checks itself:** each kind of written colour is found, and token names such as `t.accent_on` aren't mistaken for one.

**Before Fix X1 the panel already wrote no colour** (the audit found none in those files), so the guard passed on the old code too. It is there so none comes in. Its self-check shows it would catch each kind.

## What changed in X1

- **The header buttons:** at rest they sit on `t.pressed`, not on nothing. Follow on is the accent's tint with the accent icon (it was the selected row's fill).
- **Follow off:** the crosshair is drawn in `t.icon` on the rest fill. Fix V3 had dimmed it to `t.disabled`; the brief's role table overrules that. Off now shows in the tooltip and in Fix X2's status line, "Following is off". `ui.inspect`'s `followDimmed` is gone; `followSetting` (ON or OFF) stays.
- **The empty state's disc:** `t.pressed` (it was `t.field`).
- **The grace hint's line:** `t.text_secondary` (it was `t.warning`). The role table has no warning colour; the paused dot on Follow keeps `t.warning`, as Fix N's look.
