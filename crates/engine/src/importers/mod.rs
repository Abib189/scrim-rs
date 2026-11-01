//! The import registry (Fix AG4): every file Scrim opens, imports or places goes to the importer
//! whose look at its **contents** is surest, never by its name. A file's name only names the new
//! document and breaks ties. Each importer says which extensions it takes (the Open dialog, drag
//! and drop and Home's "Open .scrim, .fig…" are made from them) and reports what it couldn't bring
//! across as [`ImportWarning`]s.
//!
//! Adding an importer is one entry in [`REGISTRY`]: the UI reads everything it shows from here.

mod closed;
mod kinds;

use figcraft_format::import_report::ImportWarning;
use figcraft_format::rest::ImageResolver;
use serde_json::Value;

use crate::{DocState, EngineError, Result};

pub use closed::{CLOSED, ClosedFormat, closed_format};
pub use kinds::{FIG, FREE, IMAGES, NATIVE, OLD_SCRIM, PENPOT, REST_JSON, SKETCH, SVG};

/// What an import is given besides the bytes.
pub struct Options<'a> {
    /// The file's name (or a name standing for it): the new document's name when the file has none.
    pub name: &'a str,
    /// A REST API file JSON's local variables JSON.
    pub variables: Option<&'a Value>,
    /// Image bytes a REST API file JSON refers to.
    pub images: ImageResolver<'a>,
}

/// A file read into a document.
pub struct Imported {
    pub loaded: figcraft_format::Loaded,
    /// What couldn't be brought across exactly (empty for a Scrim document, whose repairs are in
    /// `loaded.issues`).
    pub warnings: Vec<ImportWarning>,
}

/// One kind of file Scrim reads.
pub trait Importer: Sync {
    /// A short stable id (`scrim`, `fig`, `sketch` …), for results and tests.
    fn id(&self) -> &'static str;
    /// The extensions it takes, without the dot, for the Open dialog and drag and drop.
    fn extensions(&self) -> &'static [&'static str];
    /// How Home and the drop hint name it (".fig"); `None` when it isn't named there (the image
    /// formats are "images"; REST API file JSON and the earlier Scrim's files aren't advertised).
    fn listed(&self) -> Option<&'static str> {
        None
    }
    /// Whether it is Scrim's own format: opened with its path, and saved back to `.scrim`.
    fn native(&self) -> bool {
        false
    }
    /// How sure it is that `bytes` are its kind of file, from 0 (not at all) to 100. `name` may
    /// break a tie between formats that look alike; it never makes a file one it isn't.
    fn sniff(&self, bytes: &[u8], name: &str) -> u8;
    /// Read `bytes` into a document.
    fn import(&self, bytes: &[u8], options: &Options) -> Result<Imported>;
}

/// Every importer, surest first where two could claim a file.
pub static REGISTRY: &[&dyn Importer] = &[&NATIVE, &OLD_SCRIM, &FIG, &SKETCH, &PENPOT, &FREE, &REST_JSON, &SVG, &IMAGES];

/// The importer for `bytes`: the one surest of them (the first on a tie), or none. A file named as
/// a closed format ([`CLOSED`]) has none: it is never opened, whatever it holds.
pub fn pick(bytes: &[u8], name: &str) -> Option<&'static dyn Importer> {
    if closed_format(name).is_some() {
        return None;
    }
    let mut best: Option<(&'static dyn Importer, u8)> = None;
    for imp in REGISTRY.iter().copied() {
        let c = imp.sniff(bytes, name);
        if c > 0 && best.is_none_or(|(_, b)| c > b) {
            best = Some((imp, c));
        }
    }
    best.map(|(imp, _)| imp)
}

/// Read `bytes` with the importer [`pick`] chooses.
pub fn load(bytes: &[u8], options: &Options) -> Result<(Imported, &'static dyn Importer)> {
    let imp = pick(bytes, options.name).ok_or_else(|| EngineError::Unreadable(unreadable(options.name, bytes)))?;
    Ok((imp.import(bytes, options)?, imp))
}

/// Why a file named `name`, holding `bytes`, that no importer takes can't be opened: a closed
/// format's; a zip that would unpack too far (Fix AI-3: each importer's check refuses such a zip,
/// so none recognises it); or neither.
pub fn unreadable(name: &str, bytes: &[u8]) -> Unreadable {
    if let Some(c) = closed_format(name) {
        return Unreadable::ClosedFormat(c.tool);
    }
    figcraft_format::zip_limits::refusal(bytes).map_or(Unreadable::NotSupported, Unreadable::Unpack)
}

/// Every extension the Open dialog and drag and drop take, in [`REGISTRY`]'s order.
pub fn open_extensions() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for e in REGISTRY.iter().flat_map(|i| i.extensions().iter().copied()) {
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// The image extensions (Place image).
pub fn image_extensions() -> Vec<&'static str> {
    IMAGES.extensions().to_vec()
}

/// The formats Home names, in order: ".scrim, .fig, .sketch, .penpot, .free, .svg".
pub fn listed() -> Vec<&'static str> {
    REGISTRY.iter().filter_map(|i| i.listed()).collect()
}

/// Why a file can't be opened, said so a person knows what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// An earlier (TypeScript) Scrim `.scrim` saved with a newer `schemaVersion` than Scrim reads.
    OldScrimNewer(u32),
    /// An earlier Scrim `.scrim` that is damaged.
    OldScrimDamaged(String),
    /// No importer recognises it.
    NotSupported,
    /// A closed format (Affinity, Adobe XD): Scrim can't read it, and never tries.
    ClosedFormat(&'static str),
    /// A file from another tool, in a version of its format newer than Scrim reads.
    NewerVersion { tool: &'static str, version: u32 },
    /// An archive that would unpack to far more than it holds, or has too many parts (a zip
    /// bomb, or a damaged file): refused before it fills memory (Fix AI-3).
    Unpack(figcraft_format::zip_limits::Unpack),
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unreadable::OldScrimNewer(v) => write!(
                f,
                "This .scrim file is from the earlier Scrim app and uses its file version {v}, newer than this Scrim can import (up to {}). Nothing was changed.",
                figcraft_format::old_scrim::SCHEMA_VERSION
            ),
            Unreadable::OldScrimDamaged(why) => {
                write!(f, "This .scrim file is from the earlier Scrim app, and it is damaged ({why}). Nothing was changed.")
            }
            Unreadable::NewerVersion { tool, version } => {
                write!(f, "This {tool} file uses version {version} of its format, newer than Scrim can import. Nothing was changed.")
            }
            Unreadable::ClosedFormat(tool) => {
                write!(f, "{tool} files use a closed format, so Scrim can't read them. In {tool}, export the design as SVG, then open the SVG here.")
            }
            Unreadable::NotSupported => {
                write!(f, "Scrim can't open this file: it isn't a Scrim document, an image, or a file from another design tool Scrim imports.")
            }
            Unreadable::Unpack(u) => write!(f, "Scrim didn't open this file: {u}. Nothing was changed."),
        }
    }
}

impl DocState {
    /// With the version history read from its file.
    pub fn with_versions(mut self, versions: Vec<figcraft_format::Version>) -> Self {
        self.versions = versions;
        self
    }

    /// With the branches read from its file.
    pub fn with_branches(mut self, branches: Vec<figcraft_format::Branch>) -> Self {
        self.branches = branches;
        self
    }
}

/// `warnings` as a command result gives them: `[{severity, path, message}]`.
pub fn warnings_json(warnings: &[ImportWarning]) -> Value {
    serde_json::to_value(warnings).unwrap_or(Value::Array(Vec::new()))
}

#[cfg(test)]
#[path = "../importers_tests.rs"]
mod tests;
