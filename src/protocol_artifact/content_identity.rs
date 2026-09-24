// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-051/FR-052 document content identities (QSL-220).
//!
//! A checked-handoff or native-temporal document's content identity is the
//! lowercase hex SHA-256 that `quire-canonical` computes over the RFC 8785
//! encoding of the document's identity preimage, under the document's
//! contract label as digest domain: the label's byte length as a big-endian
//! `u64`, the label, then the canonical text. ADR-013 §2 (ADR-013:113) names
//! `quire-canonical` the one RFC 8785 implementation, so the identity does
//! not depend on the order a Rust struct declares its fields in.
//!
//! Every integer in the preimage is encoded as its decimal string. RFC 8785
//! numbers are IEEE 754 doubles, and `quire-canonical` refuses an integer
//! with no exact double, but a native-temporal coordinate, order key,
//! watermark or revision is a full-range `i64`/`u64` (a nanosecond
//! timestamp is above 2^53). The emitted documents keep their JSON integers;
//! only the preimage spells them as strings.

use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

/// Why a preimage has no content identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    /// The canonical text would exceed the caller's output-byte limit.
    OutputBytes,
    /// The preimage nests deeper than the caller's JSON depth limit.
    JsonDepth,
    /// The preimage has no RFC 8785 encoding, e.g. a non-finite float.
    NotEncodable,
}

/// The content identity of `preimage` under the `contract` digest domain,
/// encoded within `max_bytes` canonical bytes and `max_depth` nesting levels.
pub(crate) fn of(
    contract: &str,
    preimage: &impl Serialize,
    max_bytes: usize,
    max_depth: usize,
) -> Result<String, Refusal> {
    let max_bytes = u64::try_from(max_bytes).unwrap_or(u64::MAX);
    let max_depth = u32::try_from(max_depth)
        .unwrap_or(u32::MAX)
        .min(quire_canonical::Limits::MAX_DEPTH);
    // Unreachable: `max_depth` is clamped to `Limits::MAX_DEPTH` just above.
    let limits =
        quire_canonical::Limits::new(max_bytes, max_depth).map_err(|_| Refusal::JsonDepth)?;
    quire_canonical::sha256_with_domain(contract.as_bytes(), &Decimal(preimage), limits)
        .map(|digest| digest.to_string())
        .map_err(|error| match error {
            quire_canonical::Error::Limit(limit) => match limit.kind {
                quire_canonical::LimitKind::NestingDepth => Refusal::JsonDepth,
                quire_canonical::LimitKind::CanonicalBytes
                | quire_canonical::LimitKind::ObjectBytes => Refusal::OutputBytes,
            },
            _ => Refusal::NotEncodable,
        })
}

/// `T` serialized with every integer as its decimal string.
struct Decimal<'a, T: ?Sized>(&'a T);

impl<T: Serialize + ?Sized> Serialize for Decimal<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(DecimalSerializer(serializer))
    }
}

/// A [`Serializer`] that forwards to `S`, spelling integers as strings.
struct DecimalSerializer<S>(S);

/// A compound serializer whose elements, keys and fields are [`Decimal`].
struct Compound<C>(C);

macro_rules! decimal {
    ($($method:ident: $ty:ty),* $(,)?) => {
        $(fn $method(self, value: $ty) -> Result<S::Ok, S::Error> {
            self.0.serialize_str(&value.to_string())
        })*
    };
}

macro_rules! forward {
    ($($method:ident: $ty:ty),* $(,)?) => {
        $(fn $method(self, value: $ty) -> Result<S::Ok, S::Error> {
            self.0.$method(value)
        })*
    };
}

impl<S: Serializer> Serializer for DecimalSerializer<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Compound<S::SerializeSeq>;
    type SerializeTuple = Compound<S::SerializeTuple>;
    type SerializeTupleStruct = Compound<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Compound<S::SerializeTupleVariant>;
    type SerializeMap = Compound<S::SerializeMap>;
    type SerializeStruct = Compound<S::SerializeStruct>;
    type SerializeStructVariant = Compound<S::SerializeStructVariant>;

    decimal!(
        serialize_i8: i8,
        serialize_i16: i16,
        serialize_i32: i32,
        serialize_i64: i64,
        serialize_i128: i128,
        serialize_u8: u8,
        serialize_u16: u16,
        serialize_u32: u32,
        serialize_u64: u64,
        serialize_u128: u128,
    );

    forward!(
        serialize_bool: bool,
        serialize_f32: f32,
        serialize_f64: f64,
        serialize_char: char,
        serialize_str: &str,
        serialize_bytes: &[u8],
        serialize_unit_struct: &'static str,
    );

    fn serialize_none(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_none()
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<S::Ok, S::Error> {
        self.0.serialize_some(&Decimal(value))
    }

    fn serialize_unit(self) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_unit_variant(name, index, variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0.serialize_newtype_struct(name, &Decimal(value))
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<S::Ok, S::Error> {
        self.0
            .serialize_newtype_variant(name, index, variant, &Decimal(value))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, S::Error> {
        self.0.serialize_seq(len).map(Compound)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, S::Error> {
        self.0.serialize_tuple(len).map(Compound)
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, S::Error> {
        self.0.serialize_tuple_struct(name, len).map(Compound)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, S::Error> {
        self.0
            .serialize_tuple_variant(name, index, variant, len)
            .map(Compound)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, S::Error> {
        self.0.serialize_map(len).map(Compound)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, S::Error> {
        self.0.serialize_struct(name, len).map(Compound)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, S::Error> {
        self.0
            .serialize_struct_variant(name, index, variant, len)
            .map(Compound)
    }

    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

impl<C: SerializeSeq> SerializeSeq for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_element(&Decimal(value))
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeTuple> SerializeTuple for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_element(&Decimal(value))
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeTupleStruct> SerializeTupleStruct for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_field(&Decimal(value))
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeTupleVariant> SerializeTupleVariant for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_field(&Decimal(value))
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeMap> SerializeMap for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), C::Error> {
        self.0.serialize_key(&Decimal(key))
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), C::Error> {
        self.0.serialize_value(&Decimal(value))
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeStruct> SerializeStruct for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), C::Error> {
        self.0.serialize_field(key, &Decimal(value))
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), C::Error> {
        self.0.skip_field(key)
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

impl<C: SerializeStructVariant> SerializeStructVariant for Compound<C> {
    type Ok = C::Ok;
    type Error = C::Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), C::Error> {
        self.0.serialize_field(key, &Decimal(value))
    }

    fn skip_field(&mut self, key: &'static str) -> Result<(), C::Error> {
        self.0.skip_field(key)
    }

    fn end(self) -> Result<C::Ok, C::Error> {
        self.0.end()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{of, Decimal, Refusal};
    use serde::Serialize;

    const CONTRACT: &str = "quire.checked-predicate/v1";

    /// Hand-written RFC 8785 text of both preimages below: members sorted by
    /// name, integers as decimal strings, no insignificant whitespace.
    const CANONICAL: &str =
        r#"{"alpha":"1","big":"-9223372036854775808","nested":{"x":true,"y":"é"},"zulu":"z"}"#;

    /// SHA-256 over `u64be(26) || "quire.checked-predicate/v1" || CANONICAL`,
    /// computed outside this crate (`printf ... | sha256sum`).
    const GOLDEN: &str = "2790eaac4c1ca4ba204a8cb727a10ae127da478a491e32b271f3cbab3eee6b7e";

    fn limits() -> quire_canonical::Limits {
        quire_canonical::Limits::new(1024, 8).unwrap()
    }

    #[derive(Serialize)]
    struct Nested {
        y: &'static str,
        x: bool,
    }

    #[derive(Serialize)]
    struct Declared {
        zulu: &'static str,
        nested: Nested,
        big: i64,
        alpha: u8,
    }

    #[derive(Serialize)]
    struct NestedSorted {
        x: bool,
        y: &'static str,
    }

    #[derive(Serialize)]
    struct Sorted {
        alpha: u8,
        big: i64,
        nested: NestedSorted,
        zulu: &'static str,
    }

    fn declared() -> Declared {
        Declared {
            zulu: "z",
            nested: Nested { y: "é", x: true },
            big: i64::MIN,
            alpha: 1,
        }
    }

    #[test]
    fn identity_matches_the_golden_vector() {
        let canonical = quire_canonical::to_vec(&Decimal(&declared()), limits()).unwrap();
        assert_eq!(std::str::from_utf8(&canonical).unwrap(), CANONICAL);
        assert_eq!(of(CONTRACT, &declared(), 1024, 8).unwrap(), GOLDEN);
    }

    #[test]
    fn identity_is_independent_of_struct_field_order() {
        let sorted = Sorted {
            alpha: 1,
            big: i64::MIN,
            nested: NestedSorted { x: true, y: "é" },
            zulu: "z",
        };
        assert_eq!(
            of(CONTRACT, &declared(), 1024, 8).unwrap(),
            of(CONTRACT, &sorted, 1024, 8).unwrap()
        );
    }

    #[test]
    fn identity_is_separated_by_contract_domain() {
        assert_ne!(
            of(CONTRACT, &declared(), 1024, 8).unwrap(),
            of("quire.checked-temporal-subject/v1", &declared(), 1024, 8).unwrap()
        );
    }

    /// Integers of every width, inside options, sequences, maps (keys too)
    /// and enum variants, are decimal strings; nothing else changes.
    #[test]
    fn every_integer_is_a_decimal_string() {
        #[derive(Serialize)]
        enum Shape {
            Unit,
            Newtype(u128),
            Tuple(i8, u16),
            Record { at: i128 },
        }
        #[derive(Serialize)]
        struct All {
            full: [u64; 2],
            keyed: BTreeMap<u32, i32>,
            maybe: Option<i64>,
            none: Option<i64>,
            shapes: Vec<Shape>,
            text: &'static str,
            flag: bool,
        }
        let value = All {
            full: [u64::MAX, 0],
            keyed: BTreeMap::from([(7, -7)]),
            maybe: Some(i64::MAX),
            none: None,
            shapes: vec![
                Shape::Unit,
                Shape::Newtype(u128::MAX),
                Shape::Tuple(-1, 2),
                Shape::Record { at: i128::MIN },
            ],
            text: "t",
            flag: true,
        };
        let canonical = quire_canonical::to_vec(&Decimal(&value), limits()).unwrap();
        assert_eq!(
            std::str::from_utf8(&canonical).unwrap(),
            concat!(
                r#"{"flag":true,"full":["18446744073709551615","0"],"keyed":{"7":"-7"},"#,
                r#""maybe":"9223372036854775807","none":null,"shapes":["Unit","#,
                r#"{"Newtype":"340282366920938463463374607431768211455"},"#,
                r#"{"Tuple":["-1","2"]},"#,
                r#"{"Record":{"at":"-170141183460469231731687303715884105728"}}],"text":"t"}"#
            )
        );
    }

    #[test]
    fn identity_refuses_past_its_limits_and_non_finite_floats() {
        assert_eq!(
            of(CONTRACT, &declared(), CANONICAL.len() - 1, 8),
            Err(Refusal::OutputBytes)
        );
        assert!(of(CONTRACT, &declared(), CANONICAL.len(), 8).is_ok());
        assert_eq!(of(CONTRACT, &declared(), 1024, 1), Err(Refusal::JsonDepth));
        assert_eq!(of(CONTRACT, &f64::NAN, 1024, 8), Err(Refusal::NotEncodable));
    }
}
