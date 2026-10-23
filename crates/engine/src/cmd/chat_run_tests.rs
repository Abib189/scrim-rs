//! Fix X4: the run card's rows come from the change log; `chat.select-run-layers` and
//! `chat.undo-run` are the person's alone; Undo run waits for the run to end, restores the
//! document exactly as one undo step, and is noted in Diagnostics without the prompt.

use serde_json::{Value, json};

use super::{RunRecord, rows};
use crate::Session;
use crate::agents::Author;

fn id(v: &Value) -> figcraft_doc::NodeId {
    crate::params::id_of(&v["id"]).unwrap()
}

/// A document, a run begun on it, and the agent's edits: a frame, three rectangles in a batch, a
/// text, then two property changes. Returns the session, the frame and the document at the start.
fn ran() -> (Session, figcraft_doc::NodeId, std::sync::Arc<figcraft_doc::Document>) {
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    let (uid, start_doc) = (s.ds().unwrap().uid, s.ds().unwrap().doc.clone());
    let start_rev = s.agents.last_edit(uid).unwrap_or(0);
    s.agents.chat_run = Some(RunRecord { uid, start_doc: start_doc.clone(), start_rev, end_rev: None, agent: "Claude Code".into() });
    s.agents.chat_running = true;
    s.set_author(Author::agent("mcp (http)"));
    let f = id(&s.execute("frame.create", &json!({"x": 100, "y": 0, "width": 300, "height": 200, "name": "Poster"})).unwrap());
    let rects: Vec<Value> = (0..3)
        .map(|k| json!({"command": "shape.rectangle", "params": {"x": 10 + k * 40, "y": 10, "width": 30, "height": 30, "parent": f.0}}))
        .collect();
    s.execute("command.batch", &json!({"commands": rects})).unwrap();
    s.execute("text.create", &json!({"text": "Welle", "x": 10, "y": 120, "parent": f.0})).unwrap();
    for name in ["Poster — Welle", "Poster — Welle 2"] {
        s.execute("node.set", &json!({"id": f.0, "props": {"name": name}})).unwrap();
    }
    s.set_author(Author::Person);
    (s, f, start_doc)
}

fn end(s: &mut Session) {
    s.agents.chat_running = false;
    let uid = s.ds().unwrap().uid;
    let last = s.agents.last_edit(uid);
    if let Some(r) = s.agents.chat_run.as_mut() {
        r.end_rev = last;
    }
}

#[test]
fn the_rows_are_the_runs_edits_by_label_with_neighbours_merged() {
    let (s, ..) = ran();
    let rows = rows(&s);
    let labels: Vec<String> = rows.iter().map(|(l, n)| if *n > 1 { format!("{l} ×{n}") } else { l.clone() }).collect();
    assert_eq!(labels, ["Create frame", "Rectangle ×3", "Text", "Set properties ×2"], "the batch's wrapper left out: {rows:?}");
}

#[test]
fn both_commands_are_the_persons_alone() {
    let (mut s, ..) = ran();
    end(&mut s);
    for route in ["mcp (http)", "cli", "control channel", "plugin X"] {
        s.set_author(Author::agent(route));
        for cmd in ["chat.select-run-layers", "chat.undo-run"] {
            let e = s.execute(cmd, &json!({})).unwrap_err().to_string();
            assert!(e.contains("Describe a change is the person's"), "{route} {cmd}: {e}");
        }
    }
}

#[test]
fn undo_run_waits_for_the_run_then_restores_the_document_exactly_as_one_step() {
    let (mut s, f, start) = ran();
    // While it runs: refused, the U3 way.
    let e = s.execute("chat.undo-run", &json!({})).unwrap_err().to_string();
    assert!(e.contains("Stop the run first"), "{e}");
    end(&mut s);
    let after_run = s.ds().unwrap().doc.clone();
    let depth = s.undo_depth().0;
    let r = s.execute("chat.undo-run", &json!({})).unwrap();
    assert_eq!(r, json!({"undone": true, "edits": 7}), "a frame, three rectangles, a text, two renames");
    assert_eq!(*s.ds().unwrap().doc, *start, "exactly the document before the run");
    assert!(!s.doc().unwrap().contains(f));
    assert_eq!(s.undo_depth().0, depth + 1, "one undo step");
    assert_eq!(s.undo_labels(1), ["Undo run"]);
    // Undoing it brings the run back, whole.
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(*s.ds().unwrap().doc, *after_run);
    // Noted without the prompt or any layer name.
    let last = s.prefs.settings.diagnostics.last().unwrap();
    assert_eq!((last.kind.as_str(), last.detail.as_str()), ("chat", "Claude Code run undone: 7 edits"));
    assert!(s.prefs.settings.diagnostics.iter().all(|e| !e.detail.contains("Welle") && !e.detail.contains("Poster")));
}

#[test]
fn undo_run_is_refused_after_the_person_edited_since() {
    let (mut s, ..) = ran();
    end(&mut s);
    s.execute("shape.ellipse", &json!({"x": 500, "y": 0, "width": 10, "height": 10})).unwrap();
    let e = s.execute("chat.undo-run", &json!({})).unwrap_err().to_string();
    assert!(e.contains("the document changed after the run began"), "{e}");
    // Without a run: says so.
    let mut fresh = Session::new();
    assert!(fresh.execute("chat.undo-run", &json!({})).unwrap_err().to_string().contains("no run"));
}

#[test]
fn select_run_layers_selects_what_the_run_touched_that_is_left() {
    let (mut s, f, _) = ran();
    end(&mut s);
    let r = s.execute("chat.select-run-layers", &json!({})).unwrap();
    assert_eq!(r["selected"], 5, "the frame, three rectangles and the text: {r}");
    assert!(s.selection().contains(&f));
    // Some gone: only what is left; all gone: says so.
    let kids: Vec<_> = s.doc().unwrap().node(f).unwrap().children.clone();
    s.execute("edit.delete", &json!({"ids": kids.iter().take(2).map(|k| k.0).collect::<Vec<_>>()})).unwrap();
    assert_eq!(s.execute("chat.select-run-layers", &json!({})).unwrap()["selected"], 3);
}
