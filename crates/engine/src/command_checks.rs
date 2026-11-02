//! Checks every command passes after it runs, before its edit is kept (moved out of `lib.rs`):
//! numbers stay finite, and a document you may only view stays as it is.

use std::sync::Arc;

use figcraft_doc::{Document, Node, NodeId};

use crate::{EngineError, Result, Session};

impl Session {
    /// Every number on a layer stays finite (Fix AI LW-2): JSON has no infinity or NaN, so a file
    /// holding one can't be reopened as it was. Refuse an edit that would leave one on a layer it
    /// changed (it is rolled back).
    pub(crate) fn check_finite_geometry(&self, id: &str, before: &Option<(Arc<Document>, Vec<NodeId>, NodeId, u64)>) -> Result<()> {
        let Some((bdoc, _, _, uid)) = before else { return Ok(()) };
        let Some(ds) = self.docs.iter().find(|d| d.uid == *uid) else { return Ok(()) };
        if Arc::ptr_eq(&ds.doc, bdoc) {
            return Ok(());
        }
        let broken = |n: &Node| !figcraft_doc::all_finite(n);
        let untouched = |n: &Node| bdoc.node_arc(n.id).zip(ds.doc.node_arc(n.id)).is_some_and(|(a, b)| Arc::ptr_eq(a, b));
        let suspect = |n: &&Node| {
            #[cfg(test)]
            EXAMINED.with(|c| c.set(c.get() + 1));
            !untouched(n) && broken(n)
        };
        // Only the layers the command added or changed (Fix AI C-6, upstream 646ac60b): the diff
        // skips what the two versions share, so this costs about the size of the edit, not of the
        // document.
        let changed = ds.doc.changed_ids(bdoc, usize::MAX).unwrap_or_default();
        match changed.iter().filter_map(|i| ds.doc.node(*i)).find(suspect) {
            Some(n) => Err(EngineError::NotFinite { cmd: id.to_string(), layer: n.name.clone() }),
            None => Ok(()),
        }
    }

    /// A live session's document you may only view (or that is on its way to a server) is
    /// read-only: refuse an edit (it is rolled back).
    pub(crate) fn check_live_access(&self, id: &str, before: &Option<(Arc<Document>, Vec<NodeId>, NodeId, u64)>) -> Result<()> {
        let Some((bdoc, _, _, uid)) = before else { return Ok(()) };
        let Some(ds) = self.docs.iter().find(|d| d.uid == *uid) else { return Ok(()) };
        match self.live_read_only(*uid) {
            Some(why) if !Arc::ptr_eq(&ds.doc, bdoc) => Err(EngineError::Disabled(id.to_string(), why.into())),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Layers the finite-geometry check looked at (Fix AI C-6's counting test).
    static EXAMINED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "command_checks_tests.rs"]
mod tests;
