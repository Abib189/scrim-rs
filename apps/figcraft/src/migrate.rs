//! The settings directory is Scrim's now (`docs/design/rebrand.md` › group 3). The first time Scrim
//! starts, when its directory doesn't exist yet and FigCraft's does, FigCraft's preferences, recent
//! files, recovery copies and installed plugins are **copied** into a new Scrim directory.
//!
//! That directory is named by the app id, `design.scrim.app`, not `Scrim`: a directory called
//! `Scrim` is the earlier TypeScript Scrim's (Electron names its data directory after the product),
//! and sharing it would mix the two apps' files and skip this copy. That directory is never read or
//! written here. FigCraft's
//! is never moved, changed or deleted, so a FigCraft install keeps working beside Scrim.
//!
//! One setting is not carried over: FigCraft's built-in share server (`UPSTREAM_SERVER`), which
//! FigCraft saved into its preferences. Scrim builds in none (rebrand step A5), so a copied
//! preference naming that server is emptied and Share ▸ Online asks for one. A server the person
//! chose themselves is kept.
//!
//! The copy goes to a temporary sibling first and is renamed into place at the end, so a failed or
//! interrupted copy never leaves a half-filled Scrim directory that would stop a later try.

use std::io;
use std::path::Path;

/// The settings directory's name under the platform's base directory: the app id (see above).
pub const NEW: &str = "design.scrim.app";
/// The TypeScript Scrim's data directory, which this app leaves alone.
#[cfg(test)]
pub const ELECTRON: &str = "Scrim";
/// FigCraft's, copied from once.
pub const OLD: &str = "FigCraft";

/// FigCraft's built-in share server (upstream's, not Scrim's).
pub const UPSTREAM_SERVER: &str = "wss://collab.craftapps.dev/";

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

/// Copy `base/FigCraft` to `base/design.scrim.app` when only the first exists.
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
    let copied = copy_tree(&old, &tmp, 0, &mut files).and_then(|()| forget_upstream_server(&tmp)).and_then(|()| std::fs::rename(&tmp, &new));
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

/// Empty the copied preferences' share server when it is FigCraft's built-in one. A state file
/// that doesn't parse is left as it is (the app then starts with default preferences anyway).
fn forget_upstream_server(dir: &Path) -> io::Result<()> {
    let path = dir.join("state.json");
    let Ok(text) = std::fs::read_to_string(&path) else { return Ok(()) };
    let Ok(mut state) = serde_json::from_str::<serde_json::Value>(&text) else { return Ok(()) };
    let Some(server) = state.pointer_mut("/prefs/publish_server") else { return Ok(()) };
    if server.as_str().map(str::trim) != Some(UPSTREAM_SERVER) {
        return Ok(());
    }
    *server = serde_json::Value::String(String::new());
    std::fs::write(&path, serde_json::to_vec(&state).map_err(io::Error::other)?)
}
