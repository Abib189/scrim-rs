//! The app's MCP endpoint (`docs/design/settings-and-connect.md`, C1): MCP over HTTP on 127.0.0.1
//! (`figcraft_mcp::http`), answered by this window. Every tool call becomes a control request on
//! the same queue the control channel feeds, run on the UI thread as the agent "mcp (http)", so
//! locks, the stale-write guard and "Agent: …" undo labels apply exactly as they do for `scrim mcp
//! --attach`. Started at launch on the port in the preferences (the next free one when it's taken),
//! restarted from Settings ▸ MCP.

use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use figcraft_engine::settings::McpSettings;
use figcraft_mcp::http::{Config, Endpoint};
use figcraft_mcp::{Backend, Server, SessionEvent};
use figcraft_ui_egui::ControlRequest;
use figcraft_ui_egui::control::ROUTE_MCP_HTTP;
use serde_json::{Value, json};

type Queue = Arc<Mutex<Vec<ControlRequest>>>;
type CtxSlot = Arc<Mutex<Option<egui::Context>>>;
/// The tools Settings ▸ Tool access turned off, shared with the server thread.
type Disabled = Arc<RwLock<Vec<String>>>;

/// How many ports after the preferred one are tried.
const TRIES: u16 = 20;
/// How long a tool call waits for the UI thread (screenshots need the window drawn).
const ANSWER: Duration = Duration::from_secs(90);
/// Requests waiting for the UI thread at once (as the control channel allows).
const MAX_QUEUED: usize = 8;

/// Tool calls answered by this window, through the UI thread's control queue.
struct InApp {
    queue: Queue,
    ctx: CtxSlot,
    disabled: Disabled,
    /// The client the server says the current request comes from (by its session).
    client: Option<String>,
    /// That request's HTTP session, when the server knows it (Stage I.1).
    session: Option<String>,
}

impl InApp {
    /// Queue `method` for the UI thread; `wait`: for its answer (`None` when it didn't come).
    fn send(&self, method: &str, params: Value, wait: bool) -> Option<Result<Value, String>> {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let mut pending = self.queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            pending.retain(|r| !r.is_cancelled());
            if pending.len() >= MAX_QUEUED {
                return Some(Err("the app is busy: too many requests are waiting".into()));
            }
            pending.push(ControlRequest {
                route: ROUTE_MCP_HTTP,
                client: self.client.clone(),
                session: self.session.clone(),
                id: json!(0),
                method: method.to_string(),
                params,
                reply: tx,
                cancelled: cancelled.clone(),
            });
        }
        if let Some(c) = self.ctx.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
            c.request_repaint();
        }
        if !wait {
            return None;
        }
        match rx.recv_timeout(ANSWER) {
            Ok(r) => Some(r.result),
            Err(_) => {
                cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
                Some(Err("timed out waiting for the Scrim window (screenshots and input need it on screen)".into()))
            }
        }
    }
}

impl Backend for InApp {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.send(method, params, true).unwrap_or_else(|| Err("no answer".into()))
    }
    fn set_client(&mut self, client: Option<&str>) {
        self.client = client.map(str::to_string);
    }
    fn set_session(&mut self, session: Option<&str>) {
        self.session = session.map(str::to_string);
    }
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        "this Scrim window (MCP over HTTP)".into()
    }
    fn disabled_tools(&mut self) -> Vec<String> {
        self.disabled.read().map(|d| d.clone()).unwrap_or_else(|p| p.into_inner().clone())
    }
}

/// The endpoint the desktop services own: started, stopped and reconfigured from the UI.
pub struct McpHandle {
    queue: Queue,
    ctx: CtxSlot,
    config: Arc<RwLock<Config>>,
    disabled: Disabled,
    endpoint: Option<Endpoint>,
    /// The settings folder, where `scrim mcp-bridge` looks for the endpoint and its tools (Fix P,
    /// `figcraft_mcp::discovery`). `None` (tests): nothing is noted.
    dir: Option<std::path::PathBuf>,
}

impl McpHandle {
    pub fn new(queue: Queue, ctx: CtxSlot) -> Self {
        Self { queue, ctx, config: Arc::new(RwLock::new(Config::default())), disabled: Disabled::default(), endpoint: None, dir: None }
    }

    /// Note the endpoint and its tools in `dir` for bridges (the desktop app's settings folder).
    pub fn with_dir(mut self, dir: Option<std::path::PathBuf>) -> Self {
        self.dir = dir;
        self
    }

    /// The tools the endpoint lists (Tool access applied), noted for bridges while it runs.
    fn note_tools(&self, settings: &McpSettings) {
        let Some(dir) = self.dir.as_deref().filter(|_| self.endpoint.is_some()) else { return };
        let tools: Vec<Value> = figcraft_mcp::tool_definitions()
            .into_iter()
            .filter(|t| !settings.disabled_tools.iter().any(|d| Some(d.as_str()) == t["name"].as_str()))
            .collect();
        if let Err(e) = figcraft_mcp::discovery::write_tools(dir, &json!({"tools": tools})) {
            log::warn!("MCP: couldn't note the tools for bridges: {e}");
        }
    }

    /// Stop the endpoint if it runs, and start it with `settings`. Returns its port.
    pub fn restart(&mut self, settings: &McpSettings) -> Result<u16, String> {
        if let Some(mut e) = self.endpoint.take() {
            e.stop();
            if let Some(dir) = self.dir.as_deref() {
                figcraft_mcp::discovery::remove_endpoint(dir);
            }
        }
        self.configure(settings);
        let (queue, ctx, disabled) = (self.queue.clone(), self.ctx.clone(), self.disabled.clone());
        let endpoint = Endpoint::start(settings.port, TRIES, self.config.clone(), move || {
            let notify = InApp { queue: queue.clone(), ctx: ctx.clone(), disabled: disabled.clone(), client: None, session: None };
            // Told to the UI thread without waiting for it: a session starting (who connected), and
            // ending (it stops showing as working, Stage I.1).
            Server::new(Box::new(InApp { queue, ctx, disabled, client: None, session: None })).with_session_hook(move |event| match event {
                SessionEvent::Started { id, info } => {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    let params = json!({"name": info.get("name"), "version": info.get("version"), "now": now, "session": id});
                    notify.send("mcp.client", params, false);
                }
                SessionEvent::Ended { id } => {
                    notify.send("mcp.session-ended", json!({"session": id}), false);
                }
            })
        })?;
        let port = endpoint.port;
        self.endpoint = Some(endpoint);
        log::info!("MCP endpoint on http://127.0.0.1:{port}/mcp");
        // Where bridges find it (Fix P): the port it got, which may be a fallback.
        if let Some(dir) = self.dir.as_deref()
            && let Err(e) = figcraft_mcp::discovery::write_endpoint(dir, port)
        {
            log::warn!("MCP: couldn't note the endpoint for bridges: {e}");
        }
        self.note_tools(settings);
        Ok(port)
    }

    /// Apply changed settings (the token, the tools turned off) to the running endpoint.
    pub fn configure(&mut self, settings: &McpSettings) {
        let token = (settings.require_auth && !settings.token.is_empty()).then(|| settings.token.clone());
        if let Ok(mut c) = self.config.write() {
            c.token = token;
        }
        self.disabled.write().unwrap_or_else(std::sync::PoisonError::into_inner).clone_from(&settings.disabled_tools);
        self.note_tools(settings);
    }
}

impl Drop for McpHandle {
    /// The app is quitting: bridges stop looking here.
    fn drop(&mut self) {
        if let (Some(dir), Some(_)) = (self.dir.as_deref(), self.endpoint.as_ref()) {
            figcraft_mcp::discovery::remove_endpoint(dir);
        }
    }
}

#[cfg(test)]
#[path = "mcp_endpoint_tests.rs"]
mod tests;
