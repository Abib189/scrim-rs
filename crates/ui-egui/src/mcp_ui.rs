//! The app's MCP endpoint, from the UI's side (`docs/design/settings-and-connect.md`, C1):
//! starting and restarting it through the platform ([`crate::Services::mcp_restart`]), applying
//! changed settings, keeping the port it got, confining its file tools to the root folder, and
//! noting the clients that connect. The commands (`mcp.*`) ask for these through UI requests.

use std::path::PathBuf;
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
            crate::settings_privacy::record(app, figcraft_engine::settings::diagnostics::Entry::McpStarted(port));
            app.session.agents.mcp.desktop_config = app.services.claude_desktop_config();
            app.session.agents.mcp.bridge = app.services.bridge_command();
            app.session.agents.mcp.client_env = app.services.agent_client_env();
            app.session.agents.mcp.port = Some(port);
            app.session.agents.mcp.error = None;
            if settings.port != port {
                app.session.prefs.settings.mcp.port = port;
                app.services.save_prefs(&app.session.prefs);
            }
        }
        Err(e) => {
            crate::settings_privacy::record(app, figcraft_engine::settings::diagnostics::Entry::McpFailed);
            app.session.agents.mcp.port = None;
            app.session.agents.mcp.error = Some(e);
        }
    }
    app.session.agents.mcp.roots = roots(&settings);
}

/// The folder the endpoint's file tools are confined to: the chosen folder, or `~/Documents/Scrim`
/// (C1.4; never the home folder).
fn roots(settings: &figcraft_engine::settings::McpSettings) -> Option<Arc<Roots>> {
    roots_for(settings, figcraft_engine::settings::default_mcp_root())
}

/// [`roots`] with `default` as the default folder. A folder that isn't there (yet), or no folder at
/// all, confines to nothing: no file may be read or written, never the whole disk.
fn roots_for(settings: &figcraft_engine::settings::McpSettings, default: Option<PathBuf>) -> Option<Arc<Roots>> {
    let dir = settings.root_dir.clone().or_else(|| default.map(|d| d.to_string_lossy().into_owned()));
    let only = dir.and_then(|d| Roots::new(std::slice::from_ref(&d), std::slice::from_ref(&d)).ok());
    only.or_else(|| Roots::new(&[], &[]).ok()).map(Arc::new)
}

/// Before an MCP request runs: make the default folder the first time a request names a file
/// (`path` anywhere in its parameters), and confine to it from then on. A chosen folder is never
/// made or changed.
pub fn ensure_root(app: &mut FigApp, params: &Value) {
    ensure_root_in(app, params, figcraft_engine::settings::default_mcp_root());
}

fn ensure_root_in(app: &mut FigApp, params: &Value, default: Option<PathBuf>) {
    let settings = app.session.prefs.settings.mcp.clone();
    let Some(dir) = default.as_ref().filter(|_| settings.root_dir.is_none()) else { return };
    if figcraft_engine::file_access::is_dir(dir) || !names_a_file(params) {
        return;
    }
    if figcraft_engine::file_access::create_dir_all(dir).is_ok() {
        app.session.agents.mcp.roots = roots_for(&settings, default);
    }
}

/// Whether `p` has a `path` (a file) anywhere in it.
fn names_a_file(p: &Value) -> bool {
    match p {
        Value::Object(m) => m.iter().any(|(k, v)| (k == "path" && v.as_str().is_some_and(|s| !s.is_empty())) || names_a_file(v)),
        Value::Array(a) => a.iter().any(names_a_file),
        _ => false,
    }
}

/// Act on an MCP request from a command; `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str, data: &Value) -> bool {
    match kind {
        RESTART_REQUEST => restart(app),
        CONFIG_REQUEST => {
            let settings = app.session.prefs.settings.mcp.clone();
            app.services.mcp_configure(&settings);
            app.session.agents.mcp.roots = roots(&settings);
            app.services.save_prefs(&app.session.prefs);
        }
        CONNECT_REQUEST => crate::connect_flow::open(app),
        figcraft_engine::cmd::agent_work::FINISH_REQUEST => crate::follow_agent::finish(app, data),
        _ => {
            return crate::desktop_flow::request(app, kind)
                || crate::connect_flow::request(app, kind)
                || crate::connect_flow::agent_clients_ui::request(app, kind, data);
        }
    }
    true
}

/// The control method `mcp.client`: the endpoint saw a client initialize (`{name, version, now}`).
pub fn client_seen(app: &mut FigApp, p: &Value) -> Result<Value, String> {
    // As announced; a client that announced no name is kept nameless (shown as "an agent").
    let name = p.get("name").and_then(Value::as_str).unwrap_or("").chars().take(80).collect::<String>();
    let version = p.get("version").and_then(Value::as_str).unwrap_or("").chars().take(40).collect::<String>();
    let now = p.get("now").and_then(Value::as_u64).unwrap_or(0);
    crate::settings_privacy::record(app, figcraft_engine::settings::diagnostics::Entry::McpClient { name: &name, version: &version });
    let clients = &mut app.session.agents.mcp.clients;
    clients.push((name, version, now));
    let excess = clients.len().saturating_sub(20);
    clients.drain(..excess);
    Ok(json!({"ok": true}))
}

#[cfg(test)]
#[path = "mcp_ui_tests.rs"]
mod tests;
