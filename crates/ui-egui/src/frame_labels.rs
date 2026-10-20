//! Name labels of top-level frames and sections on the canvas. The name sits just above the top
//! edge, outside the frame or section, followed by its handoff status icon (`dev_icon`, Stage E1);
//! on a rotated frame or section the label runs along the top edge as shown, turned with it, and
//! is cut to the edge's length. Each label's area (the status icon included) goes to the engine
//! (`ToolState::labels`), so pressing a label selects and drags its frame like pressing the frame
//! itself.

use egui::layers::ShapeIdx;
use egui::{Align2, Color32, Painter, Pos2, Rect, Shape, Vec2, emath::Rot2, pos2, vec2};
use figcraft_doc::{Document, NodeId, NodeKind};
use figcraft_geom::Point;

use crate::FigApp;
use crate::dev_icon::{self, ICON, ICON_GAP, Ink, State};
use crate::i18n::tr;
use crate::icons::Icon;
use crate::theme::{ACCENT, COMPONENT, Tokens, regular};

/// Where a label goes on screen: the corner it starts at, the angle it reads along (radians,
/// clockwise on screen) and the length of the frame's edge along that direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelEdge {
    pub corner: Pos2,
    pub angle: f32,
    pub len: f32,
}

/// The label edge of a frame whose local corners (0, 0), (w, 0) and (0, h) show at `p00`, `p10`
/// and `p01`: its top edge as seen, read along the frame's rotation. A mirrored frame reads left
/// to right as far as its rotation allows (a flipped frame's label isn't upside down), and the
/// label always sits on the side away from the frame.
pub fn label_edge(p00: Pos2, p10: Pos2, p01: Pos2) -> LabelEdge {
    let (ex, ey) = (p10 - p00, p01 - p00);
    let len = ex.length();
    if !len.is_finite() || len <= 1e-3 {
        return LabelEdge { corner: p00, angle: 0.0, len: 0.0 };
    }
    let mirrored = ex.x * ey.y - ex.y * ey.x < 0.0;
    // Read along local -x (starting at x = w) when that is the upright way for a mirrored frame.
    let reversed = mirrored && ex.x < 0.0;
    let d = if reversed { -ex } else { ex };
    // "Up" for the text; the label goes on the edge whose outside that is.
    let up = vec2(d.y, -d.x);
    let bottom = up.dot(ey) > 0.0;
    let corner = p00 + if reversed { ex } else { Vec2::ZERO } + if bottom { ey } else { Vec2::ZERO };
    LabelEdge { corner, angle: d.y.atan2(d.x), len }
}

/// Turn every shape `painter` added since `from` by `angle` (radians, clockwise) about `pivot`.
pub fn turn_since(painter: &Painter, from: ShapeIdx, pivot: Pos2, angle: f32) {
    if angle == 0.0 || !angle.is_finite() {
        return;
    }
    let rot = Rot2::from_angle(angle);
    painter.ctx().graphics_mut(|g| {
        let list = g.entry(painter.layer_id());
        for i in from.0..list.next_idx().0 {
            list.mutate_shape(ShapeIdx(i), |c| turn(&mut c.shape, rot, angle, pivot));
        }
    });
}

fn turn(shape: &mut Shape, rot: Rot2, angle: f32, pivot: Pos2) {
    let at = |p: &mut Pos2| *p = pivot + rot * (*p - pivot);
    match shape {
        Shape::Noop | Shape::Callback(_) => {}
        Shape::Vec(v) => v.iter_mut().for_each(|s| turn(s, rot, angle, pivot)),
        Shape::Circle(c) => at(&mut c.center),
        Shape::Ellipse(e) => {
            at(&mut e.center);
            e.angle += angle;
        }
        Shape::LineSegment { points, .. } => points.iter_mut().for_each(at),
        Shape::Path(p) => p.points.iter_mut().for_each(at),
        Shape::Rect(r) => {
            // Rectangles turn about their centre.
            let mut c = r.rect.center();
            at(&mut c);
            r.rect = Rect::from_center_size(c, r.rect.size());
            r.angle += angle;
        }
        Shape::Text(t) => {
            at(&mut t.pos);
            t.angle += angle;
        }
        Shape::Mesh(m) => std::sync::Arc::make_mut(m).rotate(rot, pivot),
        Shape::QuadraticBezier(q) => q.points.iter_mut().for_each(at),
        Shape::CubicBezier(c) => c.points.iter_mut().for_each(at),
    }
}

/// A title is never cut narrower than this (a shorter one shows whole).
pub const MIN_TITLE: f32 = 20.0;
/// A lead icon (the component diamond, a hidden section's eye) and the space after it.
const LEAD: f32 = 14.0;

/// How a title of natural width `title` and a status icon (`icon`) share `room` (screen px after
/// any lead icon): the title's width and whether the icon shows. The title is cut to end before the
/// icon; when both can't fit the icon goes first, then the whole label (`None`).
pub fn fit(room: f32, title: f32, icon: bool) -> Option<(f32, bool)> {
    if !(room.is_finite() && title.is_finite()) || room <= 0.0 {
        return None;
    }
    let least = MIN_TITLE.min(title);
    let beside = room - ICON_GAP - ICON;
    if icon && beside >= least {
        return Some((title.min(beside), true));
    }
    (room >= least).then_some((title.min(room), false))
}

/// The colour of the page `id` is on (what the canvas shows behind its label).
pub fn page_colour(painter: &Painter, t: &Tokens, doc: &Document, id: NodeId) -> Color32 {
    let page = doc.page_of(id).and_then(|p| doc.node(p));
    match page.map(|p| &p.kind) {
        Some(NodeKind::Page { background, .. }) => crate::theme::doc_c32(painter.ctx(), *background),
        _ => t.canvas,
    }
}

/// Draw the label of `id` (a top-level frame or a section): its name just above its top edge
/// (turned with it), after the component diamond or a hidden section's eye, then its handoff
/// status icon. The area it covers on screen (its corners) for hit testing; `None` when its frame
/// is too narrow on screen to hold it.
pub fn draw(app: &FigApp, painter: &Painter, t: &Tokens, doc: &Document, id: NodeId, selected: bool, hovered: bool) -> Option<[Pos2; 4]> {
    let n = doc.node(id)?;
    let w = doc.world_transform(id);
    let at = |x: f64, y: f64| {
        let p = w * Point::new(x, y);
        app.to_screen(p.x, p.y)
    };
    let (p00, p01) = (at(0.0, 0.0), at(0.0, n.height));
    let edge = label_edge(p00, at(n.width, 0.0), p01);
    let tl = edge.corner;
    let is_comp = matches!(n.kind, NodeKind::Component { .. } | NodeKind::ComponentSet { .. });
    let col = if is_comp {
        COMPONENT
    } else if selected || hovered {
        ACCENT
    } else {
        t.text_secondary
    };
    if n.hides_contents() {
        let height = (p01 - p00).length();
        if edge.len > 120.0 && height > 60.0 {
            let center = at(n.width / 2.0, n.height / 2.0);
            painter.text(center, Align2::CENTER_CENTER, tr("Contents hidden"), regular(11.0), t.text_tertiary);
        }
    }
    let lead_icon = if is_comp {
        Some(Icon::Component)
    } else if matches!(n.kind, NodeKind::Section { .. }) && n.hides_contents() {
        Some(Icon::EyeOff)
    } else {
        None
    };
    let lead = if lead_icon.is_some() { LEAD } else { 0.0 };
    let state = State::of(n);
    let font = regular(11.0);
    let natural = crate::bidi::name_galley(painter, &n.name, font.clone(), col, f32::MAX).size().x;
    let (title_w, show_icon) = fit(edge.len - lead, natural, state.is_some())?;
    // Drawn upright at the corner, then turned with the frame.
    let from = painter.add(Shape::Noop);
    let top = tl.y - 16.0;
    if let Some(icon) = lead_icon {
        crate::icons::paint(painter, Rect::from_min_size(pos2(tl.x, top), vec2(12.0, 12.0)), icon, col);
    }
    // Names longer than their room end in an ellipsis (a right-to-left name keeps its start).
    let galley = crate::bidi::name_galley(painter, &n.name, font, col, title_w.max(1.0));
    let size = galley.size();
    let mut end = tl.x + lead + size.x;
    painter.galley(pos2(tl.x + lead, top), galley, col);
    let mut bottom = top + size.y.max(12.0);
    let icon = state.filter(|_| show_icon).map(|s| {
        let r = Rect::from_min_size(pos2(end + ICON_GAP, tl.y - 2.0 - ICON), vec2(ICON, ICON));
        dev_icon::paint(painter, r, s, &Ink::for_page(page_colour(painter, t, doc, id)));
        end = r.max.x;
        bottom = bottom.max(r.max.y);
        (r, s)
    });
    let area = Rect::from_min_max(pos2(tl.x, tl.y - 18.0), pos2(end, bottom));
    turn_since(painter, from, tl, edge.angle);
    let rot = Rot2::from_angle(edge.angle);
    let turned = |r: Rect| [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(|p| tl + rot * (p - tl));
    if let Some((r, s)) = icon {
        dev_icon::annotate(painter, id, Rect::from_points(&turned(r)), s);
    }
    Some(turned(area))
}

/// The field over a frame's or section's name label being renamed (`UiState::rename_label`, from a
/// double-click on the label): Enter or a click elsewhere renames the layer, Escape keeps the name.
/// The rename ends when the field lets go of the keyboard or the label is gone.
pub fn rename_field(app: &mut FigApp, ui: &mut egui::Ui) {
    let Some(r) = app.ui.rename_label else { return };
    let quad = app.session.tool.labels.iter().find(|(i, _)| *i == r.id).map(|(_, q)| q.map(|p| app.to_screen(p.x, p.y)));
    let name = app.session.doc().ok().and_then(|d| d.node(r.id)).map(|n| n.name.clone());
    let (Some(quad), Some(name)) = (quad, name) else {
        app.ui.rename_label = None;
        return;
    };
    let area = Rect::from_points(&quad);
    let font = regular(11.0);
    let text_w = ui.painter().layout_no_wrap(name.clone(), font.clone(), Color32::WHITE).size().x;
    let rect = Rect::from_min_size(pos2(area.min.x, area.max.y - 16.0), vec2((text_w + 24.0).max(area.width()).clamp(80.0, 480.0), 16.0));
    let (done, open) = crate::widgets::rename_field(ui, egui::Id::new(("frame_label_rename", r.id)), &name, rect, font, r.start);
    app.ui.rename_label = open.then_some(crate::layers::Rename { id: r.id, start: false });
    if let Some(new) = done {
        app.run("object.rename", serde_json::json!({"ids": [r.id], "name": new}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Pos2, b: Pos2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn label_edges_follow_rotation_and_stay_upright_when_mirrored() {
        // Upright: the top-left corner, reading right.
        let e = label_edge(pos2(10.0, 20.0), pos2(110.0, 20.0), pos2(10.0, 70.0));
        assert!(close(e.corner, pos2(10.0, 20.0)) && e.angle == 0.0 && (e.len - 100.0).abs() < 1e-3);
        // Turned 90° clockwise on screen: the top edge runs down the right side, read downwards.
        let e = label_edge(pos2(100.0, 0.0), pos2(100.0, 80.0), pos2(50.0, 0.0));
        assert!(close(e.corner, pos2(100.0, 0.0)));
        assert!((e.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        // Turned 180°: upside down along the bottom edge as seen (rotated with the frame).
        let e = label_edge(pos2(100.0, 50.0), pos2(0.0, 50.0), pos2(100.0, 0.0));
        assert!(close(e.corner, pos2(100.0, 50.0)));
        assert!((e.angle.abs() - std::f32::consts::PI).abs() < 1e-5);
        // Flipped horizontally: still the visual top-left, reading right.
        let e = label_edge(pos2(100.0, 0.0), pos2(0.0, 0.0), pos2(100.0, 50.0));
        assert!(close(e.corner, pos2(0.0, 0.0)) && e.angle.abs() < 1e-6, "{e:?}");
        // Flipped vertically: the local bottom edge is the top as seen.
        let e = label_edge(pos2(0.0, 50.0), pos2(100.0, 50.0), pos2(0.0, 0.0));
        assert!(close(e.corner, pos2(0.0, 0.0)) && e.angle.abs() < 1e-6, "{e:?}");
        // Degenerate and non-finite frames fall back to upright at their origin.
        let e = label_edge(pos2(5.0, 5.0), pos2(5.0, 5.0), pos2(5.0, 9.0));
        assert_eq!(e, LabelEdge { corner: pos2(5.0, 5.0), angle: 0.0, len: 0.0 });
        let e = label_edge(pos2(5.0, 5.0), pos2(f32::NAN, 5.0), pos2(5.0, 9.0));
        assert_eq!(e.angle, 0.0);
    }

    #[test]
    fn turning_shapes_moves_them_about_the_pivot() {
        let rot = Rot2::from_angle(std::f32::consts::FRAC_PI_2);
        let mut s = Shape::LineSegment { points: [pos2(10.0, 0.0), pos2(20.0, 0.0)], stroke: egui::Stroke::NONE };
        turn(&mut s, rot, std::f32::consts::FRAC_PI_2, Pos2::ZERO);
        let Shape::LineSegment { points, .. } = s else { panic!() };
        assert!(close(points[0], pos2(0.0, 10.0)) && close(points[1], pos2(0.0, 20.0)));
        let mut r = Shape::rect_filled(Rect::from_min_size(pos2(10.0, -5.0), vec2(20.0, 10.0)), 0.0, Color32::RED);
        turn(&mut r, rot, std::f32::consts::FRAC_PI_2, Pos2::ZERO);
        let Shape::Rect(r) = r else { panic!() };
        assert!(close(r.rect.center(), pos2(0.0, 20.0)) && (r.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }
}
