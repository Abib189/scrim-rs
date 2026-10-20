//! Scrim's own preferences (Stage C, `docs/design/settings-and-connect.md`), kept in one struct
//! flattened into [`crate::Prefs`] so they sit in `state.json` beside the others. Each field has a
//! safe default, and a value that is missing or can't be read falls back to it (`serde(default)`
//! here, and [`Settings::sanitized`] for values that read but make no sense).

use serde::{Deserialize, Serialize};

/// The most files the Missing fonts window remembers not to open for.
pub const MAX_MUTED_FONT_FILES: usize = 500;

/// The port the MCP endpoint tries first (the brief's choice; the next free port is used and kept
/// when it's taken).
pub const DEFAULT_MCP_PORT: u16 = 7600;

/// The app's MCP endpoint (Settings ▸ MCP, Tool access).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSettings {
    /// The port last served on (the endpoint tries it first).
    pub port: u16,
    /// Requests need `Authorization: Bearer <token>`.
    pub require_auth: bool,
    /// The bearer token (made when authentication is first turned on; kept in the settings file,
    /// which only its owner can read).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub token: String,
    /// The folder file tools (open_file, save_file, export) are confined to; `None`: the home folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_dir: Option<String>,
    /// MCP tools turned off (Settings ▸ Tool access): hidden from `tools/list` and refused.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub disabled_tools: Vec<String>,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self { port: DEFAULT_MCP_PORT, require_auth: false, token: String::new(), root_dir: None, disabled_tools: Vec::new() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The MCP endpoint's settings.
    pub mcp: McpSettings,
    /// Files (their paths) the Missing fonts window doesn't open for by itself any more (its
    /// "Don't show again for this file"). Text ▸ Missing fonts still shows it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fonts_muted: Vec<String>,
}

impl Settings {
    /// These settings with anything out of range put back in range.
    pub fn sanitized(mut self) -> Self {
        if self.mcp.port < 1024 {
            self.mcp.port = DEFAULT_MCP_PORT;
        }
        if self.mcp.require_auth && self.mcp.token.len() < 32 {
            self.mcp.token = new_token();
        }
        self.mcp.disabled_tools.sort();
        self.mcp.disabled_tools.dedup();
        self.fonts_muted.retain(|p| !p.trim().is_empty());
        self.fonts_muted.dedup();
        let excess = self.fonts_muted.len().saturating_sub(MAX_MUTED_FONT_FILES);
        self.fonts_muted.drain(..excess);
        self
    }
}

/// A new bearer token: 64 hex digits from the standard library's randomly keyed hasher (keyed from the
/// operating system's random source), mixed with the time and a counter.
pub fn new_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    (0..4)
        .map(|i| {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            h.write_u64(i);
            // (The browser has no system clock here; it has no endpoint either.)
            #[cfg(not(target_arch = "wasm32"))]
            h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
            format!("{:016x}", h.finish())
        })
        .collect()
}

/// A request from a command to the UI (open a dialog or panel, show a message). Programmatic
/// callers ignore them. (Moved from `lib.rs`.)
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UiRequest {
    pub kind: String,
    pub data: serde_json::Value,
}
