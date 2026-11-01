//! A file whose saved `next_id` is behind the ids it uses (upstream FigCraft 8aa456b1, audit #1064
//! ENG-3) still opens as it is, and copying inside it never overwrites a layer.

use figcraft_doc::{Document, NodeKind};

use crate::{from_json, load_bytes, save_bytes, to_json};

#[test]
fn a_crafted_stale_counter_file_opens_and_copies_keep_every_layer() {
    let mut d = Document::new("Stale");
    let p = d.first_page().unwrap();
    let f = d.create(p, NodeKind::Frame, "Card", 0.0, 0.0, 200.0, 120.0).unwrap();
    let t = d.create(f, NodeKind::Rectangle, "Badge", 8.0, 8.0, 40.0, 20.0).unwrap();
    let mut json = to_json(&d).unwrap();
    json["next_id"] = serde_json::json!(1);
    // The document as a file holds it: opens with no issues and every layer where it was.
    let (opened, issues) = from_json(&json.to_string()).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
    let (mut opened, _) = load_bytes(&save_bytes(&opened, None).unwrap()).unwrap();
    assert_eq!(opened.node(t).map(|n| (n.name.as_str(), n.parent)), Some(("Badge", Some(f))));
    let n = opened.node_count();
    let copy = opened.duplicate_into(f, p, None).unwrap();
    assert!(![p, f, t].contains(&copy), "the copy took id {copy}");
    assert_eq!(opened.node_count(), n + 2);
    assert_eq!(opened.node(t).map(|n| (n.name.as_str(), n.parent)), Some(("Badge", Some(f))));
    assert!(opened.validate().is_empty(), "{:?}", opened.validate());
}
