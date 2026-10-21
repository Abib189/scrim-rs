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

/// An agent locking a layer is refused for what it is, not as if the layer were already locked.
#[test]
fn an_agent_locking_a_layer_is_told_locking_is_the_persons() {
    let mut s = Session::new();
    let (_, r) = scene(&mut s);
    let before = s.doc().unwrap().clone();
    let e = s.execute_as(agent(), None, "object.toggleLock", &json!({"ids": [r]})).unwrap_err().to_string();
    assert!(e.contains("an agent can't lock \"Swatch\": locking layers is the person's"), "{e}");
    assert!(!e.contains("is locked"), "it wasn't locked: {e}");
    assert_eq!(*s.doc().unwrap(), before, "rolled back");
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

/// The person's canvas gestures are in the feed too (found by Stage C's C4: gestures commit
/// without a command, so the feed showed only commands). A drawn shape and a drag each log one
/// edit by the person; an agent's command logs one by the agent.
#[test]
fn the_persons_canvas_gestures_are_in_the_feed() {
    use crate::tools::{PointerEvent, PointerKind};
    let drag = |s: &mut Session, from: (f64, f64), to: (f64, f64)| {
        s.pointer(&PointerEvent::at(PointerKind::Down, from.0, from.1, 1.0)).unwrap();
        for k in 1..=4 {
            let t = f64::from(k) / 4.0;
            s.pointer(&PointerEvent::at(PointerKind::Move, from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, 1.0)).unwrap();
        }
        s.pointer(&PointerEvent::at(PointerKind::Up, to.0, to.1, 1.0)).unwrap();
    };
    let mut s = Session::new();
    s.execute("tool.rectangle", &json!({})).unwrap();
    drag(&mut s, (100.0, 100.0), (200.0, 180.0));
    s.execute("tool.move", &json!({})).unwrap();
    drag(&mut s, (150.0, 140.0), (250.0, 140.0));
    s.set_author(agent());
    s.execute("shape.ellipse", &json!({"x": 400, "y": 100, "width": 50, "height": 50})).unwrap();
    s.set_author(Author::Person);
    let feed = s.execute("history.changes", &json!({})).unwrap();
    let rows: Vec<(String, String, bool)> = feed["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["command"].as_str().unwrap().to_string(), c["author"].as_str().unwrap().to_string(), c["agent"].as_bool().unwrap()))
        .collect();
    let gestures: Vec<_> = rows.iter().filter(|r| r.0 == "gesture").collect();
    assert_eq!(gestures.len(), 2, "the drawn rectangle and the drag: {rows:?}");
    assert!(gestures.iter().all(|g| g.1 == "person" && !g.2));
    assert_eq!(rows.last().map(|r| (r.0.as_str(), r.2)), Some(("shape.ellipse", true)), "the agent's edit, its own entry");
}

/// `history.changes {last}`: the newest edits however far the revision has moved since (it also
/// moves for selection and view changes). The AI tab's feed asked for a revision window instead
/// and lost the edits after a few hundred selection changes.
#[test]
fn the_newest_edits_are_listed_however_far_the_revision_moved() {
    let mut s = Session::new();
    for i in 0..5 {
        s.execute("shape.rectangle", &json!({"x": i * 30, "y": 0, "width": 20, "height": 20})).unwrap();
    }
    for _ in 0..300 {
        s.execute("select.none", &json!({})).unwrap();
        s.execute("select.all", &json!({})).unwrap();
    }
    let feed = s.execute("history.changes", &json!({"last": 3})).unwrap();
    let labels: Vec<&str> = feed["changes"].as_array().unwrap().iter().map(|c| c["label"].as_str().unwrap()).collect();
    assert_eq!(labels.len(), 3, "{feed}");
    assert!(feed["revision"].as_u64().unwrap() > 300, "the revision moved on");
    assert_eq!(s.execute("history.changes", &json!({})).unwrap()["changes"].as_array().unwrap().len(), 5);
}

/// Stage J: the name a person sees for an MCP client: friendly when known, as announced otherwise
/// (cleaned, cut with an ellipsis), and nothing (the caller says "an agent") when none was given.
#[test]
fn client_names_are_friendly_when_known_verbatim_otherwise_and_never_guessed() {
    use super::{NAME_MAX, client_display_name as name};
    assert_eq!(name("claude-code").as_deref(), Some("Claude Code"));
    assert_eq!(name("codex-mcp-client").as_deref(), Some("Codex"));
    assert_eq!(name("gemini-cli-mcp-client").as_deref(), Some("Gemini CLI"));
    assert_eq!(name("grok-shell-scrim").as_deref(), Some("Grok"), "Grok Build names itself after the server");
    assert_eq!(name("Visual Studio Code").as_deref(), Some("VS Code"));
    // Unknown: as announced, never "Claude".
    assert_eq!(name("Cursor").as_deref(), Some("Cursor"));
    assert_eq!(name("my-agent 0.1").as_deref(), Some("my-agent 0.1"));
    assert_eq!(name("claude-something-else").as_deref(), Some("claude-something-else"), "no guessing from a substring");
    // Nothing announced.
    assert_eq!(name(""), None);
    assert_eq!(name("   "), None);
    assert_eq!(name("\u{7}\u{1b}"), None, "only control characters");
    // Very long: cut, with an ellipsis.
    let long = "x".repeat(300);
    let shown = name(&long).unwrap();
    assert_eq!(shown.chars().count(), NAME_MAX + 1);
    assert!(shown.ends_with('…'));
    assert_eq!(name("bad\u{0}name").as_deref(), Some("badname"), "control characters dropped");
}

/// Stage J: an agent's edit records the MCP client it came from; the person's never does, and
/// whether it's an agent's edit is still the author alone.
#[test]
fn the_change_log_names_the_client_of_an_agents_edit() {
    let mut s = Session::new();
    s.agents.client = Some("codex-mcp-client".into());
    s.execute_as(Author::agent("mcp (http)"), None, "frame.create", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    // The person's edit while a client is remembered: no client.
    s.execute("frame.create", &json!({"x": 20, "y": 0, "width": 10, "height": 10})).unwrap();
    let feed = s.execute("history.changes", &json!({})).unwrap();
    let changes = feed["changes"].as_array().unwrap();
    let (a, p) = (&changes[changes.len() - 2], &changes[changes.len() - 1]);
    assert_eq!((a["agent"].clone(), a["client"].clone()), (json!(true), json!("codex-mcp-client")));
    assert_eq!((p["agent"].clone(), p["client"].clone()), (json!(false), Value::Null));
}
