//! Fix AI C-6 (upstream FigCraft 646ac60b, #1672): the check that numbers stay finite looks at the
//! layers an edit changed, not at every layer of the file, so an edit costs about its own size.
//! Counted, not timed: the check counts the layers it looks at.

use serde_json::json;

use crate::Session;

/// A settled document of `n` rectangles, and one of them.
fn large(n: usize) -> (figcraft_doc::Document, crate::NodeId) {
    let mut doc = figcraft_doc::Document::new("Large");
    let page = doc.first_page().unwrap();
    let mut last = None;
    for i in 0..n {
        last =
            Some(doc.create(page, figcraft_doc::NodeKind::Rectangle, "rect", (i % 100) as f64 * 20.0, (i / 100) as f64 * 20.0, 10.0, 10.0).unwrap());
    }
    doc.take_dirty();
    (doc, last.unwrap())
}

#[test]
fn an_edit_checks_only_the_layers_it_changed() {
    let (doc, id) = large(20_000);
    let mut s = Session::with_document(doc);
    s.execute("node.set", &json!({"ids": [id], "props": {"name": "warm"}})).unwrap();
    super::EXAMINED.with(|c| c.set(0));
    s.execute("node.set", &json!({"ids": [id], "props": {"name": "renamed"}})).unwrap();
    let examined = super::EXAMINED.with(std::cell::Cell::get);
    assert_eq!(s.doc().unwrap().node(id).unwrap().name, "renamed");
    assert!(examined <= 4, "a rename of one layer among 20,000 looked at {examined} layers");
}

#[test]
fn a_layer_already_out_of_range_does_not_block_edits_to_others() {
    // A damaged file can hold a layer with a size that isn't finite; edits to other layers go
    // through, and an edit that touches the damaged layer is refused.
    let (mut doc, id) = large(6);
    let page = doc.first_page().unwrap();
    let broken = doc.create(page, figcraft_doc::NodeKind::Rectangle, "broken", 0.0, 0.0, 10.0, 10.0).unwrap();
    doc.node_mut(broken).unwrap().width = f64::NAN;
    doc.take_dirty();
    let mut s = Session::with_document(doc);
    assert!(s.execute("node.set", &json!({"ids": [id], "props": {"name": "renamed"}})).is_ok());
    let err = s.execute("transform.move", &json!({"ids": [broken], "dx": 5, "dy": 0})).expect_err("refused").to_string();
    assert!(err.contains("broken"), "{err}");
}
