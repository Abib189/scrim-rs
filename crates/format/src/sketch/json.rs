//! Readers for Sketch JSON values: missing, wrongly typed or non-finite values read as `None`.

use figcraft_doc::color::Color;
use serde_json::Value;

/// Largest width, height, stroke, radius or offset taken from the file.
pub(super) const MAX_SIZE: f64 = 1e6;
/// Largest coordinate taken from the file.
pub(super) const MAX_COORD: f64 = 1e7;
/// Most entries read from one list (layers, fills, points, attributes…).
const MAX_LIST: usize = 100_000;
/// Longest name kept (chars).
const MAX_NAME: usize = 512;

pub(super) fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k)?.as_f64().filter(|x| x.is_finite())
}

pub(super) fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k)?.as_str()
}

pub(super) fn flag(v: &Value, k: &str) -> Option<bool> {
    let f = v.get(k)?;
    f.as_bool().or_else(|| f.as_u64().map(|n| n != 0))
}

pub(super) fn int(v: &Value, k: &str) -> Option<i64> {
    v.get(k)?.as_i64()
}

pub(super) fn list<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(|a| a.get(..a.len().min(MAX_LIST)).unwrap_or_default()).unwrap_or_default()
}

/// A layer's child layers.
pub(super) fn layers(v: &Value) -> &[Value] {
    list(v, "layers")
}

/// `s` cut to [`MAX_NAME`] chars.
pub(super) fn clip(s: &str) -> &str {
    match s.char_indices().nth(MAX_NAME) {
        Some((i, _)) => s.get(..i).unwrap_or(s),
        None => s,
    }
}

/// A `PointString`: `"{x, y}"`.
pub(super) fn point(s: &str) -> Option<(f64, f64)> {
    if s.len() > 128 {
        return None;
    }
    let inner = s.trim().strip_prefix('{')?.strip_suffix('}')?;
    let (x, y) = inner.split_once(',')?;
    let (x, y) = (x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?);
    (x.is_finite() && y.is_finite() && x.abs() <= MAX_COORD && y.abs() <= MAX_COORD).then_some((x, y))
}

/// The `PointString` at `k`.
pub(super) fn point_at(v: &Value, k: &str) -> Option<(f64, f64)> {
    point(text(v, k)?)
}

/// A `Color`: `{red, green, blue, alpha}`, each 0..1 (alpha defaults to 1).
pub(super) fn color(v: &Value) -> Option<Color> {
    let c = |k: &str| num(v, k).map(|x| x.clamp(0.0, 1.0) as f32);
    Some(Color::new(c("red")?, c("green")?, c("blue")?, c("alpha").unwrap_or(1.0)))
}

/// A frame `{x, y, width, height}`, clamped.
pub(super) fn frame(v: &Value) -> Option<(f64, f64, f64, f64)> {
    let f = v.get("frame")?;
    let c = |k: &str| num(f, k).map(|x| x.clamp(-MAX_COORD, MAX_COORD));
    let s = |k: &str| num(f, k).map(|x| x.clamp(0.0, MAX_SIZE));
    Some((c("x")?, c("y")?, s("width")?, s("height")?))
}

/// Whether `name` is a plausible archive member name: relative, no parent steps, not too long.
pub(super) fn safe_member(name: &str) -> bool {
    !name.is_empty() && name.len() <= 512 && !name.starts_with('/') && !name.contains('\\') && !name.split('/').any(|p| p == ".." || p.is_empty())
}
