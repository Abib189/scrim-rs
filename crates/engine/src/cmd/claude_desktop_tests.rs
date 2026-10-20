//! Stage F: Claude Desktop's configuration, on temporary folders only (the real file is never read
//! or written: `McpEndpoint::desktop_config` points every command test at a scratch file).
//! - **The path** on macOS, Windows and Linux.
//! - **The entry,** with and without a token.
//! - **The status:** missing, empty, invalid, the same, out of date, old entries.
//! - **The safe write:**
//!   - an invalid file is refused and left untouched;
//!   - other servers and keys keep their meaning and their text;
//!   - a backup is written first;
//!   - nothing temporary is left behind.
//! - **Only the person can write:** the commands only ask.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::*;

const URL: &str = "http://127.0.0.1:7680/mcp";

fn want() -> Value {
    entry(URL, None, npx_command(Os::Mac, Some("/Users/a/.nvm/versions/node/v24/bin/npx")))
}

/// A fresh scratch folder for one test.
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("scrim-claude-desktop-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn files(d: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

#[test]
fn the_path_on_each_platform() {
    let env = Env { home: Some(PathBuf::from("/home/ana")), appdata: Some(PathBuf::from(r"C:\Users\ana\AppData\Roaming")), xdg_config_home: None };
    assert_eq!(config_path(Os::Mac, &env), Some(PathBuf::from("/home/ana/Library/Application Support/Claude/claude_desktop_config.json")));
    assert_eq!(
        config_path(Os::Windows, &env),
        Some(PathBuf::from(r"C:\Users\ana\AppData\Roaming").join("Claude").join("claude_desktop_config.json"))
    );
    assert_eq!(config_path(Os::Linux, &env), Some(PathBuf::from("/home/ana/.config/Claude/claude_desktop_config.json")));
    let xdg = Env { xdg_config_home: Some(PathBuf::from("/xdg")), ..env.clone() };
    assert_eq!(config_path(Os::Linux, &xdg), Some(PathBuf::from("/xdg/Claude/claude_desktop_config.json")));
    // Without what each needs, or elsewhere: no path (Copy only).
    assert_eq!(config_path(Os::Windows, &Env { appdata: None, ..env.clone() }), None);
    assert_eq!(config_path(Os::Mac, &Env::default()), None);
    assert_eq!(config_path(Os::Other, &env), None);
    assert!(documented(Os::Mac) && documented(Os::Windows) && !documented(Os::Linux));
}

#[test]
fn the_entry_runs_the_pinned_bridge_to_the_live_address() {
    let e = want();
    assert_eq!(e["command"], "/Users/a/.nvm/versions/node/v24/bin/npx");
    assert_eq!(e["args"], json!(["-y", MCP_REMOTE, URL]));
    assert_eq!(e["env"]["PATH"], "/Users/a/.nvm/versions/node/v24/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin");
    // Windows names npx, as Anthropic's example does; without npx found, "npx".
    assert_eq!(entry(URL, None, npx_command(Os::Windows, Some(r"C:\node\npx.cmd"))), json!({"command": "npx", "args": ["-y", MCP_REMOTE, URL]}));
    assert_eq!(npx_command(Os::Mac, None), ("npx".to_string(), None));
    // A token travels in the environment, the header without spaces.
    let t = entry("http://127.0.0.1:7681/mcp", Some("abc"), npx_command(Os::Windows, None));
    assert_eq!(t["args"], json!(["-y", MCP_REMOTE, "http://127.0.0.1:7681/mcp", "--header", "Authorization:${SCRIM_MCP_AUTH}"]));
    assert_eq!(t["env"][TOKEN_VAR], "Bearer abc");
}

#[test]
fn the_status_is_read_from_the_file() {
    let w = want();
    let not = Found { state: State::NotConfigured, old: vec![] };
    assert_eq!(inspect(None, &w), not, "missing");
    assert_eq!(inspect(Some("  \n"), &w), not, "empty");
    assert_eq!(inspect(Some(r#"{"mcpServers": {"other": {"command": "x"}}}"#), &w), not);
    assert!(matches!(inspect(Some("{ nope"), &w).state, State::Unreadable { .. }));
    assert!(matches!(inspect(Some("[1, 2]"), &w).state, State::Unreadable { .. }));
    // Scrim's entry for this address is Configured, wherever npx lives.
    let mut same = w.clone();
    same["command"] = json!("/opt/homebrew/bin/npx");
    let text = json!({"mcpServers": {"scrim": same}}).to_string();
    assert_eq!(inspect(Some(&text), &w).state, State::Configured);
    // Another port, an older pin, or the old stdio form: out of date, with where it points.
    let other = json!({"mcpServers": {"scrim": {"command": "npx", "args": ["-y", MCP_REMOTE, "http://127.0.0.1:7600/mcp"]}}}).to_string();
    assert_eq!(inspect(Some(&other), &w).state, State::OutOfDate { target: "http://127.0.0.1:7600/mcp".into() });
    let pin = json!({"mcpServers": {"scrim": {"command": "npx", "args": ["-y", "mcp-remote@0.1.0", URL]}}}).to_string();
    assert_eq!(inspect(Some(&pin), &w).state, State::OutOfDate { target: URL.into() });
    let stdio = json!({"mcpServers": {"scrim": {"command": "/usr/local/bin/scrim", "args": ["mcp"]}}}).to_string();
    assert_eq!(inspect(Some(&stdio), &w).state, State::OutOfDate { target: "/usr/local/bin/scrim mcp".into() });
    // Old entries from before the rename.
    let old = json!({"mcpServers": {"figcraft": {"command": "figcraft-cli", "args": ["mcp"]}, "figcraft-app": {}}}).to_string();
    assert_eq!(inspect(Some(&old), &w).old, vec!["figcraft".to_string(), "figcraft-app".to_string()]);
}

/// A real-looking file: other servers, other keys, four-space indents, keys in no sorted order.
const THEIRS: &str = r#"{
    "mcpServers": {
        "zeta": {
            "command": "uvx",
            "args": ["zeta-server", "--flag"]
        },
        "filesystem": {
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/ana/Desktop"],
            "env": {"A": "1"}
        }
    },
    "globalShortcut": "Ctrl+Space",
    "anotherKey": [1, 2, 3]
}
"#;

#[test]
fn a_change_keeps_everything_else_in_meaning_and_in_text() {
    let set = Change::Set(KEY.into(), want());
    let out = changed(Some(THEIRS), &set).unwrap();
    let (a, b): (Value, Value) = (serde_json::from_str(THEIRS).unwrap(), serde_json::from_str(&out).unwrap());
    // Meaning: the same file plus scrim.
    let mut expect = a.clone();
    expect["mcpServers"][KEY] = want();
    assert_eq!(b, expect);
    // Text: the other servers and keys exactly as they were, the indentation theirs, the order theirs.
    for part in [
        r#""zeta": {
            "command": "uvx",
            "args": ["zeta-server", "--flag"]
        }"#,
        r#""globalShortcut": "Ctrl+Space",
    "anotherKey": [1, 2, 3]"#,
    ] {
        assert!(out.contains(part), "kept as written:\n{part}\nin\n{out}");
    }
    assert!(out.find("\"zeta\"").unwrap() < out.find("\"filesystem\"").unwrap(), "key order kept");
    assert!(out.contains("\n        \"scrim\": {\n            \"command\""), "the new entry in the file's own indent:\n{out}");
    // Writing it again changes nothing; replacing it keeps the rest.
    assert_eq!(changed(Some(&out), &set).unwrap(), out);
    let moved = entry("http://127.0.0.1:7681/mcp", None, npx_command(Os::Windows, None));
    let out2 = changed(Some(&out), &Change::Set(KEY.into(), moved.clone())).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&out2).unwrap()["mcpServers"][KEY], moved);
    assert!(out2.contains(r#""globalShortcut": "Ctrl+Space""#));
}

#[test]
fn missing_empty_and_odd_files() {
    let set = Change::Set(KEY.into(), want());
    for text in [None, Some(""), Some("\n  \n")] {
        let out = changed(text, &set).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), json!({"mcpServers": {KEY: want()}}), "{text:?}");
    }
    // No mcpServers yet, an empty one, a one-line file: all keep their other keys.
    for text in [r#"{"theme": "dark"}"#, "{\n  \"mcpServers\": {}\n}", r#"{"mcpServers":{"a":{"command":"x"}},"b":1}"#] {
        let out = changed(Some(text), &set).unwrap();
        let mut expect: Value = serde_json::from_str(text).unwrap();
        expect["mcpServers"][KEY] = want();
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), expect, "{text}");
    }
    // Refused: not JSON, not an object, an mcpServers that isn't an object.
    for text in ["{ \"mcpServers\": ", "[]", r#"{"mcpServers": [1]}"#, "nul"] {
        assert!(changed(Some(text), &set).is_err(), "{text}");
    }
}

#[test]
fn removing_an_old_entry_wherever_it_is() {
    let base = json!({"a": {"command": "a"}, "figcraft": {"command": "f"}, "z": {"command": "z"}});
    for keys in [vec!["figcraft", "a", "z"], vec!["a", "figcraft", "z"], vec!["a", "z", "figcraft"], vec!["figcraft"]] {
        let members: Vec<String> = keys.iter().map(|k| format!("    \"{k}\": {}", base[*k])).collect();
        let text = format!("{{\n  \"mcpServers\": {{\n{}\n  }},\n  \"keep\": true\n}}\n", members.join(",\n"));
        let out = changed(Some(&text), &Change::Remove("figcraft".into())).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["mcpServers"].get("figcraft").is_none() && v["keep"] == true, "{keys:?}:\n{out}");
        assert_eq!(v["mcpServers"].as_object().unwrap().len(), keys.len() - 1);
        assert!(out.contains("\"keep\": true"), "the rest as written");
    }
}

#[test]
fn the_write_backs_up_renames_and_never_touches_an_invalid_file() {
    let d = scratch("write");
    let path = d.join("Claude").join("claude_desktop_config.json");
    let set = Change::Set(KEY.into(), want());
    // Missing (and its folder): created, nothing to back up.
    let w = apply(&path, &set, 1_792_506_218).unwrap();
    assert_eq!(w.backup, None);
    assert_eq!(inspect(std::fs::read_to_string(&path).ok().as_deref(), &want()).state, State::Configured);
    // An existing file: backed up first, byte for byte, then replaced.
    std::fs::write(&path, THEIRS).unwrap();
    let w = apply(&path, &set, 1_792_506_218).unwrap();
    let backup = w.backup.unwrap();
    assert_eq!(backup.file_name().unwrap(), "claude_desktop_config.json.scrim-backup-20261020-142338");
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), THEIRS);
    // A second backup in the same second gets its own name.
    let w2 = apply(&path, &set, 1_792_506_218).unwrap();
    assert_eq!(w2.backup.unwrap().file_name().unwrap(), "claude_desktop_config.json.scrim-backup-20261020-142338-2");
    // Nothing temporary is left: the new text went in by rename.
    assert!(files(path.parent().unwrap()).iter().all(|f| !f.ends_with(".tmp")), "{:?}", files(path.parent().unwrap()));
    // Invalid JSON: refused, the file and the folder exactly as they were.
    let broken = "{\n  \"mcpServers\": { \"x\": \n";
    std::fs::write(&path, broken).unwrap();
    let before = files(path.parent().unwrap());
    let err = apply(&path, &set, 1_792_600_000).unwrap_err();
    assert!(err.contains("isn't valid JSON"), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
    assert_eq!(files(path.parent().unwrap()), before, "no backup, no temporary file");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn backup_stamps_are_utc() {
    assert_eq!(stamp(0), "19700101-000000");
    assert_eq!(stamp(1_792_506_218), "20261020-142338");
    assert_eq!(stamp(951_782_400), "20000229-000000");
}

#[test]
fn the_commands_report_and_only_ask() {
    let d = scratch("commands");
    let path = d.join("claude_desktop_config.json");
    let mut s = Session::new();
    s.agents.mcp.desktop_config = Some(path.clone());
    // The endpoint stopped: say so.
    assert!(s.execute("mcp.connect-claude-desktop", &json!({})).unwrap_err().to_string().contains("isn't running"));
    s.agents.mcp.port = Some(7680);
    let r = s.execute("mcp.connect-claude-desktop", &json!({"npx": "/usr/local/bin/npx"})).unwrap();
    assert_eq!(r["path"], path.to_string_lossy().as_ref());
    assert_eq!(r["state"], "not configured");
    assert_eq!(r["entry"][KEY]["args"], json!(["-y", MCP_REMOTE, URL]));
    assert!(s.ui_requests.iter().any(|q| q.kind == CONNECT_REQUEST));
    // Writing, re-registering and removing only ask: the file isn't there afterwards.
    for (cmd, dialog) in [
        ("mcp.write-claude-desktop", WRITE_DIALOG),
        ("mcp.reregister-claude-desktop", REREGISTER_DIALOG),
        ("mcp.remove-old-desktop-entry", REMOVE_OLD_DIALOG),
    ] {
        s.ui_requests.clear();
        s.execute(cmd, &json!({})).unwrap();
        assert!(s.ui_requests.iter().any(|q| q.kind == dialog), "{cmd}");
        assert!(dialog.starts_with("dialog."), "{dialog}: a dialog, which an agent's command never opens");
    }
    assert!(!path.exists(), "nothing was written");
    // The status reads the file: after a write, Configured; a different port, out of date.
    apply(&path, &Change::Set(KEY.into(), r["entry"][KEY].clone()), 0).unwrap();
    assert_eq!(s.execute("mcp.claude-desktop-status", &json!({"npx": "/usr/local/bin/npx"})).unwrap()["state"], "configured");
    s.agents.mcp.port = Some(7681);
    let st = s.execute("mcp.claude-desktop-status", &json!({})).unwrap();
    assert_eq!((st["state"].as_str(), st["registered"].as_str()), (Some("out of date"), Some(URL)));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn inside_an_agents_request_the_file_is_neither_read_nor_written() {
    let d = scratch("confined");
    let work = d.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let path = d.join("claude_desktop_config.json");
    let text = json!({"mcpServers": {"scrim": {"command": "x", "args": ["http://127.0.0.1:1/mcp"]}}}).to_string();
    std::fs::write(&path, &text).unwrap();
    let roots = std::sync::Arc::new(crate::file_access::Roots::new(&[], &[work.to_string_lossy().into_owned()]).unwrap());
    let mut s = Session::new();
    s.agents.mcp.port = Some(7680);
    s.agents.mcp.desktop_config = Some(path.clone());
    // Confined to its folder, an agent's request learns nothing from the file.
    let r = crate::file_access::confined(Some(&roots), || s.execute("mcp.claude-desktop-status", &json!({}))).unwrap();
    assert_eq!(r["state"], "unreadable");
    assert!(r["registered"].is_null() && r["old"] == json!([]), "{r}");
    // And nothing can write it there.
    let err = crate::file_access::confined(Some(&roots), || apply(&path, &Change::Remove(KEY.into()), 0)).unwrap_err();
    assert!(err.contains("only written by the person"), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    assert_eq!(files(&d), vec!["claude_desktop_config.json".to_string(), "work".to_string()]);
    let _ = std::fs::remove_dir_all(&d);
}
