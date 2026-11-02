//! Fix AI LW-4 (upstream FigCraft a648700f and 0d7d9ebd): Save never silently replaces what another
//! app wrote to the file since it was opened or saved here. Save asks (the window shows Save as a
//! copy, Overwrite and Reload from disk), `file.save {overwrite: true}` replaces it, `file.revert`
//! reloads it, Save as a new file never asks, and auto-save leaves a changed file alone.

use serde_json::json;

use crate::Session;
use crate::cmd::file::FileSnapshot;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-engine-disk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A session with `file` open, saved with one rectangle, and edited since (unsaved).
fn opened(file: &std::path::Path) -> Session {
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10, "name": "Mine"})).unwrap();
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 20, "y": 0, "width": 10, "height": 10, "name": "Unsaved"})).unwrap();
    s
}

/// Another app writes the file: a document with one layer named "Theirs".
fn write_theirs(file: &std::path::Path) -> Vec<u8> {
    let mut other = Session::new();
    other.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 30, "height": 30, "name": "Theirs"})).unwrap();
    let bytes = crate::cmd::file::file_bytes(other.ds().unwrap()).unwrap();
    // A different length, so the change shows whatever the clock's resolution.
    std::fs::write(file, &bytes).unwrap();
    bytes
}

fn names(s: &Session) -> Vec<String> {
    s.doc().unwrap().nodes().filter(|n| n.parent.is_some()).map(|n| n.name.clone()).collect()
}

fn asked(s: &Session) -> bool {
    s.ui_requests.iter().any(|r| r.kind == "dialog.saveConflict")
}

#[test]
fn save_after_another_app_changed_the_file_asks_and_writes_nothing() {
    let d = dir("ask");
    let file = d.join("Doc.scrim");
    let mut s = opened(&file);
    let theirs = write_theirs(&file);
    s.ui_requests.clear();
    assert!(s.execute("file.save", &json!({})).is_err(), "a plain save is refused");
    assert!(asked(&s), "the window is asked to show the three choices");
    assert_eq!(std::fs::read(&file).unwrap(), theirs, "the other app's file is untouched");
    assert!(s.ds().unwrap().is_dirty(), "the edits are still unsaved");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn overwrite_replaces_the_other_apps_changes() {
    let d = dir("overwrite");
    let file = d.join("Doc.scrim");
    let mut s = opened(&file);
    write_theirs(&file);
    s.execute("file.save", &json!({"overwrite": true})).expect("overwrite saves");
    let (back, _) = crate::cmd::file::load(&file.to_string_lossy()).unwrap();
    assert!(back.nodes().any(|n| n.name == "Unsaved"), "this tab's document is in the file");
    // Saved here now: the next plain save doesn't ask.
    s.execute("shape.rectangle", &json!({"x": 40, "y": 0, "width": 5, "height": 5})).unwrap();
    s.execute("file.save", &json!({})).expect("a later save of an unchanged file doesn't ask");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn reload_from_disk_brings_in_the_other_apps_file() {
    let d = dir("reload");
    let file = d.join("Doc.scrim");
    let mut s = opened(&file);
    write_theirs(&file);
    s.execute("file.revert", &json!({})).expect("reloads");
    assert_eq!(names(&s), ["Theirs"]);
    assert!(!s.ds().unwrap().is_dirty());
    // Read here now: saving again doesn't ask.
    s.execute("shape.rectangle", &json!({"x": 40, "y": 0, "width": 5, "height": 5})).unwrap();
    s.execute("file.save", &json!({})).expect("saves without asking");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn save_as_a_copy_never_asks_and_leaves_the_other_apps_file() {
    let d = dir("copy");
    let file = d.join("Doc.scrim");
    let mut s = opened(&file);
    let theirs = write_theirs(&file);
    s.ui_requests.clear();
    let copy = d.join("Doc copy.scrim");
    s.execute("file.saveAs", &json!({"path": copy.to_string_lossy()})).expect("a new path saves");
    assert!(!asked(&s));
    assert_eq!(std::fs::read(&file).unwrap(), theirs);
    assert!(crate::cmd::file::load(&copy.to_string_lossy()).unwrap().0.nodes().any(|n| n.name == "Unsaved"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_file_nobody_else_changed_saves_without_asking() {
    let d = dir("unchanged");
    let file = d.join("Doc.scrim");
    let mut s = opened(&file);
    s.ui_requests.clear();
    s.execute("file.save", &json!({})).expect("saves");
    assert!(!asked(&s));
    // Written again with the same bytes (a sync app touching it): not a change.
    let same = std::fs::read(&file).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&file, &same).unwrap();
    s.execute("shape.rectangle", &json!({"x": 40, "y": 0, "width": 5, "height": 5})).unwrap();
    s.execute("file.save", &json!({})).expect("the same bytes rewritten don't count as a change");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn auto_save_leaves_a_file_another_app_changed_alone() {
    let d = dir("auto");
    let file = d.join("Doc.scrim");
    let mut s = opened(&file);
    let theirs = write_theirs(&file);
    let snap = FileSnapshot::of(s.ds().unwrap());
    let bytes = snap.encode().unwrap();
    s.ui_requests.clear();
    assert!(!snap.save(&mut s, &bytes).expect("no error"), "nothing written");
    assert_eq!(std::fs::read(&file).unwrap(), theirs, "the file is untouched");
    assert!(s.ui_requests.iter().any(|r| r.kind == "notice.autoSavePaused"), "the window is told once");
    assert!(s.ds().unwrap().auto_save_paused(), "auto-save skips it until a save or reload");
    let again = FileSnapshot::of(s.ds().unwrap());
    let bytes = again.encode().unwrap();
    s.ui_requests.clear();
    assert!(!again.save(&mut s, &bytes).unwrap());
    assert!(!s.ui_requests.iter().any(|r| r.kind == "notice.autoSavePaused"), "only once");
    let _ = std::fs::remove_dir_all(&d);
}
