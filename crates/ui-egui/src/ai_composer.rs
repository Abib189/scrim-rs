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
use serde_json::{Value, json};

use super::{chips, elapsed, installed, picked, send_blocked};
use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::panel_view::ai_tab::icons::AiIcon;
use crate::theme::{Tokens, regular};

/// The composer's least height (Fix X3); it grows with the text.
const MIN_H: f32 = 88.0;

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
    // What the person must do (Fix Y1): the agent's CLI isn't installed, or a run stopped before it
    // began (not signed in, likely). One line, 8 above the status line, with a link to Settings ▸
    // Agents; the second can be dismissed. Why Send can't go otherwise stays its tooltip.
    let roles = crate::panel_view::ai_tab::panel_theme::roles(&t);
    let (missing, dismissible) = if found.is_empty() {
        let want = app.ui.agent_view.chat.agent.clone().unwrap_or_else(|| figcraft_engine::cmd::chat::DEFAULT_AGENT.to_string());
        let name = headless::profile(&want).map_or_else(|| want.clone(), |p| p.name.clone());
        (Some(trf("{agent} isn't installed.", &[("agent", &name)])), false)
    } else {
        let not_ready = app.ui.agent_view.chat.not_ready.clone();
        let text = current
            .as_ref()
            .filter(|(p, _)| not_ready.as_deref() == Some(p.id.as_str()))
            .map(|(p, _)| trf("{agent} couldn't start: is it signed in?", &[("agent", &p.name)]));
        (text, true)
    };
    if let Some(text) = &missing {
        seen["missingCli"] = json!(text);
        seen["missingLink"] = json!(tr("Agent settings"));
        seen["missingDismissible"] = json!(dismissible);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(RichText::new(text).font(regular(12.0)).color(roles.text_secondary));
            // The link stays whole: it moves to the next row rather than breaking across two.
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let link = ui.link(RichText::new(tr("Agent settings")).font(regular(12.0)).color(roles.link));
            crate::a11y::link(&link, tr("Agent settings"));
            if link.clicked() {
                app.run("settings.open", json!({"section": "agents"}));
            }
            if dismissible {
                let x = ui.small_button(RichText::new("×").font(regular(12.0)).color(roles.text_secondary)).on_hover_text(tr("Dismiss"));
                crate::a11y::button(&x, tr("Dismiss"));
                if x.clicked() {
                    app.run("chat.dismiss-hint", json!({"hint": "agent"}));
                }
            }
        });
        ui.add_space(8.0);
    }
    // The Undone toast, above the status line (Fix AB3).
    seen["toast"] = crate::panel_view::ai_tab::toast::draw(app, ui, &roles).unwrap_or(Value::Null);
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
    let blocked = send_blocked(app, now);
    let (mut send, mut stop) = (false, false);
    let has_selection = app.session.ds().is_ok_and(|d| !d.selection.is_empty());
    let takes_image = current.as_ref().is_some_and(|(p, _)| !p.image_args.is_empty());
    seen["selectionButton"] = json!(tr("Add the selection to the prompt"));
    seen["imageButton"] = json!(takes_image);
    // The box (Fix X3): radius 16, 16 and 14 inside, at least 88 high and growing with the text; the
    // field above a 32 px row of the agent, the selection, an image and Send.
    let hovered = ui.rect_contains_pointer(ui.available_rect_before_wrap().with_max_y(ui.cursor().min.y + 120.0));
    let edge = crate::theme::field_stroke(&t, hovered, focused);
    let r = egui::Frame::NONE.fill(roles.composer_fill).corner_radius(16).inner_margin(egui::Margin::symmetric(16, 14)).stroke(edge).show(ui, |ui| {
        ui.set_min_height(MIN_H - 28.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        let field = ui.add_enabled(
            !read_only,
            egui::TextEdit::multiline(&mut app.ui.agent_view.chat.draft)
                .id(field_id)
                .frame(egui::Frame::NONE)
                .font(regular(14.0))
                .text_color(roles.text)
                .hint_text(RichText::new(tr("Describe a change…")).font(regular(14.0)).color(roles.placeholder))
                .desired_rows(1)
                .desired_width(f32::INFINITY),
        );
        // The image attached, with its remove button.
        if let Some(path) = app.ui.agent_view.chat.image.clone() {
            let name = std::path::Path::new(&path).file_name().map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
            seen["image"] = json!(name);
            ui.horizontal(|ui| {
                ui.label(RichText::new(trf("Image: {name}", &[("name", &name)])).font(regular(12.0)).color(roles.text_secondary));
                let x = ui.small_button("×").on_hover_text(tr("Remove the image"));
                crate::a11y::button(&x, tr("Remove the image"));
                if x.clicked() {
                    app.run("chat.attach-image", json!({"path": ""}));
                }
            });
        }
        ui.horizontal(|ui| {
            ui.set_height(chips::CHIP_H);
            ui.spacing_mut().item_spacing.x = 8.0;
            // What fits before Send: the chips' labels shorten with "…" before anything clips.
            let room = (ui.available_width() - chips::SEND - 8.0).max(0.0);
            let agent_label = current.as_ref().map_or_else(String::new, |(p, _)| p.name.clone());
            // Selection and the image are round icon chips at every width (Fix Z6); the agent's
            // name has the rest, shortened with "…" when it must be. Below 280 px the agent chip
            // drops its chevron and pads 8; if its name still can't show six characters and "…",
            // the image goes into the agent menu as "Attach image…" (Fix AA3).
            let fit = chips::ChipFit::for_panel(app.ui.right_width, true);
            let want_agent = if found.is_empty() { 0.0 } else { fit.width(ui, &agent_label) };
            let agent_w = |others: f32| (room - others - 8.0).clamp(44.0, want_agent.max(44.0));
            let with_image = agent_w(chips::CHIP_H * 2.0 + 8.0);
            let image_in_menu = takes_image && !found.is_empty() && fit.room(with_image) < chips::readable_width(ui, &agent_label);
            let image_chip = takes_image && !image_in_menu;
            let agent_w = if image_chip { with_image } else { agent_w(chips::CHIP_H) };
            seen["imageInMenu"] = json!(image_in_menu);
            // The agent: the installed agents with a headless profile, then Agent settings… (the
            // gear's place before Fix X3).
            if !found.is_empty() {
                let path = current.as_ref().map_or_else(String::new, |(_, path)| path.clone());
                let (resp, shown) = chips::chip(ui, &roles, &agent_label, roles.text, fit, agent_w, &trf("Agent: {agent}", &[("agent", &agent_label)]));
                seen["agentChip"] = json!({"shown": shown, "chevron": fit.chevron, "pad": fit.pad, "rect": [resp.rect.min.x, resp.rect.min.y, resp.rect.max.x, resp.rect.max.y]});
                let resp = resp.on_hover_text(&path);
                crate::a11y::menu_button(
                    &resp,
                    &trf("Agent: {agent}", &[("agent", &agent_label)]),
                    egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp)),
                );
                crate::right::menu_popup(&resp).width(200.0).show(|ui| {
                    for (id, path) in &found {
                        let Some(p) = headless::profile(id) else { continue };
                        if ui.button(&p.name).on_hover_text(path).clicked() {
                            app.run("chat.agent", json!({"agent": id}));
                        }
                    }
                    if image_in_menu {
                        ui.separator();
                        if ui.button(tr("Attach image…")).clicked() && !read_only {
                            app.run("chat.attach-image", json!({}));
                        }
                    }
                    ui.separator();
                    if ui.button(tr("Agent settings…")).clicked() {
                        app.run("settings.open", json!({"section": "agents"}));
                    }
                });
                seen["agent"] = json!(agent_label);
                seen["agentMenu"] = json!(tr("Agent settings…"));
            }
            let sel = chips::icon_chip(ui, &roles, AiIcon::Selection, tr("Add the selection to the prompt"));
            crate::a11y::button(&sel, tr("Add the selection to the prompt"));
            let chip_json = |r: egui::Rect, icon: &str, name: &str| json!({"icon": icon, "name": name, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]});
            seen["selectionChip"] = chip_json(sel.rect, "selection", tr("Add the selection to the prompt"));
            if sel.clicked() && has_selection && !read_only {
                app.run("chat.add-selection", json!({}));
            }
            if image_chip {
                let img = chips::icon_chip(ui, &roles, AiIcon::Image, tr("Attach an image"));
                crate::a11y::button(&img, tr("Attach an image"));
                seen["imageChip"] = chip_json(img.rect, "image", tr("Attach an image"));
                if img.clicked() && !read_only {
                    app.run("chat.attach-image", json!({}));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if running {
                    stop = chips::send(ui, &roles, true, tr("Stop")).clicked();
                    seen["send"] = json!(tr("Stop"));
                } else {
                    let tip = format!("{}  {}", tr("Send"), if cfg!(target_os = "macos") { "⌘↩" } else { "Ctrl+Enter" });
                    let resp = chips::send(ui, &roles, false, &tip);
                    // Why it can't go, in its tooltip (Fix X3: no line under the composer).
                    let resp = match &blocked {
                        Some(why) => resp.on_hover_text(why),
                        None => resp,
                    };
                    send = resp.clicked() && blocked.is_none();
                    seen["send"] = json!(tr("Send"));
                }
            });
        });
        field
    });
    let field = r.inner;
    crate::a11y::text(&field, tr("Describe a change"), &app.ui.agent_view.chat.draft.clone());
    // What the composer drew in (Fix X5).
    let hexes = |c: egui::Color32| json!(crate::theme::accent::rgb_text(c));
    // The composer's text roles' fonts (Fix AA1).
    let font = |f: egui::FontId| crate::panel_view::ai_tab::panel::font_json(&f);
    seen["fonts"] =
        json!({"field": font(regular(14.0)), "placeholder": font(regular(14.0)), "chip": font(regular(13.0)), "status": font(regular(12.0))});
    seen["theme"] = json!({
        "sendFill": hexes(roles.send_fill),
        "sendGlyph": hexes(roles.send_glyph),
        "composerFill": hexes(roles.composer_fill),
        "composerEdge": hexes(edge.color),
        "chips": hexes(roles.chip_fill),
        "chipText": hexes(roles.text),
        "chipSecondary": hexes(roles.text_secondary),
        "statusText": hexes(roles.text_3),
        "chipIcon": hexes(roles.chip_icon),
        "chipHover": hexes(roles.chip_hover),
        "placeholder": hexes(roles.placeholder),
        "statusDot": seen["statusDot"].clone(),
    });
    seen["composerRect"] = json!([r.response.rect.min.x, r.response.rect.min.y, r.response.rect.max.x, r.response.rect.max.y]);
    // Why Send can't go, or how long the run has taken.
    let status = match &app.ui.agent_view.chat.running {
        Some(r) if r.stopping => tr("Stopping…").to_string(),
        Some(r) => trf("{agent} is working · {time}", &[("agent", &r.agent.name), ("time", &elapsed(now - r.started))]),
        None => blocked.clone().filter(|_| missing.is_none()).unwrap_or_default(),
    };
    // Not drawn under the composer since Fix X3: the run shows in the working banner (Fix X4), and
    // why Send can't go is its tooltip. Kept for `ui.inspect`.
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
    ui.painter().text(egui::pos2(left + 32.0, row.center().y), egui::Align2::LEFT_CENTER, text, regular(12.0), roles.text_3);
    crate::a11y::status(&resp, text, "");
    seen["following"] = json!(text);
    seen["statusDot"] = json!(crate::theme::accent::rgb_text(dot));
    ui.add_space(10.0);
}
