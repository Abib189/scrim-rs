//! The right panel's mode switch and tabs (`docs/design/panel-redesign.md`, B1): Design | Animate
//! beside the zoom menu, then the tabs Design, Prototype, Code and AI. Every switch runs a command
//! (`panel.mode.*`, `panel.tab.*`), so the person, the menus and agents switch them the same way.
//! Each window keeps its own mode and tab; the last chosen is a preference new windows open with.
//! Animate is a placeholder for a later stage: it draws a note and never touches the document.

use egui::{Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use figcraft_engine::Prefs;
use figcraft_engine::cmd::panel::{Mode, PanelView, Tab};
use serde_json::{Value, json};

use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::right::{menu_check, menu_item, menu_popup};
use crate::theme::{Tokens, regular, semibold};
use crate::{FigApp, RightTab, UiState, a11y, widgets};

#[path = "ai_tab.rs"]
pub(crate) mod ai_tab;

/// The tab a [`Tab`] draws as.
/// The right panel's scroll area. On the AI tab it doesn't scroll and keeps its own offset (Fix
/// AA6): the panel fills the height with its header, its own scrolling transcript and the composer
/// pinned below. Scrolled, it slid the header out of view (by the 80 px kept below the content, or
/// by a scrolled Design tab's offset), while `ui.inspect` still reported the header's rectangle.
pub fn scroll_area(app: &FigApp) -> egui::ScrollArea {
    let ai = app.ui.right_tab == RightTab::Ai;
    let bars = if ai { egui::scroll_area::ScrollBarVisibility::AlwaysHidden } else { egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded };
    let source = if ai { egui::scroll_area::ScrollSource::NONE } else { egui::scroll_area::ScrollSource::default() };
    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt(("right_panel", ai)).scroll_source(source).scroll_bar_visibility(bars)
}

fn right_tab(tab: Tab) -> RightTab {
    match tab {
        Tab::Design => RightTab::Design,
        Tab::Prototype => RightTab::Prototype,
        Tab::Code => RightTab::Code,
        Tab::Ai => RightTab::Ai,
    }
}

/// The right panel's name for screen readers: what it shows.
pub fn region_label(ui: &UiState) -> &'static str {
    match ui.right_tab {
        RightTab::Inspect => tr("Dev Mode"),
        _ if ui.panel_animate => tr("Animate"),
        RightTab::Design => tr("Design"),
        RightTab::Prototype => tr("Prototype"),
        RightTab::Code => tr("Code"),
        RightTab::Ai => tr("AI"),
    }
}

/// A new main window's UI state: the mode and tab chosen last.
pub fn restored(prefs: &Prefs) -> UiState {
    UiState { right_tab: right_tab(prefs.panel.tab), panel_animate: prefs.panel.mode == Mode::Animate, ..UiState::default() }
}

/// Show `tab` in Design mode, and keep it as the last choice.
fn show(app: &mut FigApp, tab: RightTab) {
    app.ui.right_tab = tab;
    app.ui.panel_animate = false;
    if let Some(tab) = Tab::ALL.into_iter().find(|t| right_tab(*t) == tab) {
        remember(app, PanelView { mode: Mode::Design, tab });
    }
}

fn remember(app: &mut FigApp, view: PanelView) {
    if app.session.prefs.panel != view {
        app.session.prefs.panel = view;
        app.services.save_prefs(&app.session.prefs);
    }
}

/// Act on a panel request from a command; `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str, data: &Value) -> bool {
    // "Describe a change" (R5): the AI tab's composer.
    if kind.starts_with("chat.") {
        return crate::follow_agent::chat_ui::request(app, kind, data);
    }
    match kind {
        figcraft_engine::cmd::panel::REQUEST => {
            let view: PanelView = serde_json::from_value(data.clone()).unwrap_or_default();
            app.ui.right_tab = right_tab(view.tab);
            app.ui.panel_animate = view.mode == Mode::Animate;
            app.services.save_prefs(&app.session.prefs);
        }
        "panel.design" => show(app, RightTab::Design),
        "panel.prototype" => show(app, RightTab::Prototype),
        "panel.designPrototype" => show(app, if app.ui.right_tab == RightTab::Prototype { RightTab::Design } else { RightTab::Prototype }),
        "panel.devMode" => {
            if app.ui.right_tab == RightTab::Inspect {
                show(app, RightTab::Design);
            } else {
                app.ui.right_tab = RightTab::Inspect;
                app.ui.panel_animate = false;
            }
        }
        _ => {
            return crate::code_tab::request(app, kind, data)
                || crate::dialogs::appearance_prefs::request(app, kind, data)
                || crate::mcp_ui::request(app, kind, data)
                || crate::settings_dialog::request(app, kind, data)
                || crate::settings_privacy::request(app, kind, data);
        }
    }
    true
}

/// The mode row (with the zoom menu), then the tabs in Design mode. `w`: the panel's width.
pub fn header(app: &mut FigApp, ui: &mut Ui, w: f32) {
    let t = Tokens::get(ui.ctx());
    let (row, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    ui.painter().line_segment([row.left_top(), row.right_top()], Stroke::new(1.0, t.border));
    mode_switch(app, ui, &t, row);
    zoom_menu(app, ui, &t, row);
    ui.painter().line_segment([row.left_bottom(), row.right_bottom()], Stroke::new(1.0, t.border));
    // The AI tab's composer, pinned at the panel's bottom (R5).
    crate::follow_agent::chat_ui::panel(app, ui);
    if app.ui.panel_animate && app.ui.right_tab != RightTab::Inspect {
        return;
    }
    let (tabs, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
    tab_row(app, ui, &t, tabs);
    ui.painter().line_segment([tabs.left_bottom(), tabs.right_bottom()], Stroke::new(1.0, t.border));
}

/// Design | Animate: two segments, each a toggle for screen readers.
fn mode_switch(app: &mut FigApp, ui: &mut Ui, t: &Tokens, row: Rect) {
    let animate = app.ui.panel_animate && app.ui.right_tab != RightTab::Inspect;
    let segments =
        [(tr("Design"), tr("Design mode"), "panel.mode.design", !animate), (tr("Animate"), tr("Animate mode"), "panel.mode.animate", animate)];
    let galleys: Vec<_> = segments.iter().map(|s| ui.painter().layout_no_wrap(s.0.into(), semibold(11.0), t.text)).collect();
    let width: f32 = galleys.iter().map(|g| g.size().x + 20.0).sum::<f32>() + 4.0;
    let pill = Rect::from_min_size(pos2(row.min.x + 8.0, row.center().y - 13.0), vec2(width, 26.0));
    ui.painter().rect_filled(pill, CornerRadius::same(6), t.field);
    let mut x = pill.min.x + 2.0;
    for ((_, name, cmd, on), galley) in segments.into_iter().zip(galleys) {
        let r = Rect::from_min_size(pos2(x, pill.min.y + 2.0), vec2(galley.size().x + 20.0, 22.0));
        let resp = ui.interact(r, ui.id().with(("panel_mode", cmd)), Sense::click()).on_hover_text(name);
        a11y::toggle(&resp, name, on);
        if on {
            ui.painter().rect_filled(r, CornerRadius::same(5), t.panel);
            ui.painter().rect_stroke(r, CornerRadius::same(5), Stroke::new(1.0, t.border), StrokeKind::Inside);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(5), t.hover);
        }
        ui.painter().galley(
            pos2(r.center().x - galley.size().x / 2.0, r.center().y - galley.size().y / 2.0),
            galley,
            if on { t.text } else { t.text_secondary },
        );
        if resp.clicked() && !on {
            app.run(cmd, json!({}));
        }
        x = r.max.x;
    }
}

/// Design, Prototype, Code and AI. The active tab is marked as Port 1 marks the selected layer: the
/// neutral fill and a 2 px accent edge. In Inspect (Dev Mode) none is.
fn tab_row(app: &mut FigApp, ui: &mut Ui, t: &Tokens, tabs: Rect) {
    let labels = [(Tab::Design, tr("Design")), (Tab::Prototype, tr("Prototype")), (Tab::Code, tr("Code")), (Tab::Ai, tr("AI"))];
    let galleys: Vec<_> = labels.iter().map(|(_, l)| ui.painter().layout_no_wrap((*l).into(), semibold(11.0), t.text)).collect();
    // Less padding when a language's labels don't fit; the last one is clipped after that.
    let text: f32 = galleys.iter().map(|g| g.size().x).sum();
    let pad = ((tabs.width() - 16.0 - text - 6.0) / 8.0).clamp(3.0, 8.0);
    let mut x = tabs.min.x + 8.0;
    for ((tab, label), galley) in labels.into_iter().zip(galleys) {
        let r = Rect::from_min_size(pos2(x, tabs.center().y - 12.0), vec2(galley.size().x + 2.0 * pad, 24.0)).intersect(tabs.shrink2(vec2(4.0, 0.0)));
        let resp = ui.interact(r, ui.id().with(("rtab", tab.id())), Sense::click()).on_hover_text(label);
        let active = app.ui.right_tab == right_tab(tab);
        a11y::tab(&resp, label, active);
        if active || resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(5), t.hover);
        }
        if active {
            let edge = Rect::from_min_max(pos2(r.min.x + pad, r.max.y + 3.0), pos2(r.max.x - pad, r.max.y + 5.0));
            ui.painter().rect_filled(edge, CornerRadius::same(1), t.accent);
        }
        ui.painter().with_clip_rect(r).galley(
            pos2(r.min.x + pad, r.center().y - galley.size().y / 2.0),
            galley,
            if active { t.text } else { t.text_secondary },
        );
        if resp.clicked() && !active {
            crate::proto_panel::open_editor(ui.ctx(), None);
            app.run(&format!("panel.tab.{}", tab.id()), json!({}));
        }
        x = r.max.x + 2.0;
    }
}

/// The Inspect tab: its own content, then the agents' review (Fix T4: the AI tab's until then).
pub fn inspect_tab(app: &mut FigApp, ui: &mut Ui, inspect: fn(&mut FigApp, &mut Ui)) {
    inspect(app, ui);
    ai_tab::review(app, ui);
}

/// What the panel shows instead of a tab's usual content: Animate's placeholder, and the Code and
/// AI tabs. `false`: draw the tab as before.
pub fn content(app: &mut FigApp, ui: &mut Ui) -> bool {
    let t = Tokens::get(ui.ctx());
    if app.ui.panel_animate && app.ui.right_tab != RightTab::Inspect {
        widgets::section_header(ui, tr("Animate"), &[], false);
        widgets::note(ui, tr("Animation comes in a later stage. This mode is a placeholder: nothing here changes your file."), t.text_secondary);
        ui.add_space(4.0);
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Back to Design"), false, true).clicked() {
                app.run("panel.mode.design", json!({}));
            }
        });
        return true;
    }
    match app.ui.right_tab {
        RightTab::Code => crate::code_tab::tab(app, ui),
        RightTab::Ai => ai_tab::tab(app, ui),
        _ => return false,
    }
    true
}

/// The zoom menu at the right of the mode row (its items are the `view.zoom*` commands and the view
/// toggles).
fn zoom_menu(app: &mut FigApp, ui: &mut Ui, t: &Tokens, row: Rect) {
    let zoom = app.session.ds().map(|d| d.view.zoom).unwrap_or(1.0);
    let zr = Rect::from_min_size(pos2(row.max.x - 64.0, row.center().y - 12.0), vec2(56.0, 24.0));
    let zresp = ui.interact(zr, ui.id().with("zoommenu"), Sense::click());
    // The value is only spelled out while a screen reader reads the tree.
    let value = if a11y::active(&zresp.ctx) { format!("{}%", (zoom * 100.0).round()) } else { String::new() };
    a11y::combo(&zresp, tr("Zoom"), &value, a11y::menu_open(&zresp));
    if zresp.hovered() {
        ui.painter().rect_filled(zr, CornerRadius::same(5), t.hover);
    }
    // A menu that sets a value: an input's edge (Fix S2).
    ui.painter().rect_stroke(zr, CornerRadius::same(5), crate::theme::field_stroke(t, zresp.hovered(), false), egui::StrokeKind::Inside);
    ui.painter().text(pos2(zr.min.x + 6.0, zr.center().y), Align2::LEFT_CENTER, format!("{}%", (zoom * 100.0).round()), regular(11.0), t.text);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(zr.max.x - 9.0, zr.center().y), vec2(10.0, 10.0)), Icon::Chevron, t.icon_secondary);
    menu_popup(&zresp).width(200.0).show(|ui| {
        for (label, cmd) in [
            (tr("Zoom in"), "view.zoomIn"),
            (tr("Zoom out"), "view.zoomOut"),
            (tr("Zoom to fit"), "view.zoomFit"),
            (tr("Zoom to 50%"), "view.zoom50"),
            (tr("Zoom to 100%"), "view.zoom100"),
            (tr("Zoom to 200%"), "view.zoom200"),
            (tr("Zoom to selection"), "view.zoomSelection"),
        ] {
            if menu_item(ui, label, crate::shortcuts::shortcut(cmd).unwrap_or("")) {
                app.run(cmd, json!({}));
                ui.close();
            }
        }
        ui.separator();
        let Ok(v) = app.session.ds().map(|d| d.view.clone()) else { return };
        for (label, scale) in [(tr("Pixel preview 1x"), 1u8), (tr("Pixel preview 2x"), 2)] {
            let on = v.pixel_preview && v.pixel_scale == scale;
            if menu_check(ui, on, label, "") {
                app.run("view.pixelPreview", json!({"on": !on, "scale": scale}));
                ui.close();
            }
        }
        let toggles: [(&str, &str, bool); 5] = [
            (tr("Pixel grid"), "view.pixelGrid", v.pixel_grid),
            (tr("Snap to pixel grid"), "view.snapPixel", v.snap_pixel),
            (tr("Layout grids"), "view.layoutGrids", v.layout_grids),
            (tr("Rulers"), "view.rulers", v.rulers),
            (tr("Outlines"), "view.outlines", v.outlines),
        ];
        for (label, cmd, on) in toggles {
            if menu_check(ui, on, label, crate::shortcuts::shortcut(cmd).unwrap_or("")) {
                app.run(cmd, json!({}));
                ui.close();
            }
        }
    });
}

#[cfg(test)]
#[path = "panel_view_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "offscreen_tests.rs"]
mod offscreen;
