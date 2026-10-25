// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-125 Instancing, FR-107: the production [`ModelLetters`] of a checked
//! temporal clause over a [`ModelSystem`].
//!
//! A `holds` atom is evaluated through the one clause evaluator (FR-107)
//! over the state's synthesized observation, as a state-graph predicate is
//! ([`super::state_predicate`]). The clause's `over` parameter is bound to
//! the state's own reference to the object under the instance's key, matched
//! by key across every population that overlaps the parameter's type, never
//! by comparing the whole reference.
//!
//! Where no such object conforms to the parameter's type, or none is held
//! under the key, the clause is evaluated with the parameter at that key: a
//! read through it is `absent-key` undefined, but its use as the reference
//! operand of a `lookup<T>(p, x)`, the guard, is not, so the guard is false
//! where `p` holds no conforming object under the key (FR-125 Instancing). An
//! atom that does not read the parameter is evaluated normally.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use qsl_semantics::check::{CheckedNode, FormulaNodeId, StatePredicate, TemporalClauseNode};
use qsl_semantics::model::observation::Observation;
use quire_checked::key::DeclarationKey;
use quire_eval::FamilyOutcome;
use quire_exact::{EffectiveId, Meter, ObjectReference, Value, ValueType};
use quire_semantic_value::catalog::InternalFault;

use super::contracts::{verdict, Verdict};
use super::walk::{rows_of, stop, Row};
use super::{state_document, ClauseOutcome, ModelSimState, ModelSystem, BROKEN};
use crate::model_subject::{AtomStop, ModelInstance, ModelLetters};
use crate::temporal::AtomReading;
use crate::value::{
    evaluate_activation_guard, evaluate_activation_guard_at_absent_key, evaluate_state_predicate,
    evaluate_state_predicate_at_absent_key,
};

/// Why a clause has no [`ClauseLetters`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LettersRefusal {
    /// The clause's `over` parameter is not an object reference, so the
    /// clause has no instance (FR-125 Instancing).
    #[error("the clause's `over` parameter is not an object reference")]
    OverNotReference,
    /// The `over` parameter's object type is declared by no model of the
    /// subject, so no reference to an object of it can be formed.
    #[error("the clause's `over` object type is declared by no model of the subject")]
    OverTypeUndeclared,
    /// A hyper clause's letters were given other than one system for each of
    /// the clause's trace variables (FR-487).
    #[error("a hyper clause needs one model system for each of its trace variables")]
    ComponentsMismatch,
}

/// The `holds` atoms of one checked temporal clause, read at the states of a
/// [`ModelSystem`].
///
/// A refused or faulted `holds` evaluation (for example an authored
/// population bound exceeded, FR-380, or a population a snapshot does not
/// list complete), a state that cannot be observed and a broken invariant are
/// each an [`AtomStop`] of [`ModelLetters::atom`], never a panic (QSL-1039).
///
/// The letters observe a state once, however many atoms and instances read
/// it: the state's digest keys the last observation, which is kept for the
/// reads that follow. The observation is not charged to the engine's meter,
/// as `state_predicates` does not charge it; charging it needs a ruling.
pub struct ClauseLetters<'s, 'a> {
    system: &'s ModelSystem<'a>,
    node: &'s TemporalClauseNode,
    parameter: EffectiveId,
    /// The declaration key of the parameter's object type.
    type_key: DeclarationKey,
    overlapping: BTreeSet<DeclarationKey>,
    /// The last state observed, by its digest.
    observed: RefCell<Option<(DigestRecord, Rc<Observed>)>>,
    /// How many states have been observed (not read from `observed`).
    observations: Cell<u64>,
}

/// One state as the letters read it: its synthesized observation and its
/// objects.
pub(super) struct Observed {
    pub(super) observation: Observation,
    pub(super) rows: Vec<Row>,
}

impl<'a> ModelSystem<'a> {
    /// The [`ModelLetters`] of `node` over this system.
    ///
    /// # Errors
    ///
    /// [`LettersRefusal::OverNotReference`] when the clause's `over`
    /// parameter is not an object reference, and
    /// [`LettersRefusal::OverTypeUndeclared`] when no model of the subject
    /// declares its type. A clause that activates `on each ... when (g)`
    /// reads `g` through [`ModelLetters::activation`] (FR-337, FR-125-AC-18).
    pub fn clause_letters<'s>(
        &'s self,
        node: &'s TemporalClauseNode,
    ) -> Result<ClauseLetters<'s, 'a>, LettersRefusal> {
        let ValueType::Reference(parameter) = node.clause().over().1 else {
            return Err(LettersRefusal::OverNotReference);
        };
        let type_key = self
            .model
            .type_key(parameter)
            .cloned()
            .ok_or(LettersRefusal::OverTypeUndeclared)?;
        Ok(ClauseLetters {
            system: self,
            node,
            parameter,
            type_key,
            overlapping: self
                .populations
                .iter()
                .filter(|population| self.overlaps(population, parameter))
                .map(|population| population.key.clone())
                .collect(),
            observed: RefCell::new(None),
            observations: Cell::new(0),
        })
    }
}

/// One `holds` condition as a predicate of the clause evaluator.
struct Condition<'n> {
    body: CheckedNode<'n>,
    slots: usize,
}

impl StatePredicate for Condition<'_> {
    fn body(&self) -> CheckedNode<'_> {
        self.body
    }

    fn slots(&self) -> usize {
        self.slots
    }
}

impl ClauseLetters<'_, '_> {
    /// The reference the `over` parameter reads as under `instance`'s key in
    /// the observed state: the first object of an overlapping population held
    /// under the key whose most-specific type conforms to the parameter's.
    fn bound(&self, rows: &[Row], instance: &ModelInstance) -> Option<ObjectReference> {
        rows.iter()
            .find(|row| {
                row.key == instance.key
                    && self.overlapping.contains(&row.population)
                    && self
                        .system
                        .types
                        .conforms(row.reference.object_type(), self.parameter)
            })
            .map(|row| row.reference.clone())
    }
}

impl ClauseLetters<'_, '_> {
    /// The reference of the parameter's type under `instance`'s key, which no
    /// object of the state need hold.
    ///
    /// # Errors
    ///
    /// A fault when the model refuses a reference under the key (an empty
    /// key), which the subject's admission refuses before it forms an
    /// instance.
    fn key_reference(&self, instance: &ModelInstance) -> Result<ObjectReference, AtomStop> {
        self.system
            .model
            .object_reference(&self.type_key, &instance.key)
            .map_err(|_| fault("instance-key-names-a-reference"))
    }

    /// How many states these letters have observed: a state read again, by
    /// another atom or instance, is not observed again.
    pub fn observations(&self) -> u64 {
        self.observations.get()
    }

    /// `state`, observed once however many reads follow.
    fn observe(&self, state: &ModelSimState) -> Result<Rc<Observed>, AtomStop> {
        let (_, digest) = self
            .system
            .state_key(state)
            .map_err(|_| AtomStop::Unobservable(stop(BROKEN)))?;
        let known = self
            .observed
            .borrow()
            .as_ref()
            .filter(|(known, _)| *known == digest)
            .map(|(_, observed)| Rc::clone(observed));
        match known {
            Some(observed) => Ok(observed),
            None => {
                let observation = self
                    .system
                    .model
                    .observation(&state.state, state_document(&digest))
                    .map_err(|_| AtomStop::Unobservable(stop(BROKEN)))?;
                let rows = rows_of(self.system, &state.state).map_err(AtomStop::Unobservable)?;
                let observed = Rc::new(Observed { observation, rows });
                self.observations
                    .set(self.observations.get().saturating_add(1));
                *self.observed.borrow_mut() = Some((digest, Rc::clone(&observed)));
                Ok(observed)
            }
        }
    }
}

/// A broken invariant of the letters.
fn fault(invariant: &'static str) -> AtomStop {
    AtomStop::Fault(InternalFault::new("model-letters", invariant))
}

/// The reading of a condition's clause `outcome`.
fn reading(outcome: &FamilyOutcome<Value>) -> Result<AtomReading, AtomStop> {
    match verdict(outcome) {
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

impl ModelLetters for ClauseLetters<'_, '_> {
    /// # Errors
    ///
    /// [`AtomStop::Incomplete`] when the meter runs out in the evaluation; [`AtomStop::Refused`] when the
    /// evaluation is refused or completes no Boolean;
    /// [`AtomStop::Unobservable`] when `state` is no state of the system; and
    /// [`AtomStop::Fault`] when `atom` is no `holds` atom of the clause,
    /// `instance`'s key names no reference, or the evaluator faults.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let (body, slots) = self
            .node
            .condition(atom)
            .ok_or_else(|| fault("engine-reads-only-holds-atoms"))?;
        let observed = self.observe(state)?;
        let condition = Condition { body, slots };
        let evaluation = match self.bound(&observed.rows, instance) {
            Some(bound) => evaluate_state_predicate(
                self.system.package,
                &condition,
                &observed.observation.environment,
                Some(bound),
                meter,
            ),
            None => evaluate_state_predicate_at_absent_key(
                self.system.package,
                &condition,
                &observed.observation.environment,
                self.key_reference(instance)?,
                instance.population.node.clone(),
                meter,
            ),
        }
        .map_err(AtomStop::Fault)?;
        reading(&evaluation.outcome)
    }

    /// The guard of the clause's `on each` activation (FR-337) for
    /// `instance` at `state`: the `over` parameter and the trigger binder are
    /// both the instance's object. A clause with no guard has no condition to
    /// read, and the engine does not ask.
    ///
    /// # Errors
    ///
    /// As [`Self::atom`]; [`AtomStop::Fault`] also when the clause has no
    /// guard.
    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let (body, slots) = self
            .node
            .guard()
            .ok_or_else(|| fault("engine-reads-the-guard-of-a-guarded-clause"))?;
        let observed = self.observe(state)?;
        let condition = Condition { body, slots };
        let evaluation = match self.bound(&observed.rows, instance) {
            Some(bound) => evaluate_activation_guard(
                self.system.package,
                &condition,
                &observed.observation.environment,
                bound,
                meter,
            ),
            None => evaluate_activation_guard_at_absent_key(
                self.system.package,
                &condition,
                &observed.observation.environment,
                self.key_reference(instance)?,
                instance.population.node.clone(),
                meter,
            ),
        }
        .map_err(AtomStop::Fault)?;
        reading(&evaluation.outcome)
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let (Some((guard, _)), Some((name, value_type))) =
            (self.node.guard(), self.node.clause().trigger())
        else {
            return Ok(BTreeSet::new());
        };
        self.system.guard_reads(
            self.node.clause().over(),
            &(name.to_owned(), value_type.clone()),
            guard,
        )
    }

    fn reads(&self, atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let (condition, _) = self
            .node
            .condition(atom)
            .ok_or(FootprintRefusal::Unresolved { what: "holds atom" })?;
        self.system.atom_reads(self.node.clause().over(), condition)
    }
}
