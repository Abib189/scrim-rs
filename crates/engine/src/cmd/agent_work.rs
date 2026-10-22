//! An agent's working state (Stage I.1, `docs/design/working-state.md`). The window shows an agent
//! as working while its requests keep coming, within the grace period after the last, until it
//! says it's done or its session ends; these commands are the engine's part.
//! - **`agent.finish-working {ids?}`** (the MCP tool `finish_working`): the calling client is done.
//!   It asks the window to end that client's working state ([`FINISH_REQUEST`]). It reads nothing
//!   and changes nothing: no document, undo, revision or journal entry (it isn't undoable, and a
//!   UI request isn't a change). Classified Read, so it runs with only read-only tools on.
//! - **`pref.agentGrace {seconds}`:** the grace period, one of [`AGENT_GRACE_SECS`] (5 s at
//!   least). Saved in Settings.

use serde_json::{Value, json};

use super::CommandSpec;
use crate::settings::AGENT_GRACE_SECS;
use crate::{Result, Session, bad, spec};

/// The UI request that ends a client's working state: `{client, session, route, ids}`.
pub const FINISH_REQUEST: &str = "agent.finishWorking";

fn finish(s: &mut Session, p: &Value) -> Result<Value> {
    let ids: Vec<u64> = match p.get("ids") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| crate::params::id_of(v).map(|id| id.0).ok_or_else(|| bad("agent.finish-working", "`ids` are layer ids")))
            .collect::<Result<_>>()?,
        Some(_) => return Err(bad("agent.finish-working", "`ids` is a list of artboard ids")),
    };
    let route = match &s.agents.author {
        crate::agents::Author::Agent(r) => Some(r.clone()),
        crate::agents::Author::Person => None,
    };
    let client = s.agents.client.clone();
    let session = s.agents.session.clone();
    s.request_ui(FINISH_REQUEST, json!({"client": client, "session": session, "route": route, "ids": ids}));
    Ok(json!({"finished": true, "client": client, "ids": ids}))
}

/// The grace a test may set through the hook (never offered, never saved).
pub const TEST_GRACE_SECS: u32 = 5;

/// `pref.agentGrace`: the person's choice, and it is marked as chosen so a migration keeps it.
/// Agents are refused (their choice would outlive their session, and a 5 s an agent set is what
/// cut the dots between a real agent's steps, Fix Q). With the test hook on, 5 s is taken for this
/// session only.
fn grace(s: &mut Session, p: &Value) -> Result<Value> {
    let secs = p.get("seconds").and_then(Value::as_u64).ok_or_else(|| bad("pref.agentGrace", "`seconds` is required: 15, 30, 60 or 120"))?;
    if s.agents.test_hooks && secs == u64::from(TEST_GRACE_SECS) {
        s.agents.grace_for_tests = Some(TEST_GRACE_SECS);
        return Ok(json!({"seconds": secs, "test": true}));
    }
    if matches!(s.agents.author, crate::agents::Author::Agent(_)) && !s.agents.test_hooks {
        return Err(bad("pref.agentGrace", "only the person sets how long the working indicator stays, in Settings ▸ General"));
    }
    let secs = u32::try_from(secs)
        .ok()
        .filter(|s| AGENT_GRACE_SECS.contains(s))
        .ok_or_else(|| bad("pref.agentGrace", format!("`seconds` is 15, 30, 60 or 120, not {secs}")))?;
    s.prefs.settings.agent_grace_secs = secs;
    s.prefs.settings.agent_grace_chosen = true;
    s.agents.grace_for_tests = None;
    Ok(json!({"seconds": secs}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "agent.finish-working",
            "Finish working",
            "{ids? (the artboards you worked on)} → {finished: true, client, ids}  (you are done on the canvas: the window stops showing you as working. Changes nothing in the document)",
            false,
            finish
        ),
        spec!(
            "pref.agentGrace",
            "Keep the working indicator after the last agent request",
            "{seconds: 15|30|60|120} → {seconds}  (how long an agent still counts as working after its last request, unless it calls finish_working. Saved in Settings; the person's alone: agents are refused)",
            false,
            grace
        ),
    ]
}

#[cfg(test)]
#[path = "agent_work_tests.rs"]
mod tests;
