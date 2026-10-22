//! Stage I3: revealing agents' work and the working indicator are reachable from MCP like every
//! command (`run_command`), each answering its state; turning them off and on never touches the
//! document (they are the window's view, saved in Settings, not in the file). Following agents is
//! the person's alone since Fix V2: an agent is refused.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::json;

#[test]
fn the_switches_answer_their_state_over_mcp_and_leave_the_document_alone() {
    let mut c = Client::new();
    c.create(json!({"type": "frame", "x": 0, "y": 0, "width": 300, "height": 200, "name": "Poster"}));
    let doc = c.doc();
    // Following is the person's alone (Fix V2): an agent is refused whatever it sends, through
    // `run_command` or `batch`.
    for params in [json!({}), json!({"on": false}), json!({"on": true})] {
        let r = c.call("run_command", json!({"command": "camera.follow-agents", "params": params}));
        assert_eq!(r["isError"], true, "{r}");
        assert!(r.to_string().contains("only the person turns following on or off"), "{r}");
    }
    let r = c.call("batch", json!({"commands": [{"command": "camera.follow-agents", "params": {}}]}));
    assert!(r.to_string().contains("only the person turns following on or off"), "{r}");
    // The reveal and the working indicator.
    assert_eq!(c.run("pref.revealAgentWork", json!({"on": false})), json!({"on": false}));
    assert_eq!(c.run("pref.revealAgentWork", json!({})), json!({"on": true}));
    assert_eq!(c.run("pref.workingIndicator", json!({"on": false})), json!({"on": false}));
    assert_eq!(c.run("pref.workingIndicator", json!({"on": true})), json!({"on": true}));
    // Stage I.1: the reveal's speed. The grace period is the person's alone (Fix Q): refused.
    let r = c.call("run_command", json!({"command": "pref.agentGrace", "params": {"seconds": 30}}));
    assert_eq!(r["isError"], true, "{r}");
    assert!(r.to_string().contains("only the person sets how long the working indicator stays"), "{r}");
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
