//! Which files the window takes (Fix AG), and the closed formats it explains instead of opening
//! (Fix AH3): an Affinity or Adobe XD file gets a short dialog saying Scrim can't read it and to
//! export SVG from the tool. The list is the engine's (`importers::CLOSED`); nothing here opens or
//! reads such a file.

use std::path::Path;

use egui::RichText;

use crate::FigApp;
use crate::i18n::{tr, trf};

/// Whether `path` names a file type that opens as a document, any importer's (the extension, in
/// any case; what it is comes from its contents once read).
pub(crate) fn opens(path: &Path) -> bool {
    let exts = figcraft_engine::importers::open_extensions();
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// The closed-format tool `path` names, if any.
pub fn closed(path: &Path) -> Option<&'static str> {
    figcraft_engine::importers::closed_format(&path.to_string_lossy()).map(|c| c.tool)
}

/// Show command `e`'s error: a closed format's dialog, else the toast `msg`.
pub(crate) fn show_error(app: &mut FigApp, e: &figcraft_engine::EngineError, msg: String) {
    match e {
        figcraft_engine::EngineError::Unreadable(figcraft_engine::importers::Unreadable::ClosedFormat(tool)) => app.ui.closed_format = Some(tool),
        _ => app.toast(msg),
    }
}

/// A file named at launch that couldn't open: a closed format's dialog (on Linux and Windows a
/// double-click arrives this way); other errors were logged.
pub fn launch_error(app: &mut FigApp, e: &figcraft_engine::EngineError) {
    if let figcraft_engine::EngineError::Unreadable(figcraft_engine::importers::Unreadable::ClosedFormat(tool)) = e {
        app.ui.closed_format = Some(tool);
    }
}

/// The closed-format dialog, while one is asked for.
pub fn dialog(app: &mut FigApp, ctx: &egui::Context) {
    let Some(tool) = app.ui.closed_format else { return };
    let t = crate::theme::Tokens::get(ctx);
    let mut open = true;
    let mut close = false;
    crate::dialogs::window(tr("Can't open this file"), ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(RichText::new(trf("{tool} files use a closed format, so Scrim can't read them.", &[("tool", &tool)])).strong());
        ui.add_space(6.0);
        ui.label(RichText::new(trf("In {tool}, export the design as SVG, then open the SVG here.", &[("tool", &tool)])).color(t.text_secondary));
        ui.add_space(12.0);
        close = crate::widgets::button(ui, tr("Close"), true, true).clicked();
    });
    if !open || close {
        app.ui.closed_format = None;
    }
}

#[cfg(test)]
#[path = "file_kinds_tests.rs"]
mod tests;
