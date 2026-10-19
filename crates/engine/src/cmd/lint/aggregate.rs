//! The rules over a whole subtree (`lint.md` §7.2 step 3): `single-font`,
//! `flat-type-hierarchy`, `monotonous-spacing`, `nested-cards`, `em-dash-overuse` and
//! `numbered-section-markers`.
//!
//! Ported from Impeccable's detector, Apache-2.0, Copyright 2025 Paul Bakaus; modified by Scrim
//! and this fork (`design-rules/NOTICE.md`).

use super::checks::is_card;
use super::helpers::{js_num, js_round, primary_font, px};
use super::{Model, Out, ancestors, decls, get, is_text};

/// The em dashes: `—`, or `--` before a non-space.
fn em_dashes(t: &str) -> usize {
    let c: Vec<char> = t.chars().collect();
    let mut n = 0;
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\u{2014}' {
            n += 1;
            i += 1;
        } else if c[i] == '-' && c.get(i + 1) == Some(&'-') && c.get(i + 2).is_some_and(|x| !x.is_whitespace()) {
            n += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    n
}

/// `\b(0[1-9]|1[0-2])\b`: two digits standing alone (JavaScript's ASCII word boundary).
fn markers(t: &str) -> Vec<String> {
    let b = t.as_bytes();
    let word = |i: usize| b.get(i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < b.len() {
        let ok = matches!((b[i], b[i + 1]), (b'0', b'1'..=b'9') | (b'1', b'0'..=b'2'));
        if ok && (i == 0 || !word(i - 1)) && !word(i + 2) {
            out.push(String::from_utf8_lossy(&b[i..i + 2]).into_owned());
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}

pub fn checks<M: Model>(m: &M, target: M::Id, nodes: &[M::Id], out: &mut Out<M::Id>) {
    let texts: Vec<M::Id> = nodes.iter().copied().filter(|n| is_text(m, *n)).collect();
    let mut fonts: Vec<String> = Vec::new();
    for n in nodes {
        if let Some(f) = primary_font(get(&decls(m, *n), "fontFamily"))
            && !fonts.contains(&f)
        {
            fonts.push(f);
        }
    }
    if fonts.len() == 1 && texts.len() >= 2 {
        out.add("single-font", target, Some(fonts[0].clone()), vec![]);
    }
    let sizes: Vec<f64> = texts.iter().filter_map(|n| px(get(&decls(m, *n), "fontSize"))).collect();
    if texts.len() >= 3 && sizes.len() >= 2 {
        let max = sizes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = sizes.iter().copied().fold(f64::INFINITY, f64::min);
        if min > 0.0 && max / min < 1.25 {
            out.add("flat-type-hierarchy", target, Some(format!("{}\u{2013}{}px", js_num(min), js_num(max))), vec![]);
        }
    }
    let mut spaces = Vec::new();
    for n in nodes {
        let d = decls(m, *n);
        for key in ["gap", "padding"] {
            if let Some(v) = px(get(&d, key)).filter(|v| *v > 0.0) {
                spaces.push(js_round(v));
            }
        }
    }
    if spaces.len() >= 4 {
        let mut counts: Vec<(f64, usize)> = Vec::new();
        for s in &spaces {
            match counts.iter_mut().find(|(v, _)| v == s) {
                Some(e) => e.1 += 1,
                None => counts.push((*s, 1)),
            }
        }
        let max = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
        let pct = max as f64 / spaces.len() as f64;
        if pct > 0.6 && counts.len() <= 3 {
            // The first value with the top count (a stable sort by count).
            let dominant = counts.iter().find(|(_, c)| *c == max).map_or(0.0, |(v, _)| *v);
            out.add("monotonous-spacing", target, Some(format!("{}px {}%", js_num(dominant), js_num(js_round(pct * 100.0)))), vec![]);
        }
    }
    for &n in nodes {
        if is_card(m, n) && ancestors(m, n).into_iter().any(|a| is_card(m, a)) {
            out.add("nested-cards", n, None, vec![]);
        }
    }
    let all: Vec<String> = nodes.iter().map(|n| if is_text(m, *n) { m.text(*n).unwrap_or_default() } else { String::new() }).collect();
    let all = all.join("  ");
    let dashes = em_dashes(&all);
    if dashes >= 5 {
        out.add("em-dash-overuse", target, Some(format!("{dashes} em-dashes")), vec![]);
    }
    let mut marks = markers(&all);
    marks.sort();
    marks.dedup();
    if marks.len() >= 3 {
        let num = |s: &str| s.parse::<i32>().unwrap_or(-1);
        let sequential = marks.windows(2).filter(|w| num(&w[1]) == num(&w[0]) + 1).count();
        if sequential >= 2 {
            out.add("numbered-section-markers", target, Some(marks.iter().take(6).cloned().collect::<Vec<_>>().join(", ")), vec![]);
        }
    }
}
