//! The toolbar's configuration: hide and show, Move always shown, move, group by dropping onto
//! another, ungroup, reset, and a kept layout that comes back valid however it was edited.

use serde_json::{Value, json};

use super::{ToolEntry, default_layout, normalized};
use crate::Session;

fn groups(s: &mut Session) -> Vec<Vec<String>> {
    let st = s.execute("toolbar.state", &json!({})).unwrap();
    st["groups"].as_array().unwrap().iter().map(|g| g.as_array().unwrap().iter().map(|e| e["tool"].as_str().unwrap().to_string()).collect()).collect()
}

fn on(s: &mut Session, tool: &str) -> bool {
    let st: Value = s.execute("toolbar.state", &json!({})).unwrap();
    st["groups"].as_array().unwrap().iter().flat_map(|g| g.as_array().unwrap().clone()).find(|e| e["tool"] == tool).unwrap()["on"].as_bool().unwrap()
}

#[test]
fn the_default_is_the_toolbar_as_it_was() {
    let mut s = Session::new();
    assert_eq!(groups(&mut s)[0], ["move", "hand", "scale"]);
    assert_eq!(groups(&mut s).len(), 6);
    assert_eq!(s.execute("toolbar.state", &json!({})).unwrap()["isDefault"], true);
}

#[test]
fn tools_hide_and_show_but_move_stays() {
    let mut s = Session::new();
    s.execute("toolbar.toggle", &json!({"tool": "slice"})).unwrap();
    assert!(!on(&mut s, "slice"));
    let e = s.execute("toolbar.toggle", &json!({"tool": "move", "on": false})).unwrap_err().to_string();
    assert!(e.contains("Move can't be hidden"), "{e}");
    s.execute("toolbar.toggle", &json!({"tool": "slice", "on": true})).unwrap();
    assert!(on(&mut s, "slice"));
    assert_eq!(s.prefs.settings.toolbar, Vec::<Vec<ToolEntry>>::new(), "back to the default: nothing kept");
    // A hidden tool's shortcut still picks it.
    s.execute("toolbar.toggle", &json!({"tool": "star"})).unwrap();
    s.execute("tool.set", &json!({"tool": "star"})).unwrap();
    assert_eq!(s.tool.tool.id(), "star");
}

#[test]
fn tools_move_group_and_ungroup() {
    let mut s = Session::new();
    // Drop Text onto Comment: one group.
    s.execute("toolbar.group", &json!({"tool": "text", "with": "comment"})).unwrap();
    assert_eq!(groups(&mut s).last().unwrap(), &["comment", "text"]);
    assert_eq!(groups(&mut s).len(), 5, "Text's own group went");
    // Take it out again: a button of its own, after the group.
    s.execute("toolbar.ungroup", &json!({"tool": "text"})).unwrap();
    assert_eq!(groups(&mut s)[4..], [vec!["comment".to_string()], vec!["text".to_string()]]);
    // Move the Star to the front of the first group, then to a new group at the end.
    s.execute("toolbar.move", &json!({"tool": "star", "group": 0, "index": 0})).unwrap();
    assert_eq!(groups(&mut s)[0], ["star", "move", "hand", "scale"]);
    let n = groups(&mut s).len();
    s.execute("toolbar.move", &json!({"tool": "star", "group": n})).unwrap();
    assert_eq!(groups(&mut s).last().unwrap(), &["star"]);
    assert!(s.execute("toolbar.group", &json!({"tool": "text", "with": "text"})).is_err());
    assert!(s.execute("toolbar.move", &json!({"tool": "nope", "group": 0})).is_err());
    s.execute("toolbar.reset", &json!({})).unwrap();
    assert_eq!(s.execute("toolbar.state", &json!({})).unwrap()["isDefault"], true);
}

#[test]
fn a_kept_layout_comes_back_valid() {
    let e = |t: &str, on: bool| ToolEntry { tool: t.into(), on };
    // Unknown and repeated tools dropped, an empty group dropped, Move forced on, missing tools added.
    let odd = vec![vec![e("move", false), e("nope", true), e("frame", true)], vec![], vec![e("frame", true), e("eyedropper", true)]];
    let fixed = normalized(&odd);
    assert_eq!(fixed[0], vec![e("move", true), e("frame", true)]);
    let all: Vec<String> = fixed.iter().flatten().map(|e| e.tool.clone()).collect();
    let want: Vec<String> = default_layout().iter().flatten().map(|e| e.tool.clone()).collect();
    assert_eq!(all.len(), want.len(), "every toolbar tool once: {all:?}");
    assert!(want.iter().all(|t| all.contains(t)));
    let kept: crate::Prefs = serde_json::from_value(json!({"toolbar": [[{"tool": "text", "on": true}]]})).unwrap();
    assert_eq!(super::layout(&kept)[0], vec![e("text", true)], "kept order first, the rest added");
}
