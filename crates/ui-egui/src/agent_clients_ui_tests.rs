//! Stage J2/J3 in the window (egui headless). Every test points the clients' configurations at its
//! own scratch home folder (`McpEndpoint::client_env`), and the programs are the test's: no real
//! configuration is read or written, and nothing real is run.
//! - **The list:** clients found on this computer first, the rest under More, each with its status
//!   and how it was verified.
//! - **A CLI client** runs its own command (an argument list) only after one confirmation; a
//!   failure shows its real error; Re-register removes then adds.
//! - **A file client** is written after one confirmation: other servers and comments kept, a backup.
//! - **An invalid file** is refused and left alone; an old entry is removed after its confirmation.
//! - **A token in plain text** is warned about and confirmed as such.
//! - **Copy only** when Scrim can't (a manual client, no way to pass the token); any other agent.
//! - **An agent** can't make Scrim run or write anything.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::{Confirm, How};
use crate::live_tests::{Harness, has_text, text_center};
use crate::{ProgramOutput, ProgramRun};
use figcraft_engine::cmd::agent_clients::{PathEnv, report};

const URL: &str = "http://127.0.0.1:7601/mcp";

#[derive(Default)]
struct Script {
    /// Programs found, by name: their paths.
    programs: Vec<(&'static str, String)>,
    /// Every program run, with its arguments.
    ran: Vec<(String, Vec<String>)>,
    /// Make the runs fail with this error.
    fail: Option<String>,
    copied: Vec<String>,
    home: PathBuf,
}

struct Platform(Arc<Mutex<Script>>);

impl crate::Services for Platform {
    fn find_program(&self, name: &str) -> Option<String> {
        self.0.lock().unwrap().programs.iter().find(|(n, _)| *n == name).map(|(_, p)| p.clone())
    }
    fn run_program(&mut self, program: &str, args: &[String]) -> ProgramRun {
        let mut s = self.0.lock().unwrap();
        s.ran.push((program.to_string(), args.to_vec()));
        let (tx, rx) = std::sync::mpsc::channel();
        let out = match &s.fail {
            Some(e) => ProgramOutput { code: Some(1), stdout: String::new(), stderr: e.clone() },
            None => ProgramOutput { code: Some(0), stdout: "Added".into(), stderr: String::new() },
        };
        let _ = tx.send(Ok(out));
        rx
    }
    fn clipboard_set_text(&mut self, s: &str) {
        self.0.lock().unwrap().copied.push(s.to_string());
    }
    fn agent_client_env(&self) -> Option<PathEnv> {
        Some(PathEnv { home: Some(self.0.lock().unwrap().home.clone()), ..PathEnv::default() })
    }
}

/// A scratch home folder for one test.
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("scrim-agent-clients-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn put(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The AI tab with the endpoint on 7601, the clients' files under `home`, and `programs` found.
fn harness(home: &Path, programs: &[(&'static str, &str)]) -> (Harness, Arc<Mutex<Script>>) {
    let (mut h, _) = Harness::new();
    let script = Arc::new(Mutex::new(Script {
        programs: programs.iter().map(|(n, p)| (*n, p.to_string())).collect(),
        home: home.to_path_buf(),
        ..Default::default()
    }));
    h.app.services = Box::new(Platform(script.clone()));
    h.app.session.agents.mcp.port = Some(7601);
    h.app.session.agents.mcp.client_env = h.app.services.agent_client_env();
    h.app.session.execute("panel.tab.ai", &json!({})).unwrap();
    h.app.handle_ui_requests();
    h.frames(3);
    (h, script)
}

/// Run command `cmd` for `client` as the person does (the panel's buttons run these; the panel is
/// below the test screen's edge).
fn run(h: &mut Harness, cmd: &str, client: &str) {
    h.app.run(cmd, json!({"client": client}));
    h.app.handle_ui_requests();
    h.frames(3);
}

/// Click the button `text` in the dialog titled `title` (the first one below the title).
fn confirm(h: &mut Harness, title: &str, text: &str) {
    let shapes = h.frames(2);
    let head = text_center(&shapes, title);
    let p = crate::live_tests::texts(&shapes)
        .into_iter()
        .filter(|(t, p, _)| t == text && p.y > head.y && (p.x - head.x).abs() < 260.0)
        .map(|(_, p, s)| p + s / 2.0)
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap_or_else(|| panic!("no {text:?} under {title:?}"));
    h.click(p);
    h.frames(4);
}

fn texts_containing(shapes: &[egui::Shape], s: &str) -> Vec<String> {
    crate::live_tests::texts(shapes).into_iter().map(|(t, _, _)| t).filter(|t| t.contains(s)).collect()
}

fn said(h: &Harness) -> Option<Result<String, String>> {
    h.app.ui.connect.others.outcome.as_ref().map(|(_, s, _)| s.clone())
}

fn backups(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.to_string_lossy().contains(".scrim-backup-")).collect()
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn found_clients_come_first_with_their_status_and_the_rest_under_more() {
    let home = scratch("list");
    put(&home.join(".cursor/mcp.json"), &json!({"mcpServers": {"scrim": {"url": URL}}}).to_string());
    let (mut h, _) = harness(&home, &[("codex", "/opt/bin/codex")]);
    assert_eq!(super::found(&mut h.app, 0.0), ["codex", "cursor"], "codex's program, Cursor's folder; nothing else");
    // Settings ▸ MCP: the same section, with room to see it.
    h.app.run("settings.open", json!({"section": "mcp"}));
    h.app.handle_ui_requests();
    let shapes = h.frames(3);
    assert!(has_text(&shapes, "Connect other agents"), "{:?}", texts_containing(&shapes, "agents"));
    assert!(has_text(&shapes, "Codex · Not configured") && has_text(&shapes, "Cursor · Configured"), "{:?}", texts_containing(&shapes, " · "));
    assert!(has_text(&shapes, "From its documentation only · 2026-10-21"), "how it was verified, and when");
    assert!(!has_text(&shapes, "Zed · Not configured"), "the rest wait under More");
    let more = figcraft_engine::cmd::agent_clients::profiles().len() - 4;
    assert!(has_text(&shapes, &format!("More ({more})")));
    assert!(!crate::live_tests::texts(&shapes).iter().any(|(t, _, _)| t.contains(" · Connected")), "a file never says Connected");
    h.app.ui.connect.others.more = true;
    h.frames(3);
    assert!(h.app.ui.connect.others.seen.contains_key("zed"), "listed under More");
    assert!(!h.app.ui.connect.others.seen.contains_key("claude-code"), "Claude Code and Claude Desktop keep their own sections");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_cli_client_runs_its_own_command_only_after_one_confirmation() {
    let home = scratch("cli");
    let (mut h, script) = harness(&home, &[("codex", "/opt/bin/codex")]);
    run(&mut h, "mcp.connect-client", "codex");
    assert_eq!(h.app.ui.connect.others.open.as_deref(), Some("codex"));
    let add = args(&["mcp", "add", "scrim", "--url", URL]);
    let r = report(&h.app.session, "codex").unwrap();
    assert_eq!(super::how(&h.app, &r, Confirm::Write), How::Run("/opt/bin/codex".into(), vec![add.clone()]));
    assert!(script.lock().unwrap().ran.is_empty(), "showing it runs nothing");
    run(&mut h, "mcp.write-client", "codex");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Add Scrim to Codex?") && has_text(&shapes, &format!("codex mcp add scrim --url {URL}")), "the exact command");
    assert!(has_text(&shapes, "Scrim doesn't restart Codex: restart it yourself afterwards."));
    assert!(script.lock().unwrap().ran.is_empty(), "nothing before the answer");
    confirm(&mut h, "Add Scrim to Codex?", "Run");
    h.frames(3);
    assert_eq!(script.lock().unwrap().ran, [("/opt/bin/codex".to_string(), add)], "an argument list");
    assert_eq!(said(&h), Some(Ok("Scrim was added to Codex. Restart Codex to use it.".into())));
    // A failure shows the program's own words.
    script.lock().unwrap().fail = Some("Error: a server named scrim already exists".into());
    run(&mut h, "mcp.write-client", "codex");
    confirm(&mut h, "Add Scrim to Codex?", "Run");
    h.frames(3);
    assert_eq!(said(&h), Some(Err("Error: a server named scrim already exists".into())));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn re_register_removes_then_adds_through_the_cli() {
    let home = scratch("rereg");
    put(&home.join(".codex/config.toml"), "# mine\n[mcp_servers.scrim]\nurl = \"http://127.0.0.1:7600/mcp\"\n");
    let (mut h, script) = harness(&home, &[("codex", "/opt/bin/codex")]);
    run(&mut h, "mcp.connect-client", "codex");
    assert_eq!(report(&h.app.session, "codex").unwrap()["state"], "out of date");
    // Write isn't asked for an out-of-date entry: Re-register is.
    run(&mut h, "mcp.write-client", "codex");
    assert!(h.app.ui.connect.others.confirm.is_none());
    run(&mut h, "mcp.reregister-client", "codex");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Re-register Scrim in Codex?"));
    assert_eq!(texts_containing(&shapes, "which points to http://127.0.0.1:7600/mcp").len(), 1, "says what it replaces");
    confirm(&mut h, "Re-register Scrim in Codex?", "Re-register");
    h.frames(4);
    let ran: Vec<Vec<String>> = script.lock().unwrap().ran.iter().map(|(_, a)| a.clone()).collect();
    assert_eq!(ran, [args(&["mcp", "remove", "scrim"]), args(&["mcp", "add", "scrim", "--url", URL])]);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_file_client_is_written_after_one_confirmation_keeping_the_rest() {
    let home = scratch("file");
    let path = home.join(".cursor/mcp.json");
    let before = "{\n  // my servers\n  \"mcpServers\": {\n    \"github\": {\"command\": \"gh\", \"args\": [\"mcp\"]}\n  }\n}\n";
    put(&path, before);
    let (mut h, _) = harness(&home, &[]);
    run(&mut h, "mcp.connect-client", "cursor");
    let r = report(&h.app.session, "cursor").unwrap();
    assert!(matches!(super::how(&h.app, &r, Confirm::Write), How::Write(p, _) if p == path));
    run(&mut h, "mcp.write-client", "cursor");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Add Scrim to Cursor?") && has_text(&shapes, &path.to_string_lossy()), "the exact file");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before, "nothing before the answer");
    confirm(&mut h, "Add Scrim to Cursor?", "Write");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("// my servers") && text.contains("\"github\": {\"command\": \"gh\", \"args\": [\"mcp\"]}"), "kept as written:\n{text}");
    assert!(text.contains(&format!("\"url\": \"{URL}\"")), "{text}");
    let b = backups(&home.join(".cursor"));
    assert_eq!(b.len(), 1);
    assert_eq!(std::fs::read_to_string(&b[0]).unwrap(), before, "the backup is the file as it was");
    let s = said(&h).unwrap().unwrap();
    assert!(s.starts_with("Scrim was added to Cursor. Restart Cursor to use it. The previous file was kept as mcp.json.scrim-backup-"), "{s}");
    assert_eq!(h.app.ui.connect.others.outcome.as_ref().map(|o| o.2.as_str()), Some("configured"), "read back from the file");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn an_invalid_file_is_left_alone_and_an_old_entry_can_be_removed() {
    let home = scratch("invalid");
    let zed = home.join(".config/zed/settings.json");
    let broken = "{ \"context_servers\": { \"x\": ";
    put(&zed, broken);
    let (mut h, _) = harness(&home, &[]);
    run(&mut h, "mcp.connect-client", "zed");
    assert_eq!(report(&h.app.session, "zed").unwrap()["state"], "unreadable");
    // Asked anyway: no confirmation, no change, no backup.
    run(&mut h, "mcp.write-client", "zed");
    assert!(!has_text(&h.frames(2), "Add Scrim to Zed?"));
    assert!(h.app.ui.connect.others.confirm.is_none());
    assert_eq!(std::fs::read_to_string(&zed).unwrap(), broken);
    assert!(backups(zed.parent().unwrap()).is_empty());
    // An old entry in OpenCode's file: removed after its own confirmation, the rest kept.
    let oc = home.join(".config/opencode/opencode.json");
    put(
        &oc,
        &json!({"mcp": {"figcraft": {"type": "remote", "url": "http://127.0.0.1:4848/mcp"}, "a": {"type": "local", "command": ["x"]}}}).to_string(),
    );
    run(&mut h, "mcp.connect-client", "opencode");
    run(&mut h, "mcp.remove-old-client-entry", "opencode");
    assert!(has_text(&h.frames(2), "Remove the old entry?"));
    confirm(&mut h, "Remove the old entry?", "Remove");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&oc).unwrap()).unwrap();
    assert!(v["mcp"].get("figcraft").is_none() && v["mcp"]["a"]["command"] == json!(["x"]), "{v}");
    assert_eq!(said(&h), Some(Ok("The old entry was removed from OpenCode.".into())));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_token_in_plain_text_is_warned_about_and_an_env_reference_is_used_elsewhere() {
    let home = scratch("token");
    let (mut h, _) = harness(&home, &[]);
    h.app.session.prefs.settings.mcp.require_auth = true;
    h.app.session.prefs.settings.mcp.token = "tok123".into();
    // Zed has no documented reference: the token itself, said first.
    run(&mut h, "mcp.connect-client", "zed");
    run(&mut h, "mcp.write-client", "zed");
    let shapes = h.frames(2);
    assert_eq!(texts_containing(&shapes, "the token itself will be stored in its configuration as plain text").len(), 1, "warned");
    assert!(has_text(&shapes, "Write the token"), "and confirmed as such");
    confirm(&mut h, "Add Scrim to Zed?", "Write the token");
    let text = std::fs::read_to_string(home.join(".config/zed/settings.json")).unwrap();
    assert!(text.contains("Bearer tok123"), "{text}");
    // Cursor reads it from the environment: the file names the variable, never the token.
    run(&mut h, "mcp.connect-client", "cursor");
    run(&mut h, "mcp.write-client", "cursor");
    let shapes = h.frames(2);
    assert!(!has_text(&shapes, "Write the token") && has_text(&shapes, "Add Scrim to Cursor?"));
    confirm(&mut h, "Add Scrim to Cursor?", "Write");
    let text = std::fs::read_to_string(home.join(".cursor/mcp.json")).unwrap();
    assert!(text.contains("${env:SCRIM_MCP_TOKEN}") && !text.contains("tok123"), "{text}");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn copy_only_when_scrim_cant_and_the_card_for_any_other_agent() {
    let home = scratch("copy");
    let (mut h, script) = harness(&home, &[]);
    // Windsurf (Devin Desktop) is manual: its docs give four different paths.
    run(&mut h, "mcp.connect-client", "windsurf");
    let r = report(&h.app.session, "windsurf").unwrap();
    assert_eq!(super::how(&h.app, &r, Confirm::Write), How::Copy);
    run(&mut h, "mcp.write-client", "windsurf");
    assert!(h.app.ui.connect.others.confirm.is_none(), "nothing to confirm");
    run(&mut h, "mcp.copy-client-entry", "windsurf");
    let copied = script.lock().unwrap().copied.clone();
    assert!(copied.iter().any(|c| c.contains("serverUrl") && c.contains(URL)), "{copied:?}");
    // The Devin CLI is installed but documents no header option: with a token, Copy.
    h.app.session.prefs.settings.mcp.require_auth = true;
    h.app.session.prefs.settings.mcp.token = "tok".into();
    script.lock().unwrap().programs.push(("devin", "/opt/bin/devin".into()));
    let r = report(&h.app.session, "devin").unwrap();
    assert_eq!(super::how(&h.app, &r, Confirm::Write), How::Copy);
    h.app.session.prefs.settings.mcp.require_auth = false;
    // Any other agent: the card (scrolled into view in the AI tab), and its Copy.
    h.app.ui.connect.others.any = true;
    h.app.ui.connect.others.open = None;
    h.app.ui.connect.others.more = false;
    let shapes = h.frames(3);
    for part in ["Any MCP client on this computer can use this canvas. Give it one of these:", "Address", "JSON"] {
        assert!(has_text(&shapes, part), "{part:?} in {:?}", texts_containing(&shapes, ""));
    }
    let at = text_center(&shapes, "Connect other agents");
    h.frame(vec![egui::Event::PointerMoved(at)]);
    for _ in 0..8 {
        h.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        }]);
    }
    let shapes = h.frames(3);
    for part in ["TOML", "Command (stdio)", &format!("npx -y mcp-remote@0.14.3 {URL}")] {
        assert!(has_text(&shapes, part), "{part:?} in {:?}", texts_containing(&shapes, ""));
    }
    assert_eq!(texts_containing(&shapes, "can't reach this computer's local address").len(), 1, "why a cloud agent can't");
    script.lock().unwrap().copied.clear();
    h.app.run("mcp.copy-agent-snippet", json!({"format": "toml"}));
    h.app.handle_ui_requests();
    assert_eq!(script.lock().unwrap().copied, [format!("[mcp_servers.scrim]\nurl = \"{URL}\"\n")]);
    assert!(script.lock().unwrap().ran.is_empty(), "nothing was run");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn an_agent_cannot_make_scrim_run_or_write_anything() {
    let home = scratch("agent");
    let (mut h, script) = harness(&home, &[("codex", "/opt/bin/codex")]);
    let ctx = h.ctx.clone();
    for cmd in ["mcp.connect-client", "mcp.write-client", "mcp.reregister-client", "mcp.remove-old-client-entry"] {
        for client in ["cursor", "codex"] {
            let _ = crate::control::handle(&mut h.app, &ctx, "engine.execute", &json!({"command": cmd, "params": {"client": client}}));
        }
    }
    h.frames(6);
    assert!(h.app.ui.connect.others.confirm.is_none(), "an agent's call opens no confirmation");
    assert!(script.lock().unwrap().ran.is_empty(), "runs nothing");
    assert!(!home.join(".cursor").exists(), "and writes nothing");
    let _ = std::fs::remove_dir_all(&home);
}
