//! Fix Q3: `SCRIM_SETTINGS_DIR` puts the settings folder elsewhere, so live checks never touch the
//! person's own settings. Here the app's binary runs as Claude Desktop's bridge (`figcraft
//! mcp-bridge`) with the override pointing at a scratch folder whose endpoint note names a test
//! endpoint: it reaches that endpoint, so it read the scratch folder and nothing else.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, RwLock};

use figcraft_mcp::http::{Config, Endpoint};
use figcraft_mcp::{Server, SessionEvent};
use serde_json::{Value, json};

#[test]
fn the_binary_reads_its_settings_from_scrim_settings_dir() {
    let base = std::env::temp_dir().join(format!("scrim-settings-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (settings, home) = (base.join("settings"), base.join("home"));
    std::fs::create_dir_all(&home).unwrap();
    // A test endpoint, noted in the scratch settings folder.
    let seen = Arc::new(Mutex::new(Vec::new()));
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let s2 = seen.clone();
    let mut endpoint = Endpoint::serve(l, Arc::new(RwLock::new(Config::default())), move || {
        Server::headless().with_session_hook(move |e| {
            if let SessionEvent::Started { info, .. } = e {
                s2.lock().unwrap().push(info["name"].as_str().unwrap_or("").to_string());
            }
        })
    })
    .unwrap();
    figcraft_mcp::discovery::write_endpoint(&settings, port).unwrap();
    // The bridge, with a scratch HOME too: only the override can lead it to the endpoint.
    let mut child = Command::new(env!("CARGO_BIN_EXE_figcraft"))
        .arg("mcp-bridge")
        .env("SCRIM_SETTINGS_DIR", &settings)
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let init = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "probe", "version": "1"}}});
    writeln!(stdin, "{init}").unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    out.read_line(&mut line).unwrap();
    let r: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(r["result"]["serverInfo"]["name"], "scrim", "{r}");
    assert_eq!(seen.lock().unwrap().clone(), ["probe (via scrim mcp-bridge)"], "it found the endpoint through SCRIM_SETTINGS_DIR");
    drop(stdin);
    assert!(child.wait().unwrap().success(), "stdin closed: it exits");
    endpoint.stop();
    let _ = std::fs::remove_dir_all(&base);
}
