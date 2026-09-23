// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-13/QC-15 exact kernel `Value`/`ValueType` and composite construction.
//!
//! This is a **cut**, not a verbatim port, of QSL's `value::composite`. Per
//! ADR-013 T-6, every kernel-crossing payload is trimmed to the bare id(s)
//! it needs, and per O-15, `TypeEnvironment`/`ObjectTypeDeclaration` (the
//! declaration *registry* -- duplicate-key checks, the recursion rule,
//! generalization, `Reference<T>` target lookup) stay layer 3 in QSL, not
//! the kernel. Concretely, against `value::composite`:
//!
//! - `ValueType::Enum(NodeKey)` becomes `ValueType::Enum(EnumShape)` (ADR-013
//!   O-14, T-6): the declaration-keyed `NodeKey` lookup is dropped, but the
//!   shape carries its admitted variants inline, as a canonically ranked list
//!   of opaque `VariantId` digests, so `ValueType::admits` needs no
//!   declaration lookup at all -- membership and rank agreement are checked
//!   against the shape the type itself carries. `Value::Enum(EnumMember)`
//!   (T-6: "no `NodeKey`"; OQ-D: "an enum value is a `VariantId` and its
//!   rank") carries one variant's identity plus its zero-based canonical
//!   rank, so ordering (`crate::key::compare_keys`) needs no declaration
//!   lookup either.
//! - `ValueType::Reference(EffectiveId)` is the same type in both (ADR-013
//!   O-05). Only the value payload diverges: `Value::Reference(ObjectReference)`
//!   carries the T-6 triple (`EffectiveId`, `UniverseId`, `ObjectId`) from
//!   [`crate::reference`], where QSL's carries its `EffectiveId` with raw
//!   `UniverseIdentity`/`ObjectIdentity` bytes.
//! - `ValueType::Quantity(QuantityUnit)` becomes `ValueType::Quantity(UnitId)`;
//!   `Value::Quantity` carries [`crate::quantity::Quantity`], a bare
//!   `(magnitude, UnitId)` pair with no unit-graph declaration.
//! - `Value::Population(Arc<PopulationBinding>)` becomes
//!   `Value::Population(PopulationId)` (ADR-013 O-13 Population row, QC-21,
//!   FR-089): the kernel carries the opaque identity alone, never the
//!   `PopulationBinding` a QSL `model` type owns. `ValueType::Population(u64)`
//!   is unchanged (T-6: "keeps `u64` count only"). FR-089-AC-5's
//!   declared-maximum comparison is a QSL-layer check: the model/evaluator
//!   resolves a `PopulationId` to its binding and compares the binding's own
//!   declared maximum there, since this leaf crate has no way to resolve a
//!   `PopulationId` to anything. Kernel `ValueType::admits` refuses every
//!   population pair outright: it never pairs `ValueType::Population` with
//!   `Value::Population`, so every `(ValueType::Population(_),
//!   Value::Population(_))` pair falls through to `admits`'s existing
//!   catch-all and returns `false`.
//! - `TypeEnvironment::record`/`tuple`/`evaluate_record`/`evaluate_tuple`
//!   become free functions taking the declared shape directly
//!   (`&[FieldDeclaration]` or `&[ValueType]`) instead of looking it up by
//!   `NodeKey` in a registry. `CompositeDeclaration` and the declaration-time
//!   `duplicate_name` self-consistency check are dropped with it: the kernel
//!   trusts the shape its caller (QSL's own `TypeEnvironment`) hands it.
//! - [`from_admitted_slots`] is a new trusted, unchecked composite
//!   constructor, `pub` (not `pub(crate)`, since QSL is a separate crate now)
//!   for QSL to call once it has independently checked a value against its
//!   own registry -- mirroring [`OptionValue::from_admitted`]'s identical
//!   role, which is likewise widened from `pub(crate)` to `pub` here.

use std::sync::Arc;

use crate::accounting::{Charge, ChargePoint, LimitKind, Meter};
use crate::collection::{CollectionType, CollectionValue};
use crate::decimal::{Decimal, DecimalType};
use crate::identity::{EffectiveId, MemberId, PopulationId, UnitId, VariantId};
use crate::ieee::{IeeeValue, IeeeWidth};
use crate::integer::{Integer, IntegerInterval};
use crate::node::NodeKey;
use crate::outcome::{Outcome, Refusal, Stop};
use crate::quantity::Quantity;
use crate::rational::{Rational, RationalDomain};
use crate::reference::ObjectReference;
use crate::text::{Text, TextType};

/// The inline, *ranked* set of an enum type's admitted variants (ADR-013
/// O-14, OQ-D ruling): the kernel `ValueType::Enum` carries its variant set
/// directly, as opaque [`VariantId`] digests, so `admits` is a pure
/// set-membership test needing no declaration lookup. Unlike the type's
/// earlier, unranked shape (`quire-exact/src/value.rs:73`, before this
/// change), the variants are held as a canonically ordered list, not a
/// digest-ordered set: FR-144's enumeration key row (FR-144-AC-9) fixes
/// canonical order as declaration position for an `ordered enum` and
/// case-identifier byte order for an unordered one, never the `VariantId`
/// digest. `EnumShape` cannot compute that order itself -- a kernel leaf
/// holds no case-name strings -- so the caller (QSL `check`, `semantic_value`)
/// supplies `variants` already in that canonical order; each variant's
/// zero-based index in the list is its *rank* ([`Self::rank`]), which
/// [`EnumMember`] carries next to its `VariantId` (OQ-D: "An enum value
/// carries its `VariantId` and its rank"). Two shapes are the same shape
/// exactly when they admit the same variants in the same canonical order and
/// agree on [`Self::is_ordered`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumShape {
    ordered: bool,
    variants: Vec<VariantId>,
}

impl EnumShape {
    /// The enum shape admitting exactly `variants`, supplied already in FR-141
    /// canonical order (declaration order when `ordered`, case-identifier byte
    /// order otherwise): the caller's responsibility, since this leaf type has
    /// no case-name strings to sort by itself.
    pub fn new(ordered: bool, variants: impl IntoIterator<Item = VariantId>) -> Self {
        Self {
            ordered,
            variants: variants.into_iter().collect(),
        }
    }

    /// Whether the declaration this shape describes selects `ordered enum`
    /// semantics (FR-141-AC-5): only an ordered enum admits an ordering
    /// operator.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// Whether `variant` is one of this shape's admitted variants.
    pub fn contains(&self, variant: VariantId) -> bool {
        self.rank(variant).is_some()
    }

    /// `variant`'s zero-based index in this shape's canonical member list, or
    /// `None` when `variant` is not one of this shape's admitted variants
    /// (ADR-013 O-14/OQ-D: "a rank is a function of its `VariantId`").
    pub fn rank(&self, variant: VariantId) -> Option<u32> {
        self.variants
            .iter()
            .position(|candidate| *candidate == variant)
            .map(|position| {
                u32::try_from(position)
                    .expect("an admitted enum has far fewer than u32::MAX variants")
            })
    }

    /// The admitted variants, in canonical (rank) order.
    pub fn variants(&self) -> impl Iterator<Item = VariantId> + '_ {
        self.variants.iter().copied()
    }
}

/// A completed enum value: its variant identity and its canonical rank next
/// to it (ADR-013 O-14, OQ-D ruling: "An enum value carries its `VariantId`
/// and its rank, the variant's zero-based index in the FR-141 canonical
/// member list"). Identity and equality use `variant` alone
/// (the kernel equality leaf match, "Identity and equality use the
/// `VariantId` only"); `rank` exists purely so the kernel's own canonical key
/// (`compare_keys`, FR-144) can order same-enum values
/// without any declaration lookup. [`ValueType::admits`] refuses a value
/// whose claimed rank disagrees with the shape's own ranked list, so a
/// well-formed `EnumMember` always has `shape.rank(member.variant()) ==
/// Some(member.rank())` for the `EnumShape` it was admitted against.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnumMember {
    variant: VariantId,
    rank: u32,
}

impl EnumMember {
    /// Pair a variant identity with its claimed canonical rank. This
    /// constructor performs no validation of its own -- [`ValueType::admits`]
    /// is where a mismatched pair is caught.
    pub fn new(variant: VariantId, rank: u32) -> Self {
        Self { variant, rank }
    }

    /// The variant identity: the only component identity and equality use.
    pub fn variant(&self) -> VariantId {
        self.variant
    }

    /// This member's zero-based index in its enum's FR-141 canonical member
    /// list.
    pub fn rank(&self) -> u32 {
        self.rank
    }
}

/// A declared kernel value type. Two types are the same type exactly when
/// they are equal, collection bounds included.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueType {
    /// `Boolean`.
    Boolean,
    /// The unbounded mathematical `Integer`.
    Integer,
    /// A bounded `Int[lo, hi]`.
    Int(IntegerInterval),
    /// A `Rational[n1, n2; d1, d2]` domain.
    Rational(RationalDomain),
    /// A `Decimal[lo, hi; smin, smax; mode]`.
    Decimal(DecimalType),
    /// A `Float32` or `Float64`.
    Float(IeeeWidth),
    /// A quantity in exactly this unit.
    Quantity(UnitId),
    /// A `Text[min, max; profile]`.
    Text(TextType),
    /// An `Enum` type admitting exactly this inline variant set (ADR-013
    /// O-14).
    Enum(EnumShape),
    /// `Option<T>`: `none` or a present `T`.
    Option(Box<ValueType>),
    /// The record or tuple declaration with this node key.
    Composite(NodeKey),
    /// A bounded collection type `K<T>[min, max]`.
    Collection(Box<CollectionType>),
    /// `Reference<T>` to an object of the object type with this effective
    /// identity (ADR-013 T-6).
    Reference(EffectiveId),
    /// The `Population<T>[N]` parameter type's declared maximum `N`.
    Population(u64),
}

impl ValueType {
    /// `K<element>[bound]`.
    pub fn collection(collection_type: CollectionType) -> Self {
        Self::Collection(Box::new(collection_type))
    }

    /// `Option<payload>`.
    pub fn option(payload: Self) -> Self {
        Self::Option(Box::new(payload))
    }

    /// Whether `value` is a member of this declared type. Composite, option
    /// and collection values carry their declared type, which must be this
    /// type; their contents were admitted at construction. An `Enum` shape
    /// admits a `Value::Enum` exactly when the shape's own ranked list agrees
    /// with the member's claimed rank for its variant (ADR-013 O-14/OQ-D): a
    /// pure lookup against the shape, no declaration lookup needed.
    pub fn admits(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Boolean, Value::Boolean(_)) | (Self::Integer, Value::Integer(_)) => true,
            (Self::Int(interval), Value::Integer(integer)) => interval.contains(integer),
            (Self::Rational(domain), Value::Rational(rational)) => domain.contains(rational),
            (Self::Decimal(declared), Value::Decimal(decimal)) => declared.contains(decimal),
            (Self::Float(width), Value::Float(float)) => float.width() == *width,
            (Self::Quantity(unit), Value::Quantity(quantity)) => quantity.unit() == *unit,
            (Self::Text(declared), Value::Text(text)) => text.text_type() == declared,
            (Self::Enum(shape), Value::Enum(member)) => {
                shape.rank(member.variant()) == Some(member.rank())
            }
            (Self::Option(payload), Value::Option(option)) => option.payload_type() == &**payload,
            (Self::Composite(declaration), Value::Composite(composite)) => {
                composite.declaration() == *declaration
            }
            (Self::Collection(declared), Value::Collection(collection)) => {
                collection.collection_type() == &**declared
            }
            (Self::Reference(object_type), Value::Reference(reference)) => {
                reference.object_type() == *object_type
            }
            // `ValueType::Population` does not pair with `Value::Population`
            // here (see this module's doc comment): FR-089-AC-5's
            // declared-maximum comparison is a QSL-layer check, performed by
            // the model/evaluator once it resolves the binding. Kernel
            // `admits` refuses population pairs outright; this pair falls
            // through to the catch-all below and returns `false`.
            (
                Self::Boolean
                | Self::Integer
                | Self::Int(_)
                | Self::Rational(_)
                | Self::Decimal(_)
                | Self::Float(_)
                | Self::Quantity(_)
                | Self::Text(_)
                | Self::Enum(_)
                | Self::Option(_)
                | Self::Composite(_)
                | Self::Collection(_)
                | Self::Reference(_)
                | Self::Population(_),
                _,
            ) => false,
        }
    }
}

/// A completed kernel value. It deliberately has no structural `PartialEq`:
/// equality is the relation [`crate::plan_equality`]/[`crate::planned_equality`]
/// plans and evaluates.
#[derive(Clone, Debug)]
pub enum Value {
    /// A Boolean.
    Boolean(bool),
    /// A mathematical integer of `Integer` or `Int[..]`.
    Integer(Integer),
    /// An exact reduced rational.
    Rational(Rational),
    /// An exact decimal.
    Decimal(Decimal),
    /// An IEEE bit pattern.
    Float(IeeeValue),
    /// A quantity in its unit.
    Quantity(Quantity),
    /// A text value of its declared type.
    Text(Text),
    /// A bare enum member identity and its canonical rank (ADR-013 T-6,
    /// OQ-D).
    Enum(EnumMember),
    /// An opaque population admission identity (ADR-013 O-13 Population
    /// row, QC-21, FR-089): never the `PopulationBinding` itself, which
    /// stays a QSL `model` type.
    Population(PopulationId),
    /// An option value.
    Option(Arc<OptionValue>),
    /// A record or tuple value.
    Composite(Arc<CompositeValue>),
    /// A collection value.
    Collection(Arc<CollectionValue>),
    /// A terminal object reference.
    Reference(ObjectReference),
}

impl Value {
    /// `occ(v)` of `quire.value.accounting/v1`: one for the value itself plus
    /// every nested occurrence, with `absent` and `null` slots counting zero
    /// and a bag occurrence counted once per multiplicity.
    pub fn occ(&self) -> Integer {
        match self {
            Self::Option(option) => option.occ.clone(),
            Self::Composite(composite) => composite.occ.clone(),
            Self::Collection(collection) => collection.occ().clone(),
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::Rational(_)
            | Self::Decimal(_)
            | Self::Float(_)
            | Self::Quantity(_)
            | Self::Text(_)
            | Self::Enum(_)
            | Self::Population(_)
            | Self::Reference(_) => Integer::one(),
        }
    }
}

/// `none` or a present value of one declared payload type.
#[derive(Clone, Debug)]
pub struct OptionValue {
    payload_type: ValueType,
    payload: Option<Value>,
    occ: Integer,
}

impl OptionValue {
    /// `none` of `Option<payload_type>`.
    pub fn none(payload_type: ValueType) -> Value {
        Value::Option(Arc::new(Self {
            payload_type,
            payload: None,
            occ: Integer::one(),
        }))
    }

    /// A present `payload` of `Option<payload_type>`.
    pub fn present(payload_type: ValueType, payload: Value) -> Result<Value, ConstructionRefusal> {
        if !payload_type.admits(&payload) {
            return Err(ConstructionRefusal {
                component: Component::Payload,
                cause: ConstructionCause::TypeMismatch,
            });
        }
        let occ = Integer::one().add(&payload.occ());
        Ok(Value::Option(Arc::new(Self {
            payload_type,
            payload: Some(payload),
            occ,
        })))
    }

    /// Materialize an already-admitted `payload` as `Option<payload_type>`,
    /// checking no structural `admits()` match. `pub`, not `pub(crate)`,
    /// since the caller checking admission (e.g. QSL's own upcast-aware
    /// `lookup<T>(p, r)`) is a separate crate from this one.
    pub fn from_admitted(payload_type: ValueType, payload: Option<Value>) -> Value {
        let occ = match &payload {
            Some(payload) => Integer::one().add(&payload.occ()),
            None => Integer::one(),
        };
        Value::Option(Arc::new(Self {
            payload_type,
            payload,
            occ,
        }))
    }

    /// The declared payload type.
    pub fn payload_type(&self) -> &ValueType {
        &self.payload_type
    }

    /// The present payload, or `None` for `none`.
    pub fn payload(&self) -> Option<&Value> {
        self.payload.as_ref()
    }
}

/// Whether a declared field admits only a present value (`f: T`) or also
/// `absent` and explicit `null` (`f: T?`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Presence {
    /// A field declared without `?`.
    Required,
    /// A field declared with `?`.
    Optional,
}

/// The state of one field slot.
#[derive(Clone, Debug)]
pub enum FieldValue {
    /// A present value.
    Present(Value),
    /// `absent`: the `?` field was omitted.
    Absent,
    /// Explicit `null`, distinct from `absent`.
    Null,
}

/// A declaration-owned named field. The kernel carries a member only as an
/// opaque [`MemberId`] digest (ADR-013 O-06): `name` is retained solely as a
/// human-readable label for `Debug`/diagnostics, and no public semantic
/// dispatch here keys on it (#213's own acceptance criterion) -- every
/// lookup, refusal component and slot match below keys on `member`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDeclaration {
    member: MemberId,
    name: String,
    value_type: ValueType,
    presence: Presence,
}

impl FieldDeclaration {
    /// The field `member` (`name: value_type` or `name: value_type?`, where
    /// `name` is a display label only -- see the struct doc comment).
    pub fn new(
        member: MemberId,
        name: impl Into<String>,
        value_type: ValueType,
        presence: Presence,
    ) -> Self {
        Self {
            member,
            name: name.into(),
            value_type,
            presence,
        }
    }

    /// The opaque member identity dispatch keys on.
    pub fn member(&self) -> MemberId {
        self.member
    }

    /// The field's human-readable label. Not used for dispatch.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The declared value type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// Whether the field was declared with `?`.
    pub fn presence(&self) -> Presence {
        self.presence
    }
}

/// Construct a record from its declared `shape` (looked up by the caller's
/// own declaration registry -- QSL's `TypeEnvironment`, not the kernel).
/// An omitted `?` field is `absent`.
pub fn record(
    shape: &[FieldDeclaration],
    declaration: NodeKey,
    fields: Vec<(MemberId, FieldValue)>,
) -> Result<Value, ConstructionRefusal> {
    let slots = fill_slots(shape, fields)?;
    Ok(composite(declaration, slots))
}

/// Construct a tuple of exactly `shape`'s declared arity.
pub fn tuple(
    shape: &[ValueType],
    declaration: NodeKey,
    positions: Vec<Value>,
) -> Result<Value, ConstructionRefusal> {
    if shape.len() != positions.len() {
        return refuse(
            Component::Value,
            ConstructionCause::WrongArity {
                declared: shape.len(),
                supplied: positions.len(),
            },
        );
    }
    if let Some(position) = shape
        .iter()
        .zip(&positions)
        .position(|(value_type, value)| !value_type.admits(value))
    {
        return refuse(
            Component::Position(position),
            ConstructionCause::TypeMismatch,
        );
    }
    let slots = positions.into_iter().map(FieldValue::Present).collect();
    Ok(composite(declaration, slots))
}

/// Evaluate a record value expression against its declared `shape`. Every
/// construction refusal is decided before any field expression runs. Field
/// expressions then run in declaration order, whatever the source order; the
/// first one that does not complete becomes the outcome and no later one
/// runs. A completed record charges `composite.result-retain`.
pub fn evaluate_record(
    shape: &[FieldDeclaration],
    declaration: NodeKey,
    fields: Vec<(MemberId, FieldExpression<'_>)>,
    meter: &mut Meter,
) -> Result<Outcome<Value>, ConstructionRefusal> {
    let mut supplied = match_members(shape, fields)?;
    let mut plan = Vec::with_capacity(shape.len());
    for field in shape {
        let expression = supplied.remove(&field.member);
        let component = || Component::Field(field.member);
        match (&expression, field.presence) {
            (None, Presence::Required) => {
                return refuse(component(), ConstructionCause::MissingField)
            }
            (Some(FieldExpression::Null), Presence::Required) => {
                return refuse(component(), ConstructionCause::NullForRequiredField)
            }
            (None | Some(FieldExpression::Null | FieldExpression::Evaluate(_)), _) => {}
        }
        plan.push((field, expression));
    }
    let mut slots = Vec::with_capacity(plan.len());
    for (field, expression) in plan {
        let slot = match expression {
            None => FieldValue::Absent,
            Some(FieldExpression::Null) => FieldValue::Null,
            Some(FieldExpression::Evaluate(expression)) => {
                match admitted(&field.value_type, expression(meter)) {
                    Ok(value) => FieldValue::Present(value),
                    Err(stop) => return Ok(Outcome::from_stop(Err(stop))),
                }
            }
        };
        slots.push(slot);
    }
    Ok(Outcome::from_stop(retain_composite(
        composite(declaration, slots.into_boxed_slice()),
        meter,
    )))
}

/// Evaluate a tuple call `T(e, ...)` against its declared `shape`: the arity
/// is checked first, then the arguments run in position order under the
/// first-stopped rule, then a completed tuple charges
/// `composite.result-retain`.
pub fn evaluate_tuple(
    shape: &[ValueType],
    declaration: NodeKey,
    positions: Vec<Deferred<'_>>,
    meter: &mut Meter,
) -> Result<Outcome<Value>, ConstructionRefusal> {
    if shape.len() != positions.len() {
        return refuse(
            Component::Value,
            ConstructionCause::WrongArity {
                declared: shape.len(),
                supplied: positions.len(),
            },
        );
    }
    let mut slots = Vec::with_capacity(shape.len());
    for (value_type, expression) in shape.iter().zip(positions) {
        match admitted(value_type, expression(meter)) {
            Ok(value) => slots.push(FieldValue::Present(value)),
            Err(stop) => return Ok(Outcome::from_stop(Err(stop))),
        }
    }
    Ok(Outcome::from_stop(retain_composite(
        composite(declaration, slots.into_boxed_slice()),
        meter,
    )))
}

/// A deferred expression: it runs only when construction reaches it.
pub type Deferred<'a> = Box<dyn FnOnce(&mut Meter) -> Outcome<Value> + 'a>;

/// A record field in a record value expression. Omitting a `?` field
/// constructs `absent`.
pub enum FieldExpression<'a> {
    /// `f: e`.
    Evaluate(Deferred<'a>),
    /// `f: null`.
    Null,
}

impl std::fmt::Debug for FieldExpression<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evaluate(_) => formatter.write_str("Evaluate(..)"),
            Self::Null => formatter.write_str("Null"),
        }
    }
}

/// The completed value of a deferred expression, which a checked program
/// guarantees is a member of `value_type`.
fn admitted(value_type: &ValueType, outcome: Outcome<Value>) -> Result<Value, Stop> {
    let value = outcome.into_stop()?;
    if value_type.admits(&value) {
        Ok(value)
    } else {
        Err(Stop::Refused(Refusal::CheckedInvariant))
    }
}

/// Charge `composite.result-retain` with `occ(result)`, then expose it.
pub(crate) fn retain_composite(value: Value, meter: &mut Meter) -> Result<Value, Stop> {
    let occ = value.occ();
    meter.charge(
        Charge::new(ChargePoint::CompositeResultRetain)
            .exact_size(LimitKind::ValueOccurrences, occ.clone())
            .exact_results(occ),
    )?;
    Ok(value)
}

/// Where a construction refusal originates.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Component {
    /// The composite value as a whole (its declaration or arity).
    Value,
    /// A named record field or object attribute, by its opaque member
    /// identity (ADR-013 O-06). No public semantic dispatch depends on
    /// display text (#213's own acceptance criterion).
    Field(MemberId),
    /// A zero-based tuple position.
    Position(usize),
    /// A zero-based collection occurrence in source order.
    Element(usize),
    /// An option payload.
    Payload,
}

/// Why a construction is refused.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConstructionCause {
    /// A required field is omitted.
    MissingField,
    /// A supplied field is not declared.
    UndeclaredField,
    /// A field is supplied twice.
    DuplicateField,
    /// `null` is supplied for a required field.
    NullForRequiredField,
    /// A tuple call has another argument count than its declared arity.
    WrongArity {
        /// Declared arity.
        declared: usize,
        /// Supplied arguments.
        supplied: usize,
    },
    /// A value that is not a member of the declared type.
    TypeMismatch,
}

/// A typed construction refusal at its originating component.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("construction refused at {component:?}: {cause:?}")]
pub struct ConstructionRefusal {
    /// The originating component.
    pub component: Component,
    /// The typed cause.
    pub cause: ConstructionCause,
}

fn refuse<T>(component: Component, cause: ConstructionCause) -> Result<T, ConstructionRefusal> {
    Err(ConstructionRefusal { component, cause })
}

/// Index supplied entries by declared member, refusing an undeclared or
/// repeated member.
fn match_members<T>(
    declared: &[FieldDeclaration],
    supplied: Vec<(MemberId, T)>,
) -> Result<std::collections::BTreeMap<MemberId, T>, ConstructionRefusal> {
    let mut by_member = std::collections::BTreeMap::new();
    for (member, entry) in supplied {
        let component = || Component::Field(member);
        if !declared.iter().any(|field| field.member == member) {
            return refuse(component(), ConstructionCause::UndeclaredField);
        }
        if by_member.insert(member, entry).is_some() {
            return refuse(component(), ConstructionCause::DuplicateField);
        }
    }
    Ok(by_member)
}

/// Declaration-ordered slots of a record from supplied fields.
pub fn fill_slots(
    declared: &[FieldDeclaration],
    supplied: Vec<(MemberId, FieldValue)>,
) -> Result<Box<[FieldValue]>, ConstructionRefusal> {
    let mut by_member = match_members(declared, supplied)?;
    let mut slots = Vec::with_capacity(declared.len());
    for field in declared {
        let component = || Component::Field(field.member);
        let slot = by_member
            .remove(&field.member)
            .unwrap_or(FieldValue::Absent);
        match (&slot, field.presence) {
            (FieldValue::Absent, Presence::Required) => {
                return refuse(component(), ConstructionCause::MissingField)
            }
            (FieldValue::Null, Presence::Required) => {
                return refuse(component(), ConstructionCause::NullForRequiredField)
            }
            (FieldValue::Present(value), _) if !field.value_type.admits(value) => {
                return refuse(component(), ConstructionCause::TypeMismatch)
            }
            (FieldValue::Present(_) | FieldValue::Absent | FieldValue::Null, _) => {}
        }
        slots.push(slot);
    }
    Ok(slots.into_boxed_slice())
}

/// `1 + occ` of every present slot.
pub(crate) fn slots_occ(slots: &[FieldValue]) -> Integer {
    slots.iter().fold(Integer::one(), |occ, slot| match slot {
        FieldValue::Present(value) => occ.add(&value.occ()),
        FieldValue::Absent | FieldValue::Null => occ,
    })
}

fn composite(declaration: NodeKey, slots: Box<[FieldValue]>) -> Value {
    let occ = slots_occ(&slots);
    Value::Composite(Arc::new(CompositeValue {
        declaration,
        slots,
        occ,
    }))
}

/// Materialize already-admitted slots as a composite value, checking
/// nothing: the caller (QSL's own `TypeEnvironment`, which is not kernel per
/// ADR-013 O-15) has already checked every slot against its own declared
/// shape. Mirrors [`OptionValue::from_admitted`]'s identical bypass role.
pub fn from_admitted_slots(declaration: NodeKey, slots: Box<[FieldValue]>) -> Value {
    composite(declaration, slots)
}

/// A record or tuple value.
#[derive(Clone, Debug)]
pub struct CompositeValue {
    declaration: NodeKey,
    slots: Box<[FieldValue]>,
    occ: Integer,
}

impl CompositeValue {
    /// The declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// Field or position states in declaration order.
    pub fn slots(&self) -> &[FieldValue] {
        &self.slots
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::accounting::ScalarLimits;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    fn generous_meter() -> Meter {
        Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        })
    }

    /// TC-307: `ValueType::Boolean` admits only `Value::Boolean`, refusing
    /// a value of another kind.
    #[trace("TC-307")]
    #[test]
    fn tc_307_admits_checks_the_matching_variant_only() {
        assert!(ValueType::Boolean.admits(&Value::Boolean(true)));
        assert!(!ValueType::Boolean.admits(&Value::Integer(Integer::one())));
    }

    /// TC-297 (FR-089-AC-6): kernel `admits` refuses every population pair;
    /// the declared-maximum comparison is the QSL layer's (FR-089-AC-5).
    #[trace("TC-297", "FR-089-AC-6")]
    #[test]
    fn admits_refuses_a_population_pair() {
        assert!(!ValueType::Population(5)
            .admits(&Value::Population(PopulationId::from_digest(digest(1)))));
    }

    /// TC-308: an `Enum` shape admits a `Value::Enum` of a variant it
    /// contains at the variant's own rank, and refuses one it does not
    /// contain (ADR-013 O-14/OQ-D, no declaration lookup).
    ///
    /// Also TC-409 (FR-088-AC-11): admission checks the whole `(VariantId,
    /// rank)` pair, not membership alone.
    #[trace("TC-308")]
    #[trace("TC-409")]
    #[test]
    fn tc_308_enum_shape_admits_only_its_own_variants() {
        let in_shape = VariantId::from_digest(digest(1));
        let out_of_shape = VariantId::from_digest(digest(2));
        let shape = ValueType::Enum(EnumShape::new(false, [in_shape]));
        assert!(shape.admits(&Value::Enum(EnumMember::new(in_shape, 0))));
        assert!(!shape.admits(&Value::Enum(EnumMember::new(out_of_shape, 0))));
        assert!(!ValueType::Boolean.admits(&Value::Enum(EnumMember::new(in_shape, 0))));
    }

    /// TC-308 (OQ-D adverse): a well-formed variant paired with the *wrong*
    /// rank is refused just as surely as an unknown variant -- admission
    /// checks the whole `(VariantId, rank)` pair against the shape's own
    /// ranked list, not membership alone.
    ///
    /// Also TC-409 step 5 (FR-088-AC-11): a value that pairs a known
    /// `VariantId` with the wrong rank is refused at admission.
    #[trace("TC-308")]
    #[trace("TC-409")]
    #[test]
    fn admits_refuses_a_known_variant_at_the_wrong_rank() {
        let first = VariantId::from_digest(digest(1));
        let second = VariantId::from_digest(digest(2));
        let shape = ValueType::Enum(EnumShape::new(true, [first, second]));
        assert!(shape.admits(&Value::Enum(EnumMember::new(first, 0))));
        assert!(shape.admits(&Value::Enum(EnumMember::new(second, 1))));
        assert!(!shape.admits(&Value::Enum(EnumMember::new(first, 1))));
        assert!(!shape.admits(&Value::Enum(EnumMember::new(second, 0))));
    }

    /// OQ-D: the shape's own rank lookup is exactly the variant's index in
    /// the canonical list the caller supplied, and an unranked (unknown)
    /// variant resolves to `None`, never a panic or a fabricated rank.
    ///
    /// Also TC-409 step 3 (FR-088-AC-11): rank is the canonical-list
    /// position.
    #[trace("TC-308")]
    #[trace("TC-409")]
    #[test]
    fn enum_shape_rank_matches_canonical_position() {
        let a = VariantId::from_digest(digest(1));
        let b = VariantId::from_digest(digest(2));
        let c = VariantId::from_digest(digest(3));
        let shape = EnumShape::new(true, [a, b]);
        assert_eq!(shape.rank(a), Some(0));
        assert_eq!(shape.rank(b), Some(1));
        assert_eq!(shape.rank(c), None);
        assert!(shape.is_ordered());
        assert_eq!(shape.variants().collect::<Vec<_>>(), [a, b]);
    }

    /// TC-309: building a record with a missing required field is refused
    /// at that field, before any other field is inspected.
    #[trace("TC-309")]
    #[test]
    fn tc_309_record_refuses_a_missing_required_field() {
        let member = MemberId::from_digest(digest(2));
        let shape = vec![FieldDeclaration::new(
            member,
            "count",
            ValueType::Integer,
            Presence::Required,
        )];
        let err = record(&shape, NodeKey::from_digest(digest(1)), vec![]).unwrap_err();
        assert_eq!(err.component, Component::Field(member));
        assert_eq!(err.cause, ConstructionCause::MissingField);
    }

    /// TC-310: a tuple built with the declared position count and types
    /// completes, and `occ` counts the tuple itself plus its one integer
    /// position.
    #[trace("TC-310")]
    #[test]
    fn tc_310_tuple_of_declared_arity_and_types_completes() {
        let shape = vec![ValueType::Integer];
        let value = tuple(
            &shape,
            NodeKey::from_digest(digest(1)),
            vec![Value::Integer(Integer::one())],
        )
        .unwrap();
        assert_eq!(value.occ(), Integer::one().add(&Integer::one()));
    }

    /// TC-311: a tuple call with the wrong argument count is refused with
    /// the declared and supplied counts named.
    #[trace("TC-311")]
    #[test]
    fn tc_311_tuple_wrong_arity_names_both_counts() {
        let shape = vec![ValueType::Integer, ValueType::Boolean];
        let err = tuple(
            &shape,
            NodeKey::from_digest(digest(1)),
            vec![Value::Integer(Integer::one())],
        )
        .unwrap_err();
        assert_eq!(
            err.cause,
            ConstructionCause::WrongArity {
                declared: 2,
                supplied: 1,
            }
        );
    }

    /// TC-354 (H-7/H-8): `fill_slots` fills a present field and, for an
    /// omitted optional field, `Absent`, both in declaration order
    /// regardless of supplied order (`fill_slots` was previously only
    /// exercised indirectly, through `record`'s missing-required-field
    /// refusal path).
    #[trace("TC-354")]
    #[test]
    fn tc_354_fill_slots_fills_present_and_absent_in_declaration_order() {
        let count = MemberId::from_digest(digest(1));
        let label = MemberId::from_digest(digest(2));
        let shape = vec![
            FieldDeclaration::new(count, "count", ValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                label,
                "label",
                ValueType::Text(
                    TextType::new(0, 10, crate::text::TextProfile::UnicodeScalars).unwrap(),
                ),
                Presence::Optional,
            ),
        ];
        let slots = fill_slots(
            &shape,
            vec![(count, FieldValue::Present(Value::Integer(Integer::one())))],
        )
        .unwrap();
        assert!(matches!(slots[0], FieldValue::Present(Value::Integer(_))));
        assert!(matches!(slots[1], FieldValue::Absent));
    }

    /// TC-355 (H-7/H-8): `evaluate_record` runs a deferred field expression
    /// and completes with `composite.result-retain` charged (`evaluate_record`
    /// had no test before this; `record`/`tuple`'s tests exercise only the
    /// non-deferred constructors).
    #[trace("TC-355")]
    #[test]
    fn tc_355_evaluate_record_completes_from_a_deferred_field() {
        let count = MemberId::from_digest(digest(1));
        let shape = vec![FieldDeclaration::new(
            count,
            "count",
            ValueType::Integer,
            Presence::Required,
        )];
        let mut meter = generous_meter();
        let fields: Vec<(MemberId, FieldExpression<'_>)> = vec![(
            count,
            FieldExpression::Evaluate(Box::new(|_meter| {
                Outcome::Completed(Value::Integer(Integer::one()))
            })),
        )];
        let outcome = evaluate_record(&shape, NodeKey::from_digest(digest(2)), fields, &mut meter)
            .expect("declared fields match");
        let value = outcome.completed().expect("charges available");
        assert_eq!(value.occ(), Integer::one().add(&Integer::one()));
    }

    /// TC-356 (H-7/H-8): `evaluate_tuple` runs deferred positional
    /// expressions in position order and completes (`evaluate_tuple` had no
    /// test before this).
    #[trace("TC-356")]
    #[test]
    fn tc_356_evaluate_tuple_completes_from_deferred_positions() {
        let shape = vec![ValueType::Integer];
        let mut meter = generous_meter();
        let positions: Vec<Deferred<'_>> = vec![Box::new(|_meter| {
            Outcome::Completed(Value::Integer(Integer::one()))
        })];
        let outcome = evaluate_tuple(
            &shape,
            NodeKey::from_digest(digest(1)),
            positions,
            &mut meter,
        )
        .expect("declared arity matches");
        let value = outcome.completed().expect("charges available");
        assert_eq!(value.occ(), Integer::one().add(&Integer::one()));
    }
}
