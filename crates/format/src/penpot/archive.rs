//! The `.penpot` zip: its member names, JSON members within size caps, and the binaries of
//! storage objects (`objects/{uuid}.{ext}`, with their `objects/{uuid}.json` metadata).

use std::collections::BTreeMap;
use std::io::Cursor;

use serde_json::Value;

use super::read::{safe_id, text};
use super::{IMAGE_BUDGET, JSON_BUDGET, MAX_IMAGE, MAX_IMAGES, MAX_JSON};
use crate::FormatError;

/// Largest storage-object metadata member read.
const MAX_OBJECT_META: u64 = 64 * 1024;

/// The zip, with what is left of the read budgets.
pub(super) struct Archive<'a> {
    zip: zip::ZipArchive<Cursor<&'a [u8]>>,
    json_left: u64,
    images: usize,
    image_bytes: u64,
    /// Storage object id → its binary member (`objects/{id}.{ext}`, the first one listed).
    objects: BTreeMap<String, String>,
}

impl<'a> Archive<'a> {
    pub(super) fn open(bytes: &'a [u8]) -> Result<Self, FormatError> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| FormatError::Corrupt(format!("not a Penpot file: {e}")))?;
        crate::zip_limits::check(&mut zip, bytes.len() as u64)?;
        let mut objects = BTreeMap::new();
        for name in zip.file_names() {
            let Some(rest) = name.strip_prefix("objects/") else { continue };
            let Some((id, ext)) = rest.split_once('.') else { continue };
            if ext == "json" || !safe_id(id) || ext.is_empty() || ext.len() > 16 || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
                continue;
            }
            objects.entry(id.to_string()).or_insert_with(|| name.to_string());
        }
        Ok(Self { zip, json_left: JSON_BUDGET, images: 0, image_bytes: 0, objects })
    }

    /// Every member name, in archive order.
    pub(super) fn names(&self) -> impl Iterator<Item = &str> {
        self.zip.file_names()
    }

    /// Whether storage object `id` has a binary in the file.
    pub(super) fn has_object(&self, id: &str) -> bool {
        self.objects.contains_key(id)
    }

    pub(super) fn has(&self, name: &str) -> bool {
        self.zip.index_for_name(name).is_some()
    }

    /// Member `name` parsed as JSON, within [`MAX_JSON`] and the file's JSON budget.
    pub(super) fn json(&mut self, name: &str) -> Result<Value, FormatError> {
        self.json_within(name, MAX_JSON)
    }

    /// Member `name` parsed as JSON, at most `limit` bytes (and within the JSON budget).
    pub(super) fn json_within(&mut self, name: &str, limit: u64) -> Result<Value, FormatError> {
        let limit = limit.min(self.json_left);
        let data = crate::read_member(&mut self.zip, name, limit)?;
        self.json_left = self.json_left.saturating_sub(data.len() as u64);
        serde_json::from_slice(&data).map_err(|e| FormatError::Corrupt(format!("{name}: {e}")))
    }

    /// The bytes of storage object `id` (`objects/{id}.{ext}`), within the image caps. When
    /// `objects/{id}.json` gives a `contentType`, it must be an image type.
    pub(super) fn object(&mut self, id: &str) -> Result<Vec<u8>, String> {
        if !safe_id(id) {
            return Err("its id isn't a valid storage object id".into());
        }
        let Some(member) = self.objects.get(id).cloned() else {
            return Err(format!("objects/{id} isn't in the file"));
        };
        let meta = format!("objects/{id}.json");
        if self.has(&meta) {
            let v = self.json_within(&meta, MAX_OBJECT_META).map_err(|e| message(&e))?;
            if let Some(t) = text(&v, "contentType")
                && !t.starts_with("image/")
            {
                return Err(format!("it is stored as “{}”, not an image", super::read::clip(t)));
            }
        }
        if self.images >= MAX_IMAGES {
            return Err(format!("the file has more than {MAX_IMAGES} images"));
        }
        let limit = MAX_IMAGE.min(IMAGE_BUDGET.saturating_sub(self.image_bytes));
        let data = crate::read_member(&mut self.zip, &member, limit).map_err(|e| message(&e))?;
        self.images += 1;
        self.image_bytes = self.image_bytes.saturating_add(data.len() as u64);
        Ok(data)
    }
}

/// A format error's words, without the "not a valid Scrim file" its display adds.
pub(super) fn message(e: &FormatError) -> String {
    match e {
        FormatError::Corrupt(m) | FormatError::Io(m) => m.clone(),
        FormatError::Newer(v) => format!("format version {v}"),
        FormatError::Unpack(u) => u.to_string(),
    }
}
