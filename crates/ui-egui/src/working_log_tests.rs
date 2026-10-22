//! R3a: the working state processed and logged on the request path, and while the window draws no
//! frames.
//! - `finish_working` logs its `end` (cause finish_working) as it is answered, with no frame.
//! - A grace period ends on time while the window is hidden: `logic` alone logs the `end`.
//! - Every transition wakes the window, and a wake-up is asked for at the grace period's end.
//! - The platform keeps the app awake from the first request to the end, and never while idle.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use egui::{RawInput, ViewportId, ViewportInfo};
use serde_json::{Value, json};

use crate::Services;
use crate::control::{ROUTE_MCP_HTTP, handle_route};
use crate::follow_agent::tests::scene;
use crate::layers_drag_tests::Ui;

/// What the platform was asked: keep the app awake (`true`) or not.
struct Platform(Arc<Mutex<Vec<bool>>>);

impl Services for Platform {
    fn keep_awake(&mut self, on: bool) {
        self.0.lock().unwrap().push(on);
    }
}

struct Run {
    u: Ui,
    awake: Arc<Mutex<Vec<bool>>>,
    wakes: Arc<Mutex<Vec<Duration>>>,
}

fn run() -> Run {
    let (mut u, _) = scene();
    u.app.session.agents.grace_for_tests = Some(5);
    let awake = Arc::new(Mutex::new(Vec::new()));
    u.app.services = Box::new(Platform(awake.clone()));
    let wakes = Arc::new(Mutex::new(Vec::new()));
    let w = wakes.clone();
    u.ctx.set_request_repaint_callback(move |i| w.lock().unwrap().push(i.delay));
    Run { u, awake, wakes }
}

impl Run {
    /// A request from Codex's session, answered as the window's queue answers it: no frame.
    fn request(&mut self, command: &str, params: Value) -> Value {
        let agents = &mut self.u.app.session.agents;
        agents.session = Some("s1".into());
        agents.client = Some("codex-mcp-client".into());
        let ctx = self.u.ctx.clone();
        let r = handle_route(&mut self.u.app, &ctx, ROUTE_MCP_HTTP, "engine.execute", &json!({"command": command, "params": params}));
        let agents = &mut self.u.app.session.agents;
        agents.session = None;
        agents.client = None;
        r.unwrap_or(Value::Null)
    }

    fn working_events(&self) -> Vec<String> {
        self.u.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "working").map(|e| e.detail.clone()).collect()
    }

    fn woken_at_once(&self) -> bool {
        self.wakes.lock().unwrap().contains(&Duration::ZERO)
    }

    /// Forget the wake-ups asked for so far, and serve the one pending (as the app does between
    /// passes), so the next request reaches the integration.
    fn forget_wakes(&self) {
        // egui owes a pass or two after a frame that asked for one: serve them all.
        for _ in 0..4 {
            let _ = self.u.ctx.run_logic(&RawInput::default(), |_| {});
        }
        self.wakes.lock().unwrap().clear();
    }

    fn awake(&self) -> Vec<bool> {
        self.awake.lock().unwrap().clone()
    }
}

#[test]
fn finish_working_logs_its_end_with_its_cause_with_no_frames_run() {
    let mut r = run();
    let frames_before = r.u.time;
    r.forget_wakes();
    r.request("frame.create", json!({"x": 300, "y": 0, "width": 100, "height": 100, "name": "Poster"}));
    let events = r.working_events();
    assert!(events.last().is_some_and(|e| e.starts_with("begin:") && e.contains("first request")), "{events:?}");
    assert!(r.woken_at_once(), "the begin wakes the window");
    assert_eq!(r.awake(), [true], "kept awake once work begins");
    r.forget_wakes();
    let answer = r.request("agent.finish-working", json!({}));
    assert_eq!(answer["finished"], json!(true), "{answer}");
    let events = r.working_events();
    assert!(events.last().is_some_and(|e| e.starts_with("end:") && e.contains("finish_working")), "{events:?}");
    assert!(r.woken_at_once(), "the end wakes the window");
    assert_eq!(r.awake(), [true, false], "let go at the end");
    assert_eq!(r.u.time, frames_before, "no frame was drawn");
}

#[test]
fn a_grace_period_ends_on_time_while_the_window_is_hidden_without_frames() {
    let mut r = run();
    r.request("frame.create", json!({"x": 300, "y": 0, "width": 100, "height": 100, "name": "Poster"}));
    // The begin's own wake-up served, the next logic pass asks to be woken at the grace's end.
    r.forget_wakes();
    let ctx = r.u.ctx.clone();
    let _ = ctx.run_logic(&RawInput::default(), |ctx| crate::follow_agent::working_log::logic(&mut r.u.app, ctx));
    let wakes = r.wakes.lock().unwrap().clone();
    assert!(wakes.iter().any(|d| (d.as_secs_f64() - 5.0).abs() < 0.1), "a wake-up asked for at the grace period's end: {wakes:?}");
    let begun = r.working_events().len();
    // The window is minimised: no frames, egui's clock stuck at the last one. The wall clock
    // carries on: put the last frame 6 s back, past the 5 s grace.
    let hidden = RawInput {
        viewports: [(ViewportId::ROOT, ViewportInfo { minimized: Some(true), ..Default::default() })].into_iter().collect(),
        ..Default::default()
    };
    let frame = r.u.app.ui.agent_view.clock.frame.expect("a frame was drawn");
    let back = web_time::Instant::now().checked_sub(Duration::from_secs(6)).expect("the clock goes back 6 s");
    r.u.app.ui.agent_view.clock.frame = Some((frame.0, back));
    // The first logic pass: the state ends.
    let _ = ctx.run_logic(&hidden, |ctx| crate::follow_agent::working_log::logic(&mut r.u.app, ctx));
    let events = r.working_events();
    assert_eq!(events.len(), begun + 1, "{events:?}");
    assert!(events.last().is_some_and(|e| e.starts_with("end:") && e.contains("grace period expired")), "{events:?}");
    assert_eq!(r.awake(), [true, false]);
    // Nothing more on the next pass.
    let _ = ctx.run_logic(&hidden, |ctx| crate::follow_agent::working_log::logic(&mut r.u.app, ctx));
    assert_eq!(r.working_events().len(), begun + 1);
}

#[test]
fn a_visible_window_reads_egui_s_clock_and_never_goes_back() {
    let mut r = run();
    let t = r.u.ctx.input(|i| i.time);
    let ctx = r.u.ctx.clone();
    assert_eq!(crate::follow_agent::working_log::clock(&mut r.u.app, &ctx), t, "drawn: egui's own clock");
    // A frame 6 s back by the wall clock doesn't move a visible window's clock.
    let back = web_time::Instant::now().checked_sub(Duration::from_secs(6)).expect("6 s back");
    r.u.app.ui.agent_view.clock.frame = Some((t, back));
    assert_eq!(crate::follow_agent::working_log::clock(&mut r.u.app, &ctx), t);
}

#[test]
fn idle_never_asks_to_keep_the_app_awake() {
    let mut r = run();
    for _ in 0..30 {
        r.u.frame(vec![]);
    }
    // The person's own edit isn't an agent at work.
    r.u.app.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 10, "height": 10}));
    for _ in 0..30 {
        r.u.frame(vec![]);
    }
    let ctx = r.u.ctx.clone();
    let _ = ctx.run_logic(&RawInput::default(), |ctx| crate::follow_agent::working_log::logic(&mut r.u.app, ctx));
    assert!(r.awake().is_empty(), "{:?}", r.awake());
    assert!(r.working_events().is_empty());
}

#[test]
fn frames_after_a_logged_end_log_nothing_twice_and_follow_still_hears_of_it() {
    let mut r = run();
    r.request("frame.create", json!({"x": 300, "y": 0, "width": 100, "height": 100, "name": "Poster"}));
    r.request("agent.finish-working", json!({}));
    let logged = r.working_events();
    assert_eq!(logged.len(), 2, "{logged:?}");
    assert!(r.u.app.ui.agent_view.ended.is_some(), "kept for the next frame");
    r.u.frame(vec![]);
    assert!(r.u.app.ui.agent_view.ended.is_none(), "the frame took it (follow's pause clears then)");
    for _ in 0..10 {
        r.u.frame(vec![]);
    }
    assert_eq!(r.working_events(), logged, "the frames logged nothing again");
}
