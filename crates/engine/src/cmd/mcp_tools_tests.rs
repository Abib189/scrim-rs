//! Tool access: turning tools and groups off and on, the counts, Restore defaults, and agents never
//! widening their own access (tools back on, authentication off, another root folder).

use serde_json::json;

use super::TOOLS;
use crate::Session;
use crate::agents::Author;
use crate::cmd::mcp::CONFIG_REQUEST;

#[test]
fn tools_and_groups_turn_off_and_on_with_counts() {
    let mut s = Session::new();
    let st = s.execute("mcp.tools", &json!({})).unwrap();
    assert_eq!((st["enabled"].as_u64(), st["total"].as_u64()), (Some(TOOLS.len() as u64), Some(TOOLS.len() as u64)));
    let reads = TOOLS.iter().filter(|t| t.2).count() as u64;
    assert_eq!(st["groups"]["read"]["total"].as_u64(), Some(reads));

    s.ui_requests.clear();
    let st = s.execute("mcp.set-tool", &json!({"tool": "export", "on": false})).unwrap();
    assert_eq!(s.prefs.settings.mcp.disabled_tools, ["export"]);
    assert_eq!(st["enabled"].as_u64(), Some(TOOLS.len() as u64 - 1));
    assert_eq!(s.ui_requests[0].kind, CONFIG_REQUEST, "the app applies it at once");

    s.execute("mcp.set-tool-group", &json!({"group": "write", "on": false})).unwrap();
    let st = s.execute("mcp.tools", &json!({})).unwrap();
    assert_eq!(st["groups"]["write"]["enabled"].as_u64(), Some(0));
    assert_eq!(st["groups"]["read"]["enabled"].as_u64(), Some(reads - 1));
    s.execute("mcp.set-tool-group", &json!({"group": "write", "on": true})).unwrap();
    assert_eq!(s.prefs.settings.mcp.disabled_tools, ["export"]);

    s.execute("mcp.restore-tools", &json!({})).unwrap();
    assert!(s.prefs.settings.mcp.disabled_tools.is_empty());
    assert!(s.execute("mcp.set-tool", &json!({"tool": "rm_rf", "on": false})).is_err(), "only known tools");
    assert!(s.execute("mcp.set-tool-group", &json!({"group": "all", "on": false})).is_err());
}

#[test]
fn an_agent_can_narrow_its_access_but_never_widen_it() {
    let mut s = Session::new();
    s.execute("mcp.set-tool", &json!({"tool": "save_file", "on": false})).unwrap();
    s.execute("mcp.toggle-auth", &json!({"on": true})).unwrap();
    s.set_author(Author::agent("mcp (http)"));
    // Narrowing: fine.
    s.execute("mcp.set-tool", &json!({"tool": "open_file", "on": false})).unwrap();
    // Widening: refused, and nothing changes.
    for (cmd, p) in [
        ("mcp.set-tool", json!({"tool": "save_file", "on": true})),
        ("mcp.set-tool-group", json!({"group": "write", "on": true})),
        ("mcp.restore-tools", json!({})),
        ("mcp.toggle-auth", json!({"on": false})),
        ("mcp.set-root-dir", json!({"path": "/"})),
    ] {
        let e = s.execute(cmd, &p).unwrap_err().to_string();
        assert!(e.contains("an agent can't"), "{cmd}: {e}");
    }
    assert_eq!(s.prefs.settings.mcp.disabled_tools, ["open_file", "save_file"]);
    assert!(s.prefs.settings.mcp.require_auth && s.prefs.settings.mcp.root_dir.is_none());
    // Turning on a tool that is already on widens nothing.
    s.execute("mcp.set-tool", &json!({"tool": "export", "on": true})).unwrap();
    // The person can.
    s.set_author(Author::Person);
    s.execute("mcp.restore-tools", &json!({})).unwrap();
    assert!(s.prefs.settings.mcp.disabled_tools.is_empty());
}

#[test]
fn copy_address_copies_the_url() {
    let mut s = Session::new();
    assert!(s.execute("mcp.copy-address", &json!({})).is_err(), "stopped");
    s.agents.mcp.port = Some(7601);
    s.ui_requests.clear();
    assert_eq!(s.execute("mcp.copy-address", &json!({})).unwrap()["url"], "http://127.0.0.1:7601/mcp");
    assert_eq!(s.ui_requests[0].data["text"], "http://127.0.0.1:7601/mcp");
}

#[test]
fn a_server_without_the_app_reads_the_saved_tool_access() {
    let dir = std::env::temp_dir().join(format!("scrim-saved-mcp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    assert!(crate::settings::saved_mcp(&dir).disabled_tools.is_empty(), "no file: the defaults");
    let mut s = Session::new();
    s.execute("mcp.set-tool", &json!({"tool": "pointer", "on": false})).unwrap();
    std::fs::write(dir.join("state.json"), json!({"prefs": s.prefs}).to_string()).unwrap();
    assert_eq!(crate::settings::saved_mcp(&dir).disabled_tools, ["pointer"]);
    std::fs::write(dir.join("state.json"), "not json").unwrap();
    assert!(crate::settings::saved_mcp(&dir).disabled_tools.is_empty(), "unreadable: the defaults");
    let _ = std::fs::remove_dir_all(&dir);
}
