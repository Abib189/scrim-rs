//! Page ruler guides survive save and reopen (Fix AI LW-1, upstream FigCraft 652bf35b), and a file
//! the old serialisation already damaged still opens with its guides: one saved after a reopen
//! has the page's `guides` key twice, which used to refuse the whole file ("duplicate field").

use std::io::{Cursor, Read, Write};

use figcraft_doc::{Document, Guide, NodeKind};

use crate::{load_bytes, save_bytes};

fn guide(vertical: bool, offset: f64) -> Guide {
    Guide { vertical, offset }
}

/// `bytes`, a saved file, with `document.json` replaced by `edit` of its text.
fn with_document(bytes: &[u8], edit: impl Fn(&str) -> String) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Cursor::new(Vec::new());
    let mut w = zip::ZipWriter::new(&mut out);
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if f.name() == "document.json" {
            data = edit(std::str::from_utf8(&data).unwrap()).into_bytes();
        }
        w.start_file(f.name().to_string(), zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(&data).unwrap();
    }
    w.finish().unwrap();
    out.into_inner()
}

#[test]
fn page_guides_survive_save_and_reopen_twice() {
    let mut d = Document::new("Guides");
    let page = d.first_page().unwrap();
    let f = d.create(page, NodeKind::Frame, "F", 0.0, 0.0, 100.0, 100.0).unwrap();
    d.node_mut(page).unwrap().guides_mut().push(guide(true, 120.0));
    d.node_mut(f).unwrap().guides_mut().push(guide(false, 40.0));
    let (mut back, issues) = load_bytes(&save_bytes(&d, None).unwrap()).unwrap();
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(back.node(page).unwrap().guides(), [guide(true, 120.0)], "the page's guide was lost");
    assert_eq!(back.node(f).unwrap().guides(), [guide(false, 40.0)]);
    // A guide added after the reopen: the next save opens, with both.
    back.node_mut(page).unwrap().guides_mut().push(guide(false, 64.0));
    let (again, _) = load_bytes(&save_bytes(&back, None).expect("saves")).expect("the second save opens");
    assert_eq!(again.node(page).unwrap().guides(), [guide(true, 120.0), guide(false, 64.0)]);
}

#[test]
fn a_file_saved_with_the_page_guides_key_twice_opens_with_its_guides_and_saves_a_valid_file() {
    let d = Document::new("Damaged");
    let page = d.first_page().unwrap();
    // The shape the old serialisation wrote after a reopen: the page kind's list (guides added
    // since) and the layer's own list (the ones read back), both as `guides`.
    let damaged = with_document(&save_bytes(&d, None).unwrap(), |json| {
        json.replacen(
            r#""type":"page""#,
            r#""type":"page","guides":[{"vertical":false,"offset":64.0}],"guides":[{"vertical":true,"offset":120.0}]"#,
            1,
        )
    });
    let before = damaged.clone();
    let (opened, _) = load_bytes(&damaged).expect("a file in the damaged shape opens");
    assert_eq!(damaged, before, "opening changed the bytes");
    assert_eq!(opened.node(page).unwrap().guides(), [guide(false, 64.0), guide(true, 120.0)]);
    // Its next save is a valid file with the guides once each.
    let saved = save_bytes(&opened, None).unwrap();
    let (again, issues) = load_bytes(&saved).expect("the next save opens");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(again.node(page).unwrap().guides(), [guide(false, 64.0), guide(true, 120.0)]);
}
