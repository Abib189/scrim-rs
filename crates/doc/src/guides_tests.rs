//! A page's ruler guides survive a JSON round trip (Fix AI LW-1, upstream FigCraft 652bf35b). A
//! page used to keep its guides in `NodeKind::Page` while every layer also has a `guides` field;
//! both were written as `guides`, the layer's field took the key on reading, so a reopened page
//! lost its guides, and saving it again wrote the key twice, which didn't open.

use crate::{Document, Guide, Node, NodeKind};

fn guide(vertical: bool, offset: f64) -> Guide {
    Guide { vertical, offset }
}

fn round_trip(n: &Node) -> Node {
    serde_json::from_value(serde_json::to_value(n).unwrap()).unwrap()
}

#[test]
fn a_pages_guides_survive_a_round_trip() {
    let mut d = Document::new("Guides");
    let page = d.first_page().unwrap();
    d.node_mut(page).unwrap().guides_mut().push(guide(true, 120.0));
    let back = round_trip(d.node(page).unwrap());
    assert_eq!(back.guides(), [guide(true, 120.0)]);
    assert!(matches!(back.kind, NodeKind::Page { .. }));
    // Written once, under the same key as before.
    let json = serde_json::to_string(d.node(page).unwrap()).unwrap();
    assert_eq!(json.matches("\"guides\"").count(), 1, "{json}");
}

#[test]
fn a_frames_guides_survive_a_round_trip_and_none_are_written_without_any() {
    let mut d = Document::new("Guides");
    let page = d.first_page().unwrap();
    let f = d.create(page, NodeKind::Frame, "F", 0.0, 0.0, 100.0, 100.0).unwrap();
    assert!(!serde_json::to_string(d.node(f).unwrap()).unwrap().contains("guides"));
    d.node_mut(f).unwrap().guides_mut().push(guide(false, 30.0));
    assert_eq!(round_trip(d.node(f).unwrap()).guides(), [guide(false, 30.0)]);
}

/// A page saved after a reopen wrote `guides` twice: the page kind's list (new guides) and then
/// the layer's (the ones read back). Both open, in file order.
#[test]
fn a_page_written_with_the_key_twice_opens_with_both_lists() {
    let d = Document::new("Guides");
    let page = d.first_page().unwrap();
    let mut v = serde_json::to_string(d.node(page).unwrap()).unwrap();
    v.pop();
    v.push_str(r#","guides":[{"vertical":true,"offset":5.0}],"guides":[{"vertical":false,"offset":7.0}]}"#);
    let n: Node = serde_json::from_str(&v).unwrap();
    assert_eq!(n.guides(), [guide(true, 5.0), guide(false, 7.0)]);
    assert_eq!(serde_json::to_string(&n).unwrap().matches("\"guides\"").count(), 1);
}

#[test]
fn a_malformed_guides_list_is_still_refused() {
    let d = Document::new("Guides");
    let mut v = serde_json::to_value(d.node(d.first_page().unwrap()).unwrap()).unwrap();
    v["guides"] = serde_json::json!([{"vertical": "yes"}]);
    assert!(serde_json::from_value::<Node>(v).is_err());
}
