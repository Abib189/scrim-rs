//! Stage E3: a frame's handoff status, now drawn as an icon, still round-trips unchanged through
//! MCP (`set_properties`, `get_node`), undo and redo, and save and open. The file format is as
//! before: the shipped example, saved before Stage E, opens with its status.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::{Value, json};

fn status(c: &mut Client, id: &Value) -> (Value, Value) {
    let n = c.node(id);
    (n["devStatus"].clone(), n.get("devChanged").cloned().unwrap_or(Value::Null))
}

#[test]
fn the_status_round_trips_through_mcp_undo_redo_and_the_file() {
    let mut c = Client::new();
    let f = c.create(json!({"type": "frame", "x": 0, "y": 0, "width": 300, "height": 200, "name": "Screen"}));
    let child = c.create(json!({"type": "rectangle", "x": 10, "y": 10, "width": 40, "height": 40, "parent": f}));
    let none = status(&mut c, &f).0;
    assert!(none.is_null() || none == "none", "{none}");
    c.ok("set_properties", json!({"ids": [f], "props": {"devStatus": "readyForDev"}}));
    assert_eq!(status(&mut c, &f).0, "readyForDev");
    // Undo and redo.
    c.run("edit.undo", json!({}));
    let undone = status(&mut c, &f).0;
    assert!(undone.is_null() || undone == "none", "{undone}");
    c.run("edit.redo", json!({}));
    assert_eq!(status(&mut c, &f).0, "readyForDev");
    // A change inside it: "Changed".
    c.ok("set_properties", json!({"ids": [child], "props": {"width": 60}}));
    assert_eq!(status(&mut c, &f), (json!("readyForDev"), json!(true)));
    // Save and open: the same status, Changed kept.
    let dir = std::env::temp_dir().join(format!("figcraft-dev-status-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("status.figcraft");
    c.ok("save_file", json!({"path": file.to_string_lossy()}));
    c.ok("open_file", json!({"path": file.to_string_lossy()}));
    assert_eq!(status(&mut c, &f), (json!("readyForDev"), json!(true)));
    c.ok("set_properties", json!({"ids": [f], "props": {"devStatus": "completed"}}));
    c.ok("save_file", json!({"path": file.to_string_lossy()}));
    c.ok("open_file", json!({"path": file.to_string_lossy()}));
    assert_eq!(status(&mut c, &f).0, "completed");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_saved_before_stage_e_opens_with_its_status() {
    let mut c = Client::new();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ridgeline-app.figcraft");
    c.ok("open_file", json!({"path": path}));
    // Home — Light (68) is marked Ready for dev in the shipped file.
    assert_eq!(status(&mut c, &json!(68)).0, "readyForDev");
}
