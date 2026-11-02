//! FigCraft board files (Fix AI C-1). Scrim has no boards, so a file upstream FigCraft saved as one
//! opens as a design; the engine asks the window to say so, and to ask before writing over it.
//! - **Opened** (`dialog.boardFile`, once per open): what that means, with **Close** (the default,
//!   Enter) and **Save a copy as .scrim…** (`file.saveAs`).
//! - **Saving** (`dialog.saveBoard`, Save or Save as onto the board file): **Save as a copy…** (the
//!   default, Enter: `file.saveAs`, which asks where), **Save anyway** (`file.save {dropBoard:
//!   true}`, writing it without its board features) and **Cancel**.
//!
//! Escape or closing the window is Close or Cancel: nothing is saved.

use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, trf};

/// Which question shows, for which file (its name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    Opened(String),
    Saving(String),
}

impl Shown {
    pub fn inspect(&self) -> Value {
        match self {
            Shown::Opened(name) => json!({"kind": "opened", "name": name}),
            Shown::Saving(name) => json!({"kind": "saving", "name": name}),
        }
    }
}

/// What the person chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Close the notice, or cancel the save.
    Close,
    SaveCopy,
    SaveAnyway,
}

/// Act on `choice`, through its command.
pub fn choose(app: &mut FigApp, choice: Choice) {
    app.ui.board_file = None;
    match choice {
        Choice::Close => {}
        Choice::SaveCopy => {
            app.run("file.saveAs", json!({}));
        }
        Choice::SaveAnyway => {
            app.run("file.save", json!({"dropBoard": true}));
        }
    }
}

/// The question, while there is one.
pub fn dialog(app: &mut FigApp, ctx: &egui::Context) {
    let Some(shown) = app.ui.board_file.clone() else { return };
    let t = crate::theme::Tokens::get(ctx);
    let mut open = true;
    let mut chosen = None;
    let (title, body, default) = match &shown {
        Shown::Opened(name) => (
            tr("Board file"),
            trf(
                "{name} is a board, a kind of whiteboard file Scrim doesn't support. It is shown as a normal design: connectors no longer follow what they join, and saving it removes its board features.",
                &[("name", name)],
            ),
            Choice::Close,
        ),
        Shown::Saving(name) => (
            tr("Save over a board?"),
            trf("{name} is a board. Saving over it removes its board features. Save a copy to keep the board file as it is.", &[("name", name)]),
            Choice::SaveCopy,
        ),
    };
    crate::dialogs::window(title, ctx).order(egui::Order::Foreground).open(&mut open).show(ctx, |ui| {
        ui.set_width(400.0);
        crate::widgets::label(ui, &body, t.text);
        ui.add_space(12.0);
        ui.horizontal(|ui| match &shown {
            Shown::Opened(_) => {
                if crate::widgets::button(ui, tr("Close"), true, true).clicked() {
                    chosen = Some(Choice::Close);
                }
                if crate::widgets::button(ui, tr("Save a copy as .scrim…"), false, true).clicked() {
                    chosen = Some(Choice::SaveCopy);
                }
            }
            Shown::Saving(_) => {
                if crate::widgets::button(ui, tr("Save as a copy…"), true, true).clicked() {
                    chosen = Some(Choice::SaveCopy);
                }
                if crate::widgets::button(ui, tr("Save anyway"), false, true).clicked() {
                    chosen = Some(Choice::SaveAnyway);
                }
                if crate::widgets::button(ui, tr("Cancel"), false, true).clicked() {
                    chosen = Some(Choice::Close);
                }
            }
        });
    });
    // Enter takes the default, which loses nothing.
    if chosen.is_none() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        chosen = Some(default);
    }
    match chosen {
        Some(c) => choose(app, c),
        None if !open => app.ui.board_file = None,
        None => {}
    }
}

#[cfg(test)]
#[path = "board_file_ui_tests.rs"]
mod tests;
