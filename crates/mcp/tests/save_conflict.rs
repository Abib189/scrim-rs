//! Fix AI LW-4 through MCP: `save_file` never silently replaces what another app wrote to the
//! file. Without `force` it returns a structured `fileChangedOnDisk` error naming the file and the
//! ways out, and writes nothing; with `force: true` it saves.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::{Value, json};

#[test]
fn save_file_reports_a_change_another_app_made_and_overwrites_only_with_force() {
    let d = std::env::temp_dir().join(format!("figcraft-mcp-conflict-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let file = d.join("doc.scrim");
    let mut c = Client::new();
    c.ok("run_command", json!({"command": "file.new"}));
    c.ok("create_node", json!({"type": "rectangle", "x": 0, "y": 0, "width": 40, "height": 30, "name": "Mine"}));
    c.ok("save_file", json!({"path": file.to_string_lossy()}));
    c.ok("create_node", json!({"type": "rectangle", "x": 50, "y": 0, "width": 10, "height": 10, "name": "Unsaved"}));
    // Another app writes the file.
    let mut other = figcraft_engine::Session::new();
    other.execute("shape.rectangle", &json!({"width": 70, "height": 70, "name": "Theirs"})).unwrap();
    let theirs = figcraft_engine::cmd::file::file_bytes(other.ds().unwrap()).unwrap();
    std::fs::write(&file, &theirs).unwrap();

    let r = c.call("save_file", json!({}));
    assert_eq!(r["isError"], true, "{r}");
    let e: Value = serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).expect("a structured error");
    assert_eq!(e["error"], "fileChangedOnDisk");
    assert_eq!(e["path"], file.to_string_lossy().as_ref());
    assert!(e["resolve"]["overwrite"].as_str().unwrap().contains("force"));
    assert_eq!(std::fs::read(&file).unwrap(), theirs, "nothing was written");

    c.ok("save_file", json!({"force": true}));
    let saved = figcraft_engine::cmd::file::load(&file.to_string_lossy()).unwrap().0;
    assert!(saved.nodes().any(|n| n.name == "Unsaved"));
    std::fs::remove_dir_all(&d).ok();
}
