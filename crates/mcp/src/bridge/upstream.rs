//! The bridge's HTTP client: one request to the app's MCP endpoint per connection, loopback only,
//! std only. Scrim's endpoint answers JSON with a `Content-Length`; a `text/event-stream` answer is
//! read too (its `data:` lines), in case.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::Value;

/// Connecting gives up after this (the app isn't there).
const CONNECT: Duration = Duration::from_millis(800);
/// An answer may take this long (a big export or render).
const ANSWER: Duration = Duration::from_secs(300);
/// The most bytes an answer may have (a screenshot is a few MB).
const MAX_BODY: usize = 64 << 20;

/// What a request got.
#[derive(Debug)]
pub enum Reply {
    /// The status, the `Mcp-Session-Id` it sent, and the JSON-RPC message (none for 202).
    Answer { status: u16, session: Option<String>, body: Option<Value> },
    /// Nothing listens there, or the connection broke: the app isn't running (or just stopped).
    Down(String),
}

/// `POST /mcp` with `body` to `addr`, in `session`, with `auth` (`Bearer …`) when set.
pub fn post(addr: SocketAddr, session: Option<&str>, auth: Option<&str>, body: &Value) -> Reply {
    send(addr, "POST", session, auth, Some(body))
}

/// `DELETE /mcp`: end `session` (the bridge is closing).
pub fn delete(addr: SocketAddr, session: &str, auth: Option<&str>) -> Reply {
    send(addr, "DELETE", Some(session), auth, None)
}

fn send(addr: SocketAddr, method: &str, session: Option<&str>, auth: Option<&str>, body: Option<&Value>) -> Reply {
    let mut s = match TcpStream::connect_timeout(&addr, CONNECT) {
        Ok(s) => s,
        Err(e) => return Reply::Down(e.to_string()),
    };
    let _ = s.set_read_timeout(Some(ANSWER));
    let _ = s.set_write_timeout(Some(ANSWER));
    let body = body.map(Value::to_string).unwrap_or_default();
    let mut head = format!("{method} /mcp HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json, text/event-stream\r\nConnection: close\r\n");
    if method == "POST" {
        head.push_str("Content-Type: application/json\r\n");
    }
    if let Some(id) = session {
        head.push_str(&format!("Mcp-Session-Id: {id}\r\n"));
    }
    if let Some(a) = auth {
        head.push_str(&format!("Authorization: {a}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    if let Err(e) = s.write_all(head.as_bytes()).and_then(|()| s.write_all(body.as_bytes())) {
        return Reply::Down(e.to_string());
    }
    read_reply(&mut BufReader::new(s))
}

fn read_reply(r: &mut impl BufRead) -> Reply {
    let mut line = String::new();
    if r.read_line(&mut line).map_or(true, |n| n == 0) {
        return Reply::Down("the connection closed without an answer".into());
    }
    let status = line.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
    let (mut len, mut session, mut chunked, mut sse) = (None, None, false, false);
    loop {
        let mut h = String::new();
        match r.read_line(&mut h) {
            Ok(0) | Err(_) => return Reply::Down("the answer's headers were cut off".into()),
            Ok(_) => {}
        }
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        let Some((k, v)) = h.split_once(':') else { continue };
        let v = v.trim();
        match k.trim().to_ascii_lowercase().as_str() {
            "content-length" => len = v.parse::<usize>().ok(),
            "mcp-session-id" => session = Some(v.to_string()),
            "transfer-encoding" => chunked = v.eq_ignore_ascii_case("chunked"),
            "content-type" => sse = v.starts_with("text/event-stream"),
            _ => {}
        }
    }
    let bytes = match (chunked, len) {
        (true, _) => read_chunked(r),
        (false, Some(n)) if n <= MAX_BODY => {
            let mut b = vec![0; n];
            r.read_exact(&mut b).map(|()| b).map_err(|e| e.to_string())
        }
        (false, Some(_)) => Err("the answer is too large".into()),
        (false, None) => {
            let mut b = Vec::new();
            r.take(MAX_BODY as u64).read_to_end(&mut b).map(|_| b).map_err(|e| e.to_string())
        }
    };
    let bytes = match bytes {
        Ok(b) => b,
        Err(e) => return Reply::Down(e),
    };
    let text = String::from_utf8_lossy(&bytes);
    let body = if sse {
        // The last `data:` message that parses (a JSON-RPC response ends the stream).
        text.lines().filter_map(|l| l.strip_prefix("data:")).filter_map(|d| serde_json::from_str(d.trim()).ok()).next_back()
    } else {
        serde_json::from_str(text.trim()).ok()
    };
    Reply::Answer { status, session, body }
}

fn read_chunked(r: &mut impl BufRead) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let mut size = String::new();
        r.read_line(&mut size).map_err(|e| e.to_string())?;
        let n = usize::from_str_radix(size.trim().split(';').next().unwrap_or("0"), 16).map_err(|e| e.to_string())?;
        if out.len() + n > MAX_BODY {
            return Err("the answer is too large".into());
        }
        let mut chunk = vec![0; n + 2];
        r.read_exact(&mut chunk).map_err(|e| e.to_string())?;
        chunk.truncate(n);
        out.extend(chunk);
        if n == 0 {
            return Ok(out);
        }
    }
}
