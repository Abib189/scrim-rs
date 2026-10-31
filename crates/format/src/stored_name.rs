//! The name a saved file gives its document, read without opening the file (Fix AF): the recent
//! files list it next to the files' own names. Only the start of the document is read, so listing
//! twenty files stays quick however large they are; a name found nowhere there gives `None`, and
//! the file then goes by its file name.

use std::io::{Read, Seek};
use std::path::Path;

/// How much of a document's JSON is read for its name. Scrim writes it after the format, the version
/// and the image index (a line per image), well within this.
const HEAD_BYTES: u64 = 64 * 1024;
/// Largest `.fig` `meta.json` read for its `file_name`.
const MAX_META_BYTES: u64 = 1024 * 1024;

/// The document name stored in the file at `path`: `name` in a `.figcraft` file's `document.json`
/// (or in a JSON file), `file_name` in a `.fig` file's `meta.json`. `None` for other kinds of file,
/// a missing, unreadable or damaged one, one without a name, or one whose name comes after its first
/// [`HEAD_BYTES`] (a large REST API file JSON keeps it after the whole document).
pub fn stored_name(path: &Path) -> Option<String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "figcraft" | "fig" | "json") {
        return None;
    }
    let mut file = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    if ext != "json"
        && let Ok(mut z) = zip::ZipArchive::new(&mut file)
    {
        return archive_name(&mut z);
    }
    file.rewind().ok()?;
    json_name(file)
}

fn archive_name<R: Read + Seek>(z: &mut zip::ZipArchive<R>) -> Option<String> {
    if z.file_names().any(|n| n == "document.json") {
        return json_name(z.by_name("document.json").ok()?);
    }
    let meta: serde_json::Value = serde_json::from_slice(&crate::read_member(z, "meta.json", MAX_META_BYTES).ok()?).ok()?;
    meta.get("file_name")?.as_str().map(str::to_string)
}

/// `name` at the top level of the JSON object `r` reads, when it comes within its first
/// [`HEAD_BYTES`]: the keys before it are skipped over, never parsed into values.
fn json_name(r: impl Read) -> Option<String> {
    let mut head = Vec::new();
    r.take(HEAD_BYTES).read_to_end(&mut head).ok()?;
    let mut rest = head.trim_ascii_start().strip_prefix(b"{")?;
    loop {
        let (key, n) = value::<String>(rest.trim_ascii_start())?;
        rest = rest.trim_ascii_start().get(n..)?.trim_ascii_start().strip_prefix(b":")?.trim_ascii_start();
        if key == "name" {
            return value::<String>(rest).map(|(name, _)| name);
        }
        let (_, n) = value::<serde::de::IgnoredAny>(rest)?;
        rest = rest.get(n..)?.trim_ascii_start().strip_prefix(b",")?;
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
#[path = "stored_name_tests.rs"]
mod tests;
