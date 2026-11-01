//! A layer's `guides` key (Fix AI LW-1, upstream FigCraft 652bf35b): left out when there are none,
//! and every `guides` list the layer has is read.
//!
//! Pages used to keep their guides in `NodeKind::Page` while the layer's own field took the
//! `guides` key on reading, so a reopened page lost its guides, and saving it then wrote two
//! `guides` keys, which a plain field refuses. Files in that shape open with both lists, in file
//! order (the page kind's, guides added since the reopen, then the layer's). The key in the file
//! is unchanged, so files from before open with their guides.

use std::fmt;

use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serializer};

use crate::Guide;

const KEY: &str = "guides";

/// Write `guides` as the layer's `guides` key, or nothing when there are none.
pub fn serialize<S: Serializer>(guides: &[Guide], s: S) -> Result<S::Ok, S::Error> {
    let mut map = s.serialize_map(None)?;
    if !guides.is_empty() {
        map.serialize_entry(KEY, guides)?;
    }
    map.end()
}

/// Read every `guides` list among the layer's fields, joined in file order.
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Guide>, D::Error> {
    d.deserialize_map(Lists)
}

/// Reads the `guides` lists of a layer's fields and skips the other fields.
struct Lists;

impl<'de> Visitor<'de> for Lists {
    type Value = Vec<Guide>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a layer's fields")
    }

    /// Every `guides` value joined, in order; a malformed one is an error.
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Vec<Guide>, A::Error> {
        let mut guides = Vec::new();
        while let Some(key) = map.next_key::<Key>()? {
            if key.0 {
                guides.extend(map.next_value::<Vec<Guide>>()?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(guides)
    }
}

/// Whether a field name is `guides`.
struct Key(bool);

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Name;
        impl Visitor<'_> for Name {
            type Value = Key;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a field name")
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Key, E> {
                Ok(Key(s == KEY))
            }
        }
        d.deserialize_identifier(Name)
    }
}
