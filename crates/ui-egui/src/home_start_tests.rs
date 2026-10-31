//! Fix AD: the window with no document shows Home; a file that arrives takes over; the last one
//! closing returns to Home; a command that needs a document says, in the interface language, that
//! none is open; and nothing is autosaved or added to recents meanwhile.

use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::click_named;

const NO_FILE: &str = "No file is open. Open a file, or start a new design file from Home.";

/// A window with no document, as Scrim now starts.
fn home() -> Ui {
    let mut u = Ui::new(Session::empty());
    u.app.ui.home = true;
    u.frame(vec![]);
    u
}

/// Home's New design file makes one blank document and shows it.
#[test]
fn new_design_file_on_home_makes_one_document() {
    let mut u = home();
    click_named(&mut u, "New design file");
    u.frame(vec![]);
    assert_eq!(u.app.session.docs.len(), 1);
    assert!(!u.app.showing_home(), "the new file shows");
}

/// File ▸ New (the command, as the menu and the + tab run it, and as an agent may) from Home makes
/// one document, and the window moves to it. Before Fix AD the window stayed on Home with the new
/// tab behind it.
#[test]
fn file_new_from_home_makes_one_document_and_shows_it() {
    let mut u = home();
    u.app.run("file.new", json!({}));
    u.frame(vec![]);
    assert_eq!(u.app.session.docs.len(), 1);
    assert!(!u.app.ui.home, "the window shows the new file, not Home");
}

/// An agent's open_file (or a file from Finder) into a window showing only Home: the file opens
/// in its own tab and the window shows it.
#[test]
fn a_file_opened_on_an_empty_home_is_shown() {
    let mut u = home();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ridgeline-app.figcraft");
    u.app.run("file.open", json!({"path": path.to_string_lossy()}));
    u.frame(vec![]);
    assert_eq!(u.app.session.docs.len(), 1);
    assert!(!u.app.ui.home);
}

/// With files open, Home chosen from the tab bar stays: an open doesn't pull the window away.
#[test]
fn home_chosen_with_files_open_stays() {
    let mut u = Ui::new(Session::new());
    u.app.ui.home = true;
    u.frame(vec![]);
    u.app.run("file.new", json!({}));
    u.frame(vec![]);
    assert!(u.app.ui.home, "Home stays: it was chosen, not empty");
}

/// Closing the last document returns to Home with no document: no Untitled takes its place.
#[test]
fn closing_the_last_document_returns_home_with_none() {
    let mut u = Ui::new(Session::new());
    u.app.ui.home = false;
    u.frame(vec![]);
    u.app.run("file.close", json!({}));
    for _ in 0..3 {
        u.frame(vec![]);
    }
    assert_eq!(u.app.session.docs.len(), 0);
    assert!(u.app.showing_home());
    assert_eq!(u.app.inspect()["home"], true, "ui.inspect says Home shows");
}

/// A command that needs a document, with none open, is refused in words: the interface's, saying
/// what to do (English here, Spanish in its catalog). Before Fix AD the window showed the
/// engine's own English "no open document" in every language.
#[test]
fn a_document_command_with_none_open_is_refused_in_the_interface_language() {
    let mut u = home();
    assert!(u.app.run("node.create", json!({"type": "rectangle"})).is_none(), "refused");
    assert_eq!(u.app.ui.last_error.as_deref(), Some(NO_FILE));
    assert_eq!(u.app.session.docs.len(), 0, "nothing made to satisfy it");
    let es = crate::i18n::own_entry("es", NO_FILE).expect("in the Spanish catalog");
    assert!(es.starts_with("No hay ningún archivo abierto"), "{es}");
}

/// Nothing is autosaved and nothing joins the recent files while no document exists, however
/// long the window stays on Home.
#[test]
fn autosave_and_recents_do_nothing_with_no_document() {
    let mut u = home();
    let recents = u.app.session.prefs.recent_files.clone();
    for _ in 0..3 {
        u.time += crate::AUTOSAVE_SECS + 1.0;
        u.frame(vec![]);
    }
    assert_eq!(u.app.session.docs.len(), 0);
    assert_eq!(u.app.session.prefs.recent_files, recents);
}
