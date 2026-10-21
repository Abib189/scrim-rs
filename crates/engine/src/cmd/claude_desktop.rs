//! Connect Claude Desktop (Stage F, `docs/design/connect-claude-desktop.md`): Scrim's entry in
//! Claude Desktop's `claude_desktop_config.json`.
//! - **The file:** `~/Library/Application Support/Claude/` (macOS), `%APPDATA%\Claude\` (Windows),
//!   as Anthropic documents; on Linux `$XDG_CONFIG_HOME/Claude/` or `~/.config/Claude/`, inferred
//!   (the docs name no Linux path).
//! - **The entry:** Claude Desktop starts local servers as programs (stdio); a URL in this file isn't
//!   one, and its Connectors reach servers from Anthropic's cloud, not this computer. So the entry
//!   runs a bridge to the window's HTTP endpoint:
//!   - **Scrim's own** (Fix P) when the app knows its binary: `<binary> mcp-bridge`, which finds the
//!     app wherever it listens and waits for it while it's closed ([`bridge_entry`]);
//!   - else `mcp-remote` (pinned) to the live address, offered as Copy only. Entries written with
//!     it are Out of date when Scrim's own bridge is available, and Re-register upgrades them.
//! - **Writing** only ever happens after the person confirms in the window (`dialog.*` requests,
//!   which an agent's commands never open):
//!   - an invalid file is refused, never overwritten;
//!   - only `mcpServers.scrim` (or an old entry) changes, spliced into the text so the rest keeps
//!     its formatting, and checked by parsing;
//!   - a timestamped backup is kept next to the file;
//!   - the new text goes to a temporary file that is renamed over it.
//! - **The status** is read from the file: Configured, Not configured, Out of date, or Unreadable.
//!   Never "connected": Claude Desktop can't be asked.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::CommandSpec;
use crate::{Session, bad, spec};

/// The bridge, pinned: `npx -y` would otherwise fetch the newest version every time Claude Desktop
/// starts. Re-register updates entries written with an older pin.
pub const MCP_REMOTE: &str = "mcp-remote@0.14.3";
/// The argument that runs Scrim's binary as a stdio bridge to the app (Fix P).
pub const BRIDGE_ARG: &str = "mcp-bridge";
/// The key Scrim's entry has under `mcpServers`.
pub const KEY: &str = "scrim";
/// The keys the entry had before the rename.
pub const OLD_KEYS: [&str; 2] = ["figcraft", "figcraft-app"];
/// The environment variable the entry passes the token in (no spaces in `args`: Claude Desktop on
/// Windows doesn't escape them).
pub const TOKEN_VAR: &str = "SCRIM_MCP_AUTH";
const FILE: &str = "claude_desktop_config.json";

/// The UI requests: open the panel; and the confirmations before Scrim writes the file.
pub const CONNECT_REQUEST: &str = "mcp.connectClaudeDesktop";
pub const WRITE_DIALOG: &str = "dialog.mcpDesktopWrite";
pub const REREGISTER_DIALOG: &str = "dialog.mcpDesktopReregister";
pub const REMOVE_OLD_DIALOG: &str = "dialog.mcpDesktopRemoveOld";

/// The platforms the file is found on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
    Other,
}

impl Os {
    pub fn this() -> Os {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        }
    }
}

/// The environment the path comes from.
#[derive(Clone, Debug, Default)]
pub struct Env {
    pub home: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
}

impl Env {
    pub fn this() -> Env {
        let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        Env { home: var("HOME").or_else(|| var("USERPROFILE")), appdata: var("APPDATA"), xdg_config_home: var("XDG_CONFIG_HOME") }
    }
}

/// Claude Desktop's configuration file on `os`; `None` where there's no known place.
pub fn config_path(os: Os, env: &Env) -> Option<PathBuf> {
    let dir = match os {
        Os::Mac => env.home.as_ref()?.join("Library").join("Application Support").join("Claude"),
        Os::Windows => env.appdata.as_ref()?.join("Claude"),
        Os::Linux => env.xdg_config_home.clone().or_else(|| env.home.as_ref().map(|h| h.join(".config")))?.join("Claude"),
        Os::Other => return None,
    };
    Some(dir.join(FILE))
}

/// Whether the path on `os` is Anthropic's documented one (Linux's is inferred).
pub fn documented(os: Os) -> bool {
    matches!(os, Os::Mac | Os::Windows)
}

/// How the entry starts npx: by name (Windows, as Anthropic's example does), or by its full path
/// with `PATH` set to its folder and the system's, since a desktop app gets a short `PATH` (npx
/// from nvm or Homebrew isn't on it, and npx needs the `node` beside it).
pub fn npx_command(os: Os, found: Option<&str>) -> (String, Option<String>) {
    match (os, found) {
        (Os::Windows, _) | (_, None) => ("npx".into(), None),
        (_, Some(path)) => {
            let dir = Path::new(path).parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
            let mut dirs: Vec<String> = vec![dir];
            for d in ["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin", "/bin"] {
                if !dirs.iter().any(|x| x == d) {
                    dirs.push(d.into());
                }
            }
            (path.to_string(), Some(dirs.into_iter().filter(|d| !d.is_empty()).collect::<Vec<_>>().join(":")))
        }
    }
}

/// Scrim's entry: `mcp-remote` forwarding to `url`, with the token when authentication is on.
pub fn entry(url: &str, token: Option<&str>, npx: (String, Option<String>)) -> Value {
    let (command, path) = npx;
    let mut args = vec![json!("-y"), json!(MCP_REMOTE), json!(url)];
    let mut env = Map::new();
    if let Some(p) = path {
        env.insert("PATH".into(), json!(p));
    }
    if let Some(t) = token {
        args.extend([json!("--header"), json!(format!("Authorization:${{{TOKEN_VAR}}}"))]);
        env.insert(TOKEN_VAR.into(), json!(format!("Bearer {t}")));
    }
    let mut e = Map::new();
    e.insert("command".into(), json!(command));
    e.insert("args".into(), Value::Array(args));
    if !env.is_empty() {
        e.insert("env".into(), Value::Object(env));
    }
    Value::Object(e)
}

/// Scrim's own bridge (Fix P): `exe mcp-bridge`, with the token when authentication is on. No
/// address: the bridge reads where the app listens each time it connects.
pub fn bridge_entry(exe: &str, token: Option<&str>) -> Value {
    let mut e = Map::new();
    e.insert("command".into(), json!(exe));
    e.insert("args".into(), json!([BRIDGE_ARG]));
    if let Some(t) = token {
        e.insert("env".into(), json!({TOKEN_VAR: format!("Bearer {t}")}));
    }
    Value::Object(e)
}

/// Whether `e` runs Scrim's own bridge.
pub fn is_bridge(e: &Value) -> bool {
    e["args"] == json!([BRIDGE_ARG])
}

/// Whether `e` runs `mcp-remote` (any version): an entry from before Fix P.
pub fn is_mcp_remote(e: &Value) -> bool {
    e["args"].as_array().is_some_and(|a| a.iter().filter_map(Value::as_str).any(|x| x == "mcp-remote" || x.starts_with("mcp-remote@")))
}

/// What the file says about Scrim.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No file, an empty one, or no `scrim` entry.
    NotConfigured,
    /// Scrim's entry, for this address.
    Configured,
    /// A `scrim` entry that isn't this window's (`target`: where it points).
    OutOfDate { target: String },
    /// The file can't be read or isn't valid JSON (`error`): it is left alone.
    Unreadable { error: String },
}

impl State {
    pub fn id(&self) -> &'static str {
        match self {
            State::NotConfigured => "not configured",
            State::Configured => "configured",
            State::OutOfDate { .. } => "out of date",
            State::Unreadable { .. } => "unreadable",
        }
    }
}

/// What the file holds: Scrim's state, and the old entries still there.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub state: State,
    pub old: Vec<String>,
    /// Scrim's entry runs `mcp-remote` (Re-register can move it to Scrim's own bridge).
    pub mcp_remote: bool,
}

/// The file's text parsed: `None` for a missing or blank file; an error for anything that isn't a
/// JSON object.
fn parse(text: Option<&str>) -> Result<Option<Map<String, Value>>, String> {
    let Some(t) = text.filter(|t| !t.trim().is_empty()) else { return Ok(None) };
    match serde_json::from_str::<Value>(t) {
        Ok(Value::Object(m)) => Ok(Some(m)),
        Ok(_) => Err("the file isn't a JSON object".into()),
        Err(e) => Err(format!("it isn't valid JSON ({e})")),
    }
}

/// Where an entry points: the URL among its arguments, or its command line.
fn target(e: &Value) -> String {
    let args: Vec<&str> = e["args"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    match args.iter().find(|a| a.starts_with("http://") || a.starts_with("https://")) {
        Some(u) => u.to_string(),
        None => {
            std::iter::once(e["command"].as_str().unwrap_or_default()).chain(args.iter().copied()).collect::<Vec<_>>().join(" ").trim().to_string()
        }
    }
}

/// Read what `text` (the file, `None` when missing) says, against the entry Scrim would write.
pub fn inspect(text: Option<&str>, want: &Value) -> Found {
    let map = match parse(text) {
        Ok(m) => m.unwrap_or_default(),
        Err(error) => return Found { state: State::Unreadable { error }, old: Vec::new(), mcp_remote: false },
    };
    let servers = map.get("mcpServers").and_then(Value::as_object);
    let old = OLD_KEYS.iter().filter(|k| servers.is_some_and(|s| s.contains_key(**k))).map(|k| k.to_string()).collect();
    let current = servers.and_then(|s| s.get(KEY));
    let state = match current {
        None => State::NotConfigured,
        // The same bridge, address and token (its npx path may differ: where node lives is the
        // person's business); Scrim's own bridge only from this binary.
        Some(e)
            if e["args"] == want["args"]
                && e["env"][TOKEN_VAR] == want["env"][TOKEN_VAR]
                && (!is_bridge(want) || e["command"] == want["command"]) =>
        {
            State::Configured
        }
        Some(e) => State::OutOfDate { target: target(e) },
    };
    Found { state, old, mcp_remote: current.is_some_and(is_mcp_remote) }
}

/// A change to `mcpServers`.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Set `mcpServers[key]` to the value.
    Set(String, Value),
    /// Remove `mcpServers[key]`.
    Remove(String),
}

/// `text` (the file, `None` when missing) with `change` made: only that entry changes, spliced into
/// the text where it can be (and checked by parsing), else the whole file pretty-printed with the
/// same meaning (`command`, `args`, `env` first in each entry, the other keys sorted). Refuses a file that isn't a JSON object, or whose `mcpServers` isn't one.
pub fn changed(text: Option<&str>, change: &Change) -> Result<String, String> {
    let mut map = parse(text)?.unwrap_or_default();
    let servers = map.entry("mcpServers").or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(servers) = servers else { return Err("its mcpServers isn't a JSON object".into()) };
    match change {
        Change::Set(k, v) => {
            servers.insert(k.clone(), v.clone());
        }
        Change::Remove(k) => {
            servers.remove(k);
        }
    }
    let want = Value::Object(map);
    if let Some(t) = text.filter(|t| !t.trim().is_empty())
        && let Some(spliced) = splice::apply(t, change)
        && serde_json::from_str::<Value>(&spliced).is_ok_and(|v| v == want)
    {
        return Ok(spliced);
    }
    splice::ordered(&want).map(|s| s + "\n").ok_or_else(|| "the file couldn't be written out".to_string())
}

/// What a write did.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub path: PathBuf,
    /// The copy of the file as it was, when there was one.
    pub backup: Option<PathBuf>,
}

/// UTC `YYYYMMDD-HHMMSS` for `secs` since 1970 (the backup's name).
pub fn stamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}{m:02}{d:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Make `change` to the file at `path`, safely. Only the window calls this, after the person
/// confirmed; no command does (they only ask), and it refuses to run inside an agent's request
/// (a `file_access` scope).
/// - the file is read now, and refused if it isn't a JSON object (nothing is written);
/// - a missing file (and its folder) is created;
/// - an existing one is first copied to `<file>.scrim-backup-<stamp>`;
/// - the new text is written to a temporary file in the same folder, then renamed over the file.
// The file is the platform's, set by the desktop app (`McpEndpoint::desktop_config`), never by a
// command, and is written only by the person's confirmed action: automation roots don't apply, and
// the backup and the rename need the file system directly.
#[allow(clippy::disallowed_methods)]
pub fn apply(path: &Path, change: &Change, now_secs: u64) -> Result<Written, String> {
    if crate::file_access::current().is_some() {
        return Err("Claude Desktop's configuration is only written by the person, in Scrim".into());
    }
    let before = match std::fs::read(path) {
        Ok(b) => Some(String::from_utf8(b).map_err(|_| "it isn't UTF-8 text".to_string())?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let text = changed(before.as_deref(), change)?;
    let dir = path.parent().ok_or("the file has no folder")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the file has no name")?;
    let backup = match &before {
        Some(b) => {
            let base = format!("{name}.scrim-backup-{}", stamp(now_secs));
            let mut p = dir.join(&base);
            let mut n = 2;
            while p.exists() && n < 100 {
                p = dir.join(format!("{base}-{n}"));
                n += 1;
            }
            std::fs::write(&p, b).map_err(|e| e.to_string())?;
            Some(p)
        }
        None => None,
    };
    let tmp = dir.join(format!(".{name}.scrim-{}.tmp", std::process::id()));
    let written = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(Written { path: path.to_path_buf(), backup })
}

/// The entry for this window (`npx`: where the window found npx), with the file's path, its
/// status and what an agent or the panel shows.
pub fn report(s: &Session, npx: Option<&str>) -> crate::Result<Value> {
    let cmd = "mcp.connect-claude-desktop";
    let url = s.agents.mcp.url().ok_or_else(|| bad(cmd, "the MCP endpoint isn't running: restart it from Settings, under MCP"))?;
    let os = Os::this();
    // Scrim's own bridge when the app knows its binary (Fix P); else mcp-remote, to copy.
    let bridge = s.agents.mcp.bridge.as_deref();
    let want = match bridge {
        Some(exe) => bridge_entry(exe, super::mcp::token(s)),
        None => entry(&url, super::mcp::token(s), npx_command(os, npx)),
    };
    let path = s.agents.mcp.desktop_config.clone();
    // Read through `file_access`: inside an agent's request (confined to its folders) the file
    // isn't read, so an agent can't look at Claude Desktop's configuration.
    let found = match &path {
        None => inspect(None, &want),
        Some(p) if !crate::file_access::allowed(p, crate::file_access::Access::Read) => {
            Found { state: State::Unreadable { error: "it is outside the folders this request may read".into() }, old: Vec::new(), mcp_remote: false }
        }
        Some(p) if !crate::file_access::exists(p) => inspect(None, &want),
        Some(p) => match crate::file_access::read(p).map(String::from_utf8) {
            Ok(Ok(text)) => inspect(Some(&text), &want),
            Ok(Err(_)) => Found { state: State::Unreadable { error: "it isn't UTF-8 text".into() }, old: Vec::new(), mcp_remote: false },
            Err(e) => Found { state: State::Unreadable { error: e.to_string() }, old: Vec::new(), mcp_remote: false },
        },
    };
    let (registered, error) = match &found.state {
        State::OutOfDate { target } => (Some(target.clone()), None),
        State::Unreadable { error } => (None, Some(error.clone())),
        _ => (None, None),
    };
    Ok(json!({
        "path": path.map(|p| p.to_string_lossy().into_owned()),
        "documentedPath": documented(os),
        "url": url,
        "entry": {KEY: want},
        "state": found.state.id(),
        "registered": registered,
        "error": error,
        "old": found.old,
        "npx": npx.is_some(),
        // Scrim's own bridge is the entry, and so the panel may write it (Fix P); without it, Copy.
        "bridge": bridge.is_some(),
        "write": bridge.is_some(),
        // The file's entry runs mcp-remote, and Scrim's own bridge can replace it.
        "upgrade": bridge.is_some() && found.mcp_remote && matches!(found.state, State::OutOfDate { .. }),
        "restart": "Quit Claude Desktop completely and open it again: it reads this file when it starts.",
    }))
}

fn npx_of(p: &Value) -> Option<&str> {
    p.get("npx").and_then(Value::as_str).filter(|s| !s.is_empty())
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "mcp.connect-claude-desktop",
            "Connect Claude Desktop",
            "{npx?} → {path, documentedPath, url, entry, state: configured|not configured|out of date|unreadable, registered?, error?, old: [keys], npx, bridge, write, upgrade, restart}  (shows the panel in the app; the file is only written after the person confirms there. The entry runs Scrim's own `mcp-bridge` when the app knows its binary, which waits for the app; else mcp-remote, to copy)",
            false,
            |s, p| {
                let r = report(s, npx_of(p))?;
                s.request_ui(CONNECT_REQUEST, r.clone());
                Ok(r)
            }
        ),
        spec!(
            "mcp.claude-desktop-status",
            "Claude Desktop status",
            "{npx?} → the same as mcp.connect-claude-desktop, without the panel (read from the file: Claude Desktop itself can't be asked)",
            false,
            |s, p| report(s, npx_of(p))
        ),
        spec!(
            "mcp.write-claude-desktop",
            "Write Scrim into Claude Desktop's configuration",
            "{} → {asked: true}  (asks the person first; only they can confirm)",
            false,
            |s, _| {
                s.request_ui(WRITE_DIALOG, json!({}));
                Ok(json!({"asked": true}))
            }
        ),
        spec!(
            "mcp.reregister-claude-desktop",
            "Re-register Scrim in Claude Desktop",
            "{} → {asked: true}  (replaces an out-of-date scrim entry after the person confirms)",
            false,
            |s, _| {
                s.request_ui(REREGISTER_DIALOG, json!({}));
                Ok(json!({"asked": true}))
            }
        ),
        spec!(
            "mcp.remove-old-desktop-entry",
            "Remove the old entry from Claude Desktop",
            "{} → {asked: true}  (removes figcraft / figcraft-app after the person confirms)",
            false,
            |s, _| {
                s.request_ui(REMOVE_OLD_DIALOG, json!({}));
                Ok(json!({"asked": true}))
            }
        ),
        spec!(
            "mcp.copy-claude-desktop-entry",
            "Copy the Claude Desktop entry",
            "{npx?} → {entry}  (the scrim entry as JSON, onto the clipboard in the app)",
            false,
            |s, p| {
                let r = report(s, npx_of(p))?;
                let text = splice::ordered(&r["entry"]).unwrap_or_default();
                s.request_ui(super::code::COPY_REQUEST, json!({"text": text, "what": "the Claude Desktop entry"}));
                Ok(json!({"entry": r["entry"]}))
            }
        ),
    ]
}

pub mod splice;

#[cfg(test)]
#[path = "claude_desktop_tests.rs"]
mod tests;
