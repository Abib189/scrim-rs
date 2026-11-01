//! The earlier Scrim's JSON `.scrim` files (Fix AG2), read from the synthetic documents the
//! TypeScript Scrim's tests wrote (`docs/scrim-transfer/cases/query-nodes.json › documents`).

use figcraft_doc::color::Color;
use figcraft_doc::{Document, LayoutMode, NodeId, NodeKind, Sizing};
use serde_json::{Value, json};

use super::import;
use crate::FormatError;
use crate::import_report::Severity;

fn fixture(name: &str) -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/cases/query-nodes.json");
    let all: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    all["documents"][name].clone()
}

fn bytes(v: &Value) -> Vec<u8> {
    // As the earlier Scrim wrote them: pretty-printed with a newline.
    let mut b = serde_json::to_vec_pretty(v).unwrap();
    b.push(b'\n');
    b
}

fn find<'a>(doc: &'a Document, name: &str) -> &'a figcraft_doc::Node {
    doc.nodes().find(|n| n.name == name).unwrap_or_else(|| panic!("no layer {name}"))
}

fn kids(doc: &Document, id: NodeId) -> Vec<String> {
    doc.node(id).unwrap().children.iter().map(|c| doc.node(*c).unwrap().name.clone()).collect()
}

#[test]
fn the_query_fixture_imports_with_its_structure_and_look() {
    let src = fixture("query");
    assert!(super::is_old_scrim(&bytes(&src)));
    let (doc, warnings) = import(&bytes(&src), "fallback").unwrap();
    assert_eq!(doc.name, "Query");
    let page = doc.first_page().unwrap();
    assert_eq!(kids(&doc, page), ["Page", "Poster", "Card", "Pro"], "the roots, in order");
    // A flex column frame at its canvas position, with its size and background.
    let p = find(&doc, "Page");
    let al = p.layout.as_ref().unwrap();
    assert_eq!((al.mode, al.gap), (LayoutMode::Vertical, 32.0));
    assert_eq!((p.transform.translation().x, p.transform.translation().y, p.width), (0.0, 0.0, 1280.0));
    assert_eq!(p.fills[0].solid_color(), Some(Color::WHITE));
    assert_eq!(kids(&doc, p.id), ["Nav", "Hero", "Photo"]);
    // A block frame (no auto layout) at its canvas position, clipping off.
    let poster = find(&doc, "Poster");
    assert!(poster.layout.is_none());
    assert_eq!((poster.transform.translation().x, poster.width, poster.height), (1400.0, 600.0, 800.0));
    // Text: characters, size, colour.
    let NodeKind::Text(logo) = &find(&doc, "Logo").kind else { panic!("Logo is text") };
    assert_eq!((logo.characters.as_str(), logo.style.font_size), ("Moth & Co", 20.0));
    assert_eq!(find(&doc, "Logo").fills[0].solid_color(), Color::from_hex("#111111"));
    // A box: a rectangle with its colour and pixel width.
    let cta = find(&doc, "CTA button");
    assert!(matches!(cta.kind, NodeKind::Rectangle));
    assert_eq!((cta.width, cta.fills[0].solid_color()), (120.0, Color::from_hex("#1a2b3c")));
    // The image is a placeholder, reported.
    let at = |path: &str| warnings.iter().filter(|w| w.path == path).map(|w| (w.severity, w.message.clone())).collect::<Vec<_>>();
    assert!(at("Page 1 / Page / Photo").iter().any(|(s, m)| *s == Severity::Dropped && m.contains("image isn't in the file")), "{warnings:#?}");
    // The instance is a copy of its component, reported.
    assert_eq!(kids(&doc, find(&doc, "Pro").id), ["Title"]);
    assert!(at("Page 1 / Pro").iter().any(|(s, m)| *s == Severity::Approximated && m.contains("Pricing Card")), "{warnings:#?}");
    // Grid layout and a percentage width are reported, with their layers' paths.
    assert!(at("Page 1 / Page / Hero").iter().any(|(_, m)| m.contains("grid layout")), "{warnings:#?}");
    assert!(at("Page 1 / Poster / Shape").iter().any(|(_, m)| m.contains("50%")), "{warnings:#?}");
}

#[test]
fn css_maps_to_auto_layout_sizing_borders_shadows_and_text() {
    let doc_json = json!({
        "schemaVersion": 1, "id": "d", "name": "Styles", "rootIds": ["row"], "canvasBackground": "#202020",
        "nodes": {
            "row": {"id": "row", "type": "frame", "name": "Row", "parentId": null, "childIds": ["a", "b", "t"],
                    "layout": {"display": "flex", "gap": "12px", "padding": "8px 16px", "justifyContent": "space-between", "alignItems": "center", "width": 400},
                    "style": {"backgroundColor": "rgba(255, 0, 0, 0.5)", "borderRadius": 8, "border": "2px solid #00ff00",
                              "boxShadow": "0 4px 12px rgba(0,0,0,0.25), inset 0 1px 0 #ffffff", "opacity": 0.9, "overflow": "hidden"},
                    "canvas": {"x": 10, "y": 20}},
            "a": {"id": "a", "type": "box", "name": "A", "parentId": "row", "childIds": [], "layout": {"flexGrow": 1, "height": 40}, "style": {"background": "#336699"}},
            "b": {"id": "b", "type": "box", "name": "B", "parentId": "row", "childIds": [], "layout": {"position": "absolute", "right": 10, "top": 5, "width": 30, "height": 30}, "style": {}},
            "t": {"id": "t", "type": "text", "name": "T", "parentId": "row", "childIds": [], "layout": {},
                  "style": {"fontFamily": "\"Inter\", sans-serif", "fontWeight": 700, "fontStyle": "italic", "fontSize": "18px", "lineHeight": 1.5,
                            "letterSpacing": "0.1em", "textAlign": "center", "textTransform": "uppercase", "textDecoration": "underline",
                            "transform": "rotate(4deg)"},
                  "text": "Hello", "states": {"hover": {"color": "red"}}}
        },
        "components": {}, "tokens": {"colors": {"brand": "#ff0000"}}
    });
    let (doc, warnings) = import(&bytes(&doc_json), "x").unwrap();
    let page = doc.first_page().unwrap();
    let Some(NodeKind::Page { background, .. }) = doc.node(page).map(|n| &n.kind) else { panic!() };
    assert_eq!(Some(*background), Color::from_hex("#202020"));
    let row = find(&doc, "Row");
    let al = row.layout.as_ref().unwrap();
    assert_eq!((al.mode, al.gap, al.padding), (LayoutMode::Horizontal, 12.0, [8.0, 16.0, 8.0, 16.0]));
    assert_eq!((format!("{:?}", al.main_align), format!("{:?}", al.cross_align)), ("SpaceBetween".into(), "Center".into()));
    assert_eq!((row.width, row.corners.radii, row.stroke.weight), (400.0, [8.0; 4], 2.0));
    assert_eq!(row.strokes[0].solid_color(), Color::from_hex("#00ff00"));
    assert_eq!(row.fills[0].solid_color().map(|c| (c.r, c.a)), Some((1.0, 0.5)));
    assert_eq!(row.effects.len(), 2, "a drop and an inner shadow");
    assert!((row.opacity - 0.9).abs() < 1e-6 && row.clip);
    assert_eq!((row.transform.translation().x, row.transform.translation().y), (10.0, 20.0));
    // Grow fills the main axis; an absolute child sits from its right edge.
    let a = find(&doc, "A");
    assert_eq!((a.child_layout.sizing_h, a.child_layout.sizing_v, a.height), (Sizing::Fill, Sizing::Fixed, 40.0));
    let b = find(&doc, "B");
    assert!(b.child_layout.absolute);
    assert_eq!((b.transform.translation().x, b.transform.translation().y), (400.0 - 10.0 - 30.0, 5.0));
    // Text style.
    let t = find(&doc, "T");
    let NodeKind::Text(td) = &t.kind else { panic!() };
    assert_eq!((td.style.font_family.as_str(), td.style.font_style.as_str(), td.style.font_size), ("Inter", "Bold Italic", 18.0));
    assert_eq!(
        format!("{:?} {:?} {:?} {:?} {:?}", td.style.line_height, td.style.letter_spacing, td.align_h, td.style.case, td.style.decoration),
        "Percent(150.0) Percent(10.0) Center Upper Underline"
    );
    // What was left out, on its layer.
    let has = |path: &str, s: Severity, part: &str| warnings.iter().any(|w| w.path == path && w.severity == s && w.message.contains(part));
    assert!(has("Page 1 / Row / T", Severity::Dropped, "transform"), "{warnings:#?}");
    assert!(has("Page 1 / Row / T", Severity::Dropped, "state styles"), "{warnings:#?}");
    assert!(has("", Severity::Dropped, "design tokens"), "{warnings:#?}");
}

#[test]
fn every_fixture_imports_without_error() {
    for name in ["query", "query-stray", "edge", "landing", "many"] {
        let (doc, _) = import(&bytes(&fixture(name)), name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(doc.nodes().count() > 1, "{name}");
    }
}

#[test]
fn newer_or_damaged_files_are_refused_never_guessed() {
    let newer = json!({"schemaVersion": 7, "id": "x", "name": "x", "rootIds": [], "nodes": {}});
    assert!(matches!(import(&bytes(&newer), "x"), Err(FormatError::Newer(7))));
    for bad in [&b"{"[..], b"[]", br#"{"schemaVersion": 1}"#, br#"{"schemaVersion": "one", "rootIds": [], "nodes": {}}"#, b""] {
        assert!(matches!(import(bad, "x"), Err(FormatError::Corrupt(_))), "{:?}", String::from_utf8_lossy(bad));
    }
    // A loop in the tree, a missing child and an unknown type are reported, not followed.
    let looped = json!({"schemaVersion": 1, "id": "x", "name": "x", "rootIds": ["a"], "nodes": {
        "a": {"id": "a", "type": "frame", "name": "A", "parentId": null, "childIds": ["b", "gone", "w"], "layout": {}, "style": {}},
        "b": {"id": "b", "type": "frame", "name": "B", "parentId": "a", "childIds": ["a"], "layout": {}, "style": {}},
        "w": {"id": "w", "type": "widget", "name": "W", "parentId": "a", "childIds": [], "layout": {}, "style": {}}}});
    let (_, warnings) = import(&bytes(&looped), "x").unwrap();
    let messages: Vec<&str> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(messages.iter().any(|m| m.contains("loop")), "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("missing")), "{messages:?}");
    assert!(messages.iter().any(|m| m.contains("`widget`")), "{messages:?}");
}
