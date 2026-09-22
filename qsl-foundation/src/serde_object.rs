// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-020/024: shared Serde object constraints for native artifact records.

use serde::{de, Deserialize, Deserializer};
use std::fmt;

/// Records stay objects even when Serde buffers them inside a tagged variant.
/// Its default struct decoder also accepts positional arrays.
pub fn from_object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    decoder: D,
) -> Result<T, D::Error> {
    struct Object<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Object<T> {
        type Value = T;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a JSON object")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }
    decoder.deserialize_map(Object(std::marker::PhantomData))
}

/// Require object syntax when decoding a record nested in a collection or envelope.
pub struct Object<T>(
    /// The decoded value, once object syntax has been confirmed.
    pub T,
);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        from_object(deserializer).map(Self)
    }
}

/// Preserve vector order and duplicates while requiring each record to be an object.
pub fn deserialize_objects<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Vec::<Object<T>>::deserialize(deserializer)
        .map(|values| values.into_iter().map(|Object(value)| value).collect())
}

/// Refuse any field on an object expected to carry none.
pub fn deserialize_empty_object<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<(), D::Error> {
    // Serde's internally tagged unit variants otherwise ignore extra fields.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    let _: Empty = from_object(deserializer)?;
    Ok(())
}
