//! Fix P: `scrim mcp-bridge` waits for the app instead of exiting. With the app's endpoint down it
//! starts, answers `initialize`, lists the noted tools, answers a tool call with "Scrim is not
//! running", and stays; once the endpoint is up the next call goes through and the client is told
//! the tool list changed; it follows the app to a new port; it ends when stdin closes (ending its
//! session there); the token never shows in its output or log; the client's name is kept.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use figcraft_mcp::bridge::{Config, NOT_RUNNING, REFUSED, run};
use figcraft_mcp::http::{Config as HttpConfig, Endpoint};
use figcraft_mcp::{Server, SessionEvent};
use serde_json::{Value, json};

/// The client's stdout, as the bridge's stdin: lines sent down a channel; closed when dropped.
struct Input {
    rx: Receiver<String>,
    buf: Vec<u8>,
}

impl Read for Input {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.buf.is_empty() {
            match self.rx.recv() {
                Ok(line) => self.buf = format!("{line}\n").into_bytes(),
                Err(_) => return Ok(0),
            }
        }
        let n = out.len().min(self.buf.len());
        out[..n].copy_from_slice(&self.buf[..n]);
        self.buf.drain(..n);
        Ok(n)
    }
}

/// The bridge's stdout, line by line.
struct Output(Sender<String>, Vec<u8>);

impl Write for Output {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.1.extend_from_slice(b);
        while let Some(i) = self.1.iter().position(|c| *c == b'\n') {
            let line: Vec<u8> = self.1.drain(..=i).collect();
            let _ = self.0.send(String::from_utf8_lossy(&line).trim().to_string());
        }
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What the app's endpoint saw: the sessions started (with the client's name) and ended.
#[derive(Default)]
struct Seen {
    started: Vec<String>,
    ended: Vec<String>,
}

fn app(listener: TcpListener, token: Option<&str>, seen: Arc<Mutex<Seen>>) -> Endpoint {
    let config = Arc::new(RwLock::new(HttpConfig { token: token.map(str::to_string) }));
    Endpoint::serve(listener, config, move || {
        Server::headless().with_session_hook(move |e| {
            let mut s = seen.lock().unwrap();
            match e {
                SessionEvent::Started { info, .. } => s.started.push(info["name"].as_str().unwrap_or("").to_string()),
                SessionEvent::Ended { id } => s.ended.push(id.to_string()),
            }
        })
    })
    .unwrap()
}

/// A loopback port nothing listens on (bound, then let go).
fn free() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap()
}

struct Bridge {
    tx: Option<Sender<String>>,
    rx: Receiver<String>,
    log: Arc<Mutex<Vec<String>>>,
    at: Arc<Mutex<SocketAddr>>,
    done: Option<std::thread::JoinHandle<()>>,
    seen_lines: Vec<String>,
}

impl Bridge {
    fn start(at: SocketAddr, auth: Option<&str>, tools: Option<Value>) -> Bridge {
        let (tx, in_rx) = channel();
        let (out_tx, rx) = channel();
        let log = Arc::new(Mutex::new(Vec::new()));
        let at = Arc::new(Mutex::new(at));
        let (l, a) = (log.clone(), at.clone());
        let cfg = Config {
            endpoint: Box::new(move || *a.lock().unwrap()),
            tools: Box::new(move || tools.clone()),
            auth: auth.map(str::to_string),
            retry: Duration::from_millis(100),
            log: Box::new(move |m| l.lock().unwrap().push(m.to_string())),
        };
        let done = std::thread::spawn(move || {
            run(BufReader::new(Input { rx: in_rx, buf: Vec::new() }), Box::new(Output(out_tx, Vec::new())), cfg).unwrap();
        });
        Bridge { tx: Some(tx), rx, log, at, done: Some(done), seen_lines: Vec::new() }
    }

    fn send(&self, msg: Value) {
        self.tx.as_ref().unwrap().send(msg.to_string()).unwrap();
    }

    /// The next line the bridge writes that matches `pick`, within `secs`.
    fn wait(&mut self, secs: u64, pick: impl Fn(&Value) -> bool) -> Value {
        let end = std::time::Instant::now() + Duration::from_secs(secs);
        loop {
            let left = end.saturating_duration_since(std::time::Instant::now());
            let line = self.rx.recv_timeout(left).unwrap_or_else(|_| panic!("nothing matching within {secs} s; saw {:?}", self.seen_lines));
            self.seen_lines.push(line.clone());
            let v: Value = serde_json::from_str(&line).unwrap();
            if pick(&v) {
                return v;
            }
        }
    }

    fn answer(&mut self, id: u64) -> Value {
        self.wait(10, |v| v["id"] == id)
    }

    fn close(mut self) -> (Vec<String>, Vec<String>) {
        drop(self.tx.take());
        self.done.take().unwrap().join().unwrap();
        let mut lines = self.seen_lines.clone();
        lines.extend(self.rx.try_iter());
        (lines, self.log.lock().unwrap().clone())
    }
}

fn init(b: &Bridge, name: &str) {
    b.send(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": name, "version": "0.1.0"}}}));
}

fn call(b: &Bridge, id: u64, tool: &str) {
    b.send(json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": tool, "arguments": {}}}));
}

fn text(r: &Value) -> String {
    r["result"]["content"][0]["text"].as_str().unwrap_or("").to_string()
}

#[test]
fn with_the_app_down_it_waits_then_proxies_once_it_is_up() {
    let at = free();
    let noted = json!({"tools": [{"name": "inspect_document", "description": "noted", "inputSchema": {"type": "object"}}]});
    let mut b = Bridge::start(at, None, Some(noted.clone()));
    init(&b, "claude-ai");
    let r = b.answer(1);
    assert_eq!(r["result"]["serverInfo"]["name"], "scrim");
    assert_eq!(r["result"]["capabilities"]["tools"]["listChanged"], true);
    b.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    b.send(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    assert_eq!(b.answer(2)["result"], noted, "the tools the app last noted");
    call(&b, 3, "inspect_document");
    let r = b.answer(3);
    assert_eq!((r["result"]["isError"].as_bool(), text(&r)), (Some(true), NOT_RUNNING.to_string()));
    std::thread::sleep(Duration::from_millis(400));
    b.send(json!({"jsonrpc": "2.0", "id": 4, "method": "ping"}));
    assert_eq!(b.answer(4)["result"], json!({}), "still alive");
    // The app comes up on that port: the waiting bridge finds it and says the list changed.
    let seen = Arc::new(Mutex::new(Seen::default()));
    let mut endpoint = app(TcpListener::bind(at).unwrap(), None, seen.clone());
    b.wait(5, |v| v["method"] == "notifications/tools/list_changed");
    call(&b, 5, "inspect_document");
    let r = b.answer(5);
    assert_eq!(r["result"]["isError"], false, "{r}");
    assert!(text(&r).contains("\"pages\"") || text(&r).contains("Page"), "the app's document: {r}");
    assert_eq!(seen.lock().unwrap().started, ["claude-ai (via scrim mcp-bridge)"], "the client's name, and the bridge's");
    b.send(json!({"jsonrpc": "2.0", "id": 6, "method": "tools/list"}));
    assert!(b.answer(6)["result"]["tools"].as_array().unwrap().len() > 5, "the app's own list now");
    let (_, log) = b.close();
    assert_eq!(seen.lock().unwrap().ended.len(), 1, "stdin closed: its session ended at the app");
    assert!(log.iter().any(|l| l.contains("waiting")) && log.iter().any(|l| l.contains("connected")), "{log:?}");
    endpoint.stop();
}

#[test]
fn it_follows_the_app_to_a_new_port() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let at = first.local_addr().unwrap();
    let mut one = app(first, None, seen.clone());
    let mut b = Bridge::start(at, None, None);
    init(&b, "local-agent-mode-scrim");
    b.answer(1);
    call(&b, 2, "inspect_document");
    assert_eq!(b.answer(2)["result"]["isError"], false);
    // The app quits, and comes back on another port (its fallback): the note says where.
    one.stop();
    let second = TcpListener::bind("127.0.0.1:0").unwrap();
    *b.at.lock().unwrap() = second.local_addr().unwrap();
    let mut two = app(second, None, seen.clone());
    call(&b, 3, "inspect_document");
    let r = b.answer(3);
    assert_eq!(r["result"]["isError"], false, "proxied to the new port: {r}");
    assert!(b.seen_lines.iter().any(|l| l.contains("notifications/tools/list_changed")), "told the list changed");
    assert_eq!(seen.lock().unwrap().started.len(), 2, "a session at each");
    b.close();
    two.stop();
}

#[test]
fn the_token_is_sent_never_shown_and_a_wrong_one_is_said_plainly() {
    let token = "s3cret-token-6f1c2b9a7e";
    let seen = Arc::new(Mutex::new(Seen::default()));
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let at = l.local_addr().unwrap();
    let mut endpoint = app(l, Some(token), seen.clone());
    let mut b = Bridge::start(at, Some(&format!("Bearer {token}")), None);
    init(&b, "claude-ai");
    b.answer(1);
    call(&b, 2, "inspect_document");
    assert_eq!(b.answer(2)["result"]["isError"], false, "the token let it in");
    let (lines, log) = b.close();
    for l in lines.iter().chain(&log) {
        assert!(!l.contains(token), "the token showed: {l}");
    }
    // A wrong token: refused, and said so.
    let mut b = Bridge::start(at, Some("Bearer wrong"), None);
    init(&b, "claude-ai");
    b.answer(1);
    call(&b, 2, "inspect_document");
    let r = b.answer(2);
    assert_eq!((r["result"]["isError"].as_bool(), text(&r)), (Some(true), REFUSED.to_string()));
    let (_, log) = b.close();
    assert!(log.iter().any(|l| l.contains("refused the token")) && !log.iter().any(|l| l.contains("wrong")), "{log:?}");
    endpoint.stop();
}

#[test]
fn closing_stdin_ends_the_bridge_even_while_it_waits() {
    let b = Bridge::start(free(), None, None);
    init(&b, "claude-ai");
    let (lines, _) = b.close();
    assert!(lines.iter().any(|l| l.contains("\"serverInfo\"")), "{lines:?}");
}

#[test]
fn the_app_notes_where_it_is_and_what_it_lists_and_takes_the_note_away() {
    use figcraft_mcp::discovery::{ENDPOINT_FILE, endpoint, remove_endpoint, tools, write_endpoint, write_tools};
    let dir = std::env::temp_dir().join(format!("scrim-discovery-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&dir);
    // Nothing noted, nothing saved: the default port.
    assert_eq!(endpoint(&dir), SocketAddr::from(([127, 0, 0, 1], 7680)));
    // Noted (atomically: no temporary file is left).
    write_endpoint(&dir, 7681).unwrap();
    assert_eq!(endpoint(&dir), SocketAddr::from(([127, 0, 0, 1], 7681)));
    let names: Vec<String> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, [ENDPOINT_FILE]);
    let noted: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(ENDPOINT_FILE)).unwrap()).unwrap();
    assert_eq!(noted, json!({"address": "127.0.0.1", "port": 7681, "pid": std::process::id()}), "no token in it");
    // Another instance's note isn't ours to remove; ours is.
    std::fs::write(dir.join(ENDPOINT_FILE), json!({"address": "127.0.0.1", "port": 7690, "pid": 1}).to_string()).unwrap();
    remove_endpoint(&dir);
    assert_eq!(endpoint(&dir).port(), 7690);
    write_endpoint(&dir, 7682).unwrap();
    remove_endpoint(&dir);
    assert!(!dir.join(ENDPOINT_FILE).exists());
    // A note naming another computer is ignored; the port saved in Settings is the fallback.
    std::fs::write(dir.join(ENDPOINT_FILE), json!({"address": "10.0.0.5", "port": 7700}).to_string()).unwrap();
    std::fs::write(dir.join("state.json"), json!({"prefs": {"settings": {"mcp": {"port": 7699}}}}).to_string()).unwrap();
    let saved = figcraft_engine::settings::saved_mcp(&dir).port;
    assert_eq!(endpoint(&dir), SocketAddr::from(([127, 0, 0, 1], saved)), "only loopback");
    // The tools the app listed.
    assert!(tools(&dir).is_none());
    write_tools(&dir, &json!({"tools": [{"name": "batch"}]})).unwrap();
    assert_eq!(tools(&dir).unwrap()["tools"][0]["name"], "batch");
    let _ = std::fs::remove_dir_all(&dir);
}
