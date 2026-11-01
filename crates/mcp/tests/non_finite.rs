//! Fix AI LW-2 through MCP: a parameter that would put a number that isn't finite on a layer is
//! refused (JSON would save it as `null`, and the file wouldn't open again), and a file saved with
//! `null` numbers by an earlier build opens with them reset, its bytes untouched.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::{Value, json};

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-mcp-finite-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn text(c: &mut Client) -> u64 {
    c.ok("run_command", json!({"command": "file.new"}));
    let made = c.ok("create_node", json!({"type": "text", "x": 0, "y": 0, "text": "Hello", "name": "Title"}));
    made["id"].as_u64().or_else(|| made["nodes"][0]["id"].as_u64()).expect("the new layer's id")
}

#[test]
fn set_properties_refuses_a_line_height_or_letter_spacing_that_isnt_finite() {
    let d = dir("params");
    let mut c = Client::new();
    let id = text(&mut c);
    for props in [json!({"lineHeight": "NaN%"}), json!({"letterSpacing": "inf%"})] {
        let r = c.call("set_properties", json!({"ids": [id], "props": props}));
        assert_eq!(r["isError"], true, "{props} was accepted: {r}");
    }
    // The document saves and opens again.
    let file = d.join("title.scrim");
    c.ok("save_file", json!({"path": file.to_string_lossy()}));
    c.ok("open_file", json!({"path": file.to_string_lossy()}));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn open_file_opens_a_file_saved_with_null_numbers_and_leaves_it_as_it_is() {
    let d = dir("open");
    let file = d.join("old.scrim");
    let mut s = figcraft_engine::Session::with_document(figcraft_doc::Document::new("Old"));
    let r = s.execute("shape.rectangle", &json!({"x": 5, "y": 5, "width": 10, "height": 10})).unwrap();
    let id = r["id"].as_u64().unwrap();
    // A document.json on its own is a Scrim file too: no archive needed.
    let mut json: Value = serde_json::to_value(s.doc().unwrap()).unwrap();
    json["nodes"][id.to_string()]["opacity"] = Value::Null;
    json["nodes"][id.to_string()]["width"] = Value::Null;
    std::fs::write(&file, json.to_string()).unwrap();
    let before = std::fs::read(&file).unwrap();
    let mut c = Client::new();
    let opened = c.ok("open_file", json!({"path": file.to_string_lossy()}));
    assert_eq!(std::fs::read(&file).unwrap(), before, "opening wrote to the file");
    assert!(opened.to_string().contains("null"), "the repair is reported: {opened}");
    std::fs::remove_dir_all(&d).ok();
}
