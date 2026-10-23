//! Settings (`docs/design/settings-and-connect.md`, C3): a rail of sections on the left, the chosen
//! section's pane on the right (it scrolls), and Done. Escape closes it.
//!
//! **Opening it:**
//! - the home sidebar's Settings row;
//! - Cmd+, (`app.preferences`, which now opens Settings at General);
//! - the Actions palette, `settings.open {section?}`.
//!
//! Settings replaces the Preferences window: its rows live in General. Every control runs a
//! registry command, so agents change the same settings over MCP. An agent's `settings.open` opens
//! no window (programmatic calls never open dialogs); it gets the settings back instead.
//!
//! The rail lists only the sections that exist: no placeholder pages.

use egui::{Align2, Context, CornerRadius, Rect, Sense, Ui, pos2, vec2};
use serde_json::Value;

use crate::i18n::{tr, tr_noop};
use crate::theme::{Tokens, regular};
use crate::{FigApp, a11y, widgets};

/// The sections, in the rail's order: (id, name).
pub const SECTIONS: &[(&str, &str)] = &[
    ("general", tr_noop("General")),
    ("toolbar", tr_noop("Toolbar")),
    ("ai", tr_noop("Agents")),
    ("mcp", tr_noop("MCP")),
    ("tools", tr_noop("Tool access")),
    ("privacy", tr_noop("Privacy")),
    ("diagnostics", tr_noop("Diagnostics")),
];

/// Open Settings at `section` (`dialog.settings`, `dialog.preferences`).
pub fn request(app: &mut FigApp, kind: &str, data: &Value) -> bool {
    match kind {
        "dialog.settings" => {
            // Settings ▸ Agents is `ai` (it was "AI & agents"); `agents` names it too (Fix T4).
            let wanted = match data.get("section").and_then(Value::as_str).unwrap_or("general") {
                "agents" => "ai",
                other => other,
            };
            app.ui.settings_section = SECTIONS.iter().find(|(id, _)| *id == wanted).map_or("general", |(id, _)| id);
            app.ui.preferences = true;
        }
        _ => return false,
    }
    true
}

/// The window, while it's open.
pub fn show(app: &mut FigApp, ctx: &Context) {
    if !app.ui.preferences {
        return;
    }
    let t = Tokens::get(ctx);
    let screen = ctx.content_rect();
    let size = vec2((screen.width() - 80.0).clamp(560.0, 820.0), (screen.height() - 80.0).clamp(420.0, 600.0));
    let mut open = true;
    let mut done = false;
    let shown =
        crate::dialogs::window(tr("Settings"), ctx).open(&mut open).fixed_size(size).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ctx, |ui| {
            let body = ui.available_rect_before_wrap();
            let rail_w = 180.0;
            let rail = Rect::from_min_size(body.min, vec2(rail_w, body.height() - 44.0));
            let pane = Rect::from_min_max(pos2(rail.max.x + 16.0, body.min.y), pos2(body.max.x, body.max.y - 44.0));
            ui.painter().line_segment([rail.right_top() + vec2(8.0, 0.0), rail.right_bottom() + vec2(8.0, 0.0)], egui::Stroke::new(1.0, t.border));
            rail_ui(app, ui, &t, rail);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(pane));
            egui::ScrollArea::vertical().id_salt(("settings_pane", app.ui.settings_section)).auto_shrink([false, false]).show(&mut child, |ui| {
                ui.set_width(pane.width() - 12.0);
                section(app, ui);
                ui.add_space(24.0);
            });
            let footer = Rect::from_min_max(pos2(pane.min.x, body.max.y - 36.0), body.max);
            let mut f = ui.new_child(egui::UiBuilder::new().max_rect(footer).layout(egui::Layout::right_to_left(egui::Align::Center)));
            done = widgets::button(&mut f, tr("Done"), true, true).clicked();
            ui.allocate_rect(body, Sense::hover());
        });
    // A modal dialog: Tab stays inside it, and what's behind can't be clicked (C1.6). Its own
    // confirmations open above it and still work.
    if let Some(r) = shown {
        ctx.memory_mut(|m| m.set_modal_layer(r.response.layer_id));
    }
    if !open || done || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ui.preferences = false;
    }
}

/// The rail: one row per section, the current one marked; each a tab for screen readers.
fn rail_ui(app: &mut FigApp, ui: &mut Ui, t: &Tokens, rail: Rect) {
    let mut y = rail.min.y;
    for (id, name) in SECTIONS {
        let r = Rect::from_min_size(pos2(rail.min.x, y), vec2(rail.width(), 30.0));
        let resp = ui.interact(r, ui.id().with(("settings_rail", *id)), Sense::click());
        let current = app.ui.settings_section == *id;
        a11y::tab(&resp, tr(name), current);
        if current {
            ui.painter().rect_filled(r, CornerRadius::same(5), t.selected_row);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(5), t.hover);
        }
        if resp.has_focus() {
            ui.painter().rect_stroke(r, CornerRadius::same(5), egui::Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
        }
        ui.painter().text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, tr(name), regular(12.0), t.text);
        if resp.clicked() {
            app.ui.settings_section = id;
        }
        y += 32.0;
    }
}

fn section(app: &mut FigApp, ui: &mut Ui) {
    // A setting changed here is noted as made by the person's Settings (Fix W4).
    app.session.agents.from_settings = true;
    // (Each section is its own module; C3 adds them one commit at a time.)
    match app.ui.settings_section {
        "toolbar" => crate::settings_toolbar::show(app, ui),
        "ai" => crate::settings_mcp::agents::show(app, ui),
        "mcp" => crate::settings_mcp::mcp(app, ui),
        "tools" => crate::settings_tools::show(app, ui),
        "privacy" => crate::settings_privacy::privacy(app, ui),
        "diagnostics" => crate::settings_privacy::show(app, ui),
        _ => crate::settings_general::show(app, ui),
    }
    app.session.agents.from_settings = false;
}

/// A section's heading.
pub fn heading(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(10.0);
    ui.label(egui::RichText::new(text).strong().color(t.text).font(regular(13.0)));
    ui.add_space(4.0);
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
