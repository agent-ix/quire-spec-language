// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 records, tuples and finite recursive values: the K3 registry's own
//! field metadata and construction-refusal reporting, over `quire_exact`'s
//! kernel `Value`/`ValueType` (QSL-131 V5).
//!
//! `Value`, `ValueType`, `OptionValue`, `CompositeValue`, `FieldValue` and
//! [`Deferred`] are `quire_exact`'s own K-designated types now (ADR-011
//! §6.1's K row); this module re-exports them rather than duplicating them.
//! [`FieldDeclaration`], [`Component`], [`ConstructionCause`] and
//! [`ConstructionRefusal`] stay QSL's own, non-kernel types (ADR-011 §6.1;
//! ADR-013 O-15): a field is identified here by its declared *name* (a
//! `str`-keyed lookup, [`match_names`]/[`fill_slots`]), not by the kernel's
//! opaque `MemberId` (`quire_exact::FieldDeclaration`'s own key) -- adopting
//! `MemberId` here needs the ADR-013 O-06 member-identity resolution that
//! `check`'s `CheckedGraph` (ADR-013 T-1, S-3) has not yet landed
//! (`value::member`'s own doc comment: "No production caller constructs a
//! `Member` yet"). `ConstructionCause` also carries `UnknownDeclaration`,
//! which the kernel's own `ConstructionCause` has no need of: the kernel's
//! `record`/`tuple` take their declared shape directly, with no registry
//! lookup to fail, while `value::declaration`'s `TypeEnvironment::record`/
//! `tuple`/`evaluate_record`/`evaluate_tuple` look a `NodeKey` up in the
//! registry first and must report that lookup's own failure. Because of
//! this, `TypeEnvironment` cannot call the kernel's checked `record`/`tuple`
//! constructors either (they need the kernel's `MemberId`-keyed
//! `FieldDeclaration`); it does its own name-keyed checking exactly as
//! before, through this module's [`fill_slots`]/[`match_names`], and then
//! calls the kernel's trusted, unchecked
//! [`from_admitted_slots`](quire_exact::from_admitted_slots) to materialize
//! the result -- mirroring `quire_exact::OptionValue::from_admitted`'s
//! identical bypass role, which `value::expression::evaluate` already calls
//! directly.
//!
//! FR-089-AC-6 (QSL-131 V5): kernel `ValueType::admits` refuses every
//! `(ValueType::Population, Value::Population)` pair outright -- the
//! declared-maximum comparison (FR-089-AC-5) is the QSL layer's own check.
//! `Population` is FR-153's own restriction: it is never nested inside a
//! record field, tuple position, option payload or collection element (every
//! such context is refused earlier, at declaration admission, by
//! `value::declaration::TypeEnvironment::type_refusal`), so none of this
//! module's `admits()` calls (`fill_slots`'s field check) ever receive a
//! `Population` pair; only `value::expression::validate`'s top-level
//! parameter-admission loop needs the FR-089-AC-5 compensation, since
//! `Population<T>[N]` is reachable there directly as a bare parameter type.

use std::collections::BTreeMap;

use quire_exact::{Charge, ChargePoint, IllTyped, LimitKind, Meter, NodeKey, Presence};
pub use quire_exact::{
    from_admitted_slots, CompositeValue, Deferred, FieldValue, OptionValue, Value, ValueType,
};

use super::stop::Stop;

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

/// Charge `composite.result-retain` with `occ(result)`, then expose it. Not
/// reusable from `quire_exact` (its own `retain_composite` is
/// `pub(crate)` there, since the kernel's checked `record`/`tuple`/
/// `evaluate_record`/`evaluate_tuple` are its only callers); this module's
/// callers are `value::declaration`'s name-keyed `evaluate_record`/
/// `evaluate_tuple`, which is why this one small charge-and-return helper
/// stays QSL's own rather than a K-copy of kernel logic.
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

/// Build a composite value of `declaration` from already name-checked
/// slots, through the kernel's trusted
/// [`from_admitted_slots`](quire_exact::from_admitted_slots) bypass:
/// `value::declaration`'s `TypeEnvironment` construction methods have
/// already checked every slot against its own declared shape, so no second,
/// kernel-side check is needed (and the kernel's own checked `record`/
/// `tuple` are not reachable here, since they take `MemberId`-keyed
/// declarations this module does not have -- see the module doc comment).
pub(crate) fn composite(declaration: NodeKey, slots: Box<[FieldValue]>) -> Value {
    from_admitted_slots(declaration, slots)
}
