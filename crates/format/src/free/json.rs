//! Readers for FREE JSON values: missing, wrongly typed or non-finite values read as `None`. The
//! README's base types: colours are ARGB hex strings with minified forms, points and sizes are
//! `[x, y]`, rects `[left, top, width, height]`, thickness `[l, t, r, b]` (or `[uniform]`,
//! `[horizontal, vertical]`), matrices `[scaleX, skewX, transX, skewY, scaleY, transY]`.

use figcraft_doc::color::Color;
use figcraft_doc::geom::Affine;
use serde_json::Value;

/// Largest width, height, stroke, radius or offset taken from the file.
pub(super) const MAX_SIZE: f64 = 1e6;
/// Largest coordinate taken from the file.
pub(super) const MAX_COORD: f64 = 1e7;
/// Most entries read from one list (layers, fills, points, inlines…).
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
    v.get(k)?.as_bool()
}

/// An integer field (an enum's value): a JSON integer, or a float with no fraction.
pub(super) fn int(v: &Value, k: &str) -> Option<i64> {
    let f = v.get(k)?;
    f.as_i64().or_else(|| f.as_f64().filter(|x| x.is_finite() && x.fract() == 0.0 && x.abs() < 1e15).map(|x| x as i64))
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

/// The finite numbers of the array `v` (None when it isn't an array of numbers, or is longer
/// than `max`).
pub(super) fn floats(v: &Value, max: usize) -> Option<Vec<f64>> {
    let a = v.as_array()?;
    if a.len() > max {
        return None;
    }
    a.iter().map(|x| x.as_f64().filter(|x| x.is_finite())).collect()
}

/// A `Point` (`[x, y]`) at `k`.
pub(super) fn point(v: &Value, k: &str) -> Option<(f64, f64)> {
    match floats(v.get(k)?, 2)?.as_slice() {
        [x, y] => Some((x.clamp(-MAX_COORD, MAX_COORD), y.clamp(-MAX_COORD, MAX_COORD))),
        _ => None,
    }
}

/// A `Size` (`[width, height]`) at `k`.
pub(super) fn size(v: &Value, k: &str) -> Option<(f64, f64)> {
    match floats(v.get(k)?, 2)?.as_slice() {
        [w, h] => Some((w.clamp(0.0, MAX_SIZE), h.clamp(0.0, MAX_SIZE))),
        _ => None,
    }
}

/// A `Rect` (`[left, top, width, height]`) at `k`.
pub(super) fn rect(v: &Value, k: &str) -> Option<(f64, f64, f64, f64)> {
    match floats(v.get(k)?, 4)?.as_slice() {
        [x, y, w, h] => {
            let c = |a: f64| a.clamp(-MAX_COORD, MAX_COORD);
            let s = |a: f64| a.clamp(0.0, MAX_SIZE);
            Some((c(*x), c(*y), s(*w), s(*h)))
        }
        _ => None,
    }
}

/// A `Thickness` at `k` as `[left, top, right, bottom]`: one value for all sides, two for
/// horizontal and vertical, or four (llms.txt and the schema's `Thickness` constructors).
pub(super) fn thickness(v: &Value, k: &str) -> Option<[f64; 4]> {
    let c = |a: f64| a.clamp(0.0, MAX_SIZE);
    match floats(v.get(k)?, 4)?.as_slice() {
        [u] => Some([c(*u); 4]),
        [h, v] => Some([c(*h), c(*v), c(*h), c(*v)]),
        [l, t, r, b] => Some([c(*l), c(*t), c(*r), c(*b)]),
        _ => None,
    }
}

/// A `Matrix` at `k`: `[scaleX, skewX, transX, skewY, scaleY, transY]`, mapping a layer's own
/// space into its parent's (`x' = scaleX·x + skewX·y + transX`, `y' = skewY·x + scaleY·y + transY`).
pub(super) fn matrix(v: &Value, k: &str) -> Option<Affine> {
    match floats(v.get(k)?, 6)?.as_slice() {
        [sx, kx, tx, ky, sy, ty] => {
            let l = |a: f64| a.clamp(-1e4, 1e4);
            let t = |a: f64| a.clamp(-MAX_COORD, MAX_COORD);
            Some(Affine::new([l(*sx), l(*ky), l(*kx), l(*sy), t(*tx), t(*ty)]))
        }
        _ => None,
    }
}

/// A `Color`: an ARGB hex string without `#`. Minified forms (llms.txt): one digit repeats six
/// times (`3` = `333333`), two repeat three times (`34` = `343434`), three double each digit
/// (`F33` = `FF3333`); six digits are opaque RGB, eight are `AARRGGBB`.
pub(super) fn color_str(s: &str) -> Option<Color> {
    if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let full = match s.len() {
        1 => s.repeat(6),
        2 => s.repeat(3),
        3 => s.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => s.to_string(),
        _ => return None,
    };
    let v = u32::from_str_radix(&full, 16).ok()?;
    let a = if full.len() == 8 { (v >> 24) as u8 } else { 255 };
    Some(Color::from_rgba8((v >> 16) as u8, (v >> 8) as u8, v as u8, a))
}

/// The `Color` at `k`.
pub(super) fn color(v: &Value, k: &str) -> Option<Color> {
    color_str(text(v, k)?)
}

/// Whether `name` is a plausible archive member name: relative, no parent steps, not too long.
pub(super) fn safe_member(name: &str) -> bool {
    !name.is_empty() && name.len() <= 512 && !name.starts_with('/') && !name.contains('\\') && !name.split('/').any(|p| p == ".." || p.is_empty())
}
