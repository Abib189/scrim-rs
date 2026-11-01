//! Fix AG through the window: Home names the formats from the import registry, a drop takes every
//! registered extension, a Sketch import shows its report (a Scrim document shows none), and a file
//! Scrim can't open says what it is.

use std::path::Path;

use figcraft_engine::Session;
use figcraft_engine::importers::Unreadable;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text};

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../format/tests/fixtures/sketch/sample.sketch");

#[test]
fn home_names_the_formats_from_the_registry() {
    assert_eq!(crate::home::formats(), ".scrim, .fig, .sketch, .penpot, .free, .svg");
    let mut s = Session::empty();
    s.prefs.getting_started_dismissed = true;
    let mut u = Ui::new(s);
    u.app.ui.home = true;
    u.frame(vec![]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, "Open .scrim, .fig, .sketch, .penpot, .free, .svg or images"), "Home's Open file card");
    assert_eq!(crate::i18n::own_entry("es", "Open {formats} or images"), Some("Abre archivos {formats} o imágenes"));
}

#[test]
fn a_drop_takes_every_registered_extension() {
    for ok in ["a.scrim", "a.figcraft", "a.sketch", "a.fig", "a.json", "a.svg", "a.PNG", "a.webp"] {
        assert!(crate::opens(Path::new(ok)), "{ok}");
    }
    for no in ["a.txt", "a.psd", "a"] {
        assert!(!crate::opens(Path::new(no)), "{no}");
    }
}

#[test]
fn a_sketch_import_shows_its_report_and_a_scrim_document_shows_none() {
    let mut u = Ui::new(Session::empty());
    u.app.run_user("file.open", json!({"path": SAMPLE})).unwrap();
    let report = u.app.ui.import_report.as_ref().expect("the Sketch import's report");
    assert!(report.counts.iter().sum::<usize>() > 0);
    assert!(report.notes.iter().all(|n| matches!(n.severity.as_str(), "dropped" | "approximated" | "info")));
    let line = report.count_line();
    u.frame(vec![]);
    let shapes = flat(&u.frame_output(vec![]).shapes);
    assert!(has_text(&shapes, &line), "the counts show: {line}");
    assert!(has_text(&shapes, "Copy report"));
    // Saved as .scrim and opened again: a Scrim document, no report.
    let d = std::env::temp_dir().join(format!("figcraft-scrim-ui-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let file = d.join("sample.scrim");
    u.app.run("file.saveAs", json!({"path": file.to_string_lossy()})).unwrap();
    u.app.ui.import_report = None;
    let mut fresh = Ui::new(Session::empty());
    fresh.app.run_user("file.open", json!({"path": file.to_string_lossy()})).unwrap();
    assert!(fresh.app.ui.import_report.is_none());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_file_scrim_cannot_open_says_what_it_is_in_english_and_spanish() {
    let text = |u: Unreadable| crate::windows::error_text(&figcraft_engine::EngineError::Unreadable(u));
    assert!(text(Unreadable::OldScrimNewer(3)).contains("earlier Scrim app and uses its file version 3"));
    assert!(text(Unreadable::OldScrimDamaged("no nodes".into())).contains("damaged (no nodes)"));
    assert!(text(Unreadable::NotSupported).starts_with("Scrim can't open this file"));
    for key in [
        "This .scrim file is from the earlier Scrim app and uses its file version {v}, newer than this Scrim can import (up to {max}). Nothing was changed.",
        "This .scrim file is from the earlier Scrim app, and it is damaged ({why}). Nothing was changed.",
        "Scrim can't open this file: it isn't a Scrim document, an image, or a file from another design tool Scrim imports.",
        "Use the .scrim or .fig extension.",
        "Drop a {formats} or image file.",
    ] {
        assert!(crate::i18n::own_entry("es", key).is_some(), "{key} in Spanish");
    }
}

/// Fix AH4: Home's card for a Sketch file shows the preview it carries, read in the background like
/// the others and again only once the file changes; one without a preview keeps the placeholder.
#[test]
fn home_shows_a_sketch_files_preview_and_reads_it_again_only_when_it_changes() {
    let d = std::env::temp_dir().join(format!("figcraft-sketch-preview-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    // The committed sample carries `previews/preview.png`; a file that isn't a readable archive
    // has none.
    let with = d.join("poster.sketch");
    let without = d.join("bare.sketch");
    std::fs::copy(SAMPLE, &with).unwrap();
    std::fs::write(&without, b"no preview in here").unwrap();
    let mut s = Session::empty();
    s.prefs.recent_files = vec![with.to_string_lossy().into_owned(), without.to_string_lossy().into_owned()];
    s.prefs.getting_started_dismissed = true;
    let mut u = Ui::new(s);
    u.app.ui.home = true;
    let picture = |u: &Ui, name: &str| {
        let files = u.app.inspect()["homeRecents"]["files"].clone();
        files.as_array().unwrap().iter().find(|f| f["name"] == name).map(|f| f["picture"].as_str().unwrap_or("").to_string()).unwrap_or_default()
    };
    let settle = |u: &mut Ui| {
        for _ in 0..400 {
            u.frame(vec![]);
            if ["poster", "bare"].iter().all(|n| !matches!(picture(u, n).as_str(), "loading" | "none" | "")) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    settle(&mut u);
    assert_eq!(picture(&u, "poster"), "picture", "the preview Sketch carries");
    assert_eq!(picture(&u, "bare"), "placeholder", "none carried: the placeholder");
    let asked = u.app.ui.home_view.recents.previews_asked;
    assert_eq!(asked, 2);
    for _ in 0..60 {
        u.frame(vec![]);
    }
    assert_eq!(u.app.ui.home_view.recents.previews_asked, asked, "an unchanged file isn't read again");
    // Changed (written again, a later modification time): read again.
    std::fs::copy(SAMPLE, &with).unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
    std::fs::File::options().write(true).open(&with).unwrap().set_modified(later).unwrap();
    u.app.ui.home = false;
    u.frame(vec![]);
    u.frame(vec![]);
    u.app.ui.home = true;
    // The reader is a thread of its own: wait for it, a frame at a time.
    for _ in 0..400 {
        u.frame(vec![]);
        if u.app.ui.home_view.recents.previews_asked > asked {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    for _ in 0..30 {
        u.frame(vec![]);
    }
    assert_eq!(u.app.ui.home_view.recents.previews_asked, asked + 1, "the changed file only");
    std::fs::remove_dir_all(&d).ok();
}
