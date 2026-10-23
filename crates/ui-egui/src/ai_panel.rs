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

use egui::{Align2, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use super::icons::{self, AiIcon};
use crate::follow_agent::chat_ui::store;
use crate::i18n::{tr, trf};
use crate::icons::Icon;
use crate::theme::{Tokens, regular, semibold};
use crate::widgets::{self, PAD};
use crate::{FigApp, a11y};

/// The header's height (Fix X2).
pub const HEADER_H: f32 = 48.0;
/// An icon button's size: the header's Follow and New chat (Fix X2), and the composer's.
pub const BUTTON: f32 = 28.0;
/// An icon button's corner radius.
const BUTTON_RADIUS: u8 = 8;
/// Between the header's two buttons.
const BUTTON_GAP: f32 = 8.0;

/// What the panel drew last frame (for `ui.inspect` and tests).
#[derive(Clone, Debug, Default)]
pub struct Seen {
    pub title: String,
    /// following | paused | off
    pub follow: String,
    pub popover: Option<Value>,
    pub empty_state: bool,
    /// Where the Follow agents button was drawn; the setting off (the crosshair dimmed) and its
    /// tooltip (Fix V3).
    pub follow_rect: Option<Rect>,
    /// The header's rectangle and New chat's (Fix X2).
    pub header_rect: Option<Rect>,
    pub new_chat_rect: Option<Rect>,
    pub follow_off: bool,
    pub follow_tip: String,
    pub read_only_note: bool,
    /// New chat could be used, and the colour its icon was drawn in (Fix U3).
    pub new_chat_enabled: bool,
    pub new_chat_color: egui::Color32,
    pub lines: usize,
    /// The working banner, the bubbles, the run's card (Fix X4).
    pub banner: Option<Value>,
    pub bubbles: usize,
    pub card: Option<Value>,
    /// Each reply's blocks as drawn: kind, text as shown, rectangle (Fix Z2).
    pub replies: Vec<Value>,
    /// The colour drawn for each role this frame (Fix X5, `ui.inspect` › `agent.aiPanel.theme`).
    pub theme: std::collections::BTreeMap<&'static str, egui::Color32>,
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
    icon_button_tip(ui, id, label, label, on, draw)
}

/// [`icon_button`] whose tooltip says more than its accessible name (Fix V3: following off).
pub(crate) fn icon_button_tip(
    ui: &mut Ui,
    id: &str,
    label: &str,
    tip: &str,
    on: Option<bool>,
    draw: impl FnOnce(&egui::Painter, Rect, egui::Color32),
) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let roles = super::panel_theme::roles(&t);
    let (r, resp) = ui.allocate_exact_size(vec2(BUTTON, BUTTON), Sense::click());
    let resp = resp.on_hover_text(tip);
    match on {
        Some(v) => a11y::toggle(&resp, label, v),
        None => a11y::button(&resp, label),
    }
    let _ = id;
    // At rest the pressed fill; a toggle that's on, the accent's tint (Fix X1).
    let fill = match (on == Some(true), resp.hovered()) {
        (true, _) => roles.follow_on_fill,
        (false, true) => t.selected_row,
        (false, false) => roles.button_fill,
    };
    ui.painter().rect_filled(r, CornerRadius::same(BUTTON_RADIUS), fill);
    if resp.has_focus() {
        ui.painter().rect_stroke(r, CornerRadius::same(BUTTON_RADIUS), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    let color = if on == Some(true) { roles.follow_on_icon } else { roles.follow_off_icon };
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
    super::run_card::banner(app, ui, &mut seen);
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
        empty_state(ui, &t, below, &mut seen);
    } else {
        super::run_card::transcript(app, ui, &lines, below.height().max(120.0), &mut seen);
    }
    seen.widest = seen.widest.max(ui.min_rect().width());
    seen.composer = std::mem::take(&mut app.ui.agent_view.chat.seen.composer);
    app.ui.agent_view.chat.seen = seen;
}

fn header(app: &mut FigApp, ui: &mut Ui, t: &Tokens, w: f32, seen: &mut Seen) {
    let roles = super::panel_theme::roles(t);
    let (row, _) = ui.allocate_exact_size(vec2(w, HEADER_H), Sense::hover());
    seen.header_rect = Some(row);
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
    let follow = plus.translate(vec2(-BUTTON - BUTTON_GAP, 0.0));
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(follow));
    let label = if paused { tr("Follow agents as they work: paused, press to resume") } else { tr("Follow agents as they work") };
    // Off is plain to see (Fix V3): the crosshair dimmed, and the tooltip says so.
    let off = state == "off";
    let tip = if off { tr("Following is off. Click to follow agents as they work") } else { label };
    (seen.follow_off, seen.follow_tip) = (off, tip.to_string());
    // Off: the crosshair in the icon colour on the button's rest fill (Fix X1); the status line says
    // "Following is off".
    let fresp =
        icon_button_tip(&mut b, "follow", label, tip, Some(state == "following" && !paused), |p, r, c| icons::paint(p, r, AiIcon::Crosshair, c));
    seen.follow_rect = Some(fresp.rect);
    // What Follow was drawn in (Fix X5): the icon, and its fill as seen over the panel.
    let on = state == "following" && !paused;
    seen.theme.insert("followIcon", if on { roles.follow_on_icon } else { roles.follow_off_icon });
    seen.theme.insert("followFill", if on { crate::theme::accent::over(roles.follow_on_fill, roles.panel) } else { roles.button_fill });
    let fresp = match crate::follow_agent::status(app) {
        Some(s) => fresp.on_hover_text(s),
        None => fresp,
    };
    if paused {
        ui.painter().circle_filled(follow.right_top() + vec2(-5.0, 5.0), 3.5, super::panel_theme::roles(t).paused_dot);
    }
    if fresp.clicked() {
        let on = paused || state != "following";
        app.run("camera.follow-agents", json!({"on": on, "origin": "button"}));
    }
    // New chat: not while a composer run goes, whose lines belong to the conversation shown (Fix U3).
    let mut b = ui.new_child(egui::UiBuilder::new().max_rect(plus));
    seen.new_chat_enabled = app.ui.agent_view.chat.running.is_none();
    seen.new_chat_rect = Some(plus);
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
    // The title (SemiBold 14, 16 from the left) and its 9 × 5 chevron 6 after it: the conversations.
    let galley = ui.painter().layout_no_wrap(title.clone(), semibold(14.0), roles.text);
    let room = (follow.min.x - 12.0 - (row.min.x + PAD) - 6.0 - 9.0).max(24.0);
    let text_w = galley.size().x.min(room);
    let title_r = Rect::from_min_max(
        pos2(row.min.x + PAD - 6.0, row.center().y - 14.0),
        pos2(row.min.x + PAD + text_w + 6.0 + 9.0 + 6.0, row.center().y + 14.0),
    );
    let tresp = ui.interact(title_r, Id::new("ai_panel_title"), Sense::click()).on_hover_text(tr("Conversations"));
    let open = app.ui.agent_view.chat.store.open;
    a11y::menu_button(&tresp, &trf("Conversation: {title}", &[("title", &title)]), open);
    if tresp.hovered() || open {
        ui.painter().rect_filled(title_r, CornerRadius::same(6), t.hover);
    }
    let text_r = Rect::from_min_size(pos2(row.min.x + PAD, row.center().y - galley.size().y / 2.0), vec2(text_w, galley.size().y));
    ui.painter().with_clip_rect(text_r.expand2(vec2(0.0, 2.0))).galley(text_r.min, galley, roles.text);
    icons::chevron(ui.painter(), pos2(text_r.max.x + 6.0 + 4.5, row.center().y), 9.0, 5.0, 1.5, roles.chevron);
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

/// No messages yet (Fix X2): a 64 px disc with the accent's 32 px core, the heading 24 below it, the
/// line 10 below that; the block centred between the header's rule and the status line, 12 above
/// the true centre.
fn empty_state(ui: &mut Ui, t: &Tokens, below: Rect, seen: &mut Seen) {
    let roles = super::panel_theme::roles(t);
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), below.height().max(160.0)), Sense::hover());
    let wrap = (r.width() - 48.0).max(80.0);
    let centred = |text: &str, font: egui::FontId, color: egui::Color32, line: Option<f32>| {
        let mut job = egui::text::LayoutJob::single_section(
            text.to_string(),
            egui::TextFormat { font_id: font, color, line_height: line, ..Default::default() },
        );
        job.wrap.max_width = wrap;
        job.halign = egui::Align::Center;
        ui.painter().layout_job(job)
    };
    let heading = centred(tr("Describe what you want to create or change."), semibold(20.0), roles.text, None);
    let body = centred(tr("Your agent edits the canvas while you watch."), regular(13.0), roles.text_secondary, Some(13.0 * 1.4));
    let block = 64.0 + 24.0 + heading.size().y + 10.0 + body.size().y;
    let top = r.center().y - 12.0 - block / 2.0;
    let c = pos2(r.center().x, top + 32.0);
    ui.painter().circle_filled(c, 32.0, roles.empty_disc);
    ui.painter().circle_filled(c, 16.0, roles.mark);
    seen.theme.insert("emptyDisc", roles.empty_disc);
    seen.theme.insert("emptyCore", roles.mark);
    let heading_r = Rect::from_center_size(pos2(r.center().x, top + 64.0 + 24.0 + heading.size().y / 2.0), heading.size());
    let body_r = Rect::from_center_size(pos2(r.center().x, heading_r.max.y + 10.0 + body.size().y / 2.0), body.size());
    ui.put(heading_r, egui::Label::new(heading));
    ui.put(body_r, egui::Label::new(body));
}

/// The colour drawn for each role last frame, `#rrggbb` as seen (Fix X5): the panel's, and the
/// composer's (drawn in its own pass).
fn theme(s: &Seen) -> Value {
    let mut out: serde_json::Map<String, Value> =
        s.theme.iter().map(|(k, c)| ((*k).to_string(), json!(crate::theme::accent::rgb_text(*c)))).collect();
    if let Some(Value::Object(c)) = s.composer.get("theme") {
        out.extend(c.clone());
    }
    Value::Object(out)
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
            "followSetting": if s.follow_off { "OFF" } else { "ON" },
            "followTooltip": s.follow_tip,
            "newChat": tr("New chat"),
            "newChatEnabled": s.new_chat_enabled,
            "newChatTooltip": if s.new_chat_enabled { tr("New chat") } else { tr("Stop the run first") },
            "newChatColor": format!("{:?}", s.new_chat_color),
        },
        "headerRect": s.header_rect.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y]),
        "newChatRect": s.new_chat_rect.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y]),
        "popover": s.popover,
        "emptyState": s.empty_state.then(|| tr("Describe what you want to create or change.")),
        "emptyLine": s.empty_state.then(|| tr("Your agent edits the canvas while you watch.")),
        "banner": s.banner,
        "bubbles": s.bubbles,
        "card": s.card,
        "replies": s.replies,
        "readOnly": s.read_only_note,
        "lines": s.lines,
        "composer": s.composer,
        "theme": theme(s),
        "width": s.width,
        "widest": s.widest,
    })
}

#[cfg(test)]
#[path = "ai_panel_tests.rs"]
mod tests;
