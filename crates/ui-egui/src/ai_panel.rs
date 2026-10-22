//! The AI panel (Fix T4, `docs/design/ai-panel.md`): a chat, top to bottom.
//! - **The header:** the conversation's title with a chevron (a new chat reads "New chat"); the
//!   Follow agents button (crosshair: `camera.follow-agents`, on, off, and paused while an agent
//!   works); and New chat (`chat.new`). No share button or avatar: not Scrim features.
//! - **The chevron's popover:** "Search conversations…", "This document | All documents"
//!   (`chat.scope`), and the conversations, the one shown checked (`chat.open`). Search matches
//!   titles and messages, here only. Another document's conversation opens read-only.
//! - **The body:** the transcript, or, empty, a chat bubble and "Describe what you want to create or
//!   change." in the middle.
//! - **The composer** is pinned below ([`crate::follow_agent::chat_ui::composer`]).
//!
//! What was drawn is kept in [`Seen`] for `ui.inspect` › `agent.aiPanel`.

use egui::{Align2, CornerRadius, Id, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use super::icons::{self, AiIcon};
use crate::follow_agent::chat_ui::{Line, store};
use crate::i18n::{tr, trf};
use crate::icons::Icon;
use crate::theme::{Tokens, regular, semibold};
use crate::widgets::{self, PAD};
use crate::{FigApp, a11y};

/// The header's height.
pub const HEADER_H: f32 = 40.0;
/// An icon button's size.
pub const BUTTON: f32 = 26.0;

/// What the panel drew last frame (for `ui.inspect` and tests).
#[derive(Clone, Debug, Default)]
pub struct Seen {
    pub title: String,
    /// following | paused | off
    pub follow: String,
    pub popover: Option<Value>,
    pub empty_state: bool,
    /// Where the Follow agents button was drawn.
    pub follow_rect: Option<Rect>,
    pub read_only_note: bool,
    /// New chat could be used, and the colour its icon was drawn in (Fix U3).
    pub new_chat_enabled: bool,
    pub new_chat_color: egui::Color32,
    pub lines: usize,
    pub composer: Value,
    /// The widest thing drawn and the panel's width, for the layout checks.
    pub widest: f32,
    pub width: f32,
}

/// An icon button with its tooltip and accessible name; `on` marks a toggle's state.
pub(crate) fn icon_button(
    ui: &mut Ui,
    id: &str,
    label: &str,
    on: Option<bool>,
    draw: impl FnOnce(&egui::Painter, Rect, egui::Color32),
) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(BUTTON, BUTTON), Sense::click());
    let resp = resp.on_hover_text(label);
    match on {
        Some(v) => a11y::toggle(&resp, label, v),
        None => a11y::button(&resp, label),
    }
    let _ = id;
    if on == Some(true) {
        ui.painter().rect_filled(r, CornerRadius::same(5), t.selected_row);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(5), t.hover);
    }
    if resp.has_focus() {
        ui.painter().rect_stroke(r, CornerRadius::same(5), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    let color = if on == Some(true) { t.accent } else { t.icon };
    draw(ui.painter(), Rect::from_center_size(r.center(), vec2(16.0, 16.0)), color);
    resp
}

/// [`icon_button`] that can't be used now: dimmed with the disabled token, its tooltip saying why;
/// neither clickable nor focusable (Tab skips it), and disabled for screen readers.
pub(crate) fn icon_button_off(ui: &mut Ui, label: &str, why: &str, draw: impl FnOnce(&egui::Painter, Rect, egui::Color32)) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(BUTTON, BUTTON), Sense::hover());
    let resp = resp.on_hover_text(why);
    a11y::button(&resp, label);
    a11y::disabled(&resp);
    draw(ui.painter(), Rect::from_center_size(r.center(), vec2(16.0, 16.0)), t.disabled);
    resp
}

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    store::load(app);
    let t = Tokens::get(ui.ctx());
    let w = ui.available_width();
    let mut seen = Seen { width: w, ..Seen::default() };
    header(app, ui, &t, w, &mut seen);
    let visible = ui.clip_rect();
    let below = Rect::from_min_max(pos2(visible.min.x, ui.cursor().min.y), visible.max);
    if app.ui.agent_view.chat.store.open {
        seen.popover = Some(popover(app, ui, &t, w));
    }
    if store::read_only(app) {
        seen.read_only_note = true;
        let doc = store::current(app).map(|c| c.doc_name.clone()).unwrap_or_default();
        widgets::note(ui, &trf("From {document}: open that document to continue.", &[("document", &doc)]), t.text_secondary);
    }
    let lines = app.ui.agent_view.chat.lines.clone();
    seen.lines = lines.len();
    if lines.is_empty() {
        seen.empty_state = true;
        empty_state(ui, &t, below);
    } else {
        transcript(ui, &t, &lines, below.height().max(120.0));
    }
    seen.widest = seen.widest.max(ui.min_rect().width());
    seen.composer = std::mem::take(&mut app.ui.agent_view.chat.seen.composer);
    app.ui.agent_view.chat.seen = seen;
}

fn header(app: &mut FigApp, ui: &mut Ui, t: &Tokens, w: f32, seen: &mut Seen) {
    let (row, _) = ui.allocate_exact_size(vec2(w, HEADER_H), Sense::hover());
    let title = store::title(app);
    seen.title = title.clone();
    let state = figcraft_engine::cmd::camera::follow_state(&app.session);
    // Paused only shows while an agent works (Fix N).
    let paused = crate::follow_agent::follow_end::shown(app);
    seen.follow = if paused {
        "paused".into()
    } else if state == "following" {
        "following".into()
    } else {
        "off".into()
    };
    // The buttons at the right: New chat, then Follow agents.
    let plus = Rect::from_min_size(pos2(row.max.x - PAD - BUTTON, row.center().y - BUTTON / 2.0), vec2(BUTTON, BUTTON));
    let follow = plus.translate(vec2(-BUTTON - 4.0, 0.0));
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(follow));
    let label = if paused { tr("Follow agents as they work: paused, press to resume") } else { tr("Follow agents as they work") };
    let fresp = icon_button(&mut b, "follow", label, Some(state == "following" && !paused), |p, r, c| icons::paint(p, r, AiIcon::Crosshair, c));
    seen.follow_rect = Some(fresp.rect);
    let fresp = match crate::follow_agent::status(app) {
        Some(s) => fresp.on_hover_text(s),
        None => fresp,
    };
    if paused {
        ui.painter().circle_filled(follow.right_top() + vec2(-5.0, 5.0), 3.5, t.warning);
    }
    if fresp.clicked() {
        let on = paused || state != "following";
        app.run("camera.follow-agents", json!({"on": on}));
    }
    // New chat: not while a composer run goes, whose lines belong to the conversation shown (Fix U3).
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(plus));
    seen.new_chat_enabled = app.ui.agent_view.chat.running.is_none();
    let plus_icon = |p: &egui::Painter, r: Rect, c: egui::Color32| crate::icons::paint(p, r, Icon::Plus, c);
    if !seen.new_chat_enabled {
        // The engine's refusal says the same (`chat::STOP_FIRST`).
        icon_button_off(&mut b, tr("New chat"), tr("Stop the run first"), plus_icon);
        seen.new_chat_color = t.disabled;
    } else {
        seen.new_chat_color = t.icon;
        if icon_button(&mut b, "new", tr("New chat"), None, plus_icon).clicked() {
            app.run("chat.new", json!({}));
        }
    }
    // The title and its chevron: the conversations.
    let title_r = Rect::from_min_max(pos2(row.min.x + PAD - 6.0, row.min.y + 7.0), pos2(follow.min.x - 6.0, row.max.y - 7.0));
    let tresp = ui.interact(title_r, Id::new("ai_panel_title"), Sense::click()).on_hover_text(tr("Conversations"));
    let open = app.ui.agent_view.chat.store.open;
    a11y::menu_button(&tresp, &trf("Conversation: {title}", &[("title", &title)]), open);
    if tresp.hovered() || open {
        ui.painter().rect_filled(title_r, CornerRadius::same(5), t.hover);
    }
    let chevron = Rect::from_center_size(pos2(title_r.max.x - 12.0, title_r.center().y), vec2(10.0, 10.0));
    let text_r = Rect::from_min_max(pos2(title_r.min.x + 6.0, title_r.min.y), pos2(chevron.min.x - 4.0, title_r.max.y));
    ui.painter().with_clip_rect(text_r).text(pos2(text_r.min.x, text_r.center().y), Align2::LEFT_CENTER, &title, semibold(12.0), t.text);
    crate::icons::paint(ui.painter(), chevron, Icon::Chevron, t.icon_secondary);
    if tresp.clicked() {
        app.ui.agent_view.chat.store.open = !open;
    }
    ui.painter().line_segment([row.left_bottom(), row.right_bottom()], Stroke::new(1.0, t.border));
}

/// The conversations: search, scope, list. What it showed, for `ui.inspect`.
fn popover(app: &mut FigApp, ui: &mut Ui, t: &Tokens, w: f32) -> Value {
    let inner = w - 2.0 * PAD;
    let r = egui::Frame::NONE
        .fill(t.menu)
        .stroke(Stroke::new(1.0, t.field_edge))
        .corner_radius(8)
        .inner_margin(egui::Margin::same(8))
        .outer_margin(egui::Margin::symmetric(PAD as i8, 4))
        .show(ui, |ui| {
            // Within the panel: its margins, the frame's inner margins and its 1 px edge.
            ui.set_width(inner - 16.0 - 2.0);
            let search_id = Id::new("ai_conversation_search");
            let st = &mut app.ui.agent_view.chat.store;
            let field = egui::Frame::NONE
                .fill(t.field)
                .stroke(crate::theme::field_stroke(t, false, ui.memory(|m| m.has_focus(search_id))))
                .corner_radius(5)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut st.query)
                            .id(search_id)
                            .frame(egui::Frame::NONE)
                            .font(regular(11.5))
                            .hint_text(tr("Search conversations…"))
                            .desired_width(f32::INFINITY),
                    )
                })
                .inner;
            a11y::text(&field, tr("Search conversations"), &st.query.clone());
            ui.add_space(6.0);
            let all = st.all;
            let scopes = [tr("This document"), tr("All documents")];
            if let Some(i) = widgets::segmented_text(ui, &scopes, usize::from(all)) {
                app.run("chat.scope", json!({"scope": if i == 1 { "all" } else { "document" }}));
            }
            ui.add_space(6.0);
            let current = app.ui.agent_view.chat.store.current.clone();
            let key = store::doc_key(app).map(|k| k.0);
            let listed: Vec<(String, String, String, bool)> = store::listed(app)
                .into_iter()
                .map(|c| (c.id.clone(), c.title.clone(), c.doc_name.clone(), Some(&c.doc) != key.as_ref()))
                .collect();
            let mut items = Vec::new();
            if listed.is_empty() {
                widgets::note(ui, tr("No conversations yet."), t.text_secondary);
            }
            for (id, title, doc, other) in listed {
                let check = current.as_deref() == Some(id.as_str());
                let rw = ui.available_width();
                let (rr, resp) = ui.allocate_exact_size(vec2(rw, if all { 36.0 } else { 26.0 }), Sense::click());
                let spoken = if other { trf("{title}, in {document}", &[("title", &title), ("document", &doc)]) } else { title.clone() };
                a11y::list_item(&resp, &spoken, check);
                if resp.hovered() {
                    ui.painter().rect_filled(rr, CornerRadius::same(5), t.hover);
                }
                let text_r = Rect::from_min_max(pos2(rr.min.x + 8.0, rr.min.y), pos2(rr.max.x - 26.0, rr.max.y));
                let p = ui.painter().with_clip_rect(text_r);
                let y = if all { rr.min.y + 12.0 } else { rr.center().y };
                p.text(pos2(text_r.min.x, y), Align2::LEFT_CENTER, &title, regular(11.5), t.text);
                if all {
                    p.text(pos2(text_r.min.x, rr.min.y + 26.0), Align2::LEFT_CENTER, &doc, regular(10.5), t.text_secondary);
                }
                if check {
                    crate::icons::paint(ui.painter(), Rect::from_center_size(pos2(rr.max.x - 13.0, rr.center().y), vec2(12.0, 12.0)), Icon::Check, t.text);
                }
                if resp.clicked() {
                    app.run("chat.open", json!({"id": id}));
                }
                items.push(json!({"id": id, "title": title, "document": doc, "current": check, "readOnly": other}));
            }
            json!({"search": app.ui.agent_view.chat.store.query, "searchPlaceholder": tr("Search conversations…"), "scope": if all { "all" } else { "document" }, "scopes": scopes, "items": items})
        });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.ui.agent_view.chat.store.open = false;
    }
    r.inner
}

fn empty_state(ui: &mut Ui, t: &Tokens, below: Rect) {
    let h = (below.height() - 40.0).max(120.0);
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let c = pos2(r.center().x, r.center().y - 30.0);
    ui.painter().circle_filled(c, 20.0, t.field);
    crate::icons::paint(ui.painter(), Rect::from_center_size(c, vec2(18.0, 18.0)), Icon::Comment, t.icon);
    let text_r = Rect::from_min_max(pos2(r.min.x + PAD, c.y + 30.0), pos2(r.max.x - PAD, r.max.y));
    ui.scope_builder(egui::UiBuilder::new().max_rect(text_r).layout(egui::Layout::top_down(egui::Align::Center)), |ui| {
        ui.add(egui::Label::new(RichText::new(tr("Describe what you want to create or change.")).font(regular(12.0)).color(t.text)).wrap());
    });
}

fn transcript(ui: &mut Ui, t: &Tokens, lines: &[Line], h: f32) {
    egui::ScrollArea::vertical().id_salt("ai_transcript").max_height(h).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
        ui.add_space(8.0);
        for l in lines {
            let (who, text, color) = match l {
                Line::You(s) => (Some(tr("You").to_string()), s.as_str(), t.text),
                Line::Reply { agent, text } => (Some(agent.clone()), text.as_str(), t.text),
                Line::Work(s) | Line::Note(s) | Line::Cost(s) => (None, s.as_str(), t.text_secondary),
                Line::Failure(s) => (None, s.as_str(), t.danger),
            };
            ui.horizontal(|ui| {
                ui.add_space(PAD);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() - PAD);
                    if let Some(who) = who {
                        ui.label(RichText::new(who).font(semibold(10.5)).color(t.text_secondary));
                    }
                    ui.label(RichText::new(text).font(regular(11.0)).color(color));
                });
            });
            ui.add_space(6.0);
        }
    });
}

/// For `ui.inspect` › `agent.aiPanel`: what the panel drew last frame.
pub fn inspect(app: &FigApp) -> Value {
    let s = &app.ui.agent_view.chat.seen;
    json!({
        "header": {
            "title": s.title,
            "chevron": true,
            "follow": s.follow,
            "followLabel": tr("Follow agents as they work"),
            "followRect": s.follow_rect.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y]),
            "newChat": tr("New chat"),
            "newChatEnabled": s.new_chat_enabled,
            "newChatTooltip": if s.new_chat_enabled { tr("New chat") } else { tr("Stop the run first") },
            "newChatColor": format!("{:?}", s.new_chat_color),
        },
        "popover": s.popover,
        "emptyState": s.empty_state.then(|| tr("Describe what you want to create or change.")),
        "readOnly": s.read_only_note,
        "lines": s.lines,
        "composer": s.composer,
        "width": s.width,
        "widest": s.widest,
    })
}

#[cfg(test)]
#[path = "ai_panel_tests.rs"]
mod tests;
