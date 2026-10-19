//! Agents and people on one document (AGENTS.md › Scrim's rules 5; `docs/scrim-transfer/
//! decisions*.md`):
//! - Who made each edit: the person (the app's own input) or an agent (MCP, the CLI's `run`,
//!   the control channel, a plugin). Each route sets its author ([`Session::set_author`]); a
//!   caller never names itself.
//! - Locks bind agents. An agent's command that would change a locked layer, or anything inside
//!   one, is refused with the reason and rolled back. The person's own edits are unchanged.
//! - A stale-write guard. An agent may say which revision it last read (`since`); a command that
//!   changes a layer someone else changed after that revision is refused, saying what changed.
//! - The change feed. Every edit is logged with its revision, author, command and layers
//!   (`history.changes {since}`, the MCP tool `get_changes`). Agents' undo steps read "Agent: …".

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Arc;

use figcraft_doc::{Document, NodeId};
use serde_json::{Value, json};

use crate::cmd::CommandSpec;
use crate::{EngineError, Result, Session, spec};

/// Who runs the commands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Author {
    /// The person at the app (its own input).
    #[default]
    Person,
    /// An agent, by its route ("mcp", "cli", "control channel", "plugin Tidy names").
    Agent(String),
}

impl Author {
    pub fn agent(route: &str) -> Self {
        Author::Agent(route.to_string())
    }

    pub fn is_agent(&self) -> bool {
        matches!(self, Author::Agent(_))
    }

    /// "person", or "agent (mcp)".
    pub fn describe(&self) -> String {
        match self {
            Author::Person => "person".into(),
            Author::Agent(route) => format!("agent ({route})"),
        }
    }
}

/// One logged edit.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub revision: u64,
    pub author: Author,
    pub command: String,
    pub label: String,
    /// The layers the edit added, changed or removed.
    pub ids: Vec<NodeId>,
}

/// How many edits each document's log keeps.
const LOG_LIMIT: usize = 2000;

/// The session's agent state: who runs the current commands, the revision the agent last read,
/// and each document's change log (by the document's `uid`).
#[derive(Clone, Debug, Default)]
pub struct Agents {
    pub author: Author,
    pub since: Option<u64>,
    logs: HashMap<u64, VecDeque<Change>>,
}

impl Agents {
    /// The undo step's label: an agent's steps say so.
    pub(crate) fn step_label(&self, label: &str) -> String {
        if self.author.is_agent() { format!("Agent: {label}") } else { label.to_string() }
    }
}

/// Layers added, changed or removed from `before` to `after` (shared nodes are unchanged).
pub(crate) fn changed_ids(before: &Document, after: &Document) -> Vec<NodeId> {
    let mut ids: BTreeSet<NodeId> =
        after.nodes().filter(|n| !before.node_arc(n.id).zip(after.node_arc(n.id)).is_some_and(|(a, b)| Arc::ptr_eq(a, b))).map(|n| n.id).collect();
    ids.extend(before.nodes().filter(|n| after.node(n.id).is_none()).map(|n| n.id));
    ids.into_iter().collect()
}

/// The locked layer that keeps `id` from changing in `doc`: itself or an ancestor.
fn lock_of(doc: &Document, id: NodeId) -> Option<NodeId> {
    std::iter::once(id).chain(doc.ancestors(id)).find(|a| doc.node(*a).is_some_and(|n| n.locked))
}

fn name(doc: &Document, id: NodeId) -> String {
    doc.node(id).map_or_else(|| format!("layer {}", id.0), |n| format!("\"{}\"", n.name))
}

impl Session {
    /// Run the next commands as `author` (the route's caller), returning who it was before.
    pub fn set_author(&mut self, author: Author) -> Author {
        std::mem::replace(&mut self.agents.author, author)
    }

    /// The revision an agent last read, for the stale-write guard of its next commands (`None`:
    /// no guard).
    pub fn set_since(&mut self, since: Option<u64>) -> Option<u64> {
        std::mem::replace(&mut self.agents.since, since)
    }

    /// Run one command as `author`, guarded by `since` when given.
    pub fn execute_as(&mut self, author: Author, since: Option<u64>, id: &str, params: &Value) -> Result<Value> {
        let (a, s) = (self.set_author(author), self.set_since(since));
        let r = self.execute(id, params);
        self.set_author(a);
        self.set_since(s);
        r
    }

    /// Refuse an agent's edit of a locked layer, or of a layer someone else changed after the
    /// revision the agent last read (the edit is then rolled back).
    pub(crate) fn check_agent(&self, cmd: &str, before: &Option<(Arc<Document>, Vec<NodeId>, NodeId, u64)>) -> Result<()> {
        let Some((bdoc, _, _, uid)) = before else { return Ok(()) };
        if !self.agents.author.is_agent() {
            return Ok(());
        }
        let Some(ds) = self.docs.iter().find(|d| d.uid == *uid) else { return Ok(()) };
        if Arc::ptr_eq(&ds.doc, bdoc) {
            return Ok(());
        }
        let ids = changed_ids(bdoc, &ds.doc);
        for &id in &ids {
            // A layer is held by a lock before the edit, or by where the edit put it.
            let held = if bdoc.node(id).is_some() { lock_of(bdoc, id) } else { None }.or_else(|| ds.doc.node(id).and_then(|_| lock_of(&ds.doc, id)));
            if let Some(lock) = held {
                let why = if lock == id {
                    format!("{} is locked: an agent can't change a locked layer (the person can unlock it)", name(bdoc, id))
                } else {
                    format!(
                        "{} is inside {}, which is locked: an agent can't change a locked layer or anything in it",
                        name(bdoc, id),
                        name(bdoc, lock)
                    )
                };
                return Err(EngineError::Disabled(cmd.to_string(), why));
            }
        }
        if let Some(since) = self.agents.since {
            let touched: BTreeSet<NodeId> = ids.iter().copied().collect();
            let mine = &self.agents.author;
            let theirs: Vec<&Change> = self
                .agents
                .logs
                .get(uid)
                .into_iter()
                .flatten()
                .filter(|c| c.revision > since && &c.author != mine && c.ids.iter().any(|i| touched.contains(i)))
                .collect();
            if !theirs.is_empty() {
                let what: Vec<String> = theirs
                    .iter()
                    .map(|c| {
                        let layers: Vec<String> = c.ids.iter().filter(|i| touched.contains(i)).map(|i| name(bdoc, *i)).collect();
                        format!("{} by the {} ({}, revision {})", layers.join(", "), c.author.describe(), c.label, c.revision)
                    })
                    .collect();
                return Err(EngineError::Disabled(
                    cmd.to_string(),
                    format!(
                        "changed since revision {since}: {}. Read it again (get_node, or get_changes since {since}) and retry with the new revision",
                        what.join("; ")
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Log a successful edit of document `uid` (it was `before`) by the current author.
    pub(crate) fn note_change(&mut self, spec: &CommandSpec, before: Option<(Arc<Document>, u64)>) {
        let Some((bdoc, uid)) = before else { return };
        let Some(ds) = self.docs.iter().find(|d| d.uid == uid) else { return };
        if Arc::ptr_eq(&ds.doc, &bdoc) {
            return;
        }
        let change = Change {
            revision: ds.revision,
            author: self.agents.author.clone(),
            command: spec.id.to_string(),
            label: spec.label.to_string(),
            ids: changed_ids(&bdoc, &ds.doc),
        };
        let log = self.agents.logs.entry(uid).or_default();
        log.push_back(change);
        if log.len() > LOG_LIMIT {
            log.pop_front();
        }
    }

    /// The active document's revision and its edits after revision `since`, oldest first.
    pub fn changes_since(&self, since: u64) -> Result<Value> {
        let ds = self.ds()?;
        let log = self.agents.logs.get(&ds.uid);
        let oldest = log.and_then(|l| l.front()).map(|c| c.revision);
        let changes: Vec<Value> = log
            .into_iter()
            .flatten()
            .filter(|c| c.revision > since)
            .map(|c| {
                let layers: Vec<Value> = c.ids.iter().map(|i| json!({"id": i.0, "name": ds.doc.node(*i).map(|n| n.name.clone())})).collect();
                json!({"revision": c.revision, "author": c.author.describe(), "agent": c.author.is_agent(), "command": c.command, "label": c.label, "layers": layers})
            })
            .collect();
        // The log keeps the last edits only: say so when `since` is older than it reaches.
        let complete = oldest.is_none_or(|o| since + 1 >= o);
        Ok(json!({"revision": ds.revision, "since": since, "complete": complete, "changes": changes}))
    }
}

fn changes_cmd(s: &mut Session, p: &Value) -> Result<Value> {
    let since = p.get("since").and_then(Value::as_u64).unwrap_or(0);
    s.changes_since(since)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "history.changes",
        "Changes since a revision",
        "{since? (revision; default 0)} → {revision (now), since, complete (false: older edits left the log), changes: [{revision, author (\"person\" | \"agent (route)\"), agent, command, label, layers: [{id, name}]}]}, oldest first",
        false,
        changes_cmd
    )]
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod tests;
