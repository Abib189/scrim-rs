//! The lint on FigCraft documents: the CSS view, hidden layers, boxes from the document, and
//! profiles and ignores stored in layer data (one undo step each).

use serde_json::{Value, json};

use crate::Session;
use crate::params::id_of;

fn id(v: &Value) -> u64 {
    id_of(&v["id"]).unwrap().0
}

fn rules(r: &Value, bucket: &str) -> Vec<(String, u64)> {
    r[bucket].as_array().unwrap().iter().map(|v| (v["ruleId"].as_str().unwrap().to_string(), v["nodeId"].as_u64().unwrap())).collect()
}

fn has(r: &Value, bucket: &str, rule: &str, node: u64) -> bool {
    rules(r, bucket).contains(&(rule.to_string(), node))
}

/// A white "Card" frame holding a 10 px pale-grey text, and a second frame overlapping it.
fn scene() -> (Session, u64, u64, u64) {
    let mut s = Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 300, "height": 200, "name": "Card"})).unwrap());
    s.execute("fill.set", &json!({"ids": [f], "color": "#FFFFFF"})).unwrap();
    let t = id(&s.execute("node.create", &json!({"type": "text", "text": "Fine print", "name": "Note", "parent": f, "x": 20, "y": 20})).unwrap());
    s.execute("node.set", &json!({"id": t, "props": {"fontSize": 10, "fills": ["#DDDDDD"]}})).unwrap();
    let g = id(&s.execute("frame.create", &json!({"x": 250, "y": 150, "width": 100, "height": 100, "name": "Badge"})).unwrap());
    (s, f, t, g)
}

#[test]
fn findings_come_from_the_layers_css_and_their_boxes() {
    let (mut s, f, t, g) = scene();
    let r = s.execute("lint.run", &json!({})).unwrap();
    assert!(has(&r, "violations", "tiny-text", t), "{r}");
    assert!(has(&r, "violations", "low-contrast", t), "{r}");
    // FigCraft's default font is on the overused list.
    assert!(has(&r, "violations", "overused-font", t), "{r}");
    // Boxes always come from the document: overlapping artboards are found.
    let overlap = r["violations"].as_array().unwrap().iter().find(|v| v["ruleId"] == "root-overlap").expect("root-overlap");
    assert_eq!(
        (overlap["nodeId"].as_u64(), &overlap["relatedNodeIds"], overlap["detail"].as_str()),
        (Some(f), &json!([g]), Some("overlaps \"Badge\" by 50×50"))
    );
    // A subtree run judges only the subtree and never reports overlapping roots.
    let r = s.execute("lint.run", &json!({"id": f})).unwrap();
    assert!(rules(&r, "violations").iter().all(|(rule, _)| rule != "root-overlap"), "{r}");
    assert_eq!(r["target"], json!(f));
    let e = s.execute("lint.run", &json!({"id": 99999})).unwrap_err().to_string();
    assert_eq!(e, "node not found: 99999");
}

#[test]
fn hidden_layers_are_not_linted() {
    let (mut s, _, t, _) = scene();
    s.execute("object.toggleVisible", &json!({"ids": [t]})).unwrap();
    let r = s.execute("lint.run", &json!({})).unwrap();
    assert!(rules(&r, "violations").iter().all(|(_, n)| *n != t), "{r}");
}

#[test]
fn an_ignore_is_stored_in_the_document_and_undone_in_one_step() {
    let (mut s, f, t, _) = scene();
    let e = s.execute("lint.ignore", &json!({"ruleId": "tiny-text", "nodeId": f})).unwrap_err().to_string();
    assert_eq!(e, "ignore_lint needs a non-empty reason saying why tiny-text is deliberate here");
    let e = s.execute("lint.ignore", &json!({"ruleId": "nope", "reason": "x"})).unwrap_err().to_string();
    assert!(e.starts_with("unknown lint rule: nope. Known rules: side-tab, border-accent-on-rounded,"), "{e}");
    let out = s.execute("lint.ignore", &json!({"ruleId": "tiny-text", "nodeId": f, "reason": "legal copy, sized by the client"})).unwrap();
    assert_eq!(out["ignores"], json!([{"ruleId": "tiny-text", "nodeId": f, "reason": "legal copy, sized by the client"}]));
    let r = s.execute("lint.run", &json!({})).unwrap();
    assert!(!has(&r, "violations", "tiny-text", t));
    let ig = &r["ignored"].as_array().unwrap().iter().find(|i| i["ruleId"] == "tiny-text").unwrap().clone();
    assert_eq!((ig["ignoreNodeId"].as_u64(), ig["reason"].as_str()), (Some(f), Some("legal copy, sized by the client")));
    // Saved with the file, and one undo step.
    let doc = s.doc().unwrap();
    assert!(doc.node(figcraft_doc::NodeId(f)).unwrap().plugin_data.contains_key("scrim.lint.ignores"));
    s.execute("edit.undo", &json!({})).unwrap();
    let r = s.execute("lint.run", &json!({})).unwrap();
    assert!(has(&r, "violations", "tiny-text", t));
    // A document-wide ignore lives on the first page.
    s.execute("lint.ignore", &json!({"ruleId": "overused-font", "reason": "the brand font"})).unwrap();
    let r = s.execute("lint.run", &json!({})).unwrap();
    assert!(!has(&r, "violations", "overused-font", t));
}

#[test]
fn an_expressive_artboard_relaxes_display_rules_but_never_contrast() {
    let (mut s, f, t, _) = scene();
    s.execute("node.set", &json!({"id": t, "props": {"fontSize": 96}})).unwrap();
    let r = s.execute("lint.run", &json!({"id": f})).unwrap();
    assert!(has(&r, "violations", "oversized-h1", t));
    let out = s.execute("lint.setProfile", &json!({"ids": [f, f], "profile": "expressive"})).unwrap();
    assert_eq!(out["roots"][0]["profile"], "expressive");
    let r = s.execute("lint.run", &json!({"id": f})).unwrap();
    assert_eq!((r["profile"].as_str(), r["profileSource"].as_str()), (Some("expressive"), Some("document")));
    assert!(r["relaxedByProfile"].as_array().unwrap().iter().any(|v| v["ruleId"] == "oversized-h1"));
    assert!(has(&r, "violations", "low-contrast", t), "contrast still counts");
    let e = s.execute("lint.setProfile", &json!({"ids": [t], "profile": "ui"})).unwrap_err().to_string();
    assert_eq!(e, format!("lint profiles apply to root artboards; {t} is inside {f}"));
    // An override judges without writing.
    let r = s.execute("lint.run", &json!({"id": f, "profile": "ui"})).unwrap();
    assert_eq!(r["profileSource"], "override");
    s.execute("edit.undo", &json!({})).unwrap();
    let r = s.execute("lint.run", &json!({"id": f})).unwrap();
    assert_eq!(r["profile"], "ui");
}

#[test]
fn the_query_reads_active_findings() {
    let (mut s, _, t, _) = scene();
    let r = s.execute("node.query", &json!({"query": "//TEXT[contains(@lint, \"tiny-text\")]"})).unwrap();
    assert_eq!(r["results"][0]["id"].as_u64(), Some(t));
    let r = s.execute("node.query", &json!({"query": "//*[@lintCount >= 3]"})).unwrap();
    assert_eq!(r["results"][0]["id"].as_u64(), Some(t));
}
