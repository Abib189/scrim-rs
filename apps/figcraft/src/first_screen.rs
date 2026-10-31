//! What the window shows first (Fix AD): the files named on the command line or recovered from a
//! run that ended without saving, else the Home screen with no document. Launching never makes an
//! Untitled file: File ▸ New, Home's New design file and the + tab do, as before. There is no
//! "restore the last session" setting, and none is added: Home is the default.

use figcraft_engine::Session;
use serde_json::json;

/// Open the files named on the command line, in order. One that can't open is logged and
/// skipped, and nothing takes its place. Returns the ids of the documents now open.
pub fn open_files(session: &mut Session, files: &[String]) -> Vec<u64> {
    for f in files {
        if let Err(e) = session.execute("file.open", &json!({"path": f})) {
            log::error!("{f}: {e}");
        }
    }
    session.docs.iter().map(|d| d.uid).collect()
}

/// Whether the window starts on Home: when it has no document to show.
pub fn starts_on_home(session: &Session) -> bool {
    session.docs.is_empty()
}

#[cfg(test)]
#[path = "first_screen_tests.rs"]
mod tests;
