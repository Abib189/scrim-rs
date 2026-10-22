//! The reveal (Stage I2, `docs/design/follow-agent.md`): layers an agent creates arrive with a
//! short, quiet entrance instead of popping in, on the canvas only.
//! - **The entrance:** opacity 0 → 1 and scale 96% → 100% about the layer's centre, ease-out. A
//!   batch is staggered in creation order (ascending ids), the stagger shrunk so the batch ends
//!   within the speed's cap (half that once frames run over budget). Over [`super::MANY`] layers,
//!   only the roots of what was created (their top-level groups) enter.
//! - **The speed** (Settings ▸ General, Stage I.1): [`FAST`] is Stage I2's 220 ms entrance, 40 ms
//!   apart, within 1.2 s; [`NORMAL`] (the default) 480 ms, 110 ms, 4 s; [`SLOW`] 700 ms, 180 ms, 6 s.
//!   A batch landing mid-reveal queues behind the running stagger, so each step reads as its own.
//! - **A move** by an agent that changed only a layer's position tweens over [`TWEEN`]; other
//!   property edits change at once. **Deleted layers** fade out over [`TWEEN`] when the edit only
//!   deleted (the document before it draws meanwhile).
//! - **Where it plays:** for layers in view when they arrive, or on their way into view (the camera
//!   follows the agent). Layers created off screen are drawn final at once: nothing is ever left
//!   hidden or half-faded.
//! - **It waits for following's flight** (Fix T3): entrances queued while following's own flight is
//!   more than [`HOLD_PX`] from where it goes are held (drawn not yet entered) until it is within
//!   that, or [`HOLD_CAP`] at the latest, so they play on screen. A wait the cap ended is kept in
//!   Settings ▸ Diagnostics ([`Entry::RevealWaited`]). Paused, off or hidden: as before.
//! - **Skipped:** the person's edits; undo and redo; opening a file; Settings ▸ "Reveal agent work"
//!   off; Animations Off or reduced motion; edits that landed while the window was hidden (answered
//!   by `control::drain_hidden`; an idle window draws no frames either, so a gap can't tell).
//! - **Long frames:** the frame right after a landing can be long (on a big file the agent's edit
//!   itself takes it): the reveal is re-timed by it, so it still plays. Later in a reveal, one stall
//!   over [`GAP`] re-times it too; a second (frames being dropped) or one over [`HIDDEN`] finishes it.
//! - **Render-only:** the canvas draws [`canvas_doc`], a copy of the document with the revealing
//!   layers' opacity and transform eased (`imbl` shares every other node). Nothing else reads it: the
//!   session's document, hit-testing, selection, undo, the change log, `inspect_document`, MCP
//!   `screenshot` (which renders the session's document), exports and saves always have the final
//!   state at once.
//! - **Diagnostics** (Fix L2, `docs/design/reveal-skip.md`): every agent edit's new layers are
//!   counted, entered or drawn final and why ([`Tally`], `ui.inspect` › `agent.reveals`), and a
//!   burst with skips is kept in Settings ▸ Diagnostics ([`Entry::Reveal`]).

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use egui::Context;
use figcraft_doc::geom::{Affine, Vec2};
use figcraft_doc::{Document, NodeId};
use figcraft_engine::settings::diagnostics::{Entry, Skip};

use super::Landed;
use crate::FigApp;

/// Entrances wait while following's flight is farther than this (screen pixels) from its target…
pub const HOLD_PX: f64 = 40.0;
/// … for this long at most (seconds).
pub const HOLD_CAP: f64 = 0.7;

/// How long a layer's entrance takes (seconds), at [`FAST`].
pub const ENTER: f64 = 0.22;
/// Between two layers' entrances in a batch, at [`FAST`].
pub const STAGGER: f64 = 0.04;
/// A batch's entrances all end within this, at [`FAST`].
pub const CAP: f64 = 1.2;
/// … and within this once frames run over budget (the third degradation step), at [`FAST`]: half
/// the cap, as at every speed.
pub const CAP_DEGRADED: f64 = 0.6;

/// A reveal's pace: each layer's entrance, the stagger between two, and the cap a batch ends within.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Speed {
    pub enter: f64,
    pub stagger: f64,
    pub cap: f64,
}

pub const FAST: Speed = Speed { enter: ENTER, stagger: STAGGER, cap: CAP };
pub const NORMAL: Speed = Speed { enter: 0.48, stagger: 0.11, cap: 4.0 };
pub const SLOW: Speed = Speed { enter: 0.70, stagger: 0.18, cap: 6.0 };

impl Speed {
    /// The setting's speed (`slow`, `normal`, `fast`; anything else is normal).
    pub fn of(setting: &str) -> Speed {
        match setting {
            "slow" => SLOW,
            "fast" => FAST,
            _ => NORMAL,
        }
    }

    /// This speed with the third degradation step taken: the cap halved (the stagger shrinks first;
    /// the entrance itself is kept).
    pub fn degraded(self) -> Speed {
        Speed { cap: self.cap / 2.0, ..self }
    }
}
/// A move's tween, and a deleted layer's fade.
pub const TWEEN: f64 = 0.16;
/// The scale an entrance starts at.
pub const START_SCALE: f64 = 0.96;
/// A gap between frames longer than this is a stall: the first in a reveal re-times it, a second
/// finishes it (frames are being dropped).
pub const GAP: f64 = 0.25;
/// A stall longer than this in a running reveal: the window was hidden. Finish.
pub const HIDDEN: f64 = 1.0;
/// The frame right after a landing is re-timed up to this long; longer, it finishes.
pub const LANDING: f64 = 5.0;

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
    /// How long it takes (an entrance keeps the speed it was queued at).
    dur: f64,
    kind: Kind,
    /// Waiting for following's flight (Fix T3): drawn not yet entered, its start not yet set.
    held: bool,
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
    /// This reveal has had its one stall already.
    stalled: bool,
    /// Entrances were queued in the frame before this one (the gap after it is the landing's).
    fresh: bool,
    /// How many of the agents' new layers entered, and why the others didn't (Fix L2).
    diag: Diagnostics,
    /// Since when entrances have waited for following's flight (Fix T3).
    wait: Option<f64>,
}

/// One agent edit's new layers: how many entered, entered with their group (over
/// [`super::MANY`]), or were drawn final at once, and why.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub at: f64,
    pub created: usize,
    pub entered: usize,
    pub with_group: usize,
    pub skipped: BTreeMap<Skip, usize>,
    pub follow: &'static str,
}

/// The reveal's counters since the window opened and the last [`RECENT`] edits' tallies, for
/// `ui.inspect` › `agent.reveals`; and what the next diagnostics event will say (Fix L2). Nothing in
/// the interface reads them.
#[derive(Default)]
pub struct Diagnostics {
    pub entered: usize,
    pub skipped: BTreeMap<Skip, usize>,
    pub recent: VecDeque<Tally>,
    /// Since the last diagnostics event: the counts, when the first skip of them was, and when the
    /// last edit was counted.
    pending: (usize, BTreeMap<Skip, usize>),
    pending_since: Option<f64>,
    last_counted: f64,
}

/// How many edits' tallies `ui.inspect` keeps.
pub const RECENT: usize = 16;
/// A `reveal` diagnostics event is written once the agents' edits have been quiet this long
/// (seconds) after a skip, so one burst is one event…
pub const LOG_QUIET: f64 = 2.0;
/// … or this long after the first skip it holds, whichever comes first.
pub const LOG_EVERY: f64 = 30.0;

impl Diagnostics {
    fn add(&mut self, t: Tally) {
        self.last_counted = t.at;
        self.entered += t.entered;
        self.pending.0 += t.entered;
        for (why, n) in &t.skipped {
            self.skip(*why, *n);
        }
        if self.recent.len() == RECENT {
            self.recent.pop_front();
        }
        self.recent.push_back(t);
    }

    fn skip(&mut self, why: Skip, n: usize) {
        self.pending_since.get_or_insert(self.last_counted);
        *self.skipped.entry(why).or_default() += n;
        *self.pending.1.entry(why).or_default() += n;
    }

    /// What the next `reveal` event says, when one is due: something was skipped since the last,
    /// and the edits have been quiet [`LOG_QUIET`], or the first skip was [`LOG_EVERY`] ago.
    fn due(&mut self, now: f64) -> Option<(usize, Vec<(Skip, usize)>)> {
        let since = self.pending_since?;
        if now - self.last_counted < LOG_QUIET && now - since < LOG_EVERY {
            return None;
        }
        self.pending_since = None;
        let (entered, skipped) = std::mem::take(&mut self.pending);
        Some((entered, skipped.into_iter().collect()))
    }
}

impl State {
    /// Something is being revealed this frame.
    pub fn active(&self) -> bool {
        !self.items.is_empty() || self.fade.is_some()
    }

    /// How many entrances wait for following's flight now (Fix T3).
    pub fn held(&self) -> usize {
        self.items.iter().filter(|i| i.held).count()
    }

    /// The layers queued or entering now, in order (for tests and `ui.inspect`).
    pub fn entering(&self) -> impl Iterator<Item = (NodeId, f64)> + '_ {
        self.items.iter().filter(|i| i.kind == Kind::Enter).map(|i| (i.id, i.start))
    }

    /// The counters and the recent edits' tallies (Fix L2).
    pub fn tallies(&self) -> &Diagnostics {
        &self.diag
    }

    /// [`Self::tallies`] as JSON, for `ui.inspect`.
    pub fn diagnostics(&self) -> serde_json::Value {
        let d = &self.diag;
        let recent: Vec<serde_json::Value> = d
            .recent
            .iter()
            .map(|t| {
                serde_json::json!({"at": t.at, "created": t.created, "entered": t.entered, "withGroup": t.with_group, "skipped": texts(&t.skipped), "follow": t.follow})
            })
            .collect();
        serde_json::json!({"entered": d.entered, "skipped": texts(&d.skipped), "recent": recent})
    }

    /// Stop every reveal: the canvas draws the final state.
    pub fn finish(&mut self) {
        let cut = self.items.iter().filter(|i| i.kind == Kind::Enter).count();
        if cut > 0 {
            self.diag.skip(Skip::CutShort, cut);
        }
        self.items.clear();
        self.fade = None;
        self.frame_doc = None;
        self.wait = None;
    }
}

/// Reasons by their words, for JSON.
fn texts(m: &BTreeMap<Skip, usize>) -> BTreeMap<&'static str, usize> {
    m.iter().map(|(k, n)| (k.text(), *n)).collect()
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// When each of `ids` (in creation order) starts: the speed's stagger apart from `first` (now, or
/// after the last entrance queued), the stagger shrunk so the last ends within the cap of `now`.
pub fn schedule(ids: &[NodeId], now: f64, first: f64, speed: Speed) -> Vec<(NodeId, f64)> {
    let latest_start = now + (speed.cap - speed.enter).max(0.0);
    let first = first.max(now).min(latest_start);
    let step = match ids.len() {
        0 | 1 => 0.0,
        n => speed.stagger.min((latest_start - first) / (n - 1) as f64),
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

/// How far (screen pixels) the camera still has to go on the flight under way, if one is.
fn flight_left(app: &FigApp) -> Option<f64> {
    let to = app.ui.camera.target()?;
    let v = &app.session.ds().ok()?.view;
    Some(((to[0] - v.cx).powi(2) + (to[1] - v.cy).powi(2)).sqrt() * v.zoom)
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
/// Every frame, before the canvas (`gap`: seconds since the last frame): queue what landed, drop
/// what's done, and make this frame's render-only document.
pub fn tick(app: &mut FigApp, ctx: &Context, now: f64, gap: f64, landed: &[Landed]) {
    let (switched_on, motion) = (app.session.prefs.settings.reveal_agent_work, !app.reduce_motion());
    let animate = switched_on && motion;
    let speed = Speed::of(&app.session.prefs.settings.reveal_speed);
    let speed = if app.ui.agent_view.budget.level >= 3 { speed.degraded() } else { speed };
    let follow = figcraft_engine::cmd::camera::follow_state(&app.session);
    let following = follow == "following";
    let view = view_rect(app);
    let Ok(doc) = app.session.ds().map(|d| d.doc.clone()) else {
        app.ui.agent_view.reveal.finish();
        return;
    };
    let before = app.ui.agent_view.before.clone();
    let landed_hidden = app.ui.agent_view.landed_hidden;
    // Following's own flight still on its way (Fix T3): new entrances wait for it.
    let far = following && app.ui.agent_view.follow.flying && flight_left(app).is_some_and(|px| px > HOLD_PX);
    let st = &mut app.ui.agent_view.reveal;
    let fresh = std::mem::take(&mut st.fresh);
    if gap > GAP && st.active() {
        let landing = fresh && gap <= LANDING;
        if landing || (gap <= HIDDEN && !st.stalled) {
            // The landing's own long frame, or one stall: the reveal waits for it.
            st.stalled |= !landing;
            st.last_start += gap;
            st.items.iter_mut().for_each(|i| i.start += gap);
            if let Some(f) = st.fade.as_mut() {
                f.start += gap;
            }
        } else {
            st.finish();
        }
    }
    if !animate {
        st.finish();
    }
    // In view now, or on its way into view (the camera follows the agent there).
    let seen =
        |id: NodeId| following || view.is_some_and(|(page, r)| doc.page_of(id) == Some(page) && doc.world_bounds(id).is_some_and(|b| meets(b, r)));
    // Why nothing of this frame's edits enters, if nothing does.
    let none = if !switched_on {
        Some(Skip::RevealOff)
    } else if !motion {
        Some(Skip::NoMotion)
    } else if landed_hidden {
        // Edits that landed while the window was hidden are drawn final.
        Some(Skip::Hidden)
    } else {
        None
    };
    for l in landed {
        // Any edit ends a delete's fade: the document before it isn't the one before any more.
        st.fade = None;
        if !l.agent {
            continue;
        }
        let candidates = entering(&doc, &l.created);
        let created = l.created.iter().filter(|id| doc.node(**id).is_some_and(|n| !n.is_page())).count();
        let mut tally = Tally { at: now, created, with_group: created.saturating_sub(candidates.len()), follow, ..Tally::default() };
        if let Some(why) = none.or(l.undo.then_some(Skip::Undo)) {
            if !candidates.is_empty() {
                tally.skipped.insert(why, candidates.len());
                st.diag.add(tally);
            }
            continue;
        }
        let new: Vec<NodeId> = candidates.iter().copied().filter(|id| seen(*id)).collect();
        if !candidates.is_empty() {
            tally.entered = new.len();
            if new.len() < candidates.len() {
                let why = if follow == "off" { Skip::OffScreenOff } else { Skip::OffScreenPaused };
                tally.skipped.insert(why, candidates.len() - new.len());
            }
            st.diag.add(tally);
        }
        // Behind the running stagger: a step landing mid-reveal never restarts or cuts it.
        let first = if st.items.iter().any(|i| i.kind == Kind::Enter) { st.last_start + speed.stagger } else { now };
        for (id, start) in schedule(&new, now, first, speed) {
            st.items.retain(|i| i.id != id);
            st.items.push(Item { id, start, dur: speed.enter, kind: Kind::Enter, held: far });
            st.last_start = start;
            st.fresh = true;
            if far && st.wait.is_none() {
                st.wait = Some(now);
            }
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
                    st.items.push(Item { id, start: now, dur: TWEEN, kind: Kind::Move(a.transform.translation()), held: false });
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
    // The wait ends once the flight is within HOLD_PX (or over), or at the cap: the held entrances
    // start now, their stagger kept.
    let mut waited = None;
    if let Some(since) = st.wait
        && (!far || now - since >= HOLD_CAP)
    {
        let delta = now - since;
        let mut n = 0;
        for i in st.items.iter_mut().filter(|i| i.held) {
            i.start += delta;
            i.held = false;
            n += 1;
        }
        st.last_start += delta;
        st.wait = None;
        if far && n > 0 {
            waited = Some((n, (delta * 1000.0).round() as u64));
        }
    }
    // The diagnostics event (Fix L2): at most one per LOG_EVERY, only when something was skipped.
    if let Some((entered, skipped)) = st.diag.due(now) {
        crate::settings_privacy::record(app, Entry::Reveal { entered, skipped: &skipped });
    }
    if let Some((layers, ms)) = waited {
        crate::settings_privacy::record(app, Entry::RevealWaited { layers, ms });
    }
    let st = &mut app.ui.agent_view.reveal;
    st.items.retain(|i| (i.held || now < i.start + i.dur) && doc.contains(i.id));
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
                let e = if it.held { 0.0 } else { ease_out_cubic((now - it.start) / it.dur) };
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

#[cfg(test)]
#[path = "reveal_skip_tests.rs"]
mod skip_tests;

#[cfg(test)]
#[path = "reveal_wait_tests.rs"]
mod wait_tests;
