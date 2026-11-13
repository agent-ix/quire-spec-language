// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-24, PA-25: the projection artifacts, the FR-351 record a
//! decisive counterexample carries, and the counterexample replay walk
//! (PA-20, ADR-018 SM-7).

use std::collections::BTreeMap;

use qsl_package::protocol_view::PortRef;
use quire_contract_model::CheckedNodeId;
use quire_exact::ObjectReference;

use super::{
    node_text, Action, BranchMerge, ChoiceBasis, GlobalStructure, MissingMember,
    ProtocolAnalysisKey,
};
use crate::identity::TracePosition;
use crate::result::{DecidingElement, SeparatingWitnessRecord};
use crate::{ObservationIdentity, RuntimeValuePath, ValuePathSubject};

/// The index of a local event within its graph.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LocalEventId(pub u32);

/// What a local event is (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalEventKind {
    /// A send to `peer`.
    Send {
        /// The receiving role.
        peer: CheckedNodeId,
        /// The message type.
        message_type: CheckedNodeId,
    },
    /// A receive from `peer`.
    Receive {
        /// The sending role.
        peer: CheckedNodeId,
        /// The message type.
        message_type: CheckedNodeId,
    },
    /// A choice the role owns.
    InternalChoice,
    /// A choice the role learns the outcome of.
    ExternalChoice,
    /// The entry of a `parallel` region.
    ParallelRegion,
    /// The exit of a `parallel` region.
    ParallelJoin,
}

/// One event of a local graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEvent {
    /// The event's identity in its graph.
    pub id: LocalEventId,
    /// What it is.
    pub kind: LocalEventKind,
}

/// A role's local graph: its events and the covering edges of its induced
/// order (PA-9).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalGraph {
    /// The events, ascending by id.
    pub events: Vec<LocalEvent>,
    /// The covering edges `(earlier, later)`, ascending.
    pub edges: Vec<(LocalEventId, LocalEventId)>,
    /// Pairs of events that are one behaviour of the role seen in two
    /// branches of a choice it merged (PA-10): alternatives, not concurrent
    /// events. Ascending.
    pub alternatives: Vec<(LocalEventId, LocalEventId)>,
}

/// The total map from each global event of the role to its local event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Correspondence(pub BTreeMap<PortRef, LocalEventId>);

/// How the role stands at one choice, and what merged there (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeEvidence {
    /// The choice event.
    pub choice: PortRef,
    /// The basis.
    pub basis: ChoiceBasis,
    /// The merged pairs of global events, for a [`ChoiceBasis::MustMerge`].
    pub merged: Vec<(PortRef, PortRef)>,
}

/// A sound projection (PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionWitness {
    /// The structural key.
    pub key: ProtocolAnalysisKey,
    /// The role's local graph.
    pub local_graph: LocalGraph,
    /// The correspondence table.
    pub correspondence: Correspondence,
    /// One entry per choice, ascending.
    pub merge_evidence: Vec<MergeEvidence>,
}

/// A minimal counterexample (PA-11, PA-25).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionCounterexample {
    /// The structural key.
    pub key: ProtocolAnalysisKey,
    /// The choice that does not merge.
    pub choice: CheckedNodeId,
    /// The choice's owner.
    pub owner: CheckedNodeId,
    /// The projected role.
    pub role: CheckedNodeId,
    /// The case body path A runs through.
    pub case_a: CheckedNodeId,
    /// The case body path B runs through.
    pub case_b: CheckedNodeId,
    /// Global events from the choice (exclusive) to the divergent action A;
    /// empty when A is the end.
    pub path_a: Vec<PortRef>,
    /// As `path_a`, for B.
    pub path_b: Vec<PortRef>,
    /// The divergent action of A; `None` is the end of the branch.
    pub action_a: Option<Action>,
    /// The divergent action of B; `None` is the end of the branch.
    pub action_b: Option<Action>,
    /// The FR-351 record (PA-24).
    pub separating_witness: SeparatingWitnessRecord,
}

impl ProjectionCounterexample {
    /// The length PA-11 minimises: the sum of the two paths' event counts.
    #[must_use]
    pub fn length(&self) -> usize {
        self.path_a.len() + self.path_b.len()
    }
}

/// The claim's assessed-subject root, the one input of the FR-351 record the
/// engine does not derive (FR-574). It is not a key member.
#[derive(Clone, Debug, Default)]
pub struct AssessedRootDraft {
    /// The observation identity.
    pub observation: Option<ObservationIdentity>,
    /// The FR-204 declared object identity.
    pub object: Option<ObjectReference>,
}

/// An assessed-subject root with every component present.
#[derive(Clone, Debug)]
pub struct AssessedRoot {
    observation: ObservationIdentity,
    object: ObjectReference,
}

impl AssessedRoot {
    /// Build the root, refusing the first absent component.
    ///
    /// # Errors
    ///
    /// [`MissingMember`] naming `assessed_root`, `assessed_root.observation`
    /// or `assessed_root.object`.
    pub fn new(draft: Option<AssessedRootDraft>) -> Result<Self, MissingMember> {
        let draft = draft.ok_or_else(|| MissingMember::new("assessed_root"))?;
        Ok(Self {
            observation: draft
                .observation
                .ok_or_else(|| MissingMember::new("assessed_root.observation"))?,
            object: draft
                .object
                .ok_or_else(|| MissingMember::new("assessed_root.object"))?,
        })
    }

    /// The FR-351 record of a decisive projection counterexample: the
    /// divergent action's node as `deciding_element`, the FR-207 path of the
    /// root's object with no steps, and the decimal `trace_position`.
    ///
    /// # Errors
    ///
    /// [`MissingMember`] naming `deciding_element` when the node does not
    /// encode.
    pub fn record(
        &self,
        deciding: &CheckedNodeId,
        trace_position: usize,
    ) -> Result<SeparatingWitnessRecord, MissingMember> {
        let text = node_text(deciding).map_err(|_| MissingMember::new("deciding_element"))?;
        Ok(SeparatingWitnessRecord {
            quantifier: None,
            deciding_element: DecidingElement::Protocol(text),
            index: None,
            value_path: RuntimeValuePath {
                observation: self.observation.clone(),
                subject: ValuePathSubject::Object(self.object.clone()),
                steps: Vec::new(),
            },
            trace_position: Some(TracePosition::new(trace_position.to_string())),
        })
    }
}

/// The evidence a `ReplaySource::ProtocolAnalysis` request carries: the
/// counterexample to re-walk (PA-20).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolAnalysisEvidence {
    /// The counterexample.
    pub counterexample: ProjectionCounterexample,
}

impl ProtocolAnalysisEvidence {
    /// The evidence's measured encoded size.
    #[must_use]
    pub fn measured_bytes(&self) -> usize {
        let ce = &self.counterexample;
        std::mem::size_of_val(ce)
            + (ce.path_a.len() + ce.path_b.len()) * std::mem::size_of::<PortRef>()
            + ce.separating_witness.measured_bytes()
    }
}

/// Why a counterexample's replay disagrees with the checked graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WalkDisagreement {
    /// The named choice is no choice of the graph.
    UnknownChoice,
    /// The role is told the outcome (or owns the choice), so nothing need
    /// merge there.
    NotMustMerge,
    /// A step is not an admitted successor of its predecessor.
    StepNotSuccessor(PortRef),
    /// A path does not end at its stated action, or the action is not the
    /// role's.
    ActionMismatch,
    /// A path leaves the case it is stated for.
    WrongBranch,
    /// The two cases merge, or diverge elsewhere than stated.
    NotTheStatedDivergence,
    /// A branch holds events beyond the horizon.
    Cut,
}

/// The replay walk of a projection counterexample: it re-walks both paths
/// against the structure, checks each step is an admitted successor, and
/// that the walk ends in the stated divergence (PA-20).
///
/// # Errors
///
/// [`WalkDisagreement`] naming what disagrees.
pub fn replay_projection_counterexample(
    structure: &GlobalStructure,
    counterexample: &ProjectionCounterexample,
    max_depth: u64,
) -> Result<(), WalkDisagreement> {
    let ce = counterexample;
    let choice = PortRef {
        node: ce.choice.clone(),
        port: qsl_package::protocol_view::Port::Enter,
    };
    match structure.classify(&ce.role, &choice) {
        None => return Err(WalkDisagreement::UnknownChoice),
        Some(ChoiceBasis::MustMerge) => {}
        Some(_) => return Err(WalkDisagreement::NotMustMerge),
    }
    for (path, action, case) in [
        (&ce.path_a, &ce.action_a, &ce.case_a),
        (&ce.path_b, &ce.action_b, &ce.case_b),
    ] {
        let mut previous = choice.clone();
        for step in path {
            if !structure.successors(&previous).contains(&step) {
                return Err(WalkDisagreement::StepNotSuccessor(step.clone()));
            }
            let inside = structure
                .event(step)
                .is_some_and(|event| event.within.contains(case));
            if !inside {
                return Err(WalkDisagreement::WrongBranch);
            }
            previous = step.clone();
        }
        match action {
            Some(action) => {
                if path.last() != Some(&action.at)
                    || structure.action_of(&action.at, &ce.role).as_ref() != Some(action)
                {
                    return Err(WalkDisagreement::ActionMismatch);
                }
            }
            None if !path.is_empty() => return Err(WalkDisagreement::ActionMismatch),
            None => {}
        }
    }
    match structure.merge(&ce.role, &ce.case_a, &ce.case_b, max_depth) {
        BranchMerge::Cut => Err(WalkDisagreement::Cut),
        BranchMerge::Merged(_) => Err(WalkDisagreement::NotTheStatedDivergence),
        BranchMerge::Diverged(found) => {
            let stated = (&ce.action_a, &ce.action_b);
            if found
                .iter()
                .any(|divergence| (&divergence.first, &divergence.second) == stated)
            {
                Ok(())
            } else {
                Err(WalkDisagreement::NotTheStatedDivergence)
            }
        }
    }
}

/// How a counterexample replay settled (ADR-018 SM-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolReplaySettlement {
    /// The walk reproduced the stated divergence.
    Reproduced,
    /// The walk disagrees with the checked graph.
    Inconclusive(WalkDisagreement),
}

/// The result of the `ReplaySource::ProtocolAnalysis` arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolReplayResult {
    /// The settlement.
    pub settlement: ProtocolReplaySettlement,
}

impl ProtocolReplayResult {
    /// The category the result carries: a reproduced walk is a success, a
    /// disagreeing one `inconclusive`.
    #[must_use]
    pub fn category(&self) -> quire_semantic_value::catalog::Category {
        use quire_semantic_value::catalog::Category;
        match self.settlement {
            ProtocolReplaySettlement::Reproduced => Category::Success,
            ProtocolReplaySettlement::Inconclusive(_) => Category::Inconclusive,
        }
    }
}
