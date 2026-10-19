//! `analyze_design` on FigCraft documents: colours, type, spacing and repeats read through the
//! CSS view, against FigCraft's variables and styles as tokens.

use serde_json::{Value, json};

use crate::Session;
use crate::params::id_of;

fn id(v: &Value) -> u64 {
    id_of(&v["id"]).unwrap().0
}

fn color<'a>(r: &'a Value, hex: &str) -> &'a Value {
    r["colors"]["values"].as_array().unwrap().iter().find(|v| v["value"] == hex).unwrap_or_else(|| panic!("{hex} in {r}"))
}

/// Three cards (an auto layout frame holding two texts) on a page, in two near-identical oranges.
fn scene() -> (Session, Vec<u64>) {
    let mut s = Session::new();
    let mut cards = Vec::new();
    for (i, fill) in ["#FF7A59", "#FF7A59", "#FF7B59"].iter().enumerate() {
        let f = id(&s
            .execute("frame.create", &json!({"x": i as f64 * 300.0, "y": 0, "width": 240, "height": 120, "name": format!("Card {i}")}))
            .unwrap());
        s.execute("layout.add", &json!({"ids": [f], "direction": "vertical"})).unwrap();
        s.execute("node.set", &json!({"id": f, "props": {"fills": [fill], "itemSpacing": 12, "padding": 16}})).unwrap();
        for t in ["Title", "Body"] {
            s.execute("node.create", &json!({"type": "text", "text": t, "name": t, "parent": f})).unwrap();
        }
        cards.push(f);
    }
    (s, cards)
}

#[test]
fn it_reports_what_the_document_uses() {
    let (mut s, cards) = scene();
    let r = s.execute("design.analyze", &json!({})).unwrap();
    assert_eq!((r["scope"].as_str(), r["nodes"].as_u64()), (Some("(document)"), Some(9)));
    // The six texts' default black, then the cards' orange.
    assert_eq!((r["colors"]["values"][0]["value"].as_str(), r["colors"]["values"][0]["count"].as_u64()), (Some("#000000"), Some(6)));
    assert_eq!(color(&r, "#ff7a59")["count"], 2);
    let near = &r["colors"]["nearDuplicates"][0];
    assert_eq!((near["suggested"].as_str(), near["values"].as_array().map(Vec::len)), (Some("#ff7a59"), Some(2)), "{r}");
    assert_eq!(r["typography"]["textNodes"], 6);
    // No spacing tokens: the 4 px grid.
    assert_eq!(r["spacing"]["scale"], "4 px grid (no spacing tokens; 1 and 2 allowed)");
    assert!(r["spacing"]["values"].as_array().unwrap().iter().any(|v| v["value"] == 12 && v["properties"]["gap"] == 3), "{r}");
    let rep = &r["repeated"][0];
    assert_eq!((rep["count"].as_u64(), rep["nodesEach"].as_u64(), rep["shape"].as_str()), (Some(3), Some(3), Some("frame:flex-column(text, text)")));
    assert_eq!(rep["ids"], json!(cards));
    let r = s.execute("design.analyze", &json!({"id": cards[0]})).unwrap();
    assert_eq!(r["nodes"], 3);
    assert_eq!(s.execute("design.analyze", &json!({"id": 999_999})).unwrap_err().to_string(), "node not found: 999999");
}

#[test]
fn variables_and_styles_are_the_tokens() {
    let (mut s, _) = scene();
    let c = s.execute("variable.createCollection", &json!({"name": "Brand"})).unwrap();
    let coll = c["id"].as_str().unwrap().to_string();
    s.execute("variable.create", &json!({"collection": coll, "name": "accent", "type": "color", "value": "#FF7A59"})).unwrap();
    s.execute("variable.create", &json!({"collection": coll, "name": "space/md", "type": "number", "value": 16})).unwrap();
    let r = s.execute("design.analyze", &json!({})).unwrap();
    assert_eq!(color(&r, "#ff7a59")["token"], json!({"name": "accent", "deltaE": 0}));
    assert_eq!(color(&r, "#ff7b59")["token"]["name"], "accent");
    assert_eq!(r["spacing"]["scale"], "spacing tokens (16)");
    assert!(r["spacing"]["offScale"].as_array().unwrap().contains(&json!(12)), "{r}");
}
