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

/// An expansion, and whether a memory bound left a store of its state
/// unexpanded (ADR-025 MB-3).
pub type Bounded<T, S> = (Expansion<T, S, ModelFinding>, bool);

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

    /// FR-124 and ADR-027 PD-1, PD-2: the class of `state`, whose expansion
    /// gave a successor exactly when `has_successor`.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when a `terminal when` predicate's evaluation
    /// stops.
    fn terminal_class(
        &self,
        state: &Self::State,
        has_successor: bool,
    ) -> Result<TerminalClass, ExpansionStop>;

    /// The model state a temporal atom reads at `state` (ADR-027 PB-2, SE-1
    /// g): the state itself for a model subject.
    fn atom_state<'s>(&self, state: &'s Self::State) -> Cow<'s, ModelSimState>;

    /// ADR-027 PD-4: the waiting threads of `state`, for a deadlock
    /// counterexample's last state; none for a model subject.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an evaluation stops.
    fn blocked(&self, state: &Self::State) -> Result<Vec<BlockedThread>, ExpansionStop>;

    /// The expansion of `state`, and whether a memory bound left a store of
    /// it unexpanded, which makes it bound-limited (ADR-025 MB-3); never
    /// for a subject with no memory bound.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] as [`TransitionSystem::successors`] returns it.
    fn successors_bounded(
        &self,
        state: &Self::State,
    ) -> Result<Bounded<Self::TransitionId, Self::State>, ExpansionStop> {
        Ok((self.successors(state)?, false))
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
        has_successor: bool,
    ) -> Result<TerminalClass, ExpansionStop> {
        ModelSystem::terminal_class(self, state, has_successor)
    }

    fn atom_state<'s>(&self, state: &'s ModelSimState) -> Cow<'s, ModelSimState> {
        Cow::Borrowed(state)
    }

    fn blocked(&self, _state: &ModelSimState) -> Result<Vec<BlockedThread>, ExpansionStop> {
        Ok(Vec::new())
    }
}
