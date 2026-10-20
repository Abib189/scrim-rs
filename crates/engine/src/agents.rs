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

/// The app's MCP endpoint (`docs/design/settings-and-connect.md`, C1), as the app reports it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct McpEndpoint {
    /// The port it listens on; `None`: stopped (or headless, where there is none).
    pub port: Option<u16>,
    /// Why it isn't running, when it failed to start.
    pub error: Option<String>,
    /// MCP clients that initialized over it: (name, version, unix seconds), newest last.
    pub clients: Vec<(String, String, u64)>,
    /// The folders its requests' file access is confined to (Settings ▸ MCP ▸ root folder).
    pub roots: Option<std::sync::Arc<crate::file_access::Roots>>,
}

/// How long after a client's last `initialize` the AI tab still calls it connected.
pub const CONNECTED_FOR_SECS: u64 = 10 * 60;

impl McpEndpoint {
    /// The URL a client connects to.
    pub fn url(&self) -> Option<String> {
        self.port.map(|p| format!("http://127.0.0.1:{p}/mcp"))
    }
}

/// The arguments of `claude` that add the app's HTTP endpoint to Claude Code, for every project
/// (`--scope user`: run from the app, the default local scope would belong to the app's working
/// directory). The header follows the name and URL (`--header` takes several values).
pub fn claude_add_args(port: u16, token: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> = ["mcp", "add", "--transport", "http", "--scope", "user", "scrim"].iter().map(|s| s.to_string()).collect();
    v.push(format!("http://127.0.0.1:{port}/mcp"));
    if let Some(t) = token {
        v.push("--header".into());
        v.push(format!("Authorization: Bearer {t}"));
    }
    v
}

/// [`claude_add_args`] as one line to show or copy (`claude ...`, the header quoted).
pub fn claude_add_command(port: u16, token: Option<&str>) -> String {
    let args = claude_add_args(port, token);
    let quoted: Vec<String> = args.iter().map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() }).collect();
    format!("claude {}", quoted.join(" "))
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
    /// The app's control channel port when it listens (`--control`): agents attach through it
    /// (`ai.status`). `None`: headless, or the app runs without it.
    pub control_port: Option<u16>,
    /// The app's MCP endpoint, as the app reports it (`mcp.status`).
    pub mcp: McpEndpoint,
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
            // A layer is held by a lock it had before the edit, or by one the edit would give it:
            // the agent locking it, or putting it inside a locked layer.
            let before = if bdoc.node(id).is_some() { lock_of(bdoc, id) } else { None };
            let after = ds.doc.node(id).and_then(|_| lock_of(&ds.doc, id));
            let why = match (before, after) {
                (Some(lock), _) if lock == id => {
                    format!("{} is locked: an agent can't change a locked layer (the person can unlock it)", name(bdoc, id))
                }
                (Some(lock), _) => format!(
                    "{} is inside {}, which is locked: an agent can't change a locked layer or anything in it",
                    name(bdoc, id),
                    name(bdoc, lock)
                ),
                (None, Some(lock)) if lock == id => {
                    format!("an agent can't lock {}: locking layers is the person's (they can lock it themselves)", name(bdoc, id))
                }
                (None, Some(lock)) => {
                    format!("an agent can't put {} inside {}, which is locked (the person can unlock it)", name(&ds.doc, id), name(&ds.doc, lock))
                }
                (None, None) => continue,
            };
            return Err(EngineError::Disabled(cmd.to_string(), why));
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
        self.note_edit(uid, &bdoc, spec.id, spec.label);
    }

    /// Log a canvas gesture (a drag, a drawn shape, a nudge) that changed document `uid` (it was
    /// `before`) as one edit, `label` its undo step's name. Gestures commit without a command, so
    /// before this the feed showed only commands, never the person's own canvas edits.
    pub(crate) fn note_gesture(&mut self, uid: u64, before: &Arc<Document>, label: &str) {
        self.note_edit(uid, before, "gesture", label);
    }

    fn note_edit(&mut self, uid: u64, bdoc: &Arc<Document>, command: &str, label: &str) {
        let Some(ds) = self.docs.iter().find(|d| d.uid == uid) else { return };
        if Arc::ptr_eq(&ds.doc, bdoc) {
            return;
        }
        let change = Change {
            revision: ds.revision,
            author: self.agents.author.clone(),
            command: command.to_string(),
            label: label.to_string(),
            ids: changed_ids(bdoc, &ds.doc),
        };
        let log = self.agents.logs.entry(uid).or_default();
        log.push_back(change);
        if log.len() > LOG_LIMIT {
            log.pop_front();
        }
    }

    /// The active document's revision and its edits after revision `since`, oldest first.
    pub fn changes_since(&self, since: u64) -> Result<Value> {
        self.changes(since, None)
    }

    /// [`changes_since`](Self::changes_since), `last`: only the newest `last` of them.
    pub fn changes(&self, since: u64, last: Option<usize>) -> Result<Value> {
        let ds = self.ds()?;
        let log = self.agents.logs.get(&ds.uid);
        let oldest = log.and_then(|l| l.front()).map(|c| c.revision);
        let newer = log.into_iter().flatten().filter(|c| c.revision > since).count();
        let changes: Vec<Value> = log
            .into_iter()
            .flatten()
            .filter(|c| c.revision > since)
            // `last`: only the newest ones (the revision also moves for selection and view changes,
            // so a revision window is no measure of how many edits there were).
            .skip(last.map_or(0, |n| newer.saturating_sub(n)))
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
    let last = p.get("last").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok());
    s.changes(since, last)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![spec!(
        "history.changes",
        "Changes since a revision",
        "{since? (revision; default 0), last? (only the newest n)} → {revision (now), since, complete (false: older edits left the log), changes: [{revision, author (\"person\" | \"agent (route)\"), agent, command, label, layers: [{id, name}]}]}, oldest first",
        false,
        changes_cmd
    )]
}

/// The command that connects Claude Code to Scrim through MCP: headless (`None`), or attached to the
/// app's control channel on `port`. The app offers it to copy, and `docs/mcp.md` shows the same text
/// (a test checks they agree). `scrim` is the CLI (formerly `figcraft-cli`).
pub fn connect_command(port: Option<u16>) -> String {
    match port {
        None => "claude mcp add scrim -- scrim mcp".to_string(),
        Some(p) => format!("claude mcp add scrim-app -- scrim mcp --attach {p}"),
    }
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod tests;
