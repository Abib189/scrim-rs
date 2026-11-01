//! Whether a value holds only finite numbers (Fix AI LW-2). JSON has no NaN or infinity: one in a
//! layer would be saved as `null`, and the file wouldn't open again. The engine refuses an edit
//! that would leave one on a layer, and imports reset them, both through [`all_finite`].

use serde::ser::{self, Serialize, Serializer};

/// Whether every float in `v` is finite, read through its serialisation (no output is made).
pub fn all_finite<T: Serialize + ?Sized>(v: &T) -> bool {
    v.serialize(Check).is_ok()
}

/// Walks a value as it would be written and fails on the first NaN or infinite float.
struct Check;

#[derive(Debug)]
struct NotFinite;

impl std::fmt::Display for NotFinite {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a number that isn't finite")
    }
}

impl std::error::Error for NotFinite {}

impl ser::Error for NotFinite {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        NotFinite
    }
}

macro_rules! fine {
    ($($name:ident($ty:ty)),* $(,)?) => {
        $(fn $name(self, _: $ty) -> Result<(), NotFinite> { Ok(()) })*
    };
}

impl Serializer for Check {
    type Ok = ();
    type Error = NotFinite;
    type SerializeSeq = Check;
    type SerializeTuple = Check;
    type SerializeTupleStruct = Check;
    type SerializeTupleVariant = Check;
    type SerializeMap = Check;
    type SerializeStruct = Check;
    type SerializeStructVariant = Check;

    fine!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8]),
    );

    fn serialize_f32(self, v: f32) -> Result<(), NotFinite> {
        if v.is_finite() { Ok(()) } else { Err(NotFinite) }
    }
    fn serialize_f64(self, v: f64) -> Result<(), NotFinite> {
        if v.is_finite() { Ok(()) } else { Err(NotFinite) }
    }
    fn serialize_none(self) -> Result<(), NotFinite> {
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<(), NotFinite> {
        v.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), NotFinite> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), NotFinite> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), NotFinite> {
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &'static str, v: &T) -> Result<(), NotFinite> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _: &'static str, _: u32, _: &'static str, v: &T) -> Result<(), NotFinite> {
        v.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Check, NotFinite> {
        Ok(Check)
    }
    fn serialize_tuple(self, _: usize) -> Result<Check, NotFinite> {
        Ok(Check)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Check, NotFinite> {
        Ok(Check)
    }
    fn serialize_tuple_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Check, NotFinite> {
        Ok(Check)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Check, NotFinite> {
        Ok(Check)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Check, NotFinite> {
        Ok(Check)
    }
    fn serialize_struct_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Check, NotFinite> {
        Ok(Check)
    }
}

impl ser::SerializeSeq for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}

impl ser::SerializeTuple for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}

impl ser::SerializeMap for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, k: &T) -> Result<(), NotFinite> {
        k.serialize(Check)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}

impl ser::SerializeStruct for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &'static str, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for Check {
    type Ok = ();
    type Error = NotFinite;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, _: &'static str, v: &T) -> Result<(), NotFinite> {
        v.serialize(Check)
    }
    fn end(self) -> Result<(), NotFinite> {
        Ok(())
    }
}
