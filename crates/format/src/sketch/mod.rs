//! Sketch (`.sketch`) import: a zip of JSON documents (Fix AG5).
//!
//! Written from Sketch's published file format only: the JSON Schema of
//! `@sketch-hq/sketch-file-format` 6.5.0 (document versions 121–146,
//! <https://github.com/sketch-hq/sketch-file-format>, served as
//! <https://unpkg.com/@sketch-hq/sketch-file-format@6.5.0/dist/file-format.schema.json>) and the
//! pages at <https://developer.sketch.com/file-format/> ("File format": `document.json` links to
//! one JSON file per page in `pages/`, bitmaps are in `images/`; "Coordinate systems": a layer's
//! `frame` is relative to its parent, and curve points are normalised 0..1 within the frame).
//! Nothing undocumented was reverse-engineered; where the schema names a field without saying
//! how to read it, the reading chosen is stated here and in the code:
//!
//! - `rotation` is in degrees, counter-clockwise on screen, about the frame's centre;
//!   `isFlippedHorizontal` / `isFlippedVertical` mirror the layer about its centre before it is
//!   rotated.
//! - `curveFrom` is the control point leaving a curve point, `curveTo` the one arriving at it,
//!   normalised like `point`; gradient `from` / `to` are normalised to the layer's box too.
//! - The schema's `MarkerType` lists 0 as "Open arrow", yet every style carries a marker type:
//!   0 is read as no line ending, 1–6 as listed.
//! - Text ranges (`location`, `length`) count UTF-16 code units; font names are PostScript names
//!   ("Inter-SemiBold"), split into a family and a style.
//!
//! Fields read: pages (`document.json` › `pages`, `pages/<id>.json`), the layer tree
//! (`artboard`, `symbolMaster`, `group`, `shapeGroup`, `rectangle`, `oval`, `polygon`, `star`,
//! `triangle`, `shapePath`, `text`, `bitmap`, `symbolInstance`, `slice`), `frame`, `rotation`,
//! flips, `isVisible`, `isLocked`, `hasClippingMask`, `booleanOperation`, `style` (`fills`,
//! `borders`, `borderOptions`, `miterLimit`, `startMarkerType` / `endMarkerType`, `shadows`,
//! `innerShadows`, `blur`, `contextSettings`, `windingRule`, `textStyle`), curve points,
//! `fixedRadius`, `numberOfPoints`, `radius`, `attributedString`, `textBehaviour`, images
//! (`MSJSONFileReference` into `images/`, `MSJSONOriginalDataReference` inline), `symbolID`,
//! `overrideValues` (`_stringValue`, `_image`), `exportOptions`, and the document's foreign
//! symbols and shared style names. Everything else a file carries is listed in the import
//! warnings with the layer it was on (see [`crate::import_report`]); nothing is dropped silently.
//! `user.json` (window and zoom state) and previews hold no design and are not read. Hostile
//! input gives an error or warnings, never a panic; members are read within size caps.

mod images;
mod json;
mod layers;
mod shapes;
mod style;
mod symbols;
#[cfg(test)]
mod tests;
mod text;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use figcraft_doc::{Document, NodeId};
use serde_json::Value;

use crate::FormatError;
use crate::import_report::{ImportWarning, Severity};
use json::text;

/// Largest `document.json`, `meta.json` or page JSON read.
const MAX_JSON: u64 = 256 * 1024 * 1024;
/// Most JSON bytes read from one file, all members together.
const JSON_BUDGET: u64 = 1024 * 1024 * 1024;
/// Largest image read.
const MAX_IMAGE: u64 = 128 * 1024 * 1024;
/// Most images, and image bytes together, read from one file.
const MAX_IMAGES: usize = 10_000;
const IMAGE_BUDGET: u64 = 1024 * 1024 * 1024;
/// How much of `document.json` [`is_sketch`] reads.
const SNIFF_JSON: u64 = 64 * 1024 * 1024;
/// Most layers imported (instances' copies of their main included).
const MAX_NODES: usize = 200_000;
/// Deepest layer nesting followed.
const MAX_DEPTH: usize = 256;
/// Most pages read.
const MAX_PAGES: usize = 1_000;
/// Most warnings kept; one more says how many were left out.
const MAX_WARNINGS: usize = 1_000;

/// Whether `bytes` are a Sketch document: a zip holding `document.json` with `"_class": "document"`
/// and `meta.json`.
pub fn is_sketch(bytes: &[u8]) -> bool {
    if !(bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")) {
        return false;
    }
    let Ok(mut z) = zip::ZipArchive::new(Cursor::new(bytes)) else { return false };
    if z.index_for_name("meta.json").is_none() {
        return false;
    }
    let Ok(data) = crate::read_member(&mut z, "document.json", SNIFF_JSON) else { return false };
    #[derive(serde::Deserialize)]
    struct Head {
        #[serde(rename = "_class")]
        class: Option<String>,
    }
    serde_json::from_slice::<Head>(&data).ok().and_then(|h| h.class).as_deref() == Some("document")
}

/// The Sketch document `bytes` as a new document, with what couldn't be brought across. `name`:
/// the document name when the file doesn't carry one.
pub fn import(bytes: &[u8], name: &str) -> Result<(Document, Vec<ImportWarning>), FormatError> {
    let z = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| FormatError::Corrupt(format!("not a Sketch file: {e}")))?;
    let mut archive = Archive { zip: z, json_left: JSON_BUDGET, images: 0, image_bytes: 0 };
    let document = archive.json("document.json")?;
    if text(&document, "_class") != Some("document") {
        return Err(FormatError::Corrupt("not a Sketch file: document.json is not a Sketch document".into()));
    }
    let meta = archive.json("meta.json")?;
    let title = if name.trim().is_empty() { "Imported Sketch file" } else { name };
    let mut cx = Ctx::new(Document::new(json::clip(title)), archive);
    cx.meta(&meta);
    cx.shared_styles(&document);
    let blank = cx.doc.first_page();
    let pages = cx.page_list(&document);
    for (i, page) in pages.iter().enumerate() {
        match page {
            PageSource::Inline(v) => cx.page(v, i),
            PageSource::File(member) => match cx.archive.json(member) {
                Ok(v) if text(&v, "_class") == Some("page") => cx.page(&v, i),
                Ok(_) => cx.warn(Severity::Dropped, "", format!("{} is not a Sketch page and was not imported.", json::clip(member))),
                Err(e) => cx.warn(Severity::Dropped, "", format!("Page {} couldn't be read ({e}) and was not imported.", json::clip(member))),
            },
        }
    }
    cx.foreign_symbols(&document);
    cx.finish_instances();
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

/// Where a page's JSON is.
enum PageSource {
    /// Embedded in `document.json` (the expanded form the schema also describes).
    Inline(Value),
    /// An archive member (`pages/<id>.json`).
    File(String),
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

    /// Image member `name`, within the image caps.
    fn image(&mut self, name: &str) -> Result<Vec<u8>, String> {
        if self.images >= MAX_IMAGES {
            return Err(format!("the file has more than {MAX_IMAGES} images"));
        }
        let limit = MAX_IMAGE.min(IMAGE_BUDGET.saturating_sub(self.image_bytes));
        let data = crate::read_member(&mut self.zip, name, limit).map_err(|e| match e {
            FormatError::Corrupt(m) | FormatError::Io(m) => m,
            FormatError::Newer(_) => "unreadable".into(),
        })?;
        self.images += 1;
        self.image_bytes = self.image_bytes.saturating_add(data.len() as u64);
        Ok(data)
    }

    /// Count `len` bytes of an inline image against the image caps; false when they don't fit.
    fn take_inline(&mut self, len: usize) -> bool {
        let len = len as u64;
        if self.images >= MAX_IMAGES || len > MAX_IMAGE || self.image_bytes.saturating_add(len) > IMAGE_BUDGET {
            return false;
        }
        self.images += 1;
        self.image_bytes += len;
        true
    }
}

/// A symbol instance waiting for its main component (every page is read first).
pub(crate) struct Pending {
    id: NodeId,
    symbol: String,
    overrides: Vec<Value>,
    path: String,
}

/// The import in progress.
pub(crate) struct Ctx<'a> {
    doc: Document,
    archive: Archive<'a>,
    warnings: Vec<ImportWarning>,
    seen: BTreeSet<(Severity, String, String)>,
    left_out: usize,
    count: usize,
    /// `symbolID` → the main component imported for it.
    masters: BTreeMap<String, NodeId>,
    /// Layers imported from the file → their `do_objectID` (overrides name layers by it).
    sketch_ids: BTreeMap<NodeId, String>,
    instances: Vec<Pending>,
    /// Shared style ids of the document → their names.
    style_names: BTreeMap<String, String>,
    /// Image reference → document image key (None: it couldn't be read).
    image_keys: BTreeMap<String, Option<String>>,
    /// How many layers set resizing constraints (reported once, for the file).
    constraints: usize,
    /// Mains whose background shows on their instances (`includeBackgroundColorInInstance`).
    instance_backgrounds: BTreeSet<NodeId>,
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
            masters: BTreeMap::new(),
            sketch_ids: BTreeMap::new(),
            instances: Vec::new(),
            style_names: BTreeMap::new(),
            image_keys: BTreeMap::new(),
            constraints: 0,
            instance_backgrounds: BTreeSet::new(),
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

    /// Notes on `meta.json`: a document version the spec followed doesn't cover.
    fn meta(&mut self, meta: &Value) {
        match meta.get("version").and_then(Value::as_u64) {
            Some(v) if (121..=146).contains(&v) => {}
            Some(v) => self.warn(
                Severity::Info,
                "",
                format!("The file is document version {v}; this import follows the published spec for versions 121 to 146."),
            ),
            None => self.warn(Severity::Info, "", "meta.json doesn't give a document version."),
        }
    }

    /// The names of the document's shared layer and text styles (local and from libraries).
    fn shared_styles(&mut self, document: &Value) {
        let mut add = |s: &Value| {
            if let (Some(id), Some(name)) = (text(s, "do_objectID"), text(s, "name")) {
                self.style_names.entry(json::clip(id).to_string()).or_insert_with(|| json::clip(name).to_string());
            }
        };
        for key in ["layerStyles", "layerTextStyles"] {
            for s in json::list(document.get(key).unwrap_or(&Value::Null), "objects") {
                add(s);
            }
        }
        for key in ["foreignLayerStyles", "foreignTextStyles"] {
            for f in json::list(document, key) {
                if let Some(s) = f.get("localSharedStyle") {
                    add(s);
                }
            }
        }
    }

    /// The pages `document.json` lists, in order: references to `pages/` members or pages
    /// embedded in it. Without a list, every `pages/*.json` member, by name.
    fn page_list(&mut self, document: &Value) -> Vec<PageSource> {
        let mut out = Vec::new();
        let listed = document.get("pages").and_then(Value::as_array);
        for p in listed.into_iter().flatten() {
            if out.len() >= MAX_PAGES {
                self.warn(Severity::Dropped, "", format!("The file has more than {MAX_PAGES} pages; the rest were not imported."));
                break;
            }
            if text(p, "_class") == Some("page") {
                out.push(PageSource::Inline(p.clone()));
                continue;
            }
            let Some(r) = text(p, "_ref").filter(|r| json::safe_member(r)) else {
                self.warn(Severity::Dropped, "", "A page reference without a valid file name was not imported.");
                continue;
            };
            let member = if self.archive.has(r) { r.to_string() } else { format!("{r}.json") };
            out.push(PageSource::File(member));
        }
        if listed.is_none() {
            let mut names: Vec<String> =
                self.archive.zip.file_names().filter(|n| n.starts_with("pages/") && n.ends_with(".json")).map(str::to_string).collect();
            names.sort();
            names.truncate(MAX_PAGES);
            out.extend(names.into_iter().map(PageSource::File));
        }
        if out.is_empty() {
            self.warn(Severity::Info, "", "The file has no pages.");
        }
        out
    }

    /// One page and its layers.
    fn page(&mut self, v: &Value, index: usize) {
        let name = text(v, "name").filter(|n| !n.trim().is_empty()).map_or_else(|| format!("Page {}", index + 1), |n| json::clip(n).to_string());
        let page = self.doc.add_page(&name);
        if v.get("layers").is_some_and(|l| !l.is_array()) {
            self.warn(Severity::Dropped, &name, "The page's layer list is malformed; its layers were not imported.");
        }
        for (key, what) in [("grid", "The page grid"), ("layout", "The page layout grid")] {
            if v.get(key).and_then(|g| g.get("isEnabled")).and_then(Value::as_bool) == Some(true) {
                self.warn(Severity::Dropped, &name, format!("{what} was not imported."));
            }
        }
        for l in json::layers(v) {
            self.layer(l, page, &name, 0, false);
        }
    }

    /// File-wide notes gathered while importing.
    fn finish_file(&mut self) {
        if self.constraints > 0 {
            let n = self.constraints;
            let (s, verb) = if n == 1 { ("", "sets") } else { ("s", "set") };
            self.warn(
                Severity::Dropped,
                "",
                format!("{n} layer{s} {verb} resizing constraints, which weren't imported: the published spec doesn't describe their bits."),
            );
        }
    }
}
