//! Suggestions (Fix AB5, the design's A1, A3 and A7), only where they apply:
//! - **With a selection,** on the quiet panel (no conversation yet, no run going): "For the selected
//!   {name}" (12, text-3) and two full-width rows (surface fill, border, radius 10): "Tidy the
//!   selected layers", "Align everything to the grid", just above the status line.
//! - **After a run finishes** (done, not undone or stopped): two chips under its card, "Tidy
//!   spacing" and "Make a variation".
//!
//! A suggestion puts its words in the composer and focuses it (`chat.use-suggestion`); it never
//! sends. None with no selection and no finished run.

use egui::{Align2, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::panel_view::ai_tab::panel_theme::Roles;
use crate::theme::regular;

/// The selection suggestions' rows, and the run chips' height.
const ROW: f32 = 36.0;
const CHIP: f32 = 30.0;

/// The selection's suggestions, when they apply. Returns the text picked, if one was, and what
/// `ui.inspect` reports.
pub fn for_selection(app: &FigApp, ui: &mut Ui, ro: &Roles) -> (Option<String>, Value) {
    let quiet = app.ui.agent_view.chat.lines.is_empty() && app.ui.agent_view.chat.running.is_none();
    let Some(name) = crate::panel_view::ai_tab::plus::selection_name(app).filter(|_| quiet) else { return (None, Value::Null) };
    let label = trf("For the selected {name}", &[("name", &name)]);
    let w = ui.available_width();
    let (lr, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
    // (The composer's area is already 16 inside the panel.)
    let shown = crate::panel_view::ai_tab::run_card::elide(ui.painter(), &label, regular(12.0), w);
    ui.painter().text(pos2(lr.min.x, lr.center().y), Align2::LEFT_CENTER, &shown, regular(12.0), ro.text_3);
    let mut picked = None;
    let mut rows = Vec::new();
    for (i, text) in [tr("Tidy the selected layers"), tr("Align everything to the grid")].into_iter().enumerate() {
        ui.add_space(6.0);
        let (r, _) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
        let resp = ui.interact(r, Id::new(("ai_suggest_selection", i)), Sense::click()).on_hover_text(tr("Put this in the prompt"));
        crate::a11y::button(&resp, text);
        let fill = if resp.hovered() { ro.chip_fill } else { ro.composer_fill };
        ui.painter().rect_filled(r, CornerRadius::same(10), fill);
        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, ro.card_edge), egui::StrokeKind::Inside);
        let shown = crate::panel_view::ai_tab::run_card::elide(ui.painter(), text, regular(13.0), r.width() - 24.0);
        ui.painter().text(pos2(r.min.x + 12.0, r.center().y), Align2::LEFT_CENTER, shown, regular(13.0), ro.text);
        if resp.clicked() {
            picked = Some(text.to_string());
        }
        rows.push(json!({"text": text, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]}));
    }
    ui.add_space(10.0);
    (picked, json!({"label": label, "labelRect": [lr.min.x, lr.min.y, lr.max.x, lr.max.y], "rows": rows}))
}

/// The chips under a finished run's card, 8 apart, wrapping to a second row when the panel is
/// narrow. Returns the text picked, if one was, and what `ui.inspect` reports.
pub fn after_run(ui: &mut Ui, ro: &Roles) -> (Option<String>, Value) {
    ui.add_space(10.0);
    let texts = [tr("Tidy spacing"), tr("Make a variation")];
    let widths: Vec<f32> = texts.iter().map(|t| ui.painter().layout_no_wrap(t.to_string(), regular(13.0), ro.text).size().x + 24.0).collect();
    let w = ui.available_width();
    let (left, right) = (16.0, w - 16.0);
    // Where each goes: along the row, onto the next when it wouldn't fit.
    let (mut x, mut y, mut spots) = (left, 0.0, Vec::new());
    for cw in &widths {
        if x > left && x + cw > right {
            x = left;
            y += CHIP + 8.0;
        }
        spots.push((x, y));
        x += cw + 8.0;
    }
    let (area, _) = ui.allocate_exact_size(vec2(w, y + CHIP), Sense::hover());
    let mut picked = None;
    let mut chips = Vec::new();
    for ((text, cw), (x, y)) in texts.iter().zip(&widths).zip(spots) {
        let r = Rect::from_min_size(pos2(area.min.x + x, area.min.y + y), vec2(*cw, CHIP));
        let resp = ui.interact(r, Id::new(("ai_suggest_after_run", *text)), Sense::click()).on_hover_text(tr("Put this in the prompt"));
        crate::a11y::button(&resp, text);
        ui.painter().rect_filled(r, CornerRadius::same((CHIP / 2.0) as u8), if resp.hovered() { ro.chip_hover } else { ro.chip_fill });
        ui.painter().text(r.center(), Align2::CENTER_CENTER, *text, regular(13.0), ro.text);
        if resp.clicked() {
            picked = Some(text.to_string());
        }
        chips.push(json!({"text": text, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]}));
    }
    (picked, json!({"chips": chips}))
}

#[cfg(test)]
#[path = "ai_suggest_tests.rs"]
mod tests;
