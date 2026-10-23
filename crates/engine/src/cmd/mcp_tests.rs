//! The MCP endpoint's commands: status, connect, copy, restart, authentication and the root folder.

use serde_json::json;

use super::{CONFIG_REQUEST, CONNECT_REQUEST, RESTART_REQUEST};
use crate::Session;
use crate::settings::DEFAULT_MCP_PORT;

fn running(port: u16) -> Session {
    let mut s = Session::new();
    s.agents.mcp.port = Some(port);
    s
}

#[test]
fn status_says_stopped_not_connected_or_connected() {
    let mut s = Session::new();
    assert_eq!(s.prefs.settings.mcp.port, DEFAULT_MCP_PORT);
    assert_eq!(s.execute("mcp.status", &json!({})).unwrap()["state"], "stopped");
    s.agents.mcp.port = Some(7601);
    let st = s.execute("mcp.status", &json!({"now": 1000})).unwrap();
    assert_eq!((&st["state"], &st["url"]), (&json!("not connected"), &json!("http://127.0.0.1:7601/mcp")));
    s.agents.mcp.clients.push(("claude-code".into(), "2.1".into(), 1000));
    assert_eq!(s.execute("mcp.status", &json!({"now": 1500})).unwrap()["state"], "connected");
    assert_eq!(s.execute("mcp.status", &json!({"now": 1000 + 11 * 60})).unwrap()["state"], "not connected", "ten minutes later");
}

#[test]
fn connect_gives_the_exact_command_and_arguments() {
    let mut s = running(7601);
    s.ui_requests.clear();
    let r = s.execute("mcp.connect-claude-code", &json!({})).unwrap();
    assert_eq!(r["command"], "claude mcp add --transport http --scope user scrim http://127.0.0.1:7601/mcp");
    assert_eq!(r["args"], json!(["mcp", "add", "--transport", "http", "--scope", "user", "scrim", "http://127.0.0.1:7601/mcp"]));
    assert_eq!(s.ui_requests[0].kind, CONNECT_REQUEST);
    let e = Session::new().execute("mcp.connect-claude-code", &json!({})).unwrap_err().to_string();
    assert!(e.contains("isn't running"), "{e}");
}

#[test]
fn authentication_makes_a_token_once_and_the_command_carries_it() {
    let mut s = running(7601);
    s.ui_requests.clear();
    let r = s.execute("mcp.toggle-auth", &json!({})).unwrap();
    assert_eq!(r["requireAuth"], true);
    assert_eq!(s.ui_requests[0].kind, CONFIG_REQUEST);
    let token = s.prefs.settings.mcp.token.clone();
    assert_eq!(token.len(), 64);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    // Shown masked; the arguments and the copied command carry it.
    assert!(r["command"].as_str().unwrap().ends_with("--header \"Authorization: Bearer <token>\""));
    let args = s.execute("mcp.connect-claude-code", &json!({})).unwrap()["args"].clone();
    assert_eq!(args.as_array().unwrap().last().unwrap(), &json!(format!("Authorization: Bearer {token}")));
    s.ui_requests.clear();
    s.execute("mcp.copy-command", &json!({})).unwrap();
    assert!(s.ui_requests[0].data["text"].as_str().unwrap().contains(&token));
    // Off and on again: the same token.
    s.execute("mcp.toggle-auth", &json!({"on": false})).unwrap();
    s.execute("mcp.toggle-auth", &json!({"on": true})).unwrap();
    assert_eq!(s.prefs.settings.mcp.token, token);
    // Two sessions make different tokens.
    let mut other = running(7601);
    other.execute("mcp.toggle-auth", &json!({})).unwrap();
    assert_ne!(other.prefs.settings.mcp.token, token);
}

#[test]
fn the_root_folder_must_be_an_absolute_folder_and_restart_is_asked_of_the_app() {
    let mut s = running(7601);
    let dir = std::env::temp_dir();
    let dir = dir.to_string_lossy();
    assert_eq!(s.execute("mcp.set-root-dir", &json!({"path": dir})).unwrap()["rootDir"], json!(dir));
    for (p, why) in [(json!({"path": "relative/dir"}), "absolute"), (json!({"path": "/no/such/scrim/folder"}), "no such folder")] {
        assert!(s.execute("mcp.set-root-dir", &p).unwrap_err().to_string().contains(why), "{p}");
    }
    assert_eq!(s.execute("mcp.set-root-dir", &json!({})).unwrap()["rootDir"], json!(null), "back to the default");
    s.ui_requests.clear();
    assert_eq!(s.execute("mcp.restart-server", &json!({})).unwrap(), json!({"requested": true}));
    assert_eq!(s.ui_requests[0].kind, RESTART_REQUEST);
}

#[test]
fn the_settings_are_kept_and_bad_values_fall_back() {
    let mut s = running(7601);
    s.execute("mcp.toggle-auth", &json!({})).unwrap();
    let saved = serde_json::to_value(&s.prefs).unwrap();
    assert_eq!(saved["mcp"]["require_auth"], true);
    let back: crate::Prefs = serde_json::from_value(saved).unwrap();
    assert_eq!(back.settings.mcp, s.prefs.settings.mcp);
    let odd: crate::Prefs = serde_json::from_value(json!({"mcp": {"port": 80, "require_auth": true, "token": "short"}})).unwrap();
    let fixed = odd.settings.sanitized();
    assert_eq!(fixed.mcp.port, DEFAULT_MCP_PORT, "a privileged port falls back");
    assert_eq!(fixed.mcp.token.len(), 64, "a short token is replaced");
    let missing: crate::Prefs = serde_json::from_value(json!({"theme": "dark"})).unwrap();
    assert_eq!(missing.settings.mcp, Default::default());
}

/// C1.2: the default is 7680. A port the first default (7600) or its fallback kept moves to it on
/// load, unless the person chose it (`mcp.set-port`); a later file keeps whatever it says.
#[test]
fn the_old_default_port_moves_to_7680_unless_it_was_chosen() {
    use crate::settings::{SETTINGS_VERSION, Settings};
    assert_eq!(DEFAULT_MCP_PORT, 7680);
    let read = |v: serde_json::Value| -> Settings { serde_json::from_value::<crate::Prefs>(v).unwrap().settings.sanitized() };
    assert_eq!(read(json!({"settings_version": 1, "mcp": {"port": 7600}})).mcp.port, 7680, "the old default");
    assert_eq!(read(json!({"settings_version": 1, "mcp": {"port": 7601}})).mcp.port, 7680, "its fallback");
    assert_eq!(read(json!({"mcp": {"port": 7605}})).mcp.port, 7680, "no version: a Stage C file before versions");
    assert_eq!(read(json!({"settings_version": 1, "mcp": {"port": 7600, "port_chosen": true}})).mcp.port, 7600, "chosen");
    assert_eq!(read(json!({"settings_version": 1, "mcp": {"port": 9000}})).mcp.port, 9000, "not the old default's");
    assert_eq!(read(json!({"settings_version": 2, "mcp": {"port": 7600}})).mcp.port, 7600, "already version 2");
    assert_eq!(read(json!({"settings_version": 1})).settings_version, SETTINGS_VERSION);

    let mut s = Session::new();
    s.ui_requests.clear();
    assert_eq!(s.execute("mcp.set-port", &json!({"port": 7600})).unwrap()["port"], 7600);
    assert!(s.prefs.settings.mcp.port_chosen);
    assert_eq!(s.ui_requests[0].kind, RESTART_REQUEST);
    assert!(s.execute("mcp.set-port", &json!({"port": 80})).is_err());
    s.set_author(crate::agents::Author::agent("mcp"));
    assert!(s.execute("mcp.set-port", &json!({"port": 7700})).unwrap_err().to_string().contains("only the person changes Scrim's saved settings"));
}

#[test]
fn reregister_only_asks() {
    let mut s = running(7681);
    s.ui_requests.clear();
    assert_eq!(s.execute("mcp.reregister", &json!({})).unwrap()["asked"], true);
    assert_eq!(s.ui_requests[0].kind, super::REREGISTER_DIALOG);
}
