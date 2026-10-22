//! Fix N: a pause lasts only while an agent works. The chip and the AI tab's Resume show only while
//! paused and working; when the work ends (finish_working, grace, a disconnect) they fade out over
//! 150 ms (at once without motion) and the pause is cleared, so the next build is followed with no
//! press; several clients keep it until the last; "Follow agents" Off never shows it; nothing is
//! written to the document, undo or the change log.

use egui::{Event, Modifiers, MouseWheelUnit, TouchPhase, vec2};
use serde_json::{Value, json};

use super::{FADE, fading, shown};
use crate::follow_agent::tests::{click, frames, view};
use crate::follow_agent::working_state::gap_tests::{calm, close, rect, send};
use crate::layers_drag_tests::Ui;

const CODEX: Option<&str> = Some("codex-mcp-client");

fn state(u: &Ui) -> &'static str {
    figcraft_engine::cmd::camera::follow_state(&u.app.session)
}

/// The person turns the wheel over the canvas: following pauses.
fn scroll(u: &mut Ui) {
    let at = u.app.canvas.rect.center();
    u.frame(vec![Event::PointerMoved(at)]);
    u.frame(vec![Event::MouseWheel { unit: MouseWheelUnit::Point, delta: vec2(0.0, -120.0), phase: TouchPhase::Move, modifiers: Modifiers::NONE }]);
    // The scroll eases the view for a few frames (and moves the revision, as a view change does).
    frames(u, 20);
}

/// The chip's AccessKit node ("Jump back to …"), if the frame has one.
fn chip_node(u: &mut Ui) -> Option<String> {
    u.ctx.enable_accesskit();
    let out = u.frame_output(vec![]);
    out.platform_output.accesskit_update?.nodes.into_iter().filter_map(|(_, n)| n.label().map(str::to_string)).find(|l| l.starts_with("Jump back to"))
}

fn follow_events(u: &Ui) -> Vec<String> {
    u.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "follow").map(|e| e.detail.clone()).collect()
}

/// What an agent's work may never touch here: the revision, undo and the change log.
fn written(u: &Ui) -> (u64, (usize, usize), Option<u64>) {
    let ds = u.app.session.ds().unwrap();
    (ds.revision, u.app.session.undo_depth(), u.app.session.agents.last_edit(ds.uid))
}

/// Codex builds in session `s`, the person scrolls: paused, the chip shown.
fn paused_build(u: &mut Ui, s: &str) {
    rect(u, Some(s), CODEX, 8000);
    frames(u, 40);
    scroll(u);
    assert_eq!(state(u), "paused");
    assert!(shown(&u.app) && u.app.ui.agent_view.follow.chip.is_some(), "the chip while paused and working");
}

#[test]
fn finish_working_fades_the_chip_out_clears_the_pause_and_the_next_build_is_followed() {
    let mut u = calm();
    paused_build(&mut u, "s1");
    assert_eq!(chip_node(&mut u).as_deref(), Some("Jump back to Codex's work"), "its AccessKit name says what it does");
    let before = written(&u);
    send(&mut u, Some("s1"), CODEX, "engine.execute", json!({"command": "agent.finish-working"}));
    // Cleared at once; the chip is only painted while it fades, with no hit area and no node.
    assert_eq!(state(&u), "following", "the pause is cleared");
    assert!(!shown(&u.app));
    let now = u.time;
    assert!(fading(&u.app, now).is_some(), "fading out");
    assert_eq!(crate::follow_agent::inspect(&u.app)["chip"]["shown"], false, "ui.inspect says it fades");
    assert_eq!(chip_node(&mut u), None, "out of the AccessKit tree while it fades");
    assert!(u.app.ui.agent_view.follow.chip.is_none(), "not pressable");
    frames(&mut u, (FADE * 60.0).ceil() as usize + 1);
    assert!(fading(&u.app, u.time).is_none(), "gone within {FADE} s");
    assert!(crate::follow_agent::inspect(&u.app)["chip"].is_null());
    assert_eq!(written(&u), before, "no revision, undo step or change-log entry");
    assert!(follow_events(&u).contains(&"follow pause cleared: the agent finished".to_string()), "{:?}", follow_events(&u));
    // The camera stays where the person left it until the next build, which it follows unasked.
    let left = view(&u);
    frames(&mut u, 30);
    assert_eq!(view(&u), left, "no fly back at the end");
    rect(&mut u, Some("s2"), CODEX, 20000);
    frames(&mut u, 40);
    assert!((view(&u).0 - 20010.0).abs() < 2000.0, "the next build is followed: {:?}", view(&u));
    assert_eq!(state(&u), "following");
}

#[test]
fn the_grace_period_and_a_disconnect_end_it_too() {
    let mut u = calm();
    u.app.session.agents.grace_for_tests = Some(5);
    paused_build(&mut u, "s1");
    frames(&mut u, 5 * 60 + 20);
    assert_eq!(state(&u), "following");
    assert!(!shown(&u.app) && u.app.ui.agent_view.follow.chip.is_none());
    assert!(follow_events(&u).contains(&"follow pause cleared: the grace period expired".to_string()), "{:?}", follow_events(&u));
    let mut u = calm();
    paused_build(&mut u, "s1");
    close(&mut u, "s1");
    frames(&mut u, 12);
    assert_eq!(state(&u), "following");
    assert!(u.app.ui.agent_view.follow.chip.is_none());
    assert!(follow_events(&u).contains(&"follow pause cleared: the agent disconnected".to_string()), "{:?}", follow_events(&u));
}

#[test]
fn with_two_clients_the_chip_stays_until_the_last_finishes() {
    let mut u = calm();
    rect(&mut u, Some("b"), Some("claude-code"), 6000);
    paused_build(&mut u, "a");
    send(&mut u, Some("a"), CODEX, "engine.execute", json!({"command": "agent.finish-working"}));
    frames(&mut u, 12);
    assert_eq!(state(&u), "paused", "Claude Code is still working");
    assert!(u.app.ui.agent_view.follow.chip.is_some());
    send(&mut u, Some("b"), Some("claude-code"), "engine.execute", json!({"command": "agent.finish-working"}));
    frames(&mut u, 12);
    assert_eq!(state(&u), "following");
    assert!(u.app.ui.agent_view.follow.chip.is_none());
}

#[test]
fn with_follow_agents_off_there_is_never_a_chip_and_a_pause_never_changes_the_setting() {
    let mut u = calm();
    u.app.run("camera.follow-agents", json!({"on": false}));
    rect(&mut u, Some("s1"), CODEX, 8000);
    frames(&mut u, 10);
    scroll(&mut u);
    assert_eq!(state(&u), "off");
    assert!(u.app.ui.agent_view.follow.chip.is_none() && chip_node(&mut u).is_none());
    assert!(!u.app.session.prefs.settings.follow_agents);
    let mut u = calm();
    paused_build(&mut u, "s1");
    assert!(u.app.session.prefs.settings.follow_agents, "a pause leaves the saved setting on");
    send(&mut u, Some("s1"), CODEX, "engine.execute", json!({"command": "agent.finish-working"}));
    frames(&mut u, 12);
    assert!(u.app.session.prefs.settings.follow_agents);
}

#[test]
fn resume_mid_work_flies_to_the_latest_work_and_clears_the_pause() {
    let mut u = calm();
    paused_build(&mut u, "s1");
    rect(&mut u, Some("s1"), CODEX, 14000);
    frames(&mut u, 10);
    let chip = u.app.ui.agent_view.follow.chip.expect("the chip");
    click(&mut u, chip.center());
    u.frame(vec![Event::PointerMoved(egui::pos2(5.0, 5.0))]);
    frames(&mut u, 40);
    assert_eq!(state(&u), "following");
    assert!((view(&u).0 - 14010.0).abs() < 2000.0, "at the latest work: {:?}", view(&u));
    assert!(follow_events(&u).contains(&"follow pause cleared: resumed".to_string()), "{:?}", follow_events(&u));
}

#[test]
fn without_motion_the_chip_goes_at_once() {
    for (motion, reduce) in [("off", false), ("system", true)] {
        let mut u = calm();
        u.app.run("pref.motion", json!({"motion": motion}));
        u.app.session.prefs.reduce_motion = reduce;
        assert!(u.app.reduce_motion(), "{motion}");
        paused_build(&mut u, "s1");
        send(&mut u, Some("s1"), CODEX, "engine.execute", json!({"command": "agent.finish-working"}));
        assert!(fading(&u.app, u.time).is_none(), "{motion}: no fade");
        assert_eq!(chip_node(&mut u), None);
        assert!(u.app.ui.agent_view.follow.chip.is_none());
    }
}

/// The AI panel's Follow agents button (Fix T4; the AI tab's Resume row before): it shows paused,
/// and resumes, only while paused and working.
#[test]
fn the_ai_panels_follow_button_shows_paused_by_the_same_rule() {
    let (mut h, _) = crate::live_tests::Harness::new();
    h.app.session.agents.mcp.port = Some(7601);
    h.app.session.execute("panel.tab.ai", &json!({})).unwrap();
    h.app.handle_ui_requests();
    h.frames(3);
    // Requests queued as the endpoint queues them, answered in the next frame.
    let req = |h: &mut crate::live_tests::Harness, p: Value| {
        h.app.control_queue.push(crate::control::ControlRequest {
            route: crate::control::ROUTE_MCP_HTTP,
            client: CODEX.map(str::to_string),
            session: Some("s1".into()),
            id: json!(1),
            method: "engine.execute".into(),
            params: p,
            reply: std::sync::mpsc::channel().0,
            cancelled: Default::default(),
        });
        h.frames(1);
    };
    req(&mut h, json!({"command": "frame.create", "params": {"x": 0, "y": 0, "width": 100, "height": 100}}));
    h.frames(30);
    h.app.run("frame.create", json!({"x": 400, "y": 0, "width": 100, "height": 100, "name": "Mine"}));
    h.frames(3);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&h.app.session), "paused");
    let follow = |h: &crate::live_tests::Harness| crate::follow_agent::inspect(&h.app)["aiPanel"]["header"]["follow"].clone();
    assert_eq!(follow(&h), "paused", "paused while working");
    req(&mut h, json!({"command": "agent.finish-working"}));
    h.frames(20);
    assert_eq!(follow(&h), "following", "not once the work ended");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&h.app.session), "following");
}
