//! Symbols: masters are main components; each instance becomes an instance of its main with a
//! copy of the main's layers (each linked to the layer it mirrors), then its text and image
//! overrides. Symbols from libraries come from the document's `foreignSymbols`.

use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::color::PaintKind;
use figcraft_doc::{Node, NodeId, NodeKind};
use serde_json::Value;

use super::json::{clip, flag, text};
use super::{Ctx, MAX_NODES, Pending};
use crate::import_report::Severity;

/// Most links followed from a copied layer back to the file layer it mirrors.
const MAX_SOURCE_HOPS: usize = 64;

impl Ctx<'_> {
    /// Symbols from libraries that instances in the file use, on a page of their own.
    pub(super) fn foreign_symbols(&mut self, document: &Value) {
        let wanted: BTreeSet<String> = self.instances.iter().map(|p| p.symbol.clone()).filter(|s| !self.masters.contains_key(s)).collect();
        let mut page = None;
        for f in super::json::list(document, "foreignSymbols") {
            let Some(m) = f.get("symbolMaster") else { continue };
            let Some(symbol) = text(m, "symbolID").map(clip) else { continue };
            if !wanted.contains(symbol) || self.masters.contains_key(symbol) {
                continue;
            }
            let p = *page.get_or_insert_with(|| self.doc.add_page("Library symbols"));
            let library = text(f, "sourceLibraryName").map_or("a library", clip).to_string();
            self.layer(m, p, "Library symbols", 0, false);
            self.warn(
                Severity::Info,
                &format!("Library symbols / {}", text(m, "name").map_or("Symbol", clip)),
                format!("This symbol comes from {library}; it was imported as a local component, unlinked from the library."),
            );
        }
    }

    /// Link every instance to its main and give it the main's layers, mains used inside other
    /// mains first.
    pub(super) fn finish_instances(&mut self) {
        let pending = std::mem::take(&mut self.instances);
        let by_id: BTreeMap<NodeId, usize> = pending.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
        let mut done = BTreeSet::new();
        let mut active = BTreeSet::new();
        for i in 0..pending.len() {
            self.expand(i, &pending, &by_id, &mut done, &mut active);
        }
    }

    /// Instance `i` of `pending`, after the instances inside its main.
    fn expand(&mut self, i: usize, pending: &[Pending], by_id: &BTreeMap<NodeId, usize>, done: &mut BTreeSet<usize>, active: &mut BTreeSet<usize>) {
        if done.contains(&i) || !active.insert(i) {
            return;
        }
        let Some(p) = pending.get(i) else { return };
        let Some(&main) = self.masters.get(&p.symbol) else {
            self.warn(Severity::Dropped, &p.path, "The instance's symbol isn't in the file; it was imported as an empty frame.");
            done.insert(i);
            return;
        };
        if self.doc.is_ancestor(main, p.id) {
            self.warn(Severity::Dropped, &p.path, "The instance is inside its own symbol; it was imported as an empty frame.");
            done.insert(i);
            return;
        }
        let inner: Vec<usize> = self.doc.subtree(main).iter().filter_map(|n| by_id.get(n).copied()).collect();
        for k in inner {
            if active.contains(&k) && !done.contains(&k) {
                self.warn(Severity::Dropped, &p.path, "The symbol contains an instance of itself; that instance was imported as an empty frame.");
                continue;
            }
            self.expand(k, pending, by_id, done, active);
        }
        self.link(p, main);
        done.insert(i);
    }

    /// Make the frame `p.id` an instance of `main` holding copies of its layers.
    fn link(&mut self, p: &Pending, main: NodeId) {
        let Some(m) = self.doc.node(main).cloned() else { return };
        // Each child of the main with its subtree, parents first.
        let trees: Vec<(NodeId, Vec<Node>)> =
            m.children.iter().map(|c| (*c, self.doc.subtree(*c).into_iter().filter_map(|id| self.doc.node(id).cloned()).collect())).collect();
        let total: usize = trees.iter().map(|(_, t)| t.len()).sum();
        if self.count.saturating_add(total) > MAX_NODES {
            self.warn(Severity::Dropped, "", format!("The file has more than {MAX_NODES} layers; the rest were not imported."));
            return;
        }
        let map: BTreeMap<NodeId, NodeId> = trees.iter().flat_map(|(_, t)| t.iter().map(|n| n.id)).map(|id| (id, self.doc.alloc_id())).collect();
        for (root, tree) in &trees {
            let _ = self.doc.paste_nodes(tree, &map, *root, p.id, None);
        }
        self.count += total;
        for (orig, copy) in &map {
            if let Some(n) = self.doc.node_mut(*copy) {
                n.source = Some(*orig);
                n.overrides.clear();
                n.prop_refs.clear();
            }
        }
        let background = self.instance_backgrounds.contains(&main);
        let Some(inst) = self.doc.node_mut(p.id) else { return };
        inst.kind = NodeKind::Instance { component: main, props: BTreeMap::new() };
        inst.source = Some(main);
        inst.clip = m.clip;
        // A symbol's background shows on its instances only when it says so.
        inst.fills = if background { m.fills.clone() } else { Vec::new() };
        // The instance's own style is kept: what differs from the main is an override.
        let mut over = BTreeSet::new();
        for (differs, group) in [(inst.fills != m.fills, "fills"), (inst.effects != m.effects, "effects"), (inst.strokes != m.strokes, "strokes")] {
            if differs {
                over.insert(group);
            }
        }
        if inst.opacity != m.opacity {
            over.insert("opacity");
        }
        let resized = (inst.width - m.width).abs() > 1e-6 || (inst.height - m.height).abs() > 1e-6;
        if resized {
            over.insert("size");
        }
        inst.overrides.extend(over.into_iter().map(str::to_string));
        if resized {
            self.warn(
                Severity::Approximated,
                &p.path,
                "The instance is resized; its layers keep the symbol's layout, as constraints weren't imported.",
            );
        }
        for o in &p.overrides {
            self.apply_override(p, o);
        }
    }

    /// One `overrideValues` entry: `<layer id>[/<layer id>…]_<property>`.
    fn apply_override(&mut self, p: &Pending, o: &Value) {
        let Some(name) = text(o, "overrideName") else { return };
        let Some((ids, prop)) = name.rsplit_once('_') else {
            self.warn(Severity::Dropped, &p.path, "An override with an unreadable name was not imported.");
            return;
        };
        let Some(target) = self.override_target(p.id, ids) else {
            self.warn(Severity::Dropped, &p.path, "An override for a layer the symbol doesn't have was not imported.");
            return;
        };
        let target_path = self.doc.node(target).map_or_else(|| p.path.clone(), |n| format!("{} / {}", p.path, n.name));
        match prop {
            "stringValue" => {
                let Some(s) = text(o, "value") else {
                    self.warn(Severity::Dropped, &target_path, "A text override without text was not imported.");
                    return;
                };
                let Some(n) = self.doc.node_mut(target) else { return };
                let Some(td) = n.kind.text_mut() else {
                    self.warn(Severity::Dropped, &target_path, "A text override on a layer that isn't text was not imported.");
                    return;
                };
                let styled = !td.runs.is_empty();
                td.characters = s.chars().take(1 << 20).collect();
                td.runs.clear();
                td.normalize();
                n.overrides.insert("characters".into());
                if styled {
                    self.warn(Severity::Approximated, &target_path, "The overridden text uses the layer's first style for all of it.");
                }
            }
            "image" => {
                let Some(key) = o.get("value").and_then(|v| self.image_key(v, &target_path)) else { return };
                let Some(n) = self.doc.node_mut(target) else { return };
                match n.fills.iter_mut().find_map(|f| if let PaintKind::Image(img) = &mut f.kind { Some(img) } else { None }) {
                    Some(img) => {
                        img.image = key;
                        n.overrides.insert("fills".into());
                    }
                    None => self.warn(Severity::Dropped, &target_path, "An image override on a layer without an image was not imported."),
                }
            }
            "symbolID" => self.warn(Severity::Dropped, &target_path, "Swapping the nested symbol was not imported; it shows its own symbol."),
            "layerStyle" => self.warn(Severity::Dropped, &target_path, "The layer style override was not imported."),
            other => self.warn(Severity::Dropped, &target_path, format!("The “{}” override was not imported.", clip(other))),
        }
    }

    /// The layer of instance `inst` that the override path `ids` (`a/b/…`, file layer ids through
    /// nested instances) names.
    fn override_target(&self, inst: NodeId, ids: &str) -> Option<NodeId> {
        let mut scope = inst;
        for id in ids.split('/') {
            scope = self.doc.subtree(scope).into_iter().skip(1).find(|n| self.origin_id(*n).as_deref() == Some(id))?;
        }
        Some(scope)
    }

    /// The file id of the layer a copied layer mirrors, following its links back.
    fn origin_id(&self, mut id: NodeId) -> Option<String> {
        for _ in 0..MAX_SOURCE_HOPS {
            if let Some(s) = self.sketch_ids.get(&id) {
                return Some(s.clone());
            }
            id = self.doc.node(id)?.source?;
        }
        None
    }
}

/// Whether symbol master JSON `j` shows its background on its instances.
pub(super) fn background_in_instance(j: &Value) -> bool {
    flag(j, "hasBackgroundColor") == Some(true) && flag(j, "includeBackgroundColorInInstance") == Some(true)
}
