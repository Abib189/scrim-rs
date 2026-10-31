//! Scrim's mark, drawn in the interface (`docs/design/rebrand.md` › The app icon; the app icon is
//! `xtask/src/icon.rs`): two identical pieces interlock into one shape, a person and an agent on one
//! surface. Each piece is a bar with a leg, corners of radius 10, in the owner's drawing's units
//! (its box is 150 × 141 at 4, 4); the second piece is the first turned 180°, 6 apart.
//!
//! egui fills only convex shapes with anti-aliasing, so a piece is drawn as its bar and its leg (two
//! rounded rectangles that overlap where they meet) plus the small inner curve where they join.

use egui::epaint::Mesh;
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, pos2, vec2};

/// The brand olive: the top piece (the brand kit's usage guide).
pub const OLIVE: Color32 = Color32::from_rgb(0x7f, 0x75, 0x00);
/// The ink the bottom piece takes on light backgrounds, and the label colour on an olive tile.
pub const INK: Color32 = Color32::from_rgb(0x11, 0x12, 0x14);

/// The drawing's box.
const BOX: (f32, f32, f32, f32) = (4.0, 4.0, 150.0, 141.0);
/// A piece in its own units: the bar 149.75 × 45 and the leg 50 wide down to 89.92; radius 10.
const BAR: (f32, f32) = (149.75, 45.0);
const LEG: (f32, f32) = (50.0, 89.92);
const R: f32 = 10.0;
/// Where each piece sits: the top piece at (4.254, 4); the bottom one turned about (153.745, 145).
const TOP_AT: (f32, f32) = (4.254, 4.0);
const TURN: (f32, f32) = (153.745, 145.0);

/// The parts of one piece, in the drawing's units, as rectangles with their corner radii
/// [nw, ne, sw, se], and the inner curve's corner and the centre of its arc.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub rects: [(Rect, [f32; 4]); 2],
    pub fillet: (Pos2, Pos2),
}

/// The two pieces: the top one, then the bottom one.
pub fn pieces() -> [Piece; 2] {
    let (tx, ty) = TOP_AT;
    let top = Piece {
        rects: [
            (Rect::from_min_size(pos2(tx, ty), vec2(BAR.0, BAR.1)), [R; 4]),
            (Rect::from_min_size(pos2(tx, ty), vec2(LEG.0, LEG.1)), [R, 0.0, R, R]),
        ],
        fillet: (pos2(tx + LEG.0, ty + BAR.1), pos2(tx + LEG.0 + R, ty + BAR.1 + R)),
    };
    // Turned 180°: (x, y) → (TURN.0 − x, TURN.1 − y); corners swap nw↔se and ne↔sw.
    let turn = |p: Pos2| pos2(TURN.0 - (p.x - tx), TURN.1 - (p.y - ty));
    let turn_rect = |(r, [nw, ne, sw, se]): (Rect, [f32; 4])| (Rect::from_two_pos(turn(r.min), turn(r.max)), [se, sw, ne, nw]);
    let bottom = Piece { rects: [turn_rect(top.rects[0]), turn_rect(top.rects[1])], fillet: (turn(top.fillet.0), turn(top.fillet.1)) };
    [top, bottom]
}

/// The mark fitted into `rect`, centred, keeping its proportions: the scale and the top-left of
/// the drawing's box.
pub fn fit(rect: Rect) -> (f32, Pos2) {
    let s = (rect.width() / BOX.2).min(rect.height() / BOX.3);
    (s, rect.center() - vec2(BOX.2, BOX.3) * s / 2.0)
}

/// Paint the mark in `rect`: the top piece in `top`, the bottom one in `bottom`. One colour for
/// both gives the one-colour mark; the 6-unit gap keeps the pieces apart.
pub fn paint(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let (s, origin) = fit(rect);
    let at = |p: Pos2| origin + (p - pos2(BOX.0, BOX.1)) * s;
    for (piece, color) in pieces().iter().zip([top, bottom]) {
        for (r, radii) in &piece.rects {
            let cr = radii.map(|v| (v * s).round().clamp(0.0, 255.0) as u8);
            let corner = CornerRadius { nw: cr[0], ne: cr[1], sw: cr[2], se: cr[3] };
            painter.rect_filled(Rect::from_two_pos(at(r.min), at(r.max)), corner, color);
        }
        // The inner curve: a fan from the corner over the quarter arc that faces it, from where the
        // arc meets the bar (level with the centre) to where it meets the leg.
        let (corner, centre) = (at(piece.fillet.0), at(piece.fillet.1));
        let d = corner - centre;
        let (from, to) = (0.0_f32.atan2(d.x), d.y.atan2(0.0));
        let sweep = (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(corner, color);
        let steps = 8u32;
        for k in 0..=steps {
            let a = from + sweep * k as f32 / steps as f32;
            mesh.colored_vertex(centre + vec2(a.cos(), a.sin()) * R * s, color);
        }
        for k in 1..=steps {
            mesh.add_triangle(0, k, k + 1);
        }
        painter.add(Shape::mesh(mesh));
    }
}

/// The mark in the brand's two colours: olive on top, `ink` below (the theme's text colour: ink on
/// light, near-white on dark, so the bottom piece never disappears).
pub fn paint_brand(painter: &Painter, rect: Rect, ink: Color32) {
    paint(painter, rect, OLIVE, ink);
}

/// The app-icon tile at `rect`: olive with `radius` corners, the mark in white and ink, 58% of the
/// tile wide and nudged up 1.5% (as `xtask/src/icon.rs`).
pub fn paint_tile(painter: &Painter, rect: Rect, radius: u8) {
    painter.rect_filled(rect, CornerRadius::same(radius), OLIVE);
    let w = rect.width() * 0.58;
    let mark = Rect::from_center_size(rect.center() - vec2(0.0, rect.height() * 0.015), vec2(w, w * 141.0 / 150.0));
    paint(painter, mark, Color32::WHITE, INK);
}

#[cfg(test)]
#[path = "brand_mark_tests.rs"]
mod tests;
