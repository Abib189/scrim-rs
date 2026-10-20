//! Settings ▸ AI & agents and Settings ▸ MCP (C3c).
//!
//! **AI & agents:** Claude Code's status and the connect flow, the same ones the AI tab shows
//! (`ai_tab::connection`, `connect_flow`). There are no models or API keys: agents bring their
//! own.
//!
//! **MCP:** the endpoint's status, address (Copy), version and Restart; Require authentication;
//! the folder file tools may use (Use default, Choose folder…); and Connect Claude Code.
//!
//! Every control runs an `mcp.*` command.

use egui::{RichText, Ui};
use serde_json::json;

use crate::i18n::{tr, trf};
use crate::settings_dialog::heading;
use crate::theme::{Tokens, regular};
use crate::{FigApp, a11y, widgets};

pub fn ai(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("AI & agents"));
    widgets::label(
        ui,
        tr("Agents work on the canvas beside you through MCP. Scrim has no models and keeps no API keys: each agent brings its own."),
        t.text_secondary,
    );
    ui.add_space(6.0);
    crate::panel_view::ai_tab::connection(app, ui);
}

pub fn mcp(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
    let st = figcraft_engine::cmd::mcp::status(&app.session, now);
    let state = st["state"].as_str().unwrap_or("stopped");

    heading(ui, tr("Server"));
    ui.horizontal(|ui| {
        let (label, color) = match state {
            "connected" => (tr("Running · a client is connected"), t.success),
            "not connected" => (tr("Running"), t.success),
            _ => (tr("Stopped"), t.danger),
        };
        let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, color);
        let resp = ui.label(RichText::new(label).color(t.text).font(regular(12.0)));
        a11y::status(&resp, tr("MCP server"), label);
        if widgets::button(ui, tr("Restart"), false, true).on_hover_text(tr("Stop and start the server, on the same port")).clicked() {
            app.run("mcp.restart-server", json!({}));
        }
    });
    if let Some(e) = st["error"].as_str() {
        widgets::label(ui, &trf("It couldn't start: {error}", &[("error", &e)]), t.danger);
    }
    ui.horizontal(|ui| {
        ui.label(tr("Address"));
        let url = st["url"].as_str().unwrap_or("—");
        ui.label(RichText::new(url).color(t.text).font(egui::FontId::monospace(12.0)));
        if widgets::button(ui, tr("Copy"), false, st["url"].is_string()).clicked() {
            app.run("mcp.copy-address", json!({}));
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("Version"));
        ui.label(RichText::new(st["version"].as_str().unwrap_or("")).color(t.text_secondary).font(regular(12.0)));
    });
    widgets::label(ui, tr("Only programs on this computer can reach it: it listens on 127.0.0.1 and refuses web pages."), t.text_secondary);

    heading(ui, tr("Authentication"));
    let auth = st["requireAuth"].as_bool().unwrap_or(false);
    if widgets::checkbox(ui, auth, tr("Require authentication")) {
        app.run("mcp.toggle-auth", json!({"on": !auth}));
    }
    widgets::label(
        ui,
        if auth {
            tr("Clients send a token Scrim made for this computer. Connect Claude Code again so it has the token.")
        } else {
            tr("Off: any program on this computer can use the server.")
        },
        t.text_secondary,
    );

    heading(ui, tr("Files agents may use"));
    let dir = st["rootDir"].as_str().map(str::to_string);
    let default = st["defaultRootDir"].as_str().unwrap_or_default().to_string();
    let shown = dir.clone().unwrap_or_else(|| trf("Documents ▸ Scrim ({path}), made the first time an agent uses a file", &[("path", &default)]));
    widgets::label(ui, &shown, t.text);
    widgets::label(ui, tr("open_file, save_file and export over this server only reach files inside this folder."), t.text_secondary);
    ui.horizontal(|ui| {
        if widgets::button(ui, tr("Use default"), false, dir.is_some()).clicked() {
            app.run("mcp.set-root-dir", json!({}));
        }
        if !cfg!(target_arch = "wasm32") && widgets::button(ui, tr("Choose folder…"), false, true).clicked() {
            app.pick_folder(tr("Files agents may use"), |app, path| {
                app.run("mcp.set-root-dir", json!({"path": path.to_string_lossy()}));
            });
        }
    });

    heading(ui, tr("Connect"));
    crate::mcp_registration::row(app, ui);
    if app.ui.connect.open {
        crate::connect_flow::panel(app, ui);
    } else if widgets::button(ui, tr("Connect Claude Code"), true, state != "stopped").clicked() {
        app.run("mcp.connect-claude-code", json!({}));
    }
}

#[cfg(test)]
#[path = "settings_mcp_tests.rs"]
mod tests;
