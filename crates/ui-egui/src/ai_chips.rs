//! The composer's controls (Fix X3, `docs/design/ai-panel.md`): the agent's pill chip, the round
//! icon chips for the selection and an image (Fix Z6), and the round Send / Stop button. Their colours are the panel's roles
//! (`ai_panel_theme.rs`); their keys and accessible names are the composer's, unchanged.

use egui::{CornerRadius, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::panel_view::ai_tab::icons::{self, AiIcon};
use crate::panel_view::ai_tab::panel_theme::Roles;
use crate::theme::{Tokens, regular};

/// A chip's height and corner radius (a pill), and its padding.
pub const CHIP_H: f32 = 32.0;
const CHIP_PAD: f32 = 12.0;
/// The agent chip's chevron: 8 × 4, 12 after the label.
const CHEVRON_W: f32 = 8.0;
const CHEVRON_GAP: f32 = 12.0;
/// Send and Stop: a 36 px circle.
pub const SEND: f32 = 36.0;

/// A pill chip's padding either side and whether it has a chevron: 12 and a chevron, or below
/// [`NARROW_BELOW`] 8 and none, so the agent's name keeps its room (Fix AA3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChipFit {
    pub pad: f32,
    pub chevron: bool,
}

/// The panel width under which the agent chip is narrow (Fix AA3).
pub const NARROW_BELOW: f32 = 280.0;

impl ChipFit {
    pub fn for_panel(width: f32, menu: bool) -> Self {
        if width < NARROW_BELOW { Self { pad: 8.0, chevron: false } } else { Self { pad: CHIP_PAD, chevron: menu } }
    }

    /// The room a chip `width` wide leaves its label.
    pub fn room(self, width: f32) -> f32 {
        (width - self.pad * 2.0 - if self.chevron { CHEVRON_GAP + CHEVRON_W } else { 0.0 }).max(8.0)
    }

    /// The width a chip with `label` wants.
    pub fn width(self, ui: &Ui, label: &str) -> f32 {
        label_width(ui, label) + self.pad * 2.0 + if self.chevron { CHEVRON_GAP + CHEVRON_W } else { 0.0 }
    }
}

/// `label`'s width in the chips' font.
pub fn label_width(ui: &Ui, label: &str) -> f32 {
    ui.painter().layout_no_wrap(label.to_string(), regular(13.0), egui::Color32::PLACEHOLDER).size().x
}

/// The least room a name needs to read (Fix AA3): all of it, or its first six characters and "…".
pub fn readable_width(ui: &Ui, label: &str) -> f32 {
    let short: String = label.chars().take(6).collect();
    if short.chars().count() == label.chars().count() { label_width(ui, label) } else { label_width(ui, &format!("{}…", short.trim_end())) }
}

/// A pill chip `width` wide (its label shortened with "…" when it doesn't fit): `label` in
/// `color`, a chevron when `fit` has one. `name` is what screen readers say and the tooltip.
/// Returns the label as shown.
pub fn chip(ui: &mut Ui, roles: &Roles, label: &str, color: egui::Color32, fit: ChipFit, width: f32, name: &str) -> (Response, String) {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(width, CHIP_H), Sense::click());
    let resp = resp.on_hover_text(name);
    ui.painter().rect_filled(r, CornerRadius::same((CHIP_H / 2.0) as u8), roles.chip_fill);
    if resp.has_focus() {
        ui.painter().rect_stroke(r, CornerRadius::same((CHIP_H / 2.0) as u8), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    let shown = crate::panel_view::ai_tab::run_card::elide(ui.painter(), label, regular(13.0), fit.room(width));
    let galley = ui.painter().layout_no_wrap(shown.clone(), regular(13.0), color);
    let chevron = fit.chevron;
    let text_x = r.min.x + fit.pad;
    ui.painter().galley(pos2(text_x, r.center().y - galley.size().y / 2.0), galley.clone(), color);
    if chevron {
        let cx = text_x + galley.size().x + CHEVRON_GAP + CHEVRON_W / 2.0;
        crate::panel_view::ai_tab::icons::chevron(ui.painter(), pos2(cx, r.center().y), CHEVRON_W, 4.0, 1.5, roles.chevron);
    }
    (resp, shown)
}

/// A round 32 px chip with `icon` alone (Fix Z6: Selection and the image, at every width, so the
/// agent's name keeps the room): `t.selected_row`, `t.pressed` hovered, the focus ring. `name` is
/// what screen readers say and the tooltip.
pub fn icon_chip(ui: &mut Ui, roles: &Roles, icon: AiIcon, name: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(CHIP_H, CHIP_H), Sense::click());
    let resp = resp.on_hover_text(name);
    ui.painter().circle_filled(r.center(), CHIP_H / 2.0, if resp.hovered() { roles.chip_hover } else { roles.chip_fill });
    if resp.has_focus() {
        ui.painter().circle_stroke(r.center(), CHIP_H / 2.0 - 1.0, Stroke::new(2.0, t.focus));
    }
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(16.0, 16.0)), icon, roles.chip_icon);
    resp
}

/// Send (an arrow, 12 × 14 at 2) or, while a run goes, Stop (a 16 × 16 square, radius 3): a 36 px
/// circle in the accent's fill, the glyph in the colour that reads on it. `tip` names it.
pub fn send(ui: &mut Ui, roles: &Roles, stop: bool, tip: &str) -> Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(SEND, SEND), Sense::click());
    let resp = resp.on_hover_text(tip);
    crate::a11y::button(&resp, tip);
    let c = r.center();
    ui.painter().circle_filled(c, SEND / 2.0, roles.send_fill);
    if resp.has_focus() {
        ui.painter().circle_stroke(c, SEND / 2.0 + 1.0, Stroke::new(2.0, t.focus));
    }
    if stop {
        ui.painter().rect_filled(Rect::from_center_size(c, vec2(16.0, 16.0)), CornerRadius::same(3), roles.send_glyph);
    } else {
        let line = Stroke::new(2.0, roles.send_glyph);
        ui.painter().line_segment([c + vec2(0.0, 7.0), c + vec2(0.0, -7.0)], line);
        ui.painter().add(egui::Shape::line(vec![c + vec2(-6.0, -1.0), c + vec2(0.0, -7.0), c + vec2(6.0, -1.0)], line));
    }
    resp
}
