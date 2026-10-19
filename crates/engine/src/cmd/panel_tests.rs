//! The panel's mode and tab commands: what they return, what they ask the UI, what they keep.

use serde_json::json;

use super::{Mode, PanelView, REQUEST, Tab};
use crate::Session;

fn run(s: &mut Session, cmd: &str) -> serde_json::Value {
    s.ui_requests.clear();
    let r = s.execute(cmd, &json!({})).unwrap();
    let [req] = s.ui_requests.as_slice() else { panic!("{cmd}: one request, got {:?}", s.ui_requests) };
    assert_eq!((req.kind.as_str(), &req.data), (REQUEST, &r), "{cmd}: the UI is asked for what the command returns");
    r
}

#[test]
fn tabs_switch_the_tab_and_the_mode_back_to_design() {
    let mut s = Session::new();
    assert_eq!(s.prefs.panel, PanelView { mode: Mode::Design, tab: Tab::Design });
    for tab in Tab::ALL {
        run(&mut s, "panel.mode.animate");
        assert_eq!(run(&mut s, &format!("panel.tab.{}", tab.id())), json!({"mode": "design", "tab": tab.id()}));
    }
}

#[test]
fn the_mode_keeps_the_tab_to_come_back_to() {
    let mut s = Session::new();
    run(&mut s, "panel.tab.code");
    assert_eq!(run(&mut s, "panel.mode.animate"), json!({"mode": "animate", "tab": "code"}));
    assert_eq!(run(&mut s, "panel.mode.design"), json!({"mode": "design", "tab": "code"}));
}

#[test]
fn the_choice_is_a_preference_and_nothing_touches_the_document() {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    let (before, steps) = (s.doc().unwrap().clone(), s.undo_depth());
    run(&mut s, "panel.tab.ai");
    run(&mut s, "panel.mode.animate");
    assert!(s.doc().unwrap() == &before, "a panel switch is not an edit");
    assert_eq!(s.undo_depth(), steps, "and leaves no undo step");
    let saved = serde_json::to_value(&s.prefs).unwrap();
    assert_eq!(saved["panel"], json!({"mode": "animate", "tab": "ai"}));
    // Preferences saved before the panel existed open on Design.
    let old: crate::Prefs = serde_json::from_value(json!({"theme": "dark"})).unwrap();
    assert_eq!(old.panel, PanelView::default());
    let partial: crate::Prefs = serde_json::from_value(json!({"panel": {"tab": "code"}})).unwrap();
    assert_eq!(partial.panel, PanelView { mode: Mode::Design, tab: Tab::Code });
}
