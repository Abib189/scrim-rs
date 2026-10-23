//! R5c: the composer in the window harness, with a scripted agent run in place of the desktop's
//! runner (the runner itself is `crates/mcp/tests/agent_run.rs`).
//! - Shown on the AI tab only, and only while "Describe a change" is on.
//! - Send's reasons when it can't go; the first use asks, with the program and the command.
//! - A run: its spec (the binary, the profile's arguments with the prompt and context line, the
//!   minimal environment, the time limit), the batch lines, the reply and the cost line.
//! - Stop (and Esc), with the partial work undoable; ⌘↩ sends.
//! - Each failure in plain words.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use serde_json::json;

use super::{Line, elapsed, send_blocked};
use crate::Services;
use crate::layers_drag_tests::Ui;
use crate::services::{AgentEnd, AgentRun, AgentRunEvent, AgentRunSpec};

/// What the scripted runs saw and were told.
#[derive(Default)]
pub(crate) struct Script {
    /// Where each agent's binary is found (none: not installed).
    pub(crate) claude: Option<String>,
    pub(crate) codex: Option<String>,
    pub(crate) specs: Vec<AgentRunSpec>,
    /// Events the current run gives on its next poll.
    pub(crate) events: VecDeque<AgentRunEvent>,
    pub(crate) stops: usize,
    pub(crate) dropped: usize,
    pub(crate) opened: Vec<String>,
    /// The conversations as the app kept them (Fix T4), and how many times.
    pub(crate) conversations: Option<String>,
    pub(crate) saves: usize,
    /// What the open dialog answers (Fix AA3), and the titles it was asked with.
    pub(crate) pick: Option<String>,
    pub(crate) picked: Vec<String>,
}

pub(crate) type Shared = Arc<Mutex<Script>>;

pub(crate) struct Platform(pub(crate) Shared);

struct Scripted(Shared);

impl AgentRun for Scripted {
    fn poll(&mut self) -> Vec<AgentRunEvent> {
        self.0.lock().unwrap().events.drain(..).collect()
    }
    fn stop(&mut self) {
        self.0.lock().unwrap().stops += 1;
    }
}

impl Drop for Scripted {
    fn drop(&mut self) {
        self.0.lock().unwrap().dropped += 1;
    }
}

impl Services for Platform {
    fn find_program(&self, name: &str) -> Option<String> {
        let s = self.0.lock().unwrap();
        match name {
            "claude" => s.claude.clone(),
            "codex" => s.codex.clone(),
            _ => None,
        }
    }
    fn start_agent(&mut self, spec: AgentRunSpec) -> Result<Box<dyn AgentRun>, String> {
        self.0.lock().unwrap().specs.push(spec);
        Ok(Box::new(Scripted(self.0.clone())))
    }
    fn agent_client_env(&self) -> Option<figcraft_engine::cmd::agent_clients::PathEnv> {
        let mut e = figcraft_engine::cmd::agent_clients::PathEnv { home: Some("/Users/someone".into()), ..Default::default() };
        e.vars.insert("PATH".into(), "/usr/bin:/bin".into());
        e.vars.insert("USER".into(), "someone".into());
        e.vars.insert("LOGNAME".into(), "someone".into());
        e.vars.insert("SECRET_OF_THE_APP".into(), "never passed on".into());
        Some(e)
    }
    fn pick_open(&mut self, title: &str, _exts: &[&str]) -> crate::DialogFuture {
        let mut s = self.0.lock().unwrap();
        s.picked.push(title.to_string());
        let path = s.pick.clone().map(std::path::PathBuf::from);
        Box::pin(std::future::ready(path))
    }

    fn open_url(&mut self, url: &str) {
        self.0.lock().unwrap().opened.push(url.to_string());
    }
    fn load_conversations(&mut self) -> Option<String> {
        self.0.lock().unwrap().conversations.clone()
    }
    fn save_conversations(&mut self, json: &str) {
        let mut s = self.0.lock().unwrap();
        s.conversations = Some(json.to_string());
        s.saves += 1;
    }
}

pub(crate) const CLAUDE: &str = "/opt/agents/bin/claude";

/// The AI tab of a window with a frame "Poster" selected, its MCP endpoint on 7690, and Claude Code
/// installed at [`CLAUDE`].
pub(crate) fn window() -> (Ui, Shared) {
    let mut s = figcraft_engine::Session::new();
    let f = s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 200, "height": 200, "name": "Poster"})).unwrap();
    s.execute("select.set", &json!({"ids": [f["id"]]})).unwrap();
    s.agents.mcp.port = Some(7690);
    let mut u = Ui::new(s);
    let script: Shared = Arc::new(Mutex::new(Script { claude: Some(CLAUDE.into()), ..Default::default() }));
    u.app.services = Box::new(Platform(script.clone()));
    u.app.run("panel.tab.ai", json!({}));
    u.frame(vec![]);
    u.frame(vec![]);
    (u, script)
}

fn lines(u: &Ui) -> Vec<Line> {
    u.app.ui.agent_view.chat.lines.clone()
}

fn type_and_send(u: &mut Ui, prompt: &str) {
    u.app.ui.agent_view.chat.draft = prompt.to_string();
    u.app.run("chat.send", json!({"prompt": prompt, "agent": "claude-code"}));
    u.frame(vec![]);
}

/// Send `prompt`, answering the first-use confirmation with Run.
pub(crate) fn send_allowed(u: &mut Ui, prompt: &str) {
    type_and_send(u, prompt);
    if u.app.ui.agent_view.chat.confirm.is_some() {
        u.app.run("chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
        u.frame(vec![]);
    }
}

pub(crate) fn feed(script: &Shared, events: Vec<AgentRunEvent>) {
    script.lock().unwrap().events.extend(events);
}

pub(crate) fn line_ev(s: &str) -> AgentRunEvent {
    AgentRunEvent::Line(s.to_string())
}

fn names(u: &mut Ui) -> Vec<(Role, String)> {
    u.ctx.enable_accesskit();
    let out = u.frame_output(vec![]);
    out.platform_output
        .accesskit_update
        .map(|up| up.nodes.into_iter().filter_map(|(_, n)| n.label().map(|l| (n.role(), l.to_string()))).collect())
        .unwrap_or_default()
}

#[test]
fn the_composer_shows_on_the_ai_tab_while_describe_a_change_is_on() {
    let (mut u, _) = window();
    assert!(names(&mut u).iter().any(|(r, n)| *r == Role::TextInput && n == "Describe a change"), "the field, named");
    u.app.run("panel.tab.design", json!({}));
    assert!(!names(&mut u).iter().any(|(_, n)| n == "Describe a change"), "not on the Design tab");
    u.app.run("panel.tab.ai", json!({}));
    u.app.run("pref.describeChange", json!({"on": false}));
    assert!(!names(&mut u).iter().any(|(_, n)| n == "Describe a change"), "off: nothing of it");
}

#[test]
fn send_says_why_it_cant_go() {
    let (mut u, script) = window();
    let now = 1.0;
    assert_eq!(send_blocked(&mut u.app, now).as_deref(), Some("Type what to change"));
    u.app.ui.agent_view.chat.draft = "make it blue".into();
    assert_eq!(send_blocked(&mut u.app, now), None);
    u.app.session.agents.mcp.port = None;
    assert_eq!(send_blocked(&mut u.app, now).as_deref(), Some("Scrim's MCP server is off: turn it on in Settings ▸ MCP"));
    u.app.session.agents.mcp.port = Some(7690);
    script.lock().unwrap().claude = None;
    assert_eq!(send_blocked(&mut u.app, now + 10.0).as_deref(), Some("No agent found on this computer"));
}

#[test]
fn with_no_agent_installed_it_says_so_with_a_link_to_settings_agents() {
    // Fix T4, then Y1: one line, "{agent} isn't installed.", and the Agent settings link.
    let (mut u, script) = window();
    script.lock().unwrap().claude = None;
    u.app.ui.agent_view.chat = Default::default();
    let n = names(&mut u);
    assert!(n.iter().any(|(_, l)| l == "Agent settings"), "{n:?}");
    assert!(!n.iter().any(|(_, l)| l.ends_with(" docs")), "no docs buttons: {n:?}");
    let line = crate::follow_agent::inspect(&u.app)["aiPanel"]["composer"]["missingCli"].clone();
    assert_eq!(line, "Claude Code isn't installed.");
    crate::layers_header::tests::click_named(&mut u, "Agent settings");
    u.app.handle_ui_requests();
    assert!(u.app.ui.preferences && u.app.ui.settings_section == "ai", "the link opens Settings ▸ Agents");
}

#[test]
fn the_first_use_asks_with_the_program_and_the_command_then_runs() {
    let (mut u, script) = window();
    type_and_send(&mut u, "make it blue");
    assert!(script.lock().unwrap().specs.is_empty(), "nothing runs before the person confirms");
    assert_eq!(u.app.ui.agent_view.chat.confirm.as_ref().map(|c| c.1.as_str()), Some(CLAUDE));
    let n = names(&mut u);
    assert!(n.iter().any(|(_, l)| l == "Run Claude Code?"), "the dialog: {n:?}");
    u.app.run("chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
    u.frame(vec![]);
    let spec = script.lock().unwrap().specs.pop().expect("it ran");
    assert_eq!(spec.program, CLAUDE);
    let after = |flag: &str| spec.args.iter().position(|a| a == flag).and_then(|i| spec.args.get(i + 1)).cloned().unwrap_or_default();
    assert!(
        after("-p").starts_with("make it blue\n\nScrim: page “Page 1”; selected: “Poster” ("),
        "the prompt and the context line: {:?}",
        after("-p")
    );
    assert!(after("--mcp-config").contains("http://127.0.0.1:7690/mcp"), "this window's endpoint");
    let keys: Vec<&str> = spec.env.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["PATH", "HOME", "USER", "LOGNAME", "SCRIM_CHAT_CHILD"], "the app's other variables aren't passed on");
    assert_eq!(spec.limit_secs, 15 * 60);
    // Asked once for this agent and this program.
    type_and_send(&mut u, "and the title");
    assert!(u.app.ui.agent_view.chat.confirm.is_none());
    assert_eq!(script.lock().unwrap().specs.len(), 0, "a run is still in progress: the window waits");
}

#[test]
fn another_program_asks_again() {
    let (mut u, script) = window();
    send_allowed(&mut u, "one");
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 1.0 }]);
    u.frame(vec![]);
    script.lock().unwrap().claude = Some("/elsewhere/claude".into());
    u.app.ui.agent_view.chat = super::State { lines: lines(&u), ..Default::default() };
    type_and_send(&mut u, "two");
    assert_eq!(u.app.ui.agent_view.chat.confirm.as_ref().map(|c| c.1.as_str()), Some("/elsewhere/claude"));
}

#[test]
fn a_run_reads_as_your_message_its_work_its_reply_and_what_it_cost() {
    let (mut u, script) = window();
    send_allowed(&mut u, "add three squares");
    // The agent's batch, through the app's MCP route, as the child's requests arrive.
    let ctx = u.ctx.clone();
    let cmds: Vec<serde_json::Value> =
        (0..3).map(|k| json!({"command": "shape.rectangle", "params": {"x": 300 + 40 * k, "y": 0, "width": 30, "height": 30}})).collect();
    crate::control::handle_route(
        &mut u.app,
        &ctx,
        crate::control::ROUTE_MCP_HTTP,
        "engine.execute",
        &json!({"command": "command.batch", "params": {"commands": cmds}}),
    )
    .unwrap();
    feed(
        &script,
        vec![
            line_ev(r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"scrim","status":"connected"}]}"#),
            line_ev(r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__scrim__batch"}]}}"#),
            line_ev(r#"{"type":"result","subtype":"success","is_error":false,"result":"Added three squares."}"#),
            AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 42.0 },
        ],
    );
    u.frame(vec![]);
    assert_eq!(
        lines(&u),
        [
            Line::You("add three squares".into()),
            Line::Work("Agent: made 3 layers".into()),
            Line::Reply { agent: "Claude Code".into(), text: "Added three squares.".into() },
            Line::Cost("0:42 · Billed by your agent's plan.".into()),
        ]
    );
    assert!(u.app.ui.agent_view.chat.running.is_none());
}

#[test]
fn tokens_and_cost_show_only_as_the_agent_reported_them() {
    let (mut u, script) = window();
    send_allowed(&mut u, "x");
    feed(
        &script,
        vec![
            line_ev(
                r#"{"type":"result","subtype":"success","result":"Done.","total_cost_usd":0.0123,"usage":{"input_tokens":1200,"output_tokens":80}}"#,
            ),
            AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 3.0 },
        ],
    );
    u.frame(vec![]);
    assert_eq!(lines(&u).last(), Some(&Line::Cost("0:03 · 1200 tokens in, 80 out, $0.01, as the agent reported".into())));
}

#[test]
fn stop_ends_the_run_and_its_partial_work_stays_undoable() {
    let (mut u, script) = window();
    send_allowed(&mut u, "build a poster");
    let ctx = u.ctx.clone();
    crate::control::handle_route(
        &mut u.app,
        &ctx,
        crate::control::ROUTE_MCP_HTTP,
        "engine.execute",
        &json!({"command": "shape.rectangle", "params": {"x": 500, "y": 0, "width": 30, "height": 30, "name": "Partial"}}),
    )
    .unwrap();
    u.frame(vec![]);
    assert!(names(&mut u).iter().any(|(r, n)| *r == Role::Button && n == "Stop"), "Stop in place of Send");
    u.app.run("chat.cancel", json!({}));
    assert_eq!(script.lock().unwrap().stops, 1, "the runner is told");
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Stopped, stderr: vec![], secs: 5.0 }]);
    u.frame(vec![]);
    assert!(lines(&u).contains(&Line::Note("Stopped. What the agent made so far stays: Undo takes it back.".into())));
    let has = |u: &Ui| u.app.session.doc().unwrap().nodes().any(|n| n.name == "Partial");
    assert!(has(&u), "the partial work stays");
    u.app.run("edit.undo", json!({}));
    assert!(!has(&u), "and is an ordinary undo step");
}

#[test]
fn command_return_sends_and_escape_stops() {
    let (mut u, script) = window();
    u.app.run("chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
    u.app.ui.agent_view.chat.draft = "make it blue".into();
    u.ctx.memory_mut(|m| m.request_focus(egui::Id::new("describe_change_field")));
    u.frame(vec![]);
    u.frame_with(
        vec![Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND }],
        Modifiers::COMMAND,
    );
    u.frame(vec![]);
    assert_eq!(script.lock().unwrap().specs.len(), 1, "⌘↩ sent it");
    u.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
    assert_eq!(script.lock().unwrap().stops, 1, "Esc stopped it");
}

#[test]
fn each_failure_is_said_in_plain_words() {
    let end = |code: i32, secs: f64, stderr: &str| AgentRunEvent::Ended { end: AgentEnd::Exited(Some(code)), stderr: vec![stderr.to_string()], secs };
    let cases: Vec<(Vec<AgentRunEvent>, &str)> = vec![
        (
            vec![end(1, 0.4, "Error: Not logged in · Please run /login (key sk-ant-api03-abcdefghijk)")],
            "Claude Code stopped before it began: Error: Not logged in · Please run /login (key sk-<token>)",
        ),
        (
            vec![line_ev(r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"mcp__scrim__batch"}]}}"#), end(2, 30.0, "boom")],
            "Claude Code stopped with an error (exit 2): boom",
        ),
        (
            vec![AgentRunEvent::Ended { end: AgentEnd::TimedOut, stderr: vec![], secs: 900.0 }],
            "Stopped after 15 min, the time limit in Settings ▸ Agents.",
        ),
        (
            vec![line_ev(r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"scrim","status":"failed"}]}"#), end(1, 8.0, "")],
            "Claude Code couldn't reach Scrim's MCP server.",
        ),
        // As the real Claude Code said it on 22 Oct 2026: its error as its result, nothing on stderr.
        (
            vec![
                line_ev(
                    r#"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in · Please run /login","usage":{"input_tokens":0,"output_tokens":0},"total_cost_usd":0}"#,
                ),
                end(1, 1.4, ""),
            ],
            "Claude Code stopped before it began: Not logged in · Please run /login",
        ),
    ];
    for (events, want) in cases {
        let (mut u, script) = window();
        send_allowed(&mut u, "x");
        feed(&script, events);
        u.frame(vec![]);
        assert!(lines(&u).contains(&Line::Failure(want.into())), "{want}: {:?}", lines(&u));
        assert!(!lines(&u).iter().any(|l| matches!(l, Line::Reply { .. })), "an error isn't shown as a reply: {:?}", lines(&u));
    }
    // Not installed, and the endpoint down: nothing is started.
    let (mut u, script) = window();
    script.lock().unwrap().claude = None;
    type_and_send(&mut u, "x");
    assert!(lines(&u).contains(&Line::Failure("Claude Code isn't installed on this computer, or Scrim can't find it.".into())));
    let (mut u, script) = window();
    u.app.session.agents.mcp.port = None;
    type_and_send(&mut u, "x");
    assert!(lines(&u).contains(&Line::Failure("Scrim's MCP server is off. Turn it on in Settings ▸ MCP, then send again.".into())));
    assert!(script.lock().unwrap().specs.is_empty());
}

#[test]
fn closing_the_window_mid_run_lets_the_run_go() {
    let (mut u, script) = window();
    send_allowed(&mut u, "x");
    assert_eq!(script.lock().unwrap().dropped, 0);
    drop(u);
    assert_eq!(script.lock().unwrap().dropped, 1, "the run's handle goes with the window, and the desktop runner kills its tree then");
}

#[test]
fn the_elapsed_time_reads_minutes_and_seconds() {
    assert_eq!(elapsed(0.0), "0:00");
    assert_eq!(elapsed(42.4), "0:42");
    assert_eq!(elapsed(725.0), "12:05");
}

#[test]
fn its_text_reads_at_4_5_to_1_and_its_field_edge_at_3_to_1_in_both_themes() {
    for dark in [false, true] {
        let t = crate::theme::Tokens::for_prefs(dark, &figcraft_engine::Prefs::default());
        for (name, c) in [("text", t.text), ("secondary", t.text_secondary), ("failure", t.danger)] {
            let r = crate::theme::accent::contrast(c, t.panel);
            assert!(r >= 4.5, "{name} on the panel, dark {dark}: {r:.2}");
        }
        let edge = crate::theme::accent::contrast(t.text_secondary, t.field);
        assert!(edge >= 3.0, "the field's edge, dark {dark}: {edge:.2}");
    }
}
