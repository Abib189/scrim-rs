//! Fix AG: `.scrim` is what Scrim saves; `.figcraft` opens forever and is never written; every
//! file is read by the importer its contents call for, whatever its name.

use std::path::{Path, PathBuf};

use figcraft_doc::{Document, NodeKind};
use serde_json::{Value, json};

use super::{REGISTRY, Unreadable, listed, open_extensions, pick};
use crate::{EngineError, Session};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-importers-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn p(d: &Path, file: &str) -> String {
    d.join(file).to_string_lossy().into_owned()
}

/// A small saved document's bytes.
fn scrim_bytes() -> Vec<u8> {
    let mut s = Session::with_document(Document::new("Poster"));
    s.execute("shape.rectangle", &json!({"x": 10, "y": 20, "width": 30, "height": 40})).unwrap();
    crate::cmd::file::file_bytes(s.ds().unwrap()).unwrap()
}

fn doc_json(s: &Session) -> Value {
    let mut v = serde_json::to_value(&*s.ds().unwrap().doc).unwrap();
    // The page shown when saved is not part of the comparison.
    v.as_object_mut().unwrap().remove("current_page");
    v
}

fn unreadable(r: crate::Result<Value>) -> Unreadable {
    match r {
        Err(EngineError::Unreadable(u)) => u,
        other => panic!("expected an unreadable-file error, got {other:?}"),
    }
}

#[test]
fn save_writes_scrim_and_a_second_save_is_byte_stable() {
    let d = dir("roundtrip");
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 1, "y": 2, "width": 3, "height": 4})).unwrap();
    // No extension: .scrim is added.
    let r = s.execute("file.saveAs", &json!({"path": p(&d, "Poster")})).unwrap();
    assert_eq!(r["path"], p(&d, "Poster.scrim"));
    let first = std::fs::read(p(&d, "Poster.scrim")).unwrap();
    assert_eq!(figcraft_format::sniff::kind(&first), figcraft_format::sniff::Kind::Native);
    s.execute("file.save", &json!({})).unwrap();
    assert_eq!(std::fs::read(p(&d, "Poster.scrim")).unwrap(), first, "a second save writes the same bytes");
    // Reloaded, it is the same document, and saving it again gives the same bytes.
    let mut again = Session::empty();
    again.execute("file.open", &json!({"path": p(&d, "Poster.scrim")})).unwrap();
    assert_eq!(doc_json(&again), doc_json(&s));
    again.execute("file.saveAs", &json!({"path": p(&d, "Copy.scrim")})).unwrap();
    assert_eq!(std::fs::read(p(&d, "Copy.scrim")).unwrap(), first, "reloaded and saved: byte for byte the same");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_figcraft_opens_unchanged_and_save_asks_for_a_scrim_beside_it() {
    let d = dir("figcraft");
    let old = p(&d, "Brand.figcraft");
    std::fs::write(&old, scrim_bytes()).unwrap();
    let before = std::fs::read(&old).unwrap();
    let mut s = Session::empty();
    let r = s.execute("file.open", &json!({"path": old})).unwrap();
    assert!(r.get("repairs").is_some() && r.get("warnings").is_none(), "a Scrim document, not an import: {r}");
    assert_eq!(s.ds().unwrap().path.as_deref(), Some(old.as_str()), "it goes by its file");
    assert_eq!(std::fs::read(&old).unwrap(), before, "opening never writes");
    // Save: refused, and the Save as dialog is asked for.
    s.ui_requests.clear();
    assert!(matches!(s.execute("file.save", &json!({})), Err(EngineError::Disabled(..))));
    assert!(s.ui_requests.iter().any(|r| r.kind == "dialog.saveAs"), "Save as opens");
    // An explicit .figcraft path is refused too; auto-save writes nothing.
    assert!(s.execute("file.saveAs", &json!({"path": old})).is_err());
    assert!(s.execute("file.save", &json!({"path": old})).is_err());
    let snapshot = crate::cmd::file::FileSnapshot::of(s.ds().unwrap());
    let bytes = snapshot.encode().unwrap();
    assert!(!crate::cmd::file::FileSnapshot::of(s.ds().unwrap()).save(&mut s, &bytes).unwrap(), "auto-save skips a .figcraft");
    assert_eq!(std::fs::read(&old).unwrap(), before, "never modified");
    // Saved as a .scrim beside it: that is the document's file from then on.
    let new = p(&d, "Brand.scrim");
    s.execute("file.saveAs", &json!({"path": new})).unwrap();
    assert_eq!(s.ds().unwrap().path.as_deref(), Some(new.as_str()));
    s.execute("file.save", &json!({})).unwrap();
    assert_eq!(std::fs::read(&old).unwrap(), before, "the .figcraft is untouched");
    assert!(Path::new(&old).exists());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn files_are_known_by_their_contents_not_their_names() {
    let d = dir("sniff");
    // A .scrim renamed .txt still opens as a Scrim document; a .figcraft renamed .scrim too.
    for name in ["renamed.txt", "old.scrim", "no-extension"] {
        std::fs::write(p(&d, name), scrim_bytes()).unwrap();
        let mut s = Session::empty();
        let r = s.execute("file.open", &json!({"path": p(&d, name)})).unwrap();
        assert!(r.get("repairs").is_some(), "{name}: {r}");
        assert!(s.doc().unwrap().nodes().any(|n| matches!(n.kind, NodeKind::Rectangle)), "{name}");
    }
    // The earlier Scrim's JSON .scrim: imported as a new unsaved document, with warnings.
    let earlier = json!({"schemaVersion": 1, "id": "x", "name": "Earlier", "rootIds": ["f"], "nodes": {
        "f": {"id": "f", "type": "frame", "name": "Frame", "parentId": null, "childIds": ["i"], "layout": {"display": "flex"}, "style": {"width": 200, "height": 100}, "canvas": {"x": 0, "y": 0}},
        "i": {"id": "i", "type": "image", "name": "Photo", "parentId": "f", "childIds": [], "layout": {}, "style": {"width": 50, "height": 50}, "src": "scrim-asset:aa.png"}}});
    std::fs::write(p(&d, "earlier.scrim"), serde_json::to_vec_pretty(&earlier).unwrap()).unwrap();
    let before = std::fs::read(p(&d, "earlier.scrim")).unwrap();
    let mut s = Session::empty();
    let r = s.execute("file.open", &json!({"path": p(&d, "earlier.scrim")})).unwrap();
    assert_eq!(s.ds().unwrap().path, None, "an import has no file of its own");
    assert_eq!(r["warnings"][0]["severity"], "dropped", "{r}");
    assert_eq!(r["warnings"][0]["path"], "Page 1 / Frame / Photo");
    assert_eq!(std::fs::read(p(&d, "earlier.scrim")).unwrap(), before, "the source is never written");
    // One saved by a newer earlier Scrim, a damaged one and garbage: a clear refusal, no document.
    let newer = json!({"schemaVersion": 9, "id": "x", "name": "x", "rootIds": [], "nodes": {}});
    std::fs::write(p(&d, "newer.scrim"), serde_json::to_vec(&newer).unwrap()).unwrap();
    std::fs::write(p(&d, "damaged.scrim"), br#"{"schemaVersion": 1, "rootIds": [], "nodes": 7}"#).unwrap();
    std::fs::write(p(&d, "garbage.scrim"), b"\x00\x01 not a design").unwrap();
    std::fs::write(p(&d, "empty.scrim"), b"").unwrap();
    let mut s = Session::empty();
    assert_eq!(unreadable(s.execute("file.open", &json!({"path": p(&d, "newer.scrim")}))), Unreadable::OldScrimNewer(9));
    assert!(matches!(unreadable(s.execute("file.open", &json!({"path": p(&d, "damaged.scrim")}))), Unreadable::OldScrimDamaged(_)));
    assert_eq!(unreadable(s.execute("file.open", &json!({"path": p(&d, "garbage.scrim")}))), Unreadable::NotSupported);
    assert_eq!(unreadable(s.execute("file.open", &json!({"path": p(&d, "empty.scrim")}))), Unreadable::NotSupported);
    assert!(s.docs.is_empty(), "nothing opened");
    std::fs::remove_dir_all(&d).ok();
}

/// Each importer behind the registry makes exactly what its reader made before.
#[test]
fn the_existing_importers_give_the_same_output_through_the_registry() {
    let same = |bytes: &[u8], name: &str, direct: Document, id: &str| {
        let o = super::Options { name, variables: None, images: &figcraft_format::rest::no_images };
        let (imported, imp) = super::load(bytes, &o).unwrap();
        assert_eq!(imp.id(), id, "{name}");
        assert_eq!(serde_json::to_value(&imported.loaded.doc).unwrap(), serde_json::to_value(&direct).unwrap(), "{name}");
        imported
    };
    // Native.
    let native = scrim_bytes();
    same(&native, "a.scrim", figcraft_format::load_bytes(&native).unwrap().0, "scrim");
    // SVG.
    let svg = br#"<?xml version="1.0"?><!-- a comment --><svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="red"/></svg>"#;
    let mut direct = Document::new("logo");
    let page = direct.first_page().unwrap();
    let notes = figcraft_svg::import_svg_with_notes(&mut direct, svg, page, 0.0, 0.0, "logo").unwrap().notes;
    let imported = same(svg, "logo.svg", direct, "svg");
    assert_eq!(imported.warnings.iter().map(|w| w.message.clone()).collect::<Vec<_>>(), notes);
    // An image.
    let png = {
        let mut d = Document::new("x");
        let p = d.first_page().unwrap();
        let id = d.create(p, NodeKind::Rectangle, "R", 0.0, 0.0, 3.0, 2.0).unwrap();
        crate::cmd::file::export_bytes(&d, &[id], figcraft_doc::ExportFormat::Png, 1.0).unwrap()
    };
    let mut direct = Document::new("photo");
    let page = direct.first_page().unwrap();
    let (hash, w, h) = figcraft_format::add_image(&mut direct, png.clone()).unwrap();
    let id = direct.create(page, NodeKind::Rectangle, "photo", 0.0, 0.0, f64::from(w), f64::from(h)).unwrap();
    direct.get_mut(id).unwrap().fills = vec![figcraft_doc::color::Paint::image(hash, figcraft_doc::color::ImageScaleMode::Fill)];
    same(&png, "photo.png", direct, "image");
    // REST API file JSON.
    let rest = serde_json::to_vec(&json!({"name": "From Figma", "document": {"id": "0:0", "type": "DOCUMENT", "children": [
        {"id": "0:1", "type": "CANVAS", "name": "Page 1", "children": [
            {"id": "1:1", "type": "RECTANGLE", "name": "Box", "absoluteBoundingBox": {"x": 0, "y": 0, "width": 10, "height": 10}, "fills": [{"type": "SOLID", "color": {"r": 1, "g": 0, "b": 0, "a": 1}}]}]}]}}))
    .unwrap();
    let (mut direct, _) = figcraft_format::rest::try_import_with(&rest, None, &figcraft_format::rest::no_images).unwrap().unwrap();
    crate::cmd::file::link_instances(&mut direct);
    same(&rest, "export.json", direct, "rest-json");
}

#[test]
fn the_extension_lists_and_homes_text_come_from_the_registry() {
    let exts = open_extensions();
    for imp in REGISTRY {
        for e in imp.extensions() {
            assert!(exts.contains(e), "{e} of {} is offered", imp.id());
        }
    }
    assert_eq!(exts.first(), Some(&"scrim"), "Scrim's own format first");
    assert!(exts.contains(&"figcraft") && exts.contains(&"sketch") && exts.contains(&"fig"));
    assert_eq!(listed(), [".scrim", ".fig", ".sketch", ".svg"]);
    // Only Scrim documents are native.
    assert_eq!(REGISTRY.iter().filter(|i| i.native()).map(|i| i.id()).collect::<Vec<_>>(), ["scrim"]);
    // Ids are unique.
    let mut ids: Vec<&str> = REGISTRY.iter().map(|i| i.id()).collect();
    ids.dedup();
    assert_eq!(ids.len(), REGISTRY.len());
}

#[test]
fn sniffing_never_guesses_from_the_name() {
    let native = scrim_bytes();
    for name in ["x.svg", "x.png", "x.sketch", "x.fig", "x.json"] {
        assert_eq!(pick(&native, name).map(|i| i.id()), Some("scrim"), "{name}");
    }
    assert!(pick(b"plain text", "x.svg").is_none());
    assert!(pick(b"", "x.png").is_none());
    assert_eq!(pick(b"<svg xmlns='http://www.w3.org/2000/svg'/>", "x.txt").map(|i| i.id()), Some("svg"));
    assert_eq!(pick(b"fig-kiwi\x0f\0\0\0", "x.dat").map(|i| i.id()), Some("fig"));
}
