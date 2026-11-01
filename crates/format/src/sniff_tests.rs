//! Telling files apart by their contents (Fix AG): a Scrim document whatever its name, the earlier
//! Scrim's JSON `.scrim`, and everything else.

use std::io::{Cursor, Write};

use figcraft_doc::Document;

use super::{Kind, kind, top_level};

fn zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut z = zip::ZipWriter::new(&mut buf);
    for (name, bytes) in members {
        z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(bytes).unwrap();
    }
    z.finish().unwrap();
    buf.into_inner()
}

#[test]
fn a_scrim_document_is_known_by_its_contents() {
    assert_eq!(kind(&crate::save_bytes(&Document::new("Poster"), None).unwrap()), Kind::Native);
    // Bare JSON, with its format key and from before it.
    assert_eq!(kind(&serde_json::to_vec(&crate::to_json(&Document::new("A")).unwrap()).unwrap()), Kind::Native);
    assert_eq!(kind(&serde_json::to_vec(&Document::new("Old")).unwrap()), Kind::Native);
    // A zip from before the format key.
    assert_eq!(kind(&zip(&[("document.json", br#"{"name": "x", "pages": [], "nodes": {}}"#)])), Kind::Native);
}

#[test]
fn the_earlier_scrim_and_other_tools_are_not_scrim_documents() {
    let old = br#"{
  "schemaVersion": 1,
  "id": "q",
  "name": "Query",
  "rootIds": [],
  "nodes": {}
}"#;
    assert_eq!(kind(old), Kind::OldScrim);
    // Sketch keeps a document.json too, with `_class`; a .fig zip holds canvas.fig.
    assert_eq!(kind(&zip(&[("document.json", br#"{"_class": "document", "pages": []}"#), ("meta.json", b"{}")])), Kind::Other);
    assert_eq!(kind(&zip(&[("canvas.fig", b"fig-kiwi"), ("document.json", br#"{"format": "figcraft"}"#)])), Kind::Other);
    assert_eq!(kind(&zip(&[("document.json", br#"{"format": "someone-else", "pages": []}"#)])), Kind::Other);
    // REST API file JSON, images, text, garbage, nothing.
    assert_eq!(kind(br#"{"document": {"type": "DOCUMENT"}, "name": "x"}"#), Kind::Other);
    for other in [&b"\x89PNG\r\n\x1a\n"[..], b"<svg/>", b"hello", b"PK\x03\x04 broken", b"", b"{", b"{\"pages\"", b"[1,2]"] {
        assert_eq!(kind(other), Kind::Other, "{other:?}");
    }
}

#[test]
fn top_level_keys_stop_at_a_cut_and_never_panic() {
    let keys = |s: &str| top_level(s.as_bytes());
    assert_eq!(keys(r#"{"a": "x", "b": {"c": 1}, "d": 2}"#), [("a".into(), Some("x".into())), ("b".into(), None), ("d".into(), None)]);
    assert_eq!(keys(r#"{"a": "x", "b": {"c": "#), [("a".into(), Some("x".into())), ("b".into(), None)]);
    for s in ["", "{", "{\"", "{\"a\"", "{\"a\":", "{,}", "{\"a\" 1}", "not json"] {
        let _ = keys(s);
    }
}
