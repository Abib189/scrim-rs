//! I3, the reveal: a 16-layer batch enters in creation order, 40 ms apart, within the cap (0.6 s
//! once degraded); the person's edits, undo and redo, opening a file, Animations Off and the switch
//! skip it; it's render-only (the document, its revision and undo history are untouched, and MCP
//! `screenshot`, an export and `inspect_document` mid-reveal equal those after); while paused it
//! plays for work in view and never leaves work that arrived off screen hidden; moves tween and
//! deletes fade; dropped frames finish it; the working indicator runs in the same frames; and
//! under load the steps are taken in order with the fade kept.

use std::sync::Arc;

use figcraft_doc::{Document, NodeId};
use serde_json::{Value, json};

use super::{CAP, CAP_DEGRADED, ENTER, STAGGER, canvas_doc, entering, schedule};
use crate::follow_agent::tests::{agent, frames, id, poster, scene};
use crate::layers_drag_tests::Ui;

/// The scene with the frame budget pinned (debug builds' frame times aren't the app's).
fn calm() -> (Ui, NodeId) {
    let (mut u, home) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    (u, home)
}

/// A poster built by an agent in one go, far to the right: the frame, then 15 layers in it (rules,
/// shapes, text), all landing before the next frame.
fn poster_batch(u: &mut Ui) -> Vec<NodeId> {
    let p = poster(u, 8000.0);
    let mut ids = vec![p];
    for i in 0..15 {
        let r = match i % 3 {
            0 => agent(u, "shape.rectangle", json!({"x": 20, "y": 20 + 16 * i, "width": 300, "height": 2, "parent": p})),
            1 => agent(u, "shape.ellipse", json!({"x": 40 + 10 * i, "y": 60, "width": 30, "height": 30, "parent": p})),
            _ => agent(u, "text.create", json!({"text": format!("Line {i}"), "x": 30, "y": 100 + 12 * i, "parent": p})),
        };
        ids.push(id(&r));
    }
    ids
}

fn doc_text(d: &Document) -> String {
    serde_json::to_string(d).unwrap()
}

fn opacity(d: &Document, id: NodeId) -> f32 {
    d.node(id).unwrap().opacity
}

#[test]
fn a_batch_is_staggered_in_creation_order_within_the_cap() {
    let ids: Vec<NodeId> = (1..=16).map(NodeId).collect();
    let s = schedule(&ids, 10.0, 10.0, CAP);
    assert_eq!(s.iter().map(|(i, _)| *i).collect::<Vec<_>>(), ids, "creation order");
    assert!(s.windows(2).all(|w| (w[1].1 - w[0].1 - STAGGER).abs() < 1e-9), "40 ms apart: {s:?}");
    // 100 layers: the stagger shrinks so the last ends within the cap.
    let many: Vec<NodeId> = (1..=100).map(NodeId).collect();
    let s = schedule(&many, 10.0, 10.0, CAP);
    let last = s.last().unwrap().1;
    assert!(last + ENTER <= 10.0 + CAP + 1e-9, "{last}");
    assert!(s.windows(2).all(|w| w[1].1 >= w[0].1), "still in order");
    let s = schedule(&many, 10.0, 10.0, CAP_DEGRADED);
    assert!(s.last().unwrap().1 + ENTER <= 10.0 + CAP_DEGRADED + 1e-9);
    // A batch queued behind another never starts later than the cap allows.
    let s = schedule(&ids, 10.0, 50.0, CAP);
    assert!(s.iter().all(|(_, t)| *t + ENTER <= 10.0 + CAP + 1e-9));
}

#[test]
fn over_60_layers_only_the_roots_of_what_was_created_enter() {
    let mut s = figcraft_engine::Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 900, "height": 900})).unwrap());
    let kids: Vec<NodeId> =
        (0..70).map(|i| id(&s.execute("shape.rectangle", &json!({"x": i * 10, "y": 5, "width": 5, "height": 5, "parent": f})).unwrap())).collect();
    let doc = s.doc().unwrap();
    let created: Vec<NodeId> = std::iter::once(f).chain(kids.iter().copied()).collect();
    assert_eq!(entering(doc, &created), [f], "the frame, not 70 leaves");
    assert_eq!(entering(doc, &kids[..10]).len(), 10, "a few: each one");
}

#[test]
fn a_16_layer_batch_enters_in_order_and_the_document_never_sees_it() {
    let (mut u, _) = calm();
    let (revision, undo) = {
        let ds = u.app.session.ds().unwrap();
        (ds.revision, ds.history.undo.len())
    };
    let ids = poster_batch(&mut u);
    let (revision, undo) = {
        let ds = u.app.session.ds().unwrap();
        assert!(ds.revision > revision && ds.history.undo.len() > undo);
        (ds.revision, ds.history.undo.len())
    };
    let session = u.app.session.ds().unwrap().doc.clone();
    let final_text = doc_text(&session);
    u.frame(vec![]);
    let queued: Vec<(NodeId, f64)> = u.app.ui.agent_view.reveal.entering().collect();
    assert_eq!(queued.iter().map(|(i, _)| *i).collect::<Vec<_>>(), ids, "every layer, in creation order");
    assert!(queued.windows(2).all(|w| (w[1].1 - w[0].1 - STAGGER).abs() < 1e-9), "40 ms apart");
    // Mid-reveal: the canvas draws a copy with the later layers still invisible; the session's
    // document is the final one, unchanged.
    frames(&mut u, 3);
    let drawn = canvas_doc(&u.app).expect("the canvas draws the reveal");
    assert!(!Arc::ptr_eq(&drawn, &u.app.session.ds().unwrap().doc));
    let last = *ids.last().unwrap();
    assert_eq!(opacity(&drawn, last), 0.0, "the last layer hasn't started");
    assert!(opacity(&drawn, ids[0]) > 0.0, "the first has");
    assert_eq!(opacity(&u.app.session.ds().unwrap().doc, last), 1.0, "the document has it final");
    assert!(Arc::ptr_eq(&u.app.session.ds().unwrap().doc, &session), "never written");
    // After the cap, nothing is revealing and the canvas draws the document itself.
    frames(&mut u, (CAP * 60.0) as usize + 10);
    assert!(canvas_doc(&u.app).is_none());
    assert_eq!(u.app.ui.agent_view.reveal.entering().count(), 0);
    let ds = u.app.session.ds().unwrap();
    assert_eq!(doc_text(&ds.doc), final_text, "the document's content, during and after");
    assert_eq!((ds.revision, ds.history.undo.len()), (revision, undo), "no revision, no undo step from the reveal");
}

#[test]
fn screenshot_export_and_inspect_mid_reveal_equal_those_after() {
    let (mut u, _) = calm();
    let ids = poster_batch(&mut u);
    // After the camera's flight (320 ms), while the 16 entrances still run (0.6 s of stagger).
    frames(&mut u, 24);
    assert!(canvas_doc(&u.app).is_some(), "mid-reveal");
    assert!(u.app.ui.agent_view.indicator.drawn.get().is_some() || u.app.ui.agent_view.indicator.outlined.get().is_some(), "and the indicator");
    let ctx = u.ctx.clone();
    let take = |u: &mut Ui| -> (Value, Value, Value) {
        let shot = crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "ui.render", &json!({"scale": 1, "ids": [ids[0].0]})).unwrap();
        let svg = crate::control::handle_route(
            &mut u.app,
            &ctx,
            "mcp (http)",
            "engine.execute",
            &json!({"command": "file.export", "params": {"format": "svg", "ids": [ids[0].0]}}),
        )
        .unwrap();
        let inspect = crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "document.inspect", &json!({})).unwrap();
        (shot, svg, inspect)
    };
    let mid = take(&mut u);
    frames(&mut u, 120);
    assert!(canvas_doc(&u.app).is_none(), "after");
    let after = take(&mut u);
    assert_eq!(mid.0, after.0, "MCP screenshot (the render of the document)");
    assert_eq!(mid.1, after.1, "the export");
    assert_eq!(mid.2, after.2, "inspect_document");
}

#[test]
fn the_persons_edits_undo_redo_a_file_open_animations_off_and_the_switch_skip_it() {
    // The person's own layers.
    let (mut u, home) = calm();
    u.app.run("shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    u.frame(vec![]);
    assert_eq!(u.app.ui.agent_view.reveal.entering().count(), 0, "the person's edit");
    // An agent's undo and redo.
    let r = id(&agent(&mut u, "shape.rectangle", json!({"x": 40, "y": 10, "width": 20, "height": 20, "parent": home})));
    frames(&mut u, 90);
    agent(&mut u, "edit.undo", json!({}));
    u.frame(vec![]);
    agent(&mut u, "edit.redo", json!({}));
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.reveal.entering().all(|(i, _)| i != r), "redo brings it back final");
    // Opening a file: its layers are history, not new work.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ridgeline-app.figcraft");
    agent(&mut u, "file.open", json!({"path": path}));
    frames(&mut u, 2);
    assert_eq!(u.app.ui.agent_view.reveal.entering().count(), 0, "file open");
    // Animations Off, then the switch off: the agent's layers appear final.
    let (mut u, home) = calm();
    u.app.run("pref.motion", json!({"motion": "off"}));
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    u.frame(vec![]);
    assert!(canvas_doc(&u.app).is_none(), "Animations Off");
    let (mut u, home) = calm();
    let r = agent(&mut u, "pref.revealAgentWork", json!({"on": false}));
    assert_eq!(r["on"], false);
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    u.frame(vec![]);
    assert!(canvas_doc(&u.app).is_none(), "Reveal agent work off");
    assert!(u.app.ui.agent_view.indicator.artboard().is_some(), "the indicator is its own switch");
}

#[test]
fn while_paused_work_in_view_reveals_and_work_off_screen_is_never_left_hidden() {
    let (mut u, home) = calm();
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    frames(&mut u, 90);
    u.app.run("frame.create", json!({"x": 9000, "y": 0, "width": 10, "height": 10, "name": "Mine"}));
    frames(&mut u, 2);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "paused");
    // In view: revealed.
    let seen = id(&agent(&mut u, "shape.rectangle", json!({"x": 60, "y": 60, "width": 20, "height": 20, "parent": home})));
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.reveal.entering().any(|(i, _)| i == seen), "in view while paused: revealed");
    frames(&mut u, 90);
    // Off screen: drawn final at once, and still final when scrolled into view.
    let far = id(&agent(&mut u, "frame.create", json!({"x": 20000, "y": 0, "width": 200, "height": 200, "name": "Far"})));
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.reveal.entering().all(|(i, _)| i != far), "off screen: no entrance");
    let ds = u.app.session.ds_mut().unwrap();
    ds.view.cx = 20100.0;
    ds.view.cy = 100.0;
    frames(&mut u, 2);
    assert!(canvas_doc(&u.app).is_none_or(|d| opacity(&d, far) == 1.0), "scrolled into view: final, never half-faded");
}

#[test]
fn a_move_tweens_a_delete_fades_and_dropped_frames_finish_it() {
    let (mut u, home) = calm();
    let r = id(&agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home})));
    frames(&mut u, 90);
    agent(&mut u, "node.set", json!({"id": r, "props": {"x": 60}}));
    frames(&mut u, 2);
    let drawn = canvas_doc(&u.app).expect("the move tweens");
    let x = |d: &Document| d.node(r).unwrap().transform.translation().x;
    let session_x = x(&u.app.session.ds().unwrap().doc);
    assert!(x(&drawn) < session_x && x(&drawn) > 10.0, "on its way: {} → {session_x}", x(&drawn));
    frames(&mut u, 30);
    assert!(canvas_doc(&u.app).is_none(), "landed");
    // A delete: the layer fades out on the canvas; the document has it gone at once.
    agent(&mut u, "edit.delete", json!({"ids": [r]}));
    frames(&mut u, 3);
    assert!(u.app.session.doc().unwrap().node(r).is_none(), "gone from the document");
    let drawn = canvas_doc(&u.app).expect("the fade");
    assert!(opacity(&drawn, r) < 1.0, "fading");
    frames(&mut u, 20);
    assert!(canvas_doc(&u.app).is_none(), "faded");
    // Frames stop (the window hidden): a reveal in progress finishes at once.
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    u.frame(vec![]);
    assert!(canvas_doc(&u.app).is_some());
    let ctx = u.ctx.clone();
    let now = ctx.input(|i| i.time);
    super::tick(&mut u.app, &ctx, now + 1.0, &[]);
    assert!(canvas_doc(&u.app).is_none(), "a gap: finished");
}

#[test]
fn the_reveal_and_the_working_indicator_run_in_the_same_frames() {
    let (mut u, _) = calm();
    poster_batch(&mut u);
    let mut both = 0;
    for _ in 0..40 {
        u.frame(vec![]);
        if u.app.ui.agent_view.reveal.active() && u.app.ui.agent_view.indicator.drawn.get().is_some() {
            both += 1;
        }
    }
    assert!(both >= 20, "both active together for most of the reveal: {both} frames");
}

#[test]
fn under_load_the_steps_come_in_order_and_the_fade_stays() {
    let (mut u, home) = calm();
    u.app.ui.agent_view.budget.frozen = false;
    let mut levels = Vec::new();
    for i in 0..60 {
        if i % 6 == 0 {
            agent(&mut u, "shape.rectangle", json!({"x": 10 + i, "y": 10, "width": 5, "height": 5, "parent": home}));
        }
        u.app.ui.frame_ms = 100.0;
        u.frame(vec![]);
        let l = u.app.ui.agent_view.budget.level;
        if levels.last() != Some(&l) {
            levels.push(l);
        }
    }
    assert_eq!(levels, [0, 1, 2, 3], "one step at a time, in order");
    // At step 3 a batch still fades in, capped at 0.6 s.
    let ctx = u.ctx.clone();
    let now = ctx.input(|i| i.time);
    let ids: Vec<NodeId> =
        (0..20).map(|i| id(&agent(&mut u, "shape.rectangle", json!({"x": 5 * i, "y": 40, "width": 4, "height": 4, "parent": home})))).collect();
    u.app.ui.frame_ms = 100.0;
    u.frame(vec![]);
    let starts: Vec<f64> = u.app.ui.agent_view.reveal.entering().filter(|(i, _)| ids.contains(i)).map(|(_, t)| t).collect();
    assert_eq!(starts.len(), 20, "the fade is kept under load");
    assert!(starts.iter().all(|t| *t + ENTER <= now + 2.0 / 60.0 + CAP_DEGRADED + 1e-9), "capped at 0.6 s");
}
