// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244's zone certificate and FR-245's check of it in the qualified core
//! (ADR-026 CF-1, CF-3, CF-4, CF-6).
//!
//! EN-6, the zone search in `qsl-analyze`, returns a [`ZoneCertificate`]
//! with every proof. [`check_zone_certificate`] verifies it with the core's
//! own zones ([`super::zone`]), reading nothing from the search but the
//! certificate. It re-admits the subject, rebuilds its timed reading and
//! recomputes every successor of every node it reaches from the initial
//! states:
//!
//! 1. the `Malformed` members: the scale (every constant of the model and
//!    the claim exact at it), the LU bounds (each at least the
//!    bound the checker computes from the model), the nodes, the edges, and
//!    the component part (present for a liveness clause only) and
//!    divergence part (for the time-lock-freedom item only), each shaped to
//!    the nodes;
//! 2. the initial coverage: each initial symbolic state is covered by a
//!    named initial node of its discrete state (`InitialMissing`, at the
//!    initial discrete state);
//! 3. the nodes in node order: every symbolic successor has an edge of its
//!    transition identity and guard part to a node of its discrete state
//!    whose zone covers it under the aLU simulation with the certificate's
//!    bounds (`SuccessorMissing`); no node is bad unless it is `locked`, and
//!    from a `locked` node no quiescent node and no cycle that lets time
//!    diverge is reachable (`BadState`);
//! 4. for a liveness clause, the component part (`product`): every edge
//!    goes to an equal or greater component (`BackwardEdge`, at the edge),
//!    and each component's reason holds (`WitnessFails`, at the component);
//! 5. for the time-lock-freedom item, the divergence part (`divergence`):
//!    each node's path leads from it to a divergent target, and its every
//!    valuation diverges (`WitnessFails`, at the node);
//! 6. `admitted`: a path of such edges from an initial node to a quiescent
//!    node, a terminal node under a tick source, or a cycle that lets time
//!    advance (`WitnessFails`, at its last node).
//!
//! The first failure is the rejection. A node no edge reaches from an
//! initial node is read nowhere and checked for nothing.
//!
//! Coverage is read under the aLU simulation with the certificate's
//! bounds; for the deadlock-freedom item under the aM bisimulation
//! (`L = U = max(L, U)`), since the simulation does not preserve deadlocks.
//! A successor matches an edge by its transition identity, guard part,
//! horizon piece, post-state and automaton state, never by its transition
//! alone, and every successor is recomputed from the node's zone held to
//! its state's own invariants.
//!
//! Time divergence is checked by sufficient conditions the checker proves
//! itself, never by the search's evidence. The `admitted` path is
//! recomputed in exact zones from the initial state. Its cycle target lets
//! time advance when, under a tick source, it takes a tick, and under a
//! dense source, some clock it resets to `r` is guarded on it by `x >= c`
//! (or `x > c`) with `c > r` before its next reset, so each repetition
//! takes at least `c - r`; and it repeats forever: the greatest set of the
//! exact valuations at its start each of which takes the cycle back into
//! the set is not empty. A strongly connected set of nodes lets no time
//! diverge when every member is urgent, or some clock that no edge inside
//! it resets is bounded above by every member state's own time invariants.
//! A certificate whose divergence needs more than these is rejected, never
//! accepted on trust.
//!
//! Supported items are a TT-1 invariant (`always holds(p)`), a TT-2 window
//! claim, whose automaton the checker rebuilds with qsl-eval's own
//! translation (FR-242), any other infinite-trace or timed clause on
//! FR-240's liveness product, whose negation automaton it rebuilds the same
//! way, and the deadlock-freedom and time-lock-freedom items. A
//! certificate numbers its automaton states in its own order; the checker
//! binds each number to the state it recomputes, one to one.
//!
//! A timed refinement's safety half (QSpec FR-430) is checked in FR-244's
//! refinement form: the part's refinement is compared
//! with the recompiled one first; each node's second component is a
//! position, not an automaton state; `check_initial` and `check_step`,
//! run on the listed positions, replace the claim automaton and its
//! badness (`InitialMismatch`, `StepFails`, `PostDiffers`); the locked
//! region, entered only by a failing step into a `locked` entry node, is
//! read on concrete steps alone; and `admitted` starts at a positioned
//! initial node of the certificate's `initials` (QSpec FR-418).

mod divergence;
mod product;

use quire_exact::NodeKey;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use qsl_eval::automaton::{
    position_automaton, AutomatonError, ClockedEdge, Read, Stamp, WindowClaim, WindowRefusal,
};
use qsl_eval::fairness::{components, EdgeLabel, FairnessError, Graph, Identity};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{
    GuardedStep, ModelSimState, ModelSimulationRequest, ModelSystem, PlaceConstraint,
    TerminalClass, TimedSubjectRefusal, TimedSystem,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckedRefinement, ClockOp, FormulaNodeId, TemporalProfile, TimedPropertyForm,
};
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef};
use quire_exact::{Cancel, Integer, Meter, Rational};
use quire_semantic_value::model_transition::ModelTransition;

use super::closure::CertificateRejection;
use super::simulation::{position_matches, CertifiedPosition, EvaluationOutcome};
use super::zone::{LuBounds, Zone, ZoneError};
use super::{CertificateLocus, CertificateMember, CertificateRule};
use crate::model_check::identity::node_occurrence;
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::IncompleteCause;
use crate::refinement::{
    check_initial, check_step, open_systems, ConcreteStep, RefinementPosition, StepVerdict,
    SubjectRefusal, Taken,
};

/// FR-244's certificate of a zone search's proof of the item `obligation`
/// names (ADR-013 O-09).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneCertificate<O> {
    /// The item it proves.
    pub obligation: O,
    /// The scale factor of every zone bound (FR-238): a bound `c` stands
    /// for `c / scale` time units.
    pub scale: Integer,
    /// Each clock place's `L` and `U`, in scaled units: the search uses one
    /// pair per place for every discrete state.
    pub lu_bounds: LuBounds,
    /// The nodes, in discovery order.
    pub nodes: Vec<CertNode>,
    /// The edges, by source node in discovery order and, from each node, in
    /// canonical transition order: one per convex part of an enabled
    /// step's guard (spec gap STD-493: FR-244 names one edge per transition
    /// identity).
    pub edges: Vec<CertEdge>,
    /// QSpec FR-418 `initials`, nonempty: the node that covers each
    /// initial state's symbolic state, one per initial state that admits a
    /// valuation, in the subject's initial-state order. FR-418 names no
    /// `initial` member; FR-244's single initial node is `initials[0]`.
    pub initials: Vec<usize>,
    /// FR-244's Büchi part (ADR-026 CF-2): for a claim decided on FR-240's
    /// liveness product, each node's component and each component's
    /// reason; `None` for a reachability proof.
    pub components: Option<Components>,
    /// FR-244's divergence part: for the time-lock-freedom item, each
    /// node's path to a divergent target, by node; `None` otherwise.
    pub divergence: Option<Vec<DivergenceWitness>>,
    /// The witness that the subject admits a time-divergent behaviour
    /// (ADR-026 TD-6).
    pub admitted: DivergenceWitness,
    /// FR-244's refinement part, for the safety half of a timed
    /// refinement (QSpec FR-430); `None` otherwise.
    pub refinement: Option<RefinementPart>,
}

/// FR-244's refinement part: the reachability part is read over the
/// refinement product, each node's `automaton_state` an index into
/// `positions`, or `positions.len()` for a node of the locked region.
///
/// The concrete initial states' nodes are the certificate's own
/// [`ZoneCertificate::initials`] (QSpec FR-418): the part lists no
/// `initials` of its own.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementPart {
    /// QSpec FR-418 `refinement_node`: the refinement node's id.
    pub refinement_node: NodeKey,
    /// The product positions, FR-149's type.
    pub positions: Vec<CertifiedPosition>,
    /// What `check_step` took on each edge, one per entry of `edges` in
    /// order; `None` for an edge into or inside the locked region.
    pub taken: Vec<Option<Taken>>,
}

/// What FR-245 compares a refinement part's identity by.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementIdentity {
    /// The refinement node's id.
    pub refinement_node: NodeKey,
}

/// What a node of FR-240's liveness product carries beyond a search
/// node: the instance whose claim automaton it reads, and how it was
/// entered, which places it in the product's acceptance sets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductMark {
    /// The index of the `over` instance, in the subject's order.
    pub instance: usize,
    /// Entered by a position: the initial position, a step or a stutter.
    pub positioned: bool,
    /// Entered by the divergence clock's reset, or under a tick source by
    /// a tick.
    pub divergent: bool,
    /// In the idle tail (FR-231): the behaviour takes no further step.
    pub tail: bool,
}

/// A product edge that is no position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SilentMove {
    /// A delay into the next cell of the node's state (FR-240's split by
    /// the fairness guards).
    Wait,
    /// The reset of the divergence clock `z`, the zone's last clock, once
    /// it has reached 1 (the non-Zeno construction).
    Diverge,
}

/// A component numbering (FR-244's Büchi part): every edge between
/// numbered nodes goes to an equal or greater index, so every cycle lies in
/// one component.
///
/// Its `Clone`, `Debug`, `PartialEq` and `Drop` read nested numberings
/// without recursion: a crafted certificate may nest strong reasons deeper
/// than the stack (SR-1647001 FND-001, FND-004).
pub struct Components {
    /// Each node's component, by node; `None` for a node the numbering does
    /// not cover.
    pub numbering: Vec<Option<usize>>,
    /// Each component's reason, by index.
    pub reasons: Vec<ComponentReason>,
}

/// A reason of one level of a flattened [`Components`]: a leaf reason, or
/// a strong reason's constraints and the level of its nested numbering.
#[derive(Debug, PartialEq)]
enum LevelReason<'c> {
    Leaf(&'c ComponentReason),
    Strong(&'c [FairnessMember], usize),
}

/// One level of a flattened [`Components`]: its numbering and reasons.
type Level<'c> = (&'c [Option<usize>], Vec<LevelReason<'c>>);

impl Components {
    /// Every numbering of `self`, nested ones included, breadth first:
    /// level 0 is `self`, and a strong reason names the later level of its
    /// nested numbering. Read without recursion.
    fn levels(&self) -> Vec<Level<'_>> {
        let mut order: Vec<&Self> = vec![self];
        let mut levels = Vec::new();
        let mut at = 0;
        while let Some(&components) = order.get(at) {
            let mut reasons = Vec::with_capacity(components.reasons.len());
            for reason in &components.reasons {
                reasons.push(match reason {
                    ComponentReason::Strong {
                        constraints,
                        nested,
                    } => {
                        order.push(nested);
                        LevelReason::Strong(constraints, order.len() - 1)
                    }
                    leaf => LevelReason::Leaf(leaf),
                });
            }
            levels.push((components.numbering.as_slice(), reasons));
            at += 1;
        }
        levels
    }
}

impl Clone for Components {
    /// Rebuilt from its levels, deepest first.
    fn clone(&self) -> Self {
        let levels = self.levels();
        let mut built: Vec<Option<Self>> = (0..levels.len()).map(|_| None).collect();
        for (index, (numbering, reasons)) in levels.iter().enumerate().rev() {
            let reasons = reasons
                .iter()
                .map(|reason| match reason {
                    // A leaf holds no numbering: its own clone is shallow.
                    LevelReason::Leaf(leaf) => (*leaf).clone(),
                    LevelReason::Strong(constraints, level) => ComponentReason::Strong {
                        constraints: constraints.to_vec(),
                        nested: Box::new(built[*level].take().unwrap_or_else(|| Self {
                            numbering: Vec::new(),
                            reasons: Vec::new(),
                        })),
                    },
                })
                .collect();
            built[index] = Some(Self {
                numbering: numbering.to_vec(),
                reasons,
            });
        }
        built
            .first_mut()
            .and_then(Option::take)
            .unwrap_or_else(|| Self {
                numbering: Vec::new(),
                reasons: Vec::new(),
            })
    }
}

impl PartialEq for Components {
    fn eq(&self, other: &Self) -> bool {
        self.levels() == other.levels()
    }
}

impl Eq for Components {}

impl fmt::Debug for Components {
    /// Its levels, breadth first, each strong reason naming its nested
    /// numbering's level.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Components")
            .field("levels", &self.levels())
            .finish()
    }
}

impl Drop for Components {
    /// Drops nested numberings without recursion: a crafted certificate
    /// may nest strong reasons deeper than the stack (SR-1647001 FND-001).
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.reasons);
        while let Some(reason) = pending.pop() {
            if let ComponentReason::Strong { mut nested, .. } = reason {
                pending.append(&mut nested.reasons);
            }
        }
    }
}

/// Why a component holds no fair accepting cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentReason {
    /// No edge joins two of its nodes: it holds no cycle.
    Acyclic,
    /// None of its nodes is in the acceptance set.
    Lacks(AcceptanceSet),
    /// The weak fairness constraint is enabled at every valuation of every
    /// node and taken by no edge inside.
    Weak(FairnessMember),
    /// The strong fairness constraints are taken by no edge inside, and the
    /// component without the nodes that enable any of them at some
    /// valuation has the nested numbering, each of whose components has its
    /// own reason.
    Strong {
        /// The constraints.
        constraints: Vec<FairnessMember>,
        /// The numbering of the rest.
        nested: Box<Components>,
    },
}

/// An acceptance set of the liveness product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptanceSet {
    /// The claim automaton's acceptance set at this index.
    Automaton(usize),
    /// The nodes entered by the divergence clock's reset or a tick.
    Divergence,
    /// The nodes entered by a position.
    Position,
}

/// One fairness constraint of the clause: the constraint at `constraint`
/// in the clause's fairness set, and for an `each` constraint the one
/// transition identity it is about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FairnessMember {
    /// The constraint's index in the clause's fairness set.
    pub constraint: usize,
    /// The transition identity of an `each` constraint; `None` for a
    /// `whole` one.
    pub transition: Option<ModelTransition>,
}

/// One certificate node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertNode {
    /// The digest of the model state's state key.
    pub state_key: DigestRecord,
    /// The claim-automaton state; 0 for an item that needs no automaton.
    pub automaton_state: usize,
    /// The zone, in scaled units, over the reference clock, the clock
    /// places and, for a TT-2 claim, the horizon clock.
    pub zone: Zone,
    /// Whether the node violates the item only on time-locked valuations,
    /// which FR-239 continues past.
    pub locked: bool,
    /// For a node of FR-240's liveness product, its instance and entry.
    pub product: Option<ProductMark>,
}

/// One certificate edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertEdge {
    /// The source node.
    pub from: usize,
    /// The step's transition identity; `None` for an idle-tail stutter
    /// position of a TT-2 claim's automaton (FR-242).
    pub transition: Option<ModelTransition>,
    /// The convex part of the step's guard taken; `None` for a stutter.
    pub part: Option<usize>,
    /// Under a dense source, the piece of a TT-2 claim's horizon axis the
    /// position lies in, as constraints on the horizon clock; `None` with
    /// no automaton, past the formula's horizon, and under a tick source
    /// (spec gap STD-493: FR-244's edge names no piece).
    pub horizon: Option<Vec<PlaceConstraint>>,
    /// The node whose zone holds, or under the aLU simulation covers, the
    /// successor.
    pub to: usize,
    /// A liveness product's silent edge; `transition` and `part` are then
    /// `None`.
    pub silent: Option<SilentMove>,
}

/// A path of certificate edges to a divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivergenceWitness {
    /// The node the path starts at.
    pub start: usize,
    /// The edges of the path, by index.
    pub path: Vec<usize>,
    /// Where it ends.
    pub target: DivergentTarget,
}

/// A divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivergentTarget {
    /// Under a dense source, a quiescent node, from which time diverges by
    /// the idle tail.
    Quiescent(usize),
    /// Under a tick source, a terminal node, whose stutter step counts as
    /// a tick (FR-231 "Digital time").
    Terminal(usize),
    /// A cycle of edges from and back to the path's last node, each
    /// repetition of which lets time advance.
    Cycle(Vec<usize>),
}

/// FR-245's budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateCheckLimits {
    /// The most certificate edges a check reads.
    pub max_certificate_edges: u64,
    /// The most certificate bytes a check reads.
    pub max_certificate_bytes: u64,
    /// The most states of the claim automaton the checker rebuilds itself
    /// (ADR-018 LA-5): reaching it stops the check as EN-6's own
    /// `max_automaton_states` stops the search (SR-1647001 FND-002).
    pub max_automaton_states: u64,
}

impl Default for CertificateCheckLimits {
    fn default() -> Self {
        Self {
            max_certificate_edges: 1 << 24,
            max_certificate_bytes: 1 << 30,
            max_automaton_states: crate::model_check::limits::DEFAULT_MAX_AUTOMATON_STATES,
        }
    }
}

/// What the check needs beside the certificate: the subject, re-admitted
/// by the checker itself, the model whose timed reading the proof is over,
/// and the item.
pub struct ZoneCertificateRequest<'a> {
    /// FR-120's request of the subject.
    pub simulation: ModelSimulationRequest<'a>,
    /// The alias of the model whose time source and timed model the
    /// subject is read under (FR-230).
    pub model: &'a str,
    /// The bound on a state's live clocks the search ran under (QSpec
    /// FR-415, `model_check.max_clocks`).
    pub max_clocks: std::num::NonZeroU64,
    /// The item.
    pub item: CertifiedItem<'a>,
}

/// The item a zone certificate proves, as the checker recompiles it.
#[derive(Clone, Copy)]
pub enum CertifiedItem<'a> {
    /// An authored clause, its atoms read through `letters`.
    Clause {
        /// The checked clause.
        clause: &'a qsl_semantics::check::CheckedTemporalClause,
        /// The seam to the one clause evaluator for its `holds` atoms.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's derived deadlock-freedom item, read over time (FR-232).
    DeadlockFreedom,
    /// FR-232's derived time-lock-freedom item.
    TimeLockFreedom,
    /// The safety half of a timed refinement (QSpec FR-430), whose concrete
    /// subject is `simulation` and whose concrete model is `model`.
    Refinement {
        /// The checked refinement.
        refinement: &'a CheckedRefinement,
        /// The package holding the abstract model.
        abstract_package: &'a CheckedPackage,
        /// The abstract model's initial states.
        abstract_initial: &'a [DocumentRef],
    },
}

/// The item an obligation names, as the checker compares it with the
/// recompiled item (FR-245): an authored clause by its `claim` occurrence,
/// or a derived item by its kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateItem<'o> {
    /// An authored clause.
    Clause(&'o OccurrenceKey),
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// A refinement's record, against the abstract initial states.
    Refinement {
        /// The refinement's requirement occurrence (FR-144).
        occurrence: &'o OccurrenceKey,
        /// The abstract initial states.
        abstract_initial: &'o BTreeSet<DocumentRef>,
    },
    /// Any other item, which no zone certificate proves.
    Other,
}

/// An item's identity, as FR-245's identity refusal names it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemIdentity {
    /// An authored clause, by its `claim` occurrence.
    Clause(OccurrenceKey),
    /// A clause its package does not list, which has no occurrence.
    UnlistedClause,
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// A refinement's record, against the abstract initial states.
    Refinement {
        /// The refinement's requirement occurrence (FR-144).
        occurrence: OccurrenceKey,
        /// The abstract initial states.
        abstract_initial: BTreeSet<DocumentRef>,
    },
    /// Any other item.
    Other,
}

impl From<CertificateItem<'_>> for ItemIdentity {
    fn from(item: CertificateItem<'_>) -> Self {
        match item {
            CertificateItem::Clause(occurrence) => Self::Clause(occurrence.clone()),
            CertificateItem::DeadlockFreedom => Self::DeadlockFreedom,
            CertificateItem::TimeLockFreedom => Self::TimeLockFreedom,
            CertificateItem::Refinement {
                occurrence,
                abstract_initial,
            } => Self::Refinement {
                occurrence: occurrence.clone(),
                abstract_initial: abstract_initial.clone(),
            },
            CertificateItem::Other => Self::Other,
        }
    }
}

/// What [`check_zone_certificate`] returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateVerdict {
    /// Every check passed: the item settles `Proved{ZoneCertified,
    /// Certified}` (FR-235).
    Accepted,
    /// A rule failed: the item settles `inconclusive`,
    /// `CertificateRejected`.
    Rejected(CertificateRejection),
    /// A budget or a cancellation stopped the check (V-7).
    Stopped(IncompleteCause),
}

/// An invariant of the checker's own that a check broke.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckerFault {
    /// A zone operation met a zone of the wrong shape.
    ZoneShape,
    /// The fairness kernels refused the graph the checker built.
    Graph,
    /// A node read as reached has no model state.
    UnreachedNode,
    /// A valid edge has no successor it covers.
    EdgeWithoutSuccessor,
    /// A polled cancellation has no cause.
    CancellationWithoutCause,
    /// The claim automaton broke an invariant of its own.
    Automaton,
    /// `check_initial` or `check_step` returned an internal fault.
    RefinementStep,
}

/// Why [`check_zone_certificate`] checked nothing: the request, not the
/// certificate, is at fault. A refusal is never an acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateRefusal<O> {
    /// The certificate proves another item than `item`, or than the
    /// recompiled item: `stale_dependency`/`content-mismatch`, naming both.
    ObligationMismatch {
        /// The certificate's obligation.
        certificate: Box<O>,
        /// The item's.
        item: Box<O>,
    },
    /// The item the certificate names is not the recompiled item:
    /// `stale_dependency`/`content-mismatch`, naming both identities.
    IdentityMismatch {
        /// The identity the certificate's obligation names.
        certificate: Box<ItemIdentity>,
        /// The recompiled item's.
        recompiled: Box<ItemIdentity>,
    },
    /// The refinement part names another refinement than the
    /// recompiled one (FR-245 "Refinement form"):
    /// `stale_dependency`/`content-mismatch`, naming both.
    RefinementMismatch {
        /// The part's.
        certificate: Box<RefinementIdentity>,
        /// The recompiled.
        recompiled: Box<RefinementIdentity>,
    },
    /// FR-106's admission of the subject failed.
    Admission(Box<AdmissionFailure>),
    /// The model's timed reading does not build (FR-231).
    TimedSubject(Box<TimedSubjectRefusal>),
    /// The item is not one this checker reads yet (see the module).
    UnsupportedItem,
    /// The clause's instances are not finite.
    Unbounded,
    /// A reading the check needs has no value: a guard, invariant, urgency,
    /// expansion or atom.
    Undecided,
    /// The checker broke an invariant of its own.
    Fault(CheckerFault),
}

impl<O> ZoneCertificateRefusal<O> {
    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ObligationMismatch { .. }
            | Self::IdentityMismatch { .. }
            | Self::RefinementMismatch { .. } => Code::StaleDependency,
            Self::UnsupportedItem => Code::UnsupportedConstruct,
            Self::Fault(_) => Code::RuntimeInvariant,
            Self::Admission(_) | Self::TimedSubject(_) | Self::Unbounded | Self::Undecided => {
                Code::InvalidRuntimeInput
            }
        }
    }
}

/// How a check ends early.
enum Halt<O> {
    Verdict(ZoneCertificateVerdict),
    Refusal(ZoneCertificateRefusal<O>),
}

impl<O> From<ZoneError> for Halt<O> {
    fn from(_: ZoneError) -> Self {
        fault(CheckerFault::ZoneShape)
    }
}

type Step<T, O> = Result<T, Halt<O>>;

/// The verdict of a check whose rebuilt claim automaton reached
/// `max_automaton_states`, `limit`.
fn automaton_stop(limit: u64) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Stopped(IncompleteCause::LimitReached {
        limit: ModelCheckLimit::MaxAutomatonStates,
        value: limit,
    })
}

fn fault<O>(fault: CheckerFault) -> Halt<O> {
    Halt::Refusal(ZoneCertificateRefusal::Fault(fault))
}

fn reject<T, O>(rule: CertificateRule, at: CertificateLocus) -> Step<T, O> {
    Err(Halt::Verdict(ZoneCertificateVerdict::Rejected(
        CertificateRejection { rule, at },
    )))
}

fn malformed<T, O>(member: CertificateMember) -> Step<T, O> {
    reject(
        CertificateRule::Malformed,
        CertificateLocus::CertificateMember(member),
    )
}

fn at_node(node: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode {
        node: u64::try_from(node).unwrap_or(u64::MAX),
    }
}

fn undecided<T, O, E>(_: E) -> Step<T, O> {
    Err(Halt::Refusal(ZoneCertificateRefusal::Undecided))
}

fn malformed_verdict(member: CertificateMember) -> ZoneCertificateVerdict {
    ZoneCertificateVerdict::Rejected(CertificateRejection {
        rule: CertificateRule::Malformed,
        at: CertificateLocus::CertificateMember(member),
    })
}

fn at_edge(edge: usize) -> CertificateLocus {
    CertificateLocus::ZoneEdge {
        edge: u64::try_from(edge).unwrap_or(u64::MAX),
    }
}

/// Why a refinement's subjects do not open, as the check's refusal.
fn subject_refusal<O>(refusal: SubjectRefusal) -> ZoneCertificateRefusal<O> {
    match refusal {
        SubjectRefusal::Admission(failure) => ZoneCertificateRefusal::Admission(failure),
        SubjectRefusal::NoUniverse => ZoneCertificateRefusal::Unbounded,
        SubjectRefusal::NoModels
        | SubjectRefusal::ProtocolSubject
        | SubjectRefusal::ProtocolAbstractSide => ZoneCertificateRefusal::UnsupportedItem,
    }
}

/// A refinement form's check (FR-245 "Refinement form"): `check_step` in
/// place of the claim automaton, over the positions the part lists.
struct Refined<'c, 'a> {
    refinement: &'c CheckedRefinement,
    abstract_system: &'c ModelSystem<'a>,
    part: &'c RefinementPart,
}

/// FR-245: check `certificate` for `request`'s item, whose obligation is
/// `item` and names the item `item_of` reads from it, under `limits`,
/// polling `cancel` throughout.
///
/// # Errors
///
/// [`ZoneCertificateRefusal`] when the certificate names another item than
/// `item` or the recompiled one, the subject does not admit, its timed
/// reading does not build, the item is not supported, or a reading has no
/// value.
pub fn check_zone_certificate<O: Clone + Eq>(
    request: ZoneCertificateRequest<'_>,
    item: &O,
    item_of: impl Fn(&O) -> CertificateItem<'_>,
    certificate: &ZoneCertificate<O>,
    limits: CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<O>> {
    let mismatch = || ZoneCertificateRefusal::ObligationMismatch {
        certificate: Box::new(certificate.obligation.clone()),
        item: Box::new(item.clone()),
    };
    if certificate.obligation != *item {
        return Err(mismatch());
    }
    // The recompiled item's identity (FR-245): a clause's `claim`
    // occurrence in the package, or the derived item's kind.
    let named = ItemIdentity::from(item_of(&certificate.obligation));
    let recompiled = match request.item {
        CertifiedItem::Clause { clause, .. } => request
            .simulation
            .package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause() == clause)
            .map_or(ItemIdentity::UnlistedClause, |node| {
                ItemIdentity::Clause(node_occurrence(node))
            }),
        CertifiedItem::DeadlockFreedom => ItemIdentity::DeadlockFreedom,
        CertifiedItem::TimeLockFreedom => ItemIdentity::TimeLockFreedom,
        CertifiedItem::Refinement {
            refinement,
            abstract_initial,
            ..
        } => refinement
            .requirement_occurrence()
            .map_or(ItemIdentity::Other, |occurrence| ItemIdentity::Refinement {
                occurrence: occurrence.clone(),
                abstract_initial: abstract_initial.iter().cloned().collect(),
            }),
    };
    if named != recompiled || recompiled == ItemIdentity::Other {
        return Err(ZoneCertificateRefusal::IdentityMismatch {
            certificate: Box::new(named),
            recompiled: Box::new(recompiled),
        });
    }
    // FR-245 "Refinement form": both packages recompiled, the part's
    // identity compared before any system is opened.
    let opened;
    let system;
    let (concrete, refined) = match request.item {
        CertifiedItem::Refinement {
            refinement,
            abstract_package,
            abstract_initial,
        } => {
            let Some(part) = &certificate.refinement else {
                return Ok(malformed_verdict(CertificateMember::Refinement));
            };
            // The refinement node is the recompiled record's occurrence's,
            // which the identity check above found.
            let ItemIdentity::Refinement { occurrence, .. } = &recompiled else {
                return Err(ZoneCertificateRefusal::IdentityMismatch {
                    certificate: Box::new(named),
                    recompiled: Box::new(recompiled),
                });
            };
            let recompiled = RefinementIdentity {
                refinement_node: occurrence.node().clone(),
            };
            let listed = RefinementIdentity {
                refinement_node: part.refinement_node.clone(),
            };
            if listed != recompiled {
                return Err(ZoneCertificateRefusal::RefinementMismatch {
                    certificate: Box::new(listed),
                    recompiled: Box::new(recompiled),
                });
            }
            opened = open_systems(
                refinement,
                &request.simulation,
                abstract_package,
                abstract_initial.to_vec(),
            )
            .map_err(subject_refusal)?;
            (
                &opened.concrete,
                Some(Refined {
                    refinement,
                    abstract_system: &opened.abstract_system,
                    part,
                }),
            )
        }
        _ => {
            system = ModelSystem::new(request.simulation)
                .map_err(|failure| ZoneCertificateRefusal::Admission(Box::new(failure)))?;
            (&system, None)
        }
    };
    let timed = TimedSystem::with_max_clocks(concrete, request.model, request.max_clocks)
        .map_err(|refusal| ZoneCertificateRefusal::TimedSubject(Box::new(refusal)))?;
    let mut live = None;
    let (claim, window) = match request.item {
        CertifiedItem::Clause { clause, letters } => {
            if !matches!(
                clause.profile(),
                TemporalProfile::InfiniteTrace | TemporalProfile::Timed
            ) {
                return Err(ZoneCertificateRefusal::UnsupportedItem);
            }
            let instances = concrete
                .instances(&clause.over().1)
                .map_err(|_| ZoneCertificateRefusal::Unbounded)?;
            let atom = qsl_eval::automaton::invariant_atom(clause);
            // A TT-2 window nesting no interval operator under another has
            // FR-242's horizon automaton; any other clause is read on the
            // liveness product.
            let flat = if atom.is_none()
                && matches!(
                    clause.timed_form(),
                    Some(TimedPropertyForm::BoundedWindow { .. })
                ) {
                match WindowClaim::new(
                    clause.formula(),
                    instances.len(),
                    timed.tick_period().cloned(),
                    limits.max_automaton_states,
                ) {
                    Ok(claim) => Some(claim),
                    Err(WindowRefusal::NestedWindow) => None,
                    Err(_) => return Err(ZoneCertificateRefusal::UnsupportedItem),
                }
            } else {
                None
            };
            let window = if atom.is_some() {
                None
            } else if let Some(claim) = flat {
                let pieces = match timed.tick_period() {
                    None => claim.pieces(timed.places().len()),
                    Some(_) => vec![(Vec::new(), Rational::from_integer(Integer::zero()))],
                };
                Some(Window {
                    claim: RefCell::new(claim),
                    pieces,
                })
            } else {
                // FR-240: any other clause, on the liveness product.
                let mut automaton = match position_automaton(clause, limits.max_automaton_states) {
                    Ok(automaton) => automaton,
                    Err(AutomatonError::StateLimit { limit }) => return Ok(automaton_stop(limit)),
                    Err(
                        AutomatonError::ProfileNotTranslated { .. }
                        | AutomatonError::RepeatedPunctualInterval
                        | AutomatonError::ClockLimit { .. },
                    ) => return Err(ZoneCertificateRefusal::UnsupportedItem),
                    Err(_) => return Err(ZoneCertificateRefusal::Fault(CheckerFault::Automaton)),
                };
                let start = match automaton.initial() {
                    Ok(start) => start,
                    Err(AutomatonError::StateLimit { limit }) => return Ok(automaton_stop(limit)),
                    Err(_) => return Err(ZoneCertificateRefusal::Fault(CheckerFault::Automaton)),
                };
                let mut operations = Vec::new();
                for constraint in clause.fairness() {
                    let operation = constraint.operation();
                    let text = concrete
                        .operation_text(operation.declaring_type(), operation.name())
                        .ok_or(ZoneCertificateRefusal::Undecided)?;
                    operations.push(text.to_owned());
                }
                live = Some(product::LiveClaim::new(
                    automaton,
                    start,
                    letters,
                    instances.clone(),
                    clause.capture_site(),
                    clause.fairness().to_vec(),
                    operations,
                ));
                None
            };
            (
                Some(Claim {
                    atom,
                    letters,
                    instances,
                }),
                window,
            )
        }
        CertifiedItem::DeadlockFreedom
        | CertifiedItem::TimeLockFreedom
        | CertifiedItem::Refinement { .. } => (None, None),
    };
    let time_lock = matches!(request.item, CertifiedItem::TimeLockFreedom);
    let count = certificate.nodes.len();
    let mut checker = Checker {
        timed: &timed,
        claim,
        window,
        live,
        time_lock,
        refined,
        certificate,
        cancel,
        cover: certificate.lu_bounds.clone(),
        leaving: vec![Vec::new(); count],
        states: vec![None; count],
        successors: (0..count).map(|_| None).collect(),
        covers: vec![None; certificate.edges.len()],
        bound: BTreeMap::new(),
        binds: BTreeMap::new(),
        known: Vec::new(),
        refinement_failures: BTreeMap::new(),
        read: Cell::new(0),
        max_edges: limits.max_certificate_edges,
        max_automaton_states: limits.max_automaton_states,
    };
    match checker.verify(limits) {
        Ok(()) => Ok(ZoneCertificateVerdict::Accepted),
        Err(Halt::Verdict(verdict)) => Ok(verdict),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

/// A clause: its TT-1 atom, or `None` for a TT-2 clause its automaton
/// reads; each atom read through `letters` for each instance.
struct Claim<'a> {
    atom: Option<FormulaNodeId>,
    letters: &'a dyn ModelLetters,
    instances: Vec<ModelInstance>,
}

/// A TT-2 clause's automaton, qsl-eval's own (FR-242), and the pieces of
/// the horizon axis its positions are read on.
struct Window {
    claim: RefCell<WindowClaim>,
    pieces: Vec<(Vec<PlaceConstraint>, Rational)>,
}

/// One recomputed symbolic successor of a node.
struct Successor {
    /// The step; `None` for an idle-tail stutter position.
    step: Option<GuardedStep>,
    part: Option<usize>,
    /// The horizon piece a dense TT-2 position lies in.
    horizon: Option<Vec<PlaceConstraint>>,
    /// Whether the successor frees the horizon clock: past the point where
    /// the claim is decided.
    frees: bool,
    digest: DigestRecord,
    /// The checker's automaton state.
    q: usize,
    zone: Zone,
    /// A liveness product's silent move.
    silent: Option<SilentMove>,
    /// A liveness product successor's instance and entry.
    mark: Option<ProductMark>,
    /// A liveness product successor's cell, over every clock of a node.
    cell: Option<Zone>,
    /// A liveness product position's claim-automaton transition.
    read: Option<ClockedEdge>,
}

fn equivalent_successor(left: &Successor, right: &Successor) -> bool {
    left.step.as_ref().map(GuardedStep::transition)
        == right.step.as_ref().map(GuardedStep::transition)
        && left.part == right.part
        && left.horizon == right.horizon
        && left.digest == right.digest
        && left.q == right.q
        && left.zone == right.zone
        && left.silent == right.silent
        && left.mark == right.mark
        && left.cell == right.cell
        && left.read == right.read
}

struct Checker<'c, 'a, O> {
    timed: &'c TimedSystem<'c, 'a>,
    claim: Option<Claim<'c>>,
    window: Option<Window>,
    /// A clause decided on FR-240's liveness product.
    live: Option<product::LiveClaim<'c>>,
    /// Whether the item is time-lock freedom, with its divergence part.
    time_lock: bool,
    /// A refinement form's check.
    refined: Option<Refined<'c, 'a>>,
    certificate: &'c ZoneCertificate<O>,
    cancel: &'c Cancel,
    /// The bounds coverage is read under: the certificate's, or for the
    /// deadlock-freedom item `L = U = max(L, U)` per clock, the aM
    /// bisimulation, since the aLU simulation does not preserve deadlocks.
    cover: LuBounds,
    /// The edges leaving each node, by index, in edge order.
    leaving: Vec<Vec<usize>>,
    /// The model state of each node an edge reaches from an initial node.
    states: Vec<Option<ModelSimState>>,
    /// Each reached node's recomputed successors.
    successors: Vec<Option<Vec<Successor>>>,
    /// For each valid edge, the successor of its source it covers.
    covers: Vec<Option<usize>>,
    /// The checker's automaton state each certificate automaton state
    /// names, and back: a certificate numbers states in its own order.
    bound: BTreeMap<usize, usize>,
    binds: BTreeMap<usize, usize>,
    /// A refinement form's position at each index of `positions`, once a
    /// passing initial state or step yields it.
    known: Vec<Option<RefinementPosition>>,
    /// A refinement form's first rejection out of each node, by node,
    /// reported in node order before the node's own checks.
    refinement_failures: BTreeMap<usize, CertificateRejection>,
    /// The certificate edges the check has read, against
    /// `max_certificate_edges`: each listed edge once, and each edge of a
    /// divergence cycle again at each round of its repetition check.
    read: Cell<u64>,
    max_edges: u64,
    max_automaton_states: u64,
}

impl<O> Checker<'_, '_, O> {
    // Not named `check`: CK-1's checked-input gate reads any `.check(..)`
    // call in a stage crate as the S3 stage constructor (TC-745).
    fn verify(&mut self, limits: CertificateCheckLimits) -> Step<(), O> {
        self.poll()?;
        self.budgets(limits)?;
        self.members()?;
        if self.claim.is_none() && self.refined.is_none() {
            // FR-232's deadlocks are not preserved by the aLU simulation:
            // coverage is read under the aM bisimulation.
            for (lower, upper) in self.cover.lower.iter_mut().zip(self.cover.upper.iter_mut()) {
                let most = lower.clone().max(upper.clone());
                lower.clone_from(&most);
                *upper = most;
            }
        }
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            self.leaving[edge.from].push(index);
        }
        self.initial()?;
        self.reach()?;
        let divergent = self.divergent()?;
        for node in 0..self.certificate.nodes.len() {
            self.poll()?;
            if let Some(rejection) = self.refinement_failures.remove(&node) {
                return Err(Halt::Verdict(ZoneCertificateVerdict::Rejected(rejection)));
            }
            if self.states[node].is_some() {
                self.node(node, &divergent)?;
            }
        }
        if self.live.is_some() {
            self.components()?;
        }
        if self.time_lock {
            self.divergence_part()?;
        }
        self.admitted()
    }

    /// Charge `count` edge reads against `max_certificate_edges`.
    fn charge(&self, count: u64) -> Step<(), O> {
        self.poll()?;
        let read = self.read.get().saturating_add(count);
        if read > self.max_edges {
            return Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::CertificateMaxEdges,
                    value: self.max_edges,
                },
            )));
        }
        self.read.set(read);
        Ok(())
    }

    fn poll(&self) -> Step<(), O> {
        if self.cancel.poll() {
            let Some(source) = self.cancel.cause() else {
                return Err(fault(CheckerFault::CancellationWithoutCause));
            };
            return Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::Cancelled { source },
            )));
        }
        Ok(())
    }

    /// `max_certificate_edges` and `max_certificate_bytes`, before any
    /// recomputation.
    fn budgets(&self, limits: CertificateCheckLimits) -> Step<(), O> {
        let stop = |limit, value| {
            Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached { limit, value },
            )))
        };
        let edges = u64::try_from(self.certificate.edges.len()).unwrap_or(u64::MAX);
        self.read.set(edges);
        if edges > limits.max_certificate_edges {
            return stop(
                ModelCheckLimit::CertificateMaxEdges,
                limits.max_certificate_edges,
            );
        }
        if certificate_bytes(self.certificate) > limits.max_certificate_bytes {
            return stop(
                ModelCheckLimit::CertificateMaxBytes,
                limits.max_certificate_bytes,
            );
        }
        Ok(())
    }

    /// The zone dimension: the reference clock, the clock places and, for
    /// a TT-2 claim, the horizon clock.
    fn dim(&self) -> usize {
        let z = self.live.is_some() && self.timed.tick_period().is_none();
        self.timed.places().len()
            + 1
            + usize::from(self.window.is_some())
            + self.automaton_clocks()
            + usize::from(z)
    }

    /// The horizon clock's index in a zone.
    fn horizon_clock(&self) -> usize {
        self.timed.places().len() + 1
    }

    /// The `Malformed` members, in member order.
    fn members(&self) -> Step<(), O> {
        let certificate = self.certificate;
        if certificate.scale <= Integer::zero() {
            return malformed(CertificateMember::Scale);
        }
        let constants = self
            .window
            .as_ref()
            .map(|window| window.claim.borrow().constants().to_vec());
        let Some((mut lower, mut upper)) =
            computed_bounds(self.timed, constants.as_deref(), &certificate.scale)
        else {
            return malformed(CertificateMember::Scale);
        };
        if self.live.is_some() {
            // The liveness product's cells compare clocks with guard
            // constants from both sides, which the aLU simulation keeps
            // only under `L = U = max(L, U)`; `z` is compared with 1.
            for (low, high) in lower.iter_mut().zip(upper.iter_mut()) {
                let most = low.clone().max(high.clone());
                low.clone_from(&most);
                *high = most;
            }
            // Each claim-automaton clock is compared with its interval's
            // ends, which are exact at the scale (FR-242).
            let (constants, every) = self.live.as_ref().map_or_else(Default::default, |live| {
                let automaton = live.automaton.borrow();
                (automaton.constants(), automaton.all_constants())
            });
            for value in &every {
                if self.scaled(value).is_err() {
                    return malformed(CertificateMember::Scale);
                }
            }
            for constant in &constants {
                let Ok(bound) = self.scaled(constant) else {
                    return malformed(CertificateMember::Scale);
                };
                lower.push(Some(bound.clone()));
                upper.push(Some(bound));
            }
            if self.timed.tick_period().is_none() {
                lower.push(Some(certificate.scale.clone()));
                upper.push(Some(certificate.scale.clone()));
            }
        }
        let at_least = |listed: &[Option<Integer>], computed: &[Option<Integer>]| {
            listed.len() == computed.len()
                && listed
                    .iter()
                    .zip(computed)
                    .all(|(listed, computed)| match (listed, computed) {
                        (_, None) => true,
                        (None, Some(_)) => false,
                        (Some(listed), Some(computed)) => listed >= computed,
                    })
        };
        if !at_least(&certificate.lu_bounds.lower, &lower)
            || !at_least(&certificate.lu_bounds.upper, &upper)
        {
            return malformed(CertificateMember::Bounds);
        }
        // A refinement form's part: present exactly for a refinement item,
        // with its positions and one `taken` per edge.
        let positions = match (&self.refined, &certificate.refinement) {
            (Some(refined), _) => {
                if refined.part.positions.is_empty() {
                    return malformed(CertificateMember::Positions);
                }
                if refined.part.taken.len() != certificate.edges.len() {
                    return malformed(CertificateMember::Edges);
                }
                Some(refined.part.positions.len())
            }
            (None, Some(_)) => return malformed(CertificateMember::Refinement),
            (None, None) => None,
        };
        let nodes = certificate.nodes.len();
        let windowed = self.window.is_some();
        let live = self.live.as_ref();
        let node_ok = |node: &CertNode| {
            node.zone.dim() == self.dim()
                && match positions {
                    // Each node's position, or the locked region's index.
                    Some(reserved) => node.automaton_state <= reserved,
                    None => windowed || live.is_some() || node.automaton_state == 0,
                }
                && match (live, node.product) {
                    (Some(live), Some(mark)) => mark.instance < live.instances.len(),
                    (None, None) => true,
                    _ => false,
                }
        };
        // QSpec FR-418: `initials` is nonempty.
        if certificate.initials.is_empty() {
            return malformed(CertificateMember::Initials);
        }
        if !certificate.nodes.iter().all(node_ok)
            || certificate.initials.iter().any(|&node| node >= nodes)
            || certificate.admitted.start >= nodes
        {
            return malformed(CertificateMember::Nodes);
        }
        // The Büchi part for a liveness product, the divergence part for
        // the time-lock-freedom item, each shaped to the nodes. The
        // time-lock item carries no component part, which FR-244's Büchi
        // part text names (spec gap STD-493, comment 8eafddf9, item 1;
        // SR-1647002 FND-003): its exact fixpoint is the stronger check.
        match (&certificate.components, live) {
            (Some(components), Some(_)) if numbering_ok(components, nodes) => {}
            (None, None) => {}
            _ => return malformed(CertificateMember::Components),
        }
        let edges = certificate.edges.len();
        let dense = self.timed.tick_period().is_none();
        let edge_ok = |edge: &CertEdge| {
            let shaped = match (&edge.transition, edge.part, edge.silent) {
                (Some(_), Some(_), None) => true,
                (None, None, None) => windowed || live.is_some(),
                (None, None, Some(_)) => live.is_some(),
                _ => false,
            };
            edge.from < nodes
                && edge.to < nodes
                && shaped
                && (edge.horizon.is_none() || (windowed && dense))
        };
        let target_ok = match &certificate.admitted.target {
            DivergentTarget::Quiescent(node) | DivergentTarget::Terminal(node) => *node < nodes,
            DivergentTarget::Cycle(cycle) => cycle.iter().all(|&edge| edge < edges),
        };
        if !certificate.edges.iter().all(edge_ok)
            || certificate.admitted.path.iter().any(|&edge| edge >= edges)
            || !target_ok
        {
            return malformed(CertificateMember::Edges);
        }
        let witness_ok = |witness: &DivergenceWitness| {
            witness.start < nodes
                && witness.path.iter().all(|&edge| edge < edges)
                && match &witness.target {
                    DivergentTarget::Quiescent(node) | DivergentTarget::Terminal(node) => {
                        *node < nodes
                    }
                    DivergentTarget::Cycle(cycle) => cycle.iter().all(|&edge| edge < edges),
                }
        };
        match (&certificate.divergence, self.time_lock) {
            (Some(witnesses), true)
                if witnesses.len() == nodes && witnesses.iter().all(witness_ok) => {}
            (None, false) => {}
            _ => return malformed(CertificateMember::Divergence),
        }
        Ok(())
    }

    /// The scaled integer of `value`; every constant of the model and the
    /// claim was checked integral under the scale in [`Self::members`].
    fn scaled(&self, value: &Rational) -> Step<Integer, O> {
        match scaled(value, &self.certificate.scale) {
            Some(value) => Ok(value),
            None => malformed(CertificateMember::Scale),
        }
    }

    fn apply_clock_updates(&self, zone: &mut Zone, step: &GuardedStep) -> Step<(), O> {
        let carries: Vec<_> = step
            .carries()
            .iter()
            .map(|(target, source)| (target + 1, source + 1))
            .collect();
        let resets: Vec<_> = step
            .resets()
            .iter()
            .map(|(place, value)| Ok((place + 1, self.scaled(value)?)))
            .collect::<Step<_, O>>()?;
        zone.assign_clocks(&carries, &resets)?;
        Ok(())
    }

    fn preimage_clock_updates(&self, zone: &Zone, step: &GuardedStep) -> Step<Zone, O> {
        let carries: Vec<_> = step
            .carries()
            .iter()
            .map(|(target, source)| (target + 1, source + 1))
            .collect();
        let resets: Vec<_> = step
            .resets()
            .iter()
            .map(|(place, value)| Ok((place + 1, self.scaled(value)?)))
            .collect::<Step<_, O>>()?;
        Ok(zone.preimage_clocks(&carries, &resets)?)
    }

    fn constrain(&self, zone: &mut Zone, constraints: &[PlaceConstraint]) -> Step<(), O> {
        for constraint in constraints {
            let clock = constraint.place() + 1;
            let bound = self.scaled(constraint.bound())?;
            let below = bound.neg();
            match constraint.op() {
                ClockOp::Less => zone.constrain(clock, 0, &bound, true)?,
                ClockOp::LessOrEqual => zone.constrain(clock, 0, &bound, false)?,
                ClockOp::Equal => {
                    zone.constrain(clock, 0, &bound, false)?;
                    zone.constrain(0, clock, &below, false)?;
                }
                ClockOp::GreaterOrEqual => zone.constrain(0, clock, &below, false)?,
                ClockOp::Greater => zone.constrain(0, clock, &below, true)?,
            }
        }
        Ok(())
    }

    fn invariants(&self, state: &ModelSimState) -> Step<Vec<PlaceConstraint>, O> {
        self.timed.invariants(state).or_else(undecided)
    }

    /// Whether time may elapse at `state`.
    fn elapses(&self, state: &ModelSimState) -> Step<bool, O> {
        Ok(
            self.timed.tick_period().is_none()
                && !self.timed.is_urgent(state).or_else(undecided)?,
        )
    }

    /// `zone` within `state`'s invariants, time let elapse unless it is
    /// urgent, and within them again.
    fn settle(&self, state: &ModelSimState, zone: &mut Zone) -> Step<(), O> {
        let invariants = self.invariants(state)?;
        self.constrain(zone, &invariants)?;
        if !zone.is_empty() && self.elapses(state)? {
            zone.up();
            self.constrain(zone, &invariants)?;
        }
        Ok(())
    }

    fn digest(&self, state: &ModelSimState) -> Step<DigestRecord, O> {
        self.timed
            .system()
            .state_key(state)
            .map(|(_, digest)| digest)
            .or_else(undecided)
    }

    /// The automaton state a position at `state` reads: position 0 when
    /// `from` is `None`.
    fn read(
        &self,
        from: Option<usize>,
        state: &ModelSimState,
        stamp: Stamp<'_>,
        stutter: bool,
    ) -> Step<usize, O> {
        let (Some(window), Some(claim)) = (&self.window, &self.claim) else {
            return Ok(0);
        };
        let limits = self.timed.system().meter_limits();
        let mut atoms = |instance: usize, atom| {
            let instance = claim
                .instances
                .get(instance)
                .ok_or_else(|| "window-instance".to_owned())?;
            claim
                .letters
                .atom(atom, instance, state, &mut Meter::new(limits))
                .map_err(|stop| format!("{stop:?}"))
        };
        let mut automaton = window.claim.borrow_mut();
        let read = match from {
            None => automaton.initial(&mut atoms),
            Some(q) => automaton.step(q, stamp, stutter, &mut atoms),
        };
        match read {
            Read::State(q) => Ok(q),
            Read::Atom(_) => Err(Halt::Refusal(ZoneCertificateRefusal::Undecided)),
            Read::AutomatonStates => Err(Halt::Verdict(automaton_stop(self.max_automaton_states))),
            Read::Fault(_) => Err(fault(CheckerFault::Automaton)),
        }
    }

    /// Whether the certificate's automaton state `listed` names the
    /// checker's `q`, binding the two when neither is bound yet.
    fn binds(&mut self, listed: usize, q: usize) -> bool {
        if self.refined.is_some() {
            // A refinement form's second component is a position, which
            // `check_step` judges on the edge, not an automaton state.
            return true;
        }
        if self.window.is_none() && self.live.is_none() {
            return listed == 0 && q == 0;
        }
        match (self.bound.get(&listed), self.binds.get(&q)) {
            (Some(&known), _) => known == q,
            (None, Some(_)) => false,
            (None, None) => {
                self.bound.insert(listed, q);
                self.binds.insert(q, listed);
                true
            }
        }
    }

    /// Each initial symbolic state is covered by a named initial node of
    /// its discrete state and claim-automaton state.
    fn initial(&mut self) -> Step<(), O> {
        if self.live.is_some() {
            return self.live_initial_nodes();
        }
        if self.refined.is_some() {
            return self.refined_initial();
        }
        let certificate = self.certificate;
        for start in self.timed.initial() {
            self.poll()?;
            let state = start.discrete().clone();
            let digest = self.digest(&state)?;
            let zero = Rational::from_integer(Integer::zero());
            let q = self.read(None, &state, Stamp::At(&zero), false)?;
            let mut zone = Zone::zero(self.dim());
            self.settle(&state, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            let mut covered = None;
            for &node in &certificate.initials {
                let listed = &certificate.nodes[node];
                if listed.state_key == digest
                    && zone.alu_covered_by(&listed.zone, &self.cover)?
                    && self.binds(listed.automaton_state, q)
                {
                    covered = Some(node);
                    break;
                }
            }
            let Some(node) = covered else {
                return reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(digest),
                );
            };
            self.states[node].get_or_insert(state);
        }
        Ok(())
    }

    /// A refinement form's initial nodes (FR-245 "Refinement form"): each
    /// concrete initial state's symbolic state is covered by its listed
    /// node (`InitialMissing`), which carries the position `check_initial`
    /// yields, or, when it fails, is a `locked` entry node with the reserved
    /// index (`InitialMismatch`). Whether a `locked` node is time-locked is
    /// the `BadState` rule's, read with the other nodes.
    fn refined_initial(&mut self) -> Step<(), O> {
        let Some(Refined {
            refinement,
            abstract_system,
            part,
        }) = self.refined
        else {
            return Ok(());
        };
        let certificate = self.certificate;
        let reserved = part.positions.len();
        self.known = vec![None; reserved];
        let mut listed = certificate.initials.iter();
        for (index, start) in self.timed.initial().iter().enumerate() {
            self.poll()?;
            let state = start.discrete().clone();
            let digest = self.digest(&state)?;
            let mut zone = Zone::zero(self.dim());
            self.settle(&state, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            let covering = match listed.next() {
                Some(&node) => {
                    let entry = &certificate.nodes[node];
                    (entry.state_key == digest && zone.alu_covered_by(&entry.zone, &self.cover)?)
                        .then_some(node)
                }
                None => None,
            };
            let Some(node) = covering else {
                return reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(digest),
                );
            };
            let entry = &certificate.nodes[node];
            let verdict = check_initial(
                refinement,
                self.timed.system(),
                abstract_system,
                index,
                state.state(),
            );
            let holds = match verdict {
                StepVerdict::Passes { post, .. } => {
                    let holds = !entry.locked
                        && entry.automaton_state < reserved
                        && self.position_is(entry.automaton_state, &post)?;
                    if holds {
                        self.know(entry.automaton_state, post);
                    }
                    holds
                }
                failed => {
                    self.unsound(&failed)?;
                    entry.locked && entry.automaton_state == reserved
                }
            };
            if !holds {
                return reject(CertificateRule::InitialMismatch, at_node(node));
            }
            self.states[node].get_or_insert(state);
        }
        Ok(())
    }

    /// A refinement form's judgement of edge `index` out of `node`, by
    /// `step`: out of a positioned node, `check_step` passes with the
    /// target's position and the edge's `taken` (`PostDiffers`), or fails
    /// into a `locked` entry node (`StepFails`); out of the locked region,
    /// the edge stays in it with no `taken` (`PostDiffers`). The first
    /// rejection out of a node is kept for the node's turn.
    fn refine_edge(&mut self, node: usize, index: usize, step: &GuardedStep) -> Step<(), O> {
        let Some(Refined {
            refinement,
            abstract_system,
            part,
        }) = self.refined
        else {
            return Ok(());
        };
        let certificate = self.certificate;
        let reserved = part.positions.len();
        let source = &certificate.nodes[node];
        let edge = &certificate.edges[index];
        let target = &certificate.nodes[edge.to];
        let taken = part.taken.get(index).and_then(Option::as_ref);
        let entry = target.locked && target.automaton_state == reserved;
        let rule = if source.automaton_state == reserved {
            // The locked region, on concrete steps only.
            (taken.is_some() || target.automaton_state != reserved)
                .then_some(CertificateRule::PostDiffers)
        } else {
            let Some(pre) = self.known.get(source.automaton_state).cloned().flatten() else {
                // A node entered by a rejected edge has no position; that
                // edge's rejection decides the check.
                return Ok(());
            };
            let verdict = check_step(
                refinement,
                self.timed.system(),
                abstract_system,
                &pre,
                &ConcreteStep {
                    position: u64::try_from(index).unwrap_or(u64::MAX),
                    transition: Some(step.transition()),
                    post: step.post(),
                },
            )
            .verdict;
            match verdict {
                StepVerdict::Passes {
                    post,
                    taken: yielded,
                } => {
                    let holds = !target.locked
                        && target.automaton_state < reserved
                        && taken == Some(&yielded)
                        && self.position_is(target.automaton_state, &post)?;
                    if holds {
                        self.know(target.automaton_state, post);
                    }
                    (!holds).then_some(CertificateRule::PostDiffers)
                }
                failed => {
                    self.unsound(&failed)?;
                    if !entry {
                        Some(CertificateRule::StepFails)
                    } else if taken.is_some() {
                        Some(CertificateRule::PostDiffers)
                    } else {
                        None
                    }
                }
            }
        };
        if let Some(rule) = rule {
            self.refinement_failures
                .entry(node)
                .or_insert(CertificateRejection {
                    rule,
                    at: at_edge(index),
                });
        }
        Ok(())
    }

    /// Whether the part's position `at` is `position`.
    fn position_is(&self, at: usize, position: &RefinementPosition) -> Step<bool, O> {
        let Some(Refined {
            abstract_system,
            part,
            ..
        }) = self.refined
        else {
            return Ok(false);
        };
        let Some(listed) = part.positions.get(at) else {
            return Ok(false);
        };
        position_matches(self.timed.system(), abstract_system, listed, position).or_else(undecided)
    }

    /// Record the position `at` holds, once.
    fn know(&mut self, at: usize, position: RefinementPosition) {
        if let Some(slot) = self.known.get_mut(at) {
            slot.get_or_insert(position);
        }
    }

    /// A failing verdict's end of the check when it is no finding: a run
    /// stop, a fault, or an `Undetermined` that only ran out of the
    /// evaluation meter, which stops the check (V-7), never rejects it.
    fn unsound(&self, verdict: &StepVerdict) -> Step<(), O> {
        let stop = |limit, value| {
            Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached { limit, value },
            )))
        };
        match verdict {
            StepVerdict::Stopped(reached) => stop(reached.limit, reached.value),
            StepVerdict::Fault(_) => Err(fault(CheckerFault::RefinementStep)),
            StepVerdict::Undetermined(undetermined) => {
                match EvaluationOutcome::of_undetermined(undetermined).meter_counter() {
                    Some(counter) => stop(
                        ModelCheckLimit::EvaluationMeter { counter },
                        crate::meter_bound(&self.timed.system().meter(), counter),
                    ),
                    None => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    /// The liveness product's initial nodes: for each initial state and
    /// instance, each position-0 successor is covered by a named initial
    /// node of its discrete state, automaton state, mark and cell.
    fn live_initial_nodes(&mut self) -> Step<(), O> {
        let certificate = self.certificate;
        for start in self.timed.initial() {
            self.poll()?;
            let state = start.discrete().clone();
            let digest = self.digest(&state)?;
            let missing = || {
                reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(digest),
                )
            };
            let Some(starts) = self.live_initial(&state)? else {
                return missing();
            };
            for (_, q, mark, zone) in starts {
                let cell = self.cell_zone_of(&state, &zone)?;
                let mut covered = None;
                for &node in &certificate.initials {
                    let listed = &certificate.nodes[node];
                    if listed.state_key == digest
                        && listed.product == Some(mark)
                        && zone.alu_covered_by(&listed.zone, &self.cover)?
                        && cell.includes(&listed.zone)?
                        && self.binds(listed.automaton_state, q)
                    {
                        covered = Some(node);
                        break;
                    }
                }
                let Some(node) = covered else {
                    return missing();
                };
                self.states[node].get_or_insert(state.clone());
            }
        }
        Ok(())
    }

    /// The model state of every node an edge reaches from an initial node,
    /// each reached node's successors, and for each edge the successor it
    /// covers: of its transition identity, guard part and horizon piece,
    /// whose post-state, automaton state and zone its target holds.
    fn reach(&mut self) -> Step<(), O> {
        let certificate = self.certificate;
        let mut queue: Vec<usize> = (0..self.states.len())
            .filter(|&node| self.states[node].is_some())
            .collect();
        let mut next = 0;
        while let Some(&node) = queue.get(next) {
            next += 1;
            self.poll()?;
            let Some(state) = self.states[node].clone() else {
                continue;
            };
            let listed = &certificate.nodes[node];
            let q = self
                .bound
                .get(&listed.automaton_state)
                .copied()
                .unwrap_or_default();
            let successors = match listed.product {
                Some(mark) if self.live.is_some() => {
                    self.live_successors(node, &state, q, mark, &listed.zone)?
                }
                _ => self.successors_of(&state, q, &listed.zone)?,
            };
            for index in self.leaving[node].clone() {
                let edge = &certificate.edges[index];
                let target = &certificate.nodes[edge.to];
                for (position, successor) in successors.iter().enumerate() {
                    let same = successor.step.as_ref().map(GuardedStep::transition)
                        == edge.transition.as_ref()
                        && successor.part == edge.part
                        && successor.horizon == edge.horizon
                        && successor.digest == target.state_key
                        && successor.silent == edge.silent
                        && successor.mark == target.product
                        && successor.zone.alu_covered_by(&target.zone, &self.cover)?
                        && match &successor.cell {
                            Some(cell) => cell.includes(&target.zone)?,
                            None => true,
                        };
                    if same && self.binds(target.automaton_state, successor.q) {
                        self.covers[index] = Some(position);
                        if let Some(step) = &successor.step {
                            self.refine_edge(node, index, step)?;
                        }
                        if self.states[edge.to].is_none() {
                            let post = successor
                                .step
                                .as_ref()
                                .map_or_else(|| state.clone(), |step| step.post().clone());
                            self.states[edge.to] = Some(post);
                            queue.push(edge.to);
                        }
                        break;
                    }
                }
            }
            self.successors[node] = Some(successors);
        }
        Ok(())
    }

    /// The symbolic successors of `(state, q, zone)`, the zone first held
    /// to the state's own invariants: one per enabled step, guard part and,
    /// for a TT-2 claim, horizon piece, and the claim's idle-tail stutter
    /// positions, as FR-239's search reads them.
    fn successors_of(
        &self,
        state: &ModelSimState,
        q: usize,
        zone: &Zone,
    ) -> Step<Vec<Successor>, O> {
        let mut zone = zone.clone();
        let invariants = self.invariants(state)?;
        self.constrain(&mut zone, &invariants)?;
        let mut found = Vec::new();
        if zone.is_empty() {
            return Ok(found);
        }
        let decided = self
            .window
            .as_ref()
            .is_none_or(|window| window.claim.borrow().decided(q));
        let tail = self.window.as_ref().is_some_and(|window| {
            window
                .claim
                .borrow()
                .state(q)
                .is_some_and(|state| state.tail)
        });
        let period = self.timed.tick_period().cloned();
        let mut positions = 0_usize;
        if !tail || decided {
            for step in self.timed.guarded_steps(state).or_else(undecided)? {
                let target = self.invariants(step.post())?;
                let digest = self.digest(step.post())?;
                let tick =
                    self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero());
                for (part, constraints) in step.parts().iter().enumerate() {
                    let mut next = zone.clone();
                    self.constrain(&mut next, constraints)?;
                    if next.is_empty() {
                        continue;
                    }
                    self.apply_clock_updates(&mut next, &step)?;
                    self.constrain(&mut next, &target)?;
                    if next.is_empty() {
                        continue;
                    }
                    positions += 1;
                    if decided {
                        if self.window.is_some() {
                            next.free(self.horizon_clock())?;
                        }
                        self.settle(step.post(), &mut next)?;
                        if !next.is_empty() {
                            found.push(Successor {
                                step: Some(step.clone()),
                                part: Some(part),
                                horizon: None,
                                frees: self.window.is_some(),
                                digest,
                                q,
                                zone: next,
                                silent: None,
                                mark: None,
                                cell: None,
                                read: None,
                            });
                        }
                        continue;
                    }
                    let advance = match (&period, tick) {
                        (Some(period), true) => period.clone(),
                        _ => Rational::from_integer(Integer::zero()),
                    };
                    for (constraints, stamp) in self.pieces() {
                        let mut at = next.clone();
                        self.constrain(&mut at, &constraints)?;
                        if at.is_empty() {
                            continue;
                        }
                        let stamp_of = match period {
                            Some(_) => Stamp::After(&advance),
                            None => Stamp::At(&stamp),
                        };
                        let read = self.read(Some(q), step.post(), stamp_of, false)?;
                        self.settle(step.post(), &mut at)?;
                        if at.is_empty() {
                            continue;
                        }
                        found.push(Successor {
                            step: Some(step.clone()),
                            part: Some(part),
                            horizon: period.is_none().then_some(constraints),
                            frees: false,
                            digest,
                            q: read,
                            zone: at,
                            silent: None,
                            mark: None,
                            cell: None,
                            read: None,
                        });
                    }
                }
            }
        }
        if decided {
            return Ok(found);
        }
        // An idle-tail stutter position: under a dense source at a
        // quiescent state, under a tick source at a terminal one (FR-231).
        let idles = match &period {
            Some(_) => tail || positions == 0,
            None => self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?,
        };
        if idles {
            let digest = self.digest(state)?;
            for (constraints, stamp) in self.pieces() {
                let mut at = zone.clone();
                self.constrain(&mut at, &constraints)?;
                if at.is_empty() {
                    continue;
                }
                let stamp_of = match &period {
                    Some(period) => Stamp::After(period),
                    None => Stamp::At(&stamp),
                };
                let read = self.read(Some(q), state, stamp_of, true)?;
                self.settle(state, &mut at)?;
                if at.is_empty() {
                    continue;
                }
                found.push(Successor {
                    step: None,
                    part: None,
                    horizon: period.is_none().then_some(constraints),
                    frees: false,
                    digest,
                    q: read,
                    zone: at,
                    silent: None,
                    mark: None,
                    cell: None,
                    read: None,
                });
            }
        }
        Ok(found)
    }

    /// The pieces of a TT-2 claim's horizon axis.
    fn pieces(&self) -> Vec<(Vec<PlaceConstraint>, Rational)> {
        self.window
            .as_ref()
            .map(|window| window.pieces.clone())
            .unwrap_or_default()
    }

    /// Node `node`'s successors are each covered by an edge, and it is bad
    /// only when it is `locked` and no time diverges after it.
    fn node(&self, node: usize, divergent: &[bool]) -> Step<(), O> {
        let count = self.successors[node].as_ref().map_or(0, Vec::len);
        let mut covered = vec![false; count];
        for &index in &self.leaving[node] {
            if let Some(position) = self.covers[index] {
                covered[position] = true;
            }
        }
        let successors = self.successors[node]
            .as_ref()
            .expect("a reached node has recomputed successors");
        // A set or bag assignment can produce several semantically equal
        // guarded steps (for example, equal element matches). The certificate
        // edge schema names their transition, guard part and resulting
        // symbolic state, so one matching edge covers all duplicate
        // recomputations with that same observable successor.
        let witnesses: Vec<usize> = covered
            .iter()
            .enumerate()
            .filter_map(|(position, &covered)| covered.then_some(position))
            .collect();
        for position in 0..successors.len() {
            if covered[position] {
                continue;
            }
            let successor = &successors[position];
            covered[position] = witnesses
                .iter()
                .any(|&other| equivalent_successor(successor, &successors[other]));
        }
        if covered.iter().any(|&covered| !covered) {
            return reject(CertificateRule::SuccessorMissing, at_node(node));
        }
        let listed = &self.certificate.nodes[node];
        if listed.locked {
            if divergent[node] {
                return reject(CertificateRule::BadState, at_node(node));
            }
        } else if self.bad(node)? {
            return reject(CertificateRule::BadState, at_node(node));
        }
        Ok(())
    }

    fn state(&self, node: usize) -> Step<&ModelSimState, O> {
        self.states[node]
            .as_ref()
            .ok_or(fault(CheckerFault::UnreachedNode))
    }

    /// Whether `node` violates the item at some valuation of its zone.
    fn bad(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        if self.time_lock || self.live.is_some() || self.refined.is_some() {
            // A refinement form's badness is `check_step`'s, judged on
            // each edge in `reach`.
            // Time-lock freedom is read from the divergence part, and a
            // liveness product's bad cycles from its components.
            return Ok(false);
        }
        let Some(claim) = &self.claim else {
            return self.deadlocked(node);
        };
        if let Some(window) = &self.window {
            let listed = self.certificate.nodes[node].automaton_state;
            let q = self
                .bound
                .get(&listed)
                .copied()
                .ok_or(fault(CheckerFault::UnreachedNode))?;
            let automaton = window.claim.borrow();
            return Ok(automaton.undefined(q).is_some()
                || automaton.violated(q).is_some()
                || automaton.insufficient(q).is_some());
        }
        let atom = claim.atom.ok_or(fault(CheckerFault::Automaton))?;
        for instance in &claim.instances {
            let mut meter = Meter::new(self.timed.system().meter_limits());
            match claim
                .letters
                .atom(atom, instance, state, &mut meter)
                .or_else(undecided)?
            {
                AtomReading::True => {}
                AtomReading::False | AtomReading::Undefined(_) => return Ok(true),
            }
        }
        Ok(false)
    }

    /// FR-232 over the zone held to its state's invariants: a quiescent
    /// node some of whose valuations enable no step after any delay. Read
    /// under the aM coverage, a deadlocked real valuation has a bisimilar
    /// deadlocked one in some node.
    fn deadlocked(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        if !self.timed.is_quiescent(state).or_else(undecided)? {
            return Ok(false);
        }
        let mut enabling = Vec::new();
        for step in self.timed.guarded_steps(state).or_else(undecided)? {
            let target = self.invariants(step.post())?;
            // Pull post-state invariants through the simultaneous carries
            // and resets to the pre-state valuations that enable this step.
            let mut pulled_back = Vec::with_capacity(target.len());
            let mut impossible = false;
            for bound in target {
                if let Some((_, value)) = step
                    .resets()
                    .iter()
                    .find(|(place, _)| *place == bound.place())
                {
                    if !bound.op().holds(value, bound.bound()) {
                        impossible = true;
                        break;
                    }
                    continue;
                }
                let place = step
                    .carries()
                    .iter()
                    .find(|(target, _)| *target == bound.place())
                    .map_or(bound.place(), |(_, source)| *source);
                pulled_back.push(PlaceConstraint::new(
                    place,
                    bound.op(),
                    bound.bound().clone(),
                ));
            }
            if impossible {
                continue;
            }
            for part in step.parts() {
                let mut zone = Zone::universe(self.dim());
                self.constrain(&mut zone, part)?;
                self.constrain(&mut zone, &pulled_back)?;
                if !zone.is_empty() {
                    zone.down();
                    enabling.push(zone);
                }
            }
        }
        if enabling.is_empty() {
            return Ok(matches!(
                self.timed.system().terminal_class(state, false),
                Ok(TerminalClass::Deadlocked)
            ));
        }
        let mut zone = self.certificate.nodes[node].zone.clone();
        self.constrain(&mut zone, &self.invariants(state)?)?;
        let mut uncovered = vec![zone];
        for cover in &enabling {
            let mut rest = Vec::new();
            for piece in &uncovered {
                rest.extend(piece.minus(cover)?);
            }
            uncovered = rest;
            if uncovered.is_empty() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The step a valid edge takes, `None` for a stutter, and the
    /// successor it covers.
    fn covered(&self, index: usize) -> Step<&Successor, O> {
        let edge = &self.certificate.edges[index];
        self.covers[index]
            .and_then(|position| {
                self.successors[edge.from]
                    .as_ref()
                    .and_then(|successors| successors.get(position))
            })
            .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))
    }

    /// For each node, whether over valid edges it reaches a node where a
    /// behaviour may idle forever, or a strongly connected set whose
    /// divergence the checker cannot rule out. The components are computed
    /// once, under the caller's `cancel`, and divergence is propagated back
    /// along the valid edges.
    fn divergent(&self) -> Step<Vec<bool>, O> {
        let count = self.certificate.nodes.len();
        let mut graph = Graph::new();
        let vertices: Vec<_> = (0..count)
            .map(|_| graph.add_vertex(std::iter::empty::<Identity>()))
            .collect();
        let mut into: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            if self.covers[index].is_some() {
                graph
                    .add_edge(vertices[edge.from], vertices[edge.to], EdgeLabel::Stutter)
                    .map_err(|_| fault(CheckerFault::Graph))?;
                into[edge.to].push(edge.from);
            }
        }
        let within: BTreeSet<_> = (0..count)
            .filter(|&node| self.states[node].is_some())
            .map(|node| vertices[node])
            .collect();
        let found = components(&graph, &within, self.cancel).map_err(|error| match error {
            FairnessError::Cancelled => match self.cancel.cause() {
                Some(source) => Halt::Verdict(ZoneCertificateVerdict::Stopped(
                    IncompleteCause::Cancelled { source },
                )),
                None => fault(CheckerFault::CancellationWithoutCause),
            },
            _ => fault(CheckerFault::Graph),
        })?;
        let mut divergent = vec![false; count];
        for (node, divergent) in divergent.iter_mut().enumerate() {
            *divergent = self.states[node].is_some() && self.idles(node)?;
        }
        for component in found.iter().filter(|component| component.has_edge()) {
            self.poll()?;
            let members: BTreeSet<usize> = component
                .vertices()
                .iter()
                .map(|vertex| vertex.index())
                .collect();
            let inside: Vec<usize> = members
                .iter()
                .flat_map(|&node| self.leaving[node].iter().copied())
                .filter(|&index| {
                    self.covers[index].is_some()
                        && members.contains(&self.certificate.edges[index].to)
                })
                .collect();
            if !self.zeno(&members, &inside)? {
                for &member in &members {
                    divergent[member] = true;
                }
            }
        }
        let mut queue: Vec<usize> = (0..count).filter(|&node| divergent[node]).collect();
        while let Some(node) = queue.pop() {
            for &from in &into[node] {
                if !divergent[from] {
                    divergent[from] = true;
                    queue.push(from);
                }
            }
        }
        Ok(divergent)
    }

    /// Whether a behaviour may idle at `node` forever: under a dense source
    /// a quiescent state where time elapses, under a tick source one that
    /// enables no step, whose stutter ticks (FR-231).
    fn idles(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        Ok(match self.timed.tick_period() {
            Some(_) => self.successors[node].as_ref().is_none_or(|successors| {
                successors.iter().all(|successor| successor.step.is_none())
            }),
            None => self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?,
        })
    }

    /// Whether an edge lets time advance under a tick source: a tick step,
    /// or a terminal state's stutter, which counts as a tick (FR-231).
    fn ticks(&self, index: usize) -> Step<bool, O> {
        Ok(match &self.covered(index)?.step {
            Some(step) => {
                self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero())
            }
            None => true,
        })
    }

    /// Whether no time diverges inside the strongly connected `members`
    /// with the `edges` among them. Under a dense source: every member is
    /// urgent, or some clock that none of `edges` assigns is bounded above
    /// by the time invariants of every member's own state, which every
    /// real valuation there satisfies. Under a tick source: none of
    /// `edges` ticks.
    fn zeno(&self, members: &BTreeSet<usize>, edges: &[usize]) -> Step<bool, O> {
        if self.timed.tick_period().is_some() {
            for &index in edges {
                if self.ticks(index)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let mut urgent = true;
        let mut bounded: Option<BTreeSet<usize>> = None;
        for &member in members {
            let state = self.state(member)?;
            urgent &= !self.elapses(state)?;
            let here: BTreeSet<usize> = self
                .invariants(state)?
                .iter()
                .filter(|bound| {
                    matches!(
                        bound.op(),
                        ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal
                    )
                })
                .map(PlaceConstraint::place)
                .collect();
            bounded = Some(match bounded {
                None => here,
                Some(earlier) => earlier.intersection(&here).copied().collect(),
            });
        }
        if urgent {
            return Ok(true);
        }
        let mut reset = BTreeSet::new();
        for &index in edges {
            if let Some(step) = &self.covered(index)?.step {
                reset.extend(step.resets().iter().map(|(place, _)| *place));
                reset.extend(
                    step.carries()
                        .iter()
                        .filter_map(|(target, source)| (target != source).then_some(*target)),
                );
            }
        }
        Ok(bounded.is_some_and(|bounded| bounded.iter().any(|place| !reset.contains(place))))
    }

    /// `admitted`: a path of valid edges from an initial node to a
    /// divergent target, recomputed in exact zones from the initial state,
    /// each held to its state's own invariants, so the path is one some
    /// behaviour takes. It proves that a time-divergent behaviour exists,
    /// as FR-244 TD-6 says, not that a fair one does: FR-240's vacuity
    /// under fairness has no certificate part (spec gap STD-493, comment
    /// 8eafddf9, item 3; SR-1647002 FND-002).
    fn admitted(&self) -> Step<(), O> {
        let witness = &self.certificate.admitted;
        let mut at = witness.start;
        let fails = |at: usize| reject(CertificateRule::WitnessFails, at_node(at));
        if !self.certificate.initials.contains(&at) || self.states[at].is_none() {
            return fails(at);
        }
        // A refinement form's witness starts at a positioned initial node: a
        // locked one has no admitted behaviour (FR-245).
        if let Some(refined) = &self.refined {
            let listed = &self.certificate.nodes[at];
            if listed.locked || listed.automaton_state >= refined.part.positions.len() {
                return fails(at);
            }
        }
        let Some(mut zone) = self.initial_zone(at)? else {
            return fails(at);
        };
        for &index in &witness.path {
            let edge = &self.certificate.edges[index];
            if edge.from != at || self.covers[index].is_none() {
                return fails(at);
            }
            zone = self.post(index, &zone)?;
            at = edge.to;
            if zone.is_empty() {
                return fails(at);
            }
        }
        let holds = match &witness.target {
            DivergentTarget::Quiescent(node) => {
                *node == at && self.timed.tick_period().is_none() && self.idles(at)?
            }
            DivergentTarget::Terminal(node) => {
                *node == at && self.timed.tick_period().is_some() && self.idles(at)?
            }
            DivergentTarget::Cycle(cycle) => {
                self.advancing_cycle(at, cycle)? && self.repeats(cycle, &zone)?
            }
        };
        if holds {
            Ok(())
        } else {
            fails(at)
        }
    }

    /// The exact zone of an initial state the initial node `node` holds.
    fn initial_zone(&self, node: usize) -> Step<Option<Zone>, O> {
        let listed = &self.certificate.nodes[node];
        for start in self.timed.initial() {
            let state = start.discrete();
            if self.digest(state)? != listed.state_key {
                continue;
            }
            let mut zone = Zone::zero(self.dim());
            if self.live.is_some() {
                let cell = self.cell_zone_of(state, &zone)?;
                self.settle_within(state, &mut zone, Some(&cell))?;
                if !zone.is_empty() {
                    return Ok(Some(zone));
                }
                continue;
            }
            let zero = Rational::from_integer(Integer::zero());
            let q = self.read(None, state, Stamp::At(&zero), false)?;
            if self.bound.get(&listed.automaton_state) != Some(&q) && self.window.is_some() {
                continue;
            }
            self.settle(state, &mut zone)?;
            if !zone.is_empty() {
                return Ok(Some(zone));
            }
        }
        Ok(None)
    }

    /// The exact successor of `zone`, at a valid edge's source, along the
    /// edge.
    fn post(&self, index: usize, zone: &Zone) -> Step<Zone, O> {
        let edge = &self.certificate.edges[index];
        let source = self.state(edge.from)?;
        let successor = self.covered(index)?;
        let mut next = zone.clone();
        if let Some(cell) = &successor.cell {
            // A liveness product's move, within the target's cell.
            let post = successor.step.as_ref().map_or(source, GuardedStep::post);
            match (&successor.step, successor.silent) {
                (Some(step), _) => {
                    let part = successor
                        .part
                        .and_then(|part| step.parts().get(part))
                        .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))?;
                    self.constrain(&mut next, part)?;
                    self.apply_clock_updates(&mut next, step)?;
                    self.constrain(&mut next, &self.invariants(post)?)?;
                }
                (None, Some(SilentMove::Wait)) => {
                    next.up();
                    self.constrain(&mut next, &self.invariants(source)?)?;
                }
                (None, Some(SilentMove::Diverge)) => {
                    let z = self.z_clock();
                    next.constrain(0, z, &self.certificate.scale.neg(), false)?;
                    if !next.is_empty() {
                        next.reset(z, &Integer::zero())?;
                    }
                }
                (None, None) => {}
            }
            if let Some(read) = &successor.read {
                self.read_at(&mut next, read)?;
            }
            self.settle_within(post, &mut next, Some(cell))?;
            return Ok(next);
        }
        let post = match &successor.step {
            Some(step) => {
                let part = successor
                    .part
                    .and_then(|part| step.parts().get(part))
                    .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))?;
                self.constrain(&mut next, part)?;
                self.apply_clock_updates(&mut next, step)?;
                self.constrain(&mut next, &self.invariants(step.post())?)?;
                step.post()
            }
            None => source,
        };
        if let Some(horizon) = &successor.horizon {
            self.constrain(&mut next, horizon)?;
        }
        if successor.frees {
            next.free(self.horizon_clock())?;
        }
        self.settle(post, &mut next)?;
        Ok(next)
    }

    /// The valuations of `target`, at a valid edge's target, whose
    /// predecessors along the edge lie at its source: the inverse of
    /// [`Self::post`].
    fn pre(&self, index: usize, target: &Zone) -> Step<Zone, O> {
        let edge = &self.certificate.edges[index];
        let source = self.state(edge.from)?;
        let successor = self.covered(index)?;
        let post = successor.step.as_ref().map_or(source, GuardedStep::post);
        let invariants = self.invariants(post)?;
        let mut zone = target.clone();
        let hold = |zone: &mut Zone| -> Step<(), O> {
            self.constrain(zone, &invariants)?;
            if let Some(cell) = &successor.cell {
                intersect(zone, cell)?;
            }
            Ok(())
        };
        hold(&mut zone)?;
        if self.elapses(post)? && !zone.is_empty() {
            zone.down();
            hold(&mut zone)?;
        }
        match successor.silent {
            Some(SilentMove::Wait) => {
                if !zone.is_empty() {
                    zone.down();
                }
            }
            Some(SilentMove::Diverge) => {
                let z = self.z_clock();
                zone.constrain(z, 0, &Integer::zero(), false)?;
                zone.free(z)?;
                zone.constrain(0, z, &self.certificate.scale.neg(), false)?;
            }
            None => {}
        }
        if let Some(read) = &successor.read {
            self.read_before(&mut zone, read)?;
        }
        if let Some(horizon) = &successor.horizon {
            self.constrain(&mut zone, horizon)?;
        }
        if successor.frees {
            zone.free(self.horizon_clock())?;
        }
        if let Some(step) = &successor.step {
            zone = self.preimage_clock_updates(&zone, step)?;
            let part = successor
                .part
                .and_then(|part| step.parts().get(part))
                .ok_or(fault(CheckerFault::EdgeWithoutSuccessor))?;
            self.constrain(&mut zone, part)?;
        }
        self.constrain(&mut zone, &self.invariants(source)?)?;
        Ok(zone)
    }

    /// Whether `cycle` can be repeated forever from some valuation of
    /// `zone`, the exact zone the path reaches at its start. The greatest
    /// set of valuations at the start, within its invariants, each of which
    /// takes the cycle back into the set, is computed from the whole
    /// invariant zone down: each round keeps the valuations whose cycle
    /// lands in the last round's set. Each round is one zone, and each of
    /// its bounds only falls, so the rounds end, at a fixed point or the
    /// empty set. A valuation of `zone` in the fixed point is reached and
    /// repeats the cycle forever: a clock the cycle never resets may drift
    /// at each repetition. Each round reads the cycle's edges again, charged
    /// to `max_certificate_edges`.
    fn repeats(&self, cycle: &[usize], zone: &Zone) -> Step<bool, O> {
        if cycle.is_empty() {
            return Ok(false);
        }
        let start = self.state(self.certificate.edges[cycle[0]].from)?;
        let invariants = self.invariants(start)?;
        let mut held = Zone::universe(self.dim());
        self.constrain(&mut held, &invariants)?;
        let length = u64::try_from(cycle.len()).unwrap_or(u64::MAX);
        loop {
            self.charge(length)?;
            let mut back = held.clone();
            for &index in cycle.iter().rev() {
                back = self.pre(index, &back)?;
                if back.is_empty() {
                    return Ok(false);
                }
            }
            // The valuations at the start from which a delay reaches the
            // first step.
            self.constrain(&mut back, &invariants)?;
            if self.elapses(start)? && !back.is_empty() {
                back.down();
                self.constrain(&mut back, &invariants)?;
            }
            let mut next = held.clone();
            intersect(&mut next, &back)?;
            if next.is_empty() {
                return Ok(false);
            }
            if next.includes(&held)? {
                let mut met = next;
                intersect(&mut met, zone)?;
                return Ok(!met.is_empty());
            }
            held = next;
        }
    }

    /// Whether `cycle` is a cycle of valid edges from and back to `at`
    /// each repetition of which lets time advance: it ticks, under a tick
    /// source; under a dense source, some clock guarded on it by a lower
    /// bound above the value its last reset before the guard set it.
    fn advancing_cycle(&self, at: usize, cycle: &[usize]) -> Step<bool, O> {
        let mut node = at;
        for &index in cycle {
            let edge = &self.certificate.edges[index];
            if edge.from != node || self.covers[index].is_none() {
                return Ok(false);
            }
            node = edge.to;
        }
        if cycle.is_empty() || node != at {
            return Ok(false);
        }
        // The divergence clock resets only once a unit has passed.
        if cycle
            .iter()
            .any(|&index| self.certificate.edges[index].silent == Some(SilentMove::Diverge))
        {
            return Ok(true);
        }
        if self.timed.tick_period().is_some() {
            for &index in cycle {
                if self.ticks(index)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let mut steps = Vec::with_capacity(cycle.len());
        for &index in cycle {
            let successor = self.covered(index)?;
            steps.push((successor.step.as_ref(), successor.part));
        }
        let length = steps.len();
        for (position, (step, part)) in steps.iter().enumerate() {
            let Some(constraints) = step
                .zip(*part)
                .and_then(|(step, part)| step.parts().get(part))
            else {
                continue;
            };
            for constraint in constraints {
                let strict_lower = match constraint.op() {
                    ClockOp::Greater => true,
                    ClockOp::GreaterOrEqual | ClockOp::Equal => false,
                    ClockOp::Less | ClockOp::LessOrEqual => continue,
                };
                // The last reset of the clock before this guard, going back
                // around the cycle; this edge's own resets come after it.
                let reset = (1..=length).find_map(|back| {
                    let (earlier, _) = steps[(position + length - back) % length];
                    earlier?
                        .resets()
                        .iter()
                        .rev()
                        .find(|(place, _)| *place == constraint.place())
                        .map(|(_, value)| value.clone())
                });
                let Some(reset) = reset else {
                    continue;
                };
                let bound = constraint.bound();
                if bound > &reset || (strict_lower && bound == &reset) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// `zone` intersected with `other`, cell by cell.
fn intersect(zone: &mut Zone, other: &Zone) -> Result<(), ZoneError> {
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            if let Some((value, strict)) = other.bound(i, j)? {
                zone.constrain(i, j, &value, strict)?;
            }
        }
    }
    Ok(())
}

/// `value` in units of `1 / scale`; `None` when the scale does not make it
/// an integer.
fn scaled(value: &Rational, scale: &Integer) -> Option<Integer> {
    let product = value.mul(&Rational::from_integer(scale.clone()));
    (product.denominator().as_big() == &num_bigint::BigInt::from(1))
        .then(|| Integer::from_big(product.numerator().as_big().clone()))
}

/// The LU bounds the checker computes from the model and, for a TT-2
/// claim, its automaton's `constants` for the horizon clock: per clock,
/// the largest constant a guard, invariant or reset compares it with from
/// below and from above, scaled; `None` when the scale makes any constant
/// not an integer.
#[allow(
    clippy::type_complexity,
    reason = "one list of lower and one of upper bounds"
)]
fn computed_bounds(
    timed: &TimedSystem<'_, '_>,
    constants: Option<&[Rational]>,
    scale: &Integer,
) -> Option<(Vec<Option<Integer>>, Vec<Option<Integer>>)> {
    let model = timed.model();
    let fields = model.clocks().len();
    let mut lower: Vec<Vec<Rational>> = vec![Vec::new(); fields];
    let mut upper: Vec<Vec<Rational>> = vec![Vec::new(); fields];
    let mut record = |clock: usize, op: ClockOp, bound: &Rational| {
        if clock >= fields {
            return;
        }
        if matches!(
            op,
            ClockOp::Greater | ClockOp::GreaterOrEqual | ClockOp::Equal
        ) {
            lower[clock].push(bound.clone());
        }
        if matches!(op, ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal) {
            upper[clock].push(bound.clone());
        }
    };
    for guard in model.guards() {
        for part in guard.parts() {
            for constraint in part.clocks() {
                record(
                    constraint.clock().index(),
                    constraint.op(),
                    constraint.bound(),
                );
            }
        }
    }
    for invariant in model.invariants() {
        for constraint in invariant.bounds() {
            record(
                constraint.clock().index(),
                constraint.op(),
                constraint.bound(),
            );
        }
    }
    for resets in model.resets() {
        for reset in resets.resets() {
            record(reset.clock().index(), ClockOp::Equal, reset.value());
        }
    }
    // Every constant, not only the largest, must be exact at the scale: a
    // checker zone reads each one.
    let all = lower
        .iter()
        .chain(&upper)
        .flatten()
        .chain(constants.into_iter().flatten());
    for value in all {
        scaled(value, scale)?;
    }
    let max = |values: &[Rational]| -> Option<Option<Integer>> {
        match values.iter().max() {
            Some(value) => scaled(value, scale).map(Some),
            None => Some(None),
        }
    };
    let mut lows = Vec::new();
    let mut highs = Vec::new();
    for place in timed.places() {
        let clock = place.clock().index();
        lows.push(max(lower.get(clock)?)?);
        highs.push(max(upper.get(clock)?)?);
    }
    if let Some(constants) = constants {
        // The horizon clock is compared with every interval end, from both
        // sides.
        lows.push(max(constants)?);
        highs.push(max(constants)?);
    }
    Some((lows, highs))
}

/// The size a check charges against `max_certificate_bytes`: each node's
/// key and zone cells, each edge, the LU bounds, the witnesses and the
/// component numbering, at their integers' magnitudes. It bounds the certificate's in-memory reading,
/// not an encoding: FR-418's byte encoding of the certificate does not
/// exist yet (spec gap STD-493).
fn certificate_bytes<O>(certificate: &ZoneCertificate<O>) -> u64 {
    let integer = |value: &Integer| value.magnitude_bits() / 8 + 1;
    let mut bytes = integer(&certificate.scale);
    for bound in certificate
        .lu_bounds
        .lower
        .iter()
        .chain(&certificate.lu_bounds.upper)
        .flatten()
    {
        bytes = bytes.saturating_add(integer(bound));
    }
    for node in &certificate.nodes {
        bytes = bytes.saturating_add(40);
        let dim = node.zone.dim();
        for i in 0..dim {
            for j in 0..dim {
                let cell = match node.zone.bound(i, j) {
                    Ok(Some((value, _))) => integer(&value) + 1,
                    _ => 1,
                };
                bytes = bytes.saturating_add(cell);
            }
        }
    }
    let edges = u64::try_from(certificate.edges.len()).unwrap_or(u64::MAX);
    let mut parts = witness_bytes(&certificate.admitted);
    if let Some(components) = &certificate.components {
        parts = parts.saturating_add(components_bytes(components));
    }
    for witness in certificate.divergence.iter().flatten() {
        parts = parts.saturating_add(witness_bytes(witness));
    }
    if let Some(part) = &certificate.refinement {
        let mut refined = 32_u64;
        for position in &part.positions {
            let history = u64::try_from(position.history.len()).unwrap_or(u64::MAX);
            let states = u64::try_from(position.abstract_states.len()).unwrap_or(u64::MAX);
            refined = refined
                .saturating_add(40)
                .saturating_add(history)
                .saturating_add(states.saturating_mul(40));
        }
        let taken = u64::try_from(part.taken.len()).unwrap_or(u64::MAX);
        parts = parts
            .saturating_add(refined)
            .saturating_add(taken.saturating_mul(8));
    }
    bytes
        .saturating_add(edges.saturating_mul(48))
        .saturating_add(parts)
}

/// [`certificate_bytes`] of a divergence witness: its edges, each 8.
fn witness_bytes(witness: &DivergenceWitness) -> u64 {
    let cycle = match &witness.target {
        DivergentTarget::Cycle(cycle) => cycle.len(),
        DivergentTarget::Quiescent(_) | DivergentTarget::Terminal(_) => 1,
    };
    u64::try_from(witness.path.len().saturating_add(cycle).saturating_add(1))
        .unwrap_or(u64::MAX)
        .saturating_mul(8)
}

/// [`certificate_bytes`] of a component numbering: each node's index and
/// each reason, 8 per entry, nested numberings included. Read without
/// recursion, however deep the nesting.
fn components_bytes(components: &Components) -> u64 {
    let mut bytes = 0_u64;
    let mut pending = vec![components];
    while let Some(components) = pending.pop() {
        let entries = components
            .numbering
            .len()
            .saturating_add(components.reasons.len());
        bytes = bytes.saturating_add(u64::try_from(entries).unwrap_or(u64::MAX).saturating_mul(8));
        for reason in &components.reasons {
            if let ComponentReason::Strong {
                constraints,
                nested,
            } = reason
            {
                let listed = u64::try_from(constraints.len()).unwrap_or(u64::MAX);
                bytes = bytes.saturating_add(listed.saturating_mul(8));
                pending.push(nested);
            }
        }
    }
    bytes
}

/// Whether `components` numbers `nodes` nodes into its reasons, and each
/// strong reason's nested numbering does too, numbering a strict subset of
/// that reason's component: the engine nests a strong reason only below a
/// node that enables one of its constraints, which the nested numbering
/// leaves out. Each level then numbers fewer nodes, so the nesting is no
/// deeper than the node count (SR-1647001 FND-001). Read without
/// recursion.
fn numbering_ok(components: &Components, nodes: usize) -> bool {
    let mut pending = vec![components];
    while let Some(components) = pending.pop() {
        if components.numbering.len() != nodes {
            return false;
        }
        let mut sizes = vec![0_usize; components.reasons.len()];
        for &index in components.numbering.iter().flatten() {
            match sizes.get_mut(index) {
                Some(size) => *size += 1,
                None => return false,
            }
        }
        for (index, reason) in components.reasons.iter().enumerate() {
            let ComponentReason::Strong { nested, .. } = reason else {
                continue;
            };
            if nested.numbering.len() != nodes {
                return false;
            }
            let mut numbered = 0_usize;
            for (inner, outer) in nested.numbering.iter().zip(&components.numbering) {
                if inner.is_some() {
                    if *outer != Some(index) {
                        return false;
                    }
                    numbered += 1;
                }
            }
            if numbered >= sizes[index] {
                return false;
            }
            pending.push(nested);
        }
    }
    true
}
