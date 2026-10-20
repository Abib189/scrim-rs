//! Settings ▸ Privacy and Settings ▸ Diagnostics (C3d).
//!
//! **Privacy** is a statement, not a switch: Scrim has no telemetry or usage statistics to turn
//! off, and goes on the network only for what the person asks for.
//!
//! **Diagnostics** lists what [`figcraft_engine::settings::diagnostics`] kept, newest first, with
//! Copy, Save…, Clear and how long to keep them. This module also records the app's events:
//! [`noted`] after every command the app runs, and [`record`] for the MCP server and fonts.

use egui::{RichText, Ui};
use figcraft_engine::cmd::diagnostics::SAVE_DIALOG;
use figcraft_engine::settings::diagnostics::{self, Entry, RETENTION_DAYS};
use serde_json::{Value, json};

use crate::i18n::{tr, tr_noop, trn};
use crate::settings_dialog::heading;
use crate::theme::{Tokens, regular};
use crate::{FigApp, widgets};

/// Seconds since 1970, now.
pub fn now() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Keep `entry` in the diagnostics.
pub fn record(app: &mut FigApp, entry: Entry) {
    diagnostics::record(&mut app.session.prefs.settings, now(), entry);
}

/// After command `id` ran from the app: a failure (its id and category, never the message), or a
/// file reverted to its saved copy.
pub fn noted(app: &mut FigApp, id: &str, error: Option<&figcraft_engine::EngineError>) {
    match error {
        Some(error) => record(app, Entry::Error { command: id, error }),
        None if id == "file.revert" => record(app, Entry::Reload),
        None => {}
    }
}

/// `dialog.diagnosticsSave`: ask where, then `diagnostics.save {path}`.
pub fn request(app: &mut FigApp, kind: &str, _data: &Value) -> bool {
    if kind != SAVE_DIALOG {
        return false;
    }
    app.pick_save(tr("Save diagnostics"), "scrim-diagnostics.txt", &["txt"], |app, path| {
        app.run("diagnostics.save", json!({"path": path.to_string_lossy()}));
    });
    true
}

pub fn privacy(_app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("Privacy"));
    for line in [
        tr_noop("Scrim sends no telemetry. It has no usage statistics, crash reports or analytics, so there is nothing here to turn off."),
        tr_noop(
            "It goes on the network only when you ask it to: sharing or joining a file online, publishing, using a team library or browsing plugins on a server you chose, and showing a file to people on your local network.",
        ),
        tr_noop("The MCP server for agents listens on this computer only (127.0.0.1)."),
        tr_noop("Diagnostics stay on this computer. They hold no document content, prompts or tokens, and leave it only if you copy or save them."),
    ] {
        widgets::label(ui, tr(line), t.text);
        ui.add_space(6.0);
    }
}

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("Diagnostics"));
    widgets::label(
        ui,
        tr(
            "What happened recently, to describe a problem: reverts, fonts a file falls back from, the MCP server and its clients, and commands that failed. Kept on this computer only.",
        ),
        t.text_secondary,
    );
    ui.add_space(6.0);
    let days = app.session.prefs.settings.diagnostics_days;
    ui.horizontal(|ui| {
        ui.label(tr("Keep for"));
        let names = [tr_noop("Don't keep"), tr_noop("1 day"), tr_noop("7 days"), tr_noop("30 days")];
        let labels: Vec<&str> = names.iter().map(|n| tr(n)).collect();
        let current = RETENTION_DAYS.iter().position(|d| *d == days).unwrap_or(2);
        if let Some(i) = widgets::segmented_text(ui, &labels, current) {
            app.run("diagnostics.retention", json!({"days": RETENTION_DAYS[i], "now": now()}));
            app.services.save_prefs(&app.session.prefs);
        }
    });
    let events = app.session.prefs.settings.diagnostics.clone();
    ui.horizontal(|ui| {
        let any = !events.is_empty();
        if widgets::button(ui, tr("Copy"), false, any).clicked() {
            app.run("diagnostics.copy", json!({}));
        }
        if !cfg!(target_arch = "wasm32") && widgets::button(ui, tr("Save…"), false, any).clicked() {
            app.run("diagnostics.save", json!({}));
        }
        if widgets::button(ui, tr("Clear"), false, any).clicked() {
            app.run("diagnostics.clear", json!({}));
            app.services.save_prefs(&app.session.prefs);
        }
    });
    ui.add_space(6.0);
    if events.is_empty() {
        widgets::label(ui, tr("Nothing yet."), t.text_secondary);
        return;
    }
    let n = trn("{n} events, newest first", events.len(), &[]);
    widgets::label(ui, &n, t.text_secondary);
    for e in events.iter().rev().take(200) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(diagnostics::timestamp(e.at)).color(t.text_tertiary).font(egui::FontId::monospace(11.0)));
            ui.label(RichText::new(&e.kind).color(t.text_secondary).font(regular(11.0)));
            ui.label(RichText::new(&e.detail).color(t.text).font(regular(11.0)));
        });
    }
}

#[cfg(test)]
#[path = "settings_privacy_tests.rs"]
mod tests;
