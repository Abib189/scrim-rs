//! A layer's `style`: fills, borders and border options, shadows, inner shadows, blur, opacity
//! and blend mode; and its export options.

use figcraft_doc::color::{BlendMode, Color, ColorStop, Gradient, GradientKind, ImagePaint, ImageScaleMode, Paint, PaintKind};
use figcraft_doc::{Blur, Effect, EffectKind, ExportFormat, ExportSetting, ExportSize, Node, NodeKind, Shadow, StrokeAlign, StrokeCap, StrokeJoin};
use serde_json::Value;

use super::Ctx;
use super::json::{self, MAX_SIZE, clip, flag, int, list, num, point_at, text};
use crate::import_report::Severity;

/// Most paints, borders or shadows read from one style.
const MAX_PAINTS: usize = 256;

/// A Sketch `BlendMode` (0 Normal … 17 Plus lighter).
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

/// `contextSettings` → (opacity, blend mode).
fn context(v: &Value) -> (f32, BlendMode) {
    let c = v.get("contextSettings");
    let opacity = c.and_then(|c| num(c, "opacity")).map_or(1.0, |o| o.clamp(0.0, 1.0) as f32);
    (opacity, blend(c.and_then(|c| int(c, "blendMode"))))
}

/// The handle perpendicular to `from` → `to`, `ratio` times as long (in the unit box).
fn width_handle(from: (f64, f64), to: (f64, f64), ratio: f64) -> (f64, f64) {
    (from.0 - (to.1 - from.1) * ratio, from.1 + (to.0 - from.0) * ratio)
}

impl Ctx<'_> {
    /// Apply `j`'s style to `n`.
    pub(super) fn style(&mut self, n: &mut Node, j: &Value, path: &str) {
        let Some(s) = j.get("style").filter(|s| s.is_object()) else { return };
        let fills: Vec<Paint> = list(s, "fills").iter().take(MAX_PAINTS).filter_map(|f| self.paint(f, path, "fill")).collect();
        match &n.kind {
            NodeKind::Group | NodeKind::Slice => {
                if fills.iter().any(|p| p.visible) {
                    self.warn(Severity::Dropped, path, "Fills on groups and slices were not imported.");
                }
            }
            // Text keeps its runs' colours unless the style fills it.
            NodeKind::Text(_) if fills.is_empty() => {}
            NodeKind::Frame | NodeKind::Component { .. } if fills.is_empty() => {}
            _ => n.fills = fills,
        }
        self.borders(n, s, path);
        n.effects = self.effects(s, path);
        let (opacity, mode) = context(s);
        n.opacity = opacity;
        if mode != BlendMode::Normal || !n.kind.is_container() {
            n.blend = mode;
        }
        if s.get("colorControls").and_then(|c| flag(c, "isEnabled")) == Some(true) {
            self.warn(Severity::Dropped, path, "Colour adjustments (hue, saturation, brightness, contrast) were not imported.");
        }
    }

    /// A fill or border's paint (`what` names it in warnings).
    fn paint(&mut self, f: &Value, path: &str, what: &str) -> Option<Paint> {
        let (opacity, blend) = context(f);
        let visible = flag(f, "isEnabled").unwrap_or(true);
        let kind = match int(f, "fillType").unwrap_or(0) {
            0 => {
                let c = f.get("color").and_then(json::color).unwrap_or(Color::BLACK);
                return Some(Paint { kind: PaintKind::Solid { color: c.with_alpha(1.0) }, opacity: opacity * c.a, visible, blend });
            }
            1 => PaintKind::Gradient(self.gradient(f.get("gradient").unwrap_or(&Value::Null), path)),
            4 if what == "fill" => {
                let Some(r) = f.get("image") else {
                    self.warn(Severity::Dropped, path, "An image fill without an image was not imported.");
                    return None;
                };
                let key = self.image_key(r, path)?;
                let (scale_mode, crop) = match int(f, "patternFillType") {
                    Some(0) => (ImageScaleMode::Tile, None),
                    // Stretch: the whole image over the box.
                    Some(2) => (ImageScaleMode::Crop, Some([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])),
                    Some(3) => (ImageScaleMode::Fit, None),
                    _ => (ImageScaleMode::Fill, None),
                };
                let scale = num(f, "patternTileScale").filter(|s| *s > 0.0).map_or(1.0, |s| s.min(1e3));
                PaintKind::Image(ImagePaint { image: key, scale_mode, scale, rotation: 0, crop, filters: Default::default() })
            }
            other => {
                self.warn(Severity::Dropped, path, format!("A {what} of type {other} was not imported."));
                return None;
            }
        };
        Some(Paint { kind, opacity, visible, blend })
    }

    /// A `gradient`: linear and radial as they are, angular with an approximated start angle.
    fn gradient(&mut self, g: &Value, path: &str) -> Gradient {
        let kind = match int(g, "gradientType") {
            Some(1) => GradientKind::Radial,
            Some(2) => GradientKind::Angular,
            _ => GradientKind::Linear,
        };
        let mut out = Gradient::default_for(kind, Color::BLACK);
        let from = point_at(g, "from").map(|(x, y)| (x.clamp(-1e3, 1e3), y.clamp(-1e3, 1e3)));
        let to = point_at(g, "to").map(|(x, y)| (x.clamp(-1e3, 1e3), y.clamp(-1e3, 1e3)));
        match (kind, from, to) {
            (GradientKind::Angular, ..) => {
                self.warn(Severity::Approximated, path, "The angular gradient's start angle is approximated.");
            }
            (_, Some(from), Some(to)) => {
                let ratio = match (kind, num(g, "elipseLength")) {
                    (GradientKind::Radial, Some(e)) if e > 0.0 => {
                        if (e - 1.0).abs() > 1e-6 {
                            self.warn(Severity::Approximated, path, "The elliptical radial gradient's shape is approximated.");
                        }
                        e.min(1e3)
                    }
                    _ => 1.0,
                };
                out.from = from;
                out.to = to;
                out.width = width_handle(from, to, ratio);
            }
            _ => self.warn(Severity::Approximated, path, "A gradient without start and end points uses the default direction."),
        }
        let stops: Vec<ColorStop> = list(g, "stops")
            .iter()
            .take(MAX_PAINTS)
            .filter_map(|s| Some(ColorStop { position: num(s, "position")?.clamp(0.0, 1.0) as f32, color: s.get("color").and_then(json::color)? }))
            .collect();
        if stops.is_empty() {
            self.warn(Severity::Approximated, path, "A gradient without colour stops uses black to transparent.");
        } else {
            out.stops = stops;
        }
        out
    }

    /// Borders (one stroke weight and position for all of them), dashes, caps, joins and line
    /// endings.
    fn borders(&mut self, n: &mut Node, s: &Value, path: &str) {
        let borders: Vec<&Value> = list(s, "borders").iter().take(MAX_PAINTS).collect();
        let mut sizing: Option<(f64, i64)> = None;
        let mut mixed = false;
        let mut strokes = Vec::new();
        for b in borders {
            let Some(p) = self.paint(b, path, "border") else { continue };
            let size = (num(b, "thickness").unwrap_or(1.0).clamp(0.0, MAX_SIZE), int(b, "position").unwrap_or(0));
            if p.visible {
                match sizing {
                    None => sizing = Some(size),
                    Some(first) => mixed |= first != size,
                }
            }
            strokes.push(p);
        }
        if mixed {
            self.warn(Severity::Approximated, path, "Borders with different thicknesses or positions all use the first one's.");
        }
        if let Some((weight, position)) =
            sizing.or_else(|| list(s, "borders").first().map(|b| (num(b, "thickness").unwrap_or(1.0), int(b, "position").unwrap_or(0))))
        {
            n.stroke.weight = weight.clamp(0.0, MAX_SIZE);
            n.stroke.align = match position {
                1 => StrokeAlign::Inside,
                2 => StrokeAlign::Outside,
                _ => StrokeAlign::Center,
            };
        }
        n.strokes = strokes;
        if let Some(o) = s.get("borderOptions").filter(|o| flag(o, "isEnabled") != Some(false)) {
            n.stroke.dashes =
                list(o, "dashPattern").iter().filter_map(Value::as_f64).filter(|d| d.is_finite()).map(|d| d.clamp(0.0, MAX_SIZE)).take(64).collect();
            if n.stroke.dashes.iter().all(|d| *d <= 0.0) {
                n.stroke.dashes.clear();
            }
            let cap = match int(o, "lineCapStyle") {
                Some(1) => StrokeCap::Round,
                Some(2) => StrokeCap::Square,
                _ => StrokeCap::None,
            };
            n.stroke.cap_start = cap;
            n.stroke.cap_end = cap;
            n.stroke.join = match int(o, "lineJoinStyle") {
                Some(1) => StrokeJoin::Round,
                Some(2) => StrokeJoin::Bevel,
                _ => StrokeJoin::Miter,
            };
        }
        if let Some(m) = num(s, "miterLimit").filter(|m| *m >= 1.0) {
            n.stroke.miter_limit = m.min(100.0);
        }
        for (key, start) in [("startMarkerType", true), ("endMarkerType", false)] {
            let cap = match int(s, key).unwrap_or(0) {
                0 => continue,
                1 => StrokeCap::TriangleArrow,
                4 => StrokeCap::CircleFilled,
                3 => {
                    self.warn(Severity::Approximated, path, "An open circle line ending was imported as a filled circle.");
                    StrokeCap::CircleFilled
                }
                _ => {
                    self.warn(Severity::Dropped, path, "A line ending (open arrow, line or square) was not imported.");
                    continue;
                }
            };
            if start {
                n.stroke.cap_start = cap;
            } else {
                n.stroke.cap_end = cap;
            }
        }
    }

    /// Shadows, inner shadows and the blur.
    fn effects(&mut self, s: &Value, path: &str) -> Vec<Effect> {
        let mut out = Vec::new();
        for (key, inner) in [("shadows", false), ("innerShadows", true)] {
            for sh in list(s, key).iter().take(MAX_PAINTS) {
                let (opacity, blend) = context(sh);
                let color = sh.get("color").and_then(json::color).unwrap_or(Color::new(0.0, 0.0, 0.0, 0.5));
                let shadow = Shadow {
                    color: color.with_alpha(color.a * opacity),
                    offset: (
                        num(sh, "offsetX").unwrap_or(0.0).clamp(-MAX_SIZE, MAX_SIZE),
                        num(sh, "offsetY").unwrap_or(0.0).clamp(-MAX_SIZE, MAX_SIZE),
                    ),
                    radius: num(sh, "blurRadius").unwrap_or(0.0).clamp(0.0, MAX_SIZE),
                    spread: num(sh, "spread").unwrap_or(0.0).clamp(-MAX_SIZE, MAX_SIZE),
                    blend,
                    show_behind: false,
                };
                let kind = if inner { EffectKind::InnerShadow(shadow) } else { EffectKind::DropShadow(shadow) };
                out.push(Effect { kind, visible: flag(sh, "isEnabled").unwrap_or(true) });
            }
        }
        // Every style carries a blur; only an enabled one shows.
        if let Some(b) = s.get("blur").filter(|b| flag(b, "isEnabled") == Some(true)) {
            let blur = Blur { radius: num(b, "radius").unwrap_or(0.0).clamp(0.0, MAX_SIZE), ..Blur::default() }.sanitized();
            let kind = match int(b, "type").unwrap_or(0) {
                3 => EffectKind::BackgroundBlur(blur),
                0 => EffectKind::LayerBlur(blur),
                t => {
                    let what = if t == 1 { "motion" } else { "zoom" };
                    self.warn(Severity::Approximated, path, format!("The {what} blur was imported as a layer blur."));
                    EffectKind::LayerBlur(blur)
                }
            };
            out.push(Effect { kind, visible: true });
        }
        out
    }

    /// The layer's export formats.
    pub(super) fn exports(&mut self, j: &Value, path: &str) -> Vec<ExportSetting> {
        let Some(o) = j.get("exportOptions") else { return Vec::new() };
        let mut out = Vec::new();
        for f in list(o, "exportFormats").iter().take(64) {
            let format = match text(f, "fileFormat") {
                Some("png") => ExportFormat::Png,
                Some("jpg") => ExportFormat::Jpg,
                Some("svg") => ExportFormat::Svg,
                Some("pdf") => ExportFormat::Pdf,
                Some("webp") => ExportFormat::Webp,
                other => {
                    self.warn(
                        Severity::Dropped,
                        path,
                        format!("The {} export format was not imported.", clip(other.unwrap_or("unknown")).to_uppercase()),
                    );
                    continue;
                }
            };
            let absolute = num(f, "absoluteSize").filter(|s| *s > 0.0).map(|s| s.min(MAX_SIZE));
            let size = match (int(f, "visibleScaleType"), absolute) {
                (Some(1), Some(v)) => ExportSize::Width { value: v },
                (Some(2), Some(v)) => ExportSize::Height { value: v },
                _ => ExportSize::Scale { value: num(f, "scale").filter(|s| *s > 0.0).map_or(1.0, |s| s.min(100.0)) },
            };
            out.push(ExportSetting { size, suffix: text(f, "name").map(clip).unwrap_or("").to_string(), format, ..ExportSetting::default() });
        }
        if !list(o, "includedLayerIds").is_empty() {
            self.warn(Severity::Dropped, path, "The export's list of included layers was not imported.");
        }
        if flag(o, "shouldTrim") == Some(true) && !out.is_empty() {
            self.warn(Severity::Dropped, path, "Trimming transparent pixels on export was not imported.");
        }
        out
    }
}
