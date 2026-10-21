//! `lint_design`, `set_lint_profile` and `ignore_lint` (Port 6) over MCP: the findings, the
//! stored profile and ignores, their errors, and the lint hint every writing tool carries.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use figcraft_engine::Session;
use figcraft_mcp::{Headless, Server};
use serde_json::{Value, json};

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

/// A white frame "Card" holding a 10 px text "Note".
fn mcp() -> (Client, u64, u64) {
    let mut s = Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 300, "height": 200, "name": "Card"})).unwrap());
    s.execute("fill.set", &json!({"ids": [f], "color": "#FFFFFF"})).unwrap();
    let t = id(&s.execute("node.create", &json!({"type": "text", "text": "Fine print", "name": "Note", "parent": f})).unwrap());
    s.execute("node.set", &json!({"id": t, "props": {"fontSize": 10}})).unwrap();
    (Client::with_server(Server::new(Box::new(Headless::with_session(s)))), f, t)
}

#[test]
fn the_tools_are_listed_and_writing_tools_carry_the_lint_hint() {
    let (mut c, ..) = mcp();
    let tools = c.rpc("tools/list", json!({}))["result"]["tools"].as_array().unwrap().clone();
    let tool = |n: &str| tools.iter().find(|t| t["name"] == n).unwrap_or_else(|| panic!("{n}")).clone();
    assert_eq!(tool("lint_design")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("set_lint_profile")["annotations"]["readOnlyHint"], false);
    assert!(tool("lint_design")["description"].as_str().unwrap().contains("'expressive' = posters, covers, editorial display type"));
    for writer in ["run_command", "batch", "create_node", "set_properties"] {
        let d = tool(writer)["description"].as_str().unwrap().to_string();
        // The lint hint, then (Stage I.1) the line every tool that changes things ends with.
        assert!(d.contains("never to silence contrast.") && d.ends_with(figcraft_mcp::FINISH_HINT), "{writer}");
    }
    let init = c.rpc("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}));
    assert!(init["result"]["instructions"].as_str().unwrap().contains("run lint_design on what you built"));
}

#[test]
fn lint_profile_and_ignore_round_trip() {
    let (mut c, f, t) = mcp();
    let r = c.ok("lint_design", json!({}));
    assert!(r["violations"].as_array().unwrap().iter().any(|v| v["ruleId"] == "tiny-text" && v["nodeId"] == json!(t)), "{r}");
    let e = c.err("ignore_lint", json!({"ruleId": "tiny-text", "nodeId": t}));
    assert_eq!(e, "ignore_lint needs a non-empty reason saying why tiny-text is deliberate here");
    let r = c.ok("ignore_lint", json!({"ruleId": "tiny-text", "nodeId": t, "reason": "a legal footnote"}));
    assert_eq!(r["ignores"][0]["reason"], "a legal footnote");
    let r = c.ok("lint_design", json!({"id": f}));
    assert_eq!(r["ignored"][0]["ruleId"], "tiny-text");
    let r = c.ok("set_lint_profile", json!({"ids": [f], "profile": "expressive"}));
    assert_eq!(r["roots"][0]["profile"], "expressive");
    let e = c.err("set_lint_profile", json!({"ids": [t], "profile": "ui"}));
    assert_eq!(e, format!("lint profiles apply to root artboards; {t} is inside {f}"));
    let e = c.err("lint_design", json!({"id": 424242}));
    assert_eq!(e, "node not found: 424242");
    // The change feed names the agent's configuration edits.
    let changes = c.ok("get_changes", json!({}));
    assert!(changes["changes"].as_array().unwrap().iter().any(|ch| ch["command"] == "lint.setProfile" && ch["agent"] == true));
}

#[test]
fn analyze_design_reports_and_never_writes() {
    let (mut c, f, _) = mcp();
    let before = c.doc();
    let r = c.ok("analyze_design", json!({}));
    assert_eq!(r["scope"], "(document)");
    assert!(r["colors"]["values"].as_array().unwrap().iter().any(|v| v["value"] == "#ffffff"), "{r}");
    let r = c.ok("analyze_design", json!({"id": f}));
    assert_eq!(r["scope"], json!(f));
    assert_eq!(c.err("analyze_design", json!({"id": 4242})), "node not found: 4242");
    assert_eq!(c.doc(), before);
}
