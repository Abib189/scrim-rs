//! Stage I.1: `agent.finish-working` asks the window to end the caller's working state and changes
//! nothing else (no revision, undo or document); agents may run it; it is classified Read. The
//! grace period takes 5, 15, 30 or 60 s, is saved with `settings_version` 5, and an old file reads
//! 15 s.

use serde_json::json;

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
fn the_grace_period_is_one_of_four_saved_and_never_under_5_s() {
    let mut s = Session::new();
    assert_eq!(s.prefs.settings.agent_grace_secs, 15, "15 s by default");
    for secs in [5, 15, 30, 60] {
        assert_eq!(s.execute("pref.agentGrace", &json!({"seconds": secs})).unwrap(), json!({"seconds": secs}));
    }
    for bad in [0, 4, 10, 61, 3600] {
        assert!(s.execute("pref.agentGrace", &json!({"seconds": bad})).is_err(), "{bad} refused");
    }
    assert_eq!(s.prefs.settings.agent_grace_secs, 60);
    let v = serde_json::to_value(&s.prefs.settings).unwrap();
    assert_eq!((v["agent_grace_secs"].clone(), v["settings_version"].clone()), (json!(60), json!(5)));
    let back: crate::settings::Settings = serde_json::from_value(v).unwrap();
    assert_eq!(back.sanitized().agent_grace_secs, 60, "round-trips");
    let old: crate::settings::Settings = serde_json::from_value(json!({"settings_version": 4})).unwrap();
    assert_eq!(old.sanitized().agent_grace_secs, 15, "a version-4 file reads 15 s");
    for (stored, read) in [(1, 5), (3, 5), (12, 15), (45, 30), (500, 60)] {
        let f: crate::settings::Settings = serde_json::from_value(json!({"agent_grace_secs": stored})).unwrap();
        assert_eq!(f.sanitized().agent_grace_secs, read, "{stored} s in a file reads {read} s");
    }
}
