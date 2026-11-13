// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-19, PA-25: the realizability artifacts (FR-571): a role's
//! actions and observations, the strategy witness and the two
//! counterexample forms.

use std::collections::BTreeMap;

use qsl_eval::simulation::protocol::ProtocolTransition;
use quire_contract_model::CheckedNodeId;

use super::ProtocolAnalysisKey;
use crate::result::SeparatingWitnessRecord;

/// What a role does next at one of its decision points (FR-571).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StrategyAction {
    /// The `send` control `node`. `payload` holds the FR-181 typed canonical JSON bytes of the
    /// sent record when the send fixes the branch of a choice the role owns
    /// (the owner's branch selection); `None` for any other send, whose
    /// payload the environment picks.
    Send {
        /// The `send` control.
        node: CheckedNodeId,
        /// The chosen record, for an owner's branch selection.
        payload: Option<Vec<u8>>,
    },
    /// Take the next message from `peer`: an external choice keyed by the
    /// message that arrives (PA-10).
    Receive {
        /// The sending role.
        peer: CheckedNodeId,
    },
    /// Take no further step.
    End,
}

/// One item of a role's local observation (PA-16).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Observed {
    /// The role sent the record `payload` at the `send` control `node`.
    Sent {
        /// The `send` control.
        node: CheckedNodeId,
        /// The record's FR-181 typed canonical JSON bytes.
        payload: Vec<u8>,
    },
    /// The role received the record `payload` at the `receive` control
    /// `node`.
    Received {
        /// The `receive` control.
        node: CheckedNodeId,
        /// The record's FR-181 typed canonical JSON bytes.
        payload: Vec<u8>,
    },
}

/// An observation class: the observation sequence every history of the
/// class shares. The declared shared facts (premise group (c)) are common
/// to every history of every role, so they never split a class and are not
/// repeated here.
pub type ObservationClass = Vec<Observed>;

/// One role's strategy: a table from observation class to the committed
/// action, over every class the exploration reached (PA-19).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleStrategy {
    /// The role node.
    pub role: CheckedNodeId,
    /// The committed action of each class, ascending by class.
    pub table: BTreeMap<ObservationClass, StrategyAction>,
}

/// What the exploration behind a witness covered (PA-19).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Closure {
    /// The maximal plays the replay walk explored.
    pub plays: u64,
    /// The horizon, `max_depth`.
    pub horizon: u64,
    /// Whether every play ended before the horizon.
    pub closed: bool,
}

/// A replayable joint strategy (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealizabilityWitness {
    /// The structural key.
    pub key: ProtocolAnalysisKey,
    /// One strategy per role, ascending by role node.
    pub strategies: Vec<RoleStrategy>,
    /// The closure record.
    pub closure: Closure,
}

/// One owner branch of the first lost owner node and an environment play
/// that defeats it (PA-18 (ii)).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchDefeat {
    /// The owner role, `None` when no owner move lies on the defeat.
    pub owner: Option<CheckedNodeId>,
    /// The owner's branch selection, `None` as for `owner`.
    pub branch: Option<StrategyAction>,
    /// The defeating play from an initial state, in step order.
    pub play: Vec<ProtocolTransition>,
}

/// A forced refutation (PA-18, PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RealizabilityCounterexample {
    /// Two histories one role cannot tell apart that need disjoint actions.
    IndistinguishableHistories {
        /// The structural key.
        key: ProtocolAnalysisKey,
        /// The role.
        role: CheckedNodeId,
        /// The first history, in step order from an initial state.
        history_a: Vec<ProtocolTransition>,
        /// The second history.
        history_b: Vec<ProtocolTransition>,
        /// The action `history_a` needs.
        action_a: StrategyAction,
        /// The action `history_b` needs.
        action_b: StrategyAction,
        /// The violated obligation's node.
        obligation: CheckedNodeId,
        /// The FR-351 record (PA-24).
        separating_witness: SeparatingWitnessRecord,
    },
    /// The environment defeats every owner choice.
    EnvironmentDefeat {
        /// The structural key.
        key: ProtocolAnalysisKey,
        /// One defeating play per owner branch, in branch order.
        defeats_per_branch: Vec<BranchDefeat>,
        /// The violated obligation's node.
        obligation: CheckedNodeId,
        /// The FR-351 record (PA-24).
        separating_witness: SeparatingWitnessRecord,
    },
}
