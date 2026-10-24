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

use std::collections::BTreeSet;

use qsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use qsl_semantics::check::{CheckedNode, FormulaNodeId, StatePredicate, TemporalClauseNode};
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, Incomplete, Meter, ObjectReference, ValueType};

use super::contracts::{verdict, Verdict};
use super::walk::{walk_of, Walk};
use super::{ClauseOutcome, ModelSimState, ModelSystem};
use crate::model_subject::{ModelInstance, ModelLetters};
use crate::temporal::AtomReading;
use crate::value::{evaluate_state_predicate, evaluate_state_predicate_at_absent_key};

/// Why a clause has no [`ClauseLetters`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LettersRefusal {
    /// The clause's `over` parameter is not an object reference, so the
    /// clause has no instance (FR-125 Instancing).
    #[error("the clause's `over` parameter is not an object reference")]
    OverNotReference,
}

/// The `holds` atoms of one checked temporal clause, read at the states of a
/// [`ModelSystem`].
///
/// This seam must not get a production caller of `check_model` before QSL-1039
/// lands: [`ModelLetters::atom`] has no channel for a refused or faulted
/// `holds` evaluation (for example an authored population bound exceeded,
/// FR-380, or a population a snapshot does not list complete), which
/// [`ClauseLetters`] can only turn into a panic. Every atom read also
/// observes its state afresh (the state's digest and observation are
/// recomputed per call) and charges the engine's meter only for the
/// evaluation itself, not the observation; QSL-1039 owns both.
///
pub struct ClauseLetters<'s, 'a> {
    system: &'s ModelSystem<'a>,
    node: &'s TemporalClauseNode,
    parameter: EffectiveId,
    overlapping: BTreeSet<DeclarationKey>,
}

impl<'a> ModelSystem<'a> {
    /// The [`ModelLetters`] of `node` over this system.
    ///
    /// The seam must not get a production caller of `check_model` before
    /// QSL-1039 lands; see [`ClauseLetters`].
    ///
    /// # Errors
    ///
    /// [`LettersRefusal::OverNotReference`] when the clause's `over`
    /// parameter is not an object reference. An emitted clause activates
    /// `on origin` (FR-337), so it has no `when` condition to read
    /// (Remaining work: QSL-898).
    pub fn clause_letters<'s>(
        &'s self,
        node: &'s TemporalClauseNode,
    ) -> Result<ClauseLetters<'s, 'a>, LettersRefusal> {
        let ValueType::Reference(parameter) = node.clause().over().1 else {
            return Err(LettersRefusal::OverNotReference);
        };
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

impl ClauseLetters<'_, '_> {
    /// The reference of the parameter's type under `instance`'s key, which no
    /// object of the state need hold.
    fn key_reference(&self, instance: &ModelInstance) -> ObjectReference {
        self.system
            .model
            .type_key(self.parameter)
            .and_then(|type_key| {
                self.system
                    .model
                    .object_reference(type_key, &instance.key)
                    .ok()
            })
            .unwrap_or_else(|| panic!("QSL-1039: the parameter's type is an object type"))
    }
}

impl ModelLetters for ClauseLetters<'_, '_> {
    /// # Panics
    ///
    /// When `atom` is no `holds` atom of the clause, when `state` is no
    /// state of the system, or when the evaluation faults, is refused or
    /// completes no Boolean: the seam has no channel for them (QSL-1039),
    /// and none is a reading.
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let (body, slots) = self.node.condition(atom).unwrap_or_else(|| {
            panic!("QSL-1039: the engine reads only the clause's `holds` atoms")
        });
        let walk = walk_of(self.system, state, None)
            .unwrap_or_else(|stop| panic!("QSL-1039: the state is observable: {stop:?}"));
        let condition = Condition { body, slots };
        let evaluation = match self.bound(&walk, instance) {
            Some(bound) => evaluate_state_predicate(
                self.system.package,
                &condition,
                &walk.observation.environment,
                Some(bound),
                meter,
            ),
            None => evaluate_state_predicate_at_absent_key(
                self.system.package,
                &condition,
                &walk.observation.environment,
                self.key_reference(instance),
                instance.population.node.clone(),
                meter,
            ),
        }
        .unwrap_or_else(|fault| panic!("QSL-1039: the evaluator does not fault: {fault:?}"));
        match verdict(&evaluation.outcome) {
            Verdict::True => Ok(AtomReading::True),
            Verdict::False => Ok(AtomReading::False),
            Verdict::Undecided(outcome) => {
                if let Some(cause) = outcome.undefined_cause() {
                    Ok(AtomReading::Undefined(cause))
                } else if let ClauseOutcome::Incomplete(incomplete) = outcome {
                    Err(incomplete)
                } else {
                    panic!(
                        "QSL-1039: the `holds` condition was refused or completed no Boolean: {outcome:?}"
                    )
                }
            }
        }
    }

    /// An emitted clause activates `on origin` (FR-337) and has no `when`
    /// condition, so it activates wherever it is read.
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
