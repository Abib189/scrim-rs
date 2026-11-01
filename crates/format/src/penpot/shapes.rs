//! Per-type shape data: corner radii (`r1`–`r4`), vector paths from SVG path data (`content`),
//! boolean operations (`boolType`), images (`metadata` → a storage object) and the `transform`.

use figcraft_doc::color::{ImageScaleMode, Paint};
use figcraft_doc::geom::{BezPath, Shape};
use figcraft_doc::{BoolOp, Node, VectorNetwork};
use serde_json::Value;

use super::read::{MAX_SIZE, clip, id_at, num, text};
use super::{Ctx, MAX_PATH_DATA};
use crate::import_report::Severity;

/// Most path elements kept from one `content` string.
const MAX_PATH_ELEMENTS: usize = 200_000;
/// How far (in page units) path data may reach outside the layer's box before it is reported.
const SLACK: f64 = 1.0;

/// A documented `boolType` as Scrim's operation. Reading: `difference` takes the upper operands
/// away from the bottom one (Scrim's Subtract).
pub(super) fn bool_op(v: &Value) -> Option<BoolOp> {
    match text(v, "boolType")? {
        "union" => Some(BoolOp::Union),
        "difference" => Some(BoolOp::Subtract),
        "intersection" => Some(BoolOp::Intersect),
        "exclude" => Some(BoolOp::Exclude),
        _ => None,
    }
}

/// SVG path data `d` as a path, None when it doesn't parse or is too large.
fn svg_path(d: &str) -> Option<BezPath> {
    if d.len() > MAX_PATH_DATA {
        return None;
    }
    let p = BezPath::from_svg(d).ok()?;
    let finite = p.elements().iter().all(|el| el.end_point().is_none_or(|q| q.x.is_finite() && q.y.is_finite()));
    (finite && p.elements().len() <= MAX_PATH_ELEMENTS).then_some(p)
}

impl Ctx<'_> {
    /// `r1`–`r4` onto `n`. Reading: they go clockwise from the top-left corner.
    pub(super) fn radii(&mut self, n: &mut Node, v: &Value, path: &str) {
        let r = |k: &str| num(v, k).map(|x| x.clamp(0.0, MAX_SIZE));
        let radii = [r("r1"), r("r2"), r("r3"), r("r4")];
        if radii.iter().all(Option::is_none) {
            return;
        }
        let radii = radii.map(|x| x.unwrap_or(0.0));
        n.corners.radii = radii;
        if radii.iter().any(|x| (x - radii[0]).abs() > 1e-9) {
            self.warn(
                Severity::Info,
                path,
                "The corners have different radii; the documentation doesn't say which corner r1–r4 name, so they were read clockwise from the top left.",
            );
        }
    }

    /// The vector network of `content` (SVG path data), moved from page coordinates into the
    /// layer's box at `(x, y)` of `w` × `h`. None, with a warning, when it can't be read.
    pub(super) fn path_network(&mut self, v: &Value, (x, y, w, h): (f64, f64, f64, f64), path: &str) -> Option<VectorNetwork> {
        let Some(content) = v.get("content") else {
            self.warn(Severity::Dropped, path, "The path has no content; it was imported as an empty path.");
            return None;
        };
        let Some(d) = content.as_str() else {
            self.warn(
                Severity::Dropped,
                path,
                "The path's content isn't SVG path data (a string), and the documentation doesn't describe another form; it was imported as an empty path.",
            );
            return None;
        };
        let Some(mut bez) = svg_path(d) else {
            self.warn(Severity::Dropped, path, "The path's SVG path data couldn't be read; it was imported as an empty path.");
            return None;
        };
        bez.apply_affine(figcraft_doc::geom::Affine::translate((-x, -y)));
        if bez.elements().is_empty() {
            return Some(VectorNetwork::default());
        }
        let b = bez.bounding_box();
        if b.x0 < -SLACK || b.y0 < -SLACK || b.x1 > w + SLACK || b.y1 > h + SLACK {
            self.warn(
                Severity::Approximated,
                path,
                "The path data reaches outside the layer's selection rectangle; it was placed reading the data as page coordinates.",
            );
        }
        let mut net = VectorNetwork::from_path(&bez);
        net.sanitize();
        Some(net)
    }

    /// The image fill of an `image` shape: `metadata.id` names a storage object
    /// (`objects/{id}.{ext}`), or a media reference (`files/{file}/media/{id}.json`) whose
    /// `mediaId` does.
    pub(super) fn image_fill(&mut self, v: &Value, w: f64, h: f64, path: &str) -> Option<Paint> {
        let Some(meta) = v.get("metadata").filter(|m| m.is_object()) else {
            self.warn(Severity::Dropped, path, "The image layer has no metadata naming its image; it was imported without the image.");
            return None;
        };
        let Some(id) = id_at(meta, "id") else {
            self.warn(Severity::Dropped, path, "The image's metadata doesn't name a valid image id; it was imported without the image.");
            return None;
        };
        let key = self.image_key(id, path)?;
        if let (Some(iw), Some(ih)) = (num(meta, "width"), num(meta, "height"))
            && iw > 0.0
            && ih > 0.0
            && w > 0.0
            && h > 0.0
            && ((iw / ih) / (w / h) - 1.0).abs() > 0.01
        {
            self.warn(
                Severity::Approximated,
                path,
                "The image's proportions differ from the layer's; it fills the layer, cropped (the documentation doesn't say how the image is fitted).",
            );
        }
        Some(Paint::image(key, ImageScaleMode::Fill))
    }

    /// The document image key for image `id` (cached); None, with a warning, when it can't be read.
    fn image_key(&mut self, id: &str, path: &str) -> Option<String> {
        if let Some(k) = self.image_keys.get(id).cloned() {
            if k.is_none() {
                self.warn(Severity::Dropped, path, "The image couldn't be read and was not imported.");
            }
            return k;
        }
        let media = format!("files/{}/media/{id}.json", self.file_id);
        let object: Result<String, String> = if self.archive.has_object(id) || !self.archive.has(&media) {
            Ok(id.to_string())
        } else {
            match self.archive.json_within(&media, 64 * 1024) {
                Ok(m) => id_at(&m, "mediaId").map(str::to_string).ok_or_else(|| "its media reference names no storage object".to_string()),
                Err(e) => Err(format!("its media reference couldn't be read: {}", super::archive::message(&e))),
            }
        };
        let bytes = object.and_then(|o| self.archive.object(&o));
        let key = match bytes.and_then(|b| crate::add_image(&mut self.doc, b).map(|(k, _, _)| k).map_err(|e| super::archive::message(&e))) {
            Ok(k) => Some(k),
            Err(e) => {
                self.warn(Severity::Dropped, path, format!("The image couldn't be read ({}) and was not imported.", clip(&e)));
                None
            }
        };
        self.image_keys.insert(id.to_string(), key.clone());
        key
    }

    /// Warn when the shape's `transform` isn't the identity: the layer is placed by its position
    /// and size alone.
    pub(super) fn transform(&mut self, v: &Value, path: &str) {
        let Some(t) = v.get("transform") else { return };
        let m: Option<Vec<f64>> =
            t.as_array().filter(|a| a.len() == 6).and_then(|a| a.iter().map(|x| x.as_f64().filter(|f| f.is_finite())).collect());
        let Some(m) = m else {
            if !t.is_null() {
                self.unread_transforms += 1;
            }
            return;
        };
        let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        if m.iter().zip(identity).any(|(a, b)| (a - b).abs() > 1e-9) {
            self.warn(
                Severity::Approximated,
                path,
                "The layer's transform (a rotation, flip or skew) wasn't applied: the documentation doesn't say how to read it. The layer is placed by its position and size.",
            );
        }
    }
}
