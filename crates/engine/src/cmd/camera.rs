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

use figcraft_doc::NodeId;
use figcraft_doc::geom::Rect;
use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::id_of;
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
    let (cx, cy) = reveal(&ds.view, bounds);
    let moved = cx != ds.view.cx || cy != ds.view.cy;
    ds.view.cx = cx;
    ds.view.cy = cy;
    let view = json!({"cx": cx, "cy": cy, "zoom": ds.view.zoom});
    let ids: Vec<u64> = ids.iter().map(|i| i.0).collect();
    s.request_ui(FLY_REQUEST, json!({"from": from, "ids": ids, "switched": switched}));
    Ok(json!({"moved": moved, "page": page.0, "switched": switched, "hidden": hidden, "view": view}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "camera.focus-layer",
        "Focus layer",
        "{id? | ids? (default: the selection)} → {moved, page, switched, hidden: [ids], view: {cx, cy, zoom}}  (the least camera move that brings the layers into view with a 32 px margin, the zoom kept; a layer bigger than the view gets its leading edge at the margin; another page is shown first; hidden layers are focused and listed)",
        false,
        focus
    )]
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
