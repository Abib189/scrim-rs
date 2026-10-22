//! Stage I.1 ID, the working state in the window, driven the way the app's MCP endpoint drives it:
//! requests on the control queue, each with its route, client and session (`ControlRequest`).
//! - A client making six batches 2.5 s apart keeps the dots on in every frame between them.
//! - Read-only calls (`screenshot`'s render, `inspect_document`, `lint_design`) keep it alive.
//! - `finish_working` ends it with the 400 ms fade; a session closing ends it too.
//! - With no finish, it ends at the grace period's end exactly.
//! - Two clients: one finishing leaves it on for the other, named after that one.
//! - A client with an unknown or empty name reads "an agent" (J1's rules).
//! - `finish_working` adds no revision or undo step, and agents may call it.

use serde_json::{Value, json};

use super::FADE_OUT;
use crate::control::{ControlRequest, ROUTE_MCP_HTTP};
use crate::follow_agent::tests::{frames, scene};
use crate::layers_drag_tests::Ui;

/// A request from MCP client `client` (`clientInfo.name`) in `session`, as the endpoint queues it;
/// answered in the next frame, and its answer returned.
fn send(u: &mut Ui, session: Option<&str>, client: Option<&str>, method: &str, params: Value) -> Value {
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

fn run(u: &mut Ui, session: Option<&str>, client: Option<&str>, command: &str, params: Value) -> Value {
    send(u, session, client, "engine.execute", json!({"command": command, "params": params}))
}

/// One step of a build: an artboard, then `n - 1` layers in it as one batch.
fn step(u: &mut Ui, session: Option<&str>, client: Option<&str>, i: usize, n: usize) {
    let r = run(u, session, client, "frame.create", json!({"x": 760 * i, "y": 0, "width": 640, "height": 480, "name": format!("Step {i}")}));
    let parent = r["id"].clone();
    let commands: Vec<Value> =
        (1..n).map(|k| json!({"command": "shape.rectangle", "params": {"x": 10 * k, "y": 10, "width": 8, "height": 8, "parent": parent}})).collect();
    run(u, session, client, "command.batch", json!({"commands": commands}));
}

/// The dots are drawn, and where the person can see them: the row's right end (the artboard's
/// top-right corner it hangs from) inside the canvas, not clipped off it.
fn shown(u: &Ui) -> bool {
    let canvas = u.app.canvas.rect;
    u.app.ui.agent_view.indicator.drawn.get().is_some_and(|(r, _)| canvas.contains(r.right_bottom()) && canvas.contains(r.left_top()))
}

fn calm() -> Ui {
    let (mut u, _) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    u
}

/// Frames for `secs` seconds (the test clock is 60 Hz), asserting the dots are drawn in each.
fn on_for(u: &mut Ui, secs: f64, what: &str) {
    for f in 0..(secs * 60.0) as usize {
        u.frame(vec![]);
        assert!(shown(u), "{what}: the dots are drawn in frame {f}");
    }
}

#[test]
fn six_steps_2_5_s_apart_keep_the_dots_in_every_frame_then_finish_fades_them_in_400_ms() {
    let mut u = calm();
    let (s, c) = (Some("codex-session"), Some("codex-mcp-client"));
    for i in 0..6 {
        step(&mut u, s, c, i, [6, 12, 18, 24, 30, 36][i]);
        if i == 0 {
            frames(&mut u, 12); // past the 160 ms fade in
        }
        on_for(&mut u, 2.5, &format!("after step {i}"));
    }
    assert_eq!(crate::follow_agent::working_text(&u.app).as_deref(), Some("Codex is working on Step 5"));
    let _ = run(&mut u, s, c, "agent.finish-working", json!({}));
    assert!(!u.app.ui.agent_view.working.working(), "finished");
    frames(&mut u, 12);
    assert!(shown(&u), "fading: still drawn 200 ms after");
    frames(&mut u, (FADE_OUT * 60.0) as usize);
    assert!(!shown(&u), "gone 400 ms after");
    assert_eq!(crate::follow_agent::finished_text(&u.app).as_deref(), Some("Codex finished"));
}

#[test]
fn read_only_calls_keep_it_alive_past_the_grace_period() {
    let mut u = calm();
    u.app.session.agents.grace_for_tests = Some(5);
    let (s, c) = (Some("s1"), Some("codex-mcp-client"));
    step(&mut u, s, c, 0, 8);
    frames(&mut u, 12);
    // 12 s of reads only, 2.5 s apart: the screenshot's render, inspect_document, lint_design.
    let reads: [(&str, Value); 3] =
        [("ui.render", json!({"scale": 1})), ("document.inspect", json!({})), ("engine.execute", json!({"command": "lint.run", "params": {}}))];
    for k in 0..5 {
        on_for(&mut u, 2.5, &format!("read {k}"));
        let (m, p) = &reads[k % 3];
        send(&mut u, s, c, m, p.clone());
    }
    assert!(u.app.ui.agent_view.working.working(), "still working 12.5 s after the last edit");
}

#[test]
fn without_a_finish_it_ends_at_the_grace_period_exactly() {
    for grace in [5u32, 15] {
        let mut u = calm();
        u.app.session.agents.grace_for_tests = Some(grace);
        step(&mut u, Some("s1"), None, 0, 6);
        let last = u.time;
        // Just before the grace period ends: on. Then the frame after its end: ended, fading.
        while u.time < last + f64::from(grace) - 2.0 / 60.0 {
            u.frame(vec![]);
        }
        assert!(u.app.ui.agent_view.working.working(), "{grace} s: on just before");
        frames(&mut u, 3);
        assert!(!u.app.ui.agent_view.working.working(), "{grace} s: ended");
        let a = u.app.ui.agent_view.working.alpha(u.time, true);
        assert!(a > 0.0 && a < 1.0, "{grace} s: the fade out has begun ({a})");
        frames(&mut u, (FADE_OUT * 60.0) as usize + 2);
        assert!(!shown(&u), "{grace} s: gone");
    }
}

#[test]
fn a_session_closing_ends_it() {
    let mut u = calm();
    step(&mut u, Some("s1"), Some("gemini-cli-mcp-client"), 0, 6);
    frames(&mut u, 12);
    assert!(u.app.ui.agent_view.working.working());
    send(&mut u, None, None, "mcp.session-ended", json!({"session": "s1"}));
    assert!(!u.app.ui.agent_view.working.working(), "ended with the session");
    assert_eq!(crate::follow_agent::finished_text(&u.app).as_deref(), Some("Gemini CLI finished"));
}

#[test]
fn with_two_clients_one_finishing_leaves_it_on_for_the_other() {
    let mut u = calm();
    step(&mut u, Some("a"), Some("codex-mcp-client"), 0, 6);
    step(&mut u, Some("b"), Some("gemini-cli-mcp-client"), 1, 6);
    frames(&mut u, 40); // the camera's flight to Step 1 (320 ms)
    assert_eq!(crate::follow_agent::working_text(&u.app).as_deref(), Some("Gemini CLI is working on Step 1"), "the latest");
    let _ = run(&mut u, Some("a"), Some("codex-mcp-client"), "agent.finish-working", json!({}));
    on_for(&mut u, 1.0, "Gemini CLI still works");
    assert_eq!(crate::follow_agent::working_text(&u.app).as_deref(), Some("Gemini CLI is working on Step 1"));
    let _ = run(&mut u, Some("b"), Some("gemini-cli-mcp-client"), "agent.finish-working", json!({}));
    assert!(!u.app.ui.agent_view.working.working());
}

#[test]
fn unknown_and_empty_names_read_as_announced_or_an_agent() {
    for (raw, want) in [("my-own-agent", "my-own-agent is working on Step 0"), ("", "An agent is working on Step 0")] {
        let mut u = calm();
        u.app.session.agents.mcp.clients.clear();
        step(&mut u, Some("s"), Some(raw).filter(|r| !r.is_empty()), 0, 6);
        frames(&mut u, 3);
        assert_eq!(crate::follow_agent::working_text(&u.app).as_deref(), Some(want), "{raw:?}");
    }
}

#[test]
fn finish_working_adds_no_revision_or_undo_step() {
    let mut u = calm();
    step(&mut u, Some("s"), Some("codex-mcp-client"), 0, 6);
    let before = {
        let ds = u.app.session.ds().unwrap();
        (ds.revision, u.app.session.undo_depth())
    };
    let _ = run(&mut u, Some("s"), Some("codex-mcp-client"), "agent.finish-working", json!({"ids": []}));
    let ds = u.app.session.ds().unwrap();
    assert_eq!((ds.revision, u.app.session.undo_depth()), before);
    assert!(!u.app.ui.agent_view.working.working(), "and it worked, from an agent");
}
