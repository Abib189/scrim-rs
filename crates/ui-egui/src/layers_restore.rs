//! Collapse all, then restore (Stage H, `docs/design/layers-collapse-and-focus.md`).
//! - **Collapse** (`layers.collapse-all`, ⌥L, the header button) keeps which rows are open (a
//!   snapshot), then closes every row on every page as D1 does. Called while a snapshot still holds,
//!   it changes nothing and keeps it, so an agent's `layers.collapse-all` stays idempotent and never
//!   restores.
//! - **Restore** (`layers.restore-expansion`, the button's next press) opens exactly the rows the
//!   snapshot kept, and nothing else.
//! - **The snapshot is dropped, for good,** by any row opened or closed afterwards (by hand, a new
//!   selection revealed, a drag held over a row), another page, another file, or a structural change
//!   to the layer tree: a layer added, deleted or moved to another parent, or an undo or redo of
//!   one. Other edits (a fill, a name, a move on the canvas, sibling order) keep it. The next press
//!   then collapses again and takes a new one.
//! - **While a filter is active** the button collapses as D1 does: no snapshot is taken or applied.
//! - Panel state only: nothing is written to the document or saved in the file.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use figcraft_doc::{Document, NodeId};

use crate::FigApp;

/// The rows open before Collapse all, and what they were open in.
#[derive(Clone, Debug)]
pub struct Snapshot {
    open: BTreeSet<NodeId>,
    /// The document's tab and the page shown.
    uid: u64,
    page: NodeId,
    /// The document as last checked (held, so a new one is never mistaken for it), and the shape of
    /// its layer tree.
    doc: Arc<Document>,
    shape: u64,
}

/// The layer tree's shape: the pages, and every layer with its parent (sibling order left out).
fn shape(doc: &Document) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    doc.pages.hash(&mut h);
    for n in doc.nodes() {
        (n.id, n.parent).hash(&mut h);
    }
    h.finish()
}

/// A filter is active: the Layers panel lists matches, not the tree.
fn filtering(app: &FigApp) -> bool {
    !app.ui.layer_filter.trim().is_empty()
}

/// Drop the snapshot once it no longer holds (each frame the panel draws, and after every command).
pub(crate) fn check(app: &mut FigApp) {
    let Some(s) = app.ui.collapse_snapshot.as_mut() else { return };
    let holds = app.session.ds().is_ok_and(|ds| {
        let same_tree = Arc::ptr_eq(&s.doc, &ds.doc) || shape(&ds.doc) == s.shape;
        if same_tree {
            s.doc = ds.doc.clone();
        }
        s.uid == ds.uid && s.page == ds.page && same_tree
    });
    // Collapse all leaves no row open: any open row since was opened by hand or by a reveal.
    if !holds || !app.ui.expanded.is_empty() {
        app.ui.collapse_snapshot = None;
    }
}

/// The button's next press restores (a snapshot holds and no filter is active); otherwise it
/// collapses.
pub(crate) fn restores_next(app: &mut FigApp) -> bool {
    check(app);
    app.ui.collapse_snapshot.is_some() && !filtering(app)
}

/// `layers.collapse-all`: keep which rows are open (unless a snapshot still holds, or a filter is
/// active), then close every row (D1).
pub(crate) fn collapse(app: &mut FigApp) {
    check(app);
    if app.ui.collapse_snapshot.is_none()
        && !filtering(app)
        && let Ok(ds) = app.session.ds()
    {
        let snap = Snapshot { open: app.ui.expanded.clone(), uid: ds.uid, page: ds.page, doc: ds.doc.clone(), shape: shape(&ds.doc) };
        app.ui.collapse_snapshot = Some(snap);
    }
    crate::layers_header::collapse_all(app);
}

/// `layers.restore-expansion`: open exactly the rows open before Collapse all, when its snapshot
/// still holds and no filter is active; otherwise nothing. The selection isn't revealed again.
pub(crate) fn restore(app: &mut FigApp) {
    if !restores_next(app) {
        return;
    }
    if let Some(s) = app.ui.collapse_snapshot.take() {
        app.ui.expanded = s.open;
        app.ui.layers_seen_sel = app.session.selection();
    }
}

/// For `ui.inspect`: the open rows, and whether the button's next press restores.
pub(crate) fn inspect(app: &FigApp) -> serde_json::Value {
    let restore_next = app.ui.collapse_snapshot.is_some() && !filtering(app);
    serde_json::json!({"open": app.ui.expanded.iter().map(|i| i.0).collect::<Vec<_>>(), "restoreNext": restore_next})
}

#[cfg(test)]
#[path = "layers_restore_tests.rs"]
mod tests;
