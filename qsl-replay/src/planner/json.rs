// SPDX-License-Identifier: AGPL-3.0-or-later
//! The planner's strict JSON reading: a duplicate-aware parse, the FR-272
//! `invalid_runtime_input` causes a request or item defect names, and the
//! member helpers the readers share.

use std::cell::RefCell;
use std::fmt;

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::proof_result::{child, first_undefined};

/// The FR-272 cause of a request or item defect (QSpec FR-492 "Admission",
/// step 1 and the `invalid-value` defects after it).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Cause {
    /// The bytes are not JSON, or not an object.
    MalformedJson,
    /// A required member is absent.
    MissingMember,
    /// A member the form does not define is present.
    UnknownMember,
    /// A member occurs twice in one object.
    DuplicateMember,
    /// A member's JSON kind is not the kind its domain takes.
    WrongValueKind,
    /// A member of the right kind lies outside its domain.
    InvalidValue,
    /// The encoded size is over the configured reader bound
    /// (`stage_limit_exceeded/input-bytes-exceeded`, not an
    /// `invalid_runtime_input` cause).
    InputBytesExceeded,
}

impl Cause {
    /// The FR-272 cause spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedJson => "malformed-json",
            Self::MissingMember => "missing-member",
            Self::UnknownMember => "unknown-member",
            Self::DuplicateMember => "duplicate-member",
            Self::WrongValueKind => "wrong-value-kind",
            Self::InvalidValue => "invalid-value",
            Self::InputBytesExceeded => "input-bytes-exceeded",
        }
    }

    /// The FR-272 code the cause belongs to.
    pub const fn code(self) -> &'static str {
        match self {
            Self::InputBytesExceeded => "stage_limit_exceeded",
            _ => "invalid_runtime_input",
        }
    }
}

/// A defect at an RFC 6901 pointer.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{}/{} at `{pointer}`", .cause.code(), .cause.as_str())]
pub struct Fault {
    /// The FR-272 cause.
    pub cause: Cause,
    /// The offending member's RFC 6901 pointer.
    pub pointer: String,
}

impl Fault {
    pub(crate) fn new(cause: Cause, pointer: impl Into<String>) -> Self {
        Self {
            cause,
            pointer: pointer.into(),
        }
    }

    pub(crate) fn missing(pointer: String) -> Self {
        Self::new(Cause::MissingMember, pointer)
    }

    pub(crate) fn unknown(pointer: String) -> Self {
        Self::new(Cause::UnknownMember, pointer)
    }

    pub(crate) fn wrong_kind(pointer: String) -> Self {
        Self::new(Cause::WrongValueKind, pointer)
    }

    pub(crate) fn invalid(pointer: String) -> Self {
        Self::new(Cause::InvalidValue, pointer)
    }

    /// This defect with `prefix` prepended to its pointer.
    pub(crate) fn under(mut self, prefix: &str) -> Self {
        self.pointer = format!("{prefix}{}", self.pointer);
        self
    }
}

/// The result of the duplicate-aware parse.
pub(crate) enum Parsed {
    /// The document, with no duplicate member.
    Document(Value),
    /// Not JSON.
    Malformed,
    /// A member occurs twice; the pointer of the second occurrence.
    Duplicate(String),
}

/// Parse `bytes` as one JSON document, reporting the first duplicate member
/// in document order. `serde_json::Value` alone drops duplicates.
pub(crate) fn parse_strict(bytes: &[u8]) -> Parsed {
    let duplicate = RefCell::new(None);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let seed = Seed {
        path: String::new(),
        duplicate: &duplicate,
    };
    let value = match seed.deserialize(&mut deserializer) {
        Ok(value) => value,
        Err(_) => return Parsed::Malformed,
    };
    if deserializer.end().is_err() {
        return Parsed::Malformed;
    }
    match duplicate.into_inner() {
        Some(pointer) => Parsed::Duplicate(pointer),
        None => Parsed::Document(value),
    }
}

struct Seed<'a> {
    path: String,
    duplicate: &'a RefCell<Option<String>>,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("a non-finite number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        loop {
            let seed = Seed {
                path: format!("{}/{}", self.path, items.len()),
                duplicate: self.duplicate,
            };
            match seq.next_element_seed(seed)? {
                Some(item) => items.push(item),
                None => return Ok(Value::Array(items)),
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut members = Map::new();
        while let Some(name) = map.next_key::<String>()? {
            let path = child(&self.path, &name);
            let value = map.next_value_seed(Seed {
                path: path.clone(),
                duplicate: self.duplicate,
            })?;
            if members.contains_key(&name) {
                let mut first = self.duplicate.borrow_mut();
                if first.is_none() {
                    *first = Some(path);
                }
            } else {
                members.insert(name, value);
            }
        }
        Ok(Value::Object(members))
    }
}

/// The JSON object at `pointer`, else `wrong-value-kind`.
pub(crate) fn object<'a>(value: &'a Value, pointer: &str) -> Result<&'a Map<String, Value>, Fault> {
    value
        .as_object()
        .ok_or_else(|| Fault::wrong_kind(pointer.to_owned()))
}

/// The array at `pointer`, else `wrong-value-kind`.
pub(crate) fn array<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>, Fault> {
    value
        .as_array()
        .ok_or_else(|| Fault::wrong_kind(pointer.to_owned()))
}

/// The string at `pointer`, else `wrong-value-kind`.
pub(crate) fn string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, Fault> {
    value
        .as_str()
        .ok_or_else(|| Fault::wrong_kind(pointer.to_owned()))
}

/// The member `name` of `members` under `base`, else `missing-member`.
pub(crate) fn require<'a>(
    members: &'a Map<String, Value>,
    name: &str,
    base: &str,
) -> Result<&'a Value, Fault> {
    members
        .get(name)
        .ok_or_else(|| Fault::missing(child(base, name)))
}

/// Check an object's members against `required` and `optional`: the first
/// absent required member in member-name order is `missing-member`, then the
/// first undefined member in member-name order is `unknown-member`.
pub(crate) fn check_members(
    members: &Map<String, Value>,
    base: &str,
    required: &[&str],
    optional: &[&str],
) -> Result<(), Fault> {
    let mut absent: Vec<&str> = required
        .iter()
        .copied()
        .filter(|name| !members.contains_key(*name))
        .collect();
    absent.sort_by(|left, right| crate::proof_result::name_order(left, right));
    if let Some(name) = absent.first() {
        return Err(Fault::missing(child(base, name)));
    }
    let defined: Vec<&str> = required.iter().chain(optional).copied().collect();
    if let Some(name) = first_undefined(members, &defined) {
        return Err(Fault::unknown(child(base, name)));
    }
    Ok(())
}

/// The largest FR-331 count, 2^53 - 1.
pub(crate) const MAX_COUNT: u64 = (1 << 53) - 1;

/// An FR-331 count: a JSON integer from 0 to 2^53 - 1.
pub(crate) fn count(value: &Value, pointer: &str) -> Result<u64, Fault> {
    match value.as_u64() {
        Some(count) if count <= MAX_COUNT => Ok(count),
        _ if value.is_u64() || value.is_i64() => Err(Fault::invalid(pointer.to_owned())),
        _ => Err(Fault::wrong_kind(pointer.to_owned())),
    }
}

/// `value` with every object's members in sorted order, written as text:
/// two values are equal exactly when their canonical texts are.
pub(crate) fn canonical_text(value: &Value) -> String {
    let mut text = String::new();
    write_canonical(value, &mut text);
    text
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(members) => {
            let mut names: Vec<&String> = members.keys().collect();
            names.sort();
            out.push('{');
            for (index, name) in names.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(name.clone()).to_string());
                out.push(':');
                if let Some(member) = members.get(name) {
                    write_canonical(member, out);
                }
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}
