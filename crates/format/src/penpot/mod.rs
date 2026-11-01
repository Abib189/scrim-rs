//! Penpot (`.penpot`, format v3) import (Fix AH1): a zip of JSON members and binary media.
//!
//! Written from Penpot's official documentation only, checked on 2026-11-01. No Penpot source
//! code (MPL-2.0) was read, copied or translated. The pages used:
//!
//! - <https://help.penpot.app/technical-guide/developer/data-model/penpot-file-format/> (the
//!   format: archive layout, manifest, file, page, shape, colour, component, media, thumbnail
//!   and storage-object fields; its manifest shows `"generatedBy": "penpot/2.12.0"` in the field
//!   table and `"penpot/2.12.0-RC1-99-g40c27591f"` in the example, with `"version": 1`);
//! - <https://help.penpot.app/technical-guide/developer/data-model/> (the data model overview:
//!   pages and components are containers of a shape tree);
//! - <https://help.penpot.app/user-guide/export-import/penpot-file-format/> (v3 is the current
//!   ZIP + JSON format; the manifest's `version` tracks the ZIP structure, currently 1).
//!
//! What the documentation describes, and is read: `manifest.json` (`type` must be
//! `penpot/export-files`; `version` 1, a later one is refused as newer; `files`; `relations`),
//! `files/{file}/pages/{page}.json` (`name`, `index`, `background`),
//! `files/{file}/pages/{page}/{shape}.json` (one per shape: `id`, `name`, `type`, `selrect`,
//! `parentId`, `x`, `y`, `width`, `height`, `fills`, `opacity`, `blendMode`, `hidden`,
//! `blocked`, `r1`–`r4`, `componentId`, `shapes`, `boolType`, `content`, `metadata`),
//! `files/{file}/components/` (`name`, `mainInstanceId`), `files/{file}/colors/` (`name`,
//! `path`, `color`, `opacity`, `gradient`), `files/{file}/media/` (`mediaId`),
//! `files/{file}/thumbnails/{tag}/{page}/{frame}.json` (`mediaId`, for [`preview`]) and
//! `objects/{uuid}.json` (`contentType`) with `objects/{uuid}.{ext}`.
//!
//! Readings (the documentation names these fields without saying how to read them; each is a
//! choice made here, not a fact from the documentation):
//!
//! - `x`, `y`, `selrect` (and `points`) are page coordinates; Scrim's transforms are relative to
//!   the parent, so each layer is placed at its position minus its parent's.
//! - A `shapes` list is in paint order, its first shape painted first (at the bottom), as Scrim's
//!   child list is. A shape with the all-zero id is the page's root: its `shapes` list orders the
//!   top-level layers; shapes whose `parentId` is the all-zero id, or names no shape of the page,
//!   are top-level too (stacked by id when nothing orders them).
//! - A shape fill is the documented colour object: a hex `color` with `opacity`, or a `gradient`
//!   whose start and end points are in the layer's unit box; a fill list is bottom first.
//! - A path's `content` that is a string is SVG path data in page coordinates.
//! - `transform` is the documented list of six numbers; one that isn't the identity isn't
//!   applied (the layer is placed by its position and size, with a warning).
//! - `r1`–`r4` go clockwise from the top-left corner; `boolType` `difference` is Scrim's
//!   Subtract; a `blendMode` is a CSS blend keyword, and `normal` on a board or group lets its
//!   layers blend with what is behind it; every board clips its content.
//! - An image shape's `metadata.id` names a storage object, or a media reference whose `mediaId`
//!   does; the image fills the layer.
//! - A library colour's `path` is its folder in the style name ("Brand/Primary").
//!
//! Not described by the documentation, so never guessed: each becomes an import warning with the
//! layer's path (see [`crate::import_report`]): text layers (their `content` and `positionData`
//! have no documented structure), strokes, shadows, blur, layout (flex and grid) settings,
//! layout grids, constraints, prototype interactions and flows, export settings, applied tokens,
//! plugin data, image fills, raw SVG layers, instances (how `componentId` / `shapeRef` link an
//! instance to its main isn't described: an instance keeps its own layers as plain layers),
//! typographies, `tokens.json`, the other files of a multi-file export and any attribute the
//! documentation doesn't list. `files/{file}.json` (revision, timestamps, feature flags) and the
//! layers panel's `collapsed` state hold no design and are not read. Nothing is dropped silently.
//! Hostile input gives an error or warnings, never a panic; members are read within size caps.

mod archive;
mod library;
mod read;
mod shapes;
mod style;
#[cfg(test)]
mod tests;
mod tree;

use std::collections::{BTreeMap, BTreeSet};

use figcraft_doc::Document;
use serde_json::Value;

use crate::FormatError;
use crate::import_report::{ImportWarning, Severity};
use archive::Archive;
use read::{clip, id_at, list, text};

/// The manifest's `type`.
const EXPORT_TYPE: &str = "penpot/export-files";
/// The manifest `version` this import follows.
const FORMAT_VERSION: u64 = 1;
/// Largest `manifest.json` read.
const MAX_MANIFEST: u64 = 4 * 1024 * 1024;
/// Largest page, shape or library JSON member read.
const MAX_JSON: u64 = 64 * 1024 * 1024;
/// Most JSON bytes read from one file, all members together.
const JSON_BUDGET: u64 = 1024 * 1024 * 1024;
/// Largest thumbnail JSON read.
const MAX_THUMBNAIL_JSON: u64 = 64 * 1024;
/// Largest image read.
const MAX_IMAGE: u64 = 128 * 1024 * 1024;
/// Most images, and image bytes together, read from one file.
const MAX_IMAGES: usize = 10_000;
const IMAGE_BUDGET: u64 = 1024 * 1024 * 1024;
/// Longest SVG path data read from one path.
const MAX_PATH_DATA: usize = 16 * 1024 * 1024;
/// Most shapes read from one file.
const MAX_SHAPES: usize = 200_000;
/// Deepest layer nesting followed.
const MAX_DEPTH: usize = 256;
/// Most pages read.
const MAX_PAGES: usize = 1_000;
/// Most warnings kept; one more says how many were left out.
const MAX_WARNINGS: usize = 1_000;

/// The archive's manifest, when `bytes` are a zip holding a `manifest.json` of a Penpot export.
fn manifest(archive: &mut Archive) -> Result<Value, FormatError> {
    let m = archive.json_within("manifest.json", MAX_MANIFEST)?;
    if text(&m, "type") != Some(EXPORT_TYPE) {
        return Err(FormatError::Corrupt(format!("not a Penpot export: manifest.json's type isn't {EXPORT_TYPE}")));
    }
    Ok(m)
}

fn zip_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
}

/// Whether `bytes` are a Penpot export: a zip whose `manifest.json` has `"type": "penpot/export-files"`.
pub fn is_penpot(bytes: &[u8]) -> bool {
    zip_magic(bytes) && Archive::open(bytes).ok().is_some_and(|mut a| manifest(&mut a).is_ok())
}

/// The Penpot export `bytes` as a new document, with what couldn't be brought across. `name`: the
/// document name when the file doesn't carry one.
pub fn import(bytes: &[u8], name: &str) -> Result<(Document, Vec<ImportWarning>), FormatError> {
    let mut archive = Archive::open(bytes)?;
    let manifest = manifest(&mut archive)?;
    match manifest.get("version").and_then(Value::as_u64) {
        Some(FORMAT_VERSION) => {}
        Some(v) if v > FORMAT_VERSION => return Err(FormatError::Newer(u32::try_from(v).unwrap_or(u32::MAX))),
        _ => return Err(FormatError::Corrupt("not a Penpot export: manifest.json has no valid format version".into())),
    }
    let Some(file) = list(&manifest, "files").first() else {
        return Err(FormatError::Corrupt("the Penpot export lists no files".into()));
    };
    let Some(file_id) = id_at(file, "id").map(str::to_string) else {
        return Err(FormatError::Corrupt("the Penpot export's first file has no valid id".into()));
    };
    let title = text(file, "name").filter(|n| !n.trim().is_empty()).unwrap_or(if name.trim().is_empty() { "Imported Penpot file" } else { name });
    let mut cx = Ctx::new(Document::new(clip(title)), archive, file_id);
    cx.manifest_notes(&manifest);
    cx.components();
    cx.colors();
    cx.unread_library();
    let blank = cx.doc.first_page();
    cx.pages();
    cx.finish_file();
    if let Some(p) = blank
        && cx.doc.pages.len() > 1
    {
        let _ = cx.doc.remove(p);
    }
    cx.doc.current_page = cx.doc.first_open_page().or_else(|| cx.doc.first_page());
    let warnings = cx.finish_warnings();
    Ok((cx.doc, warnings))
}

/// The picture the export carries of itself (a board thumbnail), as encoded image bytes; `None`
/// when it has none.
pub fn preview(bytes: &[u8]) -> Option<Vec<u8>> {
    if !zip_magic(bytes) {
        return None;
    }
    let mut archive = Archive::open(bytes).ok()?;
    manifest(&mut archive).ok()?;
    let thumbnails: Vec<String> = archive
        .names()
        .filter(|n| n.starts_with("files/") && n.contains("/thumbnails/") && n.ends_with(".json"))
        .take(1_000)
        .map(str::to_string)
        .collect();
    thumbnails.iter().find_map(|t| {
        let v = archive.json_within(t, MAX_THUMBNAIL_JSON).ok()?;
        archive.object(id_at(&v, "mediaId")?).ok()
    })
}

/// The import in progress.
pub(crate) struct Ctx<'a> {
    doc: Document,
    archive: Archive<'a>,
    /// The id of the file imported (the manifest's first).
    file_id: String,
    warnings: Vec<ImportWarning>,
    seen: BTreeSet<(Severity, String, String)>,
    left_out: usize,
    /// Shape members read.
    loaded: usize,
    /// Main instance shape id → its component's name.
    mains: BTreeMap<String, String>,
    found_mains: BTreeSet<String>,
    /// Component id → name.
    component_names: BTreeMap<String, String>,
    /// Image id → document image key (None: it couldn't be read).
    image_keys: BTreeMap<String, Option<String>>,
    /// Paths of the layers that set constraints (reported once, for the file).
    constraints: Vec<String>,
    /// Undocumented attribute → how many layers carry it.
    unknown_keys: BTreeMap<String, usize>,
    /// Layers inside instances that carry `shapeRef` / `touched`.
    instance_parts: usize,
    /// Layers whose `transform` isn't a list of six numbers.
    unread_transforms: usize,
}

impl<'a> Ctx<'a> {
    fn new(doc: Document, archive: Archive<'a>, file_id: String) -> Self {
        Self {
            doc,
            archive,
            file_id,
            warnings: Vec::new(),
            seen: BTreeSet::new(),
            left_out: 0,
            loaded: 0,
            mains: BTreeMap::new(),
            found_mains: BTreeSet::new(),
            component_names: BTreeMap::new(),
            image_keys: BTreeMap::new(),
            constraints: Vec::new(),
            unknown_keys: BTreeMap::new(),
            instance_parts: 0,
            unread_transforms: 0,
        }
    }

    /// Report something left out or changed on the layer at `path` (repeats are kept once).
    pub(crate) fn warn(&mut self, severity: Severity, path: &str, message: impl Into<String>) {
        let message = message.into();
        if self.seen.contains(&(severity, path.to_string(), message.clone())) {
            return;
        }
        if self.warnings.len() >= MAX_WARNINGS {
            self.left_out += 1;
            return;
        }
        self.seen.insert((severity, path.to_string(), message.clone()));
        self.warnings.push(ImportWarning::new(severity, path, message));
    }

    fn finish_warnings(&mut self) -> Vec<ImportWarning> {
        let mut out = std::mem::take(&mut self.warnings);
        if self.left_out > 0 {
            out.push(ImportWarning::info("", format!("{} more warnings were left out of this report.", self.left_out)));
        }
        out
    }
}
