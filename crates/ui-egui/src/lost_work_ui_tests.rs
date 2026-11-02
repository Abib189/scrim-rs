//! Fix AI stage 2b through the window: what the lost-work fixes say to a person, in English and
//! Spanish.

use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::layers_drag_tests::Ui;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-ui-lost-work-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// LW-2: a file an earlier version saved with `null` numbers opens, and the window says how many
/// were reset; an edit that would make a number that isn't finite is refused with a reason.
#[test]
fn null_numbers_and_numbers_that_arent_finite_are_explained_in_english_and_spanish() {
    let d = dir("numbers");
    let mut s = Session::new();
    let r = figcraft_engine::params::id_of(&s.execute("shape.rectangle", &json!({"width": 10, "height": 10})).unwrap()["id"]).unwrap();
    let mut v = figcraft_format::to_json(s.doc().unwrap()).unwrap();
    v["nodes"][r.0.to_string()]["opacity"] = Value::Null;
    v["nodes"][r.0.to_string()]["width"] = Value::Null;
    let file = d.join("Old.scrim");
    std::fs::write(&file, v.to_string()).unwrap();
    let mut u = Ui::new(Session::empty());
    u.app.run("file.open", json!({"path": file.to_string_lossy()}));
    u.frame(vec![]);
    assert_eq!(
        u.app.ui.toast.as_ref().map(|t| t.0.as_str()),
        Some("Old: 2 numbers saved as null by an earlier version were reset to their defaults. The file itself wasn't changed.")
    );
    let refused = crate::windows::error_text(&figcraft_engine::EngineError::NotFinite { cmd: "node.set".into(), layer: "Title".into() });
    assert!(refused.starts_with("“Title” would get a number that isn't finite"), "{refused}");
    for key in [
        "{name}: {n} numbers saved as null by an earlier version were reset to their defaults. The file itself wasn't changed.",
        "“{layer}” would get a number that isn't finite (too large, or not a number), which a file can't hold, so the change wasn't made.",
    ] {
        assert!(crate::i18n::own_entry("en", key).is_some(), "{key} in English");
        assert!(crate::i18n::own_entry("es", key).is_some_and(|t| t.contains("null") || t.contains("finito")), "{key} in Spanish");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// LW-3: saving a file whose images past the budget weren't loaded says they were kept.
#[test]
fn a_save_that_kept_images_left_in_the_file_says_so_in_english_and_spanish() {
    let mut u = Ui::new(Session::new());
    u.app.session.request_ui("notice.imagesKept", json!({"name": "Big", "count": 2}));
    u.app.handle_ui_requests();
    assert_eq!(
        u.app.ui.toast.as_ref().map(|t| t.0.as_str()),
        Some(
            "Big was saved with every image: 2 that weren't loaded (together they pass what Scrim reads from one file) were copied over from the file."
        )
    );
    let key = "{name} was saved with every image: {n} that weren't loaded (together they pass what Scrim reads from one file) were copied over from the file.";
    assert!(crate::i18n::own_entry("es", key).is_some_and(|t| t.contains("imágenes")));
}

/// Picks `path` when Save as asks where to save.
struct Picker(Option<std::path::PathBuf>);

impl crate::Services for Picker {
    fn pick_save(&mut self, _: &str, _: &str, _: &[&str]) -> crate::DialogFuture {
        Box::pin(std::future::ready(self.0.take()))
    }
}

/// An app with `file` open, saved, edited since, and then changed by another app; its bytes.
fn conflicted(file: &std::path::Path, pick: Option<std::path::PathBuf>) -> (crate::FigApp, Vec<u8>) {
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"width": 10, "height": 10, "name": "Mine"})).unwrap();
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 20, "width": 10, "height": 10, "name": "Unsaved"})).unwrap();
    let mut other = Session::new();
    other.execute("shape.rectangle", &json!({"width": 70, "height": 70, "name": "Theirs"})).unwrap();
    let theirs = figcraft_engine::cmd::file::file_bytes(other.ds().unwrap()).unwrap();
    std::fs::write(file, &theirs).unwrap();
    let mut app = crate::FigApp::new(s, Box::new(Picker(pick)));
    assert!(app.run("file.save", json!({})).is_none(), "the plain save is refused");
    assert_eq!(app.ui.save_conflict.as_deref(), Some("Doc"), "the three choices show");
    assert_eq!(app.inspect()["saveConflict"], "Doc", "and the control channel sees them");
    assert_eq!(std::fs::read(file).unwrap(), theirs);
    (app, theirs)
}

fn names(app: &crate::FigApp) -> Vec<String> {
    app.session.doc().unwrap().nodes().filter(|n| n.parent.is_some()).map(|n| n.name.clone()).collect()
}

/// LW-4: Save after another app changed the file shows Save as a copy, Overwrite and Reload from
/// disk; each runs its command, and Escape cancels.
#[test]
fn save_after_another_app_changed_the_file_offers_three_choices_and_escape_cancels() {
    use crate::save_conflict::{Choice, choose};
    let d = dir("conflict");
    let file = d.join("Doc.scrim");
    // Overwrite.
    let (mut app, _) = conflicted(&file, None);
    choose(&mut app, Choice::Overwrite);
    assert!(app.ui.save_conflict.is_none());
    assert!(figcraft_engine::cmd::file::load(&file.to_string_lossy()).unwrap().0.nodes().any(|n| n.name == "Unsaved"));
    // Reload from disk.
    let (mut app, _) = conflicted(&file, None);
    choose(&mut app, Choice::Reload);
    assert_eq!(names(&app), ["Theirs"]);
    // Save as a copy (the default): the other app's file stays as it is.
    let copy = d.join("Doc copy.scrim");
    let (mut app, theirs) = conflicted(&file, Some(copy.clone()));
    choose(&mut app, Choice::SaveCopy);
    app.poll_dialog(&egui::Context::default());
    assert!(figcraft_engine::cmd::file::load(&copy.to_string_lossy()).unwrap().0.nodes().any(|n| n.name == "Unsaved"));
    assert_eq!(std::fs::read(&file).unwrap(), theirs);
    // Escape cancels: nothing saved, the edits stay unsaved.
    let (app, theirs) = conflicted(&file, None);
    let mut u = Ui::new(Session::empty());
    u.app = app;
    u.app.ui.home = false;
    u.frame(vec![]);
    u.frame(vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
    assert!(u.app.ui.save_conflict.is_none(), "Escape closed the question");
    assert_eq!(std::fs::read(&file).unwrap(), theirs);
    assert!(u.app.session.ds().unwrap().is_dirty());
    for key in [
        "File changed on disk",
        "Another app changed {name} since you opened or last saved it here. Saving now would replace those changes.",
        "Save as a copy…",
        "Overwrite",
        "Reload from disk",
        "Auto-save paused for {name}: another app changed the file. Save to choose what to keep.",
    ] {
        assert!(crate::i18n::own_entry("es", key).is_some(), "{key} in Spanish");
    }
    let _ = std::fs::remove_dir_all(&d);
}
