//! Fix AD: with no document open (Scrim starts on Home), every command in the registry refuses
//! cleanly or works without one. None panics, and none makes a document except the ones whose job
//! that is.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::json;

use crate::{EngineError, Session};

/// The commands whose job is to make or open a document: only these may leave one behind.
fn makes_a_document(id: &str) -> bool {
    id == "file.new" || id.starts_with("template.") || id.starts_with("file.newFrom")
}

/// Every command run on a session with no document, with no parameters: no panic, no document
/// made on the caller's behalf, and the commands that act on the current document say so
/// (`EngineError::NoDocument`, "no open document").
#[test]
fn every_command_with_no_document_refuses_or_works_without_panicking() {
    let (mut panicked, mut made, mut refused) = (Vec::new(), Vec::new(), 0);
    for c in super::all_commands() {
        let mut s = Session::empty();
        match catch_unwind(AssertUnwindSafe(|| s.execute(c.id, &json!({})))) {
            Err(_) => panicked.push(c.id),
            Ok(Err(EngineError::NoDocument)) => refused += 1,
            Ok(_) => {}
        }
        if !s.docs.is_empty() && !makes_a_document(c.id) {
            made.push(c.id);
        }
    }
    assert!(panicked.is_empty(), "panicked with no document: {panicked:?}");
    assert!(made.is_empty(), "made a document unasked: {made:?}");
    assert!(refused > 100, "the document commands refuse with NoDocument ({refused} did)");
}

/// The refusal's words, which the window translates and the MCP tools recognise.
#[test]
fn the_no_document_refusal_reads_no_open_document() {
    let mut s = Session::empty();
    let e = s.execute("node.create", &json!({"type": "rectangle"})).expect_err("refused");
    assert!(matches!(e, EngineError::NoDocument), "{e}");
    assert_eq!(e.to_string(), "no open document");
    assert!(s.docs.is_empty());
}
