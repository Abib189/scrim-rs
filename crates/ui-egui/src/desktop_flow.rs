//! Connect Claude Desktop (Stage F, `docs/design/connect-claude-desktop.md`): the second target of
//! the connect flow, beside Connect Claude Code (`connect_flow`), in the AI tab and Settings ▸ MCP.
//!
//! - **The panel** (`mcp.connect-claude-desktop`) shows the exact file and the exact entry, with
//!   Write and Copy (`mcp.copy-claude-desktop-entry`).
//! - **Writing asks once** (`mcp.write-claude-desktop`, a dialog). Then
//!   `figcraft_engine::cmd::claude_desktop::apply`:
//!   - refuses an invalid file and leaves it alone;
//!   - changes only `mcpServers.scrim`;
//!   - keeps a timestamped backup;
//!   - writes by rename.
//! - **An entry for another address** is Out of date. Re-register replaces it after one confirmation;
//!   it is never overwritten silently. An old `figcraft` entry can be removed (asks again).
//! - **The status row** reads the file (Configured, Not configured, Out of date, Can't read), never
//!   "connected": Claude Desktop can't be asked. After a write, the person is told to restart Claude
//!   Desktop; Scrim never restarts or closes it.
//!
//! **Agents can't write.** The confirmations are dialogs, which an agent's commands never open.

use egui::{Context, RichText, Ui};
use figcraft_engine::cmd::claude_desktop::{self as cd, Change, KEY};
use serde_json::{Value, json};

use crate::i18n::{tr, trf};
use crate::theme::{Tokens, regular};
use crate::{FigApp, widgets};

/// A confirmation being asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Confirm {
    Write,
    Reregister,
    RemoveOld,
}

/// The flow's state (one per window).
#[derive(Default)]
pub struct Flow {
    pub open: bool,
    confirm: Option<Confirm>,
    /// What the last write said.
    outcome: Option<Result<String, String>>,
    /// The file's status as last read, and when (egui seconds): read again every two seconds while
    /// it shows, and at once after a write.
    seen: Option<(f64, Value)>,
}

/// The file's status for this window (the cached read, or a fresh one).
fn report(app: &mut FigApp, now: f64) -> Option<Value> {
    if let Some((at, v)) = &app.ui.desktop.seen
        && now - at < 2.0
    {
        return Some(v.clone());
    }
    let npx = app.services.find_program("npx");
    let v = cd::report(&app.session, npx.as_deref()).ok()?;
    app.ui.desktop.seen = Some((now, v.clone()));
    Some(v)
}

fn forget(app: &mut FigApp) {
    app.ui.desktop.seen = None;
}

/// Open the panel (`mcp.connect-claude-desktop`).
pub fn open(app: &mut FigApp) {
    app.ui.desktop = Flow { open: true, ..Flow::default() };
}

/// Act on the flow's requests; `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str) -> bool {
    let confirm = match kind {
        cd::CONNECT_REQUEST => {
            open(app);
            return true;
        }
        cd::WRITE_DIALOG => Confirm::Write,
        cd::REREGISTER_DIALOG => Confirm::Reregister,
        cd::REMOVE_OLD_DIALOG => Confirm::RemoveOld,
        _ => return false,
    };
    app.ui.desktop.open = true;
    app.ui.desktop.confirm = Some(confirm);
    true
}

fn now_secs() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Do what the person confirmed.
fn act(app: &mut FigApp, c: Confirm, r: &Value) {
    let Some(path) = r["path"].as_str().map(std::path::PathBuf::from) else { return };
    let changes: Vec<Change> = match c {
        Confirm::Write | Confirm::Reregister => vec![Change::Set(KEY.into(), r["entry"][KEY].clone())],
        Confirm::RemoveOld => r["old"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|k| Change::Remove(k.into())).collect(),
    };
    let mut backup = None;
    let mut result = Ok(());
    for ch in &changes {
        match cd::apply(&path, ch, now_secs()) {
            // The first backup is the file as the person had it.
            Ok(w) => backup = backup.or(w.backup),
            Err(e) => {
                result = Err(e);
                break;
            }
        }
    }
    forget(app);
    let kept = backup.and_then(|b| b.file_name().map(|n| n.to_string_lossy().into_owned()));
    app.ui.desktop.outcome = Some(match (result, c) {
        (Err(e), _) => Err(e),
        (Ok(()), Confirm::RemoveOld) => Ok(tr("The old entry was removed from Claude Desktop's configuration.").into()),
        (Ok(()), _) => Ok(match kept {
            Some(k) => trf(
                "Scrim was added to Claude Desktop. Quit Claude Desktop completely and open it again to use it. The previous file was kept as {name}.",
                &[("name", &k)],
            ),
            None => tr("Scrim was added to Claude Desktop. Quit Claude Desktop completely and open it again to use it.").into(),
        }),
    });
}

/// The status row's words for a state.
fn state_label(state: &str) -> &'static str {
    match state {
        "configured" => tr("Configured"),
        "out of date" => tr("Out of date"),
        "unreadable" => tr("Can't read its configuration"),
        _ => tr("Not configured"),
    }
}

/// Claude Desktop's section: the status from its file, Re-register when it is out of date, and the
/// button or the panel.
pub fn section(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Claude Desktop"), &[], false);
    let now = ui.ctx().input(|i| i.time);
    let Some(r) = report(app, now) else {
        widgets::note(ui, tr("The MCP endpoint isn't running: restart it from Settings, under MCP."), t.text_secondary);
        return;
    };
    let state = r["state"].as_str().unwrap_or_default().to_string();
    let (label, color) = (
        state_label(&state),
        match state.as_str() {
            "configured" => t.success,
            "out of date" => t.warning,
            "unreadable" => t.danger,
            _ => t.text_secondary,
        },
    );
    crate::right::row(ui, 28.0, |ui| {
        let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, color);
        let resp = ui.label(RichText::new(label).color(t.text).font(regular(12.0)));
        crate::a11y::status(&resp, label, &trf("Claude Desktop: {state}", &[("state", &label)]));
    });
    if state == "out of date" {
        let old = r["registered"].as_str().unwrap_or_default();
        let new = r["url"].as_str().unwrap_or_default();
        widgets::note(ui, tr("Registered address is out of date"), t.warning);
        widgets::note(ui, &trf("Claude Desktop points to {old}; this window is at {new}.", &[("old", &old), ("new", &new)]), t.text_secondary);
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Re-register"), true, r["path"].is_string()).clicked() {
                app.run("mcp.reregister-claude-desktop", json!({}));
            }
        });
    }
    if app.ui.desktop.open {
        panel(app, ui, &r);
    } else if state != "configured" {
        widgets::note(
            ui,
            tr("Let Claude Desktop use this canvas: Scrim adds itself to Claude Desktop's configuration, after you confirm."),
            t.text_secondary,
        );
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Connect Claude Desktop"), state == "not configured", true).clicked() {
                app.run("mcp.connect-claude-desktop", json!({}));
            }
        });
    }
}

/// The panel: the file, the entry, Write and Copy, and what happened.
fn panel(app: &mut FigApp, ui: &mut Ui, r: &Value) {
    let t = Tokens::get(ui.ctx());
    let state = r["state"].as_str().unwrap_or_default();
    let mono = |ui: &mut Ui, text: &str| {
        egui::Frame::NONE
            .fill(t.field)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .outer_margin(egui::Margin::symmetric(16, 0))
            .show(ui, |ui| {
                ui.label(RichText::new(text).monospace().color(t.text));
            });
    };
    let path = r["path"].as_str();
    let entry = cd::splice::ordered(&r["entry"]).unwrap_or_default();
    match path {
        Some(p) => {
            widgets::note(ui, tr("Scrim will add this entry to Claude Desktop's configuration file:"), t.text_secondary);
            mono(ui, p);
        }
        None => widgets::note(
            ui,
            tr("There's no known place for Claude Desktop's configuration on this system: copy the entry and add it yourself."),
            t.text_secondary,
        ),
    }
    ui.add_space(4.0);
    mono(ui, &entry);
    ui.add_space(6.0);
    if r["documentedPath"] == false && path.is_some() {
        widgets::note(
            ui,
            tr("Anthropic's documentation names no file on Linux: this is where Claude Desktop keeps it by default."),
            t.text_secondary,
        );
    }
    let npx = r["npx"] == true;
    if !npx {
        widgets::note(
            ui,
            tr(
                "Node.js's npx wasn't found on this computer, and Claude Desktop needs it to reach Scrim. Install Node.js, or copy the entry and add it yourself.",
            ),
            t.text_secondary,
        );
    }
    if app.session.prefs.settings.mcp.require_auth {
        widgets::note(ui, tr("MCP authentication is on: the token is written into this file."), t.text_secondary);
    }
    match state {
        "unreadable" => widgets::note(
            ui,
            &trf(
                "Claude Desktop's configuration can't be read, so Scrim won't change it: {error}. Fix or move the file, then try again.",
                &[("error", &r["error"].as_str().unwrap_or_default())],
            ),
            t.danger,
        ),
        "configured" => widgets::note(ui, tr("Claude Desktop already has Scrim at this address."), t.text),
        _ => {}
    }
    let can_write = npx && path.is_some() && matches!(state, "not configured" | "out of date");
    crate::right::row(ui, 32.0, |ui| {
        if can_write {
            let (label, cmd) =
                if state == "out of date" { (tr("Re-register"), "mcp.reregister-claude-desktop") } else { (tr("Write"), "mcp.write-claude-desktop") };
            if widgets::button(ui, label, true, true).clicked() {
                app.run(cmd, json!({}));
            }
        }
        if widgets::button(ui, tr("Copy"), !can_write, true).clicked() {
            app.run("mcp.copy-claude-desktop-entry", json!({"npx": app.services.find_program("npx")}));
        }
        if widgets::button(ui, tr("Close"), false, true).clicked() {
            app.ui.desktop.open = false;
        }
    });
    match &app.ui.desktop.outcome {
        // (In the text colour, like Connect Claude Code's.)
        Some(Ok(m)) => widgets::note(ui, m, t.text),
        Some(Err(e)) => widgets::note(ui, &trf("It didn't work: {error}", &[("error", e)]), t.danger),
        None => {}
    }
    let old: Vec<&str> = r["old"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    if !old.is_empty() && path.is_some() && state != "unreadable" {
        widgets::note(
            ui,
            &trf("Claude Desktop still has the old {keys} entry from before the rename.", &[("keys", &old.join(", "))]),
            t.text_secondary,
        );
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Remove the old entry"), false, true).clicked() {
                app.run("mcp.remove-old-desktop-entry", json!({}));
            }
        });
    }
}

/// The confirmations. Only the person can answer them.
pub fn dialogs(app: &mut FigApp, ctx: &Context) {
    let Some(c) = app.ui.desktop.confirm else { return };
    let t = Tokens::get(ctx);
    // Read the file again: what is asked about is what is there now.
    forget(app);
    let Some(r) = report(app, ctx.input(|i| i.time)) else {
        app.ui.desktop.confirm = None;
        return;
    };
    let path = r["path"].as_str().unwrap_or_default().to_string();
    let old: Vec<String> = r["old"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
    let state = r["state"].as_str().unwrap_or_default();
    // Nothing to do (or nothing it may do): don't ask.
    let pointless = path.is_empty()
        || state == "unreadable"
        || match c {
            Confirm::Write => state != "not configured",
            Confirm::Reregister => state != "out of date",
            Confirm::RemoveOld => old.is_empty(),
        };
    if pointless {
        app.ui.desktop.confirm = None;
        return;
    }
    let (title, body, go_label) = match c {
        Confirm::Write => (
            tr("Write to Claude Desktop's configuration?"),
            tr("Scrim will add its entry to this file. It keeps a copy of the file first, and changes nothing else in it.").to_string(),
            tr("Write"),
        ),
        Confirm::Reregister => (
            tr("Re-register Scrim in Claude Desktop?"),
            trf(
                "Scrim will replace Claude Desktop's scrim entry, which points to {old}, with this window's address. It keeps a copy of the file first.",
                &[("old", &r["registered"].as_str().unwrap_or_default())],
            ),
            tr("Re-register"),
        ),
        Confirm::RemoveOld => (
            tr("Remove the old entry?"),
            trf(
                "Scrim will remove the old {keys} entry from Claude Desktop's configuration. It keeps a copy of the file first.",
                &[("keys", &old.join(", "))],
            ),
            tr("Remove"),
        ),
    };
    let (mut go, mut cancel, mut open) = (false, false, true);
    crate::dialogs::window(title, ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.label(RichText::new(&body).font(regular(11.0)).color(t.text_secondary));
        ui.add_space(6.0);
        ui.label(RichText::new(&path).monospace().color(t.text));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            go = widgets::button(ui, go_label, true, true).clicked();
            cancel = widgets::button(ui, tr("Cancel"), false, true).clicked();
        });
    });
    if go {
        app.ui.desktop.confirm = None;
        act(app, c, &r);
    } else if cancel || !open {
        app.ui.desktop.confirm = None;
    }
}

#[cfg(test)]
#[path = "desktop_flow_tests.rs"]
mod tests;
