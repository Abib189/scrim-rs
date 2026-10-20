//! MCP over streamable HTTP (`figcraft_mcp::http`): the protocol round trip, and the requests the
//! endpoint must refuse — another Host (DNS rebinding), a web page's Origin (CSRF), a missing or
//! wrong token, oversized or chunked bodies — plus the port fallback.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, RwLock};

use figcraft_mcp::Server;
use figcraft_mcp::http::{Config, Endpoint};
use serde_json::{Value, json};

fn endpoint(token: Option<&str>) -> Endpoint {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let config = Arc::new(RwLock::new(Config { token: token.map(str::to_string) }));
    Endpoint::serve(listener, config, Server::headless).unwrap()
}

/// Send one raw request and read the status line, headers and body.
fn send(port: u16, request: &str) -> (u16, Vec<(String, String)>, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(request.as_bytes()).unwrap();
    let mut r = BufReader::new(s);
    let mut status = String::new();
    r.read_line(&mut status).unwrap();
    let code = status.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let len = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    let mut body = vec![0; len];
    r.read_exact(&mut body).unwrap();
    (code, headers, String::from_utf8_lossy(&body).into_owned())
}

fn post(port: u16, host: &str, extra: &str, body: &Value) -> (u16, String) {
    let body = body.to_string();
    let req = format!(
        "POST /mcp HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (code, _, body) = send(port, &req);
    (code, body)
}

fn init() -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}})
}

#[test]
fn the_mcp_round_trip_over_http() {
    let e = endpoint(None);
    let host = format!("127.0.0.1:{}", e.port);
    let (code, body) = post(e.port, &host, "", &init());
    assert_eq!(code, 200, "{body}");
    let r: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(r["result"]["serverInfo"]["name"], "scrim");
    // A notification has no reply: 202.
    let (code, body) = post(e.port, &host, "", &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    assert_eq!((code, body.as_str()), (202, ""));
    let (code, body) = post(e.port, &format!("localhost:{}", e.port), "", &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    assert_eq!(code, 200);
    let tools: Value = serde_json::from_str(&body).unwrap();
    assert!(tools["result"]["tools"].as_array().is_some_and(|t| t.len() > 10));
    let (code, body) = post(
        e.port,
        &host,
        "",
        &json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "run_command", "arguments": {"command": "shape.rectangle", "params": {"x": 0, "y": 0, "width": 10, "height": 10}}}}),
    );
    assert_eq!(code, 200);
    assert!(body.contains("\"isError\":false"), "{body}");
    // A keep-alive connection serves several requests.
    let one = init().to_string();
    let req = format!("POST /mcp HTTP/1.1\r\nHost: {host}\r\nContent-Length: {}\r\n\r\n{one}", one.len());
    let mut s = TcpStream::connect(("127.0.0.1", e.port)).unwrap();
    s.write_all(format!("{req}{req}").as_bytes()).unwrap();
    s.shutdown(std::net::Shutdown::Write).unwrap();
    let mut all = String::new();
    s.read_to_string(&mut all).unwrap();
    assert_eq!(all.matches("HTTP/1.1 200 OK").count(), 2, "{all}");
}

#[test]
fn requests_from_elsewhere_are_refused() {
    let e = endpoint(None);
    let port = e.port;
    let own = format!("127.0.0.1:{port}");
    for (host, extra, why) in [
        ("evil.example".to_string(), String::new(), "a rebound name"),
        (format!("evil.example:{port}"), String::new(), "a rebound name with the port"),
        (format!("192.168.1.20:{port}"), String::new(), "a LAN address"),
        (format!("127.0.0.1:{}", port.wrapping_add(1)), String::new(), "another port"),
        (own.clone(), "Origin: http://evil.example\r\n".to_string(), "a web page's Origin"),
        (own.clone(), format!("Origin: http://evil.example:{port}\r\n"), "a web page's Origin with the port"),
        (own.clone(), "Origin: null\r\n".to_string(), "an opaque Origin"),
        (own.clone(), format!("Origin: https://127.0.0.1:{port}\r\n"), "another scheme"),
    ] {
        let (code, body) = post(port, &host, &extra, &init());
        assert_eq!(code, 403, "{why}: {body}");
        assert!(!body.contains("serverInfo"), "{why}: never answered");
    }
    // No Host at all.
    let body = init().to_string();
    let (code, _, _) = send(port, &format!("POST /mcp HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}", body.len()));
    assert_eq!(code, 403);
    // The server's own Origin is fine (a page it served itself, were there one).
    assert_eq!(post(port, &own, &format!("Origin: http://127.0.0.1:{port}\r\n"), &init()).0, 200);
}

#[test]
fn a_required_token_must_be_sent() {
    let e = endpoint(Some("s3cret-token"));
    let host = format!("127.0.0.1:{}", e.port);
    assert_eq!(post(e.port, &host, "", &init()).0, 401, "no token");
    assert_eq!(post(e.port, &host, "Authorization: Bearer wrong\r\n", &init()).0, 401, "a wrong token");
    assert_eq!(post(e.port, &host, "Authorization: Bearer s3cret-token\r\n", &init()).0, 200, "the token");
}

#[test]
fn other_methods_paths_and_bodies() {
    let e = endpoint(None);
    let host = format!("127.0.0.1:{}", e.port);
    let (code, headers, _) = send(e.port, &format!("GET /mcp HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"));
    assert_eq!(code, 405);
    assert!(headers.iter().any(|(k, v)| k == "allow" && v == "POST"));
    assert_eq!(send(e.port, &format!("POST /other HTTP/1.1\r\nHost: {host}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}")).0, 404);
    // Too large: refused from the header, the body never sent.
    assert_eq!(send(e.port, &format!("POST /mcp HTTP/1.1\r\nHost: {host}\r\nContent-Length: {}\r\n\r\n", 64 * 1024 * 1024)).0, 413);
    assert_eq!(send(e.port, &format!("POST /mcp HTTP/1.1\r\nHost: {host}\r\nTransfer-Encoding: chunked\r\n\r\n")).0, 411);
}

#[test]
fn a_taken_port_falls_back_to_the_next_free_one() {
    let taken = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let e = Endpoint::start(port, 20, Arc::new(RwLock::new(Config::default())), Server::headless).unwrap();
    assert_ne!(e.port, port);
    assert!(e.port > port && e.port <= port + 20, "{} after {port}", e.port);
    assert_eq!(e.url(), format!("http://127.0.0.1:{}/mcp", e.port));
}

#[test]
fn a_restart_listens_on_the_same_port_again() {
    let mut e = endpoint(None);
    let port = e.port;
    e.stop();
    let again = Endpoint::start(port, 1, Arc::new(RwLock::new(Config::default())), Server::headless).unwrap();
    assert_eq!(again.port, port);
}

#[test]
fn initialize_reports_the_client() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let e = Endpoint::serve(listener, Arc::new(RwLock::new(Config::default())), move || {
        Server::headless().with_initialize_hook(move |info| s.lock().unwrap().push(info["name"].as_str().unwrap_or("").to_string()))
    })
    .unwrap();
    post(e.port, &format!("127.0.0.1:{}", e.port), "", &init());
    assert_eq!(*seen.lock().unwrap(), ["test"]);
}
