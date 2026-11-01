//! The library files a session has read, by path: re-read when they change on disk, and never
//! followed off this computer (a library link is stored in the document).

use std::collections::HashMap;
use std::sync::Arc;

use figcraft_doc::PublishedLibrary;

#[cfg(target_arch = "wasm32")]
use super::NO_FILES;
use super::{MAX_CACHED, MAX_LIBRARY_FILE};
use crate::{EngineError, Result};

/// Whether `path`, a library path a document names, is a file on this computer. Read as text on
/// every platform, so a Windows network share (`\\host\share`, `//host/share`, `\\?\UNC\…`),
/// a device path (`\\.\…`, `\\?\…`), a reserved device name (`CON`, `NUL.x`) or a
/// `file://host/` URL is off it wherever the file is opened (upstream FigCraft b3e65c89, audit
/// #1056).
pub fn on_this_computer(path: &str) -> bool {
    craft_docpath::is_local(path)
}

/// When a library file was last read.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Stamp {
    pub(super) len: u64,
    pub(super) modified: Option<std::time::SystemTime>,
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub(super) stamp: Option<Stamp>,
    /// When the file was read: a stamp only proves the file unchanged once its modification time
    /// is safely older than that, since file systems keep the time in coarse ticks.
    pub(super) read_at: Option<std::time::SystemTime>,
    pub(super) result: std::result::Result<Arc<PublishedLibrary>, String>,
}

/// How far a file's modification time must lie before its read for the stamp to be trusted.
const RACY_STAMP: std::time::Duration = std::time::Duration::from_secs(2);

impl Entry {
    /// A file stamped in the future (clock skew, unpacked archives) never settles: it is re-read on
    /// every load until the clock passes it.
    pub(super) fn still_valid(&self, now: &Stamp) -> bool {
        let settled = match (now.modified, self.read_at) {
            (Some(modified), Some(read_at)) => modified.checked_add(RACY_STAMP).is_some_and(|t| t < read_at),
            _ => false,
        };
        settled && self.stamp.as_ref() == Some(now)
    }
}

/// Library files read by a session, by path (re-read when they change on disk).
#[derive(Clone, Debug, Default)]
pub struct Cache {
    entries: HashMap<String, Entry>,
}

impl Cache {
    /// The published library in the file at `path`.
    pub fn load(&mut self, path: &str) -> Result<Arc<PublishedLibrary>> {
        self.load_limited(path, MAX_LIBRARY_FILE)
    }

    /// [`load`](Self::load), refusing files larger than `max_bytes`. A server library link gives
    /// what was last fetched ([`Session::load_library`](crate::Session::load_library) fetches).
    pub fn load_limited(&mut self, path: &str, max_bytes: u64) -> Result<Arc<PublishedLibrary>> {
        if crate::library_server::is_server_link(path) {
            return match self.cached(path) {
                Some(r) => r.cloned().map_err(|e| EngineError::Other(e.to_string())),
                None => Err(EngineError::Other(crate::library_server::NOT_FETCHED.into())),
            };
        }
        // The path comes from a document: a network share or device path would be contacted (and
        // on Windows get the opener's credentials) by the stat below (upstream b3e65c89).
        if !on_this_computer(path) {
            return Err(EngineError::Other(format!("{path}: a library is opened only from this computer")));
        }
        let stamp = stat(path);
        // Checked before the cache: the limit is the caller's.
        if let Ok(st) = &stamp
            && st.len > max_bytes
        {
            return Err(EngineError::Other(format!("{path} is too large to use as a library (the limit is {} MB)", max_bytes / 1024 / 1024)));
        }
        if let (Some(e), Ok(st)) = (self.entries.get(path), &stamp)
            && e.still_valid(st)
        {
            return e.result.clone().map_err(EngineError::Other);
        }
        let read_at = now();
        let result = match &stamp {
            Ok(_) => read_file(path).map(Arc::new),
            Err(e) => Err(e.clone()),
        };
        if self.entries.len() >= MAX_CACHED && !self.entries.contains_key(path) {
            self.entries.clear();
        }
        self.entries.insert(path.to_string(), Entry { stamp: stamp.ok(), read_at, result: result.clone() });
        result.map_err(EngineError::Other)
    }

    /// What the last read of `path` gave (no file access).
    pub fn cached(&self, path: &str) -> Option<std::result::Result<&Arc<PublishedLibrary>, &str>> {
        self.entries.get(path).map(|e| e.result.as_ref().map_err(String::as_str))
    }

    /// Remember `result` for `path` (a library fetched from a server).
    pub fn insert(&mut self, path: &str, result: std::result::Result<Arc<PublishedLibrary>, String>) {
        if self.entries.len() >= MAX_CACHED && !self.entries.contains_key(path) {
            self.entries.clear();
        }
        self.entries.insert(path.to_string(), Entry { stamp: None, read_at: None, result });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> Option<std::time::SystemTime> {
    Some(std::time::SystemTime::now())
}

#[cfg(target_arch = "wasm32")]
fn now() -> Option<std::time::SystemTime> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn stat(path: &str) -> std::result::Result<Stamp, String> {
    let m = crate::file_access::metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err(format!("{path} is not a file"));
    }
    Ok(Stamp { len: m.len(), modified: m.modified().ok() })
}

#[cfg(target_arch = "wasm32")]
fn stat(_path: &str) -> std::result::Result<Stamp, String> {
    Err(NO_FILES.into())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(path: &str) -> std::result::Result<PublishedLibrary, String> {
    let f = crate::file_access::open(path).map_err(|e| e.to_string())?;
    match figcraft_format::read_library(std::io::BufReader::new(f)) {
        Ok(Some(lib)) => Ok(lib),
        Ok(None) => Err(format!("{path} has no published library: open it and publish it first (Libraries ▸ Publish library…)")),
        Err(e) => Err(format!("{path}: {e}")),
    }
}

#[cfg(target_arch = "wasm32")]
fn read_file(_path: &str) -> std::result::Result<PublishedLibrary, String> {
    Err(NO_FILES.into())
}
