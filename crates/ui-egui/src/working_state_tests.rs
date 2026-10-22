//! Stage I.1, the working state on its own clock: one run across the gaps between an agent's steps,
//! the grace period's end exactly, a finish, several clients, nameless clients, and a run starting
//! during the last one's fade out.

use super::{FADE_IN, FADE_OUT, FINISHED_FOR, State, key};

const GRACE: f64 = 15.0;

/// Ticks `w` at 60 Hz from `from` to `to`, asserting it works and is fully shown at every one.
fn on_throughout(w: &mut State, from: f64, to: f64) {
    let mut t = from;
    while t < to {
        assert!(w.tick(t, GRACE, true), "working at {t}");
        assert_eq!(w.alpha(t, true), 1.0, "fully shown at {t}, no fade between steps");
        t += 1.0 / 60.0;
    }
}

#[test]
fn six_steps_with_gaps_are_one_run_with_no_fade_between_them() {
    let mut w = State::default();
    let k = key(Some("s1"), Some("codex-mcp-client"), "mcp (http)");
    w.touch(k.clone(), Some("Codex".into()), 0.0);
    w.tick(0.0, GRACE, true);
    for step in 1..6 {
        let at = f64::from(step) * 2.5;
        on_throughout(&mut w, at - 2.5 + FADE_IN, at);
        w.touch(k.clone(), None, at);
    }
    on_throughout(&mut w, 12.5, 14.0);
    assert_eq!(w.name(14.0, GRACE).as_deref(), Some("Codex"), "the name it gave first stays");
    w.finish(&k, 14.0);
    assert!(!w.tick(14.0, GRACE, true));
    assert!((w.alpha(14.0 + FADE_OUT / 2.0, true) - 0.5).abs() < 1e-9, "a 400 ms fade out");
    assert_eq!(w.alpha(14.0 + FADE_OUT, true), 0.0);
    assert_eq!(w.finished(14.5), Some(Some("Codex".into())), "\"Codex finished\"");
    assert_eq!(w.finished(14.0 + FINISHED_FOR), None, "for 2 s");
}

#[test]
fn the_grace_period_ends_it_at_exactly_the_setting() {
    for grace in [5.0, 15.0, 30.0, 60.0] {
        let mut w = State::default();
        w.touch("route:mcp (http)".into(), None, 100.0);
        assert!(w.tick(100.0 + grace - 0.001, grace, true), "still on just before {grace} s");
        assert_eq!(w.next_change(100.0 + grace - 1.0, grace), Some(1.0));
        // Seen a little late (the next frame): it ended at the grace period's end, not then.
        assert!(!w.tick(100.0 + grace + 0.1, grace, true));
        assert!((w.alpha(100.0 + grace + 0.1, true) - (1.0 - 0.1 / FADE_OUT)).abs() < 1e-9, "the fade counts from {grace} s");
    }
}

#[test]
fn with_two_clients_it_stays_for_the_one_still_working_and_names_the_latest() {
    let mut w = State::default();
    w.touch("session:a".into(), Some("Codex".into()), 0.0);
    w.touch("session:b".into(), Some("Cursor".into()), 1.0);
    w.tick(1.0, GRACE, true);
    assert_eq!(w.name(1.0, GRACE).as_deref(), Some("Cursor"), "the most recently active");
    w.touch("session:a".into(), None, 2.0);
    assert_eq!(w.name(2.0, GRACE).as_deref(), Some("Codex"));
    w.finish("session:a", 3.0);
    assert!(w.tick(3.0, GRACE, true), "Cursor still works");
    assert_eq!(w.name(3.0, GRACE).as_deref(), Some("Cursor"));
    assert_eq!(w.alpha(3.0, true), 1.0, "no fade for Codex's finish");
    assert_eq!(w.count(3.0, GRACE), 1);
    // Cursor's session ends: the run ends, named after it.
    w.finish("session:b", 4.0);
    assert!(!w.tick(4.0, GRACE, true));
    assert_eq!(w.finished(4.1), Some(Some("Cursor".into())));
}

#[test]
fn nameless_clients_are_an_agent_and_a_finish_never_comes_before_their_last_request() {
    let mut w = State::default();
    w.touch(key(None, None, "mcp (http)"), None, 5.0);
    w.tick(5.0, GRACE, true);
    assert_eq!(w.name(5.0, GRACE), None, "no name: the caller says \"an agent\"");
    // A finish read a frame early still ends after the request that sent it.
    w.finish(&key(None, None, "mcp (http)"), 4.99);
    assert!(!w.tick(5.0, GRACE, true));
    assert_eq!(w.alpha(5.0, true), 1.0 - 0.0, "ending at 5.0, the fade starts from full");
    assert_eq!(w.finished(5.1), Some(None));
    // Finishing an unknown client changes nothing.
    w.finish("session:nobody", 6.0);
}

#[test]
fn work_starting_again_during_the_fade_out_carries_on_without_a_dip() {
    let mut w = State::default();
    w.touch("session:a".into(), None, 0.0);
    w.tick(1.0, GRACE, true);
    w.finish("session:a", 1.0);
    w.tick(1.0, GRACE, true);
    let before = w.alpha(1.1, true);
    w.touch("session:a".into(), None, 1.1);
    assert!(w.tick(1.1, GRACE, true));
    assert!((w.alpha(1.1, true) - before).abs() < 1e-9, "from where the fade was ({before}), not from 0");
    assert_eq!(w.alpha(1.1 + FADE_IN, true), 1.0);
}

#[test]
fn without_motion_it_is_on_for_the_whole_run_then_off() {
    let mut w = State::default();
    w.touch("session:a".into(), None, 0.0);
    w.tick(0.0, 5.0, false);
    assert_eq!(w.alpha(0.0, false), 1.0, "no fade in");
    w.tick(4.9, 5.0, false);
    assert_eq!(w.alpha(4.9, false), 1.0);
    w.tick(5.0, 5.0, false);
    assert_eq!(w.alpha(5.0, false), 0.0, "no fade out");
}

#[test]
fn an_ended_agent_run_stops_its_own_clients_and_no_others() {
    let mut s = State::default();
    s.touch_as("session:desktop".into(), Some("Claude".into()), Some("claude-ai"), 1.0);
    s.touch_as("session:named".into(), Some("Claude Code".into()), Some("claude-code"), 1.5);
    // The run begins at 2.0; a client that first appears after it is the run's, whatever its name.
    s.touch_as("session:child".into(), None, Some("scrim-stub-agent"), 3.0);
    s.touch_as("session:desktop".into(), Some("Claude".into()), Some("claude-ai"), 3.5);
    s.touch_as("session:named".into(), Some("Claude Code".into()), Some("claude-code"), 3.6);
    s.tick(4.0, 30.0, true);
    s.finish_where(5.0, |raw| raw == Some("claude-code"), 2.0);
    s.tick(5.0, 30.0, true);
    assert_eq!(s.count(5.0, 30.0), 1, "Claude Desktop, there before the run under another name, keeps working");
    let ends: Vec<_> =
        s.transitions().into_iter().filter(|t| t.cause == figcraft_engine::settings::diagnostics::Cause::FinishWorking).map(|t| t.client).collect();
    assert_eq!(ends.len(), 2, "the run's child and the client named like its agent: {ends:?}");
}
