//! The theme is Scrim's UI system (`docs/scrim-transfer/ui/`): the ramp matches the exported
//! tokens, the accent is the one `accent::derive` gives each theme, and every text pair the chrome
//! uses reads at WCAG AA (4.5:1 for text, 3:1 for marks).

use egui::Color32;
use serde_json::Value;

use crate::theme::accent::{Ramp, contrast, derive, hex, parse_hex};
use crate::theme::{ACCENT, ACCENT_ON, Tokens};

fn tokens_json() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/ui/tokens.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn ramp(t: &Tokens) -> Ramp {
    Ramp { canvas: t.canvas, panel: t.panel, field: t.field, raised: t.pressed, selected: t.selected_row, float: t.menu }
}

fn themes() -> [(&'static str, Tokens); 2] {
    [("dark", Tokens::dark()), ("light", Tokens::light())]
}

#[test]
fn the_ramp_is_scrims() {
    let j = tokens_json();
    for (name, t) in themes() {
        let r = &j["accentRule"]["ramp"][name];
        let want = |k: &str| parse_hex(r[k].as_str().unwrap()).unwrap();
        let got = ramp(&t);
        for (k, c) in [
            ("canvas", got.canvas),
            ("panel", got.panel),
            ("field", got.field),
            ("raised", got.raised),
            ("selected", got.selected),
            ("float", got.float),
        ] {
            assert_eq!(hex(c), hex(want(k)), "{name} {k}");
        }
    }
}

#[test]
fn the_accent_is_the_one_derived_for_each_theme() {
    for (name, t) in themes() {
        let a = derive(ACCENT, &ramp(&t), t.dark).unwrap();
        assert_eq!(hex(t.accent), hex(a.mark), "{name} mark");
        assert_eq!(hex(t.accent_fill), hex(a.fill), "{name} fill");
        assert_eq!(hex(t.accent_on), hex(a.on_fill), "{name} label on the fill");
        assert_eq!(t.accent_tint, a.tint, "{name} tint");
        assert_eq!(t.focus, t.accent, "{name}: the focus ring is the mark");
    }
    assert_eq!(ACCENT_ON, derive(ACCENT, &ramp(&Tokens::dark()), true).unwrap().on_fill);
}

#[test]
fn chrome_text_reads_at_aa_in_both_themes() {
    for (name, t) in themes() {
        let text_pairs: [(&str, Color32, Color32); 13] = [
            ("text on panel", t.text, t.panel),
            ("text on field", t.text, t.field),
            ("text on selected row", t.text, t.selected_row),
            ("secondary on panel", t.text_secondary, t.panel),
            ("secondary on field", t.text_secondary, t.field),
            ("tertiary on panel", t.text_tertiary, t.panel),
            ("tertiary on field", t.text_tertiary, t.field),
            ("menu text on menu", t.menu_text, t.menu),
            ("menu text on highlighted row", t.menu_text, t.menu_hover),
            ("menu shortcut on menu", t.menu_dim, t.menu),
            ("label on accent fill", t.accent_on, t.accent_fill),
            ("label on hovered accent", t.accent_on, t.accent_hover),
            ("accent text on panel", t.accent_text, t.panel),
        ];
        for (what, fg, bg) in text_pairs {
            let r = contrast(fg, bg);
            assert!(r >= 4.5, "{name}: {what} is {r:.2}:1 ({} on {})", hex(fg), hex(bg));
        }
        for (what, fg, bg) in [
            ("accent mark on canvas", t.accent, t.canvas),
            ("focus ring on panel", t.focus, t.panel),
            ("strong border on panel", t.border_strong, t.panel),
        ] {
            let r = contrast(fg, bg);
            // Borders are decoration at 1.3:1 or more; marks and rings need 3:1.
            let floor = if what.starts_with("strong border") { 1.3 } else { 3.0 };
            assert!(r >= floor, "{name}: {what} is {r:.2}:1");
        }
    }
}
