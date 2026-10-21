//! A pause lasts only while an agent works (Fix N, `docs/design/follow-agent.md` › Fix N).
//! - **Shown:** the "Follow agent" chip at the top of the canvas, and the AI tab's paused row with
//!   its Resume, only while following is paused and the working state is on ([`shown`]).
//! - **When the work ends for every client** (finish_working, the grace period, a session closing
//!   or a disconnect), the pause is cleared ([`settle`]): the next build is followed from its first
//!   edit, and the camera stays where the person left it until then. The chip and the row fade out
//!   over [`FADE`] (at once with Animations Off or reduced motion); a faded chip has no hit area
//!   and no AccessKit node.
//! - **A resume** (the chip, Resume, `camera.follow-agents`) clears it as before.
//! - Each clearing is a `follow` event in Settings ▸ Diagnostics, with its cause.
//! - Window state only: the saved "Follow agents" setting is never changed, and nothing reaches the
//!   document, undo, the change log, MCP results, screenshots or exports.

use egui::{CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use figcraft_engine::cmd::camera::follow_state;
use figcraft_engine::settings::diagnostics::{Cause, Cleared, Entry};
use serde_json::json;

use super::{agent_now, now};
use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::theme::{Tokens, ink, semibold};

/// The chip's and the paused row's fade out when the work they paused ends.
pub const FADE: f64 = 0.15;

/// The fade out in progress: when it began, and where the chip was.
#[derive(Default, Debug)]
pub struct State {
    fade: Option<(f64, Option<Rect>)>,
}

/// The chip and the paused row show: following is paused, and an agent is working.
pub fn shown(app: &FigApp) -> bool {
    follow_state(&app.session) == "paused" && app.ui.agent_view.working.working()
}

/// The fade out's opacity at `now` (1 → 0 over [`FADE`]), while it runs.
pub fn fading(app: &FigApp, now: f64) -> Option<f32> {
    let (start, _) = app.ui.agent_view.end.fade?;
    let a = 1.0 - (now - start) / FADE;
    (a > 0.0).then_some(a as f32)
}

/// Every frame, after the working state's tick (`ended`: the cause of the last client's end this
/// tick, if one ended): a pause whose work has ended is cleared.
pub fn settle(app: &mut FigApp, now: f64, ended: Option<Cause>) {
    if !app.session.agents.follow_paused || app.ui.agent_view.working.working() {
        return;
    }
    app.session.agents.follow_paused = false;
    let why = match ended {
        Some(Cause::GraceExpired) => Cleared::GraceExpired,
        Some(Cause::SessionClosed | Cause::Disconnected) => Cleared::Disconnected,
        _ => Cleared::AgentFinished,
    };
    crate::settings_privacy::record(app, Entry::FollowCleared(why));
    let chip = app.ui.agent_view.follow.chip;
    let f = &mut app.ui.agent_view.follow;
    // Following again from the next edit, not a resume: the camera stays where the person put it.
    f.was = "following";
    f.pending = None;
    f.expected = None;
    f.paused_by = None;
    app.ui.agent_view.end.fade = (!app.reduce_motion()).then_some((now, chip));
}

/// For `ui.inspect` › `agent.chip`: shown (pressable), or fading with its opacity, else null.
pub fn inspect(app: &FigApp) -> serde_json::Value {
    match (shown(app), fading(app, now(app))) {
        (true, _) => json!({"shown": true, "rect": app.ui.agent_view.follow.chip.map(|r| [r.min.x, r.min.y, r.max.x, r.max.y])}),
        (false, Some(a)) => json!({"shown": false, "fading": a}),
        (false, None) => serde_json::Value::Null,
    }
}

/// A resume (`camera.follow-agents` while paused): noted in Settings ▸ Diagnostics.
pub fn resumed(app: &mut FigApp) {
    app.ui.agent_view.end.fade = None;
    crate::settings_privacy::record(app, Entry::FollowCleared(Cleared::Resumed));
}

/// What the chip does, for its tooltip and AccessKit name: "Jump back to Codex's work".
pub fn chip_action(app: &FigApp) -> String {
    match agent_now(app) {
        Some(name) => trf("Jump back to {agent}'s work", &[("agent", &name)]),
        None => tr("Jump back to the agent's work").to_string(),
    }
}

/// The paused chip at the top of the canvas: one click resumes. Fading out, it is only painted.
pub fn chip(app: &mut FigApp, ui: &mut Ui) {
    app.ui.agent_view.follow.chip = None;
    let now = now(app);
    let fade = fading(app, now);
    if !shown(app) && fade.is_none() {
        app.ui.agent_view.end.fade = None;
        return;
    }
    if app.canvas.rect.width() < 200.0 {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let label = tr("Follow agent");
    let galley = ui.painter().layout_no_wrap(label.to_string(), semibold(11.0), t.text);
    let size = vec2(galley.size().x + 34.0, 26.0);
    let at = Rect::from_center_size(pos2(app.canvas.rect.center().x, app.canvas.rect.min.y + 12.0 + size.y / 2.0), size);
    let (r, a, resp) = match fade {
        // Shown: it can be pressed, and it is in the AccessKit tree.
        None => {
            let action = chip_action(app);
            let resp = ui.interact(at, Id::new("follow_agent_chip"), Sense::click()).on_hover_text(&action);
            crate::a11y::button(&resp, &action);
            (at, 1.0, Some(resp))
        }
        Some(a) => {
            ui.ctx().request_repaint();
            (app.ui.agent_view.end.fade.and_then(|(_, r)| r).unwrap_or(at), a, None)
        }
    };
    let hovered = resp.as_ref().is_some_and(egui::Response::hovered);
    let p = ui.painter();
    p.rect_filled(r.translate(vec2(0.0, 1.0)), CornerRadius::same(13), egui::Color32::from_black_alpha(24).gamma_multiply(a));
    p.rect_filled(r, CornerRadius::same(13), if hovered { t.hover } else { t.panel }.gamma_multiply(a));
    p.rect_stroke(r, CornerRadius::same(13), Stroke::new(1.0, t.border.gamma_multiply(a)), egui::StrokeKind::Inside);
    p.circle_filled(pos2(r.min.x + 13.0, r.center().y), 4.0, ink::fill().gamma_multiply(a));
    p.galley_with_override_text_color(pos2(r.min.x + 23.0, r.center().y - galley.size().y / 2.0), galley, t.text.gamma_multiply(a));
    let Some(resp) = resp else { return };
    if resp.has_focus() {
        p.rect_stroke(r, CornerRadius::same(13), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    app.ui.agent_view.follow.chip = Some(r);
    if resp.clicked() {
        app.run("camera.follow-agents", json!({"on": true}));
    }
}

#[cfg(test)]
#[path = "follow_end_tests.rs"]
mod tests;
