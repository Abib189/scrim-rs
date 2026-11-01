//! App-wide preferences that commands read (moved out of `lib.rs`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{cmd, live, online, plugins, settings};

/// App-wide preferences that commands read (nudge amounts, behaviours).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub nudge_small: f64,
    pub nudge_big: f64,
    pub keep_tool: bool,
    pub rename_duplicates: bool,
    pub highlight_on_hover: bool,
    pub number_keys_opacity: bool,
    pub show_dimensions: bool,
    /// "system" by default; an explicit "light" or "dark" choice stays saved.
    pub theme: String,
    /// The widths of the main window's left and right panels (the interface keeps them within its
    /// limits; other windows have their own while they are open).
    pub panel_widths: (f32, f32),
    /// The height asked for the main window's pages list, dragged at its lower edge (`None`: a
    /// share of the panel; the interface keeps it within its limits).
    pub pages_height: Option<f32>,
    /// The interface language: a language code ("en", "es", "pt-BR") or "system" (the OS or browser
    /// language). Languages without a catalog show English.
    pub language: String,
    /// The name comments are signed with (and whose comments count as yours).
    pub user_name: String,
    /// Reduce motion: the interface's own animations are off (the system's setting turns them off
    /// too). Prototype transitions still play.
    pub reduce_motion: bool,
    /// Files that have a path are saved by themselves a few seconds after they change (the app's
    /// timer; off: only File ▸ Save writes them).
    pub auto_save: bool,
    /// `figcraft://` invite links open this app (`collab.registerLinks`); `None`: on for installed
    /// builds, off for development builds.
    pub invite_links: Option<bool>,
    /// Installed plugins (Plugins menu), in menu order.
    #[serde(serialize_with = "plugins::persisted")]
    pub plugins: Vec<plugins::InstalledPlugin>,
    /// Plugins' own settings (`setting_set`), by plugin id, then key: JSON values, capped per plugin.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plugin_settings: BTreeMap<String, BTreeMap<String, Value>>,
    /// The plugin directory browsed last (`ws://host:port/`, never with a key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_directory: Option<String>,
    /// Scrim's own preferences (Stage C).
    #[serde(flatten)]
    pub settings: settings::Settings,
    /// The accent colour (`pref.accent`): `#rrggbb`, or empty for the default.
    pub accent: String,
    /// The server files are shared through online (Share ▸ Online, join codes; Publish to server
    /// suggests it too): `wss://host/`, never with a key. [`live::DEFAULT_SERVER`] unless the user
    /// chose another; empty: online sharing is off (saved as empty, so it stays off).
    pub publish_server: String,
    /// The right panel's mode and tab chosen last (`panel.mode.*`, `panel.tab.*`).
    pub panel: cmd::panel::PanelView,
    /// Recently opened or saved files, newest first (File ▸ Open recent; at most [`cmd::file::MAX_RECENT`]).
    pub recent_files: Vec<String>,
    /// The files you put on a server without its key, with their owner tokens (passwords: see
    /// [`online`]), most recently opened first, at most [`online::MAX_ONLINE_FILES`].
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "online::read")]
    pub online_files: Vec<online::OnlineFile>,
    /// The Getting started tips on the home screen were closed (Help ▸ Getting started shows them
    /// again).
    pub getting_started_dismissed: bool,
    /// How each home screen section shows its files, by section id (`"recents"`): `"grid"` (cards
    /// with the picture each file carries) or `"list"` (a table). Missing: the grid.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub home_views: BTreeMap<String, String>,
    /// What the home screen's recent files are ordered by: `"modified"`, `"name"` or `"created"`.
    pub home_sort: String,
    /// Which way: `"desc"` (newest first, Z to A) or `"asc"`.
    pub home_order: String,
    /// The View menu's toggles turned on or off (rulers, pixel grid, snapping…), by command id:
    /// files opened or made later start with them. Missing: the toggle's default.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub view: BTreeMap<String, bool>,
    /// Recently run actions (the Actions palette's Recents), newest first, at most
    /// [`MAX_RECENT_ACTIONS`]. Entries are `command:<id>`, `plugin:<plugin>/<command>` or
    /// `widget:<plugin>`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_actions: Vec<String>,
}

/// Recently run actions kept for the Actions palette (newest first).
pub const MAX_RECENT_ACTIONS: usize = 5;

impl Default for Prefs {
    fn default() -> Self {
        Self {
            nudge_small: 1.0,
            nudge_big: 10.0,
            keep_tool: false,
            rename_duplicates: false,
            highlight_on_hover: true,
            number_keys_opacity: true,
            show_dimensions: true,
            theme: "system".into(),
            panel_widths: (240.0, 240.0),
            pages_height: None,
            language: "system".into(),
            user_name: cmd::comment::DEFAULT_AUTHOR.into(),
            reduce_motion: false,
            auto_save: false,
            invite_links: None,
            plugins: Vec::new(),
            plugin_settings: BTreeMap::new(),
            plugin_directory: None,
            publish_server: live::DEFAULT_SERVER.into(),
            accent: String::new(),
            settings: Default::default(),
            panel: Default::default(),
            recent_files: Vec::new(),
            online_files: Vec::new(),
            getting_started_dismissed: false,
            home_views: BTreeMap::new(),
            home_sort: "modified".into(),
            home_order: "desc".into(),
            view: BTreeMap::new(),
            recent_actions: Vec::new(),
        }
    }
}
