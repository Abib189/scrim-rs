//! Fix AI C-2 through the window: Quit is one command, `app.quit`. With nothing unsaved it quits at
//! once; with unsaved changes anywhere it asks first (Don't Save, Cancel, Save) and the process
//! stays until it is answered. Real OS key events and the native menu can't be driven from tests:
//! they are checked here as the command they run.

use egui::{RawInput, ViewportCommand, ViewportEvent, ViewportId, ViewportInfo};
use figcraft_engine::Session;
use serde_json::json;

use crate::{FigApp, NoServices};

/// One main-window frame, as the desktop app runs it (`close`: the window's close button). Returns
/// the commands it sent the main window.
fn frame(ctx: &egui::Context, app: &mut FigApp, close: bool) -> Vec<ViewportCommand> {
    let mut input = RawInput::default();
    if close {
        input.viewports.insert(ViewportId::ROOT, ViewportInfo { events: vec![ViewportEvent::Close], ..Default::default() });
    }
    let mut out = ctx.run_ui(input, |ui| {
        crate::windows::on_close_request(app, ui.ctx());
        app.ui(ui);
    });
    out.textures_delta.clear();
    out.viewport_output.remove(&ViewportId::ROOT).map(|v| v.commands).unwrap_or_default()
}

fn edited() -> Session {
    let mut s = Session::new();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    s
}

#[test]
fn quit_with_nothing_unsaved_quits_without_asking() {
    let ctx = egui::Context::default();
    let mut app = FigApp::new(Session::new(), Box::new(NoServices));
    frame(&ctx, &mut app, false);
    assert!(app.run("app.quit", json!({})).is_some(), "Quit is a command");
    let sent = frame(&ctx, &mut app, false);
    assert!(app.ui.quit, "it quits");
    assert!(!app.ui.confirm_close, "without asking");
    assert!(sent.contains(&ViewportCommand::Close), "{sent:?}");
}

#[test]
fn quit_with_unsaved_changes_asks_and_waits() {
    let ctx = egui::Context::default();
    let mut app = FigApp::new(edited(), Box::new(NoServices));
    frame(&ctx, &mut app, false);
    assert!(app.run("app.quit", json!({})).is_some(), "Quit is a command");
    let sent = frame(&ctx, &mut app, false);
    assert!(app.ui.confirm_close, "the question asks");
    assert!(!app.ui.quit && !sent.contains(&ViewportCommand::Close), "and the app stays: {sent:?}");
}

#[test]
fn quit_is_in_the_file_menu() {
    // ⌘Q is the system's, kept for the app menu's Quit Scrim (`apps/figcraft/src/app_menu.rs`),
    // which runs this command: no window shortcut takes it.
    assert_eq!(crate::shortcuts::shortcut("app.quit"), None);
    assert!(figcraft_engine::catalog::MENU.iter().any(|m| m.id == "app.quit" && m.menu == "File"), "File ▸ Quit Scrim");
}

/// An app asked to quit with `s`'s unsaved changes: the question shows. Save as picks `pick`.
fn asked(s: Session, pick: Option<std::path::PathBuf>) -> (egui::Context, FigApp) {
    let ctx = egui::Context::default();
    let mut app = FigApp::new(s, Box::new(Picker(pick)));
    frame(&ctx, &mut app, false);
    app.run("app.quit", json!({}));
    frame(&ctx, &mut app, false);
    assert!(app.ui.confirm_close && !app.ui.quit);
    (ctx, app)
}

/// Picks `path` when Save as asks where to save.
struct Picker(Option<std::path::PathBuf>);

impl crate::Services for Picker {
    fn pick_save(&mut self, _: &str, _: &str, _: &[&str]) -> crate::DialogFuture {
        Box::pin(std::future::ready(self.0.take()))
    }
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-ui-quit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn cancel_keeps_everything_and_dont_save_quits() {
    use crate::windows::quit::{CloseAnswer, answer};
    let (ctx, mut app) = asked(edited(), None);
    answer(&mut app, &ctx, CloseAnswer::Cancel);
    let sent = frame(&ctx, &mut app, false);
    assert!(!app.ui.quit && !app.ui.confirm_close && !sent.contains(&ViewportCommand::Close));
    assert!(app.session.ds().unwrap().is_dirty(), "the changes stay");
    // A window closed later asks about that window again, and doesn't quit.
    let (ctx, mut app) = asked(edited(), None);
    answer(&mut app, &ctx, CloseAnswer::DontSave);
    let sent = frame(&ctx, &mut app, false);
    assert!(app.ui.quit && sent.contains(&ViewportCommand::Close), "Don't Save quits: {sent:?}");
}

#[test]
fn save_saves_every_file_then_quits() {
    use crate::windows::quit::{CloseAnswer, answer};
    let d = dir("save");
    // One file saved before (saved in place) and one never saved (Save as asks where).
    let file = d.join("Kept.scrim");
    let mut s = edited();
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 20, "y": 0, "width": 10, "height": 10, "name": "Later"})).unwrap();
    s.execute("file.new", &json!({})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 10, "height": 10, "name": "New"})).unwrap();
    let picked = d.join("New.scrim");
    let (ctx, mut app) = asked(s, Some(picked.clone()));
    answer(&mut app, &ctx, CloseAnswer::Save);
    assert!(!app.ui.quit, "waits for Save as");
    app.poll_dialog(&ctx);
    let sent = frame(&ctx, &mut app, false);
    let has = |f: &std::path::Path, name: &str| figcraft_engine::cmd::file::load(&f.to_string_lossy()).unwrap().0.nodes().any(|n| n.name == name);
    assert!(has(&file, "Later"), "the saved file was saved in place");
    assert!(has(&picked, "New"), "the new file was saved where Save as said");
    assert!(app.ui.quit && sent.contains(&ViewportCommand::Close), "then it quits: {sent:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_cancelled_save_as_keeps_the_app_open() {
    use crate::windows::quit::{CloseAnswer, answer};
    let (ctx, mut app) = asked(edited(), None);
    answer(&mut app, &ctx, CloseAnswer::Save);
    app.poll_dialog(&ctx);
    let sent = frame(&ctx, &mut app, false);
    assert!(!app.ui.quit && !sent.contains(&ViewportCommand::Close), "{sent:?}");
    assert!(app.session.ds().unwrap().is_dirty());
}

#[test]
fn enter_answers_save_and_escape_cancels() {
    let d = dir("keys");
    let file = d.join("Doc.scrim");
    let mut s = edited();
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    s.execute("shape.rectangle", &json!({"x": 20, "y": 0, "width": 10, "height": 10})).unwrap();
    let key = |k| egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() };
    let (ctx, mut app) = asked(s, None);
    app.ui.home = false;
    let mut input = RawInput::default();
    input.events.push(key(egui::Key::Escape));
    ctx.run_ui(input, |ui| app.ui(ui)).textures_delta.clear();
    assert!(!app.ui.confirm_close && !app.ui.quit, "Escape cancels");
    app.run("app.quit", json!({}));
    frame(&ctx, &mut app, false);
    let mut input = RawInput::default();
    input.events.push(key(egui::Key::Enter));
    ctx.run_ui(input, |ui| app.ui(ui)).textures_delta.clear();
    assert!(app.ui.quit, "Enter saves and quits");
    assert!(!app.session.ds().unwrap().is_dirty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_agent_cannot_quit_the_app() {
    let ctx = egui::Context::default();
    let mut app = FigApp::new(Session::new(), Box::new(NoServices));
    let (reply, rx) = std::sync::mpsc::channel();
    app.control_queue.push(crate::control::ControlRequest {
        client: None,
        session: None,
        route: crate::control::ROUTE_CONTROL,
        id: json!(1),
        method: "engine.execute".into(),
        params: json!({"command": "app.quit"}),
        reply,
        cancelled: Default::default(),
    });
    // Drained as the app's logic does while the window is hidden (a frame drains it otherwise).
    let mut input = RawInput::default();
    input.viewports.insert(ViewportId::ROOT, ViewportInfo { minimized: Some(false), occluded: Some(true), ..Default::default() });
    let _ = ctx.run_logic(&input, |ctx| crate::control::drain_hidden(&mut app, ctx));
    assert!(rx.try_recv().is_ok(), "answered");
    let sent = frame(&ctx, &mut app, false);
    assert!(!app.ui.quit && !sent.contains(&ViewportCommand::Close), "nothing quit: {sent:?}");
}

#[test]
fn the_quit_words_are_in_english_and_spanish() {
    for key in [
        "Unsaved changes in {names}. Save them before quitting?",
        "Unsaved changes in {names}. Save them before closing?",
        "Don't Save",
        "Cancel",
        "Save",
    ] {
        assert!(crate::i18n::own_entry("en", key).is_some(), "{key} in English");
        assert!(crate::i18n::own_entry("es", key).is_some(), "{key} in Spanish");
    }
}
