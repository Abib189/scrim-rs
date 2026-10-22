//! R3b: a camera move an agent's command causes is the agent's own, and never pauses following.
//! - Opening a file: its fit on the canvas's first sizing, and a focus asked for before it; and a
//!   saved file by its path, as the `open_file` tool does (`file.open`, then `document.inspect`).
//! - Opening a file, or switching page, while follow's own flight is under way.
//! - A focus that doesn't need to move the camera (it still ends follow's flight).
//! - The first batch after any of these is followed and revealed.
//! - The person's own camera move afterwards still pauses.

use base64::Engine;
use serde_json::{Value, json};

use crate::control::{ControlRequest, ROUTE_MCP_HTTP};
use crate::follow_agent::tests::{frames, scene, view};
use crate::layers_drag_tests::Ui;

/// Queue a request from Claude Code's session as the app's MCP endpoint does; its answer arrives
/// on the returned channel once a frame has run.
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

/// [`queue`], answered in the next frame.
fn agent(u: &mut Ui, command: &str, params: Value) -> Value {
    let answer = queue(u, command, params);
    u.frame(vec![]);
    answer.try_recv().ok().and_then(|r| r.result.ok()).unwrap_or(Value::Null)
}

/// A small SVG file, opened as a new document (unsized: the canvas fits it on its first sizing).
fn an_svg_file() -> Value {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200"><rect x="10" y="10" width="100" height="80" fill="red"/></svg>"#;
    json!({"data": base64::engine::general_purpose::STANDARD.encode(svg), "name": "art.svg"})
}

fn state(u: &Ui) -> &'static str {
    figcraft_engine::cmd::camera::follow_state(&u.app.session)
}

fn pauses(u: &Ui) -> Vec<String> {
    u.app
        .session
        .prefs
        .settings
        .diagnostics
        .iter()
        .filter(|e| e.kind == "follow" && e.detail.starts_with("following paused"))
        .map(|e| e.detail.clone())
        .collect()
}

/// Follow's own flight to a poster far right, under way.
fn following_mid_flight() -> Ui {
    let (mut u, _) = scene();
    agent(&mut u, "frame.create", json!({"x": 8000, "y": 0, "width": 400, "height": 300, "name": "Poster"}));
    frames(&mut u, 3);
    assert!(u.app.ui.camera.flying(), "follow's flight is under way");
    u
}

/// The agent's first batch far away: followed (the camera goes there) and revealed.
fn first_batch_is_followed_and_revealed(u: &mut Ui, what: &str) {
    agent(u, "frame.create", json!({"x": 5000, "y": 0, "width": 300, "height": 300, "name": "Far"}));
    let mut revealed = false;
    for _ in 0..40 {
        u.frame(vec![]);
        revealed |= u.app.ui.agent_view.reveal.entering().count() > 0;
    }
    assert_eq!(state(u), "following", "{what}: {:?}", pauses(u));
    assert!(view(u).0 > 4900.0, "{what}: the camera went to the batch: {:?}", view(u));
    assert!(revealed, "{what}: the batch had its entrance");
    assert!(pauses(u).is_empty(), "{what}: {:?}", pauses(u));
}

#[test]
fn an_agent_opening_a_file_and_focusing_in_it_before_it_is_sized_doesnt_pause_following() {
    let (mut u, _) = scene();
    agent(&mut u, "frame.create", json!({"x": 300, "y": 0, "width": 100, "height": 100, "name": "Seen"}));
    frames(&mut u, 30);
    // Open, select and focus in one frame: the focus waits for the canvas's first sizing.
    let _open = queue(&mut u, "file.open", an_svg_file());
    let _all = queue(&mut u, "select.all", json!({}));
    let _focus = queue(&mut u, "camera.focus-layer", json!({}));
    frames(&mut u, 30);
    assert_eq!(state(&u), "following", "{:?}", pauses(&u));
    first_batch_is_followed_and_revealed(&mut u, "after opening a file");
}

#[test]
fn an_agent_opening_a_file_mid_flight_doesnt_pause_following() {
    let mut u = following_mid_flight();
    let r = agent(&mut u, "file.open", an_svg_file());
    assert_eq!(r["summary"]["layers"], json!(2), "{r}");
    frames(&mut u, 10);
    assert_eq!(state(&u), "following", "{:?}", pauses(&u));
    first_batch_is_followed_and_revealed(&mut u, "after opening a file mid-flight");
}

#[test]
fn an_agent_switching_page_mid_flight_doesnt_pause_following() {
    let (mut u, _) = scene();
    agent(&mut u, "page.add", json!({"name": "Two"}));
    agent(&mut u, "page.select", json!({"index": 0}));
    agent(&mut u, "frame.create", json!({"x": 8000, "y": 0, "width": 400, "height": 300, "name": "Poster"}));
    frames(&mut u, 3);
    assert!(u.app.ui.camera.flying(), "follow's flight is under way");
    let two = u.app.session.doc().unwrap().pages[1];
    agent(&mut u, "page.select", json!({"id": two.0}));
    frames(&mut u, 10);
    assert_eq!(u.app.session.ds().unwrap().page, two);
    assert_eq!(state(&u), "following", "{:?}", pauses(&u));
    first_batch_is_followed_and_revealed(&mut u, "after switching page");
}

#[test]
fn an_agents_focus_that_doesnt_move_the_camera_still_counts_as_its_own() {
    let mut u = following_mid_flight();
    // Near the flight's start the home frame is still in view: the focus moves nothing, but it
    // ends follow's flight.
    let doc = u.app.session.doc().unwrap();
    let home = doc.node(doc.pages[0]).map(|p| p.children[0]).unwrap();
    let r = agent(&mut u, "camera.focus-layer", json!({"id": home.0}));
    assert_eq!(r["moved"], json!(false), "{r}");
    frames(&mut u, 10);
    assert_eq!(state(&u), "following", "{:?}", pauses(&u));
    first_batch_is_followed_and_revealed(&mut u, "after a focus that moved nothing");
}

#[test]
fn the_persons_camera_move_after_an_agents_still_pauses() {
    let mut u = following_mid_flight();
    agent(&mut u, "page.add", json!({"name": "Two"}));
    let two = u.app.session.doc().unwrap().pages[1];
    agent(&mut u, "page.select", json!({"id": two.0}));
    frames(&mut u, 10);
    assert_eq!(state(&u), "following");
    // The agent is still at work (its grace hasn't run out): the person zooms in.
    u.app.run("view.zoomIn", json!({}));
    frames(&mut u, 3);
    assert_eq!(state(&u), "paused", "the person's own move pauses");
    assert_eq!(pauses(&u), ["following paused: a camera move that wasn't follow's or an agent's"]);
}

/// A request other than a command, as the endpoint queues it, answered in the next frame.
fn agent_method(u: &mut Ui, method: &str, params: Value) -> Value {
    let (reply, answer) = std::sync::mpsc::channel();
    u.app.control_queue.push(ControlRequest {
        route: ROUTE_MCP_HTTP,
        client: Some("claude-code".into()),
        session: Some("s1".into()),
        id: json!(1),
        method: method.into(),
        params,
        reply,
        cancelled: Default::default(),
    });
    u.frame(vec![]);
    answer.try_recv().ok().and_then(|r| r.result.ok()).unwrap_or(Value::Null)
}

#[test]
fn the_open_file_tool_mid_flight_doesnt_pause_following() {
    let dir = std::env::temp_dir().join(format!("scrim-r3b-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("saved.figcraft");
    let mut s = figcraft_engine::Session::new();
    s.execute("frame.create", &json!({"x": 2000, "y": 2000, "width": 300, "height": 300, "name": "Saved"})).unwrap();
    s.execute("file.saveAs", &json!({"path": path.to_string_lossy()})).unwrap();
    let mut u = following_mid_flight();
    let r = agent(&mut u, "file.open", json!({"path": path.to_string_lossy()}));
    assert_eq!(r["summary"]["layers"], json!(1), "{r}");
    let doc = agent_method(&mut u, "document.inspect", json!({"depth": 1}));
    assert!(!doc.is_null());
    frames(&mut u, 10);
    assert_eq!(state(&u), "following", "{:?}", pauses(&u));
    first_batch_is_followed_and_revealed(&mut u, "after the open_file tool");
    let _ = std::fs::remove_dir_all(&dir);
}
