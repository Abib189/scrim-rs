//! C1.2: the port is busy, the endpoint falls back to the next free one, the AI tab sees that
//! Claude Code's entry still points at the old address, and Re-register (one confirmation) fixes it.

use std::collections::VecDeque;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::live_tests::{Harness, has_text, texts};
use crate::{ProgramOutput, ProgramRun};

#[derive(Default)]
struct Script {
    ran: Vec<Vec<String>>,
    answers: VecDeque<ProgramOutput>,
    /// The ports the endpoint stand-in listens on (kept open while the test runs).
    listening: Vec<TcpListener>,
}

/// The platform: a real port search like the app's (the first free port from the preferred one) and
/// a scripted `claude`.
struct Platform(Arc<Mutex<Script>>);

impl crate::Services for Platform {
    fn mcp_restart(&mut self, settings: &figcraft_engine::settings::McpSettings) -> Result<u16, String> {
        let mut s = self.0.lock().unwrap();
        s.listening.clear();
        for port in settings.port..settings.port.saturating_add(20) {
            if let Ok(l) = TcpListener::bind(("127.0.0.1", port)) {
                s.listening.push(l);
                return Ok(port);
            }
        }
        Err("no free port".into())
    }
    fn run_program(&mut self, _program: &str, args: &[String]) -> ProgramRun {
        let mut s = self.0.lock().unwrap();
        s.ran.push(args.to_vec());
        let out = s.answers.pop_front().unwrap_or_default();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Ok(out)).unwrap();
        rx
    }
    fn find_program(&self, name: &str) -> Option<String> {
        (name == "claude").then(|| "/Users/me/.local/bin/claude".to_string())
    }
}

fn registered(url: &str) -> ProgramOutput {
    ProgramOutput {
        code: Some(0),
        stdout: format!("scrim:\n  Scope: User config (available in all your projects)\n  Type: http\n  URL: {url}\n"),
        stderr: String::new(),
    }
}

fn ok() -> ProgramOutput {
    ProgramOutput { code: Some(0), stdout: "ok".into(), stderr: String::new() }
}

fn click_lowest(h: &mut Harness, text: &str) {
    let shapes = h.frames(2);
    let at = texts(&shapes).into_iter().filter(|(t, _, _)| t == text).map(|(_, p, s)| p + s / 2.0).max_by(|a, b| a.y.total_cmp(&b.y));
    h.click(at.unwrap_or_else(|| panic!("no {text:?} on screen")));
    h.frames(4);
}

#[test]
fn a_busy_port_falls_back_the_mismatch_shows_and_reregister_fixes_it() {
    // A free port the OS just handed out, then kept busy (as OpenPencil kept 7600).
    let busy = TcpListener::bind("127.0.0.1:0").unwrap();
    let preferred = busy.local_addr().unwrap().port();
    let old = format!("http://127.0.0.1:{preferred}/mcp");
    let script = Arc::new(Mutex::new(Script::default()));
    let (mut h, _) = Harness::new();
    h.app.services = Box::new(Platform(script.clone()));
    h.app.session.prefs.settings.mcp.port = preferred;
    crate::mcp_ui::restart(&mut h.app);
    let live = h.app.session.agents.mcp.port.unwrap();
    assert!(live > preferred && live < preferred + 20, "fell back to the next free port: {live}");
    assert_eq!(h.app.session.prefs.settings.mcp.port, live, "and kept it");
    let new = format!("http://127.0.0.1:{live}/mcp");

    // Claude Code still points at the preferred port: the AI tab says so.
    script.lock().unwrap().answers = VecDeque::from([registered(&old)]);
    // The connections are in Settings ▸ Agents since Fix T4.
    h.app.session.execute("settings.open", &serde_json::json!({"section": "ai"})).unwrap();
    h.app.handle_ui_requests();
    h.app.handle_ui_requests();
    let shapes = h.frames(4);
    assert!(has_text(&shapes, "Registered address is out of date"), "the mismatch is detected");
    assert!(has_text(&shapes, &format!("Claude Code points to {old}; this window is at {new}.")));
    assert_eq!(script.lock().unwrap().ran, [vec!["mcp".to_string(), "get".into(), "scrim".into()]], "a read-only check");

    // Re-register: one confirmation, then remove (in the entry's scope) and add the live address.
    script.lock().unwrap().answers = VecDeque::from([ok(), ok(), registered(&new)]);
    click_lowest(&mut h, "Re-register");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Re-register Scrim?"), "one confirmation");
    assert_eq!(script.lock().unwrap().ran.len(), 1, "nothing runs before it");
    click_lowest(&mut h, "Re-register");
    h.frames(8);
    let ran = script.lock().unwrap().ran.clone();
    let add: Vec<String> = ["mcp", "add", "--transport", "http", "--scope", "user", "scrim", &new].iter().map(|s| s.to_string()).collect();
    assert_eq!(ran[1], ["mcp", "remove", "scrim", "--scope", "user"]);
    assert_eq!(ran[2], add);
    assert_eq!(ran[3], ["mcp", "get", "scrim"], "asked again after the change");
    let shapes = h.frames(2);
    assert!(!has_text(&shapes, "Registered address is out of date"), "fixed");
    drop(busy);
}

#[test]
fn an_agent_cannot_reregister() {
    let script = Arc::new(Mutex::new(Script::default()));
    let (mut h, _) = Harness::new();
    h.app.services = Box::new(Platform(script.clone()));
    let ctx = h.ctx.clone();
    crate::control::handle(&mut h.app, &ctx, "engine.execute", &serde_json::json!({"command": "mcp.reregister"})).unwrap();
    h.frames(3);
    assert!(script.lock().unwrap().ran.iter().all(|a| a.get(1).is_some_and(|v| v == "get")), "nothing that changes Claude Code ran");
}
