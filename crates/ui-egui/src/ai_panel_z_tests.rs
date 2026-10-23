//! Fix Z: the AI panel's header title (Z3), the transcript's fade (Z4), the composer's icon chips
//! (Z6) and the run card after Undo run (Z7).

use egui::accesskit::Role;
use serde_json::Value;

use crate::follow_agent::chat_ui::tests::{send_allowed, window};
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::tree;

fn panel(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)["aiPanel"].clone()
}

fn rect(v: &Value) -> [f64; 4] {
    let a: Vec<f64> = v.as_array().unwrap_or_else(|| panic!("a rectangle: {v}")).iter().map(|x| x.as_f64().unwrap()).collect();
    [a[0], a[1], a[2], a[3]]
}

fn themed(dark: bool, w: f32) -> Ui {
    let (mut u, _) = window();
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    u.app.ui.right_width = w;
    u.frame(vec![]);
    u
}

/// Z3: a 48-character title is cut with "…" where it doesn't fit; the chevron sits 6 after what
/// shows; Follow and New chat keep 8 apart and 16 from the right; the whole title is the tooltip and
/// in the accessible name. At 240, 320 and 400 px, light and dark.
#[test]
fn a_long_title_is_cut_with_an_ellipsis_and_the_chevron_follows_it() {
    let prompt = "Build a Swiss poster series for the Welle music festival in Hamburg";
    for dark in [false, true] {
        for w in [240.0, 320.0, 400.0] {
            let mut u = themed(dark, w);
            send_allowed(&mut u, prompt);
            u.frame(vec![]);
            let h = panel(&u)["header"].clone();
            let title = h["title"].as_str().unwrap().to_string();
            assert_eq!(title.chars().count(), 48, "{title}");
            let shown = h["titleShown"].as_str().unwrap_or_else(|| panic!("{dark} {w}: titleShown: {h}"));
            assert!(shown.ends_with('…') && title.starts_with(shown.trim_end_matches('…')), "{dark} {w}: {shown:?} of {title:?}");
            let (text, chevron) = (rect(&h["titleRect"]), rect(&h["chevronRect"]));
            assert!((chevron[0] - text[2] - 6.0).abs() < 0.6, "{dark} {w}: the chevron 6 after the text: {text:?} {chevron:?}");
            let (follow, plus) = (rect(&h["followRect"]), rect(&panel(&u)["newChatRect"]));
            assert!(chevron[2] <= follow[0] - 8.0, "{dark} {w}: the chevron clear of Follow: {chevron:?} {follow:?}");
            assert!((plus[0] - follow[2] - 8.0).abs() < 0.6, "{dark} {w}: the buttons 8 apart");
            let nodes = tree(&mut u);
            let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
            assert!((pane.x1 - plus[2] - 16.0).abs() < 0.6, "{dark} {w}: New chat 16 from the right: {plus:?} in {pane:?}");
            let name = format!("Conversation: {title}");
            assert!(nodes.iter().any(|n| n.label() == Some(name.as_str())), "{dark} {w}: the accessible name has the whole title");
            // Hovered, the tooltip says the whole title.
            let at = egui::pos2(((text[0] + text[2]) / 2.0) as f32, ((text[1] + text[3]) / 2.0) as f32);
            u.frame(vec![egui::Event::PointerMoved(at)]);
            for _ in 0..60 {
                u.frame(vec![]);
            }
            let nodes = tree(&mut u);
            assert!(nodes.iter().any(|n| n.label() == Some(title.as_str()) || n.value() == Some(title.as_str())), "{dark} {w}: the tooltip");
        }
    }
}

/// A transcript of `n` notes.
fn notes(u: &mut Ui, n: usize) {
    u.app.ui.agent_view.chat.lines = (0..n).map(|i| crate::follow_agent::chat_ui::Line::Note(format!("Note {i}"))).collect();
    u.frame(vec![]);
    u.frame(vec![]);
}

/// Z4: a 12 px fade from transparent into the panel at the transcript's bottom, only while there is
/// more below: none for a short transcript, none at the bottom of a long one, there once scrolled
/// up. Light and dark.
#[test]
fn the_transcript_fades_at_the_bottom_only_with_more_below() {
    for dark in [false, true] {
        let mut u = themed(dark, 336.0);
        notes(&mut u, 2);
        assert!(panel(&u)["fade"].is_null(), "{dark}: a short transcript has no fade");
        notes(&mut u, 60);
        assert!(panel(&u)["fade"].is_null(), "{dark}: at the bottom, nothing below");
        let nodes = tree(&mut u);
        let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
        let at = egui::pos2(((pane.x0 + pane.x1) / 2.0) as f32, 300.0);
        u.frame(vec![egui::Event::PointerMoved(at)]);
        for _ in 0..10 {
            u.frame(vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 200.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        for _ in 0..30 {
            u.frame(vec![]);
        }
        let p = panel(&u);
        let f = &p["fade"];
        assert!(f.is_object(), "{dark}: scrolled up, a fade: {p}");
        let r = rect(&f["rect"]);
        assert!((r[3] - r[1] - 12.0).abs() < 0.01, "{dark}: 12 high: {r:?}");
        assert!(r[0] >= pane.x0 - 0.5 && r[2] <= pane.x1 + 0.5, "{dark}: inside the panel");
        let t = crate::theme::Tokens::get(&u.ctx);
        assert_eq!(p["theme"]["fade"], crate::theme::accent::rgb_text(t.panel), "{dark}: into the panel's own colour");
    }
}

/// The window with Claude Code and Codex installed, Codex chosen when `image` (its CLI takes one).
fn agents(dark: bool, w: f32, image: bool) -> Ui {
    use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script};
    let mut s = figcraft_engine::Session::new();
    let f = s.execute("frame.create", &serde_json::json!({"x": 0, "y": 0, "width": 200, "height": 200, "name": "Poster"})).unwrap();
    s.execute("select.set", &serde_json::json!({"ids": [f["id"]]})).unwrap();
    s.agents.mcp.port = Some(7690);
    let mut u = Ui::new(s);
    let script = std::sync::Arc::new(std::sync::Mutex::new(Script {
        claude: Some(CLAUDE.into()),
        codex: Some("/opt/agents/bin/codex".into()),
        ..Default::default()
    }));
    u.app.services = Box::new(Platform(script));
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    u.app.ui.right_width = w;
    u.app.run("panel.tab.ai", serde_json::json!({}));
    u.frame(vec![]);
    if image {
        u.app.run("chat.agent", serde_json::json!({"agent": "codex"}));
    }
    u.frame(vec![]);
    u.frame(vec![]);
    u
}

/// Z6: Selection and the image are round 32 × 32 icon chips at every width (240, 280, 336 and
/// 400 px, light and dark, the image chip there or not), 8 apart, after the agent chip and clear of
/// Send, with their accessible names and tooltips; no "Selection" word is drawn. It fails with the
/// text chips (wider than 32 from about 300 px).
#[test]
fn selection_and_image_are_round_icon_chips_at_every_width() {
    for dark in [false, true] {
        for w in [240.0, 280.0, 336.0, 400.0] {
            for image in [false, true] {
                let mut u = agents(dark, w, image);
                let c = panel(&u)["composer"].clone();
                let at = format!("{dark} {w} {image}");
                assert_eq!(c["imageButton"], image, "{at}");
                let sel = rect(&c["selectionChip"]["rect"]);
                assert!((sel[2] - sel[0] - 32.0).abs() < 0.5 && (sel[3] - sel[1] - 32.0).abs() < 0.5, "{at}: Selection 32 × 32: {sel:?}");
                assert_eq!(c["selectionChip"]["icon"], "selection");
                let nodes = tree(&mut u);
                let find = |name: &str| nodes.iter().find(|n| n.label().is_some_and(|l| l.starts_with(name))).and_then(|n| n.bounds());
                let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
                let b = find("Add the selection to the prompt").expect("Selection's name");
                assert!((b.x0 - sel[0]).abs() < 0.5 && (b.x1 - sel[2]).abs() < 0.5, "{at}: named where it's drawn");
                let agent = find("Agent: ").expect("the agent chip");
                assert!(agent.x1 <= sel[0] - 7.5, "{at}: after the agent chip, 8 apart: {agent:?} {sel:?}");
                let mut last = sel[2];
                if image {
                    let img = rect(&c["imageChip"]["rect"]);
                    assert!((img[2] - img[0] - 32.0).abs() < 0.5 && (img[3] - img[1] - 32.0).abs() < 0.5, "{at}: Image 32 × 32: {img:?}");
                    assert!((img[0] - sel[2] - 8.0).abs() < 0.6, "{at}: 8 after Selection");
                    assert!(find("Attach an image").is_some(), "{at}: the image's name");
                    last = img[2];
                } else {
                    assert!(c["imageChip"].is_null() && find("Attach an image").is_none(), "{at}: no image chip");
                }
                let send = find("Send").expect("Send");
                assert!(last <= send.x0 - 7.5 && send.x1 <= pane.x1 + 0.5, "{at}: clear of Send and inside the panel");
                assert!(!nodes.iter().any(|n| n.label() == Some("Selection") || n.value() == Some("Selection")), "{at}: no Selection word");
            }
        }
    }
}

/// Z6: hovered, a chip's tooltip is its name, as before.
#[test]
fn the_icon_chips_keep_their_tooltips() {
    let mut u = agents(false, 336.0, true);
    for name in ["Add the selection to the prompt", "Attach an image"] {
        u.frame(vec![egui::Event::PointerMoved(egui::pos2(5.0, 5.0))]);
        let b = tree(&mut u).iter().find(|n| n.label() == Some(name)).and_then(|n| n.bounds()).expect(name);
        u.frame(vec![egui::Event::PointerMoved(egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32))]);
        for _ in 0..60 {
            u.frame(vec![]);
        }
        let named = tree(&mut u).iter().filter(|n| n.label() == Some(name) || n.value() == Some(name)).count();
        assert!(named >= 2, "{name}: the chip and its tooltip ({named})");
    }
}

/// A run that made a frame and a text, ended; the window and the document before it.
fn ended_run(w: f32) -> (Ui, figcraft_doc::Document) {
    use crate::follow_agent::chat_ui::tests::feed;
    use crate::follow_agent::tests::agent;
    use crate::services::{AgentEnd, AgentRunEvent};
    let (mut u, script) = window();
    u.app.ui.right_width = w;
    let before = u.app.session.doc().unwrap().clone();
    send_allowed(&mut u, "Build a poster called Welle");
    u.frame(vec![]);
    let f = agent(&mut u, "frame.create", serde_json::json!({"x": 400, "y": 0, "width": 300, "height": 200, "name": "Poster — Welle"}));
    agent(&mut u, "text.create", serde_json::json!({"text": "Welle", "x": 10, "y": 120, "parent": f["id"]}));
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 42.0 }]);
    u.frame(vec![]);
    u.frame(vec![]);
    (u, before)
}

fn buttons(u: &Ui) -> Vec<(String, bool, String)> {
    panel(u)["card"]["buttons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| (b["label"].as_str().unwrap().into(), b["enabled"].as_bool().unwrap(), b["tooltip"].as_str().unwrap().into()))
        .collect()
}

/// Z7: after Undo run the card says "Undone · 42 s", its rows dimmed; Select layers is off ("Those
/// layers were removed by Undo"), not focusable, in the `disabled` token; Undo run is Redo run,
/// which puts the run back as one step. Pressing the off button does nothing and logs nothing.
#[test]
fn after_undo_run_the_card_is_undone_and_redo_run_puts_it_back() {
    for dark in [false, true] {
        let (mut u, before) = ended_run(336.0);
        u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
        u.frame(vec![]);
        let after_run = u.app.session.doc().unwrap().clone();
        crate::layers_header::tests::click_named(&mut u, "Undo run");
        assert_eq!(*u.app.session.doc().unwrap(), before, "{dark}: undone");
        let card = panel(&u)["card"].clone();
        assert_eq!((card["state"].clone(), card["heading"].clone()), (serde_json::json!("undone"), serde_json::json!("Undone · 42 s")), "{dark}");
        assert!(card["rows"].as_array().unwrap().iter().all(|r| r["state"] == "undone"), "{dark}: rows dimmed: {card}");
        assert_eq!(
            buttons(&u),
            [("Select layers".to_string(), false, "Those layers were removed by Undo".to_string()), ("Redo run".into(), true, "Redo run".into())],
            "{dark}"
        );
        let t = crate::theme::Tokens::get(&u.ctx);
        assert_eq!(panel(&u)["theme"]["cardDisabled"], crate::theme::accent::rgb_text(t.disabled), "{dark}: the disabled token");
        let nodes = tree(&mut u);
        let sel = nodes.iter().find(|n| n.label() == Some("Select layers")).expect("Select layers");
        assert!(sel.is_disabled(), "{dark}: Select layers is off for screen readers");
        assert!(!sel.supports_action(egui::accesskit::Action::Focus), "{dark}: and not focusable");
        let notes = u.app.session.prefs.settings.diagnostics.len();
        crate::layers_header::tests::click_named(&mut u, "Select layers");
        assert!(u.app.session.selection().is_empty() || u.app.session.selection().iter().all(|n| before.contains(*n)), "{dark}: nothing selected");
        assert_eq!(u.app.session.prefs.settings.diagnostics.len(), notes, "{dark}: an off button runs nothing");
        // Redo run: the run back, as one step; the card is Done again.
        crate::layers_header::tests::click_named(&mut u, "Redo run");
        assert_eq!(*u.app.session.doc().unwrap(), after_run, "{dark}: redone");
        assert_eq!(panel(&u)["card"]["state"], "done", "{dark}");
        assert_eq!(buttons(&u)[1], ("Undo run".to_string(), true, "Undo run".to_string()));
        u.app.run("edit.undo", serde_json::json!({}));
        assert_eq!(*u.app.session.doc().unwrap(), before, "{dark}: ⌘Z takes the redo back");
    }
}

/// Z7: once something changed after Undo run, Redo run is off and says why, in its tooltip.
#[test]
fn redo_run_is_off_once_something_changed_since() {
    let (mut u, _) = ended_run(336.0);
    crate::layers_header::tests::click_named(&mut u, "Undo run");
    u.app.run("shape.ellipse", serde_json::json!({"x": 900, "y": 0, "width": 10, "height": 10}));
    u.frame(vec![]);
    assert_eq!(buttons(&u)[1], ("Redo run".to_string(), false, "Something changed since: Redo is no longer one step".to_string()));
    let nodes = tree(&mut u);
    let b = nodes.iter().find(|n| n.label() == Some("Redo run")).and_then(|n| n.bounds()).expect("Redo run");
    u.frame(vec![egui::Event::PointerMoved(egui::pos2(5.0, 5.0))]);
    u.frame(vec![egui::Event::PointerMoved(egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32))]);
    for _ in 0..60 {
        u.frame(vec![]);
    }
    let tip = "Something changed since: Redo is no longer one step";
    assert!(tree(&mut u).iter().any(|n| n.value() == Some(tip) || n.label() == Some(tip)), "the tooltip says why");
}

/// Z7: a refused press shows its words on one line under the buttons (secondary, 12 px, wrapped
/// inside the card), not a toast; Diagnostics says "refused" with the reason; the next action
/// clears the line.
#[test]
fn a_refused_press_says_why_under_the_buttons_until_the_next_action() {
    for w in [240.0, 336.0] {
        let (mut u, _) = ended_run(w);
        u.app.run("shape.ellipse", serde_json::json!({"x": 900, "y": 0, "width": 10, "height": 10}));
        u.app.ui.last_error = None;
        u.frame(vec![]);
        crate::layers_header::tests::click_named(&mut u, "Undo run");
        let p = panel(&u);
        let words = p["card"]["refused"].as_str().unwrap_or_else(|| panic!("{w}: a line: {p}"));
        assert!(words.starts_with("The document changed after the run began"), "{w}: {words}");
        assert!(u.app.ui.last_error.is_none(), "{w}: no toast");
        let last = u.app.session.prefs.settings.diagnostics.last().unwrap().detail.clone();
        assert_eq!(last, "chat.undo-run: refused (changed since the run)", "{w}");
        assert!(p["widest"].as_f64().unwrap() <= p["width"].as_f64().unwrap() + 1.0, "{w}: wrapped, no sideways scroll");
        let nodes = tree(&mut u);
        let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
        let card = rect(&p["card"]["rect"]);
        assert!(card[2] <= pane.x1 - 15.5, "{w}: the card, with its line, 16 inside");
        // The next action clears it.
        u.app.run("select.all", serde_json::json!({}));
        u.frame(vec![]);
        assert!(panel(&u)["card"]["refused"].is_null(), "{w}: gone on the next action");
    }
}
