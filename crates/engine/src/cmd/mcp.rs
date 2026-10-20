//! The app's MCP endpoint, as commands (`docs/design/settings-and-connect.md`, C1): its status, the
//! Claude Code connect flow, copying the command, restarting it, requiring a token, and the folder
//! file tools are confined to. The endpoint itself runs in the desktop app (`figcraft_mcp::http`);
//! these commands change its settings and ask the app (`request_ui`) to apply them. Every one is an
//! MCP route too, through `run_command`.

use serde_json::{Value, json};

use super::CommandSpec;
use crate::agents::{CONNECTED_FOR_SECS, claude_add_args, claude_add_command};
use crate::settings::new_token;
use crate::{Session, bad, spec};

/// The UI requests these commands make (the desktop app acts on them).
pub const CONNECT_REQUEST: &str = "mcp.connect";
pub const RESTART_REQUEST: &str = "mcp.restart";
pub const CONFIG_REQUEST: &str = "mcp.config";

fn token(s: &Session) -> Option<&str> {
    let m = &s.prefs.settings.mcp;
    m.require_auth.then_some(m.token.as_str()).filter(|t| !t.is_empty())
}

/// The command that connects Claude Code, with the token masked unless `reveal`.
fn command(s: &Session, reveal: bool) -> Option<String> {
    let port = s.agents.mcp.port?;
    let t = token(s);
    Some(claude_add_command(port, if reveal { t } else { t.map(|_| "<token>") }))
}

/// Whether a client initialized over the endpoint within [`CONNECTED_FOR_SECS`] of `now` (unix
/// seconds, from `p.now` or the newest client), and which.
fn connected(s: &Session, now: Option<u64>) -> Option<(String, String)> {
    let (name, version, at) = s.agents.mcp.clients.last()?;
    let now = now.unwrap_or(*at);
    (now.saturating_sub(*at) <= CONNECTED_FOR_SECS).then(|| (name.clone(), version.clone()))
}

/// `mcp.status`: everything Settings ▸ MCP and the AI tab show.
pub fn status(s: &Session, now: Option<u64>) -> Value {
    let m = &s.prefs.settings.mcp;
    let e = &s.agents.mcp;
    let state = match (e.port, connected(s, now)) {
        (None, _) => "stopped",
        (Some(_), Some(_)) => "connected",
        (Some(_), None) => "not connected",
    };
    json!({
        "state": state,
        "port": e.port,
        "url": e.url(),
        "error": e.error,
        "version": env!("CARGO_PKG_VERSION"),
        "requireAuth": m.require_auth,
        "rootDir": m.root_dir,
        "client": connected(s, now).map(|(name, version)| json!({"name": name, "version": version})),
        "command": command(s, false),
        "args": e.port.map(|p| claude_add_args(p, token(s))),
        "disabledTools": m.disabled_tools,
    })
}

fn now_of(p: &Value) -> Option<u64> {
    p.get("now").and_then(Value::as_u64)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "mcp.status",
            "MCP endpoint status",
            "{now? (unix seconds)} → {state: connected|not connected|stopped, port, url, error, version, requireAuth, rootDir, client, command, args, disabledTools}",
            false,
            |s, p| Ok(status(s, now_of(p)))
        ),
        spec!(
            "mcp.connect-claude-code",
            "Connect Claude Code",
            "{} → {command, args}  (the app shows the command and runs `claude` with these arguments after one confirmation; nothing runs without it)",
            false,
            |s, _| {
                let cmd = "mcp.connect-claude-code";
                let port = s.agents.mcp.port.ok_or_else(|| bad(cmd, "the MCP endpoint isn't running: restart it (mcp.restart-server)"))?;
                let r = json!({"command": command(s, false), "args": claude_add_args(port, token(s))});
                s.request_ui(CONNECT_REQUEST, r.clone());
                Ok(r)
            }
        ),
        spec!(
            "mcp.copy-command",
            "Copy the Claude Code connect command",
            "{} → {command}  (with the token when authentication is on; onto the clipboard in the app)",
            false,
            |s, _| {
                let full = command(s, true).ok_or_else(|| bad("mcp.copy-command", "the MCP endpoint isn't running"))?;
                s.request_ui(super::code::COPY_REQUEST, json!({"text": full, "what": "the connect command"}));
                Ok(json!({"command": command(s, false)}))
            }
        ),
        spec!(
            "mcp.restart-server",
            "Restart the MCP endpoint",
            "{} → {requested: true}  (the app stops and starts it, on its port or the next free one)",
            false,
            |s, _| {
                s.request_ui(RESTART_REQUEST, json!({}));
                Ok(json!({"requested": true}))
            }
        ),
        spec!(
            "mcp.toggle-auth",
            "Require authentication for MCP",
            "{on? (default: the opposite of now)} → {requireAuth, command}  (on: clients send Authorization: Bearer <token>; the token is made once and kept; connect again to pass it)",
            false,
            |s, p| {
                let on = p.get("on").and_then(Value::as_bool).unwrap_or(!s.prefs.settings.mcp.require_auth);
                let m = &mut s.prefs.settings.mcp;
                m.require_auth = on;
                if on && m.token.len() < 32 {
                    m.token = new_token();
                }
                s.request_ui(CONFIG_REQUEST, json!({}));
                Ok(json!({"requireAuth": on, "command": command(s, false)}))
            }
        ),
        spec!(
            "mcp.set-root-dir",
            "Folder for MCP file tools",
            "{path? (an absolute folder; none: the default, your home folder)} → {rootDir}  (open_file, save_file and export over the app's endpoint may only use files inside it)",
            false,
            |s, p| {
                let cmd = "mcp.set-root-dir";
                let dir = match p.get("path").and_then(Value::as_str).map(str::trim).filter(|d| !d.is_empty()) {
                    None => None,
                    Some(d) => {
                        let path = std::path::Path::new(d);
                        if !path.is_absolute() {
                            return Err(bad(cmd, "path must be absolute"));
                        }
                        if !crate::file_access::is_dir(path) {
                            return Err(bad(cmd, "no such folder"));
                        }
                        Some(d.to_string())
                    }
                };
                s.prefs.settings.mcp.root_dir = dir.clone();
                s.request_ui(CONFIG_REQUEST, json!({}));
                Ok(json!({"rootDir": dir}))
            }
        ),
    ]
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
