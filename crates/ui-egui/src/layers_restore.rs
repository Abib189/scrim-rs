//! Collapse all, then restore (Stage H, `docs/design/layers-collapse-and-focus.md`).
//! - **Collapse** (`layers.collapse-all`, ⌥L, the header button) keeps which rows are open and
//!   where the list was scrolled (a snapshot), then closes every row on every page as D1 does.
//!   Called while a snapshot still holds, it changes nothing and keeps it, so an agent's
//!   `layers.collapse-all` stays idempotent and never restores.
//! - **Restore** (`layers.restore-expansion`, the button's next press) opens exactly the rows the
//!   snapshot kept, and nothing else, and scrolls the list back to where it was.
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

use egui::Ui;
use egui::scroll_area::State;
use figcraft_doc::{Document, NodeId};

use crate::FigApp;

/// The Layers list's scroll area (`layers.rs`).
pub(crate) const LIST: &str = "layers_list";

/// Collapse all's state in the window (`app.ui.layers_restore`).
#[derive(Clone, Debug, Default)]
pub struct Restore {
    snapshot: Option<Snapshot>,
    /// The list's scroll offset as last seen, and one to put back before the list draws next.
    scroll: f32,
    scroll_back: Option<f32>,
}

/// The rows open before Collapse all, and what they were open in.
#[derive(Clone, Debug)]
struct Snapshot {
    open: BTreeSet<NodeId>,
    scroll: f32,
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
    let Some(s) = app.ui.layers_restore.snapshot.as_mut() else { return };
    let holds = app.session.ds().is_ok_and(|ds| {
        let same_tree = Arc::ptr_eq(&s.doc, &ds.doc) || shape(&ds.doc) == s.shape;
        if same_tree {
            s.doc = ds.doc.clone();
        }
        s.uid == ds.uid && s.page == ds.page && same_tree
    });
    // Collapse all leaves no row open: any open row since was opened by hand or by a reveal.
    if !holds || !app.ui.expanded.is_empty() {
        app.ui.layers_restore.snapshot = None;
    }
}

/// The button's next press restores (a snapshot holds and no filter is active); otherwise it
/// collapses.
pub(crate) fn restores_next(app: &mut FigApp) -> bool {
    check(app);
    app.ui.layers_restore.snapshot.is_some() && !filtering(app)
}

/// `layers.collapse-all`: keep which rows are open and the list's scroll (unless a snapshot still
/// holds, or a filter is active), then close every row (D1).
pub(crate) fn collapse(app: &mut FigApp) {
    check(app);
    if app.ui.layers_restore.snapshot.is_none()
        && !filtering(app)
        && let Ok(ds) = app.session.ds()
    {
        let (uid, page, doc, shape) = (ds.uid, ds.page, ds.doc.clone(), shape(&ds.doc));
        let (open, scroll) = (app.ui.expanded.clone(), app.ui.layers_restore.scroll);
        app.ui.layers_restore.snapshot = Some(Snapshot { open, scroll, uid, page, doc, shape });
    }
    crate::layers_header::collapse_all(app);
}

/// `layers.restore-expansion`: open exactly the rows open before Collapse all, and scroll the list
/// back, when its snapshot still holds and no filter is active; otherwise nothing. The selection
/// isn't revealed again.
pub(crate) fn restore(app: &mut FigApp) {
    if !restores_next(app) {
        return;
    }
    if let Some(s) = app.ui.layers_restore.snapshot.take() {
        app.ui.expanded = s.open;
        app.ui.layers_seen_sel = app.session.selection();
        app.ui.layers_restore.scroll_back = Some(s.scroll);
    }
}

/// From the Layers header, before the list draws (`ui` is the list's parent): note where the list is
/// scrolled (`after: false`), or put back the scroll a restore asked for (`after: true`, once the
/// header's own button has been handled).
pub(crate) fn scroll(app: &mut FigApp, ui: &Ui, after: bool) {
    let id = ui.make_persistent_id(egui::IdSalt::new(LIST));
    let r = &mut app.ui.layers_restore;
    match (after, r.scroll_back.take()) {
        (false, back) => {
            r.scroll_back = back;
            r.scroll = State::load(ui.ctx(), id).map_or(0.0, |s| s.offset.y);
        }
        (true, Some(y)) => {
            let mut s = State::load(ui.ctx(), id).unwrap_or_default();
            s.offset.y = y;
            s.store(ui.ctx(), id);
            r.scroll = y;
        }
        (true, None) => {}
    }
}

/// For `ui.inspect`: the open rows, whether the button's next press restores, and the list's
/// scroll offset.
pub(crate) fn inspect(app: &FigApp) -> serde_json::Value {
    let restore_next = app.ui.layers_restore.snapshot.is_some() && !filtering(app);
    let open: Vec<u64> = app.ui.expanded.iter().map(|i| i.0).collect();
    serde_json::json!({"open": open, "restoreNext": restore_next, "scroll": app.ui.layers_restore.scroll})
}

#[cfg(test)]
#[path = "layers_restore_tests.rs"]
mod tests;
