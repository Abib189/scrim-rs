//! What a file is, from its first bytes (Fix AG): a Scrim document (`.scrim`, or `.figcraft` from
//! before), the earlier TypeScript Scrim's JSON `.scrim`, or neither. Only the start of a file is
//! read: a zip's `document.json` up to [`HEAD_BYTES`], a JSON file's first [`HEAD_BYTES`].

use std::io::{Cursor, Read};

/// How much of a document's JSON is read to tell what it is.
pub const HEAD_BYTES: u64 = 64 * 1024;

/// What a file's contents say it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A Scrim document: a zip whose `document.json` carries `"format": "figcraft"` (the format's
    /// name since FigCraft, kept for `.scrim`) or, from before that key, `pages`; or that JSON bare.
    Native,
    /// The earlier TypeScript Scrim's `.scrim`: one JSON object with `schemaVersion` and `rootIds`.
    OldScrim,
    /// Anything else.
    Other,
}

/// What `bytes` are (see [`Kind`]).
pub fn kind(bytes: &[u8]) -> Kind {
    if is_zip(bytes) {
        return match document_json_head(bytes) {
            Some(head) if native_keys(&top_level(&head)) => Kind::Native,
            _ => Kind::Other,
        };
    }
    let head = bytes.get(..bytes.len().min(HEAD_BYTES as usize)).unwrap_or(bytes);
    let keys = top_level(head);
    let has = |k: &str| keys.iter().any(|(key, _)| key == k);
    if has("schemaVersion") && has("rootIds") && !has("pages") {
        Kind::OldScrim
    } else if native_keys(&keys) {
        Kind::Native
    } else {
        Kind::Other
    }
}

/// Whether `bytes` start like a zip archive (a local file header, or an empty archive's end record).
pub fn is_zip(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
}

/// The keys of a Scrim document: `format` naming it, or `pages` and `nodes` with no other tool's
/// marks (Sketch's `_class`, the REST API's `document`, the old Scrim's `rootIds`).
fn native_keys(keys: &[(String, Option<String>)]) -> bool {
    let has = |k: &str| keys.iter().any(|(key, _)| key == k);
    let format = keys.iter().find(|(k, _)| k == "format").and_then(|(_, v)| v.as_deref());
    match format {
        Some(f) => f == crate::FORMAT,
        None => has("pages") && !has("_class") && !has("document") && !has("rootIds"),
    }
}

/// The first [`HEAD_BYTES`] of a zip's `document.json`.
fn document_json_head(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    if z.file_names().any(|n| n == "canvas.fig") {
        return None;
    }
    let mut head = Vec::new();
    z.by_name("document.json").ok()?.take(HEAD_BYTES).read_to_end(&mut head).ok()?;
    Some(head)
}

/// The keys at the top level of the JSON object `json` starts, in order, each with its value when
/// that is a string; the other values are skipped over, never built. Stops at the end of the
/// object or of `json` (a cut-off head gives the keys before the cut).
pub fn top_level(json: &[u8]) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let Some(mut rest) = json.trim_ascii_start().strip_prefix(b"{") else { return out };
    loop {
        let Some((key, n)) = value::<String>(rest.trim_ascii_start()) else { return out };
        let Some(after) = rest.trim_ascii_start().get(n..).and_then(|r| r.trim_ascii_start().strip_prefix(b":")) else { return out };
        rest = after.trim_ascii_start();
        let text = if rest.starts_with(b"\"") { value::<String>(rest).map(|(s, _)| s) } else { None };
        out.push((key, text));
        let Some((_, n)) = value::<serde::de::IgnoredAny>(rest) else { return out };
        match rest.get(n..).and_then(|r| r.trim_ascii_start().strip_prefix(b",")) {
            Some(next) => rest = next,
            None => return out,
        }
    }
}

/// The JSON value `json` starts with and how many bytes it takes (`None` when it doesn't parse as
/// a `T`, or is cut off).
fn value<T: serde::de::DeserializeOwned>(json: &[u8]) -> Option<(T, usize)> {
    let mut values = serde_json::Deserializer::from_slice(json).into_iter::<T>();
    let v = values.next()?.ok()?;
    Some((v, values.byte_offset()))
}

#[cfg(test)]
#[path = "sniff_tests.rs"]
mod tests;
