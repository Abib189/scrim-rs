//! When Save finds that another app changed the file since it was opened or saved here (Fix AI
//! LW-4): three choices, each a command, instead of replacing those changes silently.
//! - **Save as a copy** (the default, Enter): `file.saveAs`, which asks where; the other app's file
//!   stays as it is.
//! - **Overwrite**: `file.save {overwrite: true}`, replacing what the other app wrote.
//! - **Reload from disk**: `file.revert`, loading the other app's file in place of this tab's edits.
//!
//! Escape or closing the window cancels: nothing is saved, and the edits stay unsaved.

use serde_json::json;

use crate::FigApp;
use crate::i18n::{tr, trf};

/// One of the three choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    SaveCopy,
    Overwrite,
    Reload,
}

/// Act on `choice` for the file the question is about, through its command.
pub fn choose(app: &mut FigApp, choice: Choice) {
    app.ui.save_conflict = None;
    match choice {
        Choice::SaveCopy => app.run("file.saveAs", json!({})),
        Choice::Overwrite => app.run("file.save", json!({"overwrite": true})),
        Choice::Reload => app.run("file.revert", json!({})),
    };
}

/// The question, while there is one.
pub fn dialog(app: &mut FigApp, ctx: &egui::Context) {
    let Some(name) = app.ui.save_conflict.clone() else { return };
    let t = crate::theme::Tokens::get(ctx);
    let mut open = true;
    let mut chosen = None;
    crate::dialogs::window(tr("File changed on disk"), ctx).order(egui::Order::Foreground).open(&mut open).show(ctx, |ui| {
        ui.set_width(380.0);
        crate::widgets::label(
            ui,
            &trf("Another app changed {name} since you opened or last saved it here. Saving now would replace those changes.", &[("name", &name)]),
            t.text,
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if crate::widgets::button(ui, tr("Save as a copy…"), true, true).clicked() {
                chosen = Some(Choice::SaveCopy);
            }
            if crate::widgets::button(ui, tr("Overwrite"), false, true).clicked() {
                chosen = Some(Choice::Overwrite);
            }
            if crate::widgets::button(ui, tr("Reload from disk"), false, true).clicked() {
                chosen = Some(Choice::Reload);
            }
        });
    });
    // Enter takes the default: a copy, which loses nothing.
    if chosen.is_none() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        chosen = Some(Choice::SaveCopy);
    }
    match chosen {
        Some(c) => choose(app, c),
        None if !open => app.ui.save_conflict = None,
        None => {}
    }
}
