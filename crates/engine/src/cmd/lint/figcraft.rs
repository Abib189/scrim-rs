//! The lint on FigCraft's document.
//!
//! - **The CSS view.** Each layer is described as the CSS declarations its export would carry:
//!   fills as `color` / `background*`, strokes as `border`, auto layout as `display`, `gap` and
//!   `padding`, shadows as `boxShadow`, text style as `font*`, `letterSpacing`, `lineHeight`,
//!   `textTransform`, `textAlign`. The rules read that view.
//! - **Hidden layers** are not linted.
//! - **Boxes** come from the document (page-space bounds), so the rules that need boxes always
//!   run.
//! - **Artboards** are each page's top-level layers. Overlapping ones are found within a page.
//! - **Profiles and ignores** live in layer data, so they are saved, undoable and shared:
//!   - a root's profile under `scrim.lint.profile`;
//!   - a layer's ignores under `scrim.lint.ignores`;
//!   - the document-wide ignores on the first page, under `scrim.lint.documentIgnores`.

use std::collections::BTreeMap;

use figcraft_color::{Color, GradientKind, PaintKind};
use figcraft_doc::{Document, EffectKind, LayoutMode, LetterSpacing, LineHeight, MainAlign, Node, NodeId, NodeKind, TextAlignH, TextCase};
use serde_json::{Value, json};

use super::helpers::js_num;
use super::{Css, Decls, ExtraMap, Ignore, Kind, LintConfig, Model, Profile, Rect, describe_profiles, ignore, lint_design, set_profile};
use crate::cmd::CommandSpec;
use crate::{EngineError, Result, Session, spec};

const PROFILE_KEY: &str = "scrim.lint.profile";
const IGNORES_KEY: &str = "scrim.lint.ignores";
const DOC_IGNORES_KEY: &str = "scrim.lint.documentIgnores";

/// FigCraft's document, as the lint reads it.
pub struct FigLint<'a> {
    pub doc: &'a Document,
}

/// `#rrggbb`, or `rgba(r, g, b, a)` when not opaque (a form the lint's colour parser reads).
pub fn css_color(c: Color) -> String {
    let [r, g, b, a] = c.to_rgba8();
    if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("rgba({r}, {g}, {b}, {})", js_num((f64::from(c.a) * 100.0).round() / 100.0)) }
}

fn gradient(g: &figcraft_color::Gradient, opacity: f32) -> String {
    let stops: Vec<String> = g
        .stops
        .iter()
        .map(|s| format!("{} {}%", css_color(s.color.with_alpha(s.color.a * opacity)), js_num((f64::from(s.position) * 100.0).round())))
        .collect();
    let kind = match g.kind {
        GradientKind::Linear => "linear-gradient",
        GradientKind::Radial | GradientKind::Diamond => "radial-gradient",
        GradientKind::Angular => "conic-gradient",
    };
    format!("{kind}({})", stops.join(", "))
}

fn text_css(t: &figcraft_doc::TextData, put: &mut impl FnMut(&str, Css)) {
    let st = &t.style;
    put("fontFamily", Css::Str(st.font_family.clone()));
    put("fontSize", Css::Num(st.font_size));
    put("fontWeight", Css::Num(f64::from(figcraft_doc::style_weight(&st.font_style))));
    match st.line_height {
        LineHeight::Auto => {}
        LineHeight::Pixels(px) => put("lineHeight", Css::Str(format!("{}px", js_num(px)))),
        LineHeight::Percent(pc) => put("lineHeight", Css::Str(format!("{}%", js_num(pc)))),
    }
    match st.letter_spacing {
        LetterSpacing::Pixels(px) if px != 0.0 => put("letterSpacing", Css::Str(format!("{}px", js_num(px)))),
        LetterSpacing::Percent(pc) if pc != 0.0 => put("letterSpacing", Css::Str(format!("{}em", js_num(pc / 100.0)))),
        _ => {}
    }
    match st.case {
        TextCase::Upper => put("textTransform", Css::Str("uppercase".into())),
        TextCase::Lower => put("textTransform", Css::Str("lowercase".into())),
        TextCase::Title => put("textTransform", Css::Str("capitalize".into())),
        _ => {}
    }
    let align = match t.align_h {
        TextAlignH::Left => None,
        TextAlignH::Center => Some("center"),
        TextAlignH::Right => Some("right"),
        TextAlignH::Justified => Some("justify"),
    };
    if let Some(a) = align {
        put("textAlign", Css::Str(a.into()));
    }
}

/// The CSS declarations a layer's export carries (what the rules read).
pub fn css(doc: &Document, n: &Node) -> Decls {
    let mut d: Decls = Vec::new();
    let mut put = |k: &str, v: Css| d.push((k.to_string(), v));
    put("width", Css::Num(n.width));
    put("height", Css::Num(n.height));
    let text = match &n.kind {
        NodeKind::Text(t) => Some(t),
        _ => None,
    };
    // Fills stack bottom to top: the top visible one shows.
    let top = n.fills.iter().rev().find(|p| p.visible);
    let solid = |p: &figcraft_color::Paint| Css::Str(css_color(p.solid_color().unwrap_or(Color::BLACK)));
    match (text.is_some(), top) {
        (true, Some(p)) => match &p.kind {
            PaintKind::Solid { .. } => put("color", solid(p)),
            PaintKind::Gradient(g) => {
                put("color", Css::Str("transparent".into()));
                put("background", Css::Str(gradient(g, p.opacity)));
                put("backgroundClip", Css::Str("text".into()));
            }
            _ => {}
        },
        (false, Some(p)) => match &p.kind {
            PaintKind::Solid { .. } => put("backgroundColor", solid(p)),
            PaintKind::Gradient(g) => put("background", Css::Str(gradient(g, p.opacity))),
            PaintKind::Image(i) => put("backgroundImage", Css::Str(format!("url({})", i.image))),
            PaintKind::Pattern(_) => {}
        },
        _ => {}
    }
    if let Some(stroke) = n.strokes.iter().rev().find(|p| p.visible).and_then(|p| p.solid_color()) {
        let c = css_color(stroke);
        match n.stroke.sides {
            Some(w) if w.iter().any(|x| *x != w[0]) => {
                for (side, w) in ["Top", "Right", "Bottom", "Left"].iter().zip(w) {
                    if w > 0.0 {
                        put(&format!("border{side}"), Css::Str(format!("{}px solid {c}", js_num(w))));
                    }
                }
            }
            _ if n.stroke.weight > 0.0 => put("border", Css::Str(format!("{}px solid {c}", js_num(n.stroke.weight)))),
            _ => {}
        }
    }
    match n.corners.uniform() {
        Some(r) if r > 0.0 => put("borderRadius", Css::Num(r)),
        Some(_) => {}
        None => {
            let [a, b, c, e] = n.corners.radii;
            put("borderRadius", Css::Str(format!("{a}px {b}px {c}px {e}px")));
        }
    }
    if let Some(l) = &n.layout {
        put("display", Css::Str(if l.mode == LayoutMode::Grid { "grid" } else { "flex" }.into()));
        match l.mode {
            LayoutMode::Horizontal => put("flexDirection", Css::Str("row".into())),
            LayoutMode::Vertical => put("flexDirection", Css::Str("column".into())),
            LayoutMode::Grid => {}
        }
        let auto = matches!(l.main_align, MainAlign::SpaceBetween | MainAlign::SpaceAround | MainAlign::SpaceEvenly);
        if !auto && l.gap != 0.0 {
            put("gap", Css::Num(l.gap));
        }
        let [t, r, b, lf] = l.padding;
        if t == r && r == b && b == lf {
            if t != 0.0 {
                put("padding", Css::Num(t));
            }
        } else {
            put("padding", Css::Str(format!("{t}px {r}px {b}px {lf}px")));
        }
    }
    if n.clip && text.is_none() {
        put("overflow", Css::Str("hidden".into()));
    }
    if n.child_layout.absolute && n.parent.and_then(|p| doc.node(p)).is_some_and(|p| p.layout.is_some()) {
        put("position", Css::Str("absolute".into()));
    }
    if n.opacity < 1.0 {
        put("opacity", Css::Num(f64::from(n.opacity)));
    }
    let shadows: Vec<String> = n
        .effects
        .iter()
        .filter(|e| e.visible)
        .filter_map(|e| match &e.kind {
            EffectKind::DropShadow(s) => Some(("", s)),
            EffectKind::InnerShadow(s) => Some(("inset ", s)),
            _ => None,
        })
        .map(|(inset, s)| {
            format!("{inset}{}px {}px {}px {}px {}", js_num(s.offset.0), js_num(s.offset.1), js_num(s.radius), js_num(s.spread), css_color(s.color))
        })
        .collect();
    if !shadows.is_empty() {
        put("boxShadow", Css::Str(shadows.join(", ")));
    }
    if let Some(t) = text {
        text_css(t, &mut put);
    }
    d
}

impl FigLint<'_> {
    fn node(&self, id: NodeId) -> Option<&Node> {
        self.doc.node(id).filter(|n| !n.is_page() && n.visible)
    }

    /// The stored configuration: the document-wide ignores, then each layer's in document order.
    pub fn config(&self) -> LintConfig<NodeId> {
        let mut cfg = LintConfig::default();
        let read = |n: &Node, key: &str| -> Vec<(String, String)> {
            n.plugin_data
                .get(key)
                .and_then(|s| serde_json::from_str::<Vec<Value>>(s).ok())
                .into_iter()
                .flatten()
                .filter_map(|v| Some((v["ruleId"].as_str()?.to_string(), v["reason"].as_str()?.to_string())))
                .collect()
        };
        if let Some(first) = self.doc.pages.first().and_then(|p| self.doc.node(*p)) {
            cfg.ignores.extend(read(first, DOC_IGNORES_KEY).into_iter().map(|(rule_id, reason)| Ignore { rule_id, node: None, reason }));
        }
        for p in &self.doc.pages {
            for id in self.doc.subtree(*p) {
                let Some(n) = self.doc.node(id) else { continue };
                if let Some(pr) = n.plugin_data.get(PROFILE_KEY).and_then(|s| Profile::parse(s)) {
                    cfg.profiles.push((id, pr));
                }
                cfg.ignores.extend(read(n, IGNORES_KEY).into_iter().map(|(rule_id, reason)| Ignore { rule_id, node: Some(id), reason }));
            }
        }
        cfg
    }
}

/// Write `cfg` into layer data, changing only the layers whose entries differ.
fn save(doc: &mut Document, cfg: &LintConfig<NodeId>) {
    let mut want: BTreeMap<(NodeId, &str), String> = BTreeMap::new();
    for (id, p) in &cfg.profiles {
        want.insert((*id, PROFILE_KEY), p.as_str().to_string());
    }
    let first = doc.pages.first().copied();
    let mut lists: BTreeMap<(NodeId, &str), Vec<Value>> = BTreeMap::new();
    for i in &cfg.ignores {
        let key = match (i.node, first) {
            (Some(n), _) => (n, IGNORES_KEY),
            (None, Some(f)) => (f, DOC_IGNORES_KEY),
            (None, None) => continue,
        };
        lists.entry(key).or_default().push(json!({"ruleId": i.rule_id, "reason": i.reason}));
    }
    for (k, v) in lists {
        want.insert(k, Value::Array(v).to_string());
    }
    let keys = [PROFILE_KEY, IGNORES_KEY, DOC_IGNORES_KEY];
    let mut holders: Vec<NodeId> = doc.nodes().filter(|n| keys.iter().any(|k| n.plugin_data.contains_key(*k))).map(|n| n.id).collect();
    holders.extend(want.keys().map(|(id, _)| *id));
    holders.sort();
    holders.dedup();
    for id in holders {
        let Some(n) = doc.node(id) else { continue };
        let mut data = n.plugin_data.clone();
        for key in keys {
            match want.get(&(id, key)) {
                Some(v) => {
                    data.insert(key.to_string(), v.clone());
                }
                None => {
                    data.remove(key);
                }
            }
        }
        if data != n.plugin_data
            && let Some(n) = doc.node_mut(id)
        {
            n.plugin_data = data;
        }
    }
}

impl Model for FigLint<'_> {
    type Id = NodeId;

    fn root_groups(&self) -> Vec<Vec<NodeId>> {
        self.doc
            .pages
            .iter()
            .map(|p| self.doc.node(*p).map(|pg| pg.children.iter().copied().filter(|c| self.node(*c).is_some()).collect()).unwrap_or_default())
            .collect()
    }
    fn resolve(&self, raw: &Value) -> Option<NodeId> {
        let id = crate::params::id_of(raw)?;
        self.node(id).map(|n| n.id)
    }
    fn exists(&self, id: NodeId) -> bool {
        self.node(id).is_some()
    }
    /// A top-level layer has no parent here: its page is not part of the design.
    fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id)?.parent.filter(|p| self.node(*p).is_some())
    }
    fn is_root(&self, id: NodeId) -> bool {
        self.parent(id).is_none()
    }
    fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.node(id).map(|n| n.children.iter().copied().filter(|c| self.node(*c).is_some()).collect()).unwrap_or_default()
    }
    fn name(&self, id: NodeId) -> String {
        self.doc.node(id).map(|n| n.name.clone()).unwrap_or_default()
    }
    fn json_id(&self, id: NodeId) -> Value {
        json!(id.0)
    }
    fn id_text(&self, id: NodeId) -> String {
        id.0.to_string()
    }
    fn kind(&self, id: NodeId) -> Kind {
        let Some(n) = self.node(id) else { return Kind::Other };
        if matches!(n.kind, NodeKind::Text(_)) {
            Kind::Text
        } else if n.fills.iter().any(|p| p.visible && matches!(p.kind, PaintKind::Image(_))) {
            Kind::Image
        } else {
            Kind::Other
        }
    }
    fn text(&self, id: NodeId) -> Option<String> {
        match &self.node(id)?.kind {
            NodeKind::Text(t) => Some(t.characters.clone()),
            _ => None,
        }
    }
    fn layout(&self, _id: NodeId) -> Decls {
        Vec::new()
    }
    fn style(&self, id: NodeId) -> Decls {
        self.node(id).map(|n| css(self.doc, n)).unwrap_or_default()
    }
    fn extra_maps(&self, _id: NodeId) -> Vec<ExtraMap> {
        Vec::new()
    }
    fn rect(&self, id: NodeId) -> Option<Rect> {
        self.node(id)?;
        self.doc.world_bounds(id).map(|r| Rect { x: r.x0, y: r.y0, w: r.width(), h: r.height() })
    }
}

fn other(e: String) -> EngineError {
    EngineError::Other(e)
}

fn profile_param(cmd: &str, p: &Value) -> Result<Option<Profile>> {
    match p.get("profile") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_str()
            .and_then(Profile::parse)
            .map(Some)
            .ok_or_else(|| EngineError::BadParams { cmd: cmd.into(), msg: "`profile` is ui or expressive".into() }),
    }
}

fn run_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let over = profile_param("lint.run", p)?;
    let m = FigLint { doc: s.doc()? };
    let target = match p.get("id") {
        None | Some(Value::Null) => None,
        Some(raw) => {
            Some(m.resolve(raw).ok_or_else(|| other(format!("node not found: {}", raw.as_str().map_or_else(|| raw.to_string(), str::to_string))))?)
        }
    };
    Ok(lint_design(&m, &m.config(), target, over, true))
}

fn profile_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let bad = |msg: &str| EngineError::BadParams { cmd: "lint.setProfile".into(), msg: msg.into() };
    let profile = profile_param("lint.setProfile", p)?.ok_or_else(|| bad("`profile` is required: ui or expressive"))?;
    let ids = p.get("ids").and_then(Value::as_array).cloned().unwrap_or_default();
    if ids.is_empty() || ids.len() > 500 {
        return Err(bad("`ids` holds 1 to 500 top-level layer ids"));
    }
    let m = FigLint { doc: s.doc()? };
    let mut cfg = m.config();
    let (out, changed) = set_profile(&m, &mut cfg, &ids, profile).map_err(other)?;
    if changed {
        save(s.doc_mut()?, &cfg);
    }
    Ok(out)
}

fn ignore_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let rule = p.get("ruleId").and_then(Value::as_str).filter(|r| !r.is_empty());
    let rule = rule.ok_or_else(|| EngineError::BadParams { cmd: "lint.ignore".into(), msg: "`ruleId` is required".into() })?.to_string();
    let m = FigLint { doc: s.doc()? };
    let mut cfg = m.config();
    let before = cfg.clone();
    let remove = p.get("remove").and_then(Value::as_bool).unwrap_or(false);
    let out = ignore(&m, &mut cfg, &rule, p.get("nodeId"), p.get("reason").and_then(Value::as_str), remove).map_err(other)?;
    if cfg != before {
        save(s.doc_mut()?, &cfg);
    }
    Ok(out)
}

/// `lint_design`'s description, with the profile string (`lint.md` §6.1).
pub fn lint_description() -> String {
    format!(
        "Run the deterministic design-rule linter (vendored Impeccable anti-patterns) over a node subtree (pass id) or the whole document. Returns violations with node ids, the rule, its design domain, and why it is a problem. A whole-document run also flags root artboards whose measured rects overlap (root-overlap). Each artboard is judged under its lint profile, stored in the document ({}); pass profile to judge everything in scope under one profile instead. The result names the profile used, lists informational findings and reasoned ignores separately, and counts only active violations. Run it on what you built before reporting done.",
        describe_profiles()
    )
}

/// `set_lint_profile`'s description (§6.2).
pub fn profile_description() -> String {
    format!(
        "Set the lint profile of one or more root artboards, stored in the document (undoable, persisted, seen by every later session). Profiles: {}. Use 'expressive' when the artboard is a deliberate display piece; 'ui' restores the default. Returns each root's profile and its finding counts.",
        describe_profiles()
    )
}

/// `ignore_lint`'s description (§6.3).
pub const IGNORE_DESCRIPTION: &str = "Record a reasoned exception to one lint rule in the document (undoable, persisted, shown in the editor's Ignored group with its reason). With nodeId it covers findings on that node or inside it (pass an artboard root to cover the artboard); without, the whole document. reason is required and must say why the finding is deliberate. remove: true deletes the ignore instead. Prefer set_lint_profile for a whole expressive artboard; never ignore a real contrast failure.";

/// The hint every content-writing tool's description ends with (§6.4).
pub fn write_hint() -> String {
    format!(
        " Before reporting the work done, run lint_design on what you built (id = its artboard root) and fix what it flags. When you make an artboard deliberately expressive (a poster, cover or editorial display type), call set_lint_profile with profile 'expressive' for it so lint judges it as such \u{2014} {}. Use ignore_lint only for one deliberate finding, with a specific reason; never to silence contrast.",
        describe_profiles()
    )
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "lint.run",
            "Lint design",
            "{id? (a layer: its subtree; default the whole document), profile? (ui|expressive: judge everything under it)} → {target, profile, profileSource, profiles, violationCount, byCategory, byDomain, violations: [{ruleId, category, domain, name, message, detail?, nodeId, nodeName, rootId, relatedNodeIds?}], informational, relaxedByProfile, ignored}. Read-only; hidden layers are skipped",
            false,
            run_cmd
        ),
        spec!(
            "lint.setProfile",
            "Set lint profile",
            "{ids (top-level layers, 1–500), profile: ui|expressive} → {roots: [{id, name, profile, active, relaxed, ignored}]} (stored in the document; one undo step)",
            true,
            profile_cmd
        ),
        spec!(
            "lint.ignore",
            "Ignore a lint finding",
            "{ruleId, nodeId? (it and everything in it; default the whole document), reason (why it is deliberate), remove?} → {ignores: [{ruleId, nodeId?, reason}]} (stored in the document; one undo step)",
            true,
            ignore_cmd
        ),
    ]
}
