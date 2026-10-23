use serde_json::json;

use super::*;
use crate::agents::Author;
use crate::cmd::access::{Access, access_of};

/// The routes agents come in by: the app's MCP endpoint, the control channel, the headless CLI's
/// MCP server and `scrim run`, and a plugin.
const ROUTES: [&str; 5] = ["mcp (http)", "control channel", "mcp", "cli", "plugin Tidy names"];

fn params(cmd: &str) -> serde_json::Value {
    match cmd {
        "chat.send" => json!({"prompt": "make it blue"}),
        "chat.allow-agent" => json!({"agent": "claude-code", "path": "/usr/local/bin/claude"}),
        "pref.describeChange" => json!({"on": false}),
        "pref.chatTimeLimit" => json!({"minutes": 30}),
        _ => json!({}),
    }
}

#[test]
fn every_chat_command_is_refused_for_agents_on_every_route() {
    for route in ROUTES {
        for cmd in PERSON_ONLY {
            let mut s = Session::new();
            s.set_author(Author::agent(route));
            let before = (s.prefs.settings.describe_change, s.prefs.settings.chat_time_limit_mins, s.prefs.settings.chat_allowed.len());
            let err = s.execute(cmd, &params(cmd)).unwrap_err().to_string();
            // A command that writes a setting meets Fix W3's refusal first; the others their own.
            let want = if crate::find_command(cmd).is_some_and(|c| c.setting) {
                "only the person changes Scrim's saved settings"
            } else {
                "Describe a change is the person's: agents can't start, stop or set up agent runs"
            };
            assert!(err.contains(want), "{route} {cmd}: {err}");
            assert!(s.ui_requests.iter().all(|r| !r.kind.starts_with("chat.")), "{route} {cmd}: the window isn't asked");
            let after = (s.prefs.settings.describe_change, s.prefs.settings.chat_time_limit_mins, s.prefs.settings.chat_allowed.len());
            assert_eq!(before, after, "{route} {cmd}: no setting changed");
        }
    }
}

#[test]
fn an_agents_batch_cant_carry_a_chat_command_either() {
    let mut s = Session::new();
    s.set_author(Author::agent("mcp (http)"));
    let err = s.execute("command.batch", &json!({"commands": [{"command": "chat.send", "params": {"prompt": "x"}}]})).unwrap_err().to_string();
    assert!(err.contains("agents can't start"), "{err}");
}

#[test]
fn the_person_sends_and_the_window_is_asked() {
    let mut s = Session::new();
    let r = s.execute("chat.send", &json!({"prompt": "  make it blue  "})).unwrap();
    assert_eq!(r, json!({"sent": true, "agent": "claude-code"}));
    let req = s.ui_requests.iter().find(|r| r.kind == SEND_REQUEST).unwrap();
    assert_eq!(req.data, json!({"prompt": "make it blue", "agent": "claude-code"}));
    assert_eq!(s.execute("chat.send", &json!({"prompt": "x", "agent": "codex"})).unwrap()["agent"], "codex");
    let rev = s.ds().unwrap().revision;
    s.execute("chat.cancel", &json!({})).unwrap();
    s.execute("chat.clear", &json!({})).unwrap();
    assert_eq!(s.ds().unwrap().revision, rev, "nothing written to the document");
    assert!(s.ui_requests.iter().any(|r| r.kind == CANCEL_REQUEST) && s.ui_requests.iter().any(|r| r.kind == CLEAR_REQUEST));
}

#[test]
fn a_send_without_a_prompt_with_an_unknown_agent_or_too_long_is_refused() {
    let mut s = Session::new();
    assert!(s.execute("chat.send", &json!({"prompt": "   "})).unwrap_err().to_string().contains("`prompt` is required"));
    assert!(
        s.execute("chat.send", &json!({"prompt": "x", "agent": "gpt"})).unwrap_err().to_string().contains("no agent `gpt`: claude-code or codex")
    );
    let long = "x".repeat(MAX_PROMPT + 1);
    assert!(s.execute("chat.send", &json!({"prompt": long})).is_err());
    assert!(s.ui_requests.iter().all(|r| r.kind != SEND_REQUEST));
}

#[test]
fn turned_off_or_inside_an_agent_run_nothing_is_sent() {
    let mut s = Session::new();
    s.execute("pref.describeChange", &json!({"on": false})).unwrap();
    assert!(!s.prefs.settings.describe_change);
    assert!(s.execute("chat.send", &json!({"prompt": "x"})).unwrap_err().to_string().contains("off in Settings ▸ Agents"));
    let mut s = Session::new();
    s.agents.chat_child = true;
    let err = s.execute("chat.send", &json!({"prompt": "x"})).unwrap_err().to_string();
    assert!(err.contains("started by an agent run"), "{err}");
}

#[test]
fn the_time_limit_takes_5_15_30_or_60_minutes_and_15_is_the_default() {
    let mut s = Session::new();
    assert_eq!(s.prefs.settings.chat_time_limit_mins, DEFAULT_TIME_LIMIT);
    for m in TIME_LIMITS {
        assert_eq!(s.execute("pref.chatTimeLimit", &json!({"minutes": m})).unwrap()["minutes"], m);
    }
    for m in [0, 1, 10, 61, 120] {
        assert!(s.execute("pref.chatTimeLimit", &json!({"minutes": m})).is_err(), "{m}");
    }
}

#[test]
fn an_agent_is_allowed_per_binary_and_forgotten_on_request() {
    let mut s = Session::new();
    assert!(!allowed(&s, "claude-code", "/a/claude"));
    s.execute("chat.allow-agent", &json!({"agent": "claude-code", "path": "/a/claude"})).unwrap();
    s.execute("chat.allow-agent", &json!({"agent": "claude-code", "path": "/a/claude"})).unwrap();
    assert!(allowed(&s, "claude-code", "/a/claude"));
    assert_eq!(s.prefs.settings.chat_allowed.len(), 1, "once");
    assert!(!allowed(&s, "claude-code", "/b/claude"), "another binary asks again");
    assert!(!allowed(&s, "codex", "/a/claude"));
    assert!(s.ui_requests.iter().any(|r| r.kind == ALLOWED_REQUEST));
    assert_eq!(s.execute("chat.forget-agents", &json!({})).unwrap()["forgotten"], 1);
    assert!(!allowed(&s, "claude-code", "/a/claude"));
}

#[test]
fn every_chat_command_is_registered_and_classified_write() {
    for cmd in PERSON_ONLY {
        let c = crate::find_command(cmd).unwrap_or_else(|| panic!("{cmd} registered"));
        assert_eq!(access_of(c), Some(Access::Write), "{cmd}");
        // Undo run is the one undo step (Fix X4): it puts the document back as a whole.
        assert_eq!(c.history, cmd == "chat.undo-run", "{cmd}: an undo step only for Undo run");
    }
}

#[test]
fn a_process_started_by_an_agent_run_is_told_by_its_environment() {
    use std::ffi::OsStr;
    assert!(started_by_agent_run(Some(OsStr::new("1"))));
    assert!(!started_by_agent_run(Some(OsStr::new(""))));
    assert!(!started_by_agent_run(None));
}

/// Fix T4: the AI panel's actions are commands, the person's alone, and check what they're given.
#[test]
fn the_ai_panels_commands_are_the_persons_and_check_their_parameters() {
    use super::PANEL_REQUESTS;
    let mut s = Session::new();
    for id in PANEL_REQUESTS {
        let p = match id {
            "chat.open" => json!({"id": "c1"}),
            "chat.scope" => json!({"scope": "all"}),
            "chat.agent" => json!({"agent": "codex"}),
            _ => json!({}),
        };
        assert!(s.execute(id, &p).is_ok(), "{id} for the person");
        assert_eq!(s.ui_requests.last().map(|r| r.kind.clone()), Some(id.to_string()), "{id} asks the window");
    }
    assert!(s.execute("chat.open", &json!({})).is_err(), "an id");
    assert!(s.execute("chat.scope", &json!({"scope": "everything"})).is_err());
    assert!(s.execute("chat.agent", &json!({"agent": "gpt"})).is_err());
    // An image only for an agent that takes one.
    s.prefs.settings.describe_change = true;
    assert!(s.execute("chat.send", &json!({"prompt": "x", "agent": "claude-code", "image": "/tmp/a.png"})).is_err());
    assert!(s.execute("chat.send", &json!({"prompt": "x", "agent": "codex", "image": "/tmp/a.png"})).is_ok());
    // Agents: refused, every one.
    s.set_author(crate::agents::Author::agent("mcp (http)"));
    for id in PANEL_REQUESTS {
        let e = s.execute(id, &json!({"id": "c1", "scope": "all"})).unwrap_err().to_string();
        assert!(e.contains("the person's"), "{id}: {e}");
    }
}

/// Fix T4: Settings ▸ Agents is the `ai` section; `agents` opens it too, and nothing else is new.
#[test]
fn settings_open_takes_agents_for_the_agents_section() {
    let mut s = Session::new();
    let r = s.execute("settings.open", &json!({"section": "agents"})).unwrap();
    assert_eq!(r["section"], "ai");
    assert_eq!(s.ui_requests.last().map(|q| (q.kind.as_str(), q.data["section"].clone())), Some(("dialog.settings", json!("ai"))));
    assert!(s.execute("settings.open", &json!({"section": "chat"})).is_err());
}
