//! Fix AE through the window: Escape with Option still held cancels an Option-drag, Option pressed
//! or let go mid-drag takes effect on the next frame without the pointer moving, and the copy
//! cursor follows Option over a layer that isn't selected.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use figcraft_engine::tools::Cursor;
use serde_json::json;

use crate::layers_drag_tests::Ui;

const ALT: Modifiers = Modifiers { alt: true, ctrl: false, shift: false, mac_cmd: false, command: false };

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// A window with one unselected 80×60 rectangle; its centre on screen.
fn one_rectangle() -> (Ui, NodeId, Pos2) {
    let mut s = Session::new();
    let v = s.execute("shape.rectangle", &json!({"x": 100, "y": 100, "width": 80, "height": 60})).unwrap();
    let r = figcraft_engine::params::id_of(&v["id"]).unwrap();
    s.execute("select.set", &json!({"ids": []})).unwrap();
    let mut u = Ui::new(s);
    u.frame(vec![]);
    let c = u.app.session.doc().unwrap().world_bounds(r).unwrap().center();
    let p = u.app.to_screen(c.x, c.y);
    (u, r, p)
}

fn layers(u: &Ui) -> usize {
    let ds = u.app.session.ds().unwrap();
    ds.doc.node(ds.page).unwrap().children.len()
}

/// Press at `from` and drag 100 to the right in five frames with `held`.
fn press_and_drag(u: &mut Ui, from: Pos2, held: Modifiers) {
    u.frame_with(vec![Event::PointerMoved(from)], held);
    u.frame_with(vec![button(from, true, held)], held);
    for k in 1..=5 {
        u.frame_with(vec![Event::PointerMoved(from + vec2(20.0 * k as f32, 0.0))], held);
    }
}

/// Escape while Option is still held cancels the drag: before Fix AE, Option+Escape matched no
/// shortcut, the drag went on and the release dropped the copy.
#[test]
fn escape_with_option_held_cancels_an_option_drag() {
    let (mut u, _, from) = one_rectangle();
    let before = serde_json::to_vec(&*u.app.session.ds().unwrap().doc).unwrap();
    press_and_drag(&mut u, from, ALT);
    assert_eq!(layers(&u), 2, "the copy shows while dragging");
    u.frame_with(vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: ALT }], ALT);
    u.frame_with(vec![button(from + vec2(100.0, 0.0), false, ALT)], ALT);
    u.frame(vec![]);
    assert_eq!(serde_json::to_vec(&*u.app.session.ds().unwrap().doc).unwrap(), before, "the document from before the drag");
    assert!(u.app.session.ds().unwrap().history.undo.iter().all(|h| h.label != "Duplicate"));
}

/// Option pressed mid-drag, the pointer still: on the next frame the original is back and a copy
/// is at the pointer. Let go: the copy goes and the original follows again.
#[test]
fn option_pressed_and_let_go_mid_drag_switch_on_the_next_frame() {
    let (mut u, r, from) = one_rectangle();
    press_and_drag(&mut u, from, Modifiers::default());
    let at = |u: &Ui| u.app.session.doc().unwrap().world_bounds(r).map(|b| (b.x0, b.y0)).unwrap();
    assert_eq!((layers(&u), at(&u)), (1, (200.0, 100.0)), "moving");
    u.frame_with(vec![], ALT);
    assert_eq!((layers(&u), at(&u)), (2, (100.0, 100.0)), "copying: the original snapped back");
    assert_eq!(u.app.session.tool.overlay.cursor, Cursor::Copy);
    u.frame_with(vec![], Modifiers::default());
    assert_eq!((layers(&u), at(&u)), (1, (200.0, 100.0)), "moving again");
    u.frame_with(vec![], ALT);
    u.frame_with(vec![button(from + vec2(100.0, 0.0), false, ALT)], ALT);
    u.frame(vec![]);
    assert_eq!((layers(&u), at(&u)), (2, (100.0, 100.0)), "dropped as a copy");
}

/// Over a layer that isn't selected, the copy cursor shows as soon as Option is pressed and goes
/// as soon as it is let go, without the pointer moving; it isn't shown over empty canvas.
#[test]
fn the_copy_cursor_follows_option_over_an_unselected_layer() {
    let (mut u, _, p) = one_rectangle();
    u.frame(vec![Event::PointerMoved(p)]);
    assert_eq!(u.app.session.tool.overlay.cursor, Cursor::Default);
    u.frame_with(vec![], ALT);
    u.frame_with(vec![], ALT);
    assert_eq!(u.app.session.tool.overlay.cursor, Cursor::Copy);
    u.frame_with(vec![], Modifiers::default());
    assert_eq!(u.app.session.tool.overlay.cursor, Cursor::Default);
    u.frame_with(vec![Event::PointerMoved(p + vec2(300.0, 300.0))], ALT);
    assert_eq!(u.app.session.tool.overlay.cursor, Cursor::Default, "over empty canvas");
}
