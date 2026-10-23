//! Fix Z5: repaints while nothing happens, measured in the test harness by emulating the window's
//! event loop: the next frame comes when the delay egui asked for has passed (at most 60 a second),
//! and none while it asked for none. Each frame's causes are egui's (`repaint_causes`: the file,
//! line and reason of the request that painted it). Numbers: `docs/design/idle-repaint.md`.

use std::collections::BTreeMap;
use std::time::Duration;

use egui::accesskit::Role;
use serde_json::json;

use crate::follow_agent::chat_ui::tests::{feed, send_allowed, window};
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::tree;

/// Over `secs` of idle time: the frames painted and how many each cause asked for.
pub(crate) fn idle(u: &mut Ui, secs: f64) -> (usize, BTreeMap<String, usize>) {
    let start = u.time;
    let (mut frames, mut causes) = (0, BTreeMap::new());
    loop {
        let out = u.frame_raw(egui::RawInput::default());
        if u.time - start > secs {
            break;
        }
        frames += 1;
        for c in u.ctx.repaint_causes() {
            let file = c.file.rsplit('/').next().unwrap_or(c.file);
            *causes.entry(format!("{file}:{} {}", c.line, c.reason)).or_insert(0) += 1;
        }
        let delay = out.viewport_output.get(&egui::ViewportId::ROOT).map_or(Duration::MAX, |v| v.repaint_delay);
        if delay.as_secs_f64() >= secs {
            break;
        }
        // The next frame when the delay has passed (frame_raw adds a 60th of a second).
        u.time += (delay.as_secs_f64() - 1.0 / 60.0).max(0.0);
    }
    (frames, causes)
}

/// The first 10 s (what was still moving then settles), then a steady 10 s: their rates a second.
fn measure(u: &mut Ui, what: &str) -> (f64, f64) {
    let (first, why_first) = idle(u, 10.0);
    let (steady, why) = idle(u, 10.0);
    println!("{what}: first 10 s {first} frames ({:.1}/s) by cause {why_first:?}", first as f64 / 10.0);
    println!("{what}: steady 10 s {steady} frames ({:.1}/s) by cause {why:?}", steady as f64 / 10.0);
    (first as f64 / 10.0, steady as f64 / 10.0)
}

/// Idle with the AI tab open, with the Design tab open, and with a tooltip open on Undo run (a run
/// that has ended): once what was moving has settled, at most 2 repaints a second.
#[test]
fn idle_repaints_stay_under_two_a_second() {
    let (mut u, _) = window();
    let ai = measure(&mut u, "AI tab, idle");
    u.app.run("panel.tab.design", json!({}));
    let design = measure(&mut u, "Design tab, idle");
    let (mut u, script) = window();
    send_allowed(&mut u, "Build a poster");
    u.frame(vec![]);
    crate::follow_agent::tests::agent(&mut u, "frame.create", json!({"x": 400, "y": 0, "width": 300, "height": 200, "name": "Poster"}));
    use crate::services::{AgentEnd, AgentRunEvent};
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 12.0 }]);
    u.frame(vec![]);
    u.frame(vec![]);
    // What the run started (the camera's flight, the reveal, the working indicator's fade) settles
    // first; then the pointer rests on Undo run until its tooltip shows, and stays there.
    let (moving, why) = idle(&mut u, 10.0);
    println!("Tooltip on Undo run: the 10 s after the run, {moving} frames ({:.1}/s) by cause {why:?}", moving as f64 / 10.0);
    // The button is named "Undo run"; its tooltip's text is a label's value.
    let tooltips = |u: &mut Ui| tree(u).iter().filter(|n| n.label() == Some("Undo run") || n.value() == Some("Undo run")).count();
    let b = tree(&mut u).iter().find(|n| n.role() == Role::Button && n.label() == Some("Undo run")).and_then(|n| n.bounds()).expect("Undo run");
    u.frame(vec![egui::Event::PointerMoved(egui::pos2(900.0, 700.0))]);
    u.frame(vec![egui::Event::PointerMoved(egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32))]);
    for _ in 0..60 {
        u.frame(vec![]);
    }
    assert!(tooltips(&mut u) >= 2, "the tooltip shows");
    let tip = measure(&mut u, "Tooltip on Undo run");
    assert!(tooltips(&mut u) >= 2, "the tooltip is still open at the end");
    for (what, (_, steady)) in [("AI tab", ai), ("Design tab", design), ("tooltip", tip)] {
        assert!(steady <= 2.0, "{what}: {steady} repaints a second while nothing moves");
    }
}
