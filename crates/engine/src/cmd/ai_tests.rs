//! The AI tab's commands: the connection, the change feed's select and undo, and the lint findings.

use serde_json::{Value, json};

use super::NOT_LATEST;
use crate::Session;
use crate::agents::Author;

fn agent() -> Author {
    Author::agent("mcp")
}

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

/// The newest logged change's revision.
fn latest(s: &mut Session) -> u64 {
    s.execute("history.changes", &json!({})).unwrap()["changes"].as_array().unwrap().last().unwrap()["revision"].as_u64().unwrap()
}

#[test]
fn the_status_gives_the_connect_command_for_how_agents_can_reach_the_app() {
    let mut s = Session::new();
    let off = s.execute("ai.status", &json!({})).unwrap();
    assert_eq!(off, json!({"channel": "off", "port": null, "connectCommand": "claude mcp add scrim -- scrim mcp"}));
    s.agents.control_port = Some(7979);
    let on = s.execute("ai.status", &json!({})).unwrap();
    assert_eq!(on, json!({"channel": "on", "port": 7979, "connectCommand": "claude mcp add scrim-app -- scrim mcp --attach 7979"}));
}

#[test]
fn an_agents_change_is_selected_and_undone_from_the_feed_while_it_is_the_latest() {
    let mut s = Session::new();
    let mine = id(&s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap());
    let theirs = id(&s.execute_as(agent(), None, "shape.rectangle", &json!({"x": 40, "y": 0, "width": 10, "height": 10})).unwrap());
    let rev = latest(&mut s);
    // Selecting it selects what it touched.
    assert_eq!(s.execute("ai.selectChange", &json!({"revision": rev})).unwrap(), json!({"selected": [theirs], "removed": []}));
    assert_eq!(s.selection().iter().map(|i| i.0).collect::<Vec<_>>(), [theirs]);
    assert!(super::can_undo(&s, rev));
    let r = s.execute("ai.undoChange", &json!({"revision": rev})).unwrap();
    assert_eq!((&r["undone"], &r["revision"]), (&json!(true), &json!(rev)));
    let doc = s.doc().unwrap();
    assert!(doc.contains(figcraft_doc::NodeId(mine)) && !doc.contains(figcraft_doc::NodeId(theirs)), "only the agent's change is undone");
    // Done once: it isn't the latest step any more, and its layer is reported removed.
    assert!(!super::can_undo(&s, rev));
    assert_eq!(s.execute("ai.selectChange", &json!({"revision": rev})).unwrap(), json!({"selected": [], "removed": [theirs]}));
}

#[test]
fn undo_from_the_feed_is_refused_under_later_edits_and_for_the_persons_changes() {
    let mut s = Session::new();
    s.execute_as(agent(), None, "shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    let rev = latest(&mut s);
    s.execute("shape.rectangle", &json!({"x": 40, "y": 0, "width": 10, "height": 10})).unwrap();
    let persons = latest(&mut s);
    let e = s.execute("ai.undoChange", &json!({"revision": rev})).unwrap_err().to_string();
    assert!(e.contains(NOT_LATEST), "{e}");
    let e = s.execute("ai.undoChange", &json!({"revision": persons})).unwrap_err().to_string();
    assert!(e.contains("the person's"), "{e}");
    assert!(s.execute("ai.undoChange", &json!({})).unwrap_err().to_string().contains("`revision` is required"));
    assert!(s.execute("ai.selectChange", &json!({"revision": 999})).unwrap_err().to_string().contains("no change at that revision"));
}

#[test]
fn findings_are_the_lints_grouped_by_rule_and_select_their_layers() {
    let mut s = Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 300, "height": 200})).unwrap());
    let a = id(&s.execute("text.create", &json!({"x": 10, "y": 10, "text": "One", "parent": f, "fontFamily": "Inter"})).unwrap());
    let b = id(&s.execute("text.create", &json!({"x": 10, "y": 60, "text": "Two", "parent": f, "fontFamily": "Inter"})).unwrap());
    let r = s.execute("ai.findings", &json!({})).unwrap();
    let lint = s.execute("lint.run", &json!({})).unwrap();
    assert_eq!(r["count"], lint["violationCount"]);
    let groups = r["groups"].as_array().unwrap();
    let total: u64 = groups.iter().map(|g| g["count"].as_u64().unwrap()).sum();
    assert_eq!(json!(total), lint["violationCount"], "every finding is in one group");
    assert!(groups.windows(2).all(|w| w[0]["count"].as_u64() >= w[1]["count"].as_u64()), "largest group first");
    let Some(font) = groups.iter().find(|g| g["rule"] == "overused-font") else { panic!("{r}") };
    s.execute("ai.selectFinding", &json!({"rule": "overused-font"})).unwrap();
    let mut sel: Vec<u64> = s.selection().iter().map(|i| i.0).collect();
    sel.sort_unstable();
    assert!(sel.contains(&a) && sel.contains(&b), "{sel:?} {font}");
    s.execute("ai.selectFinding", &json!({"rule": "overused-font", "index": 0})).unwrap();
    assert_eq!(s.selection().len(), 1);
    assert!(s.execute("ai.selectFinding", &json!({"rule": "no-such-rule"})).is_err());
}
