//! The composer's + menu (Fix AB4, the design's A6), in place of the Selection and Image chips: a
//! 32 px round "+" at the row's left opens a menu above the composer (surface fill, border, radius
//! 12, 8 inside, 36 px rows; a row under the pointer or the keys on the raised fill):
//! - "Add selection", the selection's name at its right in text-2: `chat.add-selection`.
//! - "Attach image": `chat.attach-image`, for an agent whose CLI takes one.
//! - "Follow the agent", a toggle (success when on): `camera.follow-agents`, the same command as
//!   the header's button, so Fix W's rules for agents hold.
//!
//! It closes on Esc, a click outside, and after Add selection or Attach image; ↑ and ↓ move along
//! its rows and Enter runs one. Every row is named for screen readers.

use egui::{Align2, CornerRadius, Id, Key, Modifiers, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, trn};
use crate::panel_view::ai_tab::panel_theme::Roles;
use crate::theme::{Tokens, regular};

/// The menu's width, its rows' height and its inside edge.
const W: f32 = 232.0;
const ROW: f32 = 36.0;
const PAD: f32 = 8.0;

/// What each row does.
#[derive(Clone, Copy, PartialEq)]
enum Row {
    AddSelection,
    AttachImage,
    Follow,
}

const ROWS: [Row; 3] = [Row::AddSelection, Row::AttachImage, Row::Follow];

/// The canvas selection as the menu and the chip name it: the layer's name, or "3 layers".
pub fn selection_name(app: &FigApp) -> Option<String> {
    let ds = app.session.ds().ok()?;
    match ds.selection.as_slice() {
        [] => None,
        [one] => ds.doc.node(*one).map(|n| n.name.clone()),
        many => Some(trn("{n} layers", many.len(), &[("n", &many.len())])),
    }
}

/// The + button, and its menu while open. Returns what `ui.inspect` reports.
pub fn plus(app: &mut FigApp, ui: &mut Ui, ro: &Roles, takes_image: bool, read_only: bool) -> Value {
    let t = Tokens::get(ui.ctx());
    let open = app.ui.agent_view.chat.plus_open;
    let name = tr("More: add the selection, attach an image, follow the agent");
    let resp = crate::follow_agent::chat_ui::chips::icon_chip(ui, ro, crate::panel_view::ai_tab::icons::AiIcon::Plus, name);
    crate::a11y::menu_button(&resp, name, open);
    if resp.clicked() {
        app.ui.agent_view.chat.plus_open = !open;
        app.ui.agent_view.chat.plus_row = 0;
    }
    let button = resp.rect;
    let mut out = json!({"rect": [button.min.x, button.min.y, button.max.x, button.max.y], "open": app.ui.agent_view.chat.plus_open});
    if !app.ui.agent_view.chat.plus_open {
        return out;
    }
    let selection = selection_name(app);
    let following = figcraft_engine::cmd::camera::follow_state(&app.session) != "off";
    let enabled = |row: Row| match row {
        Row::AddSelection => selection.is_some() && !read_only,
        Row::AttachImage => takes_image && !read_only,
        Row::Follow => true,
    };
    // The keys, while it's open: ↑ ↓ along the rows, Enter runs one, Esc closes.
    let (down, up, enter, esc) = ui.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::ArrowUp),
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    let chat = &mut app.ui.agent_view.chat;
    if down {
        chat.plus_row = (chat.plus_row + 1) % ROWS.len();
    }
    if up {
        chat.plus_row = (chat.plus_row + ROWS.len() - 1) % ROWS.len();
    }
    let keyed = chat.plus_row;
    // Above the composer, its left edge at the + button's, kept inside the panel.
    let panel = ui.clip_rect();
    let width = W.min(panel.width() - 32.0);
    let height = PAD * 2.0 + ROW * ROWS.len() as f32;
    let box_left = app.ui.agent_view.chat.seen.composer["composerRect"][0].as_f64().map_or(button.min.x, |x| x as f32);
    let left = box_left.min(panel.max.x - 16.0 - width).max(panel.min.x + 16.0);
    // Above the status line (as drawn last frame), 8 clear of it, as the design has it; its left
    // edge at the composer box's.
    let last = &app.ui.agent_view.chat.seen.composer;
    let above = last["statusRect"][1].as_f64().or_else(|| last["composerRect"][1].as_f64()).map_or(button.min.y - 60.0, |y| y as f32);
    let top = above - 8.0 - height;
    // (A layer of its own: only the window's top edge bounds it.)
    let menu = Rect::from_min_size(pos2(left, top.max(0.0)), vec2(width, height));
    let mut picked: Option<Row> = None;
    let mut rows_out = Vec::new();
    egui::Area::new(Id::new("ai_plus_menu")).order(egui::Order::Foreground).fixed_pos(menu.min).show(ui.ctx(), |ui| {
        let p = ui.painter();
        p.rect_filled(menu, CornerRadius::same(12), t.field);
        p.rect_stroke(menu, CornerRadius::same(12), Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
        for (i, row) in ROWS.into_iter().enumerate() {
            let r = Rect::from_min_size(pos2(menu.min.x + PAD, menu.min.y + PAD + ROW * i as f32), vec2(width - 2.0 * PAD, ROW));
            let on = enabled(row);
            let resp = ui.interact(r, Id::new(("ai_plus_row", i)), if on { Sense::click() } else { Sense::hover() });
            let (label, trail) = match row {
                Row::AddSelection => (tr("Add selection"), selection.clone().unwrap_or_default()),
                Row::AttachImage => (tr("Attach image"), String::new()),
                Row::Follow => (tr("Follow the agent"), String::new()),
            };
            let checked = (row == Row::Follow).then_some(following);
            crate::a11y::menu_item(&resp, label, &trail, checked);
            if !on {
                crate::a11y::disabled(&resp);
                let why = match row {
                    Row::AddSelection => tr("Select layers on the canvas first"),
                    _ => tr("This agent can't take an image"),
                };
                let _ = resp.clone().on_hover_text(why);
            }
            if (resp.hovered() || keyed == i) && on {
                ui.painter().rect_filled(r, CornerRadius::same(8), ro.chip_fill);
            }
            let color = if on { ro.text } else { ro.new_chat_disabled };
            ui.painter().text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, label, regular(13.0), color);
            if !trail.is_empty() {
                let room = (r.width() - 20.0 - ui.painter().layout_no_wrap(label.to_string(), regular(13.0), color).size().x - 16.0).max(24.0);
                let shown = crate::panel_view::ai_tab::run_card::elide(ui.painter(), &trail, regular(12.0), room);
                ui.painter().text(pos2(r.max.x - 10.0, r.center().y), Align2::RIGHT_CENTER, shown, regular(12.0), ro.text_secondary);
            }
            if row == Row::Follow {
                toggle(ui.painter(), pos2(r.max.x - 10.0 - 15.0, r.center().y), following, ro);
            }
            if on && (resp.clicked() || (enter && keyed == i)) {
                picked = Some(row);
            }
            rows_out.push(json!({"label": label, "trail": trail, "enabled": on, "on": checked, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]}));
        }
    });
    // Closed by Esc or a click outside it and the + button.
    let clicked_outside =
        ui.input(|i| i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|at| !menu.contains(at) && !button.contains(at)));
    if esc || clicked_outside {
        app.ui.agent_view.chat.plus_open = false;
    }
    match picked {
        Some(Row::AddSelection) => {
            app.run("chat.add-selection", json!({}));
            app.ui.agent_view.chat.plus_open = false;
        }
        Some(Row::AttachImage) => {
            app.run("chat.attach-image", json!({}));
            app.ui.agent_view.chat.plus_open = false;
        }
        Some(Row::Follow) => {
            app.run("camera.follow-agents", json!({"on": !following, "origin": "button"}));
        }
        None => {}
    }
    out["menu"] = json!({"rect": [menu.min.x, menu.min.y, menu.max.x, menu.max.y], "rows": rows_out, "keyed": keyed});
    out
}

/// "Follow the agent"'s switch: 30 × 18, success when on, its knob at the right.
fn toggle(p: &egui::Painter, c: egui::Pos2, on: bool, ro: &Roles) {
    let r = Rect::from_center_size(c, vec2(30.0, 18.0));
    p.rect_filled(r, CornerRadius::same(9), if on { ro.success } else { ro.chip_fill });
    let knob = if on { pos2(r.max.x - 9.0, c.y) } else { pos2(r.min.x + 9.0, c.y) };
    p.circle_filled(knob, 7.0, if on { ro.done_check } else { ro.text_3 });
}
