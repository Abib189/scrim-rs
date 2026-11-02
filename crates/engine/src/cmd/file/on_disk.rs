//! The file on disk as a tab last read or wrote it (Fix AI LW-4, upstream FigCraft a648700f and
//! 0d7d9ebd): Save never silently replaces what another app wrote to the file since. The file has
//! changed when its size or modification time differ and its content does too, so a sync app
//! writing the same bytes again isn't a change.

use std::time::SystemTime;

/// A file's size, modification time and content hash.
#[derive(Clone, Debug, PartialEq)]
pub struct OnDisk {
    len: u64,
    modified: Option<SystemTime>,
    hash: String,
    /// The file is a FigCraft board, read as a design (Fix AI C-1): Save asks before writing it.
    board: Option<figcraft_format::BoardMarks>,
}

impl OnDisk {
    /// Holding the board marks `board`, as it was read.
    pub(crate) fn with_board(self, board: Option<figcraft_format::BoardMarks>) -> Self {
        Self { board, ..self }
    }

    /// The board marks of the file as it was read (`None`: a design, or written by Scrim since).
    pub fn board(&self) -> Option<figcraft_format::BoardMarks> {
        self.board
    }
}

/// The size and modification time of the file at `path` (`None`: there is none).
pub(crate) fn stat(path: &str) -> Option<(u64, Option<SystemTime>)> {
    let m = crate::file_access::metadata(path).ok().filter(std::fs::Metadata::is_file)?;
    Some((m.len(), m.modified().ok()))
}

/// The file `meta` was taken of, holding `bytes` (read from it, or just written to it). Taken
/// before reading (upstream 0d7d9ebd), a change made during the read shows as one later.
pub(crate) fn stamp(meta: Option<(u64, Option<SystemTime>)>, bytes: &[u8]) -> Option<OnDisk> {
    meta.map(|(len, modified)| OnDisk { len, modified, hash: figcraft_format::image_hash(bytes), board: None })
}

/// Whether the file at `path` changed since it was `was`. A file that is gone hasn't (a save
/// writes it again and loses nothing).
pub(crate) fn changed(path: &str, was: &OnDisk) -> bool {
    match stat(path) {
        None => false,
        Some((len, modified)) if len == was.len && modified == was.modified => false,
        Some((len, _)) if len != was.len => true,
        // The same size at another time: compare what it holds.
        Some(_) => crate::file_access::read(path).map_or(true, |b| figcraft_format::image_hash(&b) != was.hash),
    }
}

/// What a save refused for a changed file says (the window shows its three choices instead).
pub(crate) fn conflict(name: &str) -> crate::EngineError {
    crate::EngineError::FileChangedOnDisk(format!(
        "`file.save` was refused: {name} changed on disk since it was opened or last saved here (another app wrote to it), and saving would replace those changes. Save a copy under a new name, pass `overwrite: true` to replace them, or run file.revert to load the file from disk"
    ))
}
