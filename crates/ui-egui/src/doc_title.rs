//! A document's name as the window shows it (Fix AF): the one rule in
//! [`figcraft_engine::doc_name`], with "Untitled" in the interface's language. The tab, the left
//! panel, the window title, Home, the recent files and the save and export dialogs all read it here.

use figcraft_engine::DocState;
use figcraft_engine::doc_name::DocName;

use crate::FigApp;
use crate::i18n::{tr, trf};

/// `name` as shown: "Untitled" and "Untitled 2" translated, any other name as it is.
pub fn shown(name: &DocName) -> String {
    match name {
        DocName::Named(n) => n.clone(),
        DocName::Untitled(n) if *n <= 1 => tr("Untitled").to_string(),
        DocName::Untitled(n) => trf("Untitled {n}", &[("n", n)]),
    }
}

/// What `ds` is called on screen.
pub fn of(ds: &DocState) -> String {
    shown(&ds.name())
}

/// What the document shown is called (`None` without one).
pub fn active(app: &FigApp) -> Option<String> {
    app.session.ds().ok().map(of)
}

/// The recent file at `path` as listed: the open document's name, else the name `stored` in the
/// file (Home reads it in the background), else the file's name without its extension.
pub fn of_file(app: &FigApp, path: &str, stored: Option<&str>) -> String {
    shown(&figcraft_engine::doc_name::of_file(&app.session.docs, path, stored))
}

#[cfg(test)]
#[path = "doc_name_ui_tests.rs"]
mod tests;
