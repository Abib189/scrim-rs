//! Diagnostics: what each entry says, retention, and the scan the brief asks for: no prompts,
//! document content, paths or tokens ever reach an event.

use serde_json::json;

use super::{Cause, Change, Cleared, Entry, MAX_EVENTS, Pause, Skip, record, text, timestamp};
use crate::agents::Author;
use crate::{EngineError, Session};

const NOW: u64 = 1_792_500_000;

#[test]
fn entries_say_only_their_allow_listed_facts() {
    let mut s = Session::new();
    let st = &mut s.prefs.settings;
    record(st, NOW, Entry::Reload);
    record(st, NOW, Entry::FontFallback(&["Inter".into(), "Bad\u{7}Font".into()]));
    record(st, NOW, Entry::McpStarted(7601));
    record(st, NOW, Entry::McpClient { name: "claude-code", version: "2.1.296" });
    record(st, NOW, Entry::Error { command: "text.set", error: &EngineError::Other("the text was \"hello\"".into()) });
    record(st, NOW, Entry::Error { command: "not a command at all", error: &EngineError::NoDocument });
    record(st, NOW, Entry::Error { command: "file.open", error: &EngineError::Waiting });
    record(st, NOW, Entry::FollowPaused(Pause::CanvasInput));
    record(st, NOW, Entry::FollowCleared(Cleared::AgentFinished));
    record(st, NOW, Entry::Reveal { entered: 20, skipped: &[(Skip::OffScreenPaused, 16), (Skip::CutShort, 2)] });
    let raw = "local-agent-mode-scrim (via mcp-remote 0.14.3)";
    let w = |change, cause, client, session, run| Entry::Working { change, cause, client, session, run };
    record(st, NOW, w(Change::Begin, Cause::FirstRequest, Some(raw), Some("9f8e7d6c5b4a39281706f5e4d3c2b1a0"), true));
    record(st, NOW, w(Change::Hold, Cause::SessionClosed, Some(raw), Some("9f8e7d6c-5b4a"), true));
    record(st, NOW, w(Change::End, Cause::GraceExpired, None, None, false));
    record(st, NOW, Entry::GraceTooShort { client: Some(raw), after: 8, setting: 5 });
    let lines: Vec<(&str, &str)> = st.diagnostics.iter().map(|e| (e.kind.as_str(), e.detail.as_str())).collect();
    assert_eq!(
        lines,
        [
            ("reload", "a file was reverted to its saved copy"),
            ("fontFallback", "fonts not on this computer, a fallback used: Inter, BadFont"),
            ("mcpServer", "started on 127.0.0.1:7601"),
            ("mcpClient", "claude-code 2.1.296 connected"),
            ("error", "text.set: failed"),
            ("error", "a command: no open document"),
            ("follow", "following paused: the person's press, scroll or pinch on the canvas"),
            ("follow", "follow pause cleared: the agent finished"),
            (
                "reveal",
                "agent layers: 20 entered, 18 drawn final at once (off screen, follow paused: 16; entrance cut short (dropped frames, a hidden window or a window capture): 2)"
            ),
            ("working", "begin: local-agent-mode-scrim (via mcp-remote 0.14.3) (session 9f8e7d6c…), first request; the dots are on"),
            ("working", "hold: local-agent-mode-scrim (via mcp-remote 0.14.3) (session 9f8e7d6c…), session closed; the dots are on"),
            ("working", "end: an agent (no session), grace period expired; the dots fade out"),
            ("working", "grace too short: local-agent-mode-scrim (via mcp-remote 0.14.3) came back after 8 s (the setting is 5 s)"),
        ],
        "Waiting isn't an error"
    );
    assert_eq!(timestamp(NOW), "2026-10-20 12:40:00 UTC");
    assert!(text(&st.diagnostics).starts_with("2026-10-20 12:40:00 UTC  reload  "));
}

#[test]
fn retention_drops_old_events_and_zero_keeps_none() {
    let mut s = Session::new();
    assert_eq!(s.prefs.settings.diagnostics_days, 7, "a week by default");
    record(&mut s.prefs.settings, NOW - 8 * 86_400, Entry::Reload);
    record(&mut s.prefs.settings, NOW - 86_400, Entry::McpFailed);
    record(&mut s.prefs.settings, NOW, Entry::Reload);
    assert_eq!(s.prefs.settings.diagnostics.len(), 2, "eight days old: gone");
    s.execute("diagnostics.retention", &json!({"days": 1, "now": NOW})).unwrap();
    assert_eq!(s.prefs.settings.diagnostics.len(), 2, "one day old is still kept");
    s.execute("diagnostics.retention", &json!({"days": 0})).unwrap();
    assert!(s.prefs.settings.diagnostics.is_empty());
    record(&mut s.prefs.settings, NOW, Entry::Reload);
    assert!(s.prefs.settings.diagnostics.is_empty(), "0 keeps nothing");
    assert!(s.execute("diagnostics.retention", &json!({"days": 365})).is_err());
    s.execute("diagnostics.retention", &json!({"days": 30})).unwrap();
    for i in 0..MAX_EVENTS + 5 {
        record(&mut s.prefs.settings, NOW + i as u64, Entry::Reload);
    }
    assert_eq!(s.prefs.settings.diagnostics.len(), MAX_EVENTS);
    // Kept in the preferences, and a bad value read back falls back.
    let back: crate::Prefs = serde_json::from_value(serde_json::to_value(&s.prefs).unwrap()).unwrap();
    assert_eq!(back.settings.diagnostics.len(), MAX_EVENTS);
    let odd: crate::Prefs = serde_json::from_value(json!({"diagnostics_days": 12})).unwrap();
    assert_eq!(odd.settings.diagnostics_days, 12, "read as written");
    assert_eq!(odd.settings.sanitized().diagnostics_days, 7, "then put back in range");
}

#[test]
fn the_commands_list_copy_save_and_clear_and_only_the_person_clears() {
    let mut s = Session::new();
    record(&mut s.prefs.settings, NOW, Entry::McpStarted(7600));
    record(&mut s.prefs.settings, NOW + 1, Entry::Reload);
    let st = s.execute("diagnostics.events", &json!({})).unwrap();
    assert_eq!(st["events"][0]["kind"], "reload", "newest first");
    s.ui_requests.clear();
    assert_eq!(s.execute("diagnostics.copy", &json!({})).unwrap()["lines"], 2);
    assert!(s.ui_requests[0].data["text"].as_str().unwrap().contains("started on 127.0.0.1:7600"));
    s.ui_requests.clear();
    assert_eq!(s.execute("diagnostics.save", &json!({})).unwrap()["asked"], true);
    assert_eq!(s.ui_requests[0].kind, crate::cmd::diagnostics::SAVE_DIALOG);
    let path = std::env::temp_dir().join(format!("scrim-diagnostics-{}.txt", std::process::id()));
    s.execute("diagnostics.save", &json!({"path": path.to_string_lossy()})).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);
    let _ = std::fs::remove_file(&path);
    s.set_author(Author::agent("mcp"));
    assert!(s.execute("diagnostics.clear", &json!({})).unwrap_err().to_string().contains("an agent can't"));
    s.set_author(Author::Person);
    assert_eq!(s.execute("diagnostics.clear", &json!({})).unwrap()["cleared"], 2);
}

/// The brief: "No prompts, document content or tokens in events". Drive every kind of event with
/// secrets on hand (a token, document text, a path, a prompt passed as a parameter, an error
/// message quoting them) and scan what is kept.
#[test]
fn no_secrets_reach_the_events() {
    const DOC: &str = "SECRET-DOC-CONTENT-4417";
    const PROMPT: &str = "SECRET-PROMPT-make-it-pop";
    const PATH: &str = "/Users/someone/Private/SECRET-PATH.fig";
    let mut s = Session::new();
    s.execute("mcp.toggle-auth", &json!({"on": true})).unwrap();
    let token = s.prefs.settings.mcp.token.clone();
    assert!(token.len() >= 32);
    let t = s.execute("shape.text", &json!({"x": 0, "y": 0, "text": DOC})).ok();
    // Commands failing with the secrets in their parameters and messages.
    let failures = [
        ("text.set", json!({"ids": [999_999], "text": DOC})),
        ("file.open", json!({"path": PATH})),
        ("ai.unknown", json!({"prompt": PROMPT})),
        ("mcp.set-root-dir", json!({"path": format!("relative/{token}")})),
    ];
    for (cmd, p) in failures {
        if let Err(e) = s.execute(cmd, &p) {
            let msg = e.to_string();
            record(&mut s.prefs.settings, NOW, Entry::Error { command: cmd, error: &e });
            let _ = msg;
        }
    }
    // An error whose message quotes everything at once.
    let loud = EngineError::Other(format!("{DOC} {PROMPT} {PATH} {token}"));
    record(&mut s.prefs.settings, NOW, Entry::Error { command: PROMPT, error: &loud });
    record(&mut s.prefs.settings, NOW, Entry::Reload);
    record(&mut s.prefs.settings, NOW, Entry::McpClient { name: "claude-code", version: "2.1" });
    record(&mut s.prefs.settings, NOW, Entry::FontFallback(&["Inter".into(), "Geist".into()]));
    record(&mut s.prefs.settings, NOW, Entry::FollowPaused(Pause::PersonEdit));
    record(&mut s.prefs.settings, NOW, Entry::FollowCleared(Cleared::Resumed));
    record(&mut s.prefs.settings, NOW, Entry::Reveal { entered: 3, skipped: &[(Skip::OffScreenOff, 2)] });
    // A working transition on a session id as long as a token: 8 characters of it are kept. (The
    // client's name is what it announced, kept as `mcpClient` keeps it.)
    let client = Some("local-agent-mode-scrim (via mcp-remote 0.14.3)");
    record(
        &mut s.prefs.settings,
        NOW,
        Entry::Working { change: Change::Begin, cause: Cause::FirstRequest, client, session: Some(&token), run: true },
    );
    record(&mut s.prefs.settings, NOW, Entry::GraceTooShort { client, after: 12, setting: 15 });
    let kept = serde_json::to_string(&s.prefs.settings.diagnostics).unwrap() + &text(&s.prefs.settings.diagnostics);
    assert!(s.prefs.settings.diagnostics.len() >= 6, "{kept}");
    for secret in [DOC, PROMPT, PATH, "SECRET", "/Users/", token.as_str()] {
        assert!(!kept.contains(secret), "{secret:?} reached the events: {kept}");
    }
    // Nothing that looks like a token (a long run of hex digits) either.
    let longest_hex = kept.split(|c: char| !c.is_ascii_hexdigit()).map(str::len).max().unwrap_or(0);
    assert!(longest_hex < 16, "{kept}");
    let _ = t;
}

#[test]
fn the_settings_carry_a_version_and_odd_values_fall_back() {
    let s = Session::new();
    let v = serde_json::to_value(&s.prefs).unwrap();
    assert_eq!(v["settings_version"], crate::settings::SETTINGS_VERSION);
    let read: crate::Prefs = serde_json::from_value(json!({"settings_version": 9, "motion": "wobbly", "mcp": {"port": 80}})).unwrap();
    let s = read.settings.sanitized();
    assert_eq!((s.settings_version, s.motion.as_str(), s.mcp.port), (crate::settings::SETTINGS_VERSION, "system", crate::settings::DEFAULT_MCP_PORT));
}

#[test]
fn an_agent_runs_lines_name_a_known_agent_how_it_ended_and_nothing_it_said() {
    let mut st = crate::settings::Settings::default();
    record(&mut st, NOW, Entry::ChatRun { agent: "Claude Code", ended: None });
    record(&mut st, NOW, Entry::ChatRun { agent: "Codex", ended: Some(("finished", Some(0), 42)) });
    record(&mut st, NOW, Entry::ChatRun { agent: "make my secret logo", ended: Some(("whatever it said", None, 3)) });
    let lines: Vec<(&str, &str)> = st.diagnostics.iter().map(|e| (e.kind.as_str(), e.detail.as_str())).collect();
    assert_eq!(
        lines,
        [
            ("chat", "Claude Code run started"),
            ("chat", "Codex run ended: finished, exit 0, 42 s"),
            ("chat", "an agent run ended: ended, no exit code, 3 s"),
        ],
        "only a headless profile's name and the four endings; any other text is dropped"
    );
}
