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
use figcraft_mcp::{Backend, Server};
use figcraft_ui_egui::ControlRequest;
use figcraft_ui_egui::control::ROUTE_MCP_HTTP;
use serde_json::{Value, json};

type Queue = Arc<Mutex<Vec<ControlRequest>>>;
type CtxSlot = Arc<Mutex<Option<egui::Context>>>;

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
    fn has_ui(&self) -> bool {
        true
    }
    fn describe(&self) -> String {
        "this Scrim window (MCP over HTTP)".into()
    }
}

/// The endpoint the desktop services own: started, stopped and reconfigured from the UI.
pub struct McpHandle {
    queue: Queue,
    ctx: CtxSlot,
    config: Arc<RwLock<Config>>,
    endpoint: Option<Endpoint>,
}

impl McpHandle {
    pub fn new(queue: Queue, ctx: CtxSlot) -> Self {
        Self { queue, ctx, config: Arc::new(RwLock::new(Config::default())), endpoint: None }
    }

    /// Stop the endpoint if it runs, and start it with `settings`. Returns its port.
    pub fn restart(&mut self, settings: &McpSettings) -> Result<u16, String> {
        if let Some(mut e) = self.endpoint.take() {
            e.stop();
        }
        self.configure(settings);
        let (queue, ctx) = (self.queue.clone(), self.ctx.clone());
        let endpoint = Endpoint::start(settings.port, TRIES, self.config.clone(), move || {
            let notify = InApp { queue: queue.clone(), ctx: ctx.clone() };
            Server::new(Box::new(InApp { queue, ctx })).with_initialize_hook(move |info| {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let params = json!({"name": info.get("name"), "version": info.get("version"), "now": now});
                // Told to the UI thread without waiting for it.
                notify.send("mcp.client", params, false);
            })
        })?;
        let port = endpoint.port;
        self.endpoint = Some(endpoint);
        log::info!("MCP endpoint on http://127.0.0.1:{port}/mcp");
        Ok(port)
    }

    /// Apply changed settings (the token) to the running endpoint.
    pub fn configure(&mut self, settings: &McpSettings) {
        let token = (settings.require_auth && !settings.token.is_empty()).then(|| settings.token.clone());
        if let Ok(mut c) = self.config.write() {
            c.token = token;
        }
    }
}

#[cfg(test)]
#[path = "mcp_endpoint_tests.rs"]
mod tests;
