//! FREE import tests. Every file is built in code from the README (see `builder`).

mod builder;
mod components;
mod detect;
mod layout;
mod sample;
mod style;
mod text;
mod unsupported;

use builder::*;
use figcraft_doc::{BoolOp, Document, HConstraint, MaskType, Node, NodeKind, VConstraint};
use serde_json::{Value, json};

use super::{import, is_free};
use crate::import_report::{ImportWarning, Severity};

/// Import a file of one page holding `layers` (the document must be structurally valid).
fn one(layers: Vec<Value>) -> (Document, Vec<ImportWarning>) {
    let (doc, warnings) = import(&free(&[page(1, "Page 1", layers)], json!({}), &[]), "Test").unwrap();
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

/// The translation of `n`'s transform (its position in its parent).
fn pos(n: &Node) -> (f64, f64) {
    let c = n.transform.as_coeffs();
    (c[4], c[5])
}

#[test]
fn frames_nest_and_are_placed_in_their_parent() {
    let card = with(layer("RECT", 3, "Card", [10.0, 20.0, 50.0, 40.0]), "fill", json!("F00"));
    let inner = container("FRAME", 2, "Inner", [100.0, 50.0, 200.0, 150.0], vec![card]);
    let outer = with(container("FRAME", 1, "Outer", [30.0, 40.0, 400.0, 300.0], vec![inner]), "clipContent", json!(false));
    let (doc, w) = one(vec![outer]);
    let (o, i, c) = (named(&doc, "Outer"), named(&doc, "Inner"), named(&doc, "Card"));
    assert!(matches!(o.kind, NodeKind::Frame) && matches!(i.kind, NodeKind::Frame));
    assert_eq!(i.parent, Some(o.id));
    assert_eq!(c.parent, Some(i.id));
    assert_eq!(pos(o), (30.0, 40.0));
    assert_eq!(pos(i), (100.0, 50.0));
    assert_eq!(pos(c), (10.0, 20.0));
    assert_eq!((c.width, c.height), (50.0, 40.0));
    assert!(!o.clip && i.clip, "clipContent defaults to true");
    let world = doc.world_transform(c.id).as_coeffs();
    assert_eq!((world[4], world[5]), (140.0, 110.0));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn pos_size_and_transform_place_a_layer() {
    let a = json!({"_t": "RECT", "id": gid(1), "name": "A", "pos": [5, 6], "size": [30, 20]});
    let b = json!({"_t": "RECT", "id": gid(2), "name": "B", "transform": [0, -1, 50, 1, 0, 60], "size": [10, 10]});
    let c = json!({"_t": "RECT", "id": gid(3), "name": "C"});
    let (doc, _) = one(vec![a, b, c]);
    let a = named(&doc, "A");
    assert_eq!((pos(a), a.width, a.height), ((5.0, 6.0), 30.0, 20.0));
    // `[scaleX, skewX, transX, skewY, scaleY, transY]`: the local x axis maps to (0, 1).
    let t = named(&doc, "B").transform;
    let p = t * figcraft_doc::geom::Point::new(1.0, 0.0);
    assert!(close(p.x, 50.0) && close(p.y, 61.0), "{p:?}");
    let c = named(&doc, "C");
    assert_eq!((pos(c), c.width, c.height), ((0.0, 0.0), 100.0, 100.0), "the README's defaults");
}

#[test]
fn rectangles_keep_their_corner_radii() {
    let uniform = with(layer("RECT", 1, "Uniform", [0.0, 0.0, 100.0, 50.0]), "cornerRadius", json!([12, 12, 12, 12]));
    let points = json!([[0, 0, 0, 4], [1, 0, 0, 8], [1, 1], [0, 1, 0, 16]]);
    let corners = with(layer("RECT", 2, "Corners", [0.0, 0.0, 100.0, 50.0]), "points", points);
    let frame = with(layer("FRAME", 3, "Rounded frame", [0.0, 0.0, 100.0, 50.0]), "cornerRadius", json!([1, 2, 3, 4]));
    let (doc, w) = one(vec![uniform, corners, frame]);
    assert_eq!(named(&doc, "Uniform").corners.radii, [12.0; 4]);
    assert!(matches!(named(&doc, "Corners").kind, NodeKind::Rectangle));
    assert_eq!(named(&doc, "Corners").corners.radii, [4.0, 8.0, 0.0, 16.0]);
    assert_eq!(named(&doc, "Rounded frame").corners.radii, [1.0, 2.0, 3.0, 4.0]);
    assert!(has(&w, Severity::Approximated, "Page 1 / Rounded frame", "corner"), "{w:?}");
}

#[test]
fn shapes_become_their_kinds() {
    let oval = layer("OVAL", 1, "Oval", [0.0, 0.0, 80.0, 40.0]);
    let star = with(with(layer("STAR", 2, "Star", [0.0, 0.0, 50.0, 50.0]), "rays", json!(6)), "ratio", json!(0.4));
    let polygon = with(layer("POLYGON", 3, "Polygon", [0.0, 0.0, 50.0, 50.0]), "rays", json!(5));
    let triangle = layer("TRIANGLE", 4, "Triangle", [0.0, 0.0, 50.0, 50.0]);
    let wave = with(layer("PATH", 5, "Wave", [0.0, 0.0, 100.0, 50.0]), "points", json!([[0, 1], [0.5, 0, 1, 0, 0.75, 0, 0.25, 0], [1, 1]]));
    let wave = with(wave, "open", json!(true));
    let line = with(layer("LINE", 6, "Line", [0.0, 0.0, 100.0, 0.0]), "points", json!([[0, 0], [1, 0]]));
    let (doc, w) = one(vec![oval, star, polygon, triangle, wave, line]);
    assert!(matches!(named(&doc, "Oval").kind, NodeKind::Ellipse { .. }));
    assert!(matches!(named(&doc, "Star").kind, NodeKind::Star { count: 6, inner_ratio } if close(inner_ratio, 0.4)));
    assert!(matches!(named(&doc, "Polygon").kind, NodeKind::Polygon { count: 5 }));
    assert!(matches!(named(&doc, "Triangle").kind, NodeKind::Polygon { count: 3 }));
    let NodeKind::Vector { network } = &named(&doc, "Wave").kind else { panic!("Wave is not a path") };
    assert_eq!((network.vertices.len(), network.segments.len(), network.regions.len()), (3, 2, 0));
    let mid = &network.vertices[1];
    assert!(close(mid.x, 50.0) && close(mid.y, 0.0));
    // The first segment arrives at the middle point along its `to` handle, the second leaves along `from`.
    assert_eq!(network.segments[0].tb, (-25.0, 0.0));
    assert_eq!(network.segments[1].ta, (25.0, 0.0));
    let NodeKind::Vector { network } = &named(&doc, "Line").kind else { panic!("Line is not a path") };
    assert_eq!((network.vertices.len(), network.segments.len()), (2, 1));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn groups_shapes_sections_and_slices() {
    let a = with(layer("RECT", 2, "A", [0.0, 0.0, 10.0, 10.0]), "fill", json!("F00"));
    let b = with(with(layer("OVAL", 3, "B", [5.0, 5.0, 10.0, 10.0]), "fill", json!("F00")), "boolOp", json!(1));
    let shape = with(container("SHAPE", 1, "Cutout", [100.0, 100.0, 15.0, 15.0], vec![a, b]), "fill", json!("0F0"));
    let dot = layer("OVAL", 5, "Dot", [20.0, 30.0, 10.0, 10.0]);
    let group = with(container("GROUP", 4, "Group", [50.0, 50.0, 30.0, 40.0], vec![dot]), "fill", json!("F00"));
    let section = container("SECTION", 6, "Section", [0.0, 500.0, 300.0, 200.0], vec![]);
    let slice = layer("SLICE", 7, "Slice", [0.0, 0.0, 64.0, 64.0]);
    let (doc, w) = one(vec![shape, group, section, slice]);
    assert!(matches!(named(&doc, "Cutout").kind, NodeKind::BooleanOp { op: BoolOp::Union }));
    assert_eq!(named(&doc, "Cutout").fills.len(), 1, "a shape's fill paints its combined outline");
    assert!(named(&doc, "Group").fills.is_empty());
    assert!(has(&w, Severity::Dropped, "Page 1 / Group", "groups"), "{w:?}");
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(named(&doc, "B").bool_op, Some(BoolOp::Subtract));
    assert!(matches!(named(&doc, "Group").kind, NodeKind::Group));
    let dot = doc.world_transform(named(&doc, "Dot").id).as_coeffs();
    assert_eq!((dot[4], dot[5]), (70.0, 80.0), "a group's children are relative to it");
    assert!(matches!(named(&doc, "Section").kind, NodeKind::Section { .. }));
    assert!(matches!(named(&doc, "Slice").kind, NodeKind::Slice));
}

#[test]
fn opacity_blend_visibility_locking_masks_and_constraints() {
    let r = layer("RECT", 1, "R", [0.0, 0.0, 10.0, 10.0]);
    let r = with(with(with(with(r, "opacity", json!(0.4)), "blendMode", json!(2)), "hidden", json!(true)), "locked", json!(true));
    let r = with(with(r, "constraints", json!("LRTH")), "lockAspect", json!(true));
    let m = with(with(layer("OVAL", 2, "M", [0.0, 0.0, 10.0, 10.0]), "mask", json!(true)), "maskType", json!(2));
    let (doc, w) = one(vec![r, m]);
    let r = named(&doc, "R");
    assert!(close(f64::from(r.opacity), 0.4));
    assert_eq!(r.blend, figcraft_doc::color::BlendMode::Multiply);
    assert!(!r.visible && r.locked && r.constrain_proportions);
    assert_eq!((r.constraints.horizontal, r.constraints.vertical), (HConstraint::LeftRight, VConstraint::Top));
    let m = named(&doc, "M");
    assert!(m.is_mask && m.mask_type == MaskType::Luminance);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn exports_custom_properties_and_ruler_guides() {
    let f = layer("FRAME", 1, "F", [0.0, 0.0, 100.0, 100.0]);
    let f = with(f, "export", json!([{"format": "SVG"}, {"format": "PNG", "type": 1, "size": 512, "name": "@w"}]));
    let f = with(f, "custom", json!({"plugin.key": "value"}));
    let f = with(f, "rulers", json!({"guides": [10, 20], "vGuides": [5]}));
    let (doc, w) = one(vec![f]);
    let f = named(&doc, "F");
    assert_eq!(f.exports.len(), 2);
    assert_eq!(f.exports[0].format, figcraft_doc::ExportFormat::Svg);
    assert_eq!(f.exports[1].size, figcraft_doc::ExportSize::Width { value: 512.0 });
    assert_eq!(f.exports[1].suffix, "@w");
    assert_eq!(f.plugin_data.get("plugin.key").map(String::as_str), Some("value"));
    assert_eq!(f.guides.len(), 3);
    assert!(f.guides[0].vertical && close(f.guides[0].offset, 10.0) && !f.guides[2].vertical);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn pages_follow_the_document_order_with_their_backgrounds() {
    let p1 = with(page(1, "First", vec![]), "background", json!("FF202020"));
    let p2 = page(2, "Second", vec![layer("RECT", 3, "R", [0.0, 0.0, 1.0, 1.0])]);
    let bytes = free(&[p1, p2], json!({"pages": [gid(2), gid(1)], "currentPageIndex": 1}), &[]);
    let (doc, w) = import(&bytes, "Test").unwrap();
    let names: Vec<&str> = doc.pages.iter().map(|p| doc.node(*p).unwrap().name.as_str()).collect();
    assert_eq!(names, ["Second", "First"]);
    assert_eq!(doc.current_page, Some(doc.pages[1]));
    let NodeKind::Page { background, .. } = &doc.node(doc.pages[1]).unwrap().kind else { panic!() };
    assert_eq!(background.to_rgba8(), [0x20, 0x20, 0x20, 0xFF]);
    assert!(w.is_empty(), "{w:?}");
}
