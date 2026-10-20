//! Fonts a document names that this machine doesn't have (#752): text layers, their styled
//! ranges, text styles and the string variables bound to a font family. Text in such a family
//! is drawn in a fallback font, so the user is told which ones are missing.

use std::collections::BTreeMap;

use figcraft_doc::{Document, NodeKind, StyleKind, VariableValue, Variables};
use serde_json::{Value, json};

use crate::cmd::CommandSpec;
use crate::{Result, Session, spec};

/// A family the document names that no installed or built-in font provides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    /// As the document spells it.
    pub family: String,
    /// Text layers that use it (0: only a text style or a variable names it).
    pub layers: usize,
}

/// The families `doc` names that the font database doesn't have, by name. On the desktop the
/// installed fonts are indexed in the background: ask once that's done
/// (`figcraft_text::installed_fonts_known`), or every installed family counts as missing.
pub fn missing(doc: &Document) -> Vec<Missing> {
    // Lower-case family → (spelling, layers).
    let mut used: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut add = |family: &str, layer: bool| {
        let family = family.trim();
        if family.is_empty() {
            return;
        }
        let e = used.entry(family.to_lowercase()).or_insert_with(|| (family.to_string(), 0));
        e.1 += usize::from(layer);
    };
    for n in doc.nodes() {
        if let NodeKind::Text(t) = &n.kind {
            let mut families: Vec<&str> = t.spans().into_iter().map(|(_, _, st)| st.font_family.as_str()).collect();
            families.push(&t.style.font_family);
            families.sort_by_key(|f| f.trim().to_lowercase());
            families.dedup_by_key(|f| f.trim().to_lowercase());
            for f in families {
                add(f, true);
            }
        }
        if let Some(var) = n.bound.get("fontFamily") {
            for f in string_values(&doc.variables, var) {
                add(&f, false);
            }
        }
    }
    for st in &doc.styles {
        if let StyleKind::Text { style } = &st.kind {
            add(&style.font_family, false);
        }
        if let Some(var) = st.bound.get("fontFamily") {
            for f in string_values(&doc.variables, var) {
                add(&f, false);
            }
        }
    }
    used.into_values().filter(|(family, _)| !figcraft_text::has_family(family)).map(|(family, layers)| Missing { family, layers }).collect()
}

/// Every string value of variable `id` in any mode, following aliases (at most 32 deep).
fn string_values(vars: &Variables, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![(id.to_string(), 0)];
    while let Some((id, depth)) = todo.pop() {
        let Some(var) = vars.get(&id) else { continue };
        for v in var.values.values() {
            match v {
                VariableValue::String(s) => out.push(s.clone()),
                VariableValue::Alias(a) if depth < 32 => todo.push((a.clone(), depth + 1)),
                _ => {}
            }
        }
    }
    out
}

fn missing_fonts(s: &mut Session, _: &Value) -> Result<Value> {
    let families: Vec<Value> = missing(s.doc()?).into_iter().map(|m| json!({"family": m.family, "layers": m.layers})).collect();
    s.request_ui("dialog.missingFonts", json!({}));
    Ok(json!({"families": families}))
}

/// The file the Missing fonts window is about: `path`, or the open document's.
fn file_path(s: &Session, p: &Value, cmd: &str) -> Result<String> {
    match p.get("path").and_then(Value::as_str) {
        Some(path) if !path.trim().is_empty() => Ok(path.to_string()),
        _ => s.ds()?.path.clone().ok_or_else(|| crate::bad(cmd, "the file isn't saved yet: there is no file to remember")),
    }
}

/// `fonts.muteMissing`: the Missing fonts window doesn't open by itself for this file (or does
/// again, with `muted: false`).
fn mute_missing(s: &mut Session, p: &Value) -> Result<Value> {
    let path = file_path(s, p, "fonts.muteMissing")?;
    let muted = p.get("muted").and_then(Value::as_bool).unwrap_or(true);
    let list = &mut s.prefs.settings.fonts_muted;
    list.retain(|f| f != &path);
    if muted {
        list.push(path.clone());
    }
    s.prefs.settings = std::mem::take(&mut s.prefs.settings).sanitized();
    Ok(json!({"path": path, "muted": muted}))
}

/// Whether the Missing fonts window stays closed for the file at `path`.
pub fn muted(prefs: &crate::Prefs, path: Option<&str>) -> bool {
    path.is_some_and(|p| prefs.settings.fonts_muted.iter().any(|m| m == p))
}

/// `fonts.copyMissingNames`: the missing families' names, one per line, onto the clipboard.
fn copy_missing_names(s: &mut Session, _: &Value) -> Result<Value> {
    let families: Vec<String> = missing(&s.ds()?.doc).into_iter().map(|m| m.family).collect();
    s.request_ui(crate::cmd::code::COPY_REQUEST, json!({"text": families.join("\n"), "what": "the font names"}));
    Ok(json!({"families": families}))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "text.missingFonts",
            "Missing fonts",
            "{}  fonts the open document names that aren't installed (text layers, styled ranges, text styles, font family variables); the app shows them → {families: [{family, layers}]}",
            false,
            missing_fonts
        ),
        spec!(
            "fonts.muteMissing",
            "Don't show missing fonts again for this file",
            "{path? (default: the open file), muted? (default true; false shows it again)} → {path, muted}",
            false,
            mute_missing
        ),
        spec!(
            "fonts.copyMissingNames",
            "Copy missing font names",
            "{} → {families}  (one per line, onto the clipboard in the app)",
            false,
            copy_missing_names
        ),
    ]
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
