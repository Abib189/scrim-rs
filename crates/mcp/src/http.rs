//! MCP over streamable HTTP, in its JSON-response form, on the loopback interface only
//! (`docs/design/settings-and-connect.md`, C1). The running app serves it so Claude Code (or any MCP
//! client) connects with one URL, `http://127.0.0.1:<port>/mcp`:
//! - `POST /mcp` carries one JSON-RPC message; the reply is `application/json`, or `202` for a
//!   notification;
//! - `GET` and `DELETE` are `405`: there is no server-initiated stream and no session to end.
//!
//! Every request is checked before its body is read:
//! - **Host** must name the loopback address and this port (`127.0.0.1`, `localhost` or `[::1]`), which
//!   defeats DNS rebinding;
//! - an **Origin**, when a browser sends one, must be this server's own, which defeats a web page
//!   posting to it (CSRF);
//! - with a **token** set (Settings ▸ MCP ▸ Require authentication), `Authorization: Bearer <token>` is
//!   required.
//!
//! No dependency beyond `std`: HTTP/1.1 with `Content-Length` bodies (no chunked encoding), keep-alive,
//! bounded headers and bodies, and per-connection timeouts. One thread owns the [`Server`]; connection
//! threads hand it messages one at a time, so requests run in order as they do over stdio.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::Server;

/// The path the endpoint answers on.
pub const PATH: &str = "/mcp";
/// The largest body accepted (as over stdio).
pub const MAX_BODY: usize = 32 * 1024 * 1024;
/// The most header bytes accepted.
const MAX_HEADERS: usize = 64 * 1024;
/// A connection idle this long is closed.
const IDLE: Duration = Duration::from_secs(120);
/// How long a request may wait for the server thread (the app answers on its UI thread).
const ANSWER: Duration = Duration::from_secs(120);
/// The most connections served at once.
const MAX_CONNECTIONS: usize = 16;

/// What the endpoint checks requests against; changed while it runs (Settings).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// `Some`: requests need `Authorization: Bearer <token>`.
    pub token: Option<String>,
}

/// One parsed HTTP request.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
    fn wants_close(&self) -> bool {
        self.header("connection").is_some_and(|v| v.eq_ignore_ascii_case("close"))
    }
}

/// Why a request is turned away: the status and a short reason (also the body).
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal(pub u16, pub &'static str);

/// The checks made before a request's body is read (see the module docs).
pub fn check(req: &Request, port: u16, config: &Config) -> Result<(), Refusal> {
    let host = req.header("host").ok_or(Refusal(403, "a Host header naming 127.0.0.1 is required"))?;
    if !loopback_authority(host, port) {
        return Err(Refusal(403, "Host must be 127.0.0.1, localhost or [::1] with this port"));
    }
    if let Some(origin) = req.header("origin")
        && !own_origin(origin, port)
    {
        return Err(Refusal(403, "requests from a web page are refused (Origin is not this server)"));
    }
    if let Some(token) = &config.token {
        let given = req.header("authorization").and_then(|v| v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer "))).unwrap_or("");
        if !constant_time_eq(given.trim().as_bytes(), token.as_bytes()) {
            return Err(Refusal(401, "authentication is required: Authorization: Bearer <token> (Settings > MCP)"));
        }
    }
    Ok(())
}

/// `host:port` naming the loopback interface and `port` exactly.
fn loopback_authority(value: &str, port: u16) -> bool {
    let value = value.trim().to_ascii_lowercase();
    ["127.0.0.1", "localhost", "[::1]"].iter().any(|h| value == format!("{h}:{port}"))
}

/// An `Origin` that is this server itself (`http://127.0.0.1:<port>` and the other loopback names).
fn own_origin(value: &str, port: u16) -> bool {
    value.trim().to_ascii_lowercase().strip_prefix("http://").is_some_and(|rest| loopback_authority(rest, port))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Read one request; `Ok(None)` at a clean end of the connection. Bodies over [`MAX_BODY`] and
/// chunked bodies are refused before they are read.
pub fn read_request(reader: &mut impl BufRead) -> Result<Option<Request>, Refusal> {
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let n = reader
            .take((MAX_HEADERS - head.len().min(MAX_HEADERS)) as u64 + 1)
            .read_until(b'\n', &mut line)
            .map_err(|_| Refusal(400, "unreadable request"))?;
        if n == 0 {
            return if head.is_empty() { Ok(None) } else { Err(Refusal(400, "the request ended early")) };
        }
        if head.len() + line.len() > MAX_HEADERS {
            return Err(Refusal(431, "request headers too large"));
        }
        if line == b"\r\n" || line == b"\n" {
            if head.is_empty() {
                continue; // a stray blank line between requests
            }
            break;
        }
        head.extend_from_slice(&line);
    }
    let head = String::from_utf8(head).map_err(|_| Refusal(400, "headers must be text"))?;
    let mut lines = head.lines();
    let first = lines.next().unwrap_or_default();
    let mut parts = first.split_whitespace();
    let (method, path, version) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
    if !version.starts_with("HTTP/1.") {
        return Err(Refusal(400, "HTTP/1.1 is required"));
    }
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
    let mut req = Request { method: method.to_string(), path: path.to_string(), headers, body: Vec::new() };
    if req.header("transfer-encoding").is_some() {
        return Err(Refusal(411, "send the body with Content-Length"));
    }
    let len = match req.header("content-length") {
        Some(v) => v.parse::<usize>().map_err(|_| Refusal(400, "bad Content-Length"))?,
        None => 0,
    };
    if len > MAX_BODY {
        return Err(Refusal(413, "message too large"));
    }
    req.body = vec![0; len];
    reader.read_exact(&mut req.body).map_err(|_| Refusal(400, "the body ended early"))?;
    Ok(Some(req))
}

/// Write one response.
fn respond(out: &mut impl Write, status: u16, content_type: Option<&str>, body: &[u8], extra: &[(&str, &str)]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let mut head = format!("HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n", body.len());
    if let Some(t) = content_type {
        head.push_str(&format!("Content-Type: {t}\r\n"));
    }
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    out.write_all(head.as_bytes())?;
    out.write_all(body)?;
    out.flush()
}

/// A JSON-RPC message for the server thread and where its reply goes (`None`: no reply).
type Job = (String, Sender<Option<String>>);

/// Serve one connection until it closes: requests checked, `POST /mcp` answered by `answer`.
pub fn serve_connection(
    stream: TcpStream,
    port: u16,
    config: &RwLock<Config>,
    answer: &dyn Fn(String) -> Option<Option<String>>,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(IDLE))?;
    stream.set_write_timeout(Some(IDLE))?;
    let mut out = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    loop {
        let req = match read_request(&mut reader) {
            Ok(Some(r)) => r,
            Ok(None) => return Ok(()),
            Err(Refusal(status, why)) => {
                respond(&mut out, status, Some("text/plain; charset=utf-8"), why.as_bytes(), &[("Connection", "close")])?;
                return Ok(());
            }
        };
        let close = req.wants_close();
        let checked = config.read().map(|c| check(&req, port, &c)).unwrap_or(Err(Refusal(503, "the endpoint's settings are unreadable")));
        if let Err(Refusal(status, why)) = checked {
            respond(&mut out, status, Some("text/plain; charset=utf-8"), why.as_bytes(), &[("Connection", "close")])?;
            return Ok(());
        }
        match (req.method.as_str(), req.path.split('?').next().unwrap_or_default()) {
            ("POST", PATH) => match answer(String::from_utf8_lossy(&req.body).into_owned()) {
                Some(Some(reply)) => respond(&mut out, 200, Some("application/json"), reply.as_bytes(), &[])?,
                Some(None) => respond(&mut out, 202, None, b"", &[])?,
                None => respond(&mut out, 503, Some("text/plain; charset=utf-8"), b"the app didn't answer in time", &[])?,
            },
            ("GET" | "DELETE", PATH) => {
                respond(&mut out, 405, Some("text/plain; charset=utf-8"), b"POST JSON-RPC messages to /mcp", &[("Allow", "POST")])?
            }
            _ => respond(&mut out, 404, Some("text/plain; charset=utf-8"), b"the MCP endpoint is /mcp", &[])?,
        }
        if close {
            return Ok(());
        }
    }
}

/// A running endpoint. Dropping it (or [`Endpoint::stop`]) stops accepting connections.
pub struct Endpoint {
    pub port: u16,
    stopped: Arc<AtomicBool>,
    accept: Option<std::thread::JoinHandle<()>>,
}

impl Endpoint {
    /// Listen on 127.0.0.1, at `port` or, when it's taken, the next free one within `tries` ports.
    /// `make` builds the [`Server`] on the thread that owns it (backends needn't be `Send`).
    pub fn start(port: u16, tries: u16, config: Arc<RwLock<Config>>, make: impl FnOnce() -> Server + Send + 'static) -> Result<Endpoint, String> {
        let mut last = String::new();
        for p in (0..tries.max(1)).filter_map(|i| port.checked_add(i)) {
            match TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], p))) {
                Ok(listener) => return Self::serve(listener, config, make),
                Err(e) => last = format!("127.0.0.1:{p}: {e}"),
            }
        }
        Err(format!("no free port from {port}: {last}"))
    }

    /// Serve on `listener` (bound by the caller; tests bind port 0).
    pub fn serve(listener: TcpListener, config: Arc<RwLock<Config>>, make: impl FnOnce() -> Server + Send + 'static) -> Result<Endpoint, String> {
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let stopped = Arc::new(AtomicBool::new(false));
        let (jobs, inbox): (Sender<Job>, Receiver<Job>) = channel();
        std::thread::Builder::new()
            .name("scrim-mcp-server".into())
            .spawn(move || {
                let mut server = make();
                for (message, reply) in inbox {
                    let _ = reply.send(server.handle_line(&message));
                }
            })
            .map_err(|e| e.to_string())?;
        let stop = stopped.clone();
        let accept = std::thread::Builder::new()
            .name("scrim-mcp-http".into())
            .spawn(move || {
                let open = Arc::new(AtomicUsize::new(0));
                for stream in listener.incoming() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    if open.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
                        let mut s = stream;
                        let _ = respond(&mut s, 503, Some("text/plain; charset=utf-8"), b"too many connections", &[("Connection", "close")]);
                        continue;
                    }
                    let (jobs, config, mine) = (jobs.clone(), config.clone(), open.clone());
                    open.fetch_add(1, Ordering::Relaxed);
                    let spawned = std::thread::Builder::new().name("scrim-mcp-conn".into()).spawn(move || {
                        let answer = |message: String| {
                            let (tx, rx) = channel();
                            jobs.send((message, tx)).ok()?;
                            rx.recv_timeout(ANSWER).ok()
                        };
                        let _ = serve_connection(stream, port, &config, &answer);
                        mine.fetch_sub(1, Ordering::Relaxed);
                    });
                    if spawned.is_err() {
                        open.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Endpoint { port, stopped, accept: Some(accept) })
    }

    /// Stop accepting connections and close the port (open connections finish their current
    /// request), so a restart can listen on the same port again.
    pub fn stop(&mut self) {
        if !self.stopped.swap(true, Ordering::Relaxed) {
            // Wake the accept loop so it sees the flag, then wait for it to drop the listener.
            let _ = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], self.port)), Duration::from_millis(200));
            if let Some(t) = self.accept.take() {
                let _ = t.join();
            }
        }
    }

    /// The URL a client connects to.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}{PATH}", self.port)
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        self.stop();
    }
}
