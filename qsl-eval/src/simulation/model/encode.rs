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
use quire_exact::{CollectionKind, FieldValue, IeeeWidth, ObjectReference, Value};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};

use qsl_semantics::check::VariantTypeDeclaration;
use qsl_semantics::model::key::effective_id_from_digest;
use qsl_semantics::model::state::{ModelState, StateModel};
use quire_checked::key::DeclarationKey;

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
#[ix_trace_rs::string_edge]
pub(crate) fn parse_reference(text: &str) -> Option<ObjectReference> {
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
#[ix_trace_rs::string_edge]
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

/// The declarations a value's typed canonical form reads: the package's
/// record, tuple and union types and, where the caller has them, its
/// variant (enum) types.
#[derive(Clone, Copy)]
pub(crate) struct Typing<'t> {
    /// `None` only where a value is compared, not encoded: see
    /// [`values_equal`].
    types: Option<&'t TypeEnvironment>,
    variants: &'t [VariantTypeDeclaration],
}

impl<'t> Typing<'t> {
    /// No declarations: a record, tuple or enum value has no form.
    fn none() -> Self {
        Self {
            types: None,
            variants: &[],
        }
    }

    /// The record, tuple and union types of `types` and the variant types
    /// `variants`.
    pub(crate) fn with_variants(
        types: &'t TypeEnvironment,
        variants: &'t [VariantTypeDeclaration],
    ) -> Self {
        Self {
            types: Some(types),
            variants,
        }
    }

    /// The present value of the record field `name` of the record value
    /// `value`; `None` when `value` is no record, the field is absent or
    /// its declaration is not in these types.
    pub(crate) fn component<'v>(&self, value: &'v Value, name: &str) -> Option<&'v Value> {
        let Value::Composite(composite) = value else {
            return None;
        };
        let Some(CompositeShape::Record(fields)) = self
            .types?
            .composite(composite.declaration())
            .map(CompositeDeclaration::shape)
        else {
            return None;
        };
        let position = fields.iter().position(|field| field.name() == name)?;
        match composite.slots().get(position)? {
            FieldValue::Present(value) => Some(value),
            FieldValue::Absent | FieldValue::Null => None,
        }
    }
}

/// FR-181's typed canonical form of `value`, its records, tuples, unions and
/// enums read from `typing`.
pub(crate) fn value_enc(value: &Value, typing: Typing<'_>) -> Enc {
    value_enc_with(value, typing, &|_| None)
}

/// FR-120's typed canonical form of `value`, where a record value is
/// `{"type":"record","name":…,"fields":[{name, value}…]}` with `name` the
/// declaration's checked node id, one entry per declared field in
/// declaration order and an absent field `null`; a tuple is
/// `{"type":"tuple","name":…,"components":[…]}`; a union is
/// `{"type":"union","name":…,"member":…,"components":[…]}`; and an enum is
/// `{"type":"enum","name":…,"member":…}` with `name` the variant type's node
/// id and `member` the declared case, both read from `variants`.
pub(crate) fn typed_value_enc(
    value: &Value,
    types: &TypeEnvironment,
    variants: &[VariantTypeDeclaration],
) -> Enc {
    value_enc(value, Typing::with_variants(types, variants))
}

/// FR-181's `record` or `tuple` form of `composite`, its declaration read
/// from `types`: `{"type":"record","name":…,"fields":[{name, value}…]}`
/// with `name` the declaration's checked node id, one entry per declared
/// field in declaration order and an absent field `null`, or
/// `{"type":"tuple","name":…,"components":[…]}` in declaration order; each
/// slot written by `slot_enc`.
fn composite_enc(
    composite: &quire_exact::CompositeValue,
    types: &TypeEnvironment,
    slot_enc: &dyn Fn(&FieldValue) -> Enc,
) -> Enc {
    let name = Enc::text(hex(composite.declaration().as_bytes()));
    match types
        .composite(composite.declaration())
        .map(CompositeDeclaration::shape)
    {
        Some(CompositeShape::Record(fields)) if fields.len() == composite.slots().len() => {
            let entries = fields
                .iter()
                .zip(composite.slots())
                .map(|(field, slot)| {
                    Enc::Object(vec![
                        ("name", Enc::text(field.name())),
                        ("value", slot_enc(slot)),
                    ])
                })
                .collect();
            Enc::tagged(
                "record",
                vec![("name", name), ("fields", Enc::Array(entries))],
            )
        }
        Some(CompositeShape::Tuple(_)) => Enc::tagged(
            "tuple",
            vec![
                ("name", name),
                (
                    "components",
                    Enc::Array(composite.slots().iter().map(slot_enc).collect()),
                ),
            ],
        ),
        Some(CompositeShape::Record(_)) => {
            Enc::Unencodable("a record whose slots are not its declared fields")
        }
        Some(CompositeShape::Union(_)) | None => Enc::Unencodable("an undeclared record"),
    }
}

/// A reference's replacement in a key-free encoding (FR-152): `Some` for a
/// reference the encoding names by a placeholder, `None` to encode it.
pub(super) type Placeholder<'p> = dyn Fn(&ObjectReference) -> Option<Enc> + 'p;

/// FR-181's typed canonical form of `value`, as [`value_enc`], with each
/// reference `placeholder` replaces written as its replacement, recursively
/// through options, collections, records, tuples and unions.
pub(super) fn value_enc_with(
    value: &Value,
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
) -> Enc {
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
        Value::Quantity(quantity) => {
            let magnitude = quantity.magnitude();
            Enc::tagged(
                "rational",
                vec![
                    ("numerator", Enc::text(magnitude.numerator().to_string())),
                    (
                        "denominator",
                        Enc::text(magnitude.denominator().to_string()),
                    ),
                    ("unit", Enc::text(hex(quantity.unit().as_bytes()))),
                ],
            )
        }
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
                option.payload().map_or(Enc::Null, |payload| {
                    value_enc_with(payload, typing, placeholder)
                }),
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
                .map(|element| value_enc_with(element, typing, placeholder))
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
        Value::Uuid(value) => Enc::tagged("uuid", vec![("value", Enc::text(value.to_string()))]),
        Value::Timestamp(value) => Enc::tagged(
            "timestamp",
            vec![("nanoseconds", Enc::text(value.to_string()))],
        ),
        Value::Enum(member) => typing
            .variants
            .iter()
            .find_map(|declared| {
                declared
                    .cases()
                    .iter()
                    .find(|case| case.variant() == member.variant())
                    .map(|case| (declared, case))
            })
            .map_or(
                Enc::Unencodable("an undeclared enum value"),
                |(declared, case)| {
                    Enc::tagged(
                        "enum",
                        vec![
                            ("name", Enc::text(hex(declared.node().as_bytes()))),
                            ("member", Enc::text(case.name())),
                        ],
                    )
                },
            ),
        Value::Population(_) => Enc::Unencodable("a population value"),
        Value::Composite(composite) => match typing.types {
            Some(types) => composite_enc(composite, types, &|slot| match slot {
                FieldValue::Present(inner) => value_enc_with(inner, typing, placeholder),
                FieldValue::Absent | FieldValue::Null => Enc::Null,
            }),
            None => Enc::Unencodable("a record or tuple value"),
        },
        Value::Union(union) => Enc::tagged(
            "union",
            vec![
                ("name", Enc::text(hex(union.declaration().as_bytes()))),
                ("member", Enc::text(union.member().identifier().as_str())),
                (
                    "components",
                    Enc::Array(
                        union
                            .payload()
                            .iter()
                            .map(|inner| value_enc_with(inner, typing, placeholder))
                            .collect(),
                    ),
                ),
            ],
        ),
    }
}

/// Whether two values have the same typed canonical form. A value with no
/// form equals nothing. Two records, tuples or unions are compared
/// component by component, which is what their forms compare: one
/// declaration, one member, and the same component at each slot. Two enum
/// members are compared by variant and rank, which is what an enum's form
/// names; options and collections, which may hold any of these, element by
/// element.
pub(super) fn values_equal(left: &Value, right: &Value) -> bool {
    let slot_equal = |left: &FieldValue, right: &FieldValue| match (left, right) {
        (FieldValue::Present(left), FieldValue::Present(right)) => values_equal(left, right),
        (FieldValue::Absent | FieldValue::Null, FieldValue::Absent | FieldValue::Null) => true,
        _ => false,
    };
    match (left, right) {
        (Value::Composite(left), Value::Composite(right)) => {
            left.declaration() == right.declaration()
                && left.slots().len() == right.slots().len()
                && left
                    .slots()
                    .iter()
                    .zip(right.slots())
                    .all(|(left, right)| slot_equal(left, right))
        }
        (Value::Union(left), Value::Union(right)) => {
            left.declaration() == right.declaration()
                && left.member().identifier() == right.member().identifier()
                && left.payload().len() == right.payload().len()
                && left
                    .payload()
                    .iter()
                    .zip(right.payload())
                    .all(|(left, right)| values_equal(left, right))
        }
        (Value::Enum(left), Value::Enum(right)) => left == right,
        (Value::Option(left), Value::Option(right)) => {
            left.payload_type() == right.payload_type()
                && match (left.payload(), right.payload()) {
                    (Some(left), Some(right)) => values_equal(left, right),
                    (None, None) => true,
                    _ => false,
                }
        }
        (Value::Collection(left), Value::Collection(right)) => {
            left.collection_type() == right.collection_type()
                && left.elements().len() == right.elements().len()
                && left
                    .elements()
                    .iter()
                    .zip(right.elements())
                    .all(|(left, right)| values_equal(left, right))
        }
        (
            Value::Composite(_)
            | Value::Union(_)
            | Value::Enum(_)
            | Value::Option(_)
            | Value::Collection(_),
            _,
        )
        | (
            _,
            Value::Composite(_)
            | Value::Union(_)
            | Value::Enum(_)
            | Value::Option(_)
            | Value::Collection(_),
        ) => false,
        _ => {
            let (left, right) = (untyped_value_enc(left), untyped_value_enc(right));
            !matches!(
                (&left, &right),
                (Enc::Unencodable(_), _) | (_, Enc::Unencodable(_))
            ) && left == right
        }
    }
}

/// A value's typed canonical form where no package declarations are at
/// hand: every family but records, tuples and enums, which have none here.
pub(crate) fn untyped_value_enc(value: &Value) -> Enc {
    value_enc(value, Typing::none())
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
pub(crate) fn semantic_enc(model: &StateModel, state: &ModelState, typing: Typing<'_>) -> Enc {
    let mut populations = Vec::new();
    for (population, objects) in &state.populations {
        let mut entries = Vec::new();
        for (key, object) in objects {
            let Ok(reference) = model.object_reference(&object.type_identity, key) else {
                return Enc::Unencodable("an object whose reference does not resolve");
            };
            let record = object_enc(model, object, typing, &|_| None);
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
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
) -> Enc {
    let Ok(Some(fields)) = model.fields(&object.type_identity) else {
        return Enc::Unencodable("an object of an undeclared type");
    };
    let fields = fields
        .iter()
        .map(|field| {
            // QSpec FR-181: a field declared `Option<T>` holds an option
            // value; an absent presence-optional field is a plain `null`
            // entry; a record, tuple, union or enum value takes its declared
            // form from `typing`.
            let value = object.fields.get(&field.key).map_or(Enc::Null, |value| {
                value_enc_with(value, typing, placeholder)
            });
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
pub(super) fn model_key(model: &StateModel, state: &ModelState, typing: Typing<'_>) -> ModelKey {
    ModelKey(Enc::tagged(
        "simulation-state",
        vec![
            ("semantic", semantic_enc(model, state, typing)),
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
    use quire_exact::{
        from_admitted, CollectionKind, CollectionType, Integer, NodeKey, Quantity, Rational,
        Timestamp, UnitId, Uuid, ValueType,
    };

    use super::*;

    /// FR-181's integer form is an exact decimal string.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn an_integer_encodes_as_a_tagged_decimal_string() {
        let text = untyped_value_enc(&Value::Integer(Integer::from(-12_i64)))
            .canonical_text(Limits::new(1024))
            .expect("an integer encodes");
        assert_eq!(text, r#"{"type":"integer","value":"-12"}"#);
    }

    /// FR-181-AC-19: UUID and Timestamp use their distinct typed forms.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn uuid_and_timestamp_encode_in_their_typed_forms() {
        let uuid = Uuid::from_canonical_text("00112233-4455-6677-8899-aabbccddeeff")
            .expect("canonical UUID");
        let timestamp = Timestamp::from_canonical_text("-1").expect("canonical timestamp");

        assert_eq!(
            untyped_value_enc(&Value::Uuid(uuid))
                .canonical_text(Limits::new(1024))
                .expect("a UUID encodes"),
            r#"{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}"#
        );
        assert_eq!(
            untyped_value_enc(&Value::Timestamp(timestamp))
                .canonical_text(Limits::new(1024))
                .expect("a Timestamp encodes"),
            r#"{"nanoseconds":"-1","type":"timestamp"}"#
        );
    }

    /// FR-181-AC-19/20: UUID and Timestamp set members use their typed forms
    /// and sets order those forms by their encoded JCS bytes.
    #[trace("TC-1195", "FR-120-AC-17")]
    #[test]
    fn uuid_and_timestamp_sets_encode_in_canonical_byte_order() {
        let uuid = |value| Value::Uuid(Uuid::from_canonical_text(value).expect("canonical UUID"));
        let timestamp = |value| {
            Value::Timestamp(Timestamp::from_canonical_text(value).expect("canonical timestamp"))
        };
        let ids = from_admitted(
            CollectionType::new(CollectionKind::Set, ValueType::Uuid, None),
            vec![
                uuid("00112233-4455-6677-8899-aabbccddeeff"),
                uuid("00112233-4455-6677-8899-aabbccddeefe"),
            ],
        );
        let times = from_admitted(
            CollectionType::new(CollectionKind::Set, ValueType::Timestamp, None),
            vec![timestamp("2"), timestamp("10")],
        );

        assert_eq!(
            untyped_value_enc(&ids)
                .canonical_text(Limits::new(1024))
                .expect("the UUID set encodes"),
            r#"{"elements":[{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeefe"},{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}],"type":"set"}"#
        );
        assert_eq!(
            untyped_value_enc(&times)
                .canonical_text(Limits::new(1024))
                .expect("the Timestamp set encodes"),
            r#"{"elements":[{"nanoseconds":"10","type":"timestamp"},{"nanoseconds":"2","type":"timestamp"}],"type":"set"}"#
        );
    }

    /// The declarations of FR-120-AC-16's `Tag { sku: Integer, note?:
    /// Boolean }` (node `11…`), `Pair(Boolean, Integer)` (`22…`) and `Shape`
    /// with the nullary member `none` and the member `tagged(Tag)` (`33…`),
    /// and the values `Tag { sku }`, `Pair(true, -3)` and `none`.
    fn declared() -> (TypeEnvironment, impl Fn(i64) -> Value, Value, Value) {
        use quire_exact::{from_admitted_slots, Identifier, Presence, UnionMember, UnionValue};
        use quire_semantic_value::declaration::{
            CompositeDeclaration, FieldDeclaration, UnionMemberDeclaration,
        };
        let (tag, pair, shape) = (
            NodeKey::from_digest([0x11; 32]),
            NodeKey::from_digest([0x22; 32]),
            NodeKey::from_digest([0x33; 32]),
        );
        let none = UnionMember::from_admitted(
            shape,
            NodeKey::from_digest([0x34; 32]),
            Identifier::new("none").expect("an identifier"),
        );
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    tag,
                    "Tag",
                    CompositeShape::Record(vec![
                        FieldDeclaration::new("sku", ValueType::Integer, Presence::Required),
                        FieldDeclaration::new("note", ValueType::Boolean, Presence::Optional),
                    ]),
                ),
                CompositeDeclaration::new(
                    pair,
                    "Pair",
                    CompositeShape::Tuple(vec![ValueType::Boolean, ValueType::Integer]),
                ),
                CompositeDeclaration::new(
                    shape,
                    "Shape",
                    CompositeShape::Union(vec![UnionMemberDeclaration::from_verified(
                        none.clone(),
                        vec![],
                    )]),
                ),
            ],
            [],
        )
        .expect("the declarations admit");
        let tag_value = move |sku: i64| {
            from_admitted_slots(
                tag,
                vec![
                    FieldValue::Present(Value::Integer(Integer::from(sku))),
                    FieldValue::Absent,
                ]
                .into_boxed_slice(),
            )
        };
        let pair_value = from_admitted_slots(
            pair,
            vec![
                FieldValue::Present(Value::Boolean(true)),
                FieldValue::Present(Value::Integer(Integer::from(-3_i64))),
            ]
            .into_boxed_slice(),
        );
        let none_value = UnionValue::from_admitted(none, vec![]);
        (types, tag_value, pair_value, none_value)
    }

    /// FR-120-AC-16 (TC-1195): FR-181-AC-19's record, tuple and nullary
    /// union bytes, each `name` the declaration's checked node id in 64 hex.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn a_record_tuple_and_union_encode_in_their_declared_forms() {
        let (types, tag, pair, none) = declared();
        let text = |value: &Value| {
            value_enc(value, Typing::with_variants(&types, &[]))
                .canonical_text(Limits::new(4096))
                .expect("the value encodes")
        };
        let (tag_name, pair_name, shape_name) = ("11".repeat(32), "22".repeat(32), "33".repeat(32));
        assert_eq!(
            text(&tag(7)),
            format!(
                r#"{{"fields":[{{"name":"sku","value":{{"type":"integer","value":"7"}}}},{{"name":"note","value":null}}],"name":"{tag_name}","type":"record"}}"#
            )
        );
        assert_eq!(
            text(&pair),
            format!(
                r#"{{"components":[{{"type":"boolean","value":true}},{{"type":"integer","value":"-3"}}],"name":"{pair_name}","type":"tuple"}}"#
            )
        );
        assert_eq!(
            text(&none),
            format!(r#"{{"components":[],"member":"none","name":"{shape_name}","type":"union"}}"#)
        );
        // Without the declarations a record has no form; a union keeps its.
        assert!(matches!(untyped_value_enc(&tag(7)), Enc::Unencodable(_)));
    }

    /// FR-120-AC-16 (TC-1195): the declared forms nest through an option and
    /// a set, whose records are ordered by their encoded bytes; equality
    /// compares records component by component.
    #[trace("TC-1195")]
    #[test]
    fn declared_forms_nest_through_options_and_sets() {
        let (types, tag, _, _) = declared();
        let typing = Typing::with_variants(&types, &[]);
        let record = |sku: &str| {
            format!(
                r#"{{"fields":[{{"name":"sku","value":{{"type":"integer","value":"{sku}"}}}},{{"name":"note","value":null}}],"name":"{}","type":"record"}}"#,
                "11".repeat(32)
            )
        };
        let tag_type = ValueType::Composite(NodeKey::from_digest([0x11; 32]));
        let option = quire_exact::OptionValue::from_admitted(tag_type.clone(), Some(tag(7)));
        assert_eq!(
            value_enc(&option, typing)
                .canonical_text(Limits::new(4096))
                .expect("the option encodes"),
            format!(r#"{{"type":"option","value":{}}}"#, record("7"))
        );
        let set = from_admitted(
            CollectionType::new(CollectionKind::Set, tag_type.clone(), None),
            vec![tag(7), tag(10)],
        );
        assert_eq!(
            value_enc(&set, typing)
                .canonical_text(Limits::new(4096))
                .expect("the set encodes"),
            format!(
                r#"{{"elements":[{},{}],"type":"set"}}"#,
                record("10"),
                record("7")
            )
        );
        assert!(values_equal(&tag(7), &tag(7)));
        assert!(!values_equal(&tag(7), &tag(10)));
        // Options and collections of records, and enum members, compare
        // element by element and by variant and rank (SR-1646005).
        let other = quire_exact::OptionValue::from_admitted(tag_type.clone(), Some(tag(10)));
        assert!(values_equal(&option, &option.clone()));
        assert!(!values_equal(&option, &other));
        assert!(values_equal(&set, &set.clone()));
        let member = |rank| {
            Value::Enum(quire_exact::EnumMember::new(
                quire_exact::VariantId::from_digest([0xfd; 32]),
                rank,
            ))
        };
        assert!(values_equal(&member(0), &member(0)));
        assert!(!values_equal(&member(0), &member(1)));
    }

    /// FR-120-AC-16 / FR-189-AC-15: a quantity keeps its exact magnitude and
    /// declared unit identity in the rational wire form.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn a_quantity_encodes_its_rational_magnitude_and_unit() {
        let unit = UnitId::declared(NodeKey::from_digest([0xab; 32]));
        let quantity = Value::Quantity(Quantity::new(
            Rational::new(Integer::from(3_i64), Integer::from(2_i64)).expect("nonzero"),
            unit,
        ));
        let text = untyped_value_enc(&quantity)
            .canonical_text(Limits::new(1024))
            .expect("a quantity encodes");
        assert_eq!(
            text,
            format!(
                r#"{{"denominator":"2","numerator":"3","type":"rational","unit":"{}"}}"#,
                "ab".repeat(32)
            )
        );
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
            untyped_value_enc(value)
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
