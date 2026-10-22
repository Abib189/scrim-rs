//! R5a: running an agent CLI (`figcraft_mcp::agent_run`), with the stub agent
//! (`tests/stub/stub_agent.py`) standing in for Claude Code: it takes the profile's real arguments,
//! calls a real Scrim MCP endpoint over HTTP and prints Claude Code's events.
//! - The reply, the tool calls and the edit landing in Scrim's document.
//! - Stop, the time limit and dropping the run (Scrim closing) end the whole process tree.
//! - A failing start's last error line, redacted; a non-zero exit; a missing binary.
//! - The child sees only its minimal environment, and runs in an empty folder that is removed.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use figcraft_engine::cmd::agent_clients::headless::{self, output};
use figcraft_mcp::Server;
use figcraft_mcp::agent_run::{End, Run, RunEvent, RunSpec};
use figcraft_mcp::http::{Config, Endpoint};
use serde_json::{Value, json};

fn stub() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/stub/stub_agent.py")
}

/// A headless Scrim with its MCP endpoint on a free port; its URL.
fn scrim() -> (Endpoint, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let e = Endpoint::serve(listener, Arc::new(RwLock::new(Config { token: None })), Server::headless).unwrap();
    (e, format!("http://127.0.0.1:{port}/mcp"))
}

fn home() -> PathBuf {
    let d = std::env::temp_dir().join(format!("scrim-agent-run-home-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The stub run as Claude Code's profile runs it: its real arguments, the minimal environment.
fn start(prompt: &str, url: &str, limit: Duration) -> Run {
    let p = headless::profile("claude-code").unwrap();
    let path = std::env::var("PATH").unwrap_or_default();
    let env = headless::child_env(Some(&path), Some(&home().to_string_lossy()), None, &[]);
    Run::start(RunSpec { program: stub(), args: p.argv(prompt, url, false), env, limit }).unwrap()
}

/// Every event until the end (at most `within`), the stdout lines read as Claude Code's.
fn collect(run: &Run, within: Duration) -> (Vec<String>, End, Vec<String>, f64) {
    let mut lines = Vec::new();
    let deadline = Instant::now() + within;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match run.events.recv_timeout(left).expect("the run ends in time") {
            RunEvent::Line(l) => lines.push(l),
            RunEvent::Ended { end, stderr, secs } => return (lines, end, stderr, secs),
        }
    }
}

fn outcome(lines: &[String]) -> output::Outcome {
    let mut o = output::Outcome::default();
    for l in lines {
        for e in output::parse(headless::Output::ClaudeStreamJson, l) {
            o.add(e);
        }
    }
    o
}

/// The grandchild the stub started (`slow`), once it says so.
fn grandchild(run: &Run) -> i32 {
    loop {
        if let RunEvent::Line(l) = run.events.recv_timeout(Duration::from_secs(20)).expect("the stub reports its grandchild")
            && let Ok(v) = serde_json::from_str::<Value>(&l)
            && let Some(pid) = v["grandchild"].as_i64()
        {
            return pid as i32;
        }
    }
}

fn alive(pid: i32) -> bool {
    std::process::Command::new("kill").args(["-0", &pid.to_string()]).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success())
}

/// A tool call straight to the endpoint (to read what the agent did).
fn call(url: &str, tool: &str, args: Value) -> Value {
    let port: u16 = url.trim_start_matches("http://127.0.0.1:").trim_end_matches("/mcp").parse().unwrap();
    let post = |body: Value, session: Option<&str>| -> (Option<String>, Value) {
        let body = body.to_string();
        let extra = session.map(|s| format!("Mcp-Session-Id: {s}\r\n")).unwrap_or_default();
        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.write_all(req.as_bytes()).unwrap();
        let mut r = BufReader::new(s);
        let (mut line, mut sid, mut len) = (String::new(), None, 0);
        r.read_line(&mut line).unwrap();
        loop {
            line.clear();
            r.read_line(&mut line).unwrap();
            if line.trim().is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                match k.trim().to_ascii_lowercase().as_str() {
                    "mcp-session-id" => sid = Some(v.trim().to_string()),
                    "content-length" => len = v.trim().parse().unwrap(),
                    _ => {}
                }
            }
        }
        let mut body = vec![0; len];
        r.read_exact(&mut body).unwrap();
        let text = String::from_utf8_lossy(&body).into_owned();
        (sid, serde_json::from_str(&text[text.find('{').unwrap()..]).unwrap())
    };
    let (sid, _) = post(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}),
        None,
    );
    let (_, r) = post(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": tool, "arguments": args}}), sid.as_deref());
    r
}

#[test]
fn the_stub_agent_works_through_scrims_mcp_endpoint_and_replies() {
    let (_e, url) = scrim();
    let run = start("Create one 100x100 rectangle named Test", &url, Duration::from_secs(60));
    let (lines, end, stderr, _) = collect(&run, Duration::from_secs(60));
    assert_eq!(end, End::Exited(Some(0)), "stderr: {stderr:?}");
    let o = outcome(&lines);
    assert_eq!(o.reply.as_deref(), Some("Made Test."), "{lines:?}");
    assert_eq!(o.scrim_connected, Some(true));
    assert_eq!(o.tools, 2, "create_node, then finish_working");
    assert_eq!((o.input_tokens, o.cost_usd), (None, None), "nothing reported, nothing invented");
    let found = call(&url, "inspect_document", json!({}));
    assert!(found.to_string().contains("Test"), "the rectangle is in Scrim's document: {found}");
}

#[test]
fn reported_usage_and_cost_are_read_as_reported() {
    let (_e, url) = scrim();
    let run = start("rectangle named Priced cost", &url, Duration::from_secs(60));
    let (lines, end, ..) = collect(&run, Duration::from_secs(60));
    assert_eq!(end, End::Exited(Some(0)));
    let o = outcome(&lines);
    assert_eq!((o.input_tokens, o.output_tokens, o.cost_usd), (Some(1200), Some(80), Some(0.0123)));
}

#[test]
fn stop_ends_the_whole_process_tree() {
    let (_e, url) = scrim();
    let run = start("slow 60", &url, Duration::from_secs(120));
    let g = grandchild(&run);
    assert!(alive(g));
    let t = Instant::now();
    run.stop();
    let (_, end, ..) = collect(&run, Duration::from_secs(10));
    assert_eq!(end, End::Stopped);
    assert!(t.elapsed() < Duration::from_secs(5), "stopped promptly: {:?}", t.elapsed());
    assert!(!alive(g), "the agent's own child is gone too");
    assert!(!alive(run.pid() as i32), "and the agent");
}

#[test]
fn the_time_limit_stops_it() {
    let (_e, url) = scrim();
    let run = start("slow 60", &url, Duration::from_secs(2));
    let g = grandchild(&run);
    let (_, end, _, secs) = collect(&run, Duration::from_secs(15));
    assert_eq!(end, End::TimedOut);
    assert!((2.0..8.0).contains(&secs), "{secs}");
    assert!(!alive(g));
}

#[test]
fn closing_scrim_mid_run_leaves_nothing_running() {
    let (_e, url) = scrim();
    let run = start("slow 60", &url, Duration::from_secs(120));
    let g = grandchild(&run);
    let pid = run.pid() as i32;
    drop(run);
    let gone = (0..200).any(|_| {
        std::thread::sleep(Duration::from_millis(25));
        !alive(g) && !alive(pid)
    });
    assert!(gone, "the agent and its child end when the run is dropped");
}

#[test]
fn a_failing_start_keeps_its_last_error_line_shown_redacted() {
    let (_e, url) = scrim();
    let run = start("login", &url, Duration::from_secs(30));
    let (lines, end, stderr, _) = collect(&run, Duration::from_secs(30));
    assert_eq!(end, End::Exited(Some(1)));
    assert!(lines.is_empty());
    let last = headless::redact(stderr.last().expect("an error line"), None);
    assert_eq!(last, "Error: Not logged in · Please run /login (key sk-<token>)");
}

#[test]
fn a_non_zero_exit_is_reported_with_its_code() {
    let (_e, url) = scrim();
    let run = start("rectangle named X exit3", &url, Duration::from_secs(30));
    let (_, end, stderr, _) = collect(&run, Duration::from_secs(30));
    assert_eq!(end, End::Exited(Some(3)));
    assert_eq!(stderr.last().map(String::as_str), Some("boom: something went wrong"));
}

#[test]
fn an_unreachable_scrim_is_said_by_the_agent() {
    // Nothing listens on this port: the stub can't initialize.
    let free = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let run = start("rectangle named X", &format!("http://127.0.0.1:{free}/mcp"), Duration::from_secs(30));
    let (lines, end, stderr, _) = collect(&run, Duration::from_secs(30));
    assert_eq!(end, End::Exited(Some(1)));
    assert_eq!(outcome(&lines).scrim_connected, Some(false));
    assert!(stderr.iter().any(|l| l.contains("MCP server \"scrim\" failed")), "{stderr:?}");
}

#[test]
fn a_missing_binary_is_an_error_in_plain_words() {
    let err = Run::start(RunSpec { program: PathBuf::from("/nonexistent/claude"), args: vec![], env: vec![], limit: Duration::from_secs(5) })
        .err()
        .unwrap();
    assert!(err.0.starts_with("/nonexistent/claude: "), "{err:?}");
}

#[test]
fn the_child_sees_only_its_minimal_environment() {
    let (_e, url) = scrim();
    let run = start("env rectangle named E", &url, Duration::from_secs(60));
    let (lines, end, ..) = collect(&run, Duration::from_secs(60));
    assert_eq!(end, End::Exited(Some(0)));
    let env: Vec<String> = lines
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find_map(|v| v["env"].as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect()))
        .unwrap();
    for k in ["PATH", "HOME", "SCRIM_CHAT_CHILD"] {
        assert!(env.contains(&k.to_string()), "{k} given: {env:?}");
    }
    // Cargo sets these for this test; none reach the child. (macOS adds its own text encoding.)
    let extra: Vec<&String> =
        env.iter().filter(|k| !["PATH", "HOME", "SCRIM_CHAT_CHILD", "__CF_USER_TEXT_ENCODING", "LC_CTYPE"].contains(&k.as_str())).collect();
    assert!(extra.is_empty(), "nothing else: {extra:?}");
}

#[test]
fn the_run_happens_in_an_empty_folder_that_is_removed() {
    let (_e, url) = scrim();
    let run = start("cwd rectangle named F", &url, Duration::from_secs(60));
    let (lines, end, ..) = collect(&run, Duration::from_secs(60));
    assert_eq!(end, End::Exited(Some(0)));
    let v = lines.iter().filter_map(|l| serde_json::from_str::<Value>(l).ok()).find(|v| v["cwd"].is_string()).unwrap();
    assert_eq!(v["files"], json!([]), "empty: nothing of the person's to read");
    let dir = PathBuf::from(v["cwd"].as_str().unwrap());
    assert!(dir.file_name().unwrap().to_string_lossy().starts_with("scrim-agent-run-"));
    assert!(!dir.exists(), "removed once the run ended");
}
