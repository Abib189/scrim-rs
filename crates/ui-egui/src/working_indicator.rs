//! The working indicator (Stage I2's amendment, `docs/design/follow-agent.md`): while an agent works
//! on an artboard, a row of dots above its top-right corner and a handle-free outline around it.
//! Both are painted only, in screen space: never in the document, undo, the file, MCP results,
//! `screenshot` (which renders the document) or exports.
//! - **Whose:** the top-level artboard of the agent's latest work in a burst (`follow_agent`'s), never
//!   a leaf layer, and one at a time.
//! - **The row:** [`ROWS`] × [`COLUMNS`] dots in groups of [`GROUP`] (a fixed pattern), sized in
//!   screen points, [`ABOVE`] above the artboard, right-aligned to its edge. It is placed in E2's
//!   label layout step after the labels (`canvas_labels::draw`): where a title, icon or other label
//!   is in the way it drops columns from the left; when not even one group fits there, it goes one
//!   row higher, above the labels (still right-aligned to the artboard); failing that, it hides. It
//!   is never wider than its artboard, so never detached from it.
//! - **The wave:** each dot's opacity and radius follow [`wave`], a cosine seeded per column (a pure
//!   function of seed, row, column and time; a [`LOOP`] loop). It shows for as long as the working
//!   state (`working_state`, Stage I.1) says agents work, continuous across the gaps between their
//!   steps: in over 160 ms when a run of work starts, out over 400 ms only when it ends. Reduced
//!   motion, Animations Off or a degraded frame budget: the static row ([`still`]), for the whole run.
//! - **The outline:** 2 px, the canvas overlay colour (G1), no handles and no hit area. When the
//!   person has the artboard selected, their selection's outline and handles are drawn instead.
//! - **Accessibility:** one polite status node, "{agent} is working on {artboard}" for the whole run,
//!   then "{agent} finished" for 2 s; not one per dot.

use std::cell::Cell;

use egui::{Context, Id, Painter, Pos2, Rect, Sense, Stroke, Ui, pos2};
use figcraft_doc::{Document, NodeId};

use super::Landed;
use crate::FigApp;
use crate::theme::ink;

pub const ROWS: usize = 2;
pub const COLUMNS: usize = 20;
/// Columns per group (the gaps between groups are fixed).
pub const GROUP: usize = 5;
/// A dot's diameter at its largest, in screen points.
pub const DOT: f32 = 4.0;
/// Between two dots, and the extra between two groups.
pub const SPACE: f32 = 3.0;
pub const GROUP_SPACE: f32 = 3.0;
/// From the row's bottom to the artboard's top edge …
pub const ABOVE: f32 = 12.0;
/// … or, when labels take that place, above the label row, E2's gap clear of it.
pub const ABOVE_LABELS: f32 = crate::canvas_labels::ROW_TOP + crate::canvas_labels::GAP + 1.0;
/// Fewer columns than this fit: the row hides.
pub const MIN_COLUMNS: usize = GROUP;
/// The wave's loop.
pub const LOOP: f64 = 1.6;
/// The fades (the working state's).
pub use super::working_state::{FADE_IN, FADE_OUT};

/// The artboard being worked on.
#[derive(Default)]
pub struct State {
    /// The document (`uid`) and artboard.
    on: Option<(u64, NodeId)>,
    /// Where the row was drawn this frame and with how many columns (its AccessKit node); the
    /// outline's rectangle. Set while the canvas paints.
    pub(crate) drawn: Cell<Option<(Rect, usize)>>,
    pub(crate) outlined: Cell<Option<Rect>>,
    /// Where the row was last drawn: "{agent} finished" is said there once it has faded.
    last_drawn: Cell<Option<Rect>>,
    /// The agent's newest artboard while it is still out of view (the camera on its way): the row
    /// stays on the one shown until this one is in view, so it never blinks out between steps.
    next: Option<(u64, NodeId)>,
}

impl State {
    /// The artboard shown as being worked on.
    pub fn artboard(&self) -> Option<NodeId> {
        self.on.map(|(_, id)| id)
    }
}

/// The row's width with `cols` columns.
pub fn width(cols: usize) -> f32 {
    if cols == 0 {
        return 0.0;
    }
    cols as f32 * DOT + (cols - 1) as f32 * SPACE + ((cols - 1) / GROUP) as f32 * GROUP_SPACE
}

/// The row's height.
pub fn height() -> f32 {
    ROWS as f32 * DOT + (ROWS - 1) as f32 * SPACE
}

/// Column `c`'s centre (0: the rightmost), left of the row's right edge. Groups count from the
/// right, so dropping columns from the left keeps the rest where they were.
pub fn column_x(c: usize) -> f32 {
    DOT / 2.0 + c as f32 * (DOT + SPACE) + (c / GROUP) as f32 * GROUP_SPACE
}

/// A number in 0..1 from `seed` and `col` (splitmix64): the column's own phase.
fn hash01(seed: u64, col: usize) -> f64 {
    let mut z = seed ^ (col as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// The wave at dot (`row`, `col`) and time `t`: 0..1, smooth (a cosine), a [`LOOP`] loop, its phase
/// seeded per column so neighbours differ but drift together.
pub fn wave(seed: u64, row: usize, col: usize, t: f64) -> f64 {
    let phase = hash01(seed, col) * 0.35 + col as f64 * 0.045 + row as f64 * 0.18;
    0.5 - 0.5 * (std::f64::consts::TAU * (t / LOOP + phase)).cos()
}

/// The static row (reduced motion, Animations Off, a degraded budget): the wave at a fixed moment.
pub fn still(seed: u64, row: usize, col: usize) -> f64 {
    wave(seed, row, col, 0.37 * LOOP)
}

/// Where the row goes: right-aligned at the artboard's top-right corner `corner` (screen), within
/// its on-screen width `room`, a gap clear of every rectangle in `taken` (E2's placed labels):
/// [`ABOVE`] the artboard, else [`ABOVE_LABELS`]; the most columns that fit, [`MIN_COLUMNS`] at
/// least. `None`: hidden.
pub fn place(corner: Pos2, room: f32, taken: &[Rect]) -> Option<(usize, Rect)> {
    if !(corner.x.is_finite() && corner.y.is_finite() && room.is_finite()) {
        return None;
    }
    // Its own place first, dropping columns there; then the row above the labels.
    [ABOVE, ABOVE_LABELS].into_iter().find_map(|above| {
        (MIN_COLUMNS..=COLUMNS).rev().find_map(|cols| {
            let w = width(cols);
            let r = Rect::from_min_max(pos2(corner.x - w, corner.y - above - height()), pos2(corner.x, corner.y - above));
            (w <= room && taken.iter().all(|t| !t.expand(crate::canvas_labels::GAP).intersects(r))).then_some((cols, r))
        })
    })
}

/// Every frame, before the canvas (after the working state's tick): the agent's latest artboard is
/// the one shown; it is let go once the run has ended and faded.
pub fn tick(app: &mut FigApp, ctx: &Context, now: f64, landed: &[Landed]) {
    let uid = app.session.ds().map(|d| d.uid).ok();
    let artboard = super::working_on(app).1;
    let motion = !app.reduce_motion();
    let grace = super::grace(app);
    let w = &app.ui.agent_view.working;
    let (working, alpha, expiry) = (w.working(), w.alpha(now, motion), w.next_change(now, grace));
    let finished = w.finished(now).is_some();
    // Where the row can be seen: its artboard's top-right corner, which it hangs from, inside the
    // canvas (an artboard only partly in view can have that corner off screen).
    let in_view = |id: NodeId| {
        app.session.doc().ok().and_then(|d| d.world_bounds(id)).is_some_and(|b| {
            let corner = app.to_screen(b.x1, b.y0);
            corner.is_finite() && app.canvas.rect.contains(corner)
        })
    };
    let next = match (uid, artboard) {
        (Some(uid), Some(art)) if landed.iter().any(|l| l.agent) => Some((uid, art)),
        _ => app.ui.agent_view.indicator.next,
    };
    let on = app.ui.agent_view.indicator.on;
    let (next_in, on_in) = (next.is_some_and(|(_, a)| in_view(a)), on.is_some_and(|(_, a)| in_view(a)));
    let st = &mut app.ui.agent_view.indicator;
    st.next = next;
    // Move to the newest artboard once it is in view (or when the one shown isn't either).
    if let Some((uid, art)) = st.next
        && (st.on.is_none_or(|(u, a)| u != uid || a == art) || next_in || !on_in)
    {
        st.on = Some((uid, art));
        st.next = None;
    }
    if !working && alpha <= 0.0 {
        st.on = None;
        st.next = None;
    }
    if (working || alpha > 0.0) && motion && st.on.is_some() {
        ctx.request_repaint();
    } else if let Some(t) = expiry {
        // The grace period running out ends the state: be there for it.
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(t.max(0.01)));
    } else if alpha > 0.0 || finished {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}

/// The outline and the dot row, during E2's label step (`taken`: the labels placed there).
pub fn paint(app: &FigApp, painter: &Painter, doc: &Document, sel: &[NodeId], taken: &[egui::Rect]) {
    let st = &app.ui.agent_view.indicator;
    st.drawn.set(None);
    st.outlined.set(None);
    let Some((uid, id)) = st.on else { return };
    let Ok(ds) = app.session.ds() else { return };
    if ds.uid != uid || doc.page_of(id) != Some(ds.page) {
        return;
    }
    let now = painter.ctx().input(|i| i.time);
    let motion = !app.reduce_motion();
    let a = app.ui.agent_view.working.alpha(now, motion) as f32;
    let Some(b) = doc.world_bounds(id).filter(|_| a > 0.0) else { return };
    let r = Rect::from_two_pos(app.to_screen(b.x0, b.y0), app.to_screen(b.x1, b.y1));
    if !r.is_finite() || !r.intersects(painter.clip_rect().expand(height() + ABOVE)) {
        return;
    }
    let level = app.ui.agent_view.budget.level;
    let seed = id.0;
    // The wave runs on the clock, so it carries on unbroken across the gaps between steps.
    let t = now;
    // The outline, unless the person's selection outline and handles are there.
    if !sel.contains(&id) {
        let pulse = if motion && level < 2 { 0.85 + 0.15 * wave(seed, 0, 0, t) as f32 } else { 1.0 };
        painter.rect_stroke(r, 0.0, Stroke::new(2.0, ink::overlay().gamma_multiply(a * pulse)), egui::StrokeKind::Outside);
        st.outlined.set(Some(r));
    }
    if !app.session.prefs.settings.working_indicator {
        return;
    }
    let Some((cols, row)) = place(pos2(r.max.x, r.min.y), r.width(), taken) else { return };
    let animated = motion && level < 1;
    let color = ink::overlay();
    for rw in 0..ROWS {
        for c in 0..cols {
            let v = if animated { wave(seed, rw, c, t) } else { still(seed, rw, c) } as f32;
            let centre = pos2(row.max.x - column_x(c), row.min.y + DOT / 2.0 + rw as f32 * (DOT + SPACE));
            painter.circle_filled(centre, DOT / 2.0 * (0.55 + 0.45 * v), color.gamma_multiply(a * (0.3 + 0.7 * v)));
        }
    }
    st.drawn.set(Some((row, cols)));
}

/// After the canvas: the row's one AccessKit node, a polite status naming the agent and artboard
/// while it works, then "{agent} finished" where the row was.
pub fn accessibility(app: &FigApp, ui: &mut Ui) {
    let st = &app.ui.agent_view.indicator;
    if let Some((r, _)) = st.drawn.get() {
        st.last_drawn.set(Some(r));
    }
    let (r, text) = match (super::working_text(app), super::finished_text(app)) {
        (Some(text), _) => match st.drawn.get() {
            Some((r, _)) => (r, text),
            None => return,
        },
        (None, Some(text)) => match st.last_drawn.get() {
            Some(r) => (r, text),
            None => return,
        },
        (None, None) => {
            st.last_drawn.set(None);
            return;
        }
    };
    let resp = ui.interact(r, Id::new("agent_working_indicator"), Sense::hover());
    let ctx = ui.ctx().clone();
    ctx.accesskit_node_builder(resp.id, |n| {
        n.set_role(egui::accesskit::Role::Status);
        n.set_live(egui::accesskit::Live::Polite);
        n.set_label(text);
        n.set_bounds(egui::accesskit::Rect { x0: r.min.x.into(), y0: r.min.y.into(), x1: r.max.x.into(), y1: r.max.y.into() });
    });
}

#[cfg(test)]
#[path = "working_indicator_tests.rs"]
mod tests;
