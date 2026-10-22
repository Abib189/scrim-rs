//! Fix T2: following lands the agent's latest work in the middle of the canvas.
//! - A far batch is centred, with a longer flight (capped at 600 ms); a batch at the canvas's edge
//!   is centred too, at any zoom, and the zoom is never raised.
//! - A large batch frames its artboard, centred, with a 10% margin; work across two artboards is
//!   centred on the latest edit's.
//! - Within 16 px of the target nothing moves; reduced motion jumps; Resume centres the same way,
//!   and the person's scroll still pauses. A Layers click keeps D2's least move.

use egui::{Event, Pos2};
use figcraft_doc::NodeId;
use serde_json::{Value, json};

use super::{FAR, MAX_FLIGHT_SECS, flight_secs};
use crate::camera_flight::FLIGHT_SECS;
use crate::control::{ControlRequest, ROUTE_MCP_HTTP};
use crate::follow_agent::tests::{frames, scene};
use crate::layers_drag_tests::Ui;

/// An agent's command, as the app's MCP endpoint queues it (answered in the next frame).
fn queue(u: &mut Ui, command: &str, params: Value) -> std::sync::mpsc::Receiver<crate::control::ControlReply> {
    let (reply, answer) = std::sync::mpsc::channel();
    u.app.control_queue.push(ControlRequest {
        route: ROUTE_MCP_HTTP,
        client: Some("claude-code".into()),
        session: Some("s1".into()),
        id: json!(1),
        method: "engine.execute".into(),
        params: json!({"command": command, "params": params}),
        reply,
        cancelled: Default::default(),
    });
    answer
}

fn agent(u: &mut Ui, command: &str, params: Value) -> Value {
    let answer = queue(u, command, params);
    u.frame(vec![]);
    answer.try_recv().ok().and_then(|r| r.result.ok()).unwrap_or(Value::Null)
}

fn id(v: &Value) -> NodeId {
    NodeId(v["id"].as_u64().unwrap_or_default())
}

/// The view: centre, zoom, and the canvas's size.
fn view(u: &Ui) -> (f64, f64, f64, f64, f64) {
    let v = &u.app.session.ds().unwrap().view;
    (v.cx, v.cy, v.zoom, v.vw, v.vh)
}

/// How far (screen px) the centre of `ids` is from the canvas's centre.
fn off_centre(u: &Ui, ids: &[NodeId]) -> f64 {
    let r = u.app.session.doc().unwrap().bounds_of(ids).unwrap();
    let (cx, cy, z, _, _) = view(u);
    ((r.center().x - cx).powi(2) + (r.center().y - cy).powi(2)).sqrt() * z
}

/// The scene with the camera at `zoom` on the Home frame.
fn at_zoom(zoom: f64) -> Ui {
    let (mut u, _) = scene();
    u.app.session.ds_mut().unwrap().view.zoom = zoom;
    // The test's own move, not the person's: following expects it.
    u.app.ui.agent_view.follow.expected = crate::follow_agent::view_now(&u.app);
    frames(&mut u, 2);
    u
}

/// Frames until no flight is under way (at most 2 s); the longest flight seen, in seconds.
fn settle(u: &mut Ui) -> f64 {
    let mut longest: f64 = 0.0;
    for _ in 0..120 {
        u.frame(vec![]);
        longest = longest.max(u.app.ui.camera.secs().unwrap_or(0.0));
    }
    longest
}

fn state(u: &Ui) -> &'static str {
    figcraft_engine::cmd::camera::follow_state(&u.app.session)
}

#[test]
fn the_flight_lasts_longer_for_a_longer_move_up_to_600_ms() {
    let v = figcraft_engine::ViewState { cx: 0.0, cy: 0.0, zoom: 1.0, vw: 1000.0, vh: 800.0, ..Default::default() };
    assert_eq!(flight_secs(&v, (0.0, 0.0, 1.0)), FLIGHT_SECS, "no move: 320 ms");
    let one = flight_secs(&v, (1000.0, 0.0, 1.0));
    assert!((one - (FLIGHT_SECS + (MAX_FLIGHT_SECS - FLIGHT_SECS) / FAR)).abs() < 1e-9, "a canvas width: {one}");
    assert_eq!(flight_secs(&v, (3000.0, 0.0, 1.0)), MAX_FLIGHT_SECS, "3,000 units: the cap");
    assert_eq!(flight_secs(&v, (90_000.0, 0.0, 1.0)), MAX_FLIGHT_SECS, "never longer");
}

#[test]
fn a_far_batch_lands_centred_with_a_flight_long_enough_to_read() {
    let mut u = at_zoom(1.0);
    let f = agent(&mut u, "frame.create", json!({"x": 3000, "y": 400, "width": 200, "height": 100, "name": "Far"}));
    let longest = settle(&mut u);
    assert!(off_centre(&u, &[id(&f)]) <= 2.0, "centred: {} px off", off_centre(&u, &[id(&f)]));
    assert_eq!(view(&u).2, 1.0, "the zoom kept");
    assert!((longest - MAX_FLIGHT_SECS).abs() < 1e-9, "a 3,000-unit jump flies 600 ms: {longest}");
    assert_eq!(state(&u), "following");
}

#[test]
fn a_batch_at_the_canvas_edge_is_centred_and_the_zoom_is_never_raised() {
    for zoom in [1.0, 0.5] {
        let mut u = at_zoom(zoom);
        let (cx, cy, z, vw, _) = view(&u);
        // 40 px of it in view at the right edge.
        let x = cx + vw / 2.0 / z - 40.0 / z;
        let r = agent(&mut u, "shape.rectangle", json!({"x": x, "y": cy, "width": 80.0 / z, "height": 40.0 / z}));
        settle(&mut u);
        assert!(off_centre(&u, &[id(&r)]) <= 2.0, "zoom {zoom}: {} px off", off_centre(&u, &[id(&r)]));
        assert_eq!(view(&u).2, zoom, "never raised");
    }
}

#[test]
fn a_large_batch_frames_its_artboard_centred_with_a_10_percent_margin() {
    let mut u = at_zoom(1.0);
    let (_, _, _, vw, vh) = view(&u);
    // The artboard fits at this zoom with D2's 32 px margin, but not with 10% on every side.
    let (w, h) = (vw * 0.9, vh * 0.5);
    let board = agent(&mut u, "frame.create", json!({"x": 6000, "y": 0, "width": w, "height": h, "name": "Board"}));
    let kids: Vec<Value> =
        (0..64).map(|k| json!({"command": "shape.rectangle", "params": {"x": 10 + (k % 16) * 20, "y": 10 + (k / 16) * 20, "width": 10, "height": 10, "parent": board["id"]}})).collect();
    agent(&mut u, "command.batch", json!({"label": "Batch", "commands": kids}));
    settle(&mut u);
    assert!(off_centre(&u, &[id(&board)]) <= 2.0, "centred on the artboard");
    let fit = (vw * 0.8 / w).min(vh * 0.8 / h);
    assert!((view(&u).2 - fit).abs() < 1e-6, "zoomed out just enough for a 10% margin: {} vs {fit}", view(&u).2);
}

#[test]
fn work_across_two_artboards_is_centred_on_the_latest_edits_artboard() {
    let mut u = at_zoom(0.25);
    let a = agent(&mut u, "frame.create", json!({"x": 2000, "y": 0, "width": 400, "height": 300, "name": "A"}));
    let b = agent(&mut u, "frame.create", json!({"x": 3200, "y": 0, "width": 400, "height": 300, "name": "B"}));
    settle(&mut u);
    // One burst: a layer in A, then one in B (the latest).
    let _ = queue(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": a["id"]}));
    let _ = queue(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": b["id"]}));
    // Somewhere else first, so the burst's flight is visible.
    u.app.session.ds_mut().unwrap().view.cx = 0.0;
    u.app.ui.agent_view.follow.expected = crate::follow_agent::view_now(&u.app);
    settle(&mut u);
    assert!(off_centre(&u, &[id(&b)]) <= 2.0, "on B, the latest: {} px off", off_centre(&u, &[id(&b)]));
}

#[test]
fn within_16_px_nothing_moves_and_beyond_it_the_work_is_centred() {
    let mut u = at_zoom(1.0);
    let (cx, cy, _, _, _) = view(&u);
    // Centred 10 px off: the camera stays.
    agent(&mut u, "shape.rectangle", json!({"x": cx + 10.0 - 20.0, "y": cy - 20.0, "width": 40, "height": 40}));
    settle(&mut u);
    assert_eq!((view(&u).0, view(&u).1), (cx, cy), "no shift inside the dead zone");
    // 40 px off, in full view: centred all the same.
    let r = agent(&mut u, "shape.rectangle", json!({"x": cx + 40.0 - 20.0, "y": cy - 20.0, "width": 40, "height": 40}));
    settle(&mut u);
    assert!(off_centre(&u, &[id(&r)]) <= 2.0, "{} px off", off_centre(&u, &[id(&r)]));
}

#[test]
fn with_reduced_motion_it_jumps_to_the_centre() {
    let mut u = at_zoom(1.0);
    u.app.session.prefs.settings.motion = "off".into();
    let (cx, cy, z, vw, _) = view(&u);
    let r = agent(&mut u, "shape.rectangle", json!({"x": cx + vw / 2.0 / z - 30.0, "y": cy, "width": 60, "height": 30}));
    u.frame(vec![]);
    assert!(!u.app.ui.camera.flying(), "no flight");
    assert!(off_centre(&u, &[id(&r)]) <= 2.0, "there at once: {} px off", off_centre(&u, &[id(&r)]));
}

#[test]
fn the_persons_scroll_pauses_and_resume_centres_the_latest_work() {
    let mut u = at_zoom(1.0);
    agent(&mut u, "frame.create", json!({"x": 1500, "y": 0, "width": 200, "height": 200, "name": "First"}));
    settle(&mut u);
    // The person scrolls the canvas: paused.
    let at = u.app.canvas.rect.center();
    u.frame(vec![Event::PointerMoved(at)]);
    u.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    }]);
    frames(&mut u, 30);
    assert_eq!(state(&u), "paused");
    // More work at the canvas's edge while paused: the camera stays.
    let (cx, cy, z, vw, _) = view(&u);
    let r = agent(&mut u, "shape.rectangle", json!({"x": cx + vw / 2.0 / z - 30.0, "y": cy, "width": 60, "height": 30}));
    settle(&mut u);
    assert_eq!((view(&u).0, view(&u).1), (cx, cy), "paused: no move");
    // Resume: to the latest work, centred.
    u.app.run("camera.follow-agents", json!({"on": true}));
    settle(&mut u);
    assert_eq!(state(&u), "following");
    assert!(off_centre(&u, &[id(&r)]) <= 2.0, "resumed, centred: {} px off", off_centre(&u, &[id(&r)]));
}

#[test]
fn a_layers_click_keeps_d2s_least_move_while_following_centres() {
    let mut u = at_zoom(1.0);
    let (cx, cy, z, vw, _) = view(&u);
    let r = agent(&mut u, "shape.rectangle", json!({"x": cx + vw / 2.0 / z - 30.0, "y": cy, "width": 60, "height": 30}));
    settle(&mut u);
    assert!(off_centre(&u, &[id(&r)]) <= 2.0, "following centred it");
    // The person clicks a layer at the other edge in Layers: the least move, not the centre.
    let (cx, cy, z, vw, _) = view(&u);
    let s = u.app.session.execute("shape.rectangle", &json!({"x": cx - vw / 2.0 / z - 30.0, "y": cy, "width": 60, "height": 30})).unwrap();
    let before = u.app.session.ds().unwrap().view.clone();
    let bounds = u.app.session.doc().unwrap().bounds_of(&[id(&s)]).unwrap();
    let (ex, ey) = figcraft_engine::cmd::camera::reveal(&before, bounds);
    u.app.run("camera.focus-layer", json!({"id": id(&s).0}));
    settle(&mut u);
    assert_eq!((view(&u).0, view(&u).1, view(&u).2), (ex, ey, before.zoom), "D2's least move");
    assert!(off_centre(&u, &[id(&s)]) > 2.0, "not centred");
    let _ = Pos2::ZERO;
}
