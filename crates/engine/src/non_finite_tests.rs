//! Fix AI LW-2 in the engine: an edit that would leave a number that isn't finite on a layer is
//! refused at its source (JSON saves NaN and infinity as `null`, and the file then won't open),
//! and a file saved with `null` numbers opens and imports through the registry, with one note and
//! its bytes untouched.

use serde_json::{Value, json};

use crate::Session;
use crate::params::id_of;

fn text(s: &mut Session) -> crate::NodeId {
    id_of(&s.execute("node.create", &json!({"type": "text", "x": 0, "y": 0, "text": "Hello"})).unwrap()["id"]).unwrap()
}

fn reopens(s: &Session) {
    let bytes = figcraft_format::save_bytes(s.doc().unwrap(), None).unwrap();
    assert!(figcraft_format::load_bytes(&bytes).is_ok(), "the saved file reopens");
}

#[test]
fn a_line_height_or_letter_spacing_that_isnt_finite_is_refused() {
    let mut s = Session::new();
    let t = text(&mut s);
    let before = s.doc().unwrap().clone();
    for props in [json!({"lineHeight": "NaN%"}), json!({"letterSpacing": "inf%"}), json!({"lineHeight": "1e999%"})] {
        let r = s.execute("node.set", &json!({"ids": [t], "props": props}));
        assert!(r.is_err(), "{props} was accepted");
    }
    assert_eq!(*s.doc().unwrap(), before, "nothing changed");
    reopens(&s);
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-engine-finite-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A file as an earlier build saved it with a NaN opacity and an infinite width: `null` for both.
fn damaged_file(d: &std::path::Path) -> (std::path::PathBuf, String) {
    let mut s = Session::new();
    let r = id_of(&s.execute("shape.rectangle", &json!({"x": 5, "y": 5, "width": 10, "height": 10, "name": "Badge"})).unwrap()["id"]).unwrap();
    let mut v = figcraft_format::to_json(s.doc().unwrap()).unwrap();
    v["nodes"][r.0.to_string()]["opacity"] = Value::Null;
    v["nodes"][r.0.to_string()]["width"] = Value::Null;
    let file = d.join("old.scrim");
    std::fs::write(&file, v.to_string()).unwrap();
    (file, "Badge".into())
}

#[test]
fn a_file_saved_with_null_numbers_opens_and_imports_with_a_note_and_stays_as_it_was() {
    let d = dir("open");
    let (file, name) = damaged_file(&d);
    let before = std::fs::read(&file).unwrap();
    let mut s = Session::empty();
    let opened = s.execute("file.open", &json!({"path": file.to_string_lossy()})).expect("opens");
    let repairs = opened["repairs"].as_array().cloned().unwrap_or_default();
    assert_eq!(repairs.iter().filter(|r| r.as_str().is_some_and(|r| r.contains("null"))).count(), 1, "{repairs:?}");
    let doc = s.doc().unwrap();
    let n = doc.nodes().find(|n| n.name == name).unwrap();
    assert_eq!((n.opacity, n.width), (1.0, 0.0));
    // Imported into another file too.
    s.execute("file.new", &json!({})).unwrap();
    s.execute("file.import", &json!({"path": file.to_string_lossy()})).expect("imports");
    assert!(s.doc().unwrap().nodes().any(|n| n.name == name));
    assert_eq!(std::fs::read(&file).unwrap(), before, "opening or importing wrote to the file");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_import_with_numbers_that_arent_finite_comes_in_with_them_reset() {
    let mut doc = figcraft_doc::Document::new("Imported");
    let p = doc.first_page().unwrap();
    let r = doc.create(p, figcraft_doc::NodeKind::Rectangle, "Card", 0.0, 0.0, 10.0, 10.0).unwrap();
    doc.node_mut(r).unwrap().opacity = f32::NAN;
    doc.node_mut(r).unwrap().height = f64::INFINITY;
    let reset = crate::importers::reset_non_finite(&mut doc);
    assert_eq!(reset, 2);
    let n = doc.node(r).unwrap();
    assert_eq!((n.opacity, n.height, n.width), (1.0, 0.0, 10.0));
    assert!(figcraft_doc::all_finite(n));
}
