//! The query on FigCraft's document (the [MAP] choices; `docs/scrim-ports/query-nodes.md`).
//!
//! - **Scope.** The top level is each page's layers, page by page, so `/FRAME[2]` is each page's
//!   second top-level frame. `page` limits the query to one page.
//! - **Node tests.** Each node kind by its Figma name (`FRAME`, `RECTANGLE`, `BOOLEAN_OPERATION` …),
//!   plus `IMAGE`: any layer with a visible image fill.
//! - **Attributes.** Read from the document, never from a measurement, and the same in the app,
//!   the CLI and MCP. `@style.<key>` and `@layout.<key>` read a fixed table of CSS properties;
//!   any other key is absent, as an undeclared property is in Scrim.

use std::collections::HashMap;

use figcraft_color::{Color, PaintKind};
use figcraft_doc::{Document, LayoutMode, Node, NodeId, NodeKind, Sizing, TextAlignH};
use serde_json::{Value, json};

use super::eval::{Tree, Val};
use super::parse::Attr;
use super::{Hit, HitRect, QueryError, QueryOut, Vocab, parse, run};
use crate::cmd::CommandSpec;
use crate::{EngineError, Result, Session, spec};

/// FigCraft's vocabulary.
pub const FIGCRAFT: Vocab = Vocab {
    tests: &[
        "FRAME",
        "GROUP",
        "SECTION",
        "RECTANGLE",
        "ELLIPSE",
        "POLYGON",
        "STAR",
        "LINE",
        "VECTOR",
        "TEXT",
        "BOOLEAN_OPERATION",
        "COMPONENT",
        "COMPONENT_SET",
        "INSTANCE",
        "SLICE",
        "IMAGE",
    ],
    attrs: &[
        "id",
        "name",
        "type",
        "text",
        "src",
        "width",
        "height",
        "fill",
        "color",
        "background",
        "layout",
        "direction",
        "position",
        "children",
        "component",
        "componentId",
        "hidden",
        "locked",
        "lint",
        "lintCount",
    ],
};

/// A FigCraft document, all pages or one, with the active lint findings when the query reads
/// them.
pub struct FigTree<'a> {
    pub doc: &'a Document,
    pub page: Option<NodeId>,
    pub lint: Option<HashMap<NodeId, Vec<&'static str>>>,
}

fn css_color(c: Color) -> String {
    c.to_css_hex().to_lowercase()
}

/// The first visible solid fill, as `#rrggbb` (`#rrggbbaa` when not opaque).
fn solid_fill(n: &Node) -> Option<String> {
    n.fills.iter().filter(|p| p.visible).find_map(|p| p.solid_color()).map(css_color)
}

fn image_fill(n: &Node) -> Option<&str> {
    n.fills.iter().filter(|p| p.visible).find_map(|p| match &p.kind {
        PaintKind::Image(i) => Some(i.image.as_str()),
        _ => None,
    })
}

fn is_text(n: &Node) -> bool {
    matches!(n.kind, NodeKind::Text(_))
}

impl FigTree<'_> {
    fn node(&self, id: NodeId) -> Option<&Node> {
        self.doc.node(id)
    }

    /// The parent has auto layout and this layer takes part in it.
    fn in_flow(&self, n: &Node) -> bool {
        n.parent.and_then(|p| self.node(p)).is_some_and(|p| p.layout.is_some())
    }

    /// The table behind `@style.<key>` and `@layout.<key>`, and the named attributes that read it.
    fn decl(&self, n: &Node, key: &str) -> Option<Val> {
        let num = |v: f64| Some(Val::Num(v));
        let s = |v: &str| Some(Val::Str(v.to_string()));
        let al = n.layout.as_ref();
        let text = match &n.kind {
            NodeKind::Text(t) => Some(t),
            _ => None,
        };
        match key {
            "width" => num(n.width),
            "height" => num(n.height),
            "display" => al.map(|l| Val::Str(if l.mode == LayoutMode::Grid { "grid" } else { "flex" }.into())),
            "flexDirection" => al.and_then(|l| match l.mode {
                LayoutMode::Horizontal => s("row"),
                LayoutMode::Vertical => s("column"),
                LayoutMode::Grid => None,
            }),
            "flexWrap" => al.filter(|l| l.wrap).and_then(|_| s("wrap")),
            "gap" => al.and_then(|l| num(l.gap)),
            "rowGap" | "columnGap" => al.filter(|l| l.wrap || l.mode == LayoutMode::Grid).and_then(|l| num(l.cross_gap)),
            "paddingTop" => al.and_then(|l| num(l.padding[0])),
            "paddingRight" => al.and_then(|l| num(l.padding[1])),
            "paddingBottom" => al.and_then(|l| num(l.padding[2])),
            "paddingLeft" => al.and_then(|l| num(l.padding[3])),
            "padding" => al.and_then(|l| {
                let [t, r, b, lf] = l.padding;
                if t == r && r == b && b == lf { num(t) } else { s(&format!("{t}px {r}px {b}px {lf}px")) }
            }),
            "position" => (n.child_layout.absolute && self.in_flow(n)).then(|| Val::Str("absolute".into())),
            "flexGrow" => (self.in_flow(n) && [n.child_layout.sizing_h, n.child_layout.sizing_v].contains(&Sizing::Fill)).then_some(Val::Num(1.0)),
            "opacity" => num(f64::from(n.opacity)),
            "borderRadius" => match n.corners.uniform() {
                Some(r) => num(r),
                None => {
                    let [a, b, c, d] = n.corners.radii;
                    s(&format!("{a}px {b}px {c}px {d}px"))
                }
            },
            "overflow" => n.clip.then(|| Val::Str("hidden".into())),
            "color" => text.and(solid_fill(n)).map(Val::Str),
            "background" | "backgroundColor" => (!is_text(n)).then(|| solid_fill(n)).flatten().map(Val::Str),
            "fontSize" => text.and_then(|t| num(t.style.font_size)),
            "fontFamily" => text.map(|t| Val::Str(t.style.font_family.clone())),
            "fontStyle" => text.map(|t| Val::Str(t.style.font_style.clone())),
            "textAlign" => text.map(|t| {
                Val::Str(
                    match t.align_h {
                        TextAlignH::Left => "left",
                        TextAlignH::Center => "center",
                        TextAlignH::Right => "right",
                        TextAlignH::Justified => "justify",
                    }
                    .into(),
                )
            }),
            _ => None,
        }
    }

    /// The component an instance shows, or the component itself.
    fn component<'b>(&'b self, n: &'b Node) -> Option<&'b Node> {
        match &n.kind {
            NodeKind::Instance { component, .. } => self.node(*component),
            NodeKind::Component { .. } => Some(n),
            _ => None,
        }
    }

    fn hit(&self, id: NodeId) -> Hit {
        let n = self.node(id);
        let parent = n.and_then(|n| n.parent);
        Hit {
            id: json!(id.0),
            name: n.map(|n| n.name.clone()).unwrap_or_default(),
            kind: n.map_or("", |n| n.kind.type_name()).to_lowercase(),
            element: n.map_or("", |n| n.kind.type_name()).to_string(),
            parent_id: parent.map_or(Value::Null, |p| json!(p.0)),
            rect: self.doc.world_bounds(id).map(|r| HitRect::new(r.x0, r.y0, r.width(), r.height())),
            measured_at: None,
            stale: false,
        }
    }
}

impl Tree for FigTree<'_> {
    type Id = NodeId;

    fn roots(&self) -> Vec<Vec<NodeId>> {
        let pages: Vec<NodeId> = match self.page {
            Some(p) => vec![p],
            None => self.doc.pages.clone(),
        };
        pages.into_iter().map(|p| self.children(p)).collect()
    }

    fn children(&self, id: NodeId) -> Vec<NodeId> {
        self.node(id).map(|n| n.children.iter().copied().filter(|c| self.doc.contains(*c)).collect()).unwrap_or_default()
    }

    fn is(&self, id: NodeId, test: &str) -> bool {
        let Some(n) = self.node(id) else { return false };
        if test == "IMAGE" { image_fill(n).is_some() } else { n.kind.type_name() == test }
    }

    fn attr(&self, id: NodeId, a: &Attr) -> Option<Val> {
        let n = self.node(id)?;
        if let Some(k) = &a.key {
            return self.decl(n, k);
        }
        let s = |v: &str| Some(Val::Str(v.to_string()));
        match a.name.as_str() {
            "id" => Some(Val::Num(id.0 as f64)),
            "name" => s(&n.name),
            "type" => s(&n.kind.type_name().to_lowercase()),
            "text" => match &n.kind {
                NodeKind::Text(t) => s(&t.characters),
                _ => None,
            },
            "src" => image_fill(n).and_then(s),
            "width" => self.decl(n, "width"),
            "height" => self.decl(n, "height"),
            "fill" => solid_fill(n).map(Val::Str),
            "color" => self.decl(n, "color"),
            "background" => self.decl(n, "background"),
            "layout" => self.decl(n, "display"),
            "direction" => self.decl(n, "flexDirection"),
            "position" => self.decl(n, "position"),
            "children" => Some(Val::Num(n.children.len() as f64)),
            "component" => self.component(n).and_then(|c| s(&c.name)),
            "componentId" => self.component(n).map(|c| Val::Num(c.id.0 as f64)),
            "hidden" => (!n.visible).then(|| Val::Str("true".into())),
            "locked" => n.locked.then(|| Val::Str("true".into())),
            "lint" => {
                let mut ids: Vec<&str> = self.lint.as_ref()?.get(&id).cloned().unwrap_or_default();
                ids.sort_unstable();
                ids.dedup();
                (!ids.is_empty()).then(|| Val::Str(ids.join(",")))
            }
            "lintCount" => Some(Val::Num(self.lint.as_ref()?.get(&id).map_or(0, Vec::len) as f64)),
            _ => None,
        }
    }
}

fn query_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let bad = |msg: String| EngineError::BadParams { cmd: "node.query".into(), msg };
    let text = p.get("query").and_then(Value::as_str).ok_or_else(|| bad("`query` is required: a path such as //FRAME".into()))?;
    let limit = match p.get("limit") {
        None | Some(Value::Null) => None,
        Some(v) => match v.as_f64() {
            Some(n) if n.fract() == 0.0 && (1.0..=1000.0).contains(&n) => Some(n),
            _ => return Err(bad("`limit` is a whole number from 1 to 1000".into())),
        },
    };
    let doc = s.doc()?;
    let page = match crate::params::id(p, "page") {
        Some(pg) if doc.node(pg).is_some_and(Node::is_page) => Some(pg),
        Some(pg) => return Err(bad(format!("{} is not a page", pg.0))),
        None => None,
    };
    let out = query_doc(doc, text, limit, page).map_err(|e| EngineError::Other(format!("invalid query: {e}")))?;
    serde_json::to_value(&out).map_err(|e| EngineError::Other(e.to_string()))
}

/// Run a query on a document: every page, or `page` only. `node.query` and the CLI both call it.
pub fn query_doc(doc: &Document, text: &str, limit: Option<f64>, page: Option<NodeId>) -> std::result::Result<QueryOut, QueryError> {
    let q = parse(text, &FIGCRAFT)?;
    // The lint runs once per query, and only when the query reads it.
    let lint = q.reads(super::LINT_ATTRS).then(|| {
        let m = crate::cmd::lint::FigLint { doc };
        crate::cmd::lint::active_by_node(&m, &m.config(), true)
    });
    let tree = FigTree { doc, page, lint };
    Ok(run(&tree, &q, limit, |id| tree.hit(id)))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "node.query",
        "Find layers",
        "{query (e.g. //FRAME[@layout=\"flex\"]//TEXT[contains(@name, \"Price\")]), limit? (1–1000, default 100), page? (a page id; default every page)} → {query, total, returned, truncated, results: [{id, name, type, element, parentId, rect: {x, y, w, h} (page space), stale}]} in document order. Read-only. Paths: /X is a top-level layer of a page, //X anywhere below. Node tests: FRAME GROUP SECTION RECTANGLE ELLIPSE POLYGON STAR LINE VECTOR TEXT BOOLEAN_OPERATION COMPONENT COMPONENT_SET INSTANCE SLICE, IMAGE (a visible image fill), *. Predicates: [@a], [@a=\"x\"], [@a!=\"x\"], [@a > 10], contains(@a,\"x\"), starts-with(@a,\"x\"), and, or, not(), [n], [last()]. Attributes: @id @name @type @text @src @width @height @fill @color @background @layout @direction @position @children @component @componentId @hidden @locked @lint (active lint rule ids, comma-joined) @lintCount, and @style.<key> (width height display flexDirection flexWrap gap rowGap columnGap padding paddingTop/Right/Bottom/Left position flexGrow opacity borderRadius overflow color background backgroundColor fontSize fontFamily fontStyle textAlign)",
        false,
        query_cmd
    )]
}
