//! R5d: Settings ▸ Agents › Describe a change, Settings ▸ Diagnostics' lines for a run, and the
//! first-use confirmation remembered and forgotten.
//! - The switch turns the whole composer off (and `chat.send` with it).
//! - The time limit's four values; the next run gets the one picked.
//! - The agents allowed to run are listed with their program; Forget makes each ask again.
//! - Diagnostics keep the agent's name, how a run ended, its exit code and how long it took: never
//!   the prompt or what the agent said.

use serde_json::json;

use super::tests::{CLAUDE, Shared, feed, line_ev, window};
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::click_named;
use crate::services::{AgentEnd, AgentRunEvent};

/// Settings ▸ Agents, scrolled down to Describe a change (the last of the page since Fix T4).
fn open_settings(u: &mut Ui) {
    u.app.run("settings.open", json!({"section": "ai"}));
    u.frame(vec![]);
    u.frame(vec![]);
    let name = "Describe a change in the AI tab";
    for _ in 0..40 {
        let nodes = crate::layers_header::tests::tree(u);
        let done = nodes.iter().find(|n| n.label() == Some("Done")).and_then(|n| n.bounds()).map_or(700.0, |b| b.y0);
        let at = nodes.iter().find(|n| n.label() == Some(name)).and_then(|n| n.bounds()).map(|b| b.y0);
        if at.is_some_and(|y| y < done - 260.0 && y > 150.0) {
            return;
        }
        u.frame(vec![egui::Event::PointerMoved(egui::pos2(800.0, 450.0))]);
        u.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -120.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }]);
    }
}

fn chat_events(u: &Ui) -> Vec<String> {
    u.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "chat").map(|e| e.detail.clone()).collect()
}

fn all_events(u: &Ui) -> String {
    u.app.session.prefs.settings.diagnostics.iter().map(|e| e.detail.clone()).collect::<Vec<_>>().join("\n")
}

fn specs(script: &Shared) -> Vec<crate::services::AgentRunSpec> {
    script.lock().unwrap().specs.clone()
}

#[test]
fn the_switch_turns_the_composer_off() {
    let (mut u, _) = window();
    open_settings(&mut u);
    click_named(&mut u, "Describe a change in the AI tab");
    assert!(!u.app.session.prefs.settings.describe_change);
    let e = u.app.session.execute("chat.send", &json!({"prompt": "x"})).unwrap_err().to_string();
    assert!(e.contains("off in Settings ▸ Agents"), "{e}");
}

#[test]
fn the_time_limit_picked_is_the_next_runs() {
    let (mut u, script) = window();
    open_settings(&mut u);
    click_named(&mut u, "30 min");
    assert_eq!(u.app.session.prefs.settings.chat_time_limit_mins, 30);
    u.app.ui.preferences = false;
    u.app.run("chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
    u.app.run("chat.send", json!({"prompt": "x", "agent": "claude-code"}));
    u.frame(vec![]);
    assert_eq!(specs(&script).last().map(|s| s.limit_secs), Some(30 * 60));
}

#[test]
fn allowed_agents_are_listed_with_their_program_and_forget_asks_again() {
    let (mut u, _) = window();
    u.app.run("chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
    u.app.ui.agent_view.chat.confirm = None;
    open_settings(&mut u);
    let shapes_have =
        |u: &mut Ui, text: &str| crate::layers_header::tests::tree(u).iter().any(|n| n.label().or(n.value()).is_some_and(|l| l.contains(text)));
    assert!(shapes_have(&mut u, &format!("Claude Code: {CLAUDE}")), "the program is shown");
    click_named(&mut u, "Forget");
    assert!(u.app.session.prefs.settings.chat_allowed.is_empty());
    u.app.ui.preferences = false;
    u.app.run("chat.send", json!({"prompt": "x", "agent": "claude-code"}));
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.chat.confirm.is_some(), "it asks again");
}

#[test]
fn a_runs_diagnostics_hold_its_agent_how_it_ended_and_how_long_never_the_words() {
    let (mut u, script) = window();
    super::tests::send_allowed(&mut u, "make the secret plan blue");
    feed(
        &script,
        vec![
            line_ev(r#"{"type":"result","subtype":"success","result":"I turned the secret plan blue."}"#),
            AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec!["a private error line".into()], secs: 42.2 },
        ],
    );
    u.frame(vec![]);
    assert_eq!(chat_events(&u), ["Claude Code run started", "Claude Code run ended: finished, exit 0, 42 s"]);
    let all = all_events(&u);
    for words in ["secret plan", "blue", "private error"] {
        assert!(!all.contains(words), "no {words:?} in Diagnostics: {all}");
    }
    super::tests::send_allowed(&mut u, "again");
    u.app.run("chat.cancel", json!({}));
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Stopped, stderr: vec![], secs: 3.0 }]);
    u.frame(vec![]);
    assert_eq!(chat_events(&u).last().map(String::as_str), Some("Claude Code run ended: stopped, no exit code, 3 s"));
    super::tests::send_allowed(&mut u, "and again");
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::TimedOut, stderr: vec![], secs: 900.0 }]);
    u.frame(vec![]);
    assert_eq!(chat_events(&u).last().map(String::as_str), Some("Claude Code run ended: timed out, no exit code, 900 s"));
}

#[test]
fn a_failed_start_is_logged_as_failed_with_its_code() {
    let (mut u, script) = window();
    super::tests::send_allowed(&mut u, "x");
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(1)), stderr: vec!["Not logged in".into()], secs: 0.4 }]);
    u.frame(vec![]);
    assert_eq!(chat_events(&u).last().map(String::as_str), Some("Claude Code run ended: failed, exit 1, 0 s"));
}
