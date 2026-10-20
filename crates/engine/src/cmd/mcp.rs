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
/// The confirmations before Scrim runs `claude` (dialogs: an agent's calls never open one, so an
/// agent can't make Scrim run a program).
pub const RUN_DIALOG: &str = "dialog.mcpRunConnect";
pub const REMOVE_OLD_DIALOG: &str = "dialog.mcpRemoveOld";
/// The confirmation before Claude Code's out-of-date `scrim` entry is replaced (C1.2).
pub const REREGISTER_DIALOG: &str = "dialog.mcpReregister";

/// Refuse `what` when an agent asks: widening what agents may do (turning authentication off,
/// another root folder, a tool back on) is the person's alone.
pub(crate) fn person_only(s: &Session, cmd: &str, what: &str) -> crate::Result<()> {
    if s.agents.author.is_agent() {
        return Err(bad(cmd, format!("an agent can't {what}: that's the person's, in Scrim's Settings")));
    }
    Ok(())
}

pub(crate) fn token(s: &Session) -> Option<&str> {
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
        "defaultRootDir": crate::settings::default_mcp_root().map(|p| p.to_string_lossy().into_owned()),
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
            "mcp.run-connect",
            "Run the Claude Code connect command",
            "{} → {asked: true}  (opens the confirmation; the command runs only when the person confirms it there; an agent's call opens nothing and runs nothing)",
            false,
            |s, _| {
                s.request_ui(RUN_DIALOG, json!({}));
                Ok(json!({"asked": true}))
            }
        ),
        spec!(
            "mcp.remove-old-entry",
            "Remove the old figcraft entry from Claude Code",
            "{} → {asked: true}  (opens the confirmation to remove Claude Code's `figcraft` MCP entry; as for mcp.run-connect, only the person can confirm it)",
            false,
            |s, _| {
                s.request_ui(REMOVE_OLD_DIALOG, json!({}));
                Ok(json!({"asked": true}))
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
            "mcp.reregister",
            "Re-register Scrim with Claude Code",
            "{} → {asked: true}  (when Claude Code's scrim entry points to another address: opens the confirmation to replace it with this window's; only the person can confirm)",
            false,
            |s, _| {
                s.request_ui(REREGISTER_DIALOG, json!({}));
                Ok(json!({"asked": true}))
            }
        ),
        spec!(
            "mcp.set-port",
            "MCP port",
            "{port (1024–65535)} → {port}  (the port the endpoint tries first; it restarts there, or on the next free port; the person's alone)",
            false,
            |s, p| {
                let cmd = "mcp.set-port";
                person_only(s, cmd, "move the MCP endpoint")?;
                let port = p.get("port").and_then(Value::as_u64).and_then(|n| u16::try_from(n).ok()).filter(|n| *n >= 1024);
                let port = port.ok_or_else(|| bad(cmd, "`port` is a number from 1024 to 65535"))?;
                let m = &mut s.prefs.settings.mcp;
                m.port = port;
                m.port_chosen = true;
                s.request_ui(RESTART_REQUEST, json!({}));
                Ok(json!({"port": port}))
            }
        ),
        spec!("mcp.copy-address", "Copy the MCP address", "{} → {url}  (the endpoint's address, onto the clipboard in the app)", false, |s, _| {
            let url = s.agents.mcp.url().ok_or_else(|| bad("mcp.copy-address", "the MCP endpoint isn't running"))?;
            s.request_ui(super::code::COPY_REQUEST, json!({"text": url, "what": "the MCP address"}));
            Ok(json!({"url": url}))
        }),
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
                if !on && s.prefs.settings.mcp.require_auth {
                    person_only(s, "mcp.toggle-auth", "turn MCP authentication off")?;
                }
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
                person_only(s, cmd, "change the folder MCP file tools may use")?;
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
