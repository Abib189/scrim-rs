//! Settings ▸ Toolbar (C3b): the toolbar's groups and tools, in order. Each tool can be shown or
//! hidden (a hidden tool keeps its shortcut; Move is always shown), moved, dropped onto another to
//! share one button with a menu, or taken out of its group. The mouse drags; the keyboard has a
//! button for every move. Every change is a `toolbar.*` command; Reset to default is
//! `toolbar.reset`.

use egui::{Align2, CornerRadius, RichText, Sense, Stroke, Ui, pos2, vec2};
use figcraft_engine::ToolId;
use figcraft_engine::cmd::toolbar::{ALWAYS_SHOWN, layout};
use serde_json::{Value, json};

use crate::i18n::{tr, trf};
use crate::settings_dialog::heading;
use crate::theme::{Tokens, regular};
use crate::{FigApp, a11y, icons, widgets};

/// What a drag carries: the tool's id.
#[derive(Clone)]
struct Dragged(String);

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    heading(ui, tr("Toolbar"));
    widgets::label(
        ui,
        tr("Drag a tool onto another to put them in one button with a menu. A hidden tool keeps its shortcut; Move is always shown."),
        t.text_secondary,
    );
    ui.add_space(6.0);
    let is_default = app.session.prefs.settings.toolbar.is_empty();
    let reset = widgets::button(ui, tr("Reset to default"), false, !is_default);
    if is_default {
        a11y::disabled(&reset);
    }
    if reset.clicked() {
        app.run("toolbar.reset", json!({}));
    }
    ui.add_space(8.0);
    let l = layout(&app.session.prefs);
    let mut run: Option<(&str, Value)> = None;
    let n = l.len();
    for (gi, group) in l.iter().enumerate() {
        let frame = egui::Frame::NONE.stroke(Stroke::new(1.0, t.border)).corner_radius(CornerRadius::same(6)).inner_margin(egui::Margin::same(6));
        let box_resp = frame
            .show(ui, |ui| {
                for (ti, e) in group.iter().enumerate() {
                    let Some(tool) = ToolId::from_id(&e.tool) else { continue };
                    if let Some(r) = row(ui, &t, tool, e.on, (gi, ti), (n, group.len())) {
                        run = Some(r);
                    }
                }
            })
            .response;
        // Dropped on the group (not on a tool): the end of this group.
        if let Some(d) = box_resp.dnd_release_payload::<Dragged>() {
            run.get_or_insert(("toolbar.move", json!({"tool": d.0, "group": gi})));
        }
        ui.add_space(6.0);
    }
    // A drop zone for a button of its own, at the end.
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
    let hovering = resp.dnd_hover_payload::<Dragged>().is_some();
    ui.painter().rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, if hovering { t.accent } else { t.border }), egui::StrokeKind::Inside);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, tr("Drop here for a button of its own"), regular(11.0), t.text_secondary);
    if let Some(d) = resp.dnd_release_payload::<Dragged>() {
        run = Some(("toolbar.move", json!({"tool": d.0, "group": n})));
    }
    if let Some((cmd, p)) = run {
        app.run(cmd, p);
        app.services.save_prefs(&app.session.prefs);
    }
}

/// One tool's row: the handle to drag, its icon, name and shortcut, Show, and the keyboard's moves.
/// Returns the command to run, if any.
fn row(
    ui: &mut Ui,
    t: &Tokens,
    tool: ToolId,
    on: bool,
    (gi, ti): (usize, usize),
    (groups, in_group): (usize, usize),
) -> Option<(&'static str, Value)> {
    let id = tool.id();
    let name = tr(tool.label());
    let mut out = None;
    ui.horizontal(|ui| {
        // The handle: drag it onto another tool, a group, or the drop zone.
        let (h, _) = ui.allocate_exact_size(vec2(16.0, 24.0), Sense::hover());
        let hresp = ui.interact(h, ui.id().with(("tb_drag", id)), Sense::drag()).on_hover_text(tr("Drag to move or group"));
        a11y::named_button(&hresp, &trf("Drag {tool}", &[("tool", &name)]));
        hresp.dnd_set_drag_payload(Dragged(id.to_string()));
        for k in 0..3 {
            let y = h.center().y - 4.0 + 4.0 * k as f32;
            ui.painter().line_segment([pos2(h.min.x + 3.0, y), pos2(h.max.x - 3.0, y)], Stroke::new(1.2, t.icon_secondary));
        }
        let (ir, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
        icons::paint(ui.painter(), ir, crate::toolbar::icon_of(tool), if on { t.icon } else { t.text_tertiary });
        let label = ui.label(RichText::new(name).color(if on { t.text } else { t.text_secondary }).font(regular(12.0)));
        // Dropped onto this tool: the two share a button.
        let drop = ui.interact(label.rect.union(ir), ui.id().with(("tb_drop", id)), Sense::hover());
        if let Some(d) = drop.dnd_release_payload::<Dragged>()
            && d.0 != id
        {
            out = Some(("toolbar.group", json!({"tool": d.0, "with": id})));
        }
        if drop.dnd_hover_payload::<Dragged>().is_some_and(|d| d.0 != id) {
            ui.painter().rect_stroke(drop.rect.expand(3.0), CornerRadius::same(4), Stroke::new(1.5, t.accent), egui::StrokeKind::Outside);
        }
        ui.label(RichText::new(crate::toolbar::pretty_shortcut(tool.shortcut())).color(t.text_tertiary).font(regular(11.0)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let small = |ui: &mut Ui, text: &str, tip: String, enabled: bool| {
                let resp = widgets::button(ui, text, false, enabled).on_hover_text(&tip);
                a11y::button(&resp, &tip);
                // (`widgets::button` doesn't say so itself: screen readers learn it here.)
                if !enabled {
                    a11y::disabled(&resp);
                }
                resp.clicked() && enabled
            };
            // Every row has the same buttons (disabled where they don't apply), so they line up.
            if small(ui, tr("Own button"), trf("Make {tool} a button of its own", &[("tool", &name)]), in_group > 1) {
                out = Some(("toolbar.ungroup", json!({"tool": id})));
            }
            if small(ui, tr("Join above"), trf("Put {tool} in the group above", &[("tool", &name)]), ti == 0 && gi > 0) {
                out = Some(("toolbar.move", json!({"tool": id, "group": gi - 1})));
            }
            let last = gi + 1 == groups && ti + 1 == in_group;
            if small(ui, tr("Down"), trf("Move {tool} down", &[("tool", &name)]), !last) {
                let to = if ti + 1 < in_group {
                    json!({"tool": id, "group": gi, "index": ti + 1})
                } else {
                    json!({"tool": id, "group": gi + 1, "index": 0})
                };
                out = Some(("toolbar.move", to));
            }
            if small(ui, tr("Up"), trf("Move {tool} up", &[("tool", &name)]), gi > 0 || ti > 0) {
                let to = if ti > 0 { json!({"tool": id, "group": gi, "index": ti - 1}) } else { json!({"tool": id, "group": gi - 1}) };
                out = Some(("toolbar.move", to));
            }
            let locked = id == ALWAYS_SHOWN;
            let resp = ui.add_enabled(!locked, egui::Checkbox::new(&mut on.clone(), tr("Show")));
            a11y::checkbox(&resp, &trf("Show {tool}", &[("tool", &name)]), on);
            if locked {
                resp.on_disabled_hover_text(tr("Move is always shown"));
            } else if resp.clicked() {
                out = Some(("toolbar.toggle", json!({"tool": id, "on": !on})));
            }
        });
    });
    out
}

#[cfg(test)]
#[path = "settings_toolbar_tests.rs"]
mod tests;
