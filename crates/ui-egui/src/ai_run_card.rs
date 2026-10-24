//! The AI panel's run (Fix X4, `docs/design/ai-panel.md`): the working banner under the header
//! while a composer run goes; then, in the transcript, your message as a bubble and the run's card.
//! - **The banner:** 36 high, three 6 px dots (the working indicator's rhythm, still with reduced
//!   motion), "{agent} is working", and the time.
//! - **The bubble:** right-aligned, radius 14, at most the panel's width less 56.
//! - **The card:** "Run · {time}" (then "Done" or "Stopped") and "Undo is one step"; the run's edits
//!   from the engine's change log (`chat_run::rows`), the newest 5, a done circle for each and,
//!   while it works, the newest as the active one; an indeterminate bar while it works (no
//!   percentage: the agent has no plan to measure); Select layers and Undo run.
//!
//! Every colour is a role from `ai_panel_theme.rs`.

use egui::{Align2, CornerRadius, Rect, RichText, Sense, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::FigApp;
use crate::follow_agent::chat_ui::Line;
use crate::i18n::{tr, trf};
use crate::panel_view::ai_tab::panel::Seen;
use crate::panel_view::ai_tab::panel_theme::{Roles, roles};
use crate::theme::{Tokens, regular, semibold};

/// Below this panel width the card's buttons stack (Fix Y2), and the least room either side of a
/// button's label.
pub(crate) const STACK_BELOW: f32 = 300.0;
pub(crate) const LABEL_PAD: f32 = 12.0;
/// The banner's height, and the card's rows: how many, and how far apart.
pub const BANNER_H: f32 = 36.0;
/// The transcript's fade at its bottom (Fix Z4).
const FADE: f32 = 12.0;

/// "0:42" (the banner).
pub(crate) fn clock(secs: f64) -> String {
    crate::follow_agent::chat_ui::elapsed(secs)
}

/// "42 s", or "1:05" from a minute (the card's heading).
pub(crate) fn span(secs: f64) -> String {
    if secs < 60.0 { format!("{} s", secs.max(0.0) as u64) } else { clock(secs) }
}

pub(crate) fn motion(app: &FigApp) -> bool {
    !app.reduce_motion()
}

/// The three dots' opacities now: 1, 0.65, 0.35, moving along; still with reduced motion.
fn dots(now: f64, moving: bool) -> [f32; 3] {
    let o = [1.0, 0.65, 0.35];
    let k = if moving { ((now / 0.4) as usize) % 3 } else { 0 };
    [o[(3 - k) % 3], o[(4 - k) % 3], o[(5 - k) % 3]]
}

/// The working banner, while a run goes: directly under the header's rule.
pub fn banner(app: &FigApp, ui: &mut Ui, seen: &mut Seen) {
    let Some(r) = &app.ui.agent_view.chat.running else { return };
    let t = Tokens::get(ui.ctx());
    let ro = roles(&t);
    let now = ui.ctx().input(|i| i.time);
    let w = ui.available_width();
    let (row, resp) = ui.allocate_exact_size(vec2(w, BANNER_H), Sense::hover());
    ui.painter().rect_filled(row, 0.0, ro.banner);
    let moving = motion(app);
    for (i, o) in dots(now, moving).iter().enumerate() {
        ui.painter().circle_filled(pos2(row.min.x + 16.0 + 3.0 + 10.0 * i as f32, row.center().y), 3.0, ro.mark.gamma_multiply(*o));
    }
    let text = if r.stopping { tr("Stopping…").to_string() } else { trf("{agent} is working", &[("agent", &r.agent.name)]) };
    ui.painter().text(pos2(row.min.x + 52.0, row.center().y), Align2::LEFT_CENTER, &text, regular(12.0), ro.text);
    seen.fonts.insert("banner", regular(12.0));
    let time = clock(r.elapsed(now));
    ui.painter().text(pos2(row.max.x - 16.0, row.center().y), Align2::RIGHT_CENTER, &time, regular(12.0), ro.text_secondary);
    crate::a11y::status(&resp, &text, &time);
    seen.theme.insert("banner", ro.banner);
    seen.theme.insert("workingDots", ro.mark);
    seen.banner = Some(json!({"text": text, "time": time}));
    if moving {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
}

/// The transcript: your messages as bubbles, the run's card after its message, the agent's replies
/// and notes as text.
pub fn transcript(app: &mut FigApp, ui: &mut Ui, lines: &[Line], h: f32, seen: &mut Seen) {
    let t = Tokens::get(ui.ctx());
    let ro = roles(&t);
    let run = app.ui.agent_view.chat.last_run.clone().filter(|r| r.conv == app.ui.agent_view.chat.store.current && r.line < lines.len());
    let mut actions = Vec::new();
    let out = egui::ScrollArea::vertical().id_salt("ai_transcript").max_height(h).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
        ui.add_space(24.0);
        for (i, l) in lines.iter().enumerate() {
            // The card shows the run's edits: its work lines aren't repeated.
            if matches!(l, Line::Work(_)) && run.as_ref().is_some_and(|r| i > r.line) {
                continue;
            }
            match l {
                Line::You(s) => {
                    bubble(ui, &ro, s);
                    seen.bubbles += 1;
                    seen.fonts.insert("bubble", regular(13.0));
                    seen.theme.insert("bubble", ro.bubble);
                }
                Line::Reply { agent, text } => said(ui, Some(agent), text, ro.text, &ro, seen),
                Line::Work(s) | Line::Note(s) | Line::Cost(s) => said(ui, None, s, ro.text_secondary, &ro, seen),
                Line::Failure(s) => said(ui, None, s, ro.failure, &ro, seen),
            }
            if let Some(r) = run.as_ref().filter(|r| r.line == i) {
                actions.extend(super::card::card(app, ui, &ro, r, seen));
                ui.add_space(18.0);
            }
        }
    });
    // A 12 px fade into the panel at the bottom, only while there is more below (Fix Z4).
    let below = out.content_size.y - (out.state.offset.y + out.inner_rect.height());
    if below > 0.5 {
        let r = Rect::from_min_max(pos2(out.inner_rect.min.x, out.inner_rect.max.y - FADE), out.inner_rect.max);
        fade(ui.painter(), r, ro.panel);
        seen.theme.insert("fade", ro.panel);
        seen.fade = Some(json!({"rect": [r.min.x, r.min.y, r.max.x, r.max.y], "below": below}));
    }
    for (cmd, params) in actions {
        press(app, cmd, &params);
    }
}

/// The document's revision now (any edit or selection moves it on).
pub(crate) fn revision(app: &FigApp) -> u64 {
    app.session.ds().map_or(0, |d| d.revision)
}

/// A card button's command: run as `app.run` does, but a refusal is the card's line under its
/// buttons (Fix Z7), not a toast. Diagnostics notes it either way ("chat.undo-run: refused (already
/// undone)").
fn press(app: &mut FigApp, cmd: &str, params: &Value) {
    let r = app.session.execute(cmd, params);
    app.handle_ui_requests();
    crate::settings_privacy::noted(app, cmd, r.as_ref().err());
    app.ui.agent_view.chat.refused = r.err().map(|e| {
        let words = match e {
            figcraft_engine::EngineError::Disabled(_, why) => why,
            other => other.to_string(),
        };
        (words, revision(app))
    });
}

/// The transcript's fade: `to` at the bottom of `r`, from transparent at its top.
fn fade(p: &egui::Painter, r: Rect, to: egui::Color32) {
    let clear = to.gamma_multiply(0.0);
    let mut m = egui::Mesh::default();
    for (at, c) in [(r.left_top(), clear), (r.right_top(), clear), (r.left_bottom(), to), (r.right_bottom(), to)] {
        m.colored_vertex(at, c);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    p.add(egui::Shape::mesh(m));
}

/// `text` as it fits in `max` points of `font`: whole, or cut with "…".
pub fn elide(p: &egui::Painter, text: &str, font: egui::FontId, max: f32) -> String {
    let width = |t: &str| p.layout_no_wrap(t.to_string(), font.clone(), egui::Color32::PLACEHOLDER).size().x;
    if width(text) <= max {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let cut = chars[..mid].iter().collect::<String>().trim_end().to_string() + "…";
        if width(&cut) <= max {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>().trim_end().to_string() + "…"
}

/// Your message: right-aligned, radius 14, 14 × 11 inside, at most the width less 56, 16 from the
/// right, 18 above what follows.
fn bubble(ui: &mut Ui, ro: &Roles, text: &str) {
    let w = ui.available_width();
    let max = (w - 56.0).max(80.0);
    let mut job = egui::text::LayoutJob::single_section(
        text.to_string(),
        egui::TextFormat { font_id: regular(13.0), color: ro.text, line_height: Some(13.0 * 1.4), ..Default::default() },
    );
    job.wrap.max_width = max - 28.0;
    let galley = ui.painter().layout_job(job);
    let size = vec2(galley.size().x + 28.0, galley.size().y + 22.0);
    let (row, _) = ui.allocate_exact_size(vec2(w, size.y), Sense::hover());
    let r = Rect::from_min_size(pos2(row.max.x - 16.0 - size.x, row.min.y), size);
    ui.painter().rect_filled(r, CornerRadius::same(14), ro.bubble);
    ui.put(Rect::from_min_size(r.min + vec2(14.0, 11.0), galley.size()), egui::Label::new(galley));
    ui.add_space(18.0);
}

/// An agent's reply (its Markdown drawn, Fix Z2) or a note (plain): text 16 from each side.
fn said(ui: &mut Ui, who: Option<&str>, text: &str, color: egui::Color32, ro: &Roles, seen: &mut Seen) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 16.0);
            if let Some(who) = who {
                ui.label(RichText::new(who).font(semibold(12.0)).color(ro.text_secondary));
                seen.fonts.insert("agentName", semibold(12.0));
                let blocks = crate::panel_view::ai_tab::reply::draw(ui, text, ro, &mut seen.fonts);
                seen.replies.push(json!({"agent": who, "blocks": blocks}));
                seen.theme.insert("code", ro.code);
            } else {
                ui.label(RichText::new(text).font(regular(13.0)).color(color));
                seen.fonts.insert("note", regular(13.0));
            }
        });
    });
    ui.add_space(10.0);
}
