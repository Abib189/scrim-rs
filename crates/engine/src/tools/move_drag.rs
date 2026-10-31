//! Moving layers with the Move tool, and Option-drag to duplicate (Fix AE). Option decides
//! throughout the drag, not only at the press: while it is held the originals stay and copies
//! follow the pointer; released, the originals move again. The copies are a preview until the
//! release, which commits them through `edit.duplicate` as one undo step.

use super::*;

impl Session {
    /// A move gesture's pointer move: the layers (Option: copies of them) previewed at the pointer.
    pub(super) fn move_drag(&mut self, ev: &PointerEvent, g: &Gesture) -> Result<()> {
        let Gesture::Move { start, ids, targets, copy_targets, moved, .. } = g else { return Ok(()) };
        let copy = ev.alt;
        if !*moved && within_slop(ev, *start) {
            self.tool.overlay.cursor = if copy { Cursor::Copy } else { Cursor::Default };
            self.tool.gesture = g.clone();
            return Ok(());
        }
        // A copy snaps to the layers it came from too: they stay put.
        let targets: &Targets = if copy { copy_targets } else { targets };
        let mut d = ev.p() - *start;
        if ev.shift {
            if d.x.abs() > d.y.abs() {
                d.y = 0.0;
            } else {
                d.x = 0.0;
            }
        }
        // Cmd/Ctrl (and Control on macOS) held while dragging suspends snapping.
        let snap_on = self.ds().map(|ds| ds.view.snap_objects).unwrap_or(true) && !ev.cmd && !ev.ctrl;
        let mut guides = Vec::new();
        if snap_on {
            let (c, g2) = targets.snap_move(d, SNAP_PX / ev.zoom.max(1e-6));
            d += c;
            guides = g2;
        }
        if self.ds().map(|ds| ds.view.snap_pixel).unwrap_or(true) {
            // Keep the box's origin on whole pixels (a rotated parent frame's, inside one).
            d = targets.round_move(d);
        }
        let ids = ids.clone();
        let rename = self.prefs.rename_duplicates;
        let page = self.page()?;
        let mut drop_cell = None;
        let mut drop_target = None;
        let mut in_flow = false;
        // Placed in a row or column: (is it a column, its gap) for the insertion line.
        let mut slot_axis: Option<(bool, f64)> = None;
        self.preview(|doc, sel| {
            let mut moving = ids.clone();
            if copy {
                // The copies Duplicate makes, in place: from there they move as the layers would.
                let sources = crate::cmd::edit::z_sorted(doc, &ids);
                moving = crate::cmd::duplicate::copies(doc, &sources, (Some(0.0), Some(0.0)), None, rename)?;
                *sel = moving.clone();
            }
            let p = ev.p();
            let home = moving.first().and_then(|i| doc.node(*i)).and_then(|n| n.parent);
            // Absolute children of an auto layout frame stay in it, and so do the layers of
            // groups, boolean groups and instances ([`crate::drop::keeps_layers`]); the
            // rest may change parent.
            let flow = moving.iter().all(|i| doc.node(*i).is_some_and(|n| !n.child_layout.absolute))
                && !home.is_some_and(|h| crate::drop::keeps_layers(doc, h));
            // The originals under copies are no place to drop them either.
            let exclude: Vec<NodeId> = if copy { moving.iter().chain(&ids).copied().collect() } else { moving.clone() };
            let target = if flow { crate::drop::drag_parent(doc, page, p, &exclude) } else { home.unwrap_or(page) };
            let mode = doc.node(target).and_then(|n| n.layout.as_ref()).map(|l| l.mode);
            if let (true, Some(mode)) = (flow, mode) {
                // Into (or within) an auto layout frame: the layers take the slot (a grid: the
                // cell) under the pointer, as the frame shows it now. A frame other than the
                // one they come from is outlined too.
                let cell = if mode == LayoutMode::Grid { crate::drop::grid_cell_at(doc, target, p) } else { None };
                let idx = if mode == LayoutMode::Grid { 0 } else { crate::drop::auto_layout_slot(doc, target, p, &moving).map_or(0, |(i, _)| i) };
                if let (Some(first), Some((col, row, rect))) = (moving.first().copied(), cell) {
                    crate::cmd::duplicate::place(doc, &moving, target, None)?;
                    let cl = &mut doc.get_mut(first)?.child_layout;
                    cl.grid_column = Some(col);
                    cl.grid_row = Some(row);
                    drop_cell = Some(rect);
                } else if mode != LayoutMode::Grid {
                    crate::cmd::duplicate::place(doc, &moving, target, Some(idx))?;
                    let gap = doc.node(target).and_then(|n| n.layout.as_ref()).map_or(0.0, |l| l.gap);
                    slot_axis = Some((mode == LayoutMode::Vertical, gap));
                } else {
                    crate::cmd::duplicate::place(doc, &moving, target, None)?;
                }
                drop_target = (Some(target) != home).then_some(target);
                in_flow = true;
                return Ok(());
            }
            // A free move. Layers dragged out of an auto layout flow leave it (onto the page
            // while dragging; the drop puts them in the frame under the pointer).
            for m in &moving {
                let in_al = doc.node(*m).and_then(|n| n.parent).and_then(|p| doc.node(p)).is_some_and(|p| p.layout.is_some());
                if flow && in_al {
                    doc.reparent(*m, page, None)?;
                }
                move_world(doc, *m, d.x, d.y)?;
            }
            drop_target = (flow && target != page).then_some(target);
            Ok(())
        })?;
        let placed = if copy { self.selection() } else { ids };
        // Snapping guides mean nothing while the layout places the layers; with Alt held,
        // live distances to the neighbours replace them.
        self.tool.overlay.guides = if in_flow {
            Vec::new()
        } else if ev.alt {
            let doc = self.doc()?;
            doc.bounds_of(&placed).map(|b| crate::distance::neighbours(doc, page, &placed, b)).unwrap_or_default()
        } else {
            guides
        };
        // The preview already shows the layers in their slot: the line marks it there, in
        // the middle of the gap before them (clear of the selection box).
        self.tool.overlay.insert_line = slot_axis.and_then(|(vertical, gap)| {
            let off = if gap.is_finite() { (gap / 2.0).clamp(3.0, 24.0) } else { 3.0 };
            self.doc().ok()?.bounds_of(&placed).map(|b| crate::drop::line_before(b, vertical, off))
        });
        self.tool.overlay.drop_cell = drop_cell;
        self.tool.overlay.drop_target = drop_target;
        self.tool.overlay.cursor = if self.refused(&placed, true) {
            Cursor::NotAllowed
        } else if copy {
            Cursor::Copy
        } else {
            Cursor::Default
        };
        self.tool.gesture = g.clone();
        if let Gesture::Move { moved, copy: c, .. } = &mut self.tool.gesture {
            *moved = true;
            *c = copy;
        }
        Ok(())
    }

    /// A move gesture's release. `drop_target` is the frame the layers were last shown going into.
    pub(super) fn move_up(&mut self, ev: &PointerEvent, g: Gesture, drop_target: Option<NodeId>) -> Result<()> {
        let Gesture::Move { moved, copy, start, deselect, .. } = &g else { return Ok(()) };
        if !moved {
            self.cancel_interaction()?;
            if let Some(d) = deselect {
                let sel = self.selection().into_iter().filter(|x| x != d).collect();
                self.set_selection(sel);
            }
            if deselect.is_none() && !(ev.shift || ev.cmd || ev.alt || *copy) {
                self.widget_click(*start, ev.tol() * 2.0)?;
            }
            return Ok(());
        }
        // Option pressed or let go since the last move: the release shows that first.
        let (g, drop_target) = if *copy != ev.alt {
            self.move_drag(ev, &g)?;
            self.tool.overlay.guides.clear();
            self.tool.overlay.insert_line = None;
            self.tool.overlay.drop_cell = None;
            (std::mem::take(&mut self.tool.gesture), self.tool.overlay.drop_target.take())
        } else {
            (g, drop_target)
        };
        match &g {
            Gesture::Move { copy: true, .. } => self.drop_copies(&g, drop_target),
            _ => self.drop_moved(&g, drop_target),
        }
    }

    /// Moved layers land in the frame under the pointer (or the page) where they were dropped;
    /// layers in an auto layout frame already sit in their slot.
    fn drop_moved(&mut self, g: &Gesture, drop_target: Option<NodeId>) -> Result<()> {
        let Gesture::Move { ids: moving, .. } = g else { return Ok(()) };
        let page = self.page()?;
        let doc = self.doc()?.clone();
        let in_al = moving.iter().any(|m| doc.node(*m).and_then(|n| n.parent).and_then(|p| doc.node(p)).is_some_and(|p| p.layout.is_some()));
        let target = drop_target.unwrap_or(page);
        let current = moving.first().and_then(|m| doc.node(*m)).and_then(|n| n.parent);
        if !in_al && current != Some(target) {
            let ds = self.ds_mut()?;
            let before = ds.doc.clone();
            let d = ds.doc_mut();
            for m in moving {
                if !stays_out(d, *m, target) {
                    // A layer can't go into itself; it stays where it is.
                    let _ = d.reparent(*m, target, None);
                }
            }
            crate::normalize(ds.doc_mut(), &before);
        }
        // Reordering or dragging layers out of an instance (outside its slots), or moving layers
        // inside one, are edits only its main can make: the gesture is undone.
        if self.refused(moving, true) { self.cancel_interaction() } else { self.commit_interaction() }
    }

    /// Copies dropped: the preview goes, and Duplicate makes them where they were shown, as one
    /// undo step that selects them (undone: the originals are selected again).
    fn drop_copies(&mut self, g: &Gesture, drop_target: Option<NodeId>) -> Result<()> {
        let Gesture::Move { ids, .. } = g else { return Ok(()) };
        let copies = self.selection();
        if self.refused(&copies, true) {
            return self.cancel_interaction();
        }
        let page = self.page()?;
        let doc = self.doc()?.clone();
        let sources = crate::cmd::edit::z_sorted(&doc, ids);
        let (Some(&first), Some(&source)) = (copies.first(), sources.first()) else { return self.cancel_interaction() };
        let parent_of = |i: NodeId| doc.node(i).and_then(|n| n.parent);
        let in_al = copies.iter().any(|c| parent_of(*c).and_then(|p| doc.node(p)).is_some_and(|p| p.layout.is_some()));
        // Where the copies go: their slot in an auto layout frame, else the frame they were
        // dropped over (as a move would), else the parent they were shown in.
        let target = drop_target.unwrap_or(page);
        let shown = parent_of(first).unwrap_or(page);
        let parent = if !in_al && shown != target && !copies.iter().any(|c| stays_out(&doc, *c, target)) { target } else { shown };
        let index = in_al.then(|| doc.node(parent).map(|p| p.children.iter().take_while(|c| !copies.contains(c)).count())).flatten();
        let grid = doc
            .node(parent)
            .and_then(|p| p.layout.as_ref())
            .filter(|l| l.mode == LayoutMode::Grid)
            .and_then(|_| doc.node(first))
            .and_then(|n| n.child_layout.grid_column.zip(n.child_layout.grid_row));
        // The pointer's offset, in the original's parent's space as Duplicate takes it.
        let d = doc.world_transform(first).translation() - doc.world_transform(source).translation();
        let pw = parent_of(source).map(|p| doc.world_transform(p)).unwrap_or(Affine::IDENTITY);
        let local = (pw.inverse() * Affine::translate(d) * pw).translation();
        let moved_parent = copies.iter().zip(&sources).any(|(c, s)| parent_of(*c) != parent_of(*s)) || parent != shown;
        let mut params = json!({"ids": sources, "dx": local.x, "dy": local.y});
        if moved_parent || in_al {
            params["parent"] = json!(parent);
            if let Some(i) = index {
                params["index"] = json!(i);
            }
        }
        self.cancel_interaction()?;
        self.grouped("Duplicate", |s| {
            let made = s.execute("edit.duplicate", &params)?;
            if let (Some((col, row)), Some(c)) = (grid, made["ids"].get(0)) {
                s.execute("node.set", &json!({"id": c, "props": {"gridColumn": col, "gridRow": row}}))?;
            }
            Ok(())
        })?;
        // The layout placed the copies: Duplicate has no step to repeat.
        if in_al {
            self.ds_mut()?.last_duplicate = None;
        }
        Ok(())
    }
}

/// Group, boolean group and instance layers stay in them (they only move), and nothing goes into
/// a layer whose structure only its main can change.
fn stays_out(doc: &Document, id: NodeId, target: NodeId) -> bool {
    doc.node(id).and_then(|n| n.parent).is_some_and(|f| crate::drop::keeps_layers(doc, f)) || crate::components::structure_locked(doc, target)
}

#[cfg(test)]
#[path = "../option_drag_tests.rs"]
mod tests;
