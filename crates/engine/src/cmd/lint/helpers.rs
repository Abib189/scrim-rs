//! The helpers the rules cite (`docs/scrim-transfer/specs/lint.md` §8): parsing, colour and
//! contrast, and JavaScript's number formatting (the `detail` strings depend on it).
//!
//! Ported from Impeccable's detector (`detector/shared/{color,constants}.mjs`), Apache-2.0,
//! Copyright 2025 Paul Bakaus; modified by Scrim and this fork (`design-rules/NOTICE.md`).

use super::Css;

/// A colour, alpha 0–1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// `px(v)`: a number, `"12px"` or `"12"`; anything else (`%`, `em`, several values) is `None`.
pub fn px(v: Option<&Css>) -> Option<f64> {
    match v? {
        Css::Num(n) => Some(*n),
        Css::Str(s) => {
            let s = s.trim();
            let body = s.strip_suffix("px").unwrap_or(s);
            let digits = body.strip_prefix('-').unwrap_or(body);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                return None;
            }
            parse_float(body)
        }
    }
}

/// JavaScript's `parseFloat`: the longest leading decimal number.
pub fn parse_float(s: &str) -> Option<f64> {
    let s = s.trim_start();
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        i += 1;
    }
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    }
    let digits = &s[start..i];
    if digits.is_empty() || digits == "." {
        return None;
    }
    let mut end = i;
    // An exponent counts only when digits follow it.
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'-' || b[j] == b'+') {
            j += 1;
        }
        let k = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > k {
            end = j;
        }
    }
    s[..end].parse().ok()
}

/// `String(v).toLowerCase()`, or `""`.
pub fn lower(v: Option<&Css>) -> String {
    v.map(|v| v.text().to_lowercase()).unwrap_or_default()
}

/// The first family of a font stack, unquoted and lower-cased.
pub fn primary_font(v: Option<&Css>) -> Option<String> {
    let s = v?.text();
    let first = s.split(',').next().unwrap_or("").trim();
    let first = first.strip_prefix(['\'', '"']).unwrap_or(first);
    let first = first.strip_suffix(['\'', '"']).unwrap_or(first);
    let f = first.to_lowercase();
    (!f.is_empty()).then_some(f)
}

/// `letterSpacing` as a fraction of the font size.
pub fn tracking_em(letter_spacing: Option<&Css>, font_size: Option<&Css>) -> Option<f64> {
    let ls = letter_spacing?;
    if let Css::Str(s) = ls
        && s.ends_with("em")
    {
        return parse_float(s);
    }
    let (l, f) = (px(Some(ls))?, px(font_size)?);
    (f != 0.0).then(|| l / f)
}

fn hex_digit(c: u8) -> Option<f64> {
    (c as char).to_digit(16).map(f64::from)
}

/// `parseColor`: `#rgb`, `#rrggbb`, `rgb(…)` / `rgba(…)` with integer channels (unanchored), and
/// seven names. `transparent`, other hex lengths and everything else are `None`.
pub fn parse_color(s: &str) -> Option<Rgba> {
    let s = s.trim().to_lowercase();
    if s.is_empty() || s == "transparent" {
        return None;
    }
    let named = match s.as_str() {
        "white" => Some("#ffffff"),
        "black" => Some("#000000"),
        "red" => Some("#ff0000"),
        "green" => Some("#008000"),
        "blue" => Some("#0000ff"),
        "gray" | "grey" => Some("#808080"),
        _ => None,
    };
    let s = named.map_or(s.clone(), str::to_string);
    if let Some(h) = s.strip_prefix('#') {
        let b = h.as_bytes();
        let d: Option<Vec<f64>> = b.iter().map(|c| hex_digit(*c)).collect();
        let d = d?;
        return match d.len() {
            3 => Some(Rgba { r: d[0] * 17.0, g: d[1] * 17.0, b: d[2] * 17.0, a: 1.0 }),
            6 => Some(Rgba { r: d[0] * 16.0 + d[1], g: d[2] * 16.0 + d[3], b: d[4] * 16.0 + d[5], a: 1.0 }),
            _ => None,
        };
    }
    rgb_fn(&s)
}

/// The first `rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)` in `s`.
fn rgb_fn(s: &str) -> Option<Rgba> {
    let mut from = 0;
    while let Some(k) = s[from..].find("rgb") {
        let at = from + k;
        from = at + 3;
        let mut rest = &s[at + 3..];
        rest = rest.strip_prefix('a').unwrap_or(rest);
        let Some(rest) = rest.strip_prefix('(') else { continue };
        if let Some(c) = channels(rest) {
            return Some(c);
        }
    }
    None
}

fn channels(rest: &str) -> Option<Rgba> {
    let mut it = rest;
    let mut vals = [0.0f64; 3];
    for (i, v) in vals.iter_mut().enumerate() {
        if i > 0 {
            it = it.strip_prefix(',')?.trim_start_matches(char::is_whitespace);
        }
        let n = it.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return None;
        }
        *v = it[..n].parse().ok()?;
        it = &it[n..];
    }
    let mut a = 1.0;
    if let Some(r) = it.strip_prefix(',') {
        let r = r.trim_start_matches(char::is_whitespace);
        let n = r.bytes().take_while(|b| b.is_ascii_digit() || *b == b'.').count();
        if n == 0 {
            return None;
        }
        a = parse_float(&r[..n]).unwrap_or(f64::NAN);
        it = &r[n..];
    }
    it.strip_prefix(')')?;
    Some(Rgba { r: vals[0], g: vals[1], b: vals[2], a })
}

fn spread(c: Rgba) -> f64 {
    c.r.max(c.g).max(c.b) - c.r.min(c.g).min(c.b)
}

/// Neutral: absent, empty or transparent; unreadable is not neutral; else a channel spread < 30.
pub fn is_neutral(v: Option<&str>) -> bool {
    match v {
        None => true,
        Some(s) if s.trim().is_empty() || s.trim().eq_ignore_ascii_case("transparent") => true,
        Some(s) => parse_color(s).is_some_and(|c| spread(c) < 30.0),
    }
}

pub fn has_chroma(s: &str) -> bool {
    parse_color(s).is_some_and(|c| spread(c) >= 30.0)
}

/// Scrim's gray: neutral, with a luminance from 0.03 to 0.70.
pub fn is_gray(s: &str) -> bool {
    parse_color(s).is_some_and(|c| spread(c) < 30.0 && (0.03..=0.70).contains(&luminance(c)))
}

pub fn luminance(c: Rgba) -> f64 {
    let ch = |v: f64| {
        let v = v / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b)
}

/// WCAG contrast; alpha is ignored.
pub fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// JavaScript's `Math.round`: half toward +∞.
pub fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// JavaScript's `toFixed(digits)`: the nearest decimal to the exact binary value, ties away
/// from zero (Rust's `{:.n}` breaks exact ties to even).
pub fn to_fixed(x: f64, digits: usize) -> String {
    if !x.is_finite() {
        return super::super::query::js_number(x);
    }
    let neg = x < 0.0;
    let ax = x.abs();
    // The exact decimal expansion of a double has at most 1074 fractional digits.
    let exact = format!("{ax:.1100}");
    let (int, frac) = exact.split_once('.').unwrap_or((&exact, ""));
    let keep = &frac[..digits.min(frac.len())];
    let rest = &frac[digits.min(frac.len())..];
    let up = matches!(rest.as_bytes().first(), Some(b'5'..=b'9'));
    let mut num: Vec<u8> = format!("{int}{keep}").into_bytes();
    if up {
        let mut i = num.len();
        loop {
            if i == 0 {
                num.insert(0, b'1');
                break;
            }
            i -= 1;
            if num[i] == b'9' {
                num[i] = b'0';
            } else {
                num[i] += 1;
                break;
            }
        }
    }
    let s = String::from_utf8(num).unwrap_or_default();
    let (i, f) = s.split_at(s.len() - digits);
    let body = if digits == 0 { i.to_string() } else { format!("{}.{f}", if i.is_empty() { "0" } else { i }) };
    // JavaScript keeps the sign of a negative number that rounds to zero: "-0.00".
    if neg { format!("-{body}") } else { body }
}

/// A JavaScript number as `String(n)` writes it.
pub fn js_num(x: f64) -> String {
    super::super::query::js_number(x)
}

/// UTF-16 length.
pub fn len16(s: &str) -> usize {
    s.encode_utf16().count()
}

/// The first `n` UTF-16 units.
pub fn slice16(s: &str, n: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().take(n).collect();
    String::from_utf16_lossy(&units)
}
