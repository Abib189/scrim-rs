//! Pages and the layer tree: `files/{file}/pages/{page}.json` and one
//! `files/{file}/pages/{page}/{shape}.json` per shape, nested by the containers' `shapes` lists.

use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::geom::Affine;
use figcraft_doc::{Node, NodeId, NodeKind, VectorNetwork};
use serde_json::Value;

use super::read::{NIL, clip, flag, list, num, present, rect, safe_id, text};
use super::{Ctx, MAX_DEPTH, MAX_PAGES, MAX_SHAPES};
use crate::import_report::Severity;

/// One page's shapes by id, and the ones imported so far.
struct Shapes {
    by_id: BTreeMap<String, Value>,
    seen: BTreeSet<String>,
}

/// The label a shape of `kind` gets when it has no name.
fn label(kind: &str) -> &'static str {
    match kind {
        "frame" => "Board",
        "group" => "Group",
        "rect" => "Rectangle",
        "circle" => "Ellipse",
        "path" => "Path",
        "text" => "Text",
        "image" => "Image",
        "bool" => "Boolean",
        "svg-raw" => "SVG",
        _ => "Layer",
    }
}

impl Ctx<'_> {
    /// Every page of the file, by `index` (pages without one after, by member name).
    pub(super) fn pages(&mut self) {
        let prefix = format!("files/{}/pages/", self.file_id);
        let mut pages: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for name in self.archive.names() {
            let Some(rest) = name.strip_prefix(&prefix) else { continue };
            match rest.split_once('/') {
                None => {
                    if let Some(id) = rest.strip_suffix(".json").filter(|id| safe_id(id)) {
                        pages.entry(id.to_string()).or_default();
                    }
                }
                Some((page, shape)) => {
                    if let Some(id) = shape.strip_suffix(".json").filter(|id| safe_id(id) && safe_id(page)) {
                        pages.entry(page.to_string()).or_default().push(id.to_string());
                    }
                }
            }
        }
        let mut read = Vec::new();
        for (id, shapes) in pages {
            if read.len() >= MAX_PAGES {
                self.warn(Severity::Dropped, "", format!("The file has more than {MAX_PAGES} pages; the rest were not imported."));
                break;
            }
            let member = format!("{prefix}{id}.json");
            if !self.archive.has(&member) {
                self.warn(Severity::Dropped, "", format!("The shapes of page {id} were not imported: the page itself isn't in the file."));
                continue;
            }
            match self.archive.json(&member) {
                Ok(v) if v.is_object() => read.push((num(&v, "index").map_or(f64::INFINITY, |i| i.max(-1e15)), id, v, shapes)),
                Ok(_) => self.warn(Severity::Dropped, "", format!("Page {id} isn't a JSON object and was not imported.")),
                Err(e) => {
                    self.warn(Severity::Dropped, "", format!("Page {id} couldn't be read ({}) and was not imported.", super::archive::message(&e)))
                }
            }
        }
        read.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        if read.is_empty() {
            self.warn(Severity::Info, "", "The file has no pages.");
        }
        for (i, (_, id, v, shapes)) in read.into_iter().enumerate() {
            self.page(&id, &v, &shapes, i);
        }
    }

    /// One page and its layers.
    fn page(&mut self, id: &str, v: &Value, shape_ids: &[String], ordinal: usize) {
        let name = text(v, "name").filter(|n| !n.trim().is_empty()).map_or_else(|| format!("Page {}", ordinal + 1), |n| clip(n).to_string());
        let page = self.doc.add_page(&name);
        if let Some(bg) = text(v, "background") {
            match super::read::hex(bg, 1.0) {
                Some(c) => {
                    if let Some(NodeKind::Page { background, .. }) = self.doc.node_mut(page).map(|n| &mut n.kind) {
                        *background = c.with_alpha(1.0);
                    }
                }
                None => self.warn(Severity::Approximated, &name, "The page background isn't a hex colour; the default background was used."),
            }
        }
        for (key, what) in [("flows", "Prototype flows"), ("guides", "Ruler guides"), ("options", "Page options")] {
            if present(v, key) {
                self.warn(Severity::Dropped, &name, format!("{what} weren't imported: the documentation doesn't describe their structure."));
            }
        }
        let mut shapes = Shapes { by_id: BTreeMap::new(), seen: BTreeSet::new() };
        let prefix = format!("files/{}/pages/{id}/", self.file_id);
        for sid in shape_ids {
            if self.loaded >= MAX_SHAPES {
                self.warn(Severity::Dropped, "", format!("The file has more than {MAX_SHAPES} layers; the rest were not imported."));
                break;
            }
            match self.archive.json(&format!("{prefix}{sid}.json")) {
                Ok(s) if s.is_object() => {
                    self.loaded += 1;
                    let key = text(&s, "id").map_or_else(|| sid.clone(), |i| clip(i).to_string());
                    shapes.by_id.entry(key).or_insert(s);
                }
                Ok(_) => self.warn(Severity::Dropped, &name, format!("Layer {sid} isn't a JSON object and was not imported.")),
                Err(e) => {
                    let m = format!("Layer {sid} couldn't be read ({}) and was not imported.", super::archive::message(&e));
                    self.warn(Severity::Dropped, &name, m);
                }
            }
        }
        for root in self.roots(&shapes, &name) {
            self.shape(&mut shapes, &root, page, (0.0, 0.0), &name, 0);
        }
        let left: Vec<String> = shapes.by_id.keys().filter(|k| k.as_str() != NIL && !shapes.seen.contains(*k)).cloned().collect();
        for k in left {
            let n = shapes.by_id.get(&k).and_then(|s| text(s, "name")).map_or("Layer", clip).to_string();
            self.warn(Severity::Dropped, &format!("{name} / {n}"), "The layer isn't in its parent's list of shapes, so it was not imported.");
        }
    }

    /// The page's top-level shapes, bottom first. Reading: a shape with the all-zero id is the
    /// page's root, and its `shapes` list gives the order; shapes whose `parentId` is that id
    /// (or names no shape of the page) are top-level too.
    fn roots(&mut self, shapes: &Shapes, page: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut listed = BTreeSet::new();
        if let Some(root) = shapes.by_id.get(NIL) {
            for c in list(root, "shapes").iter().filter_map(Value::as_str) {
                if listed.insert(c.to_string()) {
                    out.push(clip(c).to_string());
                }
            }
        }
        let mut unordered = 0;
        for (id, s) in &shapes.by_id {
            if id == NIL || listed.contains(id) {
                continue;
            }
            let top = match text(s, "parentId") {
                None => true,
                Some(p) => p == NIL || p == id || !shapes.by_id.contains_key(p),
            };
            if top {
                out.push(id.clone());
                unordered += 1;
            }
        }
        if unordered > 1 || (unordered == 1 && !listed.is_empty()) {
            self.warn(
                Severity::Info,
                page,
                "The file doesn't give the order of some top-level layers (no page root lists them); they were stacked by id.",
            );
        }
        out
    }

    /// Import shape `id` (and its subtree) under `parent`, whose box starts at page point
    /// `origin`. `path`: the parent's layer path.
    fn shape(&mut self, shapes: &mut Shapes, id: &str, parent: NodeId, origin: (f64, f64), path: &str, depth: usize) {
        let Some(v) = shapes.by_id.get(id).cloned() else {
            self.warn(Severity::Dropped, path, format!("A list of shapes names layer {} that isn't in the file.", clip(id)));
            return;
        };
        let kind = text(&v, "type").unwrap_or("");
        let name = text(&v, "name").filter(|n| !n.is_empty()).map_or(label(kind), clip).to_string();
        let lpath = format!("{path} / {name}");
        if !shapes.seen.insert(id.to_string()) {
            self.warn(Severity::Approximated, &lpath, "The layer is listed more than once (or inside itself); it was imported once.");
            return;
        }
        if depth > MAX_DEPTH {
            self.warn(Severity::Dropped, &lpath, format!("Layers nested deeper than {MAX_DEPTH} levels were not imported."));
            return;
        }
        let main = self.mains.get(id).cloned();
        let node_kind = match kind {
            "frame" | "group" if main.is_some() => NodeKind::Component { description: String::new(), props: Default::default() },
            "frame" => NodeKind::Frame,
            "group" => NodeKind::Group,
            "rect" | "image" => NodeKind::Rectangle,
            "circle" => NodeKind::Ellipse { start: 0.0, sweep: std::f64::consts::TAU, inner: 0.0 },
            "path" => NodeKind::Vector { network: VectorNetwork::default() },
            "bool" => match super::shapes::bool_op(&v) {
                Some(op) => NodeKind::BooleanOp { op },
                None => {
                    let m = "The boolean's boolType isn't union, difference, exclude or intersection; it was imported as a group.";
                    self.warn(Severity::Approximated, &lpath, m);
                    NodeKind::Group
                }
            },
            "text" => {
                let m = "The text layer was not imported: the documentation doesn't describe the structure of text content.";
                self.warn(Severity::Dropped, &lpath, m);
                return;
            }
            "svg-raw" => {
                let m = "The raw SVG layer was not imported: the documentation doesn't describe its content.";
                self.warn(Severity::Dropped, &lpath, m);
                return;
            }
            "" => {
                self.warn(Severity::Dropped, &lpath, "A layer without a type was not imported.");
                return;
            }
            other => {
                self.warn(Severity::Dropped, &lpath, format!("Layers of type “{}” are not supported and were not imported.", clip(other)));
                return;
            }
        };
        if let Some(c) = &main
            && !matches!(node_kind, NodeKind::Component { .. })
        {
            let m = format!("The main component “{c}” isn't a board or group; it was imported as a plain layer.");
            self.warn(Severity::Approximated, &lpath, m);
        }
        let (x, y, w, h) = self.geometry(&v, kind, &lpath);
        let mut n = Node::new(self.doc.alloc_id(), node_kind, name, w, h);
        n.transform = Affine::translate((x - origin.0, y - origin.1));
        n.fills.clear();
        n.strokes.clear();
        self.transform(&v, &lpath);
        self.common(&mut n, &v, &lpath);
        self.unsupported(&v, &lpath);
        let fills = self.fills(&v, "fills", w, h, &lpath);
        match kind {
            "frame" => {
                n.clip = true;
                if flag(&v, "showContent") == Some(true) {
                    let m = "The board's showContent flag is on; the documentation doesn't say what it does, so the board clips its content.";
                    self.warn(Severity::Approximated, &lpath, m);
                }
                self.radii(&mut n, &v, &lpath);
                n.fills = fills;
            }
            "group" => {
                n.clip = false;
                if !fills.is_empty() {
                    self.warn(Severity::Dropped, &lpath, "Fills on a group weren't imported.");
                }
            }
            "rect" => {
                self.radii(&mut n, &v, &lpath);
                n.fills = fills;
            }
            "image" => {
                self.radii(&mut n, &v, &lpath);
                n.fills = self.image_fill(&v, w, h, &lpath).into_iter().collect();
                if !fills.is_empty() {
                    self.warn(Severity::Dropped, &lpath, "Fills on the image layer weren't imported.");
                }
            }
            "path" => {
                if let Some(net) = self.path_network(&v, (x, y, w, h), &lpath) {
                    n.kind = NodeKind::Vector { network: net };
                }
                n.fills = fills;
            }
            _ => n.fills = fills,
        }
        if let Some(c) = text(&v, "componentId")
            && main.is_none()
        {
            let what = self.component_names.get(c).map_or_else(|| "a component".to_string(), |n| format!("“{n}”"));
            let m = format!("An instance of {what} was imported as plain layers: the documentation doesn't describe how instances link.");
            self.warn(Severity::Approximated, &lpath, m);
        } else if main.is_none() && (present(&v, "shapeRef") || present(&v, "touched")) {
            self.instance_parts += 1;
        }
        let container = matches!(kind, "frame" | "group" | "bool");
        let id_new = n.id;
        if self.doc.insert(n, parent, None).is_err() {
            return;
        }
        if main.is_some() {
            self.found_mains.insert(id.to_string());
        }
        let kids: Vec<String> = list(&v, "shapes").iter().filter_map(Value::as_str).map(|s| clip(s).to_string()).collect();
        if !container {
            if !kids.is_empty() {
                self.warn(Severity::Dropped, &lpath, format!("The layers listed inside this {} were not imported.", label(kind).to_lowercase()));
            }
            return;
        }
        for c in &kids {
            self.shape(shapes, c, id_new, (x, y), &lpath, depth + 1);
        }
        self.finish_container(id_new, kind, &v, (x, y, w, h), &lpath);
    }

    /// The shape's page position and size: `x`, `y`, `width`, `height` for the types the
    /// documentation gives them to, else (and as a fallback) `selrect`.
    fn geometry(&mut self, v: &Value, kind: &str, path: &str) -> (f64, f64, f64, f64) {
        let own = matches!(kind, "frame" | "rect" | "circle" | "image" | "svg-raw" | "text").then(|| rect(v)).flatten();
        if let Some(g) = own.or_else(|| v.get("selrect").and_then(rect)) {
            return g;
        }
        self.warn(Severity::Approximated, path, "The layer has no position or size; it was placed at its parent's origin with no size.");
        (0.0, 0.0, 0.0, 0.0)
    }

    /// Fit a group or boolean to its layers; a boolean without any becomes the path its
    /// `content` gives, when that reads.
    fn finish_container(&mut self, id: NodeId, kind: &str, v: &Value, g: (f64, f64, f64, f64), path: &str) {
        let empty = self.doc.node(id).is_some_and(|n| n.children.is_empty());
        if kind == "bool" && empty {
            match v.get("content").and_then(Value::as_str).map(|_| self.path_network(v, g, path)) {
                Some(Some(net)) => {
                    if let Some(n) = self.doc.node_mut(id) {
                        n.kind = NodeKind::Vector { network: net };
                    }
                    self.warn(Severity::Approximated, path, "The boolean has no operands in the file; it was imported as the path it results in.");
                }
                _ => self.warn(Severity::Dropped, path, "The boolean has no operands in the file; it was imported empty."),
            }
            return;
        }
        if self.doc.node(id).is_some_and(|n| n.kind.bounds_follow_children()) {
            let _ = self.doc.fit_to_children(id);
        }
    }
}
