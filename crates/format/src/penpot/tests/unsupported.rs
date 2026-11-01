//! What the documentation doesn't describe: each becomes a warning with the layer's path.

use super::*;

/// The warnings for one rectangle "Box" on "Page 1" with `key` set to `value`.
fn box_with(key: &str, value: Value) -> Vec<ImportWarning> {
    one(vec![with(shape(1, "rect", "Box", (0.0, 0.0, 10.0, 10.0)), key, value)]).1
}

#[test]
fn text_layers_are_reported_not_invented() {
    let t = with(shape(1, "text", "Headline", (0.0, 0.0, 200.0, 40.0)), "content", json!({"type": "root", "children": []}));
    let board = shape(2, "frame", "Board", (0.0, 0.0, 400.0, 300.0));
    let (doc, w) = one(nest(board, vec![vec![t]]));
    assert!(doc.nodes().all(|n| n.name != "Headline"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Board / Headline", "text content"), "{w:?}");
}

#[test]
fn undescribed_styles_are_reported_with_their_layer() {
    let cases = [
        ("strokes", json!([{"strokeColor": "#000000"}]), "Strokes"),
        ("shadow", json!([{"style": "drop-shadow"}]), "Shadows"),
        ("blur", json!({"value": 4}), "blur"),
        ("interactions", json!([{"eventType": "click"}]), "Prototype interactions"),
        ("exports", json!([{"type": "png", "scale": 1}]), "Export settings"),
        ("grids", json!([{"type": "square"}]), "Layout grids"),
        ("appliedTokens", json!({"fill": "brand.primary"}), "design tokens"),
        ("pluginData", json!({"plugin": {"k": "v"}}), "Plugin data"),
        ("layoutFlexDir", json!("row"), "Layout settings"),
    ];
    for (key, value, needle) in cases {
        let w = box_with(key, value);
        assert!(has(&w, Severity::Dropped, "Page 1 / Box", needle), "{key}: {w:?}");
    }
    // Empty lists carry nothing and aren't reported.
    assert!(box_with("strokes", json!([])).is_empty());
}

#[test]
fn constraints_and_unknown_attributes_are_reported_for_the_file() {
    let w = box_with("constraintsH", json!("left"));
    assert!(has(&w, Severity::Dropped, "", "resizing constraints"), "{w:?}");
    assert!(w.iter().any(|x| x.message.contains("Page 1 / Box")), "{w:?}");
    let w = box_with("proportionLock", json!(true));
    assert!(has(&w, Severity::Dropped, "", "proportionLock (1)"), "{w:?}");
}

#[test]
fn an_instance_keeps_its_layers_as_plain_layers() {
    let main = shape(10, "frame", "Button", (0.0, 0.0, 200.0, 56.0));
    let copy = with(shape(20, "frame", "Button copy", (0.0, 100.0, 200.0, 56.0)), "componentId", json!(uuid(0x7001)));
    let copy_bg = with(shape(21, "rect", "Bg", (0.0, 100.0, 200.0, 56.0)), "shapeRef", json!(uuid(11)));
    let shapes = [vec![main], nest(copy, vec![vec![copy_bg]])].concat();
    let (doc, w) = imported(Export::new().component(1, "Button", 10).page(1, "Page 1", json!({}), shapes));
    let copy = named(&doc, "Button copy");
    assert!(matches!(copy.kind, NodeKind::Frame));
    assert_eq!(copy.children.len(), 1);
    assert!(has(&w, Severity::Approximated, "Page 1 / Button copy", "An instance of “Button” was imported as plain layers"), "{w:?}");
    assert!(has(&w, Severity::Info, "", "shapeRef"), "{w:?}");
}

#[test]
fn raw_svg_is_reported() {
    let (doc, w) = one(vec![shape(1, "svg-raw", "Logo", (0.0, 0.0, 10.0, 10.0))]);
    assert!(doc.nodes().all(|n| n.name != "Logo"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Logo", "raw SVG"), "{w:?}");
}

#[test]
fn a_rotated_layer_is_placed_without_its_transform() {
    let w = box_with("transform", json!([0, 1, -1, 0, 0, 0]));
    assert!(has(&w, Severity::Approximated, "Page 1 / Box", "transform"), "{w:?}");
    let w = box_with("transform", json!({"a": 1}));
    assert!(has(&w, Severity::Approximated, "", "transform"), "{w:?}");
}

#[test]
fn a_second_file_is_reported() {
    let mut e = Export::new().page(1, "Page 1", json!({}), vec![]);
    e.manifest["files"].as_array_mut().unwrap().push(json!({"id": uuid(0xF2), "name": "Other file", "features": []}));
    let (_, w) = imported(e);
    assert!(has(&w, Severity::Dropped, "", "only the first, “Test file”"), "{w:?}");
}

#[test]
fn page_level_and_library_extras_are_reported() {
    let e = Export::new()
        .json(&format!("files/{FILE}/typographies/{}.json", uuid(1)), &json!({"id": uuid(1), "name": "Body", "fontFamily": "Inter"}))
        .json(&format!("files/{FILE}/tokens.json"), &json!({"sets": {}}))
        .page(1, "Page 1", json!({"flows": {"f": {"name": "Flow"}}, "guides": {"g": {"position": 10}}}), vec![]);
    let (_, w) = imported(e);
    assert!(has(&w, Severity::Dropped, "", "typography"), "{w:?}");
    assert!(has(&w, Severity::Dropped, "", "tokens.json"), "{w:?}");
    assert!(has(&w, Severity::Dropped, "Page 1", "Prototype flows"), "{w:?}");
    assert!(has(&w, Severity::Dropped, "Page 1", "Ruler guides"), "{w:?}");
}

#[test]
fn undescribed_fills_are_reported() {
    let w = box_with("fills", json!([{"fillColor": "#FF0000", "fillOpacity": 1}]));
    assert!(has(&w, Severity::Dropped, "Page 1 / Box", "fillColor"), "{w:?}");
    let w = box_with("fills", json!([{"image": {"id": "x"}}]));
    assert!(has(&w, Severity::Dropped, "Page 1 / Box", "image fill"), "{w:?}");
    let w = box_with("fills", json!([{"color": "red-ish"}]));
    assert!(has(&w, Severity::Dropped, "Page 1 / Box", "hex"), "{w:?}");
}

#[test]
fn path_content_that_is_not_svg_data_is_reported() {
    let p = with(shape(1, "path", "Squiggle", (0.0, 0.0, 10.0, 10.0)), "content", json!([{"command": "move-to"}]));
    let (doc, w) = one(vec![p]);
    assert!(matches!(&named(&doc, "Squiggle").kind, NodeKind::Vector { network } if network.segments.is_empty()));
    assert!(has(&w, Severity::Dropped, "Page 1 / Squiggle", "isn't SVG path data"), "{w:?}");
    let p = with(shape(1, "path", "Broken", (0.0, 0.0, 10.0, 10.0)), "content", json!("M 0 0 Q"));
    let (_, w) = one(vec![p]);
    assert!(has(&w, Severity::Dropped, "Page 1 / Broken", "couldn't be read"), "{w:?}");
}
