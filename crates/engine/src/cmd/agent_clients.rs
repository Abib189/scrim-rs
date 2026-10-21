//! Connect other agents (Stage J, `docs/design/connect-any-agent.md`): every MCP client Scrim
//! knows is a **profile**, data in `agent_clients/clients.json`, never a code path of its own.
//! - **What a profile says:** how the client adds a server (its CLI, its JSON/JSONC or TOML file,
//!   or by hand), where its file is per OS (and whether that path is documented), the entry's
//!   shape, how a token is passed, which `clientInfo.name` it announces, and two verifications
//!   (Stage K): registration (verified live, docs only, untested) and tool use (verified live,
//!   untested), each with its date and, when live, the doc that records it.
//! - **The status** comes from the client's own file, read through `file_access` (an agent's
//!   request can't read it): Configured, Not configured, Out of date (another address, or the old
//!   `figcraft` entry), Can't read. Never "connected": only the endpoint knows that.
//! - **Writing** happens only in the window, after the person confirms (the dialog commands refuse
//!   agents outright):
//!   - JSON and JSONC are edited in their own text (`jsonc`), comments kept;
//!   - TOML through `toml_edit` (`tomlfile`);
//!   - a file Scrim can't change safely is refused and left alone (Copy instead);
//!   - a timestamped backup is kept, and the new text is renamed over the file.
//! - **The live address** is always the endpoint's real one; the token, when authentication is on,
//!   goes in as an environment-variable reference wherever the client documents one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::CommandSpec;
use super::claude_desktop::{Change, Os, Written, stamp};
use crate::{EngineError, Session, bad, spec};

pub mod jsonc;
pub mod tomlfile;

/// The key Scrim's entry has.
pub const KEY: &str = "scrim";
/// The keys it had before the rename.
pub const OLD_KEYS: [&str; 2] = ["figcraft", "figcraft-app"];
/// The environment variable a client reads the token from, where it supports one.
pub const TOKEN_VAR: &str = "SCRIM_MCP_TOKEN";

/// The UI requests: open a client's panel, and the confirmations before Scrim changes its
/// configuration (`{client}`).
pub const CONNECT_REQUEST: &str = "mcp.connectClient";
pub const WRITE_DIALOG: &str = "dialog.mcpClientWrite";
pub const REREGISTER_DIALOG: &str = "dialog.mcpClientReregister";
pub const REMOVE_OLD_DIALOG: &str = "dialog.mcpClientRemoveOld";

/// A value per platform.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PerOs<T> {
    #[serde(default)]
    pub mac: T,
    #[serde(default)]
    pub linux: T,
    #[serde(default)]
    pub windows: T,
}

impl<T> PerOs<T> {
    pub fn get(&self, os: Os) -> Option<&T> {
        match os {
            Os::Mac => Some(&self.mac),
            Os::Linux => Some(&self.linux),
            Os::Windows => Some(&self.windows),
            Os::Other => None,
        }
    }
}

/// How a client is detected: a program on `PATH`, or a folder of its own.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Detect {
    #[serde(default)]
    pub programs: Vec<String>,
    #[serde(default)]
    pub dirs: Vec<String>,
}

/// A client's command line: the program and its arguments (strings, or objects passed as one JSON
/// argument), with `{url}`, `{key}`, `{token_var}`, `{token}` filled in.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Cli {
    pub program: String,
    #[serde(default)]
    pub add: Vec<Value>,
    /// Appended to `add` when authentication is on (empty: the CLI has no documented way).
    #[serde(default)]
    pub add_token: Vec<Value>,
    /// Merged into a JSON argument when authentication is on.
    #[serde(default)]
    pub token_json: Option<Value>,
    #[serde(default)]
    pub remove: Vec<Value>,
}

/// How a token reaches the client: `env-ref` (a reference to [`TOKEN_VAR`] it expands), or
/// `literal` (the token itself, written into the file: the person is warned first).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TokenSpec {
    pub style: String,
    #[serde(default)]
    pub set: Value,
}

/// A client's configuration file.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct FileSpec {
    /// `json`, `jsonc` or `toml`.
    pub format: String,
    /// Candidate paths per OS (the first that exists, else the first that resolves).
    pub paths: PerOs<Vec<String>>,
    /// Whether the client's docs name that path (else it is inferred, and the panel says so).
    #[serde(default)]
    pub documented: PerOs<bool>,
    /// The key path of the servers object (`["mcpServers"]`; `["amp.mcpServers"]` is one key).
    pub container: Vec<String>,
    pub entry: Value,
    /// Where an entry keeps its address.
    #[serde(default)]
    pub url_keys: Vec<String>,
    #[serde(default)]
    pub token: Option<TokenSpec>,
    /// Scrim may write this file (else it is read for the status only).
    #[serde(default)]
    pub write: bool,
}

/// What the person adds by hand: text, or a JSON value shown pretty-printed.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Snippet {
    pub format: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub token_set: Option<Value>,
}

/// One verification of a client (Stage K): its level, the date (when its docs were read, or the
/// live run), and the doc that records a live run. `verified-live` needs that record.
#[derive(Clone, Debug, Default, Deserialize, serde::Serialize, PartialEq)]
pub struct Check {
    pub level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// One client.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// The name a person knows it by, for its `clientInfo` (J1).
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub client_names: Vec<String>,
    #[serde(default)]
    pub client_prefixes: Vec<String>,
    #[serde(default)]
    pub docs: Vec<String>,
    /// How far adding Scrim to it was verified (`untested`, `docs-only`, `verified-live`), and when.
    pub registration: Check,
    /// Whether an agent in it was seen using Scrim's tools (`untested`, `verified-live`), and when.
    #[serde(rename = "tool-use")]
    pub tool_use: Check,
    #[serde(default)]
    pub detect: Detect,
    #[serde(default)]
    pub cli: Option<Cli>,
    #[serde(default)]
    pub file: Option<FileSpec>,
    #[serde(default)]
    pub snippet: Option<Snippet>,
    /// Connected through a flow of its own (`claude-code`: C2, `claude-desktop`: F1).
    #[serde(default)]
    pub builtin: Option<String>,
    /// Needs a stdio bridge (`mcp-remote`).
    #[serde(default)]
    pub bridge: bool,
    /// Calls from a server, so it can't reach 127.0.0.1.
    #[serde(default)]
    pub cloud: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Every profile, in the order the list shows them.
pub fn profiles() -> &'static [Profile] {
    static ALL: OnceLock<Vec<Profile>> = OnceLock::new();
    ALL.get_or_init(|| serde_json::from_str(include_str!("agent_clients/clients.json")).unwrap_or_default())
}

pub fn profile(id: &str) -> Option<&'static Profile> {
    profiles().iter().find(|p| p.id == id)
}

/// The friendly name of a client that announced `raw` as its `clientInfo.name`, from the profiles.
pub fn friendly(raw: &str) -> Option<&'static str> {
    profiles()
        .iter()
        .find(|p| p.client_names.iter().any(|n| n.eq_ignore_ascii_case(raw)) || p.client_prefixes.iter().any(|x| raw.starts_with(x.as_str())))
        .map(|p| if p.display.is_empty() { p.name.as_str() } else { p.display.as_str() })
}

/// Where paths come from: the home folder, `%APPDATA%`, `$XDG_CONFIG_HOME`, and the environment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathEnv {
    pub home: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub vars: HashMap<String, String>,
}

impl PathEnv {
    pub fn this() -> PathEnv {
        let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        let vars = std::env::vars().filter(|(_, v)| !v.is_empty()).collect();
        PathEnv { home: var("HOME").or_else(|| var("USERPROFILE")), appdata: var("APPDATA"), vars }
    }

    fn var(&self, k: &str) -> Option<String> {
        self.vars.get(k).cloned()
    }
}

/// The end of the placeholder whose `{` is at `open` (nested braces counted).
fn placeholder_end(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices().skip_while(|(i, _)| *i < open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// A path template resolved: `{home}`, `{appdata}`, `{xdg_config}` (`$XDG_CONFIG_HOME`, else
/// `{home}/.config`) and `{env:NAME|fallback}` (the fallback may hold placeholders; without one, an
/// unset variable leaves the path unresolved). `None` when something it needs isn't known.
pub fn resolve(template: &str, env: &PathEnv) -> Option<PathBuf> {
    fn expand(s: &str, env: &PathEnv) -> Option<String> {
        let mut out = String::new();
        let mut i = 0;
        while let Some(rel) = s.get(i..)?.find('{') {
            let open = i + rel;
            out.push_str(s.get(i..open)?);
            let close = placeholder_end(s, open)?;
            let name = s.get(open + 1..close)?;
            let home = || env.home.as_ref().map(|h| h.to_string_lossy().into_owned());
            let value = match name {
                "home" => home()?,
                "appdata" => env.appdata.as_ref()?.to_string_lossy().into_owned(),
                "xdg_config" => env.var("XDG_CONFIG_HOME").or_else(|| home().map(|h| format!("{h}/.config")))?,
                n if n.starts_with("env:") => {
                    let rest = n.get(4..)?;
                    let (var, fallback) = match rest.split_once('|') {
                        Some((v, f)) => (v, Some(f)),
                        None => (rest, None),
                    };
                    match (env.var(var), fallback) {
                        (Some(v), _) => v,
                        (None, Some(f)) => expand(f, env)?,
                        (None, None) => return None,
                    }
                }
                _ => return None,
            };
            out.push_str(&value);
            i = close + 1;
        }
        out.push_str(s.get(i..)?);
        Some(out)
    }
    expand(template, env).filter(|p| !p.is_empty()).map(PathBuf::from)
}

/// What templates are filled with.
struct Vars<'a> {
    url: &'a str,
    token: Option<&'a str>,
}

fn fill_str(s: &str, v: &Vars) -> String {
    s.replace("{key}", KEY).replace("{url}", v.url).replace("{token_var}", TOKEN_VAR).replace("{token}", v.token.unwrap_or("<token>"))
}

fn fill(t: &Value, v: &Vars) -> Value {
    match t {
        Value::String(s) => Value::String(fill_str(s, v)),
        Value::Array(a) => Value::Array(a.iter().map(|x| fill(x, v)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (fill_str(k, v), fill(x, v))).collect()),
        other => other.clone(),
    }
}

/// `add`'s members merged into `base` (objects merged member by member).
fn merge(base: &mut Value, add: &Value) {
    match (base, add) {
        (Value::Object(b), Value::Object(a)) => {
            for (k, v) in a {
                match b.get_mut(k) {
                    Some(existing) if existing.is_object() && v.is_object() => merge(existing, v),
                    _ => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, a) => *b = a.clone(),
    }
}

/// Scrim's entry for a file client: for `url`, with the token part when authentication is on.
pub fn file_entry(f: &FileSpec, url: &str, token: Option<&str>) -> Value {
    let v = Vars { url, token };
    let mut e = fill(&f.entry, &v);
    if let (Some(_), Some(t)) = (token, &f.token) {
        merge(&mut e, &fill(&t.set, &v));
    }
    e
}

/// A command line's arguments filled in (an object argument becomes one JSON string).
fn args(list: &[Value], v: &Vars, extra_json: Option<&Value>) -> Vec<String> {
    list.iter()
        .map(|a| match fill(a, v) {
            Value::String(s) => s,
            mut obj @ Value::Object(_) => {
                if let Some(x) = extra_json {
                    merge(&mut obj, &fill(x, v));
                }
                obj.to_string()
            }
            other => other.to_string(),
        })
        .collect()
}

/// The arguments that add Scrim (`None`: authentication is on and the CLI has no documented way to
/// pass the token).
pub fn cli_add(c: &Cli, url: &str, token: Option<&str>) -> Option<Vec<String>> {
    let v = Vars { url, token };
    let mut a = args(&c.add, &v, token.and(c.token_json.as_ref()));
    if token.is_some() {
        if c.add_token.is_empty() && c.token_json.is_none() {
            return None;
        }
        a.extend(args(&c.add_token, &v, None));
    }
    Some(a)
}

/// The arguments that remove `key` (`None`: the CLI has no documented remove).
pub fn cli_remove(c: &Cli, key: &str) -> Option<Vec<String>> {
    (!c.remove.is_empty()).then(|| c.remove.iter().map(|a| a.as_str().unwrap_or_default().replace("{key}", key)).collect())
}

/// A client's token style when authentication is on: `env-ref`, `literal`, or `none`.
pub fn token_style(p: &Profile) -> &str {
    p.file.as_ref().and_then(|f| f.token.as_ref()).map_or("none", |t| t.style.as_str())
}

/// What a client's file says about Scrim.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    NotConfigured,
    Configured,
    /// A `scrim` entry that isn't this window's (`target`: where it points).
    OutOfDate {
        target: String,
    },
    /// The file can't be read or parsed (`error`): it is left alone.
    Unreadable {
        error: String,
    },
    /// No file to read (a client added by hand, or whose file isn't known here).
    Unknown,
}

impl State {
    pub fn id(&self) -> &'static str {
        match self {
            State::NotConfigured => "not configured",
            State::Configured => "configured",
            State::OutOfDate { .. } => "out of date",
            State::Unreadable { .. } => "unreadable",
            State::Unknown => "unknown",
        }
    }
}

/// The servers in `text` (`None` when there are none yet).
fn servers(f: &FileSpec, text: Option<&str>) -> Result<Option<Map<String, Value>>, String> {
    if f.format == "toml" {
        return match tomlfile::parse(text)? {
            Some(doc) => tomlfile::servers(&doc, &f.container),
            None => Ok(None),
        };
    }
    let Some(root) = jsonc::parse(text)? else { return Ok(None) };
    let mut at = Value::Object(root);
    for seg in &f.container {
        match at.get(seg) {
            Some(v) => at = v.clone(),
            None => return Ok(None),
        }
    }
    match at {
        Value::Object(m) => Ok(Some(m)),
        _ => Err(format!("its {} isn't an object", f.container.join("."))),
    }
}

/// Where an entry points: the first of `url_keys` it has, else its JSON.
fn target(f: &FileSpec, e: &Value) -> String {
    f.url_keys.iter().find_map(|k| e.get(k).and_then(Value::as_str)).map_or_else(|| e.to_string(), str::to_string)
}

/// Whether every leaf of `want` is in `have` with the same value.
fn contains(have: &Value, want: &Value) -> bool {
    match (have, want) {
        (Value::Object(h), Value::Object(w)) => w.iter().all(|(k, v)| h.get(k).is_some_and(|x| contains(x, v))),
        (h, w) => h == w,
    }
}

/// Read what `text` (the file, `None` when missing) says, against this window's `url` and token.
pub fn inspect(f: &FileSpec, text: Option<&str>, url: &str, token: Option<&str>) -> (State, Vec<String>) {
    let servers = match servers(f, text) {
        Ok(s) => s.unwrap_or_default(),
        Err(error) => return (State::Unreadable { error }, Vec::new()),
    };
    let old = OLD_KEYS.iter().filter(|k| servers.contains_key(**k)).map(|k| k.to_string()).collect();
    let state = match servers.get(KEY) {
        None => State::NotConfigured,
        Some(e) => {
            let here = f.url_keys.iter().any(|k| e.get(k).and_then(Value::as_str) == Some(url));
            let token_ok = match (token, &f.token) {
                (Some(_), Some(t)) => contains(e, &fill(&t.set, &Vars { url, token })),
                _ => true,
            };
            if here && token_ok { State::Configured } else { State::OutOfDate { target: target(f, e) } }
        }
    };
    (state, old)
}

/// The file's path for this OS (the first candidate that exists, else the first that resolves),
/// and whether the client's docs name it.
pub fn file_path(f: &FileSpec, os: Os, env: &PathEnv) -> Option<(PathBuf, bool)> {
    let paths: Vec<PathBuf> = f.paths.get(os)?.iter().filter_map(|t| resolve(t, env)).collect();
    let documented = f.documented.get(os).copied().unwrap_or(false);
    let first = paths.first()?.clone();
    Some((paths.into_iter().find(|p| crate::file_access::exists(p)).unwrap_or(first), documented))
}

/// The entry as the person would add it to their file (JSON or TOML text), from the container down.
pub fn entry_text(f: &FileSpec, entry: &Value) -> String {
    if f.format == "toml" {
        return tomlfile::change(None, &f.container, KEY, Some(entry)).unwrap_or_default();
    }
    let mut v = json!({ KEY: entry });
    for seg in f.container.iter().rev() {
        v = json!({ seg.clone(): v });
    }
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// A hand-added client's snippet.
pub fn snippet_text(s: &Snippet, url: &str, token: Option<&str>) -> String {
    let v = Vars { url, token };
    match (&s.text, &s.value) {
        (Some(t), _) => fill_str(t, &v),
        (None, Some(val)) => {
            let mut x = fill(val, &v);
            if let (Some(_), Some(ts)) = (token, &s.token_set)
                && let Some(entry) = x.pointer_mut(&format!("/mcpServers/{KEY}"))
            {
                merge(entry, &fill(ts, &v));
            }
            serde_json::to_string_pretty(&x).unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// Everything about client `id` for this window: its plan (CLI, file, snippet), the status from
/// its file, the notes. What the panel shows and what an agent gets (who can't read the file).
/// Files are found only where the desktop app said (`McpEndpoint::client_env`); without it none is
/// read.
pub fn report(s: &Session, id: &str) -> crate::Result<Value> {
    let none = PathEnv::default();
    let known = s.agents.mcp.client_env.as_ref();
    let env = known.unwrap_or(&none);
    let cmd = "mcp.client-status";
    let p = profile(id).ok_or_else(|| bad(cmd, format!("no client `{id}`: mcp.agent-clients lists them")))?;
    let url = s.agents.mcp.url().ok_or_else(|| bad(cmd, "the MCP endpoint isn't running: restart it from Settings, under MCP"))?;
    let token = super::mcp::token(s);
    let os = Os::this();
    let mut r = json!({
        "id": p.id, "name": p.name, "display": if p.display.is_empty() { &p.name } else { &p.display },
        "docs": p.docs, "registration": p.registration, "toolUse": p.tool_use, "notes": p.notes,
        "builtin": p.builtin, "bridge": p.bridge, "cloud": p.cloud, "url": url,
        // `cliPlain` / `filePlain`: the token itself goes into the client's configuration (the
        // panel warns, and the confirmation says so), not a reference to [`TOKEN_VAR`].
        "token": {"on": token.is_some(), "style": token_style(p), "var": TOKEN_VAR,
            "cliPlain": p.cli.as_ref().is_some_and(|c| c.add_token.iter().chain(c.token_json.iter()).any(|v| v.to_string().contains("{token}"))),
            "filePlain": token_style(p) == "literal"},
        "detect": {"programs": p.detect.programs, "dirs": p.detect.dirs.iter().filter_map(|d| resolve(d, env)).map(|d| d.to_string_lossy().into_owned()).collect::<Vec<_>>()},
        "state": State::Unknown.id(),
    });
    if let Some(c) = &p.cli {
        r["cli"] = json!({"program": c.program, "add": cli_add(c, &url, token), "remove": cli_remove(c, KEY), "removeOld": OLD_KEYS.iter().filter_map(|k| cli_remove(c, k)).collect::<Vec<_>>()});
    }
    if let Some(f) = &p.file {
        let entry = file_entry(f, &url, token);
        let mut file = json!({"format": f.format, "writable": f.write, "entry": entry, "text": entry_text(f, &entry), "container": f.container});
        if let Some((path, documented)) = known.and_then(|e| file_path(f, os, e)) {
            file["path"] = json!(path.to_string_lossy());
            file["documented"] = json!(documented);
            // Read through `file_access`: inside an agent's request (confined to its folders) the
            // file isn't read, so an agent can't look at another app's configuration.
            let text = if !crate::file_access::allowed(&path, crate::file_access::Access::Read) {
                Err("it is outside the folders this request may read".to_string())
            } else if !crate::file_access::exists(&path) {
                Ok(None)
            } else {
                crate::file_access::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| String::from_utf8(b).map(Some).map_err(|_| "it isn't UTF-8 text".into()))
            };
            let (state, old) = match text {
                Ok(t) => inspect(f, t.as_deref(), &url, token),
                Err(error) => (State::Unreadable { error }, Vec::new()),
            };
            if let State::OutOfDate { target } = &state {
                r["registered"] = json!(target);
            }
            if let State::Unreadable { error } = &state {
                r["error"] = json!(error);
            }
            r["state"] = json!(state.id());
            r["old"] = json!(old);
        }
        r["file"] = file;
    }
    if let Some(sn) = &p.snippet {
        r["snippet"] = json!({"format": sn.format, "text": snippet_text(sn, &url, token)});
    }
    Ok(r)
}

/// `text` (the file, `None` when missing) with `change` made, for file client `f`. Refuses a file
/// that can't be parsed, and a JSONC file with comments it can't change while keeping them.
pub fn changed(f: &FileSpec, text: Option<&str>, change: &Change) -> Result<String, String> {
    let (key, entry) = match change {
        Change::Set(k, v) => (k.as_str(), Some(v)),
        Change::Remove(k) => (k.as_str(), None),
    };
    if f.format == "toml" {
        return tomlfile::change(text, &f.container, key, entry);
    }
    let mut want = Value::Object(jsonc::parse(text)?.unwrap_or_default());
    // The meaning the new file must have.
    {
        let mut at = &mut want;
        for seg in &f.container {
            let Value::Object(m) = at else { return Err(format!("its {} isn't an object", f.container.join("."))) };
            at = m.entry(seg.clone()).or_insert_with(|| Value::Object(Map::new()));
        }
        let Value::Object(m) = at else { return Err(format!("its {} isn't an object", f.container.join("."))) };
        match entry {
            Some(v) => {
                m.insert(key.to_string(), v.clone());
            }
            None => {
                m.remove(key);
            }
        }
    }
    let Some(t) = text.filter(|t| !t.trim().is_empty()) else {
        return serde_json::to_string_pretty(&want).map(|s| s + "\n").map_err(|e| e.to_string());
    };
    let spliced = match entry {
        Some(v) => jsonc::set(t, &f.container, key, v),
        None => jsonc::remove(t, &f.container, key),
    };
    if let Some(s) = spliced
        && serde_json::from_str::<Value>(&jsonc::strip(&s)).is_ok_and(|v| v == want)
    {
        return Ok(s);
    }
    if jsonc::has_comments(t) {
        return Err("it has comments Scrim couldn't keep while changing it: copy the entry and add it yourself".into());
    }
    serde_json::to_string_pretty(&want).map(|s| s + "\n").map_err(|e| e.to_string())
}

/// Make `change` to file client `f`'s file at `path`, safely, as Claude Desktop's is (F1). Only the
/// window calls this, after the person confirmed; it refuses inside an agent's request.
// The file is the client's, found from the platform's places, and written only by the person's
// confirmed action: automation roots don't apply, and the backup and the rename need the file
// system directly.
#[allow(clippy::disallowed_methods)]
pub fn apply(f: &FileSpec, path: &Path, change: &Change, now_secs: u64) -> Result<Written, String> {
    if crate::file_access::current().is_some() {
        return Err("another app's configuration is only written by the person, in Scrim".into());
    }
    if !f.write {
        return Err("Scrim doesn't write this file".into());
    }
    let before = match std::fs::read(path) {
        Ok(b) => Some(String::from_utf8(b).map_err(|_| "it isn't UTF-8 text".to_string())?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    let text = changed(f, before.as_deref(), change)?;
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
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(Written { path: path.to_path_buf(), backup })
}

/// The generic card's snippets, for any MCP client (`json`, `toml`, or `stdio`).
pub fn generic(s: &Session, format: &str) -> crate::Result<String> {
    let cmd = "mcp.copy-agent-snippet";
    let url = s.agents.mcp.url().ok_or_else(|| bad(cmd, "the MCP endpoint isn't running: restart it from Settings, under MCP"))?;
    let token = super::mcp::token(s).is_some();
    Ok(match format {
        "json" => {
            let mut e = json!({"url": url});
            if token {
                e["headers"] = json!({"Authorization": format!("Bearer ${{{TOKEN_VAR}}}")});
            }
            serde_json::to_string_pretty(&json!({"mcpServers": {KEY: e}})).unwrap_or_default()
        }
        "toml" => {
            let mut t = format!("[mcp_servers.{KEY}]\nurl = \"{url}\"\n");
            if token {
                t.push_str(&format!("bearer_token_env_var = \"{TOKEN_VAR}\"\n"));
            }
            t
        }
        "stdio" => match s.agents.control_port {
            Some(port) => crate::agents::connect_command(Some(port)).replacen("claude mcp add scrim-app -- ", "", 1),
            None => format!("npx -y {} {url}", super::claude_desktop::MCP_REMOTE),
        },
        other => return Err(bad(cmd, format!("`format` is json, toml or stdio, not {other}"))),
    })
}

/// A command line as a person would type it (arguments with spaces or `$` quoted for a POSIX
/// shell). Only for showing and copying: Scrim itself runs the argument list, never this text.
pub fn command_line(program: &str, args: &[String]) -> String {
    let quoted: Vec<String> =
        args.iter().map(|x| if x.contains([' ', '"', '{', '$']) { format!("'{}'", x.replace('\'', "'\\''")) } else { x.clone() }).collect();
    format!("{program} {}", quoted.join(" "))
}

/// What Copy puts on the clipboard for a client's report: its command, else its file entry, else
/// its snippet.
pub fn copy_text(r: &Value) -> String {
    let args = |v: &Value| v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect::<Vec<_>>());
    match (args(&r["cli"]["add"]), r["file"]["text"].as_str(), r["snippet"]["text"].as_str()) {
        (Some(a), _, _) => command_line(r["cli"]["program"].as_str().unwrap_or_default(), &a),
        (None, Some(t), _) => t.to_string(),
        (None, None, Some(t)) => t.to_string(),
        _ => String::new(),
    }
}

fn client_of(cmd: &str, p: &Value) -> crate::Result<String> {
    let id = p.get("client").and_then(Value::as_str).ok_or_else(|| bad(cmd, "`client` is required: an id from mcp.agent-clients"))?;
    profile(id).map(|_| id.to_string()).ok_or_else(|| bad(cmd, format!("no client `{id}`: mcp.agent-clients lists them")))
}

/// The confirm-only commands: only the person, in Scrim, may change another app's configuration.
fn ask(s: &mut Session, cmd: &str, p: &Value, dialog: &str) -> crate::Result<Value> {
    let id = client_of(cmd, p)?;
    if s.agents.author.is_agent() {
        return Err(EngineError::Disabled(
            cmd.into(),
            "only the person can change another app's configuration, in Scrim: an agent can read the status (mcp.client-status) and copy the entry"
                .into(),
        ));
    }
    s.request_ui(dialog, json!({"client": id}));
    Ok(json!({"asked": true}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "mcp.agent-clients",
            "MCP clients Scrim can connect",
            "{} → {clients: [{id, name, display, registration: {level, date?, evidence?}, toolUse: {level, date?, evidence?}, docs, state, builtin}]}  (every client with its status read from its own configuration, and how far adding Scrim to it and using Scrim's tools from it were verified; never \"connected\")",
            false,
            |s, _| {
                let list: Vec<Value> = profiles()
                    .iter()
                    .map(|p| {
                        let r = report(s, &p.id).unwrap_or_else(|_| json!({"state": State::Unknown.id()}));
                        json!({"id": p.id, "name": p.name, "display": r["display"], "registration": p.registration, "toolUse": p.tool_use, "docs": p.docs, "state": r["state"], "builtin": p.builtin})
                    })
                    .collect();
                Ok(json!({"clients": list}))
            }
        ),
        spec!(
            "mcp.client-status",
            "Agent client status",
            "{client} → {id, name, docs, registration, toolUse, cli?: {program, add, remove}, file?: {path, documented, format, writable, entry, text}, snippet?, state: configured|not configured|out of date|unreadable|unknown, registered?, error?, old, token, notes}  (read from the client's own file; an agent's request can't read it)",
            false,
            |s, p| report(s, &client_of("mcp.client-status", p)?)
        ),
        spec!(
            "mcp.connect-client",
            "Connect an agent",
            "{client} → the same as mcp.client-status  (shows the client's panel in the app; nothing changes until the person confirms there)",
            false,
            |s, p| {
                let id = client_of("mcp.connect-client", p)?;
                let r = report(s, &id)?;
                s.request_ui(CONNECT_REQUEST, json!({"client": id}));
                Ok(r)
            }
        ),
        spec!(
            "mcp.write-client",
            "Add Scrim to an agent's configuration",
            "{client} → {asked: true}  (asks the person first; refused for agents)",
            false,
            |s, p| ask(s, "mcp.write-client", p, WRITE_DIALOG)
        ),
        spec!(
            "mcp.reregister-client",
            "Re-register Scrim with an agent",
            "{client} → {asked: true}  (replaces an out-of-date scrim entry after the person confirms; refused for agents)",
            false,
            |s, p| ask(s, "mcp.reregister-client", p, REREGISTER_DIALOG)
        ),
        spec!(
            "mcp.remove-old-client-entry",
            "Remove an agent's old Scrim entry",
            "{client} → {asked: true}  (removes figcraft / figcraft-app after the person confirms; refused for agents)",
            false,
            |s, p| ask(s, "mcp.remove-old-client-entry", p, REMOVE_OLD_DIALOG)
        ),
        spec!(
            "mcp.copy-client-entry",
            "Copy an agent's Scrim entry",
            "{client} → {text}  (the command, the file entry or the snippet, onto the clipboard in the app)",
            false,
            |s, p| {
                let id = client_of("mcp.copy-client-entry", p)?;
                let r = report(s, &id)?;
                let text = copy_text(&r);
                s.request_ui(
                    super::code::COPY_REQUEST,
                    json!({"text": text, "what": format!("the {} entry", r["name"].as_str().unwrap_or_default())}),
                );
                Ok(json!({"text": text}))
            }
        ),
        spec!(
            "mcp.copy-agent-snippet",
            "Copy the connection for any agent",
            "{format: json|toml|stdio} → {text}  (a server entry for any MCP client, or the stdio command, onto the clipboard)",
            false,
            |s, p| {
                let format = p.get("format").and_then(Value::as_str).unwrap_or("json");
                let text = generic(s, format)?;
                s.request_ui(super::code::COPY_REQUEST, json!({"text": text, "what": "the connection"}));
                Ok(json!({"text": text}))
            }
        ),
    ]
}

#[cfg(test)]
#[path = "agent_clients_tests.rs"]
mod tests;
