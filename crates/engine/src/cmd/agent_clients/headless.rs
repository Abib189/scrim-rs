//! Agents Scrim can run itself, for the AI tab's composer (R5, `docs/design/chat-composer.md`).
//! - **A headless profile** is data (`headless.json`, next to the client profiles): the agent's
//!   name, its program (found by the window, `Services::find_program`), its argument template, how
//!   its output reads, its docs, and the `clientInfo` names it announces over MCP.
//! - **The arguments** are the template with its placeholders filled in, each argument on its own:
//!   nothing is split, joined or quoted, and the program is started from that list, never a shell,
//!   so nothing in a prompt is ever interpreted.
//! - **The token** never goes in an argument: the child gets it in its environment
//!   ([`TOKEN_VAR`]), and the arguments name that variable.
//! - **The environment** is minimal ([`child_env`]): `PATH`, `HOME`, the token when authentication
//!   is on, and [`CHILD_VAR`].
//! - **What is sent** is the prompt and one context line ([`context_line`]).

use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::json;

use crate::Session;

#[path = "headless_output.rs"]
pub mod output;

/// The child's environment variable holding the bearer token, when authentication is on.
pub const TOKEN_VAR: &str = "SCRIM_MCP_AUTH";
/// Set in every agent run's environment: a Scrim started from one refuses to start another.
pub const CHILD_VAR: &str = "SCRIM_CHAT_CHILD";
/// The `HOME` given to agent runs, when set (live checks: the app on a scratch `HOME`, the CLI with
/// its own login). Unset: the app's own `HOME`.
pub const CHILD_HOME_VAR: &str = "SCRIM_CHAT_CHILD_HOME";
/// What the confirmation shows in place of the prompt.
pub const PROMPT_SHOWN: &str = "<your message>";

/// How an agent's output reads.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Output {
    /// Claude Code's `--output-format stream-json`: one event per line, the last a `result`.
    #[default]
    ClaudeStreamJson,
    /// Codex's `exec --json`: one event per line.
    CodexJsonl,
}

/// An agent Scrim can run.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Headless {
    pub id: String,
    pub name: String,
    pub program: String,
    /// With `{prompt}`, `{mcp_config}`, `{mcp_url}`, `{token_var}`, and `{auth_args}` (an argument
    /// of its own: replaced by [`Headless::auth_args`] with authentication on, else by nothing).
    pub args: Vec<String>,
    #[serde(default)]
    pub auth_args: Vec<String>,
    pub output: Output,
    pub docs: String,
    #[serde(default)]
    pub client_names: Vec<String>,
    /// Where the flags were read, and when.
    #[serde(default)]
    pub verified: String,
}

/// Every headless profile, in the picker's order.
pub fn profiles() -> &'static [Headless] {
    static ALL: OnceLock<Vec<Headless>> = OnceLock::new();
    ALL.get_or_init(|| serde_json::from_str(include_str!("headless.json")).unwrap_or_default())
}

pub fn profile(id: &str) -> Option<&'static Headless> {
    profiles().iter().find(|p| p.id == id)
}

/// The MCP configuration Claude Code takes: Scrim's server alone, over HTTP. With authentication
/// on, its header names the token's variable (expanded by the CLI), never the token.
pub fn mcp_config(url: &str, auth: bool) -> String {
    let mut server = json!({"type": "http", "url": url});
    if auth {
        server["headers"] = json!({"Authorization": format!("Bearer ${{{TOKEN_VAR}}}")});
    }
    json!({"mcpServers": {"scrim": server}}).to_string()
}

fn fill(arg: &str, prompt: &str, url: &str, auth: bool) -> String {
    arg.replace("{mcp_config}", &mcp_config(url, auth)).replace("{mcp_url}", url).replace("{token_var}", TOKEN_VAR).replace("{prompt}", prompt)
}

impl Headless {
    /// The arguments for a run: the template filled in, one argument each.
    pub fn argv(&self, prompt: &str, url: &str, auth: bool) -> Vec<String> {
        let mut out = Vec::new();
        for a in &self.args {
            if a == "{auth_args}" {
                if auth {
                    out.extend(self.auth_args.iter().map(|x| fill(x, prompt, url, auth)));
                }
                continue;
            }
            out.push(fill(a, prompt, url, auth));
        }
        out
    }

    /// What the first-use confirmation shows: the binary at `path` and its arguments, with
    /// `<your message>` for the prompt.
    pub fn display(&self, path: &str, url: &str, auth: bool) -> String {
        super::command_line(path, &self.argv(PROMPT_SHOWN, url, auth))
    }

    /// Whether `client` (a `clientInfo.name`) is this agent's.
    pub fn announces(&self, client: &str) -> bool {
        self.client_names.iter().any(|n| n.eq_ignore_ascii_case(client))
    }
}

/// The child's environment, and nothing else: `PATH`, `HOME`, the token when authentication is
/// on, and [`CHILD_VAR`]. `system` are the variables the platform can't start a program without
/// (Windows: `SYSTEMROOT`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TEMP`; elsewhere none).
pub fn child_env(path: Option<&str>, home: Option<&str>, token: Option<&str>, system: &[(String, String)]) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    if let Some(p) = path {
        env.push(("PATH".into(), p.into()));
    }
    if let Some(h) = home {
        env.push(("HOME".into(), h.into()));
    }
    if let Some(t) = token {
        env.push((TOKEN_VAR.into(), t.into()));
    }
    env.push((CHILD_VAR.into(), "1".into()));
    env.extend(system.iter().filter(|(k, _)| !env.iter().any(|(e, _)| e == k)).cloned().collect::<Vec<_>>());
    env
}

/// The variables Windows can't start a program without, from this process (none elsewhere).
pub fn system_env() -> Vec<(String, String)> {
    if !cfg!(windows) {
        return Vec::new();
    }
    ["SYSTEMROOT", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP"]
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| ((*k).to_string(), v)))
        .collect()
}

/// The one context line sent after the prompt: the page's name and the selected layers (id and
/// name). Nothing else of the document.
pub fn context_line(s: &Session) -> String {
    let Ok(ds) = s.ds() else { return "Scrim: no file is open.".into() };
    let page = ds.doc.node(ds.page).map(|n| n.name.clone()).unwrap_or_default();
    let selected: Vec<String> =
        s.selection().iter().map(|id| format!("“{}” ({})", ds.doc.node(*id).map(|n| n.name.as_str()).unwrap_or(""), id.0)).collect();
    if selected.is_empty() {
        format!("Scrim: page “{page}”; nothing is selected.")
    } else {
        format!("Scrim: page “{page}”; selected: {}.", selected.join(", "))
    }
}

/// The message for the agent: the prompt, then the context line.
pub fn message(prompt: &str, s: &Session) -> String {
    format!("{}\n\n{}", prompt.trim(), context_line(s))
}

/// A line of the child's error output, safe to show: the token itself, bearer values, and API-key
/// shapes replaced by `<token>`, capped at 300 characters.
pub fn redact(line: &str, token: Option<&str>) -> String {
    static PATTERNS: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]{8,}",
            r"\b(sk-)[A-Za-z0-9_-]{8,}",
            r"\b(gh[pousr]_)[A-Za-z0-9]{8,}",
            r#"(?i)((?:api[_-]?key|token|secret|password)["']?\s*[:=]\s*["']?)[^\s"',}]{6,}"#,
        ]
        .iter()
        .filter_map(|p| regex::Regex::new(p).ok())
        .collect()
    });
    let mut out = match token.filter(|t| t.len() >= 4) {
        Some(t) => line.replace(t, "<token>"),
        None => line.to_string(),
    };
    for re in patterns {
        out = re.replace_all(&out, "${1}<token>").into_owned();
    }
    out.trim().chars().take(300).collect()
}

#[cfg(test)]
#[path = "headless_tests.rs"]
mod tests;
