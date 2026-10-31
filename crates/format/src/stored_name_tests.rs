//! Reading the name a file stores without opening it (Fix AF): `.figcraft` archives and bare JSON,
//! `.fig` `meta.json`, and `None` (never a panic) for other kinds, damaged files and names past the
//! part read.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use figcraft_doc::Document;

use super::{HEAD_BYTES, stored_name};

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-stored-name-tests-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn a_figcraft_file_gives_the_name_it_stores_placeholder_or_not() {
    let d = dir("figcraft");
    for name in ["Poster", "Untitled", "Q3  report", "Ünïcødé ✓"] {
        let p = file(&d, "a.figcraft", &crate::save_bytes(&Document::new(name), None).unwrap());
        assert_eq!(stored_name(&p).as_deref(), Some(name));
    }
    // A large document: the name comes from its start (after the format and the image index).
    let mut big = Document::new("Big");
    let page = big.first_page().unwrap();
    for i in 0..3000 {
        big.create(page, figcraft_doc::NodeKind::Rectangle, &format!("Layer {i}"), 0.0, 0.0, 10.0, 10.0).unwrap();
    }
    let bytes = crate::save_bytes(&big, None).unwrap();
    assert!(serde_json::to_vec(&big).unwrap().len() as u64 > HEAD_BYTES, "larger than the part read");
    assert_eq!(stored_name(&file(&d, "big.figcraft", &bytes)).as_deref(), Some("Big"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn json_gives_its_name_when_it_comes_within_the_part_read() {
    let d = dir("json");
    let scrim = serde_json::to_vec(&Document::new("Kit")).unwrap();
    assert_eq!(stored_name(&file(&d, "kit.json", &scrim)).as_deref(), Some("Kit"));
    assert_eq!(stored_name(&file(&d, "kit.figcraft", &scrim)).as_deref(), Some("Kit"), "a bare JSON .figcraft");
    // A REST API file JSON keeps its name after the document: found when that comes early enough.
    let rest = br#"{"document": {"id": "0:0", "type": "DOCUMENT", "children": []}, "name": "From Figma"}"#;
    assert_eq!(stored_name(&file(&d, "rest.json", rest)).as_deref(), Some("From Figma"));
    // Past the part read: no name (the file goes by its file name).
    let mut long = br#"{"document": {"pad": ""#.to_vec();
    long.extend(std::iter::repeat_n(b'x', HEAD_BYTES as usize));
    long.extend_from_slice(br#""}, "name": "Late"}"#);
    assert_eq!(stored_name(&file(&d, "long.json", &long)), None);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_fig_file_gives_its_meta_file_name() {
    let d = dir("fig");
    let mut buf = Cursor::new(Vec::new());
    let mut z = zip::ZipWriter::new(&mut buf);
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    z.start_file("canvas.fig", stored).unwrap();
    z.write_all(b"fig-kiwi").unwrap();
    z.start_file("meta.json", stored).unwrap();
    z.write_all(br#"{"client_meta": {}, "file_name": "Brand kit"}"#).unwrap();
    z.finish().unwrap();
    assert_eq!(stored_name(&file(&d, "kit.fig", &buf.into_inner())).as_deref(), Some("Brand kit"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn nothing_for_other_kinds_missing_or_damaged_files() {
    let d = dir("none");
    assert_eq!(stored_name(&d.join("missing.figcraft")), None);
    assert_eq!(stored_name(&file(&d, "a.svg", br#"<svg name="x"/>"#)), None);
    assert_eq!(stored_name(&file(&d, "a.png", b"\x89PNG")), None);
    assert_eq!(stored_name(&file(&d, "junk.figcraft", b"PK\x03\x04 not really")), None);
    assert_eq!(stored_name(&file(&d, "nameless.json", br#"{"pages": []}"#)), None);
    assert_eq!(stored_name(&file(&d, "number.json", br#"{"name": 7}"#)), None);
    std::fs::remove_dir_all(&d).ok();
}
