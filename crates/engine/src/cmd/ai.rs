//! The AI tab's commands (`docs/design/panel-redesign.md`, B3): the agent channel's state and the
//! command that connects an agent, selecting the layers of a change from the feed or of a lint
//! finding, and undoing an agent's change while it is still the latest step. The feed itself is
//! `history.changes` (Port 3) and the findings are `lint.run` (Port 6): the tab reads the same
//! commands an agent can run. No chat, no API keys, no provider settings.

use std::collections::BTreeMap;

use figcraft_doc::NodeId;
use serde_json::{Value, json};

use crate::cmd::{CommandSpec, find_command};
use crate::{EngineError, Result, Session, bad, spec};

/// Why an agent's change can't be undone from the feed.
pub const NOT_LATEST: &str = "later edits came after this change: undo them first";

fn revision_of(cmd: &str, p: &Value) -> Result<u64> {
    p.get("revision").and_then(Value::as_u64).ok_or_else(|| bad(cmd, "`revision` is required (from history.changes)"))
}

/// The active document's logged change at `revision`, as `history.changes` reports it.
fn change(s: &Session, cmd: &str, revision: u64) -> Result<Value> {
    let all = s.changes_since(revision.saturating_sub(1))?;
    all["changes"]
        .as_array()
        .and_then(|c| c.iter().find(|c| c["revision"] == revision).cloned())
        .ok_or_else(|| bad(cmd, "no change at that revision in this file's log (history.changes lists them)"))
}

fn select(s: &mut Session, ids: Vec<NodeId>) -> Result<Value> {
    let run = find_command("select.set").ok_or_else(|| EngineError::UnknownCommand("select.set".into()))?.run;
    run(s, &json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>()}))
}

/// `ai.status`: whether agents can attach to this app, and the command that connects one.
fn status(s: &mut Session, _: &Value) -> Result<Value> {
    let port = s.agents.control_port;
    Ok(json!({
        "channel": if port.is_some() { "on" } else { "off" },
        "port": port,
        "connectCommand": crate::agents::connect_command(port),
    }))
}

/// `ai.selectChange`: select the layers a change touched that still exist. Pages, and layers
/// listed only as the parent of another changed layer (their children changed), are left out.
fn select_change(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "ai.selectChange";
    let c = change(s, cmd, revision_of(cmd, p)?)?;
    let doc = s.doc()?;
    let ids: Vec<NodeId> = c["layers"].as_array().into_iter().flatten().filter_map(|l| l["id"].as_u64().map(NodeId)).collect();
    let (mut here, mut removed) = (Vec::new(), Vec::new());
    for &id in &ids {
        match doc.node(id) {
            None => removed.push(id.0),
            Some(n) if n.is_page() => {}
            Some(_) if ids.iter().any(|&other| other != id && doc.ancestors(other).contains(&id)) => {}
            Some(_) => here.push(id),
        }
    }
    select(s, here.clone())?;
    Ok(json!({"selected": here.iter().map(|i| i.0).collect::<Vec<_>>(), "removed": removed}))
}

/// Whether the change at `revision` is an agent's and still the latest undo step: the last entry
/// in the log, with the top undo step its own ("Agent: …").
fn undoable(s: &Session, revision: u64) -> Result<Value> {
    let all = s.changes_since(revision.saturating_sub(1))?;
    let changes = all["changes"].as_array().cloned().unwrap_or_default();
    let Some(c) = changes.iter().find(|c| c["revision"] == revision) else { return Ok(json!(false)) };
    let last = changes.last().is_some_and(|l| l["revision"] == revision);
    let top = s.undo_labels(1);
    let own = top.first().is_some_and(|t| Some(t.as_str()) == c["label"].as_str().map(|l| format!("Agent: {l}")).as_deref());
    Ok(json!(c["agent"] == true && last && own))
}

/// `ai.undoChange`: undo an agent's change while it is still the latest step.
fn undo_change(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "ai.undoChange";
    let revision = revision_of(cmd, p)?;
    let c = change(s, cmd, revision)?;
    if c["agent"] != true {
        return Err(bad(cmd, "that change is the person's: use Edit ▸ Undo"));
    }
    if undoable(s, revision)? != true {
        return Err(EngineError::Disabled(cmd.into(), NOT_LATEST.into()));
    }
    Ok(json!({"undone": s.undo()?, "revision": revision, "label": c["label"]}))
}

/// `ai.findings`: the design lint's findings grouped by rule, largest group first.
fn findings(s: &mut Session, _: &Value) -> Result<Value> {
    let run = find_command("lint.run").ok_or_else(|| EngineError::UnknownCommand("lint.run".into()))?.run;
    let lint = run(s, &json!({}))?;
    let mut groups: BTreeMap<String, (String, Vec<Value>)> = BTreeMap::new();
    for v in lint["violations"].as_array().into_iter().flatten() {
        let rule = v["ruleId"].as_str().unwrap_or_default().to_string();
        let entry = groups.entry(rule).or_insert_with(|| (v["name"].as_str().unwrap_or_default().to_string(), Vec::new()));
        entry.1.push(json!({"id": v["nodeId"], "name": v["nodeName"], "message": v["message"], "detail": v["detail"]}));
    }
    let mut out: Vec<Value> = groups
        .into_iter()
        .map(|(rule, (name, f))| {
            let mut g = json!({"rule": rule, "name": name, "count": f.len()});
            if BY_FAMILY.contains(&rule.as_str()) {
                g["families"] = Value::Array(families(&f));
            }
            g["findings"] = Value::Array(f);
            g
        })
        .collect();
    out.sort_by(|a, b| b["count"].as_u64().cmp(&a["count"].as_u64()));
    Ok(json!({"count": lint["violationCount"], "groups": out}))
}

/// Rules whose findings the AI tab groups by font family (C1.7): one row per family, with how many
/// layers use it. The lint names the family in each finding's `detail`, lowercase.
const BY_FAMILY: &[&str] = &["overused-font"];

/// A family's name as people write it ("plus jakarta sans" → "Plus Jakarta Sans").
fn family_name(detail: &str) -> String {
    detail
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `findings` grouped by family, the most used first: [{family, name, count, findings}].
fn families(findings: &[Value]) -> Vec<Value> {
    let mut by: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for f in findings {
        by.entry(f["detail"].as_str().unwrap_or_default().to_string()).or_default().push(f.clone());
    }
    let mut out: Vec<Value> =
        by.into_iter().map(|(fam, f)| json!({"family": fam, "name": family_name(&fam), "count": f.len(), "findings": f})).collect();
    out.sort_by(|a, b| b["count"].as_u64().cmp(&a["count"].as_u64()).then_with(|| a["name"].as_str().cmp(&b["name"].as_str())));
    out
}

/// `ai.selectFinding`: select a finding's layer (`index` within its rule's group, or within
/// `family`), or every layer of the rule (or of `family`) without `index`.
fn select_finding(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "ai.selectFinding";
    let rule = p.get("rule").and_then(Value::as_str).ok_or_else(|| bad(cmd, "`rule` is required (from ai.findings)"))?.to_string();
    let all = findings(s, &json!({}))?;
    let group = all["groups"]
        .as_array()
        .and_then(|g| g.iter().find(|g| g["rule"] == rule.as_str()).cloned())
        .ok_or_else(|| bad(cmd, "no findings for that rule"))?;
    let list = match p.get("family").and_then(Value::as_str) {
        None => group["findings"].as_array().cloned().unwrap_or_default(),
        Some(fam) => group["families"]
            .as_array()
            .and_then(|f| f.iter().find(|f| f["family"] == fam))
            .and_then(|f| f["findings"].as_array().cloned())
            .ok_or_else(|| bad(cmd, "no findings for that family in this rule"))?,
    };
    let picked: Vec<&Value> = match p.get("index").and_then(Value::as_u64) {
        Some(i) => vec![list.get(i as usize).ok_or_else(|| bad(cmd, "`index` is past the rule's findings"))?],
        None => list.iter().collect(),
    };
    let mut ids: Vec<NodeId> = picked.iter().filter_map(|f| f["id"].as_u64().map(NodeId)).collect();
    ids.dedup();
    select(s, ids)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!("ai.status", "Agent connection", "{} → {channel: on|off, port, connectCommand}", false, status),
        spec!(
            "ai.copyConnectCommand",
            "Copy the agent connect command",
            "{} → {connectCommand}  (onto the clipboard in the app)",
            false,
            |s, p| {
                let r = status(s, p)?;
                s.request_ui(super::code::COPY_REQUEST, json!({"text": r["connectCommand"], "what": "the connect command"}));
                Ok(json!({"connectCommand": r["connectCommand"]}))
            }
        ),
        spec!(
            "ai.selectChange",
            "Select a change's layers",
            "{revision} → {selected, removed}  (a change from history.changes)",
            false,
            select_change
        ),
        spec!(
            "ai.undoChange",
            "Undo an agent's change",
            "{revision} → {undone, revision, label}  (only an agent's change that is still the latest undo step; otherwise refused with the reason)",
            false,
            undo_change
        ),
        spec!(
            "ai.findings",
            "Design lint findings by rule",
            "{} → {count, groups: [{rule, name, count, findings: [{id, name, message, detail}], families? (overused-font: [{family, name, count, findings}], most used first)}]}  (lint.run, grouped; largest group first)",
            false,
            findings
        ),
        spec!(
            "ai.selectFinding",
            "Select a finding's layers",
            "{rule, family? (overused-font: one family's findings), index? (default: every layer of the rule, or of the family)} → the selection",
            false,
            select_finding
        ),
    ]
}

/// Whether the AI tab can offer Undo on the change at `revision` (see `ai.undoChange`).
pub fn can_undo(s: &Session, revision: u64) -> bool {
    undoable(s, revision).is_ok_and(|v| v == true)
}

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
