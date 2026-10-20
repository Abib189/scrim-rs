//! The Layers header (D1, `docs/design/layers-collapse-and-focus.md`): the title, **Collapse all
//! layers** and the search button, then the search row when it is open. Moved out of `layers.rs`,
//! which is at its size cap.
//!
//! Collapse all (`layers.collapse-all`, ⌥L) closes every row on every page. It is panel state only:
//! the selection, the document and its revision are untouched, and there is no undo step. Its next
//! press restores the rows it closed while nothing has changed (Stage H, `layers_restore`): the
//! button is then shown pressed, with an open tree for its icon, named "Restore expanded layers".

use egui::{Align2, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

use crate::FigApp;
use crate::a11y::Tip;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};

/// The search field's id (focused when the search opens).
fn search_id() -> Id {
    Id::new("layer_search_field")
}

/// The header row, then the search row when it is open.
pub(crate) fn show(app: &mut FigApp, ui: &mut Ui, t: &Tokens, w: f32) {
    crate::layers_restore::scroll(app, ui, false);
    let (hr, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    ui.painter().text(pos2(hr.min.x + 16.0, hr.center().y), Align2::LEFT_CENTER, tr("Layers"), semibold(11.0), t.text);
    // Collapse all layers, left of the search button; pressed while its next press restores.
    let cr = Rect::from_center_size(pos2(hr.max.x - 50.0, hr.center().y), vec2(24.0, 24.0));
    let restore = crate::layers_restore::restores_next(app);
    let tip = if restore { tr("Restore expanded layers") } else { crate::shortcuts::tip(tr("Collapse all layers"), "layers.collapse-all") };
    let cresp = ui.interact(cr, ui.id().with("layer_collapse_all_btn"), Sense::click());
    crate::a11y::toggle(&cresp, tip, restore);
    let cresp = cresp.on_hover_ui(|ui| {
        ui.label(tip);
    });
    if restore || cresp.hovered() {
        ui.painter().rect_filled(cr, CornerRadius::same(5), if restore { t.pressed } else { t.hover });
    }
    collapse_icon(ui, cr.shrink(5.0), t.icon, restore);
    if cresp.has_focus() {
        ui.painter().rect_stroke(cr, CornerRadius::same(5), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    if cresp.clicked() {
        app.run(if restore { "layers.restore-expansion" } else { "layers.collapse-all" }, json!({}));
    }
    let sr = Rect::from_center_size(pos2(hr.max.x - 22.0, hr.center().y), vec2(24.0, 24.0));
    let sresp = ui.interact(sr, ui.id().with("layer_search_btn"), Sense::click()).tip(tr("Search layers"));
    if sresp.hovered() || app.ui.layer_search {
        ui.painter().rect_filled(sr, CornerRadius::same(5), t.hover);
    }
    icons::paint(ui.painter(), sr.shrink(5.0), Icon::Search, t.icon);
    if sresp.clicked() {
        app.ui.layer_search = !app.ui.layer_search;
        app.ui.layer_filter.clear();
        if app.ui.layer_search {
            ui.memory_mut(|m| m.request_focus(search_id()));
        }
    }
    if app.ui.layer_search {
        search_field(app, ui, t, w, search_id());
    }
    // A restore (this button's, or a command's) scrolls the list back before it draws.
    crate::layers_restore::scroll(app, ui, true);
}

/// Collapse all's icon (our own drawing; `icons.rs` is at its cap): three lines, the last one
/// short; or, when its next press restores, an open tree (the second and third lines indented).
fn collapse_icon(ui: &Ui, r: Rect, color: egui::Color32, restore: bool) {
    let stroke = Stroke::new(1.3, color);
    let (x0, w) = (r.min.x + r.width() * 0.12, r.width() * 0.76);
    let lines = if restore { [(0.0, 1.0), (0.3, 1.0), (0.3, 1.0)] } else { [(0.0, 1.0), (0.0, 1.0), (0.0, 0.55)] };
    for (i, (from, to)) in lines.iter().enumerate() {
        let y = r.min.y + r.height() * (0.25 + 0.25 * i as f32);
        ui.painter().line_segment([pos2(x0 + w * from, y), pos2(x0 + w * to, y)], stroke);
    }
}

/// The search row under the Layers header (live filter; Escape closes it).
fn search_field(app: &mut FigApp, ui: &mut Ui, t: &Tokens, w: f32, id: Id) {
    let (fr, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
    let field = Rect::from_min_max(pos2(fr.min.x + 12.0, fr.min.y + 2.0), pos2(fr.max.x - 12.0, fr.max.y - 6.0));
    ui.painter().rect_filled(field, CornerRadius::same(5), t.field);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(field.min.x + 12.0, field.center().y), vec2(12.0, 12.0)), Icon::Search, t.icon_secondary);
    let te_rect = Rect::from_min_max(pos2(field.min.x + 24.0, field.center().y - 7.0), pos2(field.max.x - 6.0, field.center().y + 7.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(te_rect));
    let r = child.add(
        egui::TextEdit::singleline(&mut app.ui.layer_filter)
            .id(id)
            .frame(egui::Frame::NONE)
            .font(regular(11.0))
            .hint_text(tr("Find layers"))
            .vertical_align(egui::Align::Center)
            .desired_width(te_rect.width()),
    );
    crate::widgets::select_all_on_focus(ui, &r, &app.ui.layer_filter);
    if r.has_focus() {
        ui.painter().rect_stroke(field, CornerRadius::same(5), Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    }
    if (r.has_focus() || r.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ui.layer_search = false;
        app.ui.layer_filter.clear();
    }
}

/// D1's collapse (`layers_restore::collapse` keeps a snapshot first): every row closes, on every
/// page. The selection isn't changed, and isn't revealed again until it changes (so the collapse
/// sticks).
pub(crate) fn collapse_all(app: &mut FigApp) {
    app.ui.expanded.clear();
    app.ui.layers_seen_sel = app.session.selection();
}

#[cfg(test)]
#[path = "layers_header_tests.rs"]
pub(crate) mod tests;
