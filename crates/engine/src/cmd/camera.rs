//! Focus a layer (D2, `docs/design/layers-collapse-and-focus.md`): the camera moves the least it
//! can to bring layers into view, the way the old Scrim's Layers panel did.
//! - **The zoom is kept,** and a 32 px margin (on screen) is left around the layers.
//! - **Already in view:** the camera doesn't move. Otherwise it moves only as far as needed on each
//!   axis.
//! - **Bigger than the view** on an axis: its leading (left or top) edge goes to the margin.
//! - **Another page:** that page is shown first.
//! - **A hidden layer** is focused anyway, and the answer says so.
//!
//! The camera is set at once, so agents and headless runs are deterministic. The window then
//! animates from where it was (`FLY_REQUEST`, `camera_flight` in the UI).
//!
//! With `fit` (Stage I's follow, `docs/design/follow-agent.md`), layers that don't fit at the
//! current zoom are fitted instead: the zoom goes down just enough, never up.
//! `camera.follow-agents` turns following agents' edits on or off, or resumes it.
//! `pref.revealAgentWork` and `pref.workingIndicator` switch the canvas's reveal of an agent's new
//! layers and its working indicator (Stage I2); `pref.revealSpeed` sets the reveal's pace (I.1).

use figcraft_doc::NodeId;
use figcraft_doc::geom::Rect;
use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::{boolean, id_of};
use crate::{EngineError, Result, Session, ViewState, bad, spec};

/// The margin left around a focused layer, in screen pixels.
pub const REVEAL_MARGIN: f64 = 32.0;
/// The UI request the window animates: `{from: {cx, cy, zoom}, ids, switched}`.
pub const FLY_REQUEST: &str = "camera.fly";

/// The view centre that brings `r` (page space) into `view` with the least move, the zoom kept.
pub fn reveal(view: &ViewState, r: Rect) -> (f64, f64) {
    let zoom = view.zoom.max(1e-9);
    let axis = |c: f64, screen: f64, lo: f64, hi: f64| -> f64 {
        let half = screen / 2.0 / zoom;
        let m = (REVEAL_MARGIN / zoom).min(half);
        let (vis_lo, vis_hi) = (c - half + m, c + half - m);
        if hi - lo > vis_hi - vis_lo {
            // Too big: its leading edge at the margin.
            lo + half - m
        } else if lo < vis_lo {
            c - (vis_lo - lo)
        } else if hi > vis_hi {
            c + (hi - vis_hi)
        } else {
            c
        }
    };
    (axis(view.cx, view.vw, r.x0, r.x1), axis(view.cy, view.vh, r.y0, r.y1))
}

/// The view (centre and zoom) that shows `r` (Stage I's follow):
/// - **Partly in view and it fits:** [`reveal`], the least move, the zoom kept.
/// - **Out of view entirely:** centred on it (a least move would leave it on the far edge).
/// - **Too big at this zoom:** the zoom that fits it with the margin (lower, never higher), centred.
pub fn reveal_or_fit(view: &ViewState, r: Rect) -> (f64, f64, f64) {
    let zoom = view.zoom.max(1e-9);
    let room = |screen: f64| (screen - 2.0 * REVEAL_MARGIN).max(1.0);
    let fit = (room(view.vw) / r.width().max(1e-9)).min(room(view.vh) / r.height().max(1e-9));
    if fit >= zoom || !fit.is_finite() {
        let (hw, hh) = (view.vw / 2.0 / zoom, view.vh / 2.0 / zoom);
        let seen = r.x1 >= view.cx - hw && r.x0 <= view.cx + hw && r.y1 >= view.cy - hh && r.y0 <= view.cy + hh;
        if !seen {
            return (r.center().x, r.center().y, zoom);
        }
        let (cx, cy) = reveal(view, r);
        return (cx, cy, zoom);
    }
    let z = fit.clamp(ViewState::MIN_ZOOM, zoom);
    (r.center().x, r.center().y, z)
}

/// Whether `id` or one of its ancestors is hidden.
fn hidden(doc: &figcraft_doc::Document, id: NodeId) -> bool {
    std::iter::once(id).chain(doc.ancestors(id)).any(|i| doc.node(i).is_some_and(|n| !n.visible))
}

fn focus(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "camera.focus-layer";
    let mut ids: Vec<NodeId> = match (p.get("id"), p.get("ids")) {
        (Some(v), _) => vec![id_of(v).ok_or_else(|| bad(cmd, "`id` is a layer id"))?],
        (None, Some(Value::Array(a))) => a.iter().map(|v| id_of(v).ok_or_else(|| bad(cmd, "`ids` are layer ids"))).collect::<Result<_>>()?,
        _ => s.selection(),
    };
    if ids.is_empty() {
        return Err(bad(cmd, "nothing to focus: pass `id` or `ids`, or select layers"));
    }
    let doc = s.doc()?.clone();
    if let Some(missing) = ids.iter().find(|i| doc.node(**i).is_none()) {
        return Err(EngineError::NoNode(*missing));
    }
    // The first layer's page: shown first when it isn't the current one; layers elsewhere are left out.
    let page = doc.page_of(ids[0]).filter(|pg| doc.pages.contains(pg)).ok_or_else(|| bad(cmd, "that layer isn't on a page"))?;
    ids.retain(|i| doc.page_of(*i) == Some(page));
    let switched = page != s.page()?;
    let bounds = doc.bounds_of(&ids).ok_or_else(|| bad(cmd, "those layers have no bounds"))?;
    let hidden: Vec<u64> = ids.iter().filter(|i| hidden(&doc, **i)).map(|i| i.0).collect();
    let ds = s.ds_mut()?;
    if switched {
        ds.page = page;
    }
    let from = json!({"cx": ds.view.cx, "cy": ds.view.cy, "zoom": ds.view.zoom});
    let (cx, cy, zoom) = if boolean(p, "fit") == Some(true) {
        reveal_or_fit(&ds.view, bounds)
    } else {
        let (cx, cy) = reveal(&ds.view, bounds);
        (cx, cy, ds.view.zoom)
    };
    let moved = cx != ds.view.cx || cy != ds.view.cy || zoom != ds.view.zoom;
    ds.view.cx = cx;
    ds.view.cy = cy;
    ds.view.zoom = zoom;
    let view = json!({"cx": cx, "cy": cy, "zoom": ds.view.zoom});
    let ids: Vec<u64> = ids.iter().map(|i| i.0).collect();
    s.request_ui(FLY_REQUEST, json!({"from": from, "ids": ids, "switched": switched}));
    Ok(json!({"moved": moved, "page": page.0, "switched": switched, "hidden": hidden, "view": view}))
}

/// Following agents now: `off`, `paused` (by the person, in the window) or `following`.
pub fn follow_state(s: &Session) -> &'static str {
    match (s.prefs.settings.follow_agents, s.agents.follow_paused) {
        (false, _) => "off",
        (true, true) => "paused",
        (true, false) => "following",
    }
}

fn follow_agents(s: &mut Session, p: &Value) -> Result<Value> {
    // Without `on`: following → off; paused or off → following. The window sees a resume (paused or
    // off, then following) and flies to the latest work.
    let on = boolean(p, "on").unwrap_or(follow_state(s) != "following");
    s.prefs.settings.follow_agents = on;
    s.agents.follow_paused = false;
    Ok(json!({"on": on, "state": follow_state(s)}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "camera.focus-layer",
            "Focus layer",
            "{id? | ids? (default: the selection), fit? (zoom out to fit when they don't fit at this zoom)} → {moved, page, switched, hidden: [ids], view: {cx, cy, zoom}}  (the least camera move that brings the layers into view with a 32 px margin, the zoom kept; a layer bigger than the view gets its leading edge at the margin, or with `fit` the zoom goes down just enough to fit; another page is shown first; hidden layers are focused and listed)",
            false,
            focus
        ),
        spec!(
            "pref.revealAgentWork",
            "Reveal agent work",
            "{on? (default: toggle)} → {on}  (layers an agent creates arrive with a short entrance on the canvas; drawing only, never in the document. Animations Off or reduced motion shows them at once anyway. Saved in Settings)",
            false,
            |s, p| {
                let on = boolean(p, "on").unwrap_or(!s.prefs.settings.reveal_agent_work);
                s.prefs.settings.reveal_agent_work = on;
                Ok(json!({"on": on}))
            }
        ),
        spec!(
            "pref.revealSpeed",
            "Reveal speed",
            "{speed: slow|normal|fast} → {speed}  (how fast an agent's new layers enter on the canvas: fast is 220 ms each, 40 ms apart, a batch within 1.2 s; normal 480 ms, 110 ms, 4 s; slow 700 ms, 180 ms, 6 s. Drawing only. Saved in Settings)",
            false,
            |s, p| {
                let speed =
                    p.get("speed").and_then(Value::as_str).ok_or_else(|| bad("pref.revealSpeed", "`speed` is required: slow, normal or fast"))?;
                if !crate::settings::REVEAL_SPEEDS.contains(&speed) {
                    return Err(bad("pref.revealSpeed", format!("`speed` is slow, normal or fast, not {speed}")));
                }
                s.prefs.settings.reveal_speed = speed.to_string();
                Ok(json!({"speed": speed}))
            }
        ),
        spec!(
            "pref.workingIndicator",
            "Show working indicator",
            "{on? (default: toggle)} → {on}  (the row of dots above the artboard an agent is working on; drawing only. Saved in Settings)",
            false,
            |s, p| {
                let on = boolean(p, "on").unwrap_or(!s.prefs.settings.working_indicator);
                s.prefs.settings.working_indicator = on;
                Ok(json!({"on": on}))
            }
        ),
        spec!(
            "camera.follow-agents",
            "Follow agent",
            "{on? (default: toggle; on while paused resumes)} → {on, state: following|paused|off}  (the window's camera goes to agents' edits as they land, and stays with them; the person's own scroll, pan, zoom, canvas click or edit pauses it until they resume. Saved in Settings)",
            false,
            follow_agents
        ),
    ]
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
