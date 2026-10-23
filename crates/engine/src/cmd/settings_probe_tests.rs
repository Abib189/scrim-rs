//! Fix W2's audit probe: which commands change which saved preferences, and whether an agent's call
//! changes them. Run by hand (`cargo test -p figcraft-engine --lib probe -- --ignored --nocapture`);
//! its rows are the table in `docs/design/settings-access.md`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::Session;

fn leaves(v: &Value, path: String, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if path.is_empty() && k == "settings" {
                    leaves(x, String::new(), out);
                } else {
                    leaves(x, if path.is_empty() { k.clone() } else { format!("{path}.{k}") }, out);
                }
            }
        }
        _ => {
            out.insert(path, v.clone());
        }
    }
}

fn print(s: &Session) -> BTreeMap<String, Value> {
    let mut v = serde_json::to_value(&s.prefs).unwrap();
    v.as_object_mut().unwrap().remove("diagnostics");
    let mut out = BTreeMap::new();
    leaves(&v, String::new(), &mut out);
    out
}

#[test]
#[ignore]
fn probe() {
    let params = [
        json!({}),
        json!({"on": true}),
        json!({"on": false}),
        json!({"theme": "dark"}),
        json!({"theme": "light"}),
        json!({"preset": "blue"}),
        json!({"value": "#3366ff"}),
        json!({"speed": "slow"}),
        json!({"motion": "off"}),
        json!({"motion": "on"}),
        json!({"small": 2, "big": 20}),
        json!({"name": "Ada"}),
        json!({"language": "es"}),
        json!({"seconds": 30}),
        json!({"minutes": 30}),
        json!({"days": 1}),
        json!({"url": "wss://example.com/"}),
        json!({"server": "wss://example.com/"}),
        json!({"port": 7700}),
        json!({"tool": "export", "on": false}),
        json!({"group": "write", "on": false}),
        json!({"dir": "/tmp"}),
        json!({"family": "Inter"}),
        json!({"show": true}),
        json!({"show": false}),
        json!({"section": "recents", "view": "list"}),
        json!({"sort": "name"}),
        json!({"order": "asc"}),
        json!({"mode": "dev"}),
        json!({"tab": "ai"}),
        json!({"agent": "claude-code", "path": "/usr/bin/true"}),
        json!({"tools": ["frame"]}),
        json!({"left": 300, "right": 300}),
        json!({"height": 200}),
        json!({"require": true}),
        json!({"auth": true}),
        json!({"path": "/tmp"}),
        json!({"tool": "frame", "after": "move"}),
        json!({"tools": ["move", "frame"]}),
        json!({"group": "shapes", "tools": ["rectangle"]}),
        json!({"id": "frame"}),
        json!({"id": "frame", "index": 0}),
        json!({"server": "wss://example.com/", "confirm": true}),
    ];
    let skip =
        ["file.open", "file.save", "file.saveAs", "file.import", "file.export", "file.exportSvg", "file.placeImage", "image.place", "file.revert"];
    let mut rows: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for spec in crate::cmd::all_commands() {
        if skip.contains(&spec.id) {
            continue;
        }
        for p in &params {
            let mut s = Session::new();
            let _ = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10}));
            let before = print(&s);
            let _ = s.execute(spec.id, p);
            let after = print(&s);
            for (k, v) in &after {
                if before.get(k) != Some(v) {
                    rows.entry(spec.id)
                        .or_default()
                        .insert(k.split('.').next().unwrap_or(k).to_string() + if k.starts_with("mcp.") { k.trim_start_matches("mcp") } else { "" });
                }
            }
            for k in before.keys() {
                if !after.contains_key(k) {
                    rows.entry(spec.id).or_default().insert(k.clone());
                }
            }
        }
    }
    for (id, keys) in &rows {
        // Can an agent change it today: any call that succeeds and changes a saved preference.
        let changes = params.iter().any(|p| {
            let mut s = Session::new();
            let _ = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10}));
            s.set_author(crate::agents::Author::agent("mcp (http)"));
            let before = print(&s);
            s.execute(id, p).is_ok() && print(&s) != before
        });
        let refused = !changes;
        println!("ROW\t{id}\t{}\t{}", keys.iter().cloned().collect::<Vec<_>>().join(", "), if refused { "refused" } else { "allowed" });
    }
}
