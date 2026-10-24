//! The AI panel's own icons (Fix T4), drawn like `icons.rs`'s on a 16 px grid in `rect`: following's
//! crosshair, the composer's "+" (Fix AB4; the Selection and Image chips of Fix Z6 went into its
//! menu), and the chevrons (Fix X2). Send's arrow and Stop's square are drawn with the button
//! (`ai_chips.rs`); the composer's gear went into the agent menu (Fix X3). (`icons.rs` is over the
//! size limit, so new icons live here.)

use egui::{Color32, Painter, Rect, Shape, Stroke, vec2};

/// Which one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiIcon {
    Crosshair,
    /// The composer's "+" (Fix AB4): two 1.5 strokes, 10 long.
    Plus,
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
        AiIcon::Plus => {
            painter.line_segment([p(8.0, 3.0), p(8.0, 13.0)], fine);
            painter.line_segment([p(3.0, 8.0), p(13.0, 8.0)], fine);
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
