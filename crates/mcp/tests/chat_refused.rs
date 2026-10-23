//! R5b: the composer's commands are the person's alone. Over MCP (`run_command` and `batch`) an
//! agent is refused every one of them, with the same words, and nothing reaches the window.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use figcraft_engine::cmd::chat::PERSON_ONLY;
use serde_json::json;

const WORDS: &str = "Describe a change is the person's: agents can't start, stop or set up agent runs";

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
fn an_agent_over_mcp_is_refused_every_chat_command() {
    let mut c = Client::new();
    for cmd in PERSON_ONLY {
        let e = c.err("run_command", json!({"command": cmd, "params": params(cmd)}));
        // A command that writes a setting meets Fix W3's refusal first; the others their own.
        let want =
            if figcraft_engine::find_command(cmd).is_some_and(|s| s.setting) { "only the person changes Scrim's saved settings" } else { WORDS };
        assert!(e.contains(want), "{cmd}: {e}");
    }
}

#[test]
fn nor_can_an_agent_carry_one_in_a_batch() {
    let mut c = Client::new();
    let e = c.err("batch", json!({"commands": [{"command": "chat.send", "params": {"prompt": "x"}}]}));
    assert!(e.contains(WORDS), "{e}");
}
