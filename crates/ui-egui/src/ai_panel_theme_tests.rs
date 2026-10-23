//! Fix X1: the AI panel's colours come from the theme only.

/// The AI panel's source files: the panel, the composer, its icons and roles, the run's banner and
/// card, and the chat's state and conversations.
pub(crate) const FILES: &[(&str, &str)] = &[
    ("ai_panel.rs", include_str!("ai_panel.rs")),
    ("ai_composer.rs", include_str!("ai_composer.rs")),
    ("ai_chips.rs", include_str!("ai_chips.rs")),
    ("ai_run_card.rs", include_str!("ai_run_card.rs")),
    ("ai_icons.rs", include_str!("ai_icons.rs")),
    ("ai_panel_theme.rs", include_str!("ai_panel_theme.rs")),
    ("chat_ui.rs", include_str!("chat_ui.rs")),
    ("chat_store.rs", include_str!("chat_store.rs")),
];

/// A written colour in `src` (comments left out): a `Color32` made from numbers, a hex helper, or the
/// fixed accent and brand constants.
pub(crate) fn written_colours(src: &str) -> Vec<String> {
    let code = src.lines().map(|l| l.split("//").next().unwrap_or_default()).collect::<Vec<_>>().join("\n");
    let mut found = Vec::new();
    for pat in ["Color32::from_rgb", "from_rgba_unmultiplied", "from_rgba_premultiplied", "from_gray"] {
        if code.contains(pat) {
            found.push(pat.to_string());
        }
    }
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut word = String::new();
    // A space after the end, so the last word is looked at too.
    for c in code.chars().chain([' ']) {
        if ident(c) {
            word.push(c);
            continue;
        }
        let call = c == '(';
        if matches!(word.as_str(), "ACCENT" | "BRAND" | "ACCENT_ON" | "BRAND_ON") {
            found.push(word.clone());
        } else if word == "hex" && call {
            found.push("hex(".into());
        }
        word.clear();
    }
    found
}

#[test]
fn no_ai_panel_file_writes_a_colour() {
    for (name, src) in FILES {
        let found = written_colours(src);
        assert!(found.is_empty(), "{name} writes colours ({found:?}): use the roles in ai_panel_theme.rs");
    }
    // The guard sees each kind it's for.
    for bad in [
        "let c = Color32::from_rgb(1, 2, 3);",
        "c = theme::ACCENT;",
        "c = ACCENT_ON;",
        "x = hex(0xffffff);",
        "BRAND",
        "Color32::from_rgba_unmultiplied(1, 2, 3, 4)",
    ] {
        assert!(!written_colours(bad).is_empty(), "{bad}");
    }
    for fine in ["t.accent", "t.accent_on", "// Color32::from_rgb in a comment", "accent_fill", "rgb_text(c)"] {
        assert!(written_colours(fine).is_empty(), "{fine}");
    }
}

#[test]
fn readable_on_picks_white_or_ink_at_4_5() {
    use super::readable_on;
    use crate::theme::Tokens;
    use crate::theme::accent::{INK, contrast};
    for t in [Tokens::light(), Tokens::dark()] {
        assert!(contrast(readable_on(t.success), t.success) >= 4.5, "{:?}", t.success);
    }
    assert_eq!(readable_on(egui::Color32::BLACK), egui::Color32::WHITE);
    assert_eq!(readable_on(egui::Color32::WHITE), INK);
}
