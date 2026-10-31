//! Fix AF through the window: a document has one name in the tab, the left panel's header, the
//! window title, Home's recent files (cards, search and sort by name) and `ui.inspect`. A rename in
//! the panel and a Save as show everywhere at once; long names end in "…" with the whole name as
//! a tooltip.

use std::path::{Path, PathBuf};
use std::time::Duration;

use egui::accesskit::{Node, Role};
use egui::{Event, Key, Modifiers, Pos2};
use figcraft_doc::{Document, NodeKind};
use figcraft_engine::Session;
use figcraft_engine::doc_name::UNTITLED;
use serde_json::{Value, json};

use crate::home_recents::{Sort, SortKey};
use crate::layers_drag_tests::Ui;
use crate::live_tests::flat;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-doc-name-ui-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A `.figcraft` file storing the name `stored`, at `dir/file`.
fn saved(dir: &Path, file: &str, stored: &str) -> String {
    let mut doc = Document::new(stored);
    let page = doc.first_page().unwrap();
    doc.create(page, NodeKind::Rectangle, "Poster art", 0.0, 0.0, 200.0, 300.0).unwrap();
    let path = dir.join(file).to_string_lossy().into_owned();
    std::fs::write(&path, figcraft_format::save_bytes(&doc, None).unwrap()).unwrap();
    path
}

fn window(recent: &[&str]) -> Ui {
    let mut s = Session::empty();
    s.prefs.recent_files = recent.iter().map(|p| (*p).to_string()).collect();
    s.prefs.getting_started_dismissed = true;
    let u = Ui::new(s);
    u.ctx.enable_accesskit();
    u
}

fn nodes(u: &mut Ui) -> Vec<Node> {
    u.frame_output(vec![]).platform_output.accesskit_update.unwrap().nodes.into_iter().map(|(_, n)| n).collect()
}

/// What the window calls the document shown: the active tab, the left panel's header (the file
/// name field) and the title; and what `ui.inspect` reports for its tab.
fn names(u: &mut Ui) -> (String, String, String, Value) {
    u.app.ui.home = false;
    u.frame(vec![]);
    let nodes = nodes(u);
    let tab = file_tab(&nodes).and_then(Node::label).unwrap_or_default();
    let header = nodes.iter().find(|n| n.role() == Role::TextInput && n.label() == Some("Name")).and_then(Node::value).unwrap_or_default();
    let active = u.app.session.active;
    (tab.to_string(), header.to_string(), u.app.title(), u.app.inspect()["tabs"][active]["name"].clone())
}

/// The active file's tab: the selected tab highest in the window (the panels' tabs are below).
fn file_tab(nodes: &[Node]) -> Option<&Node> {
    let top = |n: &&Node| n.bounds().map_or(f64::MAX, |b| b.y0);
    nodes.iter().filter(|n| n.role() == Role::Tab && n.is_selected() == Some(true)).min_by(|a, b| top(a).total_cmp(&top(b)))
}

/// Home's recent files as listed, by name, once every file's stored name has been read (the reader
/// is a thread of its own).
fn home_names(u: &mut Ui) -> Vec<String> {
    u.app.ui.home = true;
    let mut out = Vec::new();
    for _ in 0..400 {
        u.frame(vec![]);
        let paths = u.app.session.prefs.recent_files.clone();
        let r = u.app.inspect()["homeRecents"]["files"].clone();
        out = r.as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap().to_string()).collect();
        // A file's stored name comes in with its first look.
        if paths.iter().all(|p| recents_read(u, p)) && out.len() == paths.len() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    out
}

/// Whether Home has looked at `path` (a file storing no name reads as none).
fn recents_read(u: &Ui, path: &str) -> bool {
    u.app.inspect()["homeRecents"]["files"].as_array().unwrap().iter().any(|f| f["path"] == path && !f["exists"].is_null())
}

fn ok(path: &str, want: &str, u: &mut Ui) {
    let (tab, header, title, inspected) = names(u);
    assert_eq!(tab, want, "the tab ({path})");
    assert_eq!(header, want, "the left panel's header");
    // (An unsaved change adds " •".)
    assert_eq!(title.trim_end_matches(" – Scrim").trim_end_matches(" •"), want, "the window title: {title}");
    assert_eq!(inspected, want, "ui.inspect");
}

/// The reported case: a file saved with the placeholder name, opened from disk.
#[test]
fn a_file_saved_as_untitled_goes_by_its_file_name_everywhere() {
    let d = dir("placeholder");
    let path = saved(&d, "zeichen-26.figcraft", UNTITLED);
    let before = std::fs::read(&path).unwrap();
    let mut u = window(&[&path]);
    assert_eq!(home_names(&mut u), ["zeichen-26"], "Home's card before opening");
    let r = u.app.run("file.open", json!({"path": path})).unwrap();
    assert_eq!(r["summary"]["name"], "zeichen-26", "what MCP open_file reports");
    ok(&path, "zeichen-26", &mut u);
    let inspected = figcraft_engine::inspect::document(&u.app.session, 0, false);
    assert_eq!(inspected["name"], "zeichen-26", "MCP open_file's document (document.inspect)");
    assert_eq!(home_names(&mut u), ["zeichen-26"], "Home's card with the file open");
    assert_eq!(std::fs::read(&path).unwrap(), before, "opening didn't write the file");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_name_someone_gave_shows_everywhere_instead_of_the_file_name() {
    let d = dir("explicit");
    let path = saved(&d, "x1.figcraft", "Poster");
    let mut u = window(&[&path]);
    assert_eq!(home_names(&mut u), ["Poster"], "Home's card reads the name from the file");
    u.app.run("file.open", json!({"path": path})).unwrap();
    ok(&path, "Poster", &mut u);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn unsaved_documents_are_untitled_then_untitled_2_in_english_and_spanish() {
    let mut u = window(&[]);
    u.app.run("file.new", json!({})).unwrap();
    ok("new", "Untitled", &mut u);
    u.app.run("file.new", json!({})).unwrap();
    ok("second", "Untitled 2", &mut u);
    assert_eq!(crate::i18n::own_entry("es", "Untitled"), Some("Sin título"));
    assert_eq!(crate::i18n::own_entry("es", "Untitled {n}"), Some("Sin título {n}"));
}

#[test]
fn save_as_names_a_new_document_after_its_file_at_once() {
    let d = dir("save-as");
    let mut u = window(&[]);
    u.app.run("file.new", json!({})).unwrap();
    let path = d.join("Spring poster.figcraft").to_string_lossy().into_owned();
    // What the Save as dialog runs once a name is chosen.
    u.app.run("file.saveAs", json!({"path": path})).unwrap();
    ok(&path, "Spring poster", &mut u);
    assert_eq!(home_names(&mut u), ["Spring poster"]);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn rename_in_the_panel_names_the_document_and_leaves_the_file_alone() {
    let d = dir("rename");
    let path = saved(&d, "zeichen-26.figcraft", UNTITLED);
    let before = std::fs::read(&path).unwrap();
    let mut u = window(&[&path]);
    u.app.run("file.open", json!({"path": path})).unwrap();
    u.app.ui.home = false;
    // File ▸ Rename starts the header's field with the name selected; typing replaces it.
    u.app.ui.rename_file = true;
    u.frame(vec![]);
    u.frame(vec![]);
    u.frame(vec![Event::Text("Festival poster".into())]);
    u.frame(vec![Event::Key { key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    assert_eq!(u.app.session.doc().unwrap().name, "Festival poster", "the document's own name");
    ok(&path, "Festival poster", &mut u);
    assert_eq!(home_names(&mut u), ["Festival poster"], "Home at once, before a save");
    assert_eq!(std::fs::read(&path).unwrap(), before, "the file isn't renamed or written");
    let files: Vec<_> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(files, ["zeichen-26.figcraft"]);
    assert_eq!(u.app.session.ds().unwrap().path.as_deref(), Some(path.as_str()));
    std::fs::remove_dir_all(&d).ok();
}

/// Home's search and its sort by name go by the names shown, not by the file names.
#[test]
fn home_search_and_sort_by_name_use_the_name_shown() {
    let d = dir("search");
    let zebra = saved(&d, "aaa.figcraft", "Zebra");
    let plain = saved(&d, "mmm.figcraft", UNTITLED);
    let mut u = window(&[&zebra, &plain]);
    Sort { key: SortKey::Name, descending: false }.store(&mut u.app.session.prefs);
    assert_eq!(home_names(&mut u), ["mmm", "Zebra"], "sorted by the names shown (by file name, aaa would come first)");
    u.app.ui.home_view.file_query = "zebra".into();
    let mut found = false;
    for _ in 0..10 {
        let nodes = nodes(&mut u);
        found = nodes.iter().any(|n| n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("Zebra")));
        if found {
            break;
        }
    }
    assert!(found, "a search for the name finds the file");
    std::fs::remove_dir_all(&d).ok();
}

/// A name too long for the tab or the header ends in "…"; hovered, the tab shows it whole.
#[test]
fn long_names_are_cut_with_an_ellipsis_and_shown_whole_in_a_tooltip() {
    let long = "A very long document name for the spring festival poster series, final version";
    let mut u = window(&[]);
    u.app.run("file.new", json!({"name": long})).unwrap();
    u.app.ui.home = false;
    let tab = file_tab(&nodes(&mut u)).unwrap().clone();
    assert_eq!(tab.label(), Some(long), "screen readers get the whole name");
    // Drawn cut short: egui lays the whole text out and ends it in "…" at the width given (about
    // 430 points whole at this size).
    let shapes = flat(&u.frame_output(vec![]).shapes);
    let drawn: Vec<f32> = crate::live_tests::texts(&shapes).into_iter().filter(|(t, _, _)| t == long).map(|(_, _, size)| size.x).collect();
    assert_eq!(drawn.len(), 2, "the tab and the header: {drawn:?}");
    assert!(drawn.iter().all(|w| *w < 220.0), "both cut short: {drawn:?}");
    let header = nodes(&mut u).into_iter().find(|n| n.role() == Role::TextInput && n.label() == Some("Name")).unwrap();
    for (what, node) in [("tab", tab), ("header", header)] {
        let b = node.bounds().unwrap();
        let over = Pos2::new(((b.x0 + b.x1) / 2.0 - 30.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
        u.frame(vec![Event::PointerMoved(Pos2::new(700.0, 500.0))]);
        u.frame(vec![Event::PointerMoved(over)]);
        // Whole: wider than the field allows, or wrapped onto more lines.
        let whole = |u: &mut Ui| {
            crate::live_tests::texts(&flat(&u.frame_output(vec![]).shapes)).iter().any(|(t, _, size)| t == long && (size.x > 300.0 || size.y > 24.0))
        };
        assert!((0..120).any(|_| whole(&mut u)), "the {what}'s tooltip has the whole name");
    }
}
