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

/// Settings ▸ Tool access over HTTP: a tool turned off is left out of tools/list and refused if
/// called anyway (nothing reaches the UI thread); turned back on, it's there at once.
#[test]
fn a_tool_turned_off_is_hidden_and_refused_over_http() {
    let queue = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    ui(queue.clone(), seen.clone());
    let mut h = McpHandle::new(queue, Arc::new(Mutex::new(None)));
    let mut settings = McpSettings { port: free_port(), disabled_tools: vec!["run_command".into()], ..Default::default() };
    let port = h.restart(&settings).unwrap_or_else(|e| panic!("{e}"));
    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
    let names = |body: &str| -> Vec<String> {
        let v: Value = serde_json::from_str(body).unwrap_or_default();
        v["result"]["tools"].as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    let listed = names(&post(port, "", &list).1);
    assert!(listed.iter().any(|n| n == "inspect_document") && !listed.iter().any(|n| n == "run_command"), "{listed:?}");
    let call = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "select.all"}}});
    let (_, body) = post(port, "", &call);
    assert!(body.contains("turned off in Scrim"), "{body}");
    assert!(!seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().any(|(m, _)| m == "engine.execute"), "refused before the UI");
    settings.disabled_tools.clear();
    h.configure(&settings);
    assert!(names(&post(port, "", &list).1).iter().any(|n| n == "run_command"), "back on at once");
}

/// C1.3 over HTTP: with create_node off, run_command and batch can't run node.create either, and
/// nothing reaches the UI thread.
#[test]
fn a_turned_off_tools_command_is_refused_through_run_command_and_batch_over_http() {
    let queue = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    ui(queue.clone(), seen.clone());
    let mut h = McpHandle::new(queue, Arc::new(Mutex::new(None)));
    let settings = McpSettings { port: free_port(), disabled_tools: vec!["create_node".into()], ..Default::default() };
    let port = h.restart(&settings).unwrap_or_else(|e| panic!("{e}"));
    let create = json!({"command": "node.create", "params": {"type": "rectangle"}});
    let run = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "run_command", "arguments": create}});
    let batch = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "batch", "arguments": {"commands": [create]}}});
    for call in [run, batch] {
        let (_, body) = post(port, "", &call);
        assert!(body.contains("does what the tool `create_node` does"), "{body}");
    }
    assert!(!seen.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().any(|(m, _)| m == "engine.execute"), "refused before the UI");
}

/// Fix P: with a settings folder (a scratch one here), the endpoint notes where it listens (the port
/// it got, a fallback included) and the tools it lists (Tool access applied), follows a restart on
/// another port, and takes its note away when the app lets go of it.
#[test]
fn the_endpoint_is_noted_for_bridges_and_the_note_goes_with_it() {
    use figcraft_mcp::discovery::{ENDPOINT_FILE, endpoint, tools};
    let dir = std::env::temp_dir().join(format!("scrim-endpoint-note-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let queue = Arc::new(Mutex::new(Vec::new()));
    let mut h = McpHandle::new(queue, Arc::new(Mutex::new(None))).with_dir(Some(dir.clone()));
    let settings = McpSettings { port: free_port(), disabled_tools: vec!["export".into()], ..Default::default() };
    let port = h.restart(&settings).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(endpoint(&dir).port(), port);
    let listed = tools(&dir).unwrap_or_else(|| panic!("the tools are noted"));
    let names: Vec<&str> = listed["tools"].as_array().into_iter().flatten().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"batch") && !names.contains(&"export"), "Tool access applied: {names:?}");
    let note = std::fs::read_to_string(dir.join(ENDPOINT_FILE)).unwrap_or_default();
    assert!(!note.contains("token"), "no token in the note: {note}");
    // Restarted on another port: the note follows.
    let again = h.restart(&McpSettings { port: free_port(), ..Default::default() }).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(endpoint(&dir).port(), again);
    drop(h);
    assert!(!dir.join(ENDPOINT_FILE).exists(), "gone when the app lets go of the endpoint");
    let _ = std::fs::remove_dir_all(&dir);
}
