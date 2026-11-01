// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-148: a memory-model-independent handle over the abstract
//! `ProtocolSystem`. Its state variant is fixed when the system is opened,
//! so candidate protocol states keep the selected memory component and
//! canonical protocol identity during refinement exploration and replay.

use qsl_eval::model_subject::ProtocolSubject;
use qsl_eval::simulation::model::{ModelFinding, ModelSimState, ModelSystem};
use qsl_eval::simulation::protocol::{
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, Ra, Sc, SubjectMemory, Tso,
};
use qsl_eval::simulation::{
    EncodingRefusal, Expansion, ExpansionStop, SubjectSystem, TransitionSystem,
};
use qsl_semantics::check::CheckedProtocol;

/// One state of the checked protocol system. The enum tag records which
/// selected memory model owns its state component.
#[derive(Clone, Debug)]
pub enum ProtocolCandidate {
    /// Sequential consistency.
    Sc(ProtocolState<Sc>),
    /// Total store order.
    Tso(ProtocolState<Tso>),
    /// Release-acquire.
    Ra(ProtocolState<Ra>),
}

/// The abstract protocol transition system used by refinement.
pub struct AbstractProtocolSystem<'a> {
    inner: ProtocolSystemKind<'a>,
}

enum ProtocolSystemKind<'a> {
    Sc(ProtocolSystem<'a, Sc>),
    Tso(ProtocolSystem<'a, Tso>),
    Ra(ProtocolSystem<'a, Ra>),
}

impl<'a> AbstractProtocolSystem<'a> {
    /// Open the selected protocol system using the model and bindings already
    /// admitted for the refinement's abstract subject.
    pub fn new(
        model: ModelSystem<'a>,
        subject: &ProtocolSubject<'a>,
    ) -> Result<Self, ProtocolRefusal> {
        let bound = qsl_eval::simulation::protocol::live_instance_bound(None);
        let inner = match SubjectMemory::of(&subject.memory, None, None) {
            SubjectMemory::Sc => ProtocolSystemKind::Sc(
                ProtocolSystem::new(model, subject, Sc)?.with_max_live_instances(bound),
            ),
            SubjectMemory::Tso(memory) => ProtocolSystemKind::Tso(
                ProtocolSystem::new(model, subject, memory)?.with_max_live_instances(bound),
            ),
            SubjectMemory::Ra(memory) => ProtocolSystemKind::Ra(
                ProtocolSystem::new(model, subject, memory)?.with_max_live_instances(bound),
            ),
        };
        Ok(Self { inner })
    }

    /// The abstract model shared with the ordinary refinement mapper.
    pub fn model(&self) -> &ModelSystem<'a> {
        match &self.inner {
            ProtocolSystemKind::Sc(system) => system.model(),
            ProtocolSystemKind::Tso(system) => system.model(),
            ProtocolSystemKind::Ra(system) => system.model(),
        }
    }

    /// Initial settled states of the abstract protocol.
    pub fn initial(&self) -> Vec<ProtocolCandidate> {
        match &self.inner {
            ProtocolSystemKind::Sc(system) => system
                .initial()
                .into_iter()
                .map(ProtocolCandidate::Sc)
                .collect(),
            ProtocolSystemKind::Tso(system) => system
                .initial()
                .into_iter()
                .map(ProtocolCandidate::Tso)
                .collect(),
            ProtocolSystemKind::Ra(system) => system
                .initial()
                .into_iter()
                .map(ProtocolCandidate::Ra)
                .collect(),
        }
    }

    /// The model state embedded in one protocol candidate.
    pub fn model_state<'s>(&self, state: &'s ProtocolCandidate) -> Option<&'s ModelSimState> {
        match (&self.inner, state) {
            (ProtocolSystemKind::Sc(_), ProtocolCandidate::Sc(state)) => Some(state.model()),
            (ProtocolSystemKind::Tso(_), ProtocolCandidate::Tso(state)) => Some(state.model()),
            (ProtocolSystemKind::Ra(_), ProtocolCandidate::Ra(state)) => Some(state.model()),
            _ => None,
        }
    }

    /// The checked clause whose node and binder identities define labels.
    pub fn protocol(&self) -> &CheckedProtocol {
        match &self.inner {
            ProtocolSystemKind::Sc(system) => system.protocol(),
            ProtocolSystemKind::Tso(system) => system.protocol(),
            ProtocolSystemKind::Ra(system) => system.protocol(),
        }
    }

    /// The full canonical key of one state, including protocol control and
    /// memory-model components.
    pub fn state_key(
        &self,
        state: &ProtocolCandidate,
    ) -> Result<(Vec<u8>, qsl_eval::simulation::StateKey), EncodingRefusal> {
        match (&self.inner, state) {
            (ProtocolSystemKind::Sc(system), ProtocolCandidate::Sc(state)) => {
                system.state_key(state)
            }
            (ProtocolSystemKind::Tso(system), ProtocolCandidate::Tso(state)) => {
                system.state_key(state)
            }
            (ProtocolSystemKind::Ra(system), ProtocolCandidate::Ra(state)) => {
                system.state_key(state)
            }
            _ => Err(EncodingRefusal::NoEncoding(
                "protocol candidate belongs to its admitted system".to_owned(),
            )),
        }
    }

    /// Canonical identity bytes of one abstract protocol transition.
    pub fn transition_key(
        &self,
        transition: &ProtocolTransition,
    ) -> Result<Vec<u8>, EncodingRefusal> {
        match &self.inner {
            ProtocolSystemKind::Sc(system) => system.transition_key(transition),
            ProtocolSystemKind::Tso(system) => system.transition_key(transition),
            ProtocolSystemKind::Ra(system) => system.transition_key(transition),
        }
    }

    /// Expand one candidate, preserving the transition IDs supplied by the
    /// admitted system.
    pub fn successors(
        &self,
        state: &ProtocolCandidate,
    ) -> Result<Expansion<ProtocolTransition, ProtocolCandidate, ModelFinding>, ExpansionStop> {
        match (&self.inner, state) {
            (ProtocolSystemKind::Sc(system), ProtocolCandidate::Sc(state)) => {
                let expansion = system.successors(state)?;
                Ok(Expansion {
                    successors: expansion
                        .successors
                        .into_iter()
                        .map(|(transition, state)| (transition, ProtocolCandidate::Sc(state)))
                        .collect(),
                    findings: expansion.findings,
                })
            }
            (ProtocolSystemKind::Tso(system), ProtocolCandidate::Tso(state)) => {
                let expansion = system.successors(state)?;
                Ok(Expansion {
                    successors: expansion
                        .successors
                        .into_iter()
                        .map(|(transition, state)| (transition, ProtocolCandidate::Tso(state)))
                        .collect(),
                    findings: expansion.findings,
                })
            }
            (ProtocolSystemKind::Ra(system), ProtocolCandidate::Ra(state)) => {
                let expansion = system.successors(state)?;
                Ok(Expansion {
                    successors: expansion
                        .successors
                        .into_iter()
                        .map(|(transition, state)| (transition, ProtocolCandidate::Ra(state)))
                        .collect(),
                    findings: expansion.findings,
                })
            }
            _ => Err(
                ProtocolRefusal::Fault(quire_semantic_value::catalog::InternalFault::new(
                    "refinement-protocol",
                    "candidate-belongs-to-admitted-system",
                ))
                .into(),
            ),
        }
    }
}
