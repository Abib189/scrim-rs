//! The AI tab (B3): the connection and its connect command, the activity feed (select, undo an
//! agent's latest change), the lint findings by rule, and keeping agents off the selection. Every
//! control runs a command, and screen readers get each by name.

use std::sync::{Arc, Mutex};

use egui::accesskit::{Node, Role};
use figcraft_engine::agents::Author;
use serde_json::{Value, json};

use crate::live_tests::{Harness, has_text, text_center};

struct Clipboard(Arc<Mutex<Vec<String>>>);

impl crate::Services for Clipboard {
    fn clipboard_set_text(&mut self, s: &str) {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(s.to_string());
    }
}

/// The AI tab on a file where an agent drew a rectangle "Badge" after the person's rectangle.
fn harness() -> (Harness, Arc<Mutex<Vec<String>>>, u64) {
    let (mut h, _) = Harness::new();
    let copied = Arc::new(Mutex::new(Vec::new()));
    h.app.services = Box::new(Clipboard(copied.clone()));
    let agent = Author::agent("mcp");
    let badge =
        h.app.session.execute_as(agent.clone(), None, "shape.rectangle", &json!({"x": 400, "y": 100, "width": 40, "height": 40})).unwrap()["id"]
            .as_u64()
            .unwrap();
    h.app.session.execute_as(agent, None, "object.rename", &json!({"ids": [badge], "name": "Badge"})).unwrap();
    h.app.session.execute("select.none", &json!({})).unwrap();
    h.app.session.execute("panel.tab.ai", &json!({})).unwrap();
    h.app.handle_ui_requests();
    h.frames(3);
    (h, copied, badge)
}

fn sel(h: &Harness) -> Vec<u64> {
    h.app.session.selection().iter().map(|i| i.0).collect()
}

#[test]
fn the_status_row_says_stopped_not_connected_or_connected() {
    let (mut h, _, _) = harness();
    let shapes = h.frames(1);
    assert!(has_text(&shapes, "Server stopped") && has_text(&shapes, "Restart"), "no endpoint in the test harness");
    h.app.session.agents.mcp.port = Some(7601);
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Not connected") && has_text(&shapes, "Connect Claude Code"), "the empty state");
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap();
    h.app.session.agents.mcp.clients.push(("claude-code".into(), "2.1.296".into(), now));
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Connected · claude-code"), "connected, and by whom");
    assert!(!has_text(&shapes, "Connect Claude Code"), "no Connect button once connected");
}

#[test]
fn the_feed_selects_a_changes_layers_and_undoes_the_agents_latest_change() {
    let (mut h, _, badge) = harness();
    let shapes = h.frames(1);
    assert!(has_text(&shapes, "Agent (mcp): Rename"), "the newest change first, with its author");
    h.click(text_center(&shapes, "Agent (mcp): Rename"));
    let shapes = h.frames(2);
    assert_eq!(sel(&h), [badge]);
    // Its Undo: the agent's rename is the latest step.
    let rename = text_center(&shapes, "Agent (mcp): Rename");
    let undo = texts_at_row(&shapes, "Undo", rename.y);
    h.click(undo);
    h.frames(2);
    let name = h.app.session.doc().unwrap().node(figcraft_doc::NodeId(badge)).map(|n| n.name.clone());
    assert_ne!(name.as_deref(), Some("Badge"), "the rename is undone, the rectangle stays");
    // Under a later edit of the person's, the agent's change can't be undone from the feed.
    h.app.session.execute("shape.rectangle", &json!({"x": 0, "y": 300, "width": 10, "height": 10})).unwrap();
    let revs: Vec<Value> = h.app.session.execute("history.changes", &json!({})).unwrap()["changes"].as_array().unwrap().clone();
    let agents = revs.iter().rev().find(|c| c["agent"] == true).unwrap()["revision"].as_u64().unwrap();
    assert!(!figcraft_engine::cmd::ai::can_undo(&h.app.session, agents));
}

/// The centre of the text `s` drawn on the row at height `y` (the feed repeats "Undo").
fn texts_at_row(shapes: &[egui::Shape], s: &str, y: f32) -> egui::Pos2 {
    crate::live_tests::texts(shapes)
        .into_iter()
        .filter(|(t, _, _)| t == s)
        .map(|(_, p, size)| p + size / 2.0)
        .min_by(|a, b| (a.y - y).abs().total_cmp(&(b.y - y).abs()))
        .unwrap_or_else(|| panic!("no {s:?} near y {y}"))
}

#[test]
fn findings_are_grouped_with_counts_and_select_their_layers() {
    let (mut h, _, _) = harness();
    let f = h.app.session.execute("frame.create", &json!({"x": 0, "y": 400, "width": 300, "height": 200})).unwrap()["id"].as_u64().unwrap();
    let mut texts = Vec::new();
    for (i, t) in ["One", "Two"].into_iter().enumerate() {
        let r = h.app.session.execute("text.create", &json!({"x": 10, "y": 410 + 40 * i, "text": t, "parent": f, "fontFamily": "Inter"})).unwrap();
        texts.push(r["id"].as_u64().unwrap());
    }
    h.app.session.execute("select.none", &json!({})).unwrap();
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Overused font"), "the rule's group");
    h.click(text_center(&shapes, "Overused font"));
    let shapes = h.frames(2);
    let mut s = sel(&h);
    s.sort_unstable();
    assert!(texts.iter().all(|t| s.contains(t)), "the rule's row selects its layers: {s:?}");
    assert!(has_text(&shapes, "One") && has_text(&shapes, "Two"), "and opens to list them");
}

#[test]
fn the_lock_keeps_agents_off_the_selection() {
    let (mut h, _, badge) = harness();
    h.app.session.execute("select.set", &json!({"ids": [badge]})).unwrap();
    let shapes = h.frames(2);
    h.click(text_center(&shapes, "Keep agents off the selection"));
    h.frames(2);
    assert!(h.app.session.doc().unwrap().node(figcraft_doc::NodeId(badge)).is_some_and(|n| n.locked));
    let e = h.app.session.execute_as(Author::agent("mcp"), None, "object.rename", &json!({"ids": [badge], "name": "Mine"})).unwrap_err();
    assert!(e.to_string().contains("locked"), "{e}");
}

fn tree(h: &mut Harness) -> Vec<Node> {
    h.ctx.enable_accesskit();
    h.frames(2);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0))),
        time: Some(100.0),
        ..Default::default()
    };
    let app = &mut h.app;
    let mut out = h.ctx.run_ui(input, |ui| app.ui(ui));
    // Another test may load fonts meanwhile (a process-wide change): the font atlas then has updates.
    out.textures_delta.clear();
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

#[test]
fn screen_readers_get_every_control_by_name() {
    let (mut h, _, _) = harness();
    let nodes = tree(&mut h);
    let named = |role: Role, label: &str| nodes.iter().any(|n| n.role() == role && n.label() == Some(label));
    for label in ["Restart", "Agent (mcp): Rename", "Undo Rename"] {
        assert!(named(Role::Button, label), "a button named {label:?}");
    }
    assert!(named(Role::CheckBox, "Keep agents off the selection"));
}

#[test]
fn reading_the_tab_every_frame_leaves_no_trace() {
    let (mut h, _, _) = harness();
    let journal = h.app.session.journal.len();
    h.frames(10);
    assert_eq!(h.app.session.journal.len(), journal, "the tab's reads aren't commands the person ran");
}

/// The feed lists the newest edits however long ago they were: selection changes move the
/// revision too, and the tab used to ask for a revision window, which a few hundred clicks emptied
/// (found by Stage C's C4 in the real app: "No changes yet" under three edits).
#[test]
fn the_feed_keeps_its_edits_after_many_selection_changes() {
    let (mut h, _, _) = harness();
    for _ in 0..200 {
        h.app.session.execute("select.all", &json!({})).unwrap();
        h.app.session.execute("select.none", &json!({})).unwrap();
    }
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Agent (mcp): Rename"), "still listed");
    assert!(!has_text(&shapes, "No changes yet. Edits by you and by agents appear here, newest first."));
}

/// C1.7: "Overused font" opens to one row per font family with its layer count; a family's row
/// selects its layers and opens to them.
#[test]
fn overused_font_findings_are_grouped_by_family() {
    let (mut h, _, _) = harness();
    for (i, family) in ["Inter", "Inter", "Inter", "Geist", "Geist"].iter().enumerate() {
        h.app.session.execute("text.create", &json!({"text": "Hello", "x": 0, "y": 300 + 40 * i, "props": {"fontFamily": family}})).unwrap();
    }
    let shapes = h.frames(2);
    h.click(text_center(&shapes, "Overused font"));
    let shapes = h.frames(2);
    assert!(has_text(&shapes, "Inter") && has_text(&shapes, "Geist"), "a row per family");
    // ("Hello" is each layer's name: the Layers panel lists them too.)
    let hellos = |shapes: &[egui::Shape]| crate::live_tests::texts(shapes).iter().filter(|(t, _, _)| t == "Hello").count();
    let before = hellos(&shapes);
    h.click(text_center(&shapes, "Geist"));
    let shapes = h.frames(2);
    assert_eq!(h.app.session.selection().len(), 2, "the family's layers are selected");
    assert_eq!(hellos(&shapes), before + 2, "and listed under it");
    let nodes = tree(&mut h);
    assert!(nodes.iter().any(|n| n.label() == Some("Inter: 3 layers")) && nodes.iter().any(|n| n.label() == Some("Geist: 2 layers")));
}
