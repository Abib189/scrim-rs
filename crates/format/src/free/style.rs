//! A layer's style: fills, borders and their options, shadows, inner shadows, the blur (glass
//! included), opacity, blend mode and corner radii; and its export options.

use figcraft_doc::color::{BlendMode, Color, ColorStop, Gradient, GradientKind, ImagePaint, ImageScaleMode, Paint, PaintKind};
use figcraft_doc::{
    Blur, Effect, EffectKind, ExportFormat, ExportSetting, ExportSize, Glass, Node, NodeKind, Shadow, StrokeAlign, StrokeCap, StrokeJoin,
};
use serde_json::Value;

use super::Ctx;
use super::json::{self, MAX_SIZE, clip, flag, int, list, num, text};
use crate::import_report::Severity;

/// Most paints, borders or shadows read from one layer.
const MAX_PAINTS: usize = 256;
/// The corner smoothing "smooth corners" are imported with.
const SMOOTH: f64 = 0.6;

/// A `BlendMode` (0 Normal … 17 PlusLighter).
pub(super) fn blend(v: Option<i64>) -> BlendMode {
    match v {
        Some(1) => BlendMode::Darken,
        Some(2) => BlendMode::Multiply,
        Some(3) => BlendMode::ColorBurn,
        Some(4) => BlendMode::Lighten,
        Some(5) => BlendMode::Screen,
        Some(6) => BlendMode::ColorDodge,
        Some(7) => BlendMode::Overlay,
        Some(8) => BlendMode::SoftLight,
        Some(9) => BlendMode::HardLight,
        Some(10) => BlendMode::Difference,
        Some(11) => BlendMode::Exclusion,
        Some(12) => BlendMode::Hue,
        Some(13) => BlendMode::Saturation,
        Some(14) => BlendMode::Color,
        Some(15) => BlendMode::Luminosity,
        Some(16) => BlendMode::PlusDarker,
        Some(17) => BlendMode::PlusLighter,
        _ => BlendMode::Normal,
    }
}

/// A solid paint of `c` (its alpha as the paint's opacity, times `opacity`).
pub(super) fn solid(c: Color, opacity: f32) -> Paint {
    let mut p = Paint::solid(c.with_alpha(1.0));
    p.opacity = (c.a * opacity).clamp(0.0, 1.0);
    p
}

/// The handle perpendicular to `from` → `to`, as long (in the unit box).
fn width_handle(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    (from.0 - (to.1 - from.1), from.1 + (to.0 - from.0))
}

impl Ctx<'_> {
    /// Apply `j`'s style to `n`.
    pub(super) fn style(&mut self, n: &mut Node, j: &Value, path: &str) {
        let fills = self.paints(j, "fills", "fill", path);
        let strokes = self.paints(j, "borders", "border", path);
        // A shape's (boolean group's) fills and borders paint its combined outline.
        if matches!(n.kind, NodeKind::Group | NodeKind::Slice) {
            if fills.iter().chain(&strokes).any(|p| p.visible) {
                self.warn(Severity::Dropped, path, "Fills and borders on groups and slices were not imported.");
            }
        } else {
            n.fills = fills;
            n.strokes = strokes;
        }
        self.border_options(n, j, path);
        n.effects = self.effects(j, path);
        n.opacity = num(j, "opacity").map_or(1.0, |o| o.clamp(0.0, 1.0) as f32);
        let mode = blend(int(j, "blendMode"));
        if mode != BlendMode::Normal || !n.kind.is_container() {
            n.blend = mode;
        }
        self.corners(n, j, path);
        self.style_links(j, path);
    }

    /// The layer's fills or borders: the list at `many` (`fills`, `borders`), else the single
    /// colour at `one` (`fill`, `border`).
    pub(super) fn paints(&mut self, j: &Value, many: &str, one: &str, path: &str) -> Vec<Paint> {
        if let Some(l) = j.get(many) {
            if !l.is_array() {
                self.warn(Severity::Dropped, path, format!("The layer's {many} couldn't be read and were not imported."));
            }
            return list(j, many).iter().take(MAX_PAINTS).filter_map(|f| self.paint(f, one, path)).collect();
        }
        match j.get(one) {
            None => Vec::new(),
            Some(c) => match c.as_str().and_then(json::color_str) {
                Some(c) => vec![solid(c, 1.0)],
                None => {
                    self.warn(Severity::Dropped, path, format!("The layer's {one} colour couldn't be read and was not imported."));
                    Vec::new()
                }
            },
        }
    }

    /// One `Fill` (`what`: "fill" or "border", for warnings).
    fn paint(&mut self, f: &Value, what: &str, path: &str) -> Option<Paint> {
        if !f.is_object() {
            self.warn(Severity::Dropped, path, format!("A {what} that couldn't be read was not imported."));
            return None;
        }
        let visible = flag(f, "enabled") != Some(false);
        let opacity = num(f, "opacity").map_or(1.0, |o| o.clamp(0.0, 1.0) as f32);
        let blend = blend(int(f, "blendMode"));
        if f.get("colorId").is_some() {
            self.warn(Severity::Dropped, path, format!("A {what}'s colour variable was not imported; it keeps its colour."));
        }
        let kind = match int(f, "type").unwrap_or(0) {
            0 => {
                let c = match f.get("color") {
                    None => Color::from_rgba8(0x80, 0x80, 0x80, 0xFF),
                    Some(c) => c.as_str().and_then(json::color_str).unwrap_or_else(|| {
                        self.warn(Severity::Approximated, path, format!("A {what} colour couldn't be read; it is grey."));
                        Color::from_rgba8(0x80, 0x80, 0x80, 0xFF)
                    }),
                };
                let mut p = solid(c, opacity);
                p.visible = visible;
                p.blend = blend;
                return Some(p);
            }
            1 => PaintKind::Gradient(self.gradient(f.get("gradient").unwrap_or(&Value::Null), path)),
            4 if what == "fill" => PaintKind::Image(self.pattern(f.get("pattern").unwrap_or(&Value::Null), path)?),
            4 => {
                self.warn(Severity::Dropped, path, "An image border was not imported.");
                return None;
            }
            other => {
                self.warn(Severity::Dropped, path, format!("A {what} of type {other} was not imported."));
                return None;
            }
        };
        Some(Paint { kind, opacity, visible, blend })
    }

    /// A `Gradient`: the points are in the layer's unit box (the README's defaults run from
    /// `[0.5,0]` to `[0.5,1]`); `side` is the width handle of radial, angular and diamond ones.
    fn gradient(&mut self, g: &Value, path: &str) -> Gradient {
        let kind = match int(g, "type").unwrap_or(0) {
            1 => GradientKind::Radial,
            2 => GradientKind::Angular,
            3 => GradientKind::Diamond,
            _ => GradientKind::Linear,
        };
        let mut out = Gradient::default_for(kind, Color::BLACK);
        let unit = |p: (f64, f64)| (p.0.clamp(-1e3, 1e3), p.1.clamp(-1e3, 1e3));
        out.from = json::point(g, "from").map_or((0.5, 0.0), unit);
        out.to = json::point(g, "to").map_or((0.5, 1.0), unit);
        out.width = match json::point(g, "side") {
            Some(s) if kind != GradientKind::Linear => unit(s),
            _ => width_handle(out.from, out.to),
        };
        let mut stops = Vec::new();
        for s in list(g, "stops").iter().take(MAX_PAINTS) {
            let position = num(s, "pos").unwrap_or(0.0).clamp(0.0, 1.0) as f32;
            let color = json::color(s, "color").unwrap_or(Color::TRANSPARENT);
            if s.get("colorId").is_some() {
                self.warn(Severity::Dropped, path, "A gradient stop's colour variable was not imported; it keeps its colour.");
            }
            stops.push(ColorStop { position, color });
        }
        if stops.is_empty() {
            self.warn(Severity::Approximated, path, "A gradient without colour stops uses black to transparent.");
        } else {
            out.stops = stops;
        }
        out
    }

    /// A `Pattern` (image fill) from `images/`.
    fn pattern(&mut self, p: &Value, path: &str) -> Option<ImagePaint> {
        let Some(name) = text(p, "image") else {
            self.warn(Severity::Dropped, path, "An image fill without an image was not imported.");
            return None;
        };
        let image = self.image_key(name, path)?;
        let scale_mode = match int(p, "type").unwrap_or(1) {
            0 => ImageScaleMode::Tile,
            3 => ImageScaleMode::Fit,
            2 => ImageScaleMode::Crop,
            4 => {
                self.warn(
                    Severity::Approximated,
                    path,
                    "A cropped image fill was imported as Fill: the README doesn't say how its crop matrix maps the image.",
                );
                ImageScaleMode::Fill
            }
            _ => ImageScaleMode::Fill,
        };
        // Stretch: the whole image over the box.
        let crop = (scale_mode == ImageScaleMode::Crop).then_some([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let degrees = num(p, "rotation").unwrap_or(0.0).clamp(-1e6, 1e6);
        let quarter = (degrees / 90.0).round();
        if (degrees - quarter * 90.0).abs() > 1e-6 {
            self.warn(Severity::Approximated, path, "The image fill's rotation was rounded to a quarter turn.");
        }
        let scale = num(p, "tileScale").filter(|s| *s > 0.0).map_or(1.0, |s| s.min(1e3));
        Some(ImagePaint { image, scale_mode, scale, rotation: (quarter as i64).rem_euclid(4) as i32, crop, filters: Default::default() })
    }

    /// Border weight (`thickness`, per side `customThickness`), position, caps, joins, dashes,
    /// miter limit and line endings.
    fn border_options(&mut self, n: &mut Node, j: &Value, path: &str) {
        n.stroke.weight = num(j, "thickness").unwrap_or(0.0).clamp(0.0, MAX_SIZE);
        if let Some([l, t, r, b]) = json::thickness(j, "customThickness").filter(|s| s.iter().any(|x| *x > 0.0)) {
            n.stroke.sides = Some([t, r, b, l]);
        }
        n.stroke.align = match int(j, "linePos").unwrap_or(0) {
            1 => StrokeAlign::Inside,
            2 => StrokeAlign::Outside,
            _ => StrokeAlign::Center,
        };
        let cap = match int(j, "lineCap").unwrap_or(0) {
            1 => StrokeCap::Round,
            2 => StrokeCap::Square,
            _ => StrokeCap::None,
        };
        n.stroke.cap_start = cap;
        n.stroke.cap_end = cap;
        n.stroke.join = match int(j, "lineJoin").unwrap_or(0) {
            1 => StrokeJoin::Round,
            2 => StrokeJoin::Bevel,
            _ => StrokeJoin::Miter,
        };
        n.stroke.dashes =
            list(j, "dash").iter().filter_map(Value::as_f64).filter(|d| d.is_finite()).map(|d| d.clamp(0.0, MAX_SIZE)).take(64).collect();
        if n.stroke.dashes.iter().all(|d| *d <= 0.0) {
            n.stroke.dashes.clear();
        }
        if let Some(m) = num(j, "miterLimit").filter(|m| *m >= 1.0) {
            n.stroke.miter_limit = m.min(100.0);
        }
        for (key, start) in [("startMarker", true), ("endMarker", false)] {
            let marker = match int(j, key).unwrap_or(0) {
                0 => continue,
                1 => StrokeCap::LineArrow,
                2 => StrokeCap::TriangleArrow,
                5 => StrokeCap::CircleFilled,
                4 => {
                    self.warn(Severity::Approximated, path, "An open circle line ending was imported as a filled circle.");
                    StrokeCap::CircleFilled
                }
                _ => {
                    self.warn(Severity::Dropped, path, "A line ending (perpendicular line or square) was not imported.");
                    continue;
                }
            };
            if start {
                n.stroke.cap_start = marker;
            } else {
                n.stroke.cap_end = marker;
            }
        }
    }

    /// Shadows, inner shadows and the blur.
    fn effects(&mut self, j: &Value, path: &str) -> Vec<Effect> {
        let mut out = Vec::new();
        for (key, inner) in [("shadows", false), ("innerShadows", true)] {
            for sh in list(j, key).iter().take(MAX_PAINTS) {
                if ["radiusId", "xId", "yId", "spreadId", "colorId"].iter().any(|k| sh.get(*k).is_some()) {
                    self.warn(Severity::Dropped, path, "A shadow's variable bindings were not imported; it keeps its values.");
                }
                let (dx, dy) = json::point(sh, "offset").unwrap_or((0.0, 0.0));
                let shadow = Shadow {
                    color: json::color(sh, "color").unwrap_or(Color::TRANSPARENT),
                    offset: (dx.clamp(-MAX_SIZE, MAX_SIZE), dy.clamp(-MAX_SIZE, MAX_SIZE)),
                    radius: num(sh, "radius").unwrap_or(0.0).clamp(0.0, MAX_SIZE),
                    spread: num(sh, "spread").unwrap_or(0.0).clamp(-MAX_SIZE, MAX_SIZE),
                    blend: blend(int(sh, "blendMode")),
                    show_behind: false,
                };
                let kind = if inner { EffectKind::InnerShadow(shadow) } else { EffectKind::DropShadow(shadow) };
                out.push(Effect { kind, visible: flag(sh, "enabled") != Some(false) });
            }
        }
        if let Some(b) = j.get("blur").filter(|b| b.is_object()) {
            out.push(self.blur(b, path));
        }
        out
    }

    /// A `BlurEffect` (enabled unless it says otherwise, version 11).
    fn blur(&mut self, b: &Value, path: &str) -> Effect {
        let visible = flag(b, "enabled") != Some(false);
        let ids = b.as_object().is_some_and(|m| m.keys().any(|k| k.ends_with("Id")));
        if ids {
            self.warn(Severity::Dropped, path, "The blur's variable bindings were not imported; it keeps its values.");
        }
        let blur = Blur::uniform(num(b, "radius").unwrap_or(10.0).clamp(0.0, MAX_SIZE)).sanitized();
        let kind = match int(b, "type").unwrap_or(0) {
            3 => {
                if num(b, "saturation").is_some_and(|s| (s - 1.0).abs() > 1e-6) {
                    self.warn(Severity::Approximated, path, "The background blur's saturation was not imported.");
                }
                EffectKind::BackgroundBlur(blur)
            }
            4 => {
                let pct = |k: &str, d: f64| num(b, k).unwrap_or(d) / 100.0;
                let glass = Glass {
                    light_angle: num(b, "glassLightAngle").unwrap_or(-45.0),
                    light_intensity: pct("glassLightIntensity", 80.0),
                    refraction: pct("glassRefraction", 80.0),
                    depth: num(b, "glassDepth").unwrap_or(20.0),
                    dispersion: pct("glassDispersion", 50.0),
                    frost: num(b, "glassFrost").unwrap_or(4.0),
                    splay: pct("glassSplay", 0.0),
                };
                EffectKind::Glass(glass.sanitized())
            }
            1 => {
                self.warn(Severity::Approximated, path, "The motion blur was imported as a layer blur (its angle was not imported).");
                EffectKind::LayerBlur(blur)
            }
            2 => {
                self.warn(Severity::Approximated, path, "The zoom blur was imported as a layer blur (its centre was not imported).");
                EffectKind::LayerBlur(blur)
            }
            0 => EffectKind::LayerBlur(blur),
            other => {
                self.warn(Severity::Approximated, path, format!("A blur of type {other} was imported as a layer blur."));
                EffectKind::LayerBlur(blur)
            }
        };
        Effect { kind, visible }
    }

    /// `cornerRadius` (four values) and `smoothCorners`.
    fn corners(&mut self, n: &mut Node, j: &Value, path: &str) {
        if let Some(c) = j.get("cornerRadius") {
            match json::floats(c, 4).as_deref() {
                Some([a, b, c, d]) => {
                    let r = [*a, *b, *c, *d].map(|x| x.clamp(0.0, MAX_SIZE));
                    if r.iter().any(|x| *x != r[0]) {
                        self.warn(
                            Severity::Approximated,
                            path,
                            "Different corner radii were applied clockwise from the top-left: the README doesn't say which corner each value is.",
                        );
                    }
                    n.corners.radii = r;
                }
                _ => self.warn(Severity::Dropped, path, "The corner radius couldn't be read and was not imported."),
            }
        }
        if flag(j, "smoothCorners") == Some(true) && n.corners.radii.iter().any(|r| *r > 0.0) {
            n.corners.smoothing = SMOOTH;
            self.warn(Severity::Approximated, path, "Smooth corners were imported with 60% corner smoothing.");
        }
    }

    /// Links to the document's colour and effect styles: reported, the layer keeps its own values.
    fn style_links(&mut self, j: &Value, path: &str) {
        for (key, what) in [("fillsId", "fill"), ("bordersId", "border"), ("effectsId", "effect")] {
            let Some(id) = text(j, key) else { continue };
            match self.style_names.get(clip(id)).cloned() {
                Some(name) => {
                    self.warn(Severity::Info, path, format!("The link to the {what} style “{name}” was not kept; the layer keeps its own values."))
                }
                None => self.warn(Severity::Dropped, path, format!("The layer's {what} style isn't in the file; the layer keeps its own values.")),
            }
        }
    }

    /// The layer's export options.
    pub(super) fn exports(&mut self, j: &Value, path: &str) -> Vec<ExportSetting> {
        let mut out = Vec::new();
        for e in list(j, "export").iter().take(64) {
            let format = match text(e, "format").unwrap_or("PNG").to_ascii_uppercase().as_str() {
                "PNG" => ExportFormat::Png,
                "JPG" | "JPEG" => ExportFormat::Jpg,
                "SVG" => ExportFormat::Svg,
                "PDF" => ExportFormat::Pdf,
                "WEBP" => ExportFormat::Webp,
                other => {
                    self.warn(Severity::Dropped, path, format!("The {} export format was not imported.", clip(other)));
                    continue;
                }
            };
            let value = num(e, "size").filter(|s| *s > 0.0).unwrap_or(1.0);
            let size = match int(e, "type").unwrap_or(0) {
                1 => ExportSize::Width { value: value.min(MAX_SIZE) },
                2 => ExportSize::Height { value: value.min(MAX_SIZE) },
                _ => ExportSize::Scale { value: value.min(100.0) },
            };
            if int(e, "naming").unwrap_or(0) != 0 {
                self.warn(Severity::Approximated, path, "An export name prefix was imported as a suffix.");
            }
            out.push(ExportSetting { size, suffix: text(e, "name").map(clip).unwrap_or("").to_string(), format, ..ExportSetting::default() });
        }
        out
    }
}
