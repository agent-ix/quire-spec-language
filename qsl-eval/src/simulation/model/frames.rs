// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Post-states": the applications of one object's operations and the
//! candidate post-states inside each operation's frame, generated lazily.
//!
//! A candidate is one choice of three things, made in this order: which
//! objects the operation deletes, which unused universe keys it creates and
//! under which type, and a value for every field the frame lets it write. Each
//! choice is an odometer, so a candidate is built and dropped one at a time
//! and a product too large to hold is never materialized.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

use qsl_semantics::model::domain_package::OperationEffect;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{
    AdmissionUsage, AdmittedObservations, Observation, SelfKind,
};
use qsl_semantics::model::population::{decide_frame, FrameDecision, FrameObject, FrameStop};
use qsl_semantics::model::state::{ModelState, StateField, StateObject};
use quire_canonical::Limits as CanonicalLimits;
use quire_exact::{ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use super::contracts::{conjunction, Conjunction};
use super::domain::{Domain, Odometer};
use super::encode::{self, value_enc, values_equal};
use super::footprints::ReadCheck;
use super::walk::{stop, Applied, Budget, ContractFinding, Row, Selected, Walk};
use super::{
    handler_anchor, identity_limits, state_document, Application, ClauseInfo, FieldChange,
    ModelFinding, ModelSimState, StepEffect, BROKEN, EXHAUSTED,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::footprint::Footprint;
use crate::simulation::key;
use qsl_foundation::digest::DigestRecord;

/// The most values a result domain may hold and still be put in canonical
/// order; a larger one stops the expansion.
const RESULT_ORDER_LIMIT: u128 = 1 << 16;

/// The identity limit every canonical text of a step is written under.
fn text_limits() -> CanonicalLimits {
    CanonicalLimits::new(identity_limits().input_bytes)
}

/// `value`'s canonical text.
fn text_of(value: &Value) -> Result<String, ExpansionStop> {
    value_enc(value)
        .canonical_text(text_limits())
        .map_err(|_| stop(BROKEN))
}

/// The transition identity of `application` applied to `args` at `row`
/// (FR-181): the operation, the receiver apart from the arguments, and one
/// canonical text per declared parameter.
fn transition_of(
    row: &Row,
    application: &Application,
    args: &[(String, Value)],
) -> Result<ModelTransition, ExpansionStop> {
    let receiver = encode::reference_enc(&row.reference)
        .canonical_text(text_limits())
        .map_err(|_| stop(BROKEN))?;
    let arguments = args
        .iter()
        .map(|(_, value)| text_of(value))
        .collect::<Result<Vec<_>, _>>()?;
    ModelTransition::new(
        application.text.clone(),
        Some(receiver),
        arguments,
        text_limits(),
    )
    .map_err(|_| stop(BROKEN))
}

/// The parameter domains of `application` over the objects of `walk`'s
/// state, one per declared parameter.
fn argument_domains(walk: &Walk<'_, '_>, application: &Application) -> Result<Vec<Domain>, ExpansionStop> {
    let references = walk.references();
    application
        .declaration
        .parameters()
        .iter()
        .map(|(_, value_type)| Domain::of(value_type, &references, walk.system.types))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| stop(BROKEN))
}

/// The argument vector `digits` selects from `domains`, by parameter name.
fn arguments_of(
    application: &Application,
    domains: &[Domain],
    digits: &[u128],
) -> Result<Vec<(String, Value)>, ExpansionStop> {
    application
        .declaration
        .parameters()
        .iter()
        .zip(domains)
        .zip(digits)
        .map(|(((name, _), domain), index)| domain.nth(*index).map(|value| (name.clone(), value)))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| stop(BROKEN))
}

/// Apply `application` to `row` for every argument vector of its parameter
/// domains under the `pre` and `post` clauses `clauses` selects: check its
/// effective precondition, and for an enabled application generate its
/// successors. This is FR-120's one application rule: `ModelSystem` calls it
/// with every applicable clause, a protocol attempt with the clauses its
/// `contracts` select (ADR-027 TS-3).
///
/// # Errors
///
/// An [`ExpansionStop`] when the successor set cannot be known.
pub(super) fn apply(
    walk: &Walk<'_, '_>,
    row: &Row,
    application: &Application,
    clauses: &Selected<'_>,
    budget: &mut Budget,
    contract: &mut Vec<ContractFinding>,
    successors: &mut Vec<Applied>,
) -> Result<(), ExpansionStop> {
    let system = walk.system;
    let domains = argument_domains(walk, application)?;
    let preconditions = &clauses.preconditions;
    let postconditions = &clauses.postconditions;

    let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
    while let Some(digits) = arguments.next_digits() {
        let args = arguments_of(application, &domains, &digits)?;
        // Every argument vector counts, enabled or not: a false or undecided
        // precondition would otherwise let one expansion run unbounded.
        budget.charge()?;
        let transition = transition_of(row, application, &args)?;
        let transition_bytes =
            quire_canonical::to_vec(&transition, text_limits()).map_err(|_| stop(BROKEN))?;
        let footprint = walk
            .enforcement
            .map(|enforcement| {
                enforcement.footprint_of(system, application, &row.reference, &args, &transition)
            })
            .transpose()?;

        let check = walk
            .enforcement
            .zip(footprint.as_ref())
            .map(|(enforcement, footprint)| {
                ReadCheck::new(&system.model, enforcement, footprint, &transition)
            });
        let verdict = conjunction(walk, preconditions, |info| {
            let observations = pre_observations(walk, info, row, &args);
            match &check {
                Some(check) => check.attach(observations),
                None => observations,
            }
        })?;
        if let Some(check) = &check {
            check.verify()?;
        }
        match verdict {
            Conjunction::True => {}
            Conjunction::False => continue,
            Conjunction::Undecided(undecided) => {
                for (clause, evaluation) in undecided {
                    contract.push(ContractFinding {
                        transition: transition_bytes.clone(),
                        candidate: None,
                        clause: clause.clone(),
                        finding: ModelFinding::ContractUndetermined {
                            transition: transition.clone(),
                            candidate: None,
                            clause,
                            evaluation,
                        },
                    });
                }
                continue;
            }
        }

        let application_run = Run {
            walk,
            row,
            application,
            args: &args,
            transition: &transition,
            transition_bytes: &transition_bytes,
            postconditions,
            footprint: footprint.as_ref(),
        };
        application_run.candidates(budget, contract, successors)?;
    }
    Ok(())
}

/// ADR-027 PD-4: the transition identity of the first argument vector of
/// `application` at `row`, in canonical order, with the first of `clauses`'
/// preconditions, in clause order, that is not true there (`None` when
/// every one is true). `None` for an application with no argument vector.
///
/// # Errors
///
/// An [`ExpansionStop`] when a precondition's evaluation stops.
pub(super) fn first_precondition(
    walk: &Walk<'_, '_>,
    row: &Row,
    application: &Application,
    clauses: &Selected<'_>,
) -> Result<Option<(ModelTransition, Option<String>)>, ExpansionStop> {
    let domains = argument_domains(walk, application)?;
    let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
    let Some(digits) = arguments.next_digits() else {
        return Ok(None);
    };
    let args = arguments_of(application, &domains, &digits)?;
    let transition = transition_of(row, application, &args)?;
    for clause in &clauses.preconditions {
        let verdict = conjunction(walk, &[*clause], |info| {
            pre_observations(walk, info, row, &args)
        })?;
        if !matches!(verdict, Conjunction::True) {
            return Ok(Some((transition, Some(clause.name.clone()))));
        }
    }
    Ok(Some((transition, None)))
}

/// The observations of a precondition: the state as `pre`, the receiver as
/// `self` and the arguments by declared name (FR-120 "Frame and observation
/// seam").
fn pre_observations(
    walk: &Walk<'_, '_>,
    clause: &ClauseInfo,
    row: &Row,
    args: &[(String, Value)],
) -> AdmittedObservations {
    AdmittedObservations {
        usage: AdmissionUsage::default(),
        clause: clause.identity,
        current: None,
        pre: Some(walk.observation.clone()),
        post: None,
        self_object: row.reference.clone(),
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: args.to_vec(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

/// One enabled application and what its candidates need.
struct Run<'w, 's, 'a> {
    walk: &'w Walk<'s, 'a>,
    row: &'w Row,
    application: &'w Application,
    args: &'w [(String, Value)],
    transition: &'w ModelTransition,
    transition_bytes: &'w [u8],
    postconditions: &'w [&'w ClauseInfo],
    /// The footprint the clauses are held to, when the expansion enforces
    /// footprints.
    footprint: Option<&'w Footprint>,
}

/// What one application's candidates share: the keys it may create objects
/// under and the fields it may write.
struct Plan<'p> {
    slots: &'p [Slot],
    writable: &'p WritableFields,
}

/// An unused universe key an operation may create an object under.
struct Slot {
    population: DeclarationKey,
    key: String,
    types: Vec<DeclarationKey>,
}

/// One object of a candidate's object set.
struct Member {
    population: DeclarationKey,
    key: String,
    type_key: DeclarationKey,
    reference: ObjectReference,
    created: bool,
}

/// One field a candidate chooses a value for.
struct Dimension<'f> {
    population: DeclarationKey,
    key: String,
    field: &'f StateField,
    domain: Domain,
}

impl Dimension<'_> {
    fn radix(&self) -> u128 {
        self.domain
            .len()
            .saturating_add(u128::from(self.field.optional))
    }

    /// The value at `index`: `Some(None)` for an absent `?` field.
    fn value(&self, index: u128) -> Option<Option<Value>> {
        if self.field.optional {
            match index.checked_sub(1) {
                None => Some(None),
                Some(position) => self.domain.nth(position).map(Some),
            }
        } else {
            self.domain.nth(index).map(Some)
        }
    }
}

/// A refusal by the frame decision.
struct FrameRefused;

/// The (population, object key, field) triples a frame lets an application
/// write.
type WritableFields = BTreeSet<(DeclarationKey, String, DeclarationKey)>;

/// What the frame decision found: the created and deleted objects, by
/// population and key.
struct Delta {
    created: Vec<(DeclarationKey, String)>,
    deleted: Vec<(DeclarationKey, String)>,
}

impl Run<'_, '_, '_> {
    fn effect(&self) -> &OperationEffect {
        self.application.declaration.effect()
    }

    /// The objects the frame lets this application delete: a type-wide
    /// `deletes` entry, `deletes self` for the receiver, and `deletes p` for
    /// the argument bound to `p`.
    fn deletable(&self) -> Vec<&Row> {
        let system = self.walk.system;
        let effect = self.effect();
        self.walk
            .rows
            .iter()
            .filter(|candidate| {
                let by_type = effect.deletes.iter().any(|grant| {
                    system
                        .model
                        .type_identity(grant)
                        .is_some_and(|target| system.types.conforms(candidate.type_id(), target))
                });
                let is_receiver = candidate.reference == self.row.reference;
                let by_argument = self.args.iter().any(|(name, value)| {
                    effect.deletes_arguments.contains(name)
                        && matches!(value, Value::Reference(bound) if *bound == candidate.reference)
                });
                by_type || (effect.deletes_receiver && is_receiver) || by_argument
            })
            .collect()
    }

    /// The unused universe keys this application may create objects under,
    /// each with the non-abstract types a created object may have: a type
    /// that conforms to a `creates` entry and to a member type of the
    /// population (FR-120 "Post-states").
    fn slots(&self) -> Vec<Slot> {
        let system = self.walk.system;
        let creatable = system.creatable_types(self.effect());
        let mut slots = Vec::new();
        for (population, keys) in &system.universes {
            let members: Vec<_> = system
                .populations
                .iter()
                .filter(|declared| declared.key == *population)
                .flat_map(|declared| declared.member_types.iter())
                .filter_map(|member| system.model.type_identity(member))
                .collect();
            let types: Vec<DeclarationKey> = creatable
                .iter()
                .filter(|candidate| {
                    system
                        .model
                        .type_identity(candidate)
                        .is_some_and(|identity| {
                            members
                                .iter()
                                .any(|member| system.types.conforms(identity, *member))
                        })
                })
                .cloned()
                .collect();
            if types.is_empty() {
                continue;
            }
            let used = self.walk.state.state.populations.get(population);
            for key in keys {
                if used.is_some_and(|objects| objects.contains_key(key)) {
                    continue;
                }
                slots.push(Slot {
                    population: population.clone(),
                    key: key.clone(),
                    types: types.clone(),
                });
            }
        }
        slots
    }

    /// Generate every candidate post-state of this application, one at a
    /// time, in canonical order, counting each against the bound.
    fn candidates(
        &self,
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
    ) -> Result<(), ExpansionStop> {
        let deletable = self.deletable();
        let slots = self.slots();
        let writable = self.writable_fields()?;
        let plan = Plan {
            slots: &slots,
            writable: &writable,
        };
        let mut deletions = Odometer::new(vec![2; deletable.len()]);
        while let Some(deleted) = deletions.next_digits() {
            let deleted: BTreeSet<(&DeclarationKey, &str)> = deletable
                .iter()
                .zip(&deleted)
                .filter(|(_, chosen)| **chosen == 1)
                .map(|(row, _)| (&row.population, row.key.as_str()))
                .collect();
            let mut creations = Odometer::new(
                slots
                    .iter()
                    .map(|slot| 1 + slot.types.len() as u128)
                    .collect(),
            );
            while let Some(created) = creations.next_digits() {
                self.fields_of_choice(&deleted, &plan, &created, budget, contract, successors)?;
            }
        }
        Ok(())
    }

    /// The post-state object set for one deletion and creation choice, and
    /// every assignment of values to the fields the frame lets the operation
    /// write.
    fn fields_of_choice(
        &self,
        deleted: &BTreeSet<(&DeclarationKey, &str)>,
        plan: &Plan<'_>,
        created: &[u128],
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
    ) -> Result<(), ExpansionStop> {
        let system = self.walk.system;
        let mut members: Vec<Member> = Vec::new();
        for row in &self.walk.rows {
            if deleted.contains(&(&row.population, row.key.as_str())) {
                continue;
            }
            let type_key = system
                .model
                .type_key(row.type_id())
                .cloned()
                .ok_or_else(|| stop(BROKEN))?;
            members.push(Member {
                population: row.population.clone(),
                key: row.key.clone(),
                type_key,
                reference: row.reference.clone(),
                created: false,
            });
        }
        for (slot, choice) in plan.slots.iter().zip(created) {
            let Some(position) = choice.checked_sub(1) else {
                continue;
            };
            let type_key = usize::try_from(position)
                .ok()
                .and_then(|position| slot.types.get(position))
                .cloned()
                .ok_or_else(|| stop(BROKEN))?;
            let reference = system
                .model
                .object_reference(&type_key, &slot.key)
                .map_err(|_| stop(BROKEN))?;
            members.push(Member {
                population: slot.population.clone(),
                key: slot.key.clone(),
                type_key,
                reference,
                created: true,
            });
        }
        members.sort_by(|left, right| {
            (&left.population, &left.key).cmp(&(&right.population, &right.key))
        });
        let post_references: Vec<ObjectReference> = members
            .iter()
            .map(|member| member.reference.clone())
            .collect();

        let mut dimensions = Vec::new();
        for member in &members {
            let Some(fields) = system.fields.get(&member.type_key) else {
                return Err(stop(BROKEN));
            };
            for field in fields {
                if !member.created
                    && !plan.writable.contains(&(
                        member.population.clone(),
                        member.key.clone(),
                        field.key.clone(),
                    ))
                {
                    continue;
                }
                let domain = Domain::of(&field.value_type, &post_references, system.types)
                    .ok_or_else(|| stop(BROKEN))?;
                dimensions.push(Dimension {
                    population: member.population.clone(),
                    key: member.key.clone(),
                    field,
                    domain,
                });
            }
        }

        let mut values = Odometer::new(dimensions.iter().map(Dimension::radix).collect());
        while let Some(digits) = values.next_digits() {
            budget.charge()?;
            let candidate =
                self.build(deleted, &members, plan.slots, created, &dimensions, &digits)?;
            if !is_closed(&candidate, &post_references) {
                continue;
            }
            let Ok(delta) = self.decide(&candidate)? else {
                // Candidates are built inside the frame, so a refusal here is
                // a broken internal invariant.
                return Err(stop(BROKEN));
            };
            self.complete(
                candidate,
                &delta,
                &post_references,
                budget,
                contract,
                successors,
            )?;
        }
        Ok(())
    }

    /// The candidate state one choice describes.
    fn build(
        &self,
        deleted: &BTreeSet<(&DeclarationKey, &str)>,
        members: &[Member],
        slots: &[Slot],
        created: &[u128],
        dimensions: &[Dimension<'_>],
        digits: &[u128],
    ) -> Result<ModelState, ExpansionStop> {
        let state = &self.walk.state.state;
        let mut populations = state.populations.clone();
        for (population, key) in deleted {
            if let Some(objects) = populations.get_mut(*population) {
                objects.remove(*key);
            }
        }
        for (slot, choice) in slots.iter().zip(created) {
            if *choice == 0 {
                continue;
            }
            let member = members
                .iter()
                .find(|member| {
                    member.created && member.population == slot.population && member.key == slot.key
                })
                .ok_or_else(|| stop(BROKEN))?;
            populations
                .entry(slot.population.clone())
                .or_default()
                .insert(
                    slot.key.clone(),
                    StateObject {
                        type_identity: member.type_key.clone(),
                        fields: BTreeMap::new(),
                    },
                );
        }
        for (dimension, digit) in dimensions.iter().zip(digits) {
            let value = dimension.value(*digit).ok_or_else(|| stop(BROKEN))?;
            let object = populations
                .get_mut(&dimension.population)
                .and_then(|objects| objects.get_mut(&dimension.key))
                .ok_or_else(|| stop(BROKEN))?;
            match value {
                Some(value) => {
                    object.fields.insert(dimension.field.key.clone(), value);
                }
                None => {
                    object.fields.remove(&dimension.field.key);
                }
            }
        }
        let mut extents = state.extents.clone();
        let removed: Vec<&ObjectReference> = self
            .walk
            .rows
            .iter()
            .filter(|row| deleted.contains(&(&row.population, row.key.as_str())))
            .map(|row| &row.reference)
            .collect();
        for extent in &mut extents {
            extent
                .instances
                .retain(|(left, right)| !removed.contains(&left) && !removed.contains(&right));
        }
        Ok(ModelState {
            populations,
            extents,
        })
    }

    /// The frame decision for `candidate` against the state, per population
    /// in ascending order (FR-120 "Frame and observation seam").
    fn decide(&self, candidate: &ModelState) -> Result<Result<Delta, FrameRefused>, ExpansionStop> {
        let system = self.walk.system;
        let pre = &self.walk.state.state;
        let empty = BTreeMap::new();
        let populations: BTreeSet<&DeclarationKey> = pre
            .populations
            .keys()
            .chain(candidate.populations.keys())
            .collect();
        let mut delta = Delta {
            created: Vec::new(),
            deleted: Vec::new(),
        };
        for population in populations {
            let before = pre.populations.get(population).unwrap_or(&empty);
            let after = candidate.populations.get(population).unwrap_or(&empty);
            let created: Vec<String> = after
                .keys()
                .filter(|key| !before.contains_key(*key))
                .cloned()
                .collect();
            let deleted: Vec<String> = before
                .keys()
                .filter(|key| !after.contains_key(*key))
                .cloned()
                .collect();
            let arguments: Vec<(String, String)> = self
                .args
                .iter()
                .filter_map(|(name, value)| match value {
                    Value::Reference(bound)
                        if before.get(bound.object().as_str()).is_some_and(|object| {
                            system.model.type_identity(&object.type_identity)
                                == Some(bound.object_type())
                        }) =>
                    {
                        Some((name.clone(), bound.object().as_str().to_owned()))
                    }
                    _ => None,
                })
                .collect();
            let index = system
                .model
                .population_index(population)
                .ok_or_else(|| stop(BROKEN))?;
            let frame = FrameDecision {
                index,
                ancestor_steps: system.model.ancestor_steps(),
                effect: self.effect(),
                declared_created: &created,
                declared_deleted: &deleted,
                receiver: (self.row.population == *population).then_some(self.row.key.as_str()),
                arguments: &arguments,
            };
            let outcome = decide_frame(
                &frame,
                &frame_objects(before),
                &frame_objects(after),
                |_, left, right| match (left, right) {
                    (None, None) => true,
                    (Some(left), Some(right)) => values_equal(left, right),
                    (Some(_), None) | (None, Some(_)) => false,
                },
                &mut |_| Ok::<(), Infallible>(()),
            );
            match outcome {
                Err(FrameStop::Refused(_)) => return Ok(Err(FrameRefused)),
                Err(FrameStop::Denied(never)) => match never {},
                Ok(found) => {
                    delta.created.extend(
                        found
                            .created
                            .into_iter()
                            .map(|key| (population.clone(), key)),
                    );
                    delta.deleted.extend(
                        found
                            .deleted
                            .into_iter()
                            .map(|key| (population.clone(), key)),
                    );
                }
            }
        }
        Ok(Ok(delta))
    }

    /// Every (population, object, field) the frame lets this application
    /// write, decided by [`decide_frame`] alone: a probe state changes that
    /// one field, and the frame decision accepts or refuses it.
    fn writable_fields(&self) -> Result<WritableFields, ExpansionStop> {
        let system = self.walk.system;
        let pre = &self.walk.state.state;
        let references = self.walk.references();
        let mut writable = BTreeSet::new();
        for row in &self.walk.rows {
            let type_key = system
                .model
                .type_key(row.type_id())
                .ok_or_else(|| stop(BROKEN))?;
            let fields = system.fields.get(type_key).ok_or_else(|| stop(BROKEN))?;
            for field in fields {
                let current = pre
                    .populations
                    .get(&row.population)
                    .and_then(|objects| objects.get(&row.key))
                    .and_then(|object| object.fields.get(&field.key));
                // A field with no finite domain (an `Integer`, a string, an
                // unbounded collection) is bounded by no frame that grants it,
                // so it cannot be written here.
                let Some(domain) = Domain::of(&field.value_type, &references, system.types) else {
                    continue;
                };
                let mut different: Option<Option<Value>> = None;
                if field.optional && current.is_some() {
                    different = Some(None);
                } else {
                    for index in 0..domain.len().min(8) {
                        let value = domain.nth(index).ok_or_else(|| stop(BROKEN))?;
                        if current.is_none_or(|held| !values_equal(held, &value)) {
                            different = Some(Some(value));
                            break;
                        }
                    }
                }
                // A field whose domain holds no other value cannot change.
                let Some(different) = different else {
                    continue;
                };
                let mut probe = pre.clone();
                let object = probe
                    .populations
                    .get_mut(&row.population)
                    .and_then(|objects| objects.get_mut(&row.key))
                    .ok_or_else(|| stop(BROKEN))?;
                match different {
                    Some(value) => {
                        object.fields.insert(field.key.clone(), value);
                    }
                    None => {
                        object.fields.remove(&field.key);
                    }
                }
                if self.decide(&probe)?.is_ok() {
                    writable.insert((row.population.clone(), row.key.clone(), field.key.clone()));
                }
            }
        }
        Ok(writable)
    }

    /// The effect of the step to `candidate`, with `result` as its result.
    fn effect_of(
        &self,
        candidate: &ModelState,
        delta: &Delta,
    ) -> Result<StepEffect, ExpansionStop> {
        let system = self.walk.system;
        let pre = &self.walk.state.state;
        let reference = |state: &ModelState, population: &DeclarationKey, key: &str| {
            state
                .populations
                .get(population)
                .and_then(|objects| objects.get(key))
                .ok_or_else(|| stop(BROKEN))
                .and_then(|object| {
                    system
                        .model
                        .object_reference(&object.type_identity, key)
                        .map_err(|_| stop(BROKEN))
                })
        };
        let mut created = delta
            .created
            .iter()
            .map(|(population, key)| reference(candidate, population, key))
            .collect::<Result<Vec<_>, _>>()?;
        let mut deleted = delta
            .deleted
            .iter()
            .map(|(population, key)| reference(pre, population, key))
            .collect::<Result<Vec<_>, _>>()?;
        created.sort();
        deleted.sort();

        let mut changed: Vec<FieldChange> = Vec::new();
        for (population, objects) in &pre.populations {
            for (key, before) in objects {
                let Some(after) = candidate
                    .populations
                    .get(population)
                    .and_then(|survivors| survivors.get(key))
                else {
                    continue;
                };
                let fields = system
                    .fields
                    .get(&before.type_identity)
                    .ok_or_else(|| stop(BROKEN))?;
                for field in fields {
                    let (left, right) =
                        (before.fields.get(&field.key), after.fields.get(&field.key));
                    let same = match (left, right) {
                        (None, None) => true,
                        (Some(left), Some(right)) => values_equal(left, right),
                        (Some(_), None) | (None, Some(_)) => false,
                    };
                    if same {
                        continue;
                    }
                    changed.push(FieldChange {
                        object: reference(pre, population, key)?,
                        field: field.name.clone(),
                        pre: left.map(text_of).transpose()?,
                        post: right.map(text_of).transpose()?,
                    });
                }
            }
        }
        changed.sort_by(|left, right| left.object.cmp(&right.object));
        Ok(StepEffect {
            created,
            deleted,
            changed,
            result: None,
        })
    }

    /// The result domain over the post-state and its values' positions in
    /// ascending order of their typed canonical bytes; `None` for an
    /// operation with no result. Only the order is held, never the values:
    /// each pair is read lazily and counted when it is evaluated.
    ///
    /// A result domain with more than [`RESULT_ORDER_LIMIT`] values cannot be
    /// ordered and stops the expansion.
    fn result_order(
        &self,
        post_references: &[ObjectReference],
    ) -> Result<Option<(Domain, Vec<u128>)>, ExpansionStop> {
        let Some(result_type) = self.application.declaration.result() else {
            return Ok(None);
        };
        let domain = Domain::of(result_type, post_references, self.walk.system.types)
            .ok_or_else(|| stop(BROKEN))?;
        if domain.len() > RESULT_ORDER_LIMIT {
            return Err(stop(EXHAUSTED));
        }
        let mut keyed = Vec::new();
        for index in 0..domain.len() {
            let value = domain.nth(index).ok_or_else(|| stop(BROKEN))?;
            let bytes = quire_canonical::to_vec(&value_enc(&value), text_limits())
                .map_err(|_| stop(BROKEN))?;
            keyed.push((bytes, index));
        }
        keyed.sort();
        Ok(Some((
            domain,
            keyed.into_iter().map(|(_, index)| index).collect(),
        )))
    }

    /// Pair `candidate` with each result and keep it as a successor when some
    /// pair's effective postcondition is true.
    fn complete(
        &self,
        candidate: ModelState,
        delta: &Delta,
        post_references: &[ObjectReference],
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
    ) -> Result<(), ExpansionStop> {
        let system = self.walk.system;
        let mut effect = self.effect_of(&candidate, delta)?;
        let ordered = self.result_order(post_references)?;
        let picks: Vec<Option<u128>> = match &ordered {
            None => vec![None],
            Some((_, order)) => order.iter().copied().map(Some).collect(),
        };
        let mut evidence: Option<(Vec<u8>, DigestRecord, Observation)> = None;
        for pick in picks {
            let result = match (&ordered, pick) {
                (Some((domain, _)), Some(index)) => {
                    Some(domain.nth(index).ok_or_else(|| stop(BROKEN))?)
                }
                _ => None,
            };
            if self.application.declaration.result().is_some() {
                budget.charge()?;
            }
            let verdict =
                if self.postconditions.is_empty() {
                    Conjunction::True
                } else {
                    if evidence.is_none() {
                        let state_key = encode::model_key(&system.model, &candidate);
                        let (bytes, digest) = key::state_key(&state_key, identity_limits())
                            .map_err(|_| stop(BROKEN))?;
                        let observation = system
                            .model
                            .observation(&candidate, state_document(&digest))
                            .map_err(|_| stop(BROKEN))?;
                        evidence = Some((bytes.as_bytes().to_vec(), digest, observation));
                    }
                    let (_, _, post) = evidence.as_ref().ok_or_else(|| stop(BROKEN))?;
                    let check = self.walk.enforcement.zip(self.footprint).map(
                        |(enforcement, footprint)| {
                            ReadCheck::new(&system.model, enforcement, footprint, self.transition)
                        },
                    );
                    let verdict = conjunction(self.walk, self.postconditions, |info| {
                        let observations = AdmittedObservations {
                            usage: AdmissionUsage::default(),
                            clause: info.identity,
                            current: None,
                            pre: Some(self.walk.observation.clone()),
                            post: Some(post.clone()),
                            self_object: self.row.reference.clone(),
                            invocation: None,
                            self_kind: SelfKind::Object,
                            receiver_pre: None,
                            receiver_post: None,
                            parameters: self.args.to_vec(),
                            result: result.clone(),
                            created: effect.created.clone(),
                            deleted: effect.deleted.clone(),
                        };
                        match &check {
                            Some(check) => check.attach(observations),
                            None => observations,
                        }
                    })?;
                    if let Some(check) = &check {
                        check.verify()?;
                    }
                    verdict
                };
            match verdict {
                Conjunction::True => {
                    effect.result = result.as_ref().map(text_of).transpose()?;
                    successors.push(Applied {
                        transition: self.transition.clone(),
                        state: ModelSimState {
                            state: candidate,
                            anchor: handler_anchor(&self.application.text),
                            reached_by: Some(effect),
                        },
                        receiver: self.row.reference.clone(),
                        arguments: self.args.iter().map(|(_, value)| value.clone()).collect(),
                        result,
                    });
                    return Ok(());
                }
                Conjunction::False => {}
                Conjunction::Undecided(undecided) => {
                    let (bytes, digest, _) = evidence.as_ref().ok_or_else(|| stop(BROKEN))?;
                    for (clause, evaluation) in undecided {
                        contract.push(ContractFinding {
                            transition: self.transition_bytes.to_vec(),
                            candidate: Some(bytes.clone()),
                            clause: clause.clone(),
                            finding: ModelFinding::ContractUndetermined {
                                transition: self.transition.clone(),
                                candidate: Some(*digest),
                                clause,
                                evaluation,
                            },
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

/// `objects` as the frame decision reads them.
fn frame_objects(objects: &BTreeMap<String, StateObject>) -> Vec<FrameObject<'_, &Value>> {
    objects
        .iter()
        .map(|(key, object)| FrameObject {
            object: key.as_str(),
            type_identity: object.type_identity.clone(),
            fields: object
                .fields
                .iter()
                .map(|(field, value)| (field.clone(), value))
                .collect(),
        })
        .collect()
}

/// Whether every reference of `candidate` names one of `references`.
fn is_closed(candidate: &ModelState, references: &[ObjectReference]) -> bool {
    let present: BTreeSet<&ObjectReference> = references.iter().collect();
    let mut named = Vec::new();
    for object in candidate.populations.values().flat_map(BTreeMap::values) {
        for value in object.fields.values() {
            collect_references(value, &mut named);
        }
    }
    named
        .into_iter()
        .all(|reference| present.contains(reference))
}

/// Every object reference `value` holds.
fn collect_references<'v>(value: &'v Value, out: &mut Vec<&'v ObjectReference>) {
    match value {
        Value::Reference(reference) => out.push(reference),
        Value::Option(option) => {
            if let Some(payload) = option.payload() {
                collect_references(payload, out);
            }
        }
        Value::Collection(collection) => {
            for element in collection.elements() {
                collect_references(element, out);
            }
        }
        Value::Boolean(_)
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
        | Value::Population(_)
        | Value::Composite(_)
        | Value::Union(_) => {}
    }
}
