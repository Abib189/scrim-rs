//! Connect Claude Code (C2), with a scripted platform that records every program run and answers
//! from a script: the exact command, one confirmation before anything runs, Replace instead of a
//! silent overwrite, the old entry's removal, Copy only without `claude`, and an agent that can't
//! make Scrim run anything.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde_json::json;

use super::{Entry, parse_get};
use crate::live_tests::{Harness, has_text, text_center};
use crate::{ProgramOutput, ProgramRun};

#[derive(Default)]
struct Script {
    /// The programs run, with their arguments.
    ran: Vec<(String, Vec<String>)>,
    /// The outputs to give, in order.
    answers: VecDeque<ProgramOutput>,
    copied: Vec<String>,
    claude: bool,
}

struct Platform(Arc<Mutex<Script>>);

impl crate::Services for Platform {
    fn run_program(&mut self, program: &str, args: &[String]) -> ProgramRun {
        let mut s = self.0.lock().unwrap();
        s.ran.push((program.to_string(), args.to_vec()));
        let out = s.answers.pop_front().unwrap_or_default();
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Ok(out)).unwrap();
        rx
    }
    fn find_program(&self, name: &str) -> Option<String> {
        (name == "claude" && self.0.lock().unwrap().claude).then(|| "/Users/me/.local/bin/claude".to_string())
    }
    fn clipboard_set_text(&mut self, s: &str) {
        self.0.lock().unwrap().copied.push(s.to_string());
    }
}

fn ok(stdout: &str) -> ProgramOutput {
    ProgramOutput { code: Some(0), stdout: stdout.into(), stderr: String::new() }
}

fn missing() -> ProgramOutput {
    ProgramOutput { code: Some(1), stdout: String::new(), stderr: "No MCP server found with name: x".into() }
}

const URL: &str = "http://127.0.0.1:7601/mcp";

/// The AI tab with the endpoint on 7601 and `claude` on this computer (or not). With `claude`, the
/// tab first asks what Claude Code has registered (C1.2, read-only): Claude Code has nothing, then
/// `answers` in order.
fn harness(claude: bool, answers: Vec<ProgramOutput>) -> (Harness, Arc<Mutex<Script>>) {
    let (mut h, _) = Harness::new();
    let answers: Vec<ProgramOutput> = if claude { std::iter::once(missing()).chain(answers).collect() } else { answers };
    let script = Arc::new(Mutex::new(Script { claude, answers: answers.into(), ..Default::default() }));
    h.app.services = Box::new(Platform(script.clone()));
    h.app.session.agents.mcp.port = Some(7601);
    // The connections are in Settings ▸ Agents since Fix T4.
    h.app.session.execute("settings.open", &serde_json::json!({"section": "ai"})).unwrap();
    h.app.handle_ui_requests();
    h.app.handle_ui_requests();
    h.frames(3);
    (h, script)
}

fn click(h: &mut Harness, text: &str) {
    // Scrolled into view in Settings ▸ Agents (Fix T4).
    let shapes = crate::live_tests::scroll_settings_to(h, text);
    h.click(text_center(&shapes, text));
    h.frames(3);
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The runs that change Claude Code (add or remove); `claude mcp get` only reads.
fn writes(script: &Arc<Mutex<Script>>) -> Vec<Vec<String>> {
    script.lock().unwrap().ran.iter().map(|(_, a)| a.clone()).filter(|a| a.get(1).is_some_and(|v| v != "get")).collect()
}

fn add_args() -> Vec<String> {
    args(&["mcp", "add", "--transport", "http", "--scope", "user", "scrim", URL])
}

#[test]
fn parse_get_reads_claude_codes_answer() {
    let found = ok(
        "scrim:\n  Scope: Local config (private to you in this project)\n  Status: ✗ Failed to connect\n  Type: http\n  URL: http://127.0.0.1:4848/mcp\n",
    );
    assert_eq!(parse_get(&found), Some(Entry { scope: Some("local".into()), target: "http://127.0.0.1:4848/mcp".into() }));
    let stdio = ok("figcraft:\n  Scope: User config (available in all your projects)\n  Type: stdio\n  Command: figcraft-cli mcp\n");
    assert_eq!(parse_get(&stdio), Some(Entry { scope: Some("user".into()), target: "figcraft-cli mcp".into() }));
    assert_eq!(parse_get(&missing()), None);
}

#[test]
fn connect_shows_the_command_runs_nothing_until_confirmed_then_adds_scrim() {
    let figcraft = ok("figcraft:\n  Scope: Local config (private to you in this project)\n  Type: stdio\n  Command: figcraft-cli mcp\n");
    let (mut h, script) = harness(true, vec![missing(), figcraft, ok("Added HTTP MCP server scrim"), ok("Removed figcraft")]);
    let shapes = h.frames(1);
    assert!(has_text(&shapes, "Not connected") && has_text(&shapes, "Connect Claude Code"), "the empty state");
    click(&mut h, "Connect Claude Code");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, &format!("claude mcp add --transport http --scope user scrim {URL}")), "the exact command");
    assert!(has_text(&shapes, "Run") && has_text(&shapes, "Copy"));
    assert!(writes(&script).is_empty(), "nothing that changes Claude Code runs to show it");
    assert_eq!(script.lock().unwrap().ran.len(), 1, "only the registration check (read-only)");
    click(&mut h, "Run");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Run this command?"), "one confirmation");
    assert_eq!(script.lock().unwrap().ran.len(), 1, "nothing more runs before it");
    // Confirm: the dialog's Run (the lower one on screen).
    let runs: Vec<_> = crate::live_tests::texts(&shapes).into_iter().filter(|(t, _, _)| t == "Run").map(|(_, p, s)| p + s / 2.0).collect();
    let confirm = runs.iter().copied().max_by(|a, b| a.y.total_cmp(&b.y)).unwrap();
    h.click(confirm);
    h.frames(8);
    let ran: Vec<_> = script.lock().unwrap().ran[1..4].to_vec();
    let claude = "/Users/me/.local/bin/claude".to_string();
    assert_eq!(
        ran,
        vec![(claude.clone(), args(&["mcp", "get", "scrim"])), (claude.clone(), args(&["mcp", "get", "figcraft"])), (claude.clone(), add_args())],
        "an argument list each time, never a shell line"
    );
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Scrim was added to Claude Code for every project. Start a new Claude Code session to use it."));
    // The old entry from before the rename: removed only when asked, and confirmed.
    click(&mut h, "Remove the old entry");
    assert!(has_text(&h.frames(2), "Remove the old entry?"));
    click(&mut h, "Remove");
    h.frames(4);
    assert_eq!(script.lock().unwrap().ran.last(), Some(&(claude, args(&["mcp", "remove", "figcraft", "--scope", "local"]))));
}

#[test]
fn an_entry_already_there_is_offered_for_replace_never_overwritten() {
    let theirs = ok("scrim:\n  Scope: Local config (private to you in this project)\n  Type: http\n  URL: http://127.0.0.1:4848/mcp\n");
    let (mut h, script) = harness(true, vec![theirs, missing(), ok("Removed"), ok("Added")]);
    click(&mut h, "Connect Claude Code");
    h.app.ui.connect.confirm_run = false;
    h.app.run("mcp.run-connect", json!({}));
    let shapes = h.frames(2);
    let runs: Vec<_> = crate::live_tests::texts(&shapes).into_iter().filter(|(t, _, _)| t == "Run").map(|(_, p, s)| p + s / 2.0).collect();
    h.click(runs.iter().copied().max_by(|a, b| a.y.total_cmp(&b.y)).unwrap());
    h.frames(6);
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Claude Code already has a scrim entry pointing to http://127.0.0.1:4848/mcp. Replace it with this window's address?"));
    assert!(writes(&script).is_empty(), "only the checks so far");
    click(&mut h, "Replace");
    h.frames(6);
    assert_eq!(writes(&script), [args(&["mcp", "remove", "scrim", "--scope", "local"]), add_args()]);
}

#[test]
fn without_claude_only_copy_is_offered() {
    let (mut h, script) = harness(false, vec![]);
    click(&mut h, "Connect Claude Code");
    let shapes = h.frames(2);
    assert!(has_text(
        &shapes,
        "Claude Code's claude command wasn't found on this computer. Copy the command and run it in a terminal where Claude Code is installed."
    ));
    assert!(!has_text(&shapes, "Run"), "no Run without claude");
    click(&mut h, "Copy");
    assert_eq!(
        script.lock().unwrap().copied.last().map(String::as_str),
        Some(format!("claude mcp add --transport http --scope user scrim {URL}").as_str())
    );
}

#[test]
fn an_agent_cannot_make_scrim_run_a_program() {
    let (mut h, script) = harness(true, vec![missing(), missing(), ok("Added")]);
    let ctx = h.ctx.clone();
    for cmd in ["mcp.connect-claude-code", "mcp.run-connect", "mcp.remove-old-entry"] {
        crate::control::handle(&mut h.app, &ctx, "engine.execute", &json!({"command": cmd})).unwrap();
    }
    h.frames(6);
    assert!(!h.app.ui.connect.confirm_run, "an agent's call opens no confirmation");
    assert!(writes(&script).is_empty(), "and nothing that changes Claude Code runs");
}

/// Addendum 2: the AI tab no longer tells people to start Scrim with --control or to use scrim mcp.
#[test]
fn the_ai_tab_shows_no_control_channel_or_scrim_mcp_copy() {
    for (name, src) in [("ai_tab.rs", include_str!("ai_tab.rs")), ("connect_flow.rs", include_str!("connect_flow.rs"))] {
        for literal in string_literals(src) {
            for bad in ["--control", "scrim mcp"] {
                assert!(!literal.contains(bad), "{name} shows {bad:?}: {literal:?}");
            }
        }
    }
}

/// The string literals in Rust source (outside comments).
fn string_literals(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '"' {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            if let Some(n) = chars.next() {
                                s.push(n);
                            }
                        }
                        '"' => break,
                        c => s.push(c),
                    }
                }
                out.push(s);
            }
        }
    }
    out
}
