//! Settings ▸ Toolbar (C3b): hiding a tool takes it off the toolbar but keeps its shortcut, Move
//! can't be hidden, dragging one tool onto another groups them into one button with a menu, the
//! keyboard's buttons move tools, and Reset to default puts the toolbar back.

use egui::accesskit::{Node, Role, Toggled};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2};
use figcraft_engine::{Session, ToolId};
use serde_json::json;

use crate::layers_drag_tests::Ui;

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let mut out = u.frame_output(vec![]);
    out.textures_delta.clear();
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

fn find<'a>(nodes: &'a [Node], role: Role, label: &str) -> Option<&'a Node> {
    nodes.iter().find(|n| n.role() == role && n.label() == Some(label))
}

fn center(n: &Node) -> Pos2 {
    let b = n.bounds().expect("bounds");
    pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)
}

fn click(u: &mut Ui, p: Pos2) {
    u.frame(vec![Event::PointerMoved(p)]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
}

/// The toolbar's buttons (the canvas's tools region), by name.
fn toolbar_buttons(nodes: &[Node]) -> Vec<String> {
    nodes.iter().filter(|n| n.role() == Role::Button && n.toggled().is_some()).filter_map(|n| n.label().map(str::to_string)).collect()
}

fn settings_at_toolbar() -> Ui {
    let mut u = Ui::new(Session::new());
    u.frame(vec![]);
    u.app.run("settings.open", json!({"section": "toolbar"}));
    u.frame(vec![]);
    u
}

#[test]
fn hiding_a_tool_takes_it_off_the_toolbar_and_keeps_its_shortcut() {
    let mut u = settings_at_toolbar();
    assert_eq!(u.app.ui.settings_section, "toolbar");
    let nodes = tree(&mut u);
    assert!(toolbar_buttons(&nodes).iter().any(|l| l == "Move"), "the Move group's button");
    let show = find(&nodes, Role::CheckBox, "Show Hand tool").expect("a Show checkbox per tool");
    assert_eq!(show.toggled(), Some(Toggled::True));
    click(&mut u, center(show));
    assert!(!figcraft_engine::cmd::toolbar::layout(&u.app.session.prefs).iter().flatten().any(|e| e.tool == "hand" && e.on));
    // Settings closed: the Move group's menu has no Hand, and H still picks the Hand tool.
    u.app.ui.preferences = false;
    let nodes = tree(&mut u);
    let flyout = center(find(&nodes, Role::Button, "Move tools").expect("the Move group's menu"));
    click(&mut u, flyout);
    let nodes = tree(&mut u);
    assert!(find(&nodes, Role::MenuItemCheckBox, "Scale").is_some(), "the menu is open");
    assert!(find(&nodes, Role::MenuItemCheckBox, "Hand tool").is_none(), "Hand is hidden from the menu");
    u.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
    assert_eq!(ToolId::Hand.shortcut(), "H");
    for pressed in [true, false] {
        u.frame(vec![Event::Key { key: Key::H, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }]);
    }
    assert_eq!(u.app.session.tool.tool, ToolId::Hand, "a hidden tool keeps its shortcut");
}

#[test]
fn move_cant_be_hidden() {
    let mut u = settings_at_toolbar();
    let nodes = tree(&mut u);
    let show = find(&nodes, Role::CheckBox, "Show Move").expect("Move's row");
    assert!(show.is_disabled(), "Move's Show is disabled");
    let e = u.app.session.execute("toolbar.toggle", &json!({"tool": "move", "on": false})).unwrap_err().to_string();
    assert!(e.contains("Move can't be hidden"), "{e}");
}

#[test]
fn dragging_a_tool_onto_another_groups_them_and_reset_puts_the_toolbar_back() {
    let mut u = settings_at_toolbar();
    let nodes = tree(&mut u);
    let from = center(find(&nodes, Role::Button, "Drag Slice").expect("Slice's handle"));
    // Onto Rectangle's name, just right of its handle and icon.
    let h = center(find(&nodes, Role::Button, "Drag Rectangle").expect("Rectangle's handle"));
    let to = pos2(h.x + 50.0, h.y);
    u.frame(vec![Event::PointerMoved(from)]);
    u.frame(vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    for k in 1..=6 {
        let p = from + (to - from) * (k as f32 / 6.0);
        u.frame(vec![Event::PointerMoved(p)]);
    }
    u.frame(vec![Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
    let l = figcraft_engine::cmd::toolbar::layout(&u.app.session.prefs);
    let tools: Vec<Vec<&str>> = l.iter().map(|g| g.iter().map(|e| e.tool.as_str()).collect()).collect();
    assert_eq!(tools[1], ["frame", "section"], "{tools:?}");
    assert_eq!(tools[2], ["rectangle", "slice", "line", "arrow", "ellipse", "polygon", "star"], "dropped onto Rectangle: after it");
    // The changed groups are menus named "Tools" now (a group as it ships keeps its name).
    u.app.ui.preferences = false;
    let nodes = tree(&mut u);
    assert!(find(&nodes, Role::Button, "Shape tools").is_none());
    assert!(find(&nodes, Role::Button, "Tools").is_some_and(|n| n.is_expanded() == Some(false)), "a menu button for the new group");
    // Reset to default.
    u.app.run("settings.open", json!({"section": "toolbar"}));
    let nodes = tree(&mut u);
    click(&mut u, center(find(&nodes, Role::Button, "Reset to default").expect("Reset")));
    assert!(u.app.session.prefs.settings.toolbar.is_empty(), "back to the default");
    assert_eq!(u.app.session.execute("toolbar.state", &json!({})).unwrap()["isDefault"], true);
}

#[test]
fn the_keyboards_buttons_move_a_tool_and_make_it_a_button_of_its_own() {
    let mut u = settings_at_toolbar();
    let nodes = tree(&mut u);
    // Hand is second in the Move group: Own button takes it out, after its group.
    click(&mut u, center(find(&nodes, Role::Button, "Make Hand tool a button of its own").expect("Own button")));
    let l = figcraft_engine::cmd::toolbar::layout(&u.app.session.prefs);
    assert_eq!(l[1].iter().map(|e| e.tool.as_str()).collect::<Vec<_>>(), ["hand"]);
    // Up, from the top of its group, puts it at the end of the group above.
    let nodes = tree(&mut u);
    click(&mut u, center(find(&nodes, Role::Button, "Move Hand tool up").expect("Up")));
    let l = figcraft_engine::cmd::toolbar::layout(&u.app.session.prefs);
    assert_eq!(l[0].iter().map(|e| e.tool.as_str()).collect::<Vec<_>>(), ["move", "scale", "hand"]);
    // And the first tool can't go up.
    let nodes = tree(&mut u);
    assert!(find(&nodes, Role::Button, "Move Move up").is_some_and(Node::is_disabled));
}
