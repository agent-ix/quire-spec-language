// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: reserve borrowed wire input before serde_json scans strings.

use super::super::{work::Work, Dimension, Error, Invalid};
use serde::{ser, Serialize};
use std::fmt;

impl ser::Error for Error {
    fn custom<T: fmt::Display>(_message: T) -> Self {
        Self::Invalid(Invalid::Encoding)
    }
}

pub(super) fn reserve(value: &impl Serialize, work: &mut Work) -> Result<(), Error> {
    value.serialize(&mut Census { work, depth: 0 })
}

struct Census<'a> {
    work: &'a mut Work,
    depth: usize,
}

impl<'w> Census<'w> {
    fn scalar(&mut self) -> Result<(), Error> {
        self.work.charge(Dimension::Depth, self.depth + 1)?;
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), Error> {
        self.scalar()?;
        self.work.charge(Dimension::ContentBytes, text.len())?;
        self.work.bytes(text.len())
    }
    fn frame(&mut self) -> Result<Frame<'_, 'w>, Error> {
        self.work.charge(Dimension::Depth, self.depth + 1)?;
        self.depth += 1;
        Ok(Frame(self))
    }
}

struct Frame<'a, 'w>(&'a mut Census<'w>);

impl Frame<'_, '_> {
    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.0.work.charge(Dimension::Entries, 1)?;
        value.serialize(&mut *self.0)
    }
    fn field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result<(), Error> {
        self.0.work.charge(Dimension::Entries, 1)?;
        self.0.text(key)?;
        value.serialize(&mut *self.0)
    }
    fn finish(self) -> Result<(), Error> {
        self.0.depth -= 1;
        Ok(())
    }
}

impl<'a, 'w> ser::Serializer for &'a mut Census<'w> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Frame<'a, 'w>;
    type SerializeTuple = Frame<'a, 'w>;
    type SerializeTupleStruct = Frame<'a, 'w>;
    type SerializeTupleVariant = Frame<'a, 'w>;
    type SerializeMap = Frame<'a, 'w>;
    type SerializeStruct = Frame<'a, 'w>;
    type SerializeStructVariant = Frame<'a, 'w>;

    fn serialize_bool(self, _value: bool) -> Result<(), Error> {
        self.scalar()
    }
    fn serialize_i8(self, _value: i8) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::StructuralInteger))
    }
    fn serialize_i16(self, _value: i16) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::StructuralInteger))
    }
    fn serialize_i32(self, _value: i32) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::StructuralInteger))
    }
    fn serialize_i64(self, _value: i64) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::StructuralInteger))
    }
    fn serialize_u8(self, value: u8) -> Result<(), Error> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u16(self, value: u16) -> Result<(), Error> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u32(self, value: u32) -> Result<(), Error> {
        self.serialize_u64(u64::from(value))
    }
    fn serialize_u64(self, value: u64) -> Result<(), Error> {
        if value > 1_048_576 {
            return Err(Error::Invalid(Invalid::StructuralInteger));
        }
        self.scalar()
    }
    fn serialize_f32(self, _value: f32) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::StructuralInteger))
    }
    fn serialize_f64(self, _value: f64) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::StructuralInteger))
    }
    fn serialize_char(self, value: char) -> Result<(), Error> {
        let mut buffer = [0; 4];
        self.text(value.encode_utf8(&mut buffer))
    }
    fn serialize_str(self, value: &str) -> Result<(), Error> {
        self.text(value)
    }
    fn serialize_bytes(self, _value: &[u8]) -> Result<(), Error> {
        Err(Error::Invalid(Invalid::Encoding))
    }
    fn serialize_none(self) -> Result<(), Error> {
        self.scalar()
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Error> {
        self.scalar()
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Error> {
        self.scalar()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.text(variant)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        let mut frame = self.frame()?;
        frame.field(variant, value)?;
        frame.finish()
    }
    fn serialize_seq(self, _length: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        self.frame()
    }
    fn serialize_tuple(self, _length: usize) -> Result<Self::SerializeTuple, Error> {
        self.frame()
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.frame()
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error::Invalid(Invalid::Encoding))
    }
    fn serialize_map(self, _length: Option<usize>) -> Result<Self::SerializeMap, Error> {
        self.frame()
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        self.frame()
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error::Invalid(Invalid::Encoding))
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, value: &T) -> Result<(), Error> {
        struct Text<'a, 'w> {
            census: &'a mut Census<'w>,
            error: Option<Error>,
        }
        impl fmt::Write for Text<'_, '_> {
            fn write_str(&mut self, value: &str) -> fmt::Result {
                self.census.text(value).map_err(|error| {
                    self.error = Some(error);
                    fmt::Error
                })
            }
        }
        let mut text = Text {
            census: self,
            error: None,
        };
        if fmt::write(&mut text, format_args!("{value}")).is_err() {
            return Err(text.error.unwrap_or(Error::Invalid(Invalid::Encoding)));
        }
        Ok(())
    }
}

impl ser::SerializeSeq for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
impl ser::SerializeTuple for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
impl ser::SerializeTupleStruct for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
impl ser::SerializeTupleVariant for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.element(value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
impl ser::SerializeMap for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        self.element(key)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(&mut *self.0)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
impl ser::SerializeStruct for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.field(key, value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
impl ser::SerializeStructVariant for Frame<'_, '_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.field(key, value)
    }
    fn end(self) -> Result<(), Error> {
        self.finish()
    }
}
