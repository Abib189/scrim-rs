//! The Undone toast (Fix AB3, the design's A5): after Undo run, "Undone: 43 edits" with "Redo" at
//! its right, just above the status line. Raised fill, radius 10, 40 high; not a modal. It goes
//! after 6 s, on a new message, and once the run is no longer undone (Redo, from here or anywhere).
//! "Redo" runs `chat.redo-run`. While it shows it asks for one frame, at its end: never every frame.

use egui::{Align2, CornerRadius, Rect, Sense, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, trn};
use crate::panel_view::ai_tab::panel_theme::Roles;
use crate::theme::{regular, semibold};

/// How long it shows, and its height.
pub const SHOWS_FOR: f64 = 6.0;
const H: f32 = 40.0;

/// Show it for an Undo run of `edits` edits (its clock starts when it is first drawn).
pub fn show_for(app: &mut FigApp, edits: usize) {
    app.ui.agent_view.chat.toast = Some((edits, None));
}

/// The toast, if one is showing: drawn, its end asked for, and what `ui.inspect` reports.
pub fn draw(app: &mut FigApp, ui: &mut Ui, ro: &Roles) -> Option<Value> {
    let now = ui.ctx().input(|i| i.time);
    let undone = figcraft_engine::cmd::chat_run::state(&app.session).undone;
    let chat = &mut app.ui.agent_view.chat;
    let (edits, until) = match chat.toast {
        Some((edits, until)) if undone => (edits, until.unwrap_or(now + SHOWS_FOR)),
        _ => {
            chat.toast = None;
            return None;
        }
    };
    if now >= until {
        chat.toast = None;
        return None;
    }
    chat.toast = Some((edits, Some(until)));
    let w = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(w, H), Sense::hover());
    let r = Rect::from_min_max(pos2(row.min.x + 16.0, row.min.y), pos2(row.max.x - 16.0, row.max.y));
    let text = trn("Undone: {n} edits", edits, &[("n", &edits)]);
    let name = trn("Undone {n} edits. Redo.", edits, &[("n", &edits)]);
    let body = ui.interact(r, ui.id().with("ai_undone_toast"), Sense::hover());
    crate::a11y::status(&body, &name, "");
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(10), ro.chip_fill);
    p.text(pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, &text, regular(13.0), ro.text);
    let redo_w = p.layout_no_wrap(tr("Redo").to_string(), semibold(13.0), ro.link).size().x;
    let redo_r = Rect::from_min_max(pos2(r.max.x - 16.0 - redo_w - 8.0, r.min.y), r.max);
    p.text(pos2(r.max.x - 16.0, r.center().y), Align2::RIGHT_CENTER, tr("Redo"), semibold(13.0), ro.link);
    let redo = ui.interact(redo_r, ui.id().with("ai_undone_toast_redo"), Sense::click()).on_hover_text(tr("Redo run"));
    crate::a11y::button(&redo, tr("Redo"));
    if redo.has_focus() {
        p.rect_stroke(redo_r, CornerRadius::same(8), egui::Stroke::new(2.0, crate::theme::Tokens::get(ui.ctx()).focus), egui::StrokeKind::Inside);
    }
    if redo.clicked() {
        crate::panel_view::ai_tab::run_card::press(app, "chat.redo-run", &json!({}));
    }
    ui.add_space(8.0);
    // One frame at its end.
    ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64((until - now).max(0.0)));
    Some(
        json!({"text": text, "action": tr("Redo"), "name": name, "endsIn": until - now, "rect": [r.min.x, r.min.y, r.max.x, r.max.y], "redoRect": [redo_r.min.x, redo_r.min.y, redo_r.max.x, redo_r.max.y]}),
    )
}

#[cfg(test)]
#[path = "ai_toast_tests.rs"]
mod tests;
