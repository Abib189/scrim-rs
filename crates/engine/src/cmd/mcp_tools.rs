//! Tool access (`docs/design/settings-and-connect.md`, C3c): which MCP tools agents get. A tool
//! turned off is left out of `tools/list` and refused if called anyway (`figcraft_mcp::Server`
//! asks its backend for [`McpSettings::disabled_tools`](crate::settings::McpSettings) on every
//! list and call). Agents may turn tools off but never on: only the person widens what agents can
//! do.
//!
//! Tool access is per tool. A command reached through `run_command` or `batch` is that tool's, so
//! turning those two off is what takes the commands away.

use serde_json::{Value, json};

use super::CommandSpec;
use super::mcp::{CONFIG_REQUEST, person_only};
use crate::{Result, Session, bad, spec};

/// Every MCP tool, in `tools/list` order: (name, title, read-only). `figcraft_mcp`'s tests check
/// this against its tool definitions and their `readOnlyHint`.
pub const TOOLS: &[(&str, &str, bool)] = &[
    ("list_commands", "List commands", true),
    ("run_command", "Run command", false),
    ("batch", "Run commands as one step", false),
    ("create_node", "Create layer", false),
    ("set_properties", "Set properties", false),
    ("inspect_document", "Inspect document", true),
    ("get_node", "Get layer", true),
    ("select", "Select layers", false),
    ("export", "Export", true),
    ("screenshot", "Screenshot", true),
    ("open_file", "Open file", false),
    ("save_file", "Save file", false),
    ("undo", "Undo", false),
    ("redo", "Redo", false),
    ("query_nodes", "Find layers", true),
    ("lint_design", "Lint design", true),
    ("set_lint_profile", "Set lint profile", false),
    ("ignore_lint", "Ignore a lint finding", false),
    ("analyze_design", "Analyze design", true),
    ("get_changes", "Changes since a revision", true),
    ("pointer", "Pointer gesture", false),
    ("key", "Press key", false),
    ("type_text", "Type text", false),
    ("inspect_ui", "Inspect UI", true),
];

fn known(name: &str) -> bool {
    TOOLS.iter().any(|(n, ..)| *n == name)
}

/// `mcp.tools`: every tool, whether it's on, and the counts.
pub fn state(s: &Session) -> Value {
    let off = &s.prefs.settings.mcp.disabled_tools;
    let on = |n: &str| !off.iter().any(|d| d == n);
    let count = |read: Option<bool>| {
        let all: Vec<_> = TOOLS.iter().filter(|(_, _, ro)| read.is_none_or(|r| r == *ro)).collect();
        json!({"enabled": all.iter().filter(|(n, ..)| on(n)).count(), "total": all.len()})
    };
    json!({
        "tools": TOOLS.iter().map(|(n, title, ro)| json!({"name": n, "title": title, "readOnly": ro, "on": on(n)})).collect::<Vec<_>>(),
        "enabled": count(None)["enabled"],
        "total": TOOLS.len(),
        "groups": {"read": count(Some(true)), "write": count(Some(false))},
    })
}

/// Turn `names` on or off. Turning one on is the person's alone.
fn set(s: &mut Session, cmd: &str, names: &[&str], on: bool) -> Result<Value> {
    if on && names.iter().any(|n| s.prefs.settings.mcp.disabled_tools.iter().any(|d| d == n)) {
        person_only(s, cmd, "turn an MCP tool back on")?;
    }
    let off = &mut s.prefs.settings.mcp.disabled_tools;
    off.retain(|d| !names.contains(&d.as_str()));
    if !on {
        off.extend(names.iter().map(|n| n.to_string()));
        off.sort();
    }
    s.request_ui(CONFIG_REQUEST, json!({}));
    Ok(state(s))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "mcp.tools",
            "MCP tool access",
            "{} → {tools: [{name, title, readOnly, on}], enabled, total, groups: {read, write: {enabled, total}}}",
            false,
            |s, _| Ok(state(s))
        ),
        spec!(
            "mcp.set-tool",
            "Turn an MCP tool on or off",
            "{tool, on} → the tool access  (off: hidden from tools/list and refused; an agent can turn tools off, never on)",
            false,
            |s, p| {
                let cmd = "mcp.set-tool";
                let tool = p
                    .get("tool")
                    .and_then(Value::as_str)
                    .filter(|t| known(t))
                    .ok_or_else(|| bad(cmd, "`tool` must be an MCP tool's name (mcp.tools lists them)"))?;
                let on = p.get("on").and_then(Value::as_bool).ok_or_else(|| bad(cmd, "`on` is required"))?;
                let tool = tool.to_string();
                set(s, cmd, &[&tool], on)
            }
        ),
        spec!(
            "mcp.set-tool-group",
            "Turn a group of MCP tools on or off",
            "{group: read|write, on} → the tool access  (read: the read-only tools; write: the others)",
            false,
            |s, p| {
                let cmd = "mcp.set-tool-group";
                let read = match p.get("group").and_then(Value::as_str) {
                    Some("read") => true,
                    Some("write") => false,
                    _ => return Err(bad(cmd, "`group` is read or write")),
                };
                let on = p.get("on").and_then(Value::as_bool).ok_or_else(|| bad(cmd, "`on` is required"))?;
                let names: Vec<&str> = TOOLS.iter().filter(|(_, _, ro)| *ro == read).map(|(n, ..)| *n).collect();
                set(s, cmd, &names, on)
            }
        ),
        spec!(
            "mcp.restore-tools",
            "Restore the default MCP tool access",
            "{} → the tool access  (every tool on; the person's alone)",
            false,
            |s, _| {
                if !s.prefs.settings.mcp.disabled_tools.is_empty() {
                    person_only(s, "mcp.restore-tools", "turn MCP tools back on")?;
                }
                s.prefs.settings.mcp.disabled_tools.clear();
                s.request_ui(CONFIG_REQUEST, json!({}));
                Ok(state(s))
            }
        ),
    ]
}

#[cfg(test)]
#[path = "mcp_tools_tests.rs"]
mod tests;
