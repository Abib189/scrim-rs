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
    assert!(params["path"].as_str().is_some_and(|p| p.ends_with("sample.sketch")));
    assert!(result["warnings"].as_array().is_some_and(|w| !w.is_empty()), "{result}");
}
