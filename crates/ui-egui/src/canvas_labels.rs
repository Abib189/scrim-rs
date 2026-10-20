//! One layout step for the labels around frames on the canvas (Stage E2,
//! `docs/design/canvas-badges.md`): frame, component and section titles and their status icons are
//! placed together each frame, before any is drawn, so no label covers another's text at any zoom.
//!
//! - **Room:** a label starts at its frame's top-left and runs at most the frame's on-screen width.
//! - **Priority**, highest first ([`Class`]): labels turned with a rotated frame (kept along their
//!   edge, their box taken), section titles (outer sections first), selected, hovered, then the
//!   rest in layer order (bottom first: the label that was there first keeps its place).
//! - **Placing:** in that order, each label takes the leftmost free stretch of its room. A placed
//!   label over its start pushes it right, past that label and a gap; one further right ends its
//!   room a gap before it. In the stretch it gets, the title is cut first, then the status icon
//!   dropped, then the label hidden (`frame_labels::fit`).
//!
//! The layout is pure (rectangles in, rectangles out); [`draw`] measures, lays out and paints.

use egui::{Painter, Pos2, Rect, pos2};
use figcraft_doc::{Document, NodeId};

use crate::FigApp;
use crate::dev_icon::{ICON, ICON_GAP};
use crate::frame_labels::{self, fit};
use crate::theme::Tokens;

/// A label row runs from this far above its frame's top edge …
pub const ROW_TOP: f32 = 18.0;
/// … to this far above it.
pub const ROW_BOTTOM: f32 = 2.0;
/// The space kept between two labels.
pub const GAP: f32 = 8.0;

/// Which labels are placed first (the first variant first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// Turned with a rotated frame or section: kept along its edge.
    Turned,
    Section,
    Selected,
    Hovered,
    Other,
}

/// A label to place, in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// Its frame's top-left as seen: the row starts here and sits above it.
    pub corner: Pos2,
    /// Its frame's on-screen width.
    pub room: f32,
    /// For a turned label, the box its whole row covers (it stays where it is).
    pub turned: Option<Rect>,
    pub class: Class,
    /// For sections, how many sections it is inside (outer sections first).
    pub depth: u32,
    /// The lead icon's width (the component diamond, a hidden section's eye), 0 without one.
    pub lead: f32,
    /// The title's natural width.
    pub title: f32,
    /// It has a status icon.
    pub icon: bool,
}

/// Where a label goes: `offset` from its corner along the row, the title's width, whether the icon
/// shows, and the box the label takes on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub offset: f32,
    pub title: f32,
    pub icon: bool,
    pub rect: Rect,
}

impl Item {
    fn width(&self, title: f32, icon: bool) -> f32 {
        self.lead + title + if icon { ICON_GAP + ICON } else { 0.0 }
    }

    fn fit(&self, avail: f32) -> Option<(f32, bool)> {
        fit(avail - self.lead, self.title, self.icon)
    }
}

/// Place `items`: each one's placement, `None` for a label with no room.
pub fn layout(items: &[Item]) -> Vec<Option<Placed>> {
    let mut order: Vec<(Class, u32, usize)> = items.iter().enumerate().map(|(i, it)| (it.class, it.depth, i)).collect();
    order.sort_unstable();
    let mut out = vec![None; items.len()];
    let mut taken: Vec<Rect> = Vec::new();
    for (_, _, i) in order {
        let Some(it) = items.get(i) else { continue };
        let placed = match it.turned {
            Some(b) => it.fit(it.room).map(|(title, icon)| Placed { offset: 0.0, title, icon, rect: b }),
            None => place(it, &taken),
        };
        if let Some(p) = placed {
            taken.push(p.rect);
        }
        if let Some(slot) = out.get_mut(i) {
            *slot = placed;
        }
    }
    out
}

/// An upright label in the leftmost free stretch of its room that holds it.
fn place(it: &Item, taken: &[Rect]) -> Option<Placed> {
    let (y0, y1) = (it.corner.y - ROW_TOP, it.corner.y - ROW_BOTTOM);
    let (lo, hi) = (it.corner.x, it.corner.x + it.room);
    if !(lo.is_finite() && hi.is_finite() && y0.is_finite()) {
        return None;
    }
    // The stretches of this row other labels take, a gap either side.
    let mut blocks: Vec<(f32, f32)> = taken.iter().filter(|r| r.max.y > y0 && r.min.y < y1).map(|r| (r.min.x - GAP, r.max.x + GAP)).collect();
    blocks.sort_by(|a, b| a.0.total_cmp(&b.0));
    blocks.push((f32::INFINITY, f32::INFINITY));
    let mut s = lo;
    for (b0, b1) in blocks {
        if b1 <= s {
            continue;
        }
        let e = b0.min(hi);
        if e > s
            && let Some((title, icon)) = it.fit(e - s)
        {
            let rect = Rect::from_min_max(pos2(s, y0), pos2(s + it.width(title, icon), y1));
            return Some(Placed { offset: s - lo, title, icon, rect });
        }
        s = s.max(b1);
        if s >= hi {
            break;
        }
    }
    None
}

/// Measure, lay out and draw the labels of `ids` (top-level frames and sections, in layer order);
/// each drawn label's area on screen (its corners) for hit testing.
pub fn draw(
    app: &FigApp,
    painter: &Painter,
    t: &Tokens,
    doc: &Document,
    ids: &[NodeId],
    sel: &[NodeId],
    hover: Option<NodeId>,
) -> Vec<(NodeId, [Pos2; 4])> {
    let labels: Vec<frame_labels::Label> =
        ids.iter().filter_map(|id| frame_labels::prepare(app, painter, t, doc, *id, sel.contains(id), hover == Some(*id))).collect();
    let items: Vec<Item> = labels.iter().map(|l| l.item).collect();
    labels.iter().zip(layout(&items)).filter_map(|(l, p)| Some((l.id, frame_labels::paint(painter, l, p?)))).collect()
}

#[cfg(test)]
#[path = "canvas_labels_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "canvas_labels_ui_tests.rs"]
mod ui_tests;
