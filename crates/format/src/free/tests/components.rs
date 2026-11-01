//! Components, instances, overrides and components from shared libraries.

use figcraft_doc::color::PaintKind;

use super::*;

/// A "Button" component (id 10, componentId `gid(100)`): a background and a label.
fn button() -> Value {
    let bg = with(layer("RECT", 11, "Background", [0.0, 0.0, 120.0, 40.0]), "fill", json!("222"));
    let label = text_layer(12, "Label", [10.0, 10.0, 100.0, 20.0], "Click", "Inter-Medium", 14.0, "FFF");
    let c = container("COMPONENT", 10, "Button", [0.0, 0.0, 120.0, 40.0], vec![bg, label]);
    let c = with(c, "autoLayout", json!({"spacing": 4, "padding": [10], "fixWidth": true, "fixHeight": true}));
    with(c, "componentId", json!(gid(100)))
}

fn instance(id: u32, name: &str, frame: [f64; 4], overrides: Value) -> Value {
    with(with(layer("INSTANCE", id, name, frame), "componentId", json!(gid(100))), "overrides", overrides)
}

fn subtree_named<'a>(doc: &'a Document, root: &Node, name: &str) -> &'a Node {
    doc.subtree(root.id).into_iter().filter_map(|i| doc.node(i)).find(|n| n.name == name).unwrap_or_else(|| panic!("no {name} under {}", root.name))
}

#[test]
fn instances_link_to_their_component_with_overrides() {
    let overrides = json!([
        {"target": [gid(12)], "text": "Buy now"},
        {"target": [gid(11)], "fills": [{"color": "0A0"}]},
        {"target": [gid(12)], "hidden": true, "opacity": 0.5}
    ]);
    let inst = instance(20, "Buy", [200.0, 0.0, 120.0, 40.0], overrides);
    let (doc, w) = one(vec![button(), inst]);
    let main = named(&doc, "Button");
    assert!(matches!(main.kind, NodeKind::Component { .. }));
    let buy = named(&doc, "Buy");
    assert!(matches!(buy.kind, NodeKind::Instance { component, .. } if component == main.id));
    assert_eq!(buy.layout, main.layout, "the instance has its component's auto layout");
    assert_eq!(pos(buy), (200.0, 0.0));
    let label = subtree_named(&doc, buy, "Label");
    assert!(matches!(&label.kind, NodeKind::Text(t) if t.characters == "Buy now"));
    assert!(!label.visible && close(f64::from(label.opacity), 0.5));
    assert!(label.overrides.contains("characters") && label.overrides.contains("visible"));
    assert_eq!(label.source, Some(named(&doc, "Label").id), "the copy mirrors the main's layer");
    let bg = subtree_named(&doc, buy, "Background");
    let PaintKind::Solid { color } = &bg.fills[0].kind else { panic!() };
    assert_eq!(color.to_rgba8(), [0, 0xAA, 0, 255]);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn an_instance_of_a_missing_component_is_an_empty_frame() {
    let inst = with(layer("INSTANCE", 20, "Lost", [0.0, 0.0, 10.0, 10.0]), "componentId", json!(gid(555)));
    let (doc, w) = one(vec![inst]);
    assert!(matches!(named(&doc, "Lost").kind, NodeKind::Frame));
    assert!(has(&w, Severity::Dropped, "Page 1 / Lost", "isn't in the file"), "{w:?}");
}

#[test]
fn components_from_shared_libraries_are_imported_for_their_instances() {
    let library = json!({"id": gid(7000), "name": "Kit", "components": [button()]});
    let inst = instance(20, "From kit", [0.0, 0.0, 120.0, 40.0], json!([]));
    let bytes = free(&[page(1, "Page 1", vec![inst])], json!({}), &[("shared/kit.json", &serde_json::to_vec(&library).unwrap())]);
    let (doc, w) = import(&bytes, "Test").unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let main = named(&doc, "Button");
    assert!(matches!(named(&doc, "From kit").kind, NodeKind::Instance { component, .. } if component == main.id));
    assert!(has(&w, Severity::Info, "Library components / Button", "Kit"), "{w:?}");
}

#[test]
fn unsupported_overrides_are_reported_on_their_layer() {
    let overrides = json!([{"target": [gid(12)], "locked": true, "fontSize": 3}, {"target": [gid(999)], "hidden": true}]);
    let (_, w) = one(vec![button(), instance(20, "Buy", [0.0, 0.0, 120.0, 40.0], overrides)]);
    assert!(has(&w, Severity::Dropped, "Page 1 / Buy / Label", "`locked`"), "{w:?}");
    assert!(has(&w, Severity::Dropped, "Page 1 / Buy", "doesn't have"), "{w:?}");
}
