//! A shape's look: fills (the documented colour object: a hex `color` with `opacity`, or a
//! `gradient`), `opacity`, `hidden`, `blocked` and `blendMode`; and the warnings for what the
//! documentation names without describing (strokes, shadows, blur, layout, grids…).

use figcraft_doc::Node;
use figcraft_doc::color::{BlendMode, ColorStop, Gradient, GradientKind, Paint};
use serde_json::Value;

use super::Ctx;
use super::read::{clip, flag, hex, list, num, opacity, present, text};
use crate::import_report::Severity;

/// Most fills or gradient stops read from one list.
const MAX_PAINTS: usize = 64;

/// The shape attributes the documentation lists (base, geometry, optional and per type). Any
/// other key is reported once for the file.
const DOCUMENTED: [&str; 44] = [
    "id",
    "name",
    "type",
    "selrect",
    "points",
    "transform",
    "transformInverse",
    "parentId",
    "frameId",
    "x",
    "y",
    "width",
    "height",
    "fills",
    "strokes",
    "opacity",
    "blendMode",
    "shadow",
    "blur",
    "constraintsH",
    "constraintsV",
    "r1",
    "r2",
    "r3",
    "r4",
    "blocked",
    "hidden",
    "collapsed",
    "componentId",
    "componentFile",
    "shapeRef",
    "touched",
    "interactions",
    "exports",
    "grids",
    "appliedTokens",
    "pluginData",
    "shapes",
    "showContent",
    "hideInViewer",
    "boolType",
    "content",
    "positionData",
    "metadata",
];

/// Attributes the documentation names whose inner structure it doesn't describe: (key, what the
/// warning says was left out).
const UNDESCRIBED: [(&str, &str); 8] = [
    ("strokes", "Strokes weren't imported: the documentation doesn't describe a stroke's structure."),
    ("shadow", "Shadows weren't imported: the documentation doesn't describe a shadow's structure."),
    ("blur", "The blur wasn't imported: the documentation doesn't describe a blur's structure."),
    ("interactions", "Prototype interactions weren't imported: the documentation doesn't describe their structure."),
    ("exports", "Export settings weren't imported: the documentation doesn't describe their structure."),
    ("grids", "Layout grids weren't imported: the documentation doesn't describe their structure."),
    ("appliedTokens", "The design tokens applied to the layer weren't imported; it keeps the values it shows."),
    ("pluginData", "Plugin data wasn't imported."),
];

impl Ctx<'_> {
    /// `opacity`, `hidden`, `blocked` and `blendMode` of shape `v` onto `n`.
    pub(super) fn common(&mut self, n: &mut Node, v: &Value, path: &str) {
        n.visible = flag(v, "hidden") != Some(true);
        n.locked = flag(v, "blocked") == Some(true);
        n.opacity = opacity(v, "opacity") as f32;
        let Some(mode) = text(v, "blendMode") else { return };
        // Reading: a blend mode is named by its CSS keyword (`multiply`, `color-burn`…). A group
        // or board set to normal is read as letting its layers blend with what is behind it.
        match BlendMode::from_name(mode) {
            Some(BlendMode::Normal | BlendMode::PassThrough) if n.kind.is_container() => n.blend = BlendMode::PassThrough,
            Some(BlendMode::PassThrough) => n.blend = BlendMode::Normal,
            Some(m) => n.blend = m,
            None => self.warn(Severity::Approximated, path, format!("The blend mode “{}” isn't one Scrim has; Normal was used.", clip(mode))),
        }
    }

    /// The paints of the fill list at `key` of `v`, for a layer of `w` × `h`. Reading: the list is
    /// in paint order, the first fill painted first (bottom).
    pub(super) fn fills(&mut self, v: &Value, key: &str, w: f64, h: f64, path: &str) -> Vec<Paint> {
        let fills = list(v, key);
        if fills.len() > MAX_PAINTS {
            self.warn(Severity::Dropped, path, format!("Only the first {MAX_PAINTS} fills were imported."));
        }
        if v.get(key).is_some_and(|f| !f.is_array() && !f.is_null()) {
            self.warn(Severity::Dropped, path, "The fills aren't a list and were not imported.");
        }
        let out: Vec<Paint> = fills.iter().take(MAX_PAINTS).filter_map(|f| self.color_paint(f, w, h, path, "fill")).collect();
        if out.len() > 1 {
            self.warn(Severity::Info, path, "The documentation doesn't give the order of fills; the first one listed was read as the bottom one.");
        }
        out
    }

    /// A colour object (`color` + `opacity`, `gradient`, or `image`) as a paint for a `w` × `h`
    /// box; None, with a warning saying why, when it can't be read. `what`: "fill" or "colour".
    pub(super) fn color_paint(&mut self, f: &Value, w: f64, h: f64, path: &str, what: &str) -> Option<Paint> {
        let alpha = opacity(f, "opacity");
        if let Some(c) = f.get("color") {
            let Some(c) = c.as_str().and_then(|s| hex(s, 1.0)) else {
                self.warn(Severity::Dropped, path, format!("A {what} whose colour isn't a hex colour was not imported."));
                return None;
            };
            let mut p = Paint::solid(c);
            p.opacity *= alpha as f32;
            return Some(p);
        }
        if let Some(g) = f.get("gradient").filter(|g| g.is_object()) {
            let mut p = Paint::gradient(self.gradient(g, w, h, path));
            p.opacity = alpha as f32;
            return Some(p);
        }
        if present(f, "image") {
            let m = format!("An image {what} wasn't imported: the documentation doesn't describe an image fill's structure.");
            self.warn(Severity::Dropped, path, m);
            return None;
        }
        let keys = f.as_object().map(|o| o.keys().take(6).map(|k| clip(k).to_string()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
        let m = if keys.is_empty() {
            format!("A {what} the documentation doesn't describe was not imported.")
        } else {
            format!("A {what} the documentation doesn't describe (with {keys}) was not imported.")
        };
        self.warn(Severity::Dropped, path, m);
        None
    }

    /// A documented `gradient` {type, startX, startY, endX, endY, stops [{color, offset,
    /// opacity}]}. Reading: the start and end points are in the layer's unit box (the example's
    /// 0 and 1); only `linear` is shown, so other types are imported as linear.
    fn gradient(&mut self, g: &Value, w: f64, h: f64, path: &str) -> Gradient {
        let mut out = Gradient::default_for(GradientKind::Linear, figcraft_doc::color::Color::BLACK);
        match text(g, "type") {
            Some("linear") => {}
            Some(other) => self.warn(
                Severity::Approximated,
                path,
                format!("The “{}” gradient was imported as a linear gradient: the documentation only shows linear ones.", clip(other)),
            ),
            None => self.warn(Severity::Approximated, path, "A gradient without a type was imported as a linear gradient."),
        }
        let at = |k: &str| num(g, k).map(|x| x.clamp(-1e3, 1e3));
        match (at("startX"), at("startY"), at("endX"), at("endY")) {
            (Some(x0), Some(y0), Some(x1), Some(y1)) => {
                out.from = (x0, y0);
                out.to = (x1, y1);
            }
            _ => self.warn(Severity::Approximated, path, "A gradient without start and end points uses the default direction."),
        }
        out.square_linear_width(w.max(1e-6), h.max(1e-6));
        let stops: Vec<ColorStop> = list(g, "stops")
            .iter()
            .take(MAX_PAINTS)
            .filter_map(|s| {
                let color = hex(text(s, "color")?, opacity(s, "opacity"))?;
                Some(ColorStop { position: num(s, "offset")?.clamp(0.0, 1.0) as f32, color })
            })
            .collect();
        if stops.len() < list(g, "stops").len().min(MAX_PAINTS) {
            self.warn(Severity::Dropped, path, "Gradient stops without a hex colour or an offset were left out.");
        }
        if stops.is_empty() {
            self.warn(Severity::Approximated, path, "A gradient without colour stops uses black to transparent.");
        } else {
            out.stops = stops;
        }
        out
    }

    /// Warnings for what shape `v` carries that isn't imported.
    pub(super) fn unsupported(&mut self, v: &Value, path: &str) {
        for (key, message) in UNDESCRIBED {
            if present(v, key) {
                self.warn(Severity::Dropped, path, message);
            }
        }
        if present(v, "constraintsH") || present(v, "constraintsV") {
            self.constraints.push(path.to_string());
        }
        if flag(v, "hideInViewer") == Some(true) {
            self.warn(Severity::Dropped, path, "Hiding the board in the viewer wasn't imported.");
        }
        let Some(o) = v.as_object() else { return };
        if o.keys().any(|k| k.starts_with("layout")) {
            self.warn(
                Severity::Dropped,
                path,
                "Layout settings weren't imported: the documentation doesn't describe them. The layers keep the positions they were saved at.",
            );
        }
        for k in o.keys().filter(|k| !k.starts_with("layout") && !DOCUMENTED.contains(&k.as_str())) {
            if self.unknown_keys.len() < 1_000 || self.unknown_keys.contains_key(k.as_str()) {
                *self.unknown_keys.entry(clip(k).to_string()).or_insert(0) += 1;
            }
        }
    }
}
