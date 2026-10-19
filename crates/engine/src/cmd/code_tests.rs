//! The Code tab's commands: the format switch, copying a layer's code, and the standalone page.

use serde_json::json;

use super::{COPY_REQUEST, FORMAT_REQUEST};
use crate::Session;

/// A frame "Card <1>" holding a text, selected.
fn session() -> (Session, u64) {
    let mut s = Session::new();
    let f = s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 200, "height": 100, "name": "Card <1>"})).unwrap()["id"].as_u64().unwrap();
    s.execute("text.create", &json!({"x": 10, "y": 10, "text": "Hello", "parent": f})).unwrap();
    s.execute("select.set", &json!({"ids": [f]})).unwrap();
    (s, f)
}

#[test]
fn the_format_is_checked_and_asked_of_the_ui() {
    let (mut s, _) = session();
    s.ui_requests.clear();
    assert_eq!(s.execute("code.setFormat", &json!({"format": "React", "unit": "rem"})).unwrap(), json!({"format": "react", "unit": "rem"}));
    let [req] = s.ui_requests.as_slice() else { panic!("{:?}", s.ui_requests) };
    assert_eq!((req.kind.as_str(), &req.data), (FORMAT_REQUEST, &json!({"format": "react", "unit": "rem"})));
    for (p, why) in [
        (json!({}), "`format` is required"),
        (json!({"format": "cobol"}), "format: css|html"),
        (json!({"format": "swiftui", "unit": "rem"}), "rem is for the web formats"),
        (json!({"format": "css", "unit": "em"}), "unit: px|rem"),
    ] {
        let e = s.execute("code.setFormat", &p).unwrap_err().to_string();
        assert!(e.contains(why), "{p}: {e}");
    }
}

#[test]
fn copying_code_is_inspect_code_onto_the_clipboard() {
    let (mut s, f) = session();
    let want = s.execute("inspect.code", &json!({"id": f, "language": "html", "unit": "rem"})).unwrap();
    s.ui_requests.clear();
    let got = s.execute("code.copy", &json!({"language": "html", "unit": "rem"})).unwrap();
    assert_eq!(got, want, "the selected layer, as inspect.code returns it");
    let [req] = s.ui_requests.as_slice() else { panic!("{:?}", s.ui_requests) };
    assert_eq!((req.kind.as_str(), &req.data["text"], &req.data["what"]), (COPY_REQUEST, &want["code"], &json!("HTML + CSS")));
}

#[test]
fn the_standalone_page_is_complete_and_fetches_nothing() {
    let (mut s, f) = session();
    let (doc, steps) = (s.doc().unwrap().clone(), s.undo_depth());
    s.ui_requests.clear();
    let r = s.execute("code.copyStandaloneHtml", &json!({})).unwrap();
    let html = r["html"].as_str().unwrap();
    assert_eq!((r["id"].as_u64(), r["name"].as_str()), (Some(f), Some("Card <1>")));
    assert!(html.starts_with("<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">"), "{html}");
    assert!(html.contains("<title>Card &lt;1&gt;</title>"), "the name, escaped");
    assert!(html.contains("content=\"default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:\""));
    let (head, body) = html.split_once("<body").unwrap();
    assert!(head.contains("<style>") && !body.contains("<style>"), "the stylesheet sits in the head");
    assert!(body.contains("Hello") && html.trim_end().ends_with("</html>"));
    for external in ["://", "@import", "<script", "<link", "src="] {
        assert!(!html.contains(external), "nothing external: {external}");
    }
    let [req] = s.ui_requests.as_slice() else { panic!("{:?}", s.ui_requests) };
    assert_eq!((req.kind.as_str(), &req.data["text"]), (COPY_REQUEST, &r["html"]));
    assert!(s.doc().unwrap() == &doc && s.undo_depth() == steps, "copying reads, never edits");
    // Without a selection there is nothing to copy.
    s.execute("select.none", &json!({})).unwrap();
    assert!(s.execute("code.copyStandaloneHtml", &json!({})).unwrap_err().to_string().contains("select a layer"));
}
