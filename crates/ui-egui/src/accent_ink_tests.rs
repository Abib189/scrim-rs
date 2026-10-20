//! G1: the accent everywhere.
//! - **The overlay accent:** the chosen colour where it reads at 3:1 on the page, else its
//!   lightness moved (the hue kept).
//! - **In the window,** with green chosen, no interactive orange is painted anywhere below the tab
//!   bar: Share, the active tool, the rail, the selection and its handles all follow. The "S"
//!   badges in the tab bar keep the brand's.
//! - **Changing the accent** while a layer is selected repaints the handles on the next frame,
//!   with no input.

use egui::epaint::ColorMode;
use egui::{Color32, Rect, Shape};
use serde_json::json;

use super::{for_page, overlay};
use crate::live_tests::Harness;
use crate::theme::accent::{contrast, to_hsl};

/// Scrim's oranges: the default, its light-theme mark, hover and pressed.
const ORANGES: [Color32; 4] = [
    Color32::from_rgb(0xFF, 0x5A, 0x1F),
    Color32::from_rgb(0xC5, 0x34, 0x00),
    Color32::from_rgb(0xEB, 0x53, 0x1D),
    Color32::from_rgb(0xF5, 0x41, 0x00),
];

/// Every colour `shapes` paint, with where.
fn colours(shapes: &[Shape]) -> Vec<(Color32, Rect)> {
    let mut out = Vec::new();
    for s in shapes {
        let at = s.visual_bounding_rect();
        let mut push = |c: Color32| {
            if c.a() > 0 {
                out.push((c, at));
            }
        };
        match s {
            Shape::Rect(r) => {
                push(r.fill);
                push(r.stroke.color);
            }
            Shape::Circle(c) => {
                push(c.fill);
                push(c.stroke.color);
            }
            Shape::LineSegment { stroke, .. } => push(stroke.color),
            Shape::Path(p) => {
                push(p.fill);
                if let ColorMode::Solid(c) = p.stroke.color {
                    push(c);
                }
            }
            Shape::Mesh(m) => m.vertices.iter().for_each(|v| push(v.color)),
            Shape::CubicBezier(b) => {
                if let ColorMode::Solid(c) = b.stroke.color {
                    push(c);
                }
            }
            _ => {}
        }
    }
    out
}

fn opaque(c: Color32) -> Color32 {
    Color32::from_rgb(c.r(), c.g(), c.b())
}

#[test]
fn the_overlay_keeps_the_accent_where_it_reads_and_keeps_its_hue_where_it_doesnt() {
    let orange = Color32::from_rgb(0xFF, 0x5A, 0x1F);
    let dark_page = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    let light_page = Color32::from_rgb(0xF5, 0xF5, 0xF5);
    assert_eq!(for_page(orange, dark_page), orange, "the default on a dark page, unchanged");
    let on_light = for_page(orange, light_page);
    assert!(contrast(on_light, light_page) >= 3.0, "{on_light:?}");
    let ((h0, _, l0), (h1, _, l1)) = (to_hsl(orange), to_hsl(on_light));
    assert!((h0 - h1).abs() < 2.0 && l1 < l0, "darker, the same hue: {h0} {h1}");
    // Every preset on every page reaches 3:1.
    for (_, [r, g, b]) in figcraft_engine::cmd::appearance::PRESETS {
        for page in [dark_page, light_page, Color32::from_rgb(0xEC, 0xEC, 0xEA), Color32::from_rgb(0x17, 0x18, 0x1A), Color32::WHITE] {
            assert!(contrast(for_page(Color32::from_rgb(r, g, b), page), page) >= 3.0);
        }
    }
}

/// The editor with the harness's rectangle selected, in `theme`, with `preset`.
fn editor(theme: &str, preset: &str) -> Harness {
    let (mut h, rect) = Harness::new();
    h.app.run("pref.theme", json!({"theme": theme}));
    h.app.run("pref.accent", json!({"preset": preset}));
    h.app.session.execute("select.set", &json!({"ids": [rect]})).unwrap();
    h.frames(4);
    h
}

#[test]
fn with_green_no_interactive_orange_is_painted() {
    for theme in ["light", "dark"] {
        let mut h = editor(theme, "green");
        for tab in ["panel.tab.design", "panel.tab.ai"] {
            h.app.session.execute(tab, &json!({})).unwrap();
            h.app.handle_ui_requests();
            let shapes = h.frames(3);
            // Below the tab bar (whose "S" badges are the brand's).
            let orange: Vec<_> = colours(&shapes).into_iter().filter(|(c, r)| ORANGES.contains(&opaque(*c)) && r.min.y > 40.0).collect();
            assert!(orange.is_empty(), "{theme} {tab}: orange still painted at {:?}", orange.iter().map(|(_, r)| *r).collect::<Vec<_>>());
            // And the green is there: the active tool, Share, the selection.
            let fill = crate::theme::Tokens::get(&h.ctx).accent_fill;
            assert!(colours(&shapes).iter().any(|(c, _)| opaque(*c) == fill), "{theme} {tab}: the accent fill");
        }
    }
}

#[test]
fn changing_the_accent_repaints_the_handles_on_the_next_frame() {
    let mut h = editor("light", "green");
    let shapes = h.frames(1);
    let green = overlay();
    assert!(colours(&shapes).iter().any(|(c, _)| opaque(*c) == green), "the handles in green");
    // The accent changes (no pointer, no key): the very next frame paints the handles in pink.
    h.app.run("pref.accent", json!({"preset": "pink"}));
    let shapes = h.frames(1);
    let pink = overlay();
    assert_ne!(pink, green);
    let seen = colours(&shapes);
    assert!(seen.iter().any(|(c, _)| opaque(*c) == pink), "the handles in pink at once");
    assert!(!seen.iter().any(|(c, _)| opaque(*c) == green), "and nothing left in green");
}
