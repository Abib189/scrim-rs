//! The home sidebar's Settings row (C3, addendum 1): pinned to the bottom of the sidebar below a
//! hairline, the same row height, icon size and hover as Recents, Online and Templates above, which
//! it never moves. It opens Settings (`settings.open`). The gear is drawn here, our own shape.

use egui::{Align2, CornerRadius, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::i18n::tr;
use crate::theme::{Tokens, regular};
use crate::{FigApp, a11y};

/// Draw the row at the bottom of `sidebar` (whose rows end at `rows_bottom`).
pub fn show(app: &mut FigApp, ui: &mut Ui, t: &Tokens, sidebar: Rect, rows_bottom: f32) {
    // At the bottom, but never over the rows above when the window is short.
    let top = (sidebar.max.y - 8.0 - 32.0).max(rows_bottom + 16.0);
    let r = Rect::from_min_size(pos2(sidebar.min.x + 8.0, top), vec2(sidebar.width() - 16.0, 32.0));
    let line_y = r.min.y - 8.0;
    ui.painter().line_segment([pos2(sidebar.min.x + 8.0, line_y), pos2(sidebar.max.x - 8.0, line_y)], Stroke::new(1.0, t.border));
    let resp = ui.interact(r, ui.id().with("home_settings"), Sense::click()).on_hover_text(tr("Settings (Cmd+,)"));
    a11y::button(&resp, tr("Settings"));
    if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(5), t.hover);
    }
    if resp.has_focus() {
        ui.painter().rect_stroke(r, CornerRadius::same(5), Stroke::new(2.0, t.focus), egui::StrokeKind::Outside);
    }
    gear(ui, Rect::from_min_size(pos2(r.min.x + 10.0, r.center().y - 8.0), vec2(16.0, 16.0)), t.icon);
    ui.painter().text(pos2(r.min.x + 36.0, r.center().y), Align2::LEFT_CENTER, tr("Settings"), regular(12.0), t.text);
    if resp.clicked() {
        app.run("settings.open", json!({}));
    }
}

/// A gear: a ring with eight square teeth, drawn as strokes in `rect`.
fn gear(ui: &Ui, rect: Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width() / 16.0;
    let stroke = Stroke::new(1.4 * s, color);
    ui.painter().circle_stroke(c, 4.6 * s, stroke);
    ui.painter().circle_stroke(c, 1.8 * s, stroke);
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        let (sin, cos) = a.sin_cos();
        let at = |r: f32| Pos2::new(c.x + cos * r * s, c.y + sin * r * s);
        ui.painter().line_segment([at(4.8), at(7.0)], stroke);
    }
}
