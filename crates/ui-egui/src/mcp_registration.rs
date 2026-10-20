//! Is Claude Code's `scrim` entry this window's address? (C1.2, `docs/design/settings-and-connect.md`)
//!
//! The endpoint keeps its port, but it moves when that port is taken (the next free one) or when
//! the person picks another. Claude Code then still points at the old address. While the AI tab or
//! Settings ▸ MCP shows, Scrim asks `claude mcp get scrim` (an argument list, never a shell; it
//! changes nothing) once per address. When the entry points elsewhere it says "Registered address
//! is out of date" and offers **Re-register**, which asks once (`mcp.reregister`) and then replaces
//! the entry the way Connect's Replace does.

use egui::{Context, Ui};
use serde_json::json;

use crate::connect_flow::{Entry, parse_get};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::{FigApp, ProgramRun, widgets};

/// What Scrim knows of Claude Code's `scrim` entry.
#[derive(Default)]
pub enum Check {
    /// Not asked yet (or asked again after a change).
    #[default]
    Unknown,
    /// Asking, for this address.
    Running(ProgramRun, String),
    /// Claude Code's entry when the endpoint was at `url` (`None`: it has none).
    Known { url: String, entry: Option<Entry> },
}

/// Ask Claude Code again (after Connect or Re-register changed its entry).
pub fn forget(app: &mut FigApp) {
    app.ui.connect.registered = Check::Unknown;
}

/// Ask Claude Code, once per address, unless the connect flow is running `claude` itself.
fn ensure(app: &mut FigApp) {
    let Some(url) = app.session.agents.mcp.url() else { return };
    let stale = match &app.ui.connect.registered {
        Check::Unknown => true,
        Check::Known { url: asked, .. } => *asked != url,
        Check::Running(..) => false,
    };
    if !stale || app.ui.connect.busy() {
        return;
    }
    let Some(claude) = app.services.find_program("claude") else { return };
    let args: Vec<String> = ["mcp", "get", "scrim"].iter().map(|s| s.to_string()).collect();
    app.ui.connect.registered = Check::Running(app.services.run_program(&claude, &args), url);
}

fn poll(app: &mut FigApp, ctx: &Context) {
    let Check::Running(rx, _) = &app.ui.connect.registered else { return };
    let answer = match rx.try_recv() {
        Ok(out) => out.ok().as_ref().and_then(parse_get),
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
            return;
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => None,
    };
    if let Check::Running(_, url) = std::mem::take(&mut app.ui.connect.registered) {
        app.ui.connect.registered = Check::Known { url, entry: answer };
    }
    ctx.request_repaint();
}

/// Claude Code's `scrim` entry, when it points somewhere other than this window.
pub fn out_of_date(app: &FigApp) -> Option<Entry> {
    match &app.ui.connect.registered {
        Check::Known { url, entry: Some(e) } if e.target != *url => Some(e.clone()),
        _ => None,
    }
}

/// The warning and Re-register, when Claude Code's entry is out of date (AI tab, Settings ▸ MCP).
pub fn row(app: &mut FigApp, ui: &mut Ui) {
    ensure(app);
    poll(app, ui.ctx());
    let Some(e) = out_of_date(app) else { return };
    let t = Tokens::get(ui.ctx());
    let now = app.session.agents.mcp.url().unwrap_or_default();
    widgets::note(ui, tr("Registered address is out of date"), t.warning);
    widgets::note(ui, &trf("Claude Code points to {old}; this window is at {new}.", &[("old", &e.target), ("new", &now)]), t.text_secondary);
    crate::right::row(ui, 32.0, |ui| {
        if widgets::button(ui, tr("Re-register"), true, !app.ui.connect.busy()).clicked() {
            app.run("mcp.reregister", json!({}));
        }
    });
}

#[cfg(test)]
#[path = "mcp_registration_tests.rs"]
mod tests;
