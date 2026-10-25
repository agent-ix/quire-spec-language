// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315: the view of a [`ModelSystem`] the SMT-LIB encoding reads. The
//! encoding declares one state copy per position, so it needs the subject's
//! objects and their fields, the initial states' field values, and each
//! object's operations with the `pre` and `post` clauses and frame that
//! apply to them, as data and not as a state to expand.

use std::collections::BTreeSet;

use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedStateClause;
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, ObjectReference, Value, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::model_transition::ModelTransition;

use super::domain::Domain;
use super::encode::{reference_enc, value_enc};
use super::frames::text_limits;
use super::{ClauseInfo, ModelSystem};
use crate::smt::{LifecycleLocus, Refusal, SmtEncodingRefusal};

/// One declared field of a universe object.
#[derive(Clone, Debug)]
pub(crate) struct SymbolicField {
    /// The field's declaration key.
    pub(crate) key: DeclarationKey,
    /// The field's declared name.
    pub(crate) name: String,
    /// The field's declared type; an optional field's payload type.
    pub(crate) value_type: ValueType,
    /// Whether the field is declared `?`.
    pub(crate) optional: bool,
    /// The position of the field among the type's declared fields.
    pub(crate) declared: usize,
    /// The number of values of the field's finite domain (absent excluded);
    /// `None` for a field with no finite domain.
    pub(crate) domain_len: Option<u128>,
    /// The domain's values in FR-120's order, left empty when the domain
    /// holds more than [`MAX_FIELD_VALUES`].
    pub(crate) values: Vec<Value>,
}

/// The values of one parameter or result, in FR-120's order.
#[derive(Clone, Debug)]
pub(crate) enum ParameterValues {
    /// A finite domain within the encoding's limit.
    Values(Vec<Value>),
    /// A type with no finite domain.
    Unbounded,
    /// A finite domain with this many values, over the encoding's limit.
    TooMany(u128),
}

/// An operation's declared result.
#[derive(Clone, Debug)]
pub(crate) enum ResultValues {
    /// The operation declares no result.
    None,
    /// The result's domain.
    Declared(ParameterValues),
}

/// One operation that applies to an object, with the clauses that apply to
/// it as that object's operation.
pub(crate) struct SymbolicOperation<'a> {
    /// The operation's declared name.
    pub(crate) name: String,
    /// The text of the operation declaration's identity, which a transition
    /// of the operation carries (`ModelTransition::operation`).
    pub(crate) text: String,
    /// The declared parameters, in order.
    pub(crate) parameters: Vec<(String, ValueType)>,
    /// Each parameter's values in FR-120's domain order.
    pub(crate) domains: Vec<ParameterValues>,
    /// The operation's declared result and its values in FR-120's result
    /// order (ascending typed canonical bytes).
    pub(crate) result: ResultValues,
    /// The fields the frame lets the operation write on every object.
    pub(crate) modifies: BTreeSet<DeclarationKey>,
    /// The fields the frame lets the operation write on its receiver alone.
    pub(crate) receiver_modifies: BTreeSet<DeclarationKey>,
    /// The applicable `pre` clauses, in ascending name order.
    pub(crate) preconditions: Vec<&'a CheckedStateClause>,
    /// The applicable `post` clauses, in ascending name order.
    pub(crate) postconditions: Vec<&'a CheckedStateClause>,
}

impl SymbolicOperation<'_> {
    /// The transition FR-120 gives this operation applied to `receiver` with
    /// `arguments`, one for each declared parameter.
    pub(crate) fn transition(
        &self,
        receiver: &ObjectReference,
        arguments: &[Value],
    ) -> Result<ModelTransition, SmtEncodingRefusal> {
        let broken = || fault("transition-identity-text");
        let receiver = reference_enc(receiver)
            .canonical_text(text_limits())
            .map_err(|_| broken())?;
        let arguments = arguments
            .iter()
            .map(|value| value_enc(value).canonical_text(text_limits()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| broken())?;
        ModelTransition::new(self.text.clone(), Some(receiver), arguments, text_limits())
            .map_err(|_| broken())
    }
}

impl SymbolicOperation<'_> {
    /// The operation applied to `arguments` as a refusal names it:
    /// `receiver.name(a, b)`, without the receiver when `receiver` is `None`.
    pub(crate) fn label(&self, receiver: Option<&str>, arguments: &[Value]) -> String {
        let texts: Vec<String> = arguments
            .iter()
            .map(|value| match value {
                Value::Integer(integer) => integer.to_string(),
                Value::Boolean(boolean) => boolean.to_string(),
                Value::Enum(member) => format!("#{}", member.rank()),
                other => value_enc(other)
                    .canonical_text(text_limits())
                    .unwrap_or_else(|_| "?".to_owned()),
            })
            .collect();
        match receiver {
            Some(receiver) => format!("{receiver}.{}({})", self.name, texts.join(", ")),
            None => format!("{}({})", self.name, texts.join(", ")),
        }
    }
}

/// One object of the subject's universe.
pub(crate) struct SymbolicObject<'a> {
    /// The object's population.
    pub(crate) population: DeclarationKey,
    /// The object's key.
    pub(crate) key: String,
    /// The object's most-specific type's effective identity.
    pub(crate) type_id: EffectiveId,
    /// The object's reference.
    pub(crate) reference: ObjectReference,
    /// The type's fields in ascending name order.
    pub(crate) fields: Vec<SymbolicField>,
    /// The operations of the object's effective view, in ascending name
    /// order.
    pub(crate) operations: Vec<SymbolicOperation<'a>>,
}

/// The subject as the encoding reads it.
pub(crate) struct SymbolicSubject<'a> {
    /// The universe objects in ascending key order.
    pub(crate) objects: Vec<SymbolicObject<'a>>,
    /// For each initial state, for each object, the present value of each
    /// field in [`SymbolicObject::fields`] order; `None` for an absent field.
    pub(crate) initial: Vec<Vec<Vec<Option<Value>>>>,
    /// The package's type environment.
    pub(crate) types: &'a TypeEnvironment,
    /// The package.
    pub(crate) package: &'a CheckedPackage,
}

/// The most values an operation parameter's domain may hold.
const MAX_DOMAIN: u128 = 1 << 16;
/// The most values of a written field's domain kept for the candidates.
const MAX_FIELD_VALUES: u128 = 1 << 16;

fn fault(invariant: &'static str) -> SmtEncodingRefusal {
    SmtEncodingRefusal::fault(invariant)
}

impl<'a> ModelSystem<'a> {
    /// The subject's universe objects, their operations and the initial
    /// states' values, for the SMT-LIB encoding (FR-315).
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal`] when the subject has no initial state, when a
    /// universe object is not held at one type by every initial state, when
    /// two populations hold an object under one key, or when an operation
    /// creates or deletes objects.
    pub(crate) fn symbolic(&self) -> Result<SymbolicSubject<'a>, SmtEncodingRefusal> {
        let package: &'a CheckedPackage = self.package;
        let first = self.initial.first().ok_or(Refusal::NoInitialState)?;
        let mut rows: Vec<(DeclarationKey, String, DeclarationKey)> = Vec::new();
        for (population, objects) in &first.state.populations {
            for (key, object) in objects {
                rows.push((
                    population.clone(),
                    key.clone(),
                    object.type_identity.clone(),
                ));
            }
        }
        for (population, keys) in &self.universes {
            for key in keys {
                if !rows
                    .iter()
                    .any(|(held, held_key, _)| held == population && held_key == key)
                {
                    return Err(
                        Refusal::ObjectLifecycle(LifecycleLocus::Object(key.to_string())).into(),
                    );
                }
            }
        }
        for state in &self.initial {
            let mismatch = rows
                .iter()
                .find(|(population, key, type_key)| {
                    !state
                        .state
                        .populations
                        .get(population)
                        .and_then(|objects| objects.get(key))
                        .is_some_and(|object| &object.type_identity == type_key)
                })
                .map(|row| row.1.clone())
                .or_else(|| {
                    state
                        .state
                        .populations
                        .iter()
                        .find_map(|(population, objects)| {
                            objects
                                .keys()
                                .find(|key| {
                                    !rows.iter().any(|(held, held_key, _)| {
                                        held == population && held_key == *key
                                    })
                                })
                                .cloned()
                        })
                });
            if let Some(object) = mismatch {
                return Err(
                    Refusal::ObjectLifecycle(LifecycleLocus::Object(object.to_string())).into(),
                );
            }
        }
        rows.sort_by(|left, right| (&left.1, &left.0).cmp(&(&right.1, &right.0)));
        let mut references = Vec::new();
        let mut identified = Vec::new();
        for (population, key, type_key) in &rows {
            let type_id = self
                .model
                .type_identity(type_key)
                .ok_or_else(|| fault("object-type-identity"))?;
            let reference = self
                .model
                .object_reference(type_key, key)
                .map_err(|_| fault("object-reference"))?;
            references.push(reference.clone());
            identified.push((population, key, type_key, type_id, reference));
        }

        let mut objects = Vec::new();
        for (population, key, type_key, type_id, reference) in identified {
            let mut fields: Vec<SymbolicField> = self
                .fields
                .get(type_key)
                .ok_or_else(|| fault("type-fields"))?
                .iter()
                .enumerate()
                .map(|(declared, field)| {
                    let domain = Domain::of(&field.value_type, &references, self.types);
                    let domain_len = domain.as_ref().map(Domain::len);
                    let values = domain
                        .filter(|domain| domain.len() <= MAX_FIELD_VALUES)
                        .map(|domain| {
                            (0..domain.len())
                                .map_while(|index| domain.nth(index))
                                .collect()
                        })
                        .unwrap_or_default();
                    SymbolicField {
                        key: field.key.clone(),
                        name: field.name.clone(),
                        value_type: field.value_type.clone(),
                        optional: field.optional,
                        declared,
                        domain_len,
                        values,
                    }
                })
                .collect();
            fields.sort_by(|left, right| left.name.cmp(&right.name));
            let operations = self.symbolic_operations(type_id, &references)?;
            objects.push(SymbolicObject {
                population: population.clone(),
                key: key.clone(),
                type_id,
                reference,
                fields,
                operations,
            });
        }

        if let Some(pair) = rows.windows(2).find(|pair| pair[0].1 == pair[1].1) {
            return Err(Refusal::ConstantName {
                name: pair[0].1.clone(),
            }
            .into());
        }

        let initial = self
            .initial
            .iter()
            .map(|state| {
                objects
                    .iter()
                    .map(|object| {
                        let held = state
                            .state
                            .populations
                            .get(&object.population)
                            .and_then(|held| held.get(&object.key));
                        object
                            .fields
                            .iter()
                            .map(|field| held.and_then(|held| held.fields.get(&field.key)).cloned())
                            .collect()
                    })
                    .collect()
            })
            .collect();
        Ok(SymbolicSubject {
            objects,
            initial,
            types: self.types,
            package,
        })
    }

    fn symbolic_operations(
        &self,
        type_id: EffectiveId,
        references: &[ObjectReference],
    ) -> Result<Vec<SymbolicOperation<'a>>, SmtEncodingRefusal> {
        let package: &'a CheckedPackage = self.package;
        let mut operations = Vec::new();
        for application in self.applications.get(&type_id).into_iter().flatten() {
            let effect = application.declaration.effect();
            let lifecycle = !effect.creates.is_empty()
                || !effect.deletes.is_empty()
                || effect.deletes_receiver
                || !effect.deletes_arguments.is_empty();
            if lifecycle {
                return Err(Refusal::ObjectLifecycle(LifecycleLocus::Operation(
                    application.name.clone(),
                ))
                .into());
            }
            let parameters = application.declaration.parameters().to_vec();
            let ordered =
                |domain: Domain, order: bool| -> Result<ParameterValues, SmtEncodingRefusal> {
                    if domain.len() > MAX_DOMAIN {
                        return Ok(ParameterValues::TooMany(domain.len()));
                    }
                    let mut values: Vec<Value> = (0..domain.len())
                        .map_while(|index| domain.nth(index))
                        .collect();
                    if order {
                        let mut keyed = Vec::with_capacity(values.len());
                        for value in values {
                            let bytes = quire_canonical::to_vec(&value_enc(&value), text_limits())
                                .map_err(|_| fault("result-canonical-bytes"))?;
                            keyed.push((bytes, value));
                        }
                        keyed.sort_by(|left, right| left.0.cmp(&right.0));
                        values = keyed.into_iter().map(|(_, value)| value).collect();
                    }
                    Ok(ParameterValues::Values(values))
                };
            let mut domains = Vec::new();
            for (position, (_, value_type)) in parameters.iter().enumerate() {
                domains.push(
                    match self.probabilistic.support(
                        application.declaring,
                        &application.name,
                        position,
                    ) {
                        Some(support) => ParameterValues::Values(support.values()),
                        None => match Domain::of(value_type, references, self.types) {
                            Some(domain) => ordered(domain, false)?,
                            None => ParameterValues::Unbounded,
                        },
                    },
                );
            }
            let result = match application.declaration.result() {
                None => ResultValues::None,
                Some(result_type) => {
                    ResultValues::Declared(match Domain::of(result_type, references, self.types) {
                        Some(domain) => ordered(domain, true)?,
                        None => ParameterValues::Unbounded,
                    })
                }
            };
            let applicable =
                |group: &[ClauseInfo]| -> Result<Vec<&'a CheckedStateClause>, SmtEncodingRefusal> {
                    group
                        .iter()
                        .filter(|clause| {
                            clause.operation.as_ref().is_some_and(|(owner, name)| {
                                *owner == application.declaring && *name == application.name
                            }) && self.types.conforms(type_id, clause.context)
                        })
                        .map(|clause| {
                            package
                                .graph()
                                .state_clause(&clause.name)
                                .ok_or_else(|| fault("clause-resolves"))
                        })
                        .collect()
                };
            operations.push(SymbolicOperation {
                name: application.name.clone(),
                text: application.text.clone(),
                parameters,
                domains,
                result,
                modifies: effect.modifies.iter().cloned().collect(),
                receiver_modifies: effect.receiver_modifies.iter().cloned().collect(),
                preconditions: applicable(&self.clauses.preconditions)?,
                postconditions: applicable(&self.clauses.postconditions)?,
            });
        }
        Ok(operations)
    }
}
