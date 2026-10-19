//! The settings directory is Scrim's now (`docs/design/rebrand.md` › group 3). The first time Scrim
//! starts, when its directory doesn't exist yet and FigCraft's does, FigCraft's preferences, recent
//! files, recovery copies and installed plugins are **copied** into a new Scrim directory. FigCraft's
//! is never moved, changed or deleted, so a FigCraft install keeps working beside Scrim.
//!
//! The copy goes to a temporary sibling first and is renamed into place at the end, so a failed or
//! interrupted copy never leaves a half-filled Scrim directory that would stop a later try.

use std::io;
use std::path::Path;

/// The settings directory's name under the platform's base directory.
pub const NEW: &str = "Scrim";
/// FigCraft's, copied from once.
pub const OLD: &str = "FigCraft";

/// How deep the copy follows directories (the settings tree is shallow).
const MAX_DEPTH: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub enum Migration {
    /// Neither directory: a fresh install.
    Fresh,
    /// FigCraft's settings were copied (this many files).
    Copied { files: usize },
    /// Scrim's directory exists already: nothing is copied, whatever FigCraft's holds.
    AlreadyThere,
    /// The copy failed; Scrim starts with fresh settings and tries again next time.
    Failed(String),
}

/// Copy `base/FigCraft` to `base/Scrim` when only the first exists.
pub fn migrate(base: &Path) -> Migration {
    let (new, old) = (base.join(NEW), base.join(OLD));
    if new.exists() {
        return Migration::AlreadyThere;
    }
    if !old.is_dir() {
        return Migration::Fresh;
    }
    let tmp = base.join(format!(".{NEW}-copying-{}", std::process::id()));
    let mut files = 0;
    let copied = copy_tree(&old, &tmp, 0, &mut files).and_then(|()| std::fs::rename(&tmp, &new));
    match copied {
        Ok(()) => Migration::Copied { files },
        Err(e) => {
            // Only the temporary copy this run made is removed; FigCraft's is never touched.
            let _ = std::fs::remove_dir_all(&tmp);
            Migration::Failed(e.to_string())
        }
    }
}

/// Copy files and directories (symbolic links are left out).
fn copy_tree(from: &Path, to: &Path, depth: usize, files: &mut usize) -> io::Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target, depth + 1, files)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)?;
            *files += 1;
        }
    }
    Ok(())
}
