//! Stage I.1 on the MCP side (`docs/design/working-state.md`):
//! - `finish_working` is a tool: listed, read-only, routed to `agent.finish-working`, and it runs
//!   while every tool that changes things is off; only turning it off refuses it.
//! - Every tool that isn't read-only ends with the finish hint, and so do the instructions.
//! - An HTTP session ends with `DELETE /mcp` (200, then 404; 400 without the header), and the app's
//!   session hook hears it start and end; the backend is told each request's known session.
//! - `scrim mcp --attach` (a server announcing its clients) tells the app who it serves.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};

use figcraft_mcp::http::{Config, Endpoint};
use figcraft_mcp::{Backend, Server, SessionEvent, tool_definitions};
use serde_json::{Value, json};

/// Every backend call (method, params) and the session it was told.
#[derive(Default)]
struct Log {
    calls: Vec<(String, Value, Option<String>)>,
}

struct Recorder {
    log: Arc<Mutex<Log>>,
    session: Option<String>,
    off: Vec<String>,
}

impl Backend for Recorder {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.log.lock().unwrap().calls.push((method.to_string(), params.clone(), self.session.clone()));
        Ok(match params.get("command").and_then(Value::as_str) {
            Some("agent.finish-working") => json!({"finished": true, "client": null, "ids": []}),
            _ => json!({}),
        })
    }
    fn set_session(&mut self, session: Option<&str>) {
        self.session = session.map(str::to_string);
    }
    fn has_ui(&self) -> bool {
        false
    }
    fn describe(&self) -> String {
        "a recorder".into()
    }
    fn disabled_tools(&mut self) -> Vec<String> {
        self.off.clone()
    }
}

fn recorder(off: &[&str]) -> (Server, Arc<Mutex<Log>>) {
    let log = Arc::new(Mutex::new(Log::default()));
    let off = off.iter().map(|s| s.to_string()).collect();
    (Server::new(Box::new(Recorder { log: log.clone(), session: None, off })), log)
}

fn init(name: &str) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": name, "version": "1"}}})
        .to_string()
}

fn finish(ids: Value) -> String {
    json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call", "params": {"name": "finish_working", "arguments": {"ids": ids}}}).to_string()
}

#[test]
fn finish_working_is_a_read_only_tool_that_runs_when_writing_tools_are_off() {
    let tools = tool_definitions();
    let t = tools.iter().find(|t| t["name"] == "finish_working").expect("listed");
    assert_eq!(t["annotations"]["readOnlyHint"], true);
    assert!(t["description"].as_str().unwrap().contains("Changes nothing in the document"));
    // Only the building tools say how to end the working state (Fix L1, `tool_list_budget.rs`).
    for t in &tools {
        let hinted = figcraft_mcp::FINISH_HINTED.contains(&t["name"].as_str().unwrap());
        assert_eq!(t["description"].as_str().unwrap().ends_with(figcraft_mcp::tools_finish_hint()), hinted, "{}", t["name"]);
    }
    let instructions = figcraft_mcp::INSTRUCTIONS;
    assert!(instructions.contains("call finish_working so the person sees you are done"));
    // With every tool that changes things off, it still runs (it's classified Read) …
    let writing: Vec<&str> = figcraft_engine::cmd::mcp_tools::TOOLS.iter().filter(|t| !t.2).map(|t| t.0).collect();
    let (mut s, log) = recorder(&writing);
    let r: Value = serde_json::from_str(&s.handle_line(&finish(json!([5]))).unwrap()).unwrap();
    assert_eq!(r["result"]["isError"], false, "{r}");
    let (method, params, _) = log.lock().unwrap().calls.last().cloned().unwrap();
    assert_eq!((method.as_str(), params["command"].as_str()), ("engine.execute", Some("agent.finish-working")));
    assert_eq!(params["params"]["ids"], json!([5]));
    // … and through run_command too; only turning finish_working itself off refuses it.
    let rc = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "agent.finish-working"}}});
    let (mut s, _) = recorder(&["finish_working"]);
    let r: Value = serde_json::from_str(&s.handle_line(&finish(json!([]))).unwrap()).unwrap();
    assert!(r["error"]["message"].as_str().unwrap().contains("turned off"), "{r}");
    let r: Value = serde_json::from_str(&s.handle_line(&rc.to_string()).unwrap()).unwrap();
    assert!(r["error"]["message"].as_str().unwrap().contains("finish_working"), "{r}");
}

#[test]
fn sessions_are_reported_and_each_request_names_its_known_session() {
    let (s, log) = recorder(&[]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    let mut s = s.with_session_hook(move |e| {
        ev.lock().unwrap().push(match e {
            SessionEvent::Started { id, info } => format!("start {id} {}", info["name"].as_str().unwrap_or("")),
            SessionEvent::Ended { id } => format!("end {id}"),
        })
    });
    let (_, id) = s.handle_session(&init("codex-mcp-client"), None);
    let id = id.unwrap();
    s.handle_session(&finish(json!([])), Some(&id));
    assert_eq!(log.lock().unwrap().calls.last().unwrap().2.as_deref(), Some(id.as_str()), "the request's session");
    s.handle_session(&finish(json!([])), Some("unknown"));
    assert_eq!(log.lock().unwrap().calls.last().unwrap().2, None, "an unknown session is none");
    assert!(s.end_session(&id));
    assert!(!s.end_session(&id), "once");
    assert_eq!(*events.lock().unwrap(), vec![format!("start {id} codex-mcp-client"), format!("end {id}")]);
}

#[test]
fn an_attached_server_tells_the_app_who_it_serves() {
    let (s, log) = recorder(&[]);
    let mut s = s.announcing_clients();
    s.handle_line(&init("codex-mcp-client"));
    let (method, params, _) = log.lock().unwrap().calls.first().cloned().unwrap();
    assert_eq!((method.as_str(), params["name"].as_str()), ("mcp.client", Some("codex-mcp-client")));
    // A plain server doesn't.
    let (mut s, log) = recorder(&[]);
    s.handle_line(&init("codex-mcp-client"));
    assert!(log.lock().unwrap().calls.is_empty());
}

fn http(port: u16, request: &str) -> (String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(request.as_bytes()).unwrap();
    let mut r = BufReader::new(s);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        head.push_str(&line);
    }
    let mut rest = String::new();
    let _ = r.read_to_string(&mut rest);
    (head, rest)
}

#[test]
fn delete_ends_an_http_session() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let ended = Arc::new(Mutex::new(Vec::new()));
    let e2 = ended.clone();
    let ep = Endpoint::serve(listener, Arc::new(RwLock::new(Config { token: None })), move || {
        Server::headless().with_session_hook(move |e| {
            if let SessionEvent::Ended { id } = e {
                e2.lock().unwrap().push(id.to_string());
            }
        })
    })
    .unwrap();
    let p = ep.port;
    let body = init("codex-mcp-client");
    let (head, _) = http(
        p,
        &format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    );
    let id = head.lines().find_map(|l| l.strip_prefix("Mcp-Session-Id: ")).expect("a session").trim().to_string();
    let delete = |sid: Option<&str>| {
        let h = sid.map(|s| format!("Mcp-Session-Id: {s}\r\n")).unwrap_or_default();
        http(p, &format!("DELETE /mcp HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\n{h}Connection: close\r\n\r\n")).0
    };
    assert!(delete(Some(&id)).starts_with("HTTP/1.1 200"), "ended");
    assert!(delete(Some(&id)).starts_with("HTTP/1.1 404"), "not twice");
    assert!(delete(None).starts_with("HTTP/1.1 400"), "the header is needed");
    assert_eq!(*ended.lock().unwrap(), vec![id]);
}
