//! Frame statistics for spotting wasted redraws: how many frames the UI drew and what asked for
//! them (the `file:line` of each repaint request), read through `ui.inspect` (`framesDrawn`,
//! `repaintCauses`) and reset with `ui.set {resetFrameStats: true}`. An idle window should draw
//! no frames at all.
//!
//! **The last 5 s** (Fix AA5): `repaintsPerSecond` and `repaintCausesRecent`, the frames drawn in
//! the 5 s before the latest frame (the one serving the request, left out) and, for each call site,
//! in how many of them it asked for a repaint. egui keeps every request of a frame as a cause, a
//! timer due in half an hour included, so a site in every frame is either what draws them or one
//! that re-asks each time; the rate is the number to trust.

use std::collections::{BTreeMap, VecDeque};

use serde_json::{Value, json};

/// How many of the most frequent repaint causes `ui.inspect` lists.
const TOP_CAUSES: usize = 12;
/// The rolling window, in seconds.
pub const WINDOW: f64 = 5.0;

#[derive(Default)]
pub struct FrameStats {
    /// Frames drawn since the app started (or the last reset).
    pub frames: u64,
    /// Repaint requests by call site, counted once per frame they asked for.
    causes: BTreeMap<(&'static str, u32), u64>,
    /// The frames of about the last [`WINDOW`] seconds: when each was drawn and its call sites.
    recent: VecDeque<(f64, Vec<(&'static str, u32)>)>,
}

impl FrameStats {
    /// Count this frame and what asked for it (call once a frame).
    pub fn record(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        let mut sites = Vec::new();
        for c in ctx.repaint_causes() {
            *self.causes.entry((c.file, c.line)).or_default() += 1;
            if !sites.contains(&(c.file, c.line)) {
                sites.push((c.file, c.line));
            }
        }
        let now = ctx.input(|i| i.time);
        self.recent.push_back((now, sites));
        while self.recent.front().is_some_and(|(t, _)| *t < now - WINDOW - 1.0) {
            self.recent.pop_front();
        }
    }

    /// The frames of the [`WINDOW`] seconds before the latest frame, which is left out (it is the
    /// one reading this, woken by the request).
    fn window(&self) -> impl Iterator<Item = &(f64, Vec<(&'static str, u32)>)> {
        let now = self.recent.back().map_or(0.0, |(t, _)| *t);
        let n = self.recent.len().saturating_sub(1);
        self.recent.iter().take(n).filter(move |(t, _)| *t >= now - WINDOW)
    }

    /// `framesDrawn`, `repaintCauses`, `repaintsPerSecond` and `repaintCausesRecent`, into
    /// `ui.inspect`'s object.
    pub fn inspect_into(&self, state: &mut Value) {
        state["framesDrawn"] = json!(self.frames);
        state["repaintCauses"] = self.causes();
        state["repaintsPerSecond"] = json!(self.per_second());
        state["repaintCausesRecent"] = self.recent_causes();
    }

    /// Repaints a second over the window.
    pub fn per_second(&self) -> f64 {
        self.window().count() as f64 / WINDOW
    }

    /// The window's call sites, most frames first: `[{at: "file:line", frames}]`.
    pub fn recent_causes(&self) -> Value {
        let mut by: BTreeMap<(&'static str, u32), u64> = BTreeMap::new();
        for (_, sites) in self.window() {
            for s in sites {
                *by.entry(*s).or_default() += 1;
            }
        }
        let mut top: Vec<_> = by.into_iter().collect();
        top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        top.into_iter().take(TOP_CAUSES).map(|((file, line), n)| json!({"at": format!("{file}:{line}"), "frames": n})).collect()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The most frequent repaint causes, most frequent first: `[{at: "file:line", count}]`.
    pub fn causes(&self) -> Value {
        let mut top: Vec<(&(&'static str, u32), &u64)> = self.causes.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1));
        top.into_iter().take(TOP_CAUSES).map(|((file, line), n)| json!({"at": format!("{file}:{line}"), "count": n})).collect()
    }
}
