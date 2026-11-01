//! Fills, gradients, images, borders, shadows and blurs.

use figcraft_doc::color::{BlendMode, GradientKind, ImageScaleMode, PaintKind};
use figcraft_doc::{EffectKind, StrokeAlign, StrokeCap, StrokeJoin};

use super::*;

fn rgba(p: &figcraft_doc::color::Paint) -> [u8; 4] {
    match &p.kind {
        PaintKind::Solid { color } => color.with_alpha(p.opacity).to_rgba8(),
        other => panic!("not a solid paint: {other:?}"),
    }
}

#[test]
fn solid_fills_read_every_colour_form() {
    let fills = json!([
        {"color": "80FF0000"},
        {"color": "3"},
        {"color": "34"},
        {"color": "45F", "opacity": 0.5, "blendMode": 5},
        {"color": "B2EBF2", "enabled": false},
        {}
    ]);
    let (doc, w) = one(vec![with(layer("RECT", 1, "R", [0.0, 0.0, 10.0, 10.0]), "fills", fills)]);
    let r = named(&doc, "R");
    let got: Vec<[u8; 4]> = r.fills.iter().map(rgba).collect();
    assert_eq!(
        got,
        [
            [255, 0, 0, 0x80],
            [0x33, 0x33, 0x33, 255],
            [0x34, 0x34, 0x34, 255],
            [0x44, 0x55, 0xFF, 128],
            [0xB2, 0xEB, 0xF2, 255],
            [0x80, 0x80, 0x80, 255]
        ]
    );
    assert_eq!(r.fills[3].blend, BlendMode::Screen);
    assert!(!r.fills[4].visible && r.fills[0].visible);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn gradients_keep_their_points_and_stops() {
    let linear =
        json!({"type": 1, "gradient": {"from": [0, 0], "to": [1, 1], "stops": [{"pos": 0, "color": "F00"}, {"pos": 1, "color": "800000FF"}]}});
    let radial = json!({"type": 1, "gradient": {"type": 1, "from": [0.5, 0.5], "to": [0.5, 1], "side": [1, 0.5], "stops": [{"color": "FFF"}]}});
    let a = with(layer("RECT", 1, "Linear", [0.0, 0.0, 10.0, 10.0]), "fills", json!([linear]));
    let b = with(layer("RECT", 2, "Radial", [0.0, 0.0, 10.0, 10.0]), "fills", json!([radial]));
    let (doc, w) = one(vec![a, b]);
    let PaintKind::Gradient(g) = &named(&doc, "Linear").fills[0].kind else { panic!() };
    assert_eq!((g.kind, g.from, g.to), (GradientKind::Linear, (0.0, 0.0), (1.0, 1.0)));
    assert_eq!(g.stops.len(), 2);
    assert_eq!(g.stops[0].color.to_rgba8(), [255, 0, 0, 255]);
    assert_eq!((g.stops[1].position, g.stops[1].color.to_rgba8()), (1.0, [0, 0, 255, 0x80]));
    let PaintKind::Gradient(g) = &named(&doc, "Radial").fills[0].kind else { panic!() };
    assert_eq!((g.kind, g.from, g.to, g.width), (GradientKind::Radial, (0.5, 0.5), (0.5, 1.0), (1.0, 0.5)));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn image_fills_bring_their_bytes() {
    let photo = png(4, 4, |x, y| [(x * 60) as u8, (y * 60) as u8, 128, 255]);
    let fill = json!({"type": 4, "pattern": {"image": "photo.png", "type": 0, "tileScale": 2}});
    let r = with(layer("RECT", 1, "Photo", [0.0, 0.0, 40.0, 40.0]), "fills", json!([fill]));
    let bytes = free(&[page(1, "Page 1", vec![r])], json!({}), &[("images/photo.png", &photo)]);
    let (doc, w) = import(&bytes, "Test").unwrap();
    let PaintKind::Image(img) = &named(&doc, "Photo").fills[0].kind else { panic!() };
    assert_eq!((img.scale_mode, img.scale), (ImageScaleMode::Tile, 2.0));
    assert_eq!(doc.images.get(&img.image).map(|i| i.bytes.as_ref().to_vec()), Some(photo));
    assert!(w.is_empty(), "{w:?}");
    let missing = with(layer("RECT", 1, "Missing", [0.0, 0.0, 40.0, 40.0]), "fills", json!([{"type": 4, "pattern": {"image": "nope.png"}}]));
    let (doc, w) = one(vec![missing]);
    assert!(named(&doc, "Missing").fills.is_empty());
    assert!(has(&w, Severity::Dropped, "Page 1 / Missing", "image"), "{w:?}");
}

#[test]
fn borders_and_their_options() {
    let r = layer("RECT", 1, "Bordered", [0.0, 0.0, 10.0, 10.0]);
    let r = with(with(with(r, "border", json!("00F")), "thickness", json!(3)), "linePos", json!(1));
    let r = with(with(with(r, "dash", json!([4, 2])), "lineCap", json!(1)), "lineJoin", json!(2));
    let sides =
        with(with(layer("FRAME", 2, "Sides", [0.0, 0.0, 10.0, 10.0]), "borders", json!([{"color": "F00"}])), "customThickness", json!([1, 2, 3, 4]));
    let arrow = with(with(layer("LINE", 3, "Arrow", [0.0, 0.0, 10.0, 0.0]), "points", json!([[0, 0], [1, 0]])), "endMarker", json!(2));
    let (doc, w) = one(vec![r, sides, arrow]);
    let b = named(&doc, "Bordered");
    assert_eq!(b.strokes.iter().map(rgba).collect::<Vec<_>>(), [[0, 0, 255, 255]]);
    assert_eq!((b.stroke.weight, b.stroke.align), (3.0, StrokeAlign::Inside));
    assert_eq!(b.stroke.dashes, [4.0, 2.0]);
    assert_eq!((b.stroke.cap_start, b.stroke.join), (StrokeCap::Round, StrokeJoin::Bevel));
    assert_eq!(named(&doc, "Sides").stroke.sides, Some([2.0, 3.0, 4.0, 1.0]), "[top, right, bottom, left] from [l, t, r, b]");
    assert_eq!(named(&doc, "Arrow").stroke.cap_end, StrokeCap::TriangleArrow);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn shadows_inner_shadows_and_blurs() {
    let r = layer("RECT", 1, "Shadowed", [0.0, 0.0, 10.0, 10.0]);
    let r = with(r, "shadows", json!([{"color": "40000000", "offset": [0, 4], "radius": 12, "spread": 2}]));
    let r = with(r, "innerShadows", json!([{"color": "FFF", "radius": 3, "enabled": false}]));
    let r = with(r, "blur", json!({"radius": 6}));
    let bg = with(layer("RECT", 2, "Frosted", [0.0, 0.0, 10.0, 10.0]), "blur", json!({"type": 3, "radius": 20}));
    let glass = json!({"type": 4, "glassLightAngle": 30, "glassLightIntensity": 50, "glassRefraction": 25, "glassDepth": 12, "glassDispersion": 10, "glassFrost": 8, "glassSplay": 40});
    let g = with(layer("RECT", 3, "Glass", [0.0, 0.0, 10.0, 10.0]), "blur", glass);
    let off = with(layer("RECT", 4, "Off", [0.0, 0.0, 10.0, 10.0]), "blur", json!({"enabled": false}));
    let motion = with(layer("RECT", 5, "Motion", [0.0, 0.0, 10.0, 10.0]), "blur", json!({"type": 1, "angle": 45}));
    let (doc, w) = one(vec![r, bg, g, off, motion]);
    let e = &named(&doc, "Shadowed").effects;
    let EffectKind::DropShadow(s) = &e[0].kind else { panic!() };
    assert_eq!((s.offset, s.radius, s.spread, s.color.to_rgba8()), ((0.0, 4.0), 12.0, 2.0, [0, 0, 0, 0x40]));
    let EffectKind::InnerShadow(s) = &e[1].kind else { panic!() };
    assert!(!e[1].visible && close(s.radius, 3.0));
    let EffectKind::LayerBlur(b) = &e[2].kind else { panic!() };
    assert!(e[2].visible && close(b.radius, 6.0));
    let EffectKind::BackgroundBlur(b) = &named(&doc, "Frosted").effects[0].kind else { panic!() };
    assert!(close(b.radius, 20.0));
    let EffectKind::Glass(g) = &named(&doc, "Glass").effects[0].kind else { panic!() };
    assert_eq!((g.light_angle, g.light_intensity, g.refraction, g.depth), (30.0, 0.5, 0.25, 12.0));
    assert_eq!((g.dispersion, g.frost, g.splay), (0.1, 8.0, 0.4));
    assert!(!named(&doc, "Off").effects[0].visible);
    assert!(matches!(named(&doc, "Motion").effects[0].kind, EffectKind::LayerBlur(_)));
    assert!(has(&w, Severity::Approximated, "Page 1 / Motion", "motion blur"), "{w:?}");
    assert_eq!(w.len(), 1, "{w:?}");
}
