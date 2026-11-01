//! The import registry (Fix AG4): every file Scrim opens, imports or places goes to the importer
//! whose look at its **contents** is surest, never by its name. A file's name only names the new
//! document and breaks ties. Each importer says which extensions it takes (the Open dialog, drag
//! and drop and Home's "Open .scrim, .fig…" are made from them) and reports what it couldn't bring
//! across as [`ImportWarning`]s.
//!
//! Adding an importer is one entry in [`REGISTRY`]: the UI reads everything it shows from here.

mod kinds;

use figcraft_format::import_report::ImportWarning;
use figcraft_format::rest::ImageResolver;
use serde_json::Value;

use crate::{DocState, EngineError, Result};

pub use kinds::{FIG, IMAGES, NATIVE, OLD_SCRIM, REST_JSON, SKETCH, SVG};

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
pub static REGISTRY: &[&dyn Importer] = &[&NATIVE, &OLD_SCRIM, &FIG, &SKETCH, &REST_JSON, &SVG, &IMAGES];

/// The importer for `bytes`: the one surest of them (the first on a tie), or none.
pub fn pick(bytes: &[u8], name: &str) -> Option<&'static dyn Importer> {
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
    let imp = pick(bytes, options.name).ok_or(EngineError::Unreadable(Unreadable::NotSupported))?;
    Ok((imp.import(bytes, options)?, imp))
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

/// The formats Home names, in order: ".scrim, .fig, .sketch, .svg".
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
            Unreadable::NotSupported => {
                write!(f, "Scrim can't open this file: it isn't a Scrim document, an image, or a file from another design tool Scrim imports.")
            }
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
