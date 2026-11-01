//! What the import can't bring across is reported with the layer path and severity.

use super::*;

#[test]
fn prototyping_and_unknown_layers_are_reported_as_dropped() {
    let hotspot = layer("MSImmutableHotspotLayer", 4, "Tap area", frame(0.0, 0.0, 10.0, 10.0));
    let linked = with(
        rectangle(5, "Next", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]),
        "flow",
        json!({"_class": "MSImmutableFlowConnection", "destinationArtboardID": "back", "animationType": 0}),
    );
    let unknown = layer("hologram", 6, "Future", frame(0.0, 0.0, 10.0, 10.0));
    let classless = json!({"name": "Mystery", "frame": frame(0.0, 0.0, 1.0, 1.0)});
    let a = with(artboard(2, "Screen", frame(0.0, 0.0, 100.0, 100.0), vec![hotspot, linked, unknown, classless]), "isFlowHome", json!(true));
    let (doc, w) = one(vec![a]);
    assert!(has(&w, Severity::Dropped, "Page 1 / Screen / Tap area", "Hotspots"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Screen / Next", "prototype link"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Screen / Future", "“hologram”"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Screen / Mystery", "_class"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Screen", "start screen"));
    // What was dropped isn't in the document; the rest is.
    assert!(doc.nodes().all(|n| n.name != "Tap area" && n.name != "Future"));
    assert!(doc.nodes().any(|n| n.name == "Next"));
}

#[test]
fn slices_keep_their_export_settings_and_unsupported_formats_are_reported() {
    let formats = json!({"_class": "exportOptions", "includedLayerIds": [], "layerOptions": 0, "shouldTrim": false, "exportFormats": [
        {"_class": "exportFormat", "absoluteSize": 0, "fileFormat": "png", "name": "@2x", "scale": 2, "visibleScaleType": 0},
        {"_class": "exportFormat", "absoluteSize": 0, "fileFormat": "tiff", "name": "", "scale": 1, "visibleScaleType": 0},
    ]});
    let slice = with(layer("slice", 3, "Icon export", frame(0.0, 0.0, 32.0, 32.0)), "exportOptions", formats);
    let (doc, w) = one(vec![slice]);
    let n = named(&doc, "Icon export");
    assert!(matches!(n.kind, NodeKind::Slice));
    assert_eq!(n.exports.len(), 1);
    assert_eq!(n.exports[0].suffix, "@2x");
    assert_eq!(n.exports[0].size, figcraft_doc::ExportSize::Scale { value: 2.0 });
    assert!(has(&w, Severity::Dropped, "Page 1 / Icon export", "TIFF export format"));
}

#[test]
fn style_features_without_a_match_are_reported() {
    let adjusted = styled(
        with(layer("bitmap", 3, "Adjusted", frame(0.0, 0.0, 10.0, 10.0)), "image", file_ref("images/missing.png")),
        "colorControls",
        json!({"_class": "colorControls", "isEnabled": true, "brightness": 10, "contrast": 0, "hue": 0, "saturation": 0}),
    );
    let mut two = rectangle(4, "Two borders", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4]);
    two["style"]["borders"] = json!([border(color(0.0, 0.0, 0.0, 1.0), 1.0, 0), border(color(1.0, 0.0, 0.0, 1.0), 4.0, 2)]);
    let arrow = styled(
        shape("shapePath", 5, "Arrow", frame(0.0, 0.0, 10.0, 0.0), vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0)], false),
        "endMarkerType",
        json!(5),
    );
    let shared = with(rectangle(6, "Shared", frame(0.0, 0.0, 1.0, 1.0), [0.0; 4]), "sharedStyleID", json!(uuid(900)));
    let orphan = with(rectangle(7, "Orphan", frame(0.0, 0.0, 1.0, 1.0), [0.0; 4]), "sharedStyleID", json!(uuid(901)));
    let pinned = with(rectangle(8, "Pinned", frame(0.0, 0.0, 1.0, 1.0), [0.0; 4]), "resizingConstraint", json!(18));
    let on_path = with(
        text(9, "Curvy", frame(0.0, 0.0, 10.0, 10.0), "hi", vec![(0, 2, attrs("Inter-Regular", 12.0, color(0.0, 0.0, 0.0, 1.0), 0))]),
        "automaticallyDrawOnUnderlyingPath",
        json!(true),
    );
    let styles = json!({"layerStyles": {"_class": "sharedStyleContainer", "objects": [{"_class": "sharedStyle", "do_objectID": uuid(900), "name": "Card/Default", "value": style()}]}});
    let bytes = sketch(&[page(1, "Page 1", vec![adjusted, two, arrow, shared, orphan, pinned, on_path])], styles, &[]);
    let (_, w) = import(&bytes, "Styles").unwrap();
    assert!(has(&w, Severity::Dropped, "Page 1 / Adjusted", "Colour adjustments"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Adjusted", "missing from the file"));
    assert!(has(&w, Severity::Approximated, "Page 1 / Two borders", "different thicknesses"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Arrow", "line ending"));
    assert!(has(&w, Severity::Info, "Page 1 / Shared", "“Card/Default”"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Orphan", "isn't in the file"));
    assert!(has(&w, Severity::Approximated, "Page 1 / Curvy", "Text on a path"));
    assert!(has(&w, Severity::Dropped, "", "1 layer sets resizing constraints"));
}

#[test]
fn layout_and_smart_layout_are_reported() {
    let grid = json!({"_class": "layoutGrid", "isEnabled": true, "columnWidth": 60, "gutterWidth": 20, "numberOfColumns": 12});
    let smart = with(
        container("group", 3, "Stack", frame(0.0, 0.0, 10.0, 10.0), vec![rectangle(4, "Item", frame(0.0, 0.0, 10.0, 10.0), [0.0; 4])]),
        "groupLayout",
        json!({"_class": "MSImmutableInferredGroupLayout", "axis": 0, "layoutAnchor": 0}),
    );
    let a = with(artboard(2, "Grid board", frame(0.0, 0.0, 100.0, 100.0), vec![smart]), "layout", grid);
    let (_, w) = one(vec![a]);
    assert!(has(&w, Severity::Dropped, "Page 1 / Grid board", "layout grid"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Grid board / Stack", "Smart layout"));
}
