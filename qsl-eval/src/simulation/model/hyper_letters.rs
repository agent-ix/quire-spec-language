// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-487 "The clause seams": the production [`HyperLetters`] of a checked
//! hyper clause over the [`ModelSystem`] of each of its trace variables.
//!
//! A `holds` atom is evaluated through the one clause evaluator (FR-107)
//! ([`CheckedPackageEvaluation::evaluate_hyper_holds`]) over the synthesized
//! observation of each component's state, each model read in the component
//! the clause's S3 check recorded for it (ADR-023 HS-4). Slot (`t`, `p`) is
//! bound as [`super::ClauseLetters`] binds a temporal clause's `over`
//! parameter, in the component `t`: to the reference of the object held under
//! `p`'s instance key, or, where component `t`'s state holds none, to the
//! absent-key reading of FR-125 Instancing. Each slot is read independently.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedHyperClause, FormulaNodeId};
use quire_checked::key::DeclarationKey;
use quire_eval::CallFailure;
use quire_exact::{Meter, ObjectReference, Value};
use quire_semantic_value::catalog::InternalFault;

use super::contracts::{verdict, Verdict};
use super::letters::{LettersRefusal, Observed};
use super::walk::{rows_of, stop};
use super::{state_document, ClauseOutcome, ModelSimState, ModelSystem, BROKEN};
use crate::hyper::{HyperLetters, HyperSlot, ParameterBinding};
use crate::model_subject::{AtomStop, ModelInstance};
use crate::simulation::StateKey;
use crate::temporal::AtomReading;
use crate::value::CheckedPackageEvaluation;

/// A component's observation of the state it was last read at.
type Cached = Option<(StateKey, Rc<Observed>)>;

/// The `holds` atoms of one checked hyper clause, read at the joint positions
/// of the [`ModelSystem`]s of its trace variables.
///
/// A refused or faulted `holds` evaluation, a state that cannot be observed
/// and a broken invariant are each an [`AtomStop`] of [`HyperLetters::atom`],
/// never a panic. Each component state is observed once however many atoms
/// and instances read it: the state's key selects the last observation of the
/// component, which is kept for the reads that follow. The observation is not
/// charged to the engine's meter.
pub struct CheckedHyperLetters<'s, 'a> {
    package: &'s CheckedPackage,
    clause: &'s CheckedHyperClause,
    /// One system per trace variable, in quantifier order.
    systems: Vec<&'s ModelSystem<'a>>,
    /// For each atom slot, the populations of its component's system that
    /// overlap the type of its object parameter.
    overlapping: Vec<BTreeSet<DeclarationKey>>,
    /// The last state of each component observed, by its canonical key.
    observed: Vec<RefCell<Cached>>,
}

impl<'s, 'a> CheckedHyperLetters<'s, 'a> {
    /// The letters of `clause`, evaluated through `package`, over `systems`,
    /// one per trace variable in quantifier order (two variables of one alias
    /// may share a system).
    ///
    /// # Errors
    ///
    /// [`LettersRefusal::ComponentsMismatch`] when `systems` holds other than
    /// one system per trace variable, and
    /// [`LettersRefusal::OverTypeUndeclared`] when the model of a slot's
    /// component declares no type for the slot's object parameter.
    pub fn new(
        package: &'s CheckedPackage,
        clause: &'s CheckedHyperClause,
        systems: Vec<&'s ModelSystem<'a>>,
    ) -> Result<Self, LettersRefusal> {
        if systems.len() != clause.quantifiers().len() {
            return Err(LettersRefusal::ComponentsMismatch);
        }
        let mut overlapping = Vec::new();
        for slot in clause.atom_slots() {
            let object = parameter_type(clause, slot.parameter)?;
            let system = systems
                .get(slot.trace)
                .ok_or(LettersRefusal::ComponentsMismatch)?;
            system
                .model
                .type_key(object.clone())
                .ok_or(LettersRefusal::OverTypeUndeclared)?;
            overlapping.push(
                system
                    .populations
                    .iter()
                    .filter(|population| system.overlaps(population, object.clone()))
                    .map(|population| population.key.clone())
                    .collect(),
            );
        }
        Ok(Self {
            package,
            clause,
            observed: systems.iter().map(|_| RefCell::new(None)).collect(),
            systems,
            overlapping,
        })
    }

    /// `state`, the state of component `component`, observed once however
    /// many reads follow.
    fn observe(&self, component: usize, state: &ModelSimState) -> Result<Rc<Observed>, AtomStop> {
        let system = self
            .systems
            .get(component)
            .ok_or_else(|| fault("a-component-has-a-system"))?;
        let cache = self
            .observed
            .get(component)
            .ok_or_else(|| fault("a-component-has-a-system"))?;
        let key = system
            .canonical_state_key(state)
            .map_err(|_| AtomStop::Unobservable(stop(BROKEN)))?;
        let known = cache
            .borrow()
            .as_ref()
            .filter(|(known, _)| *known == key)
            .map(|(_, observed)| Rc::clone(observed));
        if let Some(observed) = known {
            return Ok(observed);
        }
        let observation = system
            .model
            .observation(&state.state, state_document())
            .map_err(|_| AtomStop::Unobservable(stop(BROKEN)))?;
        let rows = rows_of(system, &state.state).map_err(AtomStop::Unobservable)?;
        let observed = Rc::new(Observed { observation, rows });
        *cache.borrow_mut() = Some((key, Rc::clone(&observed)));
        Ok(observed)
    }

    /// The binding of the atom slot `index` under `instance`'s key in the
    /// `observed` state of its component.
    fn slot(
        &self,
        index: usize,
        observed: &Observed,
        instance: &ModelInstance,
    ) -> Result<HyperSlot, AtomStop> {
        let slot = self
            .clause
            .atom_slots()
            .get(index)
            .ok_or_else(|| fault("a-slot-is-a-slot-of-the-clause"))?;
        let system = self
            .systems
            .get(slot.trace)
            .ok_or_else(|| fault("a-component-has-a-system"))?;
        let object = parameter_type(self.clause, slot.parameter)
            .map_err(|_| fault("a-parameter-is-a-parameter-of-the-clause"))?;
        let overlapping = self
            .overlapping
            .get(index)
            .ok_or_else(|| fault("a-slot-has-overlapping-populations"))?;
        let bound = observed
            .rows
            .iter()
            .find(|row| {
                row.key == instance.key
                    && overlapping.contains(&row.population)
                    && system
                        .types
                        .conforms(row.reference.object_type().clone(), object.clone())
            })
            .map(|row| row.reference.clone());
        match bound {
            Some(reference) => Ok(HyperSlot::Bound(Value::Reference(reference))),
            None => {
                let type_key = system
                    .model
                    .type_key(object)
                    .ok_or_else(|| fault("a-parameter-type-is-declared"))?;
                let key: ObjectReference = system
                    .model
                    .object_reference(type_key, &instance.key)
                    .map_err(|_| fault("instance-key-names-a-reference"))?;
                Ok(HyperSlot::Absent {
                    key,
                    population: instance.population.node.clone(),
                })
            }
        }
    }
}

/// The object type that the parameter `index` of `clause` declares.
fn parameter_type(
    clause: &CheckedHyperClause,
    index: usize,
) -> Result<quire_exact::EffectiveId, LettersRefusal> {
    clause
        .parameters()
        .get(index)
        .map(|parameter| parameter.population.object())
        .ok_or(LettersRefusal::OverTypeUndeclared)
}

/// A broken invariant of the letters.
fn fault(invariant: &'static str) -> AtomStop {
    AtomStop::Fault(InternalFault::new("hyper-letters", invariant))
}

impl HyperLetters for CheckedHyperLetters<'_, '_> {
    /// # Errors
    ///
    /// [`AtomStop::Incomplete`] when the meter runs out in the evaluation;
    /// [`AtomStop::Refused`] when the evaluation is refused or completes no
    /// Boolean; [`AtomStop::Unobservable`] when a component state is no state
    /// of its system; and [`AtomStop::Fault`] when `atom` is no `holds` atom
    /// of the clause, `states` or `instance` do not fit the clause, an
    /// instance key names no reference, or the evaluator faults.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if states.len() != self.systems.len() {
            return Err(fault("one-state-for-each-trace-variable"));
        }
        let observed = states
            .iter()
            .enumerate()
            .map(|(component, state)| self.observe(component, state))
            .collect::<Result<Vec<_>, _>>()?;
        let mut slots = Vec::with_capacity(self.clause.atom_slots().len());
        for (index, slot) in self.clause.atom_slots().iter().enumerate() {
            let parameter = self
                .clause
                .parameters()
                .get(slot.parameter)
                .ok_or_else(|| fault("a-parameter-is-a-parameter-of-the-clause"))?;
            let binding = instance
                .iter()
                .find(|binding| binding.parameter == parameter.name)
                .ok_or_else(|| fault("the-instance-binds-every-parameter"))?;
            let component = observed
                .get(slot.trace)
                .ok_or_else(|| fault("a-component-has-a-state"))?;
            slots.push(self.slot(index, component, &binding.instance)?);
        }
        let environments: Vec<_> = observed
            .iter()
            .map(|observed| &observed.observation.environment)
            .collect();
        let evaluation = self
            .package
            .evaluate_hyper_holds(self.clause, atom, slots, &environments, meter)
            .map_err(|failure| match failure {
                CallFailure::Fault(internal) => AtomStop::Fault(internal),
                _ => fault("the-evaluation-is-admitted"),
            })?;
        match verdict(&evaluation.outcome) {
            Verdict::True => Ok(AtomReading::True),
            Verdict::False => Ok(AtomReading::False),
            Verdict::Undecided(outcome) => {
                if let Some(cause) = outcome.undefined_cause() {
                    Ok(AtomReading::Undefined(cause))
                } else if let ClauseOutcome::Incomplete(incomplete) = outcome {
                    Err(AtomStop::Incomplete(incomplete))
                } else {
                    Err(AtomStop::Refused(outcome))
                }
            }
        }
    }
}
