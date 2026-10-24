# The AI panel's colours (Fix X1, 23 Oct 2026)

**Why:** the AI panel is rebuilt to Scrim's AI panel design, in dark and light (Fix X). Every colour it draws comes from the theme's tokens (`Tokens::get(ctx)`), so the light or dark theme and the person's accent (`pref.accent`) both apply. The roles are code, in `crates/ui-egui/src/ai_panel_theme.rs` › `roles`.

## Roles

| Part | Token |
|---|---|
| Panel background | `t.panel` |
| Header rule | `t.border` |
| Header buttons (Follow, New chat) at rest | fill `t.pressed` |
| Follow on | fill `t.accent_tint`, icon `t.ai_accent_text` (Fix AB1) moved until it reads at 4.5:1 on that tint over the panel (`readable_mark`, Fix X5) |
| Follow off | icon `t.icon` |
| Follow paused while an agent works (Fix N) | the dot `t.warning` |
| New chat disabled during a run (Fix U3) | `t.disabled` |
| Title and body text | `t.text` |
| Secondary text: notes, times, status, the Selection label, the grace hint | `t.text_secondary` |
| Placeholder | `t.ai_text_3` (Fix AB1) |
| Tertiary text: the status line, the card's note, its Steps count (Fix AB1) | `t.ai_text_3` |
| Accent-coloured words (a link, the toast's action) | `t.ai_accent_text`, moved until it reads on the raised fill (`readable_mark`, Fix AB1) |
| Composer | fill `t.field`; edge `theme::field_stroke(t, hovered, focused)` (rest `t.field_edge`, focused `t.accent`) |
| Chips (agent, selection, image) | fill `t.selected_row`; the Selection and Image chips `t.pressed` hovered (Fix Z6) |
| The Selection and Image chips' icons (Fix Z6) | `t.icon` (3:1 on the chip, at rest and hovered) |
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
| A run's failure line | `t.danger` |
| The transcript's bottom fade (Fix Z4) | `t.panel`, from transparent |
| A reply's inline code and code box (Fix Z2) | fill `t.field`; the text `t.text` |
| The status dot while following is off | `t.text_secondary` |

## The guard

`ai_panel_theme_tests.rs` › `no_ai_panel_file_writes_a_colour` reads the AI panel's sources and fails on a colour written in code: `Color32::from_rgb…`, `from_rgba_unmultiplied`, `from_rgba_premultiplied`, `from_gray`, a `hex(` call, or the fixed `ACCENT`, `BRAND`, `ACCENT_ON` and `BRAND_ON` constants. Comments don't count.
- **Its files:** `ai_panel.rs`, `ai_composer.rs`, `ai_chips.rs`, `ai_icons.rs`, `ai_panel_theme.rs`, `ai_run_card.rs`, `ai_reply.rs` and `ai_markdown.rs` (Fix Z2), `chat_ui.rs` and `chat_store.rs`.
- **It also checks itself:** each kind of written colour is found, and token names such as `t.accent_on` aren't mistaken for one.

**Before Fix X1 the panel already wrote no colour** (the audit found none in those files), so the guard passed on the old code too. It is there so none comes in. Its self-check shows it would catch each kind.

## What changed in X1

- **The header buttons:** at rest they sit on `t.pressed`, not on nothing. Follow on is the accent's tint with the accent icon (it was the selected row's fill).
- **Follow off:** the crosshair is drawn in `t.icon` on the rest fill. Fix V3 had dimmed it to `t.disabled`; the brief's role table overrules that. Off now shows in the tooltip and in Fix X2's status line, "Following is off". `ui.inspect`'s `followDimmed` is gone; `followSetting` (ON or OFF) stays.
- **The empty state's disc:** `t.pressed` (it was `t.field`).
- **The grace hint's line:** `t.text_secondary` (it was `t.warning`). The role table has no warning colour; the paused dot on Follow keeps `t.warning`, as Fix N's look.

## What is drawn, and the tests (Fix X5)

**`ui.inspect` › `agent.aiPanel.theme`** gives the colour actually drawn for each role, `#rrggbb` as seen, recorded where it is painted:
- **The header:** `followIcon` and `followFill` (the tint as seen over the panel).
- **Send:** `sendFill` and `sendGlyph`.
- **The run:** `workingDots`, `activeRing`, `progressFill` and `progressTrack`, `doneCircle` and `doneCheck`, and `banner`, `bubble` and `card`.
- **The empty state:** `emptyCore` and `emptyDisc`.
- **The composer:** `statusDot`, `composerFill` and `composerEdge`, `chips`, `chipText` and `chipSecondary`, and `placeholder`.

A live check can assert these without a screenshot.

**`ai_panel_theme_tests.rs`:** in both themes, with every accent preset, `#FFE066` and `#1F3BFF`, from the drawn colours:

| Check | At least |
|---|---|
| The accent's marks (dots, active ring, bar, empty core) on the panel and the field | 4.5:1 |
| Send's glyph on its fill | 4.5:1 |
| The done check on its circle | 4.5:1 |
| The Follow icon on its tint over the panel | 4.5:1 |
| Text, secondary text and the placeholder on their surfaces (panel, bubble, card, banner, chips, composer) | 4.5:1 |
| The composer's edge on the panel and on its fill; the pending ring on the card | 3:1 |

- **Fixed colours fail it:** each of the 8 roles tried fails the test when made a fixed colour: the marks, Send's glyph, the done check, the Follow icon, the text, the placeholder, the pending ring, the card.
- **The accent touches only its own roles:** another test checks that changing the accent changes only the Follow tint and icon, the link, Send's fill and glyph, and the marks.
- **One check needed a fix:** the Follow icon on its own tint came out at 4.499:1 for the default orange in light. `readable_mark` now moves the icon's lightness, the hue kept, until it reads.

## The design file's text-3 and accent-text (Fix AB1)

The AI panel design's variable collection "Scrim AI panel" (Dark / Light, read with `variable.export`) is the colour source. Two of its values differed from the app's tokens, so the AI panel has its own tokens for them. The rest of the app keeps `text_tertiary` and `accent_text`.

| Token | Dark | Light | Was (the app's) |
|---|---|---|---|
| `ai_text_3` (`ai/text-3`) | `#8A8D94` | `#6B6E75` | `#91959C` / `#5F6268` |
| `ai_accent_text` (`ai/accent-text`) | `#FF7A47` | `#B02E00` | `#FF5A1F` / `#C53400` |

- **With another accent,** `ai_accent_text` is the accent's derived mark, as `accent_text` is. The role moves it until it reads at 4.5:1 on the raised fill: a blue accent read 4.17:1 there.
- **Tests** (`ai_panel_theme_tests.rs`):
  - `text_3_and_accent_text_are_the_designs`;
  - the contrast test now also checks text-3 on the panel and on a surface, text-2 on raised, text on the bubble, accent-text on a surface and on raised, and the label on the accent fill, all at 4.5:1 in both themes with every accent.
