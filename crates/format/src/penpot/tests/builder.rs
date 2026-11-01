//! Penpot exports built in code for the tests, following the documented layout
//! (`manifest.json`, `files/{file}/pages/{page}.json`, `files/{file}/pages/{page}/{shape}.json`,
//! `files/{file}/components|colors|media/…`, `objects/{uuid}.json` + `objects/{uuid}.{ext}`),
//! with a deterministic zip writer and a small PNG encoder, so fixtures are byte-for-byte
//! repeatable.

use std::io::{Cursor, Write};

use serde_json::{Value, json};

use super::super::read::NIL;

/// The id of the (first) file of every export built here.
pub const FILE: &str = "73b59a94-3ea3-8189-8007-000000000001";

/// A UUID from a number.
pub fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// `v` with `key` set to `value`.
pub fn with(mut v: Value, key: &str, value: Value) -> Value {
    v[key] = value;
    v
}

/// A shape of `kind` with the documented base attributes, at page box `(x, y, w, h)` (and the
/// geometry attributes for the types that have them).
pub fn shape(id: u32, kind: &str, name: &str, (x, y, w, h): (f64, f64, f64, f64)) -> Value {
    let mut s = json!({
        "id": uuid(id),
        "name": name,
        "type": kind,
        "selrect": {"x": x, "y": y, "width": w, "height": h},
        "points": [{"x": x, "y": y}, {"x": x + w, "y": y}, {"x": x + w, "y": y + h}, {"x": x, "y": y + h}],
        "transform": [1, 0, 0, 1, 0, 0],
        "transformInverse": [1, 0, 0, 1, 0, 0],
        "parentId": NIL,
        "frameId": NIL,
    });
    if matches!(kind, "frame" | "rect" | "circle" | "image" | "svg-raw" | "text") {
        s["x"] = json!(x);
        s["y"] = json!(y);
        s["width"] = json!(w);
        s["height"] = json!(h);
    }
    s
}

/// The documented solid fill: `{"color": hex, "opacity": opacity}`.
pub fn fill(hex: &str, opacity: f64) -> Value {
    json!({"color": hex, "opacity": opacity})
}

/// The documented gradient colour object, from `(startX, startY)` to `(endX, endY)`.
pub fn gradient(start: (f64, f64), end: (f64, f64), stops: &[(&str, f64, f64)]) -> Value {
    let stops: Vec<Value> = stops.iter().map(|(c, offset, opacity)| json!({"color": c, "offset": offset, "opacity": opacity})).collect();
    json!({"gradient": {"type": "linear", "startX": start.0, "startY": start.1, "endX": end.0, "endY": end.1, "stops": stops}})
}

/// `parent` holding `kids` (bottom first), each a flattened subtree whose first shape is the kid:
/// the parent's `shapes` lists the kids, their `parentId` names it. Returns the flattened tree.
pub fn nest(parent: Value, kids: Vec<Vec<Value>>) -> Vec<Value> {
    let ids: Vec<Value> = kids.iter().filter_map(|k| k.first()).map(|h| h["id"].clone()).collect();
    let mut out = vec![with(parent.clone(), "shapes", Value::Array(ids))];
    for kid in kids {
        let mut it = kid.into_iter();
        if let Some(head) = it.next() {
            out.push(with(head, "parentId", parent["id"].clone()));
        }
        out.extend(it);
    }
    out
}

/// A Penpot export under construction.
pub struct Export {
    pub manifest: Value,
    members: Vec<(String, Vec<u8>)>,
}

impl Export {
    /// One file, [`FILE`], named "Test file".
    pub fn new() -> Self {
        let manifest = json!({
            "type": "penpot/export-files",
            "version": 1,
            "generatedBy": "penpot/2.12.0",
            "refer": "penpot",
            "files": [{"id": FILE, "name": "Test file", "features": ["components/v2"]}],
            "relations": [],
        });
        let file = json!({"id": FILE, "name": "Test file", "revn": 1, "createdAt": "2026-11-01T00:00:00Z", "modifiedAt": "2026-11-01T00:00:00Z", "features": ["components/v2"]});
        Self { manifest, members: vec![(format!("files/{FILE}.json"), bytes(&file))] }
    }

    /// Add member `name` holding `data`.
    pub fn member(mut self, name: &str, data: Vec<u8>) -> Self {
        self.members.push((name.to_string(), data));
        self
    }

    /// Add JSON member `name`.
    pub fn json(self, name: &str, v: &Value) -> Self {
        self.member(name, bytes(v))
    }

    /// Page `n` named `name` (`extra` page fields) holding `shapes` (every subtree flattened).
    /// A page root (the all-zero id) lists the top-level shapes in the order given.
    pub fn page(self, n: u32, name: &str, extra: Value, shapes: Vec<Value>) -> Self {
        let roots: Vec<Value> = shapes.iter().filter(|s| s["parentId"] == NIL).map(|s| s["id"].clone()).collect();
        let root = with(shape(0, "frame", "Root Frame", (0.0, 0.0, 0.01, 0.01)), "id", json!(NIL));
        self.page_with_root(n, name, extra, Some(with(root, "shapes", Value::Array(roots))), shapes)
    }

    /// [`page`](Self::page) with an explicit root shape (or none).
    pub fn page_with_root(mut self, n: u32, name: &str, extra: Value, root: Option<Value>, shapes: Vec<Value>) -> Self {
        let pid = uuid(0x9000 + n);
        let mut page = json!({"id": pid, "name": name});
        if let (Some(p), Some(e)) = (page.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                p.insert(k.clone(), v.clone());
            }
        }
        self = self.json(&format!("files/{FILE}/pages/{pid}.json"), &page);
        for s in root.into_iter().chain(shapes) {
            let id = s["id"].as_str().unwrap_or("").to_string();
            self = self.json(&format!("files/{FILE}/pages/{pid}/{id}.json"), &s);
        }
        self
    }

    /// Component `n` named `name` whose main instance is shape `main`.
    pub fn component(self, n: u32, name: &str, main: u32) -> Self {
        let c = json!({"id": uuid(0x7000 + n), "name": name, "path": "", "mainInstanceId": uuid(main), "mainInstancePage": uuid(0x9001)});
        self.json(&format!("files/{FILE}/components/{}.json", uuid(0x7000 + n)), &c)
    }

    /// Storage object `id`: `objects/{id}.json` (`contentType`) and `objects/{id}.{ext}`.
    pub fn object(self, id: &str, ext: &str, content_type: &str, data: Vec<u8>) -> Self {
        let meta = json!({"id": id, "size": data.len(), "contentType": content_type, "bucket": "file-media-object"});
        self.json(&format!("objects/{id}.json"), &meta).member(&format!("objects/{id}.{ext}"), data)
    }

    /// The zip: `manifest.json` first, then the members in the order added.
    pub fn bytes(&self) -> Vec<u8> {
        let mut all = vec![("manifest.json".to_string(), bytes(&self.manifest))];
        all.extend(self.members.iter().cloned());
        zip(&all)
    }
}

pub fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec_pretty(v).unwrap()
}

/// A stored zip of `members`, every timestamp 1980-01-01.
pub fn zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for (name, data) in members {
        z.start_file(name.as_str(), opts).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap().into_inner()
}

/// A `w` × `h` RGBA PNG of `pixel(x, y)`: zlib with stored blocks, so the bytes never change.
pub fn png(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            raw.extend_from_slice(&pixel(x, y));
        }
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push(u8::from(i + 1 == chunks.len()));
        let len = c.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [(b"IHDR", ihdr), (b"IDAT", z), (b"IEND", Vec::new())] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(&data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for byte in data {
        c ^= u32::from(*byte);
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}
