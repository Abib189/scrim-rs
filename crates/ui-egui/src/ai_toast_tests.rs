//! Fix AB3: the Undone toast.

use serde_json::{Value, json};

use crate::follow_agent::chat_ui::tests::{feed, send_allowed, window};
use crate::follow_agent::tests::agent;
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::{click_named, tree};
use crate::services::{AgentEnd, AgentRunEvent};

fn toast(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)["aiPanel"]["composer"]["toast"].clone()
}

/// A run of three edits, ended, then Undo run pressed on its card.
fn undone(dark: bool) -> (Ui, figcraft_doc::Document) {
    let (mut u, script) = window();
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    send_allowed(&mut u, "Build a poster");
    u.frame(vec![]);
    let f = agent(&mut u, "frame.create", json!({"x": 400, "y": 0, "width": 300, "height": 200, "name": "Poster"}));
    agent(&mut u, "text.create", json!({"text": "Welle", "parent": f["id"]}));
    agent(&mut u, "node.set", json!({"id": f["id"], "props": {"name": "Poster — Welle"}}));
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 20.0 }]);
    u.frame(vec![]);
    u.frame(vec![]);
    let after = u.app.session.doc().unwrap().clone();
    click_named(&mut u, "Undo run");
    (u, after)
}

/// After Undo run: "Undone: 3 edits" and "Redo", named "Undone 3 edits. Redo." for screen readers,
/// both themes; "Redo" puts the run back, and the toast goes.
#[test]
fn undo_run_shows_the_toast_and_its_redo_puts_the_run_back() {
    for dark in [false, true] {
        let (mut u, after) = undone(dark);
        let t = toast(&u);
        assert_eq!((t["text"].as_str(), t["action"].as_str()), (Some("Undone: 3 edits"), Some("Redo")), "{dark}: {t}");
        assert_eq!(t["name"], "Undone 3 edits. Redo.");
        let nodes = tree(&mut u);
        assert!(nodes.iter().any(|n| n.label() == Some("Undone 3 edits. Redo.")), "{dark}: the screen reader's name");
        click_named(&mut u, "Redo");
        assert_eq!(*u.app.session.doc().unwrap(), after, "{dark}: Redo put the run back");
        assert!(toast(&u).is_null(), "{dark}: and the toast went");
    }
}

/// It goes after 6 s, with one frame asked for at its end and none between; and at once on a new
/// message.
#[test]
fn the_toast_goes_after_six_seconds_or_on_a_new_message() {
    let (mut u, _) = undone(false);
    assert!(toast(&u).is_object());
    u.frame(vec![egui::Event::PointerGone]);
    // The fake clock (the next frame when the delay asked for has passed): what the undo set moving
    // (the camera, the indicator's fade) settles, then 2 s with the toast showing draw nothing.
    let idle = crate::panel_view::ai_tab::panel::idle_tests::idle;
    let start = u.time;
    idle(&mut u, 3.0);
    u.time = u.time.max(start + 3.0);
    let (frames, causes) = idle(&mut u, 2.0);
    assert!(toast(&u).is_object(), "still there at 5 s");
    assert_eq!(frames, 0, "no frames while it waits: {causes:?}");
    u.time = start + 6.5;
    u.frame(vec![]);
    assert!(toast(&u).is_null(), "gone after 6 s");
    let (mut u, _) = undone(false);
    assert!(toast(&u).is_object());
    send_allowed(&mut u, "Another try");
    u.frame(vec![]);
    assert!(toast(&u).is_null(), "a new message clears it");
}
