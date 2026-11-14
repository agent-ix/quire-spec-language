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
    /// The `send` control `node`; the record sent is the environment's.
    Send {
        /// The `send` control.
        node: CheckedNodeId,
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
    /// The protocol captures of one instance, which every role of it holds
    /// (FR-218), as bound when the instance starts: the RFC 8785 bytes of
    /// the capture names and FR-181 typed values. They lead an observation,
    /// one per instance the play has started, and only for a protocol that
    /// declares a capture.
    Captures {
        /// The instance's ordinal.
        instance: u64,
        /// The capture bytes.
        payload: Vec<u8>,
    },
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
/// class shares. Premise group (c) holds no fact in V1 (a non-empty set is
/// refused), so it adds nothing here.
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

/// One step of a play: the index of the successor taken, in the `qsl-eval`
/// system's canonical successor order of the state before it, and that
/// successor's transition. The index makes the walk determinate; the
/// transition must equal the one at that index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayStep {
    /// The successor's index.
    pub successor: usize,
    /// The successor's transition.
    pub transition: ProtocolTransition,
}

/// A play: the index of its initial state, in the system's canonical
/// initial order, and its steps in order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Play {
    /// The initial state's index.
    pub initial: usize,
    /// The steps.
    pub steps: Vec<PlayStep>,
}

/// Which history of an indistinguishable pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryEnd {
    /// The end of `history_a`.
    A,
    /// The end of `history_b`.
    B,
}

/// PA-18 (i) evidence that an action the role can take at both ends loses
/// at one of them: a continuation from that end whose first step is the
/// role taking the action and whose last state violates an obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LostAction {
    /// The action.
    pub action: StrategyAction,
    /// The end it loses at.
    pub at: HistoryEnd,
    /// The steps from that end to the violation.
    pub continuation: Vec<PlayStep>,
}

/// The defeating play of one owner branch of the first lost owner node
/// (PA-18 (ii)). In the `bounded-static` profile a choice's branch is fixed
/// by the environment's moves, so that node has one branch and the
/// counterexample one entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchDefeat {
    /// The defeating play.
    pub play: Play,
}

/// A forced refutation (PA-18, PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    clippy::large_enum_variant,
    reason = "a result holds at most one counterexample; boxing a variant saves nothing"
)]
pub enum RealizabilityCounterexample {
    /// Two histories one role cannot tell apart that need disjoint actions.
    IndistinguishableHistories {
        /// The structural key.
        key: ProtocolAnalysisKey,
        /// The role.
        role: CheckedNodeId,
        /// The first history.
        history_a: Play,
        /// The second history.
        history_b: Play,
        /// The action `history_a` needs.
        action_a: StrategyAction,
        /// The action `history_b` needs.
        action_b: StrategyAction,
        /// For each action the role can take at both ends, the evidence
        /// that it loses at one of them: the not-lost action sets of the two
        /// ends are disjoint (PA-18 (i)).
        lost: Vec<LostAction>,
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
