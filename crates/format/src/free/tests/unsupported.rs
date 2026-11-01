//! What isn't brought across is reported on its layer, with its severity.

use super::*;

#[test]
fn each_unsupported_feature_is_reported() {
    let base = |id: u32, name: &str| layer("RECT", id, name, [0.0, 0.0, 10.0, 10.0]);
    let flows = with(base(1, "Linked"), "flows", json!([{"trigger": {"_t": "CLICK"}, "actions": [{"_t": "BACK"}]}]));
    let binds = with(base(2, "Bound"), "binds", json!([{"field": 2, "numberId": gid(300)}]));
    let themes = with(base(3, "Themed"), "themes", json!([{"id": gid(301), "themeId": gid(302)}]));
    let tint = with(base(4, "Tinted"), "tint", json!({"color": "F00"}));
    let filters = with(base(5, "Filtered"), "imageFilters", json!({"enabled": true, "hue": 0.3}));
    let unknown = with(base(6, "Future"), "hologram", json!(true));
    let connector = layer("CONNECTOR", 7, "Arrow", [0.0, 0.0, 10.0, 10.0]);
    let alien = layer("PORTAL", 8, "Portal", [0.0, 0.0, 10.0, 10.0]);
    let grid = with(layer("FRAME", 9, "Gridded", [0.0, 0.0, 10.0, 10.0]), "layouts", json!([{"_t": "GRID", "size": 8}]));
    let home = with(with(layer("FRAME", 10, "Home", [0.0, 0.0, 10.0, 10.0]), "flowHome", json!(true)), "viewport", json!({"size": [375, 812]}));
    let props = with(
        with(layer("COMPONENT", 11, "Toggle", [0.0, 0.0, 10.0, 10.0]), "componentId", json!(gid(303))),
        "properties",
        json!([{"_t": "BOOL", "id": gid(304), "name": "On"}]),
    );
    let states = layer("STATES", 12, "Variants", [0.0, 0.0, 10.0, 10.0]);
    let donut = with(layer("OVAL", 13, "Donut", [0.0, 0.0, 10.0, 10.0]), "ratio", json!(0.5));
    let sticky = with(base(14, "Sticky"), "scrollBehavior", json!(2));
    let gallery = with(base(15, "Photo"), "photoId", json!("abc"));
    let rtl = with(with(layer("TEXT", 16, "Arabic", [0.0, 0.0, 10.0, 10.0]), "text", json!("x")), "rtl", json!(true));
    let layers = vec![flows, binds, themes, tint, filters, unknown, connector, alien, grid, home, props, states, donut, sticky, gallery, rtl];
    let (_, w) = one(layers);
    let expect = [
        (Severity::Dropped, "Page 1 / Linked", "prototype interactions"),
        (Severity::Dropped, "Page 1 / Bound", "variable binding"),
        (Severity::Dropped, "Page 1 / Themed", "variable theme"),
        (Severity::Dropped, "Page 1 / Tinted", "tint"),
        (Severity::Dropped, "Page 1 / Filtered", "Image filters"),
        (Severity::Dropped, "Page 1 / Future", "`hologram`"),
        (Severity::Dropped, "Page 1 / Arrow", "Connectors"),
        (Severity::Dropped, "Page 1 / Portal", "PORTAL"),
        (Severity::Dropped, "Page 1 / Gridded", "layout grids"),
        (Severity::Dropped, "Page 1 / Home", "starting point"),
        (Severity::Dropped, "Page 1 / Home", "viewport"),
        (Severity::Dropped, "Page 1 / Toggle", "properties"),
        (Severity::Approximated, "Page 1 / Variants", "states"),
        (Severity::Approximated, "Page 1 / Donut", "donut"),
        (Severity::Dropped, "Page 1 / Sticky", "scroll behaviour"),
        (Severity::Info, "Page 1 / Photo", "gallery"),
        (Severity::Dropped, "Page 1 / Arabic", "Right-to-left"),
    ];
    for (severity, path, needle) in expect {
        assert!(has(&w, severity, path, needle), "missing {severity:?} {path}: {needle}\n{w:#?}");
    }
}

#[test]
fn document_parts_not_imported_are_reported() {
    let extra = json!({
        "variableCollections": [{"id": gid(1), "name": "Theme", "variables": [{"_t": "COLOR", "id": gid(2), "name": "Brand"}]}],
        "colorStyles": [{"id": gid(3), "name": "Brand / Primary", "fill": "F00"}],
        "fonts": [{"data": "x.ttf", "name": "X"}],
    });
    let styled = with(layer("RECT", 4, "Styled", [0.0, 0.0, 10.0, 10.0]), "fillsId", json!(gid(3)));
    let (_, w) = import(&free(&[page(1, "Page 1", vec![styled])], extra, &[]), "x").unwrap();
    assert!(has(&w, Severity::Dropped, "", "variables (1 collections, 1 variables)"), "{w:?}");
    assert!(has(&w, Severity::Dropped, "", "shared styles"), "{w:?}");
    assert!(has(&w, Severity::Dropped, "", "fonts embedded"), "{w:?}");
    assert!(has(&w, Severity::Info, "Page 1 / Styled", "“Brand / Primary”"), "{w:?}");
}
