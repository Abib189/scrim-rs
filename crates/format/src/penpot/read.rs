//! Readers for Penpot JSON values: missing, wrongly typed or non-finite values read as `None`.

use figcraft_doc::color::Color;
use serde_json::Value;

/// Largest width, height or radius taken from the file.
pub(super) const MAX_SIZE: f64 = 1e6;
/// Largest coordinate taken from the file.
pub(super) const MAX_COORD: f64 = 1e7;
/// Most entries read from one list (fills, child ids, stops…).
const MAX_LIST: usize = 100_000;
/// Longest name kept (chars).
const MAX_NAME: usize = 512;
/// The all-zero UUID: the parent the documentation's example rectangle names (read as the page).
pub(super) const NIL: &str = "00000000-0000-0000-0000-000000000000";

pub(super) fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k)?.as_f64().filter(|x| x.is_finite())
}

pub(super) fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k)?.as_str()
}

pub(super) fn flag(v: &Value, k: &str) -> Option<bool> {
    v.get(k)?.as_bool()
}

pub(super) fn list<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(|a| a.get(..a.len().min(MAX_LIST)).unwrap_or_default()).unwrap_or_default()
}

/// Whether `v[k]` holds something: a non-empty array, object or string, `true` or a number.
pub(super) fn present(v: &Value, k: &str) -> bool {
    match v.get(k) {
        None | Some(Value::Null) => false,
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(_)) => true,
    }
}

/// `s` cut to [`MAX_NAME`] chars.
pub(super) fn clip(s: &str) -> &str {
    match s.char_indices().nth(MAX_NAME) {
        Some((i, _)) => s.get(..i).unwrap_or(s),
        None => s,
    }
}

/// Whether `id` can name an archive member: a UUID-like run of ASCII letters, digits and dashes.
pub(super) fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// The id at `k`, when it can name an archive member.
pub(super) fn id_at<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    text(v, k).filter(|s| safe_id(s))
}

/// A hex colour (`#RRGGBB`, `#RRGGBBAA`, or the shorter forms) with `opacity` (0..1) folded
/// into its alpha.
pub(super) fn hex(s: &str, opacity: f64) -> Option<Color> {
    if s.len() > 16 {
        return None;
    }
    let c = Color::from_hex(s)?;
    Some(c.with_alpha(c.a * opacity.clamp(0.0, 1.0) as f32))
}

/// `opacity` at `k` (0..1; 1 when absent).
pub(super) fn opacity(v: &Value, k: &str) -> f64 {
    num(v, k).unwrap_or(1.0).clamp(0.0, 1.0)
}

/// A position and size `{x, y, width, height}` read from `v`'s own fields, clamped.
pub(super) fn rect(v: &Value) -> Option<(f64, f64, f64, f64)> {
    let c = |k: &str| num(v, k).map(|x| x.clamp(-MAX_COORD, MAX_COORD));
    let s = |k: &str| num(v, k).map(|x| x.clamp(0.0, MAX_SIZE));
    Some((c("x")?, c("y")?, s("width")?, s("height")?))
}
