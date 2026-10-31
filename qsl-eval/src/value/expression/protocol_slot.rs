// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-390's checked protocol-slot selection and caller-supplied inputs.

use quire_eval::Evaluation;
use quire_exact::{NodeKey, Value};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::location::Location;

use qsl_package::AdmittedProtocolObservation;
use quire_semantic_value::admission::{ObservationAdmission, PathStep};
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

pub(crate) mod resolve;

/// Caller-constructible selector for one expression member of an emitted
/// protocol node (FR-390). The fields intentionally stay public; admission
/// checks that the selected node, member and index identify a slot in the
/// checked package before evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSlot {
    /// The selected protocol node's identity.
    pub node: NodeKey,
    /// The exact member name that holds the expression.
    pub member: String,
    /// The list position for a list-valued member, absent for a single slot.
    pub index: Option<usize>,
}

/// One caller-supplied binder value or a source the caller cannot provide.
#[derive(Clone, Debug)]
pub enum SuppliedValue {
    /// The binder's supplied runtime value.
    Value(Value),
    /// The binder exists, but the caller's protocol state has no value for it.
    Unavailable(CatalogCode),
}

/// The selected population for one reference occurrence in a supplied value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolReferenceOccurrence {
    /// The typed path from the supplied value's root to this reference.
    pub path: Vec<PathStep>,
    /// The exact checked population declaration that supplied the reference.
    pub population: NodeKey,
}

/// One binder value with explicit population provenance for every reference.
#[derive(Clone, Debug)]
pub struct ProtocolSuppliedValue {
    /// The value or unavailable source supplied for the binder.
    pub value: SuppliedValue,
    /// The reference occurrences in FR-106 value-walk order.
    pub references: Vec<ProtocolReferenceOccurrence>,
}

impl From<SuppliedValue> for ProtocolSuppliedValue {
    fn from(value: SuppliedValue) -> Self {
        Self {
            value,
            references: Vec::new(),
        }
    }
}

/// The values supplied for one protocol-slot evaluation (FR-390).
#[derive(Clone, Debug)]
pub struct ProtocolSupply<'a> {
    /// Binder identities and values. A sequence preserves duplicates so
    /// admission can report them before checking for missing binders.
    pub binders: Vec<(NodeKey, ProtocolSuppliedValue)>,
    /// The current admitted observation, required only when a value names an
    /// object reference.
    pub observation: Option<&'a ObservationAdmission<AdmittedProtocolObservation<'a>>>,
}

/// The result of evaluating a checked protocol slot (FR-390).
#[derive(Debug)]
pub enum ProtocolSlotEvaluation {
    /// The shared S6a evaluator's result.
    Evaluated(Evaluation),
    /// A supplied binder was unavailable and the expression read it.
    Unavailable {
        /// The binder that was unavailable.
        binder: NodeKey,
        /// The caller-provided reason it was unavailable.
        cause: CatalogCode,
        /// The checked expression read that encountered the binder.
        location: Location,
    },
}

pub(crate) fn foreign_to_observation(
    reference: &quire_exact::ObjectReference,
    observation: &AdmittedProtocolObservation<'_>,
) -> bool {
    let references = observation.environment().references();
    !references.is_empty()
        && references
            .iter()
            .all(|held| held.universe() != reference.universe())
}

/// Reference occurrences in one value, preserving their typed field, option
/// and sequence paths in deterministic depth-first order.
pub(crate) fn reference_occurrences(
    value: &Value,
    types: &TypeEnvironment,
) -> Vec<(Vec<PathStep>, quire_exact::ObjectReference)> {
    let mut references = Vec::new();
    let mut pending = vec![(value, Vec::new())];
    while let Some((value, path)) = pending.pop() {
        match value {
            Value::Reference(reference) => references.push((path, reference.clone())),
            Value::Option(option) => {
                if let Some(value) = option.payload() {
                    let mut nested = path;
                    nested.push(PathStep::OptionValue);
                    pending.push((value, nested));
                }
            }
            Value::Composite(composite) => {
                if let Some(CompositeShape::Record(fields)) = types
                    .composite(composite.declaration().clone())
                    .map(quire_semantic_value::declaration::CompositeDeclaration::shape)
                {
                    for (field, slot) in fields.iter().zip(composite.slots()).rev() {
                        if let quire_exact::FieldValue::Present(value) = slot {
                            let mut nested = path.clone();
                            nested.push(PathStep::Field(field.name().to_owned()));
                            if field.presence() == quire_exact::Presence::Optional {
                                nested.push(PathStep::OptionValue);
                            }
                            pending.push((value, nested));
                        }
                    }
                }
            }
            Value::Collection(collection) => {
                for (index, value) in collection.elements().iter().enumerate().rev() {
                    let mut nested = path.clone();
                    nested.push(PathStep::Index(index));
                    pending.push((value, nested));
                }
            }
            Value::Union(union) => {
                pending.extend(
                    union
                        .payload()
                        .iter()
                        .rev()
                        .map(|value| (value, path.clone())),
                );
            }
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Float(_)
            | Value::Quantity(_)
            | Value::String(_)
            | Value::Text(_)
            | Value::Enum(_)
            | Value::Population(_)
            | Value::Uuid(_)
            | Value::Timestamp(_) => {}
        }
    }
    references
}
