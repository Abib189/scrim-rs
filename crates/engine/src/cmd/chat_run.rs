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

/// The run's rows, oldest first, as (what happened, how many times in a row) (Fix Z1): each
/// command or batch of the run says what it did to the layers it touched, by name ("Created
/// Poster (30 layers)", "Edited 4 layers", "Deleted Bar 3"); a batch the agent labelled with a
/// sentence says that instead. Rows that say the same, one after another, are merged.
pub fn rows(s: &Session) -> Vec<(String, usize)> {
    let Some(r) = &s.agents.chat_run else { return Vec::new() };
    let mut out: Vec<(String, usize)> = Vec::new();
    // A batch's steps are covered by the batch, logged after them as a whole.
    for c in run_units(s, r) {
        for row in unit_rows(c) {
            match out.last_mut() {
                Some((text, n)) if *text == row => *n += 1,
                _ => out.push((row, 1)),
            }
        }
    }
    out
}

/// A label the agent wrote itself: a sentence (it has a space), not one of the registry's labels.
fn agent_sentence(label: &str) -> Option<String> {
    let l = label.trim();
    (l.contains(' ') && !super::all_commands().iter().any(|c| c.label == l)).then(|| l.to_string())
}

/// What one command or batch did, as rows: created, edited, deleted.
fn unit_rows(c: &crate::agents::Change) -> Vec<String> {
    use crate::agents::touched::Touch;
    if let Some(l) = c.batch_label.as_deref().and_then(agent_sentence) {
        return vec![l];
    }
    let mut rows = Vec::new();
    let t = &c.touched;
    // Created: each container or layer right on a page by its name, with the layers made inside
    // it; the others counted together.
    let created: Vec<_> = t.iter().filter(|x| x.touch == Touch::Created).collect();
    let made = |id| created.iter().any(|x| x.id == id);
    let inside = |top: figcraft_doc::NodeId| {
        created
            .iter()
            .filter(|x| {
                let mut p = x.parent;
                while let Some(id) = p {
                    if id == top {
                        return true;
                    }
                    p = created.iter().find(|y| y.id == id).and_then(|y| y.parent);
                }
                false
            })
            .count()
    };
    let mut others = (0usize, String::new());
    for x in created.iter().filter(|x| !x.parent.is_some_and(made)) {
        let n = inside(x.id);
        if x.container || x.top {
            rows.push(if n > 0 { format!("Created {} ({n} layers)", x.name) } else { format!("Created {}", x.name) });
        } else {
            others = (others.0 + 1 + n, x.name.clone());
        }
    }
    match others.0 {
        0 => {}
        1 => rows.push(format!("Created {}", others.1)),
        n => rows.push(format!("Created {n} layers")),
    }
    // Edited or moved: one by name, more counted.
    let edited: Vec<_> = t.iter().filter(|x| matches!(x.touch, Touch::Edited | Touch::Moved { .. })).collect();
    match edited.as_slice() {
        [] => {}
        [x] => rows.push(match &x.touch {
            Touch::Moved { into } if !into.is_empty() => format!("Moved {} into {into}", x.name),
            _ => format!("Edited {}", x.name),
        }),
        many => rows.push(format!("Edited {} layers", many.len())),
    }
    // Deleted: the layers removed themselves (not their contents), one or two by name.
    let deleted: Vec<_> = t.iter().filter(|x| x.touch == Touch::Deleted).collect();
    let gone = |id| deleted.iter().any(|x| x.id == id);
    let roots: Vec<_> = deleted.iter().filter(|x| !x.parent.is_some_and(gone)).collect();
    match roots.as_slice() {
        [] => {}
        [a] => rows.push(format!("Deleted {}", a.name)),
        [a, b] => rows.push(format!("Deleted {} and {}", a.name, b.name)),
        many => rows.push(format!("Deleted {} layers", many.len())),
    }
    rows
}

fn run_edits<'a>(s: &'a Session, r: &'a RunRecord) -> impl Iterator<Item = &'a crate::agents::Change> {
    s.agents
        .edits_after(r.uid, r.start_rev)
        .filter(move |c| c.author.is_agent() && c.command != "command.batch" && r.end_rev.is_none_or(|e| c.revision <= e))
}

/// The run's commands and batches, each once: a batch's steps are left out, the batch kept.
fn run_units<'a>(s: &'a Session, r: &'a RunRecord) -> impl Iterator<Item = &'a crate::agents::Change> {
    s.agents.edits_after(r.uid, r.start_rev).filter(move |c| c.author.is_agent() && !c.in_batch && r.end_rev.is_none_or(|e| c.revision <= e))
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
