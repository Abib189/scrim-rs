//! Copying layers within or into a document: duplicates and pastes with fresh ids, and the
//! references inside the copy (interactions, instances, pattern sources) remapped to it.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{DocError, DocResult, Document, Node, NodeId, NodeKind, proto};

impl Document {
    /// Deep-copy `id`'s subtree under `parent` (at `index`), returning the new root id. The copy
    /// keeps the original's local transform; ids are fresh.
    pub fn duplicate_into(&mut self, id: NodeId, parent: NodeId, index: Option<usize>) -> DocResult<NodeId> {
        let mut map = BTreeMap::new();
        let src_nodes: Vec<Node> = self.subtree(id).into_iter().filter_map(|i| self.node(i).cloned()).collect();
        for n in &src_nodes {
            let nid = self.alloc_id();
            map.insert(n.id, nid);
        }
        self.paste_nodes(&src_nodes, &map, id, parent, index)
    }

    /// Deep-copy each of `ids`' subtrees to the end of `parent`, in order, returning the new root
    /// ids. One id map covers every copy, so references between the copied subtrees (a navigation
    /// from one copied frame to another) point at the copies.
    pub fn duplicate_all_into(&mut self, ids: &[NodeId], parent: NodeId) -> DocResult<Vec<NodeId>> {
        let mut map = BTreeMap::new();
        let mut trees = Vec::with_capacity(ids.len());
        for &id in ids {
            let src_nodes: Vec<Node> = self.subtree(id).into_iter().filter_map(|i| self.node(i).cloned()).collect();
            for n in &src_nodes {
                let nid = self.alloc_id();
                map.insert(n.id, nid);
            }
            trees.push((id, src_nodes));
        }
        trees.into_iter().map(|(id, src_nodes)| self.paste_nodes(&src_nodes, &map, id, parent, None)).collect()
    }

    /// Insert copies of `nodes` (a subtree rooted at `root`, from this or another document) with ids
    /// remapped by `map`. Interactions, instance references and pattern sources pointing inside the
    /// subtree are remapped too.
    pub fn paste_nodes(
        &mut self,
        nodes: &[Node],
        map: &BTreeMap<NodeId, NodeId>,
        root: NodeId,
        parent: NodeId,
        index: Option<usize>,
    ) -> DocResult<NodeId> {
        let p = self.get(parent)?;
        if !p.kind.is_container() {
            return Err(DocError::NotContainer(parent));
        }
        let new_root = *map.get(&root).ok_or(DocError::NoNode(root))?;
        for n in nodes {
            let Some(new_id) = map.get(&n.id) else { continue };
            let mut c = n.clone();
            c.id = *new_id;
            c.children = n.children.iter().filter_map(|k| map.get(k).copied()).collect();
            c.parent = if n.id == root { Some(parent) } else { n.parent.and_then(|p| map.get(&p).copied()) };
            for it in &mut c.interactions {
                proto::walk_mut(&mut it.actions, &mut |a| remap_action(a, map));
            }
            if let NodeKind::Instance { component, .. } = &mut c.kind
                && let Some(m) = map.get(component)
            {
                *component = *m;
            }
            // A pattern tiling a layer of the copy tiles the copy's layer.
            for p in c.fills.iter_mut().chain(c.strokes.iter_mut()) {
                if let figcraft_color::PaintKind::Pattern(pt) = &mut p.kind
                    && let Some(m) = map.get(&NodeId(pt.source))
                {
                    pt.source = m.0;
                }
            }
            self.reserve_ids(*new_id);
            self.nodes.insert(*new_id, Arc::new(c));
            self.dirty.touch(*new_id);
        }
        let p = self.get_mut(parent)?;
        let at = index.unwrap_or(p.children.len()).min(p.children.len());
        p.children.insert(at, new_root);
        Ok(new_root)
    }
}

fn remap_action(a: &mut proto::Action, map: &BTreeMap<NodeId, NodeId>) {
    use proto::Action::*;
    match a {
        Navigate { dest, .. } | ScrollTo { dest, .. } | OpenOverlay { dest, .. } | SwapOverlay { dest } | ChangeTo { dest } => {
            if let Some(m) = map.get(dest) {
                *dest = *m;
            }
        }
        _ => {}
    }
}
