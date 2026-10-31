// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 TS-6, PX-2: what the model checker, its closure checker and replay
//! read of a subject's transition system beside FR-101's
//! [`TransitionSystem`]. [`crate::simulation::model::ModelSystem`] and
//! [`crate::simulation::protocol::ProtocolSystem`] implement it, so one
//! engine checks a model subject and a protocol subject.

use std::borrow::Cow;

use crate::fairness::{Constraint, Identity, Strength};
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{EncodingRefusal, StateKey};
use crate::simulation::model::{ModelFinding, ModelSimState, ModelSystem, TerminalClass};
use crate::simulation::progress::ZeroProgressError;
use crate::simulation::protocol::{
    AccessRecord, BlockedThread, DataRace, ProtocolFairness, ProtocolFairnessError,
    ProtocolTransition,
};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity};
use quire_semantic_value::model_transition::ModelTransition;

/// What a method bound left of a state unexpanded: a store the memory bound
/// cut (ADR-025 MB-3), or an `activate` step the instance bound did not
/// expand (ADR-027 AE-4). Either makes the state limited.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Cut {
    /// The memory bound left a store of the state unexpanded.
    pub memory: bool,
    /// The state has `max_live_instances` live instances and an `activate`
    /// step enabled.
    pub instances: bool,
}

/// An expansion, and what a method bound left of its state unexpanded.
pub type Bounded<T, S> = (Expansion<T, S, ModelFinding>, Cut);

/// A subject's transition identity, and the fairness set member a clause
/// over the subject carries: an operation constraint for a model subject
/// (FR-123), a protocol member with its target and origin for a protocol
/// subject (FR-212).
pub trait SubjectTransition: Clone + Eq + std::fmt::Debug {
    /// One member of a fairness set over the subject.
    type Fairness: Clone + Eq + std::fmt::Debug;

    /// The shared-location access the step makes, when it makes one under a
    /// weak memory model (ADR-025 DR-3).
    fn access(&self) -> Option<&AccessRecord> {
        None
    }

    /// The data race the step completes, when it completes one (ADR-025
    /// DR-4).
    fn race(&self) -> Option<&DataRace> {
        None
    }
}

impl SubjectTransition for ModelTransition {
    type Fairness = FairnessConstraint;
}

impl SubjectTransition for ProtocolTransition {
    type Fairness = ProtocolFairness;

    fn access(&self) -> Option<&AccessRecord> {
        ProtocolTransition::access(self)
    }

    fn race(&self) -> Option<&DataRace> {
        ProtocolTransition::race(self)
    }
}

/// Why a clause's fairness set could not be resolved to the kernels'
/// constraints over a graph's transitions (FR-339).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConstraintError {
    /// A fairness constraint names an operation the subject does not declare.
    #[error("the fairness constraint on `{name}` names an operation the subject does not declare")]
    UnknownOperation {
        /// The operation's resolved name.
        name: String,
    },
    /// The protocol subject's fairness set did not build.
    #[error(transparent)]
    Protocol(#[from] ProtocolFairnessError),
    /// A transition the graph holds has no identity number.
    #[error("a transition of the graph has no identity number")]
    Unnumbered,
}

/// A subject's transition system as EN-1 and replay read it.
pub trait SubjectSystem:
    TransitionSystem<Finding = ModelFinding, State: Clone, TransitionId: SubjectTransition>
{
    /// FR-101's complete RFC 8785 canonical state-key bytes for `state`.
    ///
    /// # Errors
    ///
    /// [`EncodingRefusal`] when the key has no RFC 8785 encoding.
    fn state_key(
        &self,
        state: &Self::State,
    ) -> Result<(Vec<u8>, crate::simulation::StateKey), EncodingRefusal>;

    /// FR-101's canonical state key for internal simulation ordering.
    fn canonical_state_key(&self, state: &Self::State) -> Result<StateKey, EncodingRefusal>;

    /// FR-101's canonical bytes of `transition`, which order successors.
    ///
    /// # Errors
    ///
    /// [`EncodingRefusal`] when the identity has no RFC 8785 encoding.
    fn transition_key(&self, transition: &Self::TransitionId) -> Result<Vec<u8>, EncodingRefusal>;

    /// FR-124 and ADR-027 PD-1, PD-2, AE-4: the class of `state`, whose
    /// bounded expansion is `expansion` and `cut`. A state left with an
    /// `activate` step unexpanded (`cut.instances`) is never terminal, and
    /// under `activation on each` a state whose every step is an `activate`
    /// is classed as one with no step.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when a `terminal when` predicate's evaluation
    /// stops.
    fn terminal_class(
        &self,
        state: &Self::State,
        expansion: &Expansion<Self::TransitionId, Self::State, ModelFinding>,
        cut: Cut,
    ) -> Result<TerminalClass, ExpansionStop>;

    /// Whether `transition` counts for interval distance (ADR-027 PB-3):
    /// every step of a model subject does, ADR-018's reading.
    fn counts_step(&self, _transition: &Self::TransitionId) -> bool {
        true
    }

    /// The kernels' fairness constraints over `transitions`, the transitions
    /// of a zero-progress graph numbered by `identity` (ADR-027 PB-3, FR-338
    /// "Protocol non-progress acceptance"): the `authored` constraints and the
    /// subject's own scheduler constraints (FR-212). Every step of a model
    /// subject counts, so it has no loop of steps that do not and none to
    /// constrain.
    ///
    /// # Errors
    ///
    /// [`ZeroProgressError`] when a constraint names an operation the subject
    /// does not admit, or a transition cannot be numbered.
    fn progress_constraints(
        &self,
        _authored: &[FairnessConstraint],
        _transitions: &[Self::TransitionId],
        _identity: &dyn Fn(&Self::TransitionId) -> Result<Identity, ZeroProgressError>,
    ) -> Result<Vec<Constraint>, ZeroProgressError> {
        Ok(Vec::new())
    }

    /// FR-339: the kernels' resolved constraints of a TP-4 clause's
    /// `authored` fairness over `transitions`, the transitions of the checked
    /// graph (its edge labels and every state's enabled set), ascending by
    /// their FR-101 canonical bytes and numbered by `identity`. A model
    /// subject resolves FR-123's constraints in the clause's order (a `whole`
    /// constraint covers every transition of its operation, an `each`
    /// constraint is one constraint per such transition, in `transitions`'
    /// order). A protocol subject resolves its fairness set (FR-212).
    ///
    /// # Errors
    ///
    /// [`ConstraintError`] when a constraint names an operation the subject
    /// does not declare, the protocol's fairness set does not build, or
    /// `identity` numbers a transition not in `transitions`.
    fn clause_constraints(
        &self,
        authored: &[FairnessConstraint],
        transitions: &[Self::TransitionId],
        identity: &dyn Fn(&Self::TransitionId) -> Option<Identity>,
    ) -> Result<Vec<Constraint>, ConstraintError>;

    /// The model state a temporal atom reads at `state` (ADR-027 PB-2, SE-1
    /// g): the state itself for a model subject.
    fn atom_state<'s>(&self, state: &'s Self::State) -> Cow<'s, ModelSimState>;

    /// ADR-027 PD-4: the waiting threads of `state`, a deadlock
    /// counterexample's last state, reached by the steps `prefix`, from
    /// which a `Displaced` cause is read; none for a model subject.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an evaluation stops.
    fn blocked(
        &self,
        state: &Self::State,
        prefix: &[&Self::TransitionId],
    ) -> Result<Vec<BlockedThread>, ExpansionStop>;

    /// The expansion of `state`, and what a method bound left of it
    /// unexpanded (ADR-025 MB-3, ADR-027 AE-4); never anything for a subject
    /// with no method bound.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] as [`TransitionSystem::successors`] returns it.
    fn successors_bounded(
        &self,
        state: &Self::State,
    ) -> Result<Bounded<Self::TransitionId, Self::State>, ExpansionStop> {
        Ok((self.successors(state)?, Cut::default()))
    }

    /// `transition` as a counterexample step reports it, `post` its
    /// post-state: a protocol step over a weak `parallel` carries the
    /// post-state's memory component (ADR-025 MK-1); every other step is
    /// its identity. Called only for the steps a counterexample or a replay
    /// reports, never for every explored successor.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the component has no canonical form.
    fn reported(
        &self,
        transition: Self::TransitionId,
        _post: &Self::State,
    ) -> Result<Self::TransitionId, ExpansionStop> {
        Ok(transition)
    }
}

impl SubjectSystem for ModelSystem<'_> {
    fn state_key(
        &self,
        state: &ModelSimState,
    ) -> Result<(Vec<u8>, crate::simulation::StateKey), EncodingRefusal> {
        ModelSystem::state_key(self, state)
    }

    fn canonical_state_key(&self, state: &ModelSimState) -> Result<StateKey, EncodingRefusal> {
        ModelSystem::canonical_state_key(self, state)
    }

    fn transition_key(&self, transition: &Self::TransitionId) -> Result<Vec<u8>, EncodingRefusal> {
        ModelSystem::transition_key(self, transition)
    }

    fn terminal_class(
        &self,
        state: &ModelSimState,
        expansion: &Expansion<Self::TransitionId, ModelSimState, ModelFinding>,
        _cut: Cut,
    ) -> Result<TerminalClass, ExpansionStop> {
        ModelSystem::terminal_class(self, state, !expansion.successors.is_empty())
    }

    fn clause_constraints(
        &self,
        authored: &[FairnessConstraint],
        transitions: &[ModelTransition],
        identity: &dyn Fn(&ModelTransition) -> Option<Identity>,
    ) -> Result<Vec<Constraint>, ConstraintError> {
        let mut constraints = Vec::new();
        for constraint in authored {
            let strength = Strength::from(constraint.kind());
            let operation = constraint.operation();
            let text = self
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or_else(|| ConstraintError::UnknownOperation {
                    name: operation.name().to_owned(),
                })?;
            let covered = transitions
                .iter()
                .filter(|transition| transition.operation() == text);
            match constraint.granularity() {
                FairnessGranularity::Whole => {
                    let members = covered
                        .map(identity)
                        .collect::<Option<Vec<_>>>()
                        .ok_or(ConstraintError::Unnumbered)?;
                    constraints.push(Constraint::new(strength, members));
                }
                FairnessGranularity::Each => {
                    for transition in covered {
                        let number = identity(transition).ok_or(ConstraintError::Unnumbered)?;
                        constraints.push(Constraint::new(strength, [number]));
                    }
                }
            }
        }
        Ok(constraints)
    }

    fn atom_state<'s>(&self, state: &'s ModelSimState) -> Cow<'s, ModelSimState> {
        Cow::Borrowed(state)
    }

    fn blocked(
        &self,
        _state: &ModelSimState,
        _prefix: &[&Self::TransitionId],
    ) -> Result<Vec<BlockedThread>, ExpansionStop> {
        Ok(Vec::new())
    }
}
