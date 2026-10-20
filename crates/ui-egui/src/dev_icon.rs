//! A frame's or section's handoff status (Stage E1, `docs/design/canvas-badges.md`).
//! - **On the canvas** it is a 14 px icon after the frame's title, the same size at every zoom: a
//!   ring with a check (Ready for dev), a ring half filled (Changed since it was marked) and a solid
//!   disc with the check cut out (Completed). One shape per state, so colour isn't the only cue.
//! - **Its colours** come from the theme set that reads on the page's own background
//!   ([`Ink::for_page`]), not the interface's theme: the canvas shows the page's colour, which is
//!   dark in either theme on a dark page.
//! - **The Dev Mode panel** keeps a worded pill ([`pill`]): it has room for the words.

use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use figcraft_doc::{DevStatus, Node};

use crate::i18n::tr;
use crate::theme::accent::contrast;
use crate::theme::{Tokens, semibold};

/// The icon's size on screen, at every zoom.
pub const ICON: f32 = 14.0;
/// The gap between a title and its icon.
pub const ICON_GAP: f32 = 4.0;

/// A handoff status as shown (Changed is Ready for dev with changes since it was marked).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Ready,
    Changed,
    Completed,
}

impl State {
    /// `n`'s status, `None` without one.
    pub fn of(n: &Node) -> Option<State> {
        match n.dev_status {
            DevStatus::None => None,
            DevStatus::ReadyForDev if n.dev_changed => Some(State::Changed),
            DevStatus::ReadyForDev => Some(State::Ready),
            DevStatus::Completed => Some(State::Completed),
        }
    }

    /// The status in words: the icon's tooltip and screen-reader name, the panel's pill.
    pub fn label(self) -> &'static str {
        match self {
            State::Ready => tr("Ready for dev"),
            State::Changed => tr("Changed"),
            State::Completed => tr("Completed"),
        }
    }

    /// The panel pill's fill (white text on it).
    fn fill(self) -> Color32 {
        match self {
            State::Ready => crate::theme::DEV_READY,
            State::Changed => crate::theme::DEV_CHANGED,
            State::Completed => crate::theme::DEV_COMPLETED,
        }
    }
}

/// The colours of canvas labels and status icons on a page: the theme set (light or dark) whose
/// secondary text reads better on the page's background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ink {
    /// The page's background, which the Completed check is cut out in.
    pub page: Color32,
    /// A title.
    pub text: Color32,
    /// A selected or hovered title.
    pub accent: Color32,
    /// A component's title and diamond.
    pub component: Color32,
    pub ready: Color32,
    pub changed: Color32,
    pub completed: Color32,
}

/// Component purple for text on a light page (5.3:1 on #ECECEA) and on a dark one (7.4:1 on
/// #17181A); the canvas outline keeps `theme::COMPONENT`, which is under 4.5:1 as text on both.
const COMPONENT_ON_LIGHT: Color32 = Color32::from_rgb(0x7B, 0x2F, 0xE0);
const COMPONENT_ON_DARK: Color32 = Color32::from_rgb(0xB7, 0x94, 0xFF);

impl Ink {
    pub fn for_page(page: Color32) -> Ink {
        let (light, dark) = (Tokens::light(), Tokens::dark());
        let on_dark = contrast(dark.text_secondary, page) > contrast(light.text_secondary, page);
        let (t, component) = if on_dark { (dark, COMPONENT_ON_DARK) } else { (light, COMPONENT_ON_LIGHT) };
        // The selected-title colour is the person's accent (Stage G), as text that reads on this page.
        Ink {
            page,
            text: t.text_secondary,
            accent: crate::accent_ink::text_on_page(page),
            component,
            ready: t.success,
            changed: t.warning,
            completed: t.text_secondary,
        }
    }

    /// The icon colour for `s`.
    pub fn of(&self, s: State) -> Color32 {
        match s {
            State::Ready => self.ready,
            State::Changed => self.changed,
            State::Completed => self.completed,
        }
    }
}

/// Draw `s`'s icon in `rect` (a 14 px square).
pub fn paint(painter: &Painter, rect: Rect, s: State, ink: &Ink) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0 - 1.0;
    let col = ink.of(s);
    let ring = Stroke::new(1.5, col);
    let check = |painter: &Painter, stroke: Stroke| {
        let k = r / 6.0;
        let pts = vec![c + vec2(-3.0, 0.2) * k, c + vec2(-0.9, 2.3) * k, c + vec2(3.1, -2.1) * k];
        painter.add(Shape::line(pts, stroke));
    };
    match s {
        State::Ready => {
            painter.circle_stroke(c, r - 0.75, ring);
            check(painter, Stroke::new(1.5, col));
        }
        State::Changed => {
            painter.circle_stroke(c, r - 0.75, ring);
            painter.add(Shape::convex_polygon(half_disc(c, r - 1.5), col, Stroke::NONE));
        }
        State::Completed => {
            painter.circle_filled(c, r, col);
            check(painter, Stroke::new(1.6, ink.page));
        }
    }
}

/// The left half of a disc (the Changed icon's fill).
fn half_disc(c: Pos2, r: f32) -> Vec<Pos2> {
    let n = 12;
    (0..=n)
        .map(|i| {
            let a = std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / n as f32;
            pos2(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

/// The icon's tooltip (while the pointer rests on it over the canvas) and its screen-reader node
/// (named by its status, at its place on screen), keyed by its frame `key`.
pub fn annotate(painter: &Painter, key: figcraft_doc::NodeId, rect: Rect, s: State) {
    let ctx = painter.ctx();
    let id = egui::Id::new(("dev_status_icon", key));
    ctx.accesskit_node_builder(id, |n| {
        n.set_role(egui::accesskit::Role::Image);
        n.set_label(s.label());
        n.set_bounds(egui::accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() });
    });
    let hover = ctx.pointer_hover_pos().filter(|p| rect.contains(*p) && ctx.layer_id_at(*p) == Some(painter.layer_id()));
    if hover.is_some() && !ctx.input(|i| i.pointer.any_down()) {
        egui::Tooltip::always_open(ctx.clone(), painter.layer_id(), id, egui::PopupAnchor::Pointer).show(|ui| ui.label(s.label()));
    }
}

/// The Dev Mode panel's worded status pill, left-centred on `at`; its area.
pub fn pill(painter: &Painter, n: &Node, at: Pos2) -> Option<Rect> {
    let s = State::of(n)?;
    let galley = painter.layout_no_wrap(s.label().to_string(), semibold(10.0), Color32::WHITE);
    let r = Rect::from_min_size(pos2(at.x, at.y - galley.size().y / 2.0 - 2.0), galley.size() + vec2(10.0, 4.0));
    painter.rect_filled(r, CornerRadius::same(4), s.fill());
    painter.galley(r.min + vec2(5.0, 2.0), galley, Color32::WHITE);
    Some(r)
}

#[cfg(test)]
#[path = "dev_icon_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "dev_icon_ui_tests.rs"]
mod ui_tests;
