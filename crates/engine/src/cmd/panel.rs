//! The right panel's mode and tab (`docs/design/panel-redesign.md`, B1): `panel.mode.*` switches
//! between Design and Animate, `panel.tab.*` between the Design, Prototype, Code and AI tabs. The
//! UI draws them; these commands are how the person's clicks, the menus and agents switch them,
//! so nothing is UI-only. Animate is a placeholder: nothing in it reads or writes the document.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::CommandSpec;
use crate::{Result, Session, spec};

/// Design (the tabs) or Animate (a placeholder for a later stage).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Design,
    Animate,
}

/// The tabs Design mode shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tab {
    #[default]
    Design,
    Prototype,
    Code,
    Ai,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Design, Tab::Prototype, Tab::Code, Tab::Ai];
    pub fn id(self) -> &'static str {
        match self {
            Tab::Design => "design",
            Tab::Prototype => "prototype",
            Tab::Code => "code",
            Tab::Ai => "ai",
        }
    }
}

/// The mode and tab chosen last: each window keeps its own, and new windows (and the next start)
/// open with the last one chosen anywhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelView {
    pub mode: Mode,
    pub tab: Tab,
}

/// What the UI is asked to show (`panel.view`, with the [`PanelView`] as its data).
pub const REQUEST: &str = "panel.view";

fn set(s: &mut Session, mode: Option<Mode>, tab: Option<Tab>) -> Result<Value> {
    let view = &mut s.prefs.panel;
    if let Some(tab) = tab {
        view.tab = tab;
        view.mode = Mode::Design;
    }
    if let Some(mode) = mode {
        view.mode = mode;
    }
    let view = json!(s.prefs.panel);
    s.request_ui(REQUEST, view.clone());
    Ok(view)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!("panel.mode.design", "Design mode", "{} → {mode, tab}", false, |s, _| set(s, Some(Mode::Design), None)),
        spec!(
            "panel.mode.animate",
            "Animate mode",
            "{} → {mode, tab}  (a placeholder: animation comes in a later stage, and nothing in this mode changes the file)",
            false,
            |s, _| set(s, Some(Mode::Animate), None)
        ),
        spec!("panel.tab.design", "Design tab", "{} → {mode: \"design\", tab}", false, |s, _| set(s, None, Some(Tab::Design))),
        spec!("panel.tab.prototype", "Prototype tab", "{} → {mode: \"design\", tab}", false, |s, _| set(s, None, Some(Tab::Prototype))),
        spec!("panel.tab.code", "Code tab", "{} → {mode: \"design\", tab}", false, |s, _| set(s, None, Some(Tab::Code))),
        spec!("panel.tab.ai", "AI tab", "{} → {mode: \"design\", tab}", false, |s, _| set(s, None, Some(Tab::Ai))),
    ]
}

#[cfg(test)]
#[path = "panel_tests.rs"]
mod tests;
