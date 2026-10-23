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
    s.agents.chat_run = Some(RunRecord { uid, start_doc: start_doc.clone(), start_rev, end_rev: None, agent: "Claude Code".into(), undone: None });
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
fn the_rows_say_what_each_command_did_by_name() {
    let (s, ..) = ran();
    let labels: Vec<String> = rows(&s).iter().map(|(l, n)| if *n > 1 { format!("{l} ×{n}") } else { l.clone() }).collect();
    // A frame (by itself), a batch of three rectangles in it, a text in it, two renames of the frame.
    assert_eq!(labels, ["Created Poster", "Created 3 layers", "Created Welle", "Edited Poster — Welle", "Edited Poster — Welle 2"]);
}

/// A run, its record begun now, of `steps` (command, params) as the agent sends them.
fn run_of(steps: &[(&str, Value)]) -> Session {
    let mut s = Session::new();
    let (uid, start_doc) = (s.ds().unwrap().uid, s.ds().unwrap().doc.clone());
    s.agents.chat_run = Some(RunRecord { uid, start_doc, start_rev: 0, end_rev: None, agent: "Claude Code".into(), undone: None });
    s.set_author(Author::agent("mcp (http)"));
    for (cmd, p) in steps {
        s.execute(cmd, p).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    }
    s.set_author(Author::Person);
    s
}

fn texts(s: &Session) -> Vec<String> {
    rows(s).into_iter().map(|(l, n)| if n > 1 { format!("{l} ×{n}") } else { l }).collect()
}

/// Fix Z1: a frame with 30 layers made in it, four layers edited, one deleted: three rows that say
/// so by name, the deleted one named as it was; they stay after the layers are gone, and say nothing
/// of the prompt (which the run never had).
#[test]
fn a_frame_with_30_layers_four_edits_and_a_deletion_are_three_rows() {
    let mut steps = vec![json!({"command": "frame.create", "params": {"x": 0, "y": 0, "width": 640, "height": 480, "name": "Poster"}})];
    steps.extend((0..30).map(|k| json!({"command": "shape.rectangle", "params": {"x": k * 20, "y": 0, "width": 10, "height": 10, "parent": {"$step": 0}, "name": format!("Bar {k}")}})));
    let mut s = run_of(&[("command.batch", json!({"commands": steps}))]);
    let doc = s.doc().unwrap().clone();
    let poster = doc.nodes().find(|n| n.name == "Poster").unwrap().id;
    let bars: Vec<u64> = doc.node(poster).unwrap().children.iter().map(|c| c.0).collect();
    s.set_author(Author::agent("mcp (http)"));
    let edits: Vec<Value> = bars[..4].iter().map(|b| json!({"command": "node.set", "params": {"id": b, "props": {"opacity": 0.5}}})).collect();
    s.execute("command.batch", &json!({"commands": edits})).unwrap();
    s.execute("edit.delete", &json!({"ids": [bars[7]]})).unwrap();
    s.set_author(Author::Person);
    let want = ["Created Poster (30 layers)", "Edited 4 layers", "Deleted Bar 7"];
    assert_eq!(texts(&s), want);
    // The person deletes the poster: the run's rows are as they were.
    s.execute("edit.delete", &json!({"ids": [poster.0]})).unwrap();
    assert_eq!(texts(&s), want, "rows don't change once the layers are gone");
    for word in ["swiss", "minimalist", "create"] {
        assert!(texts(&s).iter().all(|r| !r.to_lowercase().contains(word)) || word == "create", "{word}");
    }
}

/// Fix Z1: a batch the agent labelled with a sentence says that; the engine's own labels and a
/// one-word label don't count as one.
#[test]
fn a_batch_the_agent_labelled_says_its_label() {
    let rect = |x: i32| json!({"command": "shape.rectangle", "params": {"x": x, "y": 0, "width": 10, "height": 10}});
    let s = run_of(&[
        ("command.batch", json!({"label": "  Lay out the title and footer  ", "commands": [rect(0), rect(20)]})),
        ("command.batch", json!({"label": "Rectangle", "commands": [rect(40)]})),
        ("command.batch", json!({"label": "Bars", "commands": [rect(60), rect(80)]})),
    ]);
    // Layers right on the page are each named ("Created {name}"), so the two after it merge with it.
    assert_eq!(texts(&s), ["Lay out the title and footer", "Created Rectangle ×3"]);
}

/// Fix Z1: moves, two deletions by name, and the same row twice merged.
#[test]
fn moves_and_deletions_say_where_and_what() {
    let mut s = run_of(&[
        ("frame.create", json!({"x": 0, "y": 0, "width": 300, "height": 200, "name": "Card"})),
        ("shape.rectangle", json!({"x": 400, "y": 0, "width": 10, "height": 10, "name": "Dot"})),
        ("shape.rectangle", json!({"x": 420, "y": 0, "width": 10, "height": 10, "name": "Line"})),
    ]);
    let doc = s.doc().unwrap().clone();
    let by = |n: &str| doc.nodes().find(|x| x.name == n).unwrap().id.0;
    s.set_author(Author::agent("mcp (http)"));
    s.execute("node.reparent", &json!({"ids": [by("Dot")], "parent": by("Card")})).unwrap();
    s.execute("edit.delete", &json!({"ids": [by("Line")]})).unwrap();
    s.set_author(Author::Person);
    let t = texts(&s);
    assert_eq!(t, ["Created Card", "Created Dot", "Created Line", "Moved Dot into Card", "Deleted Line"]);
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
    assert!(e.contains("The document changed after the run began"), "{e}");
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

/// A run card state: (undone, Undo run one step, Redo run one step).
fn st(undone: bool, undo_ok: bool, redo_ok: bool) -> super::RunState {
    super::RunState { undone, undo_ok, redo_ok }
}

/// Diagnostics' line for a refused run command, as the window notes it.
fn noted(s: &mut Session, cmd: &str) -> String {
    let e = s.execute(cmd, &json!({})).unwrap_err();
    crate::settings::diagnostics::record(&mut s.prefs.settings, 1, crate::settings::diagnostics::Entry::Error { command: cmd, error: &e });
    s.prefs.settings.diagnostics.last().map(|d| d.detail.clone()).unwrap_or_default()
}

/// Fix Z7, the real run's "invalid parameters": after Undo run, pressing the card's buttons again
/// was refused as malformed calls. The undo is itself an edit since the run began, so a second
/// Undo run hit "the document changed" (a BadParams), and Select layers found none of the layers.
/// Now each refusal is "not available now" in fixed words, and Diagnostics says which.
#[test]
fn after_undo_run_the_buttons_are_refused_in_words_never_as_invalid_parameters() {
    let (mut s, ..) = ran();
    end(&mut s);
    s.execute("chat.undo-run", &json!({})).unwrap();
    assert_eq!(super::state(&s), st(true, false, true), "undone, and Redo run is one step");
    // Selecting isn't a change to the document.
    s.execute("select.all", &json!({})).unwrap();
    assert_eq!(super::state(&s), st(true, false, true), "still one step after a selection");
    let e = s.execute("chat.undo-run", &json!({})).unwrap_err();
    assert!(matches!(e, crate::EngineError::Disabled(..)), "{e:?}");
    assert!(e.to_string().contains("The run is already undone"), "{e}");
    assert_eq!(noted(&mut s, "chat.undo-run"), "chat.undo-run: refused (already undone)");
    assert_eq!(noted(&mut s, "chat.select-run-layers"), "chat.select-run-layers: refused (removed by Undo)");
    let all: Vec<String> = s.prefs.settings.diagnostics.iter().map(|d| d.detail.clone()).collect();
    assert!(all.iter().all(|d| !d.contains("invalid parameters")), "{all:?}");
    // A malformed call is still "invalid parameters".
    let bad = s.execute("command.batch", &json!({"commands": 3})).unwrap_err();
    assert_eq!(crate::settings::diagnostics::category(&bad), "invalid parameters");
}

/// Fix Z7: Redo run puts the run back as one step when nothing changed since Undo run; Undo run
/// works again after it (the run's own undo and redo don't count as changes since).
#[test]
fn redo_run_restores_the_run_as_one_step_and_undo_run_works_again() {
    let (mut s, f, start) = ran();
    end(&mut s);
    let after_run = s.ds().unwrap().doc.clone();
    s.execute("chat.undo-run", &json!({})).unwrap();
    let depth = s.undo_depth().0;
    assert_eq!(s.execute("chat.redo-run", &json!({})).unwrap(), json!({"redone": true}));
    assert_eq!(*s.ds().unwrap().doc, *after_run, "the run's document, exactly");
    assert!(s.doc().unwrap().contains(f));
    assert_eq!(s.undo_depth().0, depth + 1, "one undo step");
    assert_eq!(s.undo_labels(1), ["Redo run"]);
    assert_eq!(super::state(&s), st(false, true, false), "done, Undo run one step again");
    assert_eq!(s.execute("chat.select-run-layers", &json!({})).unwrap()["selected"], 5);
    s.execute("chat.undo-run", &json!({})).unwrap();
    assert_eq!(*s.ds().unwrap().doc, *start);
    // Redo while not undone: refused in words.
    s.execute("chat.redo-run", &json!({})).unwrap();
    assert_eq!(noted(&mut s, "chat.redo-run"), "chat.redo-run: refused (not undone)");
}

/// Fix Z7: after Undo run, any other edit makes Redo run no longer one step: refused, and the card
/// is told (`undone` says it can't).
#[test]
fn redo_run_is_refused_once_something_changed_since_the_undo() {
    let (mut s, ..) = ran();
    end(&mut s);
    s.execute("chat.undo-run", &json!({})).unwrap();
    s.execute("shape.ellipse", &json!({"x": 500, "y": 0, "width": 10, "height": 10})).unwrap();
    assert_eq!(super::state(&s), st(true, false, false), "undone, neither button");
    let e = s.execute("chat.redo-run", &json!({})).unwrap_err().to_string();
    assert!(e.contains("Something changed since: Redo is no longer one step"), "{e}");
    assert_eq!(noted(&mut s, "chat.redo-run"), "chat.redo-run: refused (changed since the undo)");
}

/// Fix Z7: Redo run is the person's alone, like the others, and refused while a run goes.
#[test]
fn redo_run_is_the_persons_alone() {
    let (mut s, ..) = ran();
    end(&mut s);
    s.execute("chat.undo-run", &json!({})).unwrap();
    s.set_author(Author::agent("mcp (http)"));
    let e = s.execute("chat.redo-run", &json!({})).unwrap_err().to_string();
    assert!(e.contains("Describe a change is the person's"), "{e}");
    s.set_author(Author::Person);
    s.agents.chat_running = true;
    assert_eq!(noted(&mut s, "chat.redo-run"), "chat.redo-run: refused (the run is going)");
}

/// Fix AA2: the card's state is the history's position. ⌘Z of Undo run brings the run back (Done,
/// Undo run one step); ⌘⇧Z makes it Undone again (Redo run one step); an edit in between turns
/// both off. Fix Z7 kept "undone" after ⌘Z, with Redo run off.
#[test]
fn the_state_follows_undo_and_redo_through_the_history() {
    let (mut s, ..) = ran();
    end(&mut s);
    let after_run = s.ds().unwrap().doc.clone();
    assert_eq!(super::state(&s), st(false, true, false), "done");
    s.execute("chat.undo-run", &json!({})).unwrap();
    assert_eq!(super::state(&s), st(true, false, true), "undone");
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(*s.ds().unwrap().doc, *after_run, "⌘Z brings the run back");
    assert_eq!(super::state(&s), st(false, true, false), "done again, Undo run one step");
    s.execute("edit.redo", &json!({})).unwrap();
    assert_eq!(super::state(&s), st(true, false, true), "⌘⇧Z: undone again");
    // Undo run works from Done-after-⌘Z, and Redo run from Undone-after-⌘⇧Z.
    s.execute("chat.redo-run", &json!({})).unwrap();
    assert_eq!(*s.ds().unwrap().doc, *after_run);
    assert_eq!(super::state(&s), st(false, true, false), "redone: done");
    s.execute("chat.undo-run", &json!({})).unwrap();
    assert_eq!(super::state(&s), st(true, false, true));
    // ⌘Z, then an edit: both off, and the card says Done (the run is there).
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("shape.ellipse", &json!({"x": 500, "y": 0, "width": 10, "height": 10})).unwrap();
    assert_eq!(super::state(&s), st(false, false, false), "both off after an edit");
    let e = s.execute("chat.undo-run", &json!({})).unwrap_err().to_string();
    assert!(e.contains("The document changed after the run began"), "{e}");
    // A selection isn't a change: the state stays.
    let (mut s, ..) = ran();
    end(&mut s);
    s.execute("chat.undo-run", &json!({})).unwrap();
    s.execute("edit.undo", &json!({})).unwrap();
    s.execute("select.all", &json!({})).unwrap();
    assert_eq!(super::state(&s), st(false, true, false));
}
