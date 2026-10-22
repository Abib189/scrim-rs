//! Where following puts the camera (Fix T2, `docs/design/follow-agent.md` › Fix T2): the agent's
//! latest work in the middle of the canvas, the area between the panels.
//! - **The target:** the centre of the work's bounds goes to the canvas's centre.
//! - **The zoom is kept** while the bounds fit with a [`MARGIN`] of the canvas on every side;
//!   otherwise it goes down just enough to fit them so, never up, and never under the minimum zoom.
//! - **A dead zone:** within [`DEAD_ZONE`] screen pixels of the target, with the zoom unchanged, the
//!   camera stays (no small shifts between small batches).
//! - **The flight** lasts from [`camera_flight::FLIGHT_SECS`] (320 ms) for a short move up to
//!   [`MAX_FLIGHT_SECS`] for a move of [`FAR`] canvas widths or more, so a far jump stays readable.
//!
//! - **Which work:** the layers the agent changed since the last flight (a batch of [`super::MANY`]
//!   or more: their artboards); when they span several artboards, the artboard of the latest edit.
//!
//! `camera.focus-layer` (a Layers click) keeps D2's least move; only following centres. Resume flies
//! the same way, to the latest work.

use std::collections::BTreeSet;

use figcraft_doc::NodeId;
use figcraft_doc::geom::Rect;
use figcraft_engine::ViewState;
use figcraft_engine::settings::diagnostics::Mover;
use serde_json::json;

use super::{camera_log, view_now};
use crate::FigApp;
use crate::camera_flight::FLIGHT_SECS;

/// The margin left on every side of the work, as a share of the canvas.
pub const MARGIN: f64 = 0.10;
/// Closer than this (screen pixels) to the target, with the zoom unchanged, the camera stays.
pub const DEAD_ZONE: f64 = 16.0;
/// The longest flight, in seconds.
pub const MAX_FLIGHT_SECS: f64 = 0.6;
/// A move this many canvas widths long, or longer, takes the longest flight.
pub const FAR: f64 = 3.0;

/// The view (centre and zoom) that puts `r` in the middle of the canvas `view` shows.
pub fn centre(view: &ViewState, r: Rect) -> (f64, f64, f64) {
    let zoom = view.zoom.max(1e-9);
    let room = 1.0 - 2.0 * MARGIN;
    let fit = (view.vw * room / r.width().max(1e-9)).min(view.vh * room / r.height().max(1e-9));
    let z = if fit.is_finite() && fit < zoom { fit.max(ViewState::MIN_ZOOM).min(zoom) } else { zoom };
    (r.center().x, r.center().y, z)
}

/// Whether moving from `view` to `to` is worth a flight: the zoom changes, or the centre moves
/// more than [`DEAD_ZONE`] on screen.
pub fn worth_moving(view: &ViewState, to: (f64, f64, f64)) -> bool {
    let (cx, cy, z) = to;
    let shift = ((cx - view.cx).powi(2) + (cy - view.cy).powi(2)).sqrt() * view.zoom;
    z != view.zoom || shift > DEAD_ZONE
}

/// How long the flight from `view` to `to` lasts: by the distance in canvas widths at the lower of
/// the two zooms, from [`FLIGHT_SECS`] up to [`MAX_FLIGHT_SECS`] at [`FAR`] widths.
pub fn flight_secs(view: &ViewState, to: (f64, f64, f64)) -> f64 {
    let (cx, cy, z) = to;
    let widths = ((cx - view.cx).powi(2) + (cy - view.cy).powi(2)).sqrt() * view.zoom.min(z) / view.vw.max(1.0);
    FLIGHT_SECS + (MAX_FLIGHT_SECS - FLIGHT_SECS) * (widths / FAR).clamp(0.0, 1.0)
}

/// The bounds to centre: `ids`, or the latest edit's artboard when they span several artboards.
fn bounds(app: &FigApp, page: NodeId, ids: &BTreeSet<NodeId>) -> Option<Rect> {
    let doc = app.session.doc().ok()?;
    let boards: BTreeSet<NodeId> = ids.iter().map(|id| doc.top_level_frame(*id).unwrap_or(*id)).collect();
    let latest = app.ui.agent_view.follow.board.filter(|b| doc.page_of(*b) == Some(page) && boards.contains(b));
    match latest {
        Some(board) if boards.len() > 1 => doc.bounds_of(&[board]),
        _ => doc.bounds_of(&ids.iter().copied().collect::<Vec<_>>()),
    }
}

/// Follow's move to the work `ids` on `page`: centred, with the eased flight (a jump with reduced
/// motion, or to another page). It moves the view only: no revision, journal entry or undo step.
pub(super) fn fly(app: &mut FigApp, page: NodeId, ids: &BTreeSet<NodeId>, now: f64) {
    let f = &mut app.ui.agent_view.follow;
    f.pending = None;
    f.last_flight = Some(now);
    let Some(r) = bounds(app, page, ids) else { return };
    let Ok(ds) = app.session.ds_mut() else { return };
    let switched = ds.page != page;
    let to = centre(&ds.view, r);
    if !switched && !worth_moving(&ds.view, to) {
        return;
    }
    let from = json!({"cx": ds.view.cx, "cy": ds.view.cy, "zoom": ds.view.zoom});
    let secs = flight_secs(&ds.view, to);
    if switched {
        ds.page = page;
    }
    (ds.view.cx, ds.view.cy, ds.view.zoom) = to;
    let target = view_now(app);
    crate::camera_flight::request(app, &json!({"from": from, "switched": switched, "secs": secs}));
    camera_log::note(app, Mover::Follow, "");
    let f = &mut app.ui.agent_view.follow;
    f.expected = target;
    f.flying = app.ui.camera.flying();
}

#[cfg(test)]
#[path = "follow_centre_tests.rs"]
mod tests;
