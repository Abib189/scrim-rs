//! The per-layer rules that read only the document (`lint.md` §7.2, steps 1.1–1.4).
//!
//! Ported from Impeccable's detector (`detector/rules/checks.mjs`,
//! `detector/engines/regex/detect-text.mjs`), Apache-2.0, Copyright 2025 Paul Bakaus; modified
//! by Scrim and this fork (`design-rules/NOTICE.md`). `size-duplicated` and
//! `style-shorthand-conflict` are Scrim's own.

use super::helpers::{
    contrast, has_chroma, is_gray, is_neutral, js_num, len16, lower, parse_color, parse_float, primary_font, px, to_fixed, tracking_em,
};
use super::{Css, Decls, Model, Out, ancestors, decls, get, is_text, motion};

const OVERUSED_FONTS: &[&str] = &[
    "inter",
    "roboto",
    "open sans",
    "lato",
    "montserrat",
    "arial",
    "helvetica",
    "fraunces",
    "instrument sans",
    "instrument serif",
    "geist",
    "geist sans",
    "geist mono",
    "mona sans",
    "plus jakarta sans",
    "space grotesk",
    "recoleta",
];

/// WCAG large text: 18pt.
pub const LARGE_TEXT_PX: f64 = 18.0 * (96.0 / 72.0);

const BUZZWORDS: &[&str] = &[
    "streamline your",
    "empower your",
    "supercharge your",
    "unleash your",
    "unleash the power",
    "leverage the power",
    "built for the modern",
    "trusted by leading",
    "trusted by the world",
    "best-in-class",
    "industry-leading",
    "world-class",
    "enterprise-grade",
    "next-generation",
    "cutting-edge",
    "transform your business",
    "revolutionize",
    "game-changer",
    "game changing",
    "mission-critical",
    "best of breed",
    "future-proof",
    "future proof",
    "seamless experience",
    "seamlessly integrate",
    "drive engagement",
    "drive growth",
    "drive results",
    "harness the power",
];

const SIZE_KEYS: &[&str] = &["width", "height", "minWidth", "minHeight", "maxWidth", "maxHeight", "flexBasis"];
const SIDES: [&str; 4] = ["Top", "Right", "Bottom", "Left"];

/// A border shorthand read token by token: the last length is the width, the last colour-like
/// token the colour.
pub struct Side {
    pub width: f64,
    pub color: Option<String>,
}

pub fn parse_border(v: Option<&Css>) -> Side {
    let mut out = Side { width: 0.0, color: None };
    let Some(v) = v else { return out };
    for tok in v.text().split_whitespace() {
        if let Some(w) = px(Some(&Css::Str(tok.to_string()))) {
            out.width = w;
        } else if parse_color(tok).is_some() || tok.starts_with('#') || tok.contains("rgb") || tok.contains("hsl") {
            out.color = Some(tok.to_string());
        }
    }
    out
}

fn side_of(d: &Decls, side: &str) -> Side {
    let all = parse_border(get(d, "border"));
    let sh = parse_border(get(d, &format!("border{side}")));
    let w = px(get(d, &format!("border{side}Width")));
    let color = get(d, &format!("border{side}Color")).map(Css::text).or(sh.color).or(all.color);
    let width = w.unwrap_or(if sh.width != 0.0 { sh.width } else { all.width });
    Side { width, color }
}

/// The node's own `style.background ?? style.backgroundColor` that `parseColor` reads, else the
/// nearest ancestor's.
fn background<M: Model>(m: &M, id: M::Id) -> Option<String> {
    std::iter::once(id).chain(ancestors(m, id)).find_map(|n| {
        let s = m.style(n);
        let v = get(&s, "background").or_else(|| get(&s, "backgroundColor"))?;
        parse_color(&v.text()).map(|_| v.text())
    })
}

/// The longhands a group key covers (itself for a longhand).
fn longhands(key: &str) -> Option<Vec<String>> {
    let corners = ["TopLeft", "TopRight", "BottomRight", "BottomLeft"];
    let parts = ["Width", "Style", "Color"];
    let t = ["transitionProperty", "transitionDuration", "transitionTimingFunction", "transitionDelay", "transitionBehavior"];
    let a = [
        "animationName",
        "animationDuration",
        "animationTimingFunction",
        "animationDelay",
        "animationIterationCount",
        "animationDirection",
        "animationFillMode",
        "animationPlayState",
    ];
    let v: Vec<String> = match key {
        "padding" | "margin" => SIDES.iter().map(|s| format!("{key}{s}")).collect(),
        "borderRadius" => corners.iter().map(|c| format!("border{c}Radius")).collect(),
        "gap" => vec!["rowGap".into(), "columnGap".into()],
        "border" => SIDES.iter().flat_map(|s| parts.iter().map(move |p| format!("border{s}{p}"))).collect(),
        "transition" => t.iter().map(|s| s.to_string()).collect(),
        "animation" => a.iter().map(|s| s.to_string()).collect(),
        _ => {
            if let Some(p) = key.strip_prefix("border").filter(|p| parts.contains(p)) {
                SIDES.iter().map(|s| format!("border{s}{p}")).collect()
            } else if let Some(s) = key.strip_prefix("border").filter(|s| SIDES.contains(s)) {
                parts.iter().map(|p| format!("border{s}{p}")).collect()
            } else {
                return None;
            }
        }
    };
    Some(v)
}

/// Whether `key` belongs to a shorthand group, as a combined key or a longhand.
fn in_group(key: &str) -> bool {
    const COMBINED: &[&str] = &[
        "padding",
        "margin",
        "borderRadius",
        "gap",
        "border",
        "borderWidth",
        "borderStyle",
        "borderColor",
        "borderTop",
        "borderRight",
        "borderBottom",
        "borderLeft",
        "transition",
        "animation",
    ];
    COMBINED.iter().any(|c| *c == key || longhands(c).is_some_and(|l| l.iter().any(|x| x == key)))
}

fn sides_of(key: &str) -> Vec<String> {
    longhands(key).unwrap_or_else(|| vec![key.to_string()])
}

fn shorthand_conflicts(layout: &Decls, style: &Decls) -> Vec<String> {
    let mut keys: Vec<&str> = Vec::new();
    for (k, _) in layout.iter().chain(style) {
        if in_group(k) && !keys.contains(&k.as_str()) {
            keys.push(k);
        }
    }
    let mut out = Vec::new();
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            let (la, lb) = (sides_of(a), sides_of(b));
            if (longhands(a).is_some() || longhands(b).is_some()) && la.iter().any(|x| lb.contains(x)) {
                out.push(format!("{a} + {b}"));
            }
        }
    }
    out
}

/// The blur of a box shadow: the third `-?\d+(\.\d+)?px` in it.
fn shadow_blur(v: &str) -> f64 {
    let b = v.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let mut j = i;
        if b[j] == b'-' {
            j += 1;
        }
        let digits = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > digits {
            let mut k = j;
            if k + 1 < b.len() && b[k] == b'.' && b[k + 1].is_ascii_digit() {
                k += 1;
                while k < b.len() && b[k].is_ascii_digit() {
                    k += 1;
                }
            }
            if v[k..].starts_with("px") {
                found.push(&v[start..k]);
                i = k + 2;
                continue;
            }
        }
        i = start + 1;
    }
    found.get(2).and_then(|s| parse_float(s)).unwrap_or(0.0)
}

pub fn is_card<M: Model>(m: &M, id: M::Id) -> bool {
    let d = decls(m, id);
    get(&d, "background").or_else(|| get(&d, "backgroundColor")).is_some()
        && px(get(&d, "borderRadius")).unwrap_or(0.0) >= 8.0
        && px(get(&d, "padding")).unwrap_or(0.0) > 0.0
        && !m.children(id).is_empty()
}

/// Steps 1.1–1.4 for one layer.
pub fn node<M: Model>(m: &M, n: M::Id, out: &mut Out<M::Id>) {
    let (layout, style) = (m.layout(n), m.style(n));
    let d = super::merge(&layout, &style);
    let dup: Vec<&str> = SIZE_KEYS.iter().copied().filter(|k| get(&layout, k).is_some() && get(&style, k).is_some()).collect();
    if !dup.is_empty() {
        out.add("size-duplicated", n, Some(dup.join(", ")), vec![]);
    }
    let mixed = shorthand_conflicts(&layout, &style);
    if !mixed.is_empty() {
        out.add("style-shorthand-conflict", n, Some(mixed.join(", ")), vec![]);
    }
    motion::checks(m, n, &d, out);

    if let Some(font) = primary_font(get(&d, "fontFamily"))
        && OVERUSED_FONTS.contains(&font.as_str())
    {
        out.add("overused-font", n, Some(font), vec![]);
    }
    let bg = format!("{} {}", lower(get(&d, "background")), lower(get(&d, "backgroundImage")));
    let clip = format!("{} {}", lower(get(&d, "backgroundClip")), lower(get(&d, "webkitBackgroundClip")));
    if bg.contains("gradient") && (clip.contains("text") || lower(get(&d, "color")) == "transparent") {
        out.add("gradient-text", n, None, vec![]);
    }
    if bg.contains("repeating-linear-gradient") || bg.contains("repeating-radial-gradient") {
        out.add("repeating-stripes-gradient", n, None, vec![]);
    }
    let sides: Vec<Side> = SIDES.iter().map(|s| side_of(&d, s)).collect();
    for (i, s) in sides.iter().enumerate() {
        let Some(color) = &s.color else { continue };
        if s.width < 2.0 || is_neutral(Some(color)) {
            continue;
        }
        let max_other = sides.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, o)| o.width).fold(f64::NEG_INFINITY, f64::max);
        if max_other <= 1.0 || s.width >= max_other * 2.0 {
            let rule = if px(get(&d, "borderRadius")).unwrap_or(0.0) > 0.0 { "border-accent-on-rounded" } else { "side-tab" };
            out.add(rule, n, Some(format!("{}px {color}", js_num(s.width))), vec![]);
            break;
        }
    }
    let border_w = px(get(&d, "borderWidth")).unwrap_or_else(|| parse_border(get(&d, "border")).width);
    if (0.5..=1.5).contains(&border_w) && shadow_blur(&lower(get(&d, "boxShadow"))) >= 24.0 {
        out.add("gpt-thin-border-wide-shadow", n, None, vec![]);
    }
    if !m.children(n).is_empty()
        && (get(&d, "background").or_else(|| get(&d, "backgroundColor")).or_else(|| get(&d, "border")).is_some())
        && let Some(pad) = px(get(&d, "padding"))
        && pad > 0.0
        && pad < 8.0
    {
        out.add("cramped-padding", n, Some(format!("{}px", js_num(pad))), vec![]);
    }
    if is_text(m, n) {
        text_rules(m, n, &d, out);
    }
}

fn text_rules<M: Model>(m: &M, n: M::Id, d: &Decls, out: &mut Out<M::Id>) {
    let text = m.text(n).unwrap_or_default();
    let font = px(get(d, "fontSize"));
    let tracking = tracking_em(get(d, "letterSpacing"), get(d, "fontSize"));
    if let Some(f) = font.filter(|f| *f < 12.0) {
        out.add("tiny-text", n, Some(format!("{}px", js_num(f))), vec![]);
    }
    if let Some(f) = font.filter(|f| *f >= 72.0) {
        out.add("oversized-h1", n, Some(format!("{}px", js_num(f))), vec![]);
    }
    if let Some(t) = tracking.filter(|t| *t <= -0.04) {
        out.add("extreme-negative-tracking", n, Some(format!("{}em", to_fixed(t, 3))), vec![]);
    }
    let body = font.is_none_or(|f| f < LARGE_TEXT_PX);
    if let Some(t) = tracking.filter(|t| *t >= 0.05)
        && body
    {
        out.add("wide-tracking", n, Some(format!("{}em", to_fixed(t, 3))), vec![]);
    }
    if lower(get(d, "textTransform")) == "uppercase" && len16(&text) > 40 && body {
        out.add("all-caps-body", n, None, vec![]);
    }
    if lower(get(d, "textAlign")) == "justify" {
        out.add("justified-text", n, None, vec![]);
    }
    let color = get(d, "color").map(Css::text);
    if let (Some(fg), Some(bg_str)) = (color.as_deref().and_then(parse_color), background(m, n))
        && let Some(bg) = parse_color(&bg_str)
    {
        let ratio = contrast(fg, bg);
        let large = font.is_some_and(|f| f >= LARGE_TEXT_PX);
        if color.as_deref().is_some_and(is_gray) && has_chroma(&bg_str) {
            out.add("gray-on-color", n, Some(format!("{}:1", to_fixed(ratio, 2))), vec![]);
        } else if ratio < if large { 3.0 } else { 4.5 } {
            out.add("low-contrast", n, Some(format!("{}:1", to_fixed(ratio, 2))), vec![]);
        }
    }
    let lowered = text.to_lowercase();
    if let Some(hit) = BUZZWORDS.iter().find(|p| lowered.contains(*p)) {
        out.add("marketing-buzzword", n, Some(format!("\"{hit}\"")), vec![]);
    }
}
