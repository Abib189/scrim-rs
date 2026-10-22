//! "Describe a change", the AI tab's composer (R5, `docs/design/chat-composer.md`): its commands.
//! - **The person's alone.** Every command here is refused for agents, on every route (MCP, the
//!   control channel, the headless CLI): an agent can't start, stop or set up agent runs.
//! - **A Scrim started by an agent run** (`SCRIM_CHAT_CHILD` in its environment,
//!   [`crate::agents::Agents::chat_child`]) refuses `chat.send`, so runs can't nest.
//! - **The commands only ask the window** (`Session::request_ui`): the window starts, stops and
//!   shows the run. They change nothing in the document.

use serde_json::{Value, json};

use super::CommandSpec;
use super::agent_clients::headless;
use crate::settings::ChatAllowed;
use crate::{Result, Session, bad, spec};

/// What the window is asked to do.
pub const SEND_REQUEST: &str = "chat.send";
pub const CANCEL_REQUEST: &str = "chat.cancel";
pub const CLEAR_REQUEST: &str = "chat.clear";
/// An agent was confirmed: the send waiting for it goes ahead.
pub const ALLOWED_REQUEST: &str = "chat.allowed";
/// The longest prompt, in characters.
pub const MAX_PROMPT: usize = 20_000;
/// The time limits Settings offers, in minutes.
pub const TIME_LIMITS: [u32; 4] = [5, 15, 30, 60];
pub const DEFAULT_TIME_LIMIT: u32 = 15;
/// The agent picked when none is named.
pub const DEFAULT_AGENT: &str = "claude-code";

/// Every command here: none of them is an agent's to run.
pub const PERSON_ONLY: [&str; 7] =
    ["chat.send", "chat.cancel", "chat.clear", "chat.allow-agent", "chat.forget-agents", "pref.describeChange", "pref.chatTimeLimit"];

/// Whether this process was started by an agent run: `SCRIM_CHAT_CHILD` set and not empty (its
/// value, `std::env::var_os`). The app reads it at startup.
pub fn started_by_agent_run(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|v| !v.is_empty())
}

/// Refused when an agent asks, whatever its route.
fn person_only(s: &Session, id: &str) -> Result<()> {
    if s.agents.author.is_agent() {
        return Err(bad(id, "Describe a change is the person's: agents can't start, stop or set up agent runs"));
    }
    Ok(())
}

fn agent_id(p: &Value, cmd: &str) -> Result<&'static headless::Headless> {
    let id = p.get("agent").and_then(Value::as_str).unwrap_or(DEFAULT_AGENT);
    headless::profile(id).ok_or_else(|| {
        let known: Vec<&str> = headless::profiles().iter().map(|h| h.id.as_str()).collect();
        bad(cmd, format!("no agent `{id}`: {}", known.join(" or ")))
    })
}

fn send(s: &mut Session, p: &Value) -> Result<Value> {
    person_only(s, "chat.send")?;
    if s.agents.chat_child {
        return Err(bad("chat.send", "this Scrim was started by an agent run, so it can't start another"));
    }
    if !s.prefs.settings.describe_change {
        return Err(bad("chat.send", "Describe a change is off in Settings ▸ AI & agents"));
    }
    let prompt = p.get("prompt").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if prompt.is_empty() {
        return Err(bad("chat.send", "`prompt` is required: what to change"));
    }
    if prompt.chars().count() > MAX_PROMPT {
        return Err(bad("chat.send", format!("the prompt is longer than {MAX_PROMPT} characters")));
    }
    let agent = agent_id(p, "chat.send")?;
    s.request_ui(SEND_REQUEST, json!({"prompt": prompt, "agent": agent.id}));
    Ok(json!({"sent": true, "agent": agent.id}))
}

fn cancel(s: &mut Session, _: &Value) -> Result<Value> {
    person_only(s, "chat.cancel")?;
    s.request_ui(CANCEL_REQUEST, json!({}));
    Ok(json!({"stopping": true}))
}

fn clear(s: &mut Session, _: &Value) -> Result<Value> {
    person_only(s, "chat.clear")?;
    s.request_ui(CLEAR_REQUEST, json!({}));
    Ok(json!({"cleared": true}))
}

/// The person confirmed running `agent` from `path` (the first-use confirmation): remembered, and
/// the send waiting for it goes ahead.
fn allow(s: &mut Session, p: &Value) -> Result<Value> {
    person_only(s, "chat.allow-agent")?;
    let agent = agent_id(p, "chat.allow-agent")?;
    let path = p
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .ok_or_else(|| bad("chat.allow-agent", "`path` is required"))?;
    let entry = ChatAllowed { agent: agent.id.clone(), path: path.to_string() };
    if !s.prefs.settings.chat_allowed.contains(&entry) {
        s.prefs.settings.chat_allowed.push(entry);
    }
    s.request_ui(ALLOWED_REQUEST, json!({"agent": agent.id, "path": path}));
    Ok(json!({"allowed": true, "agent": agent.id, "path": path}))
}

fn forget(s: &mut Session, _: &Value) -> Result<Value> {
    person_only(s, "chat.forget-agents")?;
    let n = s.prefs.settings.chat_allowed.len();
    s.prefs.settings.chat_allowed.clear();
    Ok(json!({"forgotten": n}))
}

/// Whether `agent` at `path` was confirmed before.
pub fn allowed(s: &Session, agent: &str, path: &str) -> bool {
    s.prefs.settings.chat_allowed.iter().any(|a| a.agent == agent && a.path == path)
}

fn describe_change(s: &mut Session, p: &Value) -> Result<Value> {
    person_only(s, "pref.describeChange")?;
    let on = p.get("on").and_then(Value::as_bool).ok_or_else(|| bad("pref.describeChange", "`on` is required: true or false"))?;
    s.prefs.settings.describe_change = on;
    if !on {
        s.request_ui(CANCEL_REQUEST, json!({}));
    }
    Ok(json!({"on": on}))
}

fn time_limit(s: &mut Session, p: &Value) -> Result<Value> {
    person_only(s, "pref.chatTimeLimit")?;
    let m = p
        .get("minutes")
        .and_then(Value::as_u64)
        .and_then(|m| u32::try_from(m).ok())
        .filter(|m| TIME_LIMITS.contains(m))
        .ok_or_else(|| bad("pref.chatTimeLimit", "`minutes` is 5, 15, 30 or 60"))?;
    s.prefs.settings.chat_time_limit_mins = m;
    Ok(json!({"minutes": m}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "chat.send",
            "Describe a change",
            "{prompt, agent?: claude-code|codex} → {sent, agent}  (the person's alone: runs that agent's CLI on this canvas; agents are refused)",
            false,
            send
        ),
        spec!("chat.cancel", "Stop the agent run", "{} → {stopping}  (the person's alone; agents are refused)", false, cancel),
        spec!("chat.clear", "Clear the conversation", "{} → {cleared}  (the person's alone; agents are refused)", false, clear),
        spec!(
            "chat.allow-agent",
            "Allow an agent to run",
            "{agent, path} → {allowed}  (the first-use confirmation: remembered for that agent and binary; the person's alone)",
            false,
            allow
        ),
        spec!(
            "chat.forget-agents",
            "Forget allowed agents",
            "{} → {forgotten}  (each agent asks again before its next run; the person's alone)",
            false,
            forget
        ),
        spec!(
            "pref.describeChange",
            "Describe a change",
            "{on: bool} → {on}  (the AI tab's composer on or off; the person's alone)",
            false,
            describe_change
        ),
        spec!("pref.chatTimeLimit", "Stop an agent run after", "{minutes: 5|15|30|60} → {minutes}  (the person's alone)", false, time_limit),
    ]
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod tests;
