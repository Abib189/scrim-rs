//! Fix T3: the reveal waits for following's flight, so an agent's new layers enter on screen.
//! - Following, after a far jump: the batch waits (drawn not yet entered) while the camera is more
//!   than 40 px from its target, then enters as it arrives, and counts as entered.
//! - Paused or off: a batch off screen is drawn final at once, as before.
//! - The wait ends at 700 ms even if the camera is still on its way, and that is kept in Settings ▸
//!   Diagnostics. Nothing reaches the document.

use std::sync::Arc;

use egui::Event;
use figcraft_doc::{Document, NodeId};
use serde_json::{Value, json};

use super::{HOLD_CAP, HOLD_PX, canvas_doc};
use crate::follow_agent::tests::{agent, frames, id, scene};
use crate::layers_drag_tests::Ui;

fn opacity(d: &Document, id: NodeId) -> f32 {
    d.node(id).map_or(-1.0, |n| n.opacity)
}

/// A far artboard and 12 layers in it, one batch, 3,000 units from the camera; then the frame they
/// land in. The artboard's id.
fn far_batch(u: &mut Ui) -> Vec<NodeId> {
    let board = agent(u, "frame.create", json!({"x": 3000, "y": 0, "width": 400, "height": 300, "name": "Far"}));
    let kids: Vec<Value> = (0..12)
        .map(|k| json!({"command": "shape.rectangle", "params": {"x": 10 + k * 30, "y": 10, "width": 20, "height": 20, "parent": board["id"]}}))
        .collect();
    agent(u, "command.batch", json!({"label": "Batch", "commands": kids}));
    u.frame(vec![]);
    vec![id(&board)]
}

/// How far the camera still has to go, in screen px (0 when it isn't flying).
fn left(u: &Ui) -> f64 {
    super::flight_left(&u.app).unwrap_or(0.0)
}

fn events(u: &Ui) -> Vec<String> {
    u.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "reveal").map(|e| e.detail.clone()).collect()
}

#[test]
fn following_a_far_jump_the_batch_waits_then_enters_on_screen_as_the_camera_arrives() {
    let (mut u, _) = scene();
    let ids = far_batch(&mut u);
    assert!(u.app.ui.agent_view.reveal.held() > 0, "waiting for the flight");
    let mut released_at = None;
    for f in 0..240 {
        let far = left(&u) > HOLD_PX;
        let drawn = canvas_doc(&u.app);
        if far {
            // On its way: nothing of the batch shows yet (and the document has it final).
            let d = drawn.as_ref().expect("the canvas draws the reveal");
            assert_eq!(opacity(d, ids[0]), 0.0, "frame {f}: still waiting");
            assert_eq!(opacity(&u.app.session.ds().unwrap().doc, ids[0]), 1.0);
        } else if released_at.is_none() && u.app.ui.agent_view.reveal.held() == 0 {
            released_at = Some(f);
            // Arrived: the entrance begins now, on screen.
            let d = drawn.expect("revealing as it arrives");
            assert!(opacity(&d, ids[0]) < 0.5, "frame {f}: just begun, {}", opacity(&d, ids[0]));
        }
        u.frame(vec![]);
    }
    assert!(released_at.is_some(), "released once the camera arrived");
    assert!(canvas_doc(&u.app).is_none(), "and done");
    let t = u.app.ui.agent_view.reveal.tallies();
    assert_eq!((t.entered, t.skipped.len()), (13, 0), "the artboard and its 12 layers count as entered: {:?}", t.recent);
    assert!(events(&u).is_empty(), "no cap reached: {:?}", events(&u));
}

#[test]
fn paused_or_off_a_far_batch_is_still_drawn_final_at_once() {
    for off in [false, true] {
        let (mut u, _) = scene();
        agent(&mut u, "frame.create", json!({"x": 300, "y": 0, "width": 100, "height": 100}));
        frames(&mut u, 40);
        if off {
            u.app.run("camera.follow-agents", json!({"on": false}));
        } else {
            let at = u.app.canvas.rect.center();
            u.frame(vec![Event::PointerMoved(at)]);
            u.frame(vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -60.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        frames(&mut u, 40);
        far_batch(&mut u);
        u.frame(vec![]);
        assert_eq!(u.app.ui.agent_view.reveal.held(), 0, "off {off}: nothing waits");
        assert!(canvas_doc(&u.app).is_none(), "off {off}: drawn final at once");
    }
}

#[test]
fn the_wait_ends_at_700_ms_while_the_camera_is_still_on_its_way() {
    let (mut u, _) = scene();
    let ids = far_batch(&mut u);
    // A slow flight to the same place: still on its way well past the cap.
    let to = u.app.ui.camera.target().expect("following flies");
    let v = u.app.session.ds().unwrap().view.clone();
    let from = json!({"cx": v.cx, "cy": v.cy, "zoom": v.zoom});
    let ds = u.app.session.ds_mut().unwrap();
    (ds.view.cx, ds.view.cy, ds.view.zoom) = (to[0], to[1], to[2]);
    crate::camera_flight::request(&mut u.app, &json!({"from": from, "secs": 5.0}));
    let start = u.time;
    let mut waited = 0.0;
    while u.app.ui.agent_view.reveal.held() > 0 && u.time - start < 2.0 {
        u.frame(vec![]);
        waited = u.time - start;
    }
    assert!(left(&u) > HOLD_PX, "the camera is still on its way");
    assert!((waited - HOLD_CAP).abs() <= 2.0 / 60.0, "released at the cap: {waited:.3} s");
    frames(&mut u, 6);
    let d: Arc<Document> = canvas_doc(&u.app).expect("entering");
    assert!(opacity(&d, ids[0]) > 0.0, "it enters while the camera flies");
    assert_eq!(events(&u), ["agent layers: 13 waited 700 ms for the camera, then entered while it was still on its way"]);
}
