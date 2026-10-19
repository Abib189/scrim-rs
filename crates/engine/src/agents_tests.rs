//! Agents and people on one document: locks bind agents on every route, the stale-write guard,
//! the change feed with authors, and agents' undo labels (Port 3).

use serde_json::{Value, json};

use crate::Session;
use crate::agents::Author;
use crate::params::id_of;

fn agent() -> Author {
    Author::agent("mcp")
}

fn id(v: &Value) -> figcraft_doc::NodeId {
    id_of(&v["id"]).unwrap()
}

/// A frame holding a rectangle, both made by the person.
fn scene(s: &mut Session) -> (figcraft_doc::NodeId, figcraft_doc::NodeId) {
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 200, "height": 100, "name": "Card"})).unwrap());
    let r = id(&s.execute("shape.rectangle", &json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": f})).unwrap());
    s.execute("node.set", &json!({"id": r, "props": {"name": "Swatch"}})).unwrap();
    (f, r)
}

#[test]
fn a_locked_layer_refuses_an_agent_but_not_the_person() {
    let mut s = Session::new();
    let (_, r) = scene(&mut s);
    s.execute("object.toggleLock", &json!({"ids": [r]})).unwrap();
    let before = s.doc().unwrap().clone();
    let e = s.execute_as(agent(), None, "fill.set", &json!({"ids": [r], "color": "#FF0000"})).unwrap_err().to_string();
    assert!(e.contains("\"Swatch\" is locked: an agent can't change a locked layer (the person can unlock it)"), "{e}");
    assert_eq!(*s.doc().unwrap(), before, "rolled back");
    // Deleting or unlocking it is an edit too.
    assert!(s.execute_as(agent(), None, "edit.delete", &json!({"ids": [r]})).is_err());
    assert!(s.execute_as(agent(), None, "object.toggleLock", &json!({"ids": [r]})).is_err());
    // The person's own edits are as before.
    s.execute("fill.set", &json!({"ids": [r], "color": "#00FF00"})).unwrap();
}

#[test]
fn a_locked_frame_holds_what_is_inside_it() {
    let mut s = Session::new();
    let (f, r) = scene(&mut s);
    s.execute("object.toggleLock", &json!({"ids": [f]})).unwrap();
    let e = s.execute_as(agent(), None, "fill.set", &json!({"ids": [r], "color": "#FF0000"})).unwrap_err().to_string();
    assert!(e.contains("\"Swatch\" is inside \"Card\", which is locked"), "{e}");
    // Adding a layer to it changes it.
    assert!(s.execute_as(agent(), None, "shape.rectangle", &json!({"x": 0, "y": 0, "width": 5, "height": 5, "parent": f})).is_err());
    // Outside it, agents work as usual.
    s.execute_as(agent(), None, "shape.rectangle", &json!({"x": 300, "y": 0, "width": 5, "height": 5})).unwrap();
}

#[test]
fn every_edit_is_logged_with_its_author() {
    let mut s = Session::new();
    let start = s.changes_since(0).unwrap()["revision"].as_u64().unwrap();
    let (_, r) = scene(&mut s);
    s.execute_as(agent(), None, "fill.set", &json!({"ids": [r], "color": "#FF0000"})).unwrap();
    let all = s.changes_since(start).unwrap();
    let changes = all["changes"].as_array().unwrap();
    let last = changes.last().unwrap();
    assert_eq!((last["author"].as_str(), last["agent"].as_bool(), last["command"].as_str()), (Some("agent (mcp)"), Some(true), Some("fill.set")));
    assert_eq!(last["layers"][0]["name"], "Swatch");
    assert!(changes.iter().any(|c| c["author"] == "person" && c["command"] == "frame.create"));
    // Only what came after a revision.
    let now = all["revision"].as_u64().unwrap();
    assert!(s.changes_since(now).unwrap()["changes"].as_array().unwrap().is_empty());
    // Through the command, as MCP's get_changes calls it.
    assert_eq!(s.execute("history.changes", &json!({"since": start})).unwrap()["changes"], all["changes"]);
}

#[test]
fn an_agents_undo_step_says_so() {
    let mut s = Session::new();
    let (_, r) = scene(&mut s);
    s.execute_as(agent(), None, "fill.set", &json!({"ids": [r], "color": "#FF0000"})).unwrap();
    assert!(s.undo_labels(1)[0].starts_with("Agent: "), "{:?}", s.undo_labels(1));
    s.execute("fill.set", &json!({"ids": [r], "color": "#00FF00"})).unwrap();
    assert!(!s.undo_labels(1)[0].starts_with("Agent: "), "{:?}", s.undo_labels(1));
}

#[test]
fn a_stale_write_is_refused_and_says_what_changed() {
    let mut s = Session::new();
    let (f, r) = scene(&mut s);
    let read = s.changes_since(0).unwrap()["revision"].as_u64().unwrap();
    // The person resizes the swatch after the agent read the document.
    s.execute("node.set", &json!({"id": r, "props": {"width": 40}})).unwrap();
    let e = s.execute_as(agent(), Some(read), "fill.set", &json!({"ids": [r], "color": "#FF0000"})).unwrap_err().to_string();
    assert!(e.contains(&format!("changed since revision {read}: \"Swatch\" by the person")), "{e}");
    assert!(e.contains("Read it again"), "{e}");
    assert_eq!(s.doc().unwrap().node(r).unwrap().width, 40.0, "the person's edit stays");
    // A layer nobody else touched is fine, and so is a write after reading again.
    s.execute_as(agent(), Some(read), "fill.set", &json!({"ids": [f], "color": "#FAFAFA"})).unwrap();
    let again = s.changes_since(0).unwrap()["revision"].as_u64().unwrap();
    s.execute_as(agent(), Some(again), "fill.set", &json!({"ids": [r], "color": "#FF0000"})).unwrap();
    // Its own earlier edits don't count against it.
    s.execute_as(agent(), Some(again), "fill.set", &json!({"ids": [r], "color": "#0000FF"})).unwrap();
}
