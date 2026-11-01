//! Sketch import tests. Every file is built in code from the published schema (see `builder`).

mod builder;
mod hostile;
mod sample;
mod symbols;
mod unsupported;

use builder::*;
use figcraft_doc::color::{BlendMode, GradientKind, ImageScaleMode, PaintKind};
use figcraft_doc::{
    BoolOp, Document, EffectKind, LetterSpacing, LineHeight, MaskType, Node, NodeKind, StrokeAlign, StrokeCap, StrokeJoin, TextAlignH, TextAlignV,
    TextAutoResize, TextDecoration, WindingRule,
};
use serde_json::{Value, json};

use super::{import, is_sketch};
use crate::import_report::{ImportWarning, Severity};

/// Import a file of one page holding `layers` (the document must be structurally valid).
fn one(layers: Vec<Value>) -> (Document, Vec<ImportWarning>) {
    let (doc, warnings) = import(&sketch(&[page(1, "Page 1", layers)], json!({}), &[]), "Test").unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    (doc, warnings)
}

fn named<'a>(doc: &'a Document, name: &str) -> &'a Node {
    doc.nodes().find(|n| n.name == name).unwrap_or_else(|| panic!("no layer {name}"))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn has(w: &[ImportWarning], severity: Severity, path: &str, needle: &str) -> bool {
    w.iter().any(|x| x.severity == severity && x.path == path && x.message.contains(needle))
}

fn rgb(p: &figcraft_doc::color::Paint) -> (f32, f32, f32) {
    match &p.kind {
        PaintKind::Solid { color } => (color.r, color.g, color.b),
        other => panic!("not a solid paint: {other:?}"),
    }
}

#[test]
fn pages_and_artboards_are_imported_with_their_frames() {
    let a = with(artboard(2, "Poster", frame(100.0, 50.0, 400.0, 300.0), vec![]), "backgroundColor", color(0.2, 0.4, 0.6, 1.0));
    let bytes = sketch(&[page(1, "Cover", vec![a]), page(9, "Second", vec![])], json!({}), &[]);
    let (doc, warnings) = import(&bytes, "Poster file").unwrap();
    assert_eq!(doc.name, "Poster file");
    let names: Vec<&str> = doc.pages.iter().map(|p| doc.node(*p).unwrap().name.as_str()).collect();
    assert_eq!(names, ["Cover", "Second"]);
    let f = named(&doc, "Poster");
    assert!(matches!(f.kind, NodeKind::Frame));
    assert!(f.clip);
    assert_eq!((f.width, f.height), (400.0, 300.0));
    assert_eq!(f.transform.translation().to_point(), (100.0, 50.0).into());
    assert_eq!(rgb(&f.fills[0]), (0.2, 0.4, 0.6));
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn rectangles_keep_a_radius_per_corner() {
    let r = rectangle(3, "Card", frame(10.0, 20.0, 200.0, 100.0), [4.0, 8.0, 12.0, 16.0]);
    let (doc, _) = one(vec![r]);
    let n = named(&doc, "Card");
    assert!(matches!(n.kind, NodeKind::Rectangle));
    assert_eq!(n.corners.radii, [4.0, 8.0, 12.0, 16.0]);
    // Without per-point radii, `fixedRadius` rounds every corner.
    let r = with(rectangle(4, "Pill", frame(0.0, 0.0, 80.0, 20.0), [0.0; 4]), "fixedRadius", json!(10));
    let (doc, _) = one(vec![r]);
    assert_eq!(named(&doc, "Pill").corners.radii, [10.0; 4]);
}

#[test]
fn solid_fills_keep_colour_alpha_and_opacity() {
    let mut fill = solid(color(1.0, 0.0, 0.5, 0.5));
    fill["contextSettings"]["opacity"] = json!(0.5);
    let r = styled(rectangle(3, "Box", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]), "fills", json!([fill]));
    let (doc, _) = one(vec![r]);
    let p = &named(&doc, "Box").fills[0];
    assert_eq!(rgb(p), (1.0, 0.0, 0.5));
    assert!((p.opacity - 0.25).abs() < 1e-6);
}

#[test]
fn linear_and_radial_gradients_keep_handles_and_stops() {
    let stops = [(0.0, color(1.0, 0.0, 0.0, 1.0)), (1.0, color(0.0, 0.0, 1.0, 1.0))];
    let lin = styled(rectangle(3, "Linear", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]), "fills", json!([gradient(0, "{0.5, 0}", "{0.5, 1}", &stops)]));
    let rad = styled(rectangle(4, "Radial", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]), "fills", json!([gradient(1, "{0.5, 0.5}", "{1, 0.5}", &stops)]));
    let ang = styled(rectangle(5, "Angular", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]), "fills", json!([gradient(2, "{0.5, 0}", "{0.5, 1}", &stops)]));
    let (doc, warnings) = one(vec![lin, rad, ang]);
    let g = |name: &str| match &named(&doc, name).fills[0].kind {
        PaintKind::Gradient(g) => g.clone(),
        other => panic!("{other:?}"),
    };
    let l = g("Linear");
    assert_eq!(l.kind, GradientKind::Linear);
    assert_eq!((l.from, l.to), ((0.5, 0.0), (0.5, 1.0)));
    assert_eq!(l.stops.len(), 2);
    assert_eq!((l.stops[1].position, l.stops[1].color.b), (1.0, 1.0));
    let r = g("Radial");
    assert_eq!(r.kind, GradientKind::Radial);
    assert_eq!((r.from, r.to), ((0.5, 0.5), (1.0, 0.5)));
    assert_eq!(g("Angular").kind, GradientKind::Angular);
    assert!(has(&warnings, Severity::Approximated, "Page 1 / Angular", "angular gradient"));
}

#[test]
fn borders_become_strokes() {
    let r = styled(rectangle(3, "Framed", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]), "borders", json!([border(color(0.0, 1.0, 0.0, 1.0), 3.0, 1)]));
    let r = styled(
        r,
        "borderOptions",
        json!({"_class": "borderOptions", "isEnabled": true, "dashPattern": [4, 2], "lineCapStyle": 1, "lineJoinStyle": 2}),
    );
    let (doc, _) = one(vec![r]);
    let n = named(&doc, "Framed");
    assert_eq!(rgb(&n.strokes[0]), (0.0, 1.0, 0.0));
    assert_eq!(n.stroke.weight, 3.0);
    assert_eq!(n.stroke.align, StrokeAlign::Inside);
    assert_eq!(n.stroke.dashes, [4.0, 2.0]);
    assert_eq!((n.stroke.cap_start, n.stroke.join), (StrokeCap::Round, StrokeJoin::Bevel));
}

#[test]
fn shadows_inner_shadows_and_blurs_become_effects() {
    let r = rectangle(3, "Lifted", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]);
    let r = styled(r, "shadows", json!([shadow("shadow", color(0.0, 0.0, 0.0, 0.5), 2.0, 4.0, 8.0, 1.0)]));
    let r = styled(r, "innerShadows", json!([shadow("innerShadow", color(1.0, 1.0, 1.0, 1.0), 0.0, 1.0, 2.0, 0.0)]));
    let r = styled(r, "blur", json!({"_class": "blur", "isEnabled": true, "radius": 6, "type": 0}));
    let b = styled(
        rectangle(4, "Glass", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]),
        "blur",
        json!({"_class": "blur", "isEnabled": true, "radius": 12, "type": 3}),
    );
    let m = styled(
        rectangle(5, "Moving", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]),
        "blur",
        json!({"_class": "blur", "isEnabled": true, "radius": 3, "type": 1}),
    );
    let (doc, warnings) = one(vec![r, b, m]);
    let fx = &named(&doc, "Lifted").effects;
    assert_eq!(fx.len(), 3);
    match &fx[0].kind {
        EffectKind::DropShadow(s) => {
            assert_eq!((s.offset, s.radius, s.spread), ((2.0, 4.0), 8.0, 1.0));
            assert!((s.color.a - 0.5).abs() < 1e-6);
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(&fx[1].kind, EffectKind::InnerShadow(s) if s.offset == (0.0, 1.0) && s.radius == 2.0));
    assert!(matches!(&fx[2].kind, EffectKind::LayerBlur(b) if b.radius == 6.0));
    assert!(matches!(&named(&doc, "Glass").effects[0].kind, EffectKind::BackgroundBlur(b) if b.radius == 12.0));
    assert!(has(&warnings, Severity::Approximated, "Page 1 / Moving", "motion blur"));
    // The disabled blur every style carries adds nothing.
    let (doc, _) = one(vec![rectangle(6, "Plain", frame(0.0, 0.0, 1.0, 1.0), [0.0; 4])]);
    assert!(named(&doc, "Plain").effects.is_empty());
}

#[test]
fn opacity_and_blend_come_from_the_context_settings() {
    let r = styled(
        rectangle(3, "Faded", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]),
        "contextSettings",
        json!({"_class": "graphicsContextSettings", "blendMode": 2, "opacity": 0.4}),
    );
    let (doc, _) = one(vec![r]);
    let n = named(&doc, "Faded");
    assert!((n.opacity - 0.4).abs() < 1e-6);
    assert_eq!(n.blend, BlendMode::Multiply);
}

#[test]
fn ovals_polygons_and_stars_keep_their_kind() {
    let o = oval(3, "Dot", frame(0.0, 0.0, 40.0, 20.0));
    let s = star(4, "Burst", frame(0.0, 0.0, 50.0, 50.0), 5, 0.4);
    let pts: Vec<Value> = figcraft_doc::geom::polygon_points(1.0, 1.0, 6).iter().map(|p| point(p.x, p.y, 0.0)).collect();
    let hex = with(shape("polygon", 5, "Hex", frame(0.0, 0.0, 30.0, 30.0), pts, true), "numberOfPoints", json!(6));
    let mut odd = star(6, "Odd star", frame(0.0, 0.0, 50.0, 50.0), 5, 0.4);
    odd["points"][0] = point(0.1, 0.1, 0.0);
    let (doc, warnings) = one(vec![o, s, hex, odd]);
    assert!(matches!(named(&doc, "Dot").kind, NodeKind::Ellipse { sweep, .. } if close(sweep, std::f64::consts::TAU)));
    assert!(matches!(named(&doc, "Burst").kind, NodeKind::Star { count: 5, inner_ratio } if close(inner_ratio, 0.4)));
    assert!(matches!(named(&doc, "Hex").kind, NodeKind::Polygon { count: 6 }));
    // A star whose points aren't the regular outline keeps its exact outline as a path.
    assert!(matches!(&named(&doc, "Odd star").kind, NodeKind::Vector { network } if network.vertices.len() == 10));
    assert!(has(&warnings, Severity::Info, "Page 1 / Odd star", "vector path"));
}

#[test]
fn paths_keep_points_curves_and_closing() {
    let pts = vec![point(0.0, 0.0, 0.0), curve((1.0, 0.5), (1.0, 0.25), (1.0, 0.75)), point(0.0, 1.0, 5.0)];
    let closed = shape("shapePath", 3, "Leaf", frame(0.0, 0.0, 100.0, 200.0), pts.clone(), true);
    let open = shape("shapePath", 4, "Line", frame(0.0, 0.0, 100.0, 200.0), pts, false);
    let (doc, _) = one(vec![closed, open]);
    let NodeKind::Vector { network } = &named(&doc, "Leaf").kind else { panic!() };
    assert_eq!(network.vertices.len(), 3);
    assert_eq!((network.vertices[1].x, network.vertices[1].y), (100.0, 100.0));
    assert_eq!(network.vertices[2].radius, 5.0);
    assert_eq!(network.segments.len(), 3);
    // Segment 0 arrives at the curved point by its `curveTo`, segment 1 leaves by its `curveFrom`.
    assert_eq!(network.segments[0].tb, (0.0, -50.0));
    assert_eq!(network.segments[1].ta, (0.0, 50.0));
    assert_eq!(network.regions.len(), 1);
    assert_eq!(network.regions[0].winding, WindingRule::EvenOdd);
    let NodeKind::Vector { network } = &named(&doc, "Line").kind else { panic!() };
    assert_eq!(network.segments.len(), 2);
    assert!(network.regions.is_empty());
}

#[test]
fn rotation_and_flips_turn_about_the_frame_centre() {
    let r = with(rectangle(3, "Turned", frame(0.0, 0.0, 20.0, 10.0), [0.0; 4]), "rotation", json!(90));
    let f = with(rectangle(4, "Flipped", frame(30.0, 0.0, 20.0, 10.0), [0.0; 4]), "isFlippedHorizontal", json!(true));
    let (doc, _) = one(vec![r, f]);
    let t = named(&doc, "Turned");
    let b = doc.world_bounds(t.id).unwrap();
    assert!(close(b.x0, 5.0) && close(b.y0, -5.0) && close(b.width(), 10.0) && close(b.height(), 20.0), "{b:?}");
    // Counter-clockwise on screen: the box's top-left corner ends at the bottom left.
    let tl = t.transform * figcraft_doc::geom::Point::new(0.0, 0.0);
    assert!(close(tl.x, 5.0) && close(tl.y, 15.0), "{tl:?}");
    let f = named(&doc, "Flipped");
    assert!(f.transform.determinant() < 0.0);
    let b = doc.world_bounds(f.id).unwrap();
    assert!(close(b.x0, 30.0) && close(b.width(), 20.0));
}

#[test]
fn groups_place_children_relative_to_their_frame() {
    let inner = rectangle(4, "Inside", frame(5.0, 6.0, 10.0, 10.0), [0.0; 4]);
    let g = container("group", 3, "Cluster", frame(100.0, 200.0, 15.0, 16.0), vec![inner]);
    let a = artboard(2, "Board", frame(1000.0, 0.0, 500.0, 500.0), vec![g]);
    let (doc, _) = one(vec![a]);
    assert!(matches!(named(&doc, "Cluster").kind, NodeKind::Group));
    let b = doc.world_bounds(named(&doc, "Inside").id).unwrap();
    assert_eq!((b.x0, b.y0), (1105.0, 206.0));
    let g = doc.world_bounds(named(&doc, "Cluster").id).unwrap();
    assert_eq!((g.x0, g.y0, g.width(), g.height()), (1105.0, 206.0, 10.0, 10.0));
}

#[test]
fn shape_groups_become_boolean_groups() {
    let a = rectangle(4, "Base", frame(0.0, 0.0, 20.0, 20.0), [0.0; 4]);
    let b = with(oval(5, "Hole", frame(5.0, 5.0, 10.0, 10.0)), "booleanOperation", json!(1));
    let g = container("shapeGroup", 3, "Ring", frame(0.0, 0.0, 20.0, 20.0), vec![a, b]);
    let g = styled(g, "fills", json!([solid(color(1.0, 0.0, 0.0, 1.0))]));
    let (doc, warnings) = one(vec![g]);
    let ring = named(&doc, "Ring");
    assert!(matches!(ring.kind, NodeKind::BooleanOp { op: BoolOp::Union }));
    assert_eq!(rgb(&ring.fills[0]), (1.0, 0.0, 0.0));
    assert_eq!(named(&doc, "Hole").bool_op, Some(BoolOp::Subtract));
    assert_eq!(named(&doc, "Base").bool_op, None);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn text_keeps_content_font_size_weight_colour_and_alignment() {
    let s = "Hello world";
    let t = text(3, "Title", frame(0.0, 0.0, 200.0, 40.0), s, vec![(0, 11, attrs("Inter-SemiBold", 24.0, color(0.1, 0.2, 0.3, 1.0), 2))]);
    let t = with(t, "textBehaviour", json!(2));
    let t = styled(
        t,
        "textStyle",
        json!({"_class": "textStyle", "verticalAlignment": 1, "encodedAttributes": attrs("Inter-SemiBold", 24.0, color(0.1, 0.2, 0.3, 1.0), 2)}),
    );
    let (doc, _) = one(vec![t]);
    let n = named(&doc, "Title");
    let NodeKind::Text(td) = &n.kind else { panic!() };
    assert_eq!(td.characters, s);
    assert_eq!((td.style.font_family.as_str(), td.style.font_style.as_str(), td.style.font_size), ("Inter", "Semi Bold", 24.0));
    assert_eq!(td.style.weight(), 600);
    assert_eq!((td.align_h, td.align_v, td.auto_resize), (TextAlignH::Center, TextAlignV::Middle, TextAutoResize::None));
    assert!(td.runs.is_empty());
    let (r, g, b) = rgb(&n.fills[0]);
    assert!(close(f64::from(r), 0.1) && close(f64::from(g), 0.2) && close(f64::from(b), 0.3));
}

#[test]
fn text_runs_become_styled_runs_counted_in_characters() {
    // "😀 Big": the emoji is two UTF-16 units; the run "Big" starts at unit 3, character 2.
    let mut big = attrs("Helvetica-BoldOblique", 30.0, color(1.0, 0.0, 0.0, 1.0), 0);
    big["kerning"] = json!(1.5);
    big["paragraphStyle"]["maximumLineHeight"] = json!(36);
    let t =
        text(3, "Mixed", frame(0.0, 0.0, 200.0, 40.0), "😀 Big", vec![(0, 3, attrs("Helvetica", 12.0, color(0.0, 0.0, 0.0, 1.0), 0)), (3, 3, big)]);
    let (doc, warnings) = one(vec![t]);
    let NodeKind::Text(td) = &named(&doc, "Mixed").kind else { panic!() };
    assert_eq!((td.style.font_family.as_str(), td.style.font_style.as_str(), td.style.font_size), ("Helvetica", "Regular", 12.0));
    assert_eq!(td.runs.len(), 1);
    let r = &td.runs[0];
    assert_eq!((r.start, r.end), (2, 5));
    assert_eq!((r.style.font_style.as_str(), r.style.font_size), ("Bold Italic", 30.0));
    assert_eq!(r.style.letter_spacing, LetterSpacing::Pixels(1.5));
    assert_eq!(r.style.line_height, LineHeight::Pixels(36.0));
    assert_eq!(rgb(&r.style.fills.as_ref().unwrap()[0]), (1.0, 0.0, 0.0));
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut under = attrs("Inter-Regular", 12.0, color(0.0, 0.0, 0.0, 1.0), 0);
    under["underlineStyle"] = json!(1);
    let t = text(4, "Link", frame(0.0, 0.0, 10.0, 10.0), "go", vec![(0, 2, under)]);
    let (doc, _) = one(vec![t]);
    let NodeKind::Text(td) = &named(&doc, "Link").kind else { panic!() };
    assert_eq!(td.style.decoration, TextDecoration::Underline);
}

#[test]
fn postscript_names_split_into_family_and_style() {
    use super::text::font_names;
    let f = |s: &str| font_names(s);
    assert_eq!(f("Inter-SemiBoldItalic"), ("Inter".into(), "Semi Bold Italic".into()));
    assert_eq!(f("SFProText-Regular"), ("SF Pro Text".into(), "Regular".into()));
    assert_eq!(f("HelveticaNeue"), ("Helvetica Neue".into(), "Regular".into()));
    assert_eq!(f("Arial-BoldMT"), ("Arial".into(), "Bold".into()));
    assert_eq!(f("Roboto-Semibold"), ("Roboto".into(), "Semi Bold".into()));
}

#[test]
fn bitmaps_and_image_fills_carry_the_archive_image() {
    let png = png(2, 2, |x, y| [x as u8 * 255, y as u8 * 255, 0, 255]);
    let bmp = with(layer("bitmap", 3, "Photo", frame(0.0, 0.0, 40.0, 30.0)), "image", file_ref("images/photo.png"));
    let tile = styled(rectangle(4, "Tiles", frame(0.0, 0.0, 40.0, 30.0), [0.0; 4]), "fills", json!([image_fill("images/photo", 0)]));
    let bytes = sketch(&[page(1, "Page 1", vec![bmp, tile])], json!({}), &[("images/photo.png", &png)]);
    let (doc, warnings) = import(&bytes, "Images").unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let key = |name: &str| match &named(&doc, name).fills[0].kind {
        PaintKind::Image(i) => i.clone(),
        other => panic!("{other:?}"),
    };
    let photo = key("Photo");
    assert_eq!(photo.scale_mode, ImageScaleMode::Crop);
    let data = &doc.images[&photo.image];
    assert_eq!(data.bytes.as_slice(), png.as_slice());
    assert_eq!((data.width, data.height), (2, 2));
    // The same image is stored once; the reference without its extension finds it.
    let tiles = key("Tiles");
    assert_eq!(tiles.image, photo.image);
    assert_eq!(tiles.scale_mode, ImageScaleMode::Tile);
    assert_eq!(doc.images.len(), 1);
}

#[test]
fn inline_image_data_is_decoded() {
    let png = png(1, 1, |_, _| [0, 0, 255, 255]);
    let b64 = base64_encode(&png);
    let r = json!({"_class": "MSJSONOriginalDataReference", "_ref_class": "MSImageData", "_ref": "images/x", "data": {"_data": b64}, "sha1": {"_data": "abc"}});
    let bmp = with(layer("bitmap", 3, "Inline", frame(0.0, 0.0, 1.0, 1.0)), "image", r);
    let (doc, warnings) = one(vec![bmp]);
    assert!(warnings.is_empty(), "{warnings:?}");
    let PaintKind::Image(i) = &named(&doc, "Inline").fills[0].kind else { panic!() };
    assert_eq!(doc.images[&i.image].bytes.as_slice(), png.as_slice());
    assert_eq!(super::images::base64("aGk="), Some(b"hi".to_vec()));
    assert_eq!(super::images::base64("a*"), None);
}

#[test]
fn clipping_masks_mask_the_layers_above() {
    let m = with(oval(3, "Mask", frame(0.0, 0.0, 10.0, 10.0)), "hasClippingMask", json!(true));
    let (doc, _) = one(vec![m, rectangle(4, "Masked", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4])]);
    let n = named(&doc, "Mask");
    assert!(n.is_mask);
    assert_eq!(n.mask_type, MaskType::Vector);
}

#[test]
fn is_sketch_tells_sketch_files_from_others() {
    let sketch_file = sketch(&[page(1, "Page 1", vec![])], json!({}), &[]);
    assert!(is_sketch(&sketch_file));
    let native = zip(&[("document.json".into(), br#"{"format":"figcraft","version":1,"pages":[]}"#.to_vec()), ("meta.json".into(), b"{}".to_vec())]);
    assert!(!is_sketch(&native));
    let fig = zip(&[("canvas.fig".into(), b"fig-kiwi".to_vec()), ("meta.json".into(), b"{}".to_vec())]);
    assert!(!is_sketch(&fig));
    let no_meta = zip(&[("document.json".into(), br#"{"_class":"document"}"#.to_vec())]);
    assert!(!is_sketch(&no_meta));
    assert!(!is_sketch(b"\x00\x01garbage bytes that are not a zip"));
    assert!(!is_sketch(b""));
    assert!(!is_sketch(b"PK\x03\x04truncated"));
}

#[test]
fn import_of_garbage_is_an_error() {
    assert!(import(b"", "x").is_err());
    assert!(import(b"not a zip at all", "x").is_err());
    assert!(import(b"PK\x03\x04\x00\x00", "x").is_err());
    let native = zip(&[("document.json".into(), br#"{"format":"figcraft"}"#.to_vec()), ("meta.json".into(), b"{}".to_vec())]);
    assert!(import(&native, "x").is_err());
    let broken = zip(&[("document.json".into(), b"{\"_class\": \"docu".to_vec()), ("meta.json".into(), b"{}".to_vec())]);
    assert!(import(&broken, "x").is_err());
    let no_meta = zip(&[("document.json".into(), br#"{"_class":"document","pages":[]}"#.to_vec())]);
    assert!(import(&no_meta, "x").is_err());
}

/// Standard base64 with padding (the tests' own encoder).
fn base64_encode(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                out.push(char::from(A[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}
