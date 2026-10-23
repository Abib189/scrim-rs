//! R5b: the composer's commands reach the window from the person only. Through the control
//! channel and the app's MCP route, an agent is refused each of them and the window isn't asked.

use serde_json::json;

use crate::control::{ROUTE_MCP_HTTP, handle, handle_route};
use crate::layers_drag_tests::Ui;

const WORDS: &str = "Describe a change is the person's: agents can't start, stop or set up agent runs";

#[test]
fn the_control_channel_and_the_mcp_route_are_refused_every_chat_command() {
    let mut u = Ui::new(figcraft_engine::Session::new());
    let ctx = u.ctx.clone();
    for cmd in figcraft_engine::cmd::chat::PERSON_ONLY {
        let p = json!({"command": cmd, "params": {"prompt": "x", "agent": "claude-code", "path": "/x/claude", "on": false, "minutes": 30}});
        for r in [handle(&mut u.app, &ctx, "engine.execute", &p), handle_route(&mut u.app, &ctx, ROUTE_MCP_HTTP, "engine.execute", &p)] {
            let e = r.unwrap_err();
            // A command that writes a setting meets Fix W3's refusal first; the others their own.
            let want =
                if figcraft_engine::find_command(cmd).is_some_and(|s| s.setting) { "only the person changes Scrim's saved settings" } else { WORDS };
            assert!(e.contains(want), "{cmd}: {e}");
        }
    }
    assert!(u.app.session.prefs.settings.describe_change, "nothing changed");
    assert_eq!(u.app.session.prefs.settings.chat_time_limit_mins, 15);
    assert!(u.app.session.ui_requests.iter().all(|r| !r.kind.starts_with("chat.")));
}

#[test]
fn the_person_may() {
    let mut u = Ui::new(figcraft_engine::Session::new());
    assert!(u.app.run("pref.chatTimeLimit", json!({"minutes": 30})).is_some());
    assert_eq!(u.app.session.prefs.settings.chat_time_limit_mins, 30);
}

/// Fix W3: a command that writes a saved setting is refused through the control channel and the
/// app's MCP route like through MCP's own server, and Settings ▸ Diagnostics notes each attempt.
#[test]
fn the_control_channel_and_the_mcp_route_are_refused_every_setting_command() {
    let mut u = Ui::new(figcraft_engine::Session::new());
    let ctx = u.ctx.clone();
    for (cmd, p) in [("pref.theme", json!({"theme": "dark"})), ("pref.revealSpeed", json!({"speed": "slow"})), ("view.rulers", json!({"on": true}))] {
        let p = json!({"command": cmd, "params": p});
        for (route, r) in [
            ("control channel", handle(&mut u.app, &ctx, "engine.execute", &p)),
            ("MCP", handle_route(&mut u.app, &ctx, ROUTE_MCP_HTTP, "engine.execute", &p)),
        ] {
            let e = r.unwrap_err();
            assert!(e.contains("only the person changes Scrim's saved settings"), "{route} {cmd}: {e}");
            // Noted (the working state's own line may follow it).
            let noted = u
                .app
                .session
                .prefs
                .settings
                .diagnostics
                .iter()
                .rev()
                .take(3)
                .any(|e| e.detail.starts_with(&format!("setting {cmd}: unchanged (refused: an agent asked, over")));
            assert!(noted, "{route} {cmd}: {:?}", u.app.session.prefs.settings.diagnostics.iter().rev().take(3).collect::<Vec<_>>());
        }
    }
    assert_eq!((u.app.session.prefs.theme.as_str(), u.app.session.prefs.settings.reveal_speed.as_str()), ("system", "normal"));
}
