//! Fix AI C-1: a file upstream FigCraft saved as a board opens as a design with a notice, and is
//! never written over without the person choosing it: Save asks (Save as a copy, Save anyway,
//! Cancel), `file.save {dropBoard: true}` writes it without its board marks, Save as a new file
//! never asks, and auto-save leaves it alone.

use std::io::{Read, Write};

use serde_json::{Value, json};

use crate::Session;
use crate::cmd::file::FileSnapshot;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-engine-board-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Rewrite the `document.json` of the archive at `file` with `edit`.
fn rewrite(file: &std::path::Path, edit: impl Fn(&mut Value)) {
    let bytes = std::fs::read(file).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if f.name() == "document.json" {
            let mut v: Value = serde_json::from_slice(&data).unwrap();
            edit(&mut v);
            data = serde_json::to_vec(&v).unwrap();
        }
        out.start_file(f.name(), zip::write::SimpleFileOptions::default()).unwrap();
        out.write_all(&data).unwrap();
    }
    std::fs::write(file, out.finish().unwrap().into_inner()).unwrap();
}

/// A .scrim file at `file` as upstream would write a board with one sticky note; its bytes.
fn board(file: &std::path::Path) -> Vec<u8> {
    let mut s = Session::new();
    let r = s.execute("node.create", &json!({"type": "frame", "x": 0, "y": 0, "width": 200, "height": 200, "name": "Sticky"})).unwrap();
    let id = serde_json::to_value(crate::params::id_of(&r["id"]).unwrap()).unwrap();
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    rewrite(file, |v| {
        v["kind"] = json!("board");
        v["nodes"][id.to_string()]["board"] = json!({"type": "sticky", "author": "Ana"});
    });
    std::fs::read(file).unwrap()
}

fn requests(s: &Session, kind: &str) -> usize {
    s.ui_requests.iter().filter(|r| r.kind == kind).count()
}

fn marks(file: &std::path::Path) -> Option<figcraft_format::BoardMarks> {
    figcraft_format::load_bytes_with_versions(&std::fs::read(file).unwrap()).unwrap().board
}

/// A session with the board file at `file` open and edited (unsaved).
fn opened(file: &std::path::Path) -> Session {
    let mut s = Session::new();
    s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 300, "y": 0, "width": 10, "height": 10, "name": "Mine"})).unwrap();
    s.ui_requests.clear();
    s
}

#[test]
fn opening_a_board_file_says_so_once_and_writes_nothing() {
    let d = dir("open");
    let file = d.join("Plan.scrim");
    let bytes = board(&file);
    assert_eq!(marks(&file), Some(figcraft_format::BoardMarks { kind: true, items: 1 }), "the crafted file is a board");
    let mut s = Session::new();
    s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
    assert_eq!(requests(&s, "dialog.boardFile"), 1, "the window is asked to show the notice");
    let r = s.ui_requests.iter().find(|r| r.kind == "dialog.boardFile").unwrap();
    assert_eq!(r.data["name"], "Plan");
    assert_eq!(s.execute("file.diskStatus", &json!({})).unwrap()["boardFile"], true);
    // Opening it again only switches to it.
    s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
    assert_eq!(requests(&s, "dialog.boardFile"), 1, "once per open");
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "opening writes nothing");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_design_file_opens_without_the_notice() {
    let d = dir("design");
    let file = d.join("Design.scrim");
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    let mut s = Session::new();
    s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
    assert_eq!(requests(&s, "dialog.boardFile"), 0);
    assert_eq!(s.execute("file.diskStatus", &json!({})).unwrap()["boardFile"], false);
    s.execute("shape.rectangle", &json!({"x": 20, "y": 0, "width": 10, "height": 10})).unwrap();
    s.execute("file.save", &json!({})).expect("a design saves in place");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn save_asks_and_writes_nothing() {
    let d = dir("save");
    let file = d.join("Plan.scrim");
    let bytes = board(&file);
    let mut s = opened(&file);
    assert!(s.execute("file.save", &json!({})).is_err(), "a plain save is refused");
    assert_eq!(requests(&s, "dialog.saveBoard"), 1, "the window is asked to show the three choices");
    // Save as onto the board file itself asks the same.
    assert!(s.execute("file.saveAs", &json!({"path": file.to_string_lossy()})).is_err());
    assert_eq!(requests(&s, "dialog.saveBoard"), 2);
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "the board file is untouched");
    assert!(s.ds().unwrap().is_dirty(), "the edits are still unsaved");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn save_anyway_writes_it_as_a_design() {
    let d = dir("anyway");
    let file = d.join("Plan.scrim");
    board(&file);
    let mut s = opened(&file);
    s.execute("file.save", &json!({"dropBoard": true})).expect("Save anyway saves");
    assert_eq!(marks(&file), None, "the board marks are gone, as the notice said");
    assert!(!s.ds().unwrap().is_dirty());
    // It is a design file from now on: the next save doesn't ask.
    s.execute("shape.rectangle", &json!({"x": 40, "y": 0, "width": 10, "height": 10})).unwrap();
    s.ui_requests.clear();
    s.execute("file.save", &json!({})).expect("saves in place");
    assert_eq!(requests(&s, "dialog.saveBoard"), 0);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn save_as_a_copy_leaves_the_board_file_as_it_was() {
    let d = dir("copy");
    let file = d.join("Plan.scrim");
    let bytes = board(&file);
    let copy = d.join("Plan copy.scrim");
    let mut s = opened(&file);
    s.execute("file.saveAs", &json!({"path": copy.to_string_lossy()})).expect("a copy saves");
    assert_eq!(requests(&s, "dialog.saveBoard"), 0, "a new file never asks");
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "the board file is untouched");
    assert_eq!(marks(&copy), None, "the copy is a design");
    assert_eq!(s.ds().unwrap().path.as_deref(), Some(copy.to_string_lossy().as_ref()), "the tab is the copy now");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn auto_save_leaves_a_board_file_alone() {
    let d = dir("auto");
    let file = d.join("Plan.scrim");
    let bytes = board(&file);
    let mut s = opened(&file);
    let snap = FileSnapshot::of(s.ds().unwrap());
    let encoded = snap.encode().unwrap();
    assert!(!snap.save(&mut s, &encoded).expect("no error"), "nothing written");
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "the board file is untouched");
    assert!(s.ds().unwrap().auto_save_paused(), "auto-save skips it from now on");
    assert!(s.ds().unwrap().is_dirty());
    let _ = std::fs::remove_dir_all(&d);
}
