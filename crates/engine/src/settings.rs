//! Scrim's own preferences (Stage C, `docs/design/settings-and-connect.md`), kept in one struct
//! flattened into [`crate::Prefs`] so they sit in `state.json` beside the others. Each field has a
//! safe default, and a value that is missing or can't be read falls back to it (`serde(default)`
//! here, and [`Settings::sanitized`] for values that read but make no sense).

use serde::{Deserialize, Serialize};

// (Here, not in `lib.rs`: that file is at its size cap.)
#[path = "diagnostics.rs"]
pub mod diagnostics;

/// The most files the Missing fonts window remembers not to open for.
pub const MAX_MUTED_FONT_FILES: usize = 500;

/// The port the MCP endpoint tries first (C1.2: 7680; 7600, the first default, is OpenPencil's). The
/// next free port is used and kept when it's taken.
pub const DEFAULT_MCP_PORT: u16 = 7680;

/// The ports the first default (7600) and its fallback could have kept (`TRIES` = 20).
const OLD_DEFAULT_PORTS: std::ops::Range<u16> = 7600..7620;

/// The app's MCP endpoint (Settings ▸ MCP, Tool access).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSettings {
    /// The port last served on (the endpoint tries it first).
    pub port: u16,
    /// The person chose `port` (`mcp.set-port`). Otherwise it is the default or its fallback, which a
    /// change of default may move.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub port_chosen: bool,
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
        Self { port: DEFAULT_MCP_PORT, port_chosen: false, require_auth: false, token: String::new(), root_dir: None, disabled_tools: Vec::new() }
    }
}

/// The settings folder's name under the platform's application-settings folder (the app id).
pub const APP_DIR: &str = "design.scrim.app";

/// The platform's folder for application settings (Application Support, AppData, ~/.config).
#[cfg(not(target_arch = "wasm32"))]
pub fn config_base_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base
}

/// The folder agents' file tools use unless the person chose one (C1.4): `~/Documents/Scrim`, made
/// the first time an agent's request names a file. Never the home folder itself.
pub fn default_mcp_root() -> Option<std::path::PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    #[cfg(target_arch = "wasm32")]
    let home: Option<std::ffi::OsString> = None;
    home.map(|h| std::path::PathBuf::from(h).join("Documents").join("Scrim"))
}

/// The MCP settings the app saved in `dir/state.json` (the defaults when there are none), for an MCP
/// server that runs without the app (`scrim mcp`): it honours Settings ▸ Tool access too.
#[cfg(not(target_arch = "wasm32"))]
pub fn saved_mcp(dir: &std::path::Path) -> McpSettings {
    crate::file_access::read_capped(dir.join("state.json"), 16 << 20)
        .ok()
        .and_then(|s| serde_json::from_slice::<serde_json::Value>(&s).ok())
        .and_then(|v| serde_json::from_value::<crate::Prefs>(v.get("prefs")?.clone()).ok())
        .map(|p| p.settings.sanitized().mcp)
        .unwrap_or_default()
}

/// The version of these settings `state.json` holds (`settings_version`). A file from a later
/// version is read as far as it goes, never refused.
/// - 1: Stage C.
/// - 2: the MCP port's default moved from 7600 to 7680 (C1.2).
/// - 3: `follow_agents` (Stage I), on by default.
/// - 4: `reveal_agent_work` and `working_indicator` (Stage I2), on by default.
/// - 5: `agent_grace_secs` (Stage I.1), 15 s by default.
/// - 6: `reveal_speed` (Stage I.1), normal by default.
/// - 7: the grace's default moved from 15 s to 30 s and its choices to 15, 30, 60 and 120 s (Fix Q).
///   A saved 5 or 15 that the person didn't choose (`agent_grace_chosen`) moves to 30.
pub const SETTINGS_VERSION: u32 = 7;

/// How fast an agent's new layers enter (`pref.revealSpeed`), slowest first.
pub const REVEAL_SPEEDS: [&str; 3] = ["slow", "normal", "fast"];

fn normal() -> String {
    "normal".into()
}

/// How long the working indicator stays after an agent's last request (`pref.agentGrace`): the
/// choices Settings offers, never under 5 s.
pub const AGENT_GRACE_SECS: [u32; 4] = [15, 30, 60, 120];
/// Real agents pause 5–15 s between calls while they write the next batch (Fix Q).
pub const DEFAULT_AGENT_GRACE_SECS: u32 = 30;

fn grace() -> u32 {
    DEFAULT_AGENT_GRACE_SECS
}

/// The Settings sections (`settings.open {section}`).
pub const SECTIONS: [&str; 7] = ["general", "toolbar", "ai", "mcp", "tools", "privacy", "diagnostics"];

fn yes() -> bool {
    true
}

fn system() -> String {
    "system".into()
}

fn week() -> u32 {
    diagnostics::DEFAULT_RETENTION_DAYS
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// [`SETTINGS_VERSION`] when written; missing in a file from before versions (read as 0).
    #[serde(default)]
    pub settings_version: u32,
    /// The MCP endpoint's settings.
    pub mcp: McpSettings,
    /// Interface animations: `system` (follow the system's reduce-motion setting), `on` or `off`.
    #[serde(default = "system")]
    pub motion: String,
    /// Keep recovery copies of unsaved work (on by default): off stops new copies; those kept
    /// already stay until their files are saved.
    #[serde(default = "yes")]
    pub preserve_unsaved: bool,
    /// The camera follows agents' edits (`camera.follow-agents`; on by default). Pausing it is the
    /// window's own state and isn't saved.
    #[serde(default = "yes")]
    pub follow_agents: bool,
    /// Layers an agent creates arrive with a short entrance on the canvas (`pref.revealAgentWork`).
    #[serde(default = "yes")]
    pub reveal_agent_work: bool,
    /// The dot row above the artboard an agent works on (`pref.workingIndicator`).
    #[serde(default = "yes")]
    pub working_indicator: bool,
    /// An agent counts as working for this long after its last request of any kind, unless it calls
    /// `finish_working` or its session ends (`pref.agentGrace`; one of [`AGENT_GRACE_SECS`]).
    #[serde(default = "grace")]
    pub agent_grace_secs: u32,
    /// The person chose `agent_grace_secs` (Settings ▸ General, or `pref.agentGrace` from the
    /// window): a migration leaves it as it is (Fix Q). Agents can't set it.
    #[serde(default)]
    pub agent_grace_chosen: bool,
    /// How fast an agent's new layers enter: `slow`, `normal` or `fast` (`pref.revealSpeed`; fast
    /// is Stage I2's timing).
    #[serde(default = "normal")]
    pub reveal_speed: String,
    /// The toolbar (Settings ▸ Toolbar); empty: the default (`cmd::toolbar`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub toolbar: crate::cmd::toolbar::Layout,
    /// Files (their paths) the Missing fonts window doesn't open for by itself any more (its
    /// "Don't show again for this file"). Text ▸ Missing fonts still shows it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fonts_muted: Vec<String>,
    /// Settings ▸ Diagnostics: what happened, on this computer only (`diagnostics`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<diagnostics::Event>,
    /// How many days diagnostics are kept (0: none).
    #[serde(default = "week")]
    pub diagnostics_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            settings_version: SETTINGS_VERSION,
            mcp: McpSettings::default(),
            motion: system(),
            preserve_unsaved: true,
            follow_agents: true,
            reveal_agent_work: true,
            working_indicator: true,
            agent_grace_secs: grace(),
            agent_grace_chosen: false,
            reveal_speed: normal(),
            toolbar: Vec::new(),
            fonts_muted: Vec::new(),
            diagnostics: Vec::new(),
            diagnostics_days: week(),
        }
    }
}

impl Settings {
    /// These settings with anything out of range put back in range.
    pub fn sanitized(mut self) -> Self {
        // Version 2 moved the default port. A port nobody chose that the first default (or its
        // fallback) kept moves with it; a chosen one stays.
        if self.settings_version < 2 && !self.mcp.port_chosen && OLD_DEFAULT_PORTS.contains(&self.mcp.port) {
            self.mcp.port = DEFAULT_MCP_PORT;
        }
        // Version 7 moved the grace's default to 30 s: a 5 or 15 nobody chose (the old default, or
        // a value an agent or a script set) moves with it; a chosen one stays.
        if self.settings_version < 7 && !self.agent_grace_chosen && matches!(self.agent_grace_secs, 5 | 15) {
            self.agent_grace_secs = DEFAULT_AGENT_GRACE_SECS;
        }
        self.settings_version = SETTINGS_VERSION;
        if !matches!(self.motion.as_str(), "system" | "on" | "off") {
            self.motion = system();
        }
        if self.mcp.port < 1024 {
            self.mcp.port = DEFAULT_MCP_PORT;
        }
        if self.mcp.require_auth && self.mcp.token.len() < 32 {
            self.mcp.token = new_token();
        }
        self.mcp.disabled_tools.sort();
        self.mcp.disabled_tools.dedup();
        if !REVEAL_SPEEDS.contains(&self.reveal_speed.as_str()) {
            self.reveal_speed = normal();
        }
        if !AGENT_GRACE_SECS.contains(&self.agent_grace_secs) {
            // Another value (a chosen 5 s, no longer offered, among them) goes to the nearest choice.
            let g = self.agent_grace_secs;
            self.agent_grace_secs = AGENT_GRACE_SECS.iter().copied().min_by_key(|c| c.abs_diff(g)).unwrap_or(DEFAULT_AGENT_GRACE_SECS);
        }
        if !diagnostics::RETENTION_DAYS.contains(&self.diagnostics_days) {
            self.diagnostics_days = week();
        }
        let excess = self.diagnostics.len().saturating_sub(diagnostics::MAX_EVENTS);
        self.diagnostics.drain(..excess);
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
