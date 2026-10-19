//! The settings migration on a temporary home: a fresh install, an upgrade with FigCraft's
//! settings, and both directories present.

use std::path::{Path, PathBuf};

use super::migrate::{Migration, NEW, OLD, migrate};

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("scrim-migrate-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(p: &Path, s: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, s).unwrap();
}

#[test]
fn a_fresh_install_has_nothing_to_copy() {
    let base = temp("fresh");
    assert_eq!(migrate(&base), Migration::Fresh);
    assert!(!base.join(NEW).exists(), "the app makes its directory when it first saves");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn an_upgrade_copies_figcrafts_settings_and_leaves_them_in_place() {
    let base = temp("upgrade");
    let prefs = r#"{"prefs":{"theme":"dark","recent_files":["/tmp/a.figcraft"]}}"#;
    write(&base.join(OLD).join("state.json"), prefs);
    write(&base.join(OLD).join("Recovery/doc-1.figcraft"), "copy");
    write(&base.join(OLD).join("Plugins/tidy/figcraft-plugin.json"), "{}");
    assert_eq!(migrate(&base), Migration::Copied { files: 3 });
    assert_eq!(std::fs::read_to_string(base.join(NEW).join("state.json")).unwrap(), prefs);
    assert!(base.join(NEW).join("Recovery/doc-1.figcraft").exists() && base.join(NEW).join("Plugins/tidy/figcraft-plugin.json").exists());
    // FigCraft's directory is untouched, and no temporary copy is left behind.
    assert_eq!(std::fs::read_to_string(base.join(OLD).join("state.json")).unwrap(), prefs);
    let names: Vec<String> = std::fs::read_dir(&base).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    // Only once: a second start finds Scrim's directory.
    assert_eq!(migrate(&base), Migration::AlreadyThere);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn with_both_directories_scrims_wins_and_nothing_is_copied() {
    let base = temp("both");
    write(&base.join(OLD).join("state.json"), "figcraft");
    write(&base.join(NEW).join("state.json"), "scrim");
    assert_eq!(migrate(&base), Migration::AlreadyThere);
    assert_eq!(std::fs::read_to_string(base.join(NEW).join("state.json")).unwrap(), "scrim");
    assert_eq!(std::fs::read_to_string(base.join(OLD).join("state.json")).unwrap(), "figcraft");
    let _ = std::fs::remove_dir_all(&base);
}

/// The window's app id is the one the packages install (`docs/design/rebrand.md`, group 2).
#[test]
fn the_window_app_id_matches_the_packaging() {
    let main = include_str!("main.rs");
    assert!(main.contains(r#".with_app_id("design.scrim.app")"#));
    let desktop = include_str!("../../../packaging/linux/design.scrim.app.desktop");
    assert!(desktop.contains("Name=Scrim") && desktop.contains("StartupWMClass=design.scrim.app") && desktop.contains("Icon=design.scrim.app"));
    let plist = include_str!("../../../packaging/macos/Info.plist.in");
    assert!(plist.contains("<string>design.scrim.app</string>") && plist.contains("<string>Scrim</string>"));
}
