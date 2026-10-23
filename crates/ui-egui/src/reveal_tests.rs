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

use super::{CAP, CAP_DEGRADED, ENTER, FAST, STAGGER, canvas_doc, entering, schedule};
use crate::follow_agent::tests::{agent, frames, id, poster, scene};
use crate::layers_drag_tests::Ui;

/// The scene with the frame budget pinned (debug builds' frame times aren't the app's), at Fast:
/// Stage I2's timing, which these tests pin and Fast must keep exactly (Stage I.1).
fn calm() -> (Ui, NodeId) {
    let (mut u, home) = at("fast");
    u.app.ui.agent_view.budget.frozen = true;
    (u, home)
}

/// The scene at reveal speed `speed`.
fn at(speed: &str) -> (Ui, NodeId) {
    let (mut u, home) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    u.app.session.execute("pref.revealSpeed", &json!({"speed": speed})).unwrap();
    (u, home)
}

/// `n` layers in a new frame by an agent, all landing before the next frame. Returns their ids,
/// the frame first.
fn batch(u: &mut Ui, x: f64, n: usize) -> Vec<NodeId> {
    let p = id(&agent(u, "frame.create", json!({"x": x, "y": 0, "width": 600, "height": 800, "name": "Step"})));
    let mut ids = vec![p];
    for i in 1..n {
        let r = agent(u, "shape.rectangle", json!({"x": 10 + (i % 10) * 50, "y": 10 + (i / 10) * 50, "width": 40, "height": 40, "parent": p.0}));
        ids.push(id(&r));
    }
    ids
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

/// Frames until no entrance waits for following's flight (Fix T3; 1 s at most).
fn until_released(u: &mut Ui) {
    for _ in 0..60 {
        if u.app.ui.agent_view.reveal.held() == 0 {
            return;
        }
        u.frame(vec![]);
    }
    panic!("the entrances still wait after 1 s");
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
    let s = schedule(&ids, 10.0, 10.0, FAST);
    assert_eq!(s.iter().map(|(i, _)| *i).collect::<Vec<_>>(), ids, "creation order");
    assert!(s.windows(2).all(|w| (w[1].1 - w[0].1 - STAGGER).abs() < 1e-9), "40 ms apart: {s:?}");
    // 100 layers: the stagger shrinks so the last ends within the cap.
    let many: Vec<NodeId> = (1..=100).map(NodeId).collect();
    let s = schedule(&many, 10.0, 10.0, FAST);
    let last = s.last().unwrap().1;
    assert!(last + ENTER <= 10.0 + CAP + 1e-9, "{last}");
    assert!(s.windows(2).all(|w| w[1].1 >= w[0].1), "still in order");
    let s = schedule(&many, 10.0, 10.0, FAST.degraded());
    assert!(s.last().unwrap().1 + ENTER <= 10.0 + CAP_DEGRADED + 1e-9);
    // A batch queued behind another never starts later than the cap allows.
    let s = schedule(&ids, 10.0, 50.0, FAST);
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
    // Mid-reveal (once the camera has arrived, Fix T3): the canvas draws a copy with the later
    // layers still invisible; the session's document is the final one, unchanged.
    until_released(&mut u);
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
    // After the camera's flight (up to 600 ms for a far jump, Fix T2), while the 16 entrances
    // still run.
    frames(&mut u, 40);
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
    // The person's switch (an agent is refused it: Fix W3).
    let r = u.app.run("pref.revealAgentWork", json!({"on": false})).unwrap();
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
    // (Right after a landing, a gap up to 5 s is the landing's own frame; longer, the window was hidden.)
    super::tick(&mut u.app, &ctx, now + 6.0, 6.0, &[]);
    assert!(canvas_doc(&u.app).is_none(), "the window was hidden: finished");
}

/// On a big file the agent's edit itself can take a long frame: the reveal waits for it rather than
/// being dropped, however long (up to 5 s). Later in the reveal, one stall re-times it; a second
/// (frames being dropped) finishes it. What lands after the window drew no frames is drawn final.
#[test]
fn a_long_landing_frame_delays_the_reveal_and_dropped_frames_finish_it() {
    let (mut u, home) = calm();
    // At the canvas's centre: following doesn't fly, so nothing waits for it (Fix T3).
    let r = id(&agent(&mut u, "shape.rectangle", json!({"x": 90, "y": 90, "width": 20, "height": 20, "parent": home})));
    u.frame(vec![]);
    assert_eq!(u.app.ui.agent_view.reveal.held(), 0);
    let ctx = u.ctx.clone();
    let now = ctx.input(|i| i.time);
    let start = u.app.ui.agent_view.reveal.entering().find(|(i, _)| *i == r).unwrap().1;
    // The landing's own frame took 1.2 s (the batch on a big file).
    super::tick(&mut u.app, &ctx, now + 1.2, 1.2, &[]);
    let later = u.app.ui.agent_view.reveal.entering().find(|(i, _)| *i == r).expect("still revealing after the landing's long frame").1;
    assert!((later - start - 1.2).abs() < 1e-9, "re-timed by the gap: {start} → {later}");
    assert!(canvas_doc(&u.app).is_some_and(|d| opacity(&d, r) < 1.0), "and it plays from where it was");
    // One stall later on: re-timed; a second: finished.
    super::tick(&mut u.app, &ctx, now + 1.5, 0.3, &[]);
    assert!(canvas_doc(&u.app).is_some(), "one stall: still playing");
    super::tick(&mut u.app, &ctx, now + 1.8, 0.3, &[]);
    assert!(canvas_doc(&u.app).is_none(), "a second stall: frames are being dropped, finished");
    // Edits answered while the window was hidden (`control::drain_hidden`): final at once.
    let late = super::super::Landed { agent: true, route: "mcp (http)".into(), created: vec![r], ..Default::default() };
    u.app.ui.agent_view.landed_hidden = true;
    super::tick(&mut u.app, &ctx, now + 10.0, 8.0, &[late]);
    assert!(canvas_doc(&u.app).is_none(), "landed while hidden: no entrance");
}

/// An idle window draws no frames either: the agent's first edit after a quiet minute still enters
/// (found by I4: the gap alone read an idle window as a hidden one).
#[test]
fn an_edit_after_a_quiet_minute_still_enters() {
    let (mut u, home) = calm();
    frames(&mut u, 2);
    let now = u.ctx.input(|i| i.time);
    u.app.ui.agent_view.last_frame = Some(now - 60.0);
    let r = id(&agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home})));
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.reveal.entering().any(|(i, _)| i == r), "it enters");
}

#[test]
fn the_reveal_and_the_working_indicator_run_in_the_same_frames() {
    let (mut u, _) = calm();
    poster_batch(&mut u);
    let (mut both, mut revealing) = (0, 0);
    for _ in 0..90 {
        u.frame(vec![]);
        // Entering on screen: not waiting for the camera (Fix T3).
        let entering = u.app.ui.agent_view.reveal.active() && u.app.ui.agent_view.reveal.held() == 0;
        revealing += usize::from(entering);
        if entering && u.app.ui.agent_view.indicator.drawn.get().is_some() {
            both += 1;
        }
    }
    // The entrances wait for following's flight (Fix T3), so they play under the dots.
    assert!(revealing > 0 && both * 10 >= revealing * 9, "both active together for nearly all of the reveal: {both} of {revealing} frames");
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

/// One slow render isn't a slow frame rate: the render time stays the same until the next render,
/// and only a new one counts (found by I4: one 220 ms render stepped all the way down).
#[test]
fn one_slow_render_doesnt_degrade() {
    let (mut u, home) = calm();
    u.app.ui.agent_view.budget.frozen = false;
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    u.app.ui.render_ms = 220.0;
    for _ in 0..40 {
        u.app.ui.frame_ms = 1.0;
        u.frame(vec![]);
        // (The harness renders nothing new: the 220 ms stays.)
        u.app.ui.render_ms = 220.0;
    }
    assert_eq!(u.app.ui.agent_view.budget.level, 0, "one render, counted once");
}

// ---------- Stage I.1: the reveal's speed ----------

#[test]
fn each_speed_has_its_timing_and_fast_is_the_old_one_exactly() {
    use super::{NORMAL, SLOW, Speed};
    assert_eq!(FAST, Speed { enter: 0.22, stagger: 0.04, cap: 1.2 }, "Stage I2's values");
    assert_eq!((ENTER, STAGGER, CAP, CAP_DEGRADED), (0.22, 0.04, 1.2, 0.6));
    assert_eq!(FAST.degraded().cap, CAP_DEGRADED);
    assert_eq!(NORMAL, Speed { enter: 0.48, stagger: 0.11, cap: 4.0 });
    assert_eq!(SLOW, Speed { enter: 0.70, stagger: 0.18, cap: 6.0 });
    assert_eq!(Speed::of("slow"), SLOW);
    assert_eq!(Speed::of("normal"), NORMAL);
    assert_eq!(Speed::of("fast"), FAST);
    assert_eq!(Speed::of("nonsense"), NORMAL, "the default");
    assert_eq!(figcraft_engine::settings::Settings::default().reveal_speed, "normal");
    // Degrading drops the cap first, never the entrance.
    for s in [SLOW, NORMAL, FAST] {
        assert_eq!((s.degraded().enter, s.degraded().cap), (s.enter, s.cap / 2.0));
    }
}

#[test]
fn every_speed_keeps_a_batch_within_its_cap() {
    use super::{NORMAL, SLOW};
    for speed in [SLOW, NORMAL, FAST, SLOW.degraded(), NORMAL.degraded(), FAST.degraded()] {
        for n in [1usize, 6, 36, 60] {
            let ids: Vec<NodeId> = (1..=n as u64).map(NodeId).collect();
            let s = schedule(&ids, 10.0, 10.0, speed);
            let end = s.last().unwrap().1 + speed.enter;
            assert!(end <= 10.0 + speed.cap + 1e-9, "{speed:?} × {n}: ends at {end}");
            let step = speed.stagger.min(if n > 1 { (speed.cap - speed.enter) / (n - 1) as f64 } else { 0.0 });
            assert!(s.windows(2).all(|w| (w[1].1 - w[0].1 - step).abs() < 1e-9), "{speed:?} × {n}: the stagger shrinks only to fit");
        }
    }
}

#[test]
fn a_36_layer_batch_at_normal_takes_over_2_s_and_at_most_4_s() {
    let (mut u, _) = at("normal");
    let ids = batch(&mut u, 300.0, 36);
    u.frame(vec![]);
    let q: Vec<(NodeId, f64)> = u.app.ui.agent_view.reveal.entering().collect();
    assert_eq!(q.len(), ids.len());
    let total = q.last().unwrap().1 + super::NORMAL.enter - q[0].1;
    assert!(total > 2.0 && total <= 4.0 + 1e-9, "{total} s");
    // Halfway, it is still revealing; after 4 s, done (counted once the camera has arrived).
    until_released(&mut u);
    frames(&mut u, 120);
    assert!(canvas_doc(&u.app).is_some(), "still revealing at 2 s");
    frames(&mut u, 130);
    assert!(canvas_doc(&u.app).is_none(), "done by 4.2 s");
}

#[test]
fn a_step_landing_mid_reveal_queues_behind_it_instead_of_restarting_it() {
    for speed in ["slow", "normal", "fast"] {
        let (mut u, _) = at(speed);
        let sp = super::Speed::of(speed);
        let first = batch(&mut u, 300.0, 6);
        u.frame(vec![]);
        let before: Vec<(NodeId, f64)> = u.app.ui.agent_view.reveal.entering().collect();
        frames(&mut u, 6);
        let second = batch(&mut u, 1000.0, 6);
        u.frame(vec![]);
        let all: Vec<(NodeId, f64)> = u.app.ui.agent_view.reveal.entering().collect();
        // The first step's entrances keep their times (none restarted or cut) …
        for (id, t) in before.iter().filter(|(id, _)| first.contains(id)) {
            if let Some((_, now_t)) = all.iter().find(|(i, _)| i == id) {
                assert_eq!(t, now_t, "{speed}: {id:?} kept its start");
            }
        }
        // … and the second starts one stagger after the first's last.
        let last_first = before.last().unwrap().1;
        let start_second = all.iter().find(|(i, _)| *i == second[0]).unwrap().1;
        assert!((start_second - (last_first + sp.stagger)).abs() < 1e-9, "{speed}: {start_second} after {last_first}");
    }
}

#[test]
fn at_normal_and_slow_the_reveal_is_still_render_only() {
    for speed in ["normal", "slow"] {
        let (mut u, _) = at(speed);
        let ids = batch(&mut u, 300.0, 12);
        frames(&mut u, 30);
        assert!(canvas_doc(&u.app).is_some(), "{speed}: mid-reveal");
        let ctx = u.ctx.clone();
        let take = |u: &mut Ui| -> (Value, Value) {
            let shot = crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "ui.render", &json!({"scale": 1, "ids": [ids[0].0]})).unwrap();
            let inspect = crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "document.inspect", &json!({})).unwrap();
            (shot, inspect)
        };
        let mid = take(&mut u);
        frames(&mut u, 60 * 7);
        assert!(canvas_doc(&u.app).is_none(), "{speed}: after");
        assert_eq!(mid, take(&mut u), "{speed}: the render and inspect_document, mid-reveal and after");
    }
}
