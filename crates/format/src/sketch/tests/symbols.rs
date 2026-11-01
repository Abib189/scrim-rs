//! Symbol masters and instances.

use super::*;

/// A button symbol (`symbol` 77) holding a label.
fn button() -> Value {
    let label = text(11, "Label", frame(10.0, 10.0, 80.0, 20.0), "Buy", vec![(0, 3, attrs("Inter-Bold", 14.0, color(1.0, 1.0, 1.0, 1.0), 2))]);
    let bg = styled(rectangle(12, "Background", frame(0.0, 0.0, 100.0, 40.0), [8.0; 4]), "fills", json!([solid(color(0.0, 0.3, 1.0, 1.0))]));
    master(10, 77, "Button", frame(0.0, 0.0, 100.0, 40.0), vec![bg, label])
}

#[test]
fn masters_become_components_and_instances_link_to_them() {
    let inst = instance(20, 77, "Buy button", frame(300.0, 100.0, 100.0, 40.0), vec![]);
    let bytes = sketch(&[page(1, "Screens", vec![inst]), page(2, "Symbols", vec![button()])], json!({}), &[]);
    let (doc, warnings) = import(&bytes, "Symbols").unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let main = named(&doc, "Button");
    assert!(matches!(main.kind, NodeKind::Component { .. }));
    let i = named(&doc, "Buy button");
    assert!(matches!(i.kind, NodeKind::Instance { component, .. } if component == main.id));
    assert_eq!(i.source, Some(main.id));
    assert_eq!(i.transform.translation().to_point(), (300.0, 100.0).into());
    // The instance holds a copy of each of the main's layers, linked to it.
    assert_eq!(i.children.len(), 2);
    for (copy, orig) in i.children.iter().zip(&main.children) {
        let c = doc.node(*copy).unwrap();
        assert_eq!(c.source, Some(*orig));
        assert_eq!(c.name, doc.node(*orig).unwrap().name);
    }
    let label = doc.node(i.children[1]).unwrap();
    let NodeKind::Text(td) = &label.kind else { panic!() };
    assert_eq!(td.characters, "Buy");
}

#[test]
fn text_and_image_overrides_apply_to_the_instance_copy() {
    let png = png(1, 1, |_, _| [255, 0, 0, 255]);
    let photo = with(layer("bitmap", 13, "Avatar", frame(0.0, 0.0, 10.0, 10.0)), "image", file_ref("images/a.png"));
    let mut m = button();
    m["layers"].as_array_mut().unwrap().push(photo);
    let overrides = vec![
        (format!("{}_stringValue", uuid(11)), json!("Sold out")),
        (format!("{}_image", uuid(13)), file_ref("images/b.png")),
        (format!("{}_layerStyle", uuid(12)), json!(uuid(99))),
    ];
    let inst = instance(20, 77, "Sold", frame(0.0, 100.0, 100.0, 40.0), overrides);
    let other = png_other();
    let bytes = sketch(&[page(1, "Page 1", vec![m, inst])], json!({}), &[("images/a.png", &png), ("images/b.png", &other)]);
    let (doc, warnings) = import(&bytes, "Overrides").unwrap();
    let i = named(&doc, "Sold");
    let copies: Vec<&Node> = i.children.iter().map(|c| doc.node(*c).unwrap()).collect();
    let label = copies.iter().find(|n| n.name == "Label").unwrap();
    let NodeKind::Text(td) = &label.kind else { panic!() };
    assert_eq!(td.characters, "Sold out");
    assert!(label.overrides.contains("characters"));
    // The main keeps its own text.
    let NodeKind::Text(main_td) = &doc.nodes().find(|n| n.name == "Label" && n.source.is_none()).unwrap().kind else { panic!() };
    assert_eq!(main_td.characters, "Buy");
    let avatar = copies.iter().find(|n| n.name == "Avatar").unwrap();
    let PaintKind::Image(img) = &avatar.fills[0].kind else { panic!() };
    assert_eq!(doc.images[&img.image].bytes.as_slice(), other.as_slice());
    assert!(has(&warnings, Severity::Dropped, "Page 1 / Sold / Background", "layer style override"));
}

#[test]
fn nested_instances_and_override_paths() {
    let inner = button();
    let card_btn = instance(31, 77, "Card button", frame(0.0, 50.0, 100.0, 40.0), vec![]);
    let card = master(30, 88, "Card", frame(0.0, 0.0, 200.0, 100.0), vec![card_btn]);
    let path = format!("{}/{}_stringValue", uuid(31), uuid(11));
    let inst = instance(40, 88, "My card", frame(500.0, 0.0, 200.0, 100.0), vec![(path, json!("Nested"))]);
    // The instance comes before both mains it needs.
    let (doc, warnings) = one(vec![inst, card, inner]);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let my = named(&doc, "My card");
    let nested = doc.node(my.children[0]).unwrap();
    assert!(matches!(nested.kind, NodeKind::Instance { component, .. } if component == named(&doc, "Button").id));
    let label = doc.subtree(nested.id).into_iter().filter_map(|id| doc.node(id)).find(|n| n.name == "Label").unwrap();
    let NodeKind::Text(td) = &label.kind else { panic!() };
    assert_eq!(td.characters, "Nested");
}

#[test]
fn missing_and_recursive_symbols_are_reported() {
    let lost = instance(20, 404, "Lost", frame(0.0, 0.0, 10.0, 10.0), vec![]);
    let selfish = master(30, 55, "Loop", frame(0.0, 0.0, 10.0, 10.0), vec![instance(31, 55, "Inner loop", frame(0.0, 0.0, 10.0, 10.0), vec![])]);
    let (doc, warnings) = one(vec![lost, selfish]);
    assert!(matches!(named(&doc, "Lost").kind, NodeKind::Frame));
    assert!(has(&warnings, Severity::Dropped, "Page 1 / Lost", "isn't in the file"));
    assert!(matches!(named(&doc, "Inner loop").kind, NodeKind::Frame));
    assert!(has(&warnings, Severity::Dropped, "Page 1 / Loop / Inner loop", "inside its own symbol"));
}

#[test]
fn library_symbols_come_from_foreign_symbols() {
    let foreign = json!([{"_class": "MSImmutableForeignSymbol", "do_objectID": uuid(500), "libraryID": uuid(501), "sourceLibraryName": "UI Kit",
        "symbolPrivate": false, "originalMaster": button(), "symbolMaster": button()}]);
    let inst = instance(20, 77, "From kit", frame(0.0, 0.0, 100.0, 40.0), vec![]);
    let bytes = sketch(&[page(1, "Page 1", vec![inst])], json!({"foreignSymbols": foreign}), &[]);
    let (doc, warnings) = import(&bytes, "Library").unwrap();
    let main = named(&doc, "Button");
    assert_eq!(doc.node(doc.page_of(main.id).unwrap()).unwrap().name, "Library symbols");
    assert!(matches!(named(&doc, "From kit").kind, NodeKind::Instance { component, .. } if component == main.id));
    assert!(has(&warnings, Severity::Info, "Library symbols / Button", "UI Kit"));
}

#[test]
fn resized_instances_are_approximated() {
    let inst = instance(20, 77, "Wide", frame(0.0, 100.0, 300.0, 40.0), vec![]);
    let (doc, warnings) = one(vec![button(), inst]);
    assert!(named(&doc, "Wide").overrides.contains("size"));
    assert!(has(&warnings, Severity::Approximated, "Page 1 / Wide", "resized"));
}

fn png_other() -> Vec<u8> {
    png(1, 1, |_, _| [0, 255, 0, 255])
}
