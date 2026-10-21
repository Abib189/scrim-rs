//! J2/J3: the client profiles, on scratch folders only (never a real configuration).
//! - Every profile is well formed and labelled honestly (only a live run makes "verified live").
//! - Each file client's path per OS, with its environment overrides.
//! - Entries and command lines for the live address, with and without a token (an env-var
//!   reference where the client documents one, the token itself only where it doesn't).
//! - Changing JSON, JSONC (comments and trailing commas kept) and TOML (comments and order kept):
//!   missing, empty and invalid files; other servers untouched; the same entry again; a different
//!   one replaced; the old entry removed; a container that doesn't exist yet.
//! - The safe write: a backup, no temporary file left, a missing folder made, an invalid file left
//!   alone, nothing written inside an agent's request or for a read-only profile.
//! - The confirm-only commands refuse agents; nothing reads a file without the app's environment.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Change, KEY, Os, PathEnv, State, apply, changed, cli_add, file_entry, file_path, inspect, profile, profiles, resolve};
use crate::Session;
use crate::agents::Author;

const URL: &str = "http://127.0.0.1:7681/mcp";

fn env(vars: &[(&str, &str)]) -> PathEnv {
    PathEnv {
        home: Some(PathBuf::from("/h")),
        appdata: Some(PathBuf::from("/appdata")),
        vars: vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>(),
    }
}

fn file(id: &str) -> super::FileSpec {
    profile(id).and_then(|p| p.file.clone()).unwrap_or_else(|| panic!("{id} has a file"))
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("scrim-agent-clients-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn names(d: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

#[test]
fn every_profile_is_well_formed_and_labelled_honestly() {
    let all = profiles();
    assert!(all.len() >= 16, "{}", all.len());
    for p in all {
        assert!(!p.docs.is_empty() && p.docs.iter().all(|d| d.starts_with("https://")), "{}: its docs", p.id);
        assert!(["verified-live", "docs-only", "untested"].contains(&p.verification.as_str()), "{}", p.id);
        assert_eq!(p.verified.len(), 10, "{}: a date", p.id);
        // Only a recorded live run makes "verified live" (`docs/design/connect-any-agent.md`).
        if p.verification == "verified-live" {
            assert!(["claude-code", "codex"].contains(&p.id.as_str()), "{} claims a live run that isn't recorded", p.id);
        }
        assert!(p.builtin.is_some() || p.cli.is_some() || p.file.is_some() || p.snippet.is_some(), "{}: some way to connect", p.id);
        if let Some(f) = &p.file {
            assert!(["json", "jsonc", "toml"].contains(&f.format.as_str()) && !f.container.is_empty() && !f.url_keys.is_empty(), "{}", p.id);
        }
    }
    let ids: Vec<&str> = all.iter().map(|p| p.id.as_str()).collect();
    for id in ["codex", "gemini", "cursor", "windsurf", "vscode", "cline", "zed", "opencode", "grok"] {
        assert!(ids.contains(&id), "{id} has a profile");
    }
}

#[test]
fn each_file_clients_path_per_os() {
    let e = env(&[]);
    let at = |id: &str, os: Os, e: &PathEnv| file_path(&file(id), os, e).map(|(p, d)| (p.to_string_lossy().into_owned(), d));
    assert_eq!(at("codex", Os::Mac, &e), Some(("/h/.codex/config.toml".into(), true)));
    assert_eq!(at("codex", Os::Windows, &e), Some(("/h/.codex/config.toml".into(), false)), "Windows isn't documented beyond CODEX_HOME");
    assert_eq!(at("codex", Os::Linux, &env(&[("CODEX_HOME", "/c")])), Some(("/c/config.toml".into(), true)));
    assert_eq!(at("gemini", Os::Linux, &env(&[("GEMINI_CLI_HOME", "/g")])), Some(("/g/.gemini/settings.json".into(), true)));
    assert_eq!(at("opencode", Os::Mac, &e), Some(("/h/.config/opencode/opencode.json".into(), true)));
    assert_eq!(at("opencode", Os::Linux, &env(&[("XDG_CONFIG_HOME", "/x")])), Some(("/x/opencode/opencode.json".into(), true)));
    assert_eq!(at("opencode", Os::Windows, &e).map(|p| p.1), Some(false), "inferred on Windows");
    assert_eq!(at("cursor", Os::Windows, &e), Some(("/h/.cursor/mcp.json".into(), true)));
    assert_eq!(at("vscode", Os::Mac, &e), Some(("/h/Library/Application Support/Code/User/mcp.json".into(), false)), "inferred, never written");
    assert_eq!(at("vscode", Os::Windows, &e), Some(("/appdata/Code/User/mcp.json".into(), false)));
    assert_eq!(at("cline", Os::Mac, &e), Some(("/h/.cline/data/settings/cline_mcp_settings.json".into(), true)));
    assert_eq!(at("cline", Os::Mac, &env(&[("CLINE_MCP_SETTINGS_PATH", "/s/mcp.json")])), Some(("/s/mcp.json".into(), true)));
    assert_eq!(at("zed", Os::Mac, &e), Some(("/h/.config/zed/settings.json".into(), true)));
    assert_eq!(at("zed", Os::Linux, &env(&[("XDG_CONFIG_HOME", "/x")])), Some(("/x/zed/settings.json".into(), true)));
    assert_eq!(at("zed", Os::Windows, &e), Some(("/appdata/Zed/settings.json".into(), true)));
    assert_eq!(at("grok", Os::Linux, &env(&[("GROK_HOME", "/gk")])), Some(("/gk/config.toml".into(), true)));
    assert_eq!(at("devin", Os::Windows, &e), Some(("/appdata/devin/mcp_config.json".into(), true)));
    assert_eq!(at("amp", Os::Linux, &e), Some(("/h/.config/amp/settings.json".into(), true)));
    // Without a home folder, nothing resolves (no guessing).
    assert_eq!(file_path(&file("cursor"), Os::Mac, &PathEnv::default()), None);
    assert_eq!(resolve("{env:NOPE}/x", &e), None, "an unset variable without a fallback");
}

#[test]
fn entries_and_command_lines_for_the_live_address_and_the_token() {
    // A token as an env-var reference where the client documents one.
    assert_eq!(file_entry(&file("cursor"), URL, None), json!({"url": URL}));
    assert_eq!(file_entry(&file("cursor"), URL, Some("T")), json!({"url": URL, "headers": {"Authorization": "Bearer ${env:SCRIM_MCP_TOKEN}"}}));
    assert_eq!(file_entry(&file("opencode"), URL, Some("T"))["headers"]["Authorization"], "Bearer {env:SCRIM_MCP_TOKEN}");
    assert_eq!(file_entry(&file("codex"), URL, Some("T")), json!({"url": URL, "bearer_token_env_var": "SCRIM_MCP_TOKEN"}));
    // The token itself only where there's no documented reference (the panel warns first).
    assert_eq!(file_entry(&file("zed"), URL, Some("T"))["headers"]["Authorization"], "Bearer T");
    assert_eq!(profile("zed").map(super::token_style), Some("literal"));
    // Command lines: argument lists, the live address, never a shell string.
    let cli = |id: &str| profile(id).and_then(|p| p.cli.clone()).unwrap();
    assert_eq!(cli_add(&cli("codex"), URL, None).unwrap(), ["mcp", "add", "scrim", "--url", URL]);
    assert_eq!(cli_add(&cli("codex"), URL, Some("T")).unwrap(), ["mcp", "add", "scrim", "--url", URL, "--bearer-token-env-var", "SCRIM_MCP_TOKEN"]);
    assert_eq!(cli_add(&cli("gemini"), URL, Some("T")).unwrap().last().map(String::as_str), Some("Authorization: Bearer ${SCRIM_MCP_TOKEN}"));
    let vs = cli_add(&cli("vscode"), URL, Some("T")).unwrap();
    let arg: Value = serde_json::from_str(&vs[1]).unwrap();
    assert_eq!(arg, json!({"name": "scrim", "type": "http", "url": URL, "headers": {"Authorization": "Bearer ${env:SCRIM_MCP_TOKEN}"}}));
    assert_eq!(cli_add(&cli("devin"), URL, Some("T")), None, "no documented header option: Copy instead");
    assert_eq!(super::cli_remove(&cli("codex"), "figcraft").unwrap(), ["mcp", "remove", "figcraft"]);
}

#[test]
fn jsonc_edits_keep_comments_and_other_servers() {
    let f = file("zed");
    let entry = file_entry(&f, URL, None);
    // Missing and empty files: created.
    for before in [None, Some(""), Some("  \n")] {
        let t = changed(&f, before, &Change::Set(KEY.into(), entry.clone())).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&t).unwrap(), json!({"context_servers": {"scrim": {"url": URL}}}));
    }
    // A Zed settings file with comments, other settings, another server and a trailing comma.
    let before = r#"// Zed settings
{
  // The theme.
  "theme": "One Dark",
  "context_servers": {
    /* my tool */
    "other": { "command": "x", "args": [] },
  },
  "vim_mode": true
}
"#;
    let t = changed(&f, Some(before), &Change::Set(KEY.into(), entry.clone())).unwrap();
    for kept in ["// Zed settings", "// The theme.", "/* my tool */", r#""other": { "command": "x", "args": [] },"#, r#""vim_mode": true"#] {
        assert!(t.contains(kept), "{kept:?} kept in:\n{t}");
    }
    let (state, old) = inspect(&f, Some(&t), URL, None);
    assert_eq!((state, old), (State::Configured, vec![]));
    // The same entry again: the same text.
    assert_eq!(changed(&f, Some(&t), &Change::Set(KEY.into(), entry.clone())).unwrap(), t);
    // A different address: Out of date, replaced in place.
    let other = changed(&f, Some(before), &Change::Set(KEY.into(), json!({"url": "http://127.0.0.1:9/mcp"}))).unwrap();
    assert_eq!(inspect(&f, Some(&other), URL, None).0, State::OutOfDate { target: "http://127.0.0.1:9/mcp".into() });
    let fixed = changed(&f, Some(&other), &Change::Set(KEY.into(), entry.clone())).unwrap();
    assert!(fixed.contains("/* my tool */") && inspect(&f, Some(&fixed), URL, None).0 == State::Configured);
    // A file without the container: added, the rest kept.
    let t = changed(&f, Some("{\n  // only this\n  \"theme\": \"x\"\n}\n"), &Change::Set(KEY.into(), entry)).unwrap();
    assert!(t.contains("// only this") && t.contains("\"context_servers\""), "{t}");
    // Invalid: refused (and so never written).
    assert!(changed(&f, Some("{ \"context_servers\": "), &Change::Remove(KEY.into())).is_err());
    assert!(matches!(inspect(&f, Some("[1, 2]"), URL, None).0, State::Unreadable { .. }));
}

#[test]
fn json_and_dotted_keys_and_the_old_entry() {
    // Cursor: other servers byte for byte; the old figcraft entry found and removed.
    let f = file("cursor");
    let before =
        "{\n  \"mcpServers\": {\n    \"figcraft\": {\"url\": \"http://127.0.0.1:4848/mcp\"},\n    \"github\": {\"command\": \"gh\"}\n  }\n}\n";
    let (_, old) = inspect(&f, Some(before), URL, None);
    assert_eq!(old, ["figcraft"]);
    let t = changed(&f, Some(before), &Change::Remove("figcraft".into())).unwrap();
    assert!(t.contains("\"github\": {\"command\": \"gh\"}") && !t.contains("figcraft"), "{t}");
    // Amp keeps its servers under one key with a dot in it.
    let amp = file("amp");
    let t = r#"{ "amp.mcpServers": { "scrim": { "url": "http://127.0.0.1:7681/mcp" } }, "amp.other": 1 }"#;
    assert_eq!(inspect(&amp, Some(t), URL, None).0, State::Configured);
    // OpenCode's `mcp`, with other servers.
    let oc = file("opencode");
    let before = "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"mcp\": {\"a\": {\"type\": \"local\", \"command\": [\"x\"]}}\n}\n";
    let t = changed(&oc, Some(before), &Change::Set(KEY.into(), file_entry(&oc, URL, None))).unwrap();
    let v: Value = serde_json::from_str(&t).unwrap();
    assert_eq!(v["mcp"]["a"]["command"], json!(["x"]));
    assert_eq!(v["mcp"]["scrim"], json!({"type": "remote", "url": URL, "enabled": true}));
}

#[test]
fn toml_edits_keep_comments_and_order() {
    let f = file("codex");
    let before = "# my Codex settings\nmodel = \"o3\" # the model\n\n[mcp_servers.docs]\n# a docs server\ncommand = \"npx\"\nargs = [\"-y\", \"docs\"]\n\n[profiles.fast]\nmodel = \"o4-mini\"\n";
    let t = changed(&f, Some(before), &Change::Set(KEY.into(), file_entry(&f, URL, Some("T")))).unwrap();
    for kept in ["# my Codex settings", "model = \"o3\" # the model", "# a docs server", "[profiles.fast]"] {
        assert!(t.contains(kept), "{kept:?} kept in:\n{t}");
    }
    assert!(t.contains("[mcp_servers.scrim]") && t.contains("bearer_token_env_var = \"SCRIM_MCP_TOKEN\""), "{t}");
    assert!(t.find("[mcp_servers.docs]") < t.find("[profiles.fast]"), "order kept");
    assert_eq!(inspect(&f, Some(&t), URL, Some("T")).0, State::Configured);
    assert_eq!(inspect(&f, Some(&t), URL, None).0, State::Configured, "a token entry still points here");
    // Re-registered, then removed.
    let moved = changed(&f, Some(&t), &Change::Set(KEY.into(), json!({"url": "http://127.0.0.1:9/mcp"}))).unwrap();
    assert!(matches!(inspect(&f, Some(&moved), URL, None).0, State::OutOfDate { .. }));
    let gone = changed(&f, Some(&moved), &Change::Remove(KEY.into())).unwrap();
    assert!(!gone.contains("scrim") && gone.contains("# a docs server"), "{gone}");
    // A new file: just the table.
    let fresh = changed(&f, None, &Change::Set(KEY.into(), file_entry(&f, URL, None))).unwrap();
    assert_eq!(fresh.trim(), format!("[mcp_servers.scrim]\nurl = \"{URL}\""));
    // Invalid TOML: refused.
    assert!(changed(&f, Some("[mcp_servers\nurl ="), &Change::Remove(KEY.into())).is_err());
}

#[test]
fn the_safe_write_keeps_a_backup_and_leaves_bad_files_alone() {
    let d = scratch("write");
    let f = file("cursor");
    // A missing folder and file: made.
    let path = d.join("new").join("mcp.json");
    let w = apply(&f, &path, &Change::Set(KEY.into(), file_entry(&f, URL, None)), 1_792_000_000).unwrap();
    assert!(w.backup.is_none());
    assert_eq!(inspect(&f, Some(&std::fs::read_to_string(&path).unwrap()), URL, None).0, State::Configured);
    // An existing file: a backup first, then the new text renamed over it; no temporary file left.
    let path = d.join("mcp.json");
    let before = "{\n  // keep me\n  \"mcpServers\": {\"other\": {\"url\": \"https://x\"}}\n}\n";
    std::fs::write(&path, before).unwrap();
    let w = apply(&f, &path, &Change::Set(KEY.into(), file_entry(&f, URL, None)), 1_792_000_000).unwrap();
    let backup = w.backup.unwrap();
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), before);
    assert!(std::fs::read_to_string(&path).unwrap().contains("// keep me"));
    assert!(names(&d).iter().all(|n| !n.ends_with(".tmp")), "{:?}", names(&d));
    // Invalid: refused, untouched, no backup.
    let bad = d.join("bad.json");
    std::fs::write(&bad, "{ nope").unwrap();
    assert!(apply(&f, &bad, &Change::Set(KEY.into(), json!({"url": URL})), 0).is_err());
    assert_eq!(std::fs::read_to_string(&bad).unwrap(), "{ nope");
    // A read-only profile (Scrim reads VS Code's profile file, never writes it).
    let vs = file("vscode");
    assert!(apply(&vs, &d.join("vs.json"), &Change::Set(KEY.into(), json!({})), 0).unwrap_err().contains("doesn't write"));
    // Inside an agent's request: nothing written.
    std::fs::create_dir_all(d.join("work")).unwrap();
    let roots = std::sync::Arc::new(crate::file_access::Roots::new(&[], &[d.join("work").to_string_lossy().into_owned()]).unwrap());
    let err = crate::file_access::confined(Some(&roots), || apply(&f, &path, &Change::Remove(KEY.into()), 0)).unwrap_err();
    assert!(err.contains("only written by the person"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn agents_are_refused_and_nothing_is_read_without_the_apps_environment() {
    let mut s = Session::new();
    s.agents.mcp.port = Some(7681);
    // An agent can read the status and copy, never write.
    for cmd in ["mcp.write-client", "mcp.reregister-client", "mcp.remove-old-client-entry"] {
        let e = s.execute_as(Author::agent("mcp (http)"), None, cmd, &json!({"client": "cursor"})).unwrap_err().to_string();
        assert!(e.contains("only the person"), "{cmd}: {e}");
        s.ui_requests.clear();
        assert_eq!(s.execute(cmd, &json!({"client": "cursor"})).unwrap(), json!({"asked": true}), "the person: asked");
        assert!(s.ui_requests.iter().any(|r| r.kind.starts_with("dialog.mcpClient")));
    }
    // Without the app's environment (tests, headless, the browser), no file is read.
    let r = s.execute("mcp.client-status", &json!({"client": "cursor"})).unwrap();
    assert_eq!(r["state"], "unknown");
    assert!(r["file"]["path"].is_null(), "{r}");
    // With a scratch one, its file is.
    let d = scratch("status");
    std::fs::create_dir_all(d.join(".cursor")).unwrap();
    std::fs::write(d.join(".cursor").join("mcp.json"), json!({"mcpServers": {"scrim": {"url": URL}}}).to_string()).unwrap();
    s.agents.mcp.client_env = Some(PathEnv { home: Some(d.clone()), appdata: None, vars: HashMap::new() });
    let r = s.execute("mcp.client-status", &json!({"client": "cursor"})).unwrap();
    assert_eq!(r["state"], "configured", "{r}");
    // Inside an agent's request: unreadable, not read.
    std::fs::create_dir_all(d.join("work")).unwrap();
    let roots = std::sync::Arc::new(crate::file_access::Roots::new(&[], &[d.join("work").to_string_lossy().into_owned()]).unwrap());
    let r = crate::file_access::confined(Some(&roots), || s.execute("mcp.client-status", &json!({"client": "cursor"}))).unwrap();
    assert_eq!(r["state"], "unreadable");
    // Unknown clients and the list.
    assert!(s.execute("mcp.client-status", &json!({"client": "nope"})).is_err());
    let list = s.execute("mcp.agent-clients", &json!({})).unwrap();
    assert!(list["clients"].as_array().unwrap().len() >= 16);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_generic_card() {
    let mut s = Session::new();
    s.agents.mcp.port = Some(7681);
    let j: Value = serde_json::from_str(&super::generic(&s, "json").unwrap()).unwrap();
    assert_eq!(j, json!({"mcpServers": {"scrim": {"url": URL}}}));
    assert_eq!(super::generic(&s, "toml").unwrap(), format!("[mcp_servers.scrim]\nurl = \"{URL}\"\n"));
    assert!(super::generic(&s, "stdio").unwrap().contains("mcp-remote@"), "no control port: the bridge");
    s.agents.control_port = Some(7979);
    assert_eq!(super::generic(&s, "stdio").unwrap(), "scrim mcp --attach 7979");
    assert!(super::generic(&s, "yaml").is_err());
}
