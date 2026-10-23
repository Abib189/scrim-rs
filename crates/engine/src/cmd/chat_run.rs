//! Fix X4: the composer run's card in the AI panel (`docs/design/ai-panel.md`): what the run did,
//! from the change log, and two commands for it, the person's alone.
//! - **The record** ([`RunRecord`]): the document the run began in, as it was, the change log's
//!   revision then, and its last revision once it ends. The window keeps it (`chat_ui`), from the
//!   run's start until the next run; nothing an agent sends sets it.
//! - **The rows** ([`rows`]): the run's agent edits, by the label each command carried (a batch's
//!   own wrapper left out: its steps are logged one by one), neighbours with the same label merged
//!   with a count. No prompt, path or key: the labels are the registry's.
//! - **`chat.select-run-layers`** selects the layers the run touched that still exist.
//! - **`chat.undo-run`** puts the document back as it was before the run, as one undo step: the
//!   history keeps whole documents, so restoring the run's start is one step ("Undo is one step").
//!   Refused while the run goes ("Stop the run first"), and when the person edited the document
//!   since the run began (that would be undone too).

use std::sync::Arc;

use figcraft_doc::{Document, NodeId};
use serde_json::{Value, json};

use super::CommandSpec;
use crate::settings::diagnostics::{self, Entry};
use crate::{Result, Session, bad, spec};

/// A composer run, as the window recorded it.
#[derive(Clone, Debug)]
pub struct RunRecord {
    /// The document (`uid`) the run worked on, as it was when the run began.
    pub uid: u64,
    pub start_doc: Arc<Document>,
    /// The change log's last revision when the run began, and when it ended (`None`: running).
    pub start_rev: u64,
    pub end_rev: Option<u64>,
    /// The agent's name (a headless profile's).
    pub agent: String,
}

/// The run's agent edits, oldest first, as (label, how many in a row).
pub fn rows(s: &Session) -> Vec<(String, usize)> {
    let Some(r) = &s.agents.chat_run else { return Vec::new() };
    let mut out: Vec<(String, usize)> = Vec::new();
    for c in run_edits(s, r) {
        match out.last_mut() {
            Some((label, n)) if *label == c.label => *n += 1,
            _ => out.push((c.label.clone(), 1)),
        }
    }
    out
}

fn run_edits<'a>(s: &'a Session, r: &'a RunRecord) -> impl Iterator<Item = &'a crate::agents::Change> {
    s.agents
        .edits_after(r.uid, r.start_rev)
        .filter(move |c| c.author.is_agent() && c.command != "command.batch" && r.end_rev.is_none_or(|e| c.revision <= e))
}

fn record(s: &Session, id: &str) -> Result<RunRecord> {
    s.agents.chat_run.clone().ok_or_else(|| bad(id, "there is no run to act on yet: send a change first"))
}

fn select_layers(s: &mut Session, _: &Value) -> Result<Value> {
    let id = "chat.select-run-layers";
    super::chat::person_only(s, id)?;
    let r = record(s, id)?;
    if s.ds()?.uid != r.uid {
        return Err(bad(id, "open the document the run worked on"));
    }
    let doc = s.doc()?;
    let mut ids: Vec<NodeId> = Vec::new();
    for c in run_edits(s, &r) {
        for n in &c.ids {
            let on_page = doc.page_of(*n).is_some_and(|pg| doc.pages.contains(&pg) && pg != *n);
            if !ids.contains(n) && doc.contains(*n) && on_page {
                ids.push(*n);
            }
        }
    }
    if ids.is_empty() {
        return Err(bad(id, "none of the layers the run touched are left"));
    }
    let n = ids.len();
    super::edit::select_and_page(s, ids)?;
    Ok(json!({"selected": n}))
}

fn undo(s: &mut Session, _: &Value) -> Result<Value> {
    let id = "chat.undo-run";
    super::chat::person_only(s, id)?;
    super::chat::not_running(s, id)?;
    let r = record(s, id)?;
    let uid = s.ds()?.uid;
    if uid != r.uid {
        return Err(bad(id, "open the document the run worked on"));
    }
    if s.live_doc(uid) {
        return Err(bad(id, "a file shared live can't go back as a whole: undo its steps one by one"));
    }
    // Anything else since the run began (the person's edit, or an agent's after the run ended)
    // would be undone with it.
    if s.agents.edits_after(uid, r.start_rev).any(|c| !c.author.is_agent() || r.end_rev.is_some_and(|e| c.revision > e)) {
        return Err(bad(id, "the document changed after the run began: undo those changes first, or the run's own steps one by one"));
    }
    let ds = s.ds_mut()?;
    if Arc::ptr_eq(&ds.doc, &r.start_doc) || *ds.doc == *r.start_doc {
        return Err(bad(id, "the run is already undone"));
    }
    let edits = run_edits_count(s, &r);
    let ds = s.ds_mut()?;
    ds.doc = r.start_doc.clone();
    let doc = ds.doc.clone();
    ds.selection.retain(|n| doc.contains(*n));
    let agent = r.agent.clone();
    diagnostics::record(&mut s.prefs.settings, diagnostics::now(), Entry::ChatUndone { agent: &agent, edits });
    Ok(json!({"undone": true, "edits": edits}))
}

fn run_edits_count(s: &Session, r: &RunRecord) -> usize {
    run_edits(s, r).count()
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "chat.select-run-layers",
            "Select the run's layers",
            "{} → {selected}  (selects the layers the AI panel's last run touched that are still there; the person's alone)",
            false,
            select_layers
        ),
        spec!(
            "chat.undo-run",
            "Undo run",
            "{} → {undone, edits}  (puts the document back as it was before the AI panel's last run, as one undo step; refused while it runs or after edits since; the person's alone)",
            true,
            undo
        ),
    ]
}

#[cfg(test)]
#[path = "chat_run_tests.rs"]
mod tests;
