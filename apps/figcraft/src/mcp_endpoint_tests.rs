//! The app's MCP endpoint end to end, with a stand-in UI thread: tool calls reach the control queue
//! as the agent "mcp (http)", a client's initialize is reported, a refused request never queues
//! anything, and a token set later applies at once.

use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use figcraft_engine::settings::McpSettings;
use figcraft_ui_egui::control::ROUTE_MCP_HTTP;
use figcraft_ui_egui::{ControlReply, ControlRequest};
use serde_json::{Value, json};

use super::McpHandle;

/// A free port the OS just handed out.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port()).unwrap_or(0)
}

/// A UI thread stand-in: answers each queued request with its method and route, and records them.
fn ui(queue: Arc<Mutex<Vec<ControlRequest>>>, seen: Arc<Mutex<Vec<(String, String)>>>) {
    std::thread::spawn(move || {
        for _ in 0..2000 {
            let reqs: Vec<ControlRequest> = std::mem::take(&mut *queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
            for r in reqs {
                seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((r.method.clone(), r.route.to_string()));
                let result = match r.method.as_str() {
                    "engine.execute" => Ok(json!({"ran": r.params["command"]})),
                    _ => Ok(json!({})),
                };
                let _ = r.reply.send(ControlReply { id: r.id, result });
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
}

fn post(port: u16, extra: &str, body: &Value) -> (u16, String) {
    let body = body.to_string();
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("{e}"));
    let req = format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    s.write_all(req.as_bytes()).unwrap_or_default();
    let mut all = String::new();
    BufReader::new(s).read_to_string(&mut all).unwrap_or_default();
    let code = all.lines().next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).unwrap_or(0);
    let body = all.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
    (code, body)
}

#[test]
fn tool_calls_reach_the_ui_as_the_mcp_agent() {
    let queue = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    ui(queue.clone(), seen.clone());
    let mut h = McpHandle::new(queue, Arc::new(Mutex::new(None)));
    let port = h.restart(&McpSettings { port: free_port(), ..Default::default() }).unwrap_or_else(|e| panic!("{e}"));
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "clientInfo": {"name": "claude-code", "version": "2.1.296"}}});
    assert_eq!(post(port, "", &init).0, 200);
    let call = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "select.all"}}});
    let (code, body) = post(port, "", &call);
    assert_eq!(code, 200, "{body}");
    assert!(body.contains("select.all"), "the UI's answer comes back: {body}");
    // The client was reported (without waiting), and the call ran as the MCP agent.
    for _ in 0..200 {
        if seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().any(|(m, _)| m == "mcp.client") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let seen = seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone();
    assert!(seen.contains(&("mcp.client".to_string(), ROUTE_MCP_HTTP.to_string())), "{seen:?}");
    assert!(seen.contains(&("engine.execute".to_string(), ROUTE_MCP_HTTP.to_string())), "{seen:?}");
}

#[test]
fn refused_requests_never_reach_the_ui_and_a_token_applies_at_once() {
    let queue = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    ui(queue.clone(), seen.clone());
    let mut h = McpHandle::new(queue, Arc::new(Mutex::new(None)));
    let mut settings = McpSettings { port: free_port(), ..Default::default() };
    let port = h.restart(&settings).unwrap_or_else(|e| panic!("{e}"));
    let call = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "edit.delete"}}});
    assert_eq!(post(port, "Origin: http://evil.example\r\n", &call).0, 403);
    // Authentication on: the running endpoint wants the token from now on.
    settings.require_auth = true;
    settings.token = "t".repeat(64);
    h.configure(&settings);
    assert_eq!(post(port, "", &call).0, 401);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner).is_empty(), "nothing reached the UI");
    assert_eq!(post(port, &format!("Authorization: Bearer {}\r\n", settings.token), &call).0, 200);
    // A restart keeps the port.
    assert_eq!(h.restart(&McpSettings { port, ..settings }).ok(), Some(port));
}
