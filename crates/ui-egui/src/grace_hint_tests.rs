//! Fix Q2: a real agent pauses 5–15 s between steps while it writes the next batch. At the default
//! 30 s the dots stay through gaps of 8, 12 and 20 s in every frame; at 15 s only the 20 s gap ends
//! them, and the agent's return is noted ("grace too short") with a hint, once per run, whose
//! button opens the setting. Agents can't change the setting.

use serde_json::json;

use crate::follow_agent::tests::frames;
use crate::follow_agent::working_state::gap_tests::{BRIDGE, calm, events, send};
use crate::layers_drag_tests::Ui;

const S: Option<&str> = Some("8caa29ef-session");

/// The dots this frame: drawn, at full opacity.
fn on(u: &Ui) -> bool {
    u.app.ui.agent_view.indicator.drawn.get().is_some() && u.app.ui.agent_view.working.alpha(u.time, true) >= 1.0
}

/// Steps `gaps` seconds apart (a rectangle each, from Claude Desktop's bridge); per gap, whether
/// the dots were on in every frame of it.
fn steps(u: &mut Ui, gaps: &[f64]) -> Vec<bool> {
    // A poster, then a layer in it at each step (the dots hang over the agent's artboard).
    let poster = send(
        u,
        S,
        Some(BRIDGE),
        "engine.execute",
        json!({"command": "frame.create", "params": {"x": 0, "y": 0, "width": 300, "height": 200, "name": "Poster"}}),
    )["id"]
        .clone();
    let layer = |u: &mut Ui, i: usize| {
        let p = json!({"command": "shape.rectangle", "params": {"x": 10 + 20 * i, "y": 10, "width": 10, "height": 10, "parent": poster}});
        send(u, S, Some(BRIDGE), "engine.execute", p);
    };
    frames(u, 30); // past the 160 ms fade in, and the camera's flight
    let mut each = Vec::new();
    for (i, gap) in gaps.iter().enumerate() {
        let mut all = true;
        for _ in 0..(gap * 60.0) as usize {
            u.frame(vec![]);
            all &= on(u);
        }
        each.push(all);
        layer(u, i);
        frames(u, 12);
    }
    each
}

fn grace_events(u: &Ui) -> Vec<String> {
    events(u).into_iter().filter(|e| e.starts_with("grace too short")).collect()
}

#[test]
fn at_the_default_30_s_gaps_of_8_12_and_20_s_keep_the_dots_in_every_frame() {
    let mut u = calm();
    assert_eq!(u.app.session.agent_grace(), 30, "the default");
    assert_eq!(steps(&mut u, &[8.0, 12.0, 20.0]), [true, true, true]);
    assert!(grace_events(&u).is_empty() && !u.app.ui.agent_view.grace_hint.showing);
}

#[test]
fn at_15_s_only_the_20_s_gap_breaks_and_the_return_is_noted_with_a_hint_once() {
    let mut u = calm();
    u.app.run("pref.agentGrace", json!({"seconds": 15}));
    assert_eq!(steps(&mut u, &[8.0, 12.0, 20.0]), [true, true, false]);
    assert_eq!(grace_events(&u), [format!("grace too short: {BRIDGE} came back after 20 s (the setting is 15 s)")]);
    assert!(u.app.ui.agent_view.grace_hint.showing, "the hint");
    // The button opens Settings ▸ General at the setting; the hint is done.
    super::open_setting(&mut u.app);
    u.frame(vec![]);
    assert!(u.app.ui.preferences && u.app.ui.settings_section == "general");
    assert!(!u.app.ui.agent_view.grace_hint.showing);
    u.app.ui.preferences = false;
    // Another return: noted, but the hint isn't shown again this run.
    steps(&mut u, &[20.0]);
    assert_eq!(grace_events(&u).len(), 2);
    assert!(!u.app.ui.agent_view.grace_hint.showing, "once per run");
    assert_eq!(u.app.session.prefs.settings.agent_grace_secs, 15, "nothing changes the setting on its own");
}

#[test]
fn an_agent_cant_change_the_grace_through_the_window() {
    let mut u = calm();
    let r = send(&mut u, S, Some(BRIDGE), "engine.execute", json!({"command": "pref.agentGrace", "params": {"seconds": 120}}));
    assert!(r.is_null(), "refused: {r}");
    assert_eq!((u.app.session.prefs.settings.agent_grace_secs, u.app.session.prefs.settings.agent_grace_chosen), (30, false));
}
