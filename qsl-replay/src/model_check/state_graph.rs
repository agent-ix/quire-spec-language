// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166 and FR-168 Outputs (ADR-022 GV-1 to GV-7, GX-1): what EN-1
//! decides for one state-graph item instance, and the evidence it carries.
//!
//! The engine is `qsl-analyze`'s; the core owns this vocabulary because the
//! settlement map and the replay of the evidence read it (ADR-029 CB-2).

use std::collections::BTreeSet;

use crate::identity::{DeclaredDomain, ObligationIdentity, ProfileSelection, RawSourceRef};
use crate::witness::FamilyPayload;
use qsl_eval::simulation::SampleProvenance;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::BackendId;
use qsl_foundation::ReductionKind;
use qsl_semantics::check::PredicateRole;
use qsl_semantics::check::StateGraphForm;
use quire_exact::{ObjectReference, ScalarLimits};
use quire_semantic_value::location::Location;
use serde::{Serialize, Serializer};

use crate::certificate::state_graph::StateGraphCertificate;
use crate::model_check::counterexample::ModelStep;
use crate::model_check::limits::ReachedLimit;
use crate::proof_result::{AppliedReduction, IncompleteCause};

/// A path of the subject's state graph (ADR-022 GX-1): an initial state and
/// the steps taken from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPath {
    /// The index of the path's initial state in the subject's initial list.
    pub initial: u32,
    /// The steps, in order: transition identity and post-state digest.
    pub steps: Vec<ModelStep>,
}

/// A lasso of the subject's state graph in ADR-018 CX-2's step and loop shape
/// (FR-170, FR-181): a path whose steps are the stem and then the loop.
///
/// The loop re-enters at the position `loop_entry`, so its steps are the
/// path's steps from `loop_entry` on and the last post-state of the path is
/// the state at `loop_entry`. A loop that is the one terminal stutter step
/// (ADR-018 SM-4) has no model step: `loop_entry` is the path's length, the
/// position of the terminal state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelLasso {
    /// The stem and the loop.
    pub path: ModelPath,
    /// The position the loop re-enters, in `0..=path.steps.len()`.
    pub loop_entry: u32,
}

/// The form name carried when a selected reduction does not preserve a
/// state-graph claim (FR-167, FR-169).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateGraphPropertyForm {
    /// `possible`.
    Possible,
    /// `always possible`.
    AlwaysPossible,
    /// `unique path`.
    UniquePath,
}

impl StateGraphPropertyForm {
    /// The FR-331 wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Possible => "possible",
            Self::AlwaysPossible => "always-possible",
            Self::UniquePath => "unique-path",
        }
    }
}

impl From<&StateGraphForm> for StateGraphPropertyForm {
    fn from(form: &StateGraphForm) -> Self {
        match form {
            StateGraphForm::Possible { .. } => Self::Possible,
            StateGraphForm::AlwaysPossible { .. } => Self::AlwaysPossible,
            StateGraphForm::UniquePath { .. } => Self::UniquePath,
        }
    }
}

/// One initial state's HP-5 witness: a lasso on which the body is true and
/// whose loop is fair, and its source (FR-181).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LassoWitness {
    /// The lasso.
    pub lasso: ModelLasso,
    /// How it was found.
    pub source: WitnessSource,
}

/// How a witness for one initial state was found (ADR-022 GV-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessSource {
    /// A phase 0 walk, named by its seed and trace index (FR-166).
    Sampled(SampleProvenance),
    /// Phase 1's canonical shortest path over the explored graph (FR-168).
    Explored,
    /// An unrolled witness whose finite depth was selected by FR-181.
    Unrolled {
        /// The explored depth that produced the path.
        depth: u64,
    },
}

/// QSpec FR-393's witness source on the wire: numeric provenance values are
/// decimal strings, never JSON numbers.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum WitnessSourceWire {
    Sampled { seed: String, trace_index: String },
    Explored,
    Unrolled { depth: String },
}

impl Serialize for WitnessSource {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Sampled(provenance) => WitnessSourceWire::Sampled {
                seed: provenance.seed.to_string(),
                trace_index: provenance.trace.to_string(),
            },
            Self::Explored => WitnessSourceWire::Explored,
            Self::Unrolled { depth } => WitnessSourceWire::Unrolled {
                depth: depth.to_string(),
            },
        }
        .serialize(serializer)
    }
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

/// The path a state-graph witness records, or a fair temporal product lasso
/// that FR-181 replays through its own product system (FR-170, GX-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessPath {
    /// A finite path ending at a state where the claim's target holds.
    Path(ModelPath),
    /// A temporal product lasso routed to FR-181.
    Lasso(ModelLasso),
}

/// Which closure system a trap belongs to (FR-170).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrapClosure {
    /// The forward closure in the model state graph.
    StateGraph,
    /// One initial state's part of a temporal product; FR-181 owns replay.
    Product,
}

/// FR-170's typed state-graph evidence carried by a witness envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphEvidence {
    /// One path for each initial state, with the source of each path.
    Witness {
        /// The paths in initial-state order and how each was found.
        paths: Vec<(WitnessPath, WitnessSource)>,
        /// The claim's `over` binding, when it has one.
        over: Option<qsl_eval::model_subject::ModelInstance>,
    },
    /// A stem to a state whose forward closure is claimed to have no target.
    Trap {
        /// The path to the trap state.
        stem: ModelPath,
        /// The closure system used by the claim that produced it.
        closure: TrapClosure,
        /// The claim's `over` binding, when it has one.
        over: Option<qsl_eval::model_subject::ModelInstance>,
    },
    /// Two distinct paths from one `from` state to their first `to` state.
    PathPair {
        /// The path to the `from` state.
        stem: ModelPath,
        /// The first path from the stem's final state.
        first: Vec<ModelStep>,
        /// The second path from the stem's final state.
        second: Vec<ModelStep>,
        /// The claim's `over` binding, when it has one.
        over: Option<qsl_eval::model_subject::ModelInstance>,
    },
    /// A path to the first state where a claim predicate has no value.
    Undefined {
        /// The path to the undefined evaluation.
        stem: ModelPath,
        /// The undefined predicate and its location.
        undefined: GraphUndefined,
        /// The claim's `over` binding, when it has one.
        over: Option<qsl_eval::model_subject::ModelInstance>,
    },
}

impl crate::witness::FamilyPayload for GraphEvidence {
    fn measured_bytes(&self) -> usize {
        let path_bytes = |path: &ModelPath| {
            std::mem::size_of_val(path).saturating_add(
                path.steps
                    .len()
                    .saturating_mul(std::mem::size_of::<ModelStep>()),
            )
        };
        match self {
            Self::Witness { paths, .. } => {
                paths
                    .iter()
                    .fold(std::mem::size_of_val(self), |total, (path, _)| {
                        total.saturating_add(match path {
                            WitnessPath::Path(path) => path_bytes(path),
                            WitnessPath::Lasso(lasso) => path_bytes(&lasso.path),
                        })
                    })
            }
            Self::Trap { stem, .. } | Self::Undefined { stem, .. } => {
                std::mem::size_of_val(self).saturating_add(path_bytes(stem))
            }
            Self::PathPair {
                stem,
                first,
                second,
                ..
            } => std::mem::size_of_val(self)
                .saturating_add(path_bytes(stem))
                .saturating_add(first.len().saturating_mul(std::mem::size_of::<ModelStep>()))
                .saturating_add(
                    second
                        .len()
                        .saturating_mul(std::mem::size_of::<ModelStep>()),
                ),
        }
    }
}

/// FR-393's model-graph replay envelope. Its obligation identity names the
/// subject, claim and instance; unlike backend and input replay envelopes it
/// has no replay target or nested transcript/input source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEvidenceEnvelope {
    obligation_identity: ObligationIdentity,
    occurrence_key: qsl_foundation::source::provenance::OccurrenceKey,
    claim_node: qsl_foundation::digest::WireNodeId,
    package_id: DigestRecord,
    source_digests: Vec<RawSourceRef>,
    profile_selections: Vec<ProfileSelection>,
    run_limits: ScalarLimits,
    declared_domains: Vec<DeclaredDomain>,
    backend: BackendId,
    family_payload: GraphEvidence,
}

impl GraphEvidenceEnvelope {
    /// Build a typed model-graph evidence envelope.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        obligation_identity: ObligationIdentity,
        occurrence_key: qsl_foundation::source::provenance::OccurrenceKey,
        claim_node: qsl_foundation::digest::WireNodeId,
        package_id: DigestRecord,
        source_digests: Vec<RawSourceRef>,
        profile_selections: Vec<ProfileSelection>,
        run_limits: ScalarLimits,
        declared_domains: Vec<DeclaredDomain>,
        backend: BackendId,
        family_payload: GraphEvidence,
    ) -> Self {
        Self {
            obligation_identity,
            occurrence_key,
            claim_node,
            package_id,
            source_digests,
            profile_selections,
            run_limits,
            declared_domains,
            backend,
            family_payload,
        }
    }

    /// The obligation identity of the checked subject, claim and instance.
    pub fn obligation_identity(&self) -> ObligationIdentity {
        self.obligation_identity
    }

    /// The claim's checked occurrence key.
    pub fn occurrence_key(&self) -> &qsl_foundation::source::provenance::OccurrenceKey {
        &self.occurrence_key
    }

    /// The checked package node that owns the claim.
    pub fn claim_node(&self) -> qsl_foundation::digest::WireNodeId {
        self.claim_node
    }

    /// The package identity the evidence was produced against.
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }

    /// The package's declared source references.
    pub fn source_digests(&self) -> &[RawSourceRef] {
        &self.source_digests
    }

    /// The semantic profile selections of the proving run.
    pub fn profile_selections(&self) -> &[ProfileSelection] {
        &self.profile_selections
    }

    /// The accounting limits of the proving run.
    pub fn run_limits(&self) -> ScalarLimits {
        self.run_limits
    }

    /// The finite domains the proving run declared.
    pub fn declared_domains(&self) -> &[DeclaredDomain] {
        &self.declared_domains
    }

    /// The backend identity of the proving run.
    pub fn backend(&self) -> &BackendId {
        &self.backend
    }

    /// The typed evidence payload.
    pub fn family_payload(&self) -> &GraphEvidence {
        &self.family_payload
    }

    pub(crate) fn measured_bytes(&self) -> usize {
        std::mem::size_of_val(self)
            .saturating_add(self.occurrence_key.to_string().len())
            .saturating_add(
                self.source_digests
                    .iter()
                    .map(|source| {
                        source.authority().len()
                            + source.identity().len()
                            + source.revision_namespace().len()
                            + source.revision().len()
                            + source.digest().hex().len()
                    })
                    .sum::<usize>(),
            )
            .saturating_add(
                self.profile_selections
                    .iter()
                    .map(|selection| selection.profile().len() + selection.value().len())
                    .sum::<usize>(),
            )
            .saturating_add(self.backend.as_str().len())
            .saturating_add(self.family_payload.measured_bytes())
    }
}

/// Why a node was not fully expanded (ADR-022 GM-6): its outgoing edges are
/// not known.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OpenCause {
    /// The node is at the search horizon and has a successor the run did not
    /// follow.
    MaxDepth,
    /// The node's expansion evaluated a contract conjunction undecided; or,
    /// in one instance's outcome only, a predicate of that instance read
    /// refused at the node.
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
    /// The claim predicate that has no value; absent for an undefined
    /// transition guard, terminal predicate, or state constraint (FR-170).
    pub predicate: Option<PredicateRole>,
    /// The transition whose guard has no value, when this names a guard.
    pub transition: Option<quire_semantic_value::model_transition::ModelTransition>,
    /// The `quire.simulation.state-key/v1` digest of the node.
    pub state_key: DigestRecord,
    /// The expression with no value, when the evaluator located it.
    pub locus: Option<Location>,
    /// The catalogued reason.
    pub cause: UndefinedCause,
}

/// FR-168's decision for one state-graph item instance, or its item-level
/// `NoInstance` marker when FR-125 derives no instances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateGraphOutcome {
    /// A witness for every initial state, in initial-state order, on a run
    /// that completed with no open node.
    Witnessed {
        /// One witness per initial state.
        paths: Vec<GraphWitness>,
        /// The run's certificate that every initial state reaches a target
        /// (FR-169, ADR-022 GC-1).
        certificate: StateGraphCertificate,
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
    Holds {
        /// The run's certificate of the claim (FR-169, ADR-022 GC-1).
        certificate: StateGraphCertificate,
    },
    /// The state-graph claim held over a reduced graph. No core certificate
    /// covers that quotient, so settlement retains the reductions and marks
    /// the proof uncertified (FR-169, ADR-021 RV-1).
    HoldsReduced {
        /// The applied reductions, in symmetry then partial-order order.
        reductions: Vec<AppliedReduction>,
    },
    /// A selected reduction's preservation table rejects this form before
    /// expansion (FR-169, ADR-021 RV-2).
    ReductionNotPreserving {
        /// The first selected reduction that the form does not preserve.
        reduction: ReductionKind,
        /// The claim form.
        form: StateGraphPropertyForm,
    },
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
    /// FR-125 produced no instances for the item. This is an item-level
    /// settlement marker, not a decision for a state-graph instance.
    NoInstance,
}

/// The result for one state-graph item instance, or the item-level
/// `NoInstance` marker when the item has no instances.
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
