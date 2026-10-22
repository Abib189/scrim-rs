//! Fix S2: every input's edge reads at 3:1 (WCAG 1.4.11, the boundary of a control).
//! Real screens are drawn (the properties panel's number fields, the Layers and Assets searches,
//! the AI panel's composer and its conversations' search (Fix T4), Settings ▸ General and Tool access, the batch rename dialog, the
//! shortcuts panel), in both themes and with every accent preset. For each text input, spin
//! button and combo box AccessKit reports, the smallest stroked rectangle drawn around it is its
//! edge. That edge must reach 3:1 against the field's fill, the panels and the floating surface
//! (dialogs and menus), at rest.
//! - **Exempt:** an input drawn as plain text, with no field box at all: the labels clicked to
//!   rename (the file's name, page names, frame labels). They're listed, not failed. An input on a
//!   field-coloured box without an edge fails.

use egui::Shape;
use egui::accesskit::Role;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::theme::{Tokens, accent::contrast};

/// The roles of inputs.
const INPUTS: [Role; 5] = [Role::TextInput, Role::MultilineTextInput, Role::SearchInput, Role::SpinButton, Role::ComboBox];

/// A screen: its name, and how it is opened on a window.
type Screen = (&'static str, fn(&mut Ui));

/// The screens, set up on `u`.
fn screens() -> Vec<Screen> {
    vec![
        ("properties", |u: &mut Ui| {
            u.app.run("panel.tab.design", json!({}));
        }),
        ("layers search", |u: &mut Ui| u.app.ui.layer_search = true),
        ("assets search", |u: &mut Ui| {
            u.app.run("panel.assets", json!({}));
        }),
        ("AI tab composer", |u: &mut Ui| {
            u.app.run("panel.tab.ai", json!({}));
        }),
        // Fix T4: the conversations' popover and its search.
        ("AI panel popover", |u: &mut Ui| {
            u.app.run("panel.tab.ai", json!({}));
            u.app.ui.agent_view.chat.store.open = true;
        }),
        ("Settings ▸ General", |u: &mut Ui| {
            u.app.ui.settings_section = "general";
            u.app.ui.preferences = true;
        }),
        ("Settings ▸ Tool access", |u: &mut Ui| {
            u.app.ui.settings_section = "tools";
            u.app.ui.preferences = true;
        }),
        ("batch rename", |u: &mut Ui| {
            let ids = u.app.session.selection();
            crate::dialogs::open_batch_rename(&mut u.app, &ids);
        }),
        ("shortcuts", |u: &mut Ui| u.app.ui.shortcuts_panel = true),
    ]
}

/// A window with two rectangles selected, its MCP endpoint up and an agent installed (so the
/// composer shows its field), in `theme` with accent `preset`.
fn window(dark: bool, preset: &str) -> Ui {
    let mut s = figcraft_engine::Session::new();
    let a = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 100, "height": 80})).unwrap();
    let b = s.execute("shape.rectangle", &json!({"x": 200, "y": 0, "width": 100, "height": 80})).unwrap();
    s.execute("select.set", &json!({"ids": [a["id"], b["id"]]})).unwrap();
    s.prefs.theme = if dark { "dark" } else { "light" }.into();
    s.agents.mcp.port = Some(7690);
    let mut u = Ui::new(s);
    let script = std::sync::Arc::new(std::sync::Mutex::new(crate::follow_agent::chat_ui::tests::Script {
        claude: Some("/opt/agents/bin/claude".into()),
        ..Default::default()
    }));
    u.app.services = Box::new(crate::follow_agent::chat_ui::tests::Platform(script));
    u.app.run("pref.accent", json!({"preset": preset}));
    u.ctx.enable_accesskit();
    u
}

/// Premultiplied `c` over opaque `bg`.
fn over(c: egui::Color32, bg: egui::Color32) -> egui::Color32 {
    let a = f32::from(c.a()) / 255.0;
    let mix = |f: u8, b: u8| (f32::from(f) + f32::from(b) * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgb(mix(c.r(), bg.r()), mix(c.g(), bg.g()), mix(c.b(), bg.b()))
}

/// The stroked rectangles, and those filled with the field colour.
fn rects(shapes: impl IntoIterator<Item = Shape>, field: egui::Color32, stroked: &mut Vec<egui::epaint::RectShape>, boxes: &mut Vec<egui::Rect>) {
    for s in shapes {
        match s {
            Shape::Vec(v) => rects(v, field, stroked, boxes),
            Shape::Rect(r) => {
                if r.stroke.width > 0.0 && r.stroke.color.a() > 0 {
                    stroked.push(r.clone());
                }
                if r.fill == field {
                    boxes.push(r.rect);
                }
            }
            _ => {}
        }
    }
}

/// One measured edge: the screen, the input's name, its edge colour and the lowest ratio.
#[derive(Debug)]
pub(crate) struct Edge {
    pub screen: String,
    pub name: String,
    pub color: Option<egui::Color32>,
    pub ratio: f64,
}

/// Every input on every screen, in `dark` with `preset`.
pub(crate) fn measure(dark: bool, preset: &str) -> Vec<Edge> {
    let mut found = Vec::new();
    for (screen, set) in screens() {
        let mut u = window(dark, preset);
        set(&mut u);
        u.app.handle_ui_requests();
        // Dialogs fade in: let them settle.
        for _ in 0..40 {
            u.frame(vec![]);
        }
        let out = u.frame_output(vec![]);
        let t = Tokens::get(&u.ctx);
        let (mut strokes, mut boxes) = (Vec::new(), Vec::new());
        rects(out.shapes.into_iter().map(|c| c.shape), t.field, &mut strokes, &mut boxes);
        let Some(update) = out.platform_output.accesskit_update else { continue };
        for (_, n) in update.nodes {
            if !INPUTS.contains(&n.role()) {
                continue;
            }
            let Some(b) = n.bounds() else { continue };
            let inner = egui::Rect::from_min_max(egui::pos2(b.x0 as f32, b.y0 as f32), egui::pos2(b.x1 as f32, b.y1 as f32));
            if inner.width() < 4.0 || inner.height() < 4.0 {
                continue;
            }
            // Its box: the smallest field-coloured rectangle around it (a framed TextEdit's too).
            let fits = |r: &egui::Rect| r.expand(2.0).contains_rect(inner.shrink(1.0));
            let field_box = boxes.iter().filter(|b| fits(b)).min_by(|a, b| a.area().total_cmp(&b.area()));
            // Its edge: the smallest stroked rectangle around it, on its box (or inside it); with no
            // box, at most a little larger than it.
            let edge = strokes
                .iter()
                .filter(|r| fits(&r.rect))
                .filter(|r| match field_box {
                    Some(b) => b.expand(1.5).contains_rect(r.rect),
                    None => r.rect.width() <= inner.width() + 40.0 && r.rect.height() <= inner.height() + 30.0,
                })
                .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
            let name = n.label().or(n.value()).unwrap_or("").to_string();
            let (color, ratio) = match edge {
                Some(r) => {
                    let c = r.stroke.color;
                    // A translucent edge is seen over what is under it.
                    let lowest = [t.field, t.panel, t.menu].into_iter().map(|bg| contrast(over(c, bg), bg)).fold(f64::INFINITY, f64::min);
                    (Some(c), lowest)
                }
                // No box at all: plain text clicked to rename (exempt, see above).
                None if field_box.is_none() => (None, f64::INFINITY),
                None => (None, 0.0),
            };
            found.push(Edge { screen: screen.to_string(), name, color, ratio });
        }
    }
    found
}

#[test]
fn every_input_edge_reads_at_3_to_1_in_both_themes_and_every_accent() {
    let mut low = Vec::new();
    let mut seen = 0;
    let mut lowest = [f64::INFINITY; 2];
    for dark in [false, true] {
        for (preset, _) in figcraft_engine::cmd::appearance::PRESETS {
            for e in measure(dark, preset) {
                seen += 1;
                if e.ratio.is_finite() {
                    lowest[usize::from(dark)] = lowest[usize::from(dark)].min(e.ratio);
                }
                if e.ratio < 3.0 {
                    low.push(format!(
                        "{} theme, {preset}: {} › {:?} edge {:?} at {:.2}:1",
                        if dark { "dark" } else { "light" },
                        e.screen,
                        e.name,
                        e.color,
                        e.ratio
                    ));
                }
            }
        }
    }
    assert!(seen > 100, "the screens show their inputs: {seen}");
    eprintln!("inputs measured: {seen}; lowest edge: light {:.2}:1, dark {:.2}:1", lowest[0], lowest[1]);
    low.sort();
    low.dedup();
    assert!(low.is_empty(), "{} input edges under 3:1:\n{}", low.len(), low.join("\n"));
}

#[test]
fn rest_hover_and_focus_edges_read_at_3_to_1_on_every_surface_in_every_theme_and_accent() {
    for dark in [false, true] {
        for (preset, [r, g, b]) in figcraft_engine::cmd::appearance::PRESETS {
            let base = if dark { Tokens::dark() } else { Tokens::light() };
            let t = base.with_accent(egui::Color32::from_rgb(r, g, b));
            for (state, hovered, focused) in [("rest", false, false), ("hover", true, false), ("focus", false, true)] {
                let edge = crate::theme::field_stroke(&t, hovered, focused).color;
                for (surface, bg) in [("field", t.field), ("panel", t.panel), ("menu", t.menu), ("pressed", t.pressed), ("canvas", t.canvas)] {
                    let c = contrast(edge, bg);
                    assert!(c >= 3.0, "{} {preset} {state} edge on the {surface}: {c:.2}:1", if dark { "dark" } else { "light" });
                }
            }
        }
    }
}
