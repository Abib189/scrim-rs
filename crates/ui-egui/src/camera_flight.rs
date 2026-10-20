//! The camera's flight to a focused layer (D2, `docs/design/layers-collapse-and-focus.md`).
//!
//! `camera.focus-layer` sets the camera at once; the window then animates there from where it was:
//! 320 ms, ease-in-out (the old Scrim's), the zoom eased geometrically.
//! - **A jump** when Settings ▸ Animations is off, the system asks for reduced motion, or the page
//!   changed.
//! - **Input cancels it:** if anything else moves the camera mid-flight (a scroll, pan, zoom or
//!   drag), the flight stops there and the camera stays where the person put it. Each frame
//!   compares the view with what the flight last wrote, so no input path needs to know about it.
//! - **Never before the canvas has its size:** a focus asked for before the first sizing (whose page
//!   fit would overwrite it) is kept and applied once the canvas knows its size.

use figcraft_doc::NodeId;
use figcraft_engine::cmd::camera::reveal;
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::trf;

/// How long a flight lasts, in seconds.
pub const FLIGHT_SECS: f64 = 0.32;

/// A flight in progress, in the document `uid`.
#[derive(Clone, Debug)]
struct Flight {
    uid: u64,
    from: [f64; 3],
    to: [f64; 3],
    /// When the first frame drew it (set by the first tick).
    start: Option<f64>,
    /// What this flight last wrote: anything else in the view means someone else moved it.
    last: [f64; 3],
}

/// The window's camera flight, and a focus waiting for the canvas's size.
#[derive(Default, Debug)]
pub struct State {
    flight: Option<Flight>,
    pending: Option<(u64, Vec<NodeId>)>,
}

impl State {
    /// A flight is in progress.
    pub fn flying(&self) -> bool {
        self.flight.is_some()
    }

    /// Stop the flight where the camera is now (following paused mid-flight).
    pub fn stop(&mut self) {
        self.flight = None;
    }
}

fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// Whether the canvas has had its real size in this document (the first sizing re-fits the page).
pub(crate) fn sized(app: &FigApp) -> bool {
    let r = app.canvas.rect;
    r.width() > 1.0
        && app.session.ds().is_ok_and(|ds| (ds.view.vw - f64::from(r.width())).abs() < 0.5 && (ds.view.vh - f64::from(r.height())).abs() < 0.5)
}

/// `camera.fly` from `camera.focus-layer`: animate from `from` to where the camera is now.
pub fn request(app: &mut FigApp, data: &Value) {
    let Ok(ds) = app.session.ds() else { return };
    let uid = ds.uid;
    if !sized(app) {
        let ids = data["ids"].as_array().into_iter().flatten().filter_map(figcraft_engine::params::id_of).collect();
        app.ui.camera.pending = Some((uid, ids));
        return;
    }
    let to = [ds.view.cx, ds.view.cy, ds.view.zoom];
    let f = &data["from"];
    let from = [f["cx"].as_f64().unwrap_or(to[0]), f["cy"].as_f64().unwrap_or(to[1]), f["zoom"].as_f64().unwrap_or(to[2])];
    if from == to || data["switched"] == true || app.reduce_motion() {
        app.ui.camera.flight = None;
        return;
    }
    app.ui.camera.flight = Some(Flight { uid, from, to, start: None, last: from });
    set_view(app, uid, from);
}

fn set_view(app: &mut FigApp, uid: u64, v: [f64; 3]) {
    if let Some(ds) = app.session.docs.iter_mut().find(|d| d.uid == uid) {
        ds.view.cx = v[0];
        ds.view.cy = v[1];
        ds.view.zoom = v[2];
    }
}

/// Every frame, after the canvas: a waiting focus once the canvas has its size, then the flight.
pub fn tick(app: &mut FigApp, ctx: &egui::Context) {
    if let Some((uid, ids)) = app.ui.camera.pending.clone()
        && sized(app)
    {
        app.ui.camera.pending = None;
        if let Some(ds) = app.session.docs.iter_mut().find(|d| d.uid == uid)
            && let Some(r) = ds.doc.bounds_of(&ids)
        {
            let (cx, cy) = reveal(&ds.view, r);
            ds.view.cx = cx;
            ds.view.cy = cy;
        }
    }
    let Some(f) = app.ui.camera.flight.clone() else { return };
    let Some(ds) = app.session.docs.iter().find(|d| d.uid == f.uid) else {
        app.ui.camera.flight = None;
        return;
    };
    let now_view = [ds.view.cx, ds.view.cy, ds.view.zoom];
    if now_view != f.last || app.session.ds().map(|d| d.uid).ok() != Some(f.uid) {
        // Someone else moved the camera (or the document changed): stay where they put it.
        app.ui.camera.flight = None;
        return;
    }
    let now = ctx.input(|i| i.time);
    let start = f.start.unwrap_or(now);
    let t = ((now - start) / FLIGHT_SECS).clamp(0.0, 1.0);
    let e = ease_in_out_cubic(t);
    let v = if t >= 1.0 {
        f.to
    } else {
        let zoom = f.from[2] * (f.to[2] / f.from[2]).powf(e);
        [f.from[0] + (f.to[0] - f.from[0]) * e, f.from[1] + (f.to[1] - f.from[1]) * e, zoom]
    };
    set_view(app, f.uid, v);
    app.ui.camera.flight = (t < 1.0).then_some(Flight { last: v, start: Some(start), ..f });
    ctx.request_repaint();
}

/// A Layers panel click (or Enter on a row): focus the selection, the union when several are
/// selected. A hidden layer is focused anyway and the toast says it is hidden.
pub fn focus_selection(app: &mut FigApp) {
    if app.session.selection().is_empty() {
        return;
    }
    let Some(r) = app.run("camera.focus-layer", json!({})) else { return };
    let hidden: Vec<NodeId> = r["hidden"].as_array().into_iter().flatten().filter_map(figcraft_engine::params::id_of).collect();
    if let Some(first) = hidden.first() {
        let name = app.session.doc().ok().and_then(|d| d.node(*first).map(|n| n.name.clone())).unwrap_or_default();
        let msg = if hidden.len() == 1 {
            trf("“{name}” is hidden", &[("name", &name)])
        } else {
            trf("{n} of these layers are hidden", &[("n", &hidden.len())])
        };
        app.toast(msg);
    }
}

#[cfg(test)]
#[path = "camera_flight_tests.rs"]
mod tests;
