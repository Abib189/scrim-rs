//! `scrim mcp-bridge` (Fix P, `docs/design/connect-claude-desktop.md` › Fix P): a stdio MCP server
//! that forwards to the running app's HTTP endpoint, and waits for it instead of exiting.
//! - **Where the app is:** asked again before every connection attempt ([`Config::endpoint`]: the
//!   app's `mcp-endpoint.json`, else the port saved in Settings ▸ MCP), so a port change is followed.
//! - **While the app is closed** it stays up and answers locally:
//!   - `initialize` (with `tools.listChanged`), and `tools/list` from the list the app last noted
//!     (`mcp-tools.json`), else an empty one;
//!   - every tool call with a tool error: [`NOT_RUNNING`];
//!   - and it tries the app every [`Config::retry`] (2 s) in the background.
//! - **Once the app is up** it initializes there on the client's behalf, its name as the client
//!   gave it plus [`VIA`], and proxies everything; then it sends `notifications/tools/list_changed`
//!   so the client lists the tools again. The app restarting, on the same port or another, is the
//!   same: the next request (or the retry) finds it.
//! - **The token** (`Bearer …` from `SCRIM_MCP_AUTH`) goes in each request's `Authorization`
//!   header, and is never printed.
//! - **It ends** when stdin closes, ending its session at the app first (`DELETE /mcp`).

use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::server::{INSTRUCTIONS, PROTOCOL_VERSION, SUPPORTED_VERSIONS};

#[path = "bridge/upstream.rs"]
pub mod upstream;

/// What a tool call gets while the app is closed.
pub const NOT_RUNNING: &str = "Scrim is not running. Open the Scrim app and try again.";
/// What a tool call gets when the app refuses the token.
pub const REFUSED: &str = "Scrim refused the bridge's token. In Scrim, use Re-register for Claude Desktop, then restart Claude Desktop.";
/// Appended to the client's name when the bridge initializes at the app.
pub const VIA: &str = " (via scrim mcp-bridge)";
/// The environment variable the token comes in (`Bearer <token>`).
pub const TOKEN_VAR: &str = figcraft_engine::cmd::claude_desktop::TOKEN_VAR;

/// Where the app is, what it lists, and how to reach it.
pub struct Config {
    /// The app's endpoint, asked again before every connection attempt.
    pub endpoint: Box<dyn Fn() -> SocketAddr + Send + Sync>,
    /// The tools the app last listed (`{"tools": […]}`), if it noted them.
    pub tools: Box<dyn Fn() -> Option<Value> + Send + Sync>,
    /// `Bearer <token>` when the app requires one.
    pub auth: Option<String>,
    /// How often to try the app while it's closed.
    pub retry: Duration,
    /// Where progress goes (stderr in the binary): never the token.
    pub log: Box<dyn Fn(&str) + Send + Sync>,
}

impl Config {
    /// The bridge as the binary runs it: the settings folder (`dir`, else the platform's), the
    /// token from [`TOKEN_VAR`], a 2 s retry, progress on stderr.
    pub fn from_env(dir: Option<PathBuf>) -> Config {
        let dir = dir.or_else(crate::discovery::settings_dir);
        let (d1, d2) = (dir.clone(), dir);
        Config {
            endpoint: Box::new(move || d1.as_deref().map_or_else(default_endpoint, crate::discovery::endpoint)),
            tools: Box::new(move || d2.as_deref().and_then(crate::discovery::tools)),
            auth: std::env::var(TOKEN_VAR).ok().filter(|t| !t.trim().is_empty()),
            retry: Duration::from_secs(2),
            log: Box::new(|m| {
                let _ = writeln!(std::io::stderr(), "scrim mcp-bridge: {m}");
            }),
        }
    }
}

fn default_endpoint() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], figcraft_engine::settings::DEFAULT_MCP_PORT))
}

/// The link to the app.
#[derive(Default)]
struct Link {
    addr: Option<SocketAddr>,
    session: Option<String>,
    up: bool,
    /// The app refused the token (until it accepts it).
    refused: bool,
    /// What was last logged about it, so a state is said once.
    said: String,
}

/// What the client said.
#[derive(Default)]
struct Client {
    /// Its `initialize` params.
    init: Option<Value>,
    /// It sent `notifications/initialized`.
    initialized: bool,
}

struct Shared {
    cfg: Config,
    link: Mutex<Link>,
    client: Mutex<Client>,
    out: Mutex<Box<dyn Write + Send>>,
}

impl Shared {
    fn send(&self, msg: &Value) {
        let mut out = self.out.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = writeln!(out, "{msg}").and_then(|()| out.flush());
    }

    fn say(&self, link: &mut Link, what: String) {
        if link.said != what {
            (self.cfg.log)(&what);
            link.said = what;
        }
    }

    /// Initialize at the app for the client (once it has initialized here). Returns the app's
    /// `initialize` result when the link came up.
    fn connect(&self) -> Option<Value> {
        let (params, initialized) = {
            let c = self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            (c.init.clone()?, c.initialized)
        };
        let mut link = self.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let addr = (self.cfg.endpoint)();
        let mut p = params;
        let name = p["clientInfo"]["name"].as_str().unwrap_or("").to_string();
        p["clientInfo"]["name"] = json!(format!("{}{VIA}", if name.trim().is_empty() { "an MCP client" } else { &name }));
        let init = json!({"jsonrpc": "2.0", "id": "scrim-bridge-init", "method": "initialize", "params": p});
        let auth = self.cfg.auth.as_deref();
        // Quickly, and only Scrim: another program holding the port, or one answering slowly, isn't.
        match upstream::post(addr, None, auth, &init, upstream::QUICK) {
            upstream::Reply::Answer { status: 200, session, body: Some(b) } if b["result"]["serverInfo"]["name"] == "scrim" => {
                *link = Link { addr: Some(addr), session, up: true, refused: false, said: std::mem::take(&mut link.said) };
                if initialized {
                    let n = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
                    let _ = upstream::post(addr, link.session.as_deref(), auth, &n, upstream::QUICK);
                }
                self.say(&mut link, format!("connected to Scrim at {addr}"));
                b.get("result").cloned()
            }
            upstream::Reply::Answer { status: 401, .. } => {
                link.up = false;
                link.refused = true;
                self.say(&mut link, format!("Scrim at {addr} refused the token"));
                None
            }
            upstream::Reply::Answer { status, .. } => {
                link.up = false;
                self.say(&mut link, format!("something at {addr} that isn't Scrim answered ({status}); waiting for Scrim"));
                None
            }
            upstream::Reply::Down(_) => {
                link.up = false;
                self.say(&mut link, format!("Scrim isn't running (no answer from {addr}); waiting for it"));
                None
            }
        }
    }

    /// `msg` to the app, when the link is up. `None`: the app is gone (the link is marked down).
    fn forward(&self, msg: &Value) -> Option<Value> {
        let (addr, session) = {
            let link = self.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if !link.up {
                return None;
            }
            (link.addr?, link.session.clone())
        };
        match upstream::post(addr, session.as_deref(), self.cfg.auth.as_deref(), msg, upstream::ANSWER) {
            upstream::Reply::Answer { status: 200, body: Some(b), .. } => Some(b),
            upstream::Reply::Answer { status: 202, .. } => Some(Value::Null),
            // The app restarted on the same port: its sessions are new.
            upstream::Reply::Answer { status: 404, .. } => {
                self.down();
                None
            }
            upstream::Reply::Answer { status: 401, .. } => {
                self.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner).refused = true;
                self.down();
                None
            }
            upstream::Reply::Answer { status, body, .. } => Some(body.unwrap_or_else(
                || json!({"jsonrpc": "2.0", "id": msg["id"], "error": {"code": -32603, "message": format!("Scrim answered HTTP {status}")}}),
            )),
            upstream::Reply::Down(_) => {
                self.down();
                None
            }
        }
    }

    fn down(&self) {
        let mut link = self.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        link.up = false;
        link.session = None;
    }

    fn up(&self) -> bool {
        self.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner).up
    }

    fn refused(&self) -> bool {
        self.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner).refused
    }

    /// The link came up after the client listed tools here: tell it to list them again.
    fn tools_changed(&self) {
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/tools/list_changed"}));
    }

    /// A request: to the app (connecting first if it's down), else answered here.
    fn request(&self, msg: &Value) -> Value {
        let id = msg["id"].clone();
        let method = msg["method"].as_str().unwrap_or("");
        if method == "initialize" {
            let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));
            self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).init = Some(params.clone());
            let result = match self.connect() {
                Some(mut r) => {
                    // So the client listens for the list changing when the app comes and goes.
                    r["capabilities"]["tools"]["listChanged"] = json!(true);
                    r
                }
                None => local_init(&params),
            };
            return json!({"jsonrpc": "2.0", "id": id, "result": result});
        }
        let was_up = self.up();
        if !was_up && self.connect().is_some() {
            self.tools_changed();
        }
        if let Some(r) = self.forward(msg) {
            return r;
        }
        // Gone mid-session (stopped, or restarted elsewhere): look for it once more, now.
        if self.connect().is_some() {
            self.tools_changed();
            if let Some(r) = self.forward(msg) {
                return r;
            }
        }
        local(&id, method, self.refused(), &self.cfg)
    }

    fn notification(&self, msg: &Value) {
        if msg["method"] == "notifications/initialized" {
            self.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).initialized = true;
        }
        let _ = self.forward(msg);
    }
}

/// `initialize` answered here, while the app is closed.
fn local_init(params: &Value) -> Value {
    let asked = params["protocolVersion"].as_str().unwrap_or(PROTOCOL_VERSION);
    let version = if SUPPORTED_VERSIONS.contains(&asked) { asked } else { PROTOCOL_VERSION };
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {"listChanged": true}, "resources": {}, "prompts": {}},
        "serverInfo": {"name": "scrim", "title": "Scrim", "version": env!("CARGO_PKG_VERSION")},
        "instructions": format!("{INSTRUCTIONS} Backend: waiting for the Scrim app (scrim mcp-bridge). Until it is open, tools answer that Scrim is not running."),
    })
}

/// A request answered here, while the app is closed.
fn local(id: &Value, method: &str, refused: bool, cfg: &Config) -> Value {
    let why = if refused { REFUSED } else { NOT_RUNNING };
    let result = match method {
        "ping" => json!({}),
        "tools/list" => (cfg.tools)().unwrap_or_else(|| json!({"tools": []})),
        "tools/call" => json!({"content": [{"type": "text", "text": why}], "isError": true}),
        "resources/list" => json!({"resources": []}),
        "resources/templates/list" => json!({"resourceTemplates": []}),
        "prompts/list" => json!({"prompts": []}),
        _ => return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": why}}),
    };
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Run the bridge: messages from `input` (newline-delimited JSON-RPC, a client's stdout), answers
/// to `output`, until `input` closes.
pub fn run(input: impl BufRead, output: Box<dyn Write + Send>, cfg: Config) -> std::io::Result<()> {
    let retry = cfg.retry;
    let shared = Arc::new(Shared { cfg, link: Mutex::new(Link::default()), client: Mutex::new(Client::default()), out: Mutex::new(output) });
    let stop = Arc::new(AtomicBool::new(false));
    let (sh, st) = (shared.clone(), stop.clone());
    // While the app is closed, try it every `retry`, once the client has initialized.
    let waiter = std::thread::Builder::new().name("scrim-bridge-wait".into()).spawn(move || {
        while !st.load(Ordering::Relaxed) {
            std::thread::sleep(retry);
            let waiting = sh.client.lock().unwrap_or_else(std::sync::PoisonError::into_inner).init.is_some() && !sh.up();
            if waiting && !st.load(Ordering::Relaxed) && sh.connect().is_some() {
                sh.tools_changed();
            }
        }
    })?;
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                shared.send(&json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}}));
                continue;
            }
        };
        match (msg.get("method").is_some(), msg.get("id").is_some_and(|i| !i.is_null())) {
            (true, true) => {
                let r = shared.request(&msg);
                shared.send(&r);
            }
            (true, false) => shared.notification(&msg),
            // A response to something the bridge never asks: nothing to do.
            (false, _) => {}
        }
    }
    stop.store(true, Ordering::Relaxed);
    // End the session at the app (it stops showing this client as working).
    let (addr, session) = {
        let link = shared.link.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        (link.addr, link.session.clone())
    };
    if let (Some(addr), Some(session)) = (addr, session) {
        let _ = upstream::delete(addr, &session, shared.cfg.auth.as_deref());
    }
    drop(waiter);
    Ok(())
}
