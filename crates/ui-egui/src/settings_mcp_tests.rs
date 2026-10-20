//! Settings ▸ AI & agents, MCP and Tool access (C3c): the controls run their commands, the counts
//! follow, and the connect flow is the AI tab's.

use egui::accesskit::{Node, Role, Toggled};
use egui::{Event, Modifiers, PointerButton, Pos2, pos2};
use figcraft_engine::Session;
use figcraft_engine::cmd::mcp_tools::TOOLS;
use serde_json::json;

use crate::layers_drag_tests::Ui;

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let mut out = u.frame_output(vec![]);
    out.textures_delta.clear();
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

fn find<'a>(nodes: &'a [Node], role: Role, label: &str) -> &'a Node {
    nodes.iter().find(|n| n.role() == role && n.label() == Some(label)).unwrap_or_else(|| panic!("no {role:?} named {label:?}"))
}

fn has_label(nodes: &[Node], text: &str) -> bool {
    nodes.iter().any(|n| n.label().is_some_and(|l| l.contains(text)) || n.value().is_some_and(|v| v.contains(text)))
}

fn click(u: &mut Ui, n: &Node) {
    let b = n.bounds().expect("bounds");
    let p: Pos2 = pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    u.frame(vec![Event::PointerMoved(p)]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
}

fn at(section: &str) -> Ui {
    let mut s = Session::new();
    s.agents.mcp.port = Some(7601);
    let mut u = Ui::new(s);
    u.frame(vec![]);
    u.app.run("settings.open", json!({"section": section}));
    u.frame(vec![]);
    assert_eq!(u.app.ui.settings_section, section);
    u
}

#[test]
fn the_rail_lists_the_built_sections() {
    let mut u = at("general");
    let nodes = tree(&mut u);
    for s in ["General", "Toolbar", "AI & agents", "MCP", "Tool access"] {
        find(&nodes, Role::Tab, s);
    }
}

#[test]
fn tool_access_turns_tools_and_groups_off_and_restores_them() {
    let mut u = at("tools");
    let total = TOOLS.len();
    let nodes = tree(&mut u);
    assert!(has_label(&nodes, &format!("{total} of {total} enabled")), "the count");
    let export = find(&nodes, Role::CheckBox, "export");
    assert_eq!(export.toggled(), Some(Toggled::True));
    click(&mut u, export);
    assert_eq!(u.app.session.prefs.settings.mcp.disabled_tools, ["export"]);
    let nodes = tree(&mut u);
    assert!(has_label(&nodes, &format!("{} of {total} enabled", total - 1)));
    assert_eq!(find(&nodes, Role::CheckBox, "export").toggled(), Some(Toggled::False));
    // The read-only group's switch is off now (not all on); clicking it turns the group on.
    let reads = TOOLS.iter().filter(|t| t.2).count();
    let group = find(&nodes, Role::CheckBox, &format!("All read-only tools ({} of {reads} on)", reads - 1));
    assert_eq!(group.toggled(), Some(Toggled::False));
    click(&mut u, group);
    assert!(u.app.session.prefs.settings.mcp.disabled_tools.is_empty());
    // Off with the group's switch, back with Restore defaults.
    let nodes = tree(&mut u);
    click(&mut u, find(&nodes, Role::CheckBox, &format!("All read-only tools ({reads} of {reads} on)")));
    assert_eq!(u.app.session.prefs.settings.mcp.disabled_tools.len(), reads);
    let nodes = tree(&mut u);
    click(&mut u, find(&nodes, Role::Button, "Restore defaults"));
    assert!(u.app.session.prefs.settings.mcp.disabled_tools.is_empty());
}

#[test]
fn the_mcp_section_shows_the_server_and_runs_its_commands() {
    let mut u = at("mcp");
    let nodes = tree(&mut u);
    assert!(has_label(&nodes, "Running"), "the status");
    for b in ["Restart", "Copy", "Use default", "Connect Claude Code"] {
        find(&nodes, Role::Button, b);
    }
    assert!(find(&nodes, Role::Button, "Use default").is_disabled() || u.app.session.prefs.settings.mcp.root_dir.is_none());
    let auth = find(&nodes, Role::CheckBox, "Require authentication");
    assert_eq!(auth.toggled(), Some(Toggled::False));
    click(&mut u, auth);
    assert!(u.app.session.prefs.settings.mcp.require_auth && u.app.session.prefs.settings.mcp.token.len() >= 32);
    // Connect opens the AI tab's flow, here.
    let nodes = tree(&mut u);
    click(&mut u, find(&nodes, Role::Button, "Connect Claude Code"));
    assert!(u.app.ui.connect.open, "the connect flow");
}

#[test]
fn ai_and_agents_has_the_ai_tabs_connect_and_no_keys_or_models() {
    let mut u = at("ai");
    let nodes = tree(&mut u);
    find(&nodes, Role::Button, "Connect Claude Code");
    assert!(has_label(&nodes, "Not connected"));
    assert!(!has_label(&nodes, "API key") || has_label(&nodes, "keeps no API keys"), "no key field");
    // (Inside the Settings window: the editor behind it has fields of its own.)
    let w = find(&nodes, Role::Window, "Settings").bounds().expect("bounds");
    let inside = |n: &&Node| n.bounds().is_some_and(|b| b.x0 >= w.x0 && b.x1 <= w.x1 && b.y0 >= w.y0 && b.y1 <= w.y1);
    assert!(!nodes.iter().filter(inside).any(|n| n.role() == Role::TextInput), "nothing to type a key or model into");
}

/// `scrim mcp --attach` asks the app which tools are off (`mcp.policy`, the control channel).
#[test]
fn the_control_channel_tells_attached_servers_which_tools_are_off() {
    let mut u = at("tools");
    u.app.run("mcp.set-tool", json!({"tool": "screenshot", "on": false}));
    let ctx = u.ctx.clone();
    let r = crate::control::handle(&mut u.app, &ctx, "mcp.policy", &json!({})).unwrap();
    assert_eq!(r["disabledTools"], json!(["screenshot"]));
}

/// C4: the text colours Settings draws meet 4.5:1 on the dialog (and on the rail's current row).
#[test]
fn settings_text_meets_4_5_to_1() {
    use crate::theme::Tokens;
    use crate::theme::accent::contrast;
    for (theme, t) in [("light", Tokens::light()), ("dark", Tokens::dark())] {
        let mut low = Vec::new();
        for (name, fg, bg) in [
            ("text", t.text, t.menu),
            ("text_secondary", t.text_secondary, t.menu),
            ("text_tertiary", t.text_tertiary, t.menu),
            ("danger", t.danger, t.menu),
            // (`success` colours only the status dot, always beside its text: not text itself.)
            ("warning", t.warning, t.menu),
            ("rail current", t.text, t.selected_row),
        ] {
            let c = contrast(fg, bg);
            eprintln!("{theme} {name}: {c:.2}");
            if c < 4.5 {
                low.push(format!("{theme} {name} {c:.2}"));
            }
        }
        assert!(low.is_empty(), "{low:?}");
    }
}

/// C4: the dialog without a mouse. Cmd+, opens it; Tab reaches the rail's sections, Space picks
/// one; Tab goes on into its controls, Space works them; Escape closes it.
#[test]
fn settings_can_be_used_from_the_keyboard_alone() {
    use egui::Key;
    let press = |u: &mut Ui, key: Key, modifiers: Modifiers| {
        u.frame(vec![
            Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers },
            Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers },
        ]);
        u.frame(vec![]);
    };
    let mut u = Ui::new(Session::new());
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    press(&mut u, Key::Comma, Modifiers::COMMAND);
    assert!(u.app.ui.preferences, "Cmd+, opens Settings");
    // Tab until the rail's Tool access tab has the focus.
    let mut seen = Vec::new();
    for _ in 0..60 {
        press(&mut u, Key::Tab, Modifiers::NONE);
        let name = crate::a11y::focused_name(&u.ctx);
        seen.push(name.clone());
        if name == "Tool access" {
            break;
        }
    }
    assert_eq!(seen.last().map(String::as_str), Some("Tool access"), "Tab reaches the rail: {seen:?}");
    press(&mut u, Key::Space, Modifiers::NONE);
    assert_eq!(u.app.ui.settings_section, "tools", "Space picks the section");
    // On into the pane: the first tool's switch, worked with Space.
    let mut seen = Vec::new();
    for _ in 0..40 {
        press(&mut u, Key::Tab, Modifiers::NONE);
        let name = crate::a11y::focused_name(&u.ctx);
        seen.push(name.clone());
        if name == "list_commands" {
            break;
        }
    }
    assert_eq!(seen.last().map(String::as_str), Some("list_commands"), "{seen:?}");
    press(&mut u, Key::Space, Modifiers::NONE);
    assert_eq!(u.app.session.prefs.settings.mcp.disabled_tools, ["list_commands"], "Space works the switch");
    press(&mut u, Key::Escape, Modifiers::NONE);
    assert!(!u.app.ui.preferences, "Escape closes it");
}

/// `ui.set {settings}` opens Settings at a section for captures, and false closes it.
#[test]
fn the_control_channel_can_open_settings_at_a_section_for_captures() {
    let mut u = Ui::new(Session::new());
    let ctx = u.ctx.clone();
    crate::control::handle(&mut u.app, &ctx, "ui.set", &json!({"settings": "diagnostics"})).unwrap();
    assert!(u.app.ui.preferences && u.app.ui.settings_section == "diagnostics");
    crate::control::handle(&mut u.app, &ctx, "ui.set", &json!({"settings": false})).unwrap();
    assert!(!u.app.ui.preferences);
}

/// C1.6 (found in the real app): Settings is modal. Tab never leaves it for the canvas, panels or
/// tab bar behind, so a keyboard user can't change the document by accident.
#[test]
fn tab_stays_inside_settings() {
    use egui::Key;
    let mut u = Ui::new(Session::new());
    u.app.run("shape.rectangle", json!({"x": 0, "y": 0, "width": 40, "height": 40}));
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    u.app.run("settings.open", json!({}));
    u.app.handle_ui_requests();
    u.frame(vec![]);
    u.app.ui.preferences = true;
    u.frame(vec![]);
    let inside: Vec<String> =
        ["General", "Toolbar", "AI & agents", "MCP", "Tool access", "Privacy", "Diagnostics", "Done"].iter().map(|s| s.to_string()).collect();
    let mut seen = Vec::new();
    for _ in 0..40 {
        u.frame(vec![
            Event::Key { key: Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
            Event::Key { key: Key::Tab, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
        ]);
        u.frame(vec![]);
        seen.push(crate::a11y::focused_name(&u.ctx));
    }
    for outside in ["Home", "Constrain proportions", "Rectangle", "Move"] {
        assert!(!seen.iter().any(|n| n == outside), "Tab reached {outside:?} behind Settings: {seen:?}");
    }
    assert!(seen.iter().any(|n| inside.contains(n)), "Tab moves within Settings: {seen:?}");
}
