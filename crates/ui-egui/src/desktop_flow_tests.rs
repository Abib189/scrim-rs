//! Stage F in the window (egui headless). Every test points Claude Desktop's file at its own
//! scratch folder (`McpEndpoint::desktop_config`): the real configuration is never read or written.
//! - **Connect** shows the file and the exact entry, writes nothing until the one confirmation,
//!   then writes, says to restart Claude Desktop, and reads Configured.
//! - **A file with other servers** keeps them, with a backup made.
//! - **Invalid JSON** is refused and left untouched.
//! - **An out-of-date entry** shows Re-register and is replaced only after its confirmation.
//! - **An old figcraft entry** can be removed.
//! - **Without npx,** Copy only.
//! - **An agent can't make Scrim write,** and the button is in Settings ▸ MCP and the palette too.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use crate::live_tests::{Harness, has_text, text_center};
use figcraft_engine::cmd::claude_desktop::MCP_REMOTE;

const URL: &str = "http://127.0.0.1:7601/mcp";
const NPX: &str = "/Users/me/.nvm/versions/node/v24/bin/npx";

#[derive(Default)]
struct Script {
    npx: bool,
    copied: Vec<String>,
}

struct Platform(Arc<Mutex<Script>>);

impl crate::Services for Platform {
    fn find_program(&self, name: &str) -> Option<String> {
        (name == "npx" && self.0.lock().unwrap().npx).then(|| NPX.to_string())
    }
    fn clipboard_set_text(&mut self, s: &str) {
        self.0.lock().unwrap().copied.push(s.to_string());
    }
}

/// A scratch folder for one test, with Claude Desktop's file in it (written when `text` is given).
fn scratch(name: &str, text: Option<&str>) -> (PathBuf, PathBuf) {
    let d = std::env::temp_dir().join(format!("scrim-desktop-flow-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let path = d.join("claude_desktop_config.json");
    if let Some(t) = text {
        std::fs::write(&path, t).unwrap();
    }
    (d, path)
}

/// The AI tab with the endpoint on 7601, Claude Desktop's file at `path`, npx found (or not).
fn harness(path: &Path, npx: bool) -> (Harness, Arc<Mutex<Script>>) {
    let (mut h, _) = Harness::new();
    let script = Arc::new(Mutex::new(Script { npx, ..Default::default() }));
    h.app.services = Box::new(Platform(script.clone()));
    h.app.session.agents.mcp.port = Some(7601);
    h.app.session.agents.mcp.desktop_config = Some(path.to_path_buf());
    h.app.session.execute("panel.tab.ai", &json!({})).unwrap();
    h.app.handle_ui_requests();
    h.frames(3);
    (h, script)
}

fn click(h: &mut Harness, text: &str) {
    let shapes = h.frames(2);
    h.click(text_center(&shapes, text));
    h.frames(3);
}

/// Click the button `text` in the dialog titled `title` (the first one below the title: the panel
/// has buttons with the same words).
fn confirm(h: &mut Harness, title: &str, text: &str) {
    let shapes = h.frames(2);
    let head = text_center(&shapes, title);
    let p = crate::live_tests::texts(&shapes)
        .into_iter()
        .filter(|(t, p, _)| t == text && p.y > head.y && (p.x - head.x).abs() < 260.0)
        .map(|(_, p, s)| p + s / 2.0)
        .min_by(|a, b| a.y.total_cmp(&b.y));
    h.click(p.unwrap_or_else(|| panic!("no {text:?} under {title:?}")));
    h.frames(4);
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn backups(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.to_string_lossy().contains(".scrim-backup-")).collect()
}

#[test]
fn connect_shows_the_file_and_entry_and_writes_only_after_one_confirmation() {
    let (d, path) = scratch("connect", None);
    let (mut h, _) = harness(&path, true);
    let shapes = h.frames(1);
    assert!(has_text(&shapes, "Not configured") && has_text(&shapes, "Connect Claude Desktop"), "the status and the button");
    click(&mut h, "Connect Claude Desktop");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, &path.to_string_lossy()), "the exact file");
    let entry = crate::live_tests::texts(&shapes).into_iter().find(|(t, _, _)| t.contains(MCP_REMOTE)).map(|(t, _, _)| t).expect("the entry");
    assert!(entry.contains(URL) && entry.contains(NPX) && entry.starts_with("{\n  \"scrim\": {\n    \"command\""), "{entry}");
    assert!(has_text(&shapes, "Write") && has_text(&shapes, "Copy"));
    assert!(!path.exists(), "showing it writes nothing");
    click(&mut h, "Write");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Write to Claude Desktop's configuration?"), "one confirmation");
    assert!(!path.exists(), "nothing before it is answered");
    confirm(&mut h, "Write to Claude Desktop's configuration?", "Write");
    let v = read(&path);
    assert_eq!(v["mcpServers"]["scrim"]["args"], json!(["-y", MCP_REMOTE, URL]));
    // (The panel runs below the test screen's edge: the outcome is read from the flow.)
    let said = h.app.ui.desktop.outcome.clone();
    assert_eq!(
        said,
        Some(Ok("Scrim was added to Claude Desktop. Quit Claude Desktop completely and open it again to use it.".into())),
        "the restart is the person's"
    );
    assert!(has_text(&h.frames(3), "Configured"), "read back from the file");
    assert!(backups(&d).is_empty(), "no file before, nothing to back up");
    let _ = std::fs::remove_dir_all(&d);
}

const THEIRS: &str = "{\n    \"mcpServers\": {\n        \"filesystem\": {\n            \"command\": \"npx\",\n            \"args\": [\"-y\", \"@modelcontextprotocol/server-filesystem\", \"/Users/me/Desktop\"]\n        }\n    },\n    \"globalShortcut\": \"Alt+Space\"\n}\n";

#[test]
fn other_servers_are_kept_and_the_file_is_backed_up() {
    let (d, path) = scratch("keep", Some(THEIRS));
    let (mut h, _) = harness(&path, true);
    click(&mut h, "Connect Claude Desktop");
    click(&mut h, "Write");
    confirm(&mut h, "Write to Claude Desktop's configuration?", "Write");
    let v = read(&path);
    let before: Value = serde_json::from_str(THEIRS).unwrap();
    assert_eq!(v["mcpServers"]["filesystem"], before["mcpServers"]["filesystem"]);
    assert_eq!(v["globalShortcut"], "Alt+Space");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"args\": [\"-y\", \"@modelcontextprotocol/server-filesystem\", \"/Users/me/Desktop\"]"), "kept as written:\n{text}");
    let b = backups(&d);
    assert_eq!(b.len(), 1);
    assert_eq!(std::fs::read_to_string(&b[0]).unwrap(), THEIRS, "the backup is the file as it was");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn invalid_json_is_refused_and_left_untouched() {
    let broken = "{\n  \"mcpServers\": {\n    \"x\": { \"command\": \"y\" \n";
    let (d, path) = scratch("invalid", Some(broken));
    let (mut h, _) = harness(&path, true);
    let shapes = h.frames(1);
    assert!(has_text(&shapes, "Can't read its configuration"));
    click(&mut h, "Connect Claude Desktop");
    let shapes = h.frames(2);
    assert!(!has_text(&shapes, "Write"), "no Write for a file Scrim won't change");
    assert!(crate::live_tests::texts(&shapes).iter().any(|(t, _, _)| t.contains("so Scrim won't change it")), "the error is shown");
    // Asked anyway (an old button, a command): no confirmation, no change, no backup.
    h.app.run("mcp.write-claude-desktop", json!({}));
    h.app.handle_ui_requests();
    let shapes = h.frames(3);
    assert!(!has_text(&shapes, "Write to Claude Desktop's configuration?"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
    assert!(backups(&d).is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_out_of_date_entry_is_replaced_only_after_re_register_is_confirmed() {
    let old = json!({"mcpServers": {"scrim": {"command": "npx", "args": ["-y", MCP_REMOTE, "http://127.0.0.1:7600/mcp"]}}}).to_string();
    let (d, path) = scratch("outdated", Some(&old));
    let (mut h, _) = harness(&path, true);
    let shapes = h.frames(1);
    assert!(has_text(&shapes, "Out of date") && has_text(&shapes, "Registered address is out of date"));
    assert!(has_text(&shapes, &format!("Claude Desktop points to http://127.0.0.1:7600/mcp; this window is at {URL}.")));
    click(&mut h, "Re-register");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Re-register Scrim in Claude Desktop?"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), old, "not before the answer");
    confirm(&mut h, "Re-register Scrim in Claude Desktop?", "Re-register");
    assert_eq!(read(&path)["mcpServers"]["scrim"]["args"], json!(["-y", MCP_REMOTE, URL]));
    assert_eq!(backups(&d).len(), 1);
    assert!(has_text(&h.frames(3), "Configured"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_old_figcraft_entry_can_be_removed() {
    let text = json!({"mcpServers": {"figcraft": {"command": "figcraft-cli", "args": ["mcp"]}, "other": {"command": "x"}}}).to_string();
    let (d, path) = scratch("old", Some(&text));
    let (mut h, _) = harness(&path, true);
    click(&mut h, "Connect Claude Desktop");
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Claude Desktop still has the old figcraft entry from before the rename."));
    // Its button is below the test screen's edge: the command it runs.
    h.app.run("mcp.remove-old-desktop-entry", json!({}));
    h.app.handle_ui_requests();
    assert!(has_text(&h.frames(2), "Remove the old entry?"));
    confirm(&mut h, "Remove the old entry?", "Remove");
    let v = read(&path);
    assert!(v["mcpServers"].get("figcraft").is_none() && v["mcpServers"]["other"]["command"] == "x");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn without_npx_it_says_so_and_offers_copy_only() {
    let (d, path) = scratch("nonpx", None);
    let (mut h, script) = harness(&path, false);
    click(&mut h, "Connect Claude Desktop");
    let shapes = h.frames(2);
    assert!(!has_text(&shapes, "Write"));
    assert!(crate::live_tests::texts(&shapes).iter().any(|(t, _, _)| t.starts_with("Node.js's npx wasn't found")));
    click(&mut h, "Copy");
    let copied = script.lock().unwrap().copied.clone();
    assert!(copied.iter().any(|c| c.contains(MCP_REMOTE) && c.contains(URL)), "{copied:?}");
    assert!(!path.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_agent_cannot_make_scrim_write_claude_desktops_file() {
    let (d, path) = scratch("agent", None);
    let (mut h, _) = harness(&path, true);
    let ctx = h.ctx.clone();
    for cmd in ["mcp.connect-claude-desktop", "mcp.write-claude-desktop", "mcp.reregister-claude-desktop", "mcp.remove-old-desktop-entry"] {
        crate::control::handle(&mut h.app, &ctx, "engine.execute", &json!({"command": cmd})).unwrap();
    }
    let shapes = h.frames(6);
    assert!(!has_text(&shapes, "Write to Claude Desktop's configuration?"), "an agent's call opens no confirmation");
    assert!(!path.exists(), "and nothing is written");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn settings_mcp_and_the_palette_offer_it_too() {
    let (d, path) = scratch("settings", None);
    let (mut h, _) = harness(&path, true);
    h.app.run("settings.open", json!({"section": "mcp"}));
    h.app.handle_ui_requests();
    let shapes = h.frames(3);
    assert!(texts_with(&shapes, "Connect Claude Desktop") >= 1 && has_text(&shapes, "Connect Claude Code"));
    let rows = crate::menus::palette_sections(&h.app, "Claude Desktop", crate::menus::PaletteTab::All);
    assert!(rows.iter().flat_map(|(_, r)| r).any(|r| r.label == "Connect Claude Desktop"), "the Actions palette lists it");
    let _ = std::fs::remove_dir_all(&d);
}

fn texts_with(shapes: &[egui::Shape], s: &str) -> usize {
    crate::live_tests::texts(shapes).iter().filter(|(t, _, _)| t == s).count()
}
