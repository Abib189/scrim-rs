//! Stage I3: following agents, revealing their work and the working indicator are reachable from
//! MCP like every command (`run_command`), each answering its state; turning them off and on never
//! touches the document (they are the window's view, saved in Settings, not in the file).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::json;

#[test]
fn the_switches_answer_their_state_over_mcp_and_leave_the_document_alone() {
    let mut c = Client::new();
    c.create(json!({"type": "frame", "x": 0, "y": 0, "width": 300, "height": 200, "name": "Poster"}));
    let doc = c.doc();
    // Following: on by default; toggles; on: true turns it back on.
    assert_eq!(c.run("camera.follow-agents", json!({})), json!({"on": false, "state": "off"}));
    assert_eq!(c.run("camera.follow-agents", json!({"on": true})), json!({"on": true, "state": "following"}));
    // The reveal and the working indicator.
    assert_eq!(c.run("pref.revealAgentWork", json!({"on": false})), json!({"on": false}));
    assert_eq!(c.run("pref.revealAgentWork", json!({})), json!({"on": true}));
    assert_eq!(c.run("pref.workingIndicator", json!({"on": false})), json!({"on": false}));
    assert_eq!(c.run("pref.workingIndicator", json!({"on": true})), json!({"on": true}));
    // Stage I.1: the grace period and the reveal's speed.
    assert_eq!(c.run("pref.agentGrace", json!({"seconds": 30})), json!({"seconds": 30}));
    assert_eq!(c.run("pref.revealSpeed", json!({"speed": "slow"})), json!({"speed": "slow"}));
    assert_eq!(c.run("pref.revealSpeed", json!({"speed": "fast"})), json!({"speed": "fast"}));
    // `finish_working` (the tool, and its command) changes nothing either.
    assert_eq!(c.ok("finish_working", json!({}))["finished"], true);
    assert_eq!(c.run("agent.finish-working", json!({}))["finished"], true);
    // `fit` on the focus command.
    let r = c.run("camera.focus-layer", json!({"ids": [doc["pages"][0]["children"][0]["id"].clone()], "fit": true}));
    assert!(r["view"]["zoom"].is_number(), "{r}");
    assert_eq!(c.doc(), doc, "none of it is in the document");
    // Listed with their labels for agents to find.
    let list = c.ok("list_commands", json!({"filter": "agent"})).to_string();
    for id in ["camera.follow-agents", "pref.revealAgentWork", "pref.workingIndicator", "pref.agentGrace", "agent.finish-working"] {
        assert!(list.contains(id), "{id} listed: {list}");
    }
    let list = c.ok("list_commands", json!({"filter": "reveal"})).to_string();
    assert!(list.contains("pref.revealSpeed"), "{list}");
}
