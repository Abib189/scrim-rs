//! Fix AI C-1 through the window: a FigCraft board file opens with a notice, Save offers three
//! choices, each runs its command, and Escape cancels; in English and Spanish.

use figcraft_engine::Session;
use serde_json::json;

use super::{Choice, Shown, choose};
use crate::layers_drag_tests::Ui;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-ui-board-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A board file `Plan.scrim` in `d`, as upstream FigCraft writes one with a sticky note; its path
/// and bytes.
fn board(d: &std::path::Path) -> (std::path::PathBuf, Vec<u8>) {
    let mut s = Session::new();
    let id = figcraft_engine::params::id_of(
        &s.execute("node.create", &json!({"type": "frame", "width": 200, "height": 200, "name": "Sticky"})).unwrap()["id"],
    )
    .unwrap();
    let mut v = figcraft_format::to_json(s.doc().unwrap()).unwrap();
    v["kind"] = json!("board");
    v["nodes"][id.0.to_string()]["board"] = json!({"type": "sticky"});
    let file = d.join("Plan.scrim");
    std::fs::write(&file, v.to_string()).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    (file, bytes)
}

/// Picks `path` when Save as asks where to save.
struct Picker(Option<std::path::PathBuf>);

impl crate::Services for Picker {
    fn pick_save(&mut self, _: &str, _: &str, _: &[&str]) -> crate::DialogFuture {
        Box::pin(std::future::ready(self.0.take()))
    }
}

/// An app with the board file open (its notice showing), Save as picking `pick`.
fn opened(file: &std::path::Path, pick: Option<std::path::PathBuf>) -> crate::FigApp {
    let mut app = crate::FigApp::new(Session::empty(), Box::new(Picker(pick)));
    app.run("file.open", json!({"path": file.to_string_lossy()}));
    app
}

/// [`opened`], the notice closed, edited and saved: the three choices show.
fn saving(file: &std::path::Path, pick: Option<std::path::PathBuf>) -> crate::FigApp {
    let mut app = opened(file, pick);
    choose(&mut app, Choice::Close);
    app.run("shape.rectangle", json!({"x": 300, "width": 10, "height": 10, "name": "Mine"}));
    assert!(app.run("file.save", json!({})).is_none(), "the plain save is refused");
    assert_eq!(app.ui.board_file, Some(Shown::Saving("Plan".into())), "the three choices show");
    assert!(app.ui.toast.is_none(), "the question says it, not a toast: {:?}", app.ui.toast);
    assert_eq!(app.inspect()["boardFile"], json!({"kind": "saving", "name": "Plan"}));
    app
}

fn marks(file: &std::path::Path) -> Option<figcraft_format::BoardMarks> {
    figcraft_format::load_bytes_with_versions(&std::fs::read(file).unwrap()).unwrap().board
}

#[test]
fn a_board_file_opens_with_its_notice_and_a_design_without() {
    let d = dir("open");
    let (file, bytes) = board(&d);
    let mut app = opened(&file, None);
    assert_eq!(app.ui.board_file, Some(Shown::Opened("Plan".into())));
    assert_eq!(app.inspect()["boardFile"], json!({"kind": "opened", "name": "Plan"}));
    assert_eq!(std::fs::read(&file).unwrap(), bytes, "opening writes nothing");
    choose(&mut app, Choice::Close);
    assert!(app.ui.board_file.is_none());
    // Save a copy as .scrim, from the notice.
    let copy = d.join("Plan copy.scrim");
    let mut app = opened(&file, Some(copy.clone()));
    choose(&mut app, Choice::SaveCopy);
    app.poll_dialog(&egui::Context::default());
    assert_eq!(marks(&copy), None, "the copy is a .scrim design");
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    // A design file shows nothing.
    let design = d.join("Design.scrim");
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"width": 10, "height": 10})).unwrap();
    s.execute("file.save", &json!({"path": design.to_string_lossy()})).unwrap();
    let app = opened(&design, None);
    assert!(app.ui.board_file.is_none());
    assert!(app.inspect()["boardFile"].is_null());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn save_over_a_board_offers_three_choices_and_escape_cancels() {
    let d = dir("save");
    let (file, bytes) = board(&d);
    // Save as a copy (the default): the board file stays as it is.
    let copy = d.join("Plan copy.scrim");
    let mut app = saving(&file, Some(copy.clone()));
    choose(&mut app, Choice::SaveCopy);
    app.poll_dialog(&egui::Context::default());
    assert!(figcraft_engine::cmd::file::load(&copy.to_string_lossy()).unwrap().0.nodes().any(|n| n.name == "Mine"));
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    // Cancel: nothing saved, the edits stay unsaved.
    let mut app = saving(&file, None);
    choose(&mut app, Choice::Close);
    assert!(app.ui.board_file.is_none());
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    assert!(app.session.ds().unwrap().is_dirty());
    // Escape cancels too.
    let mut u = Ui::new(Session::empty());
    u.app = saving(&file, None);
    u.app.ui.home = false;
    u.frame(vec![]);
    u.frame(vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
    assert!(u.app.ui.board_file.is_none(), "Escape closed the question");
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    // Save anyway: written as a design.
    let mut app = saving(&file, None);
    choose(&mut app, Choice::SaveAnyway);
    assert!(!app.session.ds().unwrap().is_dirty());
    assert_eq!(marks(&file), None, "the board features are gone, as the question said");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_board_questions_are_in_english_and_spanish() {
    for key in [
        "Board file",
        "{name} is a board, a kind of whiteboard file Scrim doesn't support. It is shown as a normal design: connectors no longer follow what they join, and saving it removes its board features.",
        "Save a copy as .scrim…",
        "Close",
        "Save over a board?",
        "{name} is a board. Saving over it removes its board features. Save a copy to keep the board file as it is.",
        "Save as a copy…",
        "Save anyway",
        "Cancel",
    ] {
        assert!(crate::i18n::own_entry("en", key).is_some(), "{key} in English");
        assert!(crate::i18n::own_entry("es", key).is_some(), "{key} in Spanish");
    }
}
