//! Sketch files built in code for the tests, from the published schema: layer JSON helpers, a
//! deterministic zip writer and a small PNG encoder (so fixtures are byte-for-byte repeatable).

use std::io::{Cursor, Write};

use serde_json::{Value, json};

/// A UUID-shaped id (`do_objectID`, `symbolID`) from a number.
pub fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012X}")
}

pub fn frame(x: f64, y: f64, w: f64, h: f64) -> Value {
    json!({"_class": "rect", "constrainProportions": false, "x": x, "y": y, "width": w, "height": h})
}

pub fn color(r: f64, g: f64, b: f64, a: f64) -> Value {
    json!({"_class": "color", "red": r, "green": g, "blue": b, "alpha": a})
}

/// A style with nothing on (Sketch writes a disabled blur on every style).
pub fn style() -> Value {
    json!({
        "_class": "style",
        "fills": [],
        "borders": [],
        "shadows": [],
        "innerShadows": [],
        "blur": {"_class": "blur", "isEnabled": false, "center": "{0.5, 0.5}", "motionAngle": 0, "radius": 10, "saturation": 1, "type": 0},
        "contextSettings": {"_class": "graphicsContextSettings", "blendMode": 0, "opacity": 1},
        "startMarkerType": 0,
        "endMarkerType": 0,
        "miterLimit": 10,
        "windingRule": 1,
    })
}

/// A layer of `class` with the fields every layer has.
pub fn layer(class: &str, id: u32, name: &str, f: Value) -> Value {
    json!({
        "_class": class,
        "do_objectID": uuid(id),
        "name": name,
        "nameIsFixed": false,
        "frame": f,
        "isVisible": true,
        "isLocked": false,
        "isFlippedHorizontal": false,
        "isFlippedVertical": false,
        "rotation": 0,
        "booleanOperation": -1,
        "layerListExpandedType": 0,
        "shouldBreakMaskChain": false,
        "style": style(),
    })
}

/// `v` with `key` set to `value`.
pub fn with(mut v: Value, key: &str, value: Value) -> Value {
    v[key] = value;
    v
}

/// `v` with `style.key` set to `value`.
pub fn styled(mut v: Value, key: &str, value: Value) -> Value {
    v["style"][key] = value;
    v
}

/// A container layer (`artboard`, `group`, `shapeGroup`, `symbolMaster`) holding `layers`.
pub fn container(class: &str, id: u32, name: &str, f: Value, layers: Vec<Value>) -> Value {
    with(layer(class, id, name, f), "layers", Value::Array(layers))
}

pub fn artboard(id: u32, name: &str, f: Value, layers: Vec<Value>) -> Value {
    let a = container("artboard", id, name, f, layers);
    let a = with(a, "hasBackgroundColor", json!(true));
    with(a, "backgroundColor", color(1.0, 1.0, 1.0, 1.0))
}

pub fn page(id: u32, name: &str, layers: Vec<Value>) -> Value {
    container("page", id, name, frame(0.0, 0.0, 0.0, 0.0), layers)
}

/// A straight curve point at `(x, y)` (unit box) with corner `radius`.
pub fn point(x: f64, y: f64, radius: f64) -> Value {
    let p = format!("{{{x}, {y}}}");
    json!({"_class": "curvePoint", "cornerRadius": radius, "cornerStyle": 0, "curveFrom": p, "curveTo": p, "curveMode": 1, "hasCurveFrom": false, "hasCurveTo": false, "point": p})
}

/// A curved point: `to` arrives at it, `from` leaves it (unit box).
pub fn curve(at: (f64, f64), to: (f64, f64), from: (f64, f64)) -> Value {
    let s = |(x, y): (f64, f64)| format!("{{{x}, {y}}}");
    json!({"_class": "curvePoint", "cornerRadius": 0, "cornerStyle": 0, "curveFrom": s(from), "curveTo": s(to), "curveMode": 2, "hasCurveFrom": true, "hasCurveTo": true, "point": s(at)})
}

/// A shape layer of `class` with `points`.
pub fn shape(class: &str, id: u32, name: &str, f: Value, points: Vec<Value>, closed: bool) -> Value {
    let s = with(layer(class, id, name, f), "points", Value::Array(points));
    let s = with(s, "isClosed", json!(closed));
    let s = with(s, "edited", json!(false));
    with(s, "pointRadiusBehaviour", json!(1))
}

/// A rectangle with a radius per corner (`[tl, tr, br, bl]`).
pub fn rectangle(id: u32, name: &str, f: Value, radii: [f64; 4]) -> Value {
    let [tl, tr, br, bl] = radii;
    let pts = vec![point(0.0, 0.0, tl), point(1.0, 0.0, tr), point(1.0, 1.0, br), point(0.0, 1.0, bl)];
    with(shape("rectangle", id, name, f, pts, true), "fixedRadius", json!(0))
}

pub fn oval(id: u32, name: &str, f: Value) -> Value {
    let k = 0.2238576251;
    let pts = vec![
        curve((0.5, 0.0), (0.5 - k, 0.0), (0.5 + k, 0.0)),
        curve((1.0, 0.5), (1.0, 0.5 - k), (1.0, 0.5 + k)),
        curve((0.5, 1.0), (0.5 + k, 1.0), (0.5 - k, 1.0)),
        curve((0.0, 0.5), (0.0, 0.5 + k), (0.0, 0.5 - k)),
    ];
    shape("oval", id, name, f, pts, true)
}

/// A regular star's points in the unit box: outer points on the ellipse, the top one first.
pub fn star(id: u32, name: &str, f: Value, count: u32, ratio: f64) -> Value {
    let pts = (0..count * 2)
        .map(|i| {
            let t = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * f64::from(i) / f64::from(count);
            let r = if i % 2 == 0 { 0.5 } else { 0.5 * ratio };
            point(0.5 + r * t.cos(), 0.5 + r * t.sin(), 0.0)
        })
        .collect();
    let s = shape("star", id, name, f, pts, true);
    let s = with(s, "numberOfPoints", json!(count));
    with(s, "radius", json!(ratio))
}

pub fn solid(c: Value) -> Value {
    json!({"_class": "fill", "isEnabled": true, "fillType": 0, "color": c,
        "contextSettings": {"_class": "graphicsContextSettings", "blendMode": 0, "opacity": 1},
        "noiseIndex": 0, "noiseIntensity": 0, "patternFillType": 1, "patternTileScale": 1})
}

/// A gradient fill (`kind`: 0 linear, 1 radial, 2 angular) with `(position, colour)` stops.
pub fn gradient(kind: u32, from: &str, to: &str, stops: &[(f64, Value)]) -> Value {
    let stops: Vec<Value> = stops.iter().map(|(p, c)| json!({"_class": "gradientStop", "position": p, "color": c})).collect();
    let g = json!({"_class": "gradient", "gradientType": kind, "elipseLength": 0, "from": from, "to": to, "stops": stops});
    with(with(solid(color(0.0, 0.0, 0.0, 1.0)), "fillType", json!(1)), "gradient", g)
}

/// An image fill from archive member `reference` (`patternFillType` 0 tile, 1 fill, 2 stretch, 3 fit).
pub fn image_fill(reference: &str, pattern: u32) -> Value {
    let f = with(solid(color(0.0, 0.0, 0.0, 1.0)), "fillType", json!(4));
    let f = with(f, "patternFillType", json!(pattern));
    with(f, "image", file_ref(reference))
}

pub fn file_ref(reference: &str) -> Value {
    json!({"_class": "MSJSONFileReference", "_ref_class": "MSImageData", "_ref": reference})
}

/// A border of `c`, `thickness` wide (`position`: 0 centre, 1 inside, 2 outside).
pub fn border(c: Value, thickness: f64, position: u32) -> Value {
    json!({"_class": "border", "isEnabled": true, "fillType": 0, "color": c, "position": position, "thickness": thickness,
        "contextSettings": {"_class": "graphicsContextSettings", "blendMode": 0, "opacity": 1}})
}

pub fn shadow(class: &str, c: Value, x: f64, y: f64, blur: f64, spread: f64) -> Value {
    json!({"_class": class, "isEnabled": true, "blurRadius": blur, "color": c, "offsetX": x, "offsetY": y, "spread": spread,
        "contextSettings": {"_class": "graphicsContextSettings", "blendMode": 0, "opacity": 1}})
}

/// String attributes for a run: PostScript font `font` at `size`, colour `c`, `alignment`.
pub fn attrs(font: &str, size: f64, c: Value, alignment: u32) -> Value {
    json!({
        "MSAttributedStringFontAttribute": {"_class": "fontDescriptor", "attributes": {"name": font, "size": size}},
        "MSAttributedStringColorAttribute": c,
        "kerning": 0,
        "paragraphStyle": {"_class": "paragraphStyle", "alignment": alignment},
    })
}

/// A text layer showing `string` with `(location, length, attributes)` runs (UTF-16 ranges).
pub fn text(id: u32, name: &str, f: Value, string: &str, runs: Vec<(usize, usize, Value)>) -> Value {
    let attributes: Vec<Value> = runs
        .into_iter()
        .map(|(location, length, a)| json!({"_class": "stringAttribute", "location": location, "length": length, "attributes": a}))
        .collect();
    let t = with(layer("text", id, name, f), "attributedString", json!({"_class": "attributedString", "string": string, "attributes": attributes}));
    let t = with(t, "textBehaviour", json!(1));
    with(t, "automaticallyDrawOnUnderlyingPath", json!(false))
}

/// A symbol master `symbol` holding `layers`.
pub fn master(id: u32, symbol: u32, name: &str, f: Value, layers: Vec<Value>) -> Value {
    let m = container("symbolMaster", id, name, f, layers);
    let m = with(m, "symbolID", json!(uuid(symbol)));
    let m = with(m, "hasBackgroundColor", json!(false));
    let m = with(m, "includeBackgroundColorInInstance", json!(false));
    with(m, "allowsOverrides", json!(true))
}

/// An instance of `symbol` with `overrides` (`(overrideName, value)`).
pub fn instance(id: u32, symbol: u32, name: &str, f: Value, overrides: Vec<(String, Value)>) -> Value {
    let o: Vec<Value> = overrides.into_iter().map(|(n, v)| json!({"_class": "overrideValue", "overrideName": n, "value": v})).collect();
    let i = with(layer("symbolInstance", id, name, f), "symbolID", json!(uuid(symbol)));
    let i = with(i, "overrideValues", Value::Array(o));
    with(i, "scale", json!(1))
}

/// The bytes of a `.sketch` zip: `document.json` listing `pages` (plus `extra` document
/// fields), `meta.json`, `user.json`, `pages/<id>.json` and `files` (images). Stored, with
/// fixed timestamps, so the same input always gives the same bytes.
pub fn sketch(pages: &[Value], extra: Value, files: &[(&str, &[u8])]) -> Vec<u8> {
    let refs: Vec<Value> = pages
        .iter()
        .map(|p| json!({"_class": "MSJSONFileReference", "_ref_class": "MSImmutablePage", "_ref": format!("pages/{}", p["do_objectID"].as_str().unwrap_or(""))}))
        .collect();
    let mut document = json!({
        "_class": "document",
        "do_objectID": uuid(0xD0C),
        "assets": {"_class": "assetCollection", "do_objectID": uuid(0xA55), "images": [], "colors": [], "gradients": [], "exportPresets": []},
        "colorSpace": 1,
        "currentPageIndex": 0,
        "foreignLayerStyles": [],
        "foreignSymbols": [],
        "foreignTextStyles": [],
        "layerStyles": {"_class": "sharedStyleContainer", "objects": []},
        "layerTextStyles": {"_class": "sharedTextStyleContainer", "objects": []},
        "perDocumentLibraries": [],
        "pages": refs,
    });
    if let (Some(d), Some(e)) = (document.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            d.insert(k.clone(), v.clone());
        }
    }
    let meta = json!({
        "commit": "0000000000000000000000000000000000000000",
        "pagesAndArtboards": {},
        "version": 146,
        "compatibilityVersion": 99,
        "app": "com.bohemiancoding.sketch3",
        "autosaved": 0,
        "variant": "NONAPPSTORE",
        "created": {"commit": "0000000000000000000000000000000000000000", "appVersion": "99.0", "build": 0, "app": "com.bohemiancoding.sketch3",
            "compatibilityVersion": 99, "version": 146, "variant": "NONAPPSTORE"},
        "saveHistory": ["NONAPPSTORE.0"],
        "appVersion": "99.0",
        "build": 0,
    });
    let user = json!({"document": {"pageListHeight": 100, "pageListCollapsed": 0}});
    let mut members: Vec<(String, Vec<u8>)> =
        vec![("document.json".into(), json_bytes(&document)), ("meta.json".into(), json_bytes(&meta)), ("user.json".into(), json_bytes(&user))];
    for p in pages {
        members.push((format!("pages/{}.json", p["do_objectID"].as_str().unwrap_or("")), json_bytes(p)));
    }
    for (name, bytes) in files {
        members.push(((*name).to_string(), bytes.to_vec()));
    }
    zip(&members)
}

fn json_bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}

/// A stored zip of `members`, every timestamp 1980-01-01.
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
