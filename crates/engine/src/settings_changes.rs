//! Fix W4: every change of a saved setting is noted in Settings ▸ Diagnostics with who made it:
//! "setting theme: system → dark (made by the person's Settings)". Origins only: the person's
//! Settings, the person's command, or a settings migration. An agent never changes one (Fix W3:
//! refused, and that is noted instead). Following has its own lines (Fix V3), and the markers that
//! a value was chosen (`*_chosen`) go with their value, unsaid.
//!
//! **Values:** a short plain value is shown; a name, a server, a path, a token, a list of fonts or
//! plugins, or anything long reads "changed".

use std::collections::BTreeMap;

use serde_json::Value;

use super::diagnostics::{self, Entry};
use crate::Session;

/// The saved preferences that aren't settings: the window's memory and records
/// (`docs/design/settings-access.md` › Writes that aren't settings), and the version.
pub const NOT_SETTINGS: &[&str] = &[
    "recent_files",
    "online_files",
    "recent_actions",
    "diagnostics",
    "plugin_settings",
    "panel",
    "panel_widths",
    "pages_height",
    "home_views",
    "home_sort",
    "home_order",
    "getting_started_dismissed",
    "plugin_directory",
    "settings_version",
];

/// Not said by these lines: following (its own), and the chosen markers.
fn unsaid(name: &str) -> bool {
    matches!(name, "follow_agents" | "follow_chosen") || name.ends_with("_chosen")
}

/// Settings whose value is never shown: who the person is, where they publish, a secret, or a list.
const PRIVATE: &[&str] = &["user_name", "publish_server", "mcp.token", "mcp.root_dir", "chat_allowed", "fonts_muted", "plugins", "toolbar"];

/// Who changed a setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The person, in the Settings window.
    Settings,
    /// The person, by the command another way (a menu, the Actions palette, a shortcut).
    Command,
    /// A settings migration as the app starts.
    Migration,
}

impl Origin {
    pub fn text(self) -> &'static str {
        match self {
            Origin::Settings => "the person's Settings",
            Origin::Command => "the person's command",
            Origin::Migration => "a settings migration",
        }
    }
}

fn leaves(v: &Value, path: String, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            for (k, x) in m {
                // The kept View toggles are keyed by their command (`view` › `view.rulers`).
                let name = if path == "view" || path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                leaves(x, name, out);
            }
        }
        _ => {
            out.insert(path, v.clone());
        }
    }
}

/// The saved settings in `v` (a serialized `Prefs` or `Settings`), as named leaves: `theme`,
/// `mcp.port`, `view.rulers`…
pub fn settings(v: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    if let Value::Object(m) = v {
        for (k, x) in m.iter().filter(|(k, _)| !NOT_SETTINGS.contains(&k.as_str())) {
            leaves(x, k.clone(), &mut out);
        }
    }
    out.retain(|k, _| !unsaid(k));
    out
}

/// `v` as a line shows it: a short plain value, or nothing (then the line says "changed").
fn shown(name: &str, v: Option<&Value>) -> Option<String> {
    if PRIVATE.iter().any(|p| name == *p || name.starts_with(&format!("{p}."))) {
        return None;
    }
    let text = match v {
        None => "none".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "none".into(),
        Some(Value::Array(a)) if a.is_empty() => "none".into(),
        Some(other) => other.to_string(),
    };
    (text.chars().count() <= 32 && !text.chars().any(char::is_control)).then_some(text)
}

/// Note each difference between `before` and `after` ([`settings`]) as made by `origin`.
pub fn note(s: &mut super::Settings, before: &BTreeMap<String, Value>, after: &BTreeMap<String, Value>, origin: Origin) {
    let now = diagnostics::now();
    for name in before.keys().chain(after.keys().filter(|k| !before.contains_key(*k))) {
        let (a, b) = (before.get(name), after.get(name));
        if a == b {
            continue;
        }
        let change = match (shown(name, a), shown(name, b)) {
            (Some(a), Some(b)) => format!("{a} → {b}"),
            _ => "changed".into(),
        };
        diagnostics::record(s, now, Entry::SettingChanged { name, change: &change, origin });
    }
}

/// Before a setting command the person runs (`access::gate`): what the settings are now.
pub fn before(s: &mut Session, id: &'static str) {
    let now = serde_json::to_value(&s.prefs).map(|v| settings(&v)).unwrap_or_default();
    s.agents.settings_before = Some((id, now));
}

/// After a command succeeded (`Session::note_change`): the changes the person's setting command
/// made, noted with where they made it.
pub fn after(s: &mut Session, id: &str) {
    let Some((was, before)) = s.agents.settings_before.take() else { return };
    if was != id || s.agents.author.is_agent() {
        return;
    }
    let after = serde_json::to_value(&s.prefs).map(|v| settings(&v)).unwrap_or_default();
    let origin = if s.agents.from_settings { Origin::Settings } else { Origin::Command };
    note(&mut s.prefs.settings, &before, &after, origin);
}

#[cfg(test)]
#[path = "settings_changes_tests.rs"]
mod tests;
