//! Lunacy's FREE format (`.free`) import (Fix AH2): a zip of JSON documents.
//!
//! Written from the format's published specification only: the README of
//! <https://github.com/icons8/free> (MIT licence, © 2023 Icons8), as of commit
//! `9b2fc846453f1917216a8d8202e2f31d48996115` on `main` (4 October 2026; it documents format
//! version 11, changelog dated 27.09.2026), with the repository's `llms.txt` (same commit, same
//! licence) for the base types the README only names: colours as ARGB hex with minified forms,
//! `Thickness` as 1, 2 or 4 numbers, and field names in camelCase as the README's own JSON
//! examples write them (`"_t"`, `"id"`, `"pos"`, `"fill"`). The schema's C# source (`src/cs`)
//! was not copied or translated; it was read for two points the README leaves open and says so
//! where they are used: `Thickness` with two values (horizontal, vertical) and which `CurveMode`
//! values carry which path handles. No one else's `.free` file was used; the tests build theirs.
//!
//! The archive (README): `pages/` (one JSON `Page` per file), `images/` (raster files, named by
//! image fills' `pattern.image`), `fonts/`, `shared/` (one JSON `SharedLibrary` per file),
//! `document.json`, `meta.json` (`version`: the format version) and `preview.webp`.
//!
//! Where the README names a field without saying how to read it, the reading chosen is stated
//! here and in the code:
//!
//! - Pages are matched to `document.json` › `pages` by their own `id`, in that order; pages it
//!   doesn't list follow by file name.
//! - A layer's `frame`, `pos` and `transform` place it in its parent (a group's children, like a
//!   frame's, are relative to it, as in the Sketch format FREE succeeds); layers paint first to
//!   last (the last on top), as in Sketch. `frame` and `pos` override `transform` entirely.
//! - Path points and gradient points are in the layer's unit box (the README's rectangle example
//!   and gradient defaults show it). See `shapes`, `text` and `layout` for theirs.
//! - The glass blur's settings are Scrim's glass with percentages as fractions; the light angle is
//!   taken as it is (both default to -45°).
//!
//! Brought across: pages (name, background, ruler guides), frames (nested too), components,
//! states and slots (as frames), groups, sections, shapes (boolean groups), rectangles, ovals,
//! polygons, stars, triangles, paths, lines, text (with styled runs), instances (linked to their
//! components, own style, and their hidden / opacity / blend / text / fill / border / image
//! overrides), slices, fills (solid, gradient, image), borders, shadows, inner shadows, blurs
//! (layer, background, glass; motion and zoom as layer blurs), opacity, blend modes, hidden,
//! locked, masks, constraints, export options, custom properties, auto layout and how children
//! size in it, and shared-library components that instances use. Everything else a file carries
//! is listed in the import warnings with the layer it was on (see [`crate::import_report`]):
//! variables, styles' links, prototyping, layout grids, component properties, connectors,
//! embedded fonts, and any field the README doesn't list (named in the warning). Not read, as
//! they hold no design: the document's `id`, `nudge`, `fromFigma`; a page's `origin`, `zoom`,
//! `nameFixed`; a layer's `expand` (layer list state), a group's `skipSelect` /
//! `skipConstraints`, a component's `figmaId`, `predefined` and reserved `componentType`.
//! Hostile input gives an error or warnings, never a panic; members are read within size caps.

mod components;
mod json;
mod layers;
mod layout;
mod shapes;
mod style;
#[cfg(test)]
mod tests;
mod text;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use figcraft_doc::color::Color;
use figcraft_doc::{Document, NodeId, NodeKind};
use serde_json::Value;

use crate::FormatError;
use crate::import_report::{ImportWarning, Severity};
use components::Pending;
use json::{clip, list, text};
use layers::Scope;

/// The FREE format version the README describes (its changelog's latest).
pub const FORMAT_VERSION: u64 = 11;
/// Largest `document.json`, `meta.json`, page or library JSON read.
const MAX_JSON: u64 = 256 * 1024 * 1024;
/// Most JSON bytes read from one file, all members together.
const JSON_BUDGET: u64 = 1024 * 1024 * 1024;
/// Largest image read.
const MAX_IMAGE: u64 = 128 * 1024 * 1024;
/// Most images, and image bytes together, read from one file.
const MAX_IMAGES: usize = 10_000;
const IMAGE_BUDGET: u64 = 1024 * 1024 * 1024;
/// How much of `document.json`, `meta.json` and the first page [`is_free`] reads.
const SNIFF_JSON: u64 = 64 * 1024 * 1024;
/// Largest preview read.
const MAX_PREVIEW: u64 = 32 * 1024 * 1024;
/// Most layers imported (instances' copies of their main included).
const MAX_NODES: usize = 200_000;
/// Deepest layer nesting followed.
const MAX_DEPTH: usize = 256;
/// Most pages, and shared libraries, read.
const MAX_PAGES: usize = 1_000;
/// Most warnings kept; one more says how many were left out.
const MAX_WARNINGS: usize = 1_000;
/// `meta.json` keys a Sketch file has and FREE's `Meta` doesn't.
const SKETCH_META: &[&str] =
    &["compatibilityVersion", "commit", "pagesAndArtboards", "created", "build", "coeditCompatibilityVersion", "saveHistory"];

fn zip_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
}

/// Whether `bytes` are a FREE file: a zip with `document.json` and `meta.json` that are FREE's
/// (a `document.json` without Sketch's `_class` or Scrim's `format`, a `meta.json` whose
/// `version` is a number and that has none of Sketch's own keys) and a mark only FREE has: a
/// `preview.webp`, or a page in `pages/` whose layers carry FREE's `_t` type field.
pub fn is_free(bytes: &[u8]) -> bool {
    if !zip_magic(bytes) {
        return false;
    }
    let Ok(mut z) = zip::ZipArchive::new(Cursor::new(bytes)) else { return false };
    let mut json = |name: &str| crate::read_member(&mut z, name, SNIFF_JSON).ok().and_then(|d| serde_json::from_slice::<Value>(&d).ok());
    let Some(document) = json("document.json").filter(Value::is_object) else { return false };
    if document.get("_class").is_some() || document.get("format").is_some() || document.get("pages").is_some_and(|p| !p.is_array()) {
        return false;
    }
    let Some(meta) = json("meta.json").filter(Value::is_object) else { return false };
    if meta.get("version").is_some_and(|v| !v.is_u64()) || SKETCH_META.iter().any(|k| meta.get(*k).is_some()) {
        return false;
    }
    if z.index_for_name("preview.webp").is_some() {
        return true;
    }
    let first = z.file_names().filter(|n| n.starts_with("pages/") && n.ends_with(".json")).min().map(str::to_string);
    let Some(page) = first.and_then(|f| crate::read_member(&mut z, &f, SNIFF_JSON).ok()).and_then(|d| serde_json::from_slice::<Value>(&d).ok())
    else {
        return false;
    };
    page.get("_class").is_none() && list(&page, "layers").first().is_some_and(|l| l.get("_t").is_some_and(Value::is_string))
}

/// The FREE file `bytes` as a new document, with what couldn't be brought across. `name`: the
/// document name when the file doesn't carry one.
pub fn import(bytes: &[u8], name: &str) -> Result<(Document, Vec<ImportWarning>), FormatError> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| FormatError::Corrupt(format!("not a FREE file: {e}")))?;
    crate::zip_limits::check(&mut z, bytes.len() as u64)?;
    let mut archive = Archive { zip: z, json_left: JSON_BUDGET, images: 0, image_bytes: 0 };
    let document = archive.json("document.json")?;
    if !document.is_object() || document.get("_class").is_some() || document.get("format").is_some() {
        return Err(FormatError::Corrupt("not a FREE file: document.json is not a FREE document".into()));
    }
    let meta = archive.json("meta.json")?;
    if !meta.is_object() {
        return Err(FormatError::Corrupt("not a FREE file: meta.json is not an object".into()));
    }
    let version = match meta.get("version") {
        None => 0,
        Some(v) => v.as_u64().ok_or_else(|| FormatError::Corrupt("meta.json: the format version is not a number".into()))?,
    };
    if version > FORMAT_VERSION {
        return Err(FormatError::Newer(u32::try_from(version).unwrap_or(u32::MAX)));
    }
    let title = if name.trim().is_empty() { "Imported FREE file" } else { name };
    let mut cx = Ctx::new(Document::new(clip(title)), archive);
    if version < FORMAT_VERSION {
        cx.warn(
            Severity::Info,
            "",
            format!("The file is FREE version {version}; this import follows the spec for version {FORMAT_VERSION}, so fields renamed since are reported as unknown."),
        );
    }
    cx.document_notes(&document);
    let libraries = cx.libraries();
    let blank = cx.doc.first_page();
    let pages = cx.pages(&document);
    let mut ids = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        ids.push(cx.page(page, i));
    }
    cx.library_components(&libraries);
    cx.finish_instances();
    if let Some(p) = blank
        && cx.doc.pages.len() > 1
    {
        let _ = cx.doc.remove(p);
    }
    let current = document.get("currentPageIndex").and_then(Value::as_u64).and_then(|i| ids.get(usize::try_from(i).ok()?).copied());
    cx.doc.current_page = current.or_else(|| cx.doc.first_open_page()).or_else(|| cx.doc.first_page());
    let warnings = cx.finish_warnings();
    Ok((cx.doc, warnings))
}

/// The picture the file carries of itself (`preview.webp`), as encoded image bytes.
pub fn preview(bytes: &[u8]) -> Option<Vec<u8>> {
    if !zip_magic(bytes) {
        return None;
    }
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let data = crate::read_member(&mut z, "preview.webp", MAX_PREVIEW).ok()?;
    (!data.is_empty()).then_some(data)
}

/// The zip, with what is left of the read budgets.
pub(crate) struct Archive<'a> {
    zip: zip::ZipArchive<Cursor<&'a [u8]>>,
    json_left: u64,
    images: usize,
    image_bytes: u64,
}

impl Archive<'_> {
    /// Member `name` parsed as JSON, within [`MAX_JSON`] and the file's JSON budget.
    fn json(&mut self, name: &str) -> Result<Value, FormatError> {
        let limit = MAX_JSON.min(self.json_left);
        let data = crate::read_member(&mut self.zip, name, limit)?;
        self.json_left = self.json_left.saturating_sub(data.len() as u64);
        serde_json::from_slice(&data).map_err(|e| FormatError::Corrupt(format!("{name}: {e}")))
    }

    fn has(&self, name: &str) -> bool {
        self.zip.index_for_name(name).is_some()
    }

    /// The member names under `dir` ending in `.json`, sorted.
    fn json_members(&self, dir: &str) -> Vec<String> {
        let mut names: Vec<String> = self.zip.file_names().filter(|n| n.starts_with(dir) && n.ends_with(".json")).map(str::to_string).collect();
        names.sort();
        names
    }

    /// Image member `name`, within the image caps.
    fn image(&mut self, name: &str) -> Result<Vec<u8>, String> {
        if self.images >= MAX_IMAGES {
            return Err(format!("the file has more than {MAX_IMAGES} images"));
        }
        let limit = MAX_IMAGE.min(IMAGE_BUDGET.saturating_sub(self.image_bytes));
        let data = crate::read_member(&mut self.zip, name, limit).map_err(|e| match e {
            FormatError::Corrupt(m) | FormatError::Io(m) => m,
            FormatError::Newer(_) => "unreadable".into(),
            FormatError::Unpack(u) => u.to_string(),
        })?;
        self.images += 1;
        self.image_bytes = self.image_bytes.saturating_add(data.len() as u64);
        Ok(data)
    }
}

/// The import in progress.
pub(crate) struct Ctx<'a> {
    doc: Document,
    archive: Archive<'a>,
    warnings: Vec<ImportWarning>,
    seen: BTreeSet<(Severity, String, String)>,
    left_out: usize,
    count: usize,
    /// `componentId` → the main component imported for it.
    mains: BTreeMap<String, NodeId>,
    /// Layers imported from the file → their `id` (overrides name layers by it).
    free_ids: BTreeMap<NodeId, String>,
    instances: Vec<Pending>,
    /// Style ids of the document and its libraries → their names.
    style_names: BTreeMap<String, String>,
    /// Image name → document image key (None: it couldn't be read).
    image_keys: BTreeMap<String, Option<String>>,
}

impl<'a> Ctx<'a> {
    fn new(doc: Document, archive: Archive<'a>) -> Self {
        Self {
            doc,
            archive,
            warnings: Vec::new(),
            seen: BTreeSet::new(),
            left_out: 0,
            count: 0,
            mains: BTreeMap::new(),
            free_ids: BTreeMap::new(),
            instances: Vec::new(),
            style_names: BTreeMap::new(),
            image_keys: BTreeMap::new(),
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

    /// The names of `holder`'s colour, effect, text and guide styles (a document or a library).
    fn style_index(&mut self, holder: &Value) {
        for key in ["colorStyles", "effectStyles", "textStyles", "guideStyles"] {
            for s in list(holder, key) {
                if let (Some(id), Some(name)) = (text(s, "id"), text(s, "name")) {
                    self.style_names.entry(clip(id).to_string()).or_insert_with(|| clip(name).to_string());
                }
            }
        }
    }

    /// What `document.json` holds besides its pages: styles (indexed for the layers' links),
    /// and the parts not imported.
    fn document_notes(&mut self, document: &Value) {
        self.style_index(document);
        let n = |k: &str| list(document, k).len();
        let styles = n("colorStyles") + n("effectStyles") + n("textStyles") + n("guideStyles");
        if styles > 0 {
            self.warn(
                Severity::Dropped,
                "",
                format!("The document's {styles} shared styles were not imported as styles; layers keep their own values."),
            );
        }
        let variables: usize = list(document, "variableCollections").iter().map(|c| list(c, "variables").len()).sum();
        if n("variableCollections") > 0 {
            self.warn(
                Severity::Dropped,
                "",
                format!("The document's variables ({} collections, {variables} variables) were not imported.", n("variableCollections")),
            );
        }
        if n("fonts") > 0 {
            self.warn(Severity::Dropped, "", format!("{} fonts embedded in the file were not imported; text uses installed fonts.", n("fonts")));
        }
        if n("slots") > 0 {
            self.warn(Severity::Dropped, "", "The document's component slots were not imported.");
        }
        if json::int(document, "colorSpace") == Some(2) {
            self.warn(Severity::Approximated, "", "The document uses the Display P3 colour space; its colours were read as sRGB.");
        }
    }

    /// The shared libraries in `shared/` (indexed for style names; their components are
    /// imported when instances use them).
    fn libraries(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        for member in self.archive.json_members("shared/").into_iter().take(MAX_PAGES) {
            match self.archive.json(&member) {
                Ok(v) if v.is_object() => {
                    self.style_index(&v);
                    out.push(v);
                }
                Ok(_) => self.warn(Severity::Dropped, "", format!("{} is not a shared library and was not read.", clip(&member))),
                Err(e) => self.warn(Severity::Dropped, "", format!("The shared library {} couldn't be read ({e}).", clip(&member))),
            }
        }
        out
    }

    /// Every page in `pages/`, in `document.json`'s order (by page `id`), then the rest by name.
    fn pages(&mut self, document: &Value) -> Vec<Value> {
        let members = self.archive.json_members("pages/");
        if members.len() > MAX_PAGES {
            self.warn(Severity::Dropped, "", format!("The file has more than {MAX_PAGES} pages; the rest were not imported."));
        }
        let mut read = Vec::new();
        for member in members.into_iter().take(MAX_PAGES) {
            match self.archive.json(&member) {
                Ok(v) if v.is_object() => read.push(v),
                Ok(_) => self.warn(Severity::Dropped, "", format!("{} is not a FREE page and was not imported.", clip(&member))),
                Err(e) => self.warn(Severity::Dropped, "", format!("Page {} couldn't be read ({e}) and was not imported.", clip(&member))),
            }
        }
        let mut out = Vec::with_capacity(read.len());
        for id in list(document, "pages").iter().filter_map(Value::as_str) {
            match read.iter().position(|p| text(p, "id") == Some(id)) {
                Some(i) => out.push(read.remove(i)),
                None => self.warn(Severity::Dropped, "", format!("The page {} that document.json lists isn't in the file.", clip(id))),
            }
        }
        out.extend(read);
        if out.is_empty() {
            self.warn(Severity::Info, "", "The file has no pages.");
        }
        out
    }

    /// One page and its layers.
    fn page(&mut self, v: &Value, index: usize) -> NodeId {
        let name = text(v, "name").filter(|n| !n.trim().is_empty()).map_or_else(|| format!("Page {}", index + 1), |n| clip(n).to_string());
        let page = self.doc.add_page(&name);
        if let Some(n) = self.doc.node_mut(page) {
            if let NodeKind::Page { background, .. } = &mut n.kind
                && let Some(c) = json::color(v, "background").filter(|c: &Color| c.a > 0.0)
            {
                *background = c;
            }
            n.guides = layers::guides(v.get("rulers"));
        }
        if v.get("layers").is_some_and(|l| !l.is_array()) {
            self.warn(Severity::Dropped, &name, "The page's layer list is malformed; its layers were not imported.");
        }
        if !list(v, "measurements").is_empty() {
            self.warn(Severity::Dropped, &name, "The page's distance measurements were not imported.");
        }
        if json::flag(v, "componentPage") == Some(true) {
            self.warn(Severity::Info, &name, "The page is marked as Lunacy's components page; here it is an ordinary page.");
        }
        for l in json::layers(v) {
            self.layer(l, page, &name, 0, Scope::TOP);
        }
        page
    }

    /// The document image key for the image fill `name` in `images/` (bytes added with
    /// [`crate::add_image`]); None, with a warning on `path`, when it can't be read.
    pub(super) fn image_key(&mut self, name: &str, path: &str) -> Option<String> {
        let name = clip(name).to_string();
        if let Some(k) = self.image_keys.get(&name) {
            let k = k.clone();
            if k.is_none() {
                self.warn(Severity::Dropped, path, "The image couldn't be read and was not imported.");
            }
            return k;
        }
        let member =
            [format!("images/{name}"), name.clone()].into_iter().find(|m| m.starts_with("images/") && json::safe_member(m) && self.archive.has(m));
        let bytes = match member {
            Some(m) => self.archive.image(&m),
            None => Err(format!("{} is missing from the file's images", clip(&name))),
        };
        let key = match bytes.and_then(|b| crate::add_image(&mut self.doc, b).map(|(hash, _, _)| hash).map_err(|e| e.to_string())) {
            Ok(k) => Some(k),
            Err(e) => {
                self.warn(Severity::Dropped, path, format!("The image couldn't be read ({}) and was not imported.", clip(&e)));
                None
            }
        };
        self.image_keys.insert(name, key.clone());
        key
    }
}
