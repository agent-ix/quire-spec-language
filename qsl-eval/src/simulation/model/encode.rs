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

use qsl_semantics::model::key::{effective_id_from_digest, DeclarationKey};
use qsl_semantics::model::state::{ModelState, StateModel};

/// One JSON value in FR-181's typed canonical form, before it is written.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Enc {
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
    pub(crate) fn text(value: impl Into<String>) -> Self {
        Self::Str(value.into())
    }

    /// `{"type": tag, ...members}`.
    pub(crate) fn tagged(tag: &str, mut members: Vec<(&'static str, Enc)>) -> Self {
        members.push(("type", Self::text(tag)));
        Self::Object(members)
    }

    /// This value's canonical RFC 8785 text.
    ///
    /// # Errors
    ///
    /// [`Error`] when the tree holds an unencodable value or exceeds `limits`.
    pub(crate) fn canonical_text(&self, limits: Limits) -> Result<String, Error> {
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
pub(crate) fn hex(bytes: &[u8]) -> String {
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
pub(crate) fn declaration_enc(key: &DeclarationKey) -> Enc {
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
pub(crate) fn reference_enc(reference: &ObjectReference) -> Enc {
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

/// The reference [`reference_enc`] wrote as `text`: its canonical text names
/// the members in sorted order, so the text is matched member by member.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[qsl_attrs::string_edge]
pub(super) fn parse_reference(text: &str) -> Option<ObjectReference> {
    let rest = text.strip_prefix(r#"{"identity":""#)?;
    let (identity, rest) = rest.split_once(r#"","object_type":""#)?;
    let (object_type, rest) = rest.split_once(r#"","type":"reference","universe":""#)?;
    let universe = rest.strip_suffix(r#""}"#)?;
    let identity = String::from_utf8(unhex(identity)?).ok()?;
    Some(ObjectReference::new(
        quire_exact::UniverseId::from_digest(unhex32(universe)?),
        effective_id_from_digest(unhex32(object_type)?),
        quire_exact::ObjectId::new(identity).ok()?,
    ))
}

/// The reference a parameter's canonical text names: the text of a reference,
/// or of an option holding one. `Some(None)` is an absent option.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[qsl_attrs::string_edge]
pub(super) fn parse_optional_reference(text: &str) -> Option<Option<ObjectReference>> {
    if let Some(inner) = text
        .strip_prefix(r#"{"type":"option","value":"#)
        .and_then(|rest| rest.strip_suffix('}'))
    {
        return if inner == "null" {
            Some(None)
        } else {
            parse_reference(inner).map(Some)
        };
    }
    parse_reference(text).map(Some)
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    let lower_hex = |byte: u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte);
    if !text.len().is_multiple_of(2) || !text.bytes().all(lower_hex) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok())
        .collect()
}

fn unhex32(text: &str) -> Option<[u8; 32]> {
    unhex(text)?.try_into().ok()
}

/// FR-181's typed canonical form of `value`.
pub(crate) fn value_enc(value: &Value) -> Enc {
    value_enc_with(value, &|_| None)
}

/// A reference's replacement in a key-free encoding (FR-152): `Some` for a
/// reference the encoding names by a placeholder, `None` to encode it.
pub(super) type Placeholder<'p> = dyn Fn(&ObjectReference) -> Option<Enc> + 'p;

/// FR-181's typed canonical form of `value`, with each reference
/// `placeholder` replaces written as its replacement.
pub(super) fn value_enc_with(value: &Value, placeholder: &Placeholder<'_>) -> Enc {
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
            vec![(
                "value",
                option
                    .payload()
                    .map_or(Enc::Null, |payload| value_enc_with(payload, placeholder)),
            )],
        ),
        Value::Collection(collection) => {
            let tag = match collection.collection_type().kind() {
                CollectionKind::Sequence => "sequence",
                CollectionKind::Set => "set",
                CollectionKind::Bag => "bag",
                CollectionKind::OrderedSet => "ordered-set",
            };
            let mut elements: Vec<Enc> = collection
                .elements()
                .iter()
                .map(|element| value_enc_with(element, placeholder))
                .collect();
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
        Value::Reference(reference) => {
            placeholder(reference).unwrap_or_else(|| reference_enc(reference))
        }
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
pub(crate) fn map_enc(entries: Vec<(Enc, Enc)>) -> Enc {
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
pub(crate) fn semantic_enc(model: &StateModel, state: &ModelState) -> Enc {
    let mut populations = Vec::new();
    for (population, objects) in &state.populations {
        let mut entries = Vec::new();
        for (key, object) in objects {
            let Ok(reference) = model.object_reference(&object.type_identity, key) else {
                return Enc::Unencodable("an object whose reference does not resolve");
            };
            let record = object_enc(model, object, &|_| None);
            entries.push((reference_enc(&reference), record));
        }
        populations.push((
            Enc::tagged("text", vec![("value", declaration_enc_text(population))]),
            map_enc(entries),
        ));
    }
    map_enc(populations)
}

/// One object's `record` in a state key (FR-120): its type name and its
/// declared fields in declaration order, each reference `placeholder`
/// replaces written as its replacement.
pub(super) fn object_enc(
    model: &StateModel,
    object: &qsl_semantics::model::state::StateObject,
    placeholder: &Placeholder<'_>,
) -> Enc {
    let Ok(Some(fields)) = model.fields(&object.type_identity) else {
        return Enc::Unencodable("an object of an undeclared type");
    };
    let fields = fields
        .iter()
        .map(|field| {
            // QSpec FR-181: a field declared `Option<T>` holds an option
            // value (encoded by `value_enc`); an absent presence-optional
            // field is a plain `null` entry.
            let value = object
                .fields
                .get(&field.key)
                .map_or(Enc::Null, |value| value_enc_with(value, placeholder));
            Enc::Object(vec![
                ("name", Enc::text(field.name.clone())),
                ("value", value),
            ])
        })
        .collect();
    Enc::tagged(
        "record",
        vec![
            ("name", declaration_enc_text(&object.type_identity)),
            ("fields", Enc::Array(fields)),
        ],
    )
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
            ("memory", empty_map()),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{Integer, ValueType};

    use super::*;

    /// FR-181's integer form is an exact decimal string.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn an_integer_encodes_as_a_tagged_decimal_string() {
        let text = value_enc(&Value::Integer(Integer::from(-12_i64)))
            .canonical_text(Limits::new(1024))
            .expect("an integer encodes");
        assert_eq!(text, r#"{"type":"integer","value":"-12"}"#);
    }

    /// A value with no form refuses to write rather than writing a guess.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn a_value_without_a_form_refuses_to_encode() {
        let text = Enc::Unencodable("a union value").canonical_text(Limits::new(1024));
        assert!(matches!(text, Err(Error::Serialize(_))));
    }

    /// QSpec FR-181: a field declared `Option<T>` is an option value, none
    /// `{"type":"option","value":null}` and some wrapping the encoded payload.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn an_option_value_encodes_as_an_option_object() {
        let int = ValueType::Int(
            quire_exact::IntegerInterval::new(Integer::from(0_i64), Integer::from(2_i64))
                .expect("nonempty"),
        );
        let none = quire_exact::OptionValue::none(int.clone());
        let some = quire_exact::OptionValue::from_admitted(
            int,
            Some(Value::Integer(Integer::from(1_i64))),
        );
        let text = |value: &Value| {
            value_enc(value)
                .canonical_text(Limits::new(1024))
                .expect("an option encodes")
        };
        assert_eq!(text(&none), r#"{"type":"option","value":null}"#);
        assert_eq!(
            text(&some),
            r#"{"type":"option","value":{"type":"integer","value":"1"}}"#
        );
    }
}
