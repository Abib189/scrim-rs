//! `query_nodes` (Port 5): Scrim's XPath-style query over MCP. It is read-only, its errors say
//! what the subset supports, and the CLI's `query` gives the same answer.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use figcraft_engine::Session;
use figcraft_mcp::{Headless, Server};
use serde_json::{Value, json};

fn id(v: &Value) -> u64 {
    v["id"].as_u64().unwrap()
}

/// A frame "Pricing" with auto layout, holding a text "Price" and a rectangle "Swatch".
fn mcp() -> (Client, u64, u64) {
    let mut s = Session::new();
    let f = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 300, "height": 100, "name": "Pricing"})).unwrap());
    s.execute("layout.add", &json!({"ids": [f]})).unwrap();
    s.execute("node.create", &json!({"type": "text", "text": "$12", "name": "Price", "parent": f})).unwrap();
    let r = id(&s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 40, "height": 20, "parent": f})).unwrap());
    s.execute("node.set", &json!({"id": r, "props": {"name": "Swatch"}})).unwrap();
    (Client::with_server(Server::new(Box::new(Headless::with_session(s)))), f, r)
}

#[test]
fn query_nodes_is_listed_read_only() {
    let (mut c, ..) = mcp();
    let tools = c.rpc("tools/list", json!({}));
    let t = tools["result"]["tools"].as_array().unwrap().iter().find(|t| t["name"] == "query_nodes").expect("listed");
    assert_eq!(t["annotations"]["readOnlyHint"], true);
    assert_eq!(t["inputSchema"]["required"], json!(["query"]));
}

#[test]
fn query_nodes_finds_layers_in_document_order() {
    let (mut c, f, r) = mcp();
    let before = c.doc();
    let out = c.ok("query_nodes", json!({"query": "//FRAME[@layout=\"flex\"]/*"}));
    assert_eq!((out["total"].as_u64(), out["truncated"].as_bool()), (Some(2), Some(false)));
    let ids: Vec<u64> = out["results"].as_array().unwrap().iter().map(|h| h["id"].as_u64().unwrap()).collect();
    assert_eq!(ids[1], r);
    assert!(out["results"].as_array().unwrap().iter().all(|h| h["parentId"] == json!(f)));
    assert_eq!((out["results"][1]["rect"]["w"].as_f64(), out["results"][1]["rect"]["h"].as_f64()), (Some(40.0), Some(20.0)));
    let out = c.ok("query_nodes", json!({"query": "//*", "limit": 1}));
    assert_eq!((out["total"].as_u64(), out["returned"].as_u64(), out["truncated"].as_bool()), (Some(3), Some(1), Some(true)));
    assert_eq!(c.doc(), before, "read-only");
}

#[test]
fn an_invalid_query_says_what_is_supported() {
    let (mut c, ..) = mcp();
    let e = c.err("query_nodes", json!({"query": "//TEXT/.."}));
    assert!(e.contains("invalid query: '..' (parent) is not supported: queries walk down from the document with / and // (at character 8)"), "{e}");
    let e = c.err("query_nodes", json!({"query": "//TEXT[@name = Title]"}));
    assert!(e.contains("'Title' is not a value: quote strings ('Title') (at character 16)"), "{e}");
    let e = c.err("query_nodes", json!({"query": "//*", "limit": 0}));
    assert!(e.contains("`limit` is a whole number from 1 to 1000"), "{e}");
}
