// SPDX-License-Identifier: AGPL-3.0-only
//! FR-020/NFR-007: metered Serde events over the original JSON bytes.

use std::{collections::BTreeSet, fmt};

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::{Deserialize, Deserializer};

use super::{PackageLimits, PackagePassUsage, PackagePathSegment};
use crate::Code;

#[derive(Debug)]
pub(super) struct Failure {
    pub code: Code,
    pub usage: PackagePassUsage,
    pub path: Vec<PackagePathSegment>,
    pub cause: serde_json::Error,
}

#[derive(Default)]
struct Meter {
    limits: PackageLimits,
    usage: PackagePassUsage,
    path: Vec<PackagePathSegment>,
    depth: usize,
    exhausted: bool,
    string_tags: bool,
}

impl Meter {
    fn exhausted<E: de::Error>(&mut self) -> E {
        self.exhausted = true;
        E::custom("native package traversal limit")
    }

    fn entry<E: de::Error>(&mut self) -> Result<(), E> {
        if self.usage.entries >= self.limits.entries {
            return Err(self.exhausted());
        }
        self.usage.entries += 1;
        Ok(())
    }

    fn text<E: de::Error>(&mut self, size: usize) -> Result<(), E> {
        let Some(next) = self.usage.string_bytes.checked_add(size) else {
            return Err(self.exhausted());
        };
        if next > self.limits.string_bytes {
            return Err(self.exhausted());
        }
        self.usage.string_bytes = next;
        Ok(())
    }

    fn enter<E: de::Error>(&mut self) -> Result<(), E> {
        if self.depth >= self.limits.depth {
            return Err(self.exhausted());
        }
        self.depth += 1;
        self.usage.max_depth = self.usage.max_depth.max(self.depth);
        Ok(())
    }
}

struct Measured<'a, D> {
    inner: D,
    meter: &'a mut Meter,
}
struct Wrapped<'a, V> {
    inner: V,
    meter: &'a mut Meter,
}

macro_rules! deserialize_methods {
    ($($method:ident($($name:ident: $ty:ty),*);)*) => {$ (
        fn $method<V: Visitor<'de>>(self, $($name: $ty,)* visitor: V) -> Result<V::Value, D::Error> {
            self.inner.$method($($name,)* Wrapped { inner: visitor, meter: self.meter })
        }
    )*};
}

impl<'de, D: Deserializer<'de>> Deserializer<'de> for Measured<'_, D> {
    type Error = D::Error;
    deserialize_methods! {
        deserialize_any(); deserialize_bool();
        deserialize_i8(); deserialize_i16(); deserialize_i32(); deserialize_i64(); deserialize_i128();
        deserialize_u8(); deserialize_u16(); deserialize_u32(); deserialize_u64(); deserialize_u128();
        deserialize_f32(); deserialize_f64(); deserialize_char(); deserialize_str(); deserialize_string();
        deserialize_bytes(); deserialize_byte_buf(); deserialize_option(); deserialize_unit();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_seq(); deserialize_tuple(len: usize);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_map();
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
    }
    // Package records are JSON objects, never Serde's positional struct arrays.
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        self.inner.deserialize_map(Wrapped {
            inner: visitor,
            meter: self.meter,
        })
    }
    // Serde's generated enum identifier visitor can accept numeric variant
    // indices. This JSON wire requires discriminants to be literal strings.
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.inner.deserialize_str(Wrapped {
            inner: visitor,
            meter: self.meter,
        })
    }
    // An ignored field still traverses all real JSON events and budgets.
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        self.deserialize_any(visitor)
    }
    fn is_human_readable(&self) -> bool {
        true
    }
}

macro_rules! primitive_visits {
    ($($method:ident($ty:ty);)*) => {$ (
        fn $method<E: de::Error>(self, value: $ty) -> Result<V::Value, E> { self.inner.$method(value) }
    )*};
}

impl<'de, V: Visitor<'de>> Visitor<'de> for Wrapped<'_, V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(f)
    }
    primitive_visits! {
        visit_bool(bool); visit_i8(i8); visit_i16(i16); visit_i32(i32); visit_i64(i64); visit_i128(i128);
        visit_u8(u8); visit_u16(u16); visit_u32(u32); visit_u64(u64); visit_u128(u128);
        visit_f32(f32); visit_f64(f64); visit_char(char);
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<V::Value, E> {
        self.meter.text(value.len())?;
        self.inner.visit_str(value)
    }
    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<V::Value, E> {
        self.meter.text(value.len())?;
        self.inner.visit_borrowed_str(value)
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<V::Value, E> {
        self.meter.text(value.len())?;
        self.inner.visit_string(value)
    }
    fn visit_unit<E: de::Error>(self) -> Result<V::Value, E> {
        self.inner.visit_unit()
    }
    fn visit_none<E: de::Error>(self) -> Result<V::Value, E> {
        self.inner.visit_none()
    }
    fn visit_some<D: Deserializer<'de>>(self, inner: D) -> Result<V::Value, D::Error> {
        self.inner.visit_some(Measured {
            inner,
            meter: self.meter,
        })
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, inner: D) -> Result<V::Value, D::Error> {
        self.inner.visit_newtype_struct(Measured {
            inner,
            meter: self.meter,
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, inner: A) -> Result<V::Value, A::Error> {
        self.meter.enter()?;
        let result = self.inner.visit_seq(Sequence {
            inner,
            meter: self.meter,
            index: 0,
        });
        if result.is_ok() {
            self.meter.depth -= 1;
        }
        result
    }
    fn visit_map<A: MapAccess<'de>>(self, inner: A) -> Result<V::Value, A::Error> {
        self.meter.enter()?;
        let result = self.inner.visit_map(Object {
            inner,
            meter: self.meter,
            keys: BTreeSet::new(),
        });
        if result.is_ok() {
            self.meter.depth -= 1;
        }
        result
    }
    fn visit_enum<A: EnumAccess<'de>>(self, inner: A) -> Result<V::Value, A::Error> {
        self.inner.visit_enum(Enumeration {
            inner,
            meter: self.meter,
        })
    }
}

struct Seed<'a, T> {
    inner: T,
    meter: &'a mut Meter,
    entry: bool,
}
impl<'de, T: DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<'_, T> {
    type Value = T::Value;
    fn deserialize<D: Deserializer<'de>>(self, inner: D) -> Result<T::Value, D::Error> {
        if self.entry {
            self.meter.entry()?;
        }
        self.inner.deserialize(Measured {
            inner,
            meter: self.meter,
        })
    }
}

struct Key;
impl<'de> DeserializeSeed<'de> for Key {
    type Value = String;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        String::deserialize(deserializer)
    }
}

struct Object<'a, A> {
    inner: A,
    meter: &'a mut Meter,
    keys: BTreeSet<String>,
}
impl<'de, A: MapAccess<'de>> MapAccess<'de> for Object<'_, A> {
    type Error = A::Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, A::Error> {
        let Some(key) = self.inner.next_key_seed(Seed {
            inner: Key,
            meter: self.meter,
            entry: true,
        })?
        else {
            return Ok(None);
        };
        self.meter.path.push(PackagePathSegment::Field(key.clone()));
        if !self.keys.insert(key.clone()) {
            return Err(de::Error::custom("duplicate decoded package member"));
        }
        seed.deserialize(de::value::StringDeserializer::new(key))
            .map(Some)
    }
    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, A::Error> {
        // Every v1 `kind` is a string. Enforce this before Serde's tagged-enum
        // buffering, whose generated identifier visitor also accepts numbers.
        // Recognition of an unknown wire version never applies this constraint.
        let result = if self.meter.string_tags
            && matches!(self.meter.path.last(), Some(PackagePathSegment::Field(key)) if key == "kind")
        {
            let tag = self.inner.next_value_seed(Seed {
                inner: Key,
                meter: self.meter,
                entry: false,
            })?;
            seed.deserialize(de::value::StringDeserializer::new(tag))
        } else {
            self.inner.next_value_seed(Seed {
                inner: seed,
                meter: self.meter,
                entry: false,
            })
        };
        if result.is_ok() {
            self.meter.path.pop();
        }
        result
    }
}

struct Sequence<'a, A> {
    inner: A,
    meter: &'a mut Meter,
    index: usize,
}
impl<'de, A: SeqAccess<'de>> SeqAccess<'de> for Sequence<'_, A> {
    type Error = A::Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, A::Error> {
        self.meter.path.push(PackagePathSegment::Index(self.index));
        let result = self.inner.next_element_seed(Seed {
            inner: seed,
            meter: self.meter,
            entry: true,
        });
        if result.is_ok() {
            self.meter.path.pop();
        }
        if matches!(result, Ok(Some(_))) {
            self.index += 1;
        }
        result
    }
}

struct Enumeration<'a, A> {
    inner: A,
    meter: &'a mut Meter,
}
impl<'a, 'de, A: EnumAccess<'de>> EnumAccess<'de> for Enumeration<'a, A> {
    type Error = A::Error;
    type Variant = Variant<'a, A::Variant>;
    fn variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<(T::Value, Self::Variant), A::Error> {
        let (tag, inner) = self.inner.variant_seed(Seed {
            inner: seed,
            meter: self.meter,
            entry: false,
        })?;
        Ok((
            tag,
            Variant {
                inner,
                meter: self.meter,
            },
        ))
    }
}

struct Variant<'a, A> {
    inner: A,
    meter: &'a mut Meter,
}
impl<'de, A: VariantAccess<'de>> VariantAccess<'de> for Variant<'_, A> {
    type Error = A::Error;
    fn unit_variant(self) -> Result<(), A::Error> {
        self.inner.unit_variant()
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, A::Error> {
        self.inner.newtype_variant_seed(Seed {
            inner: seed,
            meter: self.meter,
            entry: false,
        })
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, A::Error> {
        self.inner.tuple_variant(
            len,
            Wrapped {
                inner: visitor,
                meter: self.meter,
            },
        )
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, A::Error> {
        self.inner.struct_variant(
            fields,
            Wrapped {
                inner: visitor,
                meter: self.meter,
            },
        )
    }
}

pub(super) fn decode<T: DeserializeOwned>(
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<(T, PackagePassUsage), Failure> {
    decode_with_tags(bytes, limits, false)
}

pub(super) fn decode_wire<T: DeserializeOwned>(
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<(T, PackagePassUsage), Failure> {
    decode_with_tags(bytes, limits, true)
}

fn decode_with_tags<T: DeserializeOwned>(
    bytes: &[u8],
    limits: PackageLimits,
    string_tags: bool,
) -> Result<(T, PackagePassUsage), Failure> {
    let mut meter = Meter {
        limits: limits.bounded(),
        string_tags,
        ..Meter::default()
    };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    // Only this decoder disables the library guard; every container enters Meter.
    decoder.disable_recursion_limit();
    let result = T::deserialize(Measured {
        inner: &mut decoder,
        meter: &mut meter,
    });
    let result = result.and_then(|value| decoder.end().map(|()| value));
    result
        .map(|value| (value, meter.usage))
        .map_err(|cause| Failure {
            code: if meter.exhausted {
                Code::ResourceExhausted
            } else {
                Code::InvalidPackage
            },
            usage: meter.usage,
            path: meter.path,
            cause,
        })
}

/// Only the header string survives generic recognition; all other values are visited.
pub(super) struct Recognized(pub Option<String>);
impl<'de> Deserialize<'de> for Recognized {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Root;
        impl<'de> Visitor<'de> for Root {
            type Value = Recognized;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a package object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
                let mut format = None;
                while let Some(key) = object.next_key::<String>()? {
                    if key == "format" {
                        format = object.next_value::<Header>()?.0;
                    } else {
                        object.next_value::<de::IgnoredAny>()?;
                    }
                }
                Ok(Recognized(format))
            }
        }
        decoder.deserialize_map(Root)
    }
}

struct Header(Option<String>);
impl<'de> Deserialize<'de> for Header {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Probe;
        impl<'de> Visitor<'de> for Probe {
            type Value = Header;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON value")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Header, E> {
                Ok(Header(Some(value.into())))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Header, E> {
                Ok(Header(None))
            }
            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Header, E> {
                Ok(Header(None))
            }
            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Header, E> {
                Ok(Header(None))
            }
            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Header, E> {
                Ok(Header(None))
            }
            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Header, E> {
                Ok(Header(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut array: A) -> Result<Header, A::Error> {
                while array.next_element::<de::IgnoredAny>()?.is_some() {}
                Ok(Header(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Header, A::Error> {
                while object
                    .next_entry::<de::IgnoredAny, de::IgnoredAny>()?
                    .is_some()
                {}
                Ok(Header(None))
            }
        }
        decoder.deserialize_any(Probe)
    }
}

#[cfg(test)]
mod tests;
