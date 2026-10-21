//! Stage J: several MCP clients on the app's one HTTP endpoint, each request attributed to the
//! client it came from. `initialize` issues an `Mcp-Session-Id`; a request that sends it back is
//! that client's; one without a known session is taken to be the newest client's. The backend is
//! told before every request (`Backend::set_client`), so the app can name the client of each edit.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};

use figcraft_mcp::http::{Config, Endpoint};
use figcraft_mcp::{Backend, Server};
use serde_json::{Value, json};

/// A backend that remembers which client each call came from.
struct Recorder {
    client: Option<String>,
    calls: Arc<Mutex<Vec<Option<String>>>>,
}

impl Backend for Recorder {
    fn call(&mut self, _method: &str, _params: Value) -> Result<Value, String> {
        self.calls.lock().unwrap().push(self.client.clone());
        Ok(json!({"commands": []}))
    }
    fn set_client(&mut self, client: Option<&str>) {
        self.client = client.map(str::to_string);
    }
    fn has_ui(&self) -> bool {
        false
    }
    fn describe(&self) -> String {
        "a recorder".into()
    }
}

fn init(name: &str) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": name, "version": "1"}}})
        .to_string()
}

fn call() -> String {
    json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "list_commands", "arguments": {}}}).to_string()
}

#[test]
fn each_request_is_attributed_to_its_sessions_client() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut s = Server::new(Box::new(Recorder { client: None, calls: calls.clone() }));
    let (_, codex) = s.handle_session(&init("codex-mcp-client"), None);
    let (_, gemini) = s.handle_session(&init("gemini-cli-mcp-client"), None);
    let (codex, gemini) = (codex.expect("a session for Codex"), gemini.expect("and one for Gemini"));
    assert_ne!(codex, gemini);
    assert!(codex.len() >= 32 && codex.chars().all(|c| c.is_ascii_hexdigit()), "visible ASCII, unguessable: {codex}");
    // Each tool call's client (a tool may call the backend more than once: all the same client).
    let mut got: Vec<Option<String>> = Vec::new();
    // No session, or one the server doesn't know (it restarted): the newest client.
    for session in [Some(codex.as_str()), Some(gemini.as_str()), Some(codex.as_str()), None, Some("stale")] {
        let before = calls.lock().unwrap().len();
        s.handle_session(&call(), session);
        let these: Vec<Option<String>> = calls.lock().unwrap()[before..].to_vec();
        assert!(!these.is_empty() && these.windows(2).all(|w| w[0] == w[1]), "{these:?}");
        got.push(these[0].clone());
    }
    let want = ["codex-mcp-client", "gemini-cli-mcp-client", "codex-mcp-client", "gemini-cli-mcp-client", "gemini-cli-mcp-client"];
    assert_eq!(got, want.iter().map(|n| Some(n.to_string())).collect::<Vec<_>>());
    // A client that announced no name is nameless, not someone else.
    let (_, anon) =
        s.handle_session(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}).to_string(), None);
    s.handle_session(&call(), anon.as_deref());
    assert_eq!(calls.lock().unwrap().last().cloned(), Some(None));
}

#[test]
fn stdio_issues_no_session() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut s = Server::new(Box::new(Recorder { client: None, calls }));
    // `handle_line` (stdio: one client per process) answers as before, with no session.
    let r: Value = serde_json::from_str(&s.handle_line(&init("codex-mcp-client")).unwrap()).unwrap();
    assert_eq!(r["result"]["serverInfo"]["name"], "scrim");
    let (_, issued) = s.handle_session(&call(), None);
    assert_eq!(issued, None, "only an initialize issues one");
}

/// The real endpoint sends the session in the `Mcp-Session-Id` header of `initialize`'s reply.
#[test]
fn the_http_endpoint_sends_the_session_header() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let config = Arc::new(RwLock::new(Config { token: None }));
    let ep = Endpoint::serve(listener, config, Server::headless).unwrap();
    let body = init("codex-mcp-client");
    let req = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len(),
        p = ep.port
    );
    let mut s = TcpStream::connect(("127.0.0.1", ep.port)).unwrap();
    s.write_all(req.as_bytes()).unwrap();
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
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let session = head.lines().find_map(|l| l.strip_prefix("Mcp-Session-Id: ")).expect("the session header");
    assert!(session.len() >= 32, "{session}");
}
