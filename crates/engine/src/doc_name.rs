//! What a document is called (Fix AF): one rule for the tab, the left panel, the window title,
//! Home, the recent files, the save and export dialogs and what agents are told.
//!
//! 1. The name the document carries, when someone gave it one (a rename, `file.new {name}`, the
//!    name in a file from another tool). The placeholder "Untitled" and an empty name don't count.
//! 2. Else the name of its file, without the extension.
//! 3. Else "Untitled", numbered from the second unsaved one on ("Untitled 2").
//!
//! The name is only read here, never written: opening or saving a file leaves the name stored in it
//! alone, so a file saved with the placeholder takes the name of whatever file it is.

use crate::{DocState, Session};

/// The name a new document starts with, and the one a stored name of the placeholder stands for.
pub const UNTITLED: &str = "Untitled";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocName {
    Named(String),
    /// Never saved and never named: "Untitled" (1) or "Untitled 2" and on.
    Untitled(u32),
}

impl DocName {
    /// In English, as files, agents and the command results have it ("Untitled 2").
    pub fn english(&self) -> String {
        match self {
            DocName::Named(n) => n.clone(),
            DocName::Untitled(n) if *n <= 1 => UNTITLED.into(),
            DocName::Untitled(n) => format!("{UNTITLED} {n}"),
        }
    }
}

/// The stored name, when it is one someone gave: neither empty nor the placeholder.
pub fn explicit(stored: &str) -> Option<&str> {
    let n = stored.trim();
    (!n.is_empty() && n != UNTITLED).then_some(n)
}

/// The file name at `path` without its folder and extension.
pub fn file_stem(path: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    p.file_stem().or_else(|| p.file_name()).map(|s| s.to_string_lossy().into_owned()).filter(|s| !s.trim().is_empty())
}

/// The rule itself: the `stored` name if someone gave one, else the stem of `path`, else
/// "Untitled" numbered `untitled`.
pub fn resolve(stored: &str, path: Option<&str>, untitled: u32) -> DocName {
    match explicit(stored) {
        Some(n) => DocName::Named(n.to_string()),
        None => path.and_then(file_stem).map_or(DocName::Untitled(untitled.max(1)), DocName::Named),
    }
}

impl DocState {
    /// What this document is called (a version shown in its place doesn't change it).
    pub fn name(&self) -> DocName {
        resolve(&self.document_itself().name, self.path.as_deref(), self.untitled)
    }

    /// [`Self::name`] in English.
    pub fn display_name(&self) -> String {
        self.name().english()
    }
}

/// The name of the file at `path` as the recent files list it: the open document's when it is
/// open (its unsaved rename too), else the name `stored` in the file, else the file's stem.
pub fn of_file(docs: &[DocState], path: &str, stored: Option<&str>) -> DocName {
    match docs.iter().find(|d| d.path.as_deref() == Some(path)) {
        Some(d) => d.name(),
        None => resolve(stored.unwrap_or_default(), Some(path), 1),
    }
}

/// Give the document in tab `i`, when it is untitled, the lowest number no other untitled tab
/// shows: "Untitled", then "Untitled 2". It keeps it while it stays untitled.
pub(crate) fn number(s: &mut Session, i: usize) {
    let Some(ds) = s.docs.get(i) else { return };
    if !matches!(ds.name(), DocName::Untitled(_)) {
        return;
    }
    let taken: Vec<u32> = (s.docs.iter().enumerate())
        .filter(|(j, _)| *j != i)
        .filter_map(|(_, d)| match d.name() {
            DocName::Untitled(n) => Some(n),
            DocName::Named(_) => None,
        })
        .collect();
    let free = (1..).find(|n| !taken.contains(n)).unwrap_or(1);
    if let Some(ds) = s.docs.get_mut(i) {
        ds.untitled = free;
    }
}

#[cfg(test)]
#[path = "doc_name_tests.rs"]
mod tests;
