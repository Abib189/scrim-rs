//! The Code tab (B2): the format menus run `code.setFormat`, Copy runs `code.copy`, and Copy
//! standalone HTML puts a complete page on the clipboard; an agent's commands do the same.

use std::sync::{Arc, Mutex};

use figcraft_engine::codegen::CodeLang;
use serde_json::json;

use crate::live_tests::{Harness, has_text, text_center};

/// The clipboard, kept where the test can read it.
struct Clipboard(Arc<Mutex<Vec<String>>>);

impl crate::Services for Clipboard {
    fn clipboard_set_text(&mut self, s: &str) {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(s.to_string());
    }
}

/// The Code tab on the rectangle, with a clipboard the test reads.
fn harness() -> (Harness, Arc<Mutex<Vec<String>>>) {
    let (mut h, rect) = Harness::new();
    let copied = Arc::new(Mutex::new(Vec::new()));
    h.app.services = Box::new(Clipboard(copied.clone()));
    h.app.session.execute("object.rename", &json!({"ids": [rect], "name": "Hero"})).unwrap();
    h.app.session.execute("select.set", &json!({"ids": [rect]})).unwrap();
    h.app.session.execute("panel.tab.code", &json!({})).unwrap();
    h.app.handle_ui_requests();
    h.frames(3);
    (h, copied)
}

#[test]
fn the_format_menu_and_copy_run_the_commands() {
    let (mut h, copied) = harness();
    let shapes = h.frames(1);
    h.click(text_center(&shapes, "CSS"));
    let shapes = h.frames(2);
    h.click(text_center(&shapes, "React"));
    let shapes = h.frames(2);
    assert_eq!(h.app.ui.dev.code_lang, CodeLang::React);
    assert!(has_text(&shapes, "Copy React"));
    h.click(text_center(&shapes, "Copy React"));
    h.frames(1);
    let want = h.app.session.execute("inspect.code", &json!({"language": "react"})).unwrap()["code"].as_str().unwrap().to_string();
    assert_eq!(copied.lock().unwrap().last(), Some(&want), "the code, as inspect.code gives it");
    // An agent switches the tab's format with the same command.
    h.app.session.execute("code.setFormat", &json!({"format": "swiftui"})).unwrap();
    h.app.handle_ui_requests();
    let shapes = h.frames(2);
    assert!(h.app.ui.dev.code_lang == CodeLang::SwiftUi && has_text(&shapes, "Copy SwiftUI"));
    assert!(!has_text(&shapes, "rem") && !has_text(&shapes, "px"), "no unit menu outside the web formats");
}

#[test]
fn copy_standalone_html_puts_a_complete_page_on_the_clipboard() {
    let (mut h, copied) = harness();
    let (doc, steps) = (h.app.session.doc().unwrap().clone(), h.app.session.undo_depth());
    let shapes = h.frames(1);
    h.click(text_center(&shapes, "Copy standalone HTML"));
    let shapes = h.frames(2);
    let page = copied.lock().unwrap().last().cloned().unwrap_or_default();
    assert!(page.starts_with("<!doctype html>") && page.contains("<title>Hero</title>") && page.contains("default-src 'none'"), "{page}");
    assert!(has_text(&shapes, "Copied standalone HTML"), "the toast says what was copied");
    assert!(h.app.session.doc().unwrap() == &doc && h.app.session.undo_depth() == steps, "copying never edits");
}
