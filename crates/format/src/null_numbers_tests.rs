//! A NaN or infinite number never makes a file that won't open (Fix AI LW-2, with upstream
//! FigCraft 8c08035b). JSON has no such numbers: `serde_json` wrote them as `null`, and reading
//! the file back failed on the first one. Saving now writes 0 in their place, and a file saved
//! before opens with each such `null` reset to its property's default, and one warning.

use std::io::{Cursor, Read, Write};

use figcraft_doc::{Document, NodeId, NodeKind};
use serde_json::Value;

use crate::{load_bytes, load_bytes_with_versions, save_bytes};

fn rect_doc() -> (Document, NodeId) {
    let mut d = Document::new("Numbers");
    let p = d.first_page().unwrap();
    let r = d.create(p, NodeKind::Rectangle, "Card", 40.0, 10.0, 200.0, 120.0).unwrap();
    d.node_mut(r).unwrap().opacity = 0.5;
    (d, r)
}

/// `bytes`, a saved file, with `document.json` replaced by `edit` of its value.
fn with_document(bytes: &[u8], edit: impl Fn(&mut Value)) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut w = zip::ZipWriter::new(&mut out);
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if f.name() == "document.json" {
            let mut v: Value = serde_json::from_slice(&data).unwrap();
            edit(&mut v);
            data = serde_json::to_vec(&v).unwrap();
        }
        w.start_file(f.name().to_string(), zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(&data).unwrap();
    }
    w.finish().unwrap();
    out.into_inner()
}

#[test]
fn a_document_holding_nan_and_infinity_saves_a_file_that_reopens() {
    let (mut d, r) = rect_doc();
    let n = d.node_mut(r).unwrap();
    n.width = f64::NAN;
    n.height = f64::INFINITY;
    let saved = save_bytes(&d, None).expect("saves");
    let (back, issues) = load_bytes(&saved).expect("the saved file reopens");
    assert!(issues.is_empty(), "{issues:?}");
    let n = back.node(r).unwrap();
    assert_eq!((n.width, n.height), (0.0, 0.0), "written as 0");
    assert_eq!(n.opacity, 0.5, "the other numbers are as they were");
}

/// The shape earlier builds wrote: `null` for NaN or infinity. Here a property with a default
/// (opacity: 1), a number the layer must have (width: 0), and one inside a list (the
/// transform's x: 0); a `null` that reads fine (no auto layout) stays.
#[test]
fn a_file_with_nulls_where_numbers_belong_opens_with_their_defaults_and_one_warning() {
    let (d, r) = rect_doc();
    let damaged = with_document(&save_bytes(&d, None).unwrap(), |v| {
        let n = &mut v["nodes"][r.0.to_string()];
        n["opacity"] = Value::Null;
        n["width"] = Value::Null;
        n["transform"][4] = Value::Null;
        n["layout"] = Value::Null;
    });
    let before = damaged.clone();
    let loaded = load_bytes_with_versions(&damaged).expect("a file with nulls for numbers opens");
    assert_eq!(damaged, before, "opening changed the bytes");
    let n = loaded.doc.node(r).unwrap();
    assert_eq!(n.opacity, 1.0, "a property with a default takes it");
    assert_eq!(n.width, 0.0, "a number the layer must have reads as 0");
    assert_eq!(n.height, 120.0);
    assert_eq!(n.transform.as_coeffs()[4], 0.0);
    assert_eq!(n.transform.as_coeffs()[5], 10.0);
    assert!(n.layout.is_none());
    assert_eq!(loaded.repaired_numbers, 3);
    let warnings: Vec<&String> = loaded.issues.iter().filter(|i| i.contains("null")).collect();
    assert_eq!(warnings.len(), 1, "one warning: {:?}", loaded.issues);
    assert!(warnings[0].contains('3'), "{}", warnings[0]);
    // Its next save is a file with no nulls for numbers.
    let (again, issues) = load_bytes(&save_bytes(&loaded.doc, None).unwrap()).expect("the next save opens");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(again.node(r).unwrap().opacity, 1.0);
}

#[test]
fn a_file_whose_damage_isnt_a_null_number_is_still_refused() {
    let (d, r) = rect_doc();
    let bad = with_document(&save_bytes(&d, None).unwrap(), |v| v["nodes"][r.0.to_string()]["width"] = Value::from("wide"));
    assert!(load_bytes(&bad).is_err());
}
