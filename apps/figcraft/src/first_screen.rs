//! What the window shows first (Fix AD): the files named on the command line or recovered from a
//! run that ended without saving, else the Home screen with no document. Launching never makes an
//! Untitled file: File ▸ New, Home's New design file and the + tab do, as before. There is no
//! "restore the last session" setting, and none is added: Home is the default.

use figcraft_engine::{EngineError, Session};
use serde_json::{Value, json};

/// The last file opened at launch: its `file.open` parameters and outcome.
pub type LastOpen = (Value, Result<Value, EngineError>);

/// Open the files named on the command line, in order. One that can't open is logged and
/// skipped, and nothing takes its place. Returns the ids of the documents now open, and the last
/// file's `file.open` parameters and outcome, so its import report, or why it can't open (a closed
/// format's dialog, Fix AH3), shows once the window is up (Fix AG: a file opened at launch is one
/// the person opened).
pub fn open_files_with_results(session: &mut Session, files: &[String]) -> (Vec<u64>, Option<LastOpen>) {
    let mut last = None;
    for f in files {
        let params = json!({"path": f});
        let r = session.execute("file.open", &params);
        if let Err(e) = &r {
            log::error!("{f}: {e}");
        }
        last = Some((params, r));
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
