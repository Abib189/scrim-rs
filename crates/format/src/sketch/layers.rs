//! The layer tree: each Sketch layer becomes a node, placed by its frame, rotation and flips.

use figcraft_doc::geom::Affine;
use figcraft_doc::{BoolOp, MaskType, Node, NodeId, NodeKind, WindingRule};
use serde_json::Value;

use super::json::{self, clip, flag, int, list, num, text};
use super::{Ctx, MAX_DEPTH, MAX_NODES, Pending};
use crate::import_report::Severity;

/// The label a layer of `class` gets when it has no name.
fn label(class: &str) -> &'static str {
    match class {
        "artboard" => "Artboard",
        "symbolMaster" => "Symbol",
        "group" | "shapeGroup" => "Group",
        "rectangle" => "Rectangle",
        "oval" => "Oval",
        "polygon" => "Polygon",
        "star" => "Star",
        "triangle" => "Triangle",
        "shapePath" => "Path",
        "text" => "Text",
        "bitmap" => "Image",
        "symbolInstance" => "Instance",
        "slice" => "Slice",
        _ => "Layer",
    }
}

/// How a Sketch `booleanOperation` combines an operand with the ones below it.
fn bool_op(v: &Value) -> Option<BoolOp> {
    match int(v, "booleanOperation") {
        Some(0) => Some(BoolOp::Union),
        Some(1) => Some(BoolOp::Subtract),
        Some(2) => Some(BoolOp::Intersect),
        Some(3) => Some(BoolOp::Exclude),
        _ => None,
    }
}

impl Ctx<'_> {
    /// Import layer `j` (and its subtree) under `parent`. `path`: the parent's layer path.
    /// `operand`: `j` is a shape of a shape group (a boolean operand).
    pub(super) fn layer(&mut self, j: &Value, parent: NodeId, path: &str, depth: usize, operand: bool) {
        let class = text(j, "_class").unwrap_or("");
        let name = text(j, "name").filter(|n| !n.is_empty()).map_or(label(class), clip).to_string();
        let lpath = format!("{path} / {name}");
        if depth > MAX_DEPTH {
            self.warn(Severity::Dropped, &lpath, format!("Layers nested deeper than {MAX_DEPTH} levels were not imported."));
            return;
        }
        if self.count >= MAX_NODES {
            self.warn(Severity::Dropped, "", format!("The file has more than {MAX_NODES} layers; the rest were not imported."));
            return;
        }
        let (transform, w, h) = self.placement(j, &lpath);
        let kind = match class {
            "artboard" => NodeKind::Frame,
            "symbolMaster" => NodeKind::Component { description: String::new(), props: Default::default() },
            "group" => NodeKind::Group,
            "shapeGroup" => NodeKind::BooleanOp { op: BoolOp::Union },
            "rectangle" | "oval" | "polygon" | "star" | "triangle" | "shapePath" => NodeKind::Rectangle,
            "text" => NodeKind::Text(Default::default()),
            "bitmap" => NodeKind::Rectangle,
            "symbolInstance" => NodeKind::Frame,
            "slice" => NodeKind::Slice,
            "MSImmutableHotspotLayer" => {
                self.warn(Severity::Dropped, &lpath, "Hotspots (prototype click areas) were not imported.");
                return;
            }
            "" => {
                self.warn(Severity::Dropped, &lpath, "A layer without a `_class` was not imported.");
                return;
            }
            other => {
                self.warn(Severity::Dropped, &lpath, format!("Layers of type “{}” are not supported and were not imported.", clip(other)));
                return;
            }
        };
        let id = self.doc.alloc_id();
        let mut n = Node::new(id, kind, name, w, h);
        n.transform = transform;
        n.fills.clear();
        n.strokes.clear();
        // Text takes its colour from its runs unless the style fills it; shapes and bitmaps read
        // the fills the style gave them.
        if class == "text" {
            self.text(&mut n, j, &lpath);
        }
        self.common(&mut n, j, class, &lpath);
        match class {
            "rectangle" | "oval" | "polygon" | "star" | "triangle" | "shapePath" => self.shape(&mut n, class, j, &lpath),
            "bitmap" => self.bitmap(&mut n, j, &lpath),
            _ => {}
        }
        if operand {
            n.bool_op = bool_op(j);
        }
        let container = n.kind.is_container();
        let follows = n.kind.bounds_follow_children();
        if self.doc.insert(n, parent, None).is_err() {
            return;
        }
        self.count += 1;
        if let Some(oid) = text(j, "do_objectID") {
            self.sketch_ids.insert(id, clip(oid).to_string());
        }
        match class {
            "symbolMaster" => self.master(id, j, &lpath),
            "symbolInstance" if num(j, "scale").is_some_and(|s| (s - 1.0).abs() > 1e-9) => {
                self.warn(Severity::Approximated, &lpath, "The instance's scale was not applied; its layers show at the symbol's size.");
                self.pending(id, j, &lpath);
            }
            "symbolInstance" => self.pending(id, j, &lpath),
            _ => {}
        }
        let kids = json::layers(j);
        if kids.is_empty() {
            return;
        }
        if !container {
            self.warn(Severity::Dropped, &lpath, format!("The layers inside this {} were not imported.", label(class).to_lowercase()));
            return;
        }
        let shape_group = class == "shapeGroup";
        let mut masking = false;
        for c in kids {
            if masking && flag(c, "shouldBreakMaskChain") == Some(true) {
                let child = text(c, "name").map_or("Layer", clip);
                self.warn(
                    Severity::Approximated,
                    &format!("{lpath} / {child}"),
                    "The layer ends the mask chain in Sketch; here the mask below still masks it.",
                );
            }
            masking |= flag(c, "hasClippingMask") == Some(true);
            self.layer(c, id, &lpath, depth + 1, shape_group);
        }
        if shape_group {
            self.operand_winding(id, j);
        }
        if follows {
            let _ = self.doc.fit_to_children(id);
        }
    }

    /// The layer's transform in its parent (frame origin, then rotation and flips about the
    /// frame's centre) and its size.
    fn placement(&mut self, j: &Value, path: &str) -> (Affine, f64, f64) {
        let Some((x, y, w, h)) = json::frame(j) else {
            self.warn(Severity::Approximated, path, "The layer has no frame; it was placed at its parent's origin with no size.");
            return (Affine::IDENTITY, 0.0, 0.0);
        };
        let rot = num(j, "rotation").unwrap_or(0.0).clamp(-1e6, 1e6);
        let sx = if flag(j, "isFlippedHorizontal") == Some(true) { -1.0 } else { 1.0 };
        let sy = if flag(j, "isFlippedVertical") == Some(true) { -1.0 } else { 1.0 };
        let mut t = Affine::translate((x, y));
        if rot != 0.0 || sx < 0.0 || sy < 0.0 {
            let c = (w / 2.0, h / 2.0);
            t = t * Affine::translate(c) * Affine::rotate(-rot.to_radians()) * Affine::scale_non_uniform(sx, sy) * Affine::translate((-c.0, -c.1));
        }
        (t, w, h)
    }

    /// Properties every layer type has.
    fn common(&mut self, n: &mut Node, j: &Value, class: &str, path: &str) {
        n.visible = flag(j, "isVisible").unwrap_or(true);
        n.locked = flag(j, "isLocked").unwrap_or(false);
        if matches!(class, "artboard" | "symbolMaster") {
            n.clip = true;
            if let Some(c) = j.get("backgroundColor").and_then(json::color).filter(|_| flag(j, "hasBackgroundColor") == Some(true)) {
                n.fills = vec![figcraft_doc::color::Paint::solid(c)];
            } else if class == "artboard" {
                // Artboards without a background show white on the canvas.
                n.fills = vec![figcraft_doc::color::Paint::solid(figcraft_doc::color::Color::WHITE)];
            }
            for (key, what) in [("grid", "The grid"), ("layout", "The layout grid")] {
                if j.get(key).and_then(|g| g.get("isEnabled")).and_then(Value::as_bool) == Some(true) {
                    self.warn(Severity::Dropped, path, format!("{what} was not imported."));
                }
            }
            if flag(j, "isFlowHome") == Some(true) {
                self.warn(Severity::Dropped, path, "The prototype start screen setting was not imported.");
            }
        }
        self.style(n, j, path);
        if flag(j, "hasClippingMask") == Some(true) {
            n.is_mask = true;
            // The schema gives the mode as an integer without naming its values: 0 (the
            // default) is read as an outline mask, anything else as an alpha mask.
            n.mask_type = if int(j, "clippingMaskMode").unwrap_or(0) == 0 { MaskType::Vector } else { MaskType::Alpha };
        }
        n.exports = self.exports(j, path);
        if j.get("flow").is_some_and(Value::is_object) {
            self.warn(Severity::Dropped, path, "The prototype link was not imported.");
        }
        if flag(j, "isFixedToViewport") == Some(true) {
            self.warn(Severity::Dropped, path, "Fixing the layer's position when prototypes scroll was not imported.");
        }
        if j.get("groupLayout").and_then(|g| text(g, "_class")) == Some("MSImmutableInferredGroupLayout") {
            self.warn(Severity::Dropped, path, "Smart layout was not imported; the layers keep their positions.");
        }
        if let Some(sid) = text(j, "sharedStyleID").filter(|s| !s.is_empty()) {
            match self.style_names.get(clip(sid)).cloned() {
                Some(name) => self.warn(
                    Severity::Info,
                    path,
                    format!("The link to the shared style “{name}” was not kept; the layer keeps its own copy of the style."),
                ),
                None => self.warn(
                    Severity::Dropped,
                    path,
                    "The layer's shared style isn't in the file; the layer keeps its own copy of the style, unlinked.",
                ),
            }
        }
        if j.get("resizingConstraint").is_some() {
            self.constraints += 1;
        }
    }

    /// A symbol instance, linked to its main once every page is read.
    fn pending(&mut self, id: NodeId, j: &Value, path: &str) {
        match text(j, "symbolID") {
            Some(s) => self.instances.push(Pending {
                id,
                symbol: clip(s).to_string(),
                overrides: list(j, "overrideValues").to_vec(),
                path: path.to_string(),
            }),
            None => self.warn(Severity::Dropped, path, "The symbol instance names no symbol; it was imported as an empty frame."),
        }
    }

    /// A symbol master: its main component, by `symbolID`.
    fn master(&mut self, id: NodeId, j: &Value, path: &str) {
        let Some(symbol) = text(j, "symbolID").map(clip) else {
            self.warn(Severity::Dropped, path, "The symbol has no symbol ID; its instances can't be linked to it.");
            return;
        };
        if self.masters.contains_key(symbol) {
            self.warn(Severity::Approximated, path, "Another symbol has the same symbol ID; instances use the first one.");
            return;
        }
        self.masters.insert(symbol.to_string(), id);
        if super::symbols::background_in_instance(j) {
            self.instance_backgrounds.insert(id);
        }
        let locked = list(j, "overrideProperties").iter().any(|p| flag(p, "canOverride") == Some(false));
        if flag(j, "allowsOverrides") == Some(false) || locked {
            self.warn(Severity::Info, path, "Which overrides the symbol allows was not imported; every property can be overridden here.");
        }
    }

    /// A shape group's even-odd `windingRule` applies to its shapes' fills.
    fn operand_winding(&mut self, group: NodeId, j: &Value) {
        let group_rule = int(j, "windingRule").or_else(|| j.get("style").and_then(|s| int(s, "windingRule")));
        if group_rule != Some(1) {
            return;
        }
        let kids = self.doc.node(group).map(|n| n.children.clone()).unwrap_or_default();
        for k in kids {
            if let Some(NodeKind::Vector { network }) = self.doc.node_mut(k).map(|n| &mut n.kind) {
                for r in &mut network.regions {
                    r.winding = WindingRule::EvenOdd;
                }
            }
        }
    }
}
