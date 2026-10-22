// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315: the view of a [`ModelSystem`] the SMT-LIB encoding reads. The
//! encoding declares one state copy per position, so it needs the subject's
//! objects and their fields, the initial states' field values, and each
//! object's operations with the `pre` and `post` clauses and frame that
//! apply to them, as data and not as a state to expand.

use std::collections::BTreeSet;

use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedStateClause;
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{EffectiveId, ObjectReference, Value, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;

use super::domain::Domain;
use super::{ClauseInfo, ModelSystem};
use crate::smt::SmtEncodingRefusal;

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
}

/// One operation that applies to an object, with the clauses that apply to
/// it as that object's operation.
pub(crate) struct SymbolicOperation<'a> {
    /// The operation's declared name.
    pub(crate) name: String,
    /// The declared parameters, in order.
    pub(crate) parameters: Vec<(String, ValueType)>,
    /// Each parameter's values in FR-120's domain order, or `None` for a
    /// parameter with no finite domain.
    pub(crate) domains: Vec<Option<Vec<Value>>>,
    /// The fields the frame lets the operation write on every object.
    pub(crate) modifies: BTreeSet<DeclarationKey>,
    /// The fields the frame lets the operation write on its receiver alone.
    pub(crate) receiver_modifies: BTreeSet<DeclarationKey>,
    /// The applicable `pre` clauses, in ascending name order.
    pub(crate) preconditions: Vec<&'a CheckedStateClause>,
    /// The applicable `post` clauses, in ascending name order.
    pub(crate) postconditions: Vec<&'a CheckedStateClause>,
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

fn fault(invariant: &'static str) -> SmtEncodingRefusal {
    SmtEncodingRefusal::Fault(qsl_foundation::InternalFault::new(
        "smt-encoding",
        invariant,
    ))
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
        let first = self
            .initial
            .first()
            .ok_or(SmtEncodingRefusal::NoInitialState)?;
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
                    return Err(SmtEncodingRefusal::ObjectLifecycle {
                        object: key.clone(),
                    });
                }
            }
        }
        for state in &self.initial {
            let held: usize = state
                .state
                .populations
                .values()
                .map(std::collections::BTreeMap::len)
                .sum();
            let same = held == rows.len()
                && rows.iter().all(|(population, key, type_key)| {
                    state
                        .state
                        .populations
                        .get(population)
                        .and_then(|objects| objects.get(key))
                        .is_some_and(|object| &object.type_identity == type_key)
                });
            if !same {
                let object = rows.first().map(|row| row.1.clone()).unwrap_or_default();
                return Err(SmtEncodingRefusal::ObjectLifecycle { object });
            }
        }
        rows.sort_by(|left, right| (&left.1, &left.0).cmp(&(&right.1, &right.0)));
        if let Some(pair) = rows.windows(2).find(|pair| pair[0].1 == pair[1].1) {
            return Err(SmtEncodingRefusal::ObjectNameClash {
                object: pair[0].1.clone(),
            });
        }

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
                .map(|field| SymbolicField {
                    key: field.key.clone(),
                    name: field.name.clone(),
                    value_type: field.value_type.clone(),
                    optional: field.optional,
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
                return Err(SmtEncodingRefusal::OperationLifecycle {
                    operation: application.name.clone(),
                });
            }
            let parameters = application.declaration.parameters().to_vec();
            let domains = parameters
                .iter()
                .enumerate()
                .map(|(position, (_, value_type))| {
                    match self.probabilistic.support(
                        application.declaring,
                        &application.name,
                        position,
                    ) {
                        Some(support) => Some(support.values()),
                        None => Domain::of(value_type, references, self.types).map(|domain| {
                            (0..domain.len())
                                .map_while(|index| domain.nth(index))
                                .collect()
                        }),
                    }
                })
                .collect();
            let applicable = |group: &[ClauseInfo]| -> Vec<&'a CheckedStateClause> {
                group
                    .iter()
                    .filter(|clause| {
                        clause.operation.as_ref().is_some_and(|(owner, name)| {
                            *owner == application.declaring && *name == application.name
                        }) && self.types.conforms(type_id, clause.context)
                    })
                    .filter_map(|clause| package.graph().state_clause(&clause.name))
                    .collect()
            };
            operations.push(SymbolicOperation {
                name: application.name.clone(),
                parameters,
                domains,
                modifies: effect.modifies.iter().cloned().collect(),
                receiver_modifies: effect.receiver_modifies.iter().cloned().collect(),
                preconditions: applicable(&self.clauses.preconditions),
                postconditions: applicable(&self.clauses.postconditions),
            });
        }
        Ok(operations)
    }
}
