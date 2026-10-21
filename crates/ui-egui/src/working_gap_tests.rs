//! Fix M (`docs/design/working-gap.md`): the working state across a real client's batch boundaries.
//! M0: every transition (begin, end, hold) is kept in Settings ▸ Diagnostics with its cause and the
//! client as it named itself, on the request path the app's MCP endpoint uses.

use serde_json::{Value, json};

use crate::control::{ControlRequest, ROUTE_MCP_HTTP};
use crate::follow_agent::tests::{frames, scene};
use crate::layers_drag_tests::Ui;

/// The name Claude Desktop's bridge announces.
pub(crate) const BRIDGE: &str = "local-agent-mode-scrim (via mcp-remote 0.14.3)";

/// A request from `client` in `session`, as the endpoint queues it, answered in the next frame.
pub(crate) fn send(u: &mut Ui, session: Option<&str>, client: Option<&str>, method: &str, params: Value) -> Value {
    let (reply, answer) = std::sync::mpsc::channel();
    u.app.control_queue.push(ControlRequest {
        route: ROUTE_MCP_HTTP,
        client: client.map(str::to_string),
        session: session.map(str::to_string),
        id: json!(1),
        method: method.into(),
        params,
        reply,
        cancelled: Default::default(),
    });
    u.frame(vec![]);
    answer.try_recv().ok().and_then(|r| r.result.ok()).unwrap_or(Value::Null)
}

pub(crate) fn rect(u: &mut Ui, session: Option<&str>, client: Option<&str>, x: i64) -> Value {
    let p = json!({"command": "shape.rectangle", "params": {"x": x, "y": 0, "width": 20, "height": 20}});
    send(u, session, client, "engine.execute", p)
}

/// The session closing as the endpoint tells it (`DELETE /mcp`).
pub(crate) fn close(u: &mut Ui, session: &str) {
    send(u, None, None, "mcp.session-ended", json!({"session": session}));
}

pub(crate) fn calm() -> Ui {
    let (mut u, _) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    u.app.session.prefs.settings.diagnostics.clear();
    u
}

/// The working events kept so far, oldest first.
pub(crate) fn events(u: &Ui) -> Vec<String> {
    u.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "working").map(|e| e.detail.clone()).collect()
}

#[test]
fn every_transition_is_kept_with_its_cause_and_the_client() {
    let mut u = calm();
    let c = Some(BRIDGE);
    // Begin, then the session closes with nothing else working: end.
    rect(&mut u, Some("aaaaaaaa-1111"), c, 0);
    frames(&mut u, 30);
    close(&mut u, "aaaaaaaa-1111");
    frames(&mut u, 30);
    // Two clients: one finishing holds the run for the other; then the grace runs out.
    rect(&mut u, Some("bbbbbbbb-2222"), c, 40);
    rect(&mut u, Some("cccccccc-3333"), Some("codex-mcp-client"), 80);
    send(&mut u, Some("bbbbbbbb-2222"), c, "engine.execute", json!({"command": "agent.finish-working"}));
    frames(&mut u, 30);
    u.app.session.prefs.settings.agent_grace_secs = 5;
    frames(&mut u, 6 * 60);
    // An --attach connection closing is a disconnect.
    rect(&mut u, Some("attach-7"), c, 120);
    close(&mut u, "attach-7");
    frames(&mut u, 2);
    let who = format!("{BRIDGE} (session ");
    assert_eq!(
        events(&u),
        [
            format!("begin: {who}aaaaaaaa…), first request; the dots are on"),
            format!("end: {who}aaaaaaaa…), session closed; the dots fade out"),
            format!("begin: {who}bbbbbbbb…), first request; the dots are on"),
            "begin: codex-mcp-client (session cccccccc…), first request; the dots are on".to_string(),
            format!("hold: {who}bbbbbbbb…), finish_working; the dots are on"),
            "end: codex-mcp-client (session cccccccc…), grace period expired; the dots fade out".to_string(),
            format!("begin: {who}attach7…), first request; the dots are on"),
            format!("end: {who}attach7…), disconnected; the dots fade out"),
        ]
    );
}
