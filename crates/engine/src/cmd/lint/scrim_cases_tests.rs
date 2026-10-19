//! Scrim's own lint cases (`docs/scrim-transfer/cases/lint.json`): 88 cases, every expectation a
//! real output of Scrim's tools, replayed on Scrim's documents through this engine.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::{Css, Decls, ExtraMap, IgnoreEdit, Kind, LintConfig, Model, Profile, Rect, ignore, lint_design, rules, set_profile};

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/cases/lint.json");

fn cases() -> Value {
    serde_json::from_str(&std::fs::read_to_string(PATH).unwrap()).unwrap()
}

/// JSON that keeps object keys in file order (this workspace's `serde_json` sorts them, and
/// `style-shorthand-conflict` reports pairs in declaration order).
enum OVal {
    Other,
    Arr(Vec<OVal>),
    Obj(Vec<(String, OVal)>),
}

impl<'de> serde::Deserialize<'de> for OVal {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = OVal;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON")
            }
            fn visit_bool<E>(self, _: bool) -> Result<OVal, E> {
                Ok(OVal::Other)
            }
            fn visit_i64<E>(self, _: i64) -> Result<OVal, E> {
                Ok(OVal::Other)
            }
            fn visit_u64<E>(self, _: u64) -> Result<OVal, E> {
                Ok(OVal::Other)
            }
            fn visit_f64<E>(self, _: f64) -> Result<OVal, E> {
                Ok(OVal::Other)
            }
            fn visit_str<E>(self, _: &str) -> Result<OVal, E> {
                Ok(OVal::Other)
            }
            fn visit_unit<E>(self) -> Result<OVal, E> {
                Ok(OVal::Other)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<OVal, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element()? {
                    v.push(x);
                }
                Ok(OVal::Arr(v))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<OVal, A::Error> {
                let mut v = Vec::new();
                while let Some((k, x)) = a.next_entry::<String, OVal>()? {
                    v.push((k, x));
                }
                Ok(OVal::Obj(v))
            }
        }
        d.deserialize_any(V)
    }
}

impl OVal {
    fn get(&self, k: &str) -> Option<&OVal> {
        match self {
            OVal::Obj(v) => v.iter().find(|(kk, _)| kk == k).map(|(_, x)| x),
            _ => None,
        }
    }
    fn keys(&self) -> Vec<String> {
        match self {
            OVal::Obj(v) => v.iter().map(|(k, _)| k.clone()).collect(),
            _ => Vec::new(),
        }
    }
}

/// Per case, per node: the file order of its `layout` and `style` keys.
type KeyOrder = HashMap<String, (Vec<String>, Vec<String>)>;

fn key_orders() -> Vec<KeyOrder> {
    let o: OVal = serde_json::from_str(&std::fs::read_to_string(PATH).unwrap()).unwrap();
    let Some(OVal::Arr(cases)) = o.get("cases") else { return Vec::new() };
    cases
        .iter()
        .map(|c| match c.get("doc").and_then(|d| d.get("nodes")) {
            Some(OVal::Obj(nodes)) => nodes
                .iter()
                .map(|(id, n)| {
                    (id.clone(), (n.get("layout").map(OVal::keys).unwrap_or_default(), n.get("style").map(OVal::keys).unwrap_or_default()))
                })
                .collect(),
            _ => HashMap::new(),
        })
        .collect()
}

/// A Scrim document with the case's rects.
pub(crate) struct ScrimDoc<'a> {
    pub doc: &'a Value,
    pub rects: Option<&'a Value>,
    pub order: Option<&'a KeyOrder>,
}

fn map(v: &Value) -> Decls {
    v.as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| match v {
            Value::String(s) => Some((k.clone(), Css::Str(s.clone()))),
            Value::Number(n) => n.as_f64().map(|n| (k.clone(), Css::Num(n))),
            _ => None,
        })
        .collect()
}

impl<'a> ScrimDoc<'a> {
    fn node(&self, id: &str) -> Option<&'a Value> {
        self.doc["nodes"].get(id)
    }
    /// A node's `layout` or `style` map in file order.
    fn ordered(&self, id: &str, which: &str) -> Decls {
        let Some(n) = self.node(id) else { return Vec::new() };
        let mut d = map(&n[which]);
        if let Some((l, s)) = self.order.and_then(|o| o.get(id)) {
            let keys = if which == "layout" { l } else { s };
            d.sort_by_key(|(k, _)| keys.iter().position(|x| x == k).unwrap_or(usize::MAX));
        }
        d
    }
}

impl<'a> Model for ScrimDoc<'a> {
    type Id = &'a str;

    fn root_groups(&self) -> Vec<Vec<&'a str>> {
        vec![self.doc["rootIds"].as_array().unwrap().iter().filter_map(Value::as_str).collect()]
    }
    fn resolve(&self, raw: &Value) -> Option<&'a str> {
        let s = raw.as_str()?;
        self.doc["nodes"].as_object()?.get_key_value(s).map(|(k, _)| k.as_str())
    }
    fn exists(&self, id: &'a str) -> bool {
        self.node(id).is_some()
    }
    fn parent(&self, id: &'a str) -> Option<&'a str> {
        self.node(id)?["parentId"].as_str()
    }
    fn is_root(&self, id: &'a str) -> bool {
        self.parent(id).is_none()
    }
    fn children(&self, id: &'a str) -> Vec<&'a str> {
        self.node(id).map(|n| n["childIds"].as_array().unwrap().iter().filter_map(Value::as_str).collect()).unwrap_or_default()
    }
    fn name(&self, id: &'a str) -> String {
        self.node(id).and_then(|n| n["name"].as_str()).unwrap_or("").to_string()
    }
    fn json_id(&self, id: &'a str) -> Value {
        json!(id)
    }
    fn id_text(&self, id: &'a str) -> String {
        id.to_string()
    }
    fn kind(&self, id: &'a str) -> Kind {
        match self.node(id).and_then(|n| n["type"].as_str()) {
            Some("text") => Kind::Text,
            Some("image") => Kind::Image,
            _ => Kind::Other,
        }
    }
    fn text(&self, id: &'a str) -> Option<String> {
        self.node(id)?["text"].as_str().map(str::to_string)
    }
    fn layout(&self, id: &'a str) -> Decls {
        self.ordered(id, "layout")
    }
    fn style(&self, id: &'a str) -> Decls {
        self.ordered(id, "style")
    }
    fn extra_maps(&self, id: &'a str) -> Vec<ExtraMap> {
        let Some(n) = self.node(id) else { return vec![] };
        let mut out = Vec::new();
        for (state, m) in n["states"].as_object().into_iter().flatten() {
            out.push(ExtraMap { prefix: format!("{state}: "), state: Some(state.clone()), decls: map(m) });
        }
        for (q, block) in n["media"].as_object().into_iter().flatten() {
            if block.get("style").is_some_and(Value::is_object) {
                out.push(ExtraMap { prefix: format!("{q}: "), state: None, decls: map(&block["style"]) });
            }
            for (state, m) in block["states"].as_object().into_iter().flatten() {
                out.push(ExtraMap { prefix: format!("{q} {state}: "), state: Some(state.clone()), decls: map(m) });
            }
        }
        out
    }
    fn rect(&self, id: &'a str) -> Option<Rect> {
        let r = self.rects?.get(id)?;
        Some(Rect { x: r["x"].as_f64()?, y: r["y"].as_f64()?, w: r["width"].as_f64()?, h: r["height"].as_f64()? })
    }
}

/// The document's stored lint configuration.
pub(crate) fn config<'a>(m: &ScrimDoc<'a>) -> LintConfig<&'a str> {
    let mut cfg = LintConfig::default();
    let lint = &m.doc["lint"];
    for (root, p) in lint["profiles"].as_object().into_iter().flatten() {
        if let (Some(r), Some(p)) = (m.resolve(&json!(root)), p.as_str().and_then(Profile::parse)) {
            cfg.profiles.push((r, p));
        }
    }
    for i in lint["ignores"].as_array().into_iter().flatten() {
        let node = i.get("nodeId").and_then(|n| m.resolve(n).or_else(|| n.as_str().map(|_| "")));
        cfg.apply_ignore(IgnoreEdit {
            rule_id: i["ruleId"].as_str().unwrap().into(),
            node,
            reason: i["reason"].as_str().map(str::to_string),
            index: None,
        })
        .unwrap();
    }
    cfg
}

fn config_json<'a>(m: &ScrimDoc<'a>, cfg: &LintConfig<&'a str>) -> Value {
    if cfg.is_empty() {
        return Value::Null;
    }
    let profiles: serde_json::Map<String, Value> = cfg.profiles.iter().map(|(r, p)| (r.to_string(), json!(p.as_str()))).collect();
    json!({"lint": {"profiles": profiles, "ignores": cfg.ignores_json(m)}})
}

/// One tool call against the case's document and its (changing) configuration.
fn call<'a>(m: &ScrimDoc<'a>, cfg: &mut LintConfig<&'a str>, tool: &str, args: &Value) -> Result<Value, String> {
    match tool {
        "lint_design" => {
            let target = match args.get("id") {
                None => None,
                Some(raw) => Some(m.resolve(raw).ok_or_else(|| format!("node not found: {}", raw.as_str().unwrap_or("")))?),
            };
            let over = args.get("profile").and_then(Value::as_str).and_then(Profile::parse);
            // The server passes its rects, or an empty map: the rendered rules always run.
            Ok(lint_design(m, cfg, target, over, true))
        }
        "set_lint_profile" => {
            let p = Profile::parse(args["profile"].as_str().unwrap()).unwrap();
            set_profile(m, cfg, args["ids"].as_array().unwrap(), p).map(|(v, _)| v)
        }
        "ignore_lint" => ignore(
            m,
            cfg,
            args["ruleId"].as_str().unwrap(),
            args.get("nodeId"),
            args.get("reason").and_then(Value::as_str),
            args.get("remove").and_then(Value::as_bool).unwrap_or(false),
        ),
        other => Err(format!("no tool {other}")),
    }
}

#[test]
fn every_lint_case_matches_scrim() {
    let c = cases();
    let all = c["cases"].as_array().unwrap();
    let orders = key_orders();
    let mut failures = Vec::new();
    for (ci, case) in all.iter().enumerate() {
        let name = case["name"].as_str().unwrap();
        let m = ScrimDoc { doc: &case["doc"], rects: case["rects"].as_object().map(|_| &case["rects"]), order: orders.get(ci) };
        let mut cfg = config(&m);
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let got = call(&m, &mut cfg, step["tool"].as_str().unwrap(), &step["args"]);
            let want = &step["expect"];
            let ok = match &got {
                Ok(v) => want["isError"] == false && *v == want["result"],
                Err(e) => want["isError"] == true && json!(e) == want["result"],
            };
            if !ok {
                let got = match &got {
                    Ok(v) => serde_json::to_string(v).unwrap(),
                    Err(e) => format!("error {e}"),
                };
                failures.push(format!("{name} step {i} ({}):\n    got  {got}\n    want {}", step["tool"], want["result"]));
            }
        }
        if !case["docAfter"].is_null() {
            let after = config_json(&m, &cfg);
            let want = json!({"lint": case["docAfter"]["lint"]});
            if after != want && !(after.is_null() && case["docAfter"]["lint"].is_null()) {
                failures.push(format!("{name}: config after\n    got  {after}\n    want {want}"));
            }
        }
    }
    assert!(failures.is_empty(), "{} failures in {} cases:\n{}", failures.len(), all.len(), failures.join("\n"));
    assert_eq!(all.len(), 88);
}

#[test]
fn every_rule_a_check_names_is_in_the_catalog() {
    let src = [include_str!("checks.rs"), include_str!("rendered.rs"), include_str!("aggregate.rs"), include_str!("motion.rs")].join("\n");
    for r in rules::RULES {
        assert_eq!(src.contains(&format!("\"{}\"", r.id)), r.reported, "{}", r.id);
    }
    assert_eq!(rules::RULES.len(), 45);
}
