//! The AI panel's composer, pinned to its bottom (Fix T4, `docs/design/ai-panel.md`; the run itself
//! is R5's, `docs/design/chat-composer.md`).
//! - **Above the field, at most:** the grace hint's one dismissible line while it is pending, with
//!   its link to the setting (`chat.grace-setting`, `chat.dismiss-hint`); and, when the agent's CLI
//!   isn't on this computer or couldn't start (not signed in), one line with a link to Settings ▸
//!   Agents.
//! - **The field:** "Describe a change…", several lines; ⌘↩ (Ctrl+Enter) sends; Esc and Stop stop a
//!   run.
//! - **The row below:** the selection (`chat.add-selection`: its layers' names and ids into the
//!   prompt), an image (`chat.attach-image`, for agents whose CLI takes one; hidden otherwise), the
//!   agent (`chat.agent`), the settings gear (Settings ▸ Agents), and Send at the right.

use egui::{Id, RichText, Ui};
use figcraft_engine::cmd::agent_clients::headless;
use serde_json::json;

use super::{elapsed, installed, picked, send_blocked};
use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::icons::Icon;
use crate::panel_view::ai_tab::icons::{self as ai_icons, AiIcon};
use crate::panel_view::ai_tab::panel::{BUTTON, icon_button};
use crate::theme::{Tokens, regular};
use crate::widgets;

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let now = ui.ctx().input(|i| i.time);
    let mut seen = json!({"placeholder": tr("Describe a change…")});
    // The grace hint: one line while it is pending (Fix Q2, T4).
    if app.ui.agent_view.grace_hint.showing {
        seen["hintLine"] = json!(true);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("The indicator ended before the agent's next step.")).font(regular(10.5)).color(t.text_secondary));
            let link = ui.link(RichText::new(tr("Open the setting")).font(regular(10.5)));
            crate::a11y::link(&link, tr("Open the setting"));
            if link.clicked() {
                app.run("chat.grace-setting", json!({}));
            }
            let x = ui.small_button("×").on_hover_text(tr("Dismiss"));
            crate::a11y::button(&x, tr("Dismiss"));
            if x.clicked() {
                app.run("chat.dismiss-hint", json!({}));
            }
        });
        ui.add_space(4.0);
    }
    let found = installed(app, now);
    let current = picked(app, now);
    // The agent's CLI isn't here, or couldn't start: one line to Settings ▸ Agents.
    let missing = if found.is_empty() {
        Some(tr("No agent found on this computer: install Claude Code or Codex and sign in.").to_string())
    } else {
        let not_ready = app.ui.agent_view.chat.not_ready.clone();
        current
            .as_ref()
            .filter(|(p, _)| not_ready.as_deref() == Some(p.id.as_str()))
            .map(|(p, _)| trf("{agent} couldn't start: is it signed in?", &[("agent", &p.name)]))
    };
    if let Some(text) = &missing {
        seen["missingCli"] = json!(text);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(text).font(regular(10.5)).color(t.text_secondary));
            let link = ui.link(RichText::new(tr("Settings ▸ Agents")).font(regular(10.5)));
            crate::a11y::link(&link, tr("Settings ▸ Agents"));
            if link.clicked() {
                app.run("settings.open", json!({"section": "ai"}));
            }
        });
        ui.add_space(4.0);
    }
    status_line(app, ui, &t, &mut seen);
    let read_only = super::store::read_only(app);
    seen["readOnly"] = json!(read_only);
    // The field.
    let field_id = Id::new("describe_change_field");
    let running = app.ui.agent_view.chat.running.is_some();
    let focused = ui.memory(|m| m.has_focus(field_id));
    let (cmd_enter, esc) = if focused {
        ui.input_mut(|i| (i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter), i.key_pressed(egui::Key::Escape)))
    } else {
        (false, ui.input(|i| i.key_pressed(egui::Key::Escape)))
    };
    let hovered = ui.rect_contains_pointer(ui.available_rect_before_wrap().with_max_y(ui.cursor().min.y + 60.0));
    let r = egui::Frame::NONE
        .fill(t.field)
        .corner_radius(6)
        .inner_margin(egui::Margin::same(8))
        .stroke(crate::theme::field_stroke(&t, hovered, focused))
        .show(ui, |ui| {
            ui.add_enabled(
                !read_only,
                egui::TextEdit::multiline(&mut app.ui.agent_view.chat.draft)
                    .id(field_id)
                    .frame(egui::Frame::NONE)
                    .font(regular(12.0))
                    .hint_text(tr("Describe a change…"))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            )
        });
    let field = r.inner;
    crate::a11y::text(&field, tr("Describe a change"), &app.ui.agent_view.chat.draft.clone());
    // The image attached, with its remove button.
    if let Some(path) = app.ui.agent_view.chat.image.clone() {
        let name = std::path::Path::new(&path).file_name().map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
        seen["image"] = json!(name);
        ui.horizontal(|ui| {
            ui.label(RichText::new(trf("Image: {name}", &[("name", &name)])).font(regular(10.5)).color(t.text_secondary));
            let x = ui.small_button("×").on_hover_text(tr("Remove the image"));
            crate::a11y::button(&x, tr("Remove the image"));
            if x.clicked() {
                app.run("chat.attach-image", json!({"path": ""}));
            }
        });
    }
    ui.add_space(6.0);
    let blocked = send_blocked(app, now);
    let (mut send, mut stop) = (false, false);
    let has_selection = app.session.ds().is_ok_and(|d| !d.selection.is_empty());
    let takes_image = current.as_ref().is_some_and(|(p, _)| !p.image_args.is_empty());
    seen["selectionButton"] = json!(tr("Add the selection to the prompt"));
    seen["imageButton"] = json!(takes_image);
    seen["settingsButton"] = json!(tr("Agent settings"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let sel = icon_button(ui, "selection", tr("Add the selection to the prompt"), None, |p, r, c| crate::icons::paint(p, r, Icon::Move, c));
        if sel.clicked() && has_selection && !read_only {
            app.run("chat.add-selection", json!({}));
        }
        if takes_image {
            let img = icon_button(ui, "image", tr("Attach an image"), None, |p, r, c| crate::icons::paint(p, r, Icon::Image, c));
            if img.clicked() && !read_only {
                app.run("chat.attach-image", json!({}));
            }
        }
        // The agent: the installed agents with a headless profile.
        if !found.is_empty() {
            let label = current.as_ref().map_or_else(String::new, |(p, _)| p.name.clone());
            let path = current.as_ref().map_or_else(String::new, |(_, path)| path.clone());
            let right = 2.0 * BUTTON + 8.0;
            let w = (ui.available_width() - right).clamp(60.0, 150.0);
            let resp = widgets::dropdown(ui, &label, w, None).on_hover_text(&path);
            crate::a11y::menu_button(
                &resp,
                &trf("Agent: {agent}", &[("agent", &label)]),
                egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp)),
            );
            crate::right::menu_popup(&resp).width(200.0).show(|ui| {
                for (id, path) in &found {
                    let Some(p) = headless::profile(id) else { continue };
                    if ui.button(&p.name).on_hover_text(path).clicked() {
                        app.run("chat.agent", json!({"agent": id}));
                    }
                }
            });
            seen["agent"] = json!(label);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if running {
                let s = icon_button(ui, "stop", tr("Stop"), None, |p, r, c| crate::icons::paint(p, r, Icon::Close, c));
                stop = s.clicked();
                seen["send"] = json!(tr("Stop"));
            } else {
                let tip = format!("{}  {}", tr("Send"), if cfg!(target_os = "macos") { "⌘↩" } else { "Ctrl+Enter" });
                let s = icon_button(ui, "send", &tip, None, |p, r, c| ai_icons::paint(p, r, AiIcon::Send, c));
                send = s.clicked() && blocked.is_none();
                seen["send"] = json!(tr("Send"));
            }
            if icon_button(ui, "settings", tr("Agent settings"), None, |p, r, c| ai_icons::paint(p, r, AiIcon::Gear, c)).clicked() {
                app.run("settings.open", json!({"section": "ai"}));
            }
        });
    });
    // Why Send can't go, or how long the run has taken.
    let status = match &app.ui.agent_view.chat.running {
        Some(r) if r.stopping => tr("Stopping…").to_string(),
        Some(r) => trf("{agent} is working · {time}", &[("agent", &r.agent.name), ("time", &elapsed(now - r.started))]),
        None => blocked.clone().filter(|_| missing.is_none()).unwrap_or_default(),
    };
    if !status.is_empty() {
        ui.label(RichText::new(&status).font(regular(10.5)).color(t.text_secondary));
    }
    seen["status"] = json!(status);
    if cmd_enter && blocked.is_none() {
        send = true;
    }
    if esc && running && (focused || field.has_focus() || !ui.ctx().memory(|m| m.focused().is_some())) {
        stop = true;
    }
    if send {
        let agent = current.as_ref().map(|(p, _)| p.id.clone()).unwrap_or_default();
        let mut p = json!({"prompt": app.ui.agent_view.chat.draft.clone(), "agent": agent});
        if let Some(img) = app.ui.agent_view.chat.image.clone().filter(|_| takes_image) {
            p["image"] = json!(img);
        }
        app.run("chat.send", p);
    }
    if stop {
        app.run("chat.cancel", json!({}));
    }
    app.ui.agent_view.chat.seen.composer = seen;
}

/// The status line (Fix X2), always shown 10 above the composer: a 6 px dot 20 from the panel's left
/// edge and Inter 12 at 32. "Following is on" with the success dot (paused included: the header's
/// button shows a pause), or "Following is off" with a secondary dot.
fn status_line(app: &FigApp, ui: &mut Ui, t: &Tokens, seen: &mut serde_json::Value) {
    let roles = crate::panel_view::ai_tab::panel_theme::roles(t);
    let on = app.session.prefs.settings.follow_agents;
    let (text, dot) = if on { (tr("Following is on"), roles.success) } else { (tr("Following is off"), roles.status_off_dot) };
    let (row, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 14.0), egui::Sense::hover());
    let left = row.min.x - crate::widgets::PAD;
    ui.painter().circle_filled(egui::pos2(left + 23.0, row.center().y), 3.0, dot);
    ui.painter().text(egui::pos2(left + 32.0, row.center().y), egui::Align2::LEFT_CENTER, text, regular(12.0), roles.text_secondary);
    crate::a11y::status(&resp, text, "");
    seen["following"] = json!(text);
    ui.add_space(10.0);
}
