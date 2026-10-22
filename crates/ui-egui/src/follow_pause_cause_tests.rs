//! Fix S4: who moved the camera when following paused.
//! - **The report's session, in the window:** an agent opens a saved file (the `open_file` tool:
//!   `file.open` by its path, then `document.inspect`), then builds about 3,000 units away in six
//!   batches. With the file in front, with seconds between calls (no frames while idle), from Home
//!   with its tab opened later, and answered while the window is hidden after the open or
//!   throughout: following never pauses, and every camera change after the open is the agent's,
//!   the canvas's sizing or following's own.
//! - **Each change is noted where it's made:** an agent's request (with its command), following's
//!   move, the person's shortcut (the window outside the canvas), their input on the canvas, and a
//!   flight they started.
//! - **A pause for a camera move names the move behind it** (Settings ▸ Diagnostics, `ui.inspect`).

use egui::{Event, Pos2};
use figcraft_engine::settings::diagnostics::Mover;
use serde_json::{Value, json};

use crate::control::{ControlRequest, ROUTE_MCP_HTTP};
use crate::follow_agent::tests::{frames, scene};
use crate::layers_drag_tests::Ui;

/// A request from Claude Desktop's session, as the app's MCP endpoint queues it.
fn request(u: &mut Ui, method: &str, params: Value) -> std::sync::mpsc::Receiver<crate::control::ControlReply> {
    let (reply, answer) = std::sync::mpsc::channel();
    u.app.control_queue.push(ControlRequest {
        route: ROUTE_MCP_HTTP,
        client: Some("local-agent-mode-scrim (via mcp-remote 0.14.3)".into()),
        session: Some("s1".into()),
        id: json!(1),
        method: method.into(),
        params,
        reply,
        cancelled: Default::default(),
    });
    answer
}

/// A command, answered in the next frame (`hidden`: by the app's `logic` while the window is
/// minimised or covered, without a frame, as the desktop app does). `document.inspect` is a method.
fn agent(u: &mut Ui, command: &str, params: Value, hidden: bool) -> Value {
    let answer = if command == "document.inspect" {
        request(u, command, params)
    } else {
        request(u, "engine.execute", json!({"command": command, "params": params}))
    };
    if hidden {
        u.time += 1.0 / 60.0;
        let mut raw = egui::RawInput { time: Some(u.time), ..Default::default() };
        raw.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { minimized: Some(true), ..Default::default() });
        let app = &mut u.app;
        let _ = u.ctx.run_logic(&raw, |ctx| {
            crate::control::drain_hidden(app, ctx);
            crate::follow_agent::working_log::logic(app, ctx);
        });
    } else {
        u.frame(vec![]);
    }
    answer.try_recv().ok().and_then(|r| r.result.ok()).unwrap_or(Value::Null)
}

fn pauses(u: &Ui) -> Vec<String> {
    let d = &u.app.session.prefs.settings.diagnostics;
    d.iter().filter(|e| e.kind == "follow" && e.detail.starts_with("following paused")).map(|e| e.detail.clone()).collect()
}

/// The camera log's changes: who, and what.
fn log(u: &Ui) -> Vec<(String, String)> {
    let l = crate::follow_agent::camera_log::inspect(&u.app);
    l.as_array().into_iter().flatten().map(|c| (c["by"].as_str().unwrap_or("").to_string(), c["what"].as_str().unwrap_or("").to_string())).collect()
}

/// A saved file with a poster at the origin, in a folder of its own.
fn saved_file(tag: usize) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("scrim-s4-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("poster.figcraft");
    let mut s = figcraft_engine::Session::new();
    s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 800, "height": 600, "name": "Poster"})).unwrap();
    s.execute("file.saveAs", &json!({"path": path.to_string_lossy()})).unwrap();
    (dir, path.to_string_lossy().into_owned())
}

/// How the session runs.
#[derive(Clone, Copy, Debug)]
enum Way {
    /// The file in front, frames throughout.
    Front,
    /// The file in front; between calls the window draws nothing for 4 s (the agent thinking).
    Idle,
    /// Home in front; the person opens the agent's tab at the third batch.
    HomeThenTab,
    /// The open answered with the window shown, the batches while it is hidden; then shown.
    HiddenAfterOpen,
    /// Everything answered while the window is hidden; then shown.
    HiddenThroughout,
}

fn run(way: Way, tag: usize) -> Ui {
    let (dir, path) = saved_file(tag);
    let (mut u, _) = scene();
    agent(&mut u, "frame.create", json!({"x": 300, "y": 0, "width": 100, "height": 100, "name": "Seen"}), false);
    frames(&mut u, 30);
    if matches!(way, Way::HomeThenTab) {
        u.app.ui.home = true;
        frames(&mut u, 3);
    }
    let hidden_open = matches!(way, Way::HiddenThroughout);
    let hidden_batches = matches!(way, Way::HiddenAfterOpen | Way::HiddenThroughout);
    agent(&mut u, "file.open", json!({"path": path}), hidden_open);
    agent(&mut u, "document.inspect", json!({"depth": 1}), hidden_open);
    if !hidden_open {
        frames(&mut u, 30);
    }
    for i in 0..6 {
        if matches!(way, Way::HomeThenTab) && i == 2 {
            let n = u.app.session.docs.len();
            u.app.run("doc.switch", json!({"index": n - 1}));
            u.app.ui.home = false;
        }
        let x = 3000.0 + 700.0 * f64::from(i);
        let f = agent(&mut u, "frame.create", json!({"x": x, "y": 0, "width": 640, "height": 480, "name": format!("Batch {i}")}), hidden_batches);
        let kids: Vec<Value> = (0..8)
            .map(|k| json!({"command": "shape.rectangle", "params": {"x": 20.0 + f64::from(k) * 76.0, "y": 20, "width": 60, "height": 60, "parent": f["id"]}}))
            .collect();
        agent(&mut u, "command.batch", json!({"label": "Batch", "commands": kids}), hidden_batches);
        match way {
            Way::Front | Way::HomeThenTab => frames(&mut u, 90),
            Way::Idle => {
                frames(&mut u, 40);
                u.time += 4.0;
            }
            Way::HiddenAfterOpen | Way::HiddenThroughout => u.time += 4.0,
        }
    }
    frames(&mut u, 1200);
    let _ = std::fs::remove_dir_all(&dir);
    u
}

#[test]
fn the_reports_session_never_pauses_following_and_every_move_after_the_open_is_accounted_for() {
    let ways = [Way::Front, Way::Idle, Way::HomeThenTab, Way::HiddenAfterOpen, Way::HiddenThroughout];
    for (tag, way) in ways.into_iter().enumerate() {
        let u = run(way, tag);
        assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following", "{way:?}: {:?}", pauses(&u));
        assert!(pauses(&u).is_empty(), "{way:?}: {:?}", pauses(&u));
        let log = log(&u);
        let open = log.iter().position(|(by, what)| by == Mover::Agent.text() && what == "file.open");
        let after = &log[open.unwrap_or_else(|| panic!("{way:?}: the open is logged: {log:?}"))..];
        // Only the agent, the canvas's sizing and following moved the camera (from Home, the
        // person's tab switch is theirs too).
        let expected = |by: &str| [Mover::Agent, Mover::Sizing, Mover::Follow].iter().any(|m| m.text() == by);
        let others: Vec<_> = after.iter().filter(|(by, _)| !expected(by) && !matches!(way, Way::HomeThenTab)).collect();
        assert!(others.is_empty(), "{way:?}: moves after the open that weren't the agent's, the sizing's or following's: {others:?}");
        let x = u.app.session.ds().unwrap().view.cx;
        assert!(x > 3000.0, "{way:?}: the camera is on the batches: {x}");
    }
}

#[test]
fn each_camera_change_is_noted_with_where_it_was_made() {
    let (mut u, home) = scene();
    agent(&mut u, "view.set", json!({"cx": 900, "cy": 0, "zoom": 1}), false);
    agent(&mut u, "frame.create", json!({"x": 4000, "y": 0, "width": 200, "height": 200}), false);
    frames(&mut u, 40);
    u.app.run("view.zoomIn", json!({}));
    frames(&mut u, 2);
    // The person's scroll on the canvas.
    let p = Pos2::new(700.0, 400.0);
    u.frame(vec![Event::PointerMoved(p)]);
    u.frame(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    }]);
    // The scroll eases over a few frames.
    frames(&mut u, 40);
    // A focus the person asked for (a Layers click): its flight.
    u.app.session.execute("select.set", &json!({"ids": [home.0]})).unwrap();
    crate::camera_flight::focus_selection(&mut u.app);
    frames(&mut u, 40);
    let log = log(&u);
    // Each in turn, after the one before (the scene's own setup comes first).
    let mut from = 0;
    for (m, what) in [(Mover::Agent, "view.set"), (Mover::Follow, ""), (Mover::Window, ""), (Mover::Canvas, ""), (Mover::PersonFlight, "")] {
        let at = log[from..].iter().position(|(by, w)| by == m.text() && w == what);
        from += at.unwrap_or_else(|| panic!("{m:?} after entry {from}: {log:?}")) + 1;
    }
    let inspected = crate::follow_agent::inspect(&u.app);
    assert_eq!(inspected["cameraLog"].as_array().map(Vec::len), Some(log.len()), "ui.inspect has the log");
}

#[test]
fn a_pause_for_a_camera_move_names_where_it_was_made() {
    let said = |m: Mover| format!("a camera move that wasn't follow's or an agent's (made by {})", m.text());
    // The agent is still at work; the person zooms with a shortcut.
    let (mut u, _) = scene();
    agent(&mut u, "frame.create", json!({"x": 4000, "y": 0, "width": 200, "height": 200}), false);
    frames(&mut u, 40);
    u.app.run("view.zoomOut", json!({}));
    frames(&mut u, 2);
    assert_eq!(pauses(&u), [format!("following paused: {}", said(Mover::Window))]);
    assert_eq!(crate::follow_agent::inspect(&u.app)["pausedBy"], json!(said(Mover::Window)), "ui.inspect says the same");
    // The person focuses a layer from Layers: its flight, once it lands.
    let (mut u, home) = scene();
    agent(&mut u, "frame.create", json!({"x": 4000, "y": 0, "width": 200, "height": 200}), false);
    frames(&mut u, 40);
    u.app.session.execute("select.set", &json!({"ids": [home.0]})).unwrap();
    crate::camera_flight::focus_selection(&mut u.app);
    frames(&mut u, 40);
    assert_eq!(pauses(&u), [format!("following paused: {}", said(Mover::PersonFlight))]);
}
