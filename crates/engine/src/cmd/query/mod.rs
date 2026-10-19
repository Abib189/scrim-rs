//! `node.query`: find layers with a small, closed subset of XPath, Scrim's `query_nodes` (Port 5).
//! The spec is `docs/scrim-transfer/specs/query-nodes.md` and its messages are
//! `query-nodes-errors.md`. For example, `//FRAME[@layout="flex"]//TEXT[contains(@name, "Price")]`.
//!
//! It must never:
//! - **change the document.** A query only reads.
//! - **run the query as code.** It is never turned into a regular expression and never fetches anything.
//! - **answer outside the subset with an empty result.** Anything else is an error that says what
//!   is supported.
//!
//! It is a hand-written lexer and a recursive-descent parser, with no XPath library.
//!
//! The parser and the evaluator are generic: a [`Vocab`] gives the node tests and attributes, and
//! a [`Tree`] gives the document. `figcraft.rs` maps FigCraft's document (the choices are in
//! `docs/scrim-ports/query-nodes.md`). The tests replay Scrim's own cases, on Scrim's documents,
//! through the same parser and evaluator with Scrim's vocabulary (`scrim_cases_tests.rs`).

mod cli;
mod eval;
mod figcraft;
mod lex;
mod parse;

pub use cli::{CliFail, CliOutcome, run_cli};
pub use eval::{Tree, Val, evaluate, js_number, numeric};
pub use figcraft::{FIGCRAFT, FigTree, query_doc, specs};
pub use parse::{Attr, Axis, Expr, Lit, Op, Pred, Query, Step, parse};

/// A query that isn't in the subset: the message, ending in ` (at character N)` when it has a
/// position. `at` is that N: 1-based, counted in UTF-16 code units, as Scrim counts.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryError {
    pub message: String,
    pub at: Option<usize>,
}

impl QueryError {
    fn new(message: impl Into<String>, at: Option<usize>) -> Self {
        let message = message.into();
        match at {
            Some(n) => Self { message: format!("{message} (at character {n})"), at },
            None => Self { message, at },
        }
    }
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// What a model lets a query name: its node tests and its attributes. Error messages list them.
#[derive(Clone, Copy, Debug)]
pub struct Vocab {
    /// The node tests besides `*`, as written: `FRAME`, `TEXT` …
    pub tests: &'static [&'static str],
    /// The plain attributes (`@name`), without the `@`. `@style.<key>` and `@layout.<key>` are
    /// always accepted, and bare `@layout` is too when `layout` is in the list.
    pub attrs: &'static [&'static str],
}

impl Vocab {
    fn tests_list(&self) -> String {
        format!("{} or *", self.tests.join(", "))
    }
    fn attrs_list(&self) -> String {
        let attrs: Vec<String> = self.attrs.iter().map(|a| format!("@{a}")).collect();
        format!("{}, @style.<key> or @layout.<key>", attrs.join(", "))
    }
}

/// Scrim's vocabulary: its five node types and `COMPONENT` (a component master).
pub const SCRIM: Vocab = Vocab {
    tests: &["FRAME", "BOX", "TEXT", "IMAGE", "INSTANCE", "COMPONENT"],
    attrs: &[
        "id",
        "name",
        "type",
        "text",
        "src",
        "alt",
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
        "lint",
        "lintCount",
    ],
};

/// The attributes that read lint results.
pub const LINT_ATTRS: &[&str] = &["lint", "lintCount"];

/// E36: a query reads lint and the caller has none to give.
pub fn needs_lint_error() -> QueryError {
    QueryError::new("@lint and @lintCount need lint results: query through MCP (query_nodes) or the CLI", None)
}

/// The cap on results per call: `limit` (default 100) within 1..=1000.
pub fn cap(limit: Option<f64>) -> usize {
    limit.map_or(100.0, f64::floor).clamp(1.0, 1000.0) as usize
}

/// A layer's box: border-box, in page coordinates, rounded to 2 decimals.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct HitRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl HitRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        let r = |v: f64| (v * 100.0).round() / 100.0;
        Self { x: r(x), y: r(y), w: r(w), h: r(h) }
    }
}

/// One match, as the tool returns it. The fields serialise in this order (the spec's).
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Hit {
    pub id: serde_json::Value,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// The node-test name it answers to.
    pub element: String,
    #[serde(rename = "parentId")]
    pub parent_id: serde_json::Value,
    pub rect: Option<HitRect>,
    /// Scrim's editor-measured time (`Some(null)` there); FigCraft's boxes come from the document
    /// itself, so it leaves the field out.
    #[serde(rename = "measuredAt", skip_serializing_if = "Option::is_none")]
    pub measured_at: Option<serde_json::Value>,
    pub stale: bool,
}

/// The tool's result. `total` counts every match; `results` holds at most the cap.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct QueryOut {
    pub query: String,
    pub total: usize,
    pub returned: usize,
    pub truncated: bool,
    pub results: Vec<Hit>,
}

impl QueryOut {
    /// The tool's text: 2-space indented, keys in the spec's order.
    pub fn text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Run a parsed query and shape the result (spec §11): every match counted, the first `cap`
/// described by `hit`.
pub fn run<T: Tree>(t: &T, q: &Query, limit: Option<f64>, hit: impl Fn(T::Id) -> Hit) -> QueryOut {
    let ids = evaluate(t, q);
    let results: Vec<Hit> = ids.iter().take(cap(limit)).map(|i| hit(*i)).collect();
    QueryOut { query: q.text.clone(), total: ids.len(), returned: results.len(), truncated: ids.len() > results.len(), results }
}

#[cfg(test)]
#[path = "scrim_cases_tests.rs"]
mod scrim_cases_tests;

#[cfg(test)]
#[path = "figcraft_tests.rs"]
mod figcraft_tests;
