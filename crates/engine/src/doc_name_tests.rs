//! Fix AF: one name for a document. A file saved with the placeholder name goes by its file name;
//! a name someone gave wins over it; unsaved documents are "Untitled", "Untitled 2". Opening and
//! saving never write the name; renaming writes only the name.

use std::path::{Path, PathBuf};

use figcraft_doc::{Document, NodeKind};
use serde_json::json;

use super::{DocName, UNTITLED, explicit, resolve};
use crate::Session;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-doc-name-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A `.figcraft` file storing the name `stored`, written at `dir/file`.
fn saved(dir: &Path, file: &str, stored: &str) -> String {
    let mut s = Session::with_document(Document::new(stored));
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    let path = dir.join(file).to_string_lossy().into_owned();
    std::fs::write(&path, crate::cmd::file::file_bytes(s.ds().unwrap()).unwrap()).unwrap();
    path
}

fn stored_in(path: &str) -> Option<String> {
    figcraft_format::stored_name::stored_name(Path::new(path))
}

#[test]
fn the_rule() {
    assert_eq!(resolve("Poster", Some("/x/zeichen-26.figcraft"), 1), DocName::Named("Poster".into()));
    for placeholder in [UNTITLED, "", "   "] {
        assert_eq!(explicit(placeholder), None);
        assert_eq!(resolve(placeholder, Some("/x/zeichen-26.figcraft"), 1), DocName::Named("zeichen-26".into()));
        assert_eq!(resolve(placeholder, None, 1).english(), "Untitled");
        assert_eq!(resolve(placeholder, None, 3).english(), "Untitled 3");
    }
    // Only the placeholder itself is "not set".
    assert_eq!(resolve("Untitled 2", Some("/x/a.figcraft"), 1), DocName::Named("Untitled 2".into()));
    assert_eq!(resolve("untitled", Some("/x/a.figcraft"), 1), DocName::Named("untitled".into()));
    assert_eq!(resolve("archive.tar", Some("/x/b.v2.figcraft"), 1), DocName::Named("archive.tar".into()));
    assert_eq!(resolve(UNTITLED, Some("/x/b.v2.figcraft"), 1), DocName::Named("b.v2".into()));
}

/// The reported case: a file an agent made with `file.new` and saved without naming, opened from
/// disk, goes by its file name in what the engine reports (tab, panel, title and Home read the same
/// function: `crates/ui-egui/src/doc_name_ui_tests.rs`).
#[test]
fn a_file_saved_with_the_placeholder_goes_by_its_file_name() {
    let d = dir("placeholder");
    let path = saved(&d, "zeichen-26.figcraft", UNTITLED);
    let before = std::fs::read(&path).unwrap();
    let mut s = Session::empty();
    let r = s.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(r["summary"]["name"], "zeichen-26", "file.open's summary (MCP open_file)");
    assert_eq!(s.ds().unwrap().display_name(), "zeichen-26");
    assert_eq!(crate::inspect::document(&s, 1, false)["name"], "zeichen-26", "document.inspect (MCP open_file's document)");
    assert_eq!(s.status()["document"], "zeichen-26");
    assert_eq!(s.doc().unwrap().name, UNTITLED, "the stored name isn't rewritten by opening");
    assert_eq!(std::fs::read(&path).unwrap(), before, "opening doesn't change the file's bytes");
    // Opened again in a fresh session (no stale name from an earlier one, or from the recents).
    let mut again = Session::empty();
    again.prefs.recent_files = vec![path.clone()];
    again.execute("file.openRecent", &json!({"path": path})).unwrap();
    assert_eq!(again.ds().unwrap().display_name(), "zeichen-26");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_name_someone_gave_wins_over_the_file_name() {
    let d = dir("explicit");
    let path = saved(&d, "zeichen-26.figcraft", "Poster");
    let mut s = Session::empty();
    let r = s.execute("file.open", &json!({"path": path})).unwrap();
    assert_eq!(r["summary"]["name"], "Poster");
    assert_eq!(s.ds().unwrap().display_name(), "Poster");
    assert_eq!(crate::inspect::document(&s, 1, false)["name"], "Poster");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn unsaved_documents_are_untitled_then_numbered() {
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    s.execute("file.new", &json!({})).unwrap();
    let names = |s: &Session| s.docs.iter().map(crate::DocState::display_name).collect::<Vec<_>>();
    assert_eq!(names(&s), ["Untitled", "Untitled 2", "Untitled 3"]);
    // A tab keeps its number when another closes; a new one takes the lowest number free.
    s.execute("file.close", &json!({"index": 0})).unwrap();
    assert_eq!(names(&s), ["Untitled 2", "Untitled 3"]);
    s.execute("file.new", &json!({})).unwrap();
    assert_eq!(names(&s), ["Untitled 2", "Untitled 3", "Untitled"]);
    // A name given at creation is a name: no number.
    s.execute("file.new", &json!({"name": "Landing"})).unwrap();
    s.execute("file.new", &json!({})).unwrap();
    assert_eq!(names(&s), ["Untitled 2", "Untitled 3", "Untitled", "Landing", "Untitled 4"]);
}

#[test]
fn save_as_names_an_unsaved_document_after_its_file_without_writing_the_name() {
    let d = dir("save-as");
    let mut s = Session::new();
    s.execute("file.new", &json!({})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "Untitled 2");
    let path = d.join("Spring poster.figcraft").to_string_lossy().into_owned();
    s.execute("file.saveAs", &json!({"path": path})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "Spring poster", "at once, without reopening");
    assert_eq!(crate::inspect::document(&s, 1, false)["name"], "Spring poster");
    assert_eq!(stored_in(&path).as_deref(), Some(UNTITLED), "saving writes no name: the file keeps going by its file name");
    // The first untitled tab keeps its own number.
    assert_eq!(s.docs[0].display_name(), "Untitled");
    // Saved again under another name, it follows that file.
    let other = d.join("Poster v2.figcraft").to_string_lossy().into_owned();
    s.execute("file.saveAs", &json!({"path": other})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "Poster v2");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn rename_sets_the_name_and_leaves_the_file_on_disk_alone() {
    let d = dir("rename");
    let path = saved(&d, "zeichen-26.figcraft", UNTITLED);
    let before = std::fs::read(&path).unwrap();
    let mut s = Session::empty();
    s.execute("file.open", &json!({"path": path})).unwrap();
    s.execute("file.rename", &json!({"name": "Festival poster"})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "Festival poster");
    assert_eq!(s.ds().unwrap().path.as_deref(), Some(path.as_str()), "same file");
    assert_eq!(std::fs::read(&path).unwrap(), before, "the file isn't touched until it's saved");
    let files: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(files, ["zeichen-26.figcraft"], "no file renamed or added");
    // Saved, the file stores the name (and keeps its file name).
    s.execute("file.save", &json!({})).unwrap();
    assert_eq!(stored_in(&path).as_deref(), Some("Festival poster"));
    // Undone, it goes by its file name again.
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "zeichen-26");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn copies_are_named_after_the_name_shown() {
    let d = dir("copies");
    let path = saved(&d, "zeichen-26.figcraft", UNTITLED);
    let mut s = Session::empty();
    s.execute("file.open", &json!({"path": path})).unwrap();
    s.execute("file.duplicate", &json!({})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "zeichen-26 (Copy)");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_recent_files_list_names_files_as_they_are_shown() {
    let d = dir("recents");
    let plain = saved(&d, "zeichen-26.figcraft", UNTITLED);
    let named = saved(&d, "x1.figcraft", "Poster");
    let svg = d.join("logo.svg").to_string_lossy().into_owned();
    std::fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"/>"#).unwrap();
    let mut s = Session::empty();
    s.prefs.recent_files = vec![plain.clone(), named.clone(), svg.clone()];
    let names = |s: &mut Session| {
        let r = s.execute("file.openRecent", &json!({})).unwrap();
        r["files"].as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(names(&mut s), ["zeichen-26", "Poster", "logo"]);
    // An open file's rename shows at once, saved or not.
    s.execute("file.open", &json!({"path": plain})).unwrap();
    s.execute("file.rename", &json!({"name": "Festival"})).unwrap();
    assert_eq!(names(&mut s)[0..2], ["Festival", "Poster"]);
    std::fs::remove_dir_all(&d).ok();
}

/// Files from before Fix AF and from other tools open as they did, under the names they carry.
#[test]
fn old_files_and_imports_still_open() {
    let d = dir("imports");
    // Bare JSON (the oldest native form) storing the placeholder.
    let json_path = d.join("old-json.json").to_string_lossy().into_owned();
    let mut old = Document::new(UNTITLED);
    let page = old.first_page().unwrap();
    old.create(page, NodeKind::Rectangle, "R", 0.0, 0.0, 10.0, 10.0).unwrap();
    std::fs::write(&json_path, serde_json::to_vec(&old).unwrap()).unwrap();
    // REST API file JSON with its own name, and one without.
    let rest = |name: Option<&str>| {
        let mut v =
            json!({"document": {"id": "0:0", "type": "DOCUMENT", "children": [{"id": "0:1", "type": "CANVAS", "name": "Page 1", "children": []}]}});
        if let Some(n) = name {
            v["name"] = json!(n);
        }
        serde_json::to_vec(&v).unwrap()
    };
    let rest_path = d.join("export.json").to_string_lossy().into_owned();
    std::fs::write(&rest_path, rest(Some("From Figma"))).unwrap();
    let nameless = d.join("nameless.json").to_string_lossy().into_owned();
    std::fs::write(&nameless, rest(None)).unwrap();
    let mut s = Session::empty();
    s.execute("file.open", &json!({"path": json_path})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "old-json");
    s.execute("file.open", &json!({"path": rest_path})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "From Figma");
    s.execute("file.open", &json!({"path": nameless})).unwrap();
    assert_eq!(s.ds().unwrap().display_name(), "Imported file", "as before: an import has no file of its own");
    std::fs::remove_dir_all(&d).ok();
}
