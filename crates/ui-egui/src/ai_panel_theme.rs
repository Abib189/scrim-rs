//! The AI panel's colours by role (Fix X1, `docs/design/ai-panel-theme.md`): every colour the panel
//! draws comes from the theme's tokens ([`Tokens::get`]), so the light or dark theme and the
//! person's accent both apply. No colour is written here or in any AI-panel file
//! (`ai_panel_theme_tests.rs` › `no_ai_panel_file_writes_a_colour` fails if one does).

use egui::Color32;

use crate::theme::Tokens;
use crate::theme::accent::{INK, contrast, from_hsl, over, to_hsl};

/// The colour of each part of the panel, from `t`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roles {
    pub panel: Color32,
    pub header_rule: Color32,
    /// Follow and New chat at rest.
    pub button_fill: Color32,
    pub follow_on_fill: Color32,
    pub follow_on_icon: Color32,
    pub follow_off_icon: Color32,
    pub new_chat_icon: Color32,
    pub new_chat_disabled: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub placeholder: Color32,
    /// Accent-coloured words (a link).
    pub link: Color32,
    pub chevron: Color32,
    pub composer_fill: Color32,
    pub chip_fill: Color32,
    pub bubble: Color32,
    pub banner: Color32,
    pub card: Color32,
    pub card_edge: Color32,
    pub card_button_edge: Color32,
    pub send_fill: Color32,
    pub send_glyph: Color32,
    /// The working dots, the active step's ring and dot, the progress, the empty state's core.
    pub mark: Color32,
    pub empty_disc: Color32,
    /// The status dot while following, and a done step's circle.
    pub success: Color32,
    pub done_check: Color32,
    pub pending_ring: Color32,
    pub progress_track: Color32,
    pub status_off_dot: Color32,
    /// Following paused while an agent works (Fix N): the dot on the Follow button.
    pub paused_dot: Color32,
    /// A run's failure line in the transcript.
    pub failure: Color32,
    /// A reply's code: the inline span's fill and the block's box (Fix Z2).
    pub code: Color32,
    /// The Selection and Image chips' icons, and their fill hovered (Fix Z6).
    pub chip_icon: Color32,
    pub chip_hover: Color32,
}

/// The roles for theme `t` (its accent included).
pub fn roles(t: &Tokens) -> Roles {
    Roles {
        panel: t.panel,
        header_rule: t.border,
        button_fill: t.pressed,
        follow_on_fill: t.accent_tint,
        // The accent, on its own tint: moved until it reads there (Fix X5).
        follow_on_icon: readable_mark(t.accent, over(t.accent_tint, t.panel), t.dark),
        follow_off_icon: t.icon,
        new_chat_icon: t.icon,
        new_chat_disabled: t.disabled,
        text: t.text,
        text_secondary: t.text_secondary,
        placeholder: t.text_tertiary,
        link: t.accent_text,
        chevron: t.icon,
        composer_fill: t.field,
        chip_fill: t.selected_row,
        bubble: t.selected_row,
        banner: t.field,
        card: t.field,
        card_edge: t.border_strong,
        card_button_edge: t.field_edge,
        send_fill: t.accent_fill,
        send_glyph: t.accent_on,
        mark: t.accent,
        empty_disc: t.pressed,
        success: t.success,
        done_check: readable_on(t.success),
        pending_ring: t.field_edge,
        progress_track: t.border_strong,
        status_off_dot: t.text_secondary,
        paused_dot: t.warning,
        failure: t.danger,
        code: t.field,
        chip_icon: t.icon,
        chip_hover: t.pressed,
    }
}

/// `mark` where it reads at 4.5:1 on `bg`; otherwise its lightness moved (lighter on a dark theme,
/// darker on a light one), the hue kept, until it does. The accent's mark is derived for the plain
/// surfaces (`accent::derive`); on its own tint (Follow on) it can fall just short.
pub fn readable_mark(mark: Color32, bg: Color32, dark: bool) -> Color32 {
    if contrast(mark, bg) >= 4.5 {
        return mark;
    }
    let (h, s, l) = to_hsl(mark);
    let dir = if dark { 1.0 } else { -1.0 };
    (1..=200).map(|n| from_hsl(h, s, l + dir * 0.005 * f64::from(n))).find(|c| contrast(*c, bg) >= 4.5).unwrap_or(if dark {
        Color32::WHITE
    } else {
        INK
    })
}

/// White or the near-black ink, whichever reads at 4.5:1 on `fill` (the better of the two when
/// neither does): the check in a done step's circle.
pub fn readable_on(fill: Color32) -> Color32 {
    let (white, ink) = (contrast(Color32::WHITE, fill), contrast(INK, fill));
    match (white >= 4.5, ink >= 4.5) {
        (true, _) => Color32::WHITE,
        (false, true) => INK,
        _ if white >= ink => Color32::WHITE,
        _ => INK,
    }
}

#[cfg(test)]
#[path = "ai_panel_theme_tests.rs"]
mod tests;
