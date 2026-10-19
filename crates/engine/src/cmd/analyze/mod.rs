//! `analyze_design` (Port 6): what a document or a subtree actually uses, measured against its
//! own tokens. It reports colours by frequency, type styles, spacing values, and repeated subtree
//! shapes (component candidates). The spec is `docs/scrim-transfer/specs/analyze-design.md`.
//!
//! It must never:
//! - **change anything.** It reads only.
//! - **give two answers.** It is deterministic: every list has a total order, and the same input
//!   gives the same JSON.
//!
//! It is generic over a [`Source`]. `figcraft.rs` reads FigCraft's document through the lint's
//! CSS view, and the tests replay Scrim's cases on Scrim's documents.
//!
//! This is Scrim's own design (no Impeccable code), except the colour parser it shares with the
//! lint.

mod figcraft;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use regex::Regex;
use serde_json::{Map, Value, json};

use super::lint::Css;
use super::lint::helpers::{Rgba, js_num, js_round, parse_color};
pub use figcraft::{DESCRIPTION, FigSource, specs};

const LIMIT_COLORS: usize = 100;
const LIMIT_TYPE: usize = 50;
const LIMIT_SPACING: usize = 100;
const LIMIT_REPEATED: usize = 20;
const IDS_PER_REPEAT: usize = 10;
const NEAR_DELTA_E: f64 = 3.0;
const NEAR_ALPHA: f64 = 0.02;

const COLOR_PROPS: &[&str] = &[
    "color",
    "background",
    "backgroundColor",
    "backgroundImage",
    "border",
    "borderColor",
    "borderTop",
    "borderRight",
    "borderBottom",
    "borderLeft",
    "borderTopColor",
    "borderRightColor",
    "borderBottomColor",
    "borderLeftColor",
    "outline",
    "outlineColor",
    "textDecorationColor",
    "boxShadow",
    "textShadow",
    "fill",
    "stroke",
];
const SPACING_PROPS: &[(&str, usize)] = &[
    ("gap", 0),
    ("rowGap", 0),
    ("columnGap", 0),
    ("padding", 1),
    ("paddingTop", 1),
    ("paddingRight", 1),
    ("paddingBottom", 1),
    ("paddingLeft", 1),
    ("margin", 2),
    ("marginTop", 2),
    ("marginRight", 2),
    ("marginBottom", 2),
    ("marginLeft", 2),
];
const TYPE_PROPS: [&str; 5] = ["fontFamily", "fontSize", "fontWeight", "lineHeight", "letterSpacing"];

/// The document's tokens.
#[derive(Clone, Debug, Default)]
pub struct Tokens {
    /// Colour tokens: name → value as written.
    pub colors: Vec<(String, String)>,
    /// Typography tokens: name → the properties they set.
    pub typography: Vec<(String, Vec<(String, Css)>)>,
    /// Spacing tokens: name → value.
    pub spacing: Vec<(String, Css)>,
}

/// What the analysis reads.
pub trait Source {
    type Id: Copy + Eq + Hash;
    fn roots(&self) -> Vec<Self::Id>;
    fn exists(&self, id: Self::Id) -> bool;
    fn parent(&self, id: Self::Id) -> Option<Self::Id>;
    fn children(&self, id: Self::Id) -> Vec<Self::Id>;
    fn name(&self, id: Self::Id) -> String;
    fn json_id(&self, id: Self::Id) -> Value;
    /// The node's type, lower case (`text`, `frame` …). Only `text` is special.
    fn kind(&self, id: Self::Id) -> String;
    /// A declaration: `style[key] ?? layout[key]`, an empty string absent.
    fn decl(&self, id: Self::Id, key: &str) -> Option<Css>;
    /// Inline SVG paints, in drawing order.
    fn svg_paints(&self, id: Self::Id) -> Vec<String>;
    /// A component master or an instance: its subtree is left out of `repeated`.
    fn is_component(&self, id: Self::Id) -> bool;
    fn tokens(&self) -> Tokens;
}

/// `rgbaOf`: `transparent`, `#rgba` and `#rrggbbaa`, then the lint's parser.
pub fn rgba_of(s: &str) -> Option<Rgba> {
    let s = s.trim().to_lowercase();
    if s == "transparent" {
        return Some(Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 });
    }
    if let Some(h) = s.strip_prefix('#')
        && (h.len() == 4 || h.len() == 8)
        && h.bytes().all(|b| b.is_ascii_hexdigit())
    {
        let d: Vec<f64> = if h.len() == 4 {
            h.chars().map(|c| f64::from(u8::from_str_radix(&format!("{c}{c}"), 16).unwrap_or(0))).collect()
        } else {
            (0..4).map(|i| f64::from(u8::from_str_radix(&h[i * 2..i * 2 + 2], 16).unwrap_or(0))).collect()
        };
        return Some(Rgba { r: d[0], g: d[1], b: d[2], a: d[3] / 255.0 });
    }
    parse_color(&s)
}

fn hex2(n: f64) -> String {
    format!("{:02x}", js_round(n).clamp(0.0, 255.0) as u8)
}

/// `#rrggbb`, or `#rrggbbaa` when not opaque.
pub fn hex_of(c: Rgba) -> String {
    let base = format!("#{}{}{}", hex2(c.r), hex2(c.g), hex2(c.b));
    if c.a >= 1.0 { base } else { format!("{base}{}", hex2(c.a * 255.0)) }
}

fn lab(c: Rgba) -> (f64, f64, f64) {
    let lin = |v: f64| {
        let v = v / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    let (r, g, b) = (lin(c.r), lin(c.g), lin(c.b));
    let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;
    let f = |t: f64| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    (116.0 * f(y) - 16.0, 500.0 * (f(x) - f(y)), 200.0 * (f(y) - f(z)))
}

/// CIE76 ΔE; alpha is ignored.
pub fn delta_e(p: Rgba, q: Rgba) -> f64 {
    let (a, b) = (lab(p), lab(q));
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}

fn round2(v: f64) -> f64 {
    js_round(v * 100.0) / 100.0
}

/// A number as JSON writes it: whole numbers without a fraction (`16`, not `16.0`).
fn num(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9.0e15 { json!(v as i64) } else { json!(v) }
}

/// analyze's `px`: `^(-?\d*\.?\d+)(px)?$`; without `px`, only 0 is a length.
fn px(v: &str) -> Option<f64> {
    let (num, unit) = match v.strip_suffix("px") {
        Some(n) => (n, true),
        None => (v, false),
    };
    let body = num.strip_prefix('-').unwrap_or(num);
    let ok = match body.split_once('.') {
        None => !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit()),
        Some((i, f)) => !f.is_empty() && i.bytes().all(|b| b.is_ascii_digit()) && f.bytes().all(|b| b.is_ascii_digit()),
    };
    if !ok {
        return None;
    }
    let n: f64 = num.parse().ok()?;
    (unit || n == 0.0).then_some(n)
}

/// `byCount(key)`: count descending, then the key in UTF-16 order.
fn by_count<T>(v: &mut [T], count: impl Fn(&T) -> usize, key: impl Fn(&T) -> String) {
    v.sort_by(|a, b| count(b).cmp(&count(a)).then_with(|| key(a).encode_utf16().cmp(key(b).encode_utf16())));
}

fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// `analyze_design` for the document, or for `scope`'s subtree.
pub fn analyze<S: Source>(s: &S, scope: Option<S::Id>) -> Value {
    let roots = scope.map_or_else(|| s.roots(), |id| vec![id]);
    // The values the scope inherits: its ancestors' declarations, root first.
    let mut top: [Option<String>; 5] = Default::default();
    if let Some(id) = scope {
        let mut chain = Vec::new();
        let mut cur = s.parent(id);
        while let Some(p) = cur {
            chain.push(p);
            cur = s.parent(p);
        }
        for n in chain.iter().rev() {
            for (i, p) in TYPE_PROPS.iter().enumerate() {
                if let Some(v) = s.decl(*n, p) {
                    top[i] = Some(v.text());
                }
            }
        }
    }
    let mut order = Vec::new();
    let mut inherited: HashMap<S::Id, [Option<String>; 5]> = HashMap::new();
    let mut stack: Vec<(S::Id, [Option<String>; 5])> = roots.iter().rev().map(|r| (*r, top.clone())).collect();
    while let Some((id, from)) = stack.pop() {
        if !s.exists(id) {
            continue;
        }
        order.push(id);
        let mut here = from;
        for (i, p) in TYPE_PROPS.iter().enumerate() {
            if let Some(v) = s.decl(id, p) {
                here[i] = Some(v.text());
            }
        }
        for c in s.children(id).into_iter().rev() {
            stack.push((c, here.clone()));
        }
        inherited.insert(id, here);
    }
    json!({
        "scope": scope.map_or(json!("(document)"), |id| s.json_id(id)),
        "nodes": order.len(),
        "colors": colors(s, &order),
        "typography": typography(s, &order, &inherited),
        "spacing": spacing(s, &order),
        "repeated": repeated(s, &order),
    })
}

struct Use {
    hex: String,
    rgba: Rgba,
    count: usize,
}

fn colors<S: Source>(s: &S, order: &[S::Id]) -> Value {
    let literal = Regex::new(r"(?i)#[0-9a-f]{3,8}(?-u:\b)|rgba?\([^)]*\)|(?-u:\b)(?:white|black|red|green|blue|gray|grey|transparent)(?-u:\b)").ok();
    let var = Regex::new(r"var\(\s*--([A-Za-z0-9_-]+)[^()]*(?:\([^()]*\)[^()]*)*\)").ok();
    let (Some(literal), Some(var)) = (literal, var) else { return json!({}) };
    let unparsed_prop = |p: &str| {
        let l = p.to_lowercase();
        l.contains("color") || l == "background" || l.starts_with("fill") || l.starts_with("stroke")
    };
    let mut uses: Vec<Use> = Vec::new();
    let mut refs: Vec<(String, usize)> = Vec::new();
    let mut unparsed = 0;
    let count = |lit: &str, uses: &mut Vec<Use>| {
        if let Some(c) = rgba_of(lit) {
            let hex = hex_of(c);
            match uses.iter_mut().find(|u| u.hex == hex) {
                Some(u) => u.count += 1,
                None => uses.push(Use { hex, rgba: c, count: 1 }),
            }
        }
    };
    for &id in order {
        for prop in COLOR_PROPS {
            let Some(Css::Str(raw)) = s.decl(id, prop) else { continue };
            let rest = var.replace_all(&raw, |c: &regex::Captures| {
                let name = c.get(1).map_or("", |m| m.as_str()).to_string();
                match refs.iter_mut().find(|(n, _)| *n == name) {
                    Some(r) => r.1 += 1,
                    None => refs.push((name, 1)),
                }
                " ".to_string()
            });
            let found: Vec<&str> = literal.find_iter(&rest).map(|m| m.as_str()).collect();
            for lit in &found {
                count(lit, &mut uses);
            }
            let t = rest.trim();
            if found.is_empty()
                && !t.is_empty()
                && !["none", "inherit", "initial", "unset", "currentcolor", "0"].contains(&t.to_lowercase().as_str())
                && unparsed_prop(prop)
            {
                unparsed += 1;
            }
        }
        for p in s.svg_paints(id) {
            count(&p, &mut uses);
        }
    }
    let mut tokens: Vec<(String, Rgba)> = s.tokens().colors.into_iter().filter_map(|(n, v)| Some((n, rgba_of(&v)?))).collect();
    tokens.sort_by(|a, b| utf16_cmp(&a.0, &b.0));
    let token_for = |c: Rgba| -> Option<(String, f64)> {
        let mut best: Option<(String, f64)> = None;
        for (name, t) in &tokens {
            if (t.a - c.a).abs() > NEAR_ALPHA {
                continue;
            }
            let d = if hex_of(*t) == hex_of(c) { 0.0 } else { delta_e(*t, c) };
            if d <= NEAR_DELTA_E && best.as_ref().is_none_or(|(_, b)| d < *b) {
                best = Some((name.clone(), d));
            }
        }
        best
    };
    by_count(&mut uses, |u| u.count, |u| u.hex.clone());
    let mut claimed: HashSet<String> = HashSet::new();
    let mut clusters: Vec<(Vec<&Use>, &Use)> = Vec::new();
    for seed in &uses {
        if claimed.contains(&seed.hex) || seed.rgba.a == 0.0 {
            continue;
        }
        let members: Vec<&Use> = uses
            .iter()
            .filter(|o| {
                !claimed.contains(&o.hex)
                    && o.rgba.a > 0.0
                    && (o.rgba.a - seed.rgba.a).abs() <= NEAR_ALPHA
                    && delta_e(o.rgba, seed.rgba) <= NEAR_DELTA_E
            })
            .collect();
        if members.len() < 2 {
            continue;
        }
        claimed.extend(members.iter().map(|m| m.hex.clone()));
        clusters.push((members, seed));
    }
    let mut near: Vec<(usize, String, Value)> = clusters
        .into_iter()
        .map(|(members, seed)| {
            let total: usize = members.iter().map(|m| m.count).sum();
            let max = members.iter().map(|m| delta_e(m.rgba, seed.rgba)).fold(f64::NEG_INFINITY, f64::max);
            let v = json!({
                "values": members.iter().map(|m| m.hex.clone()).collect::<Vec<_>>(),
                "count": total,
                "suggested": seed.hex,
                "token": token_for(seed.rgba).map(|(n, _)| n),
                "maxDeltaE": num(round2(max)),
            });
            (total, seed.hex.clone(), v)
        })
        .collect();
    by_count(&mut near, |n| n.0, |n| n.1.clone());
    by_count(&mut refs, |r| r.1, |r| r.0.clone());
    json!({
        "distinct": uses.len(),
        "values": uses.iter().take(LIMIT_COLORS).map(|u| json!({"value": u.hex, "count": u.count, "token": token_for(u.rgba).map(|(n, d)| json!({"name": n, "deltaE": num(round2(d))}))})).collect::<Vec<_>>(),
        "nearDuplicates": near.into_iter().map(|n| n.2).collect::<Vec<_>>(),
        "tokenRefs": refs.into_iter().map(|(name, count)| json!({"name": name, "count": count})).collect::<Vec<_>>(),
        "unparsed": unparsed,
    })
}

fn same_value(style: Option<&String>, token: Option<&Css>) -> bool {
    let Some(t) = token else { return true };
    let Some(s) = style else { return false };
    let t = t.text();
    match (px(s), px(&t)) {
        (Some(a), Some(b)) => a == b,
        _ => s.trim() == t.trim(),
    }
}

fn typography<S: Source>(s: &S, order: &[S::Id], inherited: &HashMap<S::Id, [Option<String>; 5]>) -> Value {
    let mut groups: Vec<([Option<String>; 5], usize)> = Vec::new();
    let mut text_nodes = 0;
    for id in order {
        if s.kind(*id) != "text" {
            continue;
        }
        text_nodes += 1;
        let t = inherited.get(id).cloned().unwrap_or_default();
        match groups.iter_mut().find(|(g, _)| *g == t) {
            Some(g) => g.1 += 1,
            None => groups.push((t, 1)),
        }
    }
    let style_json = |g: &[Option<String>; 5], count: usize, token: Option<String>| {
        let mut m = Map::new();
        for (i, p) in TYPE_PROPS.iter().enumerate() {
            m.insert((*p).to_string(), g[i].clone().map_or(Value::Null, Value::String));
        }
        m.insert("count".into(), json!(count));
        m.insert("token".into(), token.map_or(Value::Null, Value::String));
        m
    };
    // The tie-break key serialises the style in key order, its token still null.
    let key = |g: &([Option<String>; 5], usize)| {
        let parts: Vec<String> = TYPE_PROPS
            .iter()
            .enumerate()
            .map(|(i, p)| format!("\"{p}\":{}", g.0[i].as_ref().map_or("null".to_string(), |v| Value::String(v.clone()).to_string())))
            .collect();
        format!("{{{},\"count\":{},\"token\":null}}", parts.join(","), g.1)
    };
    by_count(&mut groups, |g| g.1, key);
    let mut tokens = s.tokens().typography;
    tokens.sort_by(|a, b| utf16_cmp(&a.0, &b.0));
    let styles: Vec<Value> = groups
        .iter()
        .take(LIMIT_TYPE)
        .map(|(g, count)| {
            let hit = tokens.iter().find(|(_, tok)| {
                !tok.is_empty()
                    && TYPE_PROPS.iter().enumerate().all(|(i, p)| same_value(g[i].as_ref(), tok.iter().find(|(k, _)| k == p).map(|(_, v)| v)))
            });
            Value::Object(style_json(g, *count, hit.map(|(n, _)| n.clone())))
        })
        .collect();
    json!({"textNodes": text_nodes, "distinct": groups.len(), "styles": styles})
}

fn spacing<S: Source>(s: &S, order: &[S::Id]) -> Value {
    // value → (count, [gap, padding, margin])
    let mut uses: Vec<(f64, usize, [usize; 3])> = Vec::new();
    let mut unparsed = 0;
    let add = |v: f64, family: usize, uses: &mut Vec<(f64, usize, [usize; 3])>| match uses.iter_mut().find(|u| u.0 == v) {
        Some(u) => {
            u.1 += 1;
            u.2[family] += 1;
        }
        None => {
            let mut f = [0; 3];
            f[family] = 1;
            uses.push((v, 1, f));
        }
    };
    for &id in order {
        for (prop, family) in SPACING_PROPS {
            match s.decl(id, prop) {
                None => {}
                Some(Css::Num(n)) => {
                    if n != 0.0 {
                        add(n, *family, &mut uses);
                    }
                }
                Some(Css::Str(raw)) => {
                    for part in raw.split_whitespace() {
                        match px(part) {
                            None if part != "auto" => unparsed += 1,
                            None => {}
                            Some(0.0) => {}
                            Some(v) => add(v, *family, &mut uses),
                        }
                    }
                }
            }
        }
    }
    let mut tokens: Vec<(String, f64)> = s
        .tokens()
        .spacing
        .into_iter()
        .filter_map(|(n, v)| {
            let v = match v {
                Css::Num(x) => Some(x),
                Css::Str(t) => px(t.trim()),
            }?;
            Some((n, v))
        })
        .collect();
    tokens.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| utf16_cmp(&a.0, &b.0)));
    by_count(&mut uses, |u| u.1, |u| format!("{:0>12}", js_num(u.0)));
    // (value, count, [gap, padding, margin], on scale, token)
    type Entry = (f64, usize, [usize; 3], bool, Option<String>);
    let entries: Vec<Entry> = uses
        .iter()
        .map(|(v, c, f)| {
            let a = v.abs();
            let hit = tokens.iter().find(|(_, t)| *t == a).map(|(n, _)| n.clone());
            let on = if tokens.is_empty() { a % 4.0 == 0.0 || a == 1.0 || a == 2.0 } else { hit.is_some() };
            (*v, *c, *f, on, hit)
        })
        .collect();
    let mut off: Vec<f64> = entries.iter().filter(|e| !e.3).map(|e| e.0).collect();
    off.sort_by(f64::total_cmp);
    let scale = if tokens.is_empty() {
        "4 px grid (no spacing tokens; 1 and 2 allowed)".to_string()
    } else {
        format!("spacing tokens ({})", tokens.iter().map(|t| js_num(t.1)).collect::<Vec<_>>().join(", "))
    };
    json!({
        "scale": scale,
        "distinct": entries.len(),
        "values": entries.iter().take(LIMIT_SPACING).map(|(v, c, f, on, tok)| json!({"value": num(*v), "count": c, "properties": {"gap": f[0], "padding": f[1], "margin": f[2]}, "onScale": on, "token": tok})).collect::<Vec<_>>(),
        "offScale": off.into_iter().map(num).collect::<Vec<_>>(),
        "unparsed": unparsed,
    })
}

struct Shape {
    sig: usize,
    size: usize,
    head: String,
    label: String,
}

fn repeated<S: Source>(s: &S, order: &[S::Id]) -> Value {
    let mut in_component: HashSet<S::Id> = HashSet::new();
    for &id in order {
        if s.is_component(id) && !in_component.contains(&id) {
            let mut stack = vec![id];
            while let Some(n) = stack.pop() {
                if in_component.insert(n) {
                    stack.extend(s.children(n));
                }
            }
        }
    }
    let mut interned: HashMap<String, usize> = HashMap::new();
    let mut shapes: HashMap<S::Id, Shape> = HashMap::new();
    for &id in order.iter().rev() {
        let kids: Vec<&Shape> = s.children(id).iter().filter_map(|c| shapes.get(c)).collect();
        let display = s.decl(id, "display").map(|d| d.text());
        let head = match &display {
            None => s.kind(id),
            Some(d) if d == "flex" || d == "inline-flex" => {
                format!("{}:{d}-{}", s.kind(id), s.decl(id, "flexDirection").map_or("row".to_string(), |v| v.text()))
            }
            Some(d) => format!("{}:{d}", s.kind(id)),
        };
        let key = format!("{head}({})", kids.iter().map(|k| k.sig.to_string()).collect::<Vec<_>>().join(","));
        let next = interned.len();
        let sig = *interned.entry(key).or_insert(next);
        let size = 1 + kids.iter().map(|k| k.size).sum::<usize>();
        let label =
            if kids.is_empty() { head.clone() } else { format!("{head}({})", kids.iter().map(|k| k.head.clone()).collect::<Vec<_>>().join(", ")) };
        shapes.insert(id, Shape { sig, size, head, label });
    }
    let mut groups: Vec<(usize, Vec<S::Id>)> = Vec::new();
    for &id in order {
        let Some(sh) = shapes.get(&id) else { continue };
        if in_component.contains(&id) || sh.size < 3 {
            continue;
        }
        match groups.iter_mut().find(|g| g.0 == sh.sig) {
            Some(g) => g.1.push(id),
            None => groups.push((sh.sig, vec![id])),
        }
    }
    groups.retain(|g| g.1.len() >= 2);
    let member_of: HashMap<S::Id, usize> = groups.iter().flat_map(|(sig, ids)| ids.iter().map(move |i| (*i, *sig))).collect();
    let inside = |id: S::Id, sig: usize| {
        let mut p = s.parent(id);
        while let Some(x) = p {
            if member_of.get(&x).is_some_and(|o| *o != sig) {
                return true;
            }
            p = s.parent(x);
        }
        false
    };
    let rank: HashMap<S::Id, usize> = order.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut out: Vec<(usize, usize, usize, Value)> = groups
        .iter()
        .filter(|(sig, ids)| !ids.iter().all(|i| inside(*i, *sig)))
        .filter_map(|(_, members)| {
            let first = shapes.get(members.first()?)?;
            let mut names: Vec<String> = Vec::new();
            for m in members {
                let n = s.name(*m);
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            names.truncate(3);
            let label = if first.label.encode_utf16().count() > 160 {
                let units: Vec<u16> = first.label.encode_utf16().take(157).collect();
                format!("{}\u{2026}", String::from_utf16_lossy(&units))
            } else {
                first.label.clone()
            };
            let v = json!({
                "count": members.len(),
                "nodesEach": first.size,
                "ids": members.iter().take(IDS_PER_REPEAT).map(|m| s.json_id(*m)).collect::<Vec<_>>(),
                "names": names,
                "shape": label,
            });
            Some((members.len(), first.size, rank.get(&members[0]).copied().unwrap_or(usize::MAX), v))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    Value::Array(out.into_iter().take(LIMIT_REPEATED).map(|o| o.3).collect())
}

#[cfg(test)]
#[path = "scrim_cases_tests.rs"]
mod scrim_cases_tests;

#[cfg(test)]
#[path = "figcraft_tests.rs"]
mod figcraft_tests;
