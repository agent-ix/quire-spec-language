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

use qsl_foundation::node_wire::{node_key_from_json, node_key_to_json};
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::{
    CollectionKind, EffectiveId, FieldValue, IeeeWidth, NodeKey, ObjectReference,
    OriginalDeclarationKey, UniverseId, Value,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};
use serde_json::Value as Json;

use qsl_semantics::check::VariantTypeDeclaration;
use qsl_semantics::model::state::{ModelState, StateField, StateModel};
use quire_checked::key::DeclarationKey;

/// One JSON value in FR-181's typed canonical form, before it is written.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Enc {
    Null,
    Bool(bool),
    Str(String),
    Array(Vec<Enc>),
    /// A structural node identity, written as its `{package, path, ordinal}`
    /// object.
    Node(NodeKey),
    /// A structural effective declaration identity, written as FR-201's
    /// recursive `{owner_effective_type, original}` record.
    Effective(EffectiveId),
    /// A tagged object, keeping its fixed `type` member out of the members
    /// vector so each tagged node avoids growing that vector.
    Tagged {
        tag: &'static str,
        members: Vec<(&'static str, Enc)>,
    },
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
            Self::Node(key) => {
                let json = node_key_to_json(key)
                    .map_err(|_| Error::Serialize("a node identity nested too deeply".into()))?;
                write_json(writer, &json)
            }
            Self::Effective(id) => write_effective(writer, id),
            Self::Tagged { tag, members } => {
                writer.begin_object()?;
                for (name, value) in members {
                    writer.name(name)?;
                    value.encode_into(writer)?;
                }
                writer.name("type")?;
                writer.string(tag)?;
                writer.end_object()
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
    pub(crate) fn tagged(tag: &'static str, members: Vec<(&'static str, Enc)>) -> Self {
        Self::Tagged { tag, members }
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

/// Write the node identity object `json` (a [`node_key_to_json`] value,
/// nested no deeper than the codec's bound).
fn write_json<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, json: &Json) -> Result<(), Error> {
    match json {
        Json::Null => writer.null(),
        Json::Bool(value) => writer.bool(*value),
        Json::Number(number) => {
            let value = number
                .as_u64()
                .ok_or_else(|| Error::Serialize("a node ordinal is an unsigned integer".into()))?;
            writer.integer(i128::from(value))
        }
        Json::String(text) => writer.string(text),
        Json::Array(items) => {
            writer.begin_array()?;
            for item in items {
                write_json(writer, item)?;
            }
            writer.end_array()
        }
        Json::Object(members) => {
            writer.begin_object()?;
            for (name, value) in members {
                writer.name(name)?;
                write_json(writer, value)?;
            }
            writer.end_object()
        }
    }
}

/// Write `id` as FR-201's recursive record, outermost owner last, one loop
/// iteration per level: the writer's own nesting is the only depth.
fn write_effective<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    id: &EffectiveId,
) -> Result<(), Error> {
    let chain = id.owner_chain();
    for original in chain.iter().rev() {
        writer.begin_object()?;
        writer.name("original")?;
        writer.begin_object()?;
        writer.name("node")?;
        writer.string(original.node())?;
        writer.name("package")?;
        writer.string(original.package())?;
        writer.end_object()?;
        writer.name("owner_effective_type")?;
    }
    writer.null()?;
    for _ in chain {
        writer.end_object()?;
    }
    Ok(())
}

/// A declaration's identity text: the RFC 8785 text of its
/// `$defs.DeclarationKey` preimage `{package, node}` (FR-120 "Identities"),
/// the same text `qsl_semantics::model::key::declaration_identity_text`
/// gives, which checked clauses lower operation labels to.
pub(crate) fn declaration_enc(key: &DeclarationKey) -> Enc {
    Enc::Object(vec![
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
            (
                "universe",
                Enc::Node(reference.universe().root_type().clone()),
            ),
            (
                "object_type",
                Enc::Effective(reference.object_type().clone()),
            ),
            ("identity", Enc::text(reference.object().as_str())),
        ],
    )
}

/// The UnitId text: the RFC 8785 text of its structural identity record.
fn unit_enc(unit: &quire_exact::UnitId) -> Enc {
    identity_text(unit_identity_enc(unit), "a unit identity")
}

fn identity_text(value: Enc, what: &'static str) -> Enc {
    value
        .canonical_text(Limits::new(super::identity_limits().input_bytes))
        .map_or(Enc::Unencodable(what), Enc::text)
}

/// FR-088's UnitId identity record: `{declared: <NodeId>}` or the ordered
/// compound terms.
fn unit_identity_enc(unit: &quire_exact::UnitId) -> Enc {
    if let Some(node) = unit.declared_key() {
        return Enc::Object(vec![("declared", Enc::Node(node.clone()))]);
    }
    let terms = unit.compound_terms().unwrap_or_default();
    Enc::Object(vec![(
        "terms",
        Enc::Array(
            terms
                .iter()
                .map(|(node, exponent)| {
                    Enc::Object(vec![
                        ("unit", Enc::Node(node.clone())),
                        ("exponent", Enc::text(exponent.to_string())),
                    ])
                })
                .collect(),
        ),
    )])
}

/// A checked type node's producer declaration identity, `{package, node}`.
/// Type-environment keys are minted with the declaration's opaque producer
/// node as the final segment of their structural path.
fn declaration_key_for_node(key: &NodeKey) -> Option<DeclarationKey> {
    let Json::Object(fields) = node_key_to_json(key).ok()? else {
        return None;
    };
    let package = fields.get("package")?.as_str()?.to_owned();
    let Json::Array(path) = fields.get("path")? else {
        return None;
    };
    let node = path.last()?.as_str()?.to_owned();
    Some(DeclarationKey { package, node })
}

fn declaration_name(key: &NodeKey) -> Enc {
    declaration_key_for_node(key).map_or(Enc::Unencodable("a declaration identity"), |key| {
        identity_text(declaration_enc(&key), "a declaration identity")
    })
}

/// The first JSON value of `text` and the text after it.
fn take_json(text: &str) -> Option<(Json, &str)> {
    let mut values = serde_json::Deserializer::from_str(text).into_iter::<Json>();
    let value = values.next()?.ok()?;
    let rest = &text[values.byte_offset()..];
    Some((value, rest))
}

/// The effective identity [`write_effective`] wrote at the start of `text`,
/// and the text after it. Reads one record level per loop iteration and
/// refuses any other shape.
#[ix_trace_rs::string_edge]
fn take_effective(text: &str) -> Option<(EffectiveId, &str)> {
    let mut rest = text;
    let mut originals = Vec::new();
    loop {
        rest = rest.strip_prefix(r#"{"original":"#)?;
        let (original, after) = take_json(rest)?;
        let members = original.as_object()?;
        if members.len() != 2 {
            return None;
        }
        originals.push(OriginalDeclarationKey::new(
            members.get("package")?.as_str()?,
            members.get("node")?.as_str()?,
        ));
        rest = after.strip_prefix(r#","owner_effective_type":"#)?;
        if let Some(after_null) = rest.strip_prefix("null") {
            rest = after_null;
            break;
        }
    }
    for _ in &originals {
        rest = rest.strip_prefix('}')?;
    }
    let mut id = None;
    for original in originals.into_iter().rev() {
        id = Some(EffectiveId::new(id, original));
    }
    Some((id?, rest))
}

/// The reference [`reference_enc`] wrote at the start of `text`, and the
/// text after it: its canonical text names the members in sorted order, so
/// the text is matched member by member.
#[ix_trace_rs::string_edge]
fn take_reference(text: &str) -> Option<(ObjectReference, &str)> {
    let rest = text.strip_prefix(r#"{"identity":"#)?;
    let (identity, rest) = take_json(rest)?;
    let rest = rest.strip_prefix(r#","object_type":"#)?;
    let (object_type, rest) = take_effective(rest)?;
    let rest = rest.strip_prefix(r#","type":"reference","universe":"#)?;
    let (universe, rest) = take_json(rest)?;
    let rest = rest.strip_prefix('}')?;
    Some((
        ObjectReference::new(
            UniverseId::new(node_key_from_json(&universe).ok()?),
            object_type,
            quire_exact::ObjectId::new(identity.as_str()?).ok()?,
        ),
        rest,
    ))
}

/// The reference [`reference_enc`] wrote as `text`.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[ix_trace_rs::string_edge]
pub(crate) fn parse_reference(text: &str) -> Option<ObjectReference> {
    match take_reference(text)? {
        (reference, "") => Some(reference),
        _ => None,
    }
}

/// A timed transition's `clock_place` argument split at the object it
/// names: the text before the object's reference, the reference, and the
/// text after it. `None` for any other text.
/// `#[string_edge]`: reads the canonical text this module's own encoder wrote.
#[ix_trace_rs::string_edge]
pub(super) fn split_clock_place(text: &str) -> Option<(&str, ObjectReference, &str)> {
    if !text.ends_with(r#","type":"clock_place"}"#) {
        return None;
    }
    let marker = r#","object":"#;
    let start = text.find(marker)? + marker.len();
    let (reference, after) = take_reference(&text[start..])?;
    Some((&text[..start], reference, after))
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

/// The declarations a value's typed canonical form reads: the package's
/// record, tuple and union types and, where the caller has them, its
/// variant (enum) types.
#[derive(Clone, Copy)]
pub(crate) struct Typing<'t> {
    /// `None` only where a value is compared, not encoded: see
    /// [`values_equal`].
    types: Option<&'t TypeEnvironment>,
    variants: &'t [VariantTypeDeclaration],
    /// The element-count members of the pure-clock collections, which a
    /// record writes after its declared fields.
    clock_counts: &'t [super::ClockCount],
    /// The presence-bit members of the `Option<Clock>` fields, which a
    /// record writes as the field's option shell.
    clock_bits: &'t [(DeclarationKey, StateField)],
}

impl<'t> Typing<'t> {
    /// No declarations: a record, tuple or enum value has no form.
    fn none() -> Self {
        Self {
            types: None,
            variants: &[],
            clock_counts: &[],
            clock_bits: &[],
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
            clock_counts: &[],
            clock_bits: &[],
        }
    }

    /// These declarations with the element-count members `clock_counts`
    /// and the presence-bit members `clock_bits`.
    pub(crate) fn with_clock_members(
        self,
        clock_counts: &'t [super::ClockCount],
        clock_bits: &'t [(DeclarationKey, StateField)],
    ) -> Self {
        Self {
            clock_counts,
            clock_bits,
            ..self
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
            .composite(composite.declaration().clone())
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
/// declaration identity text, one entry per declared field in
/// declaration order and an absent field `null`; a tuple is
/// `{"type":"tuple","name":…,"components":[…]}`; a union is
/// `{"type":"union","name":…,"member":…,"components":[…]}`; and an enum is
/// `{"type":"enum","name":…,"member":…}` with `name` the variant type's
/// declaration identity text and `member` the declared case.
pub(crate) fn typed_value_enc(
    value: &Value,
    types: &TypeEnvironment,
    variants: &[VariantTypeDeclaration],
) -> Enc {
    value_enc(value, Typing::with_variants(types, variants))
}

/// FR-181's `record` or `tuple` form of `composite`, its declaration read
/// from `types`: `{"type":"record","name":…,"fields":[{name, value}…]}`
/// with `name` the declaration identity text, one entry per declared
/// field in declaration order and an absent field `null`, or
/// `{"type":"tuple","name":…,"components":[…]}` in declaration order; each
/// slot written by `slot_enc`.
fn composite_enc(
    composite: &quire_exact::CompositeValue,
    types: &TypeEnvironment,
    slot_enc: &dyn Fn(&FieldValue) -> Enc,
) -> Enc {
    let name = declaration_name(composite.declaration());
    match types
        .composite(composite.declaration().clone())
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
    value_enc_with_decimal_mode(value, typing, placeholder, false)
}

fn value_enc_with_decimal_mode(
    value: &Value,
    typing: Typing<'_>,
    placeholder: &dyn Fn(&ObjectReference) -> Option<Enc>,
    normalize_decimal: bool,
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
                    ("unit", unit_enc(quantity.unit())),
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
                    value_enc_with_decimal_mode(payload, typing, placeholder, normalize_decimal)
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
            let normalize_elements = normalize_decimal
                || matches!(
                    collection.collection_type().kind(),
                    CollectionKind::Set | CollectionKind::Bag
                );
            let mut elements: Vec<Enc> = collection
                .elements()
                .iter()
                .map(|element| {
                    value_enc_with_decimal_mode(element, typing, placeholder, normalize_elements)
                })
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
        Value::Decimal(decimal) => {
            let representation = if normalize_decimal {
                decimal.normalized()
            } else {
                decimal.representation()
            };
            Enc::tagged(
                "decimal",
                vec![
                    (
                        "coefficient",
                        Enc::text(representation.coefficient().to_string()),
                    ),
                    ("scale", Enc::text(representation.scale().to_string())),
                ],
            )
        }
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
                    .find(|case| case.variant() == *member.variant())
                    .map(|case| (declared, case))
            })
            .map_or(
                Enc::Unencodable("an undeclared enum value"),
                |(declared, case)| {
                    Enc::tagged(
                        "enum",
                        vec![
                            ("name", declaration_name(&declared.node())),
                            ("member", Enc::text(case.name())),
                        ],
                    )
                },
            ),
        Value::Population(_) => Enc::Unencodable("a population value"),
        Value::Composite(composite) => match typing.types {
            Some(types) => composite_enc(composite, types, &|slot| match slot {
                FieldValue::Present(inner) => {
                    value_enc_with_decimal_mode(inner, typing, placeholder, normalize_decimal)
                }
                FieldValue::Absent | FieldValue::Null => Enc::Null,
            }),
            None => Enc::Unencodable("a record or tuple value"),
        },
        Value::Union(union) => Enc::tagged(
            "union",
            vec![
                ("name", declaration_name(union.declaration())),
                ("member", Enc::text(union.member().identifier())),
                (
                    "components",
                    Enc::Array(
                        union
                            .payload()
                            .iter()
                            .map(|inner| {
                                value_enc_with_decimal_mode(
                                    inner,
                                    typing,
                                    placeholder,
                                    normalize_decimal,
                                )
                            })
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
/// fields in declaration order. A state that lists relationship extents adds
/// one more entry, keyed by the text `relationships` (no population key is
/// text of that form), mapping each listed relationship's identity text to
/// its completeness and its instance pairs in ascending canonical order, so
/// two states differing only in their extents have different keys.
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
    if !state.extents.is_empty() {
        populations.push((
            Enc::tagged("text", vec![("value", Enc::text("relationships"))]),
            extents_enc(state),
        ));
    }
    map_enc(populations)
}

/// The listed relationship extents of `state`: relationship identity text to
/// `{complete, instances}`, the instances as `{source, target}` pairs
/// ascending by canonical bytes, repeats kept (admission keeps a repeated
/// pair when neither end is unique).
fn extents_enc(state: &ModelState) -> Enc {
    let limits = Limits::new(u64::MAX);
    let entries = state
        .extents
        .iter()
        .map(|extent| {
            let mut pairs: Vec<(Vec<u8>, Enc)> = extent
                .instances
                .iter()
                .map(|(source, target)| {
                    let pair = Enc::Object(vec![
                        ("source", reference_enc(source)),
                        ("target", reference_enc(target)),
                    ]);
                    (
                        quire_canonical::to_vec(&pair, limits).unwrap_or_default(),
                        pair,
                    )
                })
                .collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            (
                Enc::tagged(
                    "text",
                    vec![("value", declaration_enc_text(&extent.relationship))],
                ),
                Enc::Object(vec![
                    ("complete", Enc::Bool(extent.complete)),
                    (
                        "instances",
                        Enc::Array(pairs.into_iter().map(|(_, pair)| pair).collect()),
                    ),
                ]),
            )
        })
        .collect();
    map_enc(entries)
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
    let mut fields: Vec<Enc> = fields
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
    // A collection of bare clocks holds one `Clock` tuple per element, the
    // shell FR-230 gives a present clock place, so two states that differ
    // in its element count have different keys.
    for (type_key, field, _, _) in typing.clock_counts {
        if *type_key != object.type_identity {
            continue;
        }
        let elements = match object.fields.get(&field.key) {
            Some(Value::Integer(count)) => (0..count.to_u64().unwrap_or(0))
                .map(|_| clock_shell_enc())
                .collect(),
            // FR-230-AC-42: a `Sequence<Option<Clock>>` writes each element's
            // option shell, `null` when absent and a `Clock` shell when
            // present.
            Some(Value::Collection(bits)) => bits
                .elements()
                .iter()
                .map(|bit| {
                    let shell = if matches!(bit, Value::Boolean(true)) {
                        clock_shell_enc()
                    } else {
                        Enc::Null
                    };
                    Enc::tagged("option", vec![("value", shell)])
                })
                .collect(),
            _ => continue,
        };
        fields.push(Enc::Object(vec![
            ("name", Enc::text(field.name.clone())),
            (
                "value",
                Enc::tagged("sequence", vec![("elements", Enc::Array(elements))]),
            ),
        ]));
    }
    // FR-230 "Presence bit": an `Option<Clock>` field writes its option
    // shell, `null` when absent and a `Clock` shell when present.
    for (type_key, field) in typing.clock_bits {
        if *type_key != object.type_identity {
            continue;
        }
        let present = matches!(object.fields.get(&field.key), Some(Value::Boolean(true)));
        let shell = if present {
            clock_shell_enc()
        } else {
            Enc::Null
        };
        fields.push(Enc::Object(vec![
            ("name", Enc::text(field.name.clone())),
            ("value", Enc::tagged("option", vec![("value", shell)])),
        ]));
    }
    Enc::tagged(
        "record",
        vec![
            ("name", declaration_enc_text(&object.type_identity)),
            ("fields", Enc::Array(fields)),
        ],
    )
}

/// A present clock place's shell, FR-230's
/// `{"type":"tuple","name":"Clock","components":[]}`.
pub(crate) fn clock_shell_enc() -> Enc {
    Enc::tagged(
        "tuple",
        vec![
            ("name", Enc::text("Clock")),
            ("components", Enc::Array(Vec::new())),
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
        from_admitted, CollectionKind, CollectionType, Integer, Quantity, Rational, Timestamp,
        UnitId, Uuid, ValueType,
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
        let typing = Typing::none();

        assert_eq!(
            value_enc(&Value::Uuid(uuid), typing)
                .canonical_text(Limits::new(1024))
                .expect("a UUID encodes"),
            r#"{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}"#
        );
        assert_eq!(
            value_enc(&Value::Timestamp(timestamp), typing)
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
        let typing = Typing::none();

        assert_eq!(
            value_enc(&ids, typing)
                .canonical_text(Limits::new(1024))
                .expect("the UUID set encodes"),
            r#"{"elements":[{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeefe"},{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}],"type":"set"}"#
        );
        assert_eq!(
            value_enc(&times, typing)
                .canonical_text(Limits::new(1024))
                .expect("the Timestamp set encodes"),
            r#"{"elements":[{"nanoseconds":"10","type":"timestamp"},{"nanoseconds":"2","type":"timestamp"}],"type":"set"}"#
        );
    }

    /// FR-120: a unit identity is written as its UnitId text.
    #[test]
    fn a_unit_encodes_as_its_identity_text() {
        let unit = UnitId::compound(vec![
            (crate::test_ids::node_key([1]), Integer::from(1_i64)),
            (crate::test_ids::node_key([2]), Integer::from(-2_i64)),
        ]);
        let text = Enc::Object(vec![("unit", unit_enc(&unit))])
            .canonical_text(Limits::new(1024))
            .expect("a unit encodes");
        let identity_text = unit_identity_enc(&unit)
            .canonical_text(Limits::new(1024))
            .expect("the UnitId identity has canonical text");
        assert_eq!(
            text,
            format!(
                r#"{{"unit":{}}}"#,
                serde_json::to_string(&identity_text).expect("unit text serializes")
            )
        );
    }

    fn declaration_json(seed: u8) -> String {
        serde_json::to_string(&format!(r#"{{"node":"[{seed}]","package":"fixture"}}"#))
            .expect("DeclarationKey identity text is a JSON string")
    }

    #[test]
    fn a_type_node_name_is_the_declaration_key_identity_text() {
        assert_eq!(
            declaration_name(&crate::test_ids::node_key([0x11]))
                .canonical_text(Limits::new(1024))
                .expect("a declaration name encodes"),
            declaration_json(0x11)
        );
    }

    /// An effective identity owned through `depth` declarations.
    fn owned_chain(depth: usize) -> EffectiveId {
        let mut id = None;
        for level in 0..=depth {
            id = Some(EffectiveId::new(
                id,
                OriginalDeclarationKey::new("fixture", format!("Level{level}")),
            ));
        }
        id.expect("a chain has a root")
    }

    /// FR-201: a deep owner chain writes and reads back one loop iteration per
    /// level, and the reference it travels in round-trips.
    #[test]
    fn a_deep_owner_chain_round_trips_through_a_reference() {
        let reference = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            owned_chain(5000),
            quire_exact::ObjectId::new("object").expect("an object identity"),
        );
        let text = reference_enc(&reference)
            .canonical_text(Limits::new(u64::MAX))
            .expect("the reference encodes");
        assert_eq!(parse_reference(&text), Some(reference));
    }

    /// FR-201: a reference text whose effective identity is malformed has no
    /// reading.
    #[test]
    fn a_malformed_effective_identity_is_refused() {
        let reference = ObjectReference::new(
            crate::test_ids::universe_id([1]),
            owned_chain(2),
            quire_exact::ObjectId::new("object").expect("an object identity"),
        );
        let text = reference_enc(&reference)
            .canonical_text(Limits::new(1 << 20))
            .expect("the reference encodes");
        // One closing brace short, an extra member, and a foreign shape.
        let short = text.replacen("null}}}", "null}}", 1);
        assert!(parse_reference(&short).is_none());
        let extra = text.replacen(r#"{"node":"#, r#"{"extra":1,"node":"#, 1);
        assert!(parse_reference(&extra).is_none());
        assert!(parse_reference(r#"{"identity":"o","object_type":7}"#).is_none());
    }

    /// The declarations of FR-120-AC-16's `Tag { sku: Integer, note?:
    /// Boolean }` (node `11…`), `Pair(Boolean, Integer)` (`22…`) and `Shape`
    /// with the nullary member `none` and the member `tagged(Tag)` (`33…`),
    /// and the values `Tag { sku }`, `Pair(true, -3)` and `none`.
    fn declared() -> (TypeEnvironment, impl Fn(i64) -> Value, Value, Value) {
        use quire_exact::{from_admitted_slots, Presence, UnionMember, UnionValue};
        use quire_semantic_value::declaration::{
            CompositeDeclaration, FieldDeclaration, UnionMemberDeclaration,
        };
        let (tag, pair, shape) = (
            crate::test_ids::node_key([0x11]),
            crate::test_ids::node_key([0x22]),
            crate::test_ids::node_key([0x33]),
        );
        let none = UnionMember::from_admitted(shape.clone(), "none").expect("a member name");
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    tag.clone(),
                    "Tag",
                    CompositeShape::Record(vec![
                        FieldDeclaration::new("sku", ValueType::Integer, Presence::Required),
                        FieldDeclaration::new("note", ValueType::Boolean, Presence::Optional),
                    ]),
                ),
                CompositeDeclaration::new(
                    pair.clone(),
                    "Pair",
                    CompositeShape::Tuple(vec![ValueType::Boolean, ValueType::Integer]),
                ),
                CompositeDeclaration::new(
                    shape.clone(),
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
                tag.clone(),
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

    /// FR-120-AC-16 (TC-1195): declaration names use JCS DeclarationKey text.
    #[trace("TC-1195", "FR-120-AC-16")]
    #[test]
    fn a_record_tuple_and_union_encode_in_their_declared_forms() {
        let (types, tag, pair, none) = declared();
        let text = |value: &Value| {
            value_enc(value, Typing::with_variants(&types, &[]))
                .canonical_text(Limits::new(4096))
                .expect("the value encodes")
        };
        let (tag_name, pair_name, shape_name) = (
            declaration_json(0x11),
            declaration_json(0x22),
            declaration_json(0x33),
        );
        assert_eq!(
            text(&tag(7)),
            format!(
                r#"{{"fields":[{{"name":"sku","value":{{"type":"integer","value":"7"}}}},{{"name":"note","value":null}}],"name":{tag_name},"type":"record"}}"#
            )
        );
        assert_eq!(
            text(&pair),
            format!(
                r#"{{"components":[{{"type":"boolean","value":true}},{{"type":"integer","value":"-3"}}],"name":{pair_name},"type":"tuple"}}"#
            )
        );
        assert_eq!(
            text(&none),
            format!(r#"{{"components":[],"member":"none","name":{shape_name},"type":"union"}}"#)
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
                r#"{{"fields":[{{"name":"sku","value":{{"type":"integer","value":"{sku}"}}}},{{"name":"note","value":null}}],"name":{},"type":"record"}}"#,
                declaration_json(0x11)
            )
        };
        let tag_type = ValueType::Composite(crate::test_ids::node_key([0x11]));
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
                crate::test_ids::variant_id([0xfd]),
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
        let unit = UnitId::declared(crate::test_ids::node_key([0xab]));
        let quantity = Value::Quantity(Quantity::new(
            Rational::new(Integer::from(3_i64), Integer::from(2_i64)).expect("nonzero"),
            unit.clone(),
        ));
        let text = value_enc(&quantity, Typing::none())
            .canonical_text(Limits::new(1024))
            .expect("a quantity encodes");
        assert_eq!(
            text,
            format!(
                r#"{{"denominator":"2","numerator":"3","type":"rational","unit":{}}}"#,
                serde_json::to_string(
                    &unit_identity_enc(&unit)
                        .canonical_text(Limits::new(1024))
                        .expect("unit identity text")
                )
                .expect("unit text serializes")
            )
        );
    }

    /// FR-120-AC-16/17 (TC-1195): a retained decimal keeps its representation,
    /// while set and bag elements use the normalized representation.
    #[trace("TC-1195", "FR-120-AC-16", "FR-120-AC-17")]
    #[test]
    fn decimal_values_keep_or_normalize_representation_by_context() {
        use quire_exact::{Decimal, Integer};

        let decimal = Value::Decimal(Decimal::new(Integer::from(150_i64), 2));
        let typing = Typing::none();
        let retained = value_enc(&decimal, typing)
            .canonical_text(Limits::new(1024))
            .expect("a decimal encodes");
        assert_eq!(
            retained,
            r#"{"coefficient":"150","scale":"2","type":"decimal"}"#
        );

        let decimal_type = ValueType::Decimal(
            quire_exact::DecimalType::new(
                Integer::from(0_i64),
                Integer::from(1000_i64),
                0,
                2,
                quire_exact::RoundingMode::Exact,
            )
            .expect("valid decimal type"),
        );
        for kind in [CollectionKind::Set, CollectionKind::Bag] {
            let collection = from_admitted(
                CollectionType::new(kind, decimal_type.clone(), None),
                vec![decimal.clone()],
            );
            let text = value_enc(&collection, typing)
                .canonical_text(Limits::new(1024))
                .expect("a decimal collection encodes");
            let type_name = match kind {
                CollectionKind::Set => "set",
                CollectionKind::Bag => "bag",
                _ => unreachable!(),
            };
            assert_eq!(
                text,
                format!(
                    r#"{{"elements":[{{"coefficient":"15","scale":"1","type":"decimal"}}],"type":"{type_name}"}}"#
                )
            );
        }

        let option_type = ValueType::option(decimal_type.clone());
        let option = quire_exact::OptionValue::from_admitted(decimal_type, Some(decimal));
        let nested = from_admitted(
            CollectionType::new(CollectionKind::Set, option_type, None),
            vec![option],
        );
        assert_eq!(
            value_enc(&nested, typing)
                .canonical_text(Limits::new(1024))
                .expect("a nested decimal set encodes"),
            r#"{"elements":[{"type":"option","value":{"coefficient":"15","scale":"1","type":"decimal"}}],"type":"set"}"#
        );
    }

    /// A value with no form refuses to write rather than writing a guess.
    #[trace("TC-471", "FR-120-AC-4")]
    #[test]
    fn a_value_without_a_form_refuses_to_encode() {
        let text = Enc::Unencodable("a union value").canonical_text(Limits::new(1024));
        assert!(matches!(text, Err(Error::Serialize(_))));
    }

    /// FR-120-AC-18 (TC-1195): a Population handle has no typed canonical form.
    #[trace("TC-1195", "FR-120-AC-18")]
    #[test]
    fn a_population_value_keeps_its_typed_refusal() {
        let value = Value::Population(quire_exact::PopulationId::new(crate::test_ids::node_key([
            1,
        ])));
        assert!(matches!(
            value_enc(&value, Typing::none()).canonical_text(Limits::new(1024)),
            Err(Error::Serialize(_))
        ));
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
