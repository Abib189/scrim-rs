//! Files upstream FigCraft saved as boards (Fix AI C-1). Scrim has no boards: such a file opens as
//! a design (its connectors stop following what they join, stickies stop fitting their text), and
//! writing it leaves its board features out. So opening one says so, once, and nothing writes over
//! it unless the person chose that: Save asks (`dialog.saveBoard`: Save as a copy, Save anyway,
//! Cancel), `file.save {dropBoard: true}` is Save anyway, auto-save leaves it alone, and recovery
//! copies go where they always do, never over the file. Opening it writes nothing.

use serde_json::json;

use crate::{EngineError, Session};

/// After opening file `name`: the notice, when it is a board.
pub(super) fn say(s: &mut Session, name: &str, loaded: &figcraft_format::Loaded) {
    if let Some(b) = loaded.board {
        s.request_ui("dialog.boardFile", json!({"name": name, "items": b.items}));
    }
}

/// Saving over board file `name` was refused: the window shows the three choices.
pub(super) fn refused(s: &mut Session, name: &str) -> EngineError {
    s.request_ui("dialog.saveBoard", json!({"name": name}));
    EngineError::Disabled(
        "file.save".into(),
        format!(
            "{name} is a FigCraft board, opened as a design: saving over it removes its board features. Save a copy under a new .scrim name, or pass `dropBoard: true` to save over it anyway"
        ),
    )
}

#[cfg(test)]
#[path = "board_file_tests.rs"]
mod tests;
