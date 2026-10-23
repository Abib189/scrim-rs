//! The AI panel's own icons (Fix T4), drawn like `icons.rs`'s on a 16 px grid in `rect`: following's
//! crosshair, the composer's Selection and Image chips (Fix Z6), and the chevrons (Fix X2). Send's arrow and Stop's square are drawn with the button
//! (`ai_chips.rs`); the composer's gear went into the agent menu (Fix X3). (`icons.rs` is over the
//! size limit, so new icons live here.)

use egui::{Color32, Painter, Rect, Shape, Stroke, vec2};

/// Which one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiIcon {
    Crosshair,
    /// A 14 × 14 rounded dashed square with a pointer arrow in it (the selection), stroke 1.5.
    Selection,
    /// A picture: a rounded frame, a sun and a hill, stroke 1.5.
    Image,
}

/// A rounded rectangle's outline from `a` to `b` (corner radius `r`), as points, closed.
fn rounded(a: egui::Pos2, b: egui::Pos2, r: f32) -> Vec<egui::Pos2> {
    let corners = [(b.x - r, a.y + r, -90.0_f32), (b.x - r, b.y - r, 0.0), (a.x + r, b.y - r, 90.0), (a.x + r, a.y + r, 180.0)];
    let mut out: Vec<egui::Pos2> = Vec::new();
    for (cx, cy, start) in corners {
        for k in 0..=4 {
            let t = (start + 22.5 * k as f32).to_radians();
            out.push(egui::pos2(cx + r * t.cos(), cy + r * t.sin()));
        }
    }
    if let Some(first) = out.first().copied() {
        out.push(first);
    }
    out
}

/// A chevron pointing down, `w` × `h` points, centred on `c`, its stroke `width` (Fix X2: the title's
/// 9 × 5 at 1.5; the agent chip's 8 × 4).
pub fn chevron(painter: &Painter, c: egui::Pos2, w: f32, h: f32, width: f32, color: Color32) {
    let (x, y) = (w / 2.0, h / 2.0);
    painter.add(Shape::line(vec![c + vec2(-x, -y), c + vec2(0.0, y), c + vec2(x, -y)], Stroke::new(width, color)));
}

pub fn paint(painter: &Painter, rect: Rect, icon: AiIcon, color: Color32) {
    let s = rect.width().min(rect.height()) / 16.0;
    let c = rect.center();
    let p = |x: f32, y: f32| c + vec2((x - 8.0) * s, (y - 8.0) * s);
    let line = Stroke::new(1.3 * s, color);
    let fine = Stroke::new(1.5 * s, color);
    match icon {
        AiIcon::Selection => {
            painter.extend(Shape::dashed_line(&rounded(p(1.0, 1.0), p(15.0, 15.0), 2.5 * s), fine, 2.0 * s, 1.6 * s));
            let arrow = [(6.0, 6.0), (6.0, 12.6), (7.8, 11.0), (9.0, 13.4), (10.4, 12.8), (9.2, 10.4), (11.6, 10.4)];
            painter.add(Shape::closed_line(arrow.iter().map(|(x, y)| p(*x, *y)).collect(), Stroke::new(1.2 * s, color)));
        }
        AiIcon::Image => {
            painter.add(Shape::closed_line(rounded(p(1.5, 2.5), p(14.5, 13.5), 2.0 * s), fine));
            painter.circle_filled(p(5.5, 6.2), 1.3 * s, color);
            painter.add(Shape::line(vec![p(2.0, 12.0), p(6.0, 8.6), p(9.0, 11.2), p(11.0, 9.4), p(14.0, 12.0)], fine));
        }
        AiIcon::Crosshair => {
            painter.circle_stroke(c, 4.5 * s, line);
            painter.circle_filled(c, 1.3 * s, color);
            for (a, b) in [((8.0, 1.0), (8.0, 4.0)), ((8.0, 12.0), (8.0, 15.0)), ((1.0, 8.0), (4.0, 8.0)), ((12.0, 8.0), (15.0, 8.0))] {
                painter.line_segment([p(a.0, a.1), p(b.0, b.1)], line);
            }
        }
    }
}
