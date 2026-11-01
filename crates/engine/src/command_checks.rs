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
        match ds.doc.nodes().find(|n| !untouched(n) && broken(n)) {
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
