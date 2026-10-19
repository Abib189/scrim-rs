//! The control channel is an agent's route (Port 3): a layer the person locked refuses its
//! `engine.execute`, its edits are logged as an agent's, and `since` guards a stale write; the
//! person's own input in the same app still edits the locked layer.

use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::{FigApp, NoServices};

fn handle(app: &mut FigApp, method: &str, p: Value) -> Result<Value, String> {
    let ctx = egui::Context::default();
    let mut res = Err("not run".to_string());
    crate::windows::guard(app, &ctx, |app| res = super::handle(app, &ctx, method, &p));
    res
}

fn app_with_swatch() -> (FigApp, u64) {
    let mut s = Session::new();
    let r = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 20, "height": 10})).unwrap()["id"].as_u64().unwrap();
    s.execute("node.set", &json!({"id": r, "props": {"name": "Swatch"}})).unwrap();
    (FigApp::new(s, Box::new(NoServices)), r)
}

#[test]
fn a_locked_layer_refuses_the_control_channel_but_not_the_person() {
    let (mut app, r) = app_with_swatch();
    app.run("object.toggleLock", json!({"ids": [r]}));
    let e = handle(&mut app, "engine.execute", json!({"command": "fill.set", "params": {"ids": [r], "color": "#FF0000"}})).unwrap_err();
    assert!(e.contains("\"Swatch\" is locked"), "{e}");
    // The person, through the app's own commands, still can.
    assert!(app.run("fill.set", json!({"ids": [r], "color": "#00FF00"})).is_some());
}

#[test]
fn the_control_channel_is_logged_as_an_agent_and_guarded_by_since() {
    let (mut app, r) = app_with_swatch();
    let read = app.session.changes_since(0).unwrap()["revision"].as_u64().unwrap();
    app.run("node.set", json!({"id": r, "props": {"width": 40}}));
    let e =
        handle(&mut app, "engine.execute", json!({"command": "fill.set", "params": {"ids": [r], "color": "#FF0000"}, "since": read})).unwrap_err();
    assert!(e.contains("by the person"), "{e}");
    handle(&mut app, "engine.execute", json!({"command": "fill.set", "params": {"ids": [r], "color": "#FF0000"}})).unwrap();
    let log = app.session.changes_since(0).unwrap();
    assert_eq!(log["changes"].as_array().unwrap().last().unwrap()["author"], "agent (control channel)");
    // The author is the channel's only while it handles a request.
    assert_eq!(app.session.agents.author, figcraft_engine::agents::Author::Person);
}
