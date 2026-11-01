//! Hostile files can't exhaust the stack or memory through the document model (upstream FigCraft
//! audit #1062 ENG-2 and #1058 FMT-2).

use crate::{Document, NodeKind};

/// Upstream 4da6a11b: render bounds stop where drawing stops. A crafted file nests groups far
/// deeper than anything draws; the leaf counts within the renderer's depth (256) and is left out
/// past it, as the renderer leaves it out, instead of the walk overflowing the stack.
#[test]
fn render_bounds_stop_where_drawing_stops_on_very_deep_chains() {
    let chain = |depth: usize| {
        let mut d = Document::new("x");
        let page = d.first_page().unwrap();
        let mut parent = page;
        for _ in 0..depth {
            parent = d.create(parent, NodeKind::Group, "G", 0.0, 0.0, 10.0, 10.0).unwrap();
        }
        d.create(parent, NodeKind::Rectangle, "R", 100.0, 100.0, 10.0, 10.0).unwrap();
        let top = d.node(page).unwrap().children[0];
        d.world_render_bounds(top).unwrap()
    };
    assert_eq!(chain(256).x1, 110.0);
    assert_eq!(chain(20_000).x1, 10.0);
}

/// Upstream c857f8d0: a hostile file can break an invariant once per child; `validate` keeps only
/// the first reports.
#[test]
fn validate_caps_the_issue_list() {
    let mut d = Document::new("x");
    let p = d.first_page().unwrap();
    let f = d.create(p, NodeKind::Frame, "F", 0.0, 0.0, 100.0, 100.0).unwrap();
    let g = d.create(f, NodeKind::Frame, "G", 0.0, 0.0, 10.0, 10.0).unwrap();
    let mut v = serde_json::to_value(&d).unwrap();
    v["nodes"][f.to_string()]["children"] = serde_json::json!(vec![g; 5000]);
    let bad: Document = serde_json::from_value(v).unwrap();
    let issues = bad.validate();
    assert!(!issues.is_empty());
    assert!(issues.len() <= 100, "{} issues", issues.len());
}
