//! The earlier TypeScript Scrim's `.scrim` files (Fix AG2): one JSON object, `{schemaVersion,
//! id, name, rootIds, nodes, components, tokens}`, with a flat node map whose `layout` and `style`
//! are CSS properties (`docs/design/scrim-format.md` › Findings 3). They open as a new document
//! through the import registry.
//!
//! What maps: frames and boxes (sizes, flex layout as auto layout, absolute positions,
//! backgrounds, borders, corner radius, opacity, shadows, overflow), text (font, size, weight,
//! style, colour, line height, letter spacing, alignment, case, decoration) and the canvas
//! background. What doesn't is reported, never dropped in silence:
//! - images (their bytes were in the earlier Scrim's asset library, not in the file): a grey
//!   placeholder;
//! - instances: copies of their component, with the instance's overrides;
//! - grid layout, percentages and `calc()`, CSS the model has no place for, hover and other
//!   states, reduced-motion rules, inline SVG, links, tokens.

mod css;

use std::collections::BTreeSet;

use figcraft_doc::color::{Color, Paint};
use figcraft_doc::{
    AutoLayout, CrossAlign, Document, Effect, EffectKind, LayoutMode, MainAlign, NodeId, NodeKind, Sizing, StrokeAlign, TextAlignH, TextAutoResize,
    TextCase, TextData, TextDecoration,
};
use serde_json::{Map, Value};

use crate::FormatError;
use crate::import_report::ImportWarning;

/// The newest `schemaVersion` this reads.
pub const SCHEMA_VERSION: u64 = 1;
/// Most layers read from one file, and the deepest nesting followed.
const MAX_NODES: usize = 200_000;
const MAX_DEPTH: usize = 256;

/// Whether `bytes` are an earlier Scrim `.scrim` file (see [`crate::sniff::Kind::OldScrim`]).
pub fn is_old_scrim(bytes: &[u8]) -> bool {
    crate::sniff::kind(bytes) == crate::sniff::Kind::OldScrim
}

/// The earlier Scrim file `bytes` as a new document, with what couldn't be brought across. `name`:
/// the document name when the file has none. A `schemaVersion` newer than [`SCHEMA_VERSION`] is
/// [`FormatError::Newer`]; anything that isn't such a file is [`FormatError::Corrupt`].
pub fn import(bytes: &[u8], name: &str) -> Result<(Document, Vec<ImportWarning>), FormatError> {
    let root: Value = serde_json::from_slice(bytes).map_err(|e| FormatError::Corrupt(format!("not valid JSON: {e}")))?;
    let obj = root.as_object().ok_or_else(|| FormatError::Corrupt("not a JSON object".into()))?;
    let version = obj.get("schemaVersion").and_then(Value::as_u64).ok_or_else(|| FormatError::Corrupt("no schemaVersion".into()))?;
    if version > SCHEMA_VERSION {
        return Err(FormatError::Newer(u32::try_from(version).unwrap_or(u32::MAX)));
    }
    let nodes = obj.get("nodes").and_then(Value::as_object).ok_or_else(|| FormatError::Corrupt("no `nodes` object".into()))?;
    let roots = obj.get("rootIds").and_then(Value::as_array).ok_or_else(|| FormatError::Corrupt("no `rootIds` list".into()))?;
    let doc_name = obj.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(name);
    let mut doc = Document::new(doc_name);
    let page = doc.first_page().ok_or_else(|| FormatError::Corrupt("no page".into()))?;
    let page_name = doc.node(page).map(|n| n.name.clone()).unwrap_or_default();
    let mut cx = Cx { nodes, components: obj.get("components").and_then(Value::as_object), doc: &mut doc, warnings: Vec::new(), made: 0 };
    if let Some(bg) = obj.get("canvasBackground").and_then(css::color)
        && let Some(NodeKind::Page { background, .. }) = cx.doc.node_mut(page).map(|n| &mut n.kind)
    {
        *background = bg;
    }
    for id in roots.iter().filter_map(Value::as_str) {
        let at = cx.nodes.get(id).and_then(|n| n.get("canvas")).map(|c| (num(c.get("x")), num(c.get("y")))).unwrap_or((0.0, 0.0));
        cx.build(id, page, Parent::Page(at), &page_name, &mut BTreeSet::new())?;
    }
    for (key, what) in [("tokens", "design tokens"), ("lint", "lint settings")] {
        if obj.get(key).and_then(Value::as_object).is_some_and(|o| o.values().any(|v| !is_empty(v))) {
            cx.warnings.push(ImportWarning::dropped("", format!("{what} were left out")));
        }
    }
    let warnings = cx.warnings;
    Ok((doc, warnings))
}

fn num(v: Option<&Value>) -> f64 {
    v.and_then(css::px).unwrap_or(0.0)
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.values().all(is_empty),
        _ => false,
    }
}

/// Where a layer goes.
#[derive(Clone, Copy)]
enum Parent {
    /// On the page, at its canvas position.
    Page((f64, f64)),
    /// In an auto layout frame: its direction and whether it stretches children across.
    Flow { vertical: bool, stretch: bool },
    /// In a frame without auto layout; `y` is where the next layer in the flow goes.
    Free { y: f64 },
}

struct Cx<'a> {
    nodes: &'a Map<String, Value>,
    components: Option<&'a Map<String, Value>>,
    doc: &'a mut Document,
    warnings: Vec<ImportWarning>,
    made: usize,
}

/// The `layout` and `style` keys this importer reads; any other is reported.
const READ: &[&str] = &[
    "display",
    "flexDirection",
    "flexWrap",
    "gap",
    "rowGap",
    "columnGap",
    "padding",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "alignItems",
    "justifyContent",
    "flexGrow",
    "flexShrink",
    "position",
    "top",
    "left",
    "right",
    "bottom",
    "width",
    "height",
    "minWidth",
    "minHeight",
    "maxWidth",
    "maxHeight",
    "background",
    "backgroundColor",
    "color",
    "opacity",
    "borderRadius",
    "border",
    "borderWidth",
    "borderStyle",
    "borderColor",
    "boxShadow",
    "overflow",
    "fontFamily",
    "fontSize",
    "fontWeight",
    "fontStyle",
    "lineHeight",
    "letterSpacing",
    "textAlign",
    "textTransform",
    "textDecoration",
    "cursor",
];

impl Cx<'_> {
    fn build(&mut self, id: &str, parent: NodeId, at: Parent, path: &str, seen: &mut BTreeSet<String>) -> Result<Option<(NodeId, f64)>, FormatError> {
        let Some(n) = self.nodes.get(id).and_then(Value::as_object) else {
            self.warnings.push(ImportWarning::dropped(path, format!("a layer (`{}`) is listed but missing from the file", clip(id))));
            return Ok(None);
        };
        let name = n.get("name").and_then(Value::as_str).unwrap_or(id).to_string();
        let here = format!("{path} / {name}");
        self.made += 1;
        if self.made > MAX_NODES || seen.len() > MAX_DEPTH || !seen.insert(id.to_string()) {
            self.warnings.push(ImportWarning::dropped(&here, "left out: the file nests layers in a loop or too deeply"));
            return Ok(None);
        }
        let ty = n.get("type").and_then(Value::as_str).unwrap_or("");
        let made = match ty {
            "frame" | "box" | "image" | "text" => self.layer(n, ty, &name, parent, at, &here, seen),
            "instance" => self.instance(n, &name, parent, at, &here, seen),
            other => {
                self.warnings.push(ImportWarning::dropped(&here, format!("a `{}` layer was left out", clip(other))));
                Ok(None)
            }
        };
        seen.remove(id);
        made
    }

    /// An instance: a copy of its component's layers under the instance's name, with its overrides.
    fn instance(
        &mut self,
        n: &Map<String, Value>,
        name: &str,
        parent: NodeId,
        at: Parent,
        here: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<Option<(NodeId, f64)>, FormatError> {
        let comp = n.get("componentId").and_then(Value::as_str).and_then(|c| self.components?.get(c)).and_then(Value::as_object);
        let Some(root) = comp.and_then(|c| c.get("rootId")).and_then(Value::as_str) else {
            self.warnings.push(ImportWarning::dropped(here, "an instance whose component isn't in the file was left out"));
            return Ok(None);
        };
        let mut merged = self.nodes.get(root).and_then(Value::as_object).cloned().unwrap_or_default();
        merged.insert("name".into(), Value::String(name.to_string()));
        if let Some(Value::Object(over)) = n.get("overrides") {
            let style = merged.entry("style").or_insert_with(|| Value::Object(Map::new()));
            for (k, v) in over.iter().filter(|(k, _)| k.as_str() != "text") {
                if let Some(s) = style.as_object_mut() {
                    s.insert(k.clone(), v.clone());
                }
            }
            if let Some(t) = over.get("text") {
                merged.insert("text".into(), t.clone());
            }
        }
        for key in ["layout", "canvas"] {
            if let Some(v) = n.get(key) {
                merged.insert(key.into(), v.clone());
            }
        }
        let ty = merged.get("type").and_then(Value::as_str).unwrap_or("frame").to_string();
        let comp_name = comp.and_then(|c| c.get("name")).and_then(Value::as_str).unwrap_or("its component");
        self.warnings.push(ImportWarning::approximated(here, format!("an instance of “{}” was imported as a copy of it", clip(comp_name))));
        self.layer(&merged, &ty, name, parent, at, here, seen)
    }

    /// A frame, box, image or text layer and its children. Returns it and the height it takes in a
    /// flow (for the layer after it in a frame without auto layout).
    #[allow(clippy::too_many_arguments)]
    fn layer(
        &mut self,
        n: &Map<String, Value>,
        ty: &str,
        name: &str,
        parent: NodeId,
        at: Parent,
        here: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<Option<(NodeId, f64)>, FormatError> {
        let empty = Map::new();
        let layout = n.get("layout").and_then(Value::as_object).unwrap_or(&empty);
        let style = n.get("style").and_then(Value::as_object).unwrap_or(&empty);
        let get = |k: &str| layout.get(k).or_else(|| style.get(k));
        let kids: Vec<&str> = n.get("childIds").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let display = get("display").and_then(Value::as_str).unwrap_or("block");
        let (w, h) = (css::size(get("width")), css::size(get("height")));
        let kind = match ty {
            "text" => NodeKind::Text(self.text(n, style, here)),
            "box" if kids.is_empty() && display == "block" => NodeKind::Rectangle,
            "image" => NodeKind::Rectangle,
            _ => NodeKind::Frame,
        };
        let is_text = matches!(kind, NodeKind::Text(_));
        let fallback = if is_text { 1.0 } else { 100.0 };
        let px_or = |s: Size, d: f64| if let Size::Px(x) = s { x } else { d };
        let (width, height) = (px_or(w, fallback), px_or(h, fallback));
        let absolute = get("position").and_then(Value::as_str) == Some("absolute");
        let (x, y) = match at {
            Parent::Page(p) => p,
            Parent::Free { y } if !absolute => (0.0, y),
            Parent::Flow { .. } if !absolute => (0.0, 0.0),
            _ => self.offset(get("left"), get("top"), get("right"), get("bottom"), width, height, parent, here),
        };
        let node = self.doc.create(parent, kind, name, x, y, width, height).map_err(|e| FormatError::Corrupt(e.to_string()))?;
        let mut flow_height = height;
        // Its own look.
        let fills = self.fills(style, ty, is_text, here);
        let auto = (matches!(display, "flex" | "inline-flex") && !is_text).then(|| auto_layout(layout, style));
        let effects = self.effects(style, here);
        let stroke = self.stroke(style, here);
        let radius = style.get("borderRadius").and_then(|v| css::sides(v).or_else(|| self.unread(here, "borderRadius", v)));
        if let Some(nn) = self.doc.node_mut(node) {
            nn.fills = fills;
            nn.effects = effects;
            if let Some((weight, colour)) = stroke {
                nn.strokes = vec![Paint::solid(colour)];
                nn.stroke.weight = weight;
                nn.stroke.align = StrokeAlign::Inside;
            }
            if let Some([tl, tr, br, bl]) = radius {
                nn.corners.radii = [tl, tr, br, bl];
            }
            if let Some(o) = style.get("opacity").and_then(Value::as_f64) {
                nn.opacity = o.clamp(0.0, 1.0) as f32;
            }
            nn.clip = matches!(style.get("overflow").and_then(Value::as_str), Some("hidden" | "clip"));
            nn.visible = display != "none";
            // How it sizes and sits in its parent.
            let (main_vertical, stretch) = match at {
                Parent::Flow { vertical, stretch } => (Some(vertical), stretch),
                _ => (None, false),
            };
            let grow = get("flexGrow").and_then(Value::as_f64).is_some_and(|g| g > 0.0);
            let sizing = |s: Size, along_main: bool, across: bool| match s {
                Size::Px(_) if along_main && grow => Sizing::Fill,
                Size::Px(_) => Sizing::Fixed,
                Size::Fill => Sizing::Fill,
                Size::Hug | Size::Other if across && stretch => Sizing::Fill,
                Size::Hug | Size::Other if along_main && grow => Sizing::Fill,
                Size::Hug | Size::Other => Sizing::Hug,
            };
            let in_flow = main_vertical.is_some() && !absolute;
            let hugs = auto.is_some() || is_text;
            if in_flow || hugs {
                let vertical = main_vertical.unwrap_or(false);
                let sh = sizing(w, in_flow && !vertical, in_flow && vertical);
                let sv = sizing(h, in_flow && vertical, in_flow && !vertical);
                nn.child_layout.sizing_h = if !in_flow && sh == Sizing::Fill { Sizing::Fixed } else { sh };
                nn.child_layout.sizing_v = if !in_flow && sv == Sizing::Fill { Sizing::Fixed } else { sv };
                if !hugs {
                    for s in [&mut nn.child_layout.sizing_h, &mut nn.child_layout.sizing_v] {
                        if *s == Sizing::Hug {
                            *s = Sizing::Fixed;
                        }
                    }
                }
            }
            nn.child_layout.absolute = main_vertical.is_some() && absolute;
            nn.child_layout.min_w = get("minWidth").and_then(css::px);
            nn.child_layout.max_w = get("maxWidth").and_then(css::px);
            nn.child_layout.min_h = get("minHeight").and_then(css::px);
            nn.child_layout.max_h = get("maxHeight").and_then(css::px);
            if let NodeKind::Text(t) = &mut nn.kind {
                t.auto_resize = if matches!(w, Size::Px(_)) { TextAutoResize::Height } else { TextAutoResize::WidthAndHeight };
                // A first guess; the registry measures text and lays the page out after import.
                let lines = t.characters.lines().count().max(1) as f64;
                let longest = t.characters.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f64;
                if !matches!(w, Size::Px(_)) {
                    nn.width = (longest * t.style.font_size * 0.55).max(1.0);
                }
                nn.height = lines * t.style.font_size * 1.2;
                flow_height = nn.height;
            }
            nn.layout = auto.clone();
        }
        // What it couldn't take.
        for (what, v) in [("width", w), ("height", h)] {
            if v == Size::Other {
                self.warnings.push(ImportWarning::approximated(here, format!("its {what} (`{}`) was read as hugging its content", show(get(what)))));
            } else if v == Size::Hug && auto.is_none() && !is_text && ty != "text" && !matches!(at, Parent::Flow { .. }) {
                self.warnings.push(ImportWarning::approximated(here, format!("its {what} follows its content in CSS; it was given {fallback} px")));
            }
        }
        if display == "grid" {
            self.warnings.push(ImportWarning::approximated(here, "grid layout was left out: its layers keep their order without the grid"));
        }
        if ty == "image" {
            if let Some(f) = self.doc.node_mut(node) {
                f.fills = vec![Paint::solid(Color { r: 0.85, g: 0.85, b: 0.85, a: 1.0 })];
            }
            self.warnings.push(ImportWarning::dropped(
                here,
                "the image isn't in the file (the earlier Scrim kept images in its own library): a grey placeholder stands in",
            ));
        }
        for (key, what) in [
            ("states", "hover and other state styles"),
            ("media", "reduced-motion rules"),
            ("svg", "inline vector content"),
            ("guides", "layout guides"),
            ("href", "a link"),
            ("conditions", "states it was in"),
        ] {
            if n.get(key).is_some_and(|v| !is_empty(v)) {
                self.warnings.push(ImportWarning::dropped(here, format!("{what} were left out")));
            }
        }
        let mut unread: Vec<&str> = layout.keys().chain(style.keys()).map(String::as_str).filter(|k| !READ.contains(k)).collect();
        unread.sort_unstable();
        unread.dedup();
        if !unread.is_empty() {
            self.warnings.push(ImportWarning::dropped(here, format!("CSS left out: {}", unread.join(", "))));
        }
        // Its layers.
        let into = match &auto {
            Some(al) => {
                let stretch = matches!(get("alignItems").and_then(Value::as_str), None | Some("stretch"));
                Parent::Flow { vertical: al.mode == LayoutMode::Vertical, stretch }
            }
            None => Parent::Free { y: 0.0 },
        };
        let mut flowed = false;
        let mut y = 0.0;
        for kid in kids {
            let at = match into {
                Parent::Free { .. } => Parent::Free { y },
                other => other,
            };
            let kid_absolute =
                self.nodes.get(kid).and_then(|k| k.get("layout")).and_then(|l| l.get("position")).and_then(Value::as_str) == Some("absolute");
            if let Some((_, kid_h)) = self.build(kid, node, at, here, seen)?
                && matches!(into, Parent::Free { .. })
                && !kid_absolute
            {
                y += kid_h;
                flowed = true;
            }
        }
        if flowed && ty != "text" {
            self.warnings.push(ImportWarning::approximated(here, "layers in its normal flow were stacked from the top"));
        }
        Ok(Some((node, flow_height)))
    }

    /// Where an absolutely placed layer goes in its parent: from `left`/`top`, else from
    /// `right`/`bottom` and its size.
    #[allow(clippy::too_many_arguments)]
    fn offset(
        &mut self,
        left: Option<&Value>,
        top: Option<&Value>,
        right: Option<&Value>,
        bottom: Option<&Value>,
        w: f64,
        h: f64,
        parent: NodeId,
        here: &str,
    ) -> (f64, f64) {
        let (pw, ph) = self.doc.node(parent).map_or((0.0, 0.0), |p| (p.width, p.height));
        let mut side = |near: Option<&Value>, far: Option<&Value>, size: f64, room: f64, axis: &str| -> f64 {
            if let Some(v) = near.and_then(css::px) {
                return v;
            }
            if let Some(v) = far.and_then(css::px) {
                return room - v - size;
            }
            if near.or(far).is_some() {
                self.warnings.push(ImportWarning::approximated(here, format!("its {axis} position isn't in pixels; it was placed at 0")));
            }
            0.0
        };
        let x = side(left, right, w, pw, "horizontal");
        let y = side(top, bottom, h, ph, "vertical");
        (x, y)
    }

    fn fills(&mut self, style: &Map<String, Value>, ty: &str, is_text: bool, here: &str) -> Vec<Paint> {
        if is_text {
            return vec![Paint::solid(style.get("color").and_then(css::color).unwrap_or(Color::BLACK))];
        }
        let bg = style.get("backgroundColor").or_else(|| style.get("background"));
        if let Some(v) =
            style.get("backgroundImage").or_else(|| bg.filter(|v| v.as_str().is_some_and(|s| s.contains("gradient") || s.contains("url("))))
        {
            self.warnings.push(ImportWarning::dropped(here, format!("its background (`{}`) was left out", show(Some(v)))));
        }
        match bg.and_then(css::color) {
            Some(c) if c.a > 0.0 => vec![Paint::solid(c)],
            _ if ty == "box" && bg.is_none() => Vec::new(),
            _ => Vec::new(),
        }
    }

    fn stroke(&mut self, style: &Map<String, Value>, here: &str) -> Option<(f64, Color)> {
        let from_parts = || {
            let w = style.get("borderWidth").and_then(css::px)?;
            let c = style.get("borderColor").and_then(css::color).unwrap_or(Color::BLACK);
            Some((w, c, style.get("borderStyle").and_then(Value::as_str).is_none_or(|s| s == "solid")))
        };
        let (w, c, solid) = match style.get("border") {
            Some(v) => css::border(v)?,
            None => from_parts()?,
        };
        if !solid {
            self.warnings.push(ImportWarning::approximated(here, "a dashed or dotted border was imported as a solid one"));
        }
        (w > 0.0).then_some((w, c))
    }

    fn effects(&mut self, style: &Map<String, Value>, here: &str) -> Vec<Effect> {
        let Some(v) = style.get("boxShadow") else { return Vec::new() };
        match css::shadows(v) {
            Some(list) => list
                .into_iter()
                .map(|(s, inset)| Effect { kind: if inset { EffectKind::InnerShadow(s) } else { EffectKind::DropShadow(s) }, visible: true })
                .collect(),
            None => {
                self.unread::<()>(here, "boxShadow", v);
                Vec::new()
            }
        }
    }

    fn text(&mut self, n: &Map<String, Value>, style: &Map<String, Value>, here: &str) -> TextData {
        let mut t = TextData::new(n.get("text").and_then(Value::as_str).unwrap_or_default());
        if let Some(f) = style.get("fontFamily").and_then(css::font_family) {
            t.style.font_family = f;
        }
        if let Some(s) = style.get("fontSize").and_then(css::px) {
            t.style.font_size = s.max(1.0);
        }
        let italic = style.get("fontStyle").and_then(Value::as_str) == Some("italic");
        t.style.font_style = css::font_style(style.get("fontWeight"), italic);
        if let Some(v) = style.get("lineHeight") {
            match css::line_height(v) {
                Some(lh) => t.style.line_height = lh,
                None => {
                    self.unread::<()>(here, "lineHeight", v);
                }
            }
        }
        if let Some(ls) = style.get("letterSpacing").and_then(css::letter_spacing) {
            t.style.letter_spacing = ls;
        }
        t.align_h = match style.get("textAlign").and_then(Value::as_str) {
            Some("center") => TextAlignH::Center,
            Some("right" | "end") => TextAlignH::Right,
            Some("justify") => TextAlignH::Justified,
            _ => TextAlignH::Left,
        };
        t.style.case = match style.get("textTransform").and_then(Value::as_str) {
            Some("uppercase") => TextCase::Upper,
            Some("lowercase") => TextCase::Lower,
            Some("capitalize") => TextCase::Title,
            _ => TextCase::Original,
        };
        let deco = style.get("textDecoration").and_then(Value::as_str).unwrap_or("");
        t.style.decoration = if deco.contains("underline") {
            TextDecoration::Underline
        } else if deco.contains("line-through") {
            TextDecoration::Strikethrough
        } else {
            TextDecoration::None
        };
        t
    }

    /// Report CSS property `key` whose value couldn't be read.
    fn unread<T>(&mut self, here: &str, key: &str, v: &Value) -> Option<T> {
        self.warnings.push(ImportWarning::dropped(here, format!("`{key}: {}` couldn't be read and was left out", show(Some(v)))));
        None
    }
}

/// Flex layout (`display: flex`) as auto layout.
fn auto_layout(layout: &Map<String, Value>, style: &Map<String, Value>) -> AutoLayout {
    let get = |k: &str| layout.get(k).or_else(|| style.get(k));
    let vertical = matches!(get("flexDirection").and_then(Value::as_str), Some("column" | "column-reverse"));
    let gap = get("gap").and_then(css::px).unwrap_or(0.0);
    let main_gap = get(if vertical { "rowGap" } else { "columnGap" }).and_then(css::px).unwrap_or(gap);
    let cross_gap = get(if vertical { "columnGap" } else { "rowGap" }).and_then(css::px).unwrap_or(gap);
    let mut padding = get("padding").and_then(css::sides).unwrap_or([0.0; 4]);
    for (i, k) in ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"].into_iter().enumerate() {
        if let (Some(v), Some(p)) = (get(k).and_then(css::px), padding.get_mut(i)) {
            *p = v;
        }
    }
    AutoLayout {
        mode: if vertical { LayoutMode::Vertical } else { LayoutMode::Horizontal },
        wrap: matches!(get("flexWrap").and_then(Value::as_str), Some("wrap" | "wrap-reverse")),
        gap: main_gap,
        cross_gap,
        padding,
        main_align: match get("justifyContent").and_then(Value::as_str) {
            Some("center") => MainAlign::Center,
            Some("flex-end" | "end") => MainAlign::Max,
            Some("space-between") => MainAlign::SpaceBetween,
            Some("space-around") => MainAlign::SpaceAround,
            Some("space-evenly") => MainAlign::SpaceEvenly,
            _ => MainAlign::Min,
        },
        cross_align: match get("alignItems").and_then(Value::as_str) {
            Some("center") => CrossAlign::Center,
            Some("flex-end" | "end") => CrossAlign::Max,
            Some("baseline") => CrossAlign::Baseline,
            _ => CrossAlign::Min,
        },
        ..AutoLayout::default()
    }
}

use css::Size;

fn show(v: Option<&Value>) -> String {
    let s = match v {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    clip(&s)
}

fn clip(s: &str) -> String {
    let mut out: String = s.chars().take(80).collect();
    if s.chars().nth(80).is_some() {
        out.push('…');
    }
    out
}

#[cfg(test)]
#[path = "../old_scrim_tests.rs"]
mod tests;
