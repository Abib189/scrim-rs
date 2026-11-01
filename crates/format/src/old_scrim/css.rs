//! The CSS values the earlier Scrim stored (`layout` and `style` are CSS properties in camelCase):
//! lengths, colours, borders, shadows and fonts, read as far as Scrim's model has them.

use figcraft_doc::color::{BlendMode, Color};
use figcraft_doc::{LetterSpacing, LineHeight, Shadow};
use serde_json::Value;

/// A length in pixels: a number, or a string like `"12"` or `"12px"`. `None` for anything else
/// (`"auto"`, `"50%"`, `"fit-content"`, `calc(…)`).
pub fn px(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().filter(|x| x.is_finite()),
        Value::String(s) => {
            let t = s.trim();
            t.strip_suffix("px").unwrap_or(t).trim().parse::<f64>().ok().filter(|x| x.is_finite())
        }
        _ => None,
    }
}

/// A size as the earlier Scrim meant it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    Px(f64),
    /// `auto`, `fit-content` or unset: as big as the content.
    Hug,
    /// `100%`: the parent's size.
    Fill,
    /// Something else (`50%`, `calc(…)`, `min(…)`): kept as hug, and reported.
    Other,
}

pub fn size(v: Option<&Value>) -> Size {
    let Some(v) = v else { return Size::Hug };
    if let Some(x) = px(v) {
        return Size::Px(x.max(0.0));
    }
    match v.as_str().map(str::trim) {
        Some("auto" | "fit-content" | "max-content" | "min-content") => Size::Hug,
        Some("100%" | "stretch") => Size::Fill,
        _ => Size::Other,
    }
}

/// A CSS colour (`#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`, `rgba()`, `hsl()` or a named colour).
/// `transparent` is a colour with no alpha.
pub fn color(v: &Value) -> Option<Color> {
    let s = v.as_str()?.trim();
    if s.eq_ignore_ascii_case("transparent") {
        return Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });
    }
    Color::parse_css(s)
}

/// Padding (or any four-sided length): `12`, `"12px 8px"`, `"4px 8px 4px 8px"`, as `[top, right,
/// bottom, left]`.
pub fn sides(v: &Value) -> Option<[f64; 4]> {
    if let Some(x) = px(v) {
        return Some([x; 4]);
    }
    let parts: Vec<f64> = v.as_str()?.split_whitespace().map(|p| px(&Value::String(p.into()))).collect::<Option<_>>()?;
    match parts.as_slice() {
        [a] => Some([*a; 4]),
        [v, h] => Some([*v, *h, *v, *h]),
        [t, h, b] => Some([*t, *h, *b, *h]),
        [t, r, b, l] => Some([*t, *r, *b, *l]),
        _ => None,
    }
}

/// `border: 1px solid #ccc` as its width and colour (`None` for `none`, or a style other than
/// solid, which comes back as `Some((w, c, false))`).
pub fn border(v: &Value) -> Option<(f64, Color, bool)> {
    let s = v.as_str()?.trim();
    if s.eq_ignore_ascii_case("none") || s == "0" {
        return None;
    }
    let mut width = 1.0;
    let mut colour = Color::BLACK;
    let mut solid = true;
    for part in split_outside_parens(s, ' ') {
        if let Some(w) = px(&Value::String(part.clone())) {
            width = w;
        } else if let Some(c) = Color::parse_css(&part) {
            colour = c;
        } else if matches!(part.as_str(), "dashed" | "dotted" | "double" | "groove" | "ridge" | "inset" | "outset") {
            solid = false;
        }
    }
    Some((width, colour, solid))
}

/// `box-shadow` as its shadows, each with whether it is inset; `None` for something not read.
pub fn shadows(v: &Value) -> Option<Vec<(Shadow, bool)>> {
    let s = v.as_str()?.trim();
    if s.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    split_outside_parens(s, ',').iter().map(|one| shadow(one)).collect()
}

fn shadow(s: &str) -> Option<(Shadow, bool)> {
    let mut lengths = Vec::new();
    let mut colour = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.25 };
    let mut inset = false;
    for part in split_outside_parens(s.trim(), ' ') {
        if part == "inset" {
            inset = true;
        } else if let Some(l) = px(&Value::String(part.clone())) {
            lengths.push(l);
        } else {
            colour = Color::parse_css(&part)?;
        }
    }
    let at = |i: usize| lengths.get(i).copied().unwrap_or(0.0);
    if lengths.len() < 2 || lengths.len() > 4 {
        return None;
    }
    let shadow = Shadow { color: colour, offset: (at(0), at(1)), radius: at(2), spread: at(3), blend: BlendMode::Normal, ..Default::default() };
    Some((shadow, inset))
}

/// `s` split at `sep`, but not inside parentheses (`rgba(0, 0, 0, 0.2)` stays whole); empty parts
/// are left out.
fn split_outside_parens(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if c == sep && depth == 0 {
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_string());
            }
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// The first family of a `font-family` list, without quotes.
pub fn font_family(v: &Value) -> Option<String> {
    let first = v.as_str()?.split(',').next()?.trim().trim_matches(['"', '\'']).trim();
    (!first.is_empty() && !matches!(first, "sans-serif" | "serif" | "monospace" | "system-ui")).then(|| first.to_string())
}

/// A font style name for a CSS weight (`400`, `"bold"`) and style (`italic`).
pub fn font_style(weight: Option<&Value>, italic: bool) -> String {
    let w = match weight {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(400.0),
        Some(Value::String(s)) => match s.trim() {
            "bold" | "bolder" => 700.0,
            "normal" | "lighter" => 400.0,
            other => other.parse().unwrap_or(400.0),
        },
        _ => 400.0,
    };
    let name = match w.round() as i64 {
        ..=150 => "Thin",
        151..=250 => "ExtraLight",
        251..=350 => "Light",
        351..=450 => "Regular",
        451..=550 => "Medium",
        551..=650 => "SemiBold",
        651..=750 => "Bold",
        751..=850 => "ExtraBold",
        _ => "Black",
    };
    match (name, italic) {
        ("Regular", true) => "Italic".into(),
        (n, true) => format!("{n} Italic"),
        (n, false) => n.into(),
    }
}

/// `line-height`: unitless is a multiple of the size, `%` a percentage, `px` pixels.
pub fn line_height(v: &Value) -> Option<LineHeight> {
    match v {
        Value::Number(n) => n.as_f64().map(|x| LineHeight::Percent(x * 100.0)),
        Value::String(s) => {
            let t = s.trim();
            if t == "normal" {
                Some(LineHeight::Auto)
            } else if let Some(p) = t.strip_suffix('%') {
                p.trim().parse().ok().map(LineHeight::Percent)
            } else if t.ends_with("px") {
                px(v).map(LineHeight::Pixels)
            } else {
                t.parse::<f64>().ok().map(|x| LineHeight::Percent(x * 100.0))
            }
        }
        _ => None,
    }
}

/// `letter-spacing` in pixels or `em` (a percentage of the size).
pub fn letter_spacing(v: &Value) -> Option<LetterSpacing> {
    if let Some(x) = px(v) {
        return Some(LetterSpacing::Pixels(x));
    }
    let t = v.as_str()?.trim();
    t.strip_suffix("em").and_then(|e| e.trim().parse::<f64>().ok()).map(|e| LetterSpacing::Percent(e * 100.0))
}
