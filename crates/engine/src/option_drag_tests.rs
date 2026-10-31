//! Fix AE: Option-drag to duplicate. Option decides throughout the drag (pressed mid-drag it
//! copies and the original snaps back; let go it moves again); the copy is only a preview until
//! the release, which runs `edit.duplicate` as one undo step; Escape gives the document back.

use serde_json::{Value, json};

use crate::Session;
use crate::params::id_of;
use crate::tools::{Cursor, PointerEvent, PointerKind};
use figcraft_doc::{NodeId, NodeKind};

fn ex(s: &mut Session, cmd: &str, p: Value) -> Value {
    s.execute(cmd, &p).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

fn idv(v: &Value) -> NodeId {
    id_of(if v.get("id").is_some() { &v["id"] } else { &v["ids"][0] }).unwrap()
}

fn rect(s: &mut Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    idv(&ex(s, "shape.rectangle", json!({"x": x, "y": y, "width": w, "height": h})))
}

fn frame(s: &mut Session, name: &str, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    idv(&ex(s, "frame.create", json!({"x": x, "y": y, "width": w, "height": h, "name": name})))
}

fn select(s: &mut Session, ids: &[NodeId]) {
    ex(s, "select.set", json!({"ids": ids}));
}

fn ev(kind: PointerKind, (x, y): (f64, f64), alt: bool, shift: bool) -> PointerEvent {
    PointerEvent { alt, shift, ..PointerEvent::at(kind, x, y, 1.0) }
}

/// Press at `from` and move to `to` in ten steps, Option held at step `k` when `alt(k)` (step 0 is
/// the press). The pointer stays down.
fn press_and_move(s: &mut Session, from: (f64, f64), to: (f64, f64), alt: &dyn Fn(usize) -> bool, shift: bool) {
    s.pointer(&ev(PointerKind::Down, from, alt(0), shift)).unwrap();
    for k in 1..=10 {
        let t = k as f64 / 10.0;
        s.pointer(&ev(PointerKind::Move, (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t), alt(k), shift)).unwrap();
    }
}

/// A whole drag; the release has Option as step 10 had it.
fn drag(s: &mut Session, from: (f64, f64), to: (f64, f64), alt: &dyn Fn(usize) -> bool) {
    press_and_move(s, from, to, alt, false);
    s.pointer(&ev(PointerKind::Up, to, alt(10), false)).unwrap();
}

fn held(_: usize) -> bool {
    true
}

fn never(_: usize) -> bool {
    false
}

fn at(s: &Session, id: NodeId) -> (f64, f64) {
    let b = s.doc().unwrap().world_bounds(id).unwrap();
    (b.x0, b.y0)
}

fn kids(s: &Session, parent: NodeId) -> Vec<NodeId> {
    s.doc().unwrap().node(parent).unwrap().children.clone()
}

fn page(s: &Session) -> NodeId {
    s.ds().unwrap().page
}

fn parent(s: &Session, id: NodeId) -> Option<NodeId> {
    s.doc().unwrap().node(id).and_then(|n| n.parent)
}

fn undo_labels(s: &Session) -> Vec<String> {
    s.ds().unwrap().history.undo.iter().map(|h| h.label.clone()).collect()
}

fn bytes(s: &Session) -> Vec<u8> {
    serde_json::to_vec(&*s.ds().unwrap().doc).unwrap()
}

/// Option held from the press: the original stays, one copy lands at the drop point directly above
/// it in the same parent, and the copy is selected.
#[test]
fn option_drag_leaves_the_original_and_drops_a_selected_copy_above_it() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    select(&mut s, &[]);
    drag(&mut s, (140.0, 130.0), (290.0, 170.0), &held);
    let pg = page(&s);
    let layers = kids(&s, pg);
    assert_eq!(layers.len(), 2, "one copy");
    assert_eq!(layers[0], a, "the original keeps its place, the copy goes directly above it");
    let copy = layers[1];
    assert_eq!(at(&s, a), (100.0, 100.0), "the original stays");
    assert_eq!(at(&s, copy), (250.0, 140.0), "the copy is where it was dropped");
    assert_eq!(s.selection(), vec![copy]);
    assert_eq!(undo_labels(&s).last().map(String::as_str), Some("Duplicate"));
}

/// Without Option the layer moves, as before.
#[test]
fn a_drag_without_option_moves() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    drag(&mut s, (140.0, 130.0), (290.0, 170.0), &never);
    assert_eq!(kids(&s, page(&s)), vec![a]);
    assert_eq!(at(&s, a), (250.0, 140.0));
    assert_eq!(undo_labels(&s).last().map(String::as_str), Some("Move"));
}

/// One undo removes the copy and selects the original again; redo brings the copy back, selected.
#[test]
fn one_undo_removes_the_copy_and_redo_brings_it_back() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    select(&mut s, &[a]);
    let steps = undo_labels(&s).len();
    drag(&mut s, (140.0, 130.0), (290.0, 170.0), &held);
    assert_eq!(undo_labels(&s).len(), steps + 1, "one undo step");
    let copy = s.selection()[0];
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(kids(&s, page(&s)), vec![a]);
    assert_eq!(s.selection(), vec![a], "the selection from before the drag");
    assert_eq!(undo_labels(&s).len(), steps);
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(kids(&s, page(&s)), vec![a, copy]);
    assert_eq!(at(&s, copy), (250.0, 140.0));
    assert_eq!(s.selection(), vec![copy]);
}

/// Escape mid-drag, Option held: the document is the one from before the press, byte for byte, with
/// no undo step and nothing in the command log; the release that follows does nothing.
#[test]
fn escape_mid_drag_gives_back_the_same_document_byte_for_byte() {
    let mut s = Session::new();
    let f = frame(&mut s, "Card", 0.0, 0.0, 300.0, 200.0);
    ex(&mut s, "shape.rectangle", json!({"x": 20, "y": 20, "width": 40, "height": 40, "parent": f}));
    select(&mut s, &[f]);
    let (before, steps, logged) = (bytes(&s), undo_labels(&s), s.journal.len());
    press_and_move(&mut s, (200.0, 150.0), (500.0, 300.0), &held, false);
    assert_ne!(bytes(&s), before, "the copy shows while dragging");
    s.escape().unwrap();
    assert_eq!(bytes(&s), before, "cancelled at Escape");
    s.pointer(&ev(PointerKind::Up, (500.0, 300.0), true, false)).unwrap();
    assert_eq!(bytes(&s), before);
    assert_eq!(undo_labels(&s), steps);
    assert_eq!(s.journal.len(), logged);
    assert_eq!(s.selection(), vec![f]);
}

/// Option pressed mid-drag: from then on a copy follows the pointer and the original is back where
/// it started; the release drops the copy.
#[test]
fn option_pressed_mid_drag_copies_and_the_original_snaps_back() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    press_and_move(&mut s, (140.0, 130.0), (290.0, 170.0), &never, false);
    assert_eq!(kids(&s, page(&s)).len(), 1, "moving, no copy yet");
    assert_eq!(at(&s, a), (250.0, 140.0), "the original follows the pointer");
    s.pointer(&ev(PointerKind::Move, (290.0, 170.0), true, false)).unwrap();
    assert_eq!(at(&s, a), (100.0, 100.0), "the original snaps back");
    assert_eq!(kids(&s, page(&s)).len(), 2, "a copy follows the pointer");
    s.pointer(&ev(PointerKind::Up, (290.0, 170.0), true, false)).unwrap();
    let layers = kids(&s, page(&s));
    assert_eq!(layers.len(), 2);
    assert_eq!((at(&s, a), at(&s, layers[1])), ((100.0, 100.0), (250.0, 140.0)));
}

/// Option let go mid-drag: the copy goes and the original follows the pointer; the release moves it.
#[test]
fn option_released_mid_drag_moves_instead() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    press_and_move(&mut s, (140.0, 130.0), (290.0, 170.0), &held, false);
    assert_eq!(kids(&s, page(&s)).len(), 2, "a copy while Option is held");
    s.pointer(&ev(PointerKind::Move, (290.0, 170.0), false, false)).unwrap();
    assert_eq!(kids(&s, page(&s)), vec![a], "no copy once it is let go");
    assert_eq!(at(&s, a), (250.0, 140.0));
    s.pointer(&ev(PointerKind::Up, (290.0, 170.0), false, false)).unwrap();
    assert_eq!(kids(&s, page(&s)), vec![a]);
    assert_eq!(at(&s, a), (250.0, 140.0));
    assert_eq!(undo_labels(&s).last().map(String::as_str), Some("Move"));
}

/// Pressed then let go, and let go then pressed: what counts is Option at the release.
#[test]
fn option_toggled_both_ways_mid_drag_ends_as_it_is_at_the_release() {
    let pressed_then_let_go = |k: usize| (4..=6).contains(&k);
    let let_go_then_pressed = |k: usize| k <= 3 || k >= 7;
    for (alt, copies) in [(&pressed_then_let_go as &dyn Fn(usize) -> bool, false), (&let_go_then_pressed, true)] {
        let mut s = Session::new();
        let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
        drag(&mut s, (140.0, 130.0), (290.0, 170.0), alt);
        assert_eq!(kids(&s, page(&s)).len(), if copies { 2 } else { 1 });
        assert_eq!(at(&s, a), if copies { (100.0, 100.0) } else { (250.0, 140.0) });
    }
    // Option let go between the last move and the release: a move.
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    press_and_move(&mut s, (140.0, 130.0), (290.0, 170.0), &held, false);
    s.pointer(&ev(PointerKind::Up, (290.0, 170.0), false, false)).unwrap();
    assert_eq!(kids(&s, page(&s)), vec![a]);
    assert_eq!(at(&s, a), (250.0, 140.0));
}

/// A multiple selection is copied together, keeping its layers' positions relative to each other.
#[test]
fn a_multiple_selection_is_copied_together() {
    let mut s = Session::new();
    let a = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let b = rect(&mut s, 200.0, 50.0, 50.0, 50.0);
    select(&mut s, &[a, b]);
    drag(&mut s, (50.0, 50.0), (350.0, 250.0), &held);
    assert_eq!(kids(&s, page(&s)).len(), 4);
    assert_eq!((at(&s, a), at(&s, b)), ((0.0, 0.0), (200.0, 50.0)), "the originals stay");
    let copies = s.selection();
    assert_eq!(copies.len(), 2);
    assert_eq!((at(&s, copies[0]), at(&s, copies[1])), ((300.0, 200.0), (500.0, 250.0)));
}

/// A frame is copied with everything in it, nested frames too, and its copy is a sibling of the
/// original even when dropped over it.
#[test]
fn a_frame_is_copied_deeply_and_not_into_itself() {
    let mut s = Session::new();
    let f = frame(&mut s, "Card", 0.0, 0.0, 300.0, 200.0);
    let g = idv(&ex(&mut s, "frame.create", json!({"x": 100, "y": 20, "width": 120, "height": 120, "name": "Inner", "parent": f})));
    ex(&mut s, "shape.rectangle", json!({"x": 10, "y": 10, "width": 40, "height": 40, "parent": g, "name": "Dot"}));
    ex(&mut s, "shape.rectangle", json!({"x": 20, "y": 150, "width": 40, "height": 20, "parent": f, "name": "Tag"}));
    select(&mut s, &[f]);
    // Dropped 60 to the right: the pointer is still over the original.
    drag(&mut s, (40.0, 40.0), (100.0, 60.0), &held);
    let copy = s.selection()[0];
    assert_eq!(parent(&s, copy), Some(page(&s)), "beside the original, not inside it");
    assert_eq!(kids(&s, page(&s)), vec![f, copy]);
    let doc = s.doc().unwrap();
    let names = |id: NodeId| doc.subtree(id).iter().map(|i| doc.node(*i).unwrap().name.clone()).collect::<Vec<_>>();
    assert_eq!(names(copy), names(f), "the same layers, nested the same way");
    assert!(doc.subtree(copy).iter().all(|i| !doc.subtree(f).contains(i)), "all of them new");
    assert_eq!(at(&s, copy), (60.0, 20.0));
}

/// A main component copies to a new component, an instance to an instance of the same component;
/// both are named as Duplicate names them (here with Rename duplicated layers on).
#[test]
fn components_and_instances_copy_as_duplicate_copies_them() {
    let mut s = Session::new();
    ex(&mut s, "pref.renameDuplicates", json!({"on": true}));
    let f = frame(&mut s, "Button 1", 0.0, 0.0, 100.0, 40.0);
    let comp = idv(&ex(&mut s, "component.create", json!({"ids": [f]})));
    let inst = idv(&ex(&mut s, "instance.create", json!({"component": comp, "x": 0, "y": 200})));
    select(&mut s, &[comp]);
    drag(&mut s, (50.0, 20.0), (50.0, 120.0), &held);
    let c2 = s.selection()[0];
    select(&mut s, &[inst]);
    drag(&mut s, (50.0, 220.0), (250.0, 220.0), &held);
    let i2 = s.selection()[0];
    let doc = s.doc().unwrap();
    let (n, m) = (doc.node(c2).unwrap(), doc.node(i2).unwrap());
    assert!(c2 != comp && matches!(n.kind, NodeKind::Component { .. }), "a new main component");
    assert_eq!(n.name, "Button 2");
    assert!(i2 != inst && matches!(m.kind, NodeKind::Instance { component, .. } if component == comp), "an instance of the same component");
    // Cmd+D names them the same.
    let mut t = Session::new();
    ex(&mut t, "pref.renameDuplicates", json!({"on": true}));
    let tf = frame(&mut t, "Button 1", 0.0, 0.0, 100.0, 40.0);
    let tc = idv(&ex(&mut t, "component.create", json!({"ids": [tf]})));
    let d = idv(&ex(&mut t, "edit.duplicate", json!({"ids": [tc]})));
    assert_eq!(t.doc().unwrap().node(d).unwrap().name, n.name);
}

fn auto_layout_row(s: &mut Session) -> (NodeId, Vec<NodeId>) {
    let row = frame(s, "Row", 0.0, 0.0, 400.0, 50.0);
    ex(s, "node.set", json!({"id": row, "layout": "horizontal", "gap": 0, "padding": 0, "sizingH": "fixed", "sizingV": "fixed"}));
    let cells: Vec<NodeId> = ["A", "B", "C"]
        .iter()
        .map(|n| idv(&ex(s, "shape.rectangle", json!({"x": 0, "y": 0, "width": 50, "height": 50, "parent": row, "name": n}))))
        .collect();
    (row, cells)
}

/// In an auto layout frame the copy takes the slot under the pointer and the original stays in its own.
#[test]
fn in_auto_layout_the_copy_takes_its_slot_and_the_original_stays() {
    let mut s = Session::new();
    let (row, cells) = auto_layout_row(&mut s);
    select(&mut s, &[cells[0]]);
    drag(&mut s, (25.0, 25.0), (140.0, 25.0), &held);
    let copy = s.selection()[0];
    assert_eq!(kids(&s, row), vec![cells[0], cells[1], cells[2], copy], "the copy after C, A still first");
    assert_eq!(at(&s, copy), (150.0, 0.0), "placed by the layout");
    assert_eq!(s.doc().unwrap().node(copy).unwrap().name, "A");
}

/// Dropped over another frame, the copy goes into it (as a moved layer would); dropped on empty
/// canvas, a copy of an auto layout child leaves the frame; the originals stay where they were.
#[test]
fn a_copy_is_reparented_where_it_is_dropped() {
    let mut s = Session::new();
    let (row, cells) = auto_layout_row(&mut s);
    let other = frame(&mut s, "Other", 600.0, 0.0, 200.0, 200.0);
    let r = rect(&mut s, 100.0, 300.0, 50.0, 50.0);
    drag(&mut s, (125.0, 325.0), (700.0, 100.0), &held);
    let into = s.selection()[0];
    assert_eq!(parent(&s, into), Some(other));
    assert_eq!(at(&s, into), (675.0, 75.0));
    assert_eq!((parent(&s, r), at(&s, r)), (Some(page(&s)), (100.0, 300.0)));
    select(&mut s, &[cells[1]]);
    drag(&mut s, (75.0, 25.0), (375.0, 525.0), &held);
    let out = s.selection()[0];
    assert_eq!(parent(&s, out), Some(page(&s)));
    assert_eq!(at(&s, out), (350.0, 500.0));
    assert_eq!(kids(&s, row), cells, "the row keeps its three");
}

/// Option+Shift: the copy moves along one axis only.
#[test]
fn option_shift_locks_the_copy_to_one_axis() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    press_and_move(&mut s, (140.0, 130.0), (290.0, 150.0), &held, true);
    s.pointer(&ev(PointerKind::Up, (290.0, 150.0), true, true)).unwrap();
    let copy = s.selection()[0];
    assert_ne!(copy, a);
    assert_eq!((at(&s, a), at(&s, copy)), ((100.0, 100.0), (250.0, 100.0)));
}

/// Snapping applies to the copy: a few units off the original's top edge, it lines up with it.
#[test]
fn the_copy_snaps() {
    let mut s = Session::new();
    rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    drag(&mut s, (140.0, 130.0), (340.0, 133.0), &held);
    let copy = s.selection()[0];
    assert_eq!(at(&s, copy), (300.0, 100.0));
}

/// A locked layer isn't copied: not on its own, and not with a selection it is in.
#[test]
fn a_locked_layer_is_not_copied() {
    let mut s = Session::new();
    let a = rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let l = rect(&mut s, 300.0, 0.0, 100.0, 100.0);
    ex(&mut s, "node.set", json!({"id": l, "locked": true}));
    select(&mut s, &[]);
    let before = bytes(&s);
    drag(&mut s, (350.0, 50.0), (550.0, 50.0), &held);
    assert_eq!(bytes(&s), before, "nothing copied or moved");
    select(&mut s, &[a, l]);
    drag(&mut s, (50.0, 50.0), (50.0, 250.0), &held);
    let layers = kids(&s, page(&s));
    assert_eq!(layers.len(), 3, "only the unlocked one copied");
    assert_eq!(at(&s, layers[1]), (0.0, 200.0));
    assert_eq!(at(&s, l), (300.0, 0.0));
}

/// Option on empty canvas makes a marquee; Option on a resize handle resizes from the centre.
#[test]
fn option_on_empty_canvas_or_a_handle_does_what_it_did() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    select(&mut s, &[]);
    let before = bytes(&s);
    drag(&mut s, (50.0, 50.0), (300.0, 300.0), &held);
    assert_eq!(s.selection(), vec![a], "a marquee selected it");
    assert_eq!(bytes(&s), before);
    select(&mut s, &[a]);
    drag(&mut s, (180.0, 160.0), (200.0, 180.0), &held);
    assert_eq!(kids(&s, page(&s)), vec![a], "no copy");
    let b = s.doc().unwrap().world_bounds(a).unwrap();
    assert_eq!((b.x0, b.y0, b.width(), b.height()), (80.0, 80.0, 120.0, 100.0), "resized about its centre");
}

/// Nothing is written while dragging: no command, no undo step, no autosave. The release runs
/// `edit.duplicate` once, with the drop's offset.
#[test]
fn the_drop_runs_edit_duplicate_and_nothing_is_written_before_it() {
    let mut s = Session::new();
    let a = rect(&mut s, 100.0, 100.0, 80.0, 60.0);
    select(&mut s, &[a]);
    let (logged, steps) = (s.journal.len(), undo_labels(&s));
    press_and_move(&mut s, (140.0, 130.0), (290.0, 170.0), &held, false);
    assert_eq!(s.journal.len(), logged, "no command while dragging");
    assert_eq!(undo_labels(&s), steps, "no undo step while dragging");
    assert!(s.ds().unwrap().interaction.is_some());
    assert_eq!(ex(&mut s, "version.autosave", json!({}))["saved"], false, "no autosave of the preview");
    let logged = s.journal.len();
    s.pointer(&ev(PointerKind::Up, (290.0, 170.0), true, false)).unwrap();
    let ran: Vec<&(String, Value)> = s.journal[logged..].iter().collect();
    assert_eq!(ran.len(), 1, "{ran:?}");
    assert_eq!(ran[0].0, "edit.duplicate");
    assert_eq!((id_of(&ran[0].1["ids"][0]), ran[0].1["dx"].as_f64(), ran[0].1["dy"].as_f64()), (Some(a), Some(150.0), Some(40.0)));
    assert_eq!(ex(&mut s, "version.autosave", json!({}))["saved"], true, "after the drop it autosaves as after any edit");
}

/// The copy cursor shows with Option over any layer a drag would move (selected or not), and not
/// over a locked layer or empty canvas.
#[test]
fn the_copy_cursor_shows_over_layers_a_drag_would_copy() {
    let mut s = Session::new();
    rect(&mut s, 0.0, 0.0, 100.0, 100.0);
    let l = rect(&mut s, 300.0, 0.0, 100.0, 100.0);
    ex(&mut s, "node.set", json!({"id": l, "locked": true}));
    select(&mut s, &[]);
    let cursor = |s: &mut Session, p: (f64, f64), alt: bool| {
        s.pointer(&ev(PointerKind::Hover, p, alt, false)).unwrap();
        s.tool.overlay.cursor
    };
    assert_eq!(cursor(&mut s, (50.0, 50.0), true), Cursor::Copy, "over an unselected layer");
    assert_eq!(cursor(&mut s, (50.0, 50.0), false), Cursor::Default);
    assert_ne!(cursor(&mut s, (350.0, 50.0), true), Cursor::Copy, "over a locked layer");
    assert_eq!(cursor(&mut s, (200.0, 300.0), true), Cursor::Default, "over empty canvas");
}
