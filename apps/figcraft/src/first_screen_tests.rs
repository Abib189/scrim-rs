//! Fix AD: launching opens Home, not a new Untitled file.

use figcraft_engine::Session;

use super::{open_files_with_results, starts_on_home};

fn example() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ridgeline-app.figcraft").to_string_lossy().into_owned()
}

/// No file named: no document is made, nothing is open, and the window starts on Home. (Before
/// Fix AD every launch after the first made an Untitled file and hid Home.)
#[test]
fn with_no_file_the_window_starts_on_home_with_no_document() {
    let mut s = Session::empty();
    assert!(open_files_with_results(&mut s, &[]).0.is_empty());
    assert_eq!(s.docs.len(), 0, "no Untitled file");
    assert!(starts_on_home(&s));
}

/// A file named on the command line: exactly that one document, no Untitled beside it, and the
/// window shows it.
#[test]
fn a_named_file_opens_alone() {
    let mut s = Session::empty();
    let opened = open_files_with_results(&mut s, &[example()]).0;
    assert_eq!(opened.len(), 1);
    assert_eq!(s.docs.len(), 1);
    assert!(s.docs.iter().all(|d| d.doc.name != "Untitled"), "{:?}", s.docs.iter().map(|d| &d.doc.name).collect::<Vec<_>>());
    assert!(!starts_on_home(&s));
}

/// A file that can't be opened is skipped: nothing takes its place, so the window starts on Home.
#[test]
fn a_file_that_cant_open_leaves_home() {
    let mut s = Session::empty();
    assert!(open_files_with_results(&mut s, &["/nonexistent/scrim-fix-ad.scrim".to_string()]).0.is_empty());
    assert_eq!(s.docs.len(), 0);
    assert!(starts_on_home(&s));
}

/// A file opened from Finder or the Dock: macOS starts the app with no file named, then sends the
/// file. Exactly one document follows, with no empty Untitled tab left behind (there was one
/// before Fix AD).
#[test]
fn a_file_from_finder_after_launch_opens_alone() {
    let mut s = Session::empty();
    open_files_with_results(&mut s, &[]);
    s.execute("file.open", &serde_json::json!({"path": example()})).expect("opens");
    assert_eq!(s.docs.len(), 1, "{:?}", s.docs.iter().map(|d| &d.doc.name).collect::<Vec<_>>());
}

/// Fix AG: the last file named at launch hands back its result, so an import's report can show.
#[test]
fn the_last_file_opened_at_launch_gives_its_result() {
    let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/format/tests/fixtures/sketch/sample.sketch");
    let mut s = Session::empty();
    let (opened, last) = open_files_with_results(&mut s, &[example(), sample.to_string_lossy().into_owned()]);
    assert_eq!(opened.len(), 2);
    let (params, result) = last.expect("the Sketch file's result");
    let result = result.expect("it opens");
    assert!(params["path"].as_str().is_some_and(|p| p.ends_with("sample.sketch")));
    assert!(result["warnings"].as_array().is_some_and(|w| !w.is_empty()), "{result}");
}

/// Fix AH3: a closed-format file named at launch hands back why it can't open (the window shows
/// the dialog), and nothing opens.
#[test]
fn a_closed_format_named_at_launch_hands_back_its_refusal() {
    let dir = std::env::temp_dir().join(format!("scrim-launch-closed-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("logo.afdesign");
    std::fs::write(&file, b"closed").unwrap();
    let mut s = Session::empty();
    let (opened, last) = open_files_with_results(&mut s, &[file.to_string_lossy().into_owned()]);
    assert!(opened.is_empty());
    let err = last.unwrap().1.unwrap_err();
    assert!(matches!(err, figcraft_engine::EngineError::Unreadable(figcraft_engine::importers::Unreadable::ClosedFormat("Affinity"))), "{err:?}");
    std::fs::remove_dir_all(&dir).ok();
}

/// Fix AI C-1, found by the live check: what opening a file at launch asks the window to show (a
/// board file's notice, LW-2's note about reset numbers) shows when the window comes up, not at the
/// person's first command; and an agent's first command can't drop it.
#[test]
fn what_a_file_opened_at_launch_says_shows_when_the_window_comes_up() {
    let d = std::env::temp_dir().join(format!("figcraft-launch-board-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut s = Session::new();
    s.execute("shape.rectangle", &serde_json::json!({"width": 10, "height": 10})).unwrap();
    let mut v = figcraft_format::to_json(s.doc().unwrap()).unwrap();
    v["kind"] = serde_json::json!("board");
    let file = d.join("Plan.scrim");
    std::fs::write(&file, v.to_string()).unwrap();
    let mut s = Session::empty();
    open_files_with_results(&mut s, &[file.to_string_lossy().into_owned()]);
    let mut fig = figcraft_ui_egui::FigApp::new(s, Box::new(figcraft_ui_egui::NoServices));
    super::window_up(&mut fig);
    assert_eq!(fig.ui.board_file, Some(figcraft_ui_egui::board_file::Shown::Opened("Plan".into())));
    let _ = std::fs::remove_dir_all(&d);
}
