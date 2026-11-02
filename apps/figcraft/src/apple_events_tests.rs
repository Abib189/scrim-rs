//! Documents macOS hands over in an open-documents Apple event (a Finder double-click, Open With,
//! a drop on the Dock icon) open as File › Open opens them (#289).

use std::path::PathBuf;

use figcraft_engine::Session;
use figcraft_ui_egui::{FigApp, NoServices};
use serde_json::json;

use crate::apple_events::open_documents;

#[test]
fn documents_from_finder_open_and_leave_the_home_screen() {
    let dir = std::env::temp_dir().join(format!("figcraft-apple-events-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("BLA-001.scrim");
    let mut s = Session::new();
    s.execute("file.saveAs", &json!({"path": file.to_string_lossy()})).unwrap();

    let mut app = FigApp::new(Session::empty(), Box::new(NoServices));
    app.ui.home = true;
    // A file that can't be read is reported and doesn't keep the others from opening.
    open_documents(&mut app, vec![dir.join("Missing.scrim"), file.clone()]);
    let paths: Vec<_> = app.session.docs.iter().map(|d| d.path.clone().map(PathBuf::from)).collect();
    assert_eq!(paths, [Some(file)]);
    assert!(!app.ui.home, "the opened document shows");
    assert!(app.ui.last_error.as_deref().is_some_and(|e| e.contains("Missing.scrim")), "{:?}", app.ui.last_error);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fix AI C-2: the Dock's Quit and logging out arrive as the quit Apple event, which runs the same
/// `app.quit` as the app menu: it quits at once with nothing unsaved, and asks otherwise.
#[test]
fn the_quit_event_quits_through_the_command_and_asks_about_unsaved_changes() {
    let frame = |ctx: &egui::Context, app: &mut FigApp| {
        ctx.run_ui(egui::RawInput::default(), |ui| figcraft_ui_egui::windows::on_close_request(app, ui.ctx())).textures_delta.clear();
    };
    let ctx = egui::Context::default();
    let mut app = FigApp::new(Session::new(), Box::new(NoServices));
    crate::apple_events::quit(&mut app);
    frame(&ctx, &mut app);
    assert!(app.ui.quit && !app.ui.confirm_close, "nothing unsaved: it quits");

    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"width": 10, "height": 10})).unwrap();
    let mut app = FigApp::new(s, Box::new(NoServices));
    crate::apple_events::quit(&mut app);
    frame(&ctx, &mut app);
    assert!(!app.ui.quit && app.ui.confirm_close, "unsaved changes: it asks first");
}
