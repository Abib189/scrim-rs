//! The reveal (Stage I2, `docs/design/follow-agent.md`): layers an agent creates arrive with a
//! short, quiet entrance instead of popping in, on the canvas only.
//! - **The entrance:** [`ENTER`] (220 ms), opacity 0 → 1 and scale 96% → 100% about the layer's
//!   centre, ease-out. A batch is staggered in creation order (ascending ids), [`STAGGER`] apart,
//!   the stagger shrunk so the batch ends within [`CAP`] (0.6 s once frames run over budget). Over
//!   [`super::MANY`] layers, only the roots of what was created (their top-level groups) enter.
//! - **A move** by an agent that changed only a layer's position tweens over [`TWEEN`]; other
//!   property edits change at once. **Deleted layers** fade out over [`TWEEN`] when the edit only
//!   deleted (the document before it draws meanwhile).
//! - **Where it plays:** for layers in view when they arrive, or on their way into view (the camera
//!   follows the agent). Layers created off screen are drawn final at once: nothing is ever left
//!   hidden or half-faded.
//! - **Skipped:** the person's edits; undo and redo; opening a file; Settings ▸ "Reveal agent work"
//!   off; Animations Off or reduced motion. **Finished at once** when the window was hidden (a gap
//!   over [`HIDDEN`]) or frames are being dropped (a second gap over [`GAP`] in one reveal). One
//!   long frame (the edit itself, on a big file) only delays it: the reveal is re-timed by the gap.
//! - **Render-only:** the canvas draws [`canvas_doc`], a copy of the document with the revealing
//!   layers' opacity and transform eased (`imbl` shares every other node). Nothing else reads it: the
//!   session's document, hit-testing, selection, undo, the change log, `inspect_document`, MCP
//!   `screenshot` (which renders the session's document), exports and saves always have the final
//!   state at once.

use std::sync::Arc;

use egui::Context;
use figcraft_doc::geom::{Affine, Vec2};
use figcraft_doc::{Document, NodeId};

use super::Landed;
use crate::FigApp;

/// How long a layer's entrance takes (seconds).
pub const ENTER: f64 = 0.22;
/// Between two layers' entrances in a batch.
pub const STAGGER: f64 = 0.04;
/// A batch's entrances all end within this.
pub const CAP: f64 = 1.2;
/// … and within this once frames run over budget (the third degradation step).
pub const CAP_DEGRADED: f64 = 0.6;
/// A move's tween, and a deleted layer's fade.
pub const TWEEN: f64 = 0.16;
/// The scale an entrance starts at.
pub const START_SCALE: f64 = 0.96;
/// A gap between frames longer than this is a stall: the first in a reveal re-times it, a second
/// finishes it (frames are being dropped).
pub const GAP: f64 = 0.25;
/// A gap longer than this: the window was hidden. Finish.
pub const HIDDEN: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Enter,
    /// From this translation to the layer's own.
    Move(Vec2),
}

#[derive(Clone, Copy, Debug)]
struct Item {
    id: NodeId,
    start: f64,
    kind: Kind,
}

/// Deleted layers fading out: drawn from the document before the delete while the session's is
/// still `after`.
struct Fade {
    before: Arc<Document>,
    after: Arc<Document>,
    ids: Vec<NodeId>,
    start: f64,
}

/// The reveals in progress.
#[derive(Default)]
pub struct State {
    items: Vec<Item>,
    /// When the last queued entrance starts.
    last_start: f64,
    fade: Option<Fade>,
    /// This frame's render-only document, and the session document it stands in for.
    frame_doc: Option<(Arc<Document>, Arc<Document>)>,
    last_tick: Option<f64>,
    /// This reveal has had its one stall already.
    stalled: bool,
}

impl State {
    /// Something is being revealed this frame.
    pub fn active(&self) -> bool {
        !self.items.is_empty() || self.fade.is_some()
    }

    /// The layers queued or entering now, in order (for tests and `ui.inspect`).
    pub fn entering(&self) -> impl Iterator<Item = (NodeId, f64)> + '_ {
        self.items.iter().filter(|i| i.kind == Kind::Enter).map(|i| (i.id, i.start))
    }

    /// Stop every reveal: the canvas draws the final state.
    pub fn finish(&mut self) {
        self.items.clear();
        self.fade = None;
        self.frame_doc = None;
    }
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// When each of `ids` (in creation order) starts: [`STAGGER`] apart from `first` (now, or after the
/// last entrance queued), the stagger shrunk so the last ends within `cap` of `now`.
pub fn schedule(ids: &[NodeId], now: f64, first: f64, cap: f64) -> Vec<(NodeId, f64)> {
    let latest_start = now + (cap - ENTER).max(0.0);
    let first = first.max(now).min(latest_start);
    let step = match ids.len() {
        0 | 1 => 0.0,
        n => STAGGER.min((latest_start - first) / (n - 1) as f64),
    };
    ids.iter().enumerate().map(|(i, id)| (*id, first + step * i as f64)).collect()
}

/// What enters of `created`: every layer, or over [`super::MANY`] only those whose parent wasn't
/// created with them (their top-level groups). In creation order.
pub fn entering(doc: &Document, created: &[NodeId]) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = created.iter().copied().filter(|id| doc.node(*id).is_some_and(|n| !n.is_page())).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > super::MANY {
        let all = ids.clone();
        ids.retain(|id| doc.node(*id).and_then(|n| n.parent).is_none_or(|p| all.binary_search(&p).is_err()));
    }
    ids
}

/// Whether two page rectangles touch or overlap (a line, zero wide, counts).
fn meets(a: figcraft_doc::geom::Rect, b: figcraft_doc::geom::Rect) -> bool {
    a.x0 <= b.x1 && a.x1 >= b.x0 && a.y0 <= b.y1 && a.y1 >= b.y0
}

/// The page rectangle in view.
fn view_rect(app: &FigApp) -> Option<(NodeId, figcraft_doc::geom::Rect)> {
    let ds = app.session.ds().ok()?;
    let v = &ds.view;
    let z = v.zoom.max(1e-9);
    let (hw, hh) = (v.vw / 2.0 / z, v.vh / 2.0 / z);
    Some((ds.page, figcraft_doc::geom::Rect::new(v.cx - hw, v.cy - hh, v.cx + hw, v.cy + hh)))
}

/// Every frame, before the canvas: queue what landed, drop what's done, and make this frame's
/// render-only document.
pub fn tick(app: &mut FigApp, ctx: &Context, now: f64, landed: &[Landed]) {
    let animate = app.session.prefs.settings.reveal_agent_work && !app.reduce_motion();
    let cap = if app.ui.agent_view.budget.level >= 3 { CAP_DEGRADED } else { CAP };
    let following = figcraft_engine::cmd::camera::follow_state(&app.session) == "following";
    let view = view_rect(app);
    let Ok(doc) = app.session.ds().map(|d| d.doc.clone()) else {
        app.ui.agent_view.reveal.finish();
        return;
    };
    let before = app.ui.agent_view.before.clone();
    let st = &mut app.ui.agent_view.reveal;
    let gap = st.last_tick.map(|t| now - t).filter(|g| *g > GAP);
    st.last_tick = Some(now);
    match gap {
        Some(g) if g <= HIDDEN && !st.stalled && st.active() => {
            // One long frame (the edit itself): the reveal waits for it.
            st.stalled = true;
            st.last_start += g;
            st.items.iter_mut().for_each(|i| i.start += g);
            if let Some(f) = st.fade.as_mut() {
                f.start += g;
            }
        }
        Some(_) => st.finish(),
        None => {}
    }
    if !animate {
        st.finish();
    }
    // In view now, or on its way into view (the camera follows the agent there).
    let seen =
        |id: NodeId| following || view.is_some_and(|(page, r)| doc.page_of(id) == Some(page) && doc.world_bounds(id).is_some_and(|b| meets(b, r)));
    for l in landed {
        // Any edit ends a delete's fade: the document before it isn't the one before any more.
        st.fade = None;
        if !animate || !l.agent || l.undo {
            continue;
        }
        let new: Vec<NodeId> = entering(&doc, &l.created).into_iter().filter(|id| seen(*id)).collect();
        let first = if st.items.iter().any(|i| i.kind == Kind::Enter) { st.last_start + STAGGER } else { now };
        for (id, start) in schedule(&new, now, first, cap) {
            st.items.retain(|i| i.id != id);
            st.items.push(Item { id, start, kind: Kind::Enter });
            st.last_start = start;
        }
        let Some(before) = before.as_ref() else { continue };
        // Moves: only the position changed.
        if l.changed.len() <= super::MANY {
            for &id in &l.changed {
                let (Some(a), Some(b)) = (before.node(id), doc.node(id)) else { continue };
                if a.transform == b.transform || !seen(id) {
                    continue;
                }
                let mut moved = a.clone();
                moved.transform = b.transform;
                if moved == *b {
                    st.items.retain(|i| i.id != id);
                    st.items.push(Item { id, start: now, kind: Kind::Move(a.transform.translation()) });
                }
            }
        }
        // A delete-only edit: the deleted layers fade out (only their parents changed otherwise).
        let only_deleted = l.created.is_empty()
            && !l.deleted.is_empty()
            && l.changed.iter().all(|c| before.node(*c).is_some_and(|n| n.is_page()) || l.deleted.iter().any(|d| before.ancestors(*d).contains(c)));
        let visible =
            |id: &NodeId| view.is_some_and(|(page, r)| before.page_of(*id) == Some(page) && before.world_bounds(*id).is_some_and(|b| meets(b, r)));
        if only_deleted && l.deleted.iter().any(visible) {
            let roots: Vec<NodeId> =
                l.deleted.iter().copied().filter(|d| before.node(*d).and_then(|n| n.parent).is_none_or(|p| !l.deleted.contains(&p))).collect();
            st.fade = Some(Fade { before: before.clone(), after: doc.clone(), ids: roots, start: now });
        }
    }
    st.items.retain(|i| {
        let dur = if i.kind == Kind::Enter { ENTER } else { TWEEN };
        now < i.start + dur && doc.contains(i.id)
    });
    if st.fade.as_ref().is_some_and(|f| now >= f.start + TWEEN || !Arc::ptr_eq(&f.after, &doc)) {
        st.fade = None;
    }
    if !st.active() {
        st.frame_doc = None;
        st.stalled = false;
        return;
    }
    st.frame_doc = Some((render_doc(st, &doc, now), doc));
    ctx.request_repaint();
}

/// The document the canvas draws this frame: `doc` (or, while deleted layers fade, the document
/// before the delete) with the revealing layers eased.
fn render_doc(st: &State, doc: &Arc<Document>, now: f64) -> Arc<Document> {
    let mut out = st.fade.as_ref().map_or_else(|| doc.clone(), |f| f.before.clone());
    let d = Arc::make_mut(&mut out);
    for it in &st.items {
        let Some(n) = d.node_mut(it.id) else { continue };
        match it.kind {
            Kind::Enter => {
                let e = ease_out_cubic((now - it.start) / ENTER);
                n.opacity *= e as f32;
                let s = START_SCALE + (1.0 - START_SCALE) * e;
                let c = Vec2::new(n.width / 2.0, n.height / 2.0);
                n.transform = n.transform * Affine::translate(c) * Affine::scale(s) * Affine::translate(-c);
            }
            Kind::Move(from) => {
                let e = ease_out_cubic((now - it.start) / TWEEN);
                let to = n.transform.translation();
                n.transform = n.transform.with_translation(from + (to - from) * e);
            }
        }
    }
    if let Some(f) = &st.fade {
        let e = ease_out_cubic((now - f.start) / TWEEN);
        for id in &f.ids {
            if let Some(n) = d.node_mut(*id) {
                n.opacity *= (1.0 - e) as f32;
            }
        }
    }
    out
}

/// The document the canvas draws in place of the session's while a reveal runs (render-only).
pub fn canvas_doc(app: &FigApp) -> Option<Arc<Document>> {
    let (frame, session) = app.ui.agent_view.reveal.frame_doc.as_ref()?;
    let ds = app.session.ds().ok()?;
    Arc::ptr_eq(session, &ds.doc).then(|| frame.clone())
}

#[cfg(test)]
#[path = "reveal_tests.rs"]
mod tests;
