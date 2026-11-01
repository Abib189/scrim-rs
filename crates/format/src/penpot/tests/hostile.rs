//! Hostile and broken input: an error or warnings, never a panic.

use super::*;
use crate::FormatError;

fn manifest_only(manifest: &str) -> Vec<u8> {
    zip(&[("manifest.json".into(), manifest.as_bytes().to_vec())])
}

#[test]
fn garbage_and_truncated_files_are_errors() {
    assert!(import(b"garbage", "x").is_err());
    assert!(import(&[], "x").is_err());
    let bytes = Export::new().page(1, "Page 1", json!({}), vec![shape(1, "rect", "Box", (0.0, 0.0, 1.0, 1.0))]).bytes();
    for cut in [bytes.len() / 2, bytes.len() - 10, 30] {
        assert!(import(bytes.get(..cut).unwrap(), "x").is_err(), "cut at {cut}");
    }
}

#[test]
fn a_newer_or_missing_manifest_version_is_refused() {
    let mut e = Export::new().page(1, "Page 1", json!({}), vec![]);
    e.manifest["version"] = json!(2);
    assert!(matches!(import(&e.bytes(), "x"), Err(FormatError::Newer(2))));
    e.manifest["version"] = json!("1");
    assert!(matches!(import(&e.bytes(), "x"), Err(FormatError::Corrupt(_))));
    let m = manifest_only(r#"{"version": 1, "files": [{"id": "abc", "name": "x", "features": []}]}"#);
    assert!(matches!(import(&m, "x"), Err(FormatError::Corrupt(_))));
    assert!(matches!(import(&manifest_only(r#"{"type": "penpot/export-files", "version": 1, "files": []}"#), "x"), Err(FormatError::Corrupt(_))));
    assert!(matches!(import(&manifest_only("[1, 2"), "x"), Err(FormatError::Corrupt(_))));
    let bad_id = manifest_only(r#"{"type": "penpot/export-files", "version": 1, "files": [{"id": "../../etc", "name": "x"}]}"#);
    assert!(matches!(import(&bad_id, "x"), Err(FormatError::Corrupt(_))));
}

#[test]
fn wrong_types_and_missing_keys_give_warnings() {
    let shapes = vec![
        json!({"id": uuid(1), "type": "rect"}),
        json!({"id": uuid(2), "type": "rect", "name": 5, "x": "10", "y": null, "width": -4, "height": 1e300, "selrect": [1, 2]}),
        json!({"id": uuid(3), "type": "frame", "name": "F", "x": 0, "y": 0, "width": 10, "height": 10, "shapes": {"a": 1}, "fills": "red"}),
        json!({"id": uuid(4), "type": 7, "name": "Typed"}),
        json!({"id": uuid(5), "type": "rect", "name": "R", "x": 0, "y": 0, "width": 1, "height": 1, "fills": [5, null, {"color": 3}, {"gradient": {"stops": "x"}}],
            "r1": "big", "opacity": 7, "hidden": "yes", "blendMode": 4, "transform": [1, 0, 0]}),
        json!({"id": uuid(6), "type": "image", "name": "I", "x": 0, "y": 0, "width": 1, "height": 1, "metadata": {"id": "../../secret"}}),
        json!({"id": uuid(7), "type": "path", "name": "P", "selrect": {"x": 0, "y": 0, "width": 1, "height": 1}, "content": "M 0 0 L 1e400 0"}),
        json!({"id": uuid(8), "type": "bool", "name": "B", "boolType": "xor", "shapes": [uuid(99)]}),
        json!("not an object"),
        json!([1, 2, 3]),
    ];
    let e = Export::new().page_with_root(1, "Page 1", json!({"name": 4, "background": "#zzz", "index": "first"}), None, vec![]);
    let mut e = e;
    for (i, s) in shapes.iter().enumerate() {
        e = e.json(&format!("files/{FILE}/pages/{}/{}.json", uuid(0x9001), uuid(100 + i as u32)), s);
    }
    let (doc, w) = imported(e);
    assert!(!w.is_empty());
    assert!(doc.validate().is_empty());
    let r = named(&doc, "R");
    assert_eq!(r.opacity, 1.0);
    assert!(r.visible);
}

#[test]
fn cycles_repeats_and_self_listing_import_each_layer_once() {
    let a = with(with(shape(1, "group", "A", (0.0, 0.0, 10.0, 10.0)), "shapes", json!([uuid(2), uuid(2), uuid(1)])), "parentId", json!(uuid(2)));
    let b = with(with(shape(2, "frame", "B", (0.0, 0.0, 10.0, 10.0)), "shapes", json!([uuid(1)])), "parentId", json!(uuid(1)));
    let c = with(shape(3, "rect", "C", (0.0, 0.0, 1.0, 1.0)), "parentId", json!(uuid(3)));
    let root = with(
        with(shape(0, "frame", "Root", (0.0, 0.0, 0.0, 0.0)), "id", json!(super::super::read::NIL)),
        "shapes",
        json!([uuid(1), uuid(1), uuid(3)]),
    );
    let (doc, w) = imported(Export::new().page_with_root(1, "Page 1", json!({}), Some(root), vec![a, b, c]));
    for name in ["A", "B", "C"] {
        assert_eq!(doc.nodes().filter(|n| n.name == name).count(), 1, "{name}");
    }
    assert!(w.iter().any(|x| x.message.contains("more than once")), "{w:?}");
}

#[test]
fn deep_nesting_stops_at_the_cap() {
    let depth = 300u32;
    let mut shapes = Vec::new();
    for i in 1..=depth {
        let mut g = shape(i, "group", &format!("G{i}"), (0.0, 0.0, 1.0, 1.0));
        if i < depth {
            g = with(g, "shapes", json!([uuid(i + 1)]));
        }
        if i > 1 {
            g = with(g, "parentId", json!(uuid(i - 1)));
        }
        shapes.push(g);
    }
    let (_, w) = one(shapes);
    assert!(w.iter().any(|x| x.message.contains("nested deeper")), "{w:?}");
}

#[test]
fn orphans_are_reported() {
    let lost = with(shape(5, "rect", "Lost", (0.0, 0.0, 1.0, 1.0)), "parentId", json!(uuid(6)));
    let parent = shape(6, "frame", "Parent", (0.0, 0.0, 10.0, 10.0));
    let (doc, w) = one(vec![parent, lost]);
    assert!(doc.nodes().all(|n| n.name != "Lost"));
    assert!(has(&w, Severity::Dropped, "Page 1 / Lost", "isn't in its parent's list"), "{w:?}");
}

#[test]
fn hostile_previews_are_none() {
    let thumb = json!({"mediaId": "../../../etc/passwd"});
    let e = Export::new().json(&format!("files/{FILE}/thumbnails/frame/{}/{}.json", uuid(1), uuid(2)), &thumb);
    assert_eq!(preview(&e.bytes()), None);
    let e = Export::new().json(&format!("files/{FILE}/thumbnails/frame/{}/{}.json", uuid(1), uuid(2)), &json!([1]));
    assert_eq!(preview(&e.bytes()), None);
    assert_eq!(preview(&[]), None);
}
