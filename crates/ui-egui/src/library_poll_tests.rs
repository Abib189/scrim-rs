//! Fix R2-2 on a fake clock: the panel's frames, a second apart, with the time and focus given.

use std::time::{Duration, Instant};

use super::{FOCUS_CHECK, LIBRARY_POLL, Poll};

fn links() -> Vec<String> {
    vec!["figcraft://join?server=wss%3A%2F%2Fexample.org&file=lib".to_string()]
}

/// Frames from `from` to `to` (seconds after `t0`), one a second, numbered on from `*n`; the
/// times at which the poll asked.
fn frames(p: &mut Poll, t0: Instant, n: &mut u64, from: u64, to: u64, focused: bool, links: &[String]) -> Vec<u64> {
    let mut asked = Vec::new();
    for s in from..to {
        *n += 1;
        if p.frame(t0 + Duration::from_secs(s), *n, focused, links) {
            asked.push(s);
        }
    }
    asked
}

#[test]
fn it_asks_when_the_panel_shows_then_not_before_five_minutes_then_at_five() {
    let (t0, mut p, mut n) = (Instant::now(), Poll::default(), 0);
    let five = LIBRARY_POLL.as_secs();
    assert_eq!(frames(&mut p, t0, &mut n, 0, five, true, &links()), vec![0], "once on showing, none before five minutes");
    assert_eq!(frames(&mut p, t0, &mut n, five, five + 1, true, &links()), vec![five], "at five minutes");
    assert_eq!(frames(&mut p, t0, &mut n, five + 1, 2 * five + 1, true, &links()), vec![2 * five], "and every five minutes on");
}

#[test]
fn coming_back_to_the_front_asks_at_once_at_most_every_thirty_seconds() {
    let (t0, mut p, mut n) = (Instant::now(), Poll::default(), 0);
    assert_eq!(frames(&mut p, t0, &mut n, 0, 10, true, &links()), vec![0]);
    // Away, then back after 60 s: asks at once.
    assert!(frames(&mut p, t0, &mut n, 10, 60, false, &links()).is_empty(), "nothing while the window isn't in front");
    assert_eq!(frames(&mut p, t0, &mut n, 60, 70, true, &links()), vec![60]);
    // Away and back again within 30 s of that: not again.
    assert!(frames(&mut p, t0, &mut n, 70, 75, false, &links()).is_empty());
    assert!(frames(&mut p, t0, &mut n, 75, 80, true, &links()).is_empty(), "rate-limited to once per {FOCUS_CHECK:?}");
    // Back after the 30 s: asks.
    assert!(frames(&mut p, t0, &mut n, 80, 95, false, &links()).is_empty());
    assert_eq!(frames(&mut p, t0, &mut n, 95, 100, true, &links()), vec![95]);
}

#[test]
fn nothing_is_asked_while_the_window_isnt_in_front_even_past_five_minutes() {
    let (t0, mut p, mut n) = (Instant::now(), Poll::default(), 0);
    let five = LIBRARY_POLL.as_secs();
    assert_eq!(frames(&mut p, t0, &mut n, 0, 1, true, &links()), vec![0]);
    assert!(frames(&mut p, t0, &mut n, 1, 3 * five, false, &links()).is_empty(), "fifteen minutes behind other windows: no request");
}

#[test]
fn nothing_is_asked_while_the_panel_is_hidden_and_showing_it_again_asks() {
    let (t0, mut p, mut n) = (Instant::now(), Poll::default(), 0);
    let five = LIBRARY_POLL.as_secs();
    assert_eq!(frames(&mut p, t0, &mut n, 0, 5, true, &links()), vec![0]);
    // Hidden: the panel draws no frame, and the poll has no clock of its own, so ten minutes pass
    // with no request (the frame numbers go on without it).
    n += 600;
    // Shown again: once at once, then the five-minute rhythm from there.
    let back = 2 * five + 5;
    assert_eq!(frames(&mut p, t0, &mut n, back, back + 10, true, &links()), vec![back]);
}

#[test]
fn linking_a_library_asks_at_once_and_no_libraries_ask_nothing() {
    let (t0, mut p, mut n) = (Instant::now(), Poll::default(), 0);
    assert!(frames(&mut p, t0, &mut n, 0, 30, true, &[]).is_empty(), "no team library: nothing to ask");
    assert_eq!(frames(&mut p, t0, &mut n, 30, 40, true, &links()), vec![30], "a library linked: asked at once");
    let two = vec![links()[0].clone(), "figcraft://join?server=wss%3A%2F%2Fexample.org&file=other".to_string()];
    assert_eq!(frames(&mut p, t0, &mut n, 40, 50, true, &two), vec![40], "another one: at once again");
}

#[test]
fn a_panel_flicked_back_and_forth_asks_once() {
    let (t0, mut p, mut n) = (Instant::now(), Poll::default(), 0);
    assert_eq!(frames(&mut p, t0, &mut n, 0, 2, true, &links()), vec![0]);
    // Hidden and shown again every few seconds (or another window's frames in between).
    for s in [5, 10, 15, 20] {
        n += 3;
        assert!(frames(&mut p, t0, &mut n, s, s + 2, true, &links()).is_empty(), "shown again at {s} s");
    }
}
