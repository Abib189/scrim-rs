//! The MCP route is an agent's (Port 3): a layer the person locked refuses its edits, every edit
//! is in the change feed with its author, and a write against an old revision is refused when the
//! person changed the layer since.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use figcraft_engine::Session;
use figcraft_mcp::{Headless, Server};
use serde_json::{Value, json};

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

/// The person's document: a frame "Card" holding a rectangle "Swatch".
fn persons_session() -> (Session, u64, u64) {
    let mut s = Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 200, "height": 100, "name": "Card"})).unwrap());
    let r = id(&s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": f})).unwrap());
    s.execute("node.set", &json!({"id": r, "props": {"name": "Swatch"}})).unwrap();
    (s, f, r)
}

fn mcp(s: Session) -> Client {
    Client::with_server(Server::new(Box::new(Headless::with_session(s))))
}

#[test]
fn a_layer_the_person_locked_refuses_the_agent() {
    let (mut s, f, r) = persons_session();
    s.execute("object.toggleLock", &json!({"ids": [f]})).unwrap();
    let mut c = mcp(s);
    let e = c.err("set_properties", json!({"ids": [r], "props": {"fill": "#FF0000"}}));
    assert!(e.contains("\"Swatch\" is inside \"Card\", which is locked"), "{e}");
    let e = c.err("run_command", json!({"command": "edit.delete", "params": {"ids": [f]}}));
    assert!(e.contains("\"Card\" is locked"), "{e}");
    // Elsewhere the agent works as usual.
    c.create(json!({"type": "rectangle", "x": 400, "y": 0, "width": 10, "height": 10}));
}

#[test]
fn get_changes_names_the_person_and_the_agent() {
    let (s, _, r) = persons_session();
    let mut c = mcp(s);
    c.ok("set_properties", json!({"ids": [r], "props": {"fill": "#FF0000"}}));
    let v = c.ok("get_changes", json!({}));
    let changes = v["changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c["author"] == "person" && c["command"] == "frame.create"), "{v}");
    let last = changes.last().unwrap();
    assert_eq!((last["author"].as_str(), last["agent"].as_bool()), (Some("agent (mcp)"), Some(true)), "{v}");
    let now = v["revision"].as_u64().unwrap();
    assert!(c.ok("get_changes", json!({"since": now}))["changes"].as_array().unwrap().is_empty());
}

#[test]
fn a_write_against_an_old_revision_is_refused_when_the_person_changed_the_layer() {
    let (mut s, f, r) = persons_session();
    let read = s.changes_since(0).unwrap()["revision"].as_u64().unwrap();
    s.execute("node.set", &json!({"id": r, "props": {"width": 40}})).unwrap();
    let mut c = mcp(s);
    let e = c.err("set_properties", json!({"ids": [r], "props": {"fill": "#FF0000"}, "sinceRevision": read}));
    assert!(e.contains(&format!("changed since revision {read}: \"Swatch\" by the person")), "{e}");
    // Read again, then the same write goes through; an untouched layer never needed to.
    c.ok("run_command", json!({"command": "fill.set", "params": {"ids": [f], "color": "#FAFAFA"}, "sinceRevision": read}));
    let now = c.ok("get_changes", json!({}))["revision"].as_u64().unwrap();
    c.ok("set_properties", json!({"ids": [r], "props": {"fill": "#FF0000"}, "sinceRevision": now}));
}
