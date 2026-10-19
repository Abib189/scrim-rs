//! Preferences ▸ Accent (`docs/design/panel-redesign.md`, B4): Scrim's six presets, a custom hex
//! colour and Reset, each running `pref.accent` (agents set it the same way). A sample shows the
//! accent as a primary button, with the contrast of its label, which the derivation picks for
//! 4.5:1 or better. The theme row above it runs `view.theme*`; `pref.theme` does the same.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use figcraft_engine::cmd::appearance::{self, PRESETS};
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, tr_noop, trf};
use crate::theme::accent::contrast;
use crate::theme::{Tokens, semibold};
use crate::{a11y, widgets};

/// The presets' names, for people and screen readers.
const NAMES: [&str; 6] = [tr_noop("Orange"), tr_noop("Blue"), tr_noop("Violet"), tr_noop("Green"), tr_noop("Pink"), tr_noop("Teal")];

/// The Accent row of Preferences.
pub fn row(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let current = appearance::accent(&app.session.prefs);
    ui.horizontal(|ui| {
        ui.label(tr("Accent"));
        for ((id, c), name) in PRESETS.iter().zip(NAMES) {
            let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
            let on = *c == current;
            let resp = resp.on_hover_text(tr(name));
            a11y::toggle(&resp, &trf("{name} accent", &[("name", &tr(name))]), on);
            ui.painter().circle_filled(r.center(), 9.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            if on || resp.hovered() {
                ui.painter().circle_stroke(r.center(), 11.0, Stroke::new(if on { 2.0 } else { 1.0 }, t.text));
            }
            if resp.clicked() && !on {
                app.run("pref.accent", json!({"preset": id}));
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Custom"));
        let shown = appearance::hex(current);
        if let Some(v) = widgets::text_field(ui, egui::Id::new("pref_accent_hex"), &shown, 90.0, tr("Accent colour (hex)"))
            && v.trim() != shown
        {
            app.run("pref.accent", json!({"value": v.trim()}));
        }
        let is_default = current == appearance::DEFAULT_ACCENT;
        if widgets::button(ui, tr("Reset"), false, !is_default).on_hover_text(tr("Back to Scrim's orange, #ff5a1f")).clicked() {
            app.run("pref.accent", json!({}));
        }
    });
    sample(ui, &t);
}

/// The accent as a primary button, with its label's contrast (a preview: it changes nothing).
fn sample(ui: &mut Ui, t: &Tokens) {
    let ratio = contrast(t.accent_on, t.accent_fill);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(84.0, 24.0), Sense::hover());
        ui.painter().rect_filled(r, CornerRadius::same(5), t.accent_fill);
        ui.painter().text(r.center(), Align2::CENTER_CENTER, tr("Sample"), semibold(11.0), t.accent_on);
        let mark = Rect::from_min_size(pos2(r.max.x + 8.0, r.center().y - 9.0), vec2(18.0, 18.0));
        ui.allocate_rect(mark.expand2(vec2(0.0, 3.0)), Sense::hover());
        ui.painter().rect_stroke(mark, CornerRadius::same(4), Stroke::new(2.0, t.accent), StrokeKind::Inside);
        widgets::label(ui, &trf("Text on the accent: {ratio}:1", &[("ratio", &format!("{ratio:.1}"))]), t.text_secondary);
    });
}

/// Act on an appearance request from a command (`appearance`): keep the preferences. The theme
/// and its accent are applied again on the next frame (`Tokens::for_prefs`).
pub fn request(app: &mut FigApp, kind: &str, _: &Value) -> bool {
    if kind != appearance::REQUEST {
        return false;
    }
    app.services.save_prefs(&app.session.prefs);
    true
}

#[cfg(test)]
#[path = "appearance_prefs_tests.rs"]
mod tests;
