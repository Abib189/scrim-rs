//! Closed formats (Fix AH3): files from tools whose format has no public documentation Scrim may
//! read (`docs/design/import-formats.md`: Affinity, Adobe XD). Scrim never opens them, and never
//! guesses at them: it says what they are and to export SVG from the tool. Paper, Framer and Canva
//! keep designs online, with no file of their own to drop, so they have no entry here.

/// A tool whose files Scrim can't read, and the extensions that name them.
#[derive(Debug, PartialEq, Eq)]
pub struct ClosedFormat {
    /// The tool's name, as its maker writes it (not translated).
    pub tool: &'static str,
    pub extensions: &'static [&'static str],
}

/// Every closed format, the one list the engine and the window read.
pub static CLOSED: &[ClosedFormat] =
    &[ClosedFormat { tool: "Affinity", extensions: &["af", "afdesign", "afphoto", "afpub"] }, ClosedFormat { tool: "Adobe XD", extensions: &["xd"] }];

/// The closed format a file name's extension (in any case) says it is.
pub fn closed_format(name: &str) -> Option<&'static ClosedFormat> {
    let ext = std::path::Path::new(name).extension()?.to_str()?;
    CLOSED.iter().find(|c| c.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}
