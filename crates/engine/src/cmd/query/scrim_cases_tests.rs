//! Scrim's own cases (`docs/scrim-transfer/cases/query-nodes.json`), replayed on Scrim's documents
//! through this parser and evaluator with Scrim's vocabulary. Every expectation in the file is a
//! real output of Scrim's TypeScript, so a pass is behaviour parity, not a reading of the spec.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use super::eval::{Tree, Val};
use super::parse::Attr;
use super::{CliFail, Hit, LINT_ATTRS, SCRIM, needs_lint_error, parse, run, run_cli};

fn cases() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/cases/query-nodes.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A Scrim document (spec §2) with the caller's lint results.
struct ScrimDoc<'a> {
    doc: &'a Value,
    masters: HashSet<&'a str>,
    lint: Option<HashMap<String, Vec<String>>>,
}

impl<'a> ScrimDoc<'a> {
    fn new(doc: &'a Value, lint: Option<&Value>) -> Self {
        let masters = doc["components"].as_object().into_iter().flatten().filter_map(|(_, d)| d["rootId"].as_str()).collect();
        let lint = lint.map(|l| {
            l.as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_string()).collect()))
                .collect()
        });
        Self { doc, masters, lint }
    }
    fn node(&self, id: &str) -> Option<&'a Value> {
        self.doc["nodes"].get(id)
    }
    fn element(&self, id: &str) -> String {
        if self.masters.contains(id) { "COMPONENT".into() } else { self.node(id).and_then(|n| n["type"].as_str()).unwrap_or("").to_uppercase() }
    }
    fn val(v: &Value) -> Option<Val> {
        match v {
            Value::String(s) => Some(Val::Str(s.clone())),
            Value::Number(n) => n.as_f64().map(Val::Num),
            _ => None,
        }
    }
    /// The declared value: `style` then `layout`; an empty string is absent.
    fn decl(&self, n: &Value, key: &str) -> Option<Val> {
        ["style", "layout"].iter().find_map(|m| n[*m].get(key).filter(|v| v.as_str() != Some("")).and_then(Self::val))
    }
    fn def(&self, id: &str, n: &Value) -> Option<&'a Value> {
        let defs = self.doc["components"].as_object()?;
        if self.masters.contains(id) {
            return defs.values().find(|d| d["rootId"] == id);
        }
        if n["type"] == "instance" { defs.get(n["componentId"].as_str()?) } else { None }
    }
    fn hit(&self, id: &'a str) -> Hit {
        let n = self.node(id).unwrap();
        Hit {
            id: json!(id),
            name: n["name"].as_str().unwrap_or("").into(),
            kind: n["type"].as_str().unwrap_or("").into(),
            element: self.element(id),
            parent_id: n["parentId"].clone(),
            rect: None,
            measured_at: Some(Value::Null),
            stale: true,
        }
    }
}

impl<'a> Tree for ScrimDoc<'a> {
    type Id = &'a str;

    fn roots(&self) -> Vec<Vec<&'a str>> {
        vec![self.doc["rootIds"].as_array().unwrap().iter().filter_map(Value::as_str).filter(|i| self.node(i).is_some()).collect()]
    }
    fn children(&self, id: &'a str) -> Vec<&'a str> {
        let Some(n) = self.node(id) else { return vec![] };
        n["childIds"].as_array().unwrap().iter().filter_map(Value::as_str).filter(|i| self.node(i).is_some()).collect()
    }
    fn is(&self, id: &'a str, test: &str) -> bool {
        self.element(id) == test
    }
    fn attr(&self, id: &'a str, a: &Attr) -> Option<Val> {
        let n = self.node(id)?;
        if let Some(k) = &a.key {
            return self.decl(n, k);
        }
        let raw = |k: &str| n.get(k).and_then(Self::val);
        match a.name.as_str() {
            "id" => Some(Val::Str(id.into())),
            "name" | "type" | "text" | "src" | "alt" => raw(&a.name),
            "width" | "height" => self.decl(n, &a.name),
            "fill" if n["type"] == "text" => self.decl(n, "color"),
            "fill" | "background" => self.decl(n, "background").or_else(|| self.decl(n, "backgroundColor")),
            "color" => self.decl(n, "color"),
            "layout" => self.decl(n, "display"),
            "direction" => self.decl(n, "flexDirection").or_else(|| {
                matches!(self.decl(n, "display"), Some(Val::Str(d)) if d == "flex" || d == "inline-flex").then(|| Val::Str("row".into()))
            }),
            "position" => self.decl(n, "position"),
            "children" => Some(Val::Num(n["childIds"].as_array().map_or(0, Vec::len) as f64)),
            "component" => self.def(id, n).and_then(|d| Self::val(&d["name"])),
            "componentId" => self.def(id, n).and_then(|d| Self::val(&d["id"])),
            "lint" => {
                let mut ids: Vec<String> = self.lint.as_ref()?.get(id).cloned().unwrap_or_default();
                ids.sort();
                ids.dedup();
                (!ids.is_empty()).then(|| Val::Str(ids.join(",")))
            }
            "lintCount" => Some(Val::Num(self.lint.as_ref()?.get(id).map_or(0, Vec::len) as f64)),
            _ => None,
        }
    }
}

/// Scrim's linter on a Scrim document, as the tool runs it (no editor, so no rects): the active
/// rule ids per node, through this fork's port of it (`cmd/lint`).
fn real_lint(doc: &Value) -> Value {
    let m = crate::cmd::lint::scrim_cases_tests::ScrimDoc { doc, rects: None, order: None };
    let cfg = crate::cmd::lint::scrim_cases_tests::config(&m);
    let by = crate::cmd::lint::active_by_node(&m, &cfg, true);
    let mut out = serde_json::Map::new();
    for (id, rules) in by {
        out.insert(id.to_string(), json!(rules));
    }
    Value::Object(out)
}

/// The core evaluator's answer: ids, or the error and its position.
fn core(doc: &Value, query: &str, lint: Option<&Value>) -> Result<Vec<String>, (String, Option<usize>)> {
    let q = parse(query, &SCRIM).map_err(|e| (e.message, e.at))?;
    let t = ScrimDoc::new(doc, lint);
    if q.reads(LINT_ATTRS) && t.lint.is_none() {
        let e = needs_lint_error();
        return Err((e.message, e.at));
    }
    Ok(super::evaluate(&t, &q).into_iter().map(str::to_string).collect())
}

#[test]
fn every_core_case_matches_scrim() {
    let c = cases();
    let mut failures = Vec::new();
    let all = c["cases"].as_array().unwrap();
    for case in all {
        let name = case["name"].as_str().unwrap();
        let doc = &c["documents"][case["doc"].as_str().unwrap()];
        let got = core(doc, case["query"].as_str().unwrap(), case.get("lint"));
        let want = &case["expect"];
        let ok = match (&got, want.get("ids")) {
            (Ok(ids), Some(w)) => json!(ids) == *w,
            (Err((msg, at)), None) => json!(msg) == want["error"] && json!(at) == want["at"],
            _ => false,
        };
        if !ok {
            failures.push(format!("{name}\n    query {:?}\n    got   {got:?}\n    want  {want}", case["query"]));
        }
    }
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), all.len(), failures.join("\n"));
    assert_eq!(all.len(), 244);
}

/// The MCP tool: the result JSON, or the tool-error text. Schema-rejected arguments (`limit` 0,
/// 1001, 2.5; no `query`) are checked for refusal only.
#[test]
fn every_tool_case_matches_scrim() {
    let c = cases();
    let mut checked = 0;
    for case in c["toolCases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let doc = &c["documents"][case["doc"].as_str().unwrap()];
        let args = &case["args"];
        let want = &case["expect"];
        let limit = args.get("limit").map(|l| l.as_f64().unwrap());
        let schema_ok = args["query"].is_string() && limit.is_none_or(|l| l.fract() == 0.0 && (1.0..=1000.0).contains(&l));
        if !schema_ok {
            assert_eq!(want["isError"], true, "{name}");
            checked += 1;
            continue;
        }
        // The linter-dependent case runs this fork's port of Scrim's linter (Port 6).
        let lint = if case["linterDependent"] == true { real_lint(doc) } else { json!({}) };
        let t = ScrimDoc::new(doc, Some(&lint));
        match parse(args["query"].as_str().unwrap(), &SCRIM) {
            Ok(q) => {
                let out = run(&t, &q, limit, |id| t.hit(id));
                assert_eq!(want["isError"], false, "{name}");
                assert_eq!(serde_json::to_value(&out).unwrap(), want["json"], "{name}");
                // The text's key order is pinned by the CLI's `--json` cases, which hold the text.
            }
            Err(e) => {
                assert_eq!(want["isError"], true, "{name}");
                assert_eq!(json!(format!("invalid query: {e}")), want["text"], "{name}");
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 19);
}

/// The CLI: exit code, stdout and stderr, through the same argument rules.
#[test]
fn every_cli_case_matches_scrim() {
    let c = cases();
    let mut checked = 0;
    for case in c["cliCases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let doc = &c["documents"][case["doc"].as_str().unwrap()];
        let argv: Vec<String> = case["argv"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect();
        let Some((cmd, args)) = argv.split_first() else { continue };
        let got = run_cli("scrim", &SCRIM, cmd, args, |_, query, limit| {
            let q = parse(query, &SCRIM).map_err(|e| CliFail::new("invalid_query", format!("invalid query: {e}")))?;
            let lint = real_lint(doc);
            let t = ScrimDoc::new(doc, Some(&lint));
            Ok(run(&t, &q, limit, |id| t.hit(id)))
        });
        let want = &case["expect"];
        assert_eq!(json!(got.exit), want["exit"], "{name}");
        assert_eq!(json!(got.stdout), want["stdout"], "{name}");
        assert_eq!(json!(got.stderr), want["stderr"], "{name}");
        checked += 1;
    }
    assert_eq!(checked, 25);
}
