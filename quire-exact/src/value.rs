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
//! - `ValueType::Enum(NodeKey)` is **dropped**. A bare `Value::Enum(VariantId)`
//!   (T-6: "no `NodeKey`") gives `ValueType::admits` nothing to look up an
//!   enum declaration by, so the kernel cannot type-check enum membership at
//!   all; that check now belongs to whichever layer holds the enum
//!   declaration (QSL `model`/`check`). This is a real, deliberate capability
//!   loss at the kernel boundary, not an oversight -- flagged for review.
//! - `ValueType::Reference(NodeKey)` becomes `ValueType::Reference(EffectiveId)`
//!   and `Value::Reference(ObjectReference)` carries the T-6 triple
//!   (`EffectiveId`, `UniverseId`, `ObjectId`) from [`crate::reference`],
//!   not a `NodeKey` plus raw identity bytes.
//! - `ValueType::Quantity(QuantityUnit)` becomes `ValueType::Quantity(UnitId)`;
//!   `Value::Quantity` carries [`crate::quantity::Quantity`], a bare
//!   `(magnitude, UnitId)` pair with no unit-graph declaration.
//! - `Value::Population(Arc<PopulationBinding>)` is **dropped**: a
//!   `PopulationBinding` is a QSL `model` type. `ValueType::Population(u64)`
//!   is unchanged (T-6: "keeps `u64` count only") but now admits no `Value`
//!   variant at all, since the value side of that pairing left the kernel.
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
use crate::identity::{EffectiveId, UnitId, VariantId};
use crate::ieee::{IeeeValue, IeeeWidth};
use crate::integer::{Integer, IntegerInterval};
use crate::node::NodeKey;
use crate::outcome::{Outcome, Refusal, Stop};
use crate::quantity::Quantity;
use crate::rational::{Rational, RationalDomain};
use crate::reference::ObjectReference;
use crate::text::{Text, TextType};

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
    /// type; their contents were admitted at construction. No `ValueType`
    /// variant admits a `Value::Enum` (see the module doc comment): enum
    /// membership is not a kernel-checkable fact.
    pub fn admits(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Boolean, Value::Boolean(_)) | (Self::Integer, Value::Integer(_)) => true,
            (Self::Int(interval), Value::Integer(integer)) => interval.contains(integer),
            (Self::Rational(domain), Value::Rational(rational)) => domain.contains(rational),
            (Self::Decimal(declared), Value::Decimal(decimal)) => declared.contains(decimal),
            (Self::Float(width), Value::Float(float)) => float.width() == *width,
            (Self::Quantity(unit), Value::Quantity(quantity)) => quantity.unit() == *unit,
            (Self::Text(declared), Value::Text(text)) => text.text_type() == declared,
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
            (
                Self::Boolean
                | Self::Integer
                | Self::Int(_)
                | Self::Rational(_)
                | Self::Decimal(_)
                | Self::Float(_)
                | Self::Quantity(_)
                | Self::Text(_)
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
/// equality is the relation [`crate::equality`] plans and evaluates.
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
    /// A bare enum member identity (ADR-013 T-6).
    Enum(VariantId),
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

/// A declaration-owned named field.
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

/// Construct a record from its declared `shape` (looked up by the caller's
/// own declaration registry -- QSL's `TypeEnvironment`, not the kernel).
/// An omitted `?` field is `absent`.
pub fn record(
    shape: &[FieldDeclaration],
    declaration: NodeKey,
    fields: Vec<(&str, FieldValue)>,
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
    fields: Vec<(&str, FieldExpression<'_>)>,
    meter: &mut Meter,
) -> Result<Outcome<Value>, ConstructionRefusal> {
    let mut supplied = match_names(shape, fields)?;
    let mut plan = Vec::with_capacity(shape.len());
    for field in shape {
        let expression = supplied.remove(field.name.as_str());
        let component = || Component::Field(field.name.clone());
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
    /// A named record field or object attribute.
    Field(String),
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

/// Index supplied entries by declared name, refusing an undeclared or
/// repeated name.
fn match_names<'n, T>(
    declared: &[FieldDeclaration],
    supplied: Vec<(&'n str, T)>,
) -> Result<std::collections::BTreeMap<&'n str, T>, ConstructionRefusal> {
    let mut by_name = std::collections::BTreeMap::new();
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

/// Declaration-ordered slots of a record from supplied fields.
pub fn fill_slots(
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

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    /// TC-307: `ValueType::Boolean` admits only `Value::Boolean`, refusing
    /// a value of another kind.
    #[trace("TC-307")]
    #[test]
    fn tc_307_admits_checks_the_matching_variant_only() {
        assert!(ValueType::Boolean.admits(&Value::Boolean(true)));
        assert!(!ValueType::Boolean.admits(&Value::Integer(Integer::one())));
    }

    /// TC-308: no `ValueType` admits a `Value::Enum` (the documented kernel
    /// capability loss at the T-6 boundary).
    #[trace("TC-308")]
    #[test]
    fn tc_308_no_value_type_admits_a_bare_enum_value() {
        let variant = Value::Enum(VariantId::from_digest(digest(1)));
        assert!(!ValueType::Boolean.admits(&variant));
        assert!(!ValueType::Integer.admits(&variant));
    }

    /// TC-309: building a record with a missing required field is refused
    /// at that field, before any other field is inspected.
    #[trace("TC-309")]
    #[test]
    fn tc_309_record_refuses_a_missing_required_field() {
        let shape = vec![FieldDeclaration::new(
            "count",
            ValueType::Integer,
            Presence::Required,
        )];
        let err = record(&shape, NodeKey::from_digest(digest(1)), vec![]).unwrap_err();
        assert_eq!(err.component, Component::Field("count".to_owned()));
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
}
