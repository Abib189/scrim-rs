//! Settings ▸ Tool access through the MCP server (stdio and headless): a tool turned off is left
//! out of `tools/list` and refused when called directly; the engine's list of tools (what Settings
//! shows, read-only or not) is the server's.

use figcraft_engine::Session;
use figcraft_engine::cmd::mcp_tools::TOOLS;
use figcraft_mcp::{Headless, Server};
use serde_json::{Value, json};

fn server_with_off(off: &[&str]) -> Server {
    let mut s = Session::new();
    s.prefs.settings.mcp.disabled_tools = off.iter().map(|t| t.to_string()).collect();
    let mut server = Server::new(Box::new(Headless::with_session(s)));
    server.handle(json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}));
    server
}

fn names(server: &mut Server) -> Vec<String> {
    let r = server.handle(json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).unwrap_or_default();
    r["result"]["tools"].as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect()).unwrap_or_default()
}

#[test]
fn a_tool_turned_off_is_left_out_of_tools_list() {
    let mut on = server_with_off(&[]);
    assert_eq!(names(&mut on).len(), TOOLS.len());
    let mut off = server_with_off(&["export", "create_node"]);
    let listed = names(&mut off);
    assert_eq!(listed.len(), TOOLS.len() - 2);
    assert!(!listed.iter().any(|n| n == "export" || n == "create_node"), "{listed:?}");
}

#[test]
fn a_tool_turned_off_is_refused_when_called_directly() {
    let mut server = server_with_off(&["create_node"]);
    let call = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "create_node", "arguments": {"type": "rectangle"}}});
    let r = server.handle(call).unwrap_or_default();
    let msg = r["error"]["message"].as_str().unwrap_or_default();
    assert!(msg.contains("turned off in Scrim"), "{r}");
    // Nothing was created.
    let doc = server
        .handle(json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "inspect_document", "arguments": {}}}))
        .unwrap_or_default();
    assert!(!doc.to_string().contains("Rectangle"), "{doc}");
}

#[test]
fn the_engines_tool_list_is_the_servers() {
    let defs = figcraft_mcp::tool_definitions();
    let server: Vec<(String, bool)> = defs
        .iter()
        .map(|d| (d["name"].as_str().unwrap_or_default().to_string(), d["annotations"]["readOnlyHint"].as_bool().unwrap_or(false)))
        .collect();
    let engine: Vec<(String, bool)> = TOOLS.iter().map(|(n, _, ro)| (n.to_string(), *ro)).collect();
    assert_eq!(engine, server, "cmd::mcp_tools::TOOLS must list the server's tools, in order, with their readOnlyHint");
    assert_eq!(TOOLS.iter().map(|t| t.0).collect::<Vec<_>>(), figcraft_mcp::TOOL_NAMES);
    let titles: Vec<Value> = defs.iter().map(|d| d["annotations"]["title"].clone()).collect();
    assert_eq!(titles, TOOLS.iter().map(|t| json!(t.1)).collect::<Vec<_>>(), "and their titles");
}
