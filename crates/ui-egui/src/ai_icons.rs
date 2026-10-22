//! The AI panel's own icons (Fix T4), drawn like `icons.rs`'s on a 16 px grid in `rect`: following's
//! crosshair, the settings gear and Send's arrow. (`icons.rs` is over the size limit, so new icons
//! live here.)

use egui::{Color32, Painter, Rect, Shape, Stroke, vec2};

/// Which one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiIcon {
    Crosshair,
    Gear,
    Send,
}

pub fn paint(painter: &Painter, rect: Rect, icon: AiIcon, color: Color32) {
    let s = rect.width().min(rect.height()) / 16.0;
    let c = rect.center();
    let p = |x: f32, y: f32| c + vec2((x - 8.0) * s, (y - 8.0) * s);
    let line = Stroke::new(1.3 * s, color);
    match icon {
        AiIcon::Crosshair => {
            painter.circle_stroke(c, 4.5 * s, line);
            painter.circle_filled(c, 1.3 * s, color);
            for (a, b) in [((8.0, 1.0), (8.0, 4.0)), ((8.0, 12.0), (8.0, 15.0)), ((1.0, 8.0), (4.0, 8.0)), ((12.0, 8.0), (15.0, 8.0))] {
                painter.line_segment([p(a.0, a.1), p(b.0, b.1)], line);
            }
        }
        AiIcon::Gear => {
            painter.circle_stroke(c, 2.2 * s, line);
            painter.circle_stroke(c, 5.0 * s, line);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let d = vec2(a.cos(), a.sin());
                painter.line_segment([c + d * 5.0 * s, c + d * 6.8 * s], Stroke::new(2.2 * s, color));
            }
        }
        AiIcon::Send => {
            // An arrow up, as on the reference: the stem and its head.
            painter.line_segment([p(8.0, 13.5), p(8.0, 3.0)], Stroke::new(1.6 * s, color));
            painter.add(Shape::line(vec![p(3.5, 7.5), p(8.0, 3.0), p(12.5, 7.5)], Stroke::new(1.6 * s, color)));
        }
    }
}
