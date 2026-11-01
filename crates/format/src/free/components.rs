//! Components and instances: each `COMPONENT` is a main component (by its `componentId`); each
//! `INSTANCE` becomes an instance of its main holding copies of the main's layers (each linked to
//! the layer it mirrors), then its own style and its `overrides`. Components from shared
//! libraries (`shared/*.json` › `components`) that instances use are imported on a page of
//! their own.

use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::color::PaintKind;
use figcraft_doc::{Node, NodeId, NodeKind};
use serde_json::Value;

use super::json::{clip, flag, list, num, text};
use super::layers::Scope;
use super::{Ctx, MAX_NODES};
use crate::import_report::Severity;

/// Most links followed from a copied layer back to the file layer it mirrors.
const MAX_SOURCE_HOPS: usize = 64;
/// `Override` fields brought across.
const OVERRIDE_KEYS: &[&str] = &["target", "hidden", "opacity", "blendMode", "text", "fills", "borders", "image"];

/// An instance waiting for its main component (every page is read first).
pub(super) struct Pending {
    id: NodeId,
    component: String,
    instance: Value,
    path: String,
}

impl Ctx<'_> {
    /// A `COMPONENT`: its main component, by `componentId`.
    pub(super) fn component(&mut self, id: NodeId, j: &Value, path: &str) {
        let Some(cid) = text(j, "componentId").map(clip) else {
            self.warn(Severity::Dropped, path, "The component has no component id; its instances can't be linked to it.");
            return;
        };
        if self.mains.contains_key(cid) {
            self.warn(Severity::Approximated, path, "Another component has the same component id; instances use the first one.");
        } else {
            self.mains.insert(cid.to_string(), id);
        }
        if !list(j, "properties").is_empty() {
            self.warn(Severity::Dropped, path, "The component's properties (boolean, text, swap, slot, state) were not imported.");
        }
        if !list(j, "states").is_empty() {
            self.warn(Severity::Dropped, path, "The component's state bindings were not imported.");
        }
    }

    /// An `INSTANCE`, linked to its main once every page is read.
    pub(super) fn pending(&mut self, id: NodeId, j: &Value, path: &str) {
        let Some(cid) = text(j, "componentId") else {
            self.warn(Severity::Dropped, path, "The instance names no component; it was imported as an empty frame.");
            return;
        };
        if !list(j, "settings").is_empty() {
            self.warn(Severity::Dropped, path, "The instance's component property values were not imported.");
        }
        if num(j, "scale").is_some_and(|s| s != 0.0 && (s - 1.0).abs() > 1e-9) {
            self.warn(Severity::Approximated, path, "The instance's scale was not applied; its layers show at the component's size.");
        }
        let mut instance = j.clone();
        if let Some(m) = instance.as_object_mut() {
            m.remove("layers");
        }
        self.instances.push(Pending { id, component: clip(cid).to_string(), instance, path: path.to_string() });
    }

    /// Components from shared libraries that instances use but the pages don't hold.
    pub(super) fn library_components(&mut self, libraries: &[Value]) {
        let mut page = None;
        // Library components can hold instances of other library components: repeat until
        // nothing more is wanted.
        for _ in 0..64 {
            let wanted: BTreeSet<String> = self.instances.iter().map(|p| p.component.clone()).filter(|c| !self.mains.contains_key(c)).collect();
            let mut added = false;
            for lib in libraries {
                let library = text(lib, "name").map_or("a shared library", clip).to_string();
                for c in list(lib, "components") {
                    let Some(cid) = text(c, "componentId").map(clip) else { continue };
                    if !wanted.contains(cid) || self.mains.contains_key(cid) {
                        continue;
                    }
                    let p = *page.get_or_insert_with(|| self.doc.add_page("Library components"));
                    let before = self.mains.len();
                    self.layer(c, p, "Library components", 0, Scope::TOP);
                    added |= self.mains.len() > before;
                    self.warn(
                        Severity::Info,
                        &format!("Library components / {}", text(c, "name").map_or("Component", clip)),
                        format!("This component comes from {library}; it was imported as a local component, unlinked from the library."),
                    );
                }
            }
            if !added {
                break;
            }
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
        let Some(&main) = self.mains.get(&p.component) else {
            self.warn(Severity::Dropped, &p.path, "The instance's component isn't in the file; it was imported as an empty frame.");
            done.insert(i);
            return;
        };
        if self.doc.is_ancestor(main, p.id) {
            self.warn(Severity::Dropped, &p.path, "The instance is inside its own component; it was imported as an empty frame.");
            done.insert(i);
            return;
        }
        let inner: Vec<usize> = self.doc.subtree(main).iter().filter_map(|n| by_id.get(n).copied()).collect();
        for k in inner {
            if active.contains(&k) && !done.contains(&k) {
                self.warn(Severity::Dropped, &p.path, "The component contains an instance of itself; that instance was imported as an empty frame.");
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
        let j = &p.instance;
        let own = |keys: &[&str]| keys.iter().any(|k| j.get(*k).is_some());
        let Some(inst) = self.doc.node_mut(p.id) else { return };
        inst.kind = NodeKind::Instance { component: main, props: BTreeMap::new() };
        inst.source = Some(main);
        inst.clip = m.clip;
        let mut over = BTreeSet::new();
        // What the instance doesn't set itself comes from its main; what it sets is an override.
        let groups: [(&[&str], &str); 5] = [
            (&["fill", "fills"], "fills"),
            (&["border", "borders", "thickness", "customThickness", "linePos", "lineCap", "lineJoin", "dash"], "strokes"),
            (&["shadows", "innerShadows", "blur"], "effects"),
            (&["opacity", "blendMode"], "opacity"),
            (&["cornerRadius", "smoothCorners"], "corners"),
        ];
        for (keys, group) in groups {
            if own(keys) {
                over.insert(group);
                continue;
            }
            match group {
                "fills" => inst.fills = m.fills.clone(),
                "strokes" => {
                    inst.strokes = m.strokes.clone();
                    inst.stroke = m.stroke.clone();
                }
                "effects" => inst.effects = m.effects.clone(),
                "opacity" => {
                    inst.opacity = m.opacity;
                    inst.blend = m.blend;
                }
                _ => inst.corners = m.corners,
            }
        }
        if inst.layout.is_none() {
            inst.layout = m.layout.clone();
        }
        let resized = (inst.width - m.width).abs() > 1e-6 || (inst.height - m.height).abs() > 1e-6;
        if resized {
            over.insert("size");
        }
        inst.overrides.extend(over.into_iter().map(str::to_string));
        if resized {
            self.warn(Severity::Approximated, &p.path, "The instance is resized; its layers keep the component's positions and sizes.");
        }
        for o in super::json::list(j, "overrides").iter().take(10_000) {
            self.apply_override(p, o);
        }
    }

    /// One `Override`: `target` names the layer (file layer ids through nested instances).
    fn apply_override(&mut self, p: &Pending, o: &Value) {
        let ids: Vec<&str> = list(o, "target").iter().filter_map(Value::as_str).take(64).collect();
        let Some(target) = (!ids.is_empty()).then(|| self.override_target(p.id, &ids)).flatten() else {
            self.warn(Severity::Dropped, &p.path, "An override for a layer the component doesn't have was not imported.");
            return;
        };
        let tpath = self.doc.node(target).map_or_else(|| p.path.clone(), |n| format!("{} / {}", p.path, n.name));
        if let Some(h) = flag(o, "hidden")
            && let Some(n) = self.doc.node_mut(target)
        {
            n.visible = !h;
            n.overrides.insert("visible".into());
        }
        if o.get("opacity").is_some() || o.get("blendMode").is_some() {
            let opacity = num(o, "opacity").map(|x| x.clamp(0.0, 1.0) as f32);
            let blend = super::json::int(o, "blendMode").map(|b| super::style::blend(Some(b)));
            if let Some(n) = self.doc.node_mut(target) {
                if let Some(x) = opacity {
                    n.opacity = x;
                }
                if let Some(b) = blend {
                    n.blend = b;
                }
                n.overrides.insert("opacity".into());
            }
        }
        for (key, one, group) in [("fills", "fill", "fills"), ("borders", "border", "strokes")] {
            if o.get(key).is_none() {
                continue;
            }
            let paints = self.paints(o, key, one, &tpath);
            if let Some(n) = self.doc.node_mut(target) {
                if group == "fills" {
                    n.fills = paints;
                } else {
                    n.strokes = paints;
                }
                n.overrides.insert(group.into());
            }
        }
        if let Some(s) = text(o, "text")
            && let Some(n) = self.doc.node_mut(target)
        {
            match n.kind.text_mut() {
                Some(td) => {
                    let styled = !td.runs.is_empty();
                    td.characters = s.chars().take(1 << 20).collect();
                    td.runs.clear();
                    td.normalize();
                    n.overrides.insert("characters".into());
                    if styled {
                        self.warn(Severity::Approximated, &tpath, "The overridden text uses the layer's first style for all of it.");
                    }
                }
                None => self.warn(Severity::Dropped, &tpath, "A text override on a layer that isn't text was not imported."),
            }
        }
        if let Some(name) = text(o, "image")
            && let Some(key) = self.image_key(name, &tpath)
            && let Some(n) = self.doc.node_mut(target)
        {
            match n.fills.iter_mut().find_map(|f| if let PaintKind::Image(img) = &mut f.kind { Some(img) } else { None }) {
                Some(img) => {
                    img.image = key;
                    n.overrides.insert("fills".into());
                }
                None => self.warn(Severity::Dropped, &tpath, "An image override on a layer without an image fill was not imported."),
            }
        }
        if o.get("componentId").is_some() {
            self.warn(Severity::Dropped, &tpath, "Swapping the nested instance's component was not imported; it shows its own component.");
        }
        let rest: Vec<String> = o
            .as_object()
            .into_iter()
            .flat_map(|m| m.keys())
            .filter(|k| !OVERRIDE_KEYS.contains(&k.as_str()) && k.as_str() != "componentId")
            .take(8)
            .map(|k| format!("`{}`", clip(k)))
            .collect();
        if !rest.is_empty() {
            self.warn(Severity::Dropped, &tpath, format!("Overrides of {} were not imported.", rest.join(", ")));
        }
    }

    /// The layer of instance `inst` that the override path `ids` names.
    fn override_target(&self, inst: NodeId, ids: &[&str]) -> Option<NodeId> {
        let mut scope = inst;
        for id in ids {
            scope = self.doc.subtree(scope).into_iter().skip(1).find(|n| self.origin_id(*n).as_deref() == Some(*id))?;
        }
        Some(scope)
    }

    /// The file id of the layer a copied layer mirrors, following its links back.
    fn origin_id(&self, mut id: NodeId) -> Option<String> {
        for _ in 0..MAX_SOURCE_HOPS {
            if let Some(s) = self.free_ids.get(&id) {
                return Some(s.clone());
            }
            id = self.doc.node(id)?.source?;
        }
        None
    }
}
