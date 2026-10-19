//! The query on FigCraft documents: the [MAP] choices (`figcraft.rs`), through `node.query` as
//! every route runs it.

use serde_json::{Value, json};

use crate::Session;
use crate::params::id_of;

fn id(v: &Value) -> u64 {
    id_of(&v["id"]).unwrap().0
}

/// The names of the matches, in order.
fn names(s: &mut Session, q: &str) -> Vec<String> {
    let r = s.execute("node.query", &json!({"query": q})).unwrap_or_else(|e| panic!("{q}: {e}"));
    r["results"].as_array().unwrap().iter().map(|h| h["name"].as_str().unwrap().to_string()).collect()
}

/// Page 1: an auto layout "Pricing" frame holding a "Price" text and a "Swatch" rectangle, and a
/// free "Poster" frame. Page 2: a "Card" component and a "Pro" instance of it.
fn scene() -> (Session, u64, u64) {
    let mut s = Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 300, "height": 100, "name": "Pricing"})).unwrap());
    s.execute("layout.add", &json!({"ids": [f], "direction": "vertical"})).unwrap();
    s.execute("node.create", &json!({"type": "text", "text": "$12", "name": "Price", "parent": f})).unwrap();
    let r = id(&s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 40, "height": 20, "parent": f})).unwrap());
    s.execute("node.set", &json!({"id": r, "props": {"name": "Swatch"}})).unwrap();
    s.execute("fill.set", &json!({"ids": [r], "color": "#FF5A1F"})).unwrap();
    s.execute("frame.create", &json!({"x": 400, "y": 0, "width": 200, "height": 300, "name": "Poster"})).unwrap();
    s.execute("page.add", &json!({"name": "Components"})).unwrap();
    let card = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100, "name": "Card"})).unwrap());
    s.execute("component.create", &json!({"ids": [card]})).unwrap();
    let comp = s.doc().unwrap().nodes().find(|n| n.name == "Card" && n.kind.type_name() == "COMPONENT").unwrap().id;
    let inst = id(&s.execute("instance.create", &json!({"component": comp, "x": 200, "y": 0})).unwrap());
    s.execute("node.set", &json!({"id": inst, "props": {"name": "Pro"}})).unwrap();
    (s, f, r)
}

#[test]
fn node_tests_are_figcrafts_kinds_across_pages_in_document_order() {
    let (mut s, ..) = scene();
    assert_eq!(names(&mut s, "/FRAME"), ["Pricing", "Poster"]);
    assert_eq!(names(&mut s, "//TEXT"), ["Price"]);
    assert_eq!(names(&mut s, "//RECTANGLE"), ["Swatch"]);
    assert_eq!(names(&mut s, "/COMPONENT"), ["Card"]);
    assert_eq!(names(&mut s, "//INSTANCE[@component=\"Card\"]"), ["Pro"]);
    // Positions count per parent, and a page is a parent: each page's first top-level layer.
    assert_eq!(names(&mut s, "/*[1]"), ["Pricing", "Card"]);
    // Scrim's BOX isn't a FigCraft kind: the error lists what is.
    let e = s.execute("node.query", &json!({"query": "//BOX"})).unwrap_err().to_string();
    assert!(e.starts_with("invalid query: unknown node test 'BOX': use FRAME, GROUP, SECTION"), "{e}");
    let e = s.execute("node.query", &json!({"query": "//rectangle"})).unwrap_err().to_string();
    assert!(e.contains("(did you mean RECTANGLE?)"), "{e}");
}

#[test]
fn attributes_read_the_document() {
    let (mut s, ..) = scene();
    assert_eq!(names(&mut s, "//FRAME[@layout=\"flex\"]//TEXT[contains(@name, \"Pri\")]"), ["Price"]);
    assert_eq!(names(&mut s, "//*[@direction=\"column\"]"), ["Pricing"]);
    assert_eq!(names(&mut s, "//*[@fill=\"#ff5a1f\"]"), ["Swatch"]);
    assert_eq!(names(&mut s, "//*[@background=\"#ff5a1f\"]"), ["Swatch"]);
    assert_eq!(names(&mut s, "//TEXT[@text=\"$12\"]"), ["Price"]);
    assert_eq!(names(&mut s, "//*[@width >= 300]"), ["Pricing"]);
    assert_eq!(names(&mut s, "//*[@children = 2]"), ["Pricing"]);
    assert_eq!(names(&mut s, "//TEXT[@style.fontSize > 0]"), ["Price"]);
    // An undeclared key is absent, never an error.
    assert!(names(&mut s, "//*[@style.gridArea]").is_empty());
    // Not in the subset: an error saying what is.
    let e = s.execute("node.query", &json!({"query": "//TEXT/.."})).unwrap_err().to_string();
    assert!(e.contains("'..' (parent) is not supported"), "{e}");
    let e = s.execute("node.query", &json!({"query": "//*[@alt]"})).unwrap_err().to_string();
    assert!(e.contains("unknown attribute @alt: use @id, @name"), "{e}");
}

#[test]
fn hidden_and_locked_are_presence_attributes() {
    let (mut s, _, r) = scene();
    assert!(names(&mut s, "//*[@locked or @hidden]").is_empty());
    s.execute("object.toggleLock", &json!({"ids": [r]})).unwrap();
    s.execute("object.toggleVisible", &json!({"ids": [r]})).unwrap();
    assert_eq!(names(&mut s, "//*[@locked and @hidden]"), ["Swatch"]);
    assert_eq!(names(&mut s, "//FRAME[@name=\"Pricing\"]/*[not(@hidden)]"), ["Price"]);
}

#[test]
fn results_carry_the_document_box_and_never_change_the_document() {
    let (mut s, f, r) = scene();
    let before = s.doc().unwrap().clone();
    let rev = s.changes_since(0).unwrap()["revision"].as_u64().unwrap();
    let undo = s.undo_labels(5);
    let out = s.execute("node.query", &json!({"query": "//RECTANGLE", "limit": 5})).unwrap();
    let hit = &out["results"][0];
    assert_eq!(
        (hit["id"].as_u64(), hit["parentId"].as_u64(), hit["element"].as_str(), hit["type"].as_str()),
        (Some(r), Some(f), Some("RECTANGLE"), Some("rectangle"))
    );
    assert_eq!((hit["rect"]["w"].as_f64(), hit["rect"]["h"].as_f64(), hit["stale"].as_bool()), (Some(40.0), Some(20.0), Some(false)));
    assert!(hit.get("measuredAt").is_none());
    // FigCraft's revision counts every command (it is the redraw counter), so read-only means: the
    // same document, nothing in the change feed and no undo step.
    assert_eq!(*s.doc().unwrap(), before);
    assert_eq!(s.changes_since(rev).unwrap()["changes"], json!([]));
    assert_eq!(s.undo_labels(5), undo);
}

#[test]
fn limit_and_page_scope() {
    let (mut s, ..) = scene();
    let out = s.execute("node.query", &json!({"query": "//*", "limit": 2})).unwrap();
    assert_eq!((out["total"].as_u64(), out["returned"].as_u64(), out["truncated"].as_bool()), (Some(6), Some(2), Some(true)));
    let first = s.doc().unwrap().pages[0].0;
    let out = s.execute("node.query", &json!({"query": "//*", "page": first})).unwrap();
    assert_eq!(out["total"], 4);
    let rect = s.doc().unwrap().nodes().find(|n| n.name == "Swatch").unwrap().id.0;
    for bad in [json!({"query": "//*", "limit": 0}), json!({"query": "//*", "limit": 2.5}), json!({"query": "//*", "page": rect}), json!({})] {
        assert!(s.execute("node.query", &bad).is_err(), "{bad}");
    }
}

/// Local only: medians on a 20,200-layer document (`cargo test -p figcraft-engine --lib
/// query_timing -- --ignored --nocapture`). Scrim's budget is well under 100 ms for a typical query.
#[test]
#[ignore]
fn query_timing() {
    use figcraft_doc::{AutoLayout, Document, NodeKind, TextData};
    let mut doc = Document::new("Stress");
    let page = doc.pages[0];
    for a in 0..200 {
        let f = doc.create(page, NodeKind::Frame, &format!("Board {a}"), a as f64 * 500.0, 0.0, 400.0, 800.0).unwrap();
        doc.node_mut(f).unwrap().layout = Some(AutoLayout::default());
        for b in 0..10 {
            let g = doc.create(f, NodeKind::Frame, &format!("Card {b}"), 0.0, 0.0, 400.0, 80.0).unwrap();
            for c in 0..9 {
                let (kind, name) = if c % 3 == 0 { (NodeKind::Text(TextData::new("Price")), "Title") } else { (NodeKind::Rectangle, "Swatch") };
                doc.create(g, kind, name, 0.0, 0.0, 10.0 + c as f64 * 20.0, 10.0).unwrap();
            }
        }
    }
    eprintln!("{} layers", doc.node_count());
    for q in [
        "//TEXT",
        "//FRAME[@layout=\"flex\"]//TEXT[contains(@name,\"Title\")]",
        "/FRAME[3]//RECTANGLE[@width > 100]",
        "//*[@position=\"absolute\" or not(@type=\"rectangle\")]",
        "//*//*//TEXT",
    ] {
        let mut runs: Vec<f64> = (0..33)
            .map(|_| {
                let t = std::time::Instant::now();
                std::hint::black_box(super::query_doc(&doc, q, None, None).unwrap());
                t.elapsed().as_secs_f64() * 1000.0
            })
            .skip(3)
            .collect();
        runs.sort_by(f64::total_cmp);
        eprintln!("{q}: median {:.2} ms, worst {:.2} ms", runs[runs.len() / 2], runs[runs.len() - 1]);
    }
}
