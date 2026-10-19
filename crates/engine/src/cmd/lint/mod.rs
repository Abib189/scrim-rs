//! The design lint, Scrim's `lint_design`, `ignore_lint` and `set_lint_profile` (Port 6). The
//! spec is `docs/scrim-transfer/specs/lint.md` and `lint-rules.md`; the rules are a port of
//! Impeccable's detector (Apache-2.0; `design-rules/NOTICE.md`, `design-rules/LICENSE`).
//!
//! It must never:
//! - **write.** The lint is a pure function of the document, its rects and its lint
//!   configuration. Only the two configuration commands write, each as one undo step.
//! - **guess a rect.** A rule that needs a box skips a layer without one.
//!
//! The rules read a [`Model`]: each layer's CSS declarations, its text, its tree and its box.
//! `figcraft.rs` describes FigCraft's layers that way. The tests replay Scrim's own cases on
//! Scrim's documents (`scrim_cases_tests.rs`).

mod aggregate;
mod checks;
mod config;
mod figcraft;
pub mod helpers;
mod motion;
mod rendered;
pub mod rules;

use std::collections::HashSet;
use std::hash::Hash;

use serde_json::{Map, Value, json};

pub use super::query::Val as Css;
pub use config::{Ignore, IgnoreEdit, LintConfig, Mode, Profile, describe_profiles};
pub use figcraft::{FigLint, IGNORE_DESCRIPTION, css, css_color as figcraft_css_color, lint_description, profile_description, specs, write_hint};
use rules::Rule;

/// A layer's declarations, in order.
pub type Decls = Vec<(String, Css)>;

/// The value of `key` in `d`.
pub fn get<'a>(d: &'a Decls, key: &str) -> Option<&'a Css> {
    d.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Layout, then style on top: what renders.
pub fn merge(layout: &Decls, style: &Decls) -> Decls {
    let mut out = layout.clone();
    for (k, v) in style {
        match out.iter_mut().find(|(kk, _)| kk == k) {
            Some(e) => e.1 = v.clone(),
            None => out.push((k.clone(), v.clone())),
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Image,
    Other,
}

/// A border box in canvas or page space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    /// Strict overlap: touching edges don't count.
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

/// A state or media declaration map, for the motion rules.
#[derive(Clone, Debug)]
pub struct ExtraMap {
    /// `"hover: "`, `"(prefers-reduced-motion: reduce): "`, `"(…) hover: "`.
    pub prefix: String,
    /// The state (`hover`, `active` …) for state maps.
    pub state: Option<String>,
    pub decls: Decls,
}

/// What the lint reads. Ids are opaque; `json_id` and `id_text` show them.
pub trait Model {
    type Id: Copy + Eq + Hash + std::fmt::Debug;
    /// The top-level layers, grouped by canvas (Scrim: one; FigCraft: one per page). Overlapping
    /// roots are found within a group.
    fn root_groups(&self) -> Vec<Vec<Self::Id>>;
    fn resolve(&self, raw: &Value) -> Option<Self::Id>;
    fn exists(&self, id: Self::Id) -> bool;
    fn parent(&self, id: Self::Id) -> Option<Self::Id>;
    /// A top-level layer (an artboard).
    fn is_root(&self, id: Self::Id) -> bool;
    fn children(&self, id: Self::Id) -> Vec<Self::Id>;
    fn name(&self, id: Self::Id) -> String;
    fn json_id(&self, id: Self::Id) -> Value;
    fn id_text(&self, id: Self::Id) -> String;
    fn kind(&self, id: Self::Id) -> Kind;
    fn text(&self, id: Self::Id) -> Option<String>;
    fn layout(&self, id: Self::Id) -> Decls;
    fn style(&self, id: Self::Id) -> Decls;
    /// State and media maps (the base map is layout + style).
    fn extra_maps(&self, id: Self::Id) -> Vec<ExtraMap>;
    fn rect(&self, id: Self::Id) -> Option<Rect>;
}

/// The effective declarations: layout, then style.
pub fn decls<M: Model>(m: &M, id: M::Id) -> Decls {
    merge(&m.layout(id), &m.style(id))
}

/// A text layer with text.
pub fn is_text<M: Model>(m: &M, id: M::Id) -> bool {
    m.kind(id) == Kind::Text && m.text(id).is_some_and(|t| !t.is_empty())
}

/// The top-level layer above `id` (itself for a root).
pub fn root_of<M: Model>(m: &M, id: M::Id) -> M::Id {
    let mut cur = id;
    let mut guard = 0;
    while !m.is_root(cur) && guard < 100_000 {
        match m.parent(cur) {
            Some(p) => cur = p,
            None => break,
        }
        guard += 1;
    }
    cur
}

/// The ancestors, nearest first.
pub fn ancestors<M: Model>(m: &M, id: M::Id) -> Vec<M::Id> {
    let mut out = Vec::new();
    let mut cur = m.parent(id);
    while let Some(p) = cur {
        if out.len() > 100_000 {
            break;
        }
        out.push(p);
        cur = m.parent(p);
    }
    out
}

/// The subtree in pre-order.
pub fn subtree<M: Model>(m: &M, id: M::Id) -> Vec<M::Id> {
    let mut out = Vec::new();
    let mut stack = vec![id];
    while let Some(i) = stack.pop() {
        if !m.exists(i) {
            continue;
        }
        out.push(i);
        stack.extend(m.children(i).into_iter().rev());
    }
    out
}

/// One finding, before classification.
#[derive(Clone, Debug)]
pub struct Finding<Id> {
    pub rule: &'static Rule,
    pub node: Id,
    pub root: Id,
    pub detail: Option<String>,
    pub related: Vec<Id>,
}

/// Collects a run's findings in order.
pub struct Out<Id> {
    pub root: Id,
    pub list: Vec<Finding<Id>>,
}

impl<Id: Copy> Out<Id> {
    pub fn add(&mut self, rule: &str, node: Id, detail: Option<String>, related: Vec<Id>) {
        self.list.push(Finding { rule: rules::rule(rule), node, root: self.root, detail, related });
    }
}

/// Lint `target`'s subtree (`lintNode`). `rendered`: run the rules that need boxes.
pub fn lint_node<M: Model>(m: &M, target: M::Id, rendered: bool) -> Vec<Finding<M::Id>> {
    if !m.exists(target) {
        return Vec::new();
    }
    let nodes = subtree(m, target);
    let mut out = Out { root: root_of(m, target), list: Vec::new() };
    for &n in &nodes {
        checks::node(m, n, &mut out);
    }
    if rendered {
        rendered::checks(m, &nodes, &mut out);
    }
    aggregate::checks(m, target, &nodes, &mut out);
    out.list
}

/// Lint every root, then the overlapping roots (`lintDocument`).
pub fn lint_document<M: Model>(m: &M, rendered: bool) -> Vec<Finding<M::Id>> {
    let groups = m.root_groups();
    let mut out: Vec<Finding<M::Id>> = groups.iter().flatten().flat_map(|r| lint_node(m, *r, rendered)).collect();
    if rendered {
        for g in &groups {
            out.extend(rendered::root_overlaps(m, g));
        }
    }
    out
}

fn violation<M: Model>(m: &M, f: &Finding<M::Id>) -> Value {
    let mut v = json!({
        "ruleId": f.rule.id,
        "category": f.rule.category.as_str(),
        "domain": f.rule.domain,
        "name": f.rule.name,
        "message": f.rule.message,
        "nodeId": m.json_id(f.node),
        "nodeName": m.name(f.node),
        "rootId": m.json_id(f.root),
    });
    if let Some(d) = &f.detail {
        v["detail"] = json!(d);
    }
    if !f.related.is_empty() {
        v["relatedNodeIds"] = Value::Array(f.related.iter().map(|r| m.json_id(*r)).collect());
    }
    v
}

fn bump(map: &mut Map<String, Value>, key: &str) {
    let n = map.get(key).and_then(Value::as_u64).unwrap_or(0);
    map.insert(key.to_string(), json!(n + 1));
}

/// `lint_design`'s result for `target` (`None`: the whole document), with an optional profile
/// override. `rendered`: whether boxes are known.
pub fn lint_design<M: Model>(m: &M, cfg: &LintConfig<M::Id>, target: Option<M::Id>, over: Option<Profile>, rendered: bool) -> Value {
    let findings = match target {
        Some(t) => lint_node(m, t, rendered),
        None => lint_document(m, rendered),
    };
    let mut profiles = Map::new();
    let (mut active, mut info, mut relaxed, mut ignored) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut by_cat, mut by_dom) = (Map::new(), Map::new());
    for f in &findings {
        let profile = over.unwrap_or_else(|| cfg.profile(f.root));
        if !profiles.contains_key(&m.id_text(f.root)) {
            profiles.insert(m.id_text(f.root), json!(profile.as_str()));
        }
        if let Some(ig) = cfg.matching(m, f) {
            ignored.push(json!({
                "ruleId": f.rule.id,
                "nodeId": m.json_id(f.node),
                "nodeName": m.name(f.node),
                "reason": ig.reason,
                "ignoreNodeId": ig.node.map_or(Value::Null, |n| m.json_id(n)),
            }));
            continue;
        }
        match config::mode(f.rule.id, profile) {
            Mode::Off => {
                relaxed.push(json!({"ruleId": f.rule.id, "nodeId": m.json_id(f.node), "nodeName": m.name(f.node), "rootId": m.json_id(f.root)}))
            }
            Mode::Info => info.push(violation(m, f)),
            Mode::Active => {
                bump(&mut by_cat, f.rule.category.as_str());
                bump(&mut by_dom, f.rule.domain);
                active.push(violation(m, f));
            }
        }
    }
    let (profile, source) = match (over, target) {
        (Some(p), _) => (p.as_str().to_string(), "override"),
        (None, Some(t)) => (cfg.profile(root_of(m, t)).as_str().to_string(), "document"),
        (None, None) => ("per-artboard".to_string(), "document"),
    };
    json!({
        "target": target.map_or(json!("(document)"), |t| m.json_id(t)),
        "profile": profile,
        "profileSource": source,
        "profiles": profiles,
        "violationCount": active.len(),
        "byCategory": by_cat,
        "byDomain": by_dom,
        "violations": active,
        "informational": info,
        "relaxedByProfile": relaxed,
        "ignored": ignored,
    })
}

/// The active findings' rule ids per layer, one per finding (what the query's `@lint` and
/// `@lintCount` read): not ignored, not informational, not relaxed by the artboard's profile.
pub fn active_by_node<M: Model>(m: &M, cfg: &LintConfig<M::Id>, rendered: bool) -> std::collections::HashMap<M::Id, Vec<&'static str>> {
    let mut out: std::collections::HashMap<M::Id, Vec<&'static str>> = std::collections::HashMap::new();
    for f in lint_document(m, rendered) {
        if cfg.matching(m, &f).is_none() && config::mode(f.rule.id, cfg.profile(f.root)) == Mode::Active {
            out.entry(f.node).or_default().push(f.rule.id);
        }
    }
    out
}

/// `set_lint_profile`: check every id first (nothing is written on a failure), then set each
/// root's profile. Returns the result and whether anything changed.
pub fn set_profile<M: Model>(m: &M, cfg: &mut LintConfig<M::Id>, raw_ids: &[Value], profile: Profile) -> Result<(Value, bool), String> {
    let mut ids: Vec<M::Id> = Vec::new();
    let mut seen = HashSet::new();
    for raw in raw_ids {
        let text = raw.as_str().map_or_else(|| raw.to_string(), str::to_string);
        let id = m.resolve(raw).filter(|i| m.exists(*i)).ok_or_else(|| format!("node not found: {text}"))?;
        if !m.is_root(id) {
            let parent = m.parent(id).map_or_else(String::new, |p| m.id_text(p));
            return Err(format!("lint profiles apply to root artboards; {} is inside {parent}", m.id_text(id)));
        }
        if seen.insert(id) {
            ids.push(id);
        }
    }
    let mut changed = false;
    for &id in &ids {
        if cfg.profile(id) != profile {
            cfg.set_profile(id, profile);
            changed = true;
        }
    }
    let roots: Vec<Value> = ids
        .iter()
        .map(|&id| {
            let (mut active, mut relaxed, mut ignored) = (0, 0, 0);
            let p = cfg.profile(id);
            for f in lint_node(m, id, false) {
                if cfg.matching(m, &f).is_some() {
                    ignored += 1;
                    continue;
                }
                match config::mode(f.rule.id, p) {
                    Mode::Active => active += 1,
                    Mode::Off => relaxed += 1,
                    Mode::Info => {}
                }
            }
            json!({"id": m.json_id(id), "name": m.name(id), "profile": p.as_str(), "active": active, "relaxed": relaxed, "ignored": ignored})
        })
        .collect();
    Ok((json!({"roots": roots}), changed))
}

/// `ignore_lint`: record, re-reason or remove one reasoned exception.
pub fn ignore<M: Model>(
    m: &M,
    cfg: &mut LintConfig<M::Id>,
    rule_id: &str,
    node: Option<&Value>,
    reason: Option<&str>,
    remove: bool,
) -> Result<Value, String> {
    if !rules::known(rule_id) {
        return Err(format!("unknown lint rule: {rule_id}. Known rules: {}", rules::ids().join(", ")));
    }
    let node = match node {
        None | Some(Value::Null) => None,
        Some(raw) => {
            let text = raw.as_str().map_or_else(|| raw.to_string(), str::to_string);
            Some(m.resolve(raw).filter(|i| m.exists(*i)).ok_or_else(|| format!("node not found: {text}"))?)
        }
    };
    let reason = reason.map(str::trim).unwrap_or("");
    if !remove && reason.is_empty() {
        return Err(format!("ignore_lint needs a non-empty reason saying why {rule_id} is deliberate here"));
    }
    cfg.apply_ignore(IgnoreEdit { rule_id: rule_id.to_string(), node, reason: (!remove).then(|| reason.to_string()), index: None })?;
    Ok(json!({"ignores": cfg.ignores_json(m)}))
}

#[cfg(test)]
#[path = "scrim_cases_tests.rs"]
pub(crate) mod scrim_cases_tests;

#[cfg(test)]
#[path = "figcraft_tests.rs"]
mod figcraft_tests;
