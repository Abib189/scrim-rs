use serde_json::json;

use super::output::{Event, Outcome, parse};
use super::*;

const URL: &str = "http://127.0.0.1:7690/mcp";

#[test]
fn claude_code_and_codex_are_the_profiles_in_that_order() {
    let ids: Vec<&str> = profiles().iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["claude-code", "codex"]);
    for p in profiles() {
        assert!(!p.name.is_empty() && !p.program.is_empty() && p.docs.starts_with("https://"), "{}", p.id);
        assert!(!p.client_names.is_empty(), "{}: the clientInfo it announces", p.id);
        assert!(p.args.iter().any(|a| a.contains("{prompt}")), "{}: the prompt goes in", p.id);
        assert!(!p.verified.is_empty(), "{}: where the flags were read", p.id);
    }
}

#[test]
fn claude_code_runs_in_print_mode_with_only_scrims_server_and_no_built_in_tools() {
    let p = profile("claude-code").unwrap();
    let a = p.argv("make it blue", URL, false);
    let after = |flag: &str| a.iter().position(|x| x == flag).and_then(|i| a.get(i + 1)).cloned();
    assert_eq!(a.first().map(String::as_str), Some("-p"));
    assert_eq!(after("-p").as_deref(), Some("make it blue"), "the prompt is one argument");
    assert_eq!(after("--output-format").as_deref(), Some("stream-json"));
    assert!(a.contains(&"--strict-mcp-config".to_string()) && a.contains(&"--no-session-persistence".to_string()));
    assert_eq!(after("--tools").as_deref(), Some(""), "every built-in tool off");
    assert_eq!(after("--allowedTools").as_deref(), Some("mcp__scrim"));
    assert_eq!(after("--permission-mode").as_deref(), Some("dontAsk"));
    let config: serde_json::Value = serde_json::from_str(&after("--mcp-config").unwrap()).unwrap();
    assert_eq!(config, json!({"mcpServers": {"scrim": {"type": "http", "url": URL}}}));
}

#[test]
fn with_authentication_on_the_token_is_named_never_written() {
    let token = "s3cr3t-token-value";
    for p in profiles() {
        let a = p.argv("x", URL, true);
        assert!(a.iter().all(|x| !x.contains(token)), "{}", p.id);
        assert!(a.iter().any(|x| x.contains(TOKEN_VAR)), "{}: the variable is named", p.id);
    }
    let config = mcp_config(URL, true);
    assert!(config.contains("Bearer ${SCRIM_MCP_AUTH}"), "{config}");
    let env = child_env(Some("/usr/bin"), Some("/h"), Some(token), &[]);
    assert!(env.contains(&(TOKEN_VAR.to_string(), token.to_string())), "the token is in the child's environment");
}

#[test]
fn codex_gets_its_server_by_config_overrides_and_its_auth_only_when_on() {
    let p = profile("codex").unwrap();
    let a = p.argv("x", URL, false);
    assert_eq!(a.first().map(String::as_str), Some("exec"));
    for flag in ["--json", "--ephemeral", "--skip-git-repo-check", "--ignore-user-config"] {
        assert!(a.contains(&flag.to_string()), "{flag}");
    }
    assert!(a.contains(&format!("mcp_servers.scrim.url=\"{URL}\"")));
    assert_eq!(a.last().map(String::as_str), Some("x"), "the prompt last");
    assert!(!a.iter().any(|x| x.contains("bearer_token_env_var")));
    let a = p.argv("x", URL, true);
    assert!(a.contains(&"mcp_servers.scrim.bearer_token_env_var=\"SCRIM_MCP_AUTH\"".to_string()));
}

#[test]
fn a_prompt_is_never_split_or_interpreted() {
    let nasty = "a; rm -rf ~ && $(whoami) `id` \"quoted\" {mcp_url} --tools default";
    for p in profiles() {
        let a = p.argv(nasty, URL, false);
        assert_eq!(a.iter().filter(|x| x.as_str() == nasty).count(), 1, "{}: the whole prompt, one argument, unchanged", p.id);
        assert!(!a.contains(&"default".to_string()), "{}: nothing split out of it", p.id);
    }
}

#[test]
fn the_confirmation_shows_the_binary_and_every_argument_with_a_placeholder_for_the_prompt() {
    let shown = profile("claude-code").unwrap().display("/opt/bin/claude", URL, false);
    assert!(shown.starts_with("/opt/bin/claude -p"), "{shown}");
    assert!(shown.contains(PROMPT_SHOWN) && shown.contains("--strict-mcp-config") && shown.contains(URL), "{shown}");
    assert!(shown.contains("--tools '' --allowedTools"), "the empty argument shows: {shown}");
}

#[test]
fn the_environment_is_path_home_the_token_and_the_child_mark_only() {
    let env = child_env(Some("/usr/bin:/bin"), Some("/Users/x"), None, &[]);
    let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["PATH", "HOME", CHILD_VAR]);
    assert!(env.contains(&(CHILD_VAR.to_string(), "1".to_string())));
    let win = child_env(Some("p"), Some("h"), None, &[("SYSTEMROOT".into(), "C:\\Windows".into()), ("PATH".into(), "dup".into())]);
    assert_eq!(win.iter().filter(|(k, _)| k == "PATH").count(), 1, "no variable twice");
    assert!(win.iter().any(|(k, _)| k == "SYSTEMROOT"));
}

#[test]
fn the_context_line_names_the_page_and_the_selection_and_nothing_else() {
    let mut s = Session::new();
    let f = s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100, "name": "Poster"})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 300, "y": 0, "width": 10, "height": 10, "name": "Not selected"})).unwrap();
    s.execute("select.set", &json!({"ids": [f["id"]]})).unwrap();
    let line = context_line(&s);
    assert!(line.starts_with("Scrim: page “Page 1”; selected: “Poster” ("), "{line}");
    assert!(!line.contains("Not selected"), "{line}");
    s.execute("select.none", &json!({})).unwrap();
    assert_eq!(context_line(&s), "Scrim: page “Page 1”; nothing is selected.");
    assert!(message("  make it blue ", &s).starts_with("make it blue\n\nScrim: page"));
}

#[test]
fn error_output_is_shown_with_tokens_redacted() {
    let t = "abcd1234efgh5678";
    assert_eq!(redact("auth failed for abcd1234efgh5678", Some(t)), "auth failed for <token>");
    assert_eq!(redact("Authorization: Bearer eyJhbGciOi.payload.sig", None), "Authorization: Bearer <token>");
    assert_eq!(redact("invalid key sk-ant-api03-xxxxxxxxxxxx", None), "invalid key sk-<token>");
    assert_eq!(redact(r#"{"api_key": "AIzaSyD-ABCDEFG123"}"#, None), r#"{"api_key": "<token>"}"#);
    assert_eq!(redact("Not logged in · Please run /login", None), "Not logged in · Please run /login");
    assert_eq!(redact(&"x".repeat(500), None).chars().count(), 300);
}

#[test]
fn claude_code_events_read_as_a_reply_its_usage_and_cost() {
    let o = super::Output::ClaudeStreamJson;
    assert_eq!(
        parse(o, r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"scrim","status":"connected"}]}"#),
        [Event::Started { scrim_connected: Some(true) }]
    );
    assert_eq!(
        parse(o, r#"{"type":"system","subtype":"init","mcp_servers":[{"name":"scrim","status":"failed"}]}"#),
        [Event::Started { scrim_connected: Some(false) }]
    );
    assert_eq!(
        parse(o, r#"{"type":"assistant","message":{"content":[{"type":"text","text":"ok"},{"type":"tool_use","name":"mcp__scrim__batch"}]}}"#),
        [Event::Tool("mcp__scrim__batch".into())]
    );
    let end = parse(
        o,
        r#"{"type":"result","subtype":"success","is_error":false,"result":"Made it.","total_cost_usd":0.0123,"usage":{"input_tokens":1200,"output_tokens":80}}"#,
    );
    assert_eq!(
        end,
        [
            Event::Reply("Made it.".into()),
            Event::Usage { input: Some(1200), output: Some(80) },
            Event::Cost(0.0123),
            Event::Finished { error: false }
        ]
    );
    assert_eq!(parse(o, r#"{"type":"result","subtype":"error_max_turns","is_error":false}"#), [Event::Finished { error: true }]);
    assert!(parse(o, "not json").is_empty() && parse(o, r#"{"type":"user"}"#).is_empty());
}

#[test]
fn codex_events_read_as_a_reply_its_usage_and_failures() {
    let o = super::Output::CodexJsonl;
    assert_eq!(parse(o, r#"{"type":"thread.started","thread_id":"t"}"#), [Event::Started { scrim_connected: None }]);
    assert_eq!(parse(o, r#"{"type":"item.completed","item":{"id":"1","type":"agent_message","text":"Done."}}"#), [Event::Reply("Done.".into())]);
    assert_eq!(
        parse(o, r#"{"type":"item.completed","item":{"type":"mcp_tool_call","server":"scrim","tool":"batch"}}"#),
        [Event::Tool("batch".into())]
    );
    assert_eq!(
        parse(o, r#"{"type":"turn.completed","usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":3}}"#),
        [Event::Usage { input: Some(10), output: Some(3) }, Event::Finished { error: false }]
    );
    assert_eq!(parse(o, r#"{"type":"turn.failed","error":{"message":"stream error"}}"#), [Event::Failed("stream error".into())]);
    assert_eq!(parse(o, r#"{"type":"error","message":"401 Unauthorized"}"#), [Event::Failed("401 Unauthorized".into())]);
}

#[test]
fn an_outcome_keeps_only_what_was_reported() {
    let mut out = Outcome::default();
    for e in parse(super::Output::ClaudeStreamJson, r#"{"type":"result","subtype":"success","result":"Hi"}"#) {
        out.add(e);
    }
    assert_eq!(out.reply.as_deref(), Some("Hi"));
    assert_eq!((out.input_tokens, out.output_tokens, out.cost_usd), (None, None, None), "nothing estimated");
    assert_eq!(out.finished, Some(false));
}
