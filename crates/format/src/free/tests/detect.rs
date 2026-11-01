//! Telling FREE files from other zips, the preview, and hostile input.

use super::super::preview;
use super::*;
use crate::FormatError;

const SKETCH_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sketch/sample.sketch");

fn bytes_of(members: &[(&str, &[u8])]) -> Vec<u8> {
    zip(&members.iter().map(|(n, b)| (n.to_string(), b.to_vec())).collect::<Vec<_>>())
}

#[test]
fn free_files_are_recognised() {
    let with_page = free(&[page(1, "Page 1", vec![layer("RECT", 2, "R", [0.0, 0.0, 1.0, 1.0])])], json!({}), &[]);
    assert!(is_free(&with_page));
    let tiny = webp(2, 2, |_, _| [255, 0, 0, 255]);
    let empty_with_preview = free(&[], json!({}), &[("preview.webp", &tiny)]);
    assert!(is_free(&empty_with_preview));
}

#[test]
fn other_files_are_not_free() {
    let sketch = std::fs::read(SKETCH_FIXTURE).unwrap();
    assert!(crate::sketch::is_sketch(&sketch));
    assert!(!is_free(&sketch), "the committed Sketch sample");
    // A Sketch-shaped zip with FREE's preview name is still Sketch: its document.json has `_class`.
    let doc = br#"{"_class":"document","pages":[]}"#;
    assert!(!is_free(&bytes_of(&[("document.json", doc), ("meta.json", br#"{"version":146}"#), ("preview.webp", b"RIFF")])));
    // Sketch's meta.json keys.
    let meta = br#"{"version":146,"compatibilityVersion":99,"app":"com.bohemiancoding.sketch3"}"#;
    assert!(!is_free(&bytes_of(&[("document.json", b"{}"), ("meta.json", meta), ("preview.webp", b"RIFF")])));
    // Scrim's own file.
    let native = crate::save_bytes(&Document::new("Native"), None).unwrap();
    assert!(native.starts_with(b"PK"));
    assert!(!is_free(&native));
    let figcraft = br#"{"format":"figcraft","version":1}"#;
    assert!(!is_free(&bytes_of(&[("document.json", figcraft), ("meta.json", b"{}"), ("preview.webp", b"RIFF")])));
    // A .fig zip and a Penpot export.
    assert!(!is_free(&bytes_of(&[("canvas.fig", b"fig-kiwi"), ("meta.json", br#"{"file_name":"x"}"#), ("thumbnail.png", b"")])));
    assert!(!is_free(&bytes_of(&[("manifest.json", br#"{"type":"penpot/export-files","version":1}"#)])));
    // A FREE-shaped zip without FREE's marks: no preview, and no page with `_t` layers.
    assert!(!is_free(&bytes_of(&[("document.json", b"{}"), ("meta.json", br#"{"version":11}"#)])));
    assert!(!is_free(b""));
    assert!(!is_free(b"PK\x03\x04 not really a zip"));
    assert!(!is_free(&[0u8; 64]));
}

#[test]
fn the_sample_is_not_sketch() {
    // FREE's document.json has no `"_class": "document"`, so the Sketch sniff says no; the
    // registry checks FREE first either way.
    assert!(!crate::sketch::is_sketch(&super::sample::sample()));
}

#[test]
fn the_preview_is_read_when_present() {
    let tiny = webp(3, 2, |x, _| [0, (x * 100) as u8, 200, 255]);
    let bytes = free(&[], json!({}), &[("preview.webp", &tiny)]);
    assert_eq!(preview(&bytes), Some(tiny.clone()));
    assert!(image::load_from_memory(&tiny).is_ok(), "the test preview is a valid WebP");
    assert_eq!(preview(&free(&[], json!({}), &[])), None);
    assert_eq!(preview(b"not a zip"), None);
}

#[test]
fn garbage_newer_and_truncated_files_are_errors() {
    assert!(import(b"garbage", "x").is_err());
    assert!(import(b"", "x").is_err());
    let newer = free_at(12, &[page(1, "Page 1", vec![])], json!({}), &[]);
    assert!(matches!(import(&newer, "x"), Err(FormatError::Newer(12))));
    let good = free(&[page(1, "Page 1", vec![layer("RECT", 2, "R", [0.0, 0.0, 1.0, 1.0])])], json!({}), &[]);
    for cut in [10, good.len() / 2, good.len() - 10] {
        assert!(import(&good[..cut], "x").is_err(), "cut at {cut}");
    }
    let bad_version = bytes_of(&[("document.json", b"{}"), ("meta.json", br#"{"version":"eleven"}"#)]);
    assert!(matches!(import(&bad_version, "x"), Err(FormatError::Corrupt(_))));
    let sketch = std::fs::read(SKETCH_FIXTURE).unwrap();
    assert!(matches!(import(&sketch, "x"), Err(FormatError::Corrupt(_))));
    let older = free_at(9, &[], json!({}), &[]);
    let (_, w) = import(&older, "x").unwrap();
    assert!(has(&w, Severity::Info, "", "version 9"), "{w:?}");
}

#[test]
fn hostile_json_gives_warnings_not_panics() {
    let weird = vec![
        json!(42),
        json!({"_t": 7}),
        json!({"_t": "RECT", "name": ["x"], "frame": "everywhere", "size": [1e300, -5], "fill": 12, "fills": {"no": 1}}),
        json!({"_t": "RECT", "points": [[0], "x", [1, 2, 3, 4, 5]], "cornerRadius": [1, 2], "opacity": "half", "blur": [], "shadows": [1, {"offset": "x"}]}),
        json!({"_t": "TEXT", "text": 5, "inlines": [{"range": [9, 2]}, {"range": "all"}, 3], "font": {}, "variation": [1]}),
        json!({"_t": "FRAME", "autoLayout": {"padding": [1, 2, 3], "spacing": "wide"}, "layers": {"not": "a list"}, "constraints": "XYZ"}),
        json!({"_t": "INSTANCE", "componentId": 5, "overrides": [{"target": "x"}]}),
        json!({"_t": "GROUP", "layers": [{"_t": "GROUP"}], "custom": [1, 2]}),
    ];
    let (doc, w) = one(weird);
    assert!(!w.is_empty());
    assert!(doc.nodes().count() > 1);
    // Deep nesting: 50 groups import; 400 pass the JSON reader's nesting limit and the page is
    // reported, not read.
    let nest = |levels: u32| {
        let mut deep = layer("RECT", 1, "Leaf", [0.0, 0.0, 1.0, 1.0]);
        for i in 0..levels {
            deep = container("GROUP", 2 + i, "G", [0.0, 0.0, 1.0, 1.0], vec![deep]);
        }
        free(&[page(1, "Page 1", vec![deep])], json!({}), &[])
    };
    let (doc, w) = import(&nest(50), "x").unwrap();
    assert!(doc.nodes().any(|n| n.name == "Leaf") && w.is_empty(), "{w:?}");
    let (_, w) = import(&nest(400), "x").unwrap();
    assert!(w.iter().any(|x| x.severity == Severity::Dropped && x.message.contains("couldn't be read")), "{w:?}");
    // Malformed pages and documents.
    let bad_page = bytes_of(&[("document.json", br#"{"pages":["a"]}"#), ("meta.json", br#"{"version":11}"#), ("pages/a.json", b"[1,2")]);
    let (_, w) = import(&bad_page, "x").unwrap();
    assert!(w.iter().any(|x| x.severity == Severity::Dropped && x.message.contains("couldn't be read")), "{w:?}");
    let not_object = bytes_of(&[("document.json", b"[]"), ("meta.json", b"{}")]);
    assert!(import(&not_object, "x").is_err());
}
