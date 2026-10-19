//! The AI tab, v1 (`docs/design/panel-redesign.md`, B3): the agents already working on the canvas,
//! not a chat. The connection and the command that connects an agent; the activity feed (Port 3's
//! change log), where an entry selects its layers and an agent's latest change can be undone; the
//! design lint's findings (Port 6) grouped by rule with counts; and locking the selection, which
//! agents can't change. Every row and button runs a command (`ai.*`, `object.toggleLock`), and
//! everything shown is what those commands return to an agent too. No chat, keys or providers.

use egui::{Align2, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use figcraft_engine::cmd::ai;
use serde_json::{Value, json};

use crate::i18n::{tr, trf, trn};
use crate::right::row;
use crate::theme::{Tokens, regular, semibold};
use crate::widgets::{self, PAD};
use crate::{FigApp, a11y};

/// How many of the newest changes the feed lists.
const FEED: usize = 30;
/// How many findings of a rule show when it's opened.
const PER_RULE: usize = 8;

pub fn tab(app: &mut FigApp, ui: &mut Ui) {
    connection(app, ui);
    widgets::separator(ui);
    activity(app, ui);
    widgets::separator(ui);
    findings(app, ui);
    widgets::separator(ui);
    lock(app, ui);
}

/// A read-only command's answer, for what the tab shows (an error shows as nothing). Its function
/// is called directly, not through `Session::execute`: the tab reads every frame, and `execute`
/// would end a live text preview and log each read in the command journal. The answer is the same
/// one an agent gets.
fn read(app: &mut FigApp, cmd: &str, p: Value) -> Value {
    figcraft_engine::find_command(cmd).and_then(|c| (c.run)(&mut app.session, &p).ok()).unwrap_or(Value::Null)
}

fn connection(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let copy = widgets::section_header(ui, tr("Connection"), &[(crate::icons::Icon::Export, tr("Copy the connect command"))], false).is_some();
    let status = read(app, "ai.status", json!({}));
    let text = match status["port"].as_u64() {
        Some(port) => trf("Agents can work in this window: its control channel listens on 127.0.0.1:{port}.", &[("port", &port)]),
        None => tr("This window has no control channel, so agents work on files headless. To let one work here, start Scrim with --control PORT.")
            .to_string(),
    };
    widgets::note(ui, &text, t.text_secondary);
    let command = status["connectCommand"].as_str().unwrap_or_default().to_string();
    egui::Frame::NONE
        .fill(t.field)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::same(8))
        .outer_margin(egui::Margin::symmetric(16, 0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(&command).monospace().color(t.text));
        });
    ui.add_space(4.0);
    let mut copied = copy;
    row(ui, 32.0, |ui| {
        copied |= widgets::button(ui, tr("Copy the connect command"), false, true).clicked();
    });
    if copied {
        app.run("ai.copyConnectCommand", json!({}));
    }
}

fn activity(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Activity"), &[], false);
    let Ok(ds) = app.session.ds() else { return };
    let since = ds.revision.saturating_sub(FEED as u64 * 4);
    let feed = read(app, "history.changes", json!({"since": since}));
    let changes: Vec<Value> = feed["changes"].as_array().map(|c| c.iter().rev().take(FEED).cloned().collect()).unwrap_or_default();
    if changes.is_empty() {
        widgets::note(ui, tr("No changes yet. Edits by you and by agents appear here, newest first."), t.text_secondary);
        return;
    }
    let w = ui.available_width();
    for c in changes {
        let revision = c["revision"].as_u64().unwrap_or_default();
        let agent = c["agent"] == true;
        let who = if agent { c["author"].as_str().unwrap_or_default().replacen("agent", tr("Agent"), 1) } else { tr("You").to_string() };
        let label = c["label"].as_str().unwrap_or_default();
        let layers: Vec<&str> = c["layers"].as_array().into_iter().flatten().filter_map(|l| l["name"].as_str()).take(3).collect();
        let (r, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click());
        let undo_r = Rect::from_min_size(pos2(r.max.x - PAD - 44.0, r.center().y - 11.0), vec2(44.0, 22.0));
        let can_undo = agent && ai::can_undo(&app.session, revision);
        let name = format!("{who}: {label}");
        let resp = resp.on_hover_text(tr("Select the layers this change touched"));
        a11y::button(&resp, &name);
        if resp.hovered() {
            ui.painter().rect_filled(r.shrink2(vec2(8.0, 2.0)), CornerRadius::same(5), t.hover);
        }
        let text_r = Rect::from_min_max(pos2(r.min.x + PAD, r.min.y), pos2(if agent { undo_r.min.x - 6.0 } else { r.max.x - PAD }, r.max.y));
        let painter = ui.painter().with_clip_rect(text_r);
        painter.text(pos2(text_r.min.x, r.min.y + 13.0), Align2::LEFT_CENTER, &name, semibold(11.0), if agent { t.accent } else { t.text });
        painter.text(pos2(text_r.min.x, r.min.y + 28.0), Align2::LEFT_CENTER, layers.join(", "), regular(11.0), t.text_secondary);
        if agent {
            let uresp = ui.interact(undo_r, Id::new(("ai_undo", revision)), if can_undo { Sense::click() } else { Sense::hover() });
            let tip = if can_undo {
                trf("Undo {value}", &[("value", &label)])
            } else {
                tr("Later edits came after this change: undo them first").to_string()
            };
            let uresp = uresp.on_hover_text(&tip);
            a11y::button(&uresp, &trf("Undo {value}", &[("value", &label)]));
            if can_undo && uresp.hovered() {
                ui.painter().rect_filled(undo_r, CornerRadius::same(5), t.pressed);
            }
            ui.painter().rect_stroke(undo_r, CornerRadius::same(5), egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            ui.painter().text(undo_r.center(), Align2::CENTER_CENTER, tr("Undo"), regular(11.0), if can_undo { t.text } else { t.text_tertiary });
            if can_undo && uresp.clicked() {
                app.run("ai.undoChange", json!({"revision": revision}));
                continue;
            }
        }
        if resp.clicked() {
            app.run("ai.selectChange", json!({"revision": revision}));
        }
    }
}

/// The lint findings, computed again only when the document changes.
fn cached_findings(app: &mut FigApp, ui: &Ui) -> Value {
    let key = app.session.ds().map(|d| (d.uid, d.revision)).ok();
    let id = Id::new("ai_findings");
    if let Some((k, v)) = ui.data(|d| d.get_temp::<(Option<(u64, u64)>, Value)>(id))
        && k == key
    {
        return v;
    }
    let v = read(app, "ai.findings", json!({}));
    ui.data_mut(|d| d.insert_temp(id, (key, v.clone())));
    v
}

fn findings(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Design lint"), &[], false);
    let all = cached_findings(app, ui);
    let groups = all["groups"].as_array().cloned().unwrap_or_default();
    if groups.is_empty() {
        widgets::note(ui, tr("No findings: the design lint has nothing to say about this file."), t.text_secondary);
        return;
    }
    let w = ui.available_width();
    for g in groups {
        let rule = g["rule"].as_str().unwrap_or_default().to_string();
        let count = g["count"].as_u64().unwrap_or_default();
        let open_id = Id::new(("ai_rule_open", &rule));
        let open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
        let name = g["name"].as_str().unwrap_or_default();
        let spoken = trn("{name}: {n} findings", count as usize, &[("name", &name)]);
        let resp = resp.on_hover_text(tr("Show the findings and select their layers"));
        a11y::button(&resp, &spoken);
        if resp.hovered() {
            ui.painter().rect_filled(r.shrink2(vec2(8.0, 0.0)), CornerRadius::same(5), t.hover);
        }
        crate::icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(r.min.x + PAD + 4.0, r.center().y), vec2(10.0, 10.0)),
            if open { crate::icons::Icon::Chevron } else { crate::icons::Icon::ChevronRight },
            t.icon_secondary,
        );
        let badge = count.to_string();
        let bw = 10.0 + 7.0 * badge.len() as f32;
        let br = Rect::from_min_size(pos2(r.max.x - PAD - bw, r.center().y - 9.0), vec2(bw, 18.0));
        ui.painter().rect_filled(br, CornerRadius::same(9), t.field);
        ui.painter().text(br.center(), Align2::CENTER_CENTER, &badge, semibold(11.0), t.text);
        let text_r = Rect::from_min_max(pos2(r.min.x + PAD + 14.0, r.min.y), pos2(br.min.x - 6.0, r.max.y));
        ui.painter().with_clip_rect(text_r).text(pos2(text_r.min.x, r.center().y), Align2::LEFT_CENTER, name, regular(11.0), t.text);
        if resp.clicked() {
            ui.data_mut(|d| d.insert_temp(open_id, !open));
            app.run("ai.selectFinding", json!({"rule": rule}));
        }
        if !open {
            continue;
        }
        for (i, f) in g["findings"].as_array().into_iter().flatten().take(PER_RULE).enumerate() {
            let (fr, fresp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
            let layer = f["name"].as_str().unwrap_or_default();
            let fresp = fresp.on_hover_text(f["message"].as_str().unwrap_or_default());
            a11y::button(&fresp, &trf("Select {name}", &[("name", &layer)]));
            if fresp.hovered() {
                ui.painter().rect_filled(fr.shrink2(vec2(8.0, 0.0)), CornerRadius::same(5), t.hover);
            }
            ui.painter().with_clip_rect(fr.shrink2(vec2(PAD, 0.0))).text(
                pos2(fr.min.x + PAD + 14.0, fr.center().y),
                Align2::LEFT_CENTER,
                layer,
                regular(11.0),
                t.text_secondary,
            );
            if fresp.clicked() {
                app.run("ai.selectFinding", json!({"rule": rule, "index": i}));
            }
        }
        if count as usize > PER_RULE {
            widgets::note(ui, &trn("and {n} more: the rule's row selects them all", count as usize - PER_RULE, &[]), t.text_tertiary);
        }
    }
}

fn lock(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Agents and the selection"), &[], false);
    let (sel, locked) = match app.session.ds() {
        Ok(ds) => (ds.selection.clone(), !ds.selection.is_empty() && ds.selection.iter().all(|id| ds.doc.node(*id).is_some_and(|n| n.locked))),
        Err(_) => (Vec::new(), false),
    };
    widgets::note(ui, tr("Agents can't change a locked layer or anything inside it. Your own edits aren't affected."), t.text_secondary);
    row(ui, 32.0, |ui| {
        if widgets::checkbox(ui, locked, tr("Keep agents off the selection")) && !sel.is_empty() {
            app.run("object.toggleLock", json!({"ids": sel.iter().map(|i| i.0).collect::<Vec<_>>(), "locked": !locked}));
        }
    });
    if sel.is_empty() {
        widgets::note(ui, tr("Select layers to lock them."), t.text_tertiary);
    }
}

#[cfg(test)]
#[path = "ai_tab_tests.rs"]
mod tests;
