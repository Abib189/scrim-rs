//! Fix L1: the tool list every agent carries in every request stays small.
//! - Only the five building tools end with the finish reminder; no other tool carries it.
//! - The whole `tools/list` response stays within 2% of the size measured when L1 landed.
//!   Growing it more means updating [`BASELINE_CHARS`] here on purpose, with the reason.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use figcraft_mcp::{FINISH_HINT, Server};
use serde_json::{Value, json};

/// `tools/list` in characters, measured 21 Oct 2026 after Fix L1 (25 tools). Before Stage I.1 it
/// was 28,836 (24 tools); after Stage I.1, 30,780.
const BASELINE_CHARS: usize = 29_554;

fn tools_list() -> (String, Vec<Value>) {
    let mut s = Server::headless();
    s.handle(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "budget", "version": "1"}}}));
    let r = s.handle(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).expect("an answer");
    let tools = r["result"]["tools"].as_array().expect("tools").clone();
    (serde_json::to_string(&r).expect("serialises"), tools)
}

#[test]
fn only_the_five_building_tools_carry_the_finish_reminder() {
    // Written out here, not read from the server's list: changing that list must fail this test.
    let five = ["create_node", "set_properties", "batch", "run_command", "type_text"];
    let (_, tools) = tools_list();
    let reminder = FINISH_HINT.trim();
    assert_eq!(reminder, "When done, call finish_working.");
    let mut carried = Vec::new();
    for t in &tools {
        let (name, desc) = (t["name"].as_str().unwrap(), t["description"].as_str().unwrap());
        if five.contains(&name) {
            assert!(desc.ends_with(FINISH_HINT), "{name} lost the reminder: {desc}");
        } else {
            assert!(!desc.contains(reminder) && !desc.contains("call finish_working"), "{name} carries the reminder: {desc}");
        }
        if desc.contains(reminder) {
            carried.push(name);
        }
    }
    carried.sort_unstable();
    let mut want = five.to_vec();
    want.sort_unstable();
    assert_eq!(carried, want);
    // finish_working keeps its own description.
    let fw = tools.iter().find(|t| t["name"] == "finish_working").expect("listed");
    assert!(fw["description"].as_str().unwrap().starts_with("Say you are done working on the canvas"));
}

#[test]
fn the_tool_list_stays_within_two_percent_of_its_measured_size() {
    let (text, tools) = tools_list();
    let chars = text.chars().count();
    let limit = BASELINE_CHARS + BASELINE_CHARS * 2 / 100;
    assert!(
        chars <= limit,
        "tools/list is {chars} characters ({} tools), over {limit} (the baseline {BASELINE_CHARS} + 2%). If that is deliberate, update BASELINE_CHARS and say why.",
        tools.len()
    );
}
