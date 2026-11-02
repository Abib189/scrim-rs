//! Quitting, and the answers to the unsaved-changes question (Fix AI C-2; upstream FigCraft
//! 73398ced, 0e508fd3 and 83dac86b, ported by hand).
//!
//! Every way to quit is one command, `app.quit`: the app menu's Quit Scrim and ⌘Q (the desktop
//! app's own menu: winit's ended the process through AppKit's `terminate:` without asking), the
//! Dock's Quit and logging out (the quit Apple event), and File ▸ Quit Scrim. The command asks the
//! window (`dialog.quit`, which an agent's call drops: only the person quits the app). The main
//! window then quits at once when no file has unsaved changes, and otherwise asks about all of
//! them, in every window:
//! - **Don't Save** quits;
//! - **Cancel** keeps everything open;
//! - **Save** (the default, Enter) saves each file, with Save as for one without a .scrim path,
//!   and quits once all are saved. A save that fails, or a Save as cancelled, keeps the app open
//!   with its changes.
//!
//! Closing a window asks the same question about that window's files, and closes it.
//!
//! Logging out or restarting sends the same quit event. With unsaved changes the question shows
//! and the app doesn't quit until it is answered, so macOS ends the logout after its own time
//! limit and names the app that stopped it: nothing is lost and nothing waits forever (recovery
//! copies cover the files meanwhile).

use egui::{Context, ViewportCommand, ViewportId};
use serde_json::json;

use crate::FigApp;

/// Where a quit stands.
#[derive(Debug, Default)]
pub struct Quit {
    /// `app.quit` ran: the main window acts on it at its next logic pass.
    pub(crate) asked: bool,
    /// The unsaved-changes question showing is for quitting, not closing one window.
    pub(crate) question: bool,
    /// The Save that answered it waits for Save as on this file.
    pub(crate) after_save_as: Option<u64>,
}

/// An answer to the unsaved-changes question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseAnswer {
    /// Save every file it is about, then close or quit.
    Save,
    /// Close or quit, losing the changes.
    DontSave,
    /// Keep everything open.
    Cancel,
}

/// From the main window, every logic pass (minimised too): act on a quit asked for.
pub(crate) fn poll(app: &mut FigApp, ctx: &Context) {
    if !std::mem::take(&mut app.windows.quit.asked) || app.ui.quit {
        return;
    }
    if unsaved_anywhere(app).is_empty() {
        now(app, ctx);
    } else {
        app.windows.quit.question = true;
        app.ui.confirm_close = true;
        ctx.request_repaint();
    }
}

/// Files with unsaved changes in every window (a branch tab's edits live on in its file).
fn unsaved_anywhere(app: &FigApp) -> Vec<String> {
    app.session.docs.iter().filter(|d| d.is_dirty() && d.branch_of.is_none()).map(crate::doc_title::of).collect()
}

/// What the question is about: every window's files when quitting, else this window's.
pub(crate) fn names(app: &FigApp) -> Vec<String> {
    if app.windows.quit.question { unsaved_anywhere(app) } else { app.unsaved() }
}

/// Quit: the main window closes, and nothing asks again.
fn now(app: &mut FigApp, ctx: &Context) {
    app.windows.quit = Quit::default();
    app.ui.quit = true;
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
}

/// Act on the question's answer.
pub fn answer(app: &mut FigApp, ctx: &Context, answer: CloseAnswer) {
    app.ui.confirm_close = false;
    match answer {
        CloseAnswer::Cancel => app.windows.quit.question = false,
        CloseAnswer::DontSave => done(app, ctx),
        CloseAnswer::Save => save_then_close(app, ctx),
    }
}

/// Quit, or close the window the question was about.
fn done(app: &mut FigApp, ctx: &Context) {
    if app.windows.quit.question {
        now(app, ctx);
    } else {
        crate::windows::close_current(app, ctx);
    }
}

/// Save each file the question is about, then close or quit. A file without a .scrim path asks
/// where with Save as, and this goes on once that saved it ([`after_save_as`]). A save that fails
/// shows why and keeps everything open.
fn save_then_close(app: &mut FigApp, ctx: &Context) {
    let quitting = app.windows.quit.question;
    let files: Vec<u64> =
        app.session.docs.iter().filter(|d| d.is_dirty() && d.branch_of.is_none() && (quitting || app.shows(d.uid))).map(|d| d.uid).collect();
    for uid in files {
        let Some(i) = app.session.docs.iter().position(|d| d.uid == uid) else { continue };
        app.session.active = i;
        let in_place = app.session.docs.get(i).and_then(|d| d.path.as_deref()).is_some_and(figcraft_engine::cmd::file::saves_in_place);
        if !in_place {
            app.windows.quit.after_save_as = Some(uid);
            app.save_as_dialog();
            if app.pending_dialog.is_none() {
                // No dialog could start: stay open.
                app.windows.quit.after_save_as = None;
                app.windows.quit.question = false;
            }
            ctx.request_repaint();
            return;
        }
        if app.run("file.save", json!({})).is_none() {
            app.windows.quit.question = false;
            return;
        }
    }
    done(app, ctx);
}

/// While a Save as started by Save is open: true (the question stays closed). Once it is
/// answered, saving goes on if it saved the file; cancelled or failed, everything stays open.
pub(crate) fn after_save_as(app: &mut FigApp, ctx: &Context) -> bool {
    let Some(uid) = app.windows.quit.after_save_as else { return false };
    if app.pending_dialog.is_some() {
        return true;
    }
    app.windows.quit.after_save_as = None;
    if app.session.docs.iter().find(|d| d.uid == uid).is_none_or(|d| !d.is_dirty()) {
        save_then_close(app, ctx);
    } else {
        app.windows.quit.question = false;
    }
    true
}
