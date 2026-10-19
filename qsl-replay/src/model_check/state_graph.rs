// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166 and FR-168 Outputs (ADR-022 GV-1 to GV-7, GX-1): what EN-1
//! decides for one state-graph item instance, and the evidence it carries.
//!
//! The engine is `qsl-analyze`'s; the core owns this vocabulary because the
//! settlement map and the replay of the evidence read it (ADR-029 CB-2).

use std::collections::BTreeSet;

use qsl_eval::simulation::SampleProvenance;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::PredicateRole;
use quire_exact::ObjectReference;
use quire_semantic_value::location::Location;

use crate::model_check::counterexample::ModelStep;
use crate::model_check::limits::ReachedLimit;
use crate::proof_result::IncompleteCause;

/// A path of the subject's state graph (ADR-022 GX-1): an initial state and
/// the steps taken from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPath {
    /// The index of the path's initial state in the subject's initial list.
    pub initial: u32,
    /// The steps, in order: transition identity and post-state digest.
    pub steps: Vec<ModelStep>,
}

/// How a witness for one initial state was found (ADR-022 GV-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessSource {
    /// A phase 0 walk, named by its seed and trace index (FR-166).
    Sampled(SampleProvenance),
    /// Phase 1's canonical shortest path over the explored graph (FR-168).
    Explored,
}

/// One initial state's witness: a path ending at a node where the target
/// holds, and its source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphWitness {
    /// The path.
    pub path: ModelPath,
    /// How it was found.
    pub source: WitnessSource,
}

/// Why a node was not fully expanded (ADR-022 GM-6): its outgoing edges are
/// not known.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OpenCause {
    /// The node is at the search horizon and has a successor the run did not
    /// follow.
    MaxDepth,
    /// The node's expansion evaluated a contract conjunction, or a
    /// predicate of a claim read at the node, undecided.
    UndecidedSuccessor,
    /// The node was discovered and not expanded when a limit or a
    /// cancellation stopped the run.
    NotExpanded,
}

/// How an exploration ended (FR-167).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunEnd {
    /// The frontier emptied.
    Completed,
    /// A limit or a cancellation stopped the run.
    Stopped {
        /// `ResourceExhausted` for a limit, `Cancelled` for a cancellation.
        cause: IncompleteCause,
        /// The limit reached; `None` for a cancellation or a stop no named
        /// limit caused.
        limit: Option<ReachedLimit>,
    },
}

/// QSpec FR-391's `where` and `cause` of an undefined predicate evaluation
/// (ADR-022 GV-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphUndefined {
    /// The predicate of the claim that has no value.
    pub predicate: PredicateRole,
    /// The `quire.simulation.state-key/v1` digest of the node.
    pub state_key: DigestRecord,
    /// The expression with no value, when the evaluator located it.
    pub locus: Option<Location>,
    /// The catalogued reason.
    pub cause: UndefinedCause,
}

/// FR-168's decision for one state-graph item instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateGraphOutcome {
    /// A witness for every initial state, in initial-state order, on a run
    /// that completed with no open node.
    Witnessed {
        /// One witness per initial state.
        paths: Vec<GraphWitness>,
    },
    /// A witness for every initial state on a run that a limit stopped or
    /// that ended with open nodes: a reachable undefined evaluation of the
    /// target is not ruled out.
    WitnessedUnchecked {
        /// One witness per initial state.
        paths: Vec<GraphWitness>,
        /// How the run ended.
        end: RunEnd,
        /// The causes of the run's open nodes.
        open: BTreeSet<OpenCause>,
    },
    /// A path to a trap node: a closed node whose forward closure is closed
    /// and holds no target node.
    Trapped {
        /// The canonical path to the trap.
        stem: ModelPath,
    },
    /// Two distinct step sequences from one node where `from` holds, each
    /// ending at its first `to` node.
    PathPair {
        /// The canonical path to the node.
        stem: ModelPath,
        /// The shortest path from it, ties broken by canonical edge order.
        first: Vec<ModelStep>,
        /// The least path that first deviates from `first`.
        second: Vec<ModelStep>,
    },
    /// The claim holds over every reachable node: the run completed with no
    /// open node, unreduced.
    Holds,
    /// No decisive evidence.
    NoDecision {
        /// How the run ended.
        end: RunEnd,
        /// The causes of the run's open nodes.
        open: BTreeSet<OpenCause>,
    },
    /// A predicate of the claim has no value at an explored node.
    Undefined {
        /// The canonical path to the first such node in discovery order.
        stem: ModelPath,
        /// Where and why.
        undefined: GraphUndefined,
    },
    /// The subject has no initial state.
    NoInitialState,
}

/// The decision for one instance of a state-graph item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphInstanceOutcome {
    /// The object the instance binds the claim's `over` parameter to; `None`
    /// for a claim with no `over`.
    pub over: Option<ObjectReference>,
    /// The decision.
    pub outcome: StateGraphOutcome,
    /// Phase 0's witness per initial state, in initial-state order, or none
    /// where no walk reached the target (FR-166 Outputs); empty when phase 0
    /// drew no walk for the instance.
    pub sampled: Vec<Option<GraphWitness>>,
    /// The phase 0 walks of a `possible` instance that `max_walk_steps`
    /// stopped before they visited a target state (FR-166); 0 for every
    /// other form.
    pub walks_stopped: u64,
}
