//! Fix AI C-1: `save_file` over a FigCraft board (a file upstream saved with `kind: "board"`,
//! opened in Scrim as a design) is refused with a structured error saying what saving would lose
//! and how to go on, unless `force` is passed; a new path saves a copy and leaves the board file.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::{Value, json};

/// A client with a board file `Plan.scrim` open and edited, its path and bytes.
fn opened(name: &str) -> (Client, std::path::PathBuf, std::path::PathBuf, Vec<u8>) {
    let d = std::env::temp_dir().join(format!("figcraft-mcp-board-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut doc = figcraft_doc::Document::new("Plan");
    let page = doc.first_page().unwrap();
    let sticky = doc.create(page, figcraft_doc::NodeKind::Frame, "Sticky", 0.0, 0.0, 200.0, 200.0).unwrap();
    let mut v = serde_json::to_value(&doc).unwrap();
    v["kind"] = json!("board");
    v["nodes"][sticky.0.to_string()]["board"] = json!({"type": "sticky"});
    let file = d.join("Plan.scrim");
    std::fs::write(&file, serde_json::to_vec(&v).unwrap()).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    let mut c = Client::new();
    c.ok("open_file", json!({"path": file.to_string_lossy()}));
    c.run("shape.rectangle", json!({"x": 300, "y": 0, "width": 10, "height": 10}));
    (c, d, file, bytes)
}

#[test]
fn save_file_over_a_board_is_refused_with_what_to_do() {
    let (mut c, d, file, bytes) = opened("refused");
    let text = c.err("save_file", json!({}));
    let e: Value = serde_json::from_str(&text).expect("a structured error");
    assert_eq!(e["error"], "boardFile", "{e}");
    assert_eq!(e["path"], file.to_string_lossy().as_ref());
    assert!(e["resolve"]["saveAnyway"].as_str().unwrap().contains("force: true"), "{e}");
    assert!(e["resolve"]["saveCopy"].as_str().unwrap().contains(".scrim"), "{e}");
    // The same path given explicitly is the same file.
    c.err("save_file", json!({"path": file.to_string_lossy()}));
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "the board file is untouched");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn force_saves_it_as_a_design_and_a_new_path_saves_a_copy() {
    let (mut c, d, file, bytes) = opened("force");
    let copy = d.join("Plan copy.scrim");
    c.ok("save_file", json!({"path": copy.to_string_lossy()}));
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "a copy leaves the board file as it was");
    assert!(copy.exists());

    let (mut c, d2, file, bytes) = opened("force2");
    c.ok("save_file", json!({"force": true}));
    assert_ne!(std::fs::read(&file).unwrap(), bytes, "force writes over it");
    assert_eq!(c.run("file.diskStatus", json!({}))["boardFile"], false, "a design from now on");
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&d2);
}
