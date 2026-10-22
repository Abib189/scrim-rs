//! Settings ▸ Agents (Fix T4, `docs/design/ai-panel.md`): everything about agents outside the AI
//! panel, which is a chat. The connections, the AI tab's until Fix T4: the MCP endpoint's state and
//! its clients, Claude Code's Connect, Claude Desktop's Register and Re-register, and every other
//! client with its status, registration and tool use; then agents at work (following, the reveal,
//! the dots and the grace, Fix Q's hint beside it; from Settings ▸ General); then Describe a
//! change (R5d). Every row runs the same registry commands as before. There are no models or API keys:
//! agents bring their own.

use egui::Ui;
use serde_json::json;

use crate::i18n::{tr, tr_noop, trf};
use crate::settings_dialog::heading;
use crate::theme::{Tokens, regular};
use crate::{FigApp, a11y, widgets};

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("Agents"));
    widgets::label(
        ui,
        tr("Agents work on the canvas beside you through MCP. Scrim has no models and keeps no API keys: each agent brings its own."),
        t.text_secondary,
    );
    // The connections first (the AI tab's until Fix T4), then agents at work, then Describe a change.
    connections(app, ui);
    widgets::separator(ui);
    at_work(app, ui);
    ui.add_space(6.0);
    crate::follow_agent::chat_ui::settings(app, ui);
}

/// Following, the reveal, the dots and the grace (Settings ▸ General's until Fix T4).
fn at_work(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let prefs = app.session.prefs.clone();
    heading(ui, tr("Agents at work"));
    if widgets::checkbox(ui, prefs.settings.follow_agents, tr("Follow agents as they work")) {
        app.run("camera.follow-agents", json!({"on": !prefs.settings.follow_agents}));
    }
    widgets::label(ui, tr("Keeps the agent's latest work in the middle of the canvas. Your own scroll, zoom or edit pauses it."), t.text_secondary);
    if widgets::checkbox(ui, prefs.settings.reveal_agent_work, tr("Reveal agent work")) {
        app.run("pref.revealAgentWork", json!({}));
    }
    // Each speed with its timing (Fix M3), one a row: too long for a segmented control.
    ui.label(tr("Reveal speed"));
    let speeds = [
        ("fast", tr_noop("Fast (0.2 s per layer, about 1 s per step)")),
        ("normal", tr_noop("Normal (0.5 s per layer, about 4 s per step)")),
        ("slow", tr_noop("Slow (0.7 s per layer, about 6 s per step)")),
    ];
    let current = speeds.iter().position(|(id, _)| prefs.settings.reveal_speed == *id).unwrap_or(1);
    for (i, (id, label)) in speeds.iter().enumerate() {
        if widgets::radio(ui, i == current, tr(label)) && i != current {
            app.run("pref.revealSpeed", json!({"speed": id}));
        }
    }
    if widgets::checkbox(ui, prefs.settings.working_indicator, tr("Show working indicator")) {
        app.run("pref.workingIndicator", json!({}));
    }
    widgets::label(
        ui,
        tr("New layers fade in as an agent adds them, and dots above the artboard show it is working. Animations Off shows them still."),
        t.text_secondary,
    );
    // The grace (Fix Q, T1): 15, 30, 60 (the default) or 120 s. The person's choice is kept through
    // migrations; Fix Q2's hint scrolls here.
    let grace_row = ui.label(tr("Keep the working indicator after the last agent request")).rect;
    ui.horizontal(|ui| {
        use figcraft_engine::settings::{AGENT_GRACE_SECS, DEFAULT_AGENT_GRACE_SECS};
        let labels: Vec<String> =
            AGENT_GRACE_SECS
                .iter()
                .map(|s| {
                    if *s == DEFAULT_AGENT_GRACE_SECS {
                        trf("{seconds} s (default)", &[("seconds", s)])
                    } else {
                        trf("{seconds} s", &[("seconds", s)])
                    }
                })
                .collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let current = AGENT_GRACE_SECS.iter().position(|s| *s == prefs.settings.agent_grace_secs).unwrap_or(2);
        if let Some(i) = widgets::segmented_text(ui, &labels, current) {
            app.run("pref.agentGrace", json!({"seconds": AGENT_GRACE_SECS[i]}));
        }
    });
    if std::mem::take(&mut app.ui.agent_view.focus_grace) {
        ui.scroll_to_rect(grace_row, Some(egui::Align::Center));
    }
    // The grace was too short for an agent this run (Fix Q2): said here, beside the setting.
    if app.ui.agent_view.grace_hint.shown {
        widgets::note(ui, tr("The indicator ended before the agent's next step. A longer setting keeps it on."), t.warning);
    }
    widgets::label(ui, tr("An agent that says it has finished, or disconnects, stops showing as working at once."), t.text_secondary);
}

/// Who is connected (any MCP client, by name) and the connect flows: the AI tab's until Fix T4.
pub(crate) fn connections(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    // Under the page's own "Agents" heading (Fix T4): the connections.
    widgets::section_header(ui, tr("Connections"), &[], false);
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
    let st = figcraft_engine::cmd::mcp::status(&app.session, now);
    let state = st["state"].as_str().unwrap_or("stopped");
    let (label, color) = match state {
        "connected" => (tr("Connected"), t.success),
        "not connected" => (tr("Not connected"), t.text_secondary),
        _ => (tr("Server stopped"), t.danger),
    };
    crate::right::row(ui, 28.0, |ui| {
        let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, color);
        // Every client connected now, by the name a person knows it by (Stage J).
        let names: Vec<String> = st["clients"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| c["display"].as_str().map(str::to_string).unwrap_or_else(|| tr("an agent").to_string()))
            .collect();
        let client = if names.is_empty() { String::new() } else { format!(" · {}", names.join(", ")) };
        let resp = ui.label(egui::RichText::new(format!("{label}{client}")).color(t.text).font(regular(12.0)));
        a11y::status(&resp, label, &trf("The MCP endpoint is {state}", &[("state", &label)]));
        if state == "stopped" && widgets::button(ui, tr("Restart"), false, true).clicked() {
            app.run("mcp.restart-server", json!({}));
        }
    });
    // Who is working now (Stage I.1's working state), on its own line. Plain text: the working
    // indicator's AccessKit node is the one live status, so it isn't announced twice.
    if let Some(text) = crate::follow_agent::working_text(app) {
        crate::right::row(ui, 22.0, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&text).color(t.text).font(regular(11.0))).truncate()).on_hover_text(&text);
        });
    }
    crate::mcp_registration::row(app, ui);
    if app.ui.connect.open {
        crate::connect_flow::panel(app, ui);
    } else if state != "connected" {
        widgets::note(
            ui,
            tr("Any MCP client can see and edit this canvas through Scrim: connect it once, and every session of it can use Scrim."),
            t.text_secondary,
        );
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Connect Claude Code"), true, true).clicked() {
                app.run("mcp.connect-claude-code", json!({}));
            }
        });
    }
    crate::desktop_flow::section(app, ui);
    crate::connect_flow::agent_clients_ui::section(app, ui);
}
