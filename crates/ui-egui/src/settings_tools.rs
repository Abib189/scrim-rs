//! Settings ▸ Tool access (C3c): which MCP tools agents get, grouped into the read-only ones and
//! the ones that change things, with a search, per-tool and per-group switches, "N of M enabled"
//! and Restore defaults. A tool turned off is left out of `tools/list` and refused if called
//! (`figcraft_mcp::Server`). Every switch runs `mcp.set-tool`, `mcp.set-tool-group` or
//! `mcp.restore-tools`.

use egui::{Id, RichText, Ui};
use figcraft_engine::cmd::mcp_tools;
use serde_json::{Value, json};

use crate::i18n::{tr, tr_noop, trf};
use crate::settings_dialog::heading;
use crate::theme::{Tokens, regular};
use crate::{FigApp, a11y, widgets};

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let st = mcp_tools::state(&app.session);
    heading(ui, tr("Tool access"));
    let (on, total) = (st["enabled"].as_u64().unwrap_or(0), st["total"].as_u64().unwrap_or(0));
    ui.horizontal(|ui| {
        let n = trf("{on} of {total} enabled", &[("on", &on.to_string()), ("total", &total.to_string())]);
        ui.label(RichText::new(&n).color(t.text).font(regular(12.0)));
        if widgets::button(ui, tr("Restore defaults"), false, on < total).clicked() {
            app.run("mcp.restore-tools", json!({}));
        }
    });
    widgets::label(
        ui,
        tr(
            "A tool turned off is hidden from agents and refused if they call it. run_command and batch reach every command: turn them off to take the commands away.",
        ),
        t.text_secondary,
    );
    ui.add_space(6.0);
    let search_id = Id::new("settings_tools_search");
    let mut query: String = ui.data(|d| d.get_temp(search_id)).unwrap_or_default();
    let resp = ui.add(egui::TextEdit::singleline(&mut query).hint_text(tr("Search tools")).desired_width(240.0));
    a11y::text(&resp, tr("Search tools"), &query);
    ui.data_mut(|d| d.insert_temp(search_id, query.clone()));
    let q = query.trim().to_lowercase();

    let tools = st["tools"].as_array().cloned().unwrap_or_default();
    let mut run: Option<(&str, Value)> = None;
    let groups = [
        ("read", tr_noop("Read-only tools"), tr_noop("All read-only tools ({on} of {total} on)"), true),
        ("write", tr_noop("Tools that make changes"), tr_noop("All tools that make changes ({on} of {total} on)"), false),
    ];
    for (key, name, all_label, read) in groups {
        let group = &st["groups"][key];
        let (g_on, g_total) = (group["enabled"].as_u64().unwrap_or(0), group["total"].as_u64().unwrap_or(0));
        heading(ui, tr(name));
        let all = g_on == g_total;
        let label = trf(all_label, &[("on", &g_on.to_string()), ("total", &g_total.to_string())]);
        if widgets::checkbox(ui, all, &label) {
            run = Some(("mcp.set-tool-group", json!({"group": key, "on": !all})));
        }
        ui.indent(("tools", key), |ui| {
            for tool in tools.iter().filter(|x| x["readOnly"].as_bool() == Some(read)) {
                let (id, title) = (tool["name"].as_str().unwrap_or(""), tool["title"].as_str().unwrap_or(""));
                if !q.is_empty() && !id.contains(&q) && !title.to_lowercase().contains(&q) {
                    continue;
                }
                let on = tool["on"].as_bool().unwrap_or(true);
                ui.horizontal(|ui| {
                    if widgets::checkbox(ui, on, id) {
                        run = Some(("mcp.set-tool", json!({"tool": id, "on": !on})));
                    }
                    ui.label(RichText::new(title).color(t.text_secondary).font(regular(11.0)));
                });
            }
        });
    }
    if let Some((cmd, p)) = run {
        app.run(cmd, p);
    }
}
