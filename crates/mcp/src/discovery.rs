//! Where the running app's MCP endpoint is, for `scrim mcp-bridge` (Fix P,
//! `docs/design/connect-claude-desktop.md` › Fix P).
//! - **`mcp-endpoint.json`** in the settings folder: `{address, port, pid}`, written by the app when
//!   its endpoint starts (atomically: a temporary file renamed over it) and removed when it stops
//!   or the app quits cleanly. No token: the token stays in `state.json` and the client's entry.
//! - **Without it** (an app that crashed, or one from before Fix P), the port the person chose in
//!   Settings ▸ MCP (`state.json`), on 127.0.0.1.
//! - **`mcp-tools.json`**: the app's `tools/list` answer, written on each start and when Tool access
//!   changes, so the bridge can list the tools while the app is closed.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The endpoint's address, in the settings folder.
pub const ENDPOINT_FILE: &str = "mcp-endpoint.json";
/// The tools the endpoint lists, in the settings folder.
pub const TOOLS_FILE: &str = "mcp-tools.json";
/// The most bytes either file is read up to.
const MAX: u64 = 4 << 20;

/// The settings folder (`design.scrim.app` under the platform's application-settings folder, or
/// the one `SCRIM_SETTINGS_DIR` names).
pub fn settings_dir() -> Option<PathBuf> {
    figcraft_engine::settings::settings_dir().0
}

/// `bytes` to `dir/name`, atomically: a temporary file in the same folder renamed over it.
fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, dir.join(name)));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

fn read_json(path: &Path) -> Option<Value> {
    use std::io::Read;
    let mut s = String::new();
    std::fs::File::open(path).ok()?.take(MAX).read_to_string(&mut s).ok()?;
    serde_json::from_str(&s).ok()
}

/// The app's endpoint started on `port` (loopback): note it for bridges.
pub fn write_endpoint(dir: &Path, port: u16) -> std::io::Result<()> {
    let v = json!({"address": "127.0.0.1", "port": port, "pid": std::process::id()});
    write_atomic(dir, ENDPOINT_FILE, v.to_string().as_bytes())
}

/// The app's endpoint stopped: remove the note, if it is this process's (another instance may
/// have written its own since).
pub fn remove_endpoint(dir: &Path) {
    let path = dir.join(ENDPOINT_FILE);
    if read_json(&path).is_some_and(|v| v["pid"].as_u64() == Some(u64::from(std::process::id()))) {
        let _ = std::fs::remove_file(path);
    }
}

/// Where to reach the app: the note, else the port saved in Settings ▸ MCP. Only loopback.
pub fn endpoint(dir: &Path) -> SocketAddr {
    let noted = read_json(&dir.join(ENDPOINT_FILE)).and_then(|v| {
        let ip: std::net::IpAddr = v["address"].as_str()?.parse().ok()?;
        let port = u16::try_from(v["port"].as_u64()?).ok().filter(|p| *p > 0)?;
        ip.is_loopback().then_some(SocketAddr::new(ip, port))
    });
    noted.unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], figcraft_engine::settings::saved_mcp(dir).port)))
}

/// The app instance the note names (its process id): another one at the same address is a restart.
pub fn instance(dir: &Path) -> Option<u64> {
    read_json(&dir.join(ENDPOINT_FILE))?["pid"].as_u64()
}

/// The app's `tools/list` answer (`{"tools": […]}`), for bridges to list while it's closed.
pub fn write_tools(dir: &Path, list: &Value) -> std::io::Result<()> {
    write_atomic(dir, TOOLS_FILE, list.to_string().as_bytes())
}

/// The tools the app last listed, if it noted them.
pub fn tools(dir: &Path) -> Option<Value> {
    read_json(&dir.join(TOOLS_FILE)).filter(|v| v["tools"].is_array())
}
