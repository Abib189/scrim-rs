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
