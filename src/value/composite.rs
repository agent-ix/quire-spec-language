// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 records, tuples and finite recursive values over producer-assigned
//! declaration keys.
//!
//! `ValueType`, `Value`, `OptionValue`, `FieldValue`, `FieldDeclaration` and
//! `CompositeValue`, and the construction functions over them (`fill_slots`,
//! `retain_composite`, the crate-internal `composite`/`match_names`/`refuse`), are
//! the K-designated shadow of `quire_exact::value`: this crate's own cut,
//! parameterized over this module's own `ValueType`/`Value`, not
//! `quire_exact`'s. Per `quire_exact::value`'s own module doc, `ValueType::
//! Enum` carries an inline `EnumShape` where this module's carries a
//! `NodeKey` lookup: a redesigned cut, not a duplicate. `ValueType::Reference`
//! carries the kernel `EffectiveId` in both (ADR-013 O-05: FR-143 makes a
//! reference's type component an effective-declaration identity, never a
//! checked node id). `Presence` has no divergence -- it names only
//! `Required`/`Optional` and touches neither `Value` nor `ValueType` -- so it
//! is `quire_exact`'s type here, and the quantity payloads are already the
//! kernel's: `ValueType::Quantity` carries a `UnitId` and `Value::Quantity` a
//! `quire_exact::Quantity`, with the unit graph behind `value::quantity`'s
//! unit table (ADR-013 T-6). Remaining work, Linear QSL-131.
//!
//! The registry that admits a closed set of these declarations
//! (`TypeEnvironment`, `ObjectTypeDeclaration` and friends) and the FR-149
//! check-level equality layer are not kernel types (ADR-013 O-15; ADR-011
//! §6.1); `value::declaration` owns them. `composite`, `match_names`,
//! `fill_slots`, `retain_composite` and `refuse` are `pub(crate)` because
//! `declaration`'s `TypeEnvironment` construction methods call them:
//! `composite` is the only place that can build a [`CompositeValue`]'s
//! private fields.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::collection::{CollectionType, CollectionValue};
use super::decimal::DecimalType;
use super::enumeration::EnumValue;
use super::outcome::{Outcome, Stop};
use super::text::Text;
use quire_exact::Decimal;
use quire_exact::EffectiveId;
use quire_exact::IeeeWidth;
use quire_exact::IllTyped;
use quire_exact::NodeKey;
use quire_exact::ObjectReference;
use quire_exact::Rational;
use quire_exact::TextType;
use quire_exact::{
    Charge, ChargePoint, IeeeValue, Integer, IntegerInterval, LimitKind, Meter, PopulationId,
    Presence, Quantity, RationalDomain, UnitId,
};

/// A declared complete-V1 value type. Two types are the same type exactly when
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
    /// An FR-140 `Decimal[lo, hi; smin, smax; mode]`.
    Decimal(DecimalType),
    /// An FR-148 `Float32` or `Float64`.
    Float(IeeeWidth),
    /// An FR-142 quantity in exactly the unit with this id (ADR-013 T-6);
    /// `value::quantity`'s unit table holds the unit itself.
    Quantity(UnitId),
    /// An FR-141 `Text[min, max; profile]`.
    Text(TextType),
    /// A member of the enum declaration with this node key.
    Enum(NodeKey),
    /// `Option<T>`: `none` or a present `T`.
    Option(Box<ValueType>),
    /// The record or tuple declaration with this node key.
    Composite(NodeKey),
    /// A bounded collection type `K<T>[min, max]`.
    Collection(Box<CollectionType>),
    /// `Reference<T>` to an object of the model object type with this
    /// effective-declaration identity (ADR-013 O-05).
    Reference(EffectiveId),
    /// FR-153's `Population<T>[N]` parameter type: `N` is the declared
    /// maximum an admitted
    /// [`PopulationBinding`](crate::model::population::PopulationBinding)
    /// must carry (never `T` itself, which
    /// `allInstances<T>(p)`/`lookup<T>(p, r)` name separately at each call,
    /// per FR-153's own table).
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
    /// type; their contents were admitted at construction.
    pub fn admits(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Boolean, Value::Boolean(_)) | (Self::Integer, Value::Integer(_)) => true,
            (Self::Int(interval), Value::Integer(integer)) => interval.contains(integer),
            (Self::Rational(domain), Value::Rational(rational)) => domain.contains(rational),
            (Self::Decimal(declared), Value::Decimal(decimal)) => declared.contains(decimal),
            (Self::Float(width), Value::Float(float)) => float.width() == *width,
            (Self::Quantity(unit), Value::Quantity(quantity)) => quantity.unit() == *unit,
            (Self::Text(declared), Value::Text(text)) => text.text_type() == declared,
            (Self::Enum(declaration), Value::Enum(member)) => member.declaration() == *declaration,
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
            // FR-089: `Value::Population` carries only the opaque
            // `PopulationId` a `model` admission minted, never the
            // `PopulationBinding` itself, so this structural check cannot
            // compare `maximum` against a resolved binding's own declared
            // maximum -- that comparison needs the recorded
            // `PopulationId` -> `PopulationBinding` correspondence, which
            // only `ObjectEnvironment` has access to. Presence is admitted
            // here (mirroring FR-153's own restriction of `Population<T>[N]`
            // to a bare parameter type: no nested context ever reaches this
            // arm); the declared-maximum comparison itself happens in two
            // places instead: argument admission's `validate`
            // (`value/expression/mod.rs`) checks it against every
            // `Population<T>[N]` parameter at call time (FR-089-AC-4/AC-5), and
            // `Machine::resolve_population` (`value/expression/evaluate.rs`)
            // checks it again at the `allInstances`/`lookup` sites that
            // consume the identity (FR-089-AC-3/AC-4/AC-5), refusing a
            // mismatch in either place instead.
            (Self::Population(_), Value::Population(_)) => true,
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

/// A completed complete-V1 value. It deliberately has no structural
/// `PartialEq`: equality is the FR-149 relation of
/// [`CheckedEquality::evaluate`](super::CheckedEquality::evaluate).
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
    /// A kernel quantity: its magnitude and its unit's id.
    Quantity(Quantity),
    /// A text value of its declared type.
    Text(Text),
    /// An enum member.
    Enum(EnumValue),
    /// An option value.
    Option(Arc<OptionValue>),
    /// A record or tuple value.
    Composite(Arc<CompositeValue>),
    /// A collection value.
    Collection(Arc<CollectionValue>),
    /// A terminal object reference.
    Reference(ObjectReference),
    /// FR-089: the opaque `PopulationId` a `model` admission
    /// (`admit_binding`/`admit_invocation`) minted for its admitted
    /// `PopulationBinding` -- never the binding itself, which stays a
    /// `model` type. The evaluator resolves this identity to its binding by
    /// lookup in `model`'s recorded correspondence
    /// (`ObjectEnvironment::resolve_population`), never by decoding the
    /// identity's own bytes.
    Population(PopulationId),
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
            | Self::Reference(_)
            | Self::Population(_) => Integer::one(),
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
    /// checking no structural `admits()` match. FR-153's `lookup<T>(p, r)
    /// absent empty` (`crate::value::model_query::evaluate_lookup`) calls
    /// this for a present result `found`, whose `object_type` is `r`'s own
    /// runtime most-specific type `F` (FR-143's own identity triple), not the
    /// queried `T`. Soundness does not rest on
    /// `crate::model::population::lookup`'s `type_conforms` call proving `F`
    /// conforms to `T` directly -- it checks `r`'s *declared* static type `S`
    /// against `T`, never `F` against `T`. It rests on chaining two
    /// invariants: admission guarantees `F` conforms to `S`, never `F == S`
    /// -- `S` is `r`'s declared static type, and `r` may itself be the
    /// result of an earlier upcast lookup, so `S` can already be a proper
    /// supertype of `F`. Example: `lookup<A>(p, lookup<A>(p, rb) absent
    /// refused) absent empty` has `S = A` (the outer call's declared static
    /// type) and `F = B` (`rb`'s own most-specific type, preserved through
    /// the inner call by this same soundness chain). `lookup`'s
    /// `type_conforms(S, T)` call then gives `S` conforms to `T`; so `F`
    /// conforms to `T` transitively (`F` conforms to `S` conforms to `T`).
    /// [`Self::present`]'s own `admits()` call cannot verify that chain
    /// itself -- it is exact object-type equality with no model-conformance
    /// knowledge, so it cannot tell a genuine upcast (`F` a proper subtype of
    /// `T`) from a real type mismatch -- which is why this bypasses it.
    /// Mirrors [`crate::value::collection::from_admitted`]'s same role for
    /// `allInstances`.
    pub(crate) fn from_admitted(payload_type: ValueType, payload: Option<Value>) -> Value {
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

/// A declaration-owned named field; its identity is (declaration key, name).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDeclaration {
    name: String,
    value_type: ValueType,
    presence: Presence,
}

impl FieldDeclaration {
    /// The field `name: value_type` or `name: value_type?`.
    pub fn new(name: impl Into<String>, value_type: ValueType, presence: Presence) -> Self {
        Self {
            name: name.into(),
            value_type,
            presence,
        }
    }

    /// The field identifier.
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
    /// A named record field or object attribute.
    Field(String),
    /// A zero-based tuple position.
    Position(usize),
    /// A zero-based collection occurrence in source order.
    Element(usize),
    /// An option payload.
    Payload,
}

/// Why a construction is `refused { code: ill_typed }`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConstructionCause {
    /// The declaration is not a record or tuple of the environment, or has
    /// the other shape.
    UnknownDeclaration,
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

impl ConstructionRefusal {
    /// The `refused { code }` spelling.
    pub const CODE: &'static str = IllTyped::CODE;
}

/// Refuse construction of `component` with `cause`.
pub(crate) fn refuse<T>(
    component: Component,
    cause: ConstructionCause,
) -> Result<T, ConstructionRefusal> {
    Err(ConstructionRefusal { component, cause })
}

/// Index supplied entries by declared name, refusing an undeclared or
/// repeated name. `value::declaration`'s `TypeEnvironment::evaluate_record`
/// uses it to match supplied
/// `FieldExpression`s to declared fields the same way this module's own
/// [`fill_slots`] matches supplied `FieldValue`s.
pub(crate) fn match_names<'n, T>(
    declared: &[FieldDeclaration],
    supplied: Vec<(&'n str, T)>,
) -> Result<BTreeMap<&'n str, T>, ConstructionRefusal> {
    let mut by_name = BTreeMap::new();
    for (name, entry) in supplied {
        let component = || Component::Field(name.to_owned());
        if !declared.iter().any(|field| field.name == name) {
            return refuse(component(), ConstructionCause::UndeclaredField);
        }
        if by_name.insert(name, entry).is_some() {
            return refuse(component(), ConstructionCause::DuplicateField);
        }
    }
    Ok(by_name)
}

/// Declaration-ordered slots of a record or object from supplied fields.
pub(crate) fn fill_slots(
    declared: &[FieldDeclaration],
    supplied: Vec<(&str, FieldValue)>,
) -> Result<Box<[FieldValue]>, ConstructionRefusal> {
    let mut by_name = match_names(declared, supplied)?;
    let mut slots = Vec::with_capacity(declared.len());
    for field in declared {
        let component = || Component::Field(field.name.clone());
        let slot = by_name
            .remove(field.name.as_str())
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

/// Build a composite value of `declaration` from its slots. This is the only
/// constructor of [`CompositeValue`], whose fields are private to this
/// module; `value::declaration`'s `TypeEnvironment` construction methods
/// call it.
pub(crate) fn composite(declaration: NodeKey, slots: Box<[FieldValue]>) -> Value {
    let occ = slots_occ(&slots);
    Value::Composite(Arc::new(CompositeValue {
        declaration,
        slots,
        occ,
    }))
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
