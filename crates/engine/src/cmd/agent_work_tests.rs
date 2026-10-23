//! Stage I.1: `agent.finish-working` asks the window to end the caller's working state and changes
//! nothing else (no revision, undo or document); agents may run it; it is classified Read. The
//! grace period takes 15, 30, 60 or 120 s, 60 by default (Fix T1, `settings_version` 8); a grace
//! nobody chose follows the default, a chosen one stays.

use serde_json::{Value, json};

use super::FINISH_REQUEST;
use crate::Session;
use crate::agents::Author;
use crate::cmd::access::{Access, access_of};

#[test]
fn finish_working_asks_the_window_and_changes_nothing() {
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    let (revision, undo) = (s.ds().unwrap().revision, s.undo_depth());
    let doc = s.ds().unwrap().doc.clone();
    s.agents.client = Some("codex-mcp-client".into());
    s.agents.session = Some("abc".into());
    s.ui_requests.clear();
    let r = s.execute_as(Author::agent("mcp (http)"), None, "agent.finish-working", &json!({"ids": [2]})).unwrap();
    assert_eq!(r, json!({"finished": true, "client": "codex-mcp-client", "ids": [2]}));
    let asked: Vec<_> = s.ui_requests.iter().filter(|u| u.kind == FINISH_REQUEST).collect();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].data, json!({"client": "codex-mcp-client", "session": "abc", "route": "mcp (http)", "ids": [2]}));
    assert_eq!(s.ds().unwrap().revision, revision, "no revision");
    assert_eq!(s.undo_depth(), undo, "no undo step");
    assert!(std::sync::Arc::ptr_eq(&doc, &s.ds().unwrap().doc), "the document untouched");
    assert!(s.execute("agent.finish-working", &json!({"ids": "x"})).is_err(), "ids is a list");
    assert_eq!(access_of(crate::find_command("agent.finish-working").unwrap()), Some(Access::Read));
    assert_eq!(access_of(crate::find_command("pref.agentGrace").unwrap()), Some(Access::Write));
}

#[test]
fn the_grace_period_is_15_30_60_or_120_s_60_by_default_and_the_persons_choice_is_marked() {
    let mut s = Session::new();
    assert_eq!(s.prefs.settings.agent_grace_secs, 60, "60 s by default (Fix T1)");
    assert!(!s.prefs.settings.agent_grace_chosen);
    for secs in [15, 30, 60, 120] {
        assert_eq!(s.execute("pref.agentGrace", &json!({"seconds": secs})).unwrap(), json!({"seconds": secs}));
    }
    for bad in [0, 4, 5, 10, 61, 3600] {
        assert!(s.execute("pref.agentGrace", &json!({"seconds": bad})).is_err(), "{bad} refused");
    }
    assert_eq!((s.prefs.settings.agent_grace_secs, s.prefs.settings.agent_grace_chosen), (120, true), "the person's choice, marked");
    let v = serde_json::to_value(&s.prefs.settings).unwrap();
    assert_eq!(
        (v["agent_grace_secs"].clone(), v["agent_grace_chosen"].clone(), v["settings_version"].clone()),
        (json!(120), json!(true), json!(crate::settings::SETTINGS_VERSION))
    );
    let back: crate::settings::Settings = serde_json::from_value(v).unwrap();
    assert_eq!(back.sanitized().agent_grace_secs, 120, "round-trips");
    // Out of range in a file: the nearest choice.
    for (stored, read) in [(45, 30), (500, 120), (90, 60)] {
        let f: crate::settings::Settings =
            serde_json::from_value(json!({"agent_grace_secs": stored, "agent_grace_chosen": true, "settings_version": 8})).unwrap();
        assert_eq!(f.sanitized().agent_grace_secs, read, "{stored} s in a file reads {read} s");
    }
}

#[test]
fn migration_raises_an_unchosen_grace_below_60_and_never_lowers_one() {
    let read = |v: Value| serde_json::from_value::<crate::settings::Settings>(v).unwrap().sanitized().agent_grace_secs;
    // Version 8 (Fix T1, U1): a grace below 60 s that nobody chose follows the default up to 60 s.
    for unmarked in [5, 15, 30] {
        assert_eq!(read(json!({"settings_version": 7, "agent_grace_secs": unmarked})), 60, "unmarked {unmarked}");
    }
    assert_eq!(read(json!({"settings_version": 6, "agent_grace_secs": 5})), 60, "5, not chosen, from version 6");
    // The migration never lowers a grace: an unmarked 60 or 120 stays.
    for unmarked in [60, 120] {
        assert_eq!(read(json!({"settings_version": 7, "agent_grace_secs": unmarked})), unmarked, "unmarked {unmarked} stays");
    }
    // A chosen value always stays.
    assert_eq!(read(json!({"settings_version": 7, "agent_grace_secs": 30, "agent_grace_chosen": true})), 30, "a chosen 30 stays");
    assert_eq!(read(json!({"settings_version": 7, "agent_grace_secs": 15, "agent_grace_chosen": true})), 15, "a chosen 15 stays");
    assert_eq!(read(json!({"settings_version": 6, "agent_grace_secs": 15, "agent_grace_chosen": true})), 15, "chosen 15 stays");
    assert_eq!(read(json!({"settings_version": 7, "agent_grace_secs": 120, "agent_grace_chosen": true})), 120, "a chosen 120 stays");
    assert_eq!(
        read(json!({"settings_version": 6, "agent_grace_secs": 5, "agent_grace_chosen": true})),
        15,
        "chosen 5, no longer offered: the nearest"
    );
    assert_eq!(read(json!({"settings_version": 4})), 60, "a file from before the grace");
    assert_eq!(read(json!({"settings_version": 8, "agent_grace_secs": 15})), 15, "after version 8, 15 is a choice like any");
}

#[test]
fn agents_cant_set_the_grace_and_the_test_hook_is_never_saved() {
    let mut s = Session::new();
    s.set_author(crate::agents::Author::agent("mcp (http)"));
    let e = s.execute("pref.agentGrace", &json!({"seconds": 60})).unwrap_err().to_string();
    assert!(e.contains("only the person changes Scrim's saved settings"), "{e}");
    assert_eq!((s.prefs.settings.agent_grace_secs, s.prefs.settings.agent_grace_chosen), (60, false));
    // Tests: 5 s through the hook, for the session only.
    s.enable_test_hooks();
    assert_eq!(s.execute("pref.agentGrace", &json!({"seconds": 5})).unwrap()["seconds"], 5);
    assert_eq!(s.agent_grace(), 5);
    assert_eq!(s.prefs.settings.agent_grace_secs, 60, "never in the settings");
    assert_eq!(serde_json::to_value(&s.prefs.settings).unwrap()["agent_grace_secs"], 60, "never saved");
    // Without the hook, 5 is refused even for the person.
    let mut p = Session::new();
    assert!(p.execute("pref.agentGrace", &json!({"seconds": 5})).is_err());
}
