//! The importers in [`super::REGISTRY`]: Scrim's own documents, the earlier Scrim's, `.fig`,
//! Sketch, REST API file JSON, SVG and images. Each wraps a reader that already existed (or the
//! format crate's new ones) without changing what it makes.

use figcraft_doc::{Document, NodeId, NodeKind, TextAutoResize};
use figcraft_format::import_report::ImportWarning;
use figcraft_format::sniff::{Kind, kind};
use figcraft_format::{FormatError, Loaded};

use super::{Imported, Importer, Options, Unreadable};
use crate::{EngineError, Result};

/// The file's name without folder or extension: the new document's name when it carries none.
fn stem(name: &str) -> String {
    crate::doc_name::file_stem(name).unwrap_or_else(|| crate::doc_name::UNTITLED.to_string())
}

fn other(e: FormatError) -> EngineError {
    EngineError::Other(e.to_string())
}

/// A document from another tool: its notes as warnings.
fn imported(doc: Document, notes: Vec<String>) -> Imported {
    Imported { loaded: doc.into(), warnings: notes.iter().map(|n| ImportWarning::from_note(n)).collect() }
}

/// A Scrim document, `.scrim` or `.figcraft` (or the bare JSON both started from).
pub struct Native;
pub static NATIVE: Native = Native;

impl Importer for Native {
    fn id(&self) -> &'static str {
        "scrim"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["scrim", "figcraft"]
    }
    fn listed(&self) -> Option<&'static str> {
        Some(".scrim")
    }
    fn native(&self) -> bool {
        true
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if kind(bytes) == Kind::Native { 100 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let mut loaded = figcraft_format::load_bytes_with_versions(bytes).map_err(other)?;
        if loaded.doc.name.is_empty() {
            loaded.doc.name = stem(o.name);
        }
        Ok(Imported { loaded, warnings: Vec::new() })
    }
}

/// The earlier TypeScript Scrim's JSON `.scrim`.
pub struct OldScrim;
pub static OLD_SCRIM: OldScrim = OldScrim;

impl Importer for OldScrim {
    fn id(&self) -> &'static str {
        "scrim-earlier"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["scrim"]
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if kind(bytes) == Kind::OldScrim { 100 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let (mut doc, warnings) = figcraft_format::old_scrim::import(bytes, &stem(o.name)).map_err(|e| {
            EngineError::Unreadable(match e {
                FormatError::Newer(v) => Unreadable::OldScrimNewer(v),
                FormatError::Corrupt(why) | FormatError::Io(why) => Unreadable::OldScrimDamaged(why),
            })
        })?;
        settle(&mut doc);
        Ok(Imported { loaded: doc.into(), warnings })
    }
}

/// A file the earlier Scrim laid out with CSS: its text measured and its auto layout run, as the
/// editor does after an edit.
fn settle(doc: &mut Document) {
    let m = crate::measure::TextMeasure;
    let texts: Vec<NodeId> = doc.nodes().filter(|n| matches!(n.kind, NodeKind::Text(_))).map(|n| n.id).collect();
    for id in texts {
        let size = doc.node(id).and_then(|n| match &n.kind {
            NodeKind::Text(t) if t.auto_resize == TextAutoResize::WidthAndHeight => figcraft_text::auto_size(n),
            NodeKind::Text(_) => figcraft_layout::Measure::text_height(&m, doc, id, n.width).map(|h| (n.width, h)),
            _ => None,
        });
        if let (Some((w, h)), Some(n)) = (size, doc.node_mut(id)) {
            n.width = w;
            n.height = h;
        }
    }
    for page in doc.pages.clone() {
        figcraft_layout::layout_page(doc, page, &m);
    }
}

/// Figma's `.fig` (best effort).
pub struct Fig;
pub static FIG: Fig = Fig;

impl Importer for Fig {
    fn id(&self) -> &'static str {
        "fig"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["fig"]
    }
    fn listed(&self) -> Option<&'static str> {
        Some(".fig")
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if figcraft_format::fig::is_fig(bytes) { 100 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let loaded = crate::cmd::file::load_fig(bytes, &stem(o.name)).ok_or(EngineError::Unreadable(Unreadable::NotSupported))??;
        Ok(imported_loaded(loaded))
    }
}

fn imported_loaded(mut loaded: Loaded) -> Imported {
    let warnings = std::mem::take(&mut loaded.issues).iter().map(|n| ImportWarning::from_note(n)).collect();
    Imported { loaded, warnings }
}

/// Sketch (`.sketch`).
pub struct Sketch;
pub static SKETCH: Sketch = Sketch;

impl Importer for Sketch {
    fn id(&self) -> &'static str {
        "sketch"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["sketch"]
    }
    fn listed(&self) -> Option<&'static str> {
        Some(".sketch")
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        // FREE files are made to look like Sketch's: a surer claim (FREE's own marks) wins.
        if figcraft_format::sketch::is_sketch(bytes) { 95 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let (mut doc, warnings) = figcraft_format::sketch::import(bytes, &stem(o.name)).map_err(other)?;
        crate::cmd::file::link_instances(&mut doc);
        Ok(Imported { loaded: doc.into(), warnings })
    }
}

/// Lunacy's FREE format (`.free`): a zip of JSON like Sketch's, claimed surer than Sketch.
pub struct Free;
pub static FREE: Free = Free;

impl Importer for Free {
    fn id(&self) -> &'static str {
        "free"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["free"]
    }
    fn listed(&self) -> Option<&'static str> {
        Some(".free")
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if figcraft_format::free::is_free(bytes) { 100 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let (mut doc, warnings) = figcraft_format::free::import(bytes, &stem(o.name)).map_err(|e| match e {
            FormatError::Newer(v) => EngineError::Unreadable(Unreadable::NewerVersion { tool: "Lunacy", version: v }),
            other_error => other(other_error),
        })?;
        crate::cmd::file::link_instances(&mut doc);
        Ok(Imported { loaded: doc.into(), warnings })
    }
}

/// Penpot (`.penpot`, format v3).
pub struct Penpot;
pub static PENPOT: Penpot = Penpot;

impl Importer for Penpot {
    fn id(&self) -> &'static str {
        "penpot"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["penpot"]
    }
    fn listed(&self) -> Option<&'static str> {
        Some(".penpot")
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if figcraft_format::penpot::is_penpot(bytes) { 100 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let (mut doc, warnings) = figcraft_format::penpot::import(bytes, &stem(o.name)).map_err(|e| match e {
            FormatError::Newer(v) => EngineError::Unreadable(Unreadable::NewerVersion { tool: "Penpot", version: v }),
            other_error => other(other_error),
        })?;
        crate::cmd::file::link_instances(&mut doc);
        Ok(Imported { loaded: doc.into(), warnings })
    }
}

/// Figma's REST API file JSON (and its `nodes` endpoint).
pub struct RestJson;
pub static REST_JSON: RestJson = RestJson;

impl Importer for RestJson {
    fn id(&self) -> &'static str {
        "rest-json"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if figcraft_format::sniff::is_zip(bytes) || kind(bytes) != Kind::Other {
            return 0;
        }
        let head = bytes.get(..bytes.len().min(figcraft_format::sniff::HEAD_BYTES as usize)).unwrap_or(bytes);
        let keys = figcraft_format::sniff::top_level(head);
        let has = |k: &str| keys.iter().any(|(key, _)| key == k);
        if has("document") {
            90
        } else if has("nodes") {
            60
        } else {
            0
        }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let (mut doc, notes) = figcraft_format::rest::try_import_with(bytes, o.variables, o.images)
            .ok_or(EngineError::Unreadable(Unreadable::NotSupported))?
            .map_err(other)?;
        crate::cmd::file::link_instances(&mut doc);
        Ok(imported(doc, notes))
    }
}

/// SVG: editable vectors.
pub struct Svg;
pub static SVG: Svg = Svg;

impl Importer for Svg {
    fn id(&self) -> &'static str {
        "svg"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["svg"]
    }
    fn listed(&self) -> Option<&'static str> {
        Some(".svg")
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        if is_svg(bytes) { 90 } else { 0 }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let name = stem(o.name);
        let mut doc = Document::new(name.as_str());
        let page = doc.first_page().ok_or(EngineError::NoDocument)?;
        let notes = figcraft_svg::import_svg_with_notes(&mut doc, bytes, page, 0.0, 0.0, &name).map_err(|e| EngineError::Other(e.to_string()))?.notes;
        Ok(imported(doc, notes))
    }
}

/// Whether `bytes` are SVG: XML (after a byte order mark, an XML declaration, comments or a
/// doctype) whose first element is `<svg`.
pub fn is_svg(bytes: &[u8]) -> bool {
    let head = bytes.get(..bytes.len().min(4096)).unwrap_or(bytes);
    let text = String::from_utf8_lossy(head);
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    loop {
        if rest.starts_with("<svg") {
            return true;
        }
        let skip = ["<?", "<!--", "<!"].iter().find(|p| rest.starts_with(**p));
        let end = match skip {
            Some(&"<!--") => rest.find("-->").map(|i| i + 3),
            Some(_) => rest.find('>').map(|i| i + 1),
            None => return false,
        };
        match end.and_then(|i| rest.get(i..)) {
            Some(r) => rest = r.trim_start(),
            None => return false,
        }
    }
}

/// Images: one image layer each.
pub struct Images;
pub static IMAGES: Images = Images;

impl Importer for Images {
    fn id(&self) -> &'static str {
        "image"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["png", "jpg", "jpeg", "webp", "gif", "tif", "tiff", "bmp"]
    }
    fn sniff(&self, bytes: &[u8], _: &str) -> u8 {
        let webp = bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP");
        let magic: [&[u8]; 6] = [b"\x89PNG\r\n\x1a\n", b"\xff\xd8\xff", b"GIF87a", b"GIF89a", b"II*\0", b"MM\0*"];
        if webp || magic.iter().any(|m| bytes.starts_with(m)) {
            100
        } else if bytes.starts_with(b"BM") && bytes.len() > 26 {
            // Two letters are a weak sign: any other importer that claims the file wins.
            40
        } else {
            0
        }
    }
    fn import(&self, bytes: &[u8], o: &Options) -> Result<Imported> {
        let name = stem(o.name);
        let mut doc = Document::new(name.as_str());
        let page = doc.first_page().ok_or(EngineError::NoDocument)?;
        let (hash, w, h) = figcraft_format::add_image(&mut doc, bytes.to_vec()).map_err(other)?;
        let id = doc.create(page, NodeKind::Rectangle, &name, 0.0, 0.0, f64::from(w), f64::from(h))?;
        doc.get_mut(id)?.fills = vec![figcraft_doc::color::Paint::image(hash, figcraft_doc::color::ImageScaleMode::Fill)];
        Ok(Imported { loaded: doc.into(), warnings: Vec::new() })
    }
}
