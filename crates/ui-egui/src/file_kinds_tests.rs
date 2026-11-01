//! Fix AH3 through the window: a closed-format file (Affinity, Adobe XD) dropped on Home or on the
//! canvas, or opened, shows a short dialog saying Scrim can't read it and to export SVG, and is
//! never opened. Its text comes from the engine's one list.

use egui::{Event, Key, Modifiers};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{flat, has_text};

#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

fn drop_file(u: &mut Ui, path: &std::path::Path) {
    let file: egui::DroppedFileHandle = std::sync::Arc::new(Dropped(path.to_path_buf()));
    u.frame_raw(egui::RawInput { dropped_files: vec![file], ..Default::default() });
}

fn file(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-closed-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join(name);
    std::fs::write(&p, b"a closed format's bytes").unwrap();
    p
}

fn shown(u: &mut Ui) -> Vec<egui::Shape> {
    u.frame(vec![]);
    flat(&u.frame_output(vec![]).shapes)
}

#[test]
fn a_closed_file_dropped_on_home_shows_the_dialog_and_opens_nothing() {
    let path = file("logo.afdesign");
    let mut u = Ui::new(Session::empty());
    u.app.ui.home = true;
    u.frame(vec![]);
    drop_file(&mut u, &path);
    assert_eq!(u.app.ui.closed_format, Some("Affinity"));
    assert!(u.app.session.docs.is_empty(), "never opened");
    assert!(u.app.ui.toast.is_none(), "a dialog, not a toast");
    let shapes = shown(&mut u);
    assert!(has_text(&shapes, "Can't open this file"));
    assert!(has_text(&shapes, "Affinity files use a closed format, so Scrim can't read them."));
    assert!(has_text(&shapes, "In Affinity, export the design as SVG, then open the SVG here."));
    // Escape closes it, like the other simple dialogs.
    u.frame(vec![Event::Key { key: Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }]);
    assert_eq!(u.app.ui.closed_format, None);
}

#[test]
fn a_closed_file_opened_or_dropped_on_the_canvas_shows_the_dialog_too() {
    let path = file("app.xd");
    // Opened (File ▸ Open, a recent file, Finder): the engine refuses it, the window explains.
    let mut u = Ui::new(Session::new());
    let layers = u.app.session.doc().unwrap().nodes().count();
    assert!(u.app.run_user("file.open", json!({"path": path.to_string_lossy()})).is_none());
    assert_eq!(u.app.ui.closed_format, Some("Adobe XD"));
    assert!(u.app.ui.toast.is_none());
    assert_eq!(u.app.session.docs.len(), 1);
    // Dropped on the canvas of an open file: nothing is imported.
    u.app.ui.closed_format = None;
    u.app.ui.home = false;
    u.frame(vec![]);
    drop_file(&mut u, &path);
    for _ in 0..6 {
        u.frame(vec![]);
    }
    assert_eq!(u.app.ui.closed_format, Some("Adobe XD"));
    assert_eq!(u.app.session.doc().unwrap().nodes().count(), layers, "nothing imported");
    assert!(has_text(&shown(&mut u), "Adobe XD files use a closed format, so Scrim can't read them."));
}

#[test]
fn the_dialogs_text_comes_from_one_list_and_is_in_spanish() {
    for c in figcraft_engine::importers::CLOSED {
        for e in c.extensions {
            assert_eq!(crate::file_kinds::closed(std::path::Path::new(&format!("a.{e}"))), Some(c.tool));
            assert!(!crate::opens(std::path::Path::new(&format!("a.{e}"))));
        }
    }
    for key in [
        "Can't open this file",
        "{tool} files use a closed format, so Scrim can't read them.",
        "In {tool}, export the design as SVG, then open the SVG here.",
        "{tool} files use a closed format, so Scrim can't read them. In {tool}, export the design as SVG, then open the SVG here.",
        "This {tool} file uses version {version} of its format, newer than Scrim can import. Nothing was changed.",
    ] {
        assert!(crate::i18n::own_entry("es", key).is_some(), "{key} in Spanish");
    }
}
