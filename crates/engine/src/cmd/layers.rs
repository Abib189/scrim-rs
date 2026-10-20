//! The Layers panel's own commands (D1, `docs/design/layers-collapse-and-focus.md`): state of the
//! panel, never of the document.

use serde_json::json;

use super::{CommandSpec, ok};
use crate::spec;

/// The UI request `layers.collapse-all` makes (the panel acts on it).
pub const COLLAPSE_REQUEST: &str = "layers.collapse-all";

/// Commands that change only what a panel shows: no undo step, no document change, and the
/// document's revision stays as it was (`Session::after_command`).
pub const VIEW_ONLY: &[&str] = &["layers.collapse-all"];

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "layers.collapse-all",
        "Collapse layers",
        "{} → {}  (closes every row in the Layers panel, on every page; the selection stays as it is. Panel state only: no undo step, no document change, no new revision)",
        false,
        |s, _| {
            s.request_ui(COLLAPSE_REQUEST, json!({}));
            ok()
        }
    )]
}

#[cfg(test)]
#[path = "layers_tests.rs"]
mod tests;
