//! Images a file holds that weren't loaded, kept when it is saved (Fix AI LW-3). A file whose images
//! together pass what Scrim reads from one file opens without the ones past that budget. They are
//! intact in the file, so a save copies them across from it, member by member as they are stored
//! (nothing is decoded), instead of writing only the images in memory and losing the rest. A save
//! that can't copy them is refused: nothing is written, and the file keeps them.

use std::collections::BTreeSet;
use std::io::{Seek, Write};
use std::path::PathBuf;

use crate::{FormatError, io_err};

/// The images a document's file holds that weren't loaded, and that file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LeftInFile {
    pub file: PathBuf,
    /// Their content hashes (`images/<hash>` in the file).
    pub hashes: BTreeSet<String>,
}

/// Copy the images `left` names into `z`, past the ones already `written`.
pub(crate) fn put<W: Write + Seek>(z: &mut zip::ZipWriter<W>, left: &LeftInFile, written: &BTreeSet<&String>) -> Result<(), FormatError> {
    let todo: Vec<&String> = left.hashes.iter().filter(|h| !written.contains(h)).collect();
    if todo.is_empty() {
        return Ok(());
    }
    let lost = |why: String| {
        FormatError::Io(format!(
            "{} images that weren't loaded couldn't be copied from {} ({why}), so nothing was saved and none of them is lost",
            todo.len(),
            left.file.display()
        ))
    };
    let file = std::fs::File::open(&left.file).map_err(|e| lost(e.to_string()))?;
    let mut src = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| lost(e.to_string()))?;
    for hash in &todo {
        let i = src.index_for_name(&format!("images/{hash}")).ok_or_else(|| lost(format!("the file no longer holds image {hash}")))?;
        let member = src.by_index_raw(i).map_err(|e| lost(e.to_string()))?;
        z.raw_copy_file(member).map_err(io_err)?;
    }
    Ok(())
}
