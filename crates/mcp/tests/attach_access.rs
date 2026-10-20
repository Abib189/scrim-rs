//! C1.3 through `scrim mcp --attach`: the server asks the app which tools are off (`mcp.policy`, the
//! control channel) and refuses a turned-off tool's command through run_command and batch before
//! anything is sent to the app. The app here is a stand-in control channel on a port the OS handed
//! out.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use figcraft_mcp::{Attached, Server};
use serde_json::{Value, json};

/// A control channel that answers `mcp.policy` with `off` and anything else with `{}`, recording the
/// methods it was asked.
fn stand_in(off: Vec<String>) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = asked.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (log, off) = (log.clone(), off.clone());
            std::thread::spawn(move || {
                let mut writer = stream.try_clone().unwrap();
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    let req: Value = serde_json::from_str(&line).unwrap_or_default();
                    let method = req["method"].as_str().unwrap_or_default().to_string();
                    let result = if method == "mcp.policy" { json!({"disabledTools": off}) } else { json!({}) };
                    log.lock().unwrap().push(method);
                    let reply = json!({"id": req["id"], "ok": true, "result": result}).to_string();
                    if writer.write_all(format!("{reply}\n").as_bytes()).is_err() {
                        return;
                    }
                }
            });
        }
    });
    (port, asked)
}

#[test]
fn attached_servers_refuse_a_turned_off_tools_command_through_run_command_and_batch() {
    let (port, asked) = stand_in(vec!["create_node".into()]);
    let mut server = Server::new(Box::new(Attached::port(port).unwrap()));
    server.handle(json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}}));
    let create = json!({"command": "node.create", "params": {"type": "rectangle"}});
    for (id, name, args) in
        [(1, "run_command", create.clone()), (2, "batch", json!({"commands": [create.clone()]})), (3, "create_node", json!({"type": "rectangle"}))]
    {
        let r = server
            .handle(json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": args}}))
            .unwrap_or_default();
        let msg = r["error"]["message"].as_str().unwrap_or_default();
        assert!(msg.contains("create_node") && msg.contains("turned off"), "{name}: {r}");
    }
    let asked = asked.lock().unwrap().clone();
    assert!(asked.iter().all(|m| m == "mcp.policy"), "only the policy was asked, nothing ran: {asked:?}");
    assert!(!asked.is_empty(), "the app was asked");
}
