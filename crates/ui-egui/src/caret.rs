//! The text caret's blink (Fix AA5), on Scrim's clock rather than egui's: half a second on, half
//! off, restarting at each key or click so the caret shows while you type; steady with reduced
//! motion.
//!
//! **Why:** egui's own blink asks to be woken when the caret next changes, but egui takes a frame's
//! time off every delay. It woke a frame early, found the caret unchanged, asked again at once, and
//! an immediate request draws two frames. A focused field drew about 7 frames a second in the real
//! app for a caret that changes twice. Here the wake-up lands just after each change.

use egui::{Context, Id, Visuals};

/// How long the caret shows, then hides.
const ON: f64 = 0.5;
const OFF: f64 = 0.5;
/// How long after a change to wake: more than the frame egui takes off a delay.
const LATE: f64 = 0.025;

fn start_id() -> Id {
    Id::new("scrim_caret_blink")
}

/// Set this frame's caret on `v` (egui's blink off; the caret hidden in the off half) and ask for
/// the frame of its next change.
pub fn blink(ctx: &Context, v: &mut Visuals, reduce_motion: bool) {
    v.text_cursor.blink = false;
    if reduce_motion || !ctx.text_edit_focused() {
        return;
    }
    let now = ctx.input(|i| i.time);
    let focus = ctx.memory(|m| m.focused());
    let typed = ctx.input(|i| !i.events.is_empty());
    let start = ctx.data_mut(|d| {
        let s = d.get_temp_mut_or_insert_with(start_id(), || (focus, now));
        if typed || s.0 != focus {
            *s = (focus, now);
        }
        s.1
    });
    let phase = (now - start).rem_euclid(ON + OFF);
    let shown = phase < ON;
    if !shown {
        v.text_cursor.stroke.color = v.text_cursor.stroke.color.gamma_multiply(0.0);
    }
    let next = if shown { ON - phase } else { ON + OFF - phase };
    ctx.request_repaint_after_secs((next + LATE) as f32);
}
