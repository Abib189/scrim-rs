//! Fix AG through MCP: `save_file` writes `.scrim`, `open_file` reads it back, a `.figcraft` opens
//! and is never written over, and an import (a Sketch file) returns its warnings with their layers.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::json;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-mcp-scrim-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn save_file_writes_scrim_and_open_file_reads_it_back() {
    let d = dir("roundtrip");
    let mut c = Client::new();
    c.ok("run_command", json!({"command": "file.new"}));
    c.ok("create_node", json!({"type": "rectangle", "x": 0, "y": 0, "width": 40, "height": 30, "name": "Badge"}));
    let file = d.join("badge.scrim");
    let saved = c.ok("save_file", json!({"path": file.to_string_lossy()}));
    assert_eq!(saved["path"], file.to_string_lossy().as_ref());
    let bytes = std::fs::read(&file).unwrap();
    assert!(bytes.starts_with(b"PK\x03\x04"), "the .scrim container (a zip)");
    let opened = c.ok("open_file", json!({"path": file.to_string_lossy()}));
    assert!(opened["document"]["pages"][0]["children"].to_string().contains("Badge"));
    // A .figcraft path is refused, never written.
    let refused = c.call("save_file", json!({"path": d.join("badge.figcraft").to_string_lossy()}));
    assert_eq!(refused["isError"], true);
    assert!(!d.join("badge.figcraft").exists());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_figcraft_opens_and_save_without_a_path_is_refused_until_a_scrim_is_given() {
    let d = dir("figcraft");
    let old = d.join("old.figcraft");
    let mut s = figcraft_engine::Session::with_document(figcraft_doc::Document::new("Old"));
    s.execute("shape.rectangle", &json!({"width": 10, "height": 10})).unwrap();
    std::fs::write(&old, figcraft_engine::cmd::file::file_bytes(s.ds().unwrap()).unwrap()).unwrap();
    let before = std::fs::read(&old).unwrap();
    let mut c = Client::new();
    let opened = c.ok("open_file", json!({"path": old.to_string_lossy()}));
    assert_eq!(opened["document"]["name"], "Old");
    assert_eq!(c.call("save_file", json!({}))["isError"], true, "never written over");
    c.ok("save_file", json!({"path": d.join("old.scrim").to_string_lossy()}));
    assert_eq!(std::fs::read(&old).unwrap(), before);
    assert!(d.join("old.scrim").is_file());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn open_file_imports_a_sketch_file_with_its_warnings() {
    let sample = concat!(env!("CARGO_MANIFEST_DIR"), "/../format/tests/fixtures/sketch/sample.sketch");
    let before = std::fs::read(sample).unwrap();
    let mut c = Client::new();
    let opened = c.ok("open_file", json!({"path": sample}));
    let warnings = opened["opened"]["warnings"].as_array().unwrap();
    assert!(!warnings.is_empty(), "{opened}");
    for w in warnings {
        assert!(matches!(w["severity"].as_str(), Some("dropped" | "approximated" | "info")), "{w}");
        assert!(w["path"].is_string() && w["message"].as_str().is_some_and(|m| !m.is_empty()), "{w}");
    }
    assert!(warnings.iter().any(|w| w["path"].as_str().is_some_and(|p| p.contains('/'))), "warnings name their layers: {warnings:?}");
    assert!(opened["document"]["path"].is_null(), "an import has no file of its own");
    assert_eq!(std::fs::read(sample).unwrap(), before, "the source is never written");
}
