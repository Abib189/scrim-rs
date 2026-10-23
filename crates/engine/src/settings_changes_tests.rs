//! Fix W4: each origin of a setting change is noted, with plain values only.

use serde_json::json;

use crate::Session;
use crate::agents::Author;

fn last(s: &Session) -> String {
    s.prefs.settings.diagnostics.last().map(|e| format!("{}: {}", e.kind, e.detail)).unwrap_or_default()
}

#[test]
fn each_origin_of_a_setting_change_is_noted() {
    let mut s = Session::new();
    // The person's command (a menu, the palette).
    s.execute("pref.theme", &json!({"theme": "dark"})).unwrap();
    assert_eq!(last(&s), "setting: setting theme: system → dark (made by the person's command)");
    // The person's Settings: the window says it is drawing Settings.
    s.agents.from_settings = true;
    s.execute("pref.revealSpeed", &json!({"speed": "slow"})).unwrap();
    assert_eq!(last(&s), "setting: setting reveal_speed: normal → slow (made by the person's Settings)");
    s.execute("view.rulers", &json!({"on": true})).unwrap();
    assert_eq!(last(&s), "setting: setting view.rulers: none → true (made by the person's Settings)");
    s.agents.from_settings = false;
    // A value that says who the person is, or a secret, is never shown.
    s.execute("pref.userName", &json!({"name": "Ada Lovelace"})).unwrap();
    assert_eq!(last(&s), "setting: setting user_name: changed (made by the person's command)");
    s.execute("mcp.toggle-auth", &json!({"on": true})).unwrap();
    let lines: Vec<String> = s.prefs.settings.diagnostics.iter().rev().take(2).map(|e| e.detail.clone()).collect();
    assert!(lines.contains(&"setting mcp.token: changed (made by the person's command)".to_string()), "{lines:?}");
    assert!(lines.contains(&"setting mcp.require_auth: false → true (made by the person's command)".to_string()), "{lines:?}");
    assert!(s.prefs.settings.diagnostics.iter().all(|e| !e.detail.contains("Ada") && !e.detail.contains(&s.prefs.settings.mcp.token)));
    // Nothing changed: nothing noted.
    let n = s.prefs.settings.diagnostics.len();
    s.execute("pref.theme", &json!({"theme": "dark"})).unwrap();
    assert_eq!(s.prefs.settings.diagnostics.len(), n);
    // A settings migration: the unmarked 30 s grace raised (Fix U1).
    let old: crate::settings::Settings = serde_json::from_value(json!({"settings_version": 7, "agent_grace_secs": 30})).unwrap();
    let details: Vec<String> = old.sanitized().diagnostics.into_iter().map(|e| e.detail).collect();
    assert_eq!(details, ["setting agent_grace_secs: 30 → 60 (made by a settings migration)"]);
    // Following has its own line, never this one too.
    s.execute("camera.follow-agents", &json!({"on": false, "origin": "button"})).unwrap();
    assert_eq!(last(&s), "follow: follow setting: ON → OFF (made by the person's button)");
    assert!(s.prefs.settings.diagnostics.iter().all(|e| !e.detail.starts_with("setting follow")));
}

/// An agent can't pose as the person: whatever it sends, and even while the window is drawing
/// Settings, it is refused and the line says so; nothing is noted as the person's.
#[test]
fn an_agent_cant_pose_as_the_person() {
    let mut s = Session::new();
    s.agents.from_settings = true;
    s.set_author(Author::agent("mcp (http)"));
    for p in [json!({"theme": "dark", "origin": "settings"}), json!({"theme": "dark", "from_settings": true, "author": "person"})] {
        assert!(s.execute("pref.theme", &p).is_err());
        assert!(last(&s).starts_with("setting: setting pref.theme: unchanged (refused: an agent asked, over MCP"), "{}", last(&s));
    }
    assert_eq!(s.prefs.theme, "system");
    assert!(s.prefs.settings.diagnostics.iter().all(|e| !e.detail.contains("made by the person")));
}
