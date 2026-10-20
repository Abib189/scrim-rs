use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::import_report::Report;
use crate::layers_drag_tests::Ui;

/// A saved file with text in `families` (the first two layers share the first family).
fn saved(name: &str, families: &[&str]) -> String {
    let mut s = Session::new();
    for (i, f) in families.iter().chain(families.first()).enumerate() {
        s.execute("text.create", &json!({"text": "Hamburgefonstiv", "x": 0, "y": 40 * i, "props": {"fontFamily": f}})).unwrap();
    }
    let path = std::env::temp_dir().join(format!("figcraft-missing-fonts-{name}-{}.figcraft", std::process::id()));
    let path = path.to_string_lossy().into_owned();
    s.execute("file.saveAs", &json!({"path": path})).unwrap();
    path
}

/// The installed fonts are indexed, as the desktop app does at startup.
fn fonts_indexed() -> bool {
    figcraft_text::scan_system_fonts();
    figcraft_text::installed_fonts_known()
}

fn listed(u: &Ui) -> Value {
    u.app.inspect()["missingFonts"].clone()
}

#[test]
fn opening_a_file_lists_its_missing_fonts_and_not_the_installed_ones() {
    if !fonts_indexed() {
        eprintln!("skipped: system fonts opted out");
        return;
    }
    let installed = figcraft_text::family_names().into_iter().next();
    let mut families = vec!["FigCraft Missing UI"];
    families.extend(installed.as_deref());
    let path = saved("open", &families);
    let mut u = Ui::new(Session::new());
    // Programmatic opens (control channel, MCP, scripts) never open the window.
    u.app.run("file.open", json!({"path": path}));
    u.frame(vec![]);
    assert_eq!(listed(&u), Value::Null);
    u.app.run("file.close", json!({}));

    u.app.run_user("file.open", json!({"path": path}));
    std::fs::remove_file(&path).unwrap();
    u.frame(vec![]);
    assert_eq!(listed(&u), json!({"families": [{"family": "FigCraft Missing UI", "layers": 2}], "indexing": false}));
    u.frame(vec![]);
    let shown = u.frame_output(vec![]).shapes;
    assert!(crate::import_report_tests::shown_texts(shown).iter().any(|t| t == "2 text layers"));

    // It goes with its file.
    u.app.run("file.close", json!({}));
    u.frame(vec![]);
    assert_eq!(listed(&u), Value::Null);
}

#[test]
fn a_file_without_missing_fonts_opens_no_window() {
    if !fonts_indexed() {
        eprintln!("skipped: system fonts opted out");
        return;
    }
    let Some(installed) = figcraft_text::family_names().into_iter().next() else {
        eprintln!("skipped: no fonts on this machine");
        return;
    };
    let path = saved("clean", &[&installed]);
    let mut u = Ui::new(Session::new());
    u.app.run_user("file.open", json!({"path": path}));
    std::fs::remove_file(&path).unwrap();
    u.frame(vec![]);
    assert_eq!(listed(&u), Value::Null);
    assert!(u.app.ui.font_checks.is_empty(), "the check ran");
}

#[test]
fn it_waits_for_the_import_report_to_close() {
    if !fonts_indexed() {
        eprintln!("skipped: system fonts opted out");
        return;
    }
    let path = saved("report", &["FigCraft Missing Report"]);
    let mut u = Ui::new(Session::new());
    u.app.run_user("file.open", json!({"path": path}));
    std::fs::remove_file(&path).unwrap();
    let mut report =
        Report::from_result("file.open", &json!({}), &json!({"warnings": ["Synthetic note"], "summary": {"name": "Synthetic"}})).unwrap();
    report.uid = u.app.session.ds().unwrap().uid;
    u.app.ui.import_report = Some(report);
    u.frame(vec![]);
    assert_eq!(listed(&u), Value::Null, "the import report comes first");
    u.app.ui.import_report = None;
    u.frame(vec![]);
    assert_eq!(listed(&u)["families"][0]["family"], "FigCraft Missing Report");
}

#[test]
fn text_menu_shows_the_window_for_the_file_in_front_even_with_nothing_missing() {
    assert!(fonts_indexed());
    let mut u = Ui::new(Session::new());
    assert!(u.app.run("text.missingFonts", json!({})).is_some());
    assert_eq!(listed(&u), json!({"families": [], "indexing": false}));
    u.frame(vec![]);
    let shown = u.frame_output(vec![]).shapes;
    assert!(crate::import_report_tests::shown_texts(shown).iter().any(|t| t == "All the fonts this file uses are installed"));
    let escape =
        vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }];
    u.frame(escape);
    assert_eq!(listed(&u), Value::Null, "Escape closes it");
}

#[test]
fn asked_for_before_the_fonts_are_indexed_it_lists_them_once_they_are() {
    assert!(fonts_indexed());
    let path = saved("indexing", &["FigCraft Missing Indexing"]);
    let mut u = Ui::new(Session::new());
    u.app.run("file.open", json!({"path": path}));
    std::fs::remove_file(&path).unwrap();
    // As Text ▸ Missing fonts leaves it while the startup scan is still running.
    let uid = u.app.session.ds().unwrap().uid;
    u.app.ui.missing_fonts = Some(crate::missing_fonts::Shown { uid, families: Vec::new(), indexing: true });
    u.frame(vec![]);
    assert_eq!(listed(&u), json!({"families": [{"family": "FigCraft Missing Indexing", "layers": 2}], "indexing": false}));
}

#[test]
fn placing_images_or_clearing_recents_does_not_bring_it_back() {
    assert!(fonts_indexed());
    let path = saved("place", &["FigCraft Missing Place"]);
    let svg = std::env::temp_dir().join(format!("figcraft-missing-fonts-place-{}.svg", std::process::id()));
    std::fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#).unwrap();
    let mut u = Ui::new(Session::new());
    u.app.run_user("file.open", json!({"path": path}));
    u.frame(vec![]);
    assert_eq!(listed(&u)["families"][0]["family"], "FigCraft Missing Place");
    u.app.ui.missing_fonts = None;
    // A placed SVG, the same file opened again (it only comes to the front), Clear recent.
    assert!(u.app.run_user("file.import", json!({"path": svg.to_string_lossy()})).is_some());
    assert!(u.app.run_user("file.open", json!({"path": path})).is_some());
    u.app.run_user("file.openRecent", json!({"clear": true}));
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&svg).unwrap();
    u.frame(vec![]);
    assert!(u.app.ui.font_checks.is_empty());
    assert_eq!(listed(&u), Value::Null);
}

/// The clipboard, kept where the test can read it.
struct Clipboard(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl crate::Services for Clipboard {
    fn clipboard_set_text(&mut self, s: &str) {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(s.to_string());
    }
}

/// Inter, Source Serif 4 and Geist are bundled (`assets/fonts/`): a file using only them opens
/// without the window, and a file using them with a missing one lists only that one.
#[test]
fn bundled_fonts_are_never_missing() {
    if !fonts_indexed() {
        eprintln!("skipped: system fonts opted out");
        return;
    }
    let bundled = saved("bundled", &["Inter", "Source Serif 4", "Geist"]);
    let mut u = Ui::new(Session::new());
    u.app.run_user("file.open", json!({"path": bundled}));
    u.frame(vec![]);
    u.frame(vec![]);
    assert_eq!(listed(&u), Value::Null, "no window for the bundled families");
    u.app.run("file.close", json!({}));
    let mixed = saved("mixed", &["Scrim Missing Face", "Inter", "Geist"]);
    u.app.run_user("file.open", json!({"path": mixed}));
    u.frame(vec![]);
    assert_eq!(listed(&u)["families"], json!([{"family": "Scrim Missing Face", "layers": 2}]));
    for p in [bundled, mixed] {
        std::fs::remove_file(p).unwrap();
    }
}

#[test]
fn dont_show_again_is_remembered_for_that_file_only() {
    if !fonts_indexed() {
        eprintln!("skipped: system fonts opted out");
        return;
    }
    let (a, b) = (saved("mute-a", &["Scrim Missing Face"]), saved("mute-b", &["Scrim Missing Face"]));
    let mut u = Ui::new(Session::new());
    u.app.run_user("file.open", json!({"path": a}));
    u.frame(vec![]);
    assert!(listed(&u).is_object(), "it opens the first time");
    assert_eq!(u.app.run("fonts.muteMissing", json!({})), Some(json!({"path": a, "muted": true})));
    u.app.run("file.close", json!({}));
    u.frame(vec![]);
    u.app.run_user("file.open", json!({"path": a}));
    u.frame(vec![]);
    u.frame(vec![]);
    assert_eq!(listed(&u), Value::Null, "not again for this file");
    // Text ▸ Missing fonts still shows it.
    u.app.run("text.missingFonts", json!({}));
    u.frame(vec![]);
    assert!(listed(&u).is_object());
    u.app.ui.missing_fonts = None;
    u.app.run("file.close", json!({}));
    u.app.run_user("file.open", json!({"path": b}));
    u.frame(vec![]);
    assert!(listed(&u).is_object(), "another file still shows it");
    let saved_prefs = serde_json::to_value(&u.app.session.prefs).unwrap();
    assert_eq!(saved_prefs["fonts_muted"], json!([a]), "kept in the preferences");
    for p in [a, b] {
        std::fs::remove_file(p).unwrap();
    }
}

#[test]
fn copy_font_names_copies_the_missing_families() {
    let mut s = Session::new();
    for f in ["Scrim Missing One", "Inter", "Scrim Missing Two"] {
        s.execute("text.create", &json!({"text": "Aa", "x": 0, "y": 0, "props": {"fontFamily": f}})).unwrap();
    }
    let mut u = Ui::new(s);
    let copied = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    u.app.services = Box::new(Clipboard(copied.clone()));
    let r = u.app.run("fonts.copyMissingNames", json!({})).unwrap();
    let want = ["Scrim Missing One", "Scrim Missing Two"];
    assert_eq!(r["families"], json!(want));
    assert_eq!(copied.lock().unwrap().last().map(String::as_str), Some(want.join("\n").as_str()));
}
