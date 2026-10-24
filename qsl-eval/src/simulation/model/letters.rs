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
//! under the key, every atom of the clause reads `absent-key` undefined, as
//! a state-graph predicate does. The guard exception of FR-125 (a
//! `lookup<T>(p, x)` that is absent rather than undefined at such a key)
//! is not read here: the evaluator gives a read through a reference to no
//! object no `absent-key` value to guard against.

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use qsl_semantics::check::{
    Activation, CheckedNode, FormulaNodeId, StatePredicate, TemporalClauseNode,
};
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, Incomplete, Meter, ObjectReference, ValueType};
use quire_semantic_value::catalog::{UndefinedReason, UndefinedRecord};

use super::contracts::{verdict, Verdict};
use super::walk::{walk_of, Walk};
use super::{ClauseOutcome, ModelSimState, ModelSystem};
use crate::model_subject::{ModelInstance, ModelLetters};
use crate::temporal::{AtomReading, UndefinedCause};
use crate::value::evaluate_state_predicate;

/// Why a clause has no [`ClauseLetters`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LettersRefusal {
    /// The clause's `over` parameter is not an object reference, so the
    /// clause has no instance (FR-125 Instancing).
    #[error("the clause's `over` parameter is not an object reference")]
    OverNotReference,
    /// The clause is `on each` with a `when` condition, which the checked
    /// clause does not keep for evaluation.
    #[error("the clause's `when` activation condition is not available to evaluate")]
    GuardedActivation,
}

/// The `holds` atoms of one checked temporal clause, read at the states of a
/// [`ModelSystem`].
pub struct ClauseLetters<'s, 'a> {
    system: &'s ModelSystem<'a>,
    node: &'s TemporalClauseNode,
    parameter: EffectiveId,
    overlapping: BTreeSet<DeclarationKey>,
}

impl<'a> ModelSystem<'a> {
    /// The [`ModelLetters`] of `node` over this system.
    ///
    /// # Errors
    ///
    /// [`LettersRefusal::OverNotReference`] when the clause's `over`
    /// parameter is not an object reference;
    /// [`LettersRefusal::GuardedActivation`] for an `on each` clause with a
    /// `when` condition.
    pub fn clause_letters<'s>(
        &'s self,
        node: &'s TemporalClauseNode,
    ) -> Result<ClauseLetters<'s, 'a>, LettersRefusal> {
        let ValueType::Reference(parameter) = node.clause().over().1 else {
            return Err(LettersRefusal::OverNotReference);
        };
        if node.clause().activation() == (Activation::Each { guarded: true }) {
            return Err(LettersRefusal::GuardedActivation);
        }
        Ok(ClauseLetters {
            system: self,
            node,
            parameter,
            overlapping: self
                .populations
                .iter()
                .filter(|population| self.overlaps(population, parameter))
                .map(|population| population.key.clone())
                .collect(),
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
    /// `walk`'s state: the first object of an overlapping population held
    /// under the key whose most-specific type conforms to the parameter's.
    fn bound(&self, walk: &Walk<'_, '_>, instance: &ModelInstance) -> Option<ObjectReference> {
        walk.rows
            .iter()
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

impl ModelLetters for ClauseLetters<'_, '_> {
    /// # Panics
    ///
    /// When `atom` is no `holds` atom of the clause, when `state` is no
    /// state of the system, or when the evaluation faults or is refused:
    /// the seam has no channel for them, and each is a defect, never a
    /// reading.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let (body, slots) = self
            .node
            .condition(atom)
            .unwrap_or_else(|| panic!("the engine reads only the clause's `holds` atoms"));
        let walk = walk_of(self.system, state, None)
            .unwrap_or_else(|stop| panic!("the state is observable: {stop:?}"));
        let Some(bound) = self.bound(&walk, instance) else {
            return Ok(AtomReading::Undefined(UndefinedCause::Family(
                UndefinedRecord {
                    reason: UndefinedReason::AbsentKey,
                    fields: BTreeMap::new(),
                },
            )));
        };
        let evaluation = evaluate_state_predicate(
            self.system.package,
            &Condition { body, slots },
            &walk.observation.environment,
            Some(bound),
            meter,
        )
        .unwrap_or_else(|fault| panic!("the evaluator does not fault: {fault:?}"));
        match verdict(&evaluation.outcome) {
            Verdict::True => Ok(AtomReading::True),
            Verdict::False => Ok(AtomReading::False),
            Verdict::Undecided(outcome) => {
                if let Some(cause) = outcome.undefined_cause() {
                    Ok(AtomReading::Undefined(cause))
                } else if let ClauseOutcome::Incomplete(incomplete) = outcome {
                    Err(incomplete)
                } else {
                    panic!("a `holds` condition is neither refused nor non-Boolean: {outcome:?}")
                }
            }
        }
    }

    /// An unguarded clause activates at every position it reads
    /// ([`ClauseLetters`] is never built for a guarded one).
    fn activation(
        &self,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        Ok(AtomReading::True)
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }

    fn reads(&self, atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let (condition, _) = self
            .node
            .condition(atom)
            .ok_or(FootprintRefusal::Unresolved { what: "holds atom" })?;
        self.system.atom_reads(self.node.clause().over(), condition)
    }
}
