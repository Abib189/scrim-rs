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

/// Claude Code's status and the connect flow (Settings ▸ AI & agents shows it too).
pub(crate) fn connection(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Claude Code"), &[], false);
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
    let st = figcraft_engine::cmd::mcp::status(&app.session, now);
    let state = st["state"].as_str().unwrap_or("stopped");
    let (label, color) = match state {
        "connected" => (tr("Connected"), t.success),
        "not connected" => (tr("Not connected"), t.text_secondary),
        _ => (tr("Server stopped"), t.danger),
    };
    crate::right::row(ui, 28.0, |ui| {
        let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, color);
        let client = st["client"]["name"].as_str().map(|n| format!(" · {n}")).unwrap_or_default();
        let resp = ui.label(egui::RichText::new(format!("{label}{client}")).color(t.text).font(regular(12.0)));
        a11y::status(&resp, label, &trf("The MCP endpoint is {state}", &[("state", &label)]));
        if state == "stopped" && widgets::button(ui, tr("Restart"), false, true).clicked() {
            app.run("mcp.restart-server", json!({}));
        }
    });
    crate::mcp_registration::row(app, ui);
    if app.ui.connect.open {
        crate::connect_flow::panel(app, ui);
    } else if state != "connected" {
        widgets::note(
            ui,
            tr("Let Claude Code see and edit this canvas: connect it once, and every Claude Code session can use Scrim."),
            t.text_secondary,
        );
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Connect Claude Code"), true, true).clicked() {
                app.run("mcp.connect-claude-code", json!({}));
            }
        });
    }
    crate::desktop_flow::section(app, ui);
}

fn activity(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::section_header(ui, tr("Activity"), &[], false);
    if app.session.ds().is_err() {
        return;
    }
    let feed = read(app, "history.changes", json!({"last": FEED}));
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
    for g in groups {
        let rule = g["rule"].as_str().unwrap_or_default().to_string();
        let count = g["count"].as_u64().unwrap_or_default();
        let name = g["name"].as_str().unwrap_or_default();
        let open_id = Id::new(("ai_rule_open", &rule));
        let open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        let spoken = trn("{name}: {n} findings", count as usize, &[("name", &name)]);
        if group_row(ui, &t, name, &spoken, count, open, 0.0).clicked() {
            ui.data_mut(|d| d.insert_temp(open_id, !open));
            app.run("ai.selectFinding", json!({"rule": rule}));
        }
        if !open {
            continue;
        }
        // C1.7: "Overused font" by family, one row each with its layer count, then its layers.
        if let Some(families) = g["families"].as_array() {
            for f in families {
                let family = f["family"].as_str().unwrap_or_default().to_string();
                let fname = f["name"].as_str().unwrap_or_default();
                let fcount = f["count"].as_u64().unwrap_or_default();
                let fopen_id = Id::new(("ai_family_open", &rule, &family));
                let fopen = ui.data(|d| d.get_temp::<bool>(fopen_id)).unwrap_or(false);
                let spoken = trn("{name}: {n} layers", fcount as usize, &[("name", &fname)]);
                if group_row(ui, &t, fname, &spoken, fcount, fopen, 14.0).clicked() {
                    ui.data_mut(|d| d.insert_temp(fopen_id, !fopen));
                    app.run("ai.selectFinding", json!({"rule": rule, "family": family}));
                }
                if fopen {
                    let list = f["findings"].as_array().cloned().unwrap_or_default();
                    layer_rows(app, ui, &t, json!({"rule": rule, "family": family}), &list, 28.0);
                }
            }
            continue;
        }
        let list = g["findings"].as_array().cloned().unwrap_or_default();
        layer_rows(app, ui, &t, json!({"rule": rule}), &list, 14.0);
    }
}

/// A rule's (or a family's) row: a chevron, its name and its count; `indent` from the left.
fn group_row(ui: &mut Ui, t: &Tokens, name: &str, spoken: &str, count: u64, open: bool, indent: f32) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    let resp = resp.on_hover_text(tr("Show the findings and select their layers"));
    a11y::button(&resp, spoken);
    if resp.hovered() {
        ui.painter().rect_filled(r.shrink2(vec2(8.0, 0.0)), CornerRadius::same(5), t.hover);
    }
    crate::icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(r.min.x + PAD + 4.0 + indent, r.center().y), vec2(10.0, 10.0)),
        if open { crate::icons::Icon::Chevron } else { crate::icons::Icon::ChevronRight },
        t.icon_secondary,
    );
    let badge = count.to_string();
    let bw = 10.0 + 7.0 * badge.len() as f32;
    let br = Rect::from_min_size(pos2(r.max.x - PAD - bw, r.center().y - 9.0), vec2(bw, 18.0));
    ui.painter().rect_filled(br, CornerRadius::same(9), t.field);
    ui.painter().text(br.center(), Align2::CENTER_CENTER, &badge, semibold(11.0), t.text);
    let text_r = Rect::from_min_max(pos2(r.min.x + PAD + 14.0 + indent, r.min.y), pos2(br.min.x - 6.0, r.max.y));
    ui.painter().with_clip_rect(text_r).text(pos2(text_r.min.x, r.center().y), Align2::LEFT_CENTER, name, regular(11.0), t.text);
    resp
}

/// The first [`PER_RULE`] findings' layers; a row selects its layer (`select` plus its `index`).
fn layer_rows(app: &mut FigApp, ui: &mut Ui, t: &Tokens, select: Value, list: &[Value], indent: f32) {
    let w = ui.available_width();
    for (i, f) in list.iter().take(PER_RULE).enumerate() {
        let (fr, fresp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
        let layer = f["name"].as_str().unwrap_or_default();
        let fresp = fresp.on_hover_text(f["message"].as_str().unwrap_or_default());
        a11y::button(&fresp, &trf("Select {name}", &[("name", &layer)]));
        if fresp.hovered() {
            ui.painter().rect_filled(fr.shrink2(vec2(8.0, 0.0)), CornerRadius::same(5), t.hover);
        }
        ui.painter().with_clip_rect(fr.shrink2(vec2(PAD, 0.0))).text(
            pos2(fr.min.x + PAD + indent, fr.center().y),
            Align2::LEFT_CENTER,
            layer,
            regular(11.0),
            t.text_secondary,
        );
        if fresp.clicked() {
            let mut p = select.clone();
            p["index"] = json!(i);
            app.run("ai.selectFinding", p);
        }
    }
    if list.len() > PER_RULE {
        widgets::note(ui, &trn("and {n} more: the rule's row selects them all", list.len() - PER_RULE, &[]), t.text_tertiary);
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
