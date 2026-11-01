//! The layer tree: each FREE layer becomes a node, placed by its `frame`, `pos` or `transform`
//! and `size`, with the properties every layer has (visibility, locking, opacity, blend, masks,
//! constraints, exports) and a report of what it carries that wasn't brought across.

use figcraft_doc::geom::Affine;
use figcraft_doc::{BoolOp, Constraints, Guide, HConstraint, MaskType, Node, NodeId, NodeKind, VConstraint};
use serde_json::Value;

use super::json::{self, clip, flag, int, list, num, text};
use super::{Ctx, MAX_DEPTH, MAX_NODES};
use crate::import_report::Severity;

/// Fields of the README's `Layer` (every layer type has them).
const LAYER_KEYS: &[&str] = &[
    "_t",
    "id",
    "name",
    "boolOp",
    "locked",
    "hidden",
    "isTemplate",
    "expand",
    "trim",
    "export",
    "constraints",
    "transform",
    "pos",
    "size",
    "frame",
    "lockAspect",
    "mask",
    "breakMask",
    "maskType",
    "keepScroll",
    "scrollBehavior",
    "scrollOverflow",
    "flows",
    "minWidth",
    "minHeight",
    "maxWidth",
    "maxHeight",
    "stretchWidth",
    "stretchHeight",
    "absolutePos",
    "custom",
    "themes",
    "binds",
    "fillsId",
    "bordersId",
    "effectsId",
    "opacity",
    "blendMode",
    "winding",
    "tint",
    "fill",
    "border",
    "fills",
    "borders",
    "thickness",
    "customThickness",
    "linePos",
    "lineCap",
    "lineJoin",
    "dash",
    "shadows",
    "innerShadows",
    "blur",
    "imageFilters",
    "cornerRadius",
    "smoothCorners",
    "photoId",
    "illustrationId",
    "iconId",
    "avatarId",
    "data",
    "upscaled",
    "backgroundRemove",
];
/// `Frame`'s fields (also `Component`, `States` and `Slot`).
const FRAME_KEYS: &[&str] = &["clipContent", "flowHome", "viewport", "autoLayout", "layouts", "gridsId", "rulers", "layers"];
const COMPONENT_KEYS: &[&str] = &["componentId", "predefined", "componentType", "figmaId", "properties", "states"];
const PATH_KEYS: &[&str] = &["startMarker", "endMarker", "miterLimit", "edited", "open", "points", "pathData"];
const SHAPE_EXTRA: &[&str] = &["angle", "ratio", "startAngle", "rays", "equilateral"];
const GROUP_KEYS: &[&str] = &["skipSelect", "layers", "skipConstraints", "startMarker", "endMarker", "miterLimit"];
const INSTANCE_KEYS: &[&str] = &["scale", "componentId", "autoLayout", "overrides", "settings"];
const SLICE_KEYS: &[&str] = &["hasBackground", "background", "backgroundId"];
const TEXT_KEYS: &[&str] = &[
    "text",
    "font",
    "fontSize",
    "paragraphSpacing",
    "kerning",
    "lineHeight",
    "casing",
    "align",
    "valign",
    "list",
    "variation",
    "fontOpticalSizing",
    "rtl",
    "underline",
    "strikethrough",
    "baselinePos",
    "inlines",
    "behavior",
    "clipContent",
    "truncate",
    "maxLines",
    "drawOnPath",
    "warpGlyphs",
    "verticalTrim",
    "textStyleId",
];

/// The fields a layer of type `t` may have besides [`LAYER_KEYS`].
fn type_keys(t: &str) -> &'static [&'static [&'static str]] {
    match t {
        "FRAME" | "SLOT" => &[FRAME_KEYS],
        "COMPONENT" => &[FRAME_KEYS, COMPONENT_KEYS],
        "STATES" => &[FRAME_KEYS, &["properties"]],
        "PATH" | "LINE" | "RECT" | "OVAL" | "POLYGON" | "STAR" | "TRIANGLE" => &[PATH_KEYS, SHAPE_EXTRA],
        "GROUP" | "SECTION" | "SHAPE" => &[GROUP_KEYS],
        "INSTANCE" => &[INSTANCE_KEYS],
        "SLICE" => &[SLICE_KEYS],
        "TEXT" => &[TEXT_KEYS],
        _ => &[],
    }
}

/// The label a layer of type `t` gets when it has no name.
fn label(t: &str) -> &'static str {
    match t {
        "FRAME" => "Frame",
        "COMPONENT" => "Component",
        "STATES" => "States",
        "SLOT" => "Slot",
        "GROUP" => "Group",
        "SECTION" => "Section",
        "SHAPE" => "Shape",
        "RECT" => "Rectangle",
        "OVAL" => "Oval",
        "POLYGON" => "Polygon",
        "STAR" => "Star",
        "TRIANGLE" => "Triangle",
        "PATH" => "Path",
        "LINE" => "Line",
        "TEXT" => "Text",
        "INSTANCE" => "Instance",
        "SLICE" => "Slice",
        "CONNECTOR" => "Connector",
        _ => "Layer",
    }
}

/// A `BoolOp` (-1 None … 3 Difference) for an operand of a shape.
fn bool_op(v: &Value) -> Option<BoolOp> {
    match int(v, "boolOp").unwrap_or(0) {
        0 => Some(BoolOp::Union),
        1 => Some(BoolOp::Subtract),
        2 => Some(BoolOp::Intersect),
        3 => Some(BoolOp::Exclude),
        _ => None,
    }
}

/// Where a layer is being imported.
#[derive(Clone, Copy)]
pub(super) struct Scope {
    /// The parent is an auto layout frame or instance.
    pub(super) auto_layout: bool,
    /// The layer is a path of a `SHAPE` (a boolean operand).
    pub(super) operand: bool,
}

impl Scope {
    pub(super) const TOP: Scope = Scope { auto_layout: false, operand: false };
}

impl Ctx<'_> {
    /// Import layer `j` (and its subtree) under `parent`. `path`: the parent's layer path.
    pub(super) fn layer(&mut self, j: &Value, parent: NodeId, path: &str, depth: usize, scope: Scope) {
        let t = text(j, "_t").unwrap_or("");
        let name = text(j, "name").filter(|n| !n.trim().is_empty()).map_or(label(t), clip).to_string();
        let lpath = format!("{path} / {name}");
        if !j.is_object() {
            self.warn(Severity::Dropped, path, "A layer that isn't a JSON object was not imported.");
            return;
        }
        if depth > MAX_DEPTH {
            self.warn(Severity::Dropped, &lpath, format!("Layers nested deeper than {MAX_DEPTH} levels were not imported."));
            return;
        }
        if self.count >= MAX_NODES {
            self.warn(Severity::Dropped, "", format!("The file has more than {MAX_NODES} layers; the rest were not imported."));
            return;
        }
        let kind = match t {
            "FRAME" | "STATES" | "SLOT" | "INSTANCE" => NodeKind::Frame,
            "COMPONENT" => NodeKind::Component { description: String::new(), props: Default::default() },
            "GROUP" => NodeKind::Group,
            "SECTION" => NodeKind::Section { contents_hidden: false },
            "SHAPE" => NodeKind::BooleanOp { op: BoolOp::Union },
            "RECT" | "OVAL" | "POLYGON" | "STAR" | "TRIANGLE" | "PATH" | "LINE" => NodeKind::Rectangle,
            "TEXT" => NodeKind::Text(Default::default()),
            "SLICE" => NodeKind::Slice,
            "CONNECTOR" => {
                self.warn(Severity::Dropped, &lpath, "Connectors (annotation lines between layers) were not imported.");
                return;
            }
            "" => {
                self.warn(Severity::Dropped, &lpath, "A layer without a `_t` type was not imported.");
                return;
            }
            other => {
                self.warn(Severity::Dropped, &lpath, format!("Layers of type “{}” are not supported and were not imported.", clip(other)));
                return;
            }
        };
        let (transform, w, h) = self.placement(j, &lpath);
        let id = self.doc.alloc_id();
        let mut n = Node::new(id, kind, name, w, h);
        n.transform = transform;
        n.fills.clear();
        n.strokes.clear();
        n.corners = Default::default();
        n.clip = false;
        if t == "TEXT" {
            self.text(&mut n, j, &lpath);
        }
        self.common(&mut n, j, &lpath);
        if matches!(t, "FRAME" | "COMPONENT" | "STATES" | "SLOT") {
            self.frame_props(&mut n, j, t, &lpath);
        }
        if t == "INSTANCE" {
            n.layout = j.get("autoLayout").filter(|a| a.is_object()).map(|a| self.auto_layout(a, &lpath));
        }
        if matches!(t, "RECT" | "OVAL" | "POLYGON" | "STAR" | "TRIANGLE" | "PATH" | "LINE") {
            self.shape(&mut n, t, j, &lpath);
        }
        self.child_layout(&mut n, j, scope.auto_layout, &lpath);
        if scope.operand {
            n.bool_op = bool_op(j);
        }
        if t == "SLICE" && flag(j, "hasBackground") == Some(true) {
            self.warn(Severity::Dropped, &lpath, "The slice's export background was not imported.");
        }
        self.unknown_keys(j, t, &lpath);
        let container = n.kind.is_container();
        let follows = n.kind.bounds_follow_children();
        let auto_layout = n.layout.is_some();
        if self.doc.insert(n, parent, None).is_err() {
            return;
        }
        self.count += 1;
        if let Some(fid) = text(j, "id") {
            self.free_ids.insert(id, clip(fid).to_string());
        }
        match t {
            "COMPONENT" => self.component(id, j, &lpath),
            "INSTANCE" => self.pending(id, j, &lpath),
            _ => {}
        }
        let kids = json::layers(j);
        if kids.is_empty() {
            return;
        }
        if !container || t == "INSTANCE" {
            self.warn(Severity::Dropped, &lpath, format!("The layers inside this {} were not imported.", label(t).to_lowercase()));
            return;
        }
        let inner = Scope { auto_layout, operand: t == "SHAPE" };
        for c in kids {
            self.layer(c, id, &lpath, depth + 1, inner);
        }
        if follows {
            let _ = self.doc.fit_to_children(id);
        }
    }

    /// The layer's transform in its parent and its size. `frame` overrides `pos`, `transform`
    /// and `size`; `pos` overrides `transform`; the README's defaults are `[0,0]` and `[100,100]`.
    fn placement(&mut self, j: &Value, path: &str) -> (Affine, f64, f64) {
        let matrix = json::matrix(j, "transform");
        if j.get("transform").is_some() && matrix.is_none() {
            self.warn(Severity::Approximated, path, "The layer's transform couldn't be read; it was placed at its position without rotation.");
        }
        let (w, h) = json::size(j, "size").unwrap_or((100.0, 100.0));
        let rotated = matrix.is_some_and(|m| {
            let [a, b, c, d, _, _] = m.as_coeffs();
            a != 1.0 || b != 0.0 || c != 0.0 || d != 1.0
        });
        if let Some((x, y, w, h)) = json::rect(j, "frame") {
            if rotated {
                self.warn(Severity::Approximated, path, "The layer's rotation or flip was not applied: its `frame` overrides its transform.");
            }
            return (Affine::translate((x, y)), w, h);
        }
        if let Some((x, y)) = json::point(j, "pos") {
            if rotated {
                self.warn(Severity::Approximated, path, "The layer's rotation or flip was not applied: its `pos` overrides its transform.");
            }
            return (Affine::translate((x, y)), w, h);
        }
        (matrix.unwrap_or(Affine::IDENTITY), w, h)
    }

    /// Properties every layer type has.
    fn common(&mut self, n: &mut Node, j: &Value, path: &str) {
        n.visible = flag(j, "hidden") != Some(true);
        n.locked = flag(j, "locked") == Some(true);
        n.constrain_proportions = flag(j, "lockAspect") == Some(true);
        self.style(n, j, path);
        if flag(j, "mask") == Some(true) {
            n.is_mask = true;
            n.mask_type = match int(j, "maskType").unwrap_or(0) {
                1 => MaskType::Alpha,
                2 => MaskType::Luminance,
                _ => MaskType::Vector,
            };
        }
        if flag(j, "breakMask") == Some(true) {
            self.warn(Severity::Approximated, path, "The layer ignores the mask below it in the file; here the mask still masks it.");
        }
        n.constraints = self.constraints(j, path);
        n.exports = self.exports(j, path);
        if flag(j, "trim") == Some(true) && !n.exports.is_empty() {
            self.warn(Severity::Dropped, path, "Trimming transparent pixels on export was not imported.");
        }
        if flag(j, "isTemplate") == Some(true) {
            self.warn(Severity::Dropped, path, "The layer's template mark was not imported.");
        }
        let proto = [
            (!list(j, "flows").is_empty(), "The layer's prototype interactions were not imported."),
            (flag(j, "keepScroll") == Some(true), "The prototype's keep-scroll-position setting was not imported."),
            (int(j, "scrollBehavior").is_some_and(|b| b != 0), "The layer's prototype scroll behaviour (fixed or sticky) was not imported."),
            (int(j, "scrollOverflow").is_some_and(|o| o != 0), "The layer's prototype scroll direction was not imported."),
        ];
        for (on, message) in proto {
            if on {
                self.warn(Severity::Dropped, path, message);
            }
        }
        let binds = list(j, "binds").len();
        if binds > 0 {
            let message = if binds == 1 {
                "1 variable binding on the layer was not imported; the layer keeps its current value.".to_string()
            } else {
                format!("{binds} variable bindings on the layer were not imported; the layer keeps their current values.")
            };
            self.warn(Severity::Dropped, path, message);
        }
        if !list(j, "themes").is_empty() {
            self.warn(Severity::Dropped, path, "The layer's variable theme (mode) selection was not imported.");
        }
        if j.get("tint").is_some() {
            self.warn(Severity::Dropped, path, "The layer's tint colour was not imported.");
        }
        if j.get("imageFilters").and_then(|f| flag(f, "enabled")) == Some(true) {
            self.warn(Severity::Dropped, path, "Image filters (brightness, contrast, hue, saturation) were not imported.");
        }
        if let Some(custom) = j.get("custom") {
            match custom.as_object() {
                Some(map) => {
                    for (k, v) in map.iter().take(256) {
                        match v.as_str() {
                            Some(s) => {
                                n.plugin_data.insert(clip(k).to_string(), s.chars().take(1 << 16).collect());
                            }
                            None => self.warn(Severity::Dropped, path, "A custom property whose value isn't text was not imported."),
                        }
                    }
                }
                None => self.warn(Severity::Dropped, path, "The layer's custom properties couldn't be read and were not imported."),
            }
        }
        let gallery = ["photoId", "illustrationId", "iconId", "avatarId", "data", "upscaled", "backgroundRemove"];
        if gallery.iter().any(|k| j.get(*k).is_some()) {
            self.warn(Severity::Info, path, "Lunacy's own notes on the layer (gallery source, generated text, upscaling) were not kept.");
        }
    }

    /// `constraints`: a string of flags (L left, T top, R right, B bottom, W fixed width, H
    /// fixed height); the README's default is `LTWH`.
    fn constraints(&mut self, j: &Value, path: &str) -> Constraints {
        let Some(s) = j.get("constraints") else { return Constraints::default() };
        let Some(s) = s.as_str().filter(|s| s.len() <= 16 && s.chars().all(|c| "LTRBWH".contains(c))) else {
            self.warn(Severity::Dropped, path, "The layer's resizing constraints couldn't be read and were not imported.");
            return Constraints::default();
        };
        let has = |c: char| s.contains(c);
        let horizontal = match (has('L'), has('R')) {
            (true, true) => HConstraint::LeftRight,
            (false, true) => HConstraint::Right,
            (true, false) => HConstraint::Left,
            (false, false) => {
                self.warn(
                    Severity::Approximated,
                    path,
                    "A layer pinned to neither the left nor the right was pinned left: the README doesn't say how it resizes.",
                );
                HConstraint::Left
            }
        };
        let vertical = match (has('T'), has('B')) {
            (true, true) => VConstraint::TopBottom,
            (false, true) => VConstraint::Bottom,
            (true, false) => VConstraint::Top,
            (false, false) => {
                self.warn(
                    Severity::Approximated,
                    path,
                    "A layer pinned to neither the top nor the bottom was pinned to the top: the README doesn't say how it resizes.",
                );
                VConstraint::Top
            }
        };
        Constraints { horizontal, vertical }
    }

    /// What frames, components, states and slots have besides the common properties.
    fn frame_props(&mut self, n: &mut Node, j: &Value, t: &str, path: &str) {
        n.clip = flag(j, "clipContent") != Some(false);
        n.layout = j.get("autoLayout").filter(|a| a.is_object()).map(|a| self.auto_layout(a, path));
        if j.get("autoLayout").is_some_and(|a| !a.is_object()) {
            self.warn(Severity::Dropped, path, "The frame's auto layout couldn't be read and was not imported.");
        }
        n.guides = guides(j.get("rulers"));
        if !list(j, "layouts").is_empty() || j.get("gridsId").is_some() {
            self.warn(Severity::Dropped, path, "The frame's layout grids (columns, rows, square grid) were not imported.");
        }
        if flag(j, "flowHome") == Some(true) {
            self.warn(Severity::Dropped, path, "The prototype starting point setting was not imported.");
        }
        if j.get("viewport").is_some() {
            self.warn(Severity::Dropped, path, "The frame's prototype viewport (device preset) was not imported.");
        }
        match t {
            "STATES" => {
                self.warn(Severity::Approximated, path, "The component states (variants) were imported as a frame holding their components.");
                if !list(j, "properties").is_empty() {
                    self.warn(Severity::Dropped, path, "The states' component properties were not imported.");
                }
            }
            "SLOT" => self.warn(Severity::Approximated, path, "The component slot was imported as a frame."),
            _ => {}
        }
    }

    /// Fields `j` has that the README doesn't list for its type: reported, not imported.
    fn unknown_keys(&mut self, j: &Value, t: &str, path: &str) {
        let Some(map) = j.as_object() else { return };
        let extra = type_keys(t);
        let unknown: Vec<String> = map
            .keys()
            .filter(|k| !LAYER_KEYS.contains(&k.as_str()) && !extra.iter().any(|keys| keys.contains(&k.as_str())))
            .take(9)
            .map(|k| format!("`{}`", clip(k)))
            .collect();
        if unknown.is_empty() {
            return;
        }
        let shown = unknown.iter().take(8).cloned().collect::<Vec<_>>().join(", ");
        let more = if unknown.len() > 8 { " and more" } else { "" };
        self.warn(Severity::Dropped, path, format!("Fields the FREE spec doesn't list for this layer type were not imported: {shown}{more}."));
    }
}

/// `Rulers`: `guides` are x positions of vertical guides, `vGuides` y positions of horizontal ones.
pub(super) fn guides(r: Option<&Value>) -> Vec<Guide> {
    let Some(r) = r else { return Vec::new() };
    let read = |k: &str, vertical: bool| {
        list(r, k).iter().take(1000).filter_map(|g| g.as_f64().filter(|x| x.is_finite())).map(move |o| Guide { vertical, offset: o.clamp(-1e7, 1e7) })
    };
    read("guides", true).chain(read("vGuides", false)).collect()
}

/// The number at `k`, when it is above zero.
pub(super) fn positive(v: &Value, k: &str) -> Option<f64> {
    num(v, k).filter(|x| *x > 0.0)
}
