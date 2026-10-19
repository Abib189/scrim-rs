//! Scrim's `analyze_design` cases (`docs/scrim-transfer/cases/analyze-design.json`): 8 cases on 5
//! documents, every expectation a real output of Scrim's tool, replayed on Scrim's documents.

use serde_json::{Value, json};

use super::{Source, Tokens, analyze};
use crate::cmd::lint::Css;

/// A Scrim document.
struct ScrimDoc<'a> {
    doc: &'a Value,
}

fn css(v: &Value) -> Option<Css> {
    match v {
        Value::String(s) => Some(Css::Str(s.clone())),
        Value::Number(n) => n.as_f64().map(Css::Num),
        _ => None,
    }
}

impl<'a> ScrimDoc<'a> {
    fn node(&self, id: &str) -> Option<&'a Value> {
        self.doc["nodes"].get(id)
    }
}

impl<'a> Source for ScrimDoc<'a> {
    type Id = &'a str;

    fn roots(&self) -> Vec<&'a str> {
        self.doc["rootIds"].as_array().unwrap().iter().filter_map(Value::as_str).collect()
    }
    fn exists(&self, id: &'a str) -> bool {
        self.node(id).is_some()
    }
    fn parent(&self, id: &'a str) -> Option<&'a str> {
        self.node(id)?["parentId"].as_str()
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
    fn kind(&self, id: &'a str) -> String {
        self.node(id).and_then(|n| n["type"].as_str()).unwrap_or("").to_string()
    }
    fn decl(&self, id: &'a str, key: &str) -> Option<Css> {
        let n = self.node(id)?;
        let v = n["style"].get(key).and_then(css).or_else(|| n["layout"].get(key).and_then(css));
        v.filter(|v| !matches!(v, Css::Str(s) if s.is_empty()))
    }
    fn svg_paints(&self, id: &'a str) -> Vec<String> {
        let Some(svg) = self.node(id).map(|n| &n["svg"]).filter(|s| s.is_object()) else { return vec![] };
        let mut out = Vec::new();
        fn walk(shapes: &Value, out: &mut Vec<String>) {
            for sh in shapes.as_array().into_iter().flatten() {
                for k in ["fill", "stroke"] {
                    if let Some(p) = sh[k].as_str()
                        && p != "none"
                        && !p.starts_with("url(")
                    {
                        out.push(p.to_string());
                    }
                }
                if sh["tag"] == "g" {
                    walk(&sh["children"], out);
                }
            }
        }
        walk(&svg["children"], &mut out);
        for d in svg["defs"].as_array().into_iter().flatten() {
            if d["tag"] == "clipPath" || d["tag"] == "mask" {
                continue;
            }
            for st in d["stops"].as_array().into_iter().flatten() {
                if let Some(c) = st["color"].as_str() {
                    out.push(c.to_string());
                }
            }
        }
        out
    }
    fn is_component(&self, id: &'a str) -> bool {
        let master = self.doc["components"].as_object().into_iter().flatten().any(|(_, c)| c["rootId"] == id);
        master || self.node(id).is_some_and(|n| n["type"] == "instance")
    }
    fn tokens(&self) -> Tokens {
        let t = &self.doc["tokens"];
        let pairs = |v: &Value| -> Vec<(String, Value)> { v.as_object().into_iter().flatten().map(|(k, v)| (k.clone(), v.clone())).collect() };
        Tokens {
            colors: pairs(&t["colors"]).into_iter().filter_map(|(k, v)| Some((k, v.as_str()?.to_string()))).collect(),
            typography: pairs(&t["typography"])
                .into_iter()
                .map(|(k, v)| (k, pairs(&v).into_iter().filter_map(|(p, x)| Some((p, css(&x)?))).collect()))
                .collect(),
            spacing: pairs(&t["spacing"]).into_iter().filter_map(|(k, v)| Some((k, css(&v)?))).collect(),
        }
    }
}

#[test]
fn every_analyze_case_matches_scrim() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/cases/analyze-design.json");
    let c: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let all = c["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in all {
        let name = case["name"].as_str().unwrap();
        let src = ScrimDoc { doc: &case["doc"] };
        for step in case["steps"].as_array().unwrap() {
            let want = &step["expect"];
            let got = match step["args"].get("id") {
                None => Ok(analyze(&src, None)),
                Some(raw) => {
                    let id = raw.as_str().unwrap();
                    match src.doc["nodes"].as_object().unwrap().get_key_value(id) {
                        Some((k, _)) => Ok(analyze(&src, Some(k.as_str()))),
                        None => Err(format!("node not found: {id}")),
                    }
                }
            };
            let ok = match &got {
                Ok(v) => want["isError"] == false && *v == want["result"],
                Err(e) => want["isError"] == true && json!(e) == want["result"],
            };
            if !ok {
                let got = got.map_or_else(|e| e, |v| serde_json::to_string(&v).unwrap());
                failures.push(format!("{name}:\n    got  {got}\n    want {}", want["result"]));
            }
        }
    }
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
    assert_eq!(all.len(), 8);
}
