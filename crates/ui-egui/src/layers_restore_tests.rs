//! H2: Collapse all as a toggle with restore.
//! - Collapse then restore gives back the identical set of open rows, the list's scroll, and the
//!   selection.
//! - A row opened (or opened then closed) by hand, another page, another file, a layer added,
//!   deleted or moved to another parent, and an undo or redo of one, each drop the snapshot: the
//!   next press collapses again and takes a new one. A rename keeps it.
//! - Two presses on a tree already fully collapsed restore it fully collapsed, without an error.
//! - The command (an agent's `layers.collapse-all`) never restores, however often it is called.
//! - The button's name, tooltip, key and pressed state match what its next press does.
//! - A filter: the button collapses as D1 does, and no snapshot is taken or applied.
//! - Across all of it: no document write, no new revision, no undo step, no change logged.

use std::collections::BTreeSet;
use std::sync::Arc;

use egui::accesskit::{Role, Toggled};
use egui::{Event, Pos2, pos2};
use figcraft_doc::{Document, NodeId};
use serde_json::{Value, json};

use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::{click_named, nested, rows, tree};

const COLLAPSE: &str = "Collapse all layers";
const RESTORE: &str = "Restore expanded layers";

/// The header button: its name, whether it shows pressed, and its key.
fn button(u: &mut Ui) -> (String, bool, Option<String>) {
    let nodes = tree(u);
    let b = nodes
        .iter()
        .find(|n| n.role() == Role::Button && matches!(n.label(), Some(l) if l == COLLAPSE || l == RESTORE))
        .expect("the Collapse all button");
    (b.label().unwrap_or_default().to_string(), b.toggled() == Some(Toggled::True), b.keyboard_shortcut().map(str::to_string))
}

/// The name of the button's next action.
fn next(u: &mut Ui) -> String {
    button(u).0
}

fn open(u: &Ui) -> BTreeSet<NodeId> {
    u.app.ui.expanded.clone()
}

/// What the document holds: itself, its revision, and its undo depth.
fn doc_state(u: &Ui) -> (Arc<Document>, u64, usize) {
    let ds = u.app.session.ds().unwrap();
    (ds.doc.clone(), ds.revision, ds.history.undo.len())
}

fn assert_doc_untouched(u: &Ui, before: &(Arc<Document>, u64, usize), what: &str) {
    let now = doc_state(u);
    assert!(Arc::ptr_eq(&now.0, &before.0), "{what}: no document write");
    assert_eq!(now.1, before.1, "{what}: no new revision");
    assert_eq!(now.2, before.2, "{what}: no undo step");
}

fn id(v: Option<Value>) -> NodeId {
    figcraft_engine::params::id_of(&v.expect("the command ran")["id"]).unwrap()
}

/// `nested()` with Kiln's body closed by hand and Poster's body selected: a set of open rows that
/// isn't "everything".
fn partly_open() -> (Ui, [NodeId; 4]) {
    let (mut u, ids @ [_, poster_body, _, kiln_body]) = nested();
    u.app.ui.expanded.remove(&kiln_body);
    u.app.run("select.set", json!({"ids": [poster_body]}));
    u.frame(vec![]);
    (u, ids)
}

#[test]
fn collapse_then_restore_gives_back_the_identical_rows_and_writes_nothing() {
    let (mut u, [_, poster_body, ..]) = partly_open();
    // (Reading the change log is a command too: read it before the document's state.)
    let feed_before = u.app.run("history.changes", json!({})).unwrap()["changes"].clone();
    let (before_open, before_rows, doc) = (open(&u), rows(&mut u), doc_state(&u));
    assert_eq!(before_rows.len(), 5, "{before_rows:?}");
    let (name, pressed, key) = button(&mut u);
    assert_eq!((name.as_str(), pressed), (COLLAPSE, false), "not pressed, named for the collapse");
    assert!(key.is_some_and(|k| k.contains('L')), "with its key (⌥L)");
    click_named(&mut u, COLLAPSE);
    assert_eq!(rows(&mut u), ["Kiln", "Poster"]);
    assert_eq!(u.app.session.selection(), [poster_body], "the selection survives the collapse");
    let (name, pressed, key) = button(&mut u);
    assert_eq!((name.as_str(), pressed, key), (RESTORE, true, None), "pressed, named for the restore, no key");
    assert_eq!((&u.app.inspect()["layers"]["open"], &u.app.inspect()["layers"]["restoreNext"]), (&json!([]), &json!(true)), "ui.inspect says so too");
    click_named(&mut u, RESTORE);
    assert_eq!(open(&u), before_open, "exactly the rows open before");
    assert_eq!(rows(&mut u), before_rows);
    assert_eq!(u.app.session.selection(), [poster_body], "the selection survives the restore");
    assert_eq!(button(&mut u).0, COLLAPSE, "the next press collapses again");
    assert_doc_untouched(&u, &doc, "collapse and restore");
    let feed = u.app.run("history.changes", json!({})).unwrap()["changes"].clone();
    assert_eq!(feed, feed_before, "nothing logged, so no \"Agent: …\" label");
}

/// Ten frames, each holding four rectangles, every row open: a list longer than the panel, whose
/// top-level rows alone fit in it.
fn long_list() -> Ui {
    let mut s = figcraft_engine::Session::new();
    let mut frames = Vec::new();
    for i in 0..10 {
        let f = s.execute("frame.create", &json!({"x": 120 * i, "y": 0, "width": 100, "height": 100, "name": format!("Frame {i:02}")})).unwrap();
        let f = id(Some(f));
        for _ in 0..4 {
            s.execute("shape.rectangle", &json!({"x": 5, "y": 5, "width": 20, "height": 20, "parent": f})).unwrap();
        }
        frames.push(f);
    }
    s.execute("select.none", &json!({})).unwrap();
    let mut u = Ui::new(s);
    u.app.ui.expanded.extend(frames);
    u.frame(vec![]);
    u
}

#[test]
fn restore_scrolls_the_list_back_to_where_it_was() {
    let mut u = long_list();
    // Scroll the list down with the wheel, over its rows.
    u.frame(vec![Event::PointerMoved(pos2(150.0, 500.0))]);
    for _ in 0..4 {
        u.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -150.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        }]);
    }
    for _ in 0..40 {
        u.frame(vec![]);
    }
    let scrolled = u.app.inspect()["layers"]["scroll"].as_f64().unwrap_or_default();
    assert!(scrolled > 100.0, "scrolled down first: {scrolled}");
    let before = rows(&mut u);
    assert!(!before.contains(&"Frame 09".to_string()), "the top rows are scrolled out of view: {before:?}");
    click_named(&mut u, COLLAPSE);
    assert!(rows(&mut u).contains(&"Frame 09".to_string()), "collapsed, the list is back at its top");
    click_named(&mut u, RESTORE);
    assert_eq!(rows(&mut u), before, "the same rows in view: scrolled back, not only reopened");
    assert_eq!(u.app.inspect()["layers"]["scroll"].as_f64(), Some(scrolled));
}

#[test]
fn a_row_opened_by_hand_drops_the_snapshot_and_the_next_press_takes_a_new_one() {
    let (mut u, [poster, poster_body, ..]) = partly_open();
    click_named(&mut u, COLLAPSE);
    // Open Poster with its disclosure arrow.
    let row = u.row_at(poster);
    let arrow = pos2(74.0, row.y);
    let press = |pressed| Event::PointerButton { pos: arrow, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    u.frame(vec![Event::PointerMoved(arrow)]);
    u.frame(vec![press(true)]);
    u.frame(vec![press(false)]);
    u.frame(vec![Event::PointerMoved(Pos2::new(900.0, 700.0))]);
    assert_eq!(open(&u), BTreeSet::from([poster]), "the arrow opened Poster");
    assert_eq!(next(&mut u), COLLAPSE, "a row opened by hand: the snapshot is gone");
    click_named(&mut u, COLLAPSE);
    assert!(open(&u).is_empty());
    click_named(&mut u, RESTORE);
    assert_eq!(open(&u), BTreeSet::from([poster]), "the new snapshot, not the first one");
    assert_eq!(u.app.session.selection(), [poster_body]);
    // Opened then closed again by hand: still gone, though the rows look as Collapse all left them.
    click_named(&mut u, COLLAPSE);
    u.app.ui.expanded.insert(poster);
    u.frame(vec![]);
    u.app.ui.expanded.remove(&poster);
    u.frame(vec![]);
    assert_eq!(next(&mut u), COLLAPSE, "opened and closed by hand");
    u.app.run("layers.restore-expansion", json!({}));
    assert!(open(&u).is_empty(), "the command restores nothing either");
}

#[test]
fn another_page_a_structural_change_or_another_file_drops_the_snapshot_and_a_rename_keeps_it() {
    type Change = fn(&mut Ui, &[NodeId; 4], NodeId, NodeId);
    let cases: [(&str, Change); 9] = [
        ("another page", |u, _, _, other_page| {
            u.app.run("page.select", json!({"id": other_page}));
        }),
        ("a layer added", |u, ids, _, _| {
            u.app.run("shape.rectangle", json!({"x": 1, "y": 1, "width": 5, "height": 5, "parent": ids[1]}));
        }),
        ("a layer deleted", |u, _, extra, _| {
            u.app.run("edit.delete", json!({"ids": [extra]}));
        }),
        ("a layer moved to another parent", |u, ids, extra, _| {
            u.app.run("node.reparent", json!({"ids": [extra], "parent": ids[3]}));
        }),
        ("an undo of an add made before the collapse", |u, _, _, _| {
            u.app.run("edit.undo", json!({}));
        }),
        ("an add and its undo, between two frames", |u, ids, _, _| {
            u.app.run("shape.rectangle", json!({"x": 1, "y": 1, "width": 5, "height": 5, "parent": ids[1]}));
            u.app.run("edit.undo", json!({}));
        }),
        ("a redo of a delete", |u, _, extra, _| {
            u.app.run("edit.delete", json!({"ids": [extra]}));
            u.app.run("layers.collapse-all", json!({}));
            u.app.run("edit.undo", json!({}));
            u.app.run("layers.collapse-all", json!({}));
            u.app.run("edit.redo", json!({}));
        }),
        ("another file", |u, _, _, _| {
            u.app.run("file.new", json!({}));
        }),
        ("a rename (kept)", |u, ids, _, _| {
            u.app.run("object.rename", json!({"ids": [ids[0]], "name": "Poster 2"}));
        }),
    ];
    for (what, change) in cases {
        let (mut u, ids) = partly_open();
        // A layer added before the collapse (for the delete, reparent and undo), and a second page.
        let extra = id(u.app.run("shape.rectangle", json!({"x": 2, "y": 2, "width": 5, "height": 5, "parent": ids[1]})));
        let first_page = u.app.session.ds().unwrap().page;
        let other_page = id(u.app.run("page.add", json!({"name": "Other"})));
        u.app.run("page.select", json!({"id": first_page}));
        u.app.run("select.set", json!({"ids": [ids[1]]}));
        u.frame(vec![]);
        let before = open(&u);
        click_named(&mut u, COLLAPSE);
        assert_eq!(next(&mut u), RESTORE, "{what}: a snapshot first");
        change(&mut u, &ids, extra, other_page);
        u.frame(vec![]);
        let kept = what.ends_with("(kept)");
        assert_eq!(next(&mut u), if kept { RESTORE } else { COLLAPSE }, "{what}");
        let still = open(&u);
        u.app.run("layers.restore-expansion", json!({}));
        if kept {
            assert_eq!(open(&u), before, "{what}: restored");
        } else {
            assert_eq!(open(&u), still, "{what}: the command restores nothing");
            // Back on the first file and page where it was taken: still gone, for good.
            if what == "another page" {
                u.app.run("page.select", json!({"id": first_page}));
                u.frame(vec![]);
                assert_eq!(next(&mut u), COLLAPSE, "{what}: and back again");
            }
        }
    }
}

#[test]
fn two_presses_on_a_tree_already_collapsed_restore_it_collapsed() {
    let (mut u, [poster, ..]) = nested();
    u.app.run("select.set", json!({"ids": [poster]}));
    u.app.ui.expanded.clear();
    u.frame(vec![]);
    u.app.ui.toast = None;
    click_named(&mut u, COLLAPSE);
    assert_eq!(next(&mut u), RESTORE);
    click_named(&mut u, RESTORE);
    assert!(open(&u).is_empty(), "restored to fully collapsed");
    assert_eq!(rows(&mut u), ["Kiln", "Poster"]);
    assert!(u.app.ui.toast.is_none(), "no error: {:?}", u.app.ui.toast);
    assert_eq!(u.app.session.selection(), [poster]);
}

#[test]
fn the_command_called_twice_never_restores() {
    let (mut u, _) = partly_open();
    let (before, doc) = (open(&u), doc_state(&u));
    // As the MCP route runs it: the command, then the window handles its request.
    for i in 0..3 {
        u.app.session.execute("layers.collapse-all", &json!({})).unwrap();
        u.app.handle_ui_requests();
        u.frame(vec![]);
        assert!(open(&u).is_empty(), "call {i}: collapsed, never restored");
    }
    // The snapshot from the first call is still there for whoever restores.
    u.app.session.execute("layers.restore-expansion", &json!({})).unwrap();
    u.app.handle_ui_requests();
    assert_eq!(open(&u), before);
    assert_doc_untouched(&u, &doc, "the commands");
}

#[test]
fn the_tooltip_matches_the_next_press() {
    let (mut u, _) = partly_open();
    for expect in [COLLAPSE, RESTORE] {
        // Hover the button until its tooltip shows (from outside it: egui hides a tooltip after a
        // click until the pointer leaves), then read the tooltip's text.
        u.frame(vec![Event::PointerMoved(Pos2::new(900.0, 700.0))]);
        let nodes = tree(&mut u);
        let b = nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some(expect)).expect("the button");
        let r = b.bounds().unwrap();
        let p = pos2(((r.x0 + r.x1) / 2.0) as f32, ((r.y0 + r.y1) / 2.0) as f32);
        u.frame(vec![Event::PointerMoved(p)]);
        for _ in 0..60 {
            u.frame(vec![]);
        }
        let shown: Vec<String> = tree(&mut u).iter().filter_map(|n| n.label().or(n.value()).map(str::to_string)).collect();
        let named = shown.iter().filter(|s| s.starts_with(expect)).count();
        assert!(named >= 2, "the button and its tooltip read {expect:?}: {shown:?}");
        if expect == COLLAPSE {
            click_named(&mut u, COLLAPSE);
        }
    }
}

#[test]
fn with_a_filter_the_button_collapses_and_takes_or_applies_no_snapshot() {
    let (mut u, _) = partly_open();
    u.app.ui.layer_search = true;
    u.app.ui.layer_filter = "swatch".into();
    u.frame(vec![]);
    let filtered = rows(&mut u);
    click_named(&mut u, COLLAPSE);
    assert!(open(&u).is_empty(), "D1's collapse");
    assert_eq!(rows(&mut u), filtered, "the filtered list is unchanged");
    assert!(u.app.inspect()["layers"]["restoreNext"] == json!(false), "no snapshot taken");
    assert_eq!(next(&mut u), COLLAPSE);
    // A snapshot taken before the filter is neither applied while it is active nor lost.
    let (mut u, _) = partly_open();
    let before = open(&u);
    click_named(&mut u, COLLAPSE);
    u.app.ui.layer_search = true;
    u.app.ui.layer_filter = "swatch".into();
    u.frame(vec![]);
    assert_eq!(next(&mut u), COLLAPSE, "filtering: the button collapses");
    u.app.run("layers.restore-expansion", json!({}));
    assert!(open(&u).is_empty(), "filtering: no restore");
    u.app.ui.layer_filter.clear();
    u.app.ui.layer_search = false;
    u.frame(vec![]);
    assert_eq!(next(&mut u), RESTORE);
    click_named(&mut u, RESTORE);
    assert_eq!(open(&u), before);
}

#[test]
fn the_icon_reads_at_3_to_1_in_both_states_and_both_themes() {
    use crate::theme::Tokens;
    use crate::theme::accent::contrast;
    for (theme, t) in [("light", Tokens::light()), ("dark", Tokens::dark())] {
        for (state, bg) in [("at rest", t.panel), ("hovered", t.hover), ("pressed (restore next)", t.pressed)] {
            let r = contrast(t.icon, bg);
            println!("{theme} {state}: {r:.2}:1");
            assert!(r >= 3.0, "{theme} {state}: {r:.2}:1");
        }
    }
}

/// R2: a layer deep in the tree selected without the panel revealing it first (by a command, with
/// no frame drawn before the collapse, as an agent's batch or the Assets tab leaves it), with
/// Poster's rows and Kiln's body closed by hand before.
fn deep_selection_unrevealed() -> (Ui, [NodeId; 4], NodeId) {
    let (mut u, ids @ [poster, poster_body, _, kiln_body]) = nested();
    for id in [poster, poster_body, kiln_body] {
        u.app.ui.expanded.remove(&id);
    }
    u.frame(vec![]);
    let swatch = u.app.session.doc().unwrap().node(poster_body).and_then(|n| n.children.first().copied()).expect("Poster swatch");
    u.app.run("select.set", json!({"ids": [swatch]}));
    (u, ids, swatch)
}

#[test]
fn restore_keeps_the_way_to_a_deep_selection_open_and_the_rest_exact() {
    let (mut u, [poster, poster_body, kiln, kiln_body], swatch) = deep_selection_unrevealed();
    let snapshot = open(&u);
    assert_eq!(snapshot, BTreeSet::from([kiln]), "the rows open when the collapse comes: the selection not revealed yet");
    let feed_before = u.app.run("history.changes", json!({})).unwrap()["changes"].clone();
    let doc = doc_state(&u);
    u.app.run("layers.collapse-all", json!({}));
    u.frame(vec![]);
    assert_eq!(rows(&mut u), ["Kiln", "Poster"], "Collapse all is D1's: every row closes");
    click_named(&mut u, RESTORE);
    let want = BTreeSet::from([kiln, poster, poster_body]);
    assert_eq!(open(&u), want, "the snapshot's rows, plus the way to the selection");
    assert!(!open(&u).contains(&kiln_body), "a row closed by hand before stays closed: otherwise exact");
    assert!(rows(&mut u).iter().any(|r| r == "Poster swatch"), "the selected layer is listed: {:?}", rows(&mut u));
    assert_eq!(u.app.session.selection(), [swatch], "the selection survives both presses");
    assert_eq!(button(&mut u).0, COLLAPSE);
    assert_doc_untouched(&u, &doc, "collapse and restore with a deep selection");
    let feed = u.app.run("history.changes", json!({})).unwrap()["changes"].clone();
    assert_eq!(feed, feed_before, "no undo entry and nothing logged");
}

#[test]
fn an_agents_restore_command_keeps_the_way_to_the_selection_open_too() {
    let (mut u, [poster, poster_body, kiln, _], _) = deep_selection_unrevealed();
    let doc = doc_state(&u);
    let revision = u.app.session.ds().unwrap().revision;
    crate::control::handle(&mut u.app, &u.ctx.clone(), "engine.execute", &json!({"command": "layers.collapse-all"})).unwrap();
    u.frame(vec![]);
    crate::control::handle(&mut u.app, &u.ctx.clone(), "engine.execute", &json!({"command": "layers.restore-expansion"})).unwrap();
    u.frame(vec![]);
    assert_eq!(open(&u), BTreeSet::from([kiln, poster, poster_body]));
    assert_eq!(u.app.session.ds().unwrap().revision, revision, "no revision bump");
    assert_doc_untouched(&u, &doc, "the agent's collapse and restore");
}

#[test]
fn with_its_rows_already_in_the_snapshot_the_restore_is_unchanged() {
    // The common case: the selection was revealed before the collapse, so its rows are in the
    // snapshot and the restore gives back exactly the rows open before.
    let (mut u, [_, poster_body, ..]) = partly_open();
    let before = open(&u);
    assert!(before.contains(&poster_body));
    click_named(&mut u, COLLAPSE);
    click_named(&mut u, RESTORE);
    assert_eq!(open(&u), before);
}
