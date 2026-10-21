//! Settings ▸ Diagnostics (`docs/design/settings-and-connect.md`, C3d): a short list of what
//! happened, kept on this computer only (in the preferences) for as long as the person chooses,
//! so a problem can be described. Nothing is ever sent anywhere.
//!
//! **Only allow-listed facts go in.** An event is made from an [`Entry`], never from free text:
//! - a file reverted to its saved copy (no name or path);
//! - the fonts a file falls back from (family names only);
//! - the MCP server starting (its port) or failing to;
//! - an MCP client connecting (its name and version as it gave them);
//! - a command failing (its id, when it is a registered command, and the error's category; never
//!   the message, which can quote the document).
//!
//! So no prompts, document content, paths or tokens can reach it; `diagnostics_tests.rs` scans for
//! them.

use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::settings::Settings;

/// The most events kept, whatever the retention.
pub const MAX_EVENTS: usize = 1000;
/// The retention choices, in days (0: keep nothing).
pub const RETENTION_DAYS: [u32; 4] = [0, 1, 7, 30];
/// The retention unless the person picks another.
pub const DEFAULT_RETENTION_DAYS: u32 = 7;
/// The most font families one event names.
const MAX_FAMILIES: usize = 30;

/// One thing that happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Seconds since 1970 (UTC).
    pub at: u64,
    /// `reload`, `fontFallback`, `mcpServer`, `mcpClient` or `error`.
    pub kind: String,
    pub detail: String,
}

/// What can be recorded: each variant carries only what its event may say.
pub enum Entry<'a> {
    /// A file was reverted to its saved copy (File ▸ Revert).
    Reload,
    /// A file opened with fonts it doesn't have: their family names.
    FontFallback(&'a [String]),
    /// The MCP server started, on this port.
    McpStarted(u16),
    /// The MCP server couldn't start.
    McpFailed,
    /// An MCP client initialized.
    McpClient { name: &'a str, version: &'a str },
    /// A command failed: its id and the error.
    Error { command: &'a str, error: &'a EngineError },
}

/// `s` without control characters, at most `max` characters.
fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

/// The kind of error, never its message.
pub fn category(e: &EngineError) -> &'static str {
    match e {
        EngineError::UnknownCommand(_) => "unknown command",
        EngineError::Disabled(..) => "not available now",
        EngineError::BadParams { .. } => "invalid parameters",
        EngineError::NoDocument => "no open document",
        EngineError::NoNode(_) => "no such layer",
        EngineError::Other(_) => "failed",
        EngineError::Waiting => "waiting",
        EngineError::Internal { .. } => "internal error",
    }
}

impl Entry<'_> {
    fn kind(&self) -> &'static str {
        match self {
            Entry::Reload => "reload",
            Entry::FontFallback(_) => "fontFallback",
            Entry::McpStarted(_) | Entry::McpFailed => "mcpServer",
            Entry::McpClient { .. } => "mcpClient",
            Entry::Error { .. } => "error",
        }
    }

    fn detail(&self) -> String {
        match self {
            Entry::Reload => "a file was reverted to its saved copy".into(),
            Entry::FontFallback(families) => {
                let mut names: Vec<String> = families.iter().take(MAX_FAMILIES).map(|f| clean(f, 64)).collect();
                if families.len() > MAX_FAMILIES {
                    names.push(format!("and {} more", families.len() - MAX_FAMILIES));
                }
                format!("fonts not on this computer, a fallback used: {}", names.join(", "))
            }
            Entry::McpStarted(port) => format!("started on 127.0.0.1:{port}"),
            Entry::McpFailed => "couldn't start".into(),
            Entry::McpClient { name, version } => {
                // As announced (cleaned); a client that announced no name is "an agent", never a guess.
                let name = Some(clean(name, 80)).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "an agent".into());
                format!("{name} {} connected", clean(version, 40)).replace("  ", " ")
            }
            Entry::Error { command, error } => {
                // Only a registered command's id: anything else could be text someone typed.
                let id = if crate::find_command(command).is_some() { *command } else { "a command" };
                format!("{id}: {}", category(error))
            }
        }
    }
}

/// Note `entry` at `now` (seconds since 1970), then drop what the retention no longer keeps.
pub fn record(settings: &mut Settings, now: u64, entry: Entry) {
    if matches!(entry, Entry::Error { error: EngineError::Waiting, .. }) {
        return;
    }
    settings.diagnostics.push(Event { at: now, kind: entry.kind().into(), detail: entry.detail() });
    prune(settings, now);
}

/// Drop events older than the retention allows, and the oldest past [`MAX_EVENTS`].
pub fn prune(settings: &mut Settings, now: u64) {
    let keep = u64::from(settings.diagnostics_days) * 86_400;
    settings.diagnostics.retain(|e| settings.diagnostics_days > 0 && now.saturating_sub(e.at) <= keep);
    let excess = settings.diagnostics.len().saturating_sub(MAX_EVENTS);
    settings.diagnostics.drain(..excess);
}

/// `secs` since 1970 as `2026-10-20 14:03:09 UTC`.
pub fn timestamp(secs: u64) -> String {
    // Days to a civil date (Howard Hinnant's civil_from_days).
    let z = i64::try_from(secs / 86_400).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let s = secs % 86_400;
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", s / 3600, s / 60 % 60, s % 60)
}

/// The events as text, one a line, oldest first (Copy and Save).
pub fn text(events: &[Event]) -> String {
    events.iter().map(|e| format!("{}  {}  {}\n", timestamp(e.at), e.kind, e.detail)).collect()
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
