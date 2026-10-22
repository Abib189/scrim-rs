//! The working state kept up to date between frames (R3a, `docs/design/working-state.md` › R3a).
//! - **Processed and logged at once:** after every agent request (`control::handle_route`), so a
//!   `begin`, and the `end` that `finish_working` causes, reach Settings ▸ Diagnostics as they
//!   happen, not when the window next draws. Each frame does the same ([`settle`]).
//! - **Without frames:** a hidden window (minimised or covered) draws none, but the desktop app's
//!   `logic` still runs ([`logic`]). A wake-up is asked for at the next grace period's end, so the
//!   state ends then, drawn or not.
//! - **The clock:** while the window is hidden, egui's clock stays at the last frame drawn, so it
//!   is carried on from that frame by the wall clock ([`clock`]). It never goes back.
//! - **Every transition wakes the window** (a repaint), so what it shows catches up.
//! - **No napping while agents work:** the platform is asked to keep the app awake from the
//!   state's start to its end (`Services::keep_awake`; macOS App Nap).

use egui::Context;
use figcraft_engine::settings::diagnostics::{Cause, Change, Entry};
use web_time::Instant;

use crate::FigApp;

/// The working state's clock, in egui's seconds.
#[derive(Debug, Default)]
pub struct Clock {
    /// The last frame drawn: its time, and when that was.
    pub(crate) frame: Option<(f64, Instant)>,
    /// The latest reading.
    last: f64,
}

/// The window draws no frames now (minimised or covered): egui's clock is the last frame's.
fn hidden(ctx: &Context) -> bool {
    ctx.input(|i| i.viewport().visible()) == Some(false)
}

/// A frame is being drawn at `now` (egui's time): the clock carries on from it while hidden.
pub fn frame(app: &mut FigApp, now: f64) {
    app.ui.agent_view.clock.frame = Some((now, Instant::now()));
}

/// The time now: egui's while the window draws, else the last frame's plus the time since.
pub fn clock(app: &mut FigApp, ctx: &Context) -> f64 {
    let t = ctx.input(|i| i.time);
    let c = &mut app.ui.agent_view.clock;
    let now = match c.frame {
        Some((at, when)) if hidden(ctx) => t.max(at + when.elapsed().as_secs_f64()),
        _ => t,
    };
    c.last = c.last.max(now);
    c.last
}

/// The clock's latest reading (the request being answered, or the frame being drawn).
pub fn now(app: &FigApp) -> f64 {
    app.ui.agent_view.clock.last
}

/// Bring the working state up to `now`: each transition, with its cause, into Settings ▸
/// Diagnostics; a client back soon after its grace ran out, noted (Fix Q2); the platform asked to
/// keep the app awake while agents work; the window woken when anything changed, and at the next
/// grace period's end.
pub fn settle(app: &mut FigApp, ctx: &Context, now: f64) {
    let (grace, motion) = (super::grace(app), !app.reduce_motion());
    app.ui.agent_view.working.tick(now, grace, motion);
    let changes = app.ui.agent_view.working.transitions();
    for t in &changes {
        // The run's end, for follow's pause to clear on the next frame (Fix N).
        if t.change == Change::End {
            app.ui.agent_view.ended = Some(t.cause);
        }
        let e = Entry::Working { change: t.change, cause: t.cause, client: t.client.as_deref(), session: t.session.as_deref(), run: t.run };
        crate::settings_privacy::record(app, e);
    }
    super::grace_hint::note(app);
    let working = app.ui.agent_view.working.working();
    if working != app.ui.agent_view.awake {
        app.ui.agent_view.awake = working;
        app.services.keep_awake(working);
    }
    if !changes.is_empty() {
        ctx.request_repaint();
    }
    if let Some(left) = app.ui.agent_view.working.next_change(now, grace) {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(left.max(0.0)));
    }
}

/// The app's `logic`, which runs while the window is hidden too: grace periods end on time.
pub fn logic(app: &mut FigApp, ctx: &Context) {
    let now = clock(app, ctx);
    settle(app, ctx, now);
}

/// The run's end since the last frame took it, if it ended (follow's pause clears then).
pub fn take_ended(app: &mut FigApp) -> Option<Cause> {
    app.ui.agent_view.ended.take()
}

#[cfg(test)]
#[path = "working_log_tests.rs"]
mod tests;
