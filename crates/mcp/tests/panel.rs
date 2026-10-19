//! The right panel's mode and tabs are commands, so agents switch them over MCP (`run_command`)
//! as the person does in the app (`docs/design/panel-redesign.md`, B1). Headless there is no panel
//! to draw; the commands answer the choice they keep, and never touch the document.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::json;

#[test]
fn the_panel_commands_are_reachable_over_mcp() {
    let mut c = Client::new();
    let before = c.doc();
    for tab in ["design", "prototype", "code", "ai"] {
        assert_eq!(c.run(&format!("panel.tab.{tab}"), json!({})), json!({"mode": "design", "tab": tab}));
    }
    assert_eq!(c.run("panel.mode.animate", json!({})), json!({"mode": "animate", "tab": "ai"}));
    assert_eq!(c.run("panel.mode.design", json!({})), json!({"mode": "design", "tab": "ai"}));
    assert_eq!(c.doc(), before, "switching the panel is never an edit");
    let commands = c.ok("list_commands", json!({}));
    for id in ["panel.mode.design", "panel.mode.animate", "panel.tab.design", "panel.tab.prototype", "panel.tab.code", "panel.tab.ai"] {
        assert!(commands.to_string().contains(id), "{id} is listed for agents");
    }
}

#[test]
fn the_code_tabs_commands_are_reachable_over_mcp() {
    let mut c = Client::new();
    let frame = c.run("frame.create", json!({"x": 0, "y": 0, "width": 120, "height": 80, "name": "Card"}));
    let before = c.doc();
    assert_eq!(c.run("code.setFormat", json!({"format": "html", "unit": "rem"})), json!({"format": "html", "unit": "rem"}));
    let code = c.run("code.copy", json!({"id": frame["id"], "language": "css"}));
    assert_eq!(code["language"], "css");
    assert!(code["code"].as_str().unwrap().contains("width"), "{code}");
    let page = c.run("code.copyStandaloneHtml", json!({"id": frame["id"]}));
    let html = page["html"].as_str().unwrap();
    assert!(html.starts_with("<!doctype html>") && html.contains("<title>Card</title>"), "{html}");
    assert_eq!(c.doc(), before, "reading code is never an edit");
}

#[test]
fn the_ai_tabs_commands_are_reachable_over_mcp() {
    let mut c = Client::new();
    let status = c.run("ai.status", json!({}));
    assert_eq!((&status["channel"], &status["connectCommand"]), (&json!("off"), &json!("claude mcp add scrim -- scrim mcp")));
    // An agent's own change, through MCP: in the feed, selectable, and undoable while it's the latest.
    let r = c.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 10, "height": 10}));
    let feed = c.ok("get_changes", json!({}));
    let last = feed["changes"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["agent"], true);
    let picked = c.run("ai.selectChange", json!({"revision": last["revision"]}));
    assert_eq!(picked["selected"], json!([r["id"]]));
    assert_eq!(c.run("ai.undoChange", json!({"revision": last["revision"]}))["undone"], true);
    let findings = c.run("ai.findings", json!({}));
    assert!(findings["groups"].is_array() && findings["count"].is_number(), "{findings}");
}
