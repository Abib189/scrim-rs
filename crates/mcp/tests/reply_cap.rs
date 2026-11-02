//! Fix AI C-4 (upstream FigCraft b1f0eafb, 1aa1d61b): an MCP reply fits what a client can read.
//! A stdio client reads one tool result whole, and the usual buffer is 10 MiB: a larger reply broke
//! the session. `inspect_document` answers at the deepest depth that fits, `get_node` also windows
//! a wide layer's children, and both say so with a `truncated` marker. A reply within the limit is
//! unchanged; any other tool's reply over the limit is withheld with the same marker.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::Client;
use serde_json::{Value, json};

/// A client with a document whose frame "Wide" holds `n` rectangles; the frame's id.
fn wide(n: u32) -> (Client, u64) {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let k = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("figcraft-mcp-cap-{n}-{}-{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut doc = figcraft_doc::Document::new("Big");
    let page = doc.first_page().unwrap();
    let frame = doc.create(page, figcraft_doc::NodeKind::Frame, "Wide", 0.0, 0.0, 4000.0, 4000.0).unwrap();
    for i in 0..n {
        doc.create(frame, figcraft_doc::NodeKind::Rectangle, &format!("Cell {i}"), f64::from(i % 60) * 60.0, f64::from(i / 60) * 60.0, 50.0, 50.0)
            .unwrap();
    }
    let file = d.join("big.json");
    std::fs::write(&file, serde_json::to_string(&doc).unwrap()).unwrap();
    let mut c = Client::new();
    c.ok("open_file", json!({"path": file.to_string_lossy()}));
    std::fs::remove_dir_all(&d).ok();
    (c, frame.0)
}

/// The reply's text, the size a client reads, and the text parsed.
fn reply(c: &mut Client, tool: &str, args: Value) -> (String, usize, Value) {
    let r = c.call(tool, args);
    assert_eq!(r["isError"], false, "{r}");
    let text = r["content"][0]["text"].as_str().unwrap().to_string();
    let on_wire = serde_json::to_string(&r).unwrap().len();
    let v = serde_json::from_str(&text).expect("valid JSON");
    (text, on_wire, v)
}

#[test]
fn inspect_document_answers_within_the_limit_with_a_marker() {
    let (mut c, _) = wide(1500);
    let limit = 40_000;
    let (_, on_wire, v) = reply(&mut c, "inspect_document", json!({"depth": 4, "maxBytes": limit}));
    assert!(on_wire <= limit, "{on_wire} bytes against a {limit}-byte limit");
    let t = &v["truncated"];
    assert_eq!(t["requestedDepth"], 4, "{t}");
    assert!(t["depth"].as_u64().unwrap() < 4, "{t}");
    assert!(t["requestedDepthBytes"].as_u64().unwrap() as usize > limit, "{t}");
    assert_eq!(t["maxBytes"], limit);
    assert!(t["hint"].as_str().unwrap().contains("get_node"), "{t}");
}

#[test]
fn a_reply_within_the_limit_is_unchanged_byte_for_byte() {
    let (mut c, frame) = wide(20);
    let (text, _, v) = reply(&mut c, "inspect_document", json!({"depth": 4}));
    assert!(v.get("truncated").is_none());
    let raw = c.server.backend().call("document.inspect", json!({"depth": 4, "allPages": false})).unwrap();
    assert_eq!(text, serde_json::to_string_pretty(&raw).unwrap(), "the backend's answer, untouched");
    let (text, _, v) = reply(&mut c, "get_node", json!({"id": frame, "depth": 1}));
    assert!(v.get("truncated").is_none());
    let raw = c.server.backend().call("node.get", json!({"id": frame, "depth": 1})).unwrap();
    assert_eq!(text, serde_json::to_string_pretty(&raw).unwrap());
}

#[test]
fn get_node_pages_through_a_wide_layer_in_windows() {
    let n = 1500;
    let (mut c, frame) = wide(n);
    let limit = 30_000;
    let mut seen = Vec::new();
    let mut offset = 0u64;
    for _ in 0..100 {
        let (_, on_wire, v) = reply(&mut c, "get_node", json!({"id": frame, "depth": 1, "maxBytes": limit, "offset": offset}));
        assert!(on_wire <= limit, "{on_wire} bytes against a {limit}-byte limit");
        let kids = v["children"].as_array().unwrap();
        seen.extend(kids.iter().map(|k| k["name"].as_str().unwrap().to_string()));
        let w = &v["truncated"]["children"];
        assert_eq!(w["total"], n, "{w}");
        assert_eq!(w["from"], offset);
        assert_eq!(w["shown"], kids.len());
        match w["nextOffset"].as_u64() {
            Some(next) => offset = next,
            None => break,
        }
    }
    let want: Vec<String> = (0..n).map(|i| format!("Cell {i}")).collect();
    assert_eq!(seen, want, "every child once, in order");
}

#[test]
fn another_tool_over_the_limit_says_so_instead_of_sending_it() {
    let (mut c, frame) = wide(1500);
    let limit = 20_000;
    let (_, on_wire, v) = reply(&mut c, "export", json!({"ids": [frame], "format": "svg", "maxBytes": limit}));
    assert!(on_wire <= limit, "{on_wire} bytes against a {limit}-byte limit");
    let t = &v["truncated"];
    assert!(t["bytes"].as_u64().unwrap() as usize > limit, "{t}");
    assert_eq!(t["maxBytes"], limit);
    assert!(t["hint"].as_str().unwrap().contains("`path`"), "{t}");
    // Within the default limit the same export is the SVG itself.
    let r = c.call("export", json!({"ids": [frame], "format": "svg"}));
    assert!(r["content"][0]["text"].as_str().unwrap().starts_with("<svg"), "{}", &r.to_string()[..200]);
}

#[test]
fn a_limit_below_a_kilobyte_is_refused() {
    let (mut c, _) = wide(1);
    assert!(c.err("inspect_document", json!({"maxBytes": 10})).contains("at least 1024"));
}
