//! Fix X1: the AI panel's colours come from the theme only.

/// The AI panel's source files: the panel, the composer, its icons and roles, the run's banner and
/// card, and the chat's state and conversations.
pub(crate) const FILES: &[(&str, &str)] = &[
    ("ai_panel.rs", include_str!("ai_panel.rs")),
    ("ai_composer.rs", include_str!("ai_composer.rs")),
    ("ai_chips.rs", include_str!("ai_chips.rs")),
    ("ai_run_card.rs", include_str!("ai_run_card.rs")),
    ("ai_reply.rs", include_str!("ai_reply.rs")),
    ("ai_markdown.rs", include_str!("ai_markdown.rs")),
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

/// The panel drawn in theme `dark` with accent `accent` (`#rrggbb`), idle and then mid-run with
/// the card's done and active rows: the colour drawn for each role (`ui.inspect` › `aiPanel.theme`),
/// and the theme's tokens.
pub(crate) fn drawn(dark: bool, accent: &str) -> (std::collections::BTreeMap<String, egui::Color32>, crate::theme::Tokens) {
    use crate::follow_agent::chat_ui::tests::{send_allowed, window};
    use crate::follow_agent::tests::agent;
    use serde_json::json;
    let (mut u, _) = window();
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    u.app.session.prefs.accent = accent.into();
    u.frame(vec![]);
    u.frame(vec![]);
    let mut seen = std::collections::BTreeMap::new();
    let mut read = |u: &crate::layers_drag_tests::Ui| {
        let theme = crate::follow_agent::inspect(&u.app)["aiPanel"]["theme"].clone();
        for (k, v) in theme.as_object().cloned().unwrap_or_default() {
            if let Some(c) = v.as_str().and_then(crate::theme::accent::parse_hex) {
                seen.insert(k, c);
            }
        }
    };
    read(&u);
    send_allowed(&mut u, "Build a poster called Welle");
    u.frame(vec![]);
    let f = agent(&mut u, "frame.create", json!({"x": 400, "y": 0, "width": 300, "height": 200}));
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 30, "height": 30, "parent": f["id"]}));
    agent(&mut u, "text.create", json!({"text": "Welle", "parent": f["id"]}));
    u.frame(vec![]);
    u.frame(vec![]);
    read(&u);
    (seen, crate::theme::Tokens::get(&u.ctx))
}

/// Every accent the tests try: the presets, a very pale one and a very dark one.
pub(crate) fn accents() -> Vec<String> {
    let mut v: Vec<String> = figcraft_engine::cmd::appearance::PRESETS.iter().map(|(_, c)| figcraft_engine::cmd::appearance::hex(*c)).collect();
    v.extend(["#ffe066".to_string(), "#1f3bff".to_string()]);
    v
}

/// Fix X5: in both themes and with every accent, what the panel draws reads: accent marks at 4.5:1
/// on the panel and the field; the glyph on Send/Stop at 4.5:1; the done check at 4.5:1 on its
/// circle; the Follow icon at 4.5:1 on its tint over the panel; text at 4.5:1 on its surfaces; the
/// composer's edge and the pending ring at 3:1.
#[test]
fn every_role_reads_in_both_themes_with_every_accent() {
    use crate::theme::accent::{contrast, over};
    for dark in [false, true] {
        for a in accents() {
            let (d, t) = drawn(dark, &a);
            let at = |k: &str| *d.get(k).unwrap_or_else(|| panic!("{k} not drawn ({dark} {a}): {d:?}"));
            let say = |what: &str, fg: egui::Color32, bg: egui::Color32, min: f64| {
                let c = contrast(fg, bg);
                assert!(c >= min, "{} {a}: {what} {fg:?} on {bg:?} is {c:.2}:1, under {min}", if dark { "dark" } else { "light" });
            };
            // (a) The accent's marks.
            for k in ["workingDots", "activeRing", "progressFill", "emptyCore"] {
                say(k, at(k), t.panel, 4.5);
                say(k, at(k), t.field, 4.5);
            }
            // (b) Send / Stop.
            say("send glyph", at("sendGlyph"), at("sendFill"), 4.5);
            // (c) The done check.
            say("done check", at("doneCheck"), at("doneCircle"), 4.5);
            // (d) Follow, on: its icon on its tint over the panel.
            say("follow icon", at("followIcon"), over(t.accent_tint, t.panel), 4.5);
            // (e) Text on its surfaces.
            for (what, fg, bg) in [
                ("text on the panel", t.text, t.panel),
                ("text on the bubble", at("chipText"), at("bubble")),
                ("text on the card", t.text, at("card")),
                ("text on the banner", t.text, at("banner")),
                ("text on a chip", at("chipText"), at("chips")),
                ("secondary on the panel", t.text_secondary, t.panel),
                ("secondary on the card", t.text_secondary, at("card")),
                ("secondary on a chip", at("chipSecondary"), at("chips")),
                ("placeholder on the composer", at("placeholder"), at("composerFill")),
            ] {
                say(what, fg, bg, 4.5);
            }
            // (f) Edges.
            say("composer edge on the panel", at("composerEdge"), t.panel, 3.0);
            say("composer edge on its fill", at("composerEdge"), at("composerFill"), 3.0);
            let pending = super::roles(&t).pending_ring;
            say("pending ring on the card", pending, at("card"), 3.0);
            // Fix AB1: the design's text-3 and accent-text, on the surfaces they're drawn on.
            let ro = super::roles(&t);
            for (what, fg, bg) in [
                ("text-3 on the panel", ro.text_3, t.panel),
                ("text-3 on a surface", ro.text_3, t.field),
                ("the placeholder on a surface", ro.placeholder, t.field),
                ("text-2 on raised", t.text_secondary, t.selected_row),
                ("text on the bubble", t.text, ro.bubble),
                ("accent-text on a surface", ro.link, t.field),
                ("accent-text on raised", ro.link, t.selected_row),
                ("on-accent on the accent", ro.send_glyph, ro.send_fill),
            ] {
                say(what, fg, bg, 4.5);
            }
            assert_eq!(at("statusText"), ro.text_3, "the status line is text-3");
            // The Selection and Image chips' icons, at rest and hovered (Fix Z6).
            say("chip icon on a chip", at("chipIcon"), at("chips"), 3.0);
            say("chip icon on a hovered chip", at("chipIcon"), at("chipHover"), 3.0);
        }
    }
}

/// Fix X5: the accent changes the accent's roles and none of the others.
#[test]
fn the_accent_changes_only_the_accent_roles() {
    use crate::theme::Tokens;
    for (base, other) in [(Tokens::light(), "#4a86ff"), (Tokens::dark(), "#ffe066"), (Tokens::dark(), "#1f3bff")] {
        let a = super::roles(&base);
        let b = super::roles(&base.with_accent(crate::theme::accent::parse_hex(other).unwrap()));
        assert_ne!(a.mark, b.mark, "{other}: the marks follow the accent");
        assert_ne!(a.send_fill, b.send_fill);
        // Every role but the accent's stays as it was.
        let neutral = |r: &super::Roles| {
            let mut r = *r;
            for c in [&mut r.follow_on_fill, &mut r.follow_on_icon, &mut r.link, &mut r.send_fill, &mut r.send_glyph, &mut r.mark] {
                *c = egui::Color32::TRANSPARENT;
            }
            r
        };
        assert_eq!(neutral(&a), neutral(&b), "{other}: only the accent roles change");
    }
}

/// Fix AB1: with the default accent, the AI panel's text-3 and accent-text are the design file's
/// variables ("Scrim AI panel" collection, `ai/text-3` and `ai/accent-text`), and the placeholder,
/// status line and links use them.
#[test]
fn text_3_and_accent_text_are_the_designs() {
    use crate::theme::accent::rgb_text;
    for (t, text_3, accent_text) in [(crate::theme::Tokens::dark(), "#8a8d94", "#ff7a47"), (crate::theme::Tokens::light(), "#6b6e75", "#b02e00")] {
        let ro = super::roles(&t);
        assert_eq!((rgb_text(ro.text_3), rgb_text(ro.placeholder)), (text_3.to_string(), text_3.to_string()), "dark {}", t.dark);
        assert_eq!(rgb_text(ro.link), accent_text, "dark {}", t.dark);
    }
}
