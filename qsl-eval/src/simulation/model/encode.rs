// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "State, state key and anchor" and "Applications and transition
//! identities": QSpec FR-181's typed canonical form of a model state, of a
//! value and of a transition identity.
//!
//! A value, key or identity is built as an owned [`Enc`] tree and written
//! through `quire-canonical`'s event writer, the one RFC 8785 encoder. The
//! tree's depth follows the nesting of the declared value types, which a
//! checked package bounds, so building and writing it recurse once per
//! declared nesting level and never per element.

use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::{CollectionKind, IeeeWidth, ObjectReference, Value};

use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::state::{ModelState, StateModel};

/// One JSON value in FR-181's typed canonical form, before it is written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Enc {
    Null,
    Bool(bool),
    Str(String),
    Array(Vec<Enc>),
    /// Members in any order: the writer sorts them (RFC 8785 section 3.2.3).
    Object(Vec<(&'static str, Enc)>),
    /// A value FR-181's typed canonical form does not yet cover here; it has
    /// no encoding, and writing it refuses with this description.
    Unencodable(&'static str),
}

impl Encode for Enc {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        match self {
            Self::Null => writer.null(),
            Self::Bool(value) => writer.bool(*value),
            Self::Str(value) => writer.string(value),
            Self::Array(items) => {
                writer.begin_array()?;
                for item in items {
                    item.encode_into(writer)?;
                }
                writer.end_array()
            }
            Self::Object(members) => {
                writer.begin_object()?;
                for (name, value) in members {
                    writer.name(name)?;
                    value.encode_into(writer)?;
                }
                writer.end_object()
            }
            Self::Unencodable(what) => Err(Error::Serialize(format!(
                "{what} has no typed canonical form in a model state"
            ))),
        }
    }
}

impl Enc {
    pub(super) fn text(value: impl Into<String>) -> Self {
        Self::Str(value.into())
    }

    /// `{"type": tag, ...members}`.
    fn tagged(tag: &str, mut members: Vec<(&'static str, Enc)>) -> Self {
        members.push(("type", Self::text(tag)));
        Self::Object(members)
    }

    /// This value's canonical RFC 8785 text.
    ///
    /// # Errors
    ///
    /// [`Error`] when the tree holds an unencodable value or exceeds `limits`.
    pub(super) fn canonical_text(&self, limits: Limits) -> Result<String, Error> {
        let bytes = quire_canonical::to_vec(self, limits)?;
        String::from_utf8(bytes).map_err(|_| Error::Serialize("canonical text is UTF-8".into()))
    }
}

/// The state key an engine encodes: FR-181's `simulation-state` envelope.
pub struct ModelKey(pub(super) Enc);

impl Encode for ModelKey {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.0.encode_into(writer)
    }
}

/// Lowercase hex of `bytes`.
pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// A declaration's identity text: the RFC 8785 text of its
/// `$defs.DeclarationKey` preimage (FR-120 "Identities").
pub(super) fn declaration_enc(key: &DeclarationKey) -> Enc {
    Enc::Object(vec![
        ("digest_domain", Enc::text("sha256-jcs")),
        ("node", Enc::text(key.node.clone())),
        ("package", Enc::text(key.package.clone())),
    ])
}

/// The declaration identity text of `key`.
pub(super) fn declaration_text(key: &DeclarationKey, limits: Limits) -> Result<String, Error> {
    declaration_enc(key).canonical_text(limits)
}

/// FR-181's reference triple.
pub(super) fn reference_enc(reference: &ObjectReference) -> Enc {
    Enc::tagged(
        "reference",
        vec![
            ("universe", Enc::text(hex(reference.universe().as_bytes()))),
            (
                "object_type",
                Enc::text(hex(reference.object_type().as_bytes())),
            ),
            (
                "identity",
                Enc::text(hex(reference.object().as_str().as_bytes())),
            ),
        ],
    )
}

/// FR-181's typed canonical form of `value`.
pub(super) fn value_enc(value: &Value) -> Enc {
    match value {
        Value::Boolean(flag) => Enc::tagged("boolean", vec![("value", Enc::Bool(*flag))]),
        Value::Integer(integer) => {
            Enc::tagged("integer", vec![("value", Enc::text(integer.to_string()))])
        }
        Value::Rational(rational) => Enc::tagged(
            "rational",
            vec![
                ("numerator", Enc::text(rational.numerator().to_string())),
                ("denominator", Enc::text(rational.denominator().to_string())),
            ],
        ),
        Value::Float(float) => {
            let (tag, digits) = match float.width() {
                IeeeWidth::Binary32 => ("float32", 8),
                IeeeWidth::Binary64 => ("float64", 16),
            };
            Enc::tagged(
                tag,
                vec![("bits", Enc::text(format!("{:0digits$x}", float.bits())))],
            )
        }
        Value::Text(text) => Enc::tagged("text", vec![("value", Enc::text(text.retained()))]),
        Value::String(text) => Enc::tagged("text", vec![("value", Enc::text(text.as_str()))]),
        Value::Option(option) => Enc::tagged(
            "option",
            vec![("value", option.payload().map_or(Enc::Null, value_enc))],
        ),
        Value::Collection(collection) => {
            let tag = match collection.collection_type().kind() {
                CollectionKind::Sequence => "sequence",
                CollectionKind::Set => "set",
                CollectionKind::Bag => "bag",
                CollectionKind::OrderedSet => "ordered-set",
            };
            let mut elements: Vec<Enc> = collection.elements().iter().map(value_enc).collect();
            if matches!(
                collection.collection_type().kind(),
                CollectionKind::Set | CollectionKind::Bag
            ) {
                // Sorted by each element's JCS bytes, ascending (FR-181).
                let limits = Limits::new(u64::MAX);
                elements.sort_by_cached_key(|element| {
                    quire_canonical::to_vec(element, limits).unwrap_or_default()
                });
            }
            Enc::tagged(tag, vec![("elements", Enc::Array(elements))])
        }
        Value::Reference(reference) => reference_enc(reference),
        Value::Decimal(_) => Enc::Unencodable("a decimal value"),
        Value::Quantity(_) => Enc::Unencodable("a quantity value"),
        Value::Uuid(_) => Enc::Unencodable("a uuid value"),
        Value::Timestamp(_) => Enc::Unencodable("a timestamp value"),
        Value::Enum(_) => Enc::Unencodable("an enum value"),
        Value::Population(_) => Enc::Unencodable("a population value"),
        Value::Composite(_) => Enc::Unencodable("a record or tuple value"),
        Value::Union(_) => Enc::Unencodable("a union value"),
    }
}

/// Whether two values have the same typed canonical form. A value with no
/// form equals nothing.
pub(super) fn values_equal(left: &Value, right: &Value) -> bool {
    let (left, right) = (value_enc(left), value_enc(right));
    !matches!(
        (&left, &right),
        (Enc::Unencodable(_), _) | (_, Enc::Unencodable(_))
    ) && left == right
}

/// FR-181's `map`: `{"type":"map","entries":[{"key":..,"value":..}, ...]}`
/// with the entries ascending by the key's RFC 8785 bytes.
pub(super) fn map_enc(entries: Vec<(Enc, Enc)>) -> Enc {
    let limits = Limits::new(u64::MAX);
    let mut keyed: Vec<(Vec<u8>, Enc, Enc)> = entries
        .into_iter()
        .map(|(key, value)| {
            let bytes = quire_canonical::to_vec(&key, limits).unwrap_or_default();
            (bytes, key, value)
        })
        .collect();
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    Enc::tagged(
        "map",
        vec![(
            "entries",
            Enc::Array(
                keyed
                    .into_iter()
                    .map(|(_, key, value)| Enc::Object(vec![("key", key), ("value", value)]))
                    .collect(),
            ),
        )],
    )
}

/// The `semantic` member of a model state's key (FR-120 "State, state key
/// and anchor"): each population's declaration identity text, mapped to its
/// objects' reference triples, each mapped to a `record` of its declared
/// fields in declaration order.
pub(super) fn semantic_enc(model: &StateModel, state: &ModelState) -> Enc {
    let mut populations = Vec::new();
    for (population, objects) in &state.populations {
        let mut entries = Vec::new();
        for (key, object) in objects {
            let Ok(reference) = model.object_reference(&object.type_identity, key) else {
                return Enc::Unencodable("an object whose reference does not resolve");
            };
            let Ok(Some(fields)) = model.fields(&object.type_identity) else {
                return Enc::Unencodable("an object of an undeclared type");
            };
            let fields = fields
                .iter()
                .map(|field| {
                    // A `?` field is an `Option<T>` in the typed form: present
                    // or absent, never a bare value or `null`.
                    let value = match (field.optional, object.fields.get(&field.key)) {
                        (true, present) => Enc::tagged(
                            "option",
                            vec![("value", present.map_or(Enc::Null, value_enc))],
                        ),
                        (false, Some(present)) => value_enc(present),
                        (false, None) => Enc::Null,
                    };
                    Enc::Object(vec![
                        ("name", Enc::text(field.name.clone())),
                        ("value", value),
                    ])
                })
                .collect();
            let record = Enc::tagged(
                "record",
                vec![
                    ("name", declaration_enc_text(&object.type_identity)),
                    ("fields", Enc::Array(fields)),
                ],
            );
            entries.push((reference_enc(&reference), record));
        }
        populations.push((
            Enc::tagged("text", vec![("value", declaration_enc_text(population))]),
            map_enc(entries),
        ));
    }
    map_enc(populations)
}

/// A declaration's identity text as a string value.
fn declaration_enc_text(key: &DeclarationKey) -> Enc {
    match declaration_text(key, Limits::new(u64::MAX)) {
        Ok(text) => Enc::Str(text),
        Err(_) => Enc::Unencodable("a declaration identity"),
    }
}

/// The empty `map`.
fn empty_map() -> Enc {
    map_enc(Vec::new())
}

/// FR-181's state key for a model state (FR-120 "State, state key and
/// anchor"): the five members a model state does not use are empty maps.
pub(super) fn model_key(model: &StateModel, state: &ModelState) -> ModelKey {
    ModelKey(Enc::tagged(
        "simulation-state",
        vec![
            ("semantic", semantic_enc(model, state)),
            ("control", empty_map()),
            ("queues", empty_map()),
            ("roles", empty_map()),
            ("observations", empty_map()),
            ("bounds", empty_map()),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use quire_exact::Integer;

    use super::*;

    /// FR-181's integer form is an exact decimal string.
    #[test]
    fn an_integer_encodes_as_a_tagged_decimal_string() {
        let text = value_enc(&Value::Integer(Integer::from(-12_i64)))
            .canonical_text(Limits::new(1024))
            .expect("an integer encodes");
        assert_eq!(text, r#"{"type":"integer","value":"-12"}"#);
    }

    /// A value with no form refuses to write rather than writing a guess.
    #[test]
    fn a_value_without_a_form_refuses_to_encode() {
        let text = Enc::Unencodable("a union value").canonical_text(Limits::new(1024));
        assert!(matches!(text, Err(Error::Serialize(_))));
    }
}
