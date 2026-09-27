// SPDX-License-Identifier: AGPL-3.0-or-later
//! A JSON value whose object members keep the document's own on-the-wire
//! order.
//!
//! FR-106's own admission-order rule ("Where a check walks the document...
//! it reports the first failing object and field", check 1.6's "naming the
//! first [unknown member] in document order") needs the *document's* own
//! member order, not an alphabetical one. `serde_json::Value`'s object is a
//! `serde_json::Map`, `BTreeMap`-backed unless the crate-wide
//! `preserve_order` feature is on -- and that feature stays off here: an
//! earlier attempt to enable it broke an unrelated, already-passing
//! canonical-encoding test elsewhere in this crate (`qsl-semantics/
//! Cargo.toml`'s own note on its `serde_json` dependency). `serde`'s
//! `MapAccess` already visits a JSON object's members in the document's own
//! order regardless of the target collection, so a `Vec` target, not a
//! `Map`, is all this type's `Visitor` needs to keep it -- admission's own
//! narrow, order-preserving read, reached nowhere outside this module's own
//! parent (`model::observation::document`).

use std::fmt;

use serde::de::{Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

/// A JSON value, object members in document order. See the module doc.
#[derive(Clone, Debug)]
pub(super) enum OrderedJson {
    /// `null`.
    Null,
    /// `true`/`false`.
    Bool(bool),
    /// A JSON number, kept as `serde_json` already represents one.
    Number(serde_json::Number),
    /// A JSON string.
    String(String),
    /// A JSON array, in its own order (already preserved: an array
    /// deserializes to a `Vec` regardless of `preserve_order`).
    Array(Vec<OrderedJson>),
    /// A JSON object, members in document order. A duplicate key keeps
    /// every occurrence, first to last, matching `MapAccess`'s own visits;
    /// [`OrderedJson::get`] resolves the first, the one "the first ... in
    /// document order" phrasing means.
    Object(Vec<(String, OrderedJson)>),
}

impl OrderedJson {
    /// This value's members, in document order, or `None` when it is not
    /// an object.
    pub(super) fn as_object(&self) -> Option<&[(String, OrderedJson)]> {
        match self {
            Self::Object(members) => Some(members),
            _ => None,
        }
    }

    /// This value's elements, or `None` when it is not an array.
    pub(super) fn as_array(&self) -> Option<&[OrderedJson]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// This value as a string slice, or `None` when it is not a string.
    pub(super) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// This value as a bool, or `None` when it is not one.
    pub(super) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The structural nesting depth (objects and arrays only), matching
    /// `super::json_depth`'s own count over `serde_json::Value`.
    pub(super) fn depth(&self) -> u32 {
        match self {
            Self::Object(members) => {
                1 + members
                    .iter()
                    .map(|(_, value)| value.depth())
                    .max()
                    .unwrap_or(0)
            }
            Self::Array(items) => 1 + items.iter().map(Self::depth).max().unwrap_or(0),
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) => 0,
        }
    }

    /// This value as a `serde_json::Value`, dropping its own member
    /// ordering: every remaining reader of an admitted document's *leaf*
    /// values (`document::read_raw_value`'s tagged-union recursion) only
    /// ever inspects a single-key object, where order carries no
    /// information, so converting at the leaf loses nothing check 1.6 or 6
    /// needs.
    pub(super) fn into_value(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(value),
            Self::Number(value) => serde_json::Value::Number(value),
            Self::String(value) => serde_json::Value::String(value),
            Self::Array(items) => {
                serde_json::Value::Array(items.into_iter().map(Self::into_value).collect())
            }
            Self::Object(members) => serde_json::Value::Object(
                members
                    .into_iter()
                    .map(|(key, value)| (key, value.into_value()))
                    .collect(),
            ),
        }
    }
}

/// `serde_json::Map`'s own `get`/`keys`/`contains_key` shape, over an
/// already-destructured object's ordered members. Named `member`, not
/// `get`: `[T]` already has an inherent `get` (`SliceIndex`-based), and
/// Rust's method resolution picks an inherent candidate by name alone,
/// before checking whether its bounds are satisfied -- so a same-named
/// trait method here would never be reached, not even for a `&str` no
/// `SliceIndex<[T]>` impl admits.
pub(super) trait OrderedObject {
    /// The first member named `key`, or `None`.
    fn member(&self, key: &str) -> Option<&OrderedJson>;

    /// The member names, in document order.
    fn keys(&self) -> Box<dyn Iterator<Item = &String> + '_>;

    /// Whether a member named `key` is present.
    fn contains_key(&self, key: &str) -> bool {
        self.member(key).is_some()
    }
}

impl OrderedObject for [(String, OrderedJson)] {
    fn member(&self, key: &str) -> Option<&OrderedJson> {
        self.iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    fn keys(&self) -> Box<dyn Iterator<Item = &String> + '_> {
        Box::new(self.iter().map(|(name, _)| name))
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderedJsonVisitor;

        impl<'de> Visitor<'de> for OrderedJsonVisitor {
            type Value = OrderedJson;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(OrderedJson::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                Ok(OrderedJson::Number(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(OrderedJson::Number(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
                Ok(serde_json::Number::from_f64(value)
                    .map(OrderedJson::Number)
                    .unwrap_or(OrderedJson::Null))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
                Ok(OrderedJson::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(OrderedJson::String(value))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(OrderedJson::Null)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(OrderedJson::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(OrderedJson::Array(items))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut members = Vec::new();
                while let Some((key, value)) = map.next_entry()? {
                    members.push((key, value));
                }
                Ok(OrderedJson::Object(members))
            }
        }

        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::OrderedJson;
    use ix_trace_rs::trace;

    /// TC-465 (FR-106 check 1.6): a document's own member order survives
    /// parsing, unsorted -- the property `serde_json::Value` (its object
    /// `BTreeMap`-backed) does not have.
    #[trace("TC-465", "FR-106-AC-2")]
    #[test]
    fn object_members_keep_document_order_not_alphabetical() {
        let value: OrderedJson =
            serde_json::from_str(r#"{"zebra": 1, "apple": 2, "mango": 3}"#).expect("valid JSON");
        let keys: Vec<&str> = value
            .as_object()
            .expect("an object")
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(keys, ["zebra", "apple", "mango"]);
    }

    #[test]
    fn depth_matches_json_depth_over_nested_structures() {
        let value: OrderedJson =
            serde_json::from_str(r#"{"a": [1, {"b": 2}]}"#).expect("valid JSON");
        assert_eq!(value.depth(), 3);
    }
}
