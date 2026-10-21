//! Fix L2: why an agent's new layers enter or are drawn final at once (`docs/design/reveal-skip.md`).
//! The diagnostics count every agent edit's new layers (`ui.inspect` › `agent.reveals`) and say why
//! follow paused (`agent.pausedBy`).

use figcraft_doc::NodeId;
use serde_json::{Value, json};

use crate::follow_agent::tests::{agent, frames, id, scene};
use crate::layers_drag_tests::Ui;
use figcraft_engine::settings::diagnostics::Skip;

/// The scene at `speed`, the frame budget pinned.
fn at(speed: &str) -> Ui {
    let (mut u, _) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    u.app.session.execute("pref.revealSpeed", &json!({"speed": speed})).unwrap();
    u
}

/// One step as an agent builds it: an artboard at `x`, then 35 rectangles in it as one batch: 36
/// layers, landing before the next frame.
fn step(u: &mut Ui, x: f64) -> Vec<NodeId> {
    let f = id(&agent(u, "frame.create", json!({"x": x, "y": 0, "width": 640, "height": 480, "name": "Step"})));
    let ctx = u.ctx.clone();
    let cmds: Vec<Value> = (0..35)
        .map(|k| json!({"command": "shape.rectangle", "params": {"x": 20 + (k % 8) * 76, "y": 20 + (k / 8) * 76, "width": 60, "height": 60, "parent": f.0}}))
        .collect();
    let r = crate::control::handle_route(
        &mut u.app,
        &ctx,
        "mcp (http)",
        "engine.execute",
        &json!({"command": "command.batch", "params": {"label": "Batch", "commands": cmds}}),
    );
    assert!(r.is_ok(), "{r:?}");
    let doc = u.app.session.doc().unwrap();
    let mut ids = vec![f];
    ids.extend(doc.node(f).unwrap().children.iter().copied());
    assert_eq!(ids.len(), 36);
    ids
}

fn inspect(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)
}

#[test]
fn the_diagnostics_count_what_entered_and_why_the_rest_didnt() {
    // Following: the whole step enters, wherever it lands.
    let mut u = at("fast");
    step(&mut u, 20000.0);
    u.frame(vec![]);
    let r = inspect(&u);
    assert_eq!(r["follow"], "following");
    assert_eq!(r["reveals"]["entered"], 36, "{r}");
    assert_eq!(r["reveals"]["skipped"], json!({}), "{r}");
    assert_eq!(r["pausedBy"], Value::Null);
    // The person's edit pauses following, and says so; work off screen is then drawn final, counted.
    frames(&mut u, 120);
    u.app.run("frame.create", json!({"x": -9000, "y": 0, "width": 10, "height": 10, "name": "Mine"}));
    frames(&mut u, 2);
    let r = inspect(&u);
    assert_eq!((r["follow"].as_str(), r["pausedBy"].as_str()), (Some("paused"), Some("the person's edit")), "{r}");
    step(&mut u, 40000.0);
    u.frame(vec![]);
    let r = inspect(&u);
    let off = Skip::OffScreenPaused.text();
    assert_eq!(r["reveals"]["skipped"][off], 36, "{r}");
    // Both kept as diagnostics events (Settings ▸ Diagnostics): the pause at once; the counts once
    // the agent's edits have been quiet LOG_QUIET, the whole burst in one event.
    let reveal_events = |u: &Ui| u.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "reveal").count();
    frames(&mut u, 60);
    assert_eq!(reveal_events(&u), 0, "not mid-burst");
    step(&mut u, 41000.0);
    frames(&mut u, 150);
    let events: Vec<(String, String)> = u.app.session.prefs.settings.diagnostics.iter().map(|e| (e.kind.clone(), e.detail.clone())).collect();
    assert!(events.contains(&("follow".into(), "following paused: the person's edit".into())), "{events:?}");
    assert!(
        events.contains(&("reveal".into(), "agent layers: 36 entered, 72 drawn final at once (off screen, follow paused: 72)".into())),
        "everything since the window opened, in one event: {events:?}"
    );
    assert_eq!(reveal_events(&u), 1);
    assert_eq!(r["reveals"]["entered"], 36, "none more entered: {r}");
    let last = r["reveals"]["recent"].as_array().unwrap().last().unwrap().clone();
    assert_eq!((last["follow"].as_str(), last["entered"].as_u64(), last["skipped"][off].as_u64()), (Some("paused"), Some(0), Some(1)), "{last}");
    // Resuming clears the reason.
    u.app.run("camera.follow-agents", json!({"on": true}));
    frames(&mut u, 2);
    assert_eq!(inspect(&u)["pausedBy"], Value::Null);
    // Animations Off: every new layer is counted under that reason.
    let mut u = at("normal");
    u.app.run("pref.motion", json!({"motion": "off"}));
    step(&mut u, 300.0);
    u.frame(vec![]);
    assert_eq!(inspect(&u)["reveals"]["skipped"][Skip::NoMotion.text()], 36);
}

/// Set the camera as a person would leave it: centre and zoom, the following state untouched.
fn look_at(u: &mut Ui, cx: f64, cy: f64, zoom: f64) {
    let ds = u.app.session.ds_mut().unwrap();
    (ds.view.cx, ds.view.cy, ds.view.zoom) = (cx, cy, zoom);
    crate::follow_agent::agent_moved_camera(&mut u.app);
    frames(u, 2);
}

#[test]
fn far_camera_following_every_layer_of_a_36_layer_step_enters_at_every_speed() {
    for speed in ["fast", "normal", "slow"] {
        let mut u = at(speed);
        look_at(&mut u, -6000.0, -4000.0, 0.6);
        let ids = step(&mut u, 760.0);
        u.frame(vec![]);
        // The camera is on its way, and every layer has its entrance queued, none drawn final.
        assert!(u.app.ui.camera.flying(), "{speed}: a flight to the work");
        let queued: Vec<NodeId> = u.app.ui.agent_view.reveal.entering().map(|(i, _)| i).collect();
        assert!(ids.iter().all(|i| queued.contains(i)), "{speed}: {} of 36 queued", queued.len());
        let r = inspect(&u);
        assert_eq!((r["reveals"]["entered"].as_u64(), &r["reveals"]["skipped"]), (Some(36), &json!({})), "{speed}: {r}");
        // Through the flight and after it, nothing is cut short.
        frames(&mut u, 400);
        assert_eq!(inspect(&u)["reveals"]["skipped"], json!({}), "{speed}");
        assert!(super::canvas_doc(&u.app).is_none(), "{speed}: finished");
    }
}

#[test]
fn camera_on_the_work_a_36_layer_step_enters_whole() {
    let mut u = at("normal");
    look_at(&mut u, 760.0 + 320.0, 240.0, 0.6);
    step(&mut u, 760.0);
    u.frame(vec![]);
    assert_eq!(inspect(&u)["reveals"]["entered"], 36);
    assert_eq!(u.app.ui.agent_view.reveal.entering().count(), 36);
}

#[test]
fn paused_a_36_layer_step_off_screen_is_drawn_final_and_stays_final_when_scrolled_to() {
    let mut u = at("normal");
    step(&mut u, 0.0);
    frames(&mut u, 400);
    u.app.run("frame.create", json!({"x": -9000, "y": 0, "width": 10, "height": 10, "name": "Mine"}));
    frames(&mut u, 2);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "paused");
    let ids = step(&mut u, 30000.0);
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.reveal.entering().all(|(i, _)| !ids.contains(&i)), "off screen: no entrance");
    assert_eq!(inspect(&u)["reveals"]["skipped"][Skip::OffScreenPaused.text()], 36);
    look_at(&mut u, 30320.0, 240.0, 0.6);
    let d = super::canvas_doc(&u.app);
    assert!(d.is_none_or(|d| ids.iter().all(|i| d.node(*i).is_some_and(|n| n.opacity == 1.0))), "final, never half-faded");
}

#[test]
fn mid_flight_and_mid_reveal_a_screenshot_and_an_export_equal_those_after() {
    let mut u = at("slow");
    look_at(&mut u, -6000.0, -4000.0, 0.6);
    let ids = step(&mut u, 760.0);
    frames(&mut u, 4);
    assert!(u.app.ui.camera.flying() && super::canvas_doc(&u.app).is_some(), "mid-flight and mid-reveal");
    let ctx = u.ctx.clone();
    let take = |u: &mut Ui| -> (Value, Value) {
        let shot = crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "ui.render", &json!({"scale": 1, "ids": [ids[0].0]})).unwrap();
        let svg = crate::control::handle_route(
            &mut u.app,
            &ctx,
            "mcp (http)",
            "engine.execute",
            &json!({"command": "file.export", "params": {"format": "svg", "ids": [ids[0].0]}}),
        )
        .unwrap();
        (shot, svg)
    };
    let mid = take(&mut u);
    frames(&mut u, 600);
    assert!(super::canvas_doc(&u.app).is_none(), "after");
    assert_eq!(mid, take(&mut u), "the screenshot (a render of the document) and the export");
}
