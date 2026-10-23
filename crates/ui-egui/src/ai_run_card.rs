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

use egui::{Align2, CornerRadius, Rect, Response, RichText, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::FigApp;
use crate::follow_agent::chat_ui::{LastRun, Line};
use crate::i18n::{tr, trf};
use crate::panel_view::ai_tab::panel::Seen;
use crate::panel_view::ai_tab::panel_theme::{Roles, roles};
use crate::theme::{Tokens, regular, semibold};

/// Below this panel width the card's buttons stack (Fix Y2), and the least room either side of a
/// button's label.
const STACK_BELOW: f32 = 300.0;
const LABEL_PAD: f32 = 12.0;
/// The banner's height, and the card's rows: how many, and how far apart.
pub const BANNER_H: f32 = 36.0;
const ROWS: usize = 5;
const ROW: f32 = 36.0;

/// "0:42" (the banner).
fn clock(secs: f64) -> String {
    crate::follow_agent::chat_ui::elapsed(secs)
}

/// "42 s", or "1:05" from a minute (the card's heading).
fn span(secs: f64) -> String {
    if secs < 60.0 { format!("{} s", secs.max(0.0) as u64) } else { clock(secs) }
}

fn motion(app: &FigApp) -> bool {
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
    egui::ScrollArea::vertical().id_salt("ai_transcript").max_height(h).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
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
                    seen.theme.insert("bubble", ro.bubble);
                }
                Line::Reply { agent, text } => said(ui, Some(agent), text, ro.text, &ro),
                Line::Work(s) | Line::Note(s) | Line::Cost(s) => said(ui, None, s, ro.text_secondary, &ro),
                Line::Failure(s) => said(ui, None, s, ro.failure, &ro),
            }
            if let Some(r) = run.as_ref().filter(|r| r.line == i) {
                actions.extend(card(app, ui, &ro, r, seen));
                ui.add_space(18.0);
            }
        }
    });
    for (cmd, params) in actions {
        app.run(cmd, params);
    }
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

/// An agent's reply or a note: text 16 from each side.
fn said(ui: &mut Ui, who: Option<&str>, text: &str, color: egui::Color32, ro: &Roles) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 16.0);
            if let Some(who) = who {
                ui.label(RichText::new(who).font(semibold(12.0)).color(ro.text_secondary));
            }
            ui.label(RichText::new(text).font(regular(13.0)).color(color));
        });
    });
    ui.add_space(10.0);
}

/// The run's card: what it did, and its two buttons. Returns the command a button asked for.
fn card(app: &FigApp, ui: &mut Ui, ro: &Roles, run: &LastRun, seen: &mut Seen) -> Option<(&'static str, Value)> {
    let now = ui.ctx().input(|i| i.time);
    let working = app.ui.agent_view.chat.running.is_some() && run.ended.is_none();
    let secs = run.ended.map_or_else(|| now - run.started, |(s, _)| s);
    let heading = match run.ended {
        None => trf("Run · {time}", &[("time", &span(secs))]),
        Some((_, "stopped")) => trf("Stopped · {time}", &[("time", &span(secs))]),
        Some(_) => trf("Done · {time}", &[("time", &span(secs))]),
    };
    let note = tr("Undo is one step");
    let all = figcraft_engine::cmd::chat_run::rows(&app.session);
    let earlier = all.len().saturating_sub(ROWS);
    let shown: Vec<(String, usize)> = all.into_iter().skip(earlier).collect();
    let panel_w = ui.available_width();
    let width = panel_w - 32.0;
    let inner = width - 32.0;
    let rows_h = shown.len() as f32 * ROW + if earlier > 0 { 24.0 } else { 0.0 };
    // The bar (4 above it, 4 high, 12 below), then 4 more: the buttons 16 below the bar or the rows.
    let bar_h = if working { 4.0 + 4.0 + 12.0 } else { 0.0 };
    // 16 inside, the heading (18), 12, the rows, the bar, 4, the buttons (34), 16 inside.
    // The buttons side by side, or stacked (Fix Y2): stacked below a 300 px panel, or wherever a
    // label wouldn't keep 12 px either side in half the width (a longer language, say).
    let side = (inner - 8.0) / 2.0;
    let measure = ui.painter().clone();
    let label_w = |l: &str| measure.layout_no_wrap(l.to_string(), regular(13.0), ro.text).size().x;
    let widest = label_w(tr("Select layers")).max(label_w(tr("Undo run")));
    // (The panel's own width: inside the transcript a scroll bar's room is taken off.)
    let stacked = app.ui.right_width < STACK_BELOW || widest + 2.0 * LABEL_PAD > side;
    let buttons_h = if stacked { 34.0 + 8.0 + 34.0 } else { 34.0 };
    let height = 16.0 + 18.0 + 12.0 + rows_h + bar_h + 4.0 + buttons_h + 16.0;
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let r = Rect::from_min_size(pos2(row.min.x + 16.0, row.min.y), vec2(width, height));
    ui.painter().rect_filled(r, CornerRadius::same(12), ro.card);
    ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, ro.card_edge), egui::StrokeKind::Inside);
    let x = r.min.x + 16.0;
    let mut y = r.min.y + 16.0 + 9.0;
    let p = ui.painter().clone();
    p.text(pos2(x, y), Align2::LEFT_CENTER, &heading, semibold(13.0), ro.text);
    p.text(pos2(x + inner, y), Align2::RIGHT_CENTER, note, regular(12.0), ro.text_secondary);
    y += 9.0 + 12.0;
    if earlier > 0 {
        p.text(pos2(x, y + 8.0), Align2::LEFT_CENTER, trf("{n} earlier", &[("n", &earlier)]), regular(12.0), ro.text_secondary);
        y += 24.0;
    }
    let mut listed = Vec::new();
    for (i, (label, n)) in shown.iter().enumerate() {
        let c = pos2(x + 8.0, y + ROW / 2.0);
        let active = working && i + 1 == shown.len();
        if active {
            p.circle_stroke(c, 7.0, Stroke::new(2.0, ro.mark));
            p.circle_filled(c, 3.0, ro.mark);
            seen.theme.insert("activeRing", ro.mark);
        } else {
            p.circle_filled(c, 8.0, ro.success);
            let line = Stroke::new(1.6, ro.done_check);
            p.add(egui::Shape::line(vec![c + vec2(-3.5, 0.2), c + vec2(-1.0, 2.8), c + vec2(3.8, -2.6)], line));
            seen.theme.insert("doneCircle", ro.success);
            seen.theme.insert("doneCheck", ro.done_check);
        }
        let text = if *n > 1 { format!("{label} ×{n}") } else { label.clone() };
        let font = if active { semibold(13.0) } else { regular(13.0) };
        p.with_clip_rect(Rect::from_min_max(pos2(x + 26.0, y), pos2(x + inner, y + ROW))).text(
            pos2(x + 26.0, c.y),
            Align2::LEFT_CENTER,
            &text,
            font,
            ro.text,
        );
        listed.push(json!({"label": text, "state": if active { "active" } else { "done" }}));
        y += ROW;
    }
    if working {
        y += 4.0;
        let track = Rect::from_min_size(pos2(x, y), vec2(inner, 4.0));
        p.rect_filled(track, CornerRadius::same(2), ro.progress_track);
        let k = if motion(app) { (now % 1.6 / 1.6) as f32 } else { 0.23 };
        let from = (-0.3 + 1.3 * k).clamp(0.0, 1.0);
        let to = (-0.3 + 1.3 * k + 0.3).clamp(0.0, 1.0);
        if to > from {
            p.rect_filled(Rect::from_min_max(pos2(x + inner * from, y), pos2(x + inner * to, y + 4.0)), CornerRadius::same(2), ro.mark);
        }
        seen.theme.insert("progressFill", ro.mark);
        seen.theme.insert("progressTrack", ro.progress_track);
        y += 4.0 + 12.0;
    }
    y += 4.0;
    // The two buttons, 34 high, 8 apart: equal, side by side, or each the full width, stacked.
    let (select, undo) = if stacked {
        let select = Rect::from_min_size(pos2(x, y), vec2(inner, 34.0));
        (select, select.translate(vec2(0.0, 34.0 + 8.0)))
    } else {
        let select = Rect::from_min_size(pos2(x, y), vec2(side, 34.0));
        (select, select.translate(vec2(side + 8.0, 0.0)))
    };
    let label_rect = |r: Rect, l: &str| Rect::from_center_size(r.center(), vec2(label_w(l), 13.0));
    let rect = |r: Rect| json!([r.min.x, r.min.y, r.max.x, r.max.y]);
    let (sel_label, undo_label) = (label_rect(select, tr("Select layers")), label_rect(undo, tr("Undo run")));
    let mut asked = None;
    if card_button(ui, ro, select, tr("Select layers"), None).clicked() {
        asked = Some(("chat.select-run-layers", json!({})));
    }
    // Undo run waits for the run to end (the engine refuses it too): the U3 way.
    let why = working.then(|| tr("Stop the run first"));
    if card_button(ui, ro, undo, tr("Undo run"), why).clicked() {
        asked = Some(("chat.undo-run", json!({})));
    }
    seen.theme.insert("card", ro.card);
    seen.card = Some(json!({
        "heading": heading,
        "note": note,
        "earlier": earlier,
        "rows": listed,
        "progress": working,
        "buttons": [tr("Select layers"), tr("Undo run")],
        "undoEnabled": !working,
        "stacked": stacked,
        "buttonRects": [rect(select), rect(undo)],
        "labelRects": [rect(sel_label), rect(undo_label)],
        "rect": [r.min.x, r.min.y, r.max.x, r.max.y],
    }));
    if working {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }
    asked
}

/// A card button: 1 px edge, the label in the text colour; with `why`, it can't be used now and
/// says why (dimmed, not focusable).
fn card_button(ui: &mut Ui, ro: &Roles, r: Rect, label: &str, why: Option<&str>) -> Response {
    let t = Tokens::get(ui.ctx());
    let sense = if why.is_some() { Sense::hover() } else { Sense::click() };
    let resp = ui.interact(r, ui.id().with(("ai_card_button", label)), sense).on_hover_text(why.unwrap_or(label));
    crate::a11y::button(&resp, label);
    if why.is_some() {
        crate::a11y::disabled(&resp);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(10), t.hover);
    }
    ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, ro.card_button_edge), egui::StrokeKind::Inside);
    if resp.has_focus() {
        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    let color = if why.is_some() { ro.new_chat_disabled } else { ro.text };
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, regular(13.0), color);
    resp
}
