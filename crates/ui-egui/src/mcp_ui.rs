//! The app's MCP endpoint, from the UI's side (`docs/design/settings-and-connect.md`, C1):
//! starting and restarting it through the platform ([`crate::Services::mcp_restart`]), applying
//! changed settings, keeping the port it got, confining its file tools to the root folder, and
//! noting the clients that connect. The commands (`mcp.*`) ask for these through UI requests.

use std::sync::Arc;

use figcraft_engine::cmd::mcp::{CONFIG_REQUEST, CONNECT_REQUEST, RESTART_REQUEST};
use figcraft_engine::file_access::Roots;
use serde_json::{Value, json};

use crate::FigApp;

/// Start (or restart) the endpoint with the preferences' settings, and keep the port it got.
pub fn restart(app: &mut FigApp) {
    let settings = app.session.prefs.settings.mcp.clone();
    match app.services.mcp_restart(&settings) {
        Ok(port) => {
            app.session.agents.mcp.port = Some(port);
            app.session.agents.mcp.error = None;
            if settings.port != port {
                app.session.prefs.settings.mcp.port = port;
                app.services.save_prefs(&app.session.prefs);
            }
        }
        Err(e) => {
            app.session.agents.mcp.port = None;
            app.session.agents.mcp.error = Some(e);
        }
    }
    app.session.agents.mcp.roots = roots(&settings);
}

/// The folders the endpoint's file tools are confined to: the chosen folder, or the home folder.
fn roots(settings: &figcraft_engine::settings::McpSettings) -> Option<Arc<Roots>> {
    let dir = settings.root_dir.clone().or_else(|| std::env::var("HOME").ok().or_else(|| std::env::var("USERPROFILE").ok()))?;
    Roots::new(std::slice::from_ref(&dir), std::slice::from_ref(&dir)).ok().map(Arc::new)
}

/// Act on an MCP request from a command; `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str, _data: &Value) -> bool {
    match kind {
        RESTART_REQUEST => restart(app),
        CONFIG_REQUEST => {
            let settings = app.session.prefs.settings.mcp.clone();
            app.services.mcp_configure(&settings);
            app.session.agents.mcp.roots = roots(&settings);
            app.services.save_prefs(&app.session.prefs);
        }
        CONNECT_REQUEST => crate::connect_flow::open(app),
        _ => return crate::connect_flow::request(app, kind),
    }
    true
}

/// The control method `mcp.client`: the endpoint saw a client initialize (`{name, version, now}`).
pub fn client_seen(app: &mut FigApp, p: &Value) -> Result<Value, String> {
    let name = p.get("name").and_then(Value::as_str).unwrap_or("an MCP client").chars().take(80).collect::<String>();
    let version = p.get("version").and_then(Value::as_str).unwrap_or("").chars().take(40).collect::<String>();
    let now = p.get("now").and_then(Value::as_u64).unwrap_or(0);
    let clients = &mut app.session.agents.mcp.clients;
    clients.push((name, version, now));
    let excess = clients.len().saturating_sub(20);
    clients.drain(..excess);
    Ok(json!({"ok": true}))
}
