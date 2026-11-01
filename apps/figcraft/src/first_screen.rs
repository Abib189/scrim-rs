//! What the window shows first (Fix AD): the files named on the command line or recovered from a
//! run that ended without saving, else the Home screen with no document. Launching never makes an
//! Untitled file: File ▸ New, Home's New design file and the + tab do, as before. There is no
//! "restore the last session" setting, and none is added: Home is the default.

use figcraft_engine::Session;
use serde_json::{Value, json};

/// Open the files named on the command line, in order. One that can't open is logged and
/// skipped, and nothing takes its place. Returns the ids of the documents now open, and the last
/// file's `file.open` parameters and result, so its import report shows once the window is up (Fix
/// AG: a file opened at launch is one the person opened).
pub fn open_files_with_results(session: &mut Session, files: &[String]) -> (Vec<u64>, Option<(Value, Value)>) {
    let mut last = None;
    for f in files {
        let params = json!({"path": f});
        match session.execute("file.open", &params) {
            Ok(r) => last = Some((params, r)),
            Err(e) => log::error!("{f}: {e}"),
        }
    }
    (session.docs.iter().map(|d| d.uid).collect(), last)
}

/// Whether the window starts on Home: when it has no document to show.
pub fn starts_on_home(session: &Session) -> bool {
    session.docs.is_empty()
}

#[cfg(test)]
#[path = "first_screen_tests.rs"]
mod tests;
