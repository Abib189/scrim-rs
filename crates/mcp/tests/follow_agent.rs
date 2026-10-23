//! Stage I3, then Fixes V2 and W3: following agents, revealing their work, its speed, the working
//! indicator and the grace are saved settings, the person's alone: an agent asking over MCP
//! (`run_command` or `batch`) is refused. `finish_working` and focusing layers still work, they are
//! listed for agents to find, and none of it touches the document.
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
    // The reveal, its speed, the working indicator and the grace are saved settings: the person's
    // alone since Fix W3 (the grace since Fix Q), refused through `run_command` and `batch`.
    for (cmd, params) in [
        ("pref.revealAgentWork", json!({"on": false})),
        ("pref.revealAgentWork", json!({})),
        ("pref.workingIndicator", json!({"on": false})),
        ("pref.revealSpeed", json!({"speed": "slow"})),
        ("pref.agentGrace", json!({"seconds": 30})),
    ] {
        let r = c.call("run_command", json!({"command": cmd, "params": params}));
        assert_eq!(r["isError"], true, "{cmd}: {r}");
        assert!(r.to_string().contains("only the person changes Scrim's saved settings"), "{cmd}: {r}");
    }
    let r = c.call("batch", json!({"commands": [{"command": "pref.revealSpeed", "params": {"speed": "fast"}}]}));
    assert!(r.to_string().contains("only the person changes Scrim's saved settings"), "{r}");
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
