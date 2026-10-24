//! Fix AB5: the empty state, and suggestions only where they apply.

use serde_json::{Value, json};

use crate::follow_agent::chat_ui::tests::{feed, send_allowed, window};
use crate::follow_agent::tests::agent;
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::{click_named, tree};
use crate::services::{AgentEnd, AgentRunEvent};

fn panel(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)["aiPanel"].clone()
}

/// Empty: "What should we make?" and the line naming the chosen agent, centred and wrapped to the
/// panel (no fixed width), at 240 and 336 px; the disc is accent-soft with an accent-text ring.
#[test]
fn the_empty_state_names_the_agent_and_wraps_to_the_panel() {
    for w in [240.0, 336.0] {
        let (mut u, _) = window();
        u.app.ui.right_width = w;
        u.frame(vec![]);
        let p = panel(&u);
        assert_eq!(p["emptyState"], "What should we make?");
        assert_eq!(p["emptyLine"], "Claude Code works on this canvas. Follow keeps the camera on its edits.");
        assert_eq!(p["fonts"]["emptyHeading"], json!({"family": "semibold", "size": 18.0}));
        let t = crate::theme::Tokens::get(&u.ctx);
        let ro = crate::panel_view::ai_tab::panel_theme::roles(&t);
        assert_eq!(p["theme"]["emptyDisc"], crate::theme::accent::rgb_text(t.accent_tint), "accent-soft");
        assert_eq!(p["theme"]["emptyRing"], crate::theme::accent::rgb_text(ro.link), "accent-text");
        let nodes = tree(&mut u);
        let line = p["emptyLine"].as_str().unwrap();
        let b = nodes.iter().find(|n| n.label() == Some(line) || n.value() == Some(line)).and_then(|n| n.bounds()).expect("the line");
        let pane = nodes.iter().find(|n| n.role() == egui::accesskit::Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).unwrap();
        assert!(b.x0 >= pane.x0 + 15.5 && b.x1 <= pane.x1 - 15.5, "{w}: wrapped inside the panel: {b:?} in {pane:?}");
        assert!(b.x1 - b.x0 > (pane.x1 - pane.x0) - 32.0 - 60.0 || w > 300.0, "{w}: as wide as the panel lets it ({b:?})");
    }
    // The chosen agent.
    let (mut u, _) = window();
    u.app.run("chat.agent", json!({"agent": "codex"}));
    u.frame(vec![]);
    assert!(panel(&u)["emptyLine"].as_str().unwrap().starts_with("Codex works on this canvas"));
}

/// With a selection on the quiet panel: "For the selected Poster" and the two rows; a row puts its
/// words in the composer and focuses it, and sends nothing. Without a selection: none.
#[test]
fn the_selections_suggestions_fill_the_composer_and_never_send() {
    let (mut u, script) = window();
    let s = panel(&u)["composer"]["suggestions"].clone();
    assert_eq!(s["label"], "For the selected Poster");
    let rows: Vec<&str> = s["rows"].as_array().unwrap().iter().map(|r| r["text"].as_str().unwrap()).collect();
    assert_eq!(rows, ["Tidy the selected layers", "Align everything to the grid"]);
    let c = panel(&u)["composer"]["composerRect"].clone();
    let last = s["rows"][1]["rect"][3].as_f64().unwrap();
    assert!(last <= c[1].as_f64().unwrap(), "just above the composer");
    // Full width: the composer's own edges (Fix AB7: they were 16 inside them).
    let (r0, r2) = (s["rows"][0]["rect"][0].as_f64().unwrap(), s["rows"][0]["rect"][2].as_f64().unwrap());
    assert!((r0 - c[0].as_f64().unwrap()).abs() < 1.0 && (r2 - c[2].as_f64().unwrap()).abs() < 1.0, "full width: {s} {c}");
    click_named(&mut u, "Align everything to the grid");
    u.frame(vec![]);
    assert_eq!(u.app.ui.agent_view.chat.draft, "Align everything to the grid");
    assert!(u.ctx.memory(|m| m.has_focus(egui::Id::new("describe_change_field"))), "the field is focused");
    assert!(script.lock().unwrap().specs.is_empty(), "nothing sent");
    assert!(u.app.ui.agent_view.chat.lines.is_empty(), "nothing sent");
    // No selection: none.
    u.app.run("select.set", json!({"ids": []}));
    u.frame(vec![]);
    assert!(panel(&u)["composer"]["suggestions"].is_null());
}

/// After a run finishes: "Tidy spacing" and "Make a variation" under its card; not while it works,
/// not once undone. One fills the composer and sends nothing.
#[test]
fn a_finished_run_has_two_suggestions_under_its_card() {
    let (mut u, script) = window();
    u.app.run("select.set", json!({"ids": []}));
    send_allowed(&mut u, "Build a poster");
    u.frame(vec![]);
    agent(&mut u, "frame.create", json!({"x": 400, "y": 0, "width": 300, "height": 200}));
    u.frame(vec![]);
    assert!(panel(&u)["runSuggestions"].is_null(), "not while it works");
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 9.0 }]);
    u.frame(vec![]);
    u.frame(vec![]);
    let chips: Vec<String> =
        panel(&u)["runSuggestions"]["chips"].as_array().unwrap().iter().map(|c| c["text"].as_str().unwrap().to_string()).collect();
    assert_eq!(chips, ["Tidy spacing", "Make a variation"]);
    let sent = script.lock().unwrap().specs.len();
    click_named(&mut u, "Make a variation");
    u.frame(vec![]);
    assert_eq!(u.app.ui.agent_view.chat.draft, "Make a variation");
    assert_eq!(script.lock().unwrap().specs.len(), sent, "nothing sent");
    click_named(&mut u, "Undo run");
    assert!(panel(&u)["runSuggestions"].is_null(), "none once undone");
}
