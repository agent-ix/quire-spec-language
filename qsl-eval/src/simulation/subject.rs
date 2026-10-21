// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 TS-6, PX-2: what the model checker, its closure checker and replay
//! read of a subject's transition system beside FR-101's
//! [`TransitionSystem`]. [`crate::simulation::model::ModelSystem`] and
//! [`crate::simulation::protocol::ProtocolSystem`] implement it, so one
//! engine checks a model subject and a protocol subject.

use std::borrow::Cow;

use qsl_foundation::digest::DigestRecord;

use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::EncodingRefusal;
use crate::simulation::model::{ModelFinding, ModelSimState, ModelSystem, TerminalClass};
use crate::simulation::protocol::BlockedThread;

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

/// A subject's transition system as EN-1 and replay read it.
pub trait SubjectSystem: TransitionSystem<Finding = ModelFinding, State: Clone> {
    /// FR-101's `quire.simulation.state-key/v1` digest of `state`, and its
    /// full key bytes.
    ///
    /// # Errors
    ///
    /// [`EncodingRefusal`] when the key has no RFC 8785 encoding.
    fn state_key(&self, state: &Self::State) -> Result<(Vec<u8>, DigestRecord), EncodingRefusal>;

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
    fn state_key(&self, state: &ModelSimState) -> Result<(Vec<u8>, DigestRecord), EncodingRefusal> {
        ModelSystem::state_key(self, state)
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
