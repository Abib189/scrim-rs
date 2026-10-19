//! The Motion rules (`lint-rules.md` › Motion): `bounce-easing`, `layout-transition` and
//! `image-hover-transform`, over every declaration map a layer has (the base map, each state,
//! each media block).
//!
//! Ported from Impeccable's detector, Apache-2.0, Copyright 2025 Paul Bakaus; modified by Scrim
//! (overshooting `cubic-bezier()`, non-monotonic `linear()`, the motion model) and this fork
//! (`design-rules/NOTICE.md`).

use super::helpers::parse_float;
use super::{Css, Decls, Kind, Model, Out, get};

const LAYOUT_PROPS: &[&str] = &[
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "gap",
    "row-gap",
    "column-gap",
    "top",
    "right",
    "bottom",
    "left",
    "inset",
    "flex",
    "flex-grow",
    "flex-shrink",
    "flex-basis",
    "grid-template-columns",
    "grid-template-rows",
    "grid-template",
    "font-size",
    "line-height",
    "letter-spacing",
    "border-width",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];

const T_KEYS: [&str; 5] = ["transitionProperty", "transitionDuration", "transitionTimingFunction", "transitionDelay", "transitionBehavior"];
const T_INITIAL: [&str; 5] = ["all", "0s", "ease", "0s", "normal"];
const A_KEYS: [&str; 8] = [
    "animationName",
    "animationDuration",
    "animationTimingFunction",
    "animationDelay",
    "animationIterationCount",
    "animationDirection",
    "animationFillMode",
    "animationPlayState",
];
const A_INITIAL: [&str; 8] = ["none", "0s", "ease", "0s", "1", "normal", "none", "running"];
const EASING_WORDS: &[&str] = &["linear", "ease", "ease-in", "ease-out", "ease-in-out", "step-start", "step-end"];

/// Split on commas outside parentheses.
pub fn items(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in v.chars() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth = depth.saturating_sub(1);
        }
        if ch == ',' && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// Tokens split on spaces outside parentheses.
fn tokens(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for ch in v.trim().chars() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth = depth.saturating_sub(1);
        }
        if ch.is_whitespace() && depth == 0 {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn is_time(t: &str) -> bool {
    let l = t.to_lowercase();
    let num = l.strip_suffix("ms").or_else(|| l.strip_suffix('s'));
    let Some(num) = num else { return false };
    let num = num.strip_prefix(['+', '-']).unwrap_or(num);
    match num.split_once('.') {
        None => !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()),
        Some((i, f)) => {
            (!i.is_empty() || !f.is_empty())
                && i.bytes().all(|b| b.is_ascii_digit())
                && f.bytes().all(|b| b.is_ascii_digit())
                && (!i.is_empty() || !f.is_empty())
        }
    }
}

fn is_easing(t: &str) -> bool {
    let l = t.to_lowercase();
    EASING_WORDS.contains(&l.as_str()) || l.starts_with("cubic-bezier(") || l.starts_with("steps(") || l.starts_with("linear(")
}

fn is_ident(t: &str) -> bool {
    let s = t.strip_prefix('-').unwrap_or(t);
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_') && c.all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '-')
}

fn transition_item(item: &str) -> Option<Vec<String>> {
    let mut out: [Option<String>; 5] = Default::default();
    let mut times = Vec::new();
    for t in tokens(item) {
        let l = t.to_lowercase();
        if is_time(&t) {
            times.push(t);
        } else if is_easing(&t) && out[2].is_none() {
            out[2] = Some(t);
        } else if (l == "normal" || l == "allow-discrete") && out[4].is_none() {
            out[4] = Some(t);
        } else if (is_ident(&t) || l == "none") && out[0].is_none() {
            out[0] = Some(t);
        } else {
            return None;
        }
    }
    if times.len() > 2 {
        return None;
    }
    let mut times = times.into_iter();
    out[1] = times.next();
    out[3] = times.next();
    Some(out.into_iter().zip(T_INITIAL).map(|(v, d)| v.unwrap_or_else(|| d.to_string())).collect())
}

fn animation_item(item: &str) -> Option<Vec<String>> {
    let mut out: [Option<String>; 8] = Default::default();
    let mut times = Vec::new();
    for t in tokens(item) {
        let l = t.to_lowercase();
        let count = l == "infinite" || (!t.is_empty() && parse_float(&t).is_some() && t.bytes().all(|b| b.is_ascii_digit() || b == b'.'));
        if is_time(&t) {
            times.push(t);
        } else if is_easing(&t) && out[2].is_none() {
            out[2] = Some(t);
        } else if count && out[4].is_none() {
            out[4] = Some(t);
        } else if ["normal", "reverse", "alternate", "alternate-reverse"].contains(&l.as_str()) && out[5].is_none() {
            out[5] = Some(t);
        } else if ["none", "forwards", "backwards", "both"].contains(&l.as_str()) && out[6].is_none() {
            out[6] = Some(t);
        } else if ["running", "paused"].contains(&l.as_str()) && out[7].is_none() {
            out[7] = Some(t);
        } else if (is_ident(&t) || (t.len() >= 2 && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))))
            && out[0].is_none()
        {
            out[0] = Some(t);
        } else {
            return None;
        }
    }
    if times.len() > 2 {
        return None;
    }
    let mut times = times.into_iter();
    out[1] = times.next();
    out[3] = times.next();
    Some(out.into_iter().zip(A_INITIAL).map(|(v, d)| v.unwrap_or_else(|| d.to_string())).collect())
}

/// A shorthand's longhands, or `None` when it is kept as written (a number, `var()`, a token
/// that can't be read).
fn expand_list(v: &Css, keys: &[&str], item: fn(&str) -> Option<Vec<String>>) -> Option<Vec<(String, String)>> {
    let Css::Str(s) = v else { return None };
    let s = s.trim();
    if s.is_empty() || s.to_lowercase().contains("var(") {
        return None;
    }
    if ["inherit", "initial", "unset", "revert", "revert-layer"].contains(&s.to_lowercase().as_str()) {
        return Some(keys.iter().map(|k| (k.to_string(), s.to_string())).collect());
    }
    let parsed: Option<Vec<Vec<String>>> = items(s).iter().map(|i| item(i)).collect();
    let parsed = parsed?;
    Some(keys.iter().enumerate().map(|(k, key)| (key.to_string(), parsed.iter().map(|p| p[k].clone()).collect::<Vec<_>>().join(", "))).collect())
}

/// The map with `transition` and `animation` read through their longhands when they can be.
fn expanded(m: &Decls) -> Decls {
    let mut out = m.clone();
    for (key, keys, item) in
        [("transition", &T_KEYS[..], transition_item as fn(&str) -> Option<Vec<String>>), ("animation", &A_KEYS[..], animation_item)]
    {
        let Some(v) = get(&out, key).cloned() else { continue };
        if let Some(longs) = expand_list(&v, keys, item) {
            out.retain(|(k, _)| k != key);
            for (k, val) in longs {
                match out.iter_mut().find(|(kk, _)| *kk == k) {
                    Some(e) => e.1 = Css::Str(val),
                    None => out.push((k, Css::Str(val))),
                }
            }
        }
    }
    out
}

fn easings(m: &Decls) -> Vec<String> {
    let d = expanded(m);
    let mut out = Vec::new();
    for key in ["transitionTimingFunction", "animationTimingFunction"] {
        if let Some(v) = get(&d, key) {
            out.extend(items(&v.text()));
        }
    }
    for key in ["transition", "animation"] {
        if let Some(v) = get(&d, key) {
            out.push(v.text());
        }
    }
    out
}

/// The text inside the first `name(…)` (case-insensitive).
fn call_args<'a>(e: &'a str, name: &str) -> Option<&'a str> {
    let at = e.to_lowercase().find(&format!("{name}("))?;
    let rest = &e[at + name.len() + 1..];
    let end = rest.find(')')?;
    Some(&rest[..end])
}

fn js_number_of(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t.bytes().any(|b| b.is_ascii_alphabetic() && b != b'e' && b != b'E') {
        return f64::NAN;
    }
    t.parse().unwrap_or(f64::NAN)
}

/// The easing when it bounces.
pub fn bounce_of(easing: &str) -> Option<String> {
    let e = easing.trim();
    let l = e.to_lowercase();
    if l.contains("bounce") || l.contains("elastic") {
        return Some(e.to_string());
    }
    if let Some(args) = call_args(e, "cubic-bezier") {
        let n: Vec<f64> = args.split(',').map(js_number_of).collect();
        if n.len() == 4 && n.iter().all(|x| x.is_finite()) && (n[1] < 0.0 || n[1] > 1.0 || n[3] < 0.0 || n[3] > 1.0) {
            return Some(e.to_string());
        }
    }
    if let Some(args) = call_args(e, "linear") {
        let outs: Vec<f64> = args.split(',').map(|stop| js_number_of(stop.split_whitespace().next().unwrap_or(""))).collect();
        if outs.iter().all(|x| x.is_finite()) && outs.windows(2).any(|w| w[1] < w[0]) {
            return Some(e.to_string());
        }
    }
    None
}

fn layout_transitions(m: &Decls) -> Vec<String> {
    let d = expanded(m);
    let Some(v) = get(&d, "transitionProperty") else { return Vec::new() };
    items(&v.text()).into_iter().map(|p| p.trim().to_lowercase()).filter(|p| p == "all" || LAYOUT_PROPS.contains(&p.as_str())).collect()
}

fn dedup(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for x in v {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    out
}

/// The three rules on one layer. `base` is its layout + style.
pub fn checks<M: Model>(m: &M, n: M::Id, base: &Decls, out: &mut Out<M::Id>) {
    let extra = m.extra_maps(n);
    let maps: Vec<(&str, &Decls)> = std::iter::once(("", base)).chain(extra.iter().map(|e| (e.prefix.as_str(), &e.decls))).collect();
    let (mut bounces, mut layout) = (Vec::new(), Vec::new());
    for (where_, map) in &maps {
        for e in easings(map) {
            if let Some(why) = bounce_of(&e) {
                bounces.push(format!("{where_}{why}"));
            }
        }
        for p in layout_transitions(map) {
            layout.push(format!("{where_}{}", if p == "all" { "transition-property: all (includes layout properties)".to_string() } else { p }));
        }
    }
    if !bounces.is_empty() {
        out.add("bounce-easing", n, Some(dedup(bounces).join("; ")), vec![]);
    }
    if !layout.is_empty() {
        out.add("layout-transition", n, Some(dedup(layout).join("; ")), vec![]);
    }
    if m.kind(n) == Kind::Image {
        let mut moved = Vec::new();
        for state in ["hover", "active"] {
            for e in extra.iter().filter(|e| e.state.as_deref() == Some(state)) {
                for k in ["transform", "scale", "rotate"] {
                    if let Some(v) = get(&e.decls, k)
                        && v.text().trim() != "none"
                    {
                        moved.push(format!("{}: {k} {}", if state == "active" { "pressed" } else { state }, v.text()));
                    }
                }
            }
        }
        if !moved.is_empty() {
            out.add("image-hover-transform", n, Some(dedup(moved).join("; ")), vec![]);
        }
    }
}
