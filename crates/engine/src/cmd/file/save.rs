//! Save and Save as: the file of the active tab, written only where nothing would be lost
//! without the person choosing it (another app's changes, Fix AI LW-4; a FigCraft board's
//! features, Fix AI C-1).

use serde_json::{Value, json};

use super::{board_file, branch_tabs_saved, export_fig, ext, file_bytes, file_stem, on_disk, remember, saves_in_place, string, write};
use crate::{EngineError, Result, Session};

/// Save the file of the active tab (the main file for a branch tab, with its branches), or export
/// the active document as an experimental `.fig` copy without changing its native save state.
fn save_to(s: &mut Session, path: String, overwrite: bool, drop_board: bool) -> Result<Value> {
    if ext(&path) == "fig" {
        return export_fig(s, &json!({"path": path}));
    }
    // Scrim writes .scrim files only: a .figcraft (or anything else) it opened is never written.
    if !saves_in_place(&path) {
        return Err(EngineError::Disabled("file.save".into(), format!("Scrim saves .scrim files, and never writes {path}: pass a .scrim path")));
    }
    let i = crate::branches::file_tab(s);
    let ds = s.docs.get_mut(i).ok_or(EngineError::NoDocument)?;
    // Another app wrote to this tab's own file since: ask, never replace it silently (Fix AI LW-4).
    let own = ds.path.as_deref().is_some_and(|own| crate::file_access::same_file(own, &path));
    if own && !overwrite && ds.on_disk.as_ref().is_some_and(|was| on_disk::changed(&path, was)) {
        let name = file_stem(&path);
        s.request_ui("dialog.saveConflict", json!({"name": name}));
        return Err(on_disk::conflict(&name));
    }
    // A FigCraft board loses its board features when written: only when the person chose that
    // (Fix AI C-1). Save as a new file never asks.
    if own && !drop_board && ds.on_disk.as_ref().is_some_and(|d| d.board().is_some()) {
        return Err(board_file::refused(s, &file_stem(&path)));
    }
    let bytes = file_bytes(ds)?;
    write(&path, &bytes)?;
    ds.note_on_disk(on_disk::stat(&path), &bytes);
    // Images the file held that weren't loaded were copied into this one: said so (Fix AI LW-3).
    let kept = ds.images_left_now_in(&path);
    ds.path = Some(path.clone());
    ds.mark_saved();
    ds.revision += 1;
    let uid = ds.uid;
    branch_tabs_saved(s, uid);
    remember(s, &path);
    if kept > 0 {
        s.request_ui("notice.imagesKept", json!({"name": file_stem(&path), "count": kept}));
    }
    Ok(json!({"path": path, "bytes": bytes.len()}))
}

pub(super) fn save(s: &mut Session, p: &Value) -> Result<Value> {
    let overwrite = p.get("overwrite").and_then(Value::as_bool) == Some(true);
    let drop_board = p.get("dropBoard").and_then(Value::as_bool) == Some(true);
    if let Some(path) = string(p, "path") {
        return save_to(s, path, overwrite, drop_board);
    }
    match s.docs.get(crate::branches::file_tab(s)).ok_or(EngineError::NoDocument)?.path.clone() {
        // A file opened from a .figcraft (or a .fig name, from earlier builds) is never written: Save
        // asks for a .scrim next to it, and the document is that file from then on.
        Some(path) if !saves_in_place(&path) => {
            s.request_ui("dialog.saveAs", json!({}));
            Err(EngineError::Disabled(
                "file.save".into(),
                format!("{path} isn't a .scrim file and is never written: choose Save as or pass a .scrim `path`"),
            ))
        }
        Some(path) => save_to(s, path, overwrite, drop_board),
        None => {
            s.request_ui("dialog.saveAs", json!({}));
            Err(EngineError::Disabled("file.save".into(), "the file has no path yet: pass `path`".into()))
        }
    }
}

pub(super) fn save_as(s: &mut Session, p: &Value) -> Result<Value> {
    match string(p, "path") {
        Some(mut path) => {
            // Any other name gets .scrim added (a .figcraft or .json one is refused by `save_to`).
            if !matches!(ext(&path).as_str(), "scrim" | "fig" | "figcraft" | "json") {
                path.push_str(".scrim");
            }
            save_to(s, path, false, false)
        }
        None => {
            s.request_ui("dialog.saveAs", json!({}));
            Err(EngineError::Disabled("file.saveAs".into(), "pass `path`".into()))
        }
    }
}
