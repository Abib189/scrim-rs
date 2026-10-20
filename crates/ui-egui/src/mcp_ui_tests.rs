//! C1.4: agents' file tools default to ~/Documents/Scrim (made on first use), never the home folder;
//! a folder that isn't there confines to nothing; a chosen folder is never changed.

use std::path::{Path, PathBuf};

use figcraft_engine::Session;
use figcraft_engine::file_access::Access;
use figcraft_engine::settings::McpSettings;
use serde_json::json;

use super::{ensure_root_in, roots_for};
use crate::layers_drag_tests::Ui;

/// A scratch "home folder" of its own.
fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("scrim-c14-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir).unwrap()
}

fn allowed(roots: &figcraft_engine::file_access::Roots, path: &Path) -> bool {
    roots.check(path, Access::Write).is_ok()
}

#[test]
fn the_default_folder_is_documents_scrim_and_never_the_home_folder() {
    assert!(figcraft_engine::settings::default_mcp_root().is_some_and(|p| p.ends_with("Documents/Scrim")));
    let h = home("default");
    let default = h.join("Documents").join("Scrim");
    let roots = roots_for(&McpSettings::default(), Some(default.clone())).unwrap();
    assert!(!allowed(&roots, &h.join("notes.txt")), "not the home folder");
    assert!(!allowed(&roots, &default.join("a.png")), "nothing at all until the folder exists");
    assert!(!default.exists(), "not made before it is used");
    let _ = std::fs::remove_dir_all(h);
}

#[test]
fn the_folder_is_made_the_first_time_a_request_names_a_file() {
    let h = home("lazy");
    let default = h.join("Documents").join("Scrim");
    let mut u = Ui::new(Session::new());
    u.app.session.agents.mcp.roots = roots_for(&u.app.session.prefs.settings.mcp, Some(default.clone()));
    ensure_root_in(&mut u.app, &json!({"command": "select.all"}), Some(default.clone()));
    assert!(!default.exists(), "a request without a file makes nothing");
    ensure_root_in(
        &mut u.app,
        &json!({"command": "file.export", "params": {"path": default.join("a.png").to_string_lossy()}}),
        Some(default.clone()),
    );
    assert!(default.is_dir(), "made on first use");
    let roots = u.app.session.agents.mcp.roots.clone().unwrap();
    assert!(allowed(&roots, &default.join("a.png")), "and file tools may use it now");
    assert!(!allowed(&roots, &h.join("a.png")), "but not the home folder");
    let _ = std::fs::remove_dir_all(h);
}

#[test]
fn a_chosen_folder_is_never_changed_or_made() {
    let h = home("chosen");
    let chosen = h.join("Mine");
    let default = h.join("Documents").join("Scrim");
    let mut u = Ui::new(Session::new());
    u.app.session.prefs.settings.mcp.root_dir = Some(chosen.to_string_lossy().into_owned());
    ensure_root_in(&mut u.app, &json!({"params": {"path": "/tmp/x.png"}}), Some(default.clone()));
    assert!(!default.exists() && !chosen.exists(), "nothing made");
    assert_eq!(u.app.session.prefs.settings.mcp.root_dir.as_deref(), Some(chosen.to_string_lossy().as_ref()), "unchanged");
    let roots = roots_for(&u.app.session.prefs.settings.mcp, Some(default)).unwrap();
    assert!(!allowed(&roots, &h.join("a.png")), "a chosen folder that isn't there confines to nothing");
    let _ = std::fs::remove_dir_all(h);
}
