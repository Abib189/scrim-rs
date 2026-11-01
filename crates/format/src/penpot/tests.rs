//! Penpot import tests. Every export is built in code from the documented layout (see `builder`).

mod builder;
mod hostile;
mod sample;
mod unsupported;

use builder::*;
use figcraft_doc::color::{BlendMode, PaintKind};
use figcraft_doc::{BoolOp, Document, Node, NodeKind, StyleKind};
use serde_json::{Value, json};

use super::{import, is_penpot, preview};
use crate::import_report::{ImportWarning, Severity};

/// Import an export of one page ("Page 1") holding `shapes`; the document must be valid.
fn one(shapes: Vec<Value>) -> (Document, Vec<ImportWarning>) {
    imported(Export::new().page(1, "Page 1", json!({}), shapes))
}

fn imported(e: Export) -> (Document, Vec<ImportWarning>) {
    let (doc, warnings) = import(&e.bytes(), "Test").unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    (doc, warnings)
}

fn named<'a>(doc: &'a Document, name: &str) -> &'a Node {
    doc.nodes().find(|n| n.name == name && !n.is_page()).unwrap_or_else(|| panic!("no layer {name}"))
}

fn at(n: &Node) -> (f64, f64) {
    let p = n.transform.translation();
    (p.x, p.y)
}

fn has(w: &[ImportWarning], severity: Severity, path: &str, needle: &str) -> bool {
    w.iter().any(|x| x.severity == severity && x.path == path && x.message.contains(needle))
}

fn rgba(p: &figcraft_doc::color::Paint) -> (u8, u8, u8, f32) {
    match &p.kind {
        PaintKind::Solid { color } => {
            let [r, g, b, _] = color.to_rgba8();
            (r, g, b, p.opacity)
        }
        other => panic!("not a solid paint: {other:?}"),
    }
}

#[test]
fn pages_keep_their_names_order_and_background() {
    let e = Export::new().page(1, "Second", json!({"index": 1, "background": "#112233"}), vec![]).page(
        2,
        "First",
        json!({"index": 0}),
        vec![shape(1, "rect", "Box", (0.0, 0.0, 10.0, 10.0))],
    );
    let (doc, w) = imported(e);
    assert_eq!(doc.name, "Test file");
    let names: Vec<&str> = doc.pages.iter().map(|p| doc.node(*p).unwrap().name.as_str()).collect();
    assert_eq!(names, ["First", "Second"]);
    let second = doc.node(doc.pages[1]).unwrap();
    let NodeKind::Page { background, .. } = &second.kind else { panic!() };
    assert_eq!(background.to_rgba8(), [0x11, 0x22, 0x33, 255]);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn a_board_becomes_a_clipping_frame_at_its_position_and_size() {
    let board = with(shape(1, "frame", "Poster", (100.0, 50.0, 400.0, 300.0)), "fills", json!([fill("#FFFFFF", 1.0)]));
    let (doc, w) = one(vec![board]);
    let f = named(&doc, "Poster");
    assert!(matches!(f.kind, NodeKind::Frame));
    assert!(f.clip);
    assert_eq!((f.width, f.height), (400.0, 300.0));
    assert_eq!(at(f), (100.0, 50.0));
    assert_eq!(rgba(&f.fills[0]), (255, 255, 255, 1.0));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn nested_layers_are_placed_relative_to_their_parent() {
    let board = shape(1, "frame", "Board", (100.0, 50.0, 400.0, 300.0));
    let card = shape(2, "rect", "Card", (130.0, 80.0, 50.0, 40.0));
    let inner = shape(3, "frame", "Inner", (200.0, 100.0, 100.0, 100.0));
    let dot = shape(4, "circle", "Dot", (210.0, 120.0, 10.0, 10.0));
    let tree = nest(board, vec![vec![card], nest(inner, vec![vec![dot]])]);
    let (doc, _) = one(tree);
    assert_eq!(at(named(&doc, "Card")), (30.0, 30.0));
    assert_eq!(at(named(&doc, "Inner")), (100.0, 50.0));
    assert_eq!(at(named(&doc, "Dot")), (10.0, 20.0));
    let world = doc.world_transform(named(&doc, "Dot").id).translation();
    assert_eq!((world.x, world.y), (210.0, 120.0));
    // Children are in paint order, the first listed at the bottom.
    let b = named(&doc, "Board");
    assert_eq!(b.children, [named(&doc, "Card").id, named(&doc, "Inner").id]);
}

#[test]
fn a_group_wraps_its_layers() {
    let board = shape(1, "frame", "Board", (0.0, 0.0, 400.0, 300.0));
    let group = shape(2, "group", "Badge", (40.0, 40.0, 160.0, 60.0));
    let a = shape(3, "rect", "Pill", (40.0, 40.0, 160.0, 40.0));
    let b = shape(4, "circle", "Dot", (180.0, 80.0, 20.0, 20.0));
    let (doc, w) = one(nest(board, vec![nest(group, vec![vec![a], vec![b]])]));
    let g = named(&doc, "Badge");
    assert!(matches!(g.kind, NodeKind::Group));
    assert_eq!(g.children.len(), 2);
    assert_eq!(at(g), (40.0, 40.0));
    assert_eq!((g.width, g.height), (160.0, 60.0));
    let dot = doc.world_transform(named(&doc, "Dot").id).translation();
    assert_eq!((dot.x, dot.y), (180.0, 80.0));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn rectangles_keep_their_corner_radii() {
    let r = shape(1, "rect", "Card", (10.0, 20.0, 200.0, 100.0));
    let r = with(with(with(with(r, "r1", json!(4)), "r2", json!(8)), "r3", json!(12)), "r4", json!(16));
    let even = ["r1", "r2", "r3", "r4"].iter().fold(shape(2, "rect", "Even", (0.0, 0.0, 10.0, 10.0)), |s, k| with(s, k, json!(8)));
    let (doc, w) = one(vec![r, even]);
    let n = named(&doc, "Card");
    assert!(matches!(n.kind, NodeKind::Rectangle));
    assert_eq!(n.corners.radii, [4.0, 8.0, 12.0, 16.0]);
    assert_eq!(named(&doc, "Even").corners.radii, [8.0; 4]);
    assert!(has(&w, Severity::Info, "Page 1 / Card", "r1–r4"), "{w:?}");
    assert!(!w.iter().any(|x| x.path.ends_with("Even")), "{w:?}");
}

#[test]
fn circles_become_ellipses() {
    let (doc, _) = one(vec![shape(1, "circle", "Sun", (400.0, 80.0, 120.0, 90.0))]);
    let n = named(&doc, "Sun");
    assert!(matches!(n.kind, NodeKind::Ellipse { .. }));
    assert_eq!((n.width, n.height), (120.0, 90.0));
    assert_eq!(at(n), (400.0, 80.0));
}

#[test]
fn paths_are_read_from_svg_path_data() {
    let p = with(shape(1, "path", "Wedge", (10.0, 10.0, 100.0, 50.0)), "content", json!("M10 10 L110 10 L110 60 Z"));
    let (doc, w) = one(vec![p]);
    let n = named(&doc, "Wedge");
    let NodeKind::Vector { network } = &n.kind else { panic!("not a vector: {:?}", n.kind) };
    assert_eq!(at(n), (10.0, 10.0));
    let pts: Vec<(f64, f64)> = network.vertices.iter().map(|v| (v.x, v.y)).collect();
    assert_eq!(pts, [(0.0, 0.0), (100.0, 0.0), (100.0, 50.0)]);
    assert_eq!(network.segments.len(), 3);
    assert_eq!(network.regions.len(), 1);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn booleans_keep_their_operation_and_operands() {
    let b = with(shape(1, "bool", "Cut", (0.0, 0.0, 100.0, 100.0)), "boolType", json!("difference"));
    let a = shape(2, "rect", "Base", (0.0, 0.0, 100.0, 100.0));
    let c = shape(3, "circle", "Hole", (25.0, 25.0, 50.0, 50.0));
    let (doc, w) = one(nest(b, vec![vec![a], vec![c]]));
    let n = named(&doc, "Cut");
    assert!(matches!(n.kind, NodeKind::BooleanOp { op: BoolOp::Subtract }));
    assert_eq!(n.children, [named(&doc, "Base").id, named(&doc, "Hole").id]);
    assert!(w.is_empty(), "{w:?}");
    for (t, op) in [("union", BoolOp::Union), ("intersection", BoolOp::Intersect), ("exclude", BoolOp::Exclude)] {
        let b = with(shape(1, "bool", "B", (0.0, 0.0, 10.0, 10.0)), "boolType", json!(t));
        let (doc, _) = one(nest(b, vec![vec![shape(2, "rect", "R", (0.0, 0.0, 10.0, 10.0))]]));
        assert!(matches!(named(&doc, "B").kind, NodeKind::BooleanOp { op: o } if o == op));
    }
}

#[test]
fn solid_fills_keep_colour_and_opacity() {
    let r = with(shape(1, "rect", "Box", (0.0, 0.0, 10.0, 10.0)), "fills", json!([fill("#FF5733", 0.5)]));
    let (doc, w) = one(vec![r]);
    assert_eq!(rgba(&named(&doc, "Box").fills[0]), (0xFF, 0x57, 0x33, 0.5));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn gradient_fills_keep_their_stops_and_direction() {
    let g = gradient((0.0, 0.0), (1.0, 1.0), &[("#FF6B6B", 0.0, 1.0), ("#4ECDC4", 1.0, 0.5)]);
    let r = with(shape(1, "rect", "Sunset", (0.0, 0.0, 100.0, 100.0)), "fills", json!([g]));
    let (doc, w) = one(vec![r]);
    let PaintKind::Gradient(g) = &named(&doc, "Sunset").fills[0].kind else { panic!() };
    assert_eq!((g.from, g.to), ((0.0, 0.0), (1.0, 1.0)));
    assert_eq!(g.stops.len(), 2);
    assert_eq!(g.stops[0].color.to_rgba8(), [0xFF, 0x6B, 0x6B, 255]);
    assert_eq!((g.stops[1].position, g.stops[1].color.to_rgba8()), (1.0, [0x4E, 0xCD, 0xC4, 128]));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn opacity_visibility_lock_and_blend_are_kept() {
    let r = shape(1, "rect", "Ghost", (0.0, 0.0, 10.0, 10.0));
    let r = with(with(with(with(r, "opacity", json!(0.25)), "hidden", json!(true)), "blocked", json!(true)), "blendMode", json!("multiply"));
    let g = with(shape(2, "group", "Plain", (0.0, 0.0, 10.0, 10.0)), "blendMode", json!("normal"));
    let tree = [vec![r], nest(g, vec![vec![shape(3, "rect", "In", (0.0, 0.0, 10.0, 10.0))]])].concat();
    let (doc, w) = one(tree);
    let n = named(&doc, "Ghost");
    assert_eq!((n.opacity, n.visible, n.locked, n.blend), (0.25, false, true, BlendMode::Multiply));
    assert_eq!(named(&doc, "Plain").blend, BlendMode::PassThrough);
    assert!(w.is_empty(), "{w:?}");
    let odd = with(shape(1, "rect", "Odd", (0.0, 0.0, 10.0, 10.0)), "blendMode", json!("glow"));
    let (_, w) = one(vec![odd]);
    assert!(has(&w, Severity::Approximated, "Page 1 / Odd", "blend mode"), "{w:?}");
}

/// A 4 × 4 red-to-blue PNG.
fn tiny_png() -> Vec<u8> {
    png(4, 4, |x, _| [255 - (x * 60) as u8, 0, (x * 60) as u8, 255])
}

#[test]
fn image_shapes_fill_with_their_stored_image() {
    let oid = "0039433d-adc8-430d-b2c3-d884dea6e050";
    let img = with(shape(1, "image", "Photo", (0.0, 0.0, 40.0, 40.0)), "metadata", json!({"width": 4, "height": 4, "mtype": "image/png", "id": oid}));
    let (doc, w) = imported(Export::new().object(oid, "png", "image/png", tiny_png()).page(1, "Page 1", json!({}), vec![img]));
    let n = named(&doc, "Photo");
    assert!(matches!(n.kind, NodeKind::Rectangle));
    let PaintKind::Image(p) = &n.fills[0].kind else { panic!() };
    assert_eq!(doc.images.get(&p.image).map(|i| (i.bytes.as_slice() == tiny_png().as_slice(), i.width)), Some((true, 4)));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn images_are_found_through_media_references() {
    let (media, oid) = (uuid(0x501), uuid(0x502));
    let img = with(shape(1, "image", "Photo", (0.0, 0.0, 4.0, 4.0)), "metadata", json!({"width": 4, "height": 4, "mtype": "image/png", "id": media}));
    let reference = json!({"id": media, "name": "photo", "width": 4, "height": 4, "mtype": "image/png", "mediaId": oid});
    let e = Export::new().json(&format!("files/{FILE}/media/{media}.json"), &reference).object(&oid, "png", "image/png", tiny_png());
    let (doc, w) = imported(e.page(1, "Page 1", json!({}), vec![img]));
    assert!(matches!(named(&doc, "Photo").fills[0].kind, PaintKind::Image(_)));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn an_unreadable_image_is_reported() {
    let img = with(shape(1, "image", "Photo", (0.0, 0.0, 4.0, 4.0)), "metadata", json!({"id": uuid(0x600)}));
    let e = Export::new().object(&uuid(0x600), "txt", "text/plain", b"hello".to_vec());
    let (doc, w) = imported(e.page(1, "Page 1", json!({}), vec![img]));
    assert!(named(&doc, "Photo").fills.is_empty());
    assert!(has(&w, Severity::Dropped, "Page 1 / Photo", "not an image"), "{w:?}");
}

#[test]
fn a_main_instance_becomes_a_component() {
    let main = with(shape(10, "frame", "Button", (0.0, 0.0, 200.0, 56.0)), "componentId", json!(uuid(0x7001)));
    let label_bg = shape(11, "rect", "Button background", (0.0, 0.0, 200.0, 56.0));
    let (doc, w) = imported(Export::new().component(1, "Button", 10).page(1, "Page 1", json!({}), nest(main, vec![vec![label_bg]])));
    let n = named(&doc, "Button");
    assert!(matches!(n.kind, NodeKind::Component { .. }));
    assert!(n.clip);
    assert_eq!(n.children.len(), 1);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn a_component_without_its_main_on_a_page_is_reported() {
    let (_, w) = imported(Export::new().component(1, "Ghost", 99).page(1, "Page 1", json!({}), vec![]));
    assert!(has(&w, Severity::Dropped, "", "“Ghost” wasn't imported"), "{w:?}");
}

#[test]
fn library_colours_become_paint_styles() {
    let solid = json!({"id": uuid(0x801), "name": "Primary", "path": "Brand", "color": "#3366FF", "opacity": 1});
    let grad = json!({"id": uuid(0x802), "name": "Sunset Gradient", "gradient": {"type": "linear", "startX": 0, "startY": 0, "endX": 1, "endY": 1,
        "stops": [{"color": "#FF6B6B", "offset": 0, "opacity": 1}, {"color": "#4ECDC4", "offset": 1, "opacity": 1}]}});
    let e = Export::new()
        .json(&format!("files/{FILE}/colors/{}.json", uuid(0x801)), &solid)
        .json(&format!("files/{FILE}/colors/{}.json", uuid(0x802)), &grad)
        .page(1, "Page 1", json!({}), vec![]);
    let (doc, w) = imported(e);
    let names: Vec<&str> = doc.styles.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Brand/Primary", "Sunset Gradient"]);
    assert!(matches!(&doc.styles[1].kind, StyleKind::Paint { paints } if matches!(paints[0].kind, PaintKind::Gradient(_))));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn is_penpot_tells_exports_from_other_files() {
    assert!(is_penpot(&Export::new().page(1, "Page 1", json!({}), vec![]).bytes()));
    let sketch = zip(&[("document.json".into(), br#"{"_class":"document"}"#.to_vec()), ("meta.json".into(), b"{}".to_vec())]);
    let scrim = zip(&[("document.json".into(), br#"{"format":"figcraft","version":1}"#.to_vec())]);
    let other_type = zip(&[("manifest.json".into(), br#"{"type":"penpot/export-other","version":1,"files":[]}"#.to_vec())]);
    for bytes in [sketch, scrim, other_type, b"fig-kiwi\0\0\0\0".to_vec(), b"garbage".to_vec(), Vec::new()] {
        assert!(!is_penpot(&bytes));
    }
}

#[test]
fn preview_is_the_first_thumbnail() {
    let oid = uuid(0x900);
    let thumb = json!({"fileId": FILE, "pageId": uuid(0x9001), "frameId": uuid(1), "tag": "frame", "mediaId": oid});
    let e = Export::new()
        .object(&oid, "png", "image/png", tiny_png())
        .json(&format!("files/{FILE}/thumbnails/frame/{}/{}.json", uuid(0x9001), uuid(1)), &thumb)
        .page(1, "Page 1", json!({}), vec![]);
    assert_eq!(preview(&e.bytes()), Some(tiny_png()));
    assert_eq!(preview(&Export::new().page(1, "Page 1", json!({}), vec![]).bytes()), None);
    assert_eq!(preview(b"garbage"), None);
}
