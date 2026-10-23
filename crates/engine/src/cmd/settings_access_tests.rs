//! Fix W3: a command that writes a saved setting is marked in the registry
//! ([`CommandSpec::setting`]) and refused to agents on every route, with one refusal and one
//! Settings ▸ Diagnostics line; the allowlist ([`AGENT_SETTINGS`]) is short and works; and a
//! command that writes a setting without being marked fails [`every_command_that_writes_a_setting_is_marked`].
//! The audit's probe (`docs/design/settings-access.md`) is here too, run by hand.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::{AGENT_SETTINGS, SETTINGS, SETTINGS_PERSON_ONLY};
use crate::Session;
use crate::agents::Author;

/// The saved preferences that aren't settings: the window's memory and records
/// (`settings-access.md` › Writes that aren't settings).
const NOT_SETTINGS: &[&str] = &[
    "recent_files",
    "online_files",
    "recent_actions",
    "diagnostics",
    "plugin_settings",
    "panel",
    "panel_widths",
    "pages_height",
    "home_views",
    "home_sort",
    "home_order",
    "getting_started_dismissed",
    "plugin_directory",
    "settings_version",
];

fn leaves(v: &Value, path: String, out: &mut BTreeMap<String, Value>) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            for (k, x) in m {
                leaves(x, if path.is_empty() { k.clone() } else { format!("{path}.{k}") }, out);
            }
        }
        _ => {
            out.insert(path, v.clone());
        }
    }
}

/// The saved settings, as leaves (`mcp.port`, `view.view.rulers`…): what an agent must never change.
fn settings(s: &Session) -> BTreeMap<String, Value> {
    let mut v = serde_json::to_value(&s.prefs).unwrap();
    let m = v.as_object_mut().unwrap();
    for k in NOT_SETTINGS {
        m.remove(*k);
    }
    let mut out = BTreeMap::new();
    leaves(&v, String::new(), &mut out);
    out
}

/// Parameters that reach most commands' writes.
fn params() -> Vec<Value> {
    vec![
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
        json!({"tool": "export", "on": true}),
        json!({"group": "write", "on": false}),
        json!({"path": "/tmp"}),
        json!({"show": true}),
        json!({"show": false}),
        json!({"agent": "claude-code", "path": "/usr/bin/true"}),
        json!({"tool": "frame", "after": "move"}),
        json!({"tools": ["move", "frame"]}),
        json!({"id": "frame"}),
    ]
}

/// A session with a document and a layer, as `who`.
fn session(who: Author) -> Session {
    let mut s = Session::new();
    let _ = s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10}));
    s.set_author(who);
    s
}

/// Commands that need what a test can't give (real files, a server).
const SKIP: &[&str] =
    &["file.open", "file.save", "file.saveAs", "file.import", "file.export", "file.exportSvg", "file.placeImage", "image.place", "file.revert"];

/// Each command run as the person with each of [`params`]: the settings it changed.
fn writers() -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut rows: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for spec in crate::cmd::all_commands().iter().filter(|c| !SKIP.contains(&c.id)) {
        for p in params() {
            let mut s = session(Author::Person);
            let before = settings(&s);
            let _ = s.execute(spec.id, &p);
            let after = settings(&s);
            for k in before.keys().chain(after.keys()) {
                if before.get(k) != after.get(k) {
                    rows.entry(spec.id).or_default().insert(k.clone());
                }
            }
        }
    }
    rows
}

#[test]
fn every_command_that_writes_a_setting_is_marked() {
    let unmarked: Vec<String> = writers()
        .into_iter()
        .filter(|(id, _)| !crate::find_command(id).is_some_and(|c| c.setting))
        .map(|(id, keys)| format!("{id} ({})", keys.into_iter().collect::<Vec<_>>().join(", ")))
        .collect();
    assert!(unmarked.is_empty(), "these write a saved setting: add them to `access::SETTINGS` (agents are then refused them): {unmarked:?}");
    // The list holds registered commands only, each marked by the registry.
    for id in SETTINGS {
        assert!(crate::find_command(id).is_some_and(|c| c.setting), "{id}: registered and marked");
    }
    assert_eq!(crate::cmd::all_commands().iter().filter(|c| c.setting).count(), SETTINGS.len());
    // The allowlist is short, real and says why.
    assert!(AGENT_SETTINGS.len() <= 3, "keep the allowlist short");
    for (id, why) in AGENT_SETTINGS {
        assert!(crate::find_command(id).is_some() && why.len() > 20, "{id}: a registered command with its reason");
    }
}

#[test]
fn every_setting_command_is_refused_to_an_agent_on_every_route_and_noted() {
    for route in ["mcp (http)", "mcp", "cli", "control channel", "plugin Tidy \"names\""] {
        for id in SETTINGS.iter().filter(|id| **id != "camera.follow-agents") {
            for p in [json!({}), json!({"on": true}), json!({"on": false, "origin": "settings"})] {
                let mut s = session(Author::agent(route));
                s.agents.session = Some("3f1520b6-77aa".into());
                let before = settings(&s);
                // mcp.set-tool with `on: false` is the allowlist's: tested below.
                if *id == "mcp.set-tool" && p["on"] == false {
                    continue;
                }
                let e = s.execute(id, &p).expect_err("refused").to_string();
                assert!(e.contains(SETTINGS_PERSON_ONLY), "{route} {id} {p}: {e}");
                assert_eq!(settings(&s), before, "{route} {id}: nothing changed");
                let line = s.prefs.settings.diagnostics.last().map(|e| (e.kind.clone(), e.detail.clone())).unwrap_or_default();
                assert_eq!(line.0, "setting", "{route} {id}");
                assert!(line.1.starts_with(&format!("setting {id}: unchanged (refused: an agent asked, ")), "{route} {id}: {}", line.1);
                assert!(!line.1.contains("Tidy") && !line.1.contains("names") && !line.1.contains("origin"), "nothing the agent sent: {}", line.1);
            }
        }
    }
    // Through `command.batch` too.
    let mut s = session(Author::agent("mcp (http)"));
    let e = s.execute("command.batch", &json!({"commands": [{"command": "pref.theme", "params": {"theme": "dark"}}]})).expect_err("refused");
    assert!(e.to_string().contains(SETTINGS_PERSON_ONLY), "{e}");
    assert_eq!(s.prefs.theme, "system");
}

#[test]
fn the_allowlist_lets_an_agent_finish_and_turn_a_tool_off_and_nothing_more() {
    let mut s = session(Author::agent("mcp (http)"));
    assert_eq!(s.execute("agent.finish-working", &json!({})).unwrap()["finished"], true);
    s.execute("mcp.set-tool", &json!({"tool": "export", "on": false})).unwrap();
    assert_eq!(s.prefs.settings.mcp.disabled_tools, ["export"]);
    for p in [json!({"tool": "export", "on": true}), json!({"tool": "open_file"}), json!({"tool": "open_file", "on": "false"})] {
        assert!(s.execute("mcp.set-tool", &p).is_err(), "{p}: only `on: false`");
    }
    assert_eq!(s.prefs.settings.mcp.disabled_tools, ["export"], "never turned back on");
    // The grace's test hook (session only, never saved), and only with the hooks on.
    assert!(s.execute("pref.agentGrace", &json!({"seconds": 5})).is_err());
    s.enable_test_hooks();
    assert_eq!(s.execute("pref.agentGrace", &json!({"seconds": 5})).unwrap()["seconds"], 5);
    assert_eq!(s.prefs.settings.agent_grace_secs, crate::settings::DEFAULT_AGENT_GRACE_SECS, "never saved");
    // The person runs every one of them.
    let mut p = session(Author::Person);
    assert_eq!(p.execute("pref.theme", &json!({"theme": "dark"})).unwrap()["theme"], "dark");
}

#[test]
#[ignore]
fn probe() {
    // Fix W2's audit: which commands change which settings, and whether an agent's call does.
    for (id, keys) in writers() {
        let changes = params().into_iter().any(|p| {
            let mut s = session(Author::agent("mcp (http)"));
            let before = settings(&s);
            s.execute(id, &p).is_ok() && settings(&s) != before
        });
        println!("ROW\t{id}\t{}\t{}", keys.into_iter().collect::<Vec<_>>().join(", "), if changes { "allowed" } else { "refused" });
    }
}
