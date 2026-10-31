//! The mark's geometry: two identical pieces, the second turned 180°, 6 apart, inside the drawing's
//! box; fitted into any rectangle without stretching.

use egui::{Rect, pos2, vec2};

use super::{fit, pieces};

fn bounds(p: &super::Piece) -> Rect {
    p.rects[0].0.union(p.rects[1].0)
}

/// The two pieces fill the drawing's 150 × 141 box between them and are the same shape: the second
/// is the first turned about the box's centre.
#[test]
fn the_two_pieces_are_one_shape_turned_180_degrees() {
    let [top, bottom] = pieces();
    let all = bounds(&top).union(bounds(&bottom));
    assert!((all.min.x - 4.0).abs() < 0.3 && (all.min.y - 4.0).abs() < 0.3, "{all:?}");
    assert!((all.width() - 150.0).abs() < 0.3 && (all.height() - 141.0).abs() < 0.3, "{all:?}");
    for (a, b) in top.rects.iter().zip(&bottom.rects) {
        assert!((a.0.size() - b.0.size()).length() < 0.01, "same parts: {a:?} {b:?}");
    }
    let c = all.center();
    let turned = |p: egui::Pos2| pos2(2.0 * c.x - p.x, 2.0 * c.y - p.y);
    assert!((turned(top.fillet.0) - bottom.fillet.0).length() < 0.5, "the inner curves face each other");
}

/// The pieces don't touch: 6 units between the top bar and the bottom leg, and between the top leg
/// and the bottom bar, so the one-colour mark still reads as two pieces.
#[test]
fn the_pieces_keep_a_6_unit_gap() {
    let [top, bottom] = pieces();
    let (top_bar, top_leg) = (top.rects[0].0, top.rects[1].0);
    let (bottom_bar, bottom_leg) = (bottom.rects[0].0, bottom.rects[1].0);
    assert!((bottom_leg.min.y - top_bar.max.y - 6.0).abs() < 0.2, "{top_bar:?} {bottom_leg:?}");
    assert!((bottom_bar.min.y - top_leg.max.y - 6.0).abs() < 0.2, "{top_leg:?} {bottom_bar:?}");
}

/// Fitted into a tab's 14 × 14 or a wide rectangle, the mark keeps its 150 : 141 proportion and is
/// centred.
#[test]
fn the_mark_fits_without_stretching() {
    for r in [Rect::from_min_size(pos2(10.0, 20.0), vec2(14.0, 14.0)), Rect::from_min_size(pos2(0.0, 0.0), vec2(80.0, 30.0))] {
        let (s, origin) = fit(r);
        let (w, h) = (150.0 * s, 141.0 * s);
        assert!(w <= r.width() + 0.01 && h <= r.height() + 0.01, "{r:?}");
        let centre = origin + vec2(w, h) / 2.0;
        assert!((centre - r.center()).length() < 0.01, "{r:?}");
    }
}
