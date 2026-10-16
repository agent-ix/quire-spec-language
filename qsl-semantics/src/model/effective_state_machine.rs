//! A completed view's answer for a state machine's effective states and
//! transitions, and for type conformance (FR-081, FR-208).
//!
//! States and transitions are not members: they have no effective identity,
//! so nothing here adds an entry to [`EffectiveView::declarations`] or a byte
//! to any identity, and reading charges no normalization work.

use crate::model::domain_package::{DomainPackageRecord, StateMachineRecord};
use crate::model::key::DeclarationKey;
use crate::model::normalize::EffectiveView;

/// One effective state of a state machine: a state its declaring machine
/// authored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveState<'a> {
    /// The machine that declares the state: the queried machine or an ancestor.
    pub declaring: &'a DeclarationKey,
    /// The state's original declaration key.
    pub key: &'a DeclarationKey,
    /// The state's position in the declaring machine's list.
    pub position: usize,
    /// The authored state name.
    pub name: &'a str,
}

/// One effective transition of a state machine, exactly as its declaring
/// machine authored it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveTransition<'a> {
    /// The machine that declares the transition.
    pub declaring: &'a DeclarationKey,
    /// The transition's original declaration key.
    pub key: &'a DeclarationKey,
    /// The transition's position in the declaring machine's list.
    pub position: usize,
    /// The source state key.
    pub from: &'a DeclarationKey,
    /// The target state key.
    pub to: &'a DeclarationKey,
    /// The trigger operation key.
    pub trigger: &'a DeclarationKey,
    /// The optional guard clause id.
    pub guard: Option<&'a str>,
    /// The emitted event type keys, in authored order.
    pub emits: &'a [DeclarationKey],
}

/// A state machine's effective states and transitions: its own plus those of
/// every ancestor, each ordered by declaring machine key ascending as UTF-8
/// bytes, then position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveStateMachine<'a> {
    /// The effective states.
    pub states: Vec<EffectiveState<'a>>,
    /// The effective transitions.
    pub transitions: Vec<EffectiveTransition<'a>>,
}

impl EffectiveView {
    /// The effective states and transitions of the state machine type
    /// `machine`, or `None` when `machine` names no state machine type of
    /// the package (FR-081, FR-208).
    pub fn state_machine(&self, machine: &DeclarationKey) -> Option<EffectiveStateMachine<'_>> {
        let index = self.model_index();
        let records = &self.domain_package().records;
        let mut machines: Vec<&StateMachineRecord> = Vec::new();
        for record in records {
            if let DomainPackageRecord::StateMachine(candidate) = record {
                if index.conforms_admitted(machine, &candidate.key) {
                    machines.push(candidate);
                }
            }
        }
        if !machines.iter().any(|candidate| candidate.key == *machine) {
            return None;
        }
        machines.sort_by(|a, b| a.key.cmp(&b.key));
        let mut view = EffectiveStateMachine {
            states: Vec::new(),
            transitions: Vec::new(),
        };
        for declaring in machines {
            view.states.extend(
                declaring
                    .states
                    .iter()
                    .enumerate()
                    .map(|(position, state)| EffectiveState {
                        declaring: &declaring.key,
                        key: &state.key,
                        position,
                        name: &state.name,
                    }),
            );
            view.transitions
                .extend(
                    declaring
                        .transitions
                        .iter()
                        .enumerate()
                        .map(|(position, transition)| EffectiveTransition {
                            declaring: &declaring.key,
                            key: &transition.key,
                            position,
                            from: &transition.from,
                            to: &transition.to,
                            trigger: &transition.trigger,
                            guard: transition.guard.as_deref(),
                            emits: &transition.emits,
                        }),
                );
        }
        Some(view)
    }

    /// Whether `s` conforms to `t`: the same key, or a chain of declared
    /// `supertypes` edges from `s` to `t` (FR-082). Two types that share only
    /// a display name do not conform. Every edge was admitted against the
    /// `ancestor_steps` limit when this view completed, so reading it adds no
    /// normalization charge and no limit outcome.
    pub fn conforms(&self, s: &DeclarationKey, t: &DeclarationKey) -> bool {
        self.model_index().conforms_admitted(s, t)
    }
}
