//! Fix AD: Scrim starts on its Home screen with no document. An MCP tool that needs one answers
//! with a structured error saying none is open and how to open one, rather than crashing or making
//! a document on the agent's behalf; open_file then works.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use figcraft_engine::Session;
use figcraft_mcp::{Headless, Server};
use serde_json::{Value, json};

fn home() -> Client {
    Client::with_server(Server::new(Box::new(Headless::with_session(Session::empty()))))
}

fn example() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ridgeline-app.figcraft").to_string_lossy().into_owned()
}

/// The error, parsed: `no_document`, its message, and how to open one.
fn no_document(e: &str) -> Value {
    let v: Value = serde_json::from_str(e).unwrap_or_else(|_| panic!("structured, not plain text: {e}"));
    assert_eq!(v["error"], "no_document", "{v}");
    assert!(v["message"].as_str().unwrap().contains("No document is open"), "{v}");
    let how = v["howToOpen"].as_str().unwrap();
    assert!(how.contains("open_file") && how.contains("file.new"), "{v}");
    v
}

#[test]
fn a_tool_that_needs_a_document_says_none_is_open_and_how_to_open_one() {
    let mut c = home();
    no_document(&c.err("create_node", json!({"type": "rectangle", "x": 0, "y": 0, "width": 10, "height": 10})));
    no_document(&c.err("run_command", json!({"command": "select.all"})));
    // Nothing was made to satisfy the agent: asking again is refused the same way.
    no_document(&c.err("create_node", json!({"type": "rectangle"})));
}

#[test]
fn open_file_works_with_only_home_showing() {
    let mut c = home();
    no_document(&c.err("create_node", json!({"type": "rectangle"})));
    c.ok("open_file", json!({"path": example()}));
    let doc = c.ok("inspect_document", json!({"depth": 0}));
    let doc = doc.get("document").unwrap_or(&doc);
    assert_eq!(doc["name"], "Ridgeline app", "{doc}");
    c.create(json!({"type": "rectangle", "x": 0, "y": 0, "width": 10, "height": 10}));
}
