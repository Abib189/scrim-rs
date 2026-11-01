//! What an import couldn't bring across exactly (Fix AG): one entry per thing left out or changed,
//! with the layer it was on. Every importer reports this way, so the import report and agents read
//! one shape.

use serde::Serialize;

/// How much of something an import kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Left out: nothing of it is in the new document.
    Dropped,
    /// Brought across, but not exactly (a gradient as its first colour, say).
    Approximated,
    /// Worth knowing, nothing lost (a font that isn't installed, say).
    Info,
}

impl Severity {
    pub const ALL: [Severity; 3] = [Severity::Dropped, Severity::Approximated, Severity::Info];

    pub fn id(self) -> &'static str {
        match self {
            Severity::Dropped => "dropped",
            Severity::Approximated => "approximated",
            Severity::Info => "info",
        }
    }
}

/// One thing an import left out or changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ImportWarning {
    pub severity: Severity,
    /// The layer it was on, as names from the page down ("Page 1 / Card / Title"); empty for the
    /// file as a whole.
    pub path: String,
    /// What was left out or changed, in English.
    pub message: String,
}

impl ImportWarning {
    pub fn new(severity: Severity, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { severity, path: path.into(), message: message.into() }
    }

    pub fn dropped(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Dropped, path, message)
    }

    pub fn approximated(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Approximated, path, message)
    }

    pub fn info(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Info, path, message)
    }

    /// A note from an importer that reports in sentences without a layer (the `.fig`, REST API
    /// file JSON and SVG importers): left out when it says so, approximated when it says so, else
    /// for information. Its words are kept as they are.
    pub fn from_note(note: &str) -> Self {
        let n = note.to_ascii_lowercase();
        let dropped = [
            "skipped",
            "left out",
            "not imported",
            "weren't imported",
            "wasn't imported",
            "not supported",
            "ignored",
            "dropped",
            "lost",
            "without it",
        ];
        let approximated = ["approximat", "import as", "imported as", "instead", "flattened", "converted"];
        let severity = if dropped.iter().any(|w| n.contains(w)) {
            Severity::Dropped
        } else if approximated.iter().any(|w| n.contains(w)) {
            Severity::Approximated
        } else {
            Severity::Info
        };
        Self::new(severity, "", note)
    }
}

/// How many of `warnings` there are of each severity, in [`Severity::ALL`]'s order.
pub fn counts(warnings: &[ImportWarning]) -> [(Severity, usize); 3] {
    Severity::ALL.map(|s| (s, warnings.iter().filter(|w| w.severity == s).count()))
}
