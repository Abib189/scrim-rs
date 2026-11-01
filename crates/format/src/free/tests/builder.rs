//! FREE files built in code for the tests, from the README: layer JSON helpers, a deterministic
//! zip writer, and PNG and WebP images encoded in code (so fixtures are byte-for-byte repeatable).

use std::io::{Cursor, Write};

use serde_json::{Value, json};

/// A ShortGUID-shaped id (22 characters) from a number.
pub fn gid(n: u32) -> String {
    format!("Test{n:018}")
}

/// A layer of type `t` placed by `frame` (`[left, top, width, height]`).
pub fn layer(t: &str, id: u32, name: &str, frame: [f64; 4]) -> Value {
    json!({"_t": t, "id": gid(id), "name": name, "frame": frame})
}

/// `v` with `key` set to `value`.
pub fn with(mut v: Value, key: &str, value: Value) -> Value {
    v[key] = value;
    v
}

/// A container layer (`FRAME`, `GROUP`, `COMPONENT`, …) holding `layers`.
pub fn container(t: &str, id: u32, name: &str, frame: [f64; 4], layers: Vec<Value>) -> Value {
    with(layer(t, id, name, frame), "layers", Value::Array(layers))
}

pub fn text_layer(id: u32, name: &str, frame: [f64; 4], s: &str, font: &str, size: f64, fill: &str) -> Value {
    let t = with(layer("TEXT", id, name, frame), "text", json!(s));
    let t = with(t, "font", json!(font));
    let t = with(t, "fontSize", json!(size));
    with(t, "fill", json!(fill))
}

/// A page of `layers`.
pub fn page(id: u32, name: &str, layers: Vec<Value>) -> Value {
    json!({"id": gid(id), "name": name, "layers": layers})
}

/// A `.free` file: `document.json` (listing the pages, merged with `extra`), `meta.json` at
/// `version`, one `pages/<id>.json` per page, and `files`.
pub fn free_at(version: u64, pages: &[Value], extra: Value, files: &[(&str, &[u8])]) -> Vec<u8> {
    let ids: Vec<Value> = pages.iter().map(|p| p["id"].clone()).collect();
    let mut document = json!({"id": gid(9999), "pages": ids});
    if let (Some(d), Some(e)) = (document.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            d.insert(k.clone(), v.clone());
        }
    }
    let meta = json!({"version": version, "app": "Scrim tests", "variant": "test", "appVersion": "1.0"});
    let mut members =
        vec![("document.json".to_string(), serde_json::to_vec(&document).unwrap()), ("meta.json".to_string(), serde_json::to_vec(&meta).unwrap())];
    for p in pages {
        members.push((format!("pages/{}.json", p["id"].as_str().unwrap_or("page")), serde_json::to_vec(p).unwrap()));
    }
    for (name, bytes) in files {
        members.push((name.to_string(), bytes.to_vec()));
    }
    zip(&members)
}

/// A `.free` file at the README's version.
pub fn free(pages: &[Value], extra: Value, files: &[(&str, &[u8])]) -> Vec<u8> {
    free_at(11, pages, extra, files)
}

/// A zip of `members`, stored, with fixed timestamps and permissions.
pub fn zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for (name, bytes) in members {
        z.start_file(name.as_str(), opts).unwrap();
        z.write_all(bytes).unwrap();
    }
    z.finish().unwrap().into_inner()
}

/// A `w` × `h` RGBA image from `pixel`.
fn rgba(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> image::RgbaImage {
    image::RgbaImage::from_fn(w, h, |x, y| image::Rgba(pixel(x, y)))
}

pub fn png(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    rgba(w, h, pixel).write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// A lossless WebP.
pub fn webp(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let img = rgba(w, h, pixel);
    let mut out = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut out).encode(img.as_raw(), w, h, image::ExtendedColorType::Rgba8).unwrap();
    out
}
