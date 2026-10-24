//! The run's card as a summary (Fix AB2, `docs/design/ai-panel.md`; the design's A2 to A5), top to
//! bottom:
//! - **The header row:** the state (SemiBold 13): "Working", "Done · 1:47", "Undone · 17:52"; at its
//!   right (12, text-3) the time while it works, "Undo is one step" while Undo run is live, "Redo is
//!   one step" while Redo run is, nothing otherwise (Fix AA2's rules).
//! - **The summary row:** a 10 px dot (the accent while working, success when done, text-3 when
//!   undone) and SemiBold 14: "18 layers so far" while working, "43 layers changed" after.
//! - **The detail line** (12, text-2): "Created 2 frames and 36 layers, edited 4, deleted 1.", or
//!   while working "Now: {the newest step}". From the run's recorded edits (`chat_run::counts`),
//!   never from comparing documents.
//! - **While working:** a 4 px indeterminate bar (raised track, the accent's mark).
//! - **The Steps row** between hairlines: "Steps" and the count ("5", "3 of 4" while working: the
//!   newest is under way and the agent has no plan to count ahead), a chevron at the right. Closed
//!   at first; `chat.toggle-run-steps` opens the checklist (the chevron turns from right to up, as in the design's A4), per run, not
//!   kept.
//! - **The actions row** once the run has ended: Select layers and Undo run, 36 high, equal; after
//!   Undo run, Select layers off and Redo run. Stacked below 300 px (Fix Y2).
//!
//! Undone, the summary and detail rows are drawn at half strength. Every colour is a role.

use egui::{Align2, CornerRadius, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use super::run_card::{LABEL_PAD, STACK_BELOW, clock, elide, motion, revision, span};
use crate::FigApp;
use crate::follow_agent::chat_ui::LastRun;
use crate::i18n::{tr, trf, trn};
use crate::panel_view::ai_tab::panel::Seen;
use crate::panel_view::ai_tab::panel_theme::Roles;
use crate::theme::{Tokens, regular, semibold};

/// How many steps the checklist lists (the newest), and a step's row.
const STEPS: usize = 5;
const STEP_ROW: f32 = 26.0;
/// The Steps row, the buttons, and the card's inside edge.
const STEPS_H: f32 = 36.0;
const BUTTON_H: f32 = 36.0;
const PAD: f32 = 16.0;
/// How strong the undone summary and detail are.
const UNDONE_ALPHA: f32 = 0.5;

/// "Created 2 frames and 36 layers, edited 4, deleted 1.": the parts that aren't zero, `None` when
/// the run changed nothing.
pub fn detail(c: &figcraft_engine::cmd::chat_run::RunCounts) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let frames = trn("{n} frames", c.frames, &[("n", &c.frames)]);
    let layers = trn("{n} layers", c.layers, &[("n", &c.layers)]);
    match (c.frames, c.layers) {
        (0, 0) => {}
        (_, 0) => parts.push(trf("Created {what}", &[("what", &frames)])),
        (0, _) => parts.push(trf("Created {what}", &[("what", &layers)])),
        _ => parts.push(trf("Created {frames} and {layers}", &[("frames", &frames), ("layers", &layers)])),
    }
    // Each part's own catalogue entry: capitalised when it opens the line.
    if c.edited > 0 {
        let n = c.edited;
        parts.push(if parts.is_empty() { trn("Edited {n}", n, &[("n", &n)]) } else { trn("edited {n}", n, &[("n", &n)]) });
    }
    if c.deleted > 0 {
        let n = c.deleted;
        parts.push(if parts.is_empty() { trn("Deleted {n}", n, &[("n", &n)]) } else { trn("deleted {n}", n, &[("n", &n)]) });
    }
    (!parts.is_empty()).then(|| trf("{parts}.", &[("parts", &parts.join(tr(", ")))]))
}

/// The run's card. Returns the command a button or the Steps row asked for.
pub fn card(app: &FigApp, ui: &mut Ui, ro: &Roles, run: &LastRun, seen: &mut Seen) -> Option<(&'static str, Value)> {
    let now = ui.ctx().input(|i| i.time);
    let working = app.ui.agent_view.chat.running.is_some() && run.ended.is_none();
    let secs = run.ended.map_or_else(|| now - run.started, |(s, _)| s);
    // From the history's position (Fix AA2): ⌘Z of Undo run brings the card back to Done, ⌘⇧Z to
    // Undone, and any other change turns both buttons off.
    let st = figcraft_engine::cmd::chat_run::state(&app.session);
    let (undone, redo_ok, undo_ok) = (st.undone, st.redo_ok, st.undo_ok);
    let stopped = matches!(run.ended, Some((_, "stopped")));
    let heading = match run.ended {
        None => tr("Working").to_string(),
        Some(_) if undone => trf("Undone · {time}", &[("time", &span(secs))]),
        Some(_) if stopped => trf("Stopped · {time}", &[("time", &span(secs))]),
        Some(_) => trf("Done · {time}", &[("time", &span(secs))]),
    };
    let note = if working {
        clock(secs)
    } else if undone && redo_ok {
        tr("Redo is one step").to_string()
    } else if !undone && undo_ok {
        tr("Undo is one step").to_string()
    } else {
        String::new()
    };
    let counts = figcraft_engine::cmd::chat_run::counts(&app.session);
    let total = counts.total();
    let summary = if working { trn("{n} layers so far", total, &[("n", &total)]) } else { trn("{n} layers changed", total, &[("n", &total)]) };
    let all = figcraft_engine::cmd::chat_run::rows(&app.session);
    let step_text = |(label, n): &(String, usize)| if *n > 1 { format!("{label} ×{n}") } else { label.clone() };
    let detail_text = if working { all.last().map(|s| trf("Now: {step}", &[("step", &step_text(s))])) } else { detail(&counts) };
    let steps_count =
        if working && !all.is_empty() { trf("{done} of {all}", &[("done", &(all.len() - 1)), ("all", &all.len())]) } else { all.len().to_string() };
    let open = app.ui.agent_view.chat.steps_open;
    let earlier = all.len().saturating_sub(STEPS);
    let listed: Vec<&(String, usize)> = all.iter().skip(earlier).collect();

    let width = ui.available_width() - 32.0;
    let inner = width - 2.0 * PAD;
    let p = ui.painter().clone();
    let detail_galley = detail_text.as_ref().map(|d| {
        let mut job = egui::text::LayoutJob::single_section(
            d.clone(),
            // Coloured when painted, so it can be dimmed once undone.
            egui::TextFormat { font_id: regular(12.0), color: egui::Color32::PLACEHOLDER, line_height: Some(12.0 * 1.45), ..Default::default() },
        );
        job.wrap.max_width = inner;
        p.layout_job(job)
    });
    let label_w = |l: &str| p.layout_no_wrap(l.to_string(), regular(13.0), ro.text).size().x;
    let second = if undone { tr("Redo run") } else { tr("Undo run") };
    let side = (inner - 8.0) / 2.0;
    let stacked = app.ui.right_width < STACK_BELOW || label_w(tr("Select layers")).max(label_w(second)) + 2.0 * LABEL_PAD > side;
    let actions = !working;
    let buttons_h = if !actions {
        0.0
    } else if stacked {
        12.0 + BUTTON_H * 2.0 + 8.0
    } else {
        12.0 + BUTTON_H
    };
    // A refused press's words under the buttons, until the next action (Fix Z7).
    let rev = revision(app);
    let refused = app.ui.agent_view.chat.refused.as_ref().filter(|(_, at)| *at == rev).map(|(w, _)| w.clone());
    let refused_galley = refused.as_ref().map(|w| {
        let mut job = egui::text::LayoutJob::single_section(
            w.clone(),
            egui::TextFormat { font_id: regular(12.0), color: ro.text_secondary, line_height: Some(12.0 * 1.4), ..Default::default() },
        );
        job.wrap.max_width = inner;
        p.layout_job(job)
    });
    let detail_h = detail_galley.as_ref().map_or(0.0, |g| 8.0 + g.size().y);
    let bar_h = if working { 12.0 + 4.0 } else { 0.0 };
    let list_h = if open { listed.len() as f32 * STEP_ROW + if earlier > 0 { STEP_ROW } else { 0.0 } + 8.0 } else { 0.0 };
    let refused_h = refused_galley.as_ref().map_or(0.0, |g| 8.0 + g.size().y);
    // 16, the header (18), 12, the summary (20), the detail, the bar, 12, the Steps row between
    // hairlines, its list, the buttons, the refusal, 16.
    let height = PAD + 18.0 + 12.0 + 20.0 + detail_h + bar_h + 12.0 + STEPS_H + list_h + buttons_h + refused_h + PAD;
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let r = Rect::from_min_size(pos2(row.min.x + 16.0, row.min.y), vec2(width, height));
    p.rect_filled(r, CornerRadius::same(12), ro.card);
    p.rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, ro.card_edge), egui::StrokeKind::Inside);
    seen.theme.insert("card", ro.card);
    let x = r.min.x + PAD;
    let mut y = r.min.y + PAD;

    // The header row.
    p.text(pos2(x, y + 9.0), Align2::LEFT_CENTER, &heading, semibold(13.0), ro.text);
    p.text(pos2(x + inner, y + 9.0), Align2::RIGHT_CENTER, &note, regular(12.0), ro.text_3);
    seen.fonts.insert("cardHeading", semibold(13.0));
    seen.fonts.insert("cardNote", regular(12.0));
    seen.theme.insert("cardNote", ro.text_3);
    y += 18.0 + 12.0;

    // The summary row, then the detail line (half strength once undone).
    let dim = |c: egui::Color32| if undone { c.gamma_multiply(UNDONE_ALPHA) } else { c };
    let dot = if working {
        ro.mark
    } else if undone || stopped {
        ro.text_3
    } else {
        ro.success
    };
    p.circle_filled(pos2(x + 5.0, y + 10.0), 5.0, dim(dot));
    p.text(pos2(x + 18.0, y + 10.0), Align2::LEFT_CENTER, &summary, semibold(14.0), dim(ro.text));
    seen.fonts.insert("cardSummary", semibold(14.0));
    seen.theme.insert("cardDot", dot);
    y += 20.0;
    if let Some(g) = detail_galley {
        y += 8.0;
        let size = g.size();
        p.galley(pos2(x, y), g, dim(ro.text_secondary));
        seen.theme.insert("cardDetail", dim(ro.text_secondary));
        seen.fonts.insert("cardDetail", regular(12.0));
        y += size.y;
    }

    // The bar, while it works: indeterminate, at the banner's pace (still with reduced motion).
    if working {
        y += 12.0;
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
        y += 4.0;
    }

    // The Steps row, between hairlines.
    y += 12.0;
    let hair = Stroke::new(1.0, ro.header_rule);
    p.line_segment([pos2(x, y), pos2(x + inner, y)], hair);
    let steps_r = Rect::from_min_size(pos2(x, y), vec2(inner, STEPS_H));
    let mut asked = None;
    let sresp = ui.interact(steps_r, ui.id().with("ai_card_steps"), Sense::click()).on_hover_text(tr("Show or hide the run's steps"));
    crate::a11y::menu_button(&sresp, &trf("Steps, {count}", &[("count", &steps_count)]), open);
    if sresp.hovered() {
        p.rect_filled(steps_r, CornerRadius::same(4), Tokens::get(ui.ctx()).hover);
    }
    let steps_label = p.layout_no_wrap(tr("Steps").to_string(), regular(13.0), ro.text_secondary);
    let steps_w = steps_label.size().x;
    p.galley(pos2(x, steps_r.center().y - steps_label.size().y / 2.0), steps_label, ro.text_secondary);
    p.text(pos2(x + steps_w + 8.0, steps_r.center().y), Align2::LEFT_CENTER, &steps_count, regular(12.0), ro.text_3);
    chevron(&p, pos2(x + inner - 6.0, steps_r.center().y), open, ro.chevron);
    if sresp.clicked() {
        asked = Some(("chat.toggle-run-steps", json!({})));
    }
    y += STEPS_H;
    p.line_segment([pos2(x, y), pos2(x + inner, y)], hair);
    let mut steps = Vec::new();
    for (i, s) in listed.iter().enumerate() {
        let text = step_text(s);
        let active = working && i + 1 == listed.len();
        let font = if active { semibold(13.0) } else { regular(13.0) };
        let shown = elide(&p, &text, font, inner - 26.0);
        steps.push(json!({"label": text, "shown": shown, "state": if undone { "undone" } else if active { "active" } else { "done" }}));
    }
    if open {
        y += 4.0;
        if earlier > 0 {
            p.text(pos2(x, y + STEP_ROW / 2.0), Align2::LEFT_CENTER, trf("{n} earlier", &[("n", &earlier)]), regular(12.0), ro.text_3);
            y += STEP_ROW;
        }
        for (i, s) in listed.iter().enumerate() {
            let c = pos2(x + 8.0, y + STEP_ROW / 2.0);
            let active = working && i + 1 == listed.len();
            if active {
                p.circle_stroke(c, 7.0, Stroke::new(2.0, ro.mark));
                p.circle_filled(c, 3.0, ro.mark);
                seen.theme.insert("activeRing", ro.mark);
            } else {
                p.circle_filled(c, 8.0, dim(ro.success));
                p.add(egui::Shape::line(vec![c + vec2(-3.5, 0.2), c + vec2(-1.0, 2.8), c + vec2(3.8, -2.6)], Stroke::new(1.6, ro.done_check)));
                seen.theme.insert("doneCircle", ro.success);
                seen.theme.insert("doneCheck", ro.done_check);
            }
            let font = if active { semibold(13.0) } else { regular(13.0) };
            let shown = elide(&p, &step_text(s), font.clone(), inner - 26.0);
            p.text(pos2(x + 26.0, c.y), Align2::LEFT_CENTER, &shown, font, ro.text_secondary);
            seen.fonts.insert(if active { "activeRow" } else { "row" }, if active { semibold(13.0) } else { regular(13.0) });
            y += STEP_ROW;
        }
        y += 4.0;
    }

    // The actions, once it has ended.
    let rect = |r: Rect| json!([r.min.x, r.min.y, r.max.x, r.max.y]);
    let sel_why = undone.then(|| tr("Those layers were removed by Undo"));
    let (second_cmd, why) = if undone {
        ("chat.redo-run", (!redo_ok).then(|| tr("Something changed since: Redo is no longer one step")))
    } else {
        ("chat.undo-run", (!undo_ok).then(|| tr("Something changed since the run: Undo run is no longer one step")))
    };
    let mut buttons = Vec::new();
    let mut button_rects = Vec::new();
    let mut label_rects = Vec::new();
    if actions {
        y += 12.0;
        let (select, other) = if stacked {
            let select = Rect::from_min_size(pos2(x, y), vec2(inner, BUTTON_H));
            (select, select.translate(vec2(0.0, BUTTON_H + 8.0)))
        } else {
            let select = Rect::from_min_size(pos2(x, y), vec2(side, BUTTON_H));
            (select, select.translate(vec2(side + 8.0, 0.0)))
        };
        if card_button(ui, ro, select, tr("Select layers"), sel_why).clicked() {
            asked = Some(("chat.select-run-layers", json!({})));
        }
        if card_button(ui, ro, other, second, why).clicked() {
            asked = Some((second_cmd, json!({})));
        }
        seen.fonts.insert("cardButton", regular(13.0));
        if sel_why.is_some() || why.is_some() {
            seen.theme.insert("cardDisabled", ro.new_chat_disabled);
        }
        let button = |label: &str, why: Option<&str>| json!({"label": label, "enabled": why.is_none(), "tooltip": why.unwrap_or(label)});
        buttons = vec![button(tr("Select layers"), sel_why), button(second, why)];
        let label_rect = |r: Rect, l: &str| Rect::from_center_size(r.center(), vec2(label_w(l), 13.0));
        button_rects = vec![rect(select), rect(other)];
        label_rects = vec![rect(label_rect(select, tr("Select layers"))), rect(label_rect(other, second))];
        if let Some(g) = refused_galley {
            p.galley(pos2(x, other.max.y + 8.0), g, ro.text_secondary);
        }
    }
    seen.card = Some(json!({
        "heading": heading,
        "note": (!note.is_empty()).then_some(note),
        "summary": summary,
        "detail": detail_text,
        "steps": {"count": steps_count, "open": open, "chevron": if open { "up" } else { "right" }, "rect": rect(steps_r)},
        "earlier": earlier,
        "rows": steps,
        "progress": working,
        "state": if working { "working" } else if undone { "undone" } else if stopped { "stopped" } else { "done" },
        "buttons": buttons,
        "undoEnabled": !working && !undone && undo_ok,
        "refused": refused,
        "stacked": stacked,
        "buttonRects": button_rects,
        "labelRects": label_rects,
        "rect": rect(r),
    }));
    // The bar moves at the banner's pace (one repaint for both); with reduced motion only the time
    // changes, once a second.
    if working {
        let next = if motion(app) { 0.1 } else { 1.0 - secs.fract() + 0.01 };
        ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(next));
        if let Some(c) = seen.card.as_mut() {
            c["repaintIn"] = json!(next);
        }
    }
    asked
}

/// The Steps row's chevron: pointing right, or up when the list is open (the design's A4).
fn chevron(p: &egui::Painter, c: egui::Pos2, open: bool, color: egui::Color32) {
    let line = Stroke::new(1.5, color);
    let pts = if open {
        vec![c + vec2(-4.5, 2.5), c + vec2(0.0, -2.5), c + vec2(4.5, 2.5)]
    } else {
        vec![c + vec2(-2.5, -4.5), c + vec2(2.5, 0.0), c + vec2(-2.5, 4.5)]
    };
    p.add(egui::Shape::line(pts, line));
}

/// A card button: 1 px edge, the label in the text colour; with `why`, it can't be used now and
/// says why (dimmed, not focusable).
pub(crate) fn card_button(ui: &mut Ui, ro: &Roles, r: Rect, label: &str, why: Option<&str>) -> Response {
    let t = Tokens::get(ui.ctx());
    let sense = if why.is_some() { Sense::hover() } else { Sense::click() };
    let resp = ui.interact(r, ui.id().with(("ai_card_button", label)), sense).on_hover_text(why.unwrap_or(label));
    crate::a11y::button(&resp, label);
    if why.is_some() {
        crate::a11y::disabled(&resp);
    } else if resp.hovered() {
        ui.painter().rect_filled(r, CornerRadius::same(10), t.hover);
    }
    // Off: its edge and label dimmed (the design's A5).
    let edge = if why.is_some() { ro.card_edge } else { ro.card_button_edge };
    ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    if resp.has_focus() {
        ui.painter().rect_stroke(r, CornerRadius::same(10), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    let color = if why.is_some() { ro.new_chat_disabled } else { ro.text };
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, regular(13.0), color);
    resp
}

#[cfg(test)]
#[path = "ai_card_tests.rs"]
mod tests;
