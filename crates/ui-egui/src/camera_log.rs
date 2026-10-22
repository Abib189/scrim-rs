//! Who moved the camera (Fix S4). Each change of the view (its document, page, centre or zoom) is
//! noted where it's made, with where that was ([`Mover`]):
//! - an agent's request, with its command or method (`control::handle_agent`);
//! - a flight's frame, an agent's, following's or the person's (`camera_flight::tick`);
//! - following's own move (`follow_agent::fly`);
//! - the canvas: its sizing when its size changed, else the person's input (after `canvas::show`);
//! - anything else, found where following looks (the start of `before_canvas`): the window outside
//!   the canvas, a shortcut, a menu, a panel, a tab or a collaborator's view.
//!
//! The last [`KEEP`] changes are in `ui.inspect` (`agent.cameraLog`), and a pause for a camera move
//! names the change behind it in Settings ▸ Diagnostics. Window state only: nothing is written to
//! the document, and nothing is kept between launches.

use std::collections::VecDeque;

use figcraft_doc::NodeId;
use figcraft_engine::settings::diagnostics::Mover;
use serde_json::{Value, json};

use crate::FigApp;

/// How many changes are kept.
pub const KEEP: usize = 24;

/// A view: document, page, centre and zoom.
pub type View = (u64, NodeId, [f64; 3]);

/// One change, or a run of them from the same place (a flight's frames).
#[derive(Clone, Debug)]
struct Change {
    at: f64,
    by: Mover,
    /// The agent's command or method; empty otherwise.
    what: String,
    to: Option<View>,
    count: u32,
}

#[derive(Default)]
pub struct Log {
    /// The view as last noted.
    last: Option<View>,
    /// The canvas's view size before the canvas drew this frame.
    size: Option<(f64, f64)>,
    changes: VecDeque<Change>,
}

fn view_now(app: &FigApp) -> Option<View> {
    app.session.ds().ok().map(|ds| (ds.uid, ds.page, [ds.view.cx, ds.view.cy, ds.view.zoom]))
}

/// The view changed since it was last noted? Then `by` changed it.
pub fn note(app: &mut FigApp, by: Mover, what: &str) {
    let view = view_now(app);
    let at = super::working_log::now(app);
    let log = &mut app.ui.agent_view.cameras;
    if view == log.last {
        return;
    }
    log.last = view;
    match log.changes.back_mut() {
        Some(c) if c.by == by && c.what == what => {
            c.at = at;
            c.to = view;
            c.count += 1;
        }
        _ => {
            log.changes.push_back(Change { at, by, what: what.to_string(), to: view, count: 1 });
            if log.changes.len() > KEEP {
                log.changes.pop_front();
            }
        }
    }
}

/// Before following looks: what changed since the last note was made outside the canvas; then the
/// canvas's size, to tell its sizing from the person's input.
pub fn before_canvas(app: &mut FigApp) {
    note(app, Mover::Window, "");
    app.ui.agent_view.cameras.size = app.session.ds().ok().map(|ds| (ds.view.vw, ds.view.vh));
}

/// After the canvas drew: its sizing, or the person's input on it.
pub fn after_canvas(app: &mut FigApp) {
    let size = app.session.ds().ok().map(|ds| (ds.view.vw, ds.view.vh));
    let by = if size == app.ui.agent_view.cameras.size { Mover::Canvas } else { Mover::Sizing };
    note(app, by, "");
}

/// Who made the change that brought the camera to where it is now.
pub fn mover(app: &FigApp) -> Mover {
    let log = &app.ui.agent_view.cameras;
    log.changes.iter().rev().find(|c| c.to == log.last).map_or(Mover::Window, |c| c.by)
}

/// For `ui.inspect`: the kept changes, oldest first.
pub fn inspect(app: &FigApp) -> Value {
    let changes = app.ui.agent_view.cameras.changes.iter().map(|c| {
        json!({
            "at": c.at,
            "by": c.by.text(),
            "what": c.what,
            "to": c.to.map(|(uid, page, v)| json!({"document": uid, "page": page.0, "cx": v[0], "cy": v[1], "zoom": v[2]})),
            "count": c.count,
        })
    });
    Value::Array(changes.collect())
}
