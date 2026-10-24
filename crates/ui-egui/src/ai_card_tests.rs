//! Fix AB2: the run card as a summary (the design's A2 to A5).

use serde_json::{Value, json};

use crate::follow_agent::chat_ui::tests::{feed, send_allowed, window};
use crate::follow_agent::tests::agent;
use crate::layers_drag_tests::Ui;
use crate::services::{AgentEnd, AgentRunEvent};

fn card(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)["aiPanel"]["card"].clone()
}

fn theme(u: &Ui, k: &str) -> String {
    crate::follow_agent::inspect(&u.app)["aiPanel"]["theme"][k].as_str().unwrap_or_default().to_string()
}

/// A run: a frame with three bars, a text, one bar renamed, one bar deleted; working until `end`.
fn run(dark: bool) -> (Ui, crate::follow_agent::chat_ui::tests::Shared) {
    let (mut u, script) = window();
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    send_allowed(&mut u, "Build a poster called Welle");
    u.frame(vec![]);
    let f = agent(&mut u, "frame.create", json!({"x": 400, "y": 0, "width": 300, "height": 200, "name": "Poster"}));
    let bars: Vec<Value> = (0..3)
        .map(|k| json!({"command": "shape.rectangle", "params": {"x": 10 + k * 40, "y": 10, "width": 30, "height": 30, "parent": f["id"]}}))
        .collect();
    let made = agent(&mut u, "command.batch", json!({"commands": bars}));
    agent(&mut u, "text.create", json!({"text": "Welle", "parent": f["id"]}));
    agent(&mut u, "node.set", json!({"id": made[0]["id"], "props": {"name": "Tall bar"}}));
    agent(&mut u, "edit.delete", json!({"ids": [made[2]["id"]]}));
    u.frame(vec![]);
    u.frame(vec![]);
    (u, script)
}

fn end(u: &mut Ui, script: &crate::follow_agent::chat_ui::tests::Shared) {
    feed(script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 107.0 }]);
    u.frame(vec![]);
    u.frame(vec![]);
}

/// The detail line from the counts: the parts that aren't zero, plurals from the catalogue.
#[test]
fn the_detail_line_says_what_the_counts_are() {
    use figcraft_engine::cmd::chat_run::RunCounts;
    let d = |frames, layers, edited, deleted| super::detail(&RunCounts { frames, layers, edited, deleted });
    assert_eq!(d(2, 36, 4, 1).as_deref(), Some("Created 2 frames and 36 layers, edited 4, deleted 1."));
    assert_eq!(d(1, 0, 0, 0).as_deref(), Some("Created 1 frame."));
    assert_eq!(d(0, 1, 0, 2).as_deref(), Some("Created 1 layer, deleted 2."));
    assert_eq!(d(0, 0, 3, 0).as_deref(), Some("Edited 3."));
    assert_eq!(d(0, 0, 0, 0), None, "nothing changed: no line");
}

/// While it works: "Working" with the time, the accent's dot and "{n} layers so far", "Now: …" (the
/// newest step), the bar, "Steps" with "{done} of {listed}", and no buttons. Ended: "Done · 1:47",
/// "Undo is one step", success's dot, "{n} layers changed", the counts, the buttons 36 high.
/// Undone: "Undone · 1:47", "Redo is one step", text-3's dot, Redo run. Both themes.
#[test]
fn the_card_summarises_the_run_in_each_state() {
    for dark in [false, true] {
        let (mut u, script) = run(dark);
        let t = crate::theme::Tokens::get(&u.ctx);
        let ro = crate::panel_view::ai_tab::panel_theme::roles(&t);
        let hex = crate::theme::accent::rgb_text;
        let c = card(&u);
        assert_eq!((c["heading"].as_str(), c["state"].as_str()), (Some("Working"), Some("working")), "{dark}: {c}");
        assert!(c["note"].as_str().is_some_and(|n| n.contains(':')), "{dark}: the time: {c}");
        // A frame, three bars (one deleted after: nothing), a text; one bar renamed (still created).
        assert_eq!(c["summary"], "4 layers so far", "{dark}");
        assert!(c["detail"].as_str().is_some_and(|d| d.starts_with("Now: ")), "{dark}: {c}");
        assert_eq!(c["progress"], true);
        let rows = c["rows"].as_array().unwrap().len();
        assert_eq!(c["steps"]["count"], format!("{} of {rows}", rows - 1), "{dark}");
        assert!(c["buttons"].as_array().unwrap().is_empty(), "{dark}: no buttons while it works");
        assert_eq!(theme(&u, "cardDot"), hex(ro.mark), "{dark}: the accent's dot");
        end(&mut u, &script);
        let c = card(&u);
        assert_eq!((c["heading"].as_str(), c["note"].as_str()), (Some("Done · 1:47"), Some("Undo is one step")), "{dark}");
        assert_eq!(c["summary"], "4 layers changed");
        assert_eq!(c["detail"], "Created 1 frame and 3 layers.", "{dark}");
        assert_eq!(c["steps"]["count"], rows.to_string());
        assert_eq!(theme(&u, "cardDot"), hex(ro.success), "{dark}: success's dot");
        for b in c["buttonRects"].as_array().unwrap() {
            let b: Vec<f64> = b.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
            assert!((b[3] - b[1] - 36.0).abs() < 0.5, "{dark}: 36 high");
        }
        crate::layers_header::tests::click_named(&mut u, "Undo run");
        let c = card(&u);
        assert_eq!((c["heading"].as_str(), c["note"].as_str()), (Some("Undone · 1:47"), Some("Redo is one step")), "{dark}");
        assert_eq!(c["summary"], "4 layers changed", "the counts stay");
        assert_eq!(theme(&u, "cardDot"), hex(ro.text_3), "{dark}: text-3's dot");
        let dimmed = crate::follow_agent::inspect(&u.app)["aiPanel"]["theme"]["cardDetail"].clone();
        assert_ne!(dimmed.as_str(), Some(hex(ro.text_secondary).as_str()), "{dark}: the detail at half strength");
        assert_eq!(c["buttons"][1]["label"], "Redo run");
    }
}

/// The steps are closed at first; the Steps row (`chat.toggle-run-steps`) opens and closes them;
/// a new run starts closed.
#[test]
fn the_steps_open_and_close_and_start_closed() {
    let (mut u, script) = run(false);
    end(&mut u, &script);
    assert_eq!(card(&u)["steps"]["open"], false, "closed at first");
    let r: Vec<f32> = card(&u)["steps"]["rect"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect();
    let at = egui::pos2((r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0);
    for pressed in [true, false] {
        u.frame(vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
        ]);
    }
    u.frame(vec![]);
    assert_eq!(card(&u)["steps"]["open"], true, "the Steps row opens them");
    u.app.run("chat.toggle-run-steps", json!({}));
    u.frame(vec![]);
    assert_eq!(card(&u)["steps"]["open"], false, "the command closes them");
    u.app.run("chat.toggle-run-steps", json!({}));
    send_allowed(&mut u, "Another");
    u.frame(vec![]);
    assert_eq!(card(&u)["steps"]["open"], false, "a new run starts closed");
}

/// While it works the card asks for its next frame in 0.1 s (the banner's pace: one frame for
/// both), and with reduced motion at the next second, for its time: never every frame. (The window
/// draws more while an agent works: the canvas's indicator and the run's output poll.) Ended, it
/// asks for none.
#[test]
fn the_card_asks_for_frames_at_the_banners_pace_or_once_a_second() {
    for reduce in [false, true] {
        let (mut u, script) = run(false);
        u.app.session.prefs.reduce_motion = reduce;
        u.frame(vec![]);
        let next = card(&u)["repaintIn"].as_f64().unwrap_or(0.0);
        if reduce {
            assert!((0.01..=1.01).contains(&next) && next >= 0.01, "reduced motion: at the next second: {next}");
        } else {
            assert!((next - 0.1).abs() < 1e-9, "the banner's pace: {next}");
        }
        end(&mut u, &script);
        assert!(card(&u)["repaintIn"].is_null(), "ended: no frame asked for");
    }
}
