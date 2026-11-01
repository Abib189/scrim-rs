//! What a layer paints, as page-coordinate bounds: effects, strokes and children included.

use figcraft_geom::Rect;

use crate::{Document, Node, NodeId, NodeKind};

/// Deepest descendant [`Document::world_render_bounds`] follows: as deep as the renderer draws
/// (`figcraft_render`'s `MAX_EXPORT_DEPTH`), so a crafted chain can't overflow the stack (upstream
/// FigCraft 4da6a11b, audit #1062).
pub const MAX_RENDER_DEPTH: usize = 256;

impl Document {
    /// Bounds of everything painted by a node, effects and strokes included (page coordinates):
    /// a clipping frame's own area, else that and what its visible children show (masked
    /// children only within their mask). A group is what its children show, grown by its effects.
    pub fn world_render_bounds(&self, id: NodeId) -> Option<Rect> {
        self.render_bounds_at(id, 0)
    }

    /// [`world_render_bounds`](Self::world_render_bounds) of a layer `depth` levels down: past
    /// [`MAX_RENDER_DEPTH`] only its own area, as nothing deeper draws.
    fn render_bounds_at(&self, id: NodeId, depth: usize) -> Option<Rect> {
        let n = self.node(id)?;
        let own = n.own_paint_bounds(self.world_transform(id));
        if (n.clip && n.kind.is_frame_like() && !n.kind.bounds_follow_children()) || depth >= MAX_RENDER_DEPTH {
            return Some(own);
        }
        Some(match (self.children_render_bounds(n, depth + 1), &n.kind) {
            (Some(kids), NodeKind::Group) => n.grow_by_effects(kids),
            (Some(kids), _) => own.union(kids),
            (None, _) => own,
        })
    }

    /// What `n`'s visible children show, together: each mask's run within the mask's bounds.
    fn children_render_bounds(&self, n: &Node, depth: usize) -> Option<Rect> {
        let shown = |i: &NodeId| self.node(*i).is_some_and(|c| c.visible && !c.is_mask);
        let mut out: Option<Rect> = None;
        for run in self.paint_runs(n) {
            let ids = n.children.get(run).unwrap_or(&[]);
            let mask = ids.first().copied().filter(|m| self.node(*m).is_some_and(|c| c.is_mask && c.visible));
            let r = ids.iter().filter(|i| shown(i)).filter_map(|i| self.render_bounds_at(*i, depth)).reduce(|a, b| a.union(b));
            let r = match mask {
                Some(m) => r.zip(self.render_bounds_at(m, depth)).map(|(a, b)| a.intersect(b)).filter(|x| x.width() > 0.0 && x.height() > 0.0),
                None => r,
            };
            out = match (out, r) {
                (Some(a), Some(b)) => Some(a.union(b)),
                (a, b) => a.or(b),
            };
        }
        out
    }
}
