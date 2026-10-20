//! D1: `layers.collapse-all` is panel state only: it asks the panel, and leaves no undo step, no
//! document change, no new revision and no agent label behind.

use serde_json::json;

use super::COLLAPSE_REQUEST;
use crate::Session;
use crate::agents::Author;

#[test]
fn collapse_all_is_panel_state_only() {
    let mut s = Session::new();
    s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    for author in [Author::Person, Author::agent("mcp")] {
        s.set_author(author);
        let (doc, revision, undo) = {
            let ds = s.ds().unwrap();
            (ds.doc.clone(), ds.revision, ds.history.undo.len())
        };
        s.ui_requests.clear();
        s.execute("layers.collapse-all", &json!({})).unwrap();
        let ds = s.ds().unwrap();
        assert!(std::sync::Arc::ptr_eq(&ds.doc, &doc), "no document write");
        assert_eq!(ds.revision, revision, "no new revision");
        assert_eq!(ds.history.undo.len(), undo, "no undo step");
        assert_eq!(s.ui_requests[0].kind, COLLAPSE_REQUEST);
        // Idempotent: again, the same.
        s.execute("layers.collapse-all", &json!({})).unwrap();
        assert_eq!(s.ds().unwrap().revision, revision);
    }
    let feed = s.execute("history.changes", &json!({})).unwrap();
    assert!(!feed.to_string().contains("collapse"), "no change logged, so no \"Agent: …\" label: {feed}");
}
