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
                // Since Fix AA3 a narrow row keeps the agent's name and puts the image in its menu.
                if image && c["imageInMenu"] == true {
                    assert!(c["imageChip"].is_null() && find("Attach an image").is_none(), "{at}: the image is in the agent menu");
                } else if image {
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
/// clears the line. (Since Fix AA2 Undo run is off once something changed, so the refusal here is
/// Select layers after the person deleted every layer the run made.)
#[test]
fn a_refused_press_says_why_under_the_buttons_until_the_next_action() {
    for w in [240.0, 336.0] {
        let (mut u, _) = ended_run(w);
        // The person deletes the run's frame (its text goes with it).
        let ids: Vec<serde_json::Value> =
            u.app.session.doc().unwrap().find_by_name("Poster — Welle", None).into_iter().map(|n| serde_json::json!(n.0)).collect();
        assert_eq!(ids.len(), 1, "{w}");
        u.app.run("select.set", serde_json::json!({"ids": ids}));
        u.app.run("edit.delete", serde_json::json!({}));
        u.app.ui.last_error = None;
        u.frame(vec![]);
        crate::layers_header::tests::click_named(&mut u, "Select layers");
        let p = panel(&u);
        let words = p["card"]["refused"].as_str().unwrap_or_else(|| panic!("{w}: a line: {p}"));
        assert_eq!(words, "None of the layers the run touched are left", "{w}");
        assert!(u.app.ui.last_error.is_none(), "{w}: no toast");
        let last = u.app.session.prefs.settings.diagnostics.last().unwrap().detail.clone();
        assert_eq!(last, "chat.select-run-layers: refused (no layers left)", "{w}");
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

fn card_view(u: &Ui) -> (String, Value, Vec<(String, bool, String)>) {
    let c = panel(u)["card"].clone();
    (c["heading"].as_str().unwrap_or_default().to_string(), c["note"].clone(), buttons(u))
}

/// Fix AA2, both themes: Undo run, then ⌘Z: the card is Done again with Undo run live and "Undo
/// is one step"; ⌘⇧Z: Undone with Redo run live and "Redo is one step"; ⌘Z and an edit: both off
/// and no note. Fix Z kept the card Undone after ⌘Z, with Redo run off.
#[test]
fn the_card_follows_cmd_z_and_cmd_shift_z() {
    for dark in [false, true] {
        let (mut u, _) = ended_run(336.0);
        u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
        u.frame(vec![]);
        crate::layers_header::tests::click_named(&mut u, "Undo run");
        let (h, note, b) = card_view(&u);
        assert_eq!((h.as_str(), note.as_str()), ("Undone · 42 s", Some("Redo is one step")), "{dark}");
        assert_eq!((b[1].0.as_str(), b[1].1), ("Redo run", true), "{dark}");
        u.app.run("edit.undo", serde_json::json!({}));
        u.frame(vec![]);
        let (h, note, b) = card_view(&u);
        assert_eq!((h.as_str(), note.as_str()), ("Done · 42 s", Some("Undo is one step")), "{dark}: ⌘Z brings it back");
        assert_eq!(b, [("Select layers".to_string(), true, "Select layers".to_string()), ("Undo run".into(), true, "Undo run".into())], "{dark}");
        assert!(panel(&u)["card"]["rows"].as_array().unwrap().iter().all(|r| r["state"] == "done"), "{dark}");
        u.app.run("edit.redo", serde_json::json!({}));
        u.frame(vec![]);
        let (h, note, b) = card_view(&u);
        assert_eq!((h.as_str(), note.as_str()), ("Undone · 42 s", Some("Redo is one step")), "{dark}: ⌘⇧Z");
        assert_eq!((b[1].0.as_str(), b[1].1), ("Redo run", true), "{dark}");
        // ⌘Z, then an edit in between: both off, no note.
        u.app.run("edit.undo", serde_json::json!({}));
        u.app.run("shape.ellipse", serde_json::json!({"x": 900, "y": 0, "width": 10, "height": 10}));
        u.frame(vec![]);
        let (h, note, b) = card_view(&u);
        assert_eq!(h, "Done · 42 s", "{dark}");
        assert!(note.is_null(), "{dark}: no note when neither can go");
        assert!(!b[1].1 && b[1].0 == "Undo run", "{dark}: Undo run off: {b:?}");
        assert_eq!(b[1].2, "Something changed since the run: Undo run is no longer one step");
    }
}

/// Fix AA3: the window with Claude Code or Codex (whose CLI takes an image) chosen, at `w` px.
fn composer(dark: bool, w: f32, codex: bool) -> Ui {
    agents(dark, w, codex)
}

/// Fix AA3: at 240, 260, 280 and 336 px, light and dark, Claude Code and Codex:
/// the agent chip always shows a readable name (all of it, or six characters and "…"), below 280 px
/// without its chevron; when that needs the image chip's room, the image is "Attach image…" in the
/// agent menu; the row never overflows; every control keeps its name and at least a 24 px target.
#[test]
fn the_agent_chip_shows_a_readable_name_at_every_width() {
    for dark in [false, true] {
        for w in [240.0, 260.0, 280.0, 336.0] {
            for codex in [false, true] {
                {
                    let at = format!("{dark} {w} codex={codex}");
                    let mut u = composer(dark, w, codex);
                    let c = panel(&u)["composer"].clone();
                    let name = if codex { "Codex" } else { "Claude Code" };
                    let chip = &c["agentChip"];
                    let shown = chip["shown"].as_str().unwrap_or_else(|| panic!("{at}: {c}"));
                    let readable =
                        shown == name || (shown.ends_with('…') && shown.chars().count() >= 7 && name.starts_with(shown.trim_end_matches('…')));
                    assert!(readable, "{at}: the agent's name reads: {shown:?}");
                    assert_eq!(chip["chevron"], w >= 280.0, "{at}: the chevron only from 280 px");
                    let in_menu = c["imageInMenu"].as_bool().unwrap();
                    assert_eq!(c["imageChip"].is_object(), codex && !in_menu, "{at}");
                    assert!(!in_menu || codex, "{at}");
                    let nodes = tree(&mut u);
                    let ai = crate::i18n::tr("AI").to_string();
                    let pane = nodes
                        .iter()
                        .find(|n| n.role() == Role::Pane && n.label() == Some(ai.as_str()))
                        .and_then(|n| n.bounds())
                        .expect("the AI pane");
                    let composer = rect(&c["composerRect"]);
                    let mut names: Vec<String> = vec![crate::i18n::trf("Agent: {agent}", &[("agent", &name)])];
                    names.push(crate::i18n::tr("Add the selection to the prompt").to_string());
                    if codex && !in_menu {
                        names.push(crate::i18n::tr("Attach an image").to_string());
                    }
                    let mut last = f64::MIN;
                    for n in &names {
                        let b = nodes
                            .iter()
                            .find(|x| x.label() == Some(n.as_str()))
                            .and_then(|x| x.bounds())
                            .unwrap_or_else(|| panic!("{at}: {n} named"));
                        assert!(b.x1 - b.x0 >= 24.0 && b.y1 - b.y0 >= 24.0, "{at}: {n} has a 24 px target: {b:?}");
                        assert!(b.x0 >= last + 7.5, "{at}: {n} after the one before, 8 apart");
                        assert!(b.x0 >= composer[0] && b.x1 <= composer[2], "{at}: {n} inside the composer");
                        last = b.x1;
                    }
                    let send = nodes
                        .iter()
                        .find(|x| x.label().is_some_and(|l| l.starts_with(crate::i18n::tr("Send"))))
                        .and_then(|x| x.bounds())
                        .expect("Send");
                    assert!(send.x0 >= last + 7.5 && send.x1 <= composer[2] && send.x1 <= pane.x1, "{at}: Send clear and inside");
                    let p = panel(&u);
                    assert!(p["widest"].as_f64().unwrap() <= p["width"].as_f64().unwrap() + 1.0, "{at}: no sideways scroll");
                }
            }
        }
    }
}

/// Fix AA3, Spanish: the row's visible words are the agent's name and icons, so a longer language
/// can't push it over; what Spanish changes is what the controls are called, and each has its
/// Spanish name. (The UI's language is process-wide, so tests don't switch it while others run.)
#[test]
fn the_rows_controls_have_their_spanish_names() {
    for key in ["Agent: {agent}", "Add the selection to the prompt", "Attach an image", "Attach image…", "Agent settings…", "Send", "Stop"] {
        let es = crate::i18n::own_entry("es", key).unwrap_or_else(|| panic!("{key} in Spanish"));
        assert!(!es.is_empty() && es != key, "{key}: {es}");
    }
}

/// Fix AA3: at 240 px with Codex, "Attach image…" in the agent menu does what the image chip does:
/// asks for an image and attaches it.
#[test]
fn attach_image_is_in_the_agent_menu_when_the_chip_has_no_room() {
    use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script};
    let mut u = composer(false, 240.0, true);
    let script = std::sync::Arc::new(std::sync::Mutex::new(Script {
        claude: Some(CLAUDE.into()),
        codex: Some("/opt/agents/bin/codex".into()),
        pick: Some("/tmp/ref.png".into()),
        ..Default::default()
    }));
    u.app.services = Box::new(Platform(script.clone()));
    u.frame(vec![]);
    assert_eq!(panel(&u)["composer"]["imageInMenu"], true);
    assert!(panel(&u)["composer"]["imageChip"].is_null(), "no chip");
    crate::layers_header::tests::click_named(&mut u, "Agent: Codex");
    crate::layers_header::tests::click_named(&mut u, "Attach image…");
    for _ in 0..3 {
        u.frame(vec![]);
    }
    assert_eq!(script.lock().unwrap().picked, ["Attach an image"], "the chip's dialog");
    assert_eq!(u.app.ui.agent_view.chat.image.as_deref(), Some("/tmp/ref.png"), "attached");
}
