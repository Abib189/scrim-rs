//! Fix AI C-1: a file upstream FigCraft saved as a board is told apart when it is read. Its marks
//! are `kind: "board"` on the document and a `board` object on a layer (upstream 8167f13a); Scrim
//! has no boards, so the reader reports them for the window to say so, and the document opens as a
//! design.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use figcraft_format::{BoardMarks, load_bytes_with_versions};
use serde_json::{Value, json};

/// A document with one frame, as JSON, and the frame's id.
fn design() -> (Value, Value) {
    let mut d = figcraft_doc::Document::new("Board");
    let p = d.first_page().unwrap();
    let f = d.create(p, figcraft_doc::NodeKind::Frame, "Sticky", 0.0, 0.0, 200.0, 200.0).unwrap();
    (serde_json::to_value(&d).unwrap(), serde_json::to_value(f).unwrap())
}

fn marks(v: &Value) -> Option<BoardMarks> {
    load_bytes_with_versions(&serde_json::to_vec(v).unwrap()).unwrap().board
}

#[test]
fn a_board_kind_is_reported() {
    let (mut v, _) = design();
    v["kind"] = json!("board");
    assert_eq!(marks(&v), Some(BoardMarks { kind: true, items: 0 }));
}

#[test]
fn layers_carrying_a_board_object_are_counted() {
    let (mut v, f) = design();
    v["nodes"][f.to_string()]["board"] = json!({"type": "sticky", "author": "Ana"});
    assert!(v["nodes"][f.to_string()].is_object(), "the frame is keyed by its id: {}", v["nodes"]);
    assert_eq!(marks(&v), Some(BoardMarks { kind: false, items: 1 }));
}

#[test]
fn a_design_is_not_a_board_even_with_board_in_its_names() {
    let (mut v, f) = design();
    assert_eq!(marks(&v), None);
    // A layer named "board", and text holding what the key looks like, aren't marks.
    v["nodes"][f.to_string()]["name"] = json!("\"board\": {\"type\": \"sticky\"}");
    v["name"] = json!("board");
    v["kind"] = json!("design");
    assert_eq!(marks(&v), None);
}
