//! Copies never take a live id (upstream FigCraft 8aa456b1 and 46b02d44, audit #1064 ENG-3): a
//! file whose saved `next_id` is behind the ids it uses must not make a duplicate or a paste
//! overwrite layers that exist.

use std::collections::BTreeMap;

use crate::{Document, Node, NodeId, NodeKind};

/// A frame `F` with one rectangle `A` on the first page, read back from JSON with `next_id` 1,
/// as a file written by a buggy or hand-edited tool would be.
fn stale() -> (Document, NodeId, NodeId, NodeId) {
    let mut d = Document::new("x");
    let p = d.first_page().unwrap();
    let f = d.create(p, NodeKind::Frame, "F", 0.0, 0.0, 100.0, 100.0).unwrap();
    let a = d.create(f, NodeKind::Rectangle, "A", 0.0, 0.0, 10.0, 10.0).unwrap();
    let mut v = serde_json::to_value(&d).unwrap();
    v["next_id"] = serde_json::json!(1);
    (serde_json::from_value(v).unwrap(), p, f, a)
}

fn names(d: &Document, ids: &[NodeId]) -> Vec<(String, Option<NodeId>)> {
    ids.iter().map(|i| d.node(*i).map(|n| (n.name.clone(), n.parent)).unwrap_or_default()).collect()
}

#[test]
fn a_stale_counter_never_hands_out_a_live_id() {
    let (mut d, p, f, a) = stale();
    for _ in 0..4 {
        let id = d.alloc_id();
        assert!(!d.contains(id), "alloc_id handed out {id}, which is in use");
    }
    assert!(![p, f, a].contains(&d.alloc_id()));
}

#[test]
fn duplicates_in_a_file_with_a_stale_counter_keep_every_layer() {
    let (mut d, p, f, a) = stale();
    let before = names(&d, &[p, f, a]);
    let n = d.node_count();
    let copy = d.duplicate_into(f, p, None).unwrap();
    assert!(![p, f, a].contains(&copy), "the copy took id {copy}");
    assert_eq!(d.node_count(), n + 2);
    let more = d.duplicate_all_into(&[f], p).unwrap();
    assert_eq!(d.node_count(), n + 4);
    assert!(more.iter().all(|c| ![p, f, a, copy].contains(c)));
    assert_eq!(names(&d, &[p, f, a]), before, "an original layer changed");
    assert!(d.validate().is_empty(), "{:?}", d.validate());
}

#[test]
fn a_paste_onto_existing_or_repeated_ids_is_refused_and_changes_nothing() {
    let (mut d, p, f, a) = stale();
    let nodes: Vec<Node> = d.subtree(f).into_iter().filter_map(|i| d.node(i).cloned()).collect();
    let snapshot = d.clone();
    // One new id is a layer that exists.
    let mut map: BTreeMap<NodeId, NodeId> = BTreeMap::new();
    map.insert(f, NodeId(9_000));
    map.insert(a, p);
    assert!(d.paste_nodes(&nodes, &map, f, p, None).is_err());
    assert_eq!(d, snapshot);
    assert_eq!(d.node(p).map(|n| n.is_page()), Some(true));
    // Two nodes mapped to the same new id.
    let mut map: BTreeMap<NodeId, NodeId> = BTreeMap::new();
    map.insert(f, NodeId(9_001));
    map.insert(a, NodeId(9_001));
    assert!(d.paste_nodes(&nodes, &map, f, p, None).is_err());
    assert_eq!(d, snapshot);
}
