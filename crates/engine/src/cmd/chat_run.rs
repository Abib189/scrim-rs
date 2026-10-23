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
//! - **`chat.redo-run`** (Fix Z7) puts the run back, as one step, when nothing changed since Undo
//!   run.
//! - **Refusals** (Fix Z7) are "not available now" with fixed words ([`REFUSALS`]): the card shows
//!   them under its buttons, and Diagnostics notes "chat.undo-run: refused (already undone)".
//!   "Invalid parameters" is for malformed calls only.

use std::sync::Arc;

use figcraft_doc::{Document, NodeId};
use serde_json::{Value, json};

use super::CommandSpec;
use crate::settings::diagnostics::{self, Entry};
use crate::{EngineError, Result, Session, spec};

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
    /// Set by the latest Undo run (Fix Z7); where the history stands against it says whether the
    /// run is undone now (Fix AA2).
    pub undone: Option<Undone>,
}

/// The latest Undo run of the run: the document as the run left it, and the length of the undo
/// history with the "Undo run" step on it (Fix AA2).
#[derive(Clone, Debug)]
pub struct Undone {
    pub end_doc: Arc<Document>,
    pub at: usize,
}

/// The run card's state (Fix AA2), from the history's position: the run undone, and whether Undo
/// run and Redo run are one step now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunState {
    pub undone: bool,
    pub undo_ok: bool,
    pub redo_ok: bool,
}

/// The history steps' labels (the registry's).
const UNDO_STEP: &str = "Undo run";
const REDO_STEP: &str = "Redo run";

/// Where the history stands against the latest Undo run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pos {
    /// Right after it (or back there by ⌘⇧Z, or by ⌘Z of what came after).
    Undone,
    /// Right before it: ⌘Z took it back, or Redo run is the step after it.
    Back,
    /// Anywhere else: something else changed (`undone`: the Undo run is still in the history).
    Other { undone: bool },
}

/// Compared on lengths and labels (no document is diffed): the step at `at - 1` is the Undo run,
/// and nothing but Redo run (or nothing) lies past it. ⌘⇧Z rebuilds a step around the document of
/// the moment, so a step is known by its place and label, not its document.
fn position(s: &Session, r: &RunRecord, u: &Undone) -> Pos {
    let Ok(ds) = s.ds() else { return Pos::Other { undone: false } };
    if ds.uid != r.uid {
        return Pos::Other { undone: false };
    }
    let (undo, redo) = (&ds.history.undo, &ds.history.redo);
    let step = |e: Option<&crate::HistoryEntry>, label: &str| e.is_some_and(|e| e.label == label);
    let ours = u.at.checked_sub(1).and_then(|i| undo.get(i)).is_some_and(|e| e.label == UNDO_STEP);
    if undo.len() == u.at && ours {
        Pos::Undone
    } else if (undo.len() + 1 == u.at && step(redo.last(), UNDO_STEP)) || (undo.len() == u.at + 1 && ours && step(undo.last(), REDO_STEP)) {
        Pos::Back
    } else {
        Pos::Other { undone: ours && !step(undo.get(u.at), REDO_STEP) }
    }
}

/// Why Undo run can't go now, if it can't: the command's checks, which the card shows too.
fn undo_refusal(s: &Session, r: &RunRecord) -> Option<&'static str> {
    if s.agents.chat_running {
        return Some(super::chat::STOP_FIRST);
    }
    let uid = s.ds().map_or(0, |d| d.uid);
    if uid != r.uid {
        return Some(OTHER_DOC);
    }
    if s.live_doc(uid) {
        return Some(LIVE);
    }
    match r.undone.as_ref().map(|u| position(s, r, u)) {
        Some(Pos::Undone) => Some(ALREADY),
        Some(Pos::Back) => None,
        Some(Pos::Other { .. }) => Some(CHANGED),
        // Anything else since the run began (the person's edit, or an agent's after the run
        // ended) would be undone with it.
        None => s.agents.edits_after(uid, r.start_rev).any(|c| !c.author.is_agent() || r.end_rev.is_some_and(|e| c.revision > e)).then_some(CHANGED),
    }
}

const NO_RUN: &str = "There is no run to act on yet: send a change first";
const OTHER_DOC: &str = "Open the document the run worked on";
const LIVE: &str = "A file shared live can't go back as a whole: undo its steps one by one";
const CHANGED: &str = "The document changed after the run began: undo those changes first, or the run's own steps one by one";
const ALREADY: &str = "The run is already undone";
const NONE_LEFT: &str = "None of the layers the run touched are left";
/// The card's tooltips for its disabled buttons, and the refusals for the same.
pub const REMOVED_BY_UNDO: &str = "Those layers were removed by Undo";
pub const REDO_CHANGED: &str = "Something changed since: Redo is no longer one step";
const NOT_UNDONE: &str = "The run isn't undone: there is nothing to redo";

/// Each refusal's words, and the short reason Diagnostics notes (Fix Z7). Fixed text only: no
/// prompt, path or name.
pub const REFUSALS: &[(&str, &str)] = &[
    (NO_RUN, "no run yet"),
    (OTHER_DOC, "another document"),
    (LIVE, "shared live"),
    (CHANGED, "changed since the run"),
    (ALREADY, "already undone"),
    (NONE_LEFT, "no layers left"),
    (REMOVED_BY_UNDO, "removed by Undo"),
    (REDO_CHANGED, "changed since the undo"),
    (NOT_UNDONE, "not undone"),
    (super::chat::STOP_FIRST, "the run is going"),
];

fn refuse(id: &str, why: &str) -> EngineError {
    EngineError::Disabled(id.into(), why.into())
}

/// A run command's refusal, as Diagnostics notes it ("already undone"); `None` for anything else.
pub fn refusal(e: &EngineError) -> Option<&'static str> {
    let EngineError::Disabled(cmd, why) = e else { return None };
    let ours = ["chat.undo-run", "chat.redo-run", "chat.select-run-layers"].contains(&cmd.as_str());
    REFUSALS.iter().find(|(w, _)| ours && w == why).map(|(_, short)| *short)
}

/// The run card's state now (Fix AA2): after Undo run, undone with Redo run one step; after ⌘Z of
/// it (or Redo run), done with Undo run one step; after ⌘⇧Z, undone again; once anything else
/// changed, neither button.
pub fn state(s: &Session) -> RunState {
    let Some(r) = &s.agents.chat_run else { return RunState::default() };
    let pos = r.undone.as_ref().map(|u| position(s, r, u));
    RunState {
        undone: matches!(pos, Some(Pos::Undone | Pos::Other { undone: true })),
        undo_ok: undo_refusal(s, r).is_none(),
        redo_ok: !s.agents.chat_running && pos == Some(Pos::Undone),
    }
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
    s.agents.chat_run.clone().ok_or_else(|| refuse(id, NO_RUN))
}

fn select_layers(s: &mut Session, _: &Value) -> Result<Value> {
    let id = "chat.select-run-layers";
    super::chat::person_only(s, id)?;
    let r = record(s, id)?;
    if s.ds()?.uid != r.uid {
        return Err(refuse(id, OTHER_DOC));
    }
    if state(s).undone {
        return Err(refuse(id, REMOVED_BY_UNDO));
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
        return Err(refuse(id, NONE_LEFT));
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
    // Undone already, something else changed, or (never undone) an edit since the run began: the
    // history's position says which (Fix AA2).
    if let Some(why) = undo_refusal(s, &r) {
        return Err(refuse(id, why));
    }
    let ds = s.ds_mut()?;
    if Arc::ptr_eq(&ds.doc, &r.start_doc) || *ds.doc == *r.start_doc {
        return Err(refuse(id, ALREADY));
    }
    let edits = run_edits_count(s, &r);
    let ds = s.ds_mut()?;
    // The "Undo run" step lands on the history after this returns (whole documents: the one before
    // is `end_doc`), so the history is then this long.
    let at = (ds.history.undo.len() + 1).min(ds.history.limit.max(1));
    let end_doc = std::mem::replace(&mut ds.doc, r.start_doc.clone());
    let doc = ds.doc.clone();
    ds.selection.retain(|n| doc.contains(*n));
    if let Some(rec) = s.agents.chat_run.as_mut() {
        rec.undone = Some(Undone { end_doc, at });
    }
    let agent = r.agent.clone();
    diagnostics::record(&mut s.prefs.settings, diagnostics::now(), Entry::ChatUndone { agent: &agent, edits });
    Ok(json!({"undone": true, "edits": edits}))
}

/// Redo run: the run back as the undo found it, as one step, while nothing else changed since.
fn redo(s: &mut Session, _: &Value) -> Result<Value> {
    let id = "chat.redo-run";
    super::chat::person_only(s, id)?;
    super::chat::not_running(s, id)?;
    let r = record(s, id)?;
    if s.ds()?.uid != r.uid {
        return Err(refuse(id, OTHER_DOC));
    }
    let Some(u) = r.undone.clone() else { return Err(refuse(id, NOT_UNDONE)) };
    match position(s, &r, &u) {
        Pos::Undone => {}
        Pos::Back => return Err(refuse(id, NOT_UNDONE)),
        Pos::Other { .. } => return Err(refuse(id, REDO_CHANGED)),
    }
    // A step of its own ("Redo run"); the record stays, so the history's position keeps telling
    // the card where it stands.
    s.ds_mut()?.doc = u.end_doc;
    Ok(json!({"redone": true}))
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
        spec!(
            "chat.redo-run",
            "Redo run",
            "{} → {redone}  (puts the AI panel's last run back after Undo run, as one undo step; refused when anything changed since; the person's alone)",
            true,
            redo
        ),
    ]
}

#[cfg(test)]
#[path = "chat_run_tests.rs"]
mod tests;
