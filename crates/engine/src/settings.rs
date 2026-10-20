//! Scrim's own preferences (Stage C, `docs/design/settings-and-connect.md`), kept in one struct
//! flattened into [`crate::Prefs`] so they sit in `state.json` beside the others. Each field has a
//! safe default, and a value that is missing or can't be read falls back to it (`serde(default)`
//! here, and [`Settings::sanitized`] for values that read but make no sense).

use serde::{Deserialize, Serialize};

/// The most files the Missing fonts window remembers not to open for.
pub const MAX_MUTED_FONT_FILES: usize = 500;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Files (their paths) the Missing fonts window doesn't open for by itself any more (its
    /// "Don't show again for this file"). Text ▸ Missing fonts still shows it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fonts_muted: Vec<String>,
}

impl Settings {
    /// These settings with anything out of range put back in range.
    pub fn sanitized(mut self) -> Self {
        self.fonts_muted.retain(|p| !p.trim().is_empty());
        self.fonts_muted.dedup();
        let excess = self.fonts_muted.len().saturating_sub(MAX_MUTED_FONT_FILES);
        self.fonts_muted.drain(..excess);
        self
    }
}

/// A request from a command to the UI (open a dialog or panel, show a message). Programmatic
/// callers ignore them. (Moved from `lib.rs`.)
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UiRequest {
    pub kind: String,
    pub data: serde_json::Value,
}
