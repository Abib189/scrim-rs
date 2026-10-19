//! `analyze_design` on FigCraft's document.
//!
//! - **Declarations.** Layers are read through the lint's CSS view (`cmd/lint/figcraft.rs`), so
//!   the colours, type styles and spacing are the ones the export carries. Hidden layers are
//!   included, as Scrim includes everything in scope.
//! - **The tokens:**
//!   - **colours:** colour variables (their collection's default mode), and paint styles with
//!     one solid paint;
//!   - **typography:** the text styles;
//!   - **spacing:** number variables scoped to gaps (or named for spacing, padding or gaps when
//!     they have no scopes).
//! - **`repeated`** leaves out components, component sets and instances.

use figcraft_doc::{Document, LetterSpacing, LineHeight, NodeId, NodeKind, StyleKind, VariableType, VariableValue};
use serde_json::{Value, json};

use super::{Source, Tokens, analyze};
use crate::cmd::CommandSpec;
use crate::cmd::lint::helpers::js_num;
use crate::cmd::lint::{Css, css, get};
use crate::{EngineError, Result, Session, spec};

pub struct FigSource<'a> {
    pub doc: &'a Document,
}

impl FigSource<'_> {
    fn spacing_variable(v: &figcraft_doc::Variable) -> bool {
        match &v.scopes {
            Some(s) if !s.iter().any(|x| x == "ALL_SCOPES") => s.iter().any(|x| x == "GAP"),
            _ => {
                let n = v.name.to_lowercase();
                ["spac", "gap", "padding", "gutter"].iter().any(|k| n.contains(k))
            }
        }
    }
}

impl Source for FigSource<'_> {
    type Id = NodeId;

    fn roots(&self) -> Vec<NodeId> {
        self.doc.pages.iter().flat_map(|p| self.doc.node(*p).map(|pg| pg.children.clone()).unwrap_or_default()).collect()
    }
    fn exists(&self, id: NodeId) -> bool {
        self.doc.node(id).is_some_and(|n| !n.is_page())
    }
    fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.doc.node(id)?.parent.filter(|p| self.exists(*p))
    }
    fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.doc.node(id).map(|n| n.children.clone()).unwrap_or_default()
    }
    fn name(&self, id: NodeId) -> String {
        self.doc.node(id).map(|n| n.name.clone()).unwrap_or_default()
    }
    fn json_id(&self, id: NodeId) -> Value {
        json!(id.0)
    }
    fn kind(&self, id: NodeId) -> String {
        self.doc.node(id).map_or(String::new(), |n| n.kind.type_name().to_lowercase())
    }
    fn decl(&self, id: NodeId, key: &str) -> Option<Css> {
        let n = self.doc.node(id)?;
        get(&css(self.doc, n), key).filter(|v| !matches!(v, Css::Str(s) if s.is_empty())).cloned()
    }
    fn svg_paints(&self, _id: NodeId) -> Vec<String> {
        Vec::new()
    }
    fn is_component(&self, id: NodeId) -> bool {
        self.doc.node(id).is_some_and(|n| matches!(n.kind, NodeKind::Component { .. } | NodeKind::ComponentSet { .. } | NodeKind::Instance { .. }))
    }
    fn tokens(&self) -> Tokens {
        let mut t = Tokens::default();
        let vars = &self.doc.variables;
        for v in &vars.variables {
            let mode = vars.collection(&v.collection).and_then(|c| c.default_mode().cloned());
            let Some(value) = mode.and_then(|m| v.values.get(&m)).or_else(|| v.values.values().next()) else { continue };
            match (v.kind, value) {
                (VariableType::Color, VariableValue::Color(c)) => t.colors.push((v.name.clone(), crate::cmd::lint::figcraft_css_color(*c))),
                (VariableType::Number, VariableValue::Number(n)) if Self::spacing_variable(v) => t.spacing.push((v.name.clone(), Css::Num(*n))),
                _ => {}
            }
        }
        for s in &self.doc.styles {
            match &s.kind {
                StyleKind::Paint { paints } if paints.len() == 1 => {
                    if let Some(c) = paints[0].solid_color() {
                        t.colors.push((s.name.clone(), crate::cmd::lint::figcraft_css_color(c)));
                    }
                }
                StyleKind::Text { style } => {
                    let mut p = vec![
                        ("fontFamily".to_string(), Css::Str(style.font_family.clone())),
                        ("fontSize".to_string(), Css::Num(style.font_size)),
                        ("fontWeight".to_string(), Css::Num(f64::from(figcraft_doc::style_weight(&style.font_style)))),
                    ];
                    match style.line_height {
                        LineHeight::Auto => {}
                        LineHeight::Pixels(px) => p.push(("lineHeight".into(), Css::Str(format!("{}px", js_num(px))))),
                        LineHeight::Percent(pc) => p.push(("lineHeight".into(), Css::Str(format!("{}%", js_num(pc))))),
                    }
                    match style.letter_spacing {
                        LetterSpacing::Pixels(px) if px != 0.0 => p.push(("letterSpacing".into(), Css::Str(format!("{}px", js_num(px))))),
                        LetterSpacing::Percent(pc) if pc != 0.0 => p.push(("letterSpacing".into(), Css::Str(format!("{}em", js_num(pc / 100.0))))),
                        _ => {}
                    }
                    t.typography.push((s.name.clone(), p));
                }
                _ => {}
            }
        }
        t
    }
}

fn analyze_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let doc = s.doc()?;
    let src = FigSource { doc };
    let scope = match p.get("id") {
        None | Some(Value::Null) => None,
        Some(raw) => {
            let id = crate::params::id_of(raw).filter(|i| src.exists(*i));
            Some(id.ok_or_else(|| EngineError::Other(format!("node not found: {}", raw.as_str().map_or_else(|| raw.to_string(), str::to_string))))?)
        }
    };
    Ok(analyze(&src, scope))
}

/// `analyze_design`'s description (`analyze-design.md` §1).
pub const DESCRIPTION: &str = "Report what the document (or a node's subtree: pass id) actually uses, against its own tokens. Read-only. colors: every colour by frequency (hex, alpha kept), each matched to a colour token exactly or within \u{394}E 3, near-duplicate clusters with the value to keep, and var(--token) references. typography: text styles (family, size, weight, line height, letter spacing, inherited) by frequency, matched to typography tokens. spacing: gap / padding / margin values by frequency, off-scale ones flagged against the spacing tokens (a 4 px grid when there are none). repeated: subtrees of the same shape that appear more than once \u{2014} component candidates (top 20). Use it to find drift before tidying a design; it never changes anything.";

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "design.analyze",
        "Analyze design",
        "{id? (a layer: its subtree; default the whole document)} → {scope, nodes, colors: {distinct, values, nearDuplicates, tokenRefs, unparsed}, typography: {textNodes, distinct, styles}, spacing: {scale, distinct, values, offScale, unparsed}, repeated: [{count, nodesEach, ids, names, shape}]}. Read-only. Tokens are colour variables and solid paint styles, text styles, and number variables scoped to gaps",
        false,
        analyze_cmd
    )]
}
