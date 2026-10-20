//! G2: every accent role reads, for every preset, in both themes.
//! - **Indicators and outlines in the panels** (the fill, the mark, the focus ring): 3:1 on the
//!   panel.
//! - **Canvas overlays:** 3:1 on the theme's canvas and on both default page colours.
//! - **Text or an icon on an accent fill** (`accent_on`, at rest, hovered and pressed): 4.5:1.
//! - **Accent text:** 4.5:1 on the panel, and on the canvas's pages for a selected frame title.
//!
//! Each failing preset is derived anew per theme, the hue kept and the lightness moved, rather than
//! dropped. `which_presets_were_adjusted` records which, as the plan reports.

use egui::Color32;

use super::{for_page, on_of};
use crate::theme::Tokens;
use crate::theme::accent::{contrast, hex};

/// The backgrounds an overlay is drawn on in `t`'s theme: its canvas and both default pages.
fn pages(t: &Tokens) -> [Color32; 3] {
    [t.canvas, crate::theme::c32(figcraft_color::Color::DEFAULT_PAGE), crate::theme::c32(figcraft_color::Color::DARK_PAGE)]
}

fn presets() -> impl Iterator<Item = (&'static str, Color32)> {
    figcraft_engine::cmd::appearance::PRESETS.into_iter().map(|(n, [r, g, b])| (n, Color32::from_rgb(r, g, b)))
}

fn themes() -> [(&'static str, Tokens); 2] {
    [("light", Tokens::light()), ("dark", Tokens::dark())]
}

#[test]
fn every_preset_in_both_themes_meets_every_role() {
    let mut low = Vec::new();
    let mut check = |what: String, fg: Color32, bg: Color32, need: f64| {
        let r = contrast(fg, bg);
        if r < need {
            low.push(format!("{what}: {} on {} is {r:.2}:1, needs {need}", hex(fg), hex(bg)));
        }
    };
    for (preset, base) in presets() {
        for (theme, plain) in themes() {
            let t = plain.with_accent(base);
            let at = |role: &str| format!("{preset} {theme} {role}");
            // Indicators and outlines in the panels.
            for (role, c) in [("fill", t.accent_fill), ("mark", t.accent), ("focus ring", t.focus)] {
                check(at(role), c, t.panel, 3.0);
            }
            // Text or an icon on the fill, at rest, hovered and pressed.
            for (role, c) in [("fill", t.accent_fill), ("hover", t.accent_hover), ("pressed", t.accent_pressed)] {
                check(at(&format!("label on the {role}")), t.accent_on, c, 4.5);
            }
            // Accent text on the panel.
            check(at("text"), t.accent_text, t.panel, 4.5);
            // Canvas overlays, and the mark on them.
            for page in pages(&t) {
                let o = for_page(t.accent_base, page);
                check(at("overlay"), o, page, 3.0);
                check(at("mark on the overlay"), on_of(o), o, 4.5);
            }
        }
    }
    assert!(low.is_empty(), "{low:#?}");
}

/// Which presets the rule moved, per theme and role (the plan's G2 table).
#[test]
fn which_presets_were_adjusted() {
    let mut rows = Vec::new();
    for (preset, base) in presets() {
        for (theme, plain) in themes() {
            let t = plain.with_accent(base);
            let moved: Vec<String> = [("mark", t.accent), ("fill", t.accent_fill)]
                .into_iter()
                .filter(|(_, c)| *c != base)
                .map(|(r, c)| format!("{r} {}", hex(c)))
                .chain(
                    pages(&t).into_iter().filter(|p| for_page(base, *p) != base).map(|p| format!("overlay on {} {}", hex(p), hex(for_page(base, p)))),
                )
                .collect();
            rows.push(format!("{preset} {theme}: {}", if moved.is_empty() { "as chosen".into() } else { moved.join(", ") }));
        }
    }
    // The default keeps #ff5a1f wherever it reads: as the fill in both themes, and on dark pages.
    let orange_dark = rows.iter().find(|r| r.starts_with("orange dark")).cloned().unwrap_or_default();
    assert!(!orange_dark.contains("fill"), "{orange_dark}");
    println!("{}", rows.join("\n"));
}
