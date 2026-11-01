//! Small zip bombs can't hang Scrim or fill its memory (Fix AI-3): every archive reader (the
//! `.scrim` and `.figcraft` file, `.fig`, Sketch, Penpot and FREE) refuses a member that would
//! unpack to far more than it holds, and an archive with too many parts, with a clear reason,
//! before reading on. Each bomb is built here: a member of 40 MiB of spaces, which deflate packs
//! into about 40 KB.

use std::io::{Cursor, Write};

use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::FormatError;
use crate::zip_limits::{MAX_ENTRIES, Unpack};

/// A zip of `members`, deflated.
fn zipped(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut w = zip::ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, data) in members {
        w.start_file(*name, opts).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap();
    buf.into_inner()
}

/// 40 MiB of spaces: about a thousand times what it packs to.
fn bomb() -> Vec<u8> {
    vec![b' '; 40 << 20]
}

/// `e` is the refusal of an archive that would unpack too far.
fn refused(what: &str, e: &FormatError) {
    assert!(e.to_string().contains("unpack"), "{what}: {e}");
    assert!(matches!(e, FormatError::Unpack(_)), "{what}: {e:?}");
}

#[test]
fn a_scrim_or_figcraft_file_whose_document_unpacks_a_thousandfold_is_refused() {
    let bytes = zipped(&[("document.json", bomb())]);
    assert!(bytes.len() < 1 << 20, "the bomb is small: {} bytes", bytes.len());
    refused(".scrim", &crate::load_bytes(&bytes).expect_err("refused"));
    let Err(e) = crate::load_bytes_with_versions(&bytes) else { panic!(".figcraft: not refused") };
    refused(".figcraft", &e);
}

#[test]
fn a_scrim_file_with_an_image_that_unpacks_a_thousandfold_is_refused() {
    let doc = crate::save_bytes(&figcraft_doc::Document::new("x"), None).unwrap();
    let mut z = zip::ZipArchive::new(Cursor::new(&doc[..])).unwrap();
    let json = crate::read_member(&mut z, "document.json", 1 << 20).unwrap();
    let bytes = zipped(&[("document.json", json), ("images/0123abcd", bomb())]);
    refused(".scrim image", &crate::load_bytes(&bytes).expect_err("refused"));
}

#[test]
fn a_fig_file_whose_canvas_unpacks_a_thousandfold_is_refused() {
    let bytes = zipped(&[("canvas.fig", bomb()), ("meta.json", b"{}".to_vec())]);
    refused(".fig", &crate::fig::import(&bytes, "bomb").expect_err("refused"));
}

#[test]
fn a_sketch_file_whose_document_unpacks_a_thousandfold_is_refused() {
    let bytes = zipped(&[("document.json", bomb()), ("meta.json", b"{}".to_vec())]);
    refused("Sketch", &crate::sketch::import(&bytes, "bomb").expect_err("refused"));
}

#[test]
fn a_penpot_file_whose_manifest_unpacks_a_thousandfold_is_refused() {
    let bytes = zipped(&[("manifest.json", bomb())]);
    refused("Penpot", &crate::penpot::import(&bytes, "bomb").expect_err("refused"));
}

#[test]
fn a_free_file_whose_document_unpacks_a_thousandfold_is_refused() {
    let bytes = zipped(&[("document.json", bomb()), ("meta.json", b"{}".to_vec())]);
    refused("FREE", &crate::free::import(&bytes, "bomb").expect_err("refused"));
}

#[test]
fn an_archive_with_more_parts_than_a_file_has_is_refused_before_any_is_read() {
    let names: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("p/{i}")).collect();
    let members: Vec<(&str, Vec<u8>)> = names.iter().map(|n| (n.as_str(), Vec::new())).collect();
    let bytes = zipped(&members);
    let e = crate::sketch::import(&bytes, "many").expect_err("refused");
    refused("entries", &e);
    assert!(matches!(e, FormatError::Unpack(Unpack::Entries { .. })), "{e:?}");
    refused("entries .scrim", &crate::load_bytes(&bytes).expect_err("refused"));
}

#[test]
fn honest_files_still_open() {
    // Packed JSON and pictures stay far within the limits: a saved document round-trips.
    let mut d = figcraft_doc::Document::new("Plain");
    let p = d.first_page().unwrap();
    for i in 0..500 {
        d.create(p, figcraft_doc::NodeKind::Rectangle, &format!("R{i}"), f64::from(i), 0.0, 10.0, 10.0).unwrap();
    }
    let (back, _) = crate::load_bytes(&crate::save_bytes(&d, None).unwrap()).unwrap();
    assert_eq!(back.node_count(), d.node_count());
}
