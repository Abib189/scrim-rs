//! The file's library and file-wide notes: components (their main instances become Scrim
//! components), library colours (paint styles), typographies and design tokens (reported), the
//! manifest's other files and library links, and what was counted while importing layers.

use figcraft_doc::{Style, StyleKind};
use serde_json::Value;

use super::Ctx;
use super::read::{clip, id_at, list, present, safe_id, text};
use crate::import_report::Severity;

/// Most library entries (components, colours) read.
const MAX_ENTRIES: usize = 20_000;
/// Most layer paths named in a file-wide note.
const NAMED: usize = 3;

impl Ctx<'_> {
    /// The JSON members `files/{file}/{folder}/{id}.json`, by name, at most [`MAX_ENTRIES`].
    fn entries(&mut self, folder: &str) -> Vec<String> {
        let prefix = format!("files/{}/{folder}/", self.file_id);
        let mut out: Vec<String> = self
            .archive
            .names()
            .filter(|n| n.strip_prefix(&prefix).and_then(|r| r.strip_suffix(".json")).is_some_and(safe_id))
            .map(str::to_string)
            .collect();
        out.sort();
        if out.len() > MAX_ENTRIES {
            out.truncate(MAX_ENTRIES);
            self.warn(Severity::Dropped, "", format!("Only the first {MAX_ENTRIES} entries of the library's {folder} were read."));
        }
        out
    }

    /// Components: `mainInstanceId` → the component's name (its shape becomes a component).
    pub(super) fn components(&mut self) {
        for member in self.entries("components") {
            let c = match self.archive.json(&member) {
                Ok(c) => c,
                Err(e) => {
                    let m = format!("A component ({}) couldn't be read: {}", clip(&member), super::archive::message(&e));
                    self.warn(Severity::Dropped, "", m);
                    continue;
                }
            };
            let name = text(&c, "name").filter(|n| !n.is_empty()).map_or("Component", clip).to_string();
            if let Some(id) = id_at(&c, "id") {
                self.component_names.insert(id.to_string(), name.clone());
            }
            match id_at(&c, "mainInstanceId") {
                Some(main) => {
                    self.mains.entry(main.to_string()).or_insert(name);
                }
                None => self.warn(Severity::Dropped, "", format!("The component “{name}” names no main instance and was not imported.")),
            }
        }
    }

    /// Library colours as paint styles. Reading: a colour's `path` is its folder, joined to its
    /// name with “/” as Scrim's style names are.
    pub(super) fn colors(&mut self) {
        for member in self.entries("colors") {
            let c = match self.archive.json(&member) {
                Ok(c) => c,
                Err(e) => {
                    let m = format!("A library colour ({}) couldn't be read: {}", clip(&member), super::archive::message(&e));
                    self.warn(Severity::Dropped, "", m);
                    continue;
                }
            };
            let name = text(&c, "name").filter(|n| !n.trim().is_empty()).map_or("Colour", clip).trim().to_string();
            let name = match text(&c, "path").map(|p| clip(p).trim().trim_matches('/').trim()).filter(|p| !p.is_empty()) {
                Some(p) => format!("{p}/{name}"),
                None => name,
            };
            let label = format!("Library colour “{name}”");
            let Some(paint) = self.color_paint(&c, 1.0, 1.0, &label, "colour") else { continue };
            let id = self.doc.new_style_id();
            self.doc.styles.push(Style {
                id,
                name,
                description: String::new(),
                kind: StyleKind::Paint { paints: vec![paint] },
                bound: Default::default(),
            });
        }
    }

    /// Typographies and design tokens: reported, not imported.
    pub(super) fn unread_library(&mut self) {
        let n = self.entries("typographies").len();
        if n > 0 {
            let s = if n == 1 { "y" } else { "ies" };
            self.warn(
                Severity::Dropped,
                "",
                format!("{n} typograph{s} (text styles) weren't imported: text can't be read from what the documentation describes."),
            );
        }
        if self.archive.has(&format!("files/{}/tokens.json", self.file_id)) {
            self.warn(Severity::Dropped, "", "Design tokens (tokens.json) weren't imported: the documentation doesn't describe their structure.");
        }
    }

    /// The manifest's other files and library links.
    pub(super) fn manifest_notes(&mut self, manifest: &Value) {
        let files = list(manifest, "files");
        if files.len() > 1 {
            let first = files.first().and_then(|f| text(f, "name")).map_or("the first one", clip).to_string();
            let m = format!("The export holds {} files; only the first, “{first}”, was imported.", files.len());
            self.warn(Severity::Dropped, "", m);
        }
        if present(manifest, "relations") {
            self.warn(Severity::Info, "", "Links between the exported files and their shared libraries weren't kept.");
        }
    }

    /// File-wide notes gathered while importing layers.
    pub(super) fn finish_file(&mut self) {
        let missing: Vec<String> = self.mains.iter().filter(|(id, _)| !self.found_mains.contains(*id)).map(|(_, n)| n.clone()).collect();
        for name in missing {
            let m = format!(
                "The component “{name}” wasn't imported: its main instance isn't on the file's pages (the shapes a deleted component keeps aren't described by the documentation)."
            );
            self.warn(Severity::Dropped, "", m);
        }
        let paths = std::mem::take(&mut self.constraints);
        if !paths.is_empty() {
            let named: Vec<&str> = paths.iter().take(NAMED).map(String::as_str).collect();
            let more = if paths.len() > NAMED { ", …" } else { "" };
            let (s, verb) = if paths.len() == 1 { ("", "sets") } else { ("s", "set") };
            let m = format!(
                "{} layer{s} {verb} resizing constraints, which weren't imported: the documentation doesn't describe their values ({}{more}).",
                paths.len(),
                named.join("; ")
            );
            self.warn(Severity::Dropped, "", m);
        }
        if self.instance_parts > 0 {
            let n = self.instance_parts;
            let m = format!(
                "{n} layer{} inside instances {} to their component's layers (shapeRef, touched); the links weren't kept.",
                plural(n),
                verb(n, "links", "link")
            );
            self.warn(Severity::Info, "", m);
        }
        if self.unread_transforms > 0 {
            let n = self.unread_transforms;
            let m = format!(
                "{n} layer{} {} a transform that isn't the documented list of six numbers; it wasn't read.",
                plural(n),
                verb(n, "has", "have")
            );
            self.warn(Severity::Approximated, "", m);
        }
        let keys = std::mem::take(&mut self.unknown_keys);
        if !keys.is_empty() {
            let names: Vec<String> = keys.iter().take(20).map(|(k, n)| format!("{k} ({n})")).collect();
            let more = if keys.len() > 20 { ", …" } else { "" };
            let m = format!("Layer attributes the documentation doesn't list weren't imported: {}{more}.", names.join(", "));
            self.warn(Severity::Dropped, "", m);
        }
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn verb(n: usize, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 { one } else { many }
}
