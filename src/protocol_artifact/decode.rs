// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: allocation-free Serde census before allocating closed wire records.
//!
//! Both passes use serde_json. The census constructs no generic JSON tree and
//! never invokes the native parser. Its complete entry/content reservation
//! precedes the typed pass; serde_json's temporary escape buffer is bounded by
//! the already checked payload size and charged input traversal.

use std::fmt;

use serde::de::{self, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use super::{wire, work::Work, Dimension, Error, Invalid};

struct Census<'a> {
    work: &'a mut Work,
    error: &'a mut Option<Error>,
    depth: usize,
    entry: bool,
}

impl Census<'_> {
    fn fail<E: de::Error>(&mut self, error: Error) -> E {
        *self.error = Some(error);
        E::custom("compiled protocol wire census refused")
    }

    fn charge<E: de::Error>(&mut self, dimension: Dimension, amount: usize) -> Result<(), E> {
        self.work
            .charge(dimension, amount)
            .map_err(|error| self.fail(error.into()))
    }
}

impl<'de> DeserializeSeed<'de> for Census<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(mut self, decoder: D) -> Result<(), D::Error> {
        if self.entry {
            self.charge(Dimension::Entries, 1)?;
        }
        self.charge(Dimension::Depth, self.depth)?;
        decoder.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Census<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded compiled protocol JSON")
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_bool<E: de::Error>(self, _value: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(mut self, value: u64) -> Result<(), E> {
        if value > 1_048_576 {
            Err(self.fail(Error::Invalid(Invalid::StructuralInteger)))
        } else {
            Ok(())
        }
    }
    fn visit_i64<E: de::Error>(mut self, _value: i64) -> Result<(), E> {
        Err(self.fail(Error::Invalid(Invalid::StructuralInteger)))
    }
    fn visit_f64<E: de::Error>(mut self, _value: f64) -> Result<(), E> {
        Err(self.fail(Error::Invalid(Invalid::StructuralInteger)))
    }
    fn visit_str<E: de::Error>(mut self, text: &str) -> Result<(), E> {
        self.charge(Dimension::ContentBytes, text.len())?;
        self.charge(Dimension::ByteWork, text.len())
    }
    fn visit_borrowed_str<E: de::Error>(self, text: &'de str) -> Result<(), E> {
        self.visit_str(text)
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        // A successful key reserves its member before the corresponding value.
        while map
            .next_key_seed(Census {
                work: self.work,
                error: self.error,
                depth: self.depth + 1,
                entry: true,
            })?
            .is_some()
        {
            map.next_value_seed(Census {
                work: self.work,
                error: self.error,
                depth: self.depth + 1,
                entry: false,
            })?;
        }
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        // SeqAccess calls the seed only for an actual element, not the end token.
        while sequence
            .next_element_seed(Census {
                work: self.work,
                error: self.error,
                depth: self.depth + 1,
                entry: true,
            })?
            .is_some()
        {}
        Ok(())
    }
}

pub(super) fn bounded<T: DeserializeOwned>(bytes: &[u8], work: &mut Work) -> Result<T, Error> {
    work.bytes(bytes.len())?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    // The census charges our lower, caller-selected depth before descending.
    // Its successful whole-input pass bounds the subsequent typed decode too.
    decoder.disable_recursion_limit();
    let mut refusal = None;
    let census = Census {
        work,
        error: &mut refusal,
        depth: 1,
        entry: false,
    };
    if let Err(error) = census
        .deserialize(&mut decoder)
        .and_then(|()| decoder.end())
    {
        return Err(refusal.unwrap_or_else(|| json_error(&error)));
    }
    // The census reserved all final record/table/string storage. This charge
    // precedes the second traversal and any owned wire-string allocation.
    work.bytes(bytes.len())?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    decoder.disable_recursion_limit();
    let package = T::deserialize(&mut decoder).map_err(|error| json_error(&error))?;
    decoder.end().map_err(|error| json_error(&error))?;
    Ok(package)
}

pub(super) fn package(bytes: &[u8], work: &mut Work) -> Result<wire::Package, Error> {
    bounded(bytes, work)
}

fn json_error(error: &serde_json::Error) -> Error {
    Error::Json {
        line: error.line(),
        column: error.column(),
    }
}
