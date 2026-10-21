//! J1: the agent's name comes from the MCP client it is, never a guess. For Claude Code, Codex,
//! Gemini CLI, Cursor (unknown to the table, shown as announced), an unknown name, no name and a
//! very long name: the AI tab's "Connected · …", "Following … · Poster", the working indicator's
//! AccessKit name, the Activity feed's row and the diagnostics event. Follow, the reveal and the
//! dots behave the same for every one; with several clients, the latest to edit is named.

use egui::Shape;
use serde_json::{Value, json};

use crate::follow_agent::tests::{frames, scene};
use crate::layers_drag_tests::Ui;

/// Run `cmd` as the MCP client `client` over the app's endpoint route (as `handle_request` does
/// for a request from that client's session).
fn agent_as(u: &mut Ui, client: &str, cmd: &str, params: Value) -> Value {
    let ctx = u.ctx.clone();
    let before = u.app.session.agents.client.replace(client.to_string());
    let r = crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "engine.execute", &json!({"command": cmd, "params": params}));
    u.app.session.agents.client = before;
    r.unwrap()
}

/// The client initializes (the endpoint's `mcp.client` report).
fn connect(u: &mut Ui, client: &str) {
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap();
    crate::mcp_ui::client_seen(&mut u.app, &json!({"name": client, "version": "1", "now": now})).unwrap();
}

fn texts(u: &mut Ui) -> Vec<String> {
    let out = u.frame_output(vec![]);
    let shapes: Vec<Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
    crate::live_tests::texts(&shapes).into_iter().map(|(t, _, _)| t).collect()
}

/// What a person sees for each client name: (announced, shown in a sentence's middle, at its start).
fn cases() -> Vec<(String, String, String)> {
    let long = format!("very-long-agent-{}", "x".repeat(200));
    let cut = format!("{}…", long.chars().take(figcraft_engine::agents::NAME_MAX).collect::<String>());
    vec![
        ("claude-code".into(), "Claude Code".into(), "Claude Code".into()),
        ("codex-mcp-client".into(), "Codex".into(), "Codex".into()),
        ("gemini-cli-mcp-client".into(), "Gemini CLI".into(), "Gemini CLI".into()),
        ("Cursor".into(), "Cursor".into(), "Cursor".into()),
        ("my-new-agent".into(), "my-new-agent".into(), "my-new-agent".into()),
        ("".into(), "an agent".into(), "An agent".into()),
        (long, cut.clone(), cut),
    ]
}

/// What follow, the reveal and the dots did: the camera (cx, cy at 0.1, zoom at 0.001), layers
/// entering, the dots' columns.
#[derive(Debug, PartialEq)]
struct Behaviour {
    view: (i64, i64, i64),
    entering: usize,
    cols: Option<usize>,
}

#[test]
fn every_place_names_the_client_and_behaves_the_same_for_each() {
    let mut behaviour: Vec<(String, Behaviour)> = Vec::new();
    for (raw, mid, start) in cases() {
        let (mut u, _) = scene();
        u.app.ui.agent_view.budget.frozen = true;
        u.ctx.enable_accesskit();
        u.app.session.agents.mcp.port = Some(7601);
        u.app.run("panel.tab.ai", json!({}));
        connect(&mut u, &raw);
        let p = agent_as(&mut u, &raw, "frame.create", json!({"x": 8000, "y": 0, "width": 400, "height": 300, "name": "Poster"}));
        let pid = p["id"].clone();
        agent_as(&mut u, &raw, "shape.rectangle", json!({"x": 10, "y": 10, "width": 40, "height": 40, "parent": pid}));
        u.app.run("select.none", json!({}));
        u.frame(vec![]);
        let entering = u.app.ui.agent_view.reveal.entering().count();
        frames(&mut u, 30);
        // Follow's text, the indicator's name (AccessKit), the AI tab.
        assert_eq!(crate::follow_agent::status(&u.app), Some(format!("Following {mid} · Poster")), "{raw:?}");
        let working = format!("{start} is working on Poster");
        assert_eq!(crate::follow_agent::working_text(&u.app).as_deref(), Some(working.as_str()), "{raw:?}");
        let out = u.frame_output(vec![]);
        let nodes: Vec<egui::accesskit::Node> = out.platform_output.accesskit_update.expect("a tree").nodes.into_iter().map(|(_, n)| n).collect();
        assert_eq!(nodes.iter().filter(|n| n.label() == Some(working.as_str())).count(), 1, "{raw:?}: one polite status");
        let seen = texts(&mut u);
        let connected = format!("Connected · {mid}");
        assert!(seen.contains(&connected), "{raw:?}: {connected:?} in {seen:?}");
        let row = format!("{start}: Create frame");
        let row = if raw.is_empty() { "Agent (mcp (http)): Create frame".to_string() } else { row };
        assert!(seen.contains(&row), "{raw:?}: the Activity row {row:?}");
        assert!(!seen.iter().any(|t| t.contains("Following Claude ·") && !raw.starts_with("claude")), "never Claude for another client");
        // Diagnostics: as announced (cleaned), or "an agent".
        let d = u.app.session.prefs.settings.diagnostics.iter().rev().find(|e| e.kind == "mcpClient").map(|e| e.detail.clone()).unwrap_or_default();
        let want =
            if raw.is_empty() { "an agent 1 connected".to_string() } else { format!("{} 1 connected", raw.chars().take(80).collect::<String>()) };
        assert_eq!(d, want, "{raw:?}");
        // Behaviour: the camera, the reveal and the dots, the same for every client.
        let v = &u.app.session.ds().unwrap().view;
        let cols = u.app.ui.agent_view.indicator.drawn.get().map(|(_, c)| c);
        behaviour.push((raw.clone(), Behaviour { view: ((v.cx * 10.0) as i64, (v.cy * 10.0) as i64, (v.zoom * 1000.0) as i64), entering, cols }));
    }
    let first = &behaviour[0];
    for b in &behaviour {
        assert_eq!(b.1, first.1, "{:?} behaves like {:?}", b.0, first.0);
    }
    assert!(first.1.entering > 0 && first.1.cols.is_some(), "the reveal and the dots ran: {first:?}");
}

#[test]
fn with_several_clients_the_latest_to_edit_is_named_and_all_are_listed() {
    let (mut u, home) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    u.app.session.agents.mcp.port = Some(7601);
    connect(&mut u, "codex-mcp-client");
    connect(&mut u, "gemini-cli-mcp-client");
    // Gemini initialized last, but Codex edits: Codex is the one working.
    agent_as(&mut u, "codex-mcp-client", "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    frames(&mut u, 3);
    assert_eq!(crate::follow_agent::status(&u.app).as_deref(), Some("Following Codex · Home"));
    agent_as(&mut u, "gemini-cli-mcp-client", "shape.rectangle", json!({"x": 40, "y": 10, "width": 20, "height": 20, "parent": home}));
    frames(&mut u, 3);
    assert_eq!(crate::follow_agent::working_text(&u.app).as_deref(), Some("Gemini CLI is working on Home"));
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap();
    let st = figcraft_engine::cmd::mcp::status(&u.app.session, Some(now));
    let names: Vec<&str> = st["clients"].as_array().unwrap().iter().filter_map(|c| c["display"].as_str()).collect();
    assert_eq!(names, ["Gemini CLI", "Codex"], "both listed, newest first");
}
