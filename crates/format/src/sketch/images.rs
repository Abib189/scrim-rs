//! Images: bitmap layers and image fills, from the archive's `images/` folder
//! (`MSJSONFileReference`) or inline base64 (`MSJSONOriginalDataReference`).

use figcraft_doc::Node;
use figcraft_doc::color::{BlendMode, ImagePaint, ImageScaleMode, Paint, PaintKind};
use serde_json::Value;

use super::Ctx;
use super::json::{clip, flag, point, safe_member, text};
use crate::import_report::Severity;

/// Longest inline base64 string read.
const MAX_BASE64: usize = 180 * 1024 * 1024;

/// Standard base64 (RFC 4648, with or without padding; whitespace ignored), None when malformed.
pub(super) fn base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut padding = false;
    for b in s.bytes() {
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                padding = true;
                continue;
            }
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        if padding {
            return None;
        }
        acc = ((acc << 6) | u32::from(v)) & 0x00FF_FFFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    (bits < 6).then_some(out)
}

impl Ctx<'_> {
    /// The document image key for an image reference `r` (bytes added with [`crate::add_image`]);
    /// None, with a warning on `path`, when it can't be read.
    pub(super) fn image_key(&mut self, r: &Value, path: &str) -> Option<String> {
        let class = text(r, "_class");
        let cache = match class {
            Some("MSJSONFileReference") => text(r, "_ref").map(|s| format!("file:{}", clip(s))),
            Some("MSJSONOriginalDataReference") => {
                text(r, "sha1").or_else(|| r.get("sha1").and_then(|s| text(s, "_data"))).map(|s| format!("data:{}", clip(s)))
            }
            _ => None,
        };
        if let Some(k) = cache.as_ref().and_then(|c| self.image_keys.get(c)) {
            let k = k.clone();
            if k.is_none() {
                self.warn(Severity::Dropped, path, "The image couldn't be read and was not imported.");
            }
            return k;
        }
        let bytes = match class {
            Some("MSJSONFileReference") => self.image_member(text(r, "_ref").unwrap_or("")),
            Some("MSJSONOriginalDataReference") => self.inline_image(r),
            _ => Err("it is not an image reference".to_string()),
        };
        let key = match bytes.and_then(|b| crate::add_image(&mut self.doc, b).map(|(hash, _, _)| hash).map_err(|e| e.to_string())) {
            Ok(k) => Some(k),
            Err(e) => {
                self.warn(Severity::Dropped, path, format!("The image couldn't be read ({}) and was not imported.", clip(&e)));
                None
            }
        };
        if let Some(c) = cache {
            self.image_keys.insert(c, key.clone());
        }
        key
    }

    /// The bytes of image member `name` (Sketch writes it with or without its extension).
    fn image_member(&mut self, name: &str) -> Result<Vec<u8>, String> {
        if !safe_member(name) {
            return Err("its file name is invalid".into());
        }
        let found = [name.to_string(), format!("{name}.png"), format!("{name}.jpg")].into_iter().find(|n| self.archive.has(n));
        match found {
            Some(n) => self.archive.image(&n),
            None => Err(format!("{} is missing from the file", clip(name))),
        }
    }

    /// The bytes of an inline image (`data._data`, base64).
    fn inline_image(&mut self, r: &Value) -> Result<Vec<u8>, String> {
        let data = r.get("data").and_then(|d| text(d, "_data")).ok_or("it has no data")?;
        if data.len() > MAX_BASE64 {
            return Err("it is too large".into());
        }
        let bytes = base64(data).ok_or("its data is not valid base64")?;
        if !self.archive.take_inline(bytes.len()) {
            return Err("the file's images are too large".into());
        }
        Ok(bytes)
    }

    /// A bitmap layer: a rectangle filled with its image, stretched to the frame, under the
    /// style's fills.
    pub(super) fn bitmap(&mut self, n: &mut Node, j: &Value, path: &str) {
        let shown = n.fills.iter().any(|p| p.visible);
        if shown {
            let how = if flag(j, "fillReplacesImage") == Some(true) { "in place of the image's pixels" } else { "with the image" };
            self.warn(Severity::Approximated, path, format!("The bitmap's fills, shown {how} in Sketch, were placed over the image."));
        }
        if text(j, "clippingMask").is_some_and(|m| !full_box(m)) {
            self.warn(Severity::Dropped, path, "The bitmap's crop outline was not imported; the whole image shows.");
        }
        let Some(r) = j.get("image") else {
            self.warn(Severity::Dropped, path, "The bitmap has no image; it was imported as an empty rectangle.");
            return;
        };
        let Some(key) = self.image_key(r, path) else { return };
        let image = ImagePaint {
            image: key,
            scale_mode: ImageScaleMode::Crop,
            scale: 1.0,
            rotation: 0,
            crop: Some([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            filters: Default::default(),
        };
        n.fills.insert(0, Paint { kind: PaintKind::Image(image), opacity: 1.0, visible: true, blend: BlendMode::Normal });
    }
}

/// Whether a `PointListString` outline is the whole unit box.
fn full_box(s: &str) -> bool {
    let Some(inner) = s.trim().strip_prefix('{').and_then(|t| t.strip_suffix('}')) else { return false };
    let mut corners = 0;
    for part in inner.split('}').map(|p| p.trim_start_matches([',', ' '])).filter(|p| !p.is_empty()) {
        match point(&format!("{part}}}")) {
            Some((x, y)) if (x == 0.0 || x == 1.0) && (y == 0.0 || y == 1.0) => corners += 1,
            _ => return false,
        }
    }
    corners == 4
}
