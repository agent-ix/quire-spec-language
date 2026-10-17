// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-366: the invariants of a model type, evaluated once for every value
//! they range over, each false result located by its QSpec FR-207 runtime
//! value path.
//!
//! [`evaluate`] collects the values an admitted observation reaches, pairs
//! each with every checked invariant that applies to its type, orders the
//! pairs by path and clause key, and evaluates them under the one `Meter` the
//! caller gives it. A result is the FR-107 [`Evaluation`] of the invariant.

use qsl_foundation::diagnostic::InternalFault;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedTypeInvariant, TypeSlot};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{AdmittedObservations, Observation, SelfKind};
use quire_exact::{FieldValue, Incomplete, Meter, ObjectReference, Outcome, Presence, Value};
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

use super::evaluate::{Evaluation, Machine};
use super::s6a::separation::{
    observation_identity, InvocationBinding, RuntimeValuePath, ValuePathStep, ValuePathSubject,
};
use super::CallFailure;

/// One invariant's result for one value (FR-366 "Evaluation per reachable
/// value").
#[derive(Debug)]
pub struct ValueInvariantResult {
    /// The invariant's clause key.
    pub clause: DeclarationKey,
    /// The QSpec FR-207 runtime value path of the value the invariant was
    /// evaluated over.
    pub path: RuntimeValuePath,
    /// The FR-107 evaluation of the invariant over that value: a linked
    /// invariant completes `true` or `false`, and a false result is a Boolean
    /// with no diagnostic code or cause.
    pub evaluation: Evaluation,
}

impl ValueInvariantResult {
    /// Whether the invariant holds for the value.
    pub fn holds(&self) -> bool {
        matches!(
            self.evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
        )
    }
}

/// The outcome of evaluating every invariant over every value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueInvariantOutcome {
    /// Every result is true, or there is none.
    True,
    /// Some result is false.
    False,
    /// A charge was denied: the evaluation stopped at the invariant whose
    /// evaluation was denied, which has no result.
    Incomplete(Incomplete),
}

/// FR-366: the results of `CheckedPackageEvaluation::evaluate_value_invariants` and
/// their outcome.
#[derive(Debug)]
pub struct ValueInvariantEvaluation {
    /// One result for each invariant and value, in the FR-207 key order of
    /// their paths, then by clause key.
    pub results: Vec<ValueInvariantResult>,
    /// The combined outcome.
    pub outcome: ValueInvariantOutcome,
}

/// One value to check, with the environment its invariants read.
struct Pending<'a> {
    path: RuntimeValuePath,
    value: Value,
    slot: TypeSlot,
    environment: Option<&'a ObjectEnvironment>,
}

/// A value still to be walked.
struct Walk {
    path: RuntimeValuePath,
    value: Value,
}

fn fault(reason: &'static str) -> CallFailure {
    CallFailure::Fault(InternalFault::new("S6a", reason))
}

/// The values `observations` reach, as `(path, value, type)` triples:
/// objects with their members, the invocation's parameters and result, and
/// a state machine's context in each side of the invocation.
fn collect<'a>(
    types: &TypeEnvironment,
    observations: &'a AdmittedObservations,
) -> Result<Vec<Pending<'a>>, CallFailure> {
    let mut pending = Vec::new();
    if let Some(current) = &observations.current {
        let own = observation_identity(&current.identity);
        for reference in current.environment.references() {
            let root = RuntimeValuePath {
                observation: own.clone(),
                subject: ValuePathSubject::Object(reference.clone()),
                steps: Vec::new(),
            };
            push_object(types, &mut pending, root, reference, &current.environment)?;
        }
    }
    let invocation = observations.invocation.as_ref().map(observation_identity);
    if let (Some(invocation), Some(pre), Some(post)) = (
        invocation,
        observations.pre.as_ref(),
        observations.post.as_ref(),
    ) {
        let binding = |observation: &Observation, binding: InvocationBinding| RuntimeValuePath {
            observation: observation_identity(&observation.identity),
            subject: ValuePathSubject::Invocation {
                invocation: invocation.clone(),
                binding,
            },
            steps: Vec::new(),
        };
        let mut walks = Vec::new();
        for (name, value) in &observations.parameters {
            walks.push(Walk {
                path: binding(pre, InvocationBinding::Parameter(name.clone())),
                value: value.clone(),
            });
        }
        if let Some(result) = &observations.result {
            walks.push(Walk {
                path: binding(post, InvocationBinding::Result),
                value: result.clone(),
            });
        }
        walk(types, &mut pending, walks)?;
        if observations.self_kind == SelfKind::Context {
            for (reference, side) in [
                (observations.receiver_pre.as_ref(), pre),
                (observations.receiver_post.as_ref(), post),
            ] {
                let Some(reference) = reference else {
                    continue;
                };
                let root = binding(side, InvocationBinding::Receiver);
                pending.push(Pending {
                    path: root,
                    value: Value::Reference(reference.clone()),
                    slot: TypeSlot::Object(reference.object_type()),
                    environment: Some(&side.environment),
                });
            }
        }
    }
    Ok(pending)
}

/// The object `reference` of `environment` as a value of its type, then each
/// member of it walked.
fn push_object<'a>(
    types: &TypeEnvironment,
    pending: &mut Vec<Pending<'a>>,
    root: RuntimeValuePath,
    reference: &ObjectReference,
    environment: &'a ObjectEnvironment,
) -> Result<(), CallFailure> {
    pending.push(Pending {
        path: root.clone(),
        value: Value::Reference(reference.clone()),
        slot: TypeSlot::Object(reference.object_type()),
        environment: Some(environment),
    });
    let attributes = types
        .attributes(reference.object_type())
        .ok_or_else(|| fault("admitted-object-type-not-declared"))?;
    let mut walks = Vec::new();
    for attribute in attributes {
        let slot = environment
            .objects()
            .attribute(types, reference, &attribute.identity())
            .ok_or_else(|| fault("admitted-object-member-not-stored"))?;
        let FieldValue::Present(value) = slot else {
            continue;
        };
        let mut path = root.clone();
        path.steps
            .push(ValuePathStep::Member(attribute.field().name().to_owned()));
        present_payload(&mut path, attribute.field().presence());
        walks.push(Walk {
            path,
            value: value.clone(),
        });
    }
    walk(types, pending, walks)?;
    Ok(())
}

/// A present slot of an optional field is the payload of an optional value
/// (QSpec FR-207's option-value selection).
fn present_payload(path: &mut RuntimeValuePath, presence: Presence) {
    if presence == Presence::Optional {
        path.steps.push(ValuePathStep::OptionValue);
    }
}

/// Every record or event value contained in the walked values, each at its
/// path: a field of a record, the payload of a present optional and an
/// element of a collection. A reference is not followed.
fn walk(
    types: &TypeEnvironment,
    pending: &mut Vec<Pending<'_>>,
    start: Vec<Walk>,
) -> Result<(), CallFailure> {
    let mut stack = start;
    while let Some(Walk { path, value }) = stack.pop() {
        match &value {
            Value::Composite(composite) => {
                pending.push(Pending {
                    path: path.clone(),
                    value: value.clone(),
                    slot: TypeSlot::Composite(composite.declaration()),
                    environment: None,
                });
                let Some(CompositeShape::Record(fields)) = types
                    .composite(composite.declaration())
                    .map(quire_semantic_value::declaration::CompositeDeclaration::shape)
                else {
                    continue;
                };
                for (field, slot) in fields.iter().zip(composite.slots()) {
                    if let FieldValue::Present(inner) = slot {
                        let mut path = path.clone();
                        path.steps
                            .push(ValuePathStep::Field(field.name().to_owned()));
                        present_payload(&mut path, field.presence());
                        stack.push(Walk {
                            path,
                            value: inner.clone(),
                        });
                    }
                }
            }
            Value::Option(option) => {
                if let Some(inner) = option.payload() {
                    let mut path = path.clone();
                    path.steps.push(ValuePathStep::OptionValue);
                    stack.push(Walk {
                        path,
                        value: inner.clone(),
                    });
                }
            }
            Value::Collection(collection) => {
                for (index, element) in collection.elements().iter().enumerate() {
                    let mut path = path.clone();
                    let index =
                        u64::try_from(index).map_err(|_| fault("collection-index-exceeds-u64"))?;
                    path.steps.push(ValuePathStep::Index(index));
                    stack.push(Walk {
                        path,
                        value: element.clone(),
                    });
                }
            }
            Value::Reference(_)
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Float(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::String(_)
            | Value::Uuid(_)
            | Value::Timestamp(_)
            | Value::Enum(_)
            | Value::Union(_)
            | Value::Population(_) => {}
        }
    }
    Ok(())
}

/// FR-366: evaluate every checked type invariant of `package` over every
/// value `observations` reach, under the one `meter`.
pub(super) fn evaluate(
    package: &CheckedPackage,
    observations: &AdmittedObservations,
    meter: &mut Meter,
) -> Result<ValueInvariantEvaluation, CallFailure> {
    let graph = package.graph();
    let invariants = graph.type_invariants();
    let mut tasks: Vec<(&CheckedTypeInvariant, Pending<'_>)> = Vec::new();
    for value in collect(graph.scope().types(), observations)? {
        for invariant in invariants
            .iter()
            .filter(|candidate| candidate.applies_to(value.slot))
        {
            tasks.push((
                invariant,
                Pending {
                    path: value.path.clone(),
                    value: value.value.clone(),
                    slot: value.slot,
                    environment: value.environment,
                },
            ));
        }
    }
    tasks.sort_by(|(left_clause, left), (right_clause, right)| {
        (&left.path, left_clause.clause()).cmp(&(&right.path, right_clause.clause()))
    });
    let empty = ObjectEnvironment::default();
    let no_reads = std::collections::BTreeMap::new();
    let mut results = Vec::with_capacity(tasks.len());
    let mut violated = false;
    for (invariant, task) in tasks {
        let objects = task.environment.unwrap_or(&empty);
        let evaluation = Machine::with_pre(
            graph.scope(),
            graph,
            objects,
            None,
            Some(&no_reads),
            meter,
            graph.dispatch_tables(),
        )
        .run(invariant.body(), invariant.slots(), vec![task.value])
        .map_err(CallFailure::Fault)?;
        match &evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(holds))) => {
                violated |= !holds;
            }
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                return Ok(ValueInvariantEvaluation {
                    results,
                    outcome: ValueInvariantOutcome::Incomplete(incomplete.clone()),
                });
            }
            _ => return Err(fault("linked-type-invariant-completed-no-boolean")),
        }
        results.push(ValueInvariantResult {
            clause: invariant.clause().clone(),
            path: task.path,
            evaluation,
        });
    }
    Ok(ValueInvariantEvaluation {
        results,
        outcome: if violated {
            ValueInvariantOutcome::False
        } else {
            ValueInvariantOutcome::True
        },
    })
}
