//! R5e: Describe a change end to end, as the desktop app runs it, without a window on screen (a
//! locked Mac draws none): the window's own code frame by frame on the wall clock, the app's real
//! MCP endpoint on a free port (never 7680), the real runner (`agent_runs`), and a real child
//! process. Nothing is saved: these services keep no settings.
//! - `stub_*`: the stub agent (`crates/mcp/tests/stub/stub_agent.py`) through the composer: the
//!   dots, follow and the reveal while it works, the transcript, Stop with the partial work undone,
//!   and the time limit.
//! - `real_claude_once`: one real Claude Code run, only with `SCRIM_LIVE_CLAUDE=1`; its `HOME` is
//!   the person's (for its own sign-in), Scrim's is in-process and saves nothing.
//!
//! All are ignored by default (they start processes and take seconds):
//! `cargo test -p figcraft --bin figcraft chat_e2e -- --ignored --test-threads=1`. With
//! `SCRIM_CHAT_LIVE_OUT=<dir>` each writes its log there, paths replaced by placeholders.

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::{Pos2, RawInput, Rect, vec2};
use figcraft_engine::cmd::agent_clients::PathEnv;
use figcraft_ui_egui::follow_agent::chat_ui::Line;
use figcraft_ui_egui::{AgentRun, AgentRunSpec, FigApp, Services};
use serde_json::{Value, json};

use crate::mcp_endpoint::McpHandle;

type Queue = Arc<Mutex<Vec<figcraft_ui_egui::ControlRequest>>>;

struct E2e {
    mcp: McpHandle,
    program: Option<String>,
    home: String,
    limit: Option<u64>,
}

impl Services for E2e {
    fn mcp_restart(&mut self, settings: &figcraft_engine::settings::McpSettings) -> Result<u16, String> {
        self.mcp.restart(settings)
    }
    fn find_program(&self, name: &str) -> Option<String> {
        (name == "claude").then(|| self.program.clone()).flatten()
    }
    fn agent_client_env(&self) -> Option<PathEnv> {
        let mut e = PathEnv { home: Some(PathBuf::from(&self.home)), ..Default::default() };
        e.vars.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
        for k in ["USER", "LOGNAME"] {
            if let Ok(v) = std::env::var(k) {
                e.vars.insert(k.into(), v);
            }
        }
        if let Some(l) = self.limit {
            e.vars.insert(figcraft_ui_egui::follow_agent::chat_ui::LIMIT_VAR.into(), l.to_string());
        }
        Some(e)
    }
    fn start_agent(&mut self, spec: AgentRunSpec) -> Result<Box<dyn AgentRun>, String> {
        crate::agent_runs::start(spec)
    }
}

fn stub() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/mcp/tests/stub/stub_agent.py").canonicalize().unwrap().to_string_lossy().into_owned()
}

fn scratch_home() -> String {
    let d = std::env::temp_dir().join(format!("scrim-chat-e2e-home-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.to_string_lossy().into_owned()
}

/// The app, as `main` sets it up, with its MCP endpoint on a free port.
struct World {
    ctx: egui::Context,
    app: FigApp,
    queue: Queue,
    start: Instant,
    port: u16,
    log: Vec<Value>,
}

impl World {
    fn new(program: Option<String>, home: String, limit: Option<u64>) -> Self {
        let queue: Queue = Arc::new(Mutex::new(Vec::new()));
        let ctx = egui::Context::default();
        let slot = Arc::new(Mutex::new(Some(ctx.clone())));
        let mcp = McpHandle::new(queue.clone(), slot);
        let free = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut s = figcraft_engine::Session::new();
        s.prefs.settings.mcp.port = free;
        let mut app = FigApp::new(s, Box::new(E2e { mcp, program, home, limit }));
        figcraft_ui_egui::mcp_ui::restart(&mut app);
        let port = app.session.agents.mcp.port.expect("the endpoint started");
        assert_ne!(port, 7680, "never the person's Scrim");
        let mut w = World { ctx, app, queue, start: Instant::now(), port, log: Vec::new() };
        w.app.run("panel.tab.ai", json!({}));
        for _ in 0..5 {
            w.frame();
        }
        w
    }

    /// One frame on the wall clock, as the app runs it: the requests that arrived, logic, then the
    /// window.
    fn frame(&mut self) {
        let time = self.start.elapsed().as_secs_f64();
        crate::control_server::drain_queue(&self.queue, &mut self.app.control_queue);
        self.app.logic(&self.ctx);
        figcraft_ui_egui::follow_agent::working_log::logic(&mut self.app, &self.ctx);
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440.0, 900.0))), time: Some(time), ..Default::default() };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.ui(ui));
        out.textures_delta.clear();
    }

    /// Frames until `done`, at most `secs`; what was seen on the way.
    fn until(&mut self, what: &str, secs: f64, done: impl Fn(&FigApp) -> bool) -> Seen {
        let deadline = Instant::now() + Duration::from_secs_f64(secs);
        let mut seen = Seen::default();
        loop {
            self.frame();
            let v = &self.app.ui.agent_view;
            seen.working |= v.working.working();
            seen.revealed |= v.reveal.entering().count() > 0;
            seen.paused |= figcraft_engine::cmd::camera::follow_state(&self.app.session) == "paused";
            if done(&self.app) {
                self.note(what);
                return seen;
            }
            assert!(Instant::now() < deadline, "{what}: not within {secs} s; transcript {:?}", lines(&self.app));
            std::thread::sleep(Duration::from_millis(8));
        }
    }

    fn note(&mut self, what: &str) {
        let t = (self.start.elapsed().as_secs_f64() * 100.0).round() / 100.0;
        self.log.push(json!({"t": t, "what": what}));
    }

    fn send(&mut self, prompt: &str, program: &str) {
        self.app.run("chat.allow-agent", json!({"agent": "claude-code", "path": program}));
        self.app.ui.agent_view.chat.draft = prompt.into();
        self.app.run("chat.send", json!({"prompt": prompt, "agent": "claude-code"}));
        self.note("sent");
    }

    /// The log, when `SCRIM_CHAT_LIVE_OUT` names a folder, with paths as placeholders.
    fn write(&self, name: &str, extra: Value) {
        let Some(dir) = std::env::var_os("SCRIM_CHAT_LIVE_OUT") else { return };
        let mut v = json!({"events": self.log, "transcript": lines(&self.app).iter().map(|l| format!("{l:?}")).collect::<Vec<_>>(),
            "diagnostics": self.app.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "chat" || e.kind == "working").map(|e| e.detail.clone()).collect::<Vec<_>>(),
            "endpoint_port_was_not_7680": self.port != 7680});
        if let (Some(a), Some(b)) = (v.as_object_mut(), extra.as_object()) {
            a.extend(b.clone());
        }
        let mut text = serde_json::to_string_pretty(&v).unwrap();
        for (from, to) in [(stub(), "<stub>".to_string()), (std::env::temp_dir().to_string_lossy().trim_end_matches('/').to_string(), "<tmp>".into())]
        {
            text = text.replace(&from, &to);
        }
        if let Some(home) = std::env::var_os("HOME") {
            text = text.replace(home.to_string_lossy().as_ref(), "/Users/<user>");
        }
        std::fs::write(PathBuf::from(dir).join(name), text).unwrap();
    }
}

#[derive(Default, Debug)]
struct Seen {
    working: bool,
    revealed: bool,
    paused: bool,
}

fn lines(app: &FigApp) -> Vec<Line> {
    app.ui.agent_view.chat.lines.clone()
}

fn running(app: &FigApp) -> bool {
    app.ui.agent_view.chat.running.is_some()
}

fn rectangles(app: &FigApp) -> usize {
    app.session.doc().map(|d| d.nodes().filter(|n| matches!(n.kind, figcraft_engine::doc::NodeKind::Rectangle)).count()).unwrap_or(0)
}

fn group_alive(pgid: u32) -> bool {
    std::process::Command::new("kill").args(["-0", "--", &format!("-{pgid}")]).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success())
}

/// The stub's process (its group's id), by the prompt it was given.
fn stub_pid(prompt: &str) -> Option<u32> {
    let out = std::process::Command::new("pgrep").args(["-f", &format!("stub_agent.py -p {prompt}")]).output().ok()?;
    String::from_utf8_lossy(&out.stdout).split_whitespace().next()?.parse().ok()
}

#[test]
#[ignore = "starts processes; run with --ignored"]
fn stub_end_to_end_through_the_composer() {
    let stub = stub();
    let mut w = World::new(Some(stub.clone()), scratch_home(), None);
    w.send("batches 3 slow 2", &stub);
    w.until("running", 5.0, running);
    let seen = w.until("three batches landed", 30.0, |a| rectangles(a) >= 9);
    assert!(seen.working, "the dots: an agent at work");
    let seen2 = w.until("ended", 30.0, |a| !running(a));
    let l = lines(&w.app);
    assert_eq!(l.first(), Some(&Line::You("batches 3 slow 2".into())));
    assert_eq!(l.iter().filter(|x| **x == Line::Work("Agent: made 3 layers".into())).count(), 3, "{l:?}");
    assert!(l.contains(&Line::Reply { agent: "Claude Code".into(), text: "Made batch 1, batch 2, batch 3.".into() }), "{l:?}");
    assert!(matches!(l.last(), Some(Line::Cost(c)) if c.ends_with("· Billed by your agent's plan.")), "{l:?}");
    w.until("the dots gone", 5.0, |a| !a.ui.agent_view.working.working());
    assert!(seen.revealed || seen2.revealed, "the new layers had their entrance");
    assert!(!seen.paused && !seen2.paused, "following never paused");
    w.write("stub-run.json", json!({"seen_working": seen.working, "seen_reveal": seen.revealed || seen2.revealed, "follow_paused": seen.paused || seen2.paused, "rectangles": rectangles(&w.app)}));
}

#[test]
#[ignore = "starts processes; run with --ignored"]
fn stub_stop_mid_run_kills_the_tree_and_undo_takes_the_work_back() {
    let stub = stub();
    let mut w = World::new(Some(stub.clone()), scratch_home(), None);
    let prompt = "batches 2 slow 37";
    w.send(prompt, &stub);
    w.until("two batches landed", 30.0, |a| rectangles(a) >= 6);
    w.until("the stub waits", 10.0, |_| stub_pid(prompt).is_some());
    let pgid = stub_pid(prompt).unwrap();
    assert!(group_alive(pgid));
    let t = Instant::now();
    w.app.run("chat.cancel", json!({}));
    w.note("Stop pressed");
    w.until("stopped", 10.0, |a| !running(a));
    let stop_secs = t.elapsed().as_secs_f64();
    assert!(!group_alive(pgid), "the agent and its own child are gone");
    w.until("the dots gone at once", 2.0, |a| !a.ui.agent_view.working.working());
    assert!(lines(&w.app).contains(&Line::Note("Stopped. What the agent made so far stays: Undo takes it back.".into())));
    assert_eq!(rectangles(&w.app), 6, "the partial work stays");
    w.app.run("edit.undo", json!({}));
    w.app.run("edit.undo", json!({}));
    w.frame();
    assert_eq!(rectangles(&w.app), 0, "two batches, two undo steps");
    w.write("stub-stop.json", json!({"stop_to_ended_secs": (stop_secs * 100.0).round() / 100.0, "process_group_left": group_alive(pgid), "rectangles_after_two_undos": rectangles(&w.app)}));
}

#[test]
#[ignore = "starts processes; run with --ignored"]
fn stub_time_limit_stops_the_run() {
    let stub = stub();
    let mut w = World::new(Some(stub.clone()), scratch_home(), Some(3));
    w.send("rectangle named Capped slow 40", &stub);
    w.until("timed out", 20.0, |a| !running(a));
    let l = lines(&w.app);
    assert!(l.contains(&Line::Failure("Stopped after 3 s, this run's time limit.".into())), "{l:?}");
    assert_eq!(rectangles(&w.app), 1, "what it made stays");
    w.write("stub-time-limit.json", json!({"rectangles": rectangles(&w.app)}));
}

/// One real Claude Code run. Only with `SCRIM_LIVE_CLAUDE=1`; it uses the person's plan once.
#[test]
#[ignore = "runs the real claude once; needs SCRIM_LIVE_CLAUDE=1"]
fn real_claude_once() {
    if std::env::var("SCRIM_LIVE_CLAUDE").as_deref() != Ok("1") {
        eprintln!("skipped: set SCRIM_LIVE_CLAUDE=1");
        return;
    }
    let program = crate::services::find_program("claude").expect("claude on this computer");
    let home = std::env::var("HOME").unwrap();
    let mut w = World::new(Some(program.clone()), home, Some(120));
    w.send("Create one 100x100 rectangle named Test", &program);
    let seen = w.until("running", 5.0, running);
    let seen2 = w.until("ended", 150.0, |a| !running(a));
    let made = w
        .app
        .session
        .doc()
        .map(|d| d.nodes().any(|n| n.name == "Test" && matches!(n.kind, figcraft_engine::doc::NodeKind::Rectangle)))
        .unwrap_or(false);
    let size = w.app.session.doc().ok().and_then(|d| d.nodes().find(|n| n.name == "Test").map(|n| (n.width, n.height)));
    w.write("real-claude.json", json!({"rectangle_named_test": made, "size": size, "seen_working": seen.working || seen2.working, "follow_paused": seen.paused || seen2.paused}));
    eprintln!("transcript: {:#?}", lines(&w.app));
}
