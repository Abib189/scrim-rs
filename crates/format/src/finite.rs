//! Saved files never hold `null` where a number belongs (Fix AI LW-2, upstream FigCraft 8c08035b).
//! JSON has no NaN or infinity: `serde_json` writes them as `null`, and the reader then refuses the
//! whole file. Everything Scrim writes as a document goes through [`Finite`], which writes `0` in
//! their place. The engine refuses edits that would make one, so this is the last line of defence.

use serde::{Serialize, Serializer};

/// A serializer adapter that writes `0` in place of every NaN or infinite float. JSON has no such
/// numbers (`serde_json` writes `null`, which the reader rejects), so one bad value must not stop
/// Save, autosave or recovery copies of the whole document. Everything else passes through.
pub(crate) struct Finite<S>(pub(crate) S);

/// Serializes a value through a [`Finite`] adapter around the serializer it is written with.
struct FiniteValue<'a, T: ?Sized>(&'a T);

impl<T: ?Sized + Serialize> Serialize for FiniteValue<'_, T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(Finite(s))
    }
}

macro_rules! finite_pass {
    ($($name:ident($ty:ty)),* $(,)?) => {
        $(fn $name(self, v: $ty) -> Result<S::Ok, S::Error> { self.0.$name(v) })*
    };
}

impl<S: Serializer> Serializer for Finite<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Finite<S::SerializeSeq>;
    type SerializeTuple = Finite<S::SerializeTuple>;
    type SerializeTupleStruct = Finite<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Finite<S::SerializeTupleVariant>;
    type SerializeMap = Finite<S::SerializeMap>;
    type SerializeStruct = Finite<S::SerializeStruct>;
    type SerializeStructVariant = Finite<S::SerializeStructVariant>;

    finite_pass!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_i128(i128),
        serialize_u128(u128),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8])
    );

    fn serialize_f32(self, v: f32) -> Result<S::Ok, S::Error> {
        self.0.serialize_f32(if v.is_finite() { v } else { 0.0 })
    }
    fn serialize_f64(self, v: f64) -> Result<S::Ok, S::Error> {
        self.0.serialize_f64(if v.is_finite() { v } else { 0.0 })
    }
    fn serialize_none(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_none()
    }
    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_some(&FiniteValue(v))
    }
    fn serialize_unit(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_struct(name)
    }
    fn serialize_unit_variant(self, name: &'static str, idx: u32, variant: &'static str) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_variant(name, idx, variant)
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, name: &'static str, v: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_struct(name, &FiniteValue(v))
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, name: &'static str, idx: u32, variant: &'static str, v: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_variant(name, idx, variant, &FiniteValue(v))
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, S::Error> {
        self.0.serialize_seq(len).map(Finite)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, S::Error> {
        self.0.serialize_tuple(len).map(Finite)
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, S::Error> {
        self.0.serialize_tuple_struct(name, len).map(Finite)
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        idx: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, S::Error> {
        self.0.serialize_tuple_variant(name, idx, variant, len).map(Finite)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, S::Error> {
        self.0.serialize_map(len).map(Finite)
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, S::Error> {
        self.0.serialize_struct(name, len).map(Finite)
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        idx: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, S::Error> {
        self.0.serialize_struct_variant(name, idx, variant, len).map(Finite)
    }
    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

macro_rules! finite_elements {
    ($($trait:ident::$method:ident),* $(,)?) => {
        $(impl<S: serde::ser::$trait> serde::ser::$trait for Finite<S> {
            type Ok = S::Ok;
            type Error = S::Error;
            fn $method<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), S::Error> {
                self.0.$method(&FiniteValue(v))
            }
            fn end(self) -> Result<S::Ok, S::Error> {
                self.0.end()
            }
        })*
    };
}

finite_elements!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field
);

impl<S: serde::ser::SerializeMap> serde::ser::SerializeMap for Finite<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, k: &T) -> Result<(), S::Error> {
        self.0.serialize_key(&FiniteValue(k))
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), S::Error> {
        self.0.serialize_value(&FiniteValue(v))
    }
    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: serde::ser::SerializeStruct> serde::ser::SerializeStruct for Finite<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, v: &T) -> Result<(), S::Error> {
        self.0.serialize_field(key, &FiniteValue(v))
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), S::Error> {
        self.0.skip_field(key)
    }
    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

impl<S: serde::ser::SerializeStructVariant> serde::ser::SerializeStructVariant for Finite<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, v: &T) -> Result<(), S::Error> {
        self.0.serialize_field(key, &FiniteValue(v))
    }
    fn skip_field(&mut self, key: &'static str) -> Result<(), S::Error> {
        self.0.skip_field(key)
    }
    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}
