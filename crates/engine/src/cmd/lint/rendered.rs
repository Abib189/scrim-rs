//! The rules that need boxes (`lint.md` §7.2 step 2, §7.4; `lint-rules.md`): `text-overflow`,
//! `clipped-overflow-container`, `line-length`, `body-text-viewport-edge`, `hero-eyebrow-chip`,
//! `text-occluded` and `root-overlap`. A layer without a box is skipped, never guessed.
//!
//! Ported from Impeccable's detector (the rules it runs in a live browser), Apache-2.0,
//! Copyright 2025 Paul Bakaus; modified by Scrim (measured rects, its thresholds, and its own
//! `text-occluded` and `root-overlap`) and this fork (`design-rules/NOTICE.md`).

use std::collections::HashMap;

use super::helpers::{contrast, js_num, js_round, len16, lower, parse_color, parse_float, px, slice16, tracking_em};
use super::{Css, Finding, Kind, Model, Out, Rect, ancestors, decls, get, is_text, rules, subtree};

const BODY_MAX_PX: f64 = 24.0;
const EDGE_PX: f64 = 16.0;
const MIN_VISIBLE: f64 = 0.6;
const MAX_CPL: f64 = 80.0;
const OCCLUDED_SHARE: f64 = 0.25;
const CAMOUFLAGE_SHARE: f64 = 0.1;
const CAMOUFLAGE_RATIO: f64 = 1.5;

/// `Math.round(x)` as JavaScript prints it.
fn r(x: f64) -> String {
    js_num(js_round(x))
}

/// The node's own value of `key`, else the nearest ancestor's.
fn inherited<M: Model>(m: &M, id: M::Id, key: &str) -> Option<Css> {
    std::iter::once(id).chain(ancestors(m, id)).find_map(|n| get(&decls(m, n), key).cloned())
}

pub fn font_size_px<M: Model>(m: &M, id: M::Id) -> f64 {
    let v = inherited(m, id, "fontSize");
    if let Some(p) = px(v.as_ref()) {
        return p;
    }
    if let Some(Css::Str(s)) = &v {
        let t = s.trim();
        let num = t.strip_suffix("rem").or_else(|| t.strip_suffix("em"));
        if let Some(n) = num.filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit() || b == b'.')) {
            return parse_float(n).unwrap_or(f64::NAN) * 16.0;
        }
    }
    16.0
}

fn line_height_px<M: Model>(m: &M, id: M::Id) -> f64 {
    let font = font_size_px(m, id);
    match inherited(m, id, "lineHeight") {
        Some(Css::Num(v)) => {
            if v < 5.0 {
                v * font
            } else {
                v
            }
        }
        Some(Css::Str(s)) => {
            let t = s.trim();
            let f = || parse_float(t).unwrap_or(f64::NAN);
            if t.ends_with("px") {
                f()
            } else if t.ends_with('%') {
                f() / 100.0 * font
            } else if t.ends_with("em") || (!t.is_empty() && t.bytes().all(|b| b.is_ascii_digit() || b == b'.')) {
                f() * font
            } else {
                1.2 * font
            }
        }
        None => 1.2 * font,
    }
}

const CLIPS: &[&str] = &["hidden", "clip", "auto", "scroll"];

/// `style ?? layout` for one key.
fn decl<M: Model>(m: &M, id: M::Id, key: &str) -> Option<Css> {
    let style = m.style(id);
    match get(&style, key) {
        Some(v) => Some(v.clone()),
        None => get(&m.layout(id), key).cloned(),
    }
}

/// Whether a layer clips its overflow, per axis.
fn clips<M: Model>(m: &M, id: M::Id) -> (bool, bool) {
    let both = decl(m, id, "overflow").map(|v| v.text()).unwrap_or_default();
    let parts: Vec<&str> = both.split_whitespace().collect();
    let ox = parts.first().copied().unwrap_or("");
    let oy = parts.get(1).copied().unwrap_or(ox);
    let x = decl(m, id, "overflowX").map_or(ox.to_string(), |v| v.text());
    let y = decl(m, id, "overflowY").map_or(oy.to_string(), |v| v.text());
    (CLIPS.contains(&x.as_str()), CLIPS.contains(&y.as_str()))
}

fn first_px(v: Option<Css>) -> Option<f64> {
    let v = v?;
    if let Css::Num(n) = v {
        return Some(n);
    }
    v.text().split_whitespace().find_map(|t| px(Some(&Css::Str(t.to_string()))))
}

/// The border widths `[top, right, bottom, left]` that the padding box leaves out.
fn borders<M: Model>(m: &M, id: M::Id) -> [f64; 4] {
    let sides = ["Top", "Right", "Bottom", "Left"];
    let width_keys: Vec<String> = sides.iter().map(|s| format!("border{s}Width")).collect();
    let bw = decl(m, id, "borderWidth");
    let mut w = [0.0; 4];
    if bw.is_some() || width_keys.iter().any(|k| decl(m, id, k).is_some()) {
        if let Some(v) = bw {
            let vals: Vec<f64> = match &v {
                Css::Num(n) => vec![*n],
                Css::Str(s) => s.split_whitespace().map(|t| px(Some(&Css::Str(t.into()))).unwrap_or(0.0)).collect(),
            };
            w = match vals.as_slice() {
                [a] => [*a; 4],
                [a, b] => [*a, *b, *a, *b],
                [a, b, c] => [*a, *b, *c, *b],
                [a, b, c, d, ..] => [*a, *b, *c, *d],
                [] => [0.0; 4],
            };
        }
        for (i, k) in width_keys.iter().enumerate() {
            if let Some(v) = decl(m, id, k) {
                w[i] = px(Some(&v)).unwrap_or(0.0);
            }
        }
    } else {
        w = [first_px(decl(m, id, "border")).unwrap_or(0.0); 4];
        for (i, s) in sides.iter().enumerate() {
            if let Some(v) = first_px(decl(m, id, &format!("border{s}"))) {
                w[i] = v;
            }
        }
    }
    w
}

#[derive(Clone, Copy)]
struct Bounds {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

/// The part of `id`'s box its clipping ancestors leave visible (`None`: nothing).
fn visible<M: Model>(m: &M, id: M::Id, cache: &mut HashMap<M::Id, Bounds>) -> Option<Rect> {
    let r = m.rect(id)?;
    let b = clip_for(m, m.parent(id), cache);
    let (x1, y1, x2, y2) = (r.x.max(b.x1), r.y.max(b.y1), r.right().min(b.x2), r.bottom().min(b.y2));
    (x2 > x1 && y2 > y1).then_some(Rect { x: x1, y: y1, w: x2 - x1, h: y2 - y1 })
}

fn clip_for<M: Model>(m: &M, id: Option<M::Id>, cache: &mut HashMap<M::Id, Bounds>) -> Bounds {
    let open = Bounds { x1: f64::NEG_INFINITY, y1: f64::NEG_INFINITY, x2: f64::INFINITY, y2: f64::INFINITY };
    // The chain from the top down, so each ancestor's bounds build on its parent's.
    let mut chain = Vec::new();
    let mut cur = id;
    while let Some(c) = cur {
        if cache.contains_key(&c) || chain.len() > 100_000 {
            break;
        }
        chain.push(c);
        cur = m.parent(c);
    }
    let mut b = cur.and_then(|c| cache.get(&c).copied()).unwrap_or(open);
    for &c in chain.iter().rev() {
        if let Some(r) = m.rect(c) {
            let (ax, ay) = clips(m, c);
            if ax || ay {
                let [t, rt, bt, l] = borders(m, c);
                let pb = Rect { x: r.x + l, y: r.y + t, w: r.w - l - rt, h: r.h - t - bt };
                if ax {
                    b.x1 = b.x1.max(pb.x);
                    b.x2 = b.x2.min(pb.right());
                }
                if ay {
                    b.y1 = b.y1.max(pb.y);
                    b.y2 = b.y2.min(pb.bottom());
                }
            }
        }
        cache.insert(c, b);
    }
    b
}

fn clipping_ancestor<M: Model>(m: &M, id: M::Id) -> Option<M::Id> {
    ancestors(m, id).into_iter().find(|a| {
        let (x, y) = clips(m, *a);
        x || y
    })
}

/// `[layer, z, index]`: CSS paint order among siblings.
fn paint_rank<M: Model>(m: &M, id: M::Id, index: usize) -> (i32, f64, usize) {
    let d = decls(m, id);
    let z = px(get(&d, "zIndex"));
    let pos = lower(get(&d, "position"));
    let positioned = (!pos.is_empty() && pos != "static") || get(&d, "transform").is_some();
    match z {
        Some(z) if z < 0.0 => (-1, z, index),
        Some(z) if z > 0.0 => (2, z, index),
        _ => (i32::from(positioned), 0.0, index),
    }
}

fn above(a: (i32, f64, usize), b: (i32, f64, usize)) -> bool {
    if a.0 != b.0 {
        a.0 > b.0
    } else if a.1 != b.1 {
        a.1 > b.1
    } else {
        a.2 > b.2
    }
}

/// The layer's own opaque fill (alpha ≥ 0.9).
fn opaque_fill<M: Model>(m: &M, id: M::Id) -> Option<String> {
    let d = decls(m, id);
    let bg = get(&d, "backgroundColor").or_else(|| get(&d, "background"))?;
    parse_color(&bg.text()).filter(|c| c.a >= 0.9).map(|_| bg.text())
}

/// The share of `t` covered by `covers`, on a 16 × 8 grid of cell centres.
fn coverage(t: &Rect, covers: &[Rect]) -> f64 {
    if covers.is_empty() || t.w <= 0.0 || t.h <= 0.0 {
        return 0.0;
    }
    let mut hit = 0;
    for i in 0..16 {
        let x = t.x + (f64::from(i) + 0.5) / 16.0 * t.w;
        for j in 0..8 {
            let y = t.y + (f64::from(j) + 0.5) / 8.0 * t.h;
            if covers.iter().any(|c| x >= c.x && x < c.right() && y >= c.y && y < c.bottom()) {
                hit += 1;
            }
        }
    }
    f64::from(hit) / 128.0
}

fn short(t: &str) -> String {
    if len16(t) > 18 { format!("{}\u{2026}", slice16(t, 18)) } else { t.to_string() }
}

/// Step 2: the rendered checks over the subtree, in pre-order.
pub fn checks<M: Model>(m: &M, nodes: &[M::Id], out: &mut Out<M::Id>) {
    let artboard = out.root;
    let board = m.rect(artboard);
    let mut cache = HashMap::new();
    for &n in nodes {
        let Some(rect) = m.rect(n) else { continue };
        let parent = m.parent(n);
        let parent_rect = parent.and_then(|p| m.rect(p));
        let d = decls(m, n);
        if let (Some(p), Some(pr)) = (parent, parent_rect)
            && !["absolute", "fixed"].contains(&lower(get(&d, "position")).as_str())
        {
            let spill = (pr.x - rect.x).max(rect.right() - pr.right());
            if spill > 1.0 {
                out.add("text-overflow", n, Some(format!("spills {}px past {}", r(spill), m.name(p))), vec![]);
            }
        }
        if matches!(m.kind(n), Kind::Text | Kind::Image) && rect.w > 0.0 && rect.h > 0.0 {
            let share = visible(m, n, &mut cache).map_or(0.0, |v| v.w * v.h / (rect.w * rect.h));
            if share < MIN_VISIBLE {
                let clip = clipping_ancestor(m, n);
                let detail = format!("{}% visible{}", r(share * 100.0), clip.map_or(String::new(), |c| format!(" in {}", m.name(c))));
                out.add("clipped-overflow-container", n, Some(detail), clip.into_iter().collect());
            }
        }
        if !is_text(m, n) {
            continue;
        }
        let font = font_size_px(m, n);
        let text = m.text(n).unwrap_or_default();
        let len = len16(&text) as f64;
        if font < BODY_MAX_PX {
            let lines = js_round(rect.h / line_height_px(m, n)).max(1.0);
            let cpl = len / lines;
            if cpl > MAX_CPL {
                out.add("line-length", n, Some(format!("~{} chars/line", r(cpl))), vec![]);
            }
        }
        if let Some(b) = board
            && font < BODY_MAX_PX
            && len >= 40.0
            && n != artboard
        {
            let gap = (rect.x - b.x).min(b.right() - rect.right());
            if gap < EDGE_PX {
                out.add("body-text-viewport-edge", n, Some(format!("{}px from the edge", js_num(js_round(gap).max(0.0)))), vec![]);
            }
        }
        if let Some(b) = board
            && font <= 14.0
        {
            eyebrow(m, n, &text, font, rect, b, nodes, out);
        }
        occluded(m, n, rect, out);
    }
}

#[allow(clippy::too_many_arguments)]
fn eyebrow<M: Model>(m: &M, n: M::Id, text: &str, font: f64, rect: Rect, board: Rect, nodes: &[M::Id], out: &mut Out<M::Id>) {
    let holder = match m.parent(n) {
        Some(p) if m.children(p).len() == 1 => p,
        _ => n,
    };
    let hd = decls(m, holder);
    let pill = if opaque_fill(m, holder).is_some() || get(&hd, "border").or_else(|| get(&hd, "borderWidth")).is_some() {
        px(get(&hd, "borderRadius")).unwrap_or(0.0) >= 8.0 || lower(get(&hd, "borderRadius")).contains('%')
    } else {
        false
    };
    let tracked = tracking_em(inherited(m, n, "letterSpacing").as_ref(), Some(&Css::Num(font))).unwrap_or(0.0) >= 0.05;
    let caps = lower(inherited(m, n, "textTransform").as_ref()) == "uppercase"
        || (text == text.to_uppercase() && text.bytes().any(|b| b.is_ascii_uppercase()));
    if !(tracked || caps || pill) {
        return;
    }
    let label = m.rect(holder).unwrap_or(rect);
    for &h in nodes {
        let Some(hr) = m.rect(h) else { continue };
        if h == n || !is_text(m, h) || font_size_px(m, h) < 48.0 || hr.y - board.y > 1000.0 {
            continue;
        }
        let gap = hr.y - label.bottom();
        if (0.0..=40.0).contains(&gap) && label.x < hr.right() && hr.x < label.right() {
            let what = if pill { "pill" } else { "eyebrow" };
            out.add("hero-eyebrow-chip", n, Some(format!("{what} above \"{}\"", short(&m.text(h).unwrap_or_default()))), vec![h]);
            return;
        }
    }
}

fn occluded<M: Model>(m: &M, n: M::Id, rect: Rect, out: &mut Out<M::Id>) {
    let ink = parse_color(&inherited(m, n, "color").map_or("#000000".to_string(), |c| c.text()));
    let mut best: Option<(f64, M::Id, &str)> = None;
    let mut a = n;
    while let Some(p) = m.parent(a) {
        let kids = m.children(p);
        let rank_a = paint_rank(m, a, kids.iter().position(|k| *k == a).unwrap_or(0));
        for (i, &s) in kids.iter().enumerate() {
            if s == a {
                continue;
            }
            let sib_above = above(paint_rank(m, s, i), rank_a);
            let (mut over, mut camo) = (Vec::new(), Vec::new());
            for o in subtree(m, s) {
                let Some(r) = m.rect(o) else { continue };
                let fill = opaque_fill(m, o).or_else(|| (m.kind(o) == Kind::Image).then(|| "#000000".to_string()));
                let Some(fill) = fill else { continue };
                if !r.overlaps(&rect) {
                    continue;
                }
                if sib_above {
                    over.push(r);
                } else if let (Some(ink), Some(f)) = (ink, parse_color(&fill))
                    && m.kind(o) != Kind::Image
                    && contrast(ink, f) < CAMOUFLAGE_RATIO
                {
                    camo.push(r);
                }
            }
            for (list, how, min) in [(&over, "covered by", OCCLUDED_SHARE), (&camo, "lost against", CAMOUFLAGE_SHARE)] {
                let share = coverage(&rect, list);
                if share >= min && best.is_none_or(|(b, ..)| share > b) {
                    best = Some((share, s, how));
                }
            }
        }
        a = p;
    }
    if let Some((share, by, how)) = best {
        out.add("text-occluded", n, Some(format!("{}% {how} {}", r(share * 100.0), m.name(by))), vec![by]);
    }
}

/// `root-overlap`: each pair of roots in a group whose boxes overlap, on the earlier root.
pub fn root_overlaps<M: Model>(m: &M, roots: &[M::Id]) -> Vec<Finding<M::Id>> {
    let items: Vec<(M::Id, Rect)> = roots.iter().filter_map(|r| Some((*r, m.rect(*r)?))).collect();
    let mut out = Vec::new();
    for (i, (a, ra)) in items.iter().enumerate() {
        for (b, rb) in &items[i + 1..] {
            if ra.overlaps(rb) {
                let w = js_round(ra.right().min(rb.right()) - ra.x.max(rb.x));
                let h = js_round(ra.bottom().min(rb.bottom()) - ra.y.max(rb.y));
                out.push(Finding {
                    rule: rules::rule("root-overlap"),
                    node: *a,
                    root: *a,
                    detail: Some(format!("overlaps \"{}\" by {}\u{d7}{}", m.name(*b), js_num(w), js_num(h))),
                    related: vec![*b],
                });
            }
        }
    }
    out
}
