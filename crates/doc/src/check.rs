//! Checking a document's structure, and repairing what fails (lenient loading of untrusted files).

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{DevStatus, Document, NodeId};

/// The most issues [`Document::validate`] reports.
const MAX_ISSUES: usize = 100;

impl Document {
    /// Check structural invariants (used by tests and after loading untrusted files).
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        // A hostile file can break an invariant once per child: only the first reports are kept
        // (upstream FigCraft c857f8d0, audit #1058).
        let note = |issues: &mut Vec<String>, msg: &dyn Fn() -> String| {
            if issues.len() < MAX_ISSUES {
                issues.push(msg());
            }
        };
        for (id, n) in self.nodes.iter() {
            if n.id != *id {
                note(&mut issues, &|| format!("node key {id} has id {}", n.id));
            }
            let mut seen = std::collections::BTreeSet::new();
            for c in &n.children {
                if !seen.insert(*c) {
                    note(&mut issues, &|| format!("child {c} listed twice in {id}"));
                }
                match self.node(*c) {
                    Some(cn) if cn.parent == Some(*id) => {}
                    Some(_) => note(&mut issues, &|| format!("child {c} of {id} has another parent")),
                    None => note(&mut issues, &|| format!("child {c} of {id} missing")),
                }
            }
            if let Some(p) = n.parent
                && !self.node(p).is_some_and(|pn| pn.children.contains(id))
            {
                note(&mut issues, &|| format!("{id} not listed in parent {p}"));
            }
            // The library store's root is a page that is never listed.
            let listed = self.pages.contains(id) || (n.is_page() && self.libraries.store == Some(*id));
            if n.is_page() != listed {
                note(&mut issues, &|| format!("page list mismatch for {id}"));
            }
            if n.is_page() && n.parent.is_some() {
                note(&mut issues, &|| format!("page {id} has a parent"));
            }
        }
        let reachable = self.reachable();
        for id in self.nodes.keys().filter(|k| !reachable.contains(*k)) {
            note(&mut issues, &|| format!("{id} is not under a page (cycle or detached)"));
        }
        issues
    }

    /// Every node reachable from a page through reciprocal parent/child links, each once (cycles
    /// and repeated children are walked once, so this is linear in the node count).
    fn reachable(&self) -> std::collections::BTreeSet<NodeId> {
        let mut seen = std::collections::BTreeSet::new();
        let mut stack: Vec<NodeId> = self.nodes.iter().filter(|(_, n)| n.is_page() && n.parent.is_none()).map(|(k, _)| *k).collect();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(n) = self.nodes.get(&id) else { continue };
            stack.extend(n.children.iter().filter(|c| !seen.contains(*c) && self.nodes.get(*c).is_some_and(|cn| cn.parent == Some(id))));
        }
        seen
    }

    /// Repair what [`validate`](Self::validate) reports (lenient loading of untrusted files).
    pub fn repair(&mut self) {
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in &ids {
            if let Some(n) = self.nodes.get_mut(id) {
                let n = Arc::make_mut(n);
                n.id = *id;
                // Pages are roots.
                if n.is_page() {
                    n.parent = None;
                }
            }
        }
        // Drop dangling children, children claimed by another parent and repeats.
        let parents: BTreeMap<NodeId, Option<NodeId>> = self.nodes.iter().map(|(k, n)| (*k, n.parent)).collect();
        for id in &ids {
            let Some(n) = self.nodes.get(id) else { continue };
            let mut seen = std::collections::BTreeSet::new();
            let keep: Vec<NodeId> =
                n.children.iter().copied().filter(|c| parents.get(c).copied().flatten() == Some(*id) && seen.insert(*c)).collect();
            if keep != n.children
                && let Some(n) = self.nodes.get_mut(id)
            {
                Arc::make_mut(n).children = keep;
            }
        }
        // Nodes not under a page (orphans, detached subtrees, cycles) are removed.
        let reachable = self.reachable();
        let gone: Vec<NodeId> = self.nodes.keys().filter(|k| !reachable.contains(*k)).copied().collect();
        for k in gone {
            self.nodes.remove(&k);
        }
        // Snapshots of layers that are gone or no longer changed since marked ready for dev, or that
        // don't start with their layer, are dropped.
        self.dev_snapshots.retain(|k, snap| {
            self.nodes.get(k).is_some_and(|n| n.dev_status == DevStatus::ReadyForDev && n.dev_changed)
                && snap.nodes.first().is_some_and(|n| n.id == *k)
        });
        let store = self.libraries.store.filter(|s| self.nodes.get(s).is_some_and(|n| n.is_page()));
        if store != self.libraries.store {
            Arc::make_mut(&mut self.libraries).store = store;
        }
        self.pages.retain(|p| self.nodes.get(p).is_some_and(|n| n.is_page()) && Some(*p) != store);
        for (k, n) in self.nodes.iter() {
            if n.is_page() && !self.pages.contains(k) && Some(*k) != store {
                self.pages.push(*k);
            }
        }
        if self.pages.is_empty() {
            let p = self.add_page("Page 1");
            self.current_page = Some(p);
        }
        let client = self.client_id();
        self.set_client_id(client);
        if self.current_page.is_none_or(|p| !self.pages.contains(&p)) {
            self.current_page = self.pages.first().copied();
        }
        self.link_layer_text_styles();
    }
}
