// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-485: what the `operation-contract` check arm reads of a model
//! system.
//!
//! The arm decides a claim over *pre-states of the declared types*, not over
//! reachable states (FR-485 "Common premises"). This module gives it those
//! pre-states and the same readings FR-120's expansion makes, so the arm and
//! EN-1 evaluate one contract relation:
//!
//! * [`ModelSystem::declared_states`] enumerates every closed state over the
//!   system's universes: each universe key absent or holding an object of a
//!   non-abstract member type of its population, and each such object's
//!   fields over their FR-120 domains.
//! * [`ModelSystem::invariant_findings_at`] reads a state's invariants and
//!   unit bounds (FR-120 "Invariants").
//! * [`ModelSystem::operation_cases`] applies one operation at a state under
//!   the `pre` and `post` clauses a selection admits, and gives each
//!   successor with its receiver, arguments and result, and each contract
//!   clause that decided no Boolean with the case it was evaluated at.
//! * [`ModelSystem::precondition_cases`] evaluates one precondition alone at
//!   every argument vector.

use std::collections::BTreeMap;

use qsl_semantics::model::observation::{AnchorKind, SelectedAnchor};
use qsl_semantics::model::state::{ModelState, StateObject};
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, NodeKey, ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use super::application::OperationRef;
use super::domain::{Domain, Odometer};
use super::frames;
use super::walk::{rows_of, walk_of, Applied};
use super::{ClauseInfo, ClauseOutcome, ModelFinding, ModelSimState, ModelSystem};
use crate::simulation::expansion::ExpansionStop;

/// The kind of a state clause the system evaluates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractClauseKind {
    /// An `invariant` of its context.
    Invariant,
    /// A `pre` clause of an operation.
    Precondition,
    /// A `post` clause of an operation.
    Postcondition,
}

/// A state clause as the system evaluates it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractClause {
    /// The clause's declared name, the name its findings carry.
    pub name: String,
    /// The clause's checked identity.
    pub identity: NodeKey,
    /// The clause's kind.
    pub kind: ContractClauseKind,
    /// The clause's context type.
    pub context: EffectiveId,
    /// The operation a `pre` or `post` clause resolves to: its declaring
    /// type and name.
    pub operation: Option<(EffectiveId, String)>,
}

/// Why a system's declared pre-states cannot be enumerated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclaredStatesRefusal {
    /// The model lists a relationship, whose extents the enumeration does
    /// not range over.
    Relationship,
    /// The model holds a clock member, which has no value domain here.
    ClockMember,
    /// A field of a member type has no finite domain.
    Field {
        /// The field's declared name.
        field: String,
    },
}

/// One universe key a declared state may hold an object under.
struct Slot {
    population: DeclarationKey,
    key: String,
    /// The non-abstract types an object under the key may have.
    types: Vec<DeclarationKey>,
}

/// One field of one present object of a declared state.
struct Dimension {
    population: DeclarationKey,
    key: String,
    field: DeclarationKey,
    optional: bool,
    domain: Domain,
}

impl Dimension {
    fn radix(&self) -> u128 {
        self.domain.len().saturating_add(u128::from(self.optional))
    }

    /// The value at `index`: `Some(None)` for an absent `?` field.
    fn value(&self, index: u128) -> Option<Option<Value>> {
        if self.optional {
            match index.checked_sub(1) {
                None => Some(None),
                Some(position) => self.domain.nth(position).map(Some),
            }
        } else {
            self.domain.nth(index).map(Some)
        }
    }
}

/// The field assignments of one membership choice.
struct Fields {
    objects: BTreeMap<DeclarationKey, BTreeMap<String, StateObject>>,
    dimensions: Vec<Dimension>,
    values: Odometer,
}

/// Every declared pre-state of a system (QSpec FR-485 "Common premises"
/// premise 1), memberships in odometer order (the last universe key
/// fastest, absent first), then field values in FR-120's domain order.
pub struct DeclaredStates<'s, 'a> {
    system: &'s ModelSystem<'a>,
    anchor: SelectedAnchor,
    slots: Vec<Slot>,
    memberships: Odometer,
    current: Option<Fields>,
}

impl Iterator for DeclaredStates<'_, '_> {
    type Item = Result<ModelSimState, ExpansionStop>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(fields) = &mut self.current {
                if let Some(digits) = fields.values.next_digits() {
                    return Some(self.build(&digits));
                }
                self.current = None;
            }
            let membership = self.memberships.next_digits()?;
            match self.fields_of(&membership) {
                Ok(fields) => self.current = Some(fields),
                Err(stop) => return Some(Err(stop)),
            }
        }
    }
}

impl DeclaredStates<'_, '_> {
    /// The objects and field dimensions of one membership choice.
    fn fields_of(&self, membership: &[u128]) -> Result<Fields, ExpansionStop> {
        let system = self.system;
        let mut objects: BTreeMap<DeclarationKey, BTreeMap<String, StateObject>> = BTreeMap::new();
        let mut present = Vec::new();
        for (slot, choice) in self.slots.iter().zip(membership) {
            let Some(position) = choice.checked_sub(1) else {
                continue;
            };
            let type_key = usize::try_from(position)
                .ok()
                .and_then(|position| slot.types.get(position))
                .cloned()
                .ok_or_else(broken)?;
            let reference = system
                .model
                .object_reference(&type_key, &slot.key)
                .map_err(|_| broken())?;
            objects.entry(slot.population.clone()).or_default().insert(
                slot.key.clone(),
                StateObject {
                    type_identity: type_key.clone(),
                    fields: BTreeMap::new(),
                },
            );
            present.push((slot, type_key, reference));
        }
        let references: Vec<ObjectReference> = present
            .iter()
            .map(|(_, _, reference)| reference.clone())
            .collect();
        let mut dimensions = Vec::new();
        for (slot, type_key, _) in &present {
            let fields = system.fields.get(type_key).ok_or_else(broken)?;
            for field in fields {
                let domain = system
                    .field_domain(type_key, field, &references)
                    .ok_or_else(broken)?;
                dimensions.push(Dimension {
                    population: slot.population.clone(),
                    key: slot.key.clone(),
                    field: field.key.clone(),
                    optional: field.optional,
                    domain,
                });
            }
        }
        let values = Odometer::new(dimensions.iter().map(Dimension::radix).collect());
        Ok(Fields {
            objects,
            dimensions,
            values,
        })
    }

    /// The state `digits` selects from the current membership.
    fn build(&self, digits: &[u128]) -> Result<ModelSimState, ExpansionStop> {
        let fields = self.current.as_ref().ok_or_else(broken)?;
        let mut populations = fields.objects.clone();
        for (dimension, digit) in fields.dimensions.iter().zip(digits) {
            let value = dimension.value(*digit).ok_or_else(broken)?;
            let object = populations
                .get_mut(&dimension.population)
                .and_then(|objects| objects.get_mut(&dimension.key))
                .ok_or_else(broken)?;
            if let Some(value) = value {
                object.fields.insert(dimension.field.clone(), value);
            }
        }
        Ok(self.system.pre_state(
            ModelState {
                populations,
                extents: Vec::new(),
            },
            self.anchor.clone(),
        ))
    }
}

fn broken() -> ExpansionStop {
    super::walk::stop(super::BROKEN)
}

/// One successor of one application at a pre-state (QSpec FR-485 "The
/// oracle": a behaviour of the contract relation).
#[derive(Clone, Debug)]
pub struct OperationStep {
    /// The FR-120 transition identity.
    pub transition: ModelTransition,
    /// The receiver.
    pub receiver: ObjectReference,
    /// The arguments, in parameter order.
    pub arguments: Vec<Value>,
    /// The result; `None` for an operation with no result.
    pub result: Option<Value>,
    /// The post-state.
    pub post: ModelSimState,
}

impl From<Applied> for OperationStep {
    fn from(applied: Applied) -> Self {
        Self {
            transition: applied.transition,
            receiver: applied.receiver,
            arguments: applied.arguments,
            result: applied.result,
            post: applied.state,
        }
    }
}

/// A `pre` or `post` clause that decided no Boolean at one application.
#[derive(Clone, Debug)]
pub struct UndecidedContract {
    /// The application's transition identity.
    pub transition: ModelTransition,
    /// The receiver.
    pub receiver: ObjectReference,
    /// The arguments, in parameter order.
    pub arguments: Vec<Value>,
    /// The clause's name.
    pub clause: String,
    /// Why it decided no Boolean.
    pub evaluation: ClauseOutcome,
    /// For a postcondition, the candidate post-state and result it was
    /// evaluated at; `None` for a precondition.
    pub post: Option<(ModelState, Option<Value>)>,
}

/// What applying one operation at a pre-state gave.
#[derive(Clone, Debug, Default)]
pub struct OperationCases {
    /// Every successor, receivers in ascending (population, key) order.
    pub steps: Vec<OperationStep>,
    /// Every contract clause that decided no Boolean, in FR-120's sibling
    /// order.
    pub undecided: Vec<UndecidedContract>,
    /// The argument vectors whose selected preconditions held, enabled
    /// whether or not a candidate satisfied the postconditions.
    pub enabled: usize,
}

/// One argument vector of an operation and how its one precondition read
/// there (QSpec FR-485 "Clause").
#[derive(Clone, Debug)]
pub struct PreconditionCase {
    /// The application's transition identity.
    pub transition: ModelTransition,
    /// The receiver.
    pub receiver: ObjectReference,
    /// The arguments, in parameter order.
    pub arguments: Vec<Value>,
    /// The evaluation that decided no Boolean; `None` when the clause
    /// decided one.
    pub undecided: Option<ClauseOutcome>,
}

fn clause_of(info: &ClauseInfo, kind: ContractClauseKind) -> ContractClause {
    ContractClause {
        name: info.name.clone(),
        identity: info.identity,
        kind,
        context: info.context,
        operation: info.operation.clone(),
    }
}

impl<'a> ModelSystem<'a> {
    /// Every state clause this system evaluates, invariants first, each kind
    /// in ascending name order.
    pub fn contract_clauses(&self) -> Vec<ContractClause> {
        let clauses = &self.clauses;
        clauses
            .invariants
            .iter()
            .map(|info| clause_of(info, ContractClauseKind::Invariant))
            .chain(
                clauses
                    .preconditions
                    .iter()
                    .map(|info| clause_of(info, ContractClauseKind::Precondition)),
            )
            .chain(
                clauses
                    .postconditions
                    .iter()
                    .map(|info| clause_of(info, ContractClauseKind::Postcondition)),
            )
            .collect()
    }

    /// Whether the object type `object` conforms to `ancestor`.
    pub fn conforms(&self, object: EffectiveId, ancestor: EffectiveId) -> bool {
        self.types.conforms(object, ancestor)
    }

    /// Every operation an object of the type `context`, or of a type that
    /// conforms to it, applies: its declaring type and name, in ascending
    /// order, each once.
    pub fn context_operations(&self, context: EffectiveId) -> Vec<(EffectiveId, String)> {
        let mut operations: Vec<(EffectiveId, String)> = self
            .applications
            .iter()
            .filter(|(type_id, _)| self.types.conforms(**type_id, context))
            .flat_map(|(_, applications)| applications)
            .map(|application| (application.declaring, application.name.clone()))
            .collect();
        operations.sort();
        operations.dedup();
        operations
    }

    /// `state` as a pre-state anchored at `anchor`, reached by no step.
    pub fn pre_state(&self, state: ModelState, anchor: SelectedAnchor) -> ModelSimState {
        ModelSimState {
            state,
            anchor,
            reached_by: None,
        }
    }

    /// The anchor a declared pre-state carries: the first initial state's,
    /// else an unnamed initialization. The anchor is no part of a state's
    /// key.
    pub fn declared_anchor(&self) -> SelectedAnchor {
        self.initial.first().map_or_else(
            || SelectedAnchor {
                kind: AnchorKind::Initialization,
                name: String::new(),
            },
            |initial| initial.anchor.clone(),
        )
    }

    /// Every declared pre-state of this system over its universes (QSpec
    /// FR-485 "Common premises" premise 1).
    ///
    /// # Errors
    ///
    /// [`DeclaredStatesRefusal`] when the model has a relationship or a clock
    /// member, or a member type's field has no finite domain.
    pub fn declared_states(&self) -> Result<DeclaredStates<'_, 'a>, DeclaredStatesRefusal> {
        if self
            .initial
            .iter()
            .any(|initial| !initial.state.extents.is_empty())
        {
            return Err(DeclaredStatesRefusal::Relationship);
        }
        if !self.clock_counts.is_empty() || !self.clock_bits.is_empty() {
            return Err(DeclaredStatesRefusal::ClockMember);
        }
        let mut slots = Vec::new();
        for population in &self.populations {
            let Some(keys) = self.universes.get(&population.key) else {
                continue;
            };
            let types = self.instantiable(&population.member_types);
            for type_key in &types {
                for field in self.fields.get(type_key).into_iter().flatten() {
                    if super::domain::Domain::of(
                        &field.value_type,
                        &[],
                        self.types,
                        &self.record_orders,
                    )
                    .is_none()
                        && !matches!(field.value_type, quire_exact::ValueType::Reference(_))
                    {
                        return Err(DeclaredStatesRefusal::Field {
                            field: field.name.clone(),
                        });
                    }
                }
            }
            for key in keys {
                slots.push(Slot {
                    population: population.key.clone(),
                    key: key.clone(),
                    types: types.clone(),
                });
            }
        }
        let memberships = Odometer::new(
            slots
                .iter()
                .map(|slot| 1 + slot.types.len() as u128)
                .collect(),
        );
        Ok(DeclaredStates {
            system: self,
            anchor: self.declared_anchor(),
            slots,
            memberships,
            current: None,
        })
    }

    /// The non-abstract object types that conform to one of `members`, in
    /// ascending declaration-key order.
    fn instantiable(&self, members: &[DeclarationKey]) -> Vec<DeclarationKey> {
        let mut instantiable = Vec::new();
        for declaration in self.types.object_types() {
            let Some(key) = self.model.type_key(declaration.key()) else {
                continue;
            };
            if self.model.is_abstract(key) {
                continue;
            }
            let conforms = members.iter().any(|member| {
                self.model
                    .type_identity(member)
                    .is_some_and(|target| self.types.conforms(declaration.key(), target))
            });
            if conforms {
                instantiable.push(key.clone());
            }
        }
        instantiable.sort();
        instantiable.dedup();
        instantiable
    }

    /// FR-120 "Invariants": the invariant and unit-bound findings of
    /// `state`.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an evaluation stops.
    pub fn invariant_findings_at(
        &self,
        state: &ModelSimState,
    ) -> Result<Vec<ModelFinding>, ExpansionStop> {
        self.invariant_findings(state)
    }

    /// Apply the operation `name` declared by `declaring` at every object of
    /// `state` whose type conforms to `receiver`, under the `pre` and `post`
    /// clauses whose checked identity `select` admits.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the candidate bound runs out or an
    /// evaluation stops.
    pub fn operation_cases(
        &self,
        state: &ModelSimState,
        declaring: EffectiveId,
        name: &str,
        receiver: EffectiveId,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<OperationCases, ExpansionStop> {
        let operation = OperationRef {
            declaring,
            name,
            receiver,
        };
        let walk = walk_of(self, state, None)?;
        let mut budget = self.budget();
        let mut contract = Vec::new();
        let mut successors = Vec::new();
        let mut enabled = Vec::new();
        for row in &walk.rows {
            let Some(application) = self.application_at(row, &operation) else {
                continue;
            };
            let clauses = walk.selected(row, application, select);
            frames::apply(
                &walk,
                row,
                application,
                &clauses,
                &mut budget,
                frames::Outputs {
                    contract: &mut contract,
                    successors: &mut successors,
                    enabled: Some(&mut enabled),
                    enabled_transitions: None,
                },
            )?;
        }
        let undecided = contract
            .into_iter()
            .filter_map(|entry| {
                let ModelFinding::ContractUndetermined {
                    transition,
                    clause,
                    evaluation,
                    ..
                } = entry.finding
                else {
                    return None;
                };
                let case = *entry.case;
                Some(UndecidedContract {
                    transition,
                    receiver: case.receiver,
                    arguments: case.arguments,
                    clause,
                    evaluation,
                    post: case.post,
                })
            })
            .collect();
        Ok(OperationCases {
            steps: successors.into_iter().map(OperationStep::from).collect(),
            undecided,
            enabled: enabled.len(),
        })
    }

    /// QSpec FR-485 "Clause": the precondition whose checked identity is
    /// `clause`, evaluated alone at every application of its operation at
    /// `state`.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the candidate bound runs out or an
    /// evaluation stops.
    pub fn precondition_cases(
        &self,
        state: &ModelSimState,
        clause: NodeKey,
    ) -> Result<Vec<PreconditionCase>, ExpansionStop> {
        let Some(info) = self
            .clauses
            .preconditions
            .iter()
            .find(|info| info.identity == clause)
        else {
            return Ok(Vec::new());
        };
        let Some((declaring, name)) = &info.operation else {
            return Ok(Vec::new());
        };
        let operation = OperationRef {
            declaring: *declaring,
            name,
            receiver: info.context,
        };
        let walk = walk_of(self, state, None)?;
        let mut budget = self.budget();
        let mut cases = Vec::new();
        for row in &walk.rows {
            let Some(application) = self.application_at(row, &operation) else {
                continue;
            };
            if walk
                .applicable(std::slice::from_ref(info), row, *declaring, name)
                .is_empty()
            {
                continue;
            }
            for case in frames::precondition_cases(&walk, row, application, info, &mut budget)? {
                cases.push(PreconditionCase {
                    transition: case.transition,
                    receiver: row.reference.clone(),
                    arguments: case.arguments,
                    undecided: case.undecided,
                });
            }
        }
        Ok(cases)
    }

    /// The objects of `state`, in ascending (population, key) order.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an object is not one of this model's.
    pub fn objects_of(&self, state: &ModelSimState) -> Result<Vec<ObjectReference>, ExpansionStop> {
        Ok(rows_of(self, &state.state)?
            .into_iter()
            .map(|row| row.reference)
            .collect())
    }
}
