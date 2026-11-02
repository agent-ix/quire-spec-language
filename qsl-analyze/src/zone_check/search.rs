// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-239: EN-6's passed/waiting search of symbolic states `(s, Z)` over a
//! timed subject (ADR-026 EZ-1, EZ-3 to EZ-5), deciding a TT-1 timed
//! invariant, a TT-2 bounded window, the deadlock-freedom item or, by
//! FR-240's time-lock search on the completed graph
//! ([`super::time_lock`]), the time-lock-freedom item. Any other clause
//! with no timed interval is decided after the search, once the subject
//! admits a behaviour, on FR-240's liveness product
//! ([`super::liveness`]).
//!
//! A symbolic state pairs a model state with a zone over the subject's clock
//! places, scaled once to integers ([`scale_constants`]). Successors come
//! from FR-120's relation in canonical transition order, one per convex part
//! of a step's guard. A new symbolic state is covered when a stored one with
//! the same model state has a zone whose aLU abstraction includes it; the
//! covering edge is retained, and stored zones are never extrapolated.
//!
//! A TT-1 claim's `holds` atom reads only the model state, so the claim
//! automaton has one carried state and needs no product: a symbolic state
//! violates the claim when its atom is false, or undefined, for some
//! instance. A violation ends the search only when it is admitted: some
//! valuation of it has a time-divergent continuation, by FR-240's test
//! ([`super::divergence`]). Its counterexample ends at such a valuation. A
//! violation with none holds vacuously, is kept as a `locked` node, and the
//! search goes on; nodes are judged in breadth-first order.
//!
//! Under a symmetry the request selects (QSpec FR-430), each symbolic state
//! is stored canonicalised: its model state by FR-152's permutation, and its
//! zone with each object's clocks moved to those of the object the
//! permutation maps it to. A counterexample is lifted back to concrete
//! states through the permutations, as FR-161 lifts EN-1's.

use std::collections::{BTreeMap, VecDeque};

use num_bigint::BigInt;
use num_traits::Zero;
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{
    GuardedStep, ModelSimState, PlaceConstraint, TerminalClass, TimedStop, TimedSystem,
};
use qsl_eval::simulation::{NotSimulated, Permutation, RequiresBound, SymmetryGroup};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::ReductionKind;
use qsl_replay::{
    AppliedReduction, CounterexampleKind, Lasso, ModelIdentity, ObligationIdentity, ReductionCause,
    SymmetryBrokenCause, SymmetryRefusal, TaskAction, TaskProductStateKey, TaskSearchEdge,
    TaskSearchEvidence, TaskSearchFrontier, TaskSearchLimit, TaskSearchLimitKind, TaskSearchNode,
    TaskSearchOpen, TaskSearchStopReason, TaskSearchTransition, TaskSubtraction,
    TaskTransitionIdentity, TaskTransitionTemplate, TemporalCounterexample, TimedPoint,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use qsl_semantics::check::{
    CheckedScheduleClaim, CheckedTemporalClause, ClockOp, FormulaNodeId, TemporalProfile,
    TimedPropertyForm,
};
use qsl_semantics::family::ClassifyFailure;
use quire_checked::key::DeclarationKey;
use quire_contract_model::CheckedSchedulabilityClaimRef;
use quire_exact::{Cancel, Integer, Meter, Rational};

use super::certificate::Proof;
use super::concretise::{concretise, Concrete, Target};
use super::divergence::Escape;
use super::liveness::Liveness;
use super::task_product::{
    edf_tie_order, task_transition_template, TaskClockLayout, TaskCompletionError, TaskJobPhase,
    TaskProductRoute, TaskProductSubject, TaskProductSubjectError, TaskSchedulerError,
    TaskSchedulerState,
};
use super::{scale_constants, Bound, Dbm, DbmError};
use crate::model_check::{admit, closure, instance_groups, ClosureRefusal};
use qsl_eval::automaton::{
    position_automaton, AutomatonError, PositionAutomaton, Read, Stamp, WindowClaim, WindowFault,
    WindowRefusal,
};

type TaskClockUpdates = (Vec<usize>, Vec<(usize, Rational)>);
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use qsl_route::reductions::Reductions;

/// FR-239's budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZoneCheckLimits {
    /// Stored symbolic states.
    pub max_symbolic_states: u64,
    /// Retained symbolic edges, real and covering.
    pub max_symbolic_transitions: u64,
    /// Claim-automaton states.
    pub max_automaton_states: u64,
    /// Bytes of stored zones.
    pub max_zone_bytes: u64,
    /// Task-product nodes whose successor expansion may be examined by a
    /// bounded FR-331 search.
    pub max_task_product_work: u64,
}

impl Default for ZoneCheckLimits {
    fn default() -> Self {
        Self {
            max_symbolic_states: 1 << 20,
            max_symbolic_transitions: 1 << 24,
            max_automaton_states: 1 << 20,
            max_zone_bytes: 1 << 30,
            max_task_product_work: 1 << 20,
        }
    }
}

/// The item EN-6 decides.
#[derive(Clone, Copy)]
pub enum ZoneCheckItem<'a> {
    /// A TT-1 clause `always holds(A)` whose atom reads the model state, a
    /// TT-2 `BoundedWindow` clause read on origin (FR-242), or any other
    /// infinite-trace or timed-profile clause, read over positions (FR-240),
    /// through FR-242's timed claim automaton when it has a timed
    /// interval.
    Clause {
        /// The checked clause.
        clause: &'a CheckedTemporalClause,
        /// The seam to the one clause evaluator for its atoms.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's deadlock-freedom item, read over time (FR-232).
    DeadlockFreedom,
    /// FR-232's time-lock-freedom item, decided by FR-240's time-lock
    /// search.
    TimeLockFreedom,
    /// A checked schedulability claim over an admitted task automaton
    /// (FR-249), lowered into this timed subject by EN-6.
    TaskSchedulability {
        /// The checked claim binds this task product's task set and policy.
        claim: &'a CheckedScheduleClaim,
        /// The exact checked-package occurrence of this authored claim.
        occurrence: &'a OccurrenceKey,
        /// The authenticated FR-322/FR-482 claim view from Contract IR.
        admitted: CheckedSchedulabilityClaimRef<'a>,
    },
}

/// A request to EN-6.
pub struct ZoneCheckRequest<'r, 's, 'a, S> {
    /// The timed subject (FR-231).
    pub subject: &'r TimedSystem<'s, 'a>,
    /// The item.
    pub item: ZoneCheckItem<'r>,
    /// The item's obligation identity, which its certificate names
    /// (FR-244).
    pub obligation: ItemObligation<S>,
    /// The budgets.
    pub limits: ZoneCheckLimits,
    /// The reductions the request selects (FR-159); EN-6 applies symmetry
    /// only (QSpec FR-430).
    pub reductions: &'r Reductions,
}

/// A reduction cause EN-6's pre-check settles, with no state explored
/// (FR-159, QSpec FR-430).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneReduction {
    /// ADR-018 V-8: a selected reduction EN-6 does not apply. Over any
    /// item, partial-order reduction or a state constraint (QSpec FR-430),
    /// named first in the order partial-order, constraint; failing that,
    /// symmetry over an item decided on the liveness product or by the
    /// time-lock search, which do not read a symmetry-reduced graph.
    Unsupported {
        /// The reduction.
        reduction: ReductionKind,
    },
    /// FR-151: the initial states are not closed under a declaration's
    /// group.
    SymmetryBroken {
        /// The declaration's population.
        population: DeclarationKey,
        /// Why.
        cause: SymmetryBrokenCause,
    },
}

/// The budget a run reached, with its setting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZoneCheckLimit {
    /// `max_symbolic_states`.
    MaxSymbolicStates(u64),
    /// `max_symbolic_transitions`.
    MaxSymbolicTransitions(u64),
    /// `max_automaton_states`.
    MaxAutomatonStates(u64),
    /// `max_zone_bytes`.
    MaxZoneBytes(u64),
    /// The timed subject's `max_clocks` (QSpec FR-415 "Cardinality bound"):
    /// a state's live clocks exceed it.
    MaxClocks(u64),
    /// The task subject's `max_task_product_work` (QSpec FR-331): bounded
    /// search has started this many task-product node expansions.
    TaskProductWork(u64),
}

/// Why a run stopped with no verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZoneStop {
    /// A budget was reached.
    LimitReached(ZoneCheckLimit),
    /// A bounded task product reached a concrete pending release whose
    /// continuous task choice cannot be finitely enumerated.
    TaskProductUnresolved,
    /// The caller's `Cancel` handle was cancelled.
    Cancelled,
}

/// Why a completed run decided nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneUndecided {
    /// The subject has no initial state.
    NoInitialState,
    /// An expansion or a timed member's reading had no value.
    UndecidedSuccessor(TimedStop),
    /// No initial symbolic state has a time-divergent behaviour (ADR-026
    /// TD-6): the claim would hold vacuously.
    NoAdmittedBehaviour,
    /// FR-241: a violation has a time-divergent continuation, but no
    /// concretisation of its path ends at a valuation that has one. FR-241
    /// defines it for lassos; a finite path reuses it (spec gap STD-493).
    LassoNotConcretized,
    /// A quiescent symbolic state has deadlocked valuations only after a
    /// delay, which no position of a counterexample names: FR-236 and QSpec
    /// FR-417 give `final_delay` to time-locks only (spec gap STD-493).
    DeadlockAfterDelay,
    /// FR-242 over ADR-024 WA-4: no violation was found, but an admitted
    /// behaviour read a window aggregate short of data for this instance,
    /// which the evaluator reports `InsufficientData`, so the claim is not
    /// proved.
    InsufficientData(ModelInstance),
}

/// The run's counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ZoneStats {
    /// Stored symbolic states.
    pub symbolic_states: u64,
    /// Retained edges, real and covering.
    pub retained_edges: u64,
    /// Claim-automaton states.
    pub automaton_states: u64,
    /// Bytes of stored zones.
    pub zone_bytes: u64,
}

impl ZoneStats {
    /// The counts of `self` and `other` together.
    fn plus(self, other: Self) -> Self {
        Self {
            symbolic_states: self.symbolic_states.saturating_add(other.symbolic_states),
            retained_edges: self.retained_edges.saturating_add(other.retained_edges),
            automaton_states: self.automaton_states.saturating_add(other.automaton_states),
            zone_bytes: self.zone_bytes.saturating_add(other.zone_bytes),
        }
    }
}

/// Where [`Run::insert_at`] put a symbolic state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Inserted {
    /// A new node.
    New(usize),
    /// The stored node that covers it.
    Covered(usize),
}

/// A stored symbolic state.
#[derive(Clone, Debug)]
pub(super) struct SymbolicNode {
    /// The model state.
    pub state: ModelSimState,
    /// FR-249's separate scheduler control, present only on task products.
    pub task_state: Option<TaskSchedulerState>,
    /// The zone over clock places, in scaled units.
    pub zone: Dbm,
    /// The index of its initial state, for an initial node.
    pub initial: Option<usize>,
    /// Whether it violates the item only on time-locked valuations.
    pub locked: bool,
    /// The claim-automaton state; 0 with no automaton.
    pub q: usize,
    /// The permutation that canonicalised it (QSpec FR-430); the identity
    /// under the trivial group.
    pub permutation: Permutation,
}

/// A retained edge: a successor by a step's convex guard part, in
/// canonical order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SymbolicEdge {
    /// The source node.
    pub from: usize,
    /// The target node: the successor itself, or the node whose aLU
    /// abstraction covers it.
    pub to: usize,
    /// Whether `to` covers the successor rather than holding it.
    pub covering: bool,
    /// The step's index among the source's guarded steps and the convex
    /// part's index among its parts; `None` for an idle-tail stutter
    /// position (FR-242).
    pub step: Option<(usize, usize)>,
    /// The piece of the horizon clock's axis a TT-2 position lies in, by
    /// index into the run's pieces; `None` with no automaton, under a tick
    /// source, and past the formula's horizon.
    pub piece: Option<usize>,
    /// The FR-419 derived scheduler action, when this is a task edge.
    pub task_transition: Option<TaskTransitionIdentity>,
    /// The finite FR-418 task-edge template, instantiated from the source or
    /// target live-job rank before canonical closure merges its identity.
    pub task_template: Option<TaskTransitionTemplate>,
    /// Bounded-subtraction rows performed by this task transition.
    pub task_subtractions: Vec<TaskSubtraction>,
}

/// How a new node is reached: an edge from a stored node, with its step
/// and piece.
#[derive(Clone, Debug)]
pub(super) struct Arrival {
    pub(super) from: usize,
    pub(super) step: Option<(usize, usize)>,
    pub(super) piece: Option<usize>,
    pub(super) task_transition: Option<TaskTransitionIdentity>,
    pub(super) task_subtractions: Vec<TaskSubtraction>,
}

impl SymbolicEdge {
    pub(super) fn ends(&self) -> (usize, usize) {
        (self.from, self.to)
    }
}

/// FR-239's outcome.
#[derive(Clone, Debug)]
pub enum ZoneCheckOutcome<S> {
    /// No admitted violation is reachable, and an initial state has a
    /// time-divergent behaviour.
    Holds {
        /// FR-244's certificate, built from the retained graph.
        certificate: Box<qsl_replay::ZoneCertificate<ItemObligation<S>>>,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// No admitted violation is reachable in the symmetry-reduced graph of
    /// any instance the run checked (FR-160, QSpec FR-430). No core checker
    /// reads a reduced graph, so it carries no certificate.
    HoldsReduced {
        /// The applied symmetry, one entry per instance checked.
        reductions: Vec<AppliedReduction>,
        /// The runs' counts together.
        stats: ZoneStats,
    },
    /// A reduction cause the pre-check settled, with no state explored.
    Reduction {
        /// The cause.
        cause: ZoneReduction,
        /// The run's counts: none.
        stats: ZoneStats,
    },
    /// The first admitted violation in breadth-first order, concretised
    /// with exact delays (FR-241), to be replayed before it counts.
    Violated {
        /// The counterexample.
        counterexample: Box<TemporalCounterexample>,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// A task deadline miss with its exact source and scheduler transitions.
    /// The settlement layer must replay this typed prefix and its divergent
    /// continuation before exposing `refuted`.
    TaskViolated {
        /// The task-derived counterexample.
        counterexample: Box<TemporalCounterexample<TaskSearchTransition>>,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// The run decided nothing.
    Undecided {
        /// Why.
        cause: ZoneUndecided,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// The run stopped.
    Stopped {
        /// Why.
        cause: ZoneStop,
        /// The run's counts.
        stats: ZoneStats,
        /// FR-331's replayable bounded task-product frontier when this run
        /// stopped at a task-search exploration limit.
        task_search: Option<Box<TaskSearchEvidence<ObligationIdentity>>>,
    },
}

/// Why EN-6 does not take the request.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ZoneCheckRefusal {
    /// The clause is of no profile EN-6 reads, a TT-2 window's negation
    /// starts a punctual interval at many positions, which no timed
    /// automaton reads (FR-242; Alur and Dill, 1994), or its claim
    /// automaton needs more clocks than the construction builds.
    #[error("the clause is of no form EN-6 decides")]
    UnsupportedForm,
    /// The subject has an unbounded population root (FR-239, FR-126):
    /// refused before any state is explored.
    #[error("the subject has an unbounded domain and requires a proof bound")]
    RequiresBound(RequiresBound),
    /// The subject's extent could not be classified (FR-126): neither
    /// `RequiresBound` nor an exploration.
    #[error("the subject's extent could not be classified: {0:?}")]
    Extent(Box<ClassifyFailure>),
    /// An atom's evaluation could not give a reading.
    #[error("the atom's evaluation stopped: {0}")]
    Atom(String),
    /// The request's obligation is not its item's: an authored claim's
    /// occurrence for a clause, `deadlock-freedom` or `time-lock-freedom`
    /// for those items.
    #[error("the request's obligation is not its item's")]
    ObligationMismatch,
    /// The clause's negation automaton could not be built (FR-240).
    #[error("the clause's automaton could not be built: {0}")]
    Automaton(AutomatonError),
    /// The refinement checker has not mapped clock places moved by a
    /// collection-valued write yet.
    #[error("the refinement checker does not map carried clock places")]
    CarriedClocks,
    /// A symmetry declaration does not admit (FR-151).
    #[error(transparent)]
    Symmetry(SymmetryRefusal),
    /// An invariant of the engine broke.
    #[error("the zone engine broke an invariant: {0}")]
    Fault(&'static str),
    /// The checked claim, task set and source release declarations do not
    /// form an admitted task-automaton subject.
    #[error("the checked claim cannot form a task-product subject: {0}")]
    TaskProductSubject(#[from] TaskProductSubjectError),
}

impl From<WindowRefusal> for ZoneCheckRefusal {
    fn from(refusal: WindowRefusal) -> Self {
        match refusal {
            WindowRefusal::UnsupportedForm | WindowRefusal::NestedWindow => Self::UnsupportedForm,
            WindowRefusal::FormulaNode | WindowRefusal::Broken(_) => Self::Fault("window-formula"),
        }
    }
}

impl From<DbmError> for ZoneCheckRefusal {
    fn from(_: DbmError) -> Self {
        Self::Fault("dbm-shape")
    }
}

/// What makes a node bad.
#[derive(Clone, Debug)]
pub(super) enum Badness {
    /// The claim's atom is false for the instance.
    False(ModelInstance),
    /// A condition of the claim, an atom or the activation's, is undefined
    /// for the instance.
    Undefined(
        ModelInstance,
        qsl_eval::temporal::UndefinedCause,
        UndefinedLocus,
    ),
    /// The node holds deadlocked valuations at entry, within the zone.
    Deadlock(Dbm),
}

/// A halt of the search.
pub(super) enum Halt {
    Outcome(Box<Searched>),
    Refusal(ZoneCheckRefusal),
}

impl From<ZoneCheckRefusal> for Halt {
    fn from(refusal: ZoneCheckRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<DbmError> for Halt {
    fn from(error: DbmError) -> Self {
        Self::Refusal(error.into())
    }
}

/// FR-239: decide `request.item` over `request.subject`.
///
/// # Errors
///
/// [`ZoneCheckRefusal`] when the clause is of no form EN-6 decides, its
/// automaton does not build, its instances are not finite, an atom gives
/// no reading, or the engine breaks an invariant.
pub fn check_timed_model<S: Clone>(
    request: ZoneCheckRequest<'_, '_, '_, S>,
    cancel: &Cancel,
) -> Result<ZoneCheckOutcome<S>, ZoneCheckRefusal> {
    // The certificate names the request's obligation, which must be the
    // item's: an authored claim's occurrence, or `deadlock-freedom`.
    let matches = match request.item {
        ZoneCheckItem::Clause { .. } => {
            matches!(request.obligation.kind, ObligationKind::Authored(_))
        }
        ZoneCheckItem::DeadlockFreedom => {
            request.obligation.kind == ObligationKind::DeadlockFreedom
        }
        ZoneCheckItem::TimeLockFreedom => {
            request.obligation.kind == ObligationKind::TimeLockFreedom
        }
        ZoneCheckItem::TaskSchedulability {
            claim, occurrence, ..
        } => {
            claim.occurrence().as_ref() == Some(occurrence)
                && matches!(
                    &request.obligation.kind,
                    ObligationKind::TaskSchedulability(obligation_occurrence)
                        if obligation_occurrence == occurrence
                )
        }
    };
    if !matches {
        return Err(ZoneCheckRefusal::ObligationMismatch);
    }
    // QSpec FR-430: EN-6 applies symmetry and no other reduction, so a
    // request selecting partial-order reduction or a state constraint
    // settles V-8 before anything is read, naming the first in the order
    // partial-order, constraint.
    let reductions = request.reductions;
    if let Some(reduction) = reductions
        .selected()
        .into_iter()
        .find(|reduction| *reduction != ReductionKind::Symmetry)
    {
        return Ok(reduced(ZoneReduction::Unsupported { reduction }));
    }
    // FR-239's bound check ran when the timed subject was built.
    let system = request.subject.system();
    let mut symmetric = !reductions.symmetry.is_empty();
    // EN-6 applies symmetry to its search only: the time-lock search and
    // the liveness product read the unreduced graph's edges, so an item
    // decided by either settles V-8 naming symmetry, as an unapplied
    // reduction does, before its declarations are admitted.
    if symmetric && reads_unreduced_graph(&request)? {
        return Ok(reduced(ZoneReduction::Unsupported {
            reduction: ReductionKind::Symmetry,
        }));
    }
    // FR-151: the declarations admit, and the initial states are closed
    // under their group.
    let group = admit(system, &reductions.symmetry).map_err(ZoneCheckRefusal::Symmetry)?;
    let closed = closure(system, &group).map_err(|refusal| match refusal {
        ClosureRefusal::Encoding(_) => ZoneCheckRefusal::Fault("state-key"),
        ClosureRefusal::Unordered => ZoneCheckRefusal::Fault("symmetry-closure"),
    })?;
    if let Some(ReductionCause::SymmetryBroken { population, cause }) = closed {
        return Ok(reduced(ZoneReduction::SymmetryBroken { population, cause }));
    }
    let task_subject = match request.item {
        ZoneCheckItem::TaskSchedulability {
            claim, admitted, ..
        } => Some(TaskProductSubject::new_admitted(
            system.package().graph().task_sets(),
            claim,
            admitted,
            system,
        )?),
        _ => None,
    };
    let task_search_identity = match (request.item, request.subject.system().over_identity()) {
        (ZoneCheckItem::TaskSchedulability { occurrence, .. }, Some(model)) => {
            Some(ObligationIdentity::new(
                occurrence.node().clone(),
                Some(ModelIdentity::new(model)),
                occurrence.origin().ordinal(),
            ))
        }
        _ => None,
    };
    // FR-151 "Instances": under symmetry, one run per orbit of the `over`
    // binding, bound to its least key, under its stabiliser; otherwise one
    // run over every instance.
    let plans = match request.item {
        ZoneCheckItem::Clause { clause, .. } => {
            let instances = system.instances(&clause.over().1).map_err(not_simulated)?;
            // With no instance there is no orbit to reduce: the item runs
            // unreduced, so its proof is certified.
            symmetric &= !instances.is_empty();
            if symmetric {
                instance_groups(Some(&instances), &group)
                    .into_iter()
                    .map(|instance| Plan {
                        instances: instance.representative.into_iter().collect(),
                        group: instance.group,
                        orbit: instance.orbit,
                    })
                    .collect()
            } else {
                vec![Plan {
                    instances,
                    group: SymmetryGroup::default(),
                    orbit: Vec::new(),
                }]
            }
        }
        ZoneCheckItem::DeadlockFreedom | ZoneCheckItem::TimeLockFreedom => vec![Plan {
            instances: Vec::new(),
            group: group.clone(),
            orbit: Vec::new(),
        }],
        ZoneCheckItem::TaskSchedulability { .. } => vec![Plan {
            instances: Vec::new(),
            group: group.clone(),
            orbit: Vec::new(),
        }],
    };
    let mut applied = Vec::new();
    let mut total = ZoneStats::default();
    let mut proved = None;
    for plan in plans {
        let (judged, window, live) = judge_plan(&request, plan.instances)?;
        let automaton_constants = live
            .as_ref()
            .map_or_else(Vec::new, PositionAutomaton::all_constants);
        let mut run = Run::new(
            request.subject,
            judged,
            window,
            &automaton_constants,
            request.limits,
            cancel,
            task_subject.clone(),
            task_search_identity.clone(),
        );
        run.live = live;
        run.group = plan.group.clone();
        run.reduced = symmetric;
        let searched = match run.search() {
            Ok(searched) => searched,
            Err(Halt::Outcome(searched)) => *searched,
            Err(Halt::Refusal(refusal)) => return Err(refusal),
        };
        match searched {
            Searched::Holds { proof, stats } => {
                total = total.plus(stats);
                proved = Some(proof);
            }
            Searched::HoldsReduced { stats } => {
                total = total.plus(stats);
                applied.push(AppliedReduction::Symmetry {
                    groups: group.classes.clone(),
                    orbit: plan.orbit,
                    stabiliser: plan.group.classes,
                });
            }
            Searched::Violated {
                counterexample,
                stats,
            } => {
                return Ok(ZoneCheckOutcome::Violated {
                    counterexample,
                    stats: total.plus(stats),
                })
            }
            Searched::TaskViolated {
                counterexample,
                stats,
            } => {
                return Ok(ZoneCheckOutcome::TaskViolated {
                    counterexample,
                    stats: total.plus(stats),
                })
            }
            Searched::Undecided { cause, stats } => {
                return Ok(ZoneCheckOutcome::Undecided {
                    cause,
                    stats: total.plus(stats),
                })
            }
            Searched::Stopped {
                cause,
                stats,
                task_search,
            } => {
                return Ok(ZoneCheckOutcome::Stopped {
                    cause,
                    stats: total.plus(stats),
                    task_search,
                })
            }
        }
    }
    Ok(match proved {
        Some(proof) if !symmetric => ZoneCheckOutcome::Holds {
            certificate: Box::new(proof.proving(request.obligation)),
            stats: total,
        },
        // Unreachable by design: a run under a selected symmetry sets
        // `reduced`, whose proof is always `HoldsReduced`.
        Some(_) => return Err(ZoneCheckRefusal::Fault("reduced-certificate")),
        None => ZoneCheckOutcome::HoldsReduced {
            reductions: applied,
            stats: total,
        },
    })
}

/// Whether `request`'s item is decided on a graph EN-6 does not reduce:
/// the time-lock-freedom item, or a clause with no TT-1 atom and no flat
/// TT-2 window, which the liveness product decides (FR-240).
fn reads_unreduced_graph<S>(
    request: &ZoneCheckRequest<'_, '_, '_, S>,
) -> Result<bool, ZoneCheckRefusal> {
    Ok(match request.item {
        ZoneCheckItem::TimeLockFreedom => true,
        ZoneCheckItem::TaskSchedulability { .. } => true,
        ZoneCheckItem::DeadlockFreedom => false,
        ZoneCheckItem::Clause { clause, .. } => {
            if qsl_eval::automaton::invariant_atom(clause).is_some() {
                false
            } else {
                let instances = request
                    .subject
                    .system()
                    .instances(&clause.over().1)
                    .map_err(not_simulated)?;
                flat_window(
                    clause,
                    instances.len(),
                    request.subject.tick_period().cloned(),
                    request.limits.max_automaton_states,
                )?
                .is_none()
            }
        }
    })
}

/// One run of a request: the instances its claim reads, the group it is
/// reduced by, and the keys its one bound instance stands for (FR-151).
struct Plan {
    instances: Vec<ModelInstance>,
    group: SymmetryGroup,
    orbit: Vec<String>,
}

/// The outcome the pre-check settles, with no state explored.
fn reduced<S>(cause: ZoneReduction) -> ZoneCheckOutcome<S> {
    ZoneCheckOutcome::Reduction {
        cause,
        stats: ZoneStats::default(),
    }
}

/// What a run over `instances` judges: the clause read by TT-1's atom,
/// TT-2's window automaton or FR-240's liveness product, or a derived item.
fn judge_plan<'r, S>(
    request: &ZoneCheckRequest<'r, '_, '_, S>,
    instances: Vec<ModelInstance>,
) -> Result<(Judged<'r>, Option<WindowClaim>, Option<PositionAutomaton>), ZoneCheckRefusal> {
    let mut window = None;
    let mut live = None;
    let judged = match request.item {
        ZoneCheckItem::Clause { clause, letters } => {
            if let Some(atom) = qsl_eval::automaton::invariant_atom(clause) {
                if !matches!(
                    clause.profile(),
                    TemporalProfile::InfiniteTrace | TemporalProfile::Timed
                ) {
                    return Err(ZoneCheckRefusal::UnsupportedForm);
                }
                Judged::Claim(Claim {
                    clause,
                    atom: Some(atom),
                    letters,
                    instances,
                })
            } else if let Some(claim) = flat_window(
                clause,
                instances.len(),
                request.subject.tick_period().cloned(),
                request.limits.max_automaton_states,
            )? {
                window = Some(claim);
                Judged::Claim(Claim {
                    clause,
                    atom: None,
                    letters,
                    instances,
                })
            } else if matches!(
                clause.profile(),
                TemporalProfile::InfiniteTrace | TemporalProfile::Timed
            ) {
                // FR-240: any other clause is decided on the liveness
                // product, which reads it over positions; one with a timed
                // interval through FR-242's timed claim automaton (TT-3,
                // TT-4, and a TT-2 window nesting one interval operator
                // under another). A punctual interval started at many
                // positions has no timed automaton, and a claim needing
                // more clocks than the construction builds is no form EN-6
                // decides: both are `UnsupportedForm`, as the FR-245
                // checker reads them (`UnsupportedItem`).
                let automaton = position_automaton(clause, request.limits.max_automaton_states)
                    .map_err(|error| match error {
                        AutomatonError::ProfileNotTranslated { .. }
                        | AutomatonError::RepeatedPunctualInterval
                        | AutomatonError::ClockLimit { .. } => ZoneCheckRefusal::UnsupportedForm,
                        other => ZoneCheckRefusal::Automaton(other),
                    })?;
                live = Some(automaton);
                Judged::Liveness(Claim {
                    clause,
                    atom: None,
                    letters,
                    instances,
                })
            } else {
                return Err(ZoneCheckRefusal::UnsupportedForm);
            }
        }
        ZoneCheckItem::DeadlockFreedom => Judged::DeadlockFreedom,
        ZoneCheckItem::TimeLockFreedom => Judged::TimeLockFreedom,
        ZoneCheckItem::TaskSchedulability { .. } => Judged::TaskSchedulability,
    };
    Ok((judged, window, live))
}

/// QSpec FR-430: the canonical form of the symbolic state `(state, zone)`
/// of `subject` under `group`, with the permutation that gives it: the
/// model state by FR-152's permutation, ties between objects of equal
/// state in key order, and the zone with each object's clocks moved to
/// those of the object the permutation maps it to.
///
/// # Errors
///
/// [`ZoneCheckRefusal::Fault`] when the canonicaliser stops or a clock has
/// no image.
pub fn canonical_symbolic_state(
    subject: &TimedSystem<'_, '_>,
    group: &SymmetryGroup,
    state: &ModelSimState,
    zone: &Dbm,
) -> Result<(ModelSimState, Dbm, Permutation), ZoneCheckRefusal> {
    let (canonical, permutation) = subject
        .system()
        .canonicalise(state, group)
        .map_err(|_| ZoneCheckRefusal::Fault("canonicalise"))?;
    let zone = renamed_zone(subject, zone, &permutation)?;
    Ok((canonical, zone, permutation))
}

/// `zone` with each of `subject`'s objects' clocks moved to those of the
/// object `permutation` maps it to.
fn renamed_zone(
    subject: &TimedSystem<'_, '_>,
    zone: &Dbm,
    permutation: &Permutation,
) -> Result<Dbm, ZoneCheckRefusal> {
    let images = subject
        .place_images(permutation)
        .ok_or(ZoneCheckRefusal::Fault("clock-image"))?;
    Ok(zone.renamed(&images)?)
}

/// A run's outcome before its certificate names the item's obligation.
pub(super) enum Searched {
    Holds {
        proof: Box<Proof>,
        stats: ZoneStats,
    },
    /// No admitted violation in a symmetry-reduced graph, which no
    /// certificate covers.
    HoldsReduced {
        stats: ZoneStats,
    },
    Violated {
        counterexample: Box<TemporalCounterexample>,
        stats: ZoneStats,
    },
    TaskViolated {
        counterexample: Box<TemporalCounterexample<TaskSearchTransition>>,
        stats: ZoneStats,
    },
    Undecided {
        cause: ZoneUndecided,
        stats: ZoneStats,
    },
    Stopped {
        cause: ZoneStop,
        stats: ZoneStats,
        task_search: Option<Box<TaskSearchEvidence<ObligationIdentity>>>,
    },
}

/// What a run judges its nodes by.
pub(super) enum Judged<'r> {
    /// An authored clause.
    Claim(Claim<'r>),
    /// The deadlock-freedom item: a node holding deadlocked valuations.
    DeadlockFreedom,
    /// The time-lock-freedom item, which judges no node during the search:
    /// FR-240's time-lock search runs on the completed graph.
    TimeLockFreedom,
    /// An authored clause decided on FR-240's liveness product, built
    /// after the search, which judges no node.
    Liveness(Claim<'r>),
    /// A refinement's safety half (QSpec FR-430), which judges its edges,
    /// not its nodes ([`super::refine`]); a node's `q` is its refinement
    /// position.
    Refinement,
    /// FR-249's checked task schedulability invariant.
    TaskSchedulability,
}

pub(super) struct Claim<'r> {
    pub(super) clause: &'r CheckedTemporalClause,
    pub(super) letters: &'r dyn ModelLetters,
    /// A TT-1 clause's atom; `None` for a TT-2 clause, which the run's
    /// [`WindowClaim`] reads, and for a liveness product's.
    pub(super) atom: Option<FormulaNodeId>,
    pub(super) instances: Vec<ModelInstance>,
}

pub(super) struct Run<'r, 's, 'a> {
    pub(super) timed: &'r TimedSystem<'s, 'a>,
    judged: Judged<'r>,
    /// A TT-2 clause's automaton, whose horizon clock is the last clock of
    /// every zone.
    pub(super) window: Option<WindowClaim>,
    /// The negation automaton of a clause FR-240's liveness product
    /// decides.
    pub(super) live: Option<PositionAutomaton>,
    /// The pieces of the horizon axis a TT-2 position is read on, computed
    /// once: under a tick source, one, since the automaton keeps the stamp.
    pub(super) pieces: Vec<(Vec<PlaceConstraint>, Rational)>,
    pub(super) limits: ZoneCheckLimits,
    pub(super) cancel: &'r Cancel,
    pub(super) scale: BigInt,
    pub(super) lower: Vec<Option<BigInt>>,
    pub(super) upper: Vec<Option<BigInt>>,
    pub(super) nodes: Vec<SymbolicNode>,
    keys: Vec<Vec<u8>>,
    expanded: Vec<bool>,
    /// The real edge that created each node, `None` for an initial one.
    parent: Vec<Option<usize>>,
    pub(super) edges: Vec<SymbolicEdge>,
    /// The retained edges leaving each node, by index into `edges`.
    pub(super) out: Vec<Vec<usize>>,
    passed: BTreeMap<Vec<u8>, Vec<usize>>,
    /// The bytes of every zone stored, by the search and by its divergence
    /// checks.
    zone_bytes: u64,
    /// The symbolic states and transitions the divergence checks stored,
    /// charged with the search's own against the same budgets.
    escape_states: u64,
    escape_edges: u64,
    /// The guarded steps of each model state reached, by its key.
    steps: BTreeMap<Vec<u8>, Vec<GuardedStep>>,
    /// The first instance an admitted behaviour read short of data for.
    insufficient: Option<ModelInstance>,
    /// The group the run's states are canonicalised under (QSpec FR-430).
    pub(super) group: SymmetryGroup,
    /// Whether the request selected symmetry, so a proof is `Reduced`.
    reduced: bool,
    /// FR-249's checked task-automaton contract, present on task searches.
    pub(super) task_subject: Option<TaskProductSubject>,
    /// Appended task-clock positions, when the exact finite route is eligible.
    pub(super) task_clock_layout: Option<TaskClockLayout>,
    /// FR-331 identity for a bounded task-search frontier.
    task_search_identity: Option<ObligationIdentity>,
    /// Task-product node expansions started by this bounded search.
    task_product_work: u64,
    /// Nodes whose concrete pending task releases have unresolved dynamics.
    unresolved_task_nodes: Vec<usize>,
    /// Reachable deadline misses. Exact task searches continue through them
    /// so a mixed source/task positive-time lasso can be replayed.
    task_miss_nodes: Vec<(usize, TaskTransitionIdentity, Dbm)>,
}

impl<'r, 's, 'a> Run<'r, 's, 'a> {
    #[allow(
        clippy::too_many_arguments,
        reason = "Run construction keeps the checked analysis inputs explicit"
    )]
    pub(super) fn new(
        timed: &'r TimedSystem<'s, 'a>,
        judged: Judged<'r>,
        window: Option<WindowClaim>,
        automaton_constants: &[Rational],
        limits: ZoneCheckLimits,
        cancel: &'r Cancel,
        task_subject: Option<TaskProductSubject>,
        task_search_identity: Option<ObligationIdentity>,
    ) -> Self {
        let mut constants = automaton_constants.to_vec();
        if let Some(TaskProductRoute::Exact(product)) =
            task_subject.as_ref().map(TaskProductSubject::route)
        {
            for task in &product.tasks {
                constants.extend([
                    task.interarrival.clone(),
                    task.relative_deadline.clone(),
                    task.max_jitter.clone(),
                    task.work.clone(),
                ]);
            }
        }
        let (scale, mut lower, mut upper) = scaled_bounds(timed, window.as_ref(), &constants);
        if matches!(judged, Judged::DeadlockFreedom | Judged::TimeLockFreedom) {
            // The deadlock-freedom and time-lock-freedom items read
            // valuations whose absence of behaviour the aLU simulation does
            // not preserve: they cover under the aM bisimulation,
            // `L = U = max(L, U)` per clock, which the deadlock item's
            // certificate checker reads (FR-245) and FR-240's trap search
            // needs (`super::time_lock`).
            for (lower, upper) in lower.iter_mut().zip(&mut upper) {
                let most = lower.clone().max(upper.clone());
                lower.clone_from(&most);
                *upper = most;
            }
        }
        let pieces = match (&window, timed.tick_period()) {
            (Some(window), None) => window.pieces(timed.places().len()),
            _ => vec![(Vec::new(), zero_stamp())],
        };
        let task_clock_layout = match task_subject.as_ref().map(TaskProductSubject::route) {
            Some(TaskProductRoute::Exact(product)) => {
                let base_dim = timed.places().len() + 1 + usize::from(window.is_some());
                match TaskClockLayout::new(product, base_dim, &scale) {
                    Ok(layout) => {
                        for ceiling in layout.ceilings() {
                            lower.push(Some(BigInt::from(0)));
                            upper.push(Some(ceiling.maximum.as_big().clone()));
                        }
                        Some(layout)
                    }
                    Err(_) => None,
                }
            }
            Some(TaskProductRoute::Bounded(_) | TaskProductRoute::NoRelease) | None => None,
        };
        Self {
            timed,
            judged,
            window,
            live: None,
            pieces,
            limits,
            cancel,
            scale,
            lower,
            upper,
            nodes: Vec::new(),
            keys: Vec::new(),
            expanded: Vec::new(),
            parent: Vec::new(),
            edges: Vec::new(),
            out: Vec::new(),
            passed: BTreeMap::new(),
            zone_bytes: 0,
            escape_states: 0,
            escape_edges: 0,
            steps: BTreeMap::new(),
            insufficient: None,
            group: SymmetryGroup::default(),
            reduced: false,
            task_subject,
            task_clock_layout,
            task_search_identity,
            task_product_work: 0,
            unresolved_task_nodes: Vec::new(),
            task_miss_nodes: Vec::new(),
        }
    }

    /// The authored clause the run judges, if any.
    pub(super) fn claim(&self) -> Option<&Claim<'r>> {
        match &self.judged {
            Judged::Claim(claim) | Judged::Liveness(claim) => Some(claim),
            Judged::DeadlockFreedom
            | Judged::TimeLockFreedom
            | Judged::Refinement
            | Judged::TaskSchedulability => None,
        }
    }

    pub(super) fn stats(&self) -> ZoneStats {
        ZoneStats {
            symbolic_states: widen(self.nodes.len()),
            retained_edges: widen(self.edges.len()),
            automaton_states: self
                .window
                .as_ref()
                .map_or(0, |window| widen(window.state_count())),
            zone_bytes: self.zone_bytes,
        }
    }

    pub(super) fn stop(&self, cause: ZoneStop) -> Halt {
        let task_search = match self.task_search_evidence(cause) {
            Ok(evidence) => evidence.map(Box::new),
            Err(refusal) => return Halt::Refusal(refusal),
        };
        Halt::Outcome(Box::new(Searched::Stopped {
            cause,
            stats: self.stats(),
            task_search,
        }))
    }

    fn task_search_evidence(
        &self,
        cause: ZoneStop,
    ) -> Result<Option<TaskSearchEvidence<ObligationIdentity>>, ZoneCheckRefusal> {
        if !matches!(
            self.task_subject.as_ref().map(TaskProductSubject::route),
            Some(TaskProductRoute::Bounded(_))
        ) {
            return Ok(None);
        }
        let Some(identity) = self.task_search_identity.clone() else {
            return Ok(None);
        };
        let (limit, reason) = match cause {
            ZoneStop::TaskProductUnresolved => (None, TaskSearchStopReason::UnresolvedChoice),
            ZoneStop::LimitReached(limit) => {
                let (kind, value, reason) = match limit {
                    ZoneCheckLimit::MaxSymbolicStates(value) => (
                        TaskSearchLimitKind::States,
                        value,
                        TaskSearchStopReason::StateLimit,
                    ),
                    ZoneCheckLimit::MaxSymbolicTransitions(value) => (
                        TaskSearchLimitKind::Transitions,
                        value,
                        TaskSearchStopReason::TransitionLimit,
                    ),
                    ZoneCheckLimit::MaxClocks(value) => (
                        TaskSearchLimitKind::Clocks,
                        value,
                        TaskSearchStopReason::ClockLimit,
                    ),
                    ZoneCheckLimit::TaskProductWork(value) => (
                        TaskSearchLimitKind::Work,
                        value,
                        TaskSearchStopReason::WorkLimit,
                    ),
                    ZoneCheckLimit::MaxAutomatonStates(_) | ZoneCheckLimit::MaxZoneBytes(_) => {
                        return Ok(None);
                    }
                };
                (
                    Some(TaskSearchLimit {
                        kind,
                        value,
                        setting: kind.setting().to_owned(),
                    }),
                    reason,
                )
            }
            ZoneStop::Cancelled => return Ok(None),
        };
        let unresolved_stop = matches!(cause, ZoneStop::TaskProductUnresolved);
        let roots: Vec<_> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.initial.map(|_| index))
            .collect();
        if roots.is_empty() {
            return Err(ZoneCheckRefusal::Fault("task-search-root"));
        }
        let mut depths = vec![u64::MAX; self.nodes.len()];
        let mut queue = VecDeque::new();
        for root in roots {
            depths[root] = 0;
            queue.push_back(root);
        }
        while let Some(node) = queue.pop_front() {
            for edge in &self.out[node] {
                let target = self.edges[*edge].to;
                if depths[target] == u64::MAX {
                    depths[target] = depths[node].saturating_add(1);
                    queue.push_back(target);
                }
            }
        }
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            let scheduler = node
                .task_state
                .as_ref()
                .ok_or(ZoneCheckRefusal::Fault("task-search-scheduler"))?;
            let (_, source) = self
                .timed
                .system()
                .state_key(&node.state)
                .map_err(|_| ZoneCheckRefusal::Fault("task-search-state-key"))?;
            let state = TaskProductStateKey::new(
                source,
                scheduler
                    .canonical_control_key()
                    .map_err(|_| ZoneCheckRefusal::Fault("task-search-state-key"))?,
            )
            .into_state_key()
            .map_err(|_| ZoneCheckRefusal::Fault("task-search-state-key"))?;
            nodes.push(TaskSearchNode {
                state,
                depth: depths[index],
                zone: super::certificate::core_zone(&node.zone)
                    .map_err(|_| ZoneCheckRefusal::Fault("task-search-zone"))?,
                expanded: self.expanded[index],
            });
        }
        let mut edges = Vec::with_capacity(self.edges.len());
        for edge in &self.edges {
            let transition = match (&edge.task_transition, edge.step) {
                (Some(task), _) => TaskSearchTransition::Task(Box::new(task.clone())),
                (None, Some((step, _))) => {
                    let source = self
                        .nodes
                        .get(edge.from)
                        .ok_or(ZoneCheckRefusal::Fault("task-search-edge"))?;
                    let (key, _) = self
                        .timed
                        .system()
                        .state_key(&source.state)
                        .map_err(|_| ZoneCheckRefusal::Fault("task-search-state-key"))?;
                    let steps = self
                        .steps
                        .get(&key)
                        .ok_or(ZoneCheckRefusal::Fault("task-search-steps"))?;
                    let transition = steps
                        .get(step)
                        .ok_or(ZoneCheckRefusal::Fault("task-search-edge"))?
                        .transition()
                        .clone();
                    TaskSearchTransition::Source(transition)
                }
                (None, None) => return Err(ZoneCheckRefusal::Fault("task-search-edge")),
            };
            edges.push(TaskSearchEdge {
                node: edge.from,
                transition,
                target: edge.to,
            });
        }
        let frontier =
            self.expanded
                .iter()
                .enumerate()
                .filter_map(|(node, expanded)| {
                    if *expanded {
                        return None;
                    }
                    let pending = self.nodes[node]
                        .task_state
                        .as_ref()
                        .is_some_and(|state| !state.pending_releases().is_empty());
                    let open = if pending {
                        let mut open = vec![TaskSearchOpen::Demand, TaskSearchOpen::Queue];
                        if self.task_subject.as_ref().is_some_and(|subject| {
                            subject.task_set().tasks.iter().any(|task| {
                                task.criticality == qsl_semantics::check::Criticality::Hi
                            })
                        }) {
                            open.push(TaskSearchOpen::BudgetChange);
                        }
                        open
                    } else {
                        vec![TaskSearchOpen::Successors]
                    };
                    Some(TaskSearchFrontier {
                        node,
                        reason: if unresolved_stop {
                            TaskSearchStopReason::UnresolvedChoice
                        } else {
                            reason
                        },
                        open,
                    })
                })
                .collect();
        let evidence = TaskSearchEvidence {
            identity,
            nodes,
            edges,
            frontier,
            limit,
        };
        if !evidence.has_valid_shape() {
            return Err(ZoneCheckRefusal::Fault("task-search-shape"));
        }
        Ok(Some(evidence))
    }

    pub(super) fn undecided(&self, cause: ZoneUndecided) -> Halt {
        Halt::Outcome(Box::new(Searched::Undecided {
            cause,
            stats: self.stats(),
        }))
    }

    pub(super) fn poll(&self) -> Result<(), Halt> {
        if self.cancel.poll() {
            return Err(self.stop(ZoneStop::Cancelled));
        }
        Ok(())
    }

    pub(super) fn timed<T>(&self, result: Result<T, TimedStop>) -> Result<T, Halt> {
        result.map_err(|stop| match stop {
            TimedStop::MaxClocks { max_clocks } => self.stop(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxClocks(max_clocks),
            )),
            stop => self.undecided(ZoneUndecided::UndecidedSuccessor(stop)),
        })
    }

    pub(super) fn scaled(&self, value: &Rational) -> BigInt {
        scaled(value, &self.scale)
    }

    /// The dimension of every zone: the reference clock, the clock places
    /// and, for a TT-2 clause, the horizon clock.
    pub(super) fn dim(&self) -> usize {
        self.task_clock_layout.as_ref().map_or_else(
            || self.timed.places().len() + 1 + usize::from(self.window.is_some()),
            TaskClockLayout::dim,
        )
    }

    /// Intersect `zone` with `constraints`.
    pub(super) fn constrain(
        &self,
        zone: &mut Dbm,
        constraints: &[PlaceConstraint],
    ) -> Result<(), DbmError> {
        constrain_at(zone, constraints, &self.scale)
    }

    /// Whether time may elapse at `state`.
    pub(super) fn elapses(&self, state: &ModelSimState) -> Result<bool, Halt> {
        Ok(self.timed.tick_period().is_none() && !self.timed(self.timed.is_urgent(state))?)
    }

    /// `zone` intersected with `state`'s invariants, let time elapse unless
    /// `state` is urgent, and intersected with them again.
    pub(super) fn settle(&self, state: &ModelSimState, zone: &mut Dbm) -> Result<(), Halt> {
        self.settle_at(state, zone, &self.scale)
    }

    /// [`Self::settle`] with bounds in units of `1 / factor`.
    pub(super) fn settle_at(
        &self,
        state: &ModelSimState,
        zone: &mut Dbm,
        factor: &BigInt,
    ) -> Result<(), Halt> {
        let invariants = self.timed(self.timed.invariants(state))?;
        constrain_at(zone, &invariants, factor)?;
        if !zone.is_empty() && self.elapses(state)? {
            zone.up();
            constrain_at(zone, &invariants, factor)?;
        }
        Ok(())
    }

    /// FR-249's source plus scheduler invariants, with release activation
    /// edges taking no time. All auxiliary task clocks remain unit-rate.
    fn settle_product_at(
        &self,
        state: &ModelSimState,
        scheduler: &TaskSchedulerState,
        zone: &mut Dbm,
        factor: &BigInt,
    ) -> Result<(), Halt> {
        let invariants = self.timed(self.timed.invariants(state))?;
        constrain_at(zone, &invariants, factor)?;
        self.task_invariants(scheduler, zone, factor)?;
        if !zone.is_empty()
            && scheduler.pending_releases().is_empty()
            && !self.task_action_urgent(scheduler)?
            && self.elapses(state)?
        {
            zone.up();
            constrain_at(zone, &invariants, factor)?;
            self.task_invariants(scheduler, zone, factor)?;
        }
        Ok(())
    }

    /// Interarrival, jitter, service and nominal-deadline constraints of one
    /// checked fixed-work product state.
    fn task_invariants(
        &self,
        scheduler: &TaskSchedulerState,
        zone: &mut Dbm,
        _factor: &BigInt,
    ) -> Result<(), Halt> {
        let Some(subject) = &self.task_subject else {
            return Ok(());
        };
        let TaskProductRoute::Exact(product) = subject.route() else {
            // The bounded route explores the source graph until a checked
            // release reaches the frontier. It has no task clocks because
            // the unresolved continuous task choice is recorded explicitly.
            return Ok(());
        };
        let layout = self
            .task_clock_layout
            .clone()
            .ok_or(ZoneCheckRefusal::Fault("task-clock-layout"))?;
        for job in scheduler.jobs() {
            let task = product
                .tasks
                .iter()
                .find(|task| {
                    task.task_set == job.identity.taskset && task.task == job.identity.task
                })
                .ok_or(ZoneCheckRefusal::Fault("task-job-binding"))?;
            if job.phase == TaskJobPhase::Missed {
                continue;
            }
            let Some(service) = layout.service_clock(&job.identity) else {
                return Err(ZoneCheckRefusal::Fault("task-service-clock").into());
            };
            zone.constrain(service.index, 0, Bound::le(service.maximum))?;
            if job.phase == super::task_product::TaskJobPhase::Running {
                zone.constrain(service.index, 0, Bound::le(self.scaled(&task.work)))?;
            }
            if job.phase != super::task_product::TaskJobPhase::Missed {
                let deadline = layout
                    .deadline_index(&job.identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-deadline-clock"))?;
                zone.constrain(deadline, 0, Bound::le(self.scaled(&task.relative_deadline)))?;
            }
            if job.phase == super::task_product::TaskJobPhase::Pending {
                if let Some(jitter) = layout.jitter_index(&job.identity) {
                    zone.constrain(jitter, 0, Bound::le(self.scaled(&task.max_jitter)))?;
                }
            }
        }
        Ok(())
    }

    /// The symbolic successors of `(state, zone)` in canonical order, each
    /// with its step's and part's indices among `state`'s guarded steps, and
    /// whether the step is the tick operation of a tick source. Bounds are
    /// in units of `1 / factor`; a clock past the places is untouched but
    /// for elapsing time.
    pub(super) fn successors(
        &mut self,
        state: &ModelSimState,
        zone: &Dbm,
        factor: &BigInt,
    ) -> Result<Vec<Successor>, Halt> {
        let mut found = Vec::new();
        for mut successor in self.positions(state, zone, factor)? {
            self.settle_at(&successor.state, &mut successor.zone, factor)?;
            if !successor.zone.is_empty() {
                found.push(successor);
            }
        }
        Ok(found)
    }

    /// Source steps and their checked task-release declarations as one
    /// product successor relation. Every source and task clock shares one
    /// dense-time delay before the next discrete edge.
    fn product_successors(
        &mut self,
        state: &ModelSimState,
        scheduler: &TaskSchedulerState,
        zone: &Dbm,
        factor: &BigInt,
    ) -> Result<Vec<Successor>, Halt> {
        // A checked release is an immediate derived activation. Drain every
        // declaration queued by the preceding source transition before the
        // source can take another step; otherwise a source self-loop could
        // accumulate an unbounded queue of duplicate pending releases.
        if !scheduler.pending_releases().is_empty() {
            return Ok(Vec::new());
        }
        let mut found = Vec::new();
        for mut successor in self.positions(state, zone, factor)? {
            let mut next_scheduler = scheduler.clone();
            if !self.guard_task_releases(scheduler, state, successor.step, &mut successor.zone)? {
                continue;
            }
            self.queue_task_releases(&mut next_scheduler, state, successor.step)?;
            self.settle_product_at(
                &successor.state,
                &next_scheduler,
                &mut successor.zone,
                factor,
            )?;
            if !successor.zone.is_empty() {
                successor.task_state = Some(next_scheduler);
                found.push(successor);
            }
        }
        Ok(found)
    }

    /// Constrain the source transition by every checked release's shared
    /// inter-arrival clock before the source step is taken.
    fn guard_task_releases(
        &mut self,
        scheduler: &TaskSchedulerState,
        state: &ModelSimState,
        step_index: usize,
        zone: &mut Dbm,
    ) -> Result<bool, Halt> {
        let (product, releases) = match &self.task_subject {
            Some(subject) => match subject.route() {
                TaskProductRoute::Exact(product) => (product.clone(), subject.releases().to_vec()),
                TaskProductRoute::Bounded(_) | TaskProductRoute::NoRelease => return Ok(true),
            },
            None => return Ok(true),
        };
        let layout = self
            .task_clock_layout
            .clone()
            .ok_or(ZoneCheckRefusal::Fault("task-clock-layout"))?;
        let steps = self.steps_for(state)?;
        let step = steps
            .get(step_index)
            .ok_or(ZoneCheckRefusal::Fault("task-release-step"))?;
        let operation = step.transition().operation();
        let releases: Vec<_> = releases
            .iter()
            .filter(|release| {
                self.timed
                    .system()
                    .operation_text(release.operation().declaring(), release.operation().name())
                    == Some(operation)
            })
            .cloned()
            .collect();
        for release in releases {
            let arrival = layout
                .arrival_index(release.task_set(), release.task())
                .ok_or(ZoneCheckRefusal::Fault("task-arrival-clock"))?;
            if !scheduler
                .constrain_arrival_guard(&product, release.task(), arrival, &self.scale, zone)
                .map_err(|_| ZoneCheckRefusal::Fault("task-arrival-guard"))?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// [`Self::successors`] at the instant of each step: the valuations
    /// the step reaches before time elapses at its target, which a claim
    /// automaton reads at the step's position.
    fn positions(
        &mut self,
        state: &ModelSimState,
        zone: &Dbm,
        factor: &BigInt,
    ) -> Result<Vec<Successor>, Halt> {
        let mut found = Vec::new();
        for (index, step) in self.steps_for(state)?.iter().enumerate() {
            let target = self.timed(self.timed.invariants(step.post()))?;
            let tick = self.timed.tick_of(step.operation())
                > Rational::from_integer(quire_exact::Integer::zero());
            for (part_index, part) in step.parts().iter().enumerate() {
                self.poll()?;
                let mut next = zone.clone();
                constrain_at(&mut next, part, factor)?;
                if next.is_empty() {
                    continue;
                }
                apply_step_clocks(&mut next, step, factor)?;
                constrain_at(&mut next, &target, factor)?;
                if next.is_empty() {
                    continue;
                }
                found.push(Successor {
                    step: index,
                    part: part_index,
                    state: step.post().clone(),
                    zone: next,
                    tick,
                    task_state: None,
                    task_transition: None,
                });
            }
        }
        Ok(found)
    }

    fn search(&mut self) -> Result<Searched, Halt> {
        let initial = self.timed.initial();
        if initial.is_empty() {
            return Err(self.undecided(ZoneUndecided::NoInitialState));
        }
        if self.window.is_some() && self.limits.max_automaton_states == 0 {
            return Err(
                self.stop(ZoneStop::LimitReached(ZoneCheckLimit::MaxAutomatonStates(
                    0,
                ))),
            );
        }
        let dim = self.dim();
        let mut queue = VecDeque::new();
        let mut roots = Vec::new();
        let mut delayed_deadlock = false;
        for (index, state) in initial.iter().enumerate() {
            let discrete = state.discrete().clone();
            let mut zone = Dbm::zero(dim);
            let task_state = self
                .task_subject
                .as_ref()
                .map(|_| TaskSchedulerState::default());
            // Position 0 is read at time 0, before time elapses.
            let q = match self.window {
                Some(_) => {
                    self.window_position(None, &discrete, Stamp::At(&zero_stamp()), false)?
                }
                None => 0,
            };
            if let Some(scheduler) = &task_state {
                self.settle_product_at(&discrete, scheduler, &mut zone, &self.scale.clone())?;
            } else {
                self.settle(&discrete, &mut zone)?;
            }
            if zone.is_empty() {
                continue;
            }
            if let Some(node) =
                self.insert_with_task(discrete, task_state, q, zone, Some(index), None)?
            {
                roots.push(node);
                queue.push_back(node);
                delayed_deadlock |= self.judge(node)?;
            }
        }
        if roots.is_empty() {
            return Err(self.undecided(ZoneUndecided::NoInitialState));
        }
        while let Some(node) = queue.pop_front() {
            self.poll()?;
            if self.unresolved_task_nodes.contains(&node) {
                continue;
            }
            if matches!(
                self.task_subject.as_ref().map(TaskProductSubject::route),
                Some(TaskProductRoute::Bounded(_))
            ) && self.nodes[node]
                .task_state
                .as_ref()
                .is_some_and(|state| !state.pending_releases().is_empty())
            {
                self.unresolved_task_nodes.push(node);
                continue;
            }
            for created in self.expand(node)? {
                queue.push_back(created);
                delayed_deadlock |= self.judge(created)?;
            }
        }
        if !self.task_miss_nodes.is_empty() {
            for (edge, job, zone) in self.task_miss_nodes.clone() {
                if let Some(counterexample) = self.task_miss_counterexample(edge, &job, &zone)? {
                    return Ok(Searched::TaskViolated {
                        counterexample: Box::new(counterexample),
                        stats: self.stats(),
                    });
                }
            }
            // A reached miss with no replayable positive-time continuation
            // bars Holds. It is not a Refuted result or a fabricated search
            // limit; the retained task-search graph remains the evidence.
            if matches!(self.judged, Judged::TaskSchedulability) {
                return Err(self.undecided(ZoneUndecided::LassoNotConcretized));
            }
        }
        if !self.unresolved_task_nodes.is_empty() {
            return Err(self.stop(ZoneStop::TaskProductUnresolved));
        }
        // FR-240: the time-lock-freedom item's search runs on the completed
        // graph, before vacuity, which bars a proof only.
        if matches!(self.judged, Judged::TimeLockFreedom) {
            if let Some(counterexample) = self.first_time_lock()? {
                return Ok(Searched::Violated {
                    counterexample: Box::new(counterexample),
                    stats: self.stats(),
                });
            }
        }
        // Vacuity (ADR-026 TD-6): an initial state with a time-divergent
        // behaviour.
        let mut admitted = None;
        for &root in &roots {
            let escape = self.node_escape(root)?;
            if escape.diverges {
                admitted = Some((root, escape.witness));
                break;
            }
        }
        match admitted {
            None => Err(self.undecided(ZoneUndecided::NoAdmittedBehaviour)),
            // STD-493: a deadlock reached only by waiting at a quiescent
            // state has no counterexample form (FR-236 gives `final_delay`
            // to time-locks only), so it is neither proved away nor refuted.
            Some(_) if delayed_deadlock => Err(self.undecided(ZoneUndecided::DeadlockAfterDelay)),
            // ADR-024 WA-4: an admitted behaviour read a window short of
            // data, which the evaluator reports `InsufficientData`.
            Some(_) if self.insufficient.is_some() => {
                let instance = self.insufficient.take();
                Err(self.undecided(ZoneUndecided::InsufficientData(
                    instance.ok_or(ZoneCheckRefusal::Fault("insufficient-instance"))?,
                )))
            }
            // FR-160: a symmetry-reduced graph has no certificate.
            Some(_) if self.reduced => Ok(Searched::HoldsReduced {
                stats: self.stats(),
            }),
            Some((root, witness)) => {
                // FR-240: a claim decided on the liveness product, once
                // the subject admits a behaviour; its proof is the
                // product's (FR-244).
                if matches!(self.judged, Judged::Liveness(_)) {
                    let proof = match self.liveness()? {
                        Liveness::Violated(counterexample) => {
                            return Ok(Searched::Violated {
                                counterexample,
                                stats: self.stats(),
                            });
                        }
                        Liveness::Proved(products) => self.live_certificate(products)?,
                    };
                    return Ok(Searched::Holds {
                        proof: Box::new(proof),
                        stats: self.stats(),
                    });
                }
                let witness = witness.ok_or(ZoneCheckRefusal::Fault("divergence-witness"))?;
                let time_lock = matches!(self.judged, Judged::TimeLockFreedom);
                let proof = self.certificate(&roots, root, &witness, time_lock)?;
                Ok(Searched::Holds {
                    proof: Box::new(proof),
                    stats: self.stats(),
                })
            }
        }
    }

    /// Store a source/task product state with its scheduler control in the
    /// discrete key. Ordinary model searches pass `None` for the control.
    pub(super) fn insert_with_task(
        &mut self,
        state: ModelSimState,
        task_state: Option<TaskSchedulerState>,
        q: usize,
        zone: Dbm,
        initial: Option<usize>,
        arrival: Option<Arrival>,
    ) -> Result<Option<usize>, Halt> {
        Ok(
            match self.insert_task_at(state, task_state, q, zone, initial, arrival)? {
                Inserted::New(node) => Some(node),
                Inserted::Covered(_) => None,
            },
        )
    }

    /// Store a source state, naming the covering node when it is covered.
    pub(super) fn insert_at(
        &mut self,
        state: ModelSimState,
        q: usize,
        zone: Dbm,
        initial: Option<usize>,
        arrival: Option<Arrival>,
    ) -> Result<Inserted, Halt> {
        self.insert_task_at(state, None, q, zone, initial, arrival)
    }

    /// [`Self::insert_at`] with FR-249 scheduler control.
    pub(super) fn insert_task_at(
        &mut self,
        state: ModelSimState,
        task_state: Option<TaskSchedulerState>,
        q: usize,
        zone: Dbm,
        initial: Option<usize>,
        arrival: Option<Arrival>,
    ) -> Result<Inserted, Halt> {
        let (state, mut zone, permutation) = self.canonical(state, zone)?;
        // A place that is not live at the state (an absent or dropped
        // `Option<Clock>`, a union case that does not hold) has no valuation
        // (FR-230 "Presence bit"): its dimension is free, so nodes differing
        // only in a dead clock are one.
        for (index, place) in self.timed.places().iter().enumerate() {
            if !self.timed.live(place, &state) {
                zone.free(index + 1)?;
            }
        }
        let (mut key, _) = self
            .timed
            .system()
            .state_key(&state)
            .map_err(|_| ZoneCheckRefusal::Fault("state-key"))?;
        key.extend_from_slice(&widen(q).to_be_bytes());
        match &task_state {
            Some(task_state) => {
                key.push(1);
                let task_key = task_state
                    .canonical_control_key()
                    .map_err(|_| ZoneCheckRefusal::Fault("task-state-key"))?;
                key.extend_from_slice(&widen(task_key.len()).to_be_bytes());
                key.extend_from_slice(&task_key);
            }
            None => key.push(0),
        }
        let task_template = arrival
            .as_ref()
            .and_then(|arrival| {
                arrival
                    .task_transition
                    .as_ref()
                    .map(|identity| (arrival, identity))
            })
            .map(|(arrival, identity)| {
                let scheduler = if identity.action == TaskAction::Activate {
                    task_state.as_ref()
                } else {
                    self.nodes
                        .get(arrival.from)
                        .and_then(|node| node.task_state.as_ref())
                }
                .ok_or(ZoneCheckRefusal::Fault("task-template-state"))?;
                let job_rank = scheduler
                    .job_rank(identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-template-rank"))?;
                Ok::<_, ZoneCheckRefusal>(task_transition_template(identity, job_rank))
            })
            .transpose()?;
        if let Some(stored) = self.passed.get(&key) {
            for &candidate in stored {
                if zone.alu_covered_by(&self.nodes[candidate].zone, &self.lower, &self.upper)? {
                    if let Some(arrival) = arrival {
                        self.retain(SymbolicEdge {
                            from: arrival.from,
                            to: candidate,
                            covering: true,
                            step: arrival.step,
                            piece: arrival.piece,
                            task_transition: arrival.task_transition.clone(),
                            task_template: task_template.clone(),
                            task_subtractions: arrival.task_subtractions.clone(),
                        })?;
                        self.record_task_miss(
                            self.edges.len() - 1,
                            arrival.task_transition.as_ref(),
                            &zone,
                        );
                    }
                    return Ok(Inserted::Covered(candidate));
                }
            }
        }
        self.charge_state(&zone)?;
        let index = self.nodes.len();
        self.nodes.push(SymbolicNode {
            state,
            task_state,
            zone,
            initial,
            locked: false,
            q,
            permutation,
        });
        self.keys.push(key.clone());
        self.expanded.push(false);
        self.out.push(Vec::new());
        self.passed.entry(key).or_default().push(index);
        let parent = match arrival {
            Some(arrival) => {
                self.retain(SymbolicEdge {
                    from: arrival.from,
                    to: index,
                    covering: false,
                    step: arrival.step,
                    piece: arrival.piece,
                    task_transition: arrival.task_transition.clone(),
                    task_template: task_template.clone(),
                    task_subtractions: arrival.task_subtractions.clone(),
                })?;
                let zone = self.nodes[index].zone.clone();
                self.record_task_miss(
                    self.edges.len() - 1,
                    arrival.task_transition.as_ref(),
                    &zone,
                );
                Some(self.edges.len() - 1)
            }
            None => None,
        };
        self.parent.push(parent);
        Ok(Inserted::New(index))
    }

    /// Keep every exact deadline-miss edge, including one whose target zone
    /// is covered by a previously stored product node.
    fn record_task_miss(
        &mut self,
        edge: usize,
        transition: Option<&TaskTransitionIdentity>,
        zone: &Dbm,
    ) {
        let Some(identity) =
            transition.filter(|identity| identity.action == TaskAction::DeadlineMiss)
        else {
            return;
        };
        if !self
            .task_miss_nodes
            .iter()
            .any(|(prior_edge, _, _)| *prior_edge == edge)
        {
            self.task_miss_nodes
                .push((edge, identity.clone(), zone.clone()));
        }
    }

    /// `(state, zone)` canonicalised under the run's group, with the
    /// permutation applied ([`canonical_symbolic_state`]).
    fn canonical(
        &self,
        state: ModelSimState,
        zone: Dbm,
    ) -> Result<(ModelSimState, Dbm, Permutation), Halt> {
        if self.group.is_trivial() {
            return Ok((state, zone, Permutation::identity()));
        }
        Ok(canonical_symbolic_state(
            self.timed,
            &self.group,
            &state,
            &zone,
        )?)
    }

    /// `zone` with each object's clocks moved to those of the object
    /// `permutation` maps it to (QSpec FR-430).
    pub(super) fn renamed(&self, zone: &Dbm, permutation: &Permutation) -> Result<Dbm, Halt> {
        Ok(renamed_zone(self.timed, zone, permutation)?)
    }

    /// Charge one stored symbolic state with `zone` against
    /// `max_symbolic_states` and `max_zone_bytes`, counting the search's
    /// states and every divergence check's together.
    pub(super) fn charge_state(&mut self, zone: &Dbm) -> Result<(), Halt> {
        self.charge_vertex(zone_bytes(zone))
    }

    /// Charge one stored symbolic state of `bytes` zone bytes against
    /// `max_symbolic_states` and `max_zone_bytes`.
    pub(super) fn charge_vertex(&mut self, bytes: u64) -> Result<(), Halt> {
        let states = widen(self.nodes.len()).saturating_add(self.escape_states);
        if states >= self.limits.max_symbolic_states {
            return Err(
                self.stop(ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(
                    self.limits.max_symbolic_states,
                ))),
            );
        }
        self.charge_bytes(bytes)
    }

    /// Charge `bytes` of stored zones against `max_zone_bytes`.
    pub(super) fn charge_bytes(&mut self, bytes: u64) -> Result<(), Halt> {
        if self.zone_bytes.saturating_add(bytes) > self.limits.max_zone_bytes {
            return Err(
                self.stop(ZoneStop::LimitReached(ZoneCheckLimit::MaxZoneBytes(
                    self.limits.max_zone_bytes,
                ))),
            );
        }
        self.zone_bytes += bytes;
        Ok(())
    }

    /// Charge one symbolic transition against `max_symbolic_transitions`,
    /// the search's and every divergence check's together.
    pub(super) fn charge_edge(&mut self) -> Result<(), Halt> {
        let edges = widen(self.edges.len()).saturating_add(self.escape_edges);
        if edges >= self.limits.max_symbolic_transitions {
            return Err(self.stop(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxSymbolicTransitions(self.limits.max_symbolic_transitions),
            )));
        }
        Ok(())
    }

    /// The symbolic transitions `max_symbolic_transitions` leaves, which
    /// FR-241's loop elimination may keep as constraints at once.
    pub(super) fn transitions_left(&self) -> u64 {
        let used = widen(self.edges.len()).saturating_add(self.escape_edges);
        self.limits.max_symbolic_transitions.saturating_sub(used)
    }

    /// Count a state a divergence check stored, once charged.
    pub(super) fn escape_state(&mut self) {
        self.escape_states += 1;
    }

    /// Count a transition a divergence check stored, once charged.
    pub(super) fn escape_edge(&mut self) {
        self.escape_edges += 1;
    }

    fn retain(&mut self, edge: SymbolicEdge) -> Result<(), Halt> {
        self.charge_edge()?;
        let (from, _) = edge.ends();
        if let Some(out) = self.out.get_mut(from) {
            out.push(self.edges.len());
        }
        self.edges.push(edge);
        Ok(())
    }

    /// The guarded steps of `node`'s model state, computed once.
    pub(super) fn steps_of(&mut self, node: usize) -> Result<Vec<GuardedStep>, Halt> {
        let state = self.nodes[node].state.clone();
        self.steps_for(&state)
    }

    /// The guarded steps of `state`, computed once per model state.
    pub(super) fn steps_for(&mut self, state: &ModelSimState) -> Result<Vec<GuardedStep>, Halt> {
        let key = self.key(state)?;
        if let Some(steps) = self.steps.get(&key) {
            return Ok(steps.clone());
        }
        let steps = self.timed(self.timed.guarded_steps(state))?;
        self.steps.insert(key, steps.clone());
        Ok(steps)
    }

    /// Add the checked releases enabled by one source transition to its
    /// separate task control. The source transition retains its full
    /// operation/receiver/argument identity; each release becomes a following
    /// typed zero-time activation edge in declaration order.
    fn queue_task_releases(
        &mut self,
        scheduler: &mut TaskSchedulerState,
        state: &ModelSimState,
        step_index: usize,
    ) -> Result<(), Halt> {
        let Some(subject) = &self.task_subject else {
            return Ok(());
        };
        let releases = subject.releases().to_vec();
        let steps = self.steps_for(state)?;
        let step = steps
            .get(step_index)
            .ok_or(ZoneCheckRefusal::Fault("task-release-step"))?;
        let transition = step.transition().clone();
        let system = self.timed.system();
        let releases: Vec<_> = releases
            .iter()
            .filter(|release| {
                system.operation_text(release.operation().declaring(), release.operation().name())
                    == Some(transition.operation())
            })
            .cloned()
            .collect();
        scheduler.enqueue_releases(releases, transition);
        Ok(())
    }

    /// Construct the next pending FR-419 activation as an independent
    /// zero-time product successor.
    fn task_activation_successor(
        &self,
        scheduler: &TaskSchedulerState,
        zone: &Dbm,
    ) -> Result<Option<TaskActivationSuccessor>, Halt> {
        let Some(subject) = &self.task_subject else {
            return Ok(None);
        };
        let TaskProductRoute::Exact(product) = subject.route() else {
            return Ok(None);
        };
        let Some(pending) = scheduler.pending_releases().first() else {
            return Ok(None);
        };
        let layout = self
            .task_clock_layout
            .as_ref()
            .ok_or(ZoneCheckRefusal::Fault("task-clock-layout"))?;
        let arrival = layout
            .arrival_index(pending.release.task_set(), pending.release.task())
            .ok_or(ZoneCheckRefusal::Fault("task-arrival-clock"))?;
        let mut next_zone = zone.clone();
        if !scheduler
            .apply_arrival_guard(
                product,
                pending.release.task(),
                arrival,
                &self.scale,
                &mut next_zone,
            )
            .map_err(|_| ZoneCheckRefusal::Fault("task-arrival-guard"))?
        {
            return Ok(None);
        }
        let mut next_scheduler = scheduler.clone();
        let identity = next_scheduler
            .activate_release(
                product,
                &pending.release,
                self.timed.system(),
                pending.source.clone(),
                Rational::from_integer(quire_exact::Integer::zero()),
                Rational::from_integer(quire_exact::Integer::zero()),
            )
            .map_err(|_| ZoneCheckRefusal::Fault("task-activation"))?;
        let service = layout
            .service_clock(&identity)
            .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
        let deadline = layout
            .deadline_index(&identity)
            .ok_or(ZoneCheckRefusal::Fault("task-deadline-clock"))?;
        let arrival = layout
            .arrival_index(pending.release.task_set(), pending.release.task())
            .ok_or(ZoneCheckRefusal::Fault("task-arrival-clock"))?;
        let mut resets = vec![
            (arrival, BigInt::from(0)),
            (service.index, BigInt::from(0)),
            (deadline, BigInt::from(0)),
        ];
        if let Some(jitter) = layout.jitter_index(&identity) {
            resets.push((jitter, BigInt::from(0)));
        }
        next_zone.assign_clocks(&[], &resets)?;
        next_scheduler
            .remove_pending_release(0)
            .ok_or(ZoneCheckRefusal::Fault("task-pending-release"))?;
        Ok(Some(TaskActivationSuccessor {
            scheduler: next_scheduler,
            zone: next_zone,
            transition: identity,
            subtractions: Vec::new(),
        }))
    }

    fn task_dispatch_successors(
        &self,
        scheduler: &TaskSchedulerState,
        zone: &Dbm,
        product: &super::task_product::FixedWorkTaskProduct,
        layout: &TaskClockLayout,
    ) -> Result<Vec<TaskActivationSuccessor>, Halt> {
        if product.policy != qsl_semantics::check::SchedulingPolicy::Edf {
            let mut next = scheduler.clone();
            return match next.dispatch(product) {
                Ok(transition) => {
                    let mut next_zone = zone.clone();
                    if transition.action == TaskAction::Dispatch {
                        let service = layout
                            .service_clock(&transition)
                            .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                        next_zone.reset(service.index, &BigInt::from(0))?;
                    }
                    Ok(vec![TaskActivationSuccessor {
                        scheduler: next,
                        zone: next_zone,
                        transition,
                        subtractions: Vec::new(),
                    }])
                }
                Err(TaskSchedulerError::ActionNotEnabled) => Ok(Vec::new()),
                Err(_) => Err(ZoneCheckRefusal::Fault("task-dispatch").into()),
            };
        }
        let candidates: Vec<_> = scheduler
            .jobs()
            .iter()
            .filter(|job| matches!(job.phase, TaskJobPhase::Ready | TaskJobPhase::Suspended))
            .collect();
        let mut successors = Vec::new();
        for candidate in &candidates {
            let mut selected_zone = zone.clone();
            let mut enabled = true;
            for other in &candidates {
                if candidate.identity == other.identity {
                    continue;
                }
                if !self.constrain_edf_precedence(
                    product,
                    layout,
                    &mut selected_zone,
                    &candidate.identity,
                    &other.identity,
                )? {
                    enabled = false;
                    break;
                }
            }
            if !enabled {
                continue;
            }
            let mut next = scheduler.clone();
            let transition = next
                .dispatch_selected(&candidate.identity)
                .map_err(|_| ZoneCheckRefusal::Fault("task-dispatch"))?;
            if transition.action == TaskAction::Dispatch {
                let service = layout
                    .service_clock(&transition)
                    .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                selected_zone.reset(service.index, &BigInt::from(0))?;
            }
            successors.push(TaskActivationSuccessor {
                scheduler: next,
                zone: selected_zone,
                transition,
                subtractions: Vec::new(),
            });
        }
        Ok(successors)
    }

    fn task_preemption_successors(
        &self,
        scheduler: &TaskSchedulerState,
        zone: &Dbm,
        product: &super::task_product::FixedWorkTaskProduct,
        layout: &TaskClockLayout,
    ) -> Result<Vec<TaskActivationSuccessor>, Halt> {
        let Some(running) = scheduler.running() else {
            return Ok(Vec::new());
        };
        if product.policy != qsl_semantics::check::SchedulingPolicy::Edf {
            let mut next = scheduler.clone();
            return match next.preempt(product) {
                Ok(transition) => Ok(vec![TaskActivationSuccessor {
                    scheduler: next,
                    zone: zone.clone(),
                    transition,
                    subtractions: Vec::new(),
                }]),
                Err(TaskSchedulerError::ActionNotEnabled) => Ok(Vec::new()),
                Err(_) => Err(ZoneCheckRefusal::Fault("task-preempt").into()),
            };
        }
        let mut successors = Vec::new();
        for candidate in scheduler
            .jobs()
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Ready)
        {
            let mut selected_zone = zone.clone();
            if !self.constrain_edf_precedence(
                product,
                layout,
                &mut selected_zone,
                &candidate.identity,
                running,
            )? {
                continue;
            }
            let mut next = scheduler.clone();
            let transition = next
                .preempt_for(&candidate.identity)
                .map_err(|_| ZoneCheckRefusal::Fault("task-preempt"))?;
            successors.push(TaskActivationSuccessor {
                scheduler: next,
                zone: selected_zone,
                transition,
                subtractions: Vec::new(),
            });
        }
        Ok(successors)
    }

    fn constrain_edf_precedence(
        &self,
        product: &super::task_product::FixedWorkTaskProduct,
        layout: &TaskClockLayout,
        zone: &mut Dbm,
        left: &TaskTransitionIdentity,
        right: &TaskTransitionIdentity,
    ) -> Result<bool, Halt> {
        let left_task = product
            .tasks
            .iter()
            .find(|task| task.task_set == left.taskset && task.task == left.task)
            .ok_or(ZoneCheckRefusal::Fault("task-edf-task"))?;
        let right_task = product
            .tasks
            .iter()
            .find(|task| task.task_set == right.taskset && task.task == right.task)
            .ok_or(ZoneCheckRefusal::Fault("task-edf-task"))?;
        let left_clock = layout
            .deadline_index(left)
            .ok_or(ZoneCheckRefusal::Fault("task-deadline-clock"))?;
        let right_clock = layout
            .deadline_index(right)
            .ok_or(ZoneCheckRefusal::Fault("task-deadline-clock"))?;
        let delta =
            self.scaled(&right_task.relative_deadline) - self.scaled(&left_task.relative_deadline);
        let bound = if edf_tie_order(left, right) == std::cmp::Ordering::Less {
            Bound::le(delta)
        } else {
            Bound::lt(delta)
        };
        zone.constrain(right_clock, left_clock, bound)?;
        Ok(!zone.is_empty())
    }

    /// The exact scheduler's enabled zero-time actions at this product node.
    fn task_scheduler_successors(
        &self,
        scheduler: &TaskSchedulerState,
        zone: &Dbm,
    ) -> Result<Vec<TaskActivationSuccessor>, Halt> {
        // Materialize every task bound to the just-taken source activation
        // before dispatch, preemption, completion, or another source action.
        if !scheduler.pending_releases().is_empty() {
            return Ok(Vec::new());
        }
        let Some(subject) = &self.task_subject else {
            return Ok(Vec::new());
        };
        let TaskProductRoute::Exact(product) = subject.route() else {
            return Ok(Vec::new());
        };
        let layout = self
            .task_clock_layout
            .as_ref()
            .ok_or(ZoneCheckRefusal::Fault("task-clock-layout"))?;
        let mut successors = Vec::new();

        successors.extend(self.task_dispatch_successors(scheduler, zone, product, layout)?);
        successors.extend(self.task_preemption_successors(scheduler, zone, product, layout)?);

        let service_clocks: Vec<_> = scheduler
            .jobs()
            .iter()
            .map(|job| {
                layout
                    .service_clock(&job.identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))
            })
            .collect::<Result<_, _>>()?;
        let mut completed = scheduler.clone();
        let mut completion_zone = zone.clone();
        match completed.complete_with_subtractions(
            product,
            &mut completion_zone,
            &self.scale,
            &service_clocks,
            None,
        ) {
            Ok((transition, subtractions)) => successors.push(TaskActivationSuccessor {
                scheduler: completed,
                zone: completion_zone,
                transition,
                subtractions,
            }),
            Err(TaskCompletionError::Scheduler(TaskSchedulerError::ActionNotEnabled))
            | Err(TaskCompletionError::EmptySuccessor) => {}
            Err(_) => return Err(ZoneCheckRefusal::Fault("task-completion").into()),
        }

        for job in scheduler.jobs() {
            if job.phase == TaskJobPhase::Missed {
                continue;
            }
            let task = product
                .tasks
                .iter()
                .find(|task| {
                    task.task_set == job.identity.taskset && task.task == job.identity.task
                })
                .ok_or(ZoneCheckRefusal::Fault("task-job-binding"))?;
            let deadline = layout
                .deadline_index(&job.identity)
                .ok_or(ZoneCheckRefusal::Fault("task-deadline-clock"))?;
            let scaled_deadline = self.scaled(&task.relative_deadline);
            let mut miss_zone = zone.clone();
            miss_zone.constrain(deadline, 0, Bound::le(scaled_deadline.clone()))?;
            miss_zone.constrain(0, deadline, Bound::le(-scaled_deadline))?;
            if miss_zone.is_empty() {
                continue;
            }
            let work = self.scaled(&task.work);
            if job.phase == TaskJobPhase::Running {
                let service = layout
                    .service_clock(&job.identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                miss_zone.constrain(service.index, 0, Bound::lt(work))?;
            } else if job.phase == TaskJobPhase::Suspended {
                if let Some(running) = &scheduler.running {
                    let running_clock = layout
                        .service_clock(running)
                        .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                    let suspended_clock = layout
                        .service_clock(&job.identity)
                        .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                    miss_zone.constrain(
                        suspended_clock.index,
                        running_clock.index,
                        Bound::lt(work),
                    )?;
                } else {
                    let service = layout
                        .service_clock(&job.identity)
                        .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                    miss_zone.constrain(service.index, 0, Bound::lt(work))?;
                }
            } else if work.is_zero() {
                continue;
            }
            let mut missed = scheduler.clone();
            let transition = missed
                .deadline_miss(&job.identity)
                .map_err(|_| ZoneCheckRefusal::Fault("task-deadline-miss"))?;
            let service = layout
                .service_clock(&job.identity)
                .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
            miss_zone.free(service.index)?;
            miss_zone.free(deadline)?;
            successors.push(TaskActivationSuccessor {
                scheduler: missed,
                zone: miss_zone,
                transition,
                subtractions: Vec::new(),
            });
        }
        Ok(successors)
    }

    /// Whether an enabled source-authorized scheduler edge must occur before
    /// this product state may let time elapse.
    fn task_action_urgent(&self, scheduler: &TaskSchedulerState) -> Result<bool, Halt> {
        let Some(subject) = &self.task_subject else {
            return Ok(false);
        };
        let TaskProductRoute::Exact(product) = subject.route() else {
            return Ok(false);
        };
        let mut dispatch = scheduler.clone();
        if dispatch.dispatch(product).is_ok() {
            return Ok(true);
        }
        let mut preempt = scheduler.clone();
        Ok(preempt.preempt(product).is_ok())
    }

    /// `state`'s state-key bytes.
    pub(super) fn key(&self, state: &ModelSimState) -> Result<Vec<u8>, Halt> {
        self.timed
            .system()
            .state_key(state)
            .map(|(key, _)| key)
            .map_err(|_| ZoneCheckRefusal::Fault("state-key").into())
    }

    /// Expand `node` once: the nodes it creates, in canonical order.
    fn expand(&mut self, node: usize) -> Result<Vec<usize>, Halt> {
        if self.expanded[node] {
            return Ok(Vec::new());
        }
        if matches!(
            self.task_subject.as_ref().map(TaskProductSubject::route),
            Some(TaskProductRoute::Bounded(_))
        ) {
            if self.task_product_work >= self.limits.max_task_product_work {
                return Err(
                    self.stop(ZoneStop::LimitReached(ZoneCheckLimit::TaskProductWork(
                        self.limits.max_task_product_work,
                    ))),
                );
            }
            self.task_product_work += 1;
        }
        let state = self.nodes[node].state.clone();
        let task_state = self.nodes[node].task_state.clone();
        let zone = self.nodes[node].zone.clone();
        let q = self.nodes[node].q;
        let scale = self.scale.clone();
        let mut created = Vec::new();
        // With no automaton, and once a TT-2 automaton state has decided
        // every obligation, past the formula's horizon (FR-242, ADR-026
        // EZ-9), a step reads no position: the automaton state stays, no
        // atom is read and no piece of the horizon axis splits the zone,
        // whose horizon clock no longer matters. The zone graph goes on, so
        // the certificate keeps its edges and its `admitted` path.
        let frozen = self.window.as_ref().is_none_or(|window| window.decided(q));
        if frozen {
            let horizon = self.window.as_ref().map(|_| self.timed.places().len() + 1);
            let successors = if let Some(scheduler) = &task_state {
                self.product_successors(&state, scheduler, &zone, &scale)?
            } else {
                self.successors(&state, &zone, &scale)?
            };
            for mut successor in successors {
                if let Some(h) = horizon {
                    successor.zone.free(h)?;
                }
                let arrival = Arrival {
                    from: node,
                    step: Some((successor.step, successor.part)),
                    piece: None,
                    task_transition: successor.task_transition.clone(),
                    task_subtractions: Vec::new(),
                };
                let next_task_state = successor.task_state.clone().or_else(|| task_state.clone());
                if let Some(new) = self.insert_with_task(
                    successor.state,
                    next_task_state,
                    q,
                    successor.zone,
                    None,
                    Some(arrival),
                )? {
                    created.push(new);
                }
            }
            if let Some(scheduler) = task_state.as_ref() {
                if let Some(mut successor) = self.task_activation_successor(scheduler, &zone)? {
                    self.settle_product_at(
                        &state,
                        &successor.scheduler,
                        &mut successor.zone,
                        &scale,
                    )?;
                    let arrival = Arrival {
                        from: node,
                        step: None,
                        piece: None,
                        task_transition: Some(successor.transition),
                        task_subtractions: successor.subtractions,
                    };
                    if let Some(new) = self.insert_with_task(
                        state.clone(),
                        Some(successor.scheduler),
                        q,
                        successor.zone,
                        None,
                        Some(arrival),
                    )? {
                        created.push(new);
                    }
                }
                for mut successor in self.task_scheduler_successors(scheduler, &zone)? {
                    self.settle_product_at(
                        &state,
                        &successor.scheduler,
                        &mut successor.zone,
                        &scale,
                    )?;
                    let arrival = Arrival {
                        from: node,
                        step: None,
                        piece: None,
                        task_transition: Some(successor.transition),
                        task_subtractions: successor.subtractions,
                    };
                    if let Some(new) = self.insert_with_task(
                        state.clone(),
                        Some(successor.scheduler),
                        q,
                        successor.zone,
                        None,
                        Some(arrival),
                    )? {
                        created.push(new);
                    }
                }
            }
            self.expanded[node] = true;
            return Ok(created);
        }
        let Some(window) = self.window.as_ref() else {
            return Err(ZoneCheckRefusal::Fault("window").into());
        };
        let tail = window.state(q).is_some_and(|state| state.tail);
        let period = self.timed.tick_period().cloned();
        let pieces = self.pieces.clone();
        // A step's position, read on each piece of the horizon axis, or
        // under a tick source after its tick; none in the idle tail, which
        // takes no further step.
        let positions = if tail {
            Vec::new()
        } else {
            self.positions(&state, &zone, &scale)?
        };
        let terminal = positions.is_empty();
        for successor in positions {
            for (piece, (constraints, stamp)) in pieces.iter().enumerate() {
                let mut at = successor.zone.clone();
                self.constrain(&mut at, constraints)?;
                if at.is_empty() {
                    continue;
                }
                let advance = match (&period, successor.tick) {
                    (Some(period), true) => period.clone(),
                    _ => zero_stamp(),
                };
                let stamp = match period {
                    Some(_) => Stamp::After(&advance),
                    None => Stamp::At(stamp),
                };
                let read = self.window_step(q, &successor.state, stamp, false)?;
                self.settle(&successor.state, &mut at)?;
                if at.is_empty() {
                    continue;
                }
                let arrival = Arrival {
                    from: node,
                    step: Some((successor.step, successor.part)),
                    piece: period.is_none().then_some(piece),
                    task_transition: successor.task_transition.clone(),
                    task_subtractions: Vec::new(),
                };
                let mut next_task_state =
                    successor.task_state.clone().or_else(|| task_state.clone());
                if let (Some(scheduler), Some((step, _))) = (
                    next_task_state.as_mut(),
                    Some((successor.step, successor.part)),
                ) {
                    self.queue_task_releases(scheduler, &state, step)?;
                }
                if let Some(new) = self.insert_with_task(
                    successor.state.clone(),
                    next_task_state,
                    read,
                    at,
                    None,
                    Some(arrival),
                )? {
                    created.push(new);
                }
            }
        }
        // The behaviour may idle: under a dense source at a quiescent state,
        // a stutter position after a delay; under a tick source at a
        // terminal state, whose stutter step is a tick (FR-231). Either
        // leaves it in the idle tail.
        let idles = match &period {
            Some(_) => tail || terminal,
            None => self.timed(self.timed.is_quiescent(&state))? && self.elapses(&state)?,
        };
        if idles {
            for (piece, (constraints, stamp)) in pieces.iter().enumerate() {
                let mut at = zone.clone();
                self.constrain(&mut at, constraints)?;
                if at.is_empty() {
                    continue;
                }
                let stamp = match &period {
                    Some(period) => Stamp::After(period),
                    None => Stamp::At(stamp),
                };
                let read = self.window_step(q, &state, stamp, true)?;
                self.settle(&state, &mut at)?;
                let arrival = Arrival {
                    from: node,
                    step: None,
                    piece: period.is_none().then_some(piece),
                    task_transition: None,
                    task_subtractions: Vec::new(),
                };
                if let Some(new) = self.insert_with_task(
                    state.clone(),
                    task_state.clone(),
                    read,
                    at,
                    None,
                    Some(arrival),
                )? {
                    created.push(new);
                }
            }
        }
        if let Some(scheduler) = task_state.as_ref() {
            if let Some(mut successor) = self.task_activation_successor(scheduler, &zone)? {
                self.settle_product_at(&state, &successor.scheduler, &mut successor.zone, &scale)?;
                let arrival = Arrival {
                    from: node,
                    step: None,
                    piece: None,
                    task_transition: Some(successor.transition),
                    task_subtractions: successor.subtractions,
                };
                if let Some(new) = self.insert_with_task(
                    state.clone(),
                    Some(successor.scheduler),
                    q,
                    successor.zone,
                    None,
                    Some(arrival),
                )? {
                    created.push(new);
                }
            }
            for mut successor in self.task_scheduler_successors(scheduler, &zone)? {
                self.settle_product_at(&state, &successor.scheduler, &mut successor.zone, &scale)?;
                let arrival = Arrival {
                    from: node,
                    step: None,
                    piece: None,
                    task_transition: Some(successor.transition),
                    task_subtractions: successor.subtractions,
                };
                if let Some(new) = self.insert_with_task(
                    state.clone(),
                    Some(successor.scheduler),
                    q,
                    successor.zone,
                    None,
                    Some(arrival),
                )? {
                    created.push(new);
                }
            }
        }
        self.expanded[node] = true;
        Ok(created)
    }

    /// The automaton state a position reads from `q`.
    fn window_step(
        &mut self,
        q: usize,
        state: &ModelSimState,
        stamp: Stamp<'_>,
        stutter: bool,
    ) -> Result<usize, Halt> {
        self.window_position(Some(q), state, stamp, stutter)
    }

    /// The automaton state position 0 (`from` = `None`) or a later position
    /// reads at `state` and time stamp `stamp`, each atom read through the
    /// claim's letters for the instance it names.
    fn window_position(
        &mut self,
        from: Option<usize>,
        state: &ModelSimState,
        stamp: Stamp<'_>,
        stutter: bool,
    ) -> Result<usize, Halt> {
        let Judged::Claim(claim) = &self.judged else {
            return Err(ZoneCheckRefusal::Fault("window-claim").into());
        };
        let window = self
            .window
            .as_mut()
            .ok_or(ZoneCheckRefusal::Fault("window"))?;
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
        let read = match from {
            None => window.initial(&mut atoms),
            Some(q) => window.step(q, stamp, stutter, &mut atoms),
        };
        self.read(read)
    }

    /// The automaton state of a [`Read`], or the halt it names.
    fn read(&self, read: Read) -> Result<usize, Halt> {
        match read {
            Read::State(q) => Ok(q),
            Read::Fault(WindowFault::UnknownState) => {
                Err(ZoneCheckRefusal::Fault("window-state").into())
            }
            Read::Fault(_) => Err(ZoneCheckRefusal::Fault("window-stamp").into()),
            Read::Atom(stop) => Err(ZoneCheckRefusal::Atom(stop).into()),
            Read::AutomatonStates => Err(self.stop(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxAutomatonStates(self.limits.max_automaton_states),
            ))),
        }
    }

    /// Judge a new node: end the run on an admitted violation, mark a locked
    /// one. `true` when the node is a deadlock reached only after a delay.
    fn judge(&mut self, node: usize) -> Result<bool, Halt> {
        let badness = match self.badness(node)? {
            Some(Ok(badness)) => badness,
            Some(Err(())) => return Ok(true),
            None => {
                self.note_insufficient(node)?;
                return Ok(false);
            }
        };
        let escape = self.node_escape(node)?;
        if !escape.diverges {
            // Every behaviour through it is Zeno: the violation holds
            // vacuously and the time-lock-freedom item reports it.
            self.nodes[node].locked = true;
            return Ok(false);
        }
        let counterexample = self.counterexample(node, badness, escape)?;
        Err(Halt::Outcome(Box::new(Searched::Violated {
            counterexample: Box::new(counterexample),
            stats: self.stats(),
        })))
    }

    /// Keep the first instance an admitted behaviour through `node` read
    /// short of data for: with no violation, the evaluator reports that
    /// behaviour `InsufficientData`, never `Holds`.
    fn note_insufficient(&mut self, node: usize) -> Result<(), Halt> {
        let Some(at) = self
            .window
            .as_ref()
            .and_then(|window| window.insufficient(self.nodes[node].q))
        else {
            return Ok(());
        };
        if !self.node_escape(node)?.diverges {
            // Every behaviour through it is Zeno: as a violation on
            // time-locked valuations, it is kept `locked`, which the
            // certificate checker confirms (FR-245).
            self.nodes[node].locked = true;
            return Ok(());
        }
        if self.insufficient.is_none() {
            let instance = self
                .claim()
                .and_then(|claim| claim.instances.get(at))
                .cloned()
                .ok_or(ZoneCheckRefusal::Fault("window-instance"))?;
            self.insufficient = Some(instance);
        }
        Ok(())
    }

    /// FR-240's test from `node`.
    pub(super) fn node_escape(&mut self, node: usize) -> Result<Escape, Halt> {
        let state = self.nodes[node].state.clone();
        let zone = self.nodes[node].zone.clone();
        let (scale, lower, upper) = (self.scale.clone(), self.lower.clone(), self.upper.clone());
        self.escape(&state, &zone, &scale, &lower, &upper)
    }

    /// Whether `node` violates the item: `Some(Err(()))` for a deadlock only
    /// after a delay.
    #[allow(
        clippy::type_complexity,
        reason = "no violation, a violation, or one only after a delay"
    )]
    fn badness(&mut self, node: usize) -> Result<Option<Result<Badness, ()>>, Halt> {
        if matches!(self.judged, Judged::TaskSchedulability) {
            return Ok(None);
        }
        let claim = match &self.judged {
            Judged::Claim(claim) => claim,
            Judged::DeadlockFreedom => return self.deadlock(node),
            Judged::TimeLockFreedom
            | Judged::Liveness(_)
            | Judged::Refinement
            | Judged::TaskSchedulability => return Ok(None),
        };
        if let Some(window) = &self.window {
            // A TT-2 clause is decided by its automaton state.
            let q = self.nodes[node].q;
            let instance = |at: usize| {
                claim
                    .instances
                    .get(at)
                    .cloned()
                    .ok_or(ZoneCheckRefusal::Fault("window-instance"))
            };
            if let Some(undefined) = window.undefined(q) {
                return Ok(Some(Ok(Badness::Undefined(
                    instance(undefined.instance)?,
                    undefined.cause.clone(),
                    UndefinedLocus::Atom(undefined.atom),
                ))));
            }
            return match window.violated(q) {
                Some(at) => Ok(Some(Ok(Badness::False(instance(at)?)))),
                None => Ok(None),
            };
        }
        let atom = claim.atom.ok_or(ZoneCheckRefusal::Fault("claim-atom"))?;
        let state = &self.nodes[node].state;
        for instance in &claim.instances {
            let mut meter = Meter::new(self.timed.system().meter_limits());
            let reading = claim
                .letters
                .atom(atom, instance, state, &mut meter)
                .map_err(|stop| ZoneCheckRefusal::Atom(format!("{stop:?}")))?;
            match reading {
                AtomReading::True => {}
                AtomReading::False => return Ok(Some(Ok(Badness::False(instance.clone())))),
                AtomReading::Undefined(cause) => {
                    return Ok(Some(Ok(Badness::Undefined(
                        instance.clone(),
                        cause,
                        UndefinedLocus::Atom(atom),
                    ))))
                }
            }
        }
        Ok(None)
    }

    /// FR-232 over a zone: the deadlocked valuations of a quiescent node.
    #[allow(
        clippy::type_complexity,
        reason = "no deadlock, a deadlock at entry, or one only after a delay"
    )]
    fn deadlock(&mut self, node: usize) -> Result<Option<Result<Badness, ()>>, Halt> {
        let state = self.nodes[node].state.clone();
        if !self.timed(self.timed.is_quiescent(&state))? {
            return Ok(None);
        }
        let dim = self.nodes[node].zone.dim();
        // The valuations from which some step is enabled after a delay.
        let mut enabling = self.enabled(node, dim)?;
        for zone in &mut enabling {
            zone.down();
        }
        if enabling.is_empty() {
            return Ok(match self.timed.system().terminal_class(&state, false) {
                Ok(TerminalClass::Deadlocked) => {
                    Some(Ok(Badness::Deadlock(self.nodes[node].zone.clone())))
                }
                Ok(_) => None,
                Err(stop) => {
                    return Err(self.undecided(ZoneUndecided::UndecidedSuccessor(
                        TimedStop::Expansion(stop),
                    )))
                }
            });
        }
        let zone = self.nodes[node].zone.clone();
        let uncovered = uncovered(&zone, &enabling)?;
        if uncovered.is_empty() {
            return Ok(None);
        }
        // The node's entry valuations: those its real edge, or the initial
        // zero valuation, gives before time elapses.
        let entry = self.entry_zone(node)?;
        for part in &uncovered {
            let mut witness = part.clone();
            intersect(&mut witness, &entry)?;
            if !witness.is_empty() {
                return Ok(Some(Ok(Badness::Deadlock(witness))));
            }
        }
        Ok(Some(Err(())))
    }

    /// The valuations at which some step of `node`'s model state is
    /// enabled (FR-232 "Fairness over time"): a convex part of its guard
    /// holds and its target satisfies the time invariants applying there,
    /// one zone of dimension `dim` per part.
    pub(super) fn enabled(&mut self, node: usize, dim: usize) -> Result<Vec<Dbm>, Halt> {
        let mut enabled = Vec::new();
        for step in self.steps_of(node)? {
            for part in self.enabling_parts(&step)? {
                let mut zone = Dbm::universe(dim);
                self.constrain(&mut zone, &part)?;
                if !zone.is_empty() {
                    enabled.push(zone);
                }
            }
        }
        Ok(enabled)
    }

    /// FR-232's enabledness of `step` over clocks: each convex part of its
    /// guard with the target's time invariants on the clocks the step does
    /// not reset; none when a reset value breaks a target bound, a bound
    /// that is then constant over delays.
    pub(super) fn enabling_parts(
        &self,
        step: &GuardedStep,
    ) -> Result<Vec<Vec<PlaceConstraint>>, Halt> {
        let target = self.timed(self.timed.invariants(step.post()))?;
        let mut pulled_back = Vec::with_capacity(target.len());
        for bound in target {
            if let Some((_, value)) = step
                .resets()
                .iter()
                .find(|(place, _)| *place == bound.place())
            {
                if !bound.op().holds(value, bound.bound()) {
                    return Ok(Vec::new());
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
        Ok(step
            .parts()
            .iter()
            .map(|part| part.iter().chain(&pulled_back).cloned().collect())
            .collect())
    }

    /// The node whose real edge created `node`; `None` for an initial one.
    pub(super) fn parent_of(&self, node: usize) -> Option<usize> {
        self.parent[node]
            .and_then(|edge| self.edges.get(edge))
            .filter(|edge| !edge.covering)
            .map(|edge| edge.from)
    }

    /// The valuations `node` is entered at, before time elapses.
    fn entry_zone(&mut self, node: usize) -> Result<Dbm, Halt> {
        let dim = self.nodes[node].zone.dim();
        let state = self.nodes[node].state.clone();
        let invariants = self.timed(self.timed.invariants(&state))?;
        let mut zone = match self.parent[node].map(|edge| self.edges[edge].clone()) {
            None => Dbm::zero(dim),
            Some(SymbolicEdge {
                from,
                covering: false,
                step: Some((step, part)),
                ..
            }) => {
                let steps = self.steps_of(from)?;
                let step = steps
                    .get(step)
                    .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                let mut zone = self.nodes[from].zone.clone();
                self.constrain(
                    &mut zone,
                    step.parts()
                        .get(part)
                        .ok_or(ZoneCheckRefusal::Fault("edge-part"))?,
                )?;
                apply_step_clocks(&mut zone, step, &self.scale)?;
                zone
            }
            Some(_) => return Err(ZoneCheckRefusal::Fault("entry-parent").into()),
        };
        self.constrain(&mut zone, &invariants)?;
        Ok(zone)
    }

    /// The valuations, at the entry of the first source of `path`, from
    /// which its model steps are taken, each after an admissible delay: the
    /// exact predecessors of every valuation of the quiescent state it ends
    /// at, all of which diverge.
    pub(super) fn pre_path(&mut self, path: &[(ModelSimState, usize, usize)]) -> Result<Dbm, Halt> {
        let mut region = Dbm::universe(self.dim());
        for (source, step, part) in path.iter().rev() {
            let steps = self.steps_for(source)?;
            let step = steps
                .get(*step)
                .ok_or(ZoneCheckRefusal::Fault("path-step"))?;
            let part = step
                .parts()
                .get(*part)
                .ok_or(ZoneCheckRefusal::Fault("path-part"))?;
            let target = self.timed(self.timed.invariants(step.post()))?;
            self.constrain(&mut region, &target)?;
            region = preimage_step_clocks(&region, step, &self.scale)?;
            self.constrain(&mut region, part)?;
            let invariants = self.timed(self.timed.invariants(source))?;
            self.constrain(&mut region, &invariants)?;
            if self.elapses(source)? {
                region.down();
            }
        }
        Ok(region)
    }

    /// Whether the last valuation of `concrete`, at `state`, has a
    /// time-divergent continuation: FR-240's test from that one point, in
    /// units fine enough to hold it.
    pub(super) fn point_diverges(
        &mut self,
        state: &ModelSimState,
        concrete: &Concrete,
    ) -> Result<bool, Halt> {
        use num_integer::Integer as _;
        let factor = concrete
            .last_clocks
            .iter()
            .fold(self.scale.clone(), |factor, value| {
                factor.lcm(value.denominator().as_big())
            });
        let finer = &factor / &self.scale;
        let rescale = |bounds: &[Option<BigInt>]| -> Vec<Option<BigInt>> {
            bounds
                .iter()
                .map(|bound| bound.as_ref().map(|value| value * &finer))
                .collect()
        };
        let (lower, upper) = (rescale(&self.lower), rescale(&self.upper));
        let mut point = Dbm::universe(self.dim());
        for (place, value) in concrete.last_clocks.iter().enumerate() {
            let value = scaled(value, &factor);
            point.constrain(place + 1, 0, Bound::le(value.clone()))?;
            point.constrain(0, place + 1, Bound::le(-value))?;
        }
        Ok(self
            .escape(state, &point, &factor, &lower, &upper)?
            .diverges)
    }

    /// The stages of the real edges to `node` from its initial node, that
    /// node's initial-state index, and the number of idle-tail stutters
    /// among them.
    pub(super) fn stages_to(
        &mut self,
        node: usize,
    ) -> Result<(usize, Vec<super::concretise::Stage>, usize), Halt> {
        let lifted = self.lifted_to(node)?;
        Ok((lifted.initial, lifted.stages, lifted.stutters))
    }

    /// The real edges to `node` from its initial node, lifted to concrete
    /// states (QSpec FR-430, as FR-161 lifts EN-1's paths): `σ` starts as
    /// the inverse of the permutation that canonicalised the initial state,
    /// each canonical step `t` becomes the concrete step `σ(t)`, and `σ`
    /// becomes `σ ∘ π⁻¹` for the permutation `π` that canonicalised the
    /// step's target. Under the trivial group the path is already concrete.
    pub(super) fn lifted_to(&mut self, node: usize) -> Result<LiftedPath, Halt> {
        let mut chain = Vec::new();
        let mut at = node;
        while let Some(edge) = self.parent[at] {
            let edge = self.edges[edge].clone();
            if edge.covering {
                return Err(ZoneCheckRefusal::Fault("covering-parent").into());
            }
            chain.push((
                edge.from,
                at,
                edge.step,
                edge.piece,
                edge.task_transition,
                edge.task_subtractions,
            ));
            at = edge.from;
        }
        chain.reverse();
        let initial = self.nodes[at]
            .initial
            .ok_or(ZoneCheckRefusal::Fault("path-root"))?;
        let trivial = self.group.is_trivial();
        let mut sigma = self.nodes[at].permutation.inverse();
        let mut concrete = self.permuted(&self.nodes[at].state, &sigma)?;
        let pieces = self.window.as_ref().map(|_| self.pieces.clone());
        let mut stages = Vec::with_capacity(chain.len());
        let mut stutters = 0_usize;
        for (from, to, step, piece, task_transition, task_subtractions) in chain {
            let step = match step {
                Some((index, part)) => {
                    let steps = self.steps_of(from)?;
                    let canonical = steps
                        .get(index)
                        .cloned()
                        .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                    let step = if trivial {
                        canonical
                    } else {
                        let wanted = self
                            .timed
                            .system()
                            .permute_transition(canonical.transition(), &sigma)
                            .map_err(|_| ZoneCheckRefusal::Fault("lifted-transition"))?;
                        self.steps_for(&concrete)?
                            .into_iter()
                            .find(|step| *step.transition() == wanted)
                            .ok_or(ZoneCheckRefusal::Fault("lifted-step"))?
                    };
                    Some((step, part))
                }
                None => None,
            };
            let post = step
                .as_ref()
                .map_or_else(|| concrete.clone(), |(step, _)| step.post().clone());
            let task_product_step = if self.task_subject.is_some() {
                let transition = match (&step, task_transition) {
                    (Some((step, _)), _) => {
                        Some(TaskSearchTransition::Source(step.transition().clone()))
                    }
                    (None, Some(task)) => Some(TaskSearchTransition::Task(Box::new(task))),
                    (None, None) => None,
                };
                transition
                    .map(|transition| {
                        let target = self
                            .nodes
                            .get(to)
                            .ok_or(ZoneCheckRefusal::Fault("task-witness-target"))?;
                        let (_, source) = self
                            .timed
                            .system()
                            .state_key(&target.state)
                            .map_err(|_| ZoneCheckRefusal::Fault("task-witness-state-key"))?;
                        let scheduler = target
                            .task_state
                            .as_ref()
                            .ok_or(ZoneCheckRefusal::Fault("task-witness-scheduler"))?
                            .canonical_control_key()
                            .map_err(|_| ZoneCheckRefusal::Fault("task-witness-state-key"))?;
                        let state_key = TaskProductStateKey::new(source, scheduler)
                            .into_state_key()
                            .map_err(|_| ZoneCheckRefusal::Fault("task-witness-state-key"))?;
                        Ok::<_, ZoneCheckRefusal>((transition, state_key))
                    })
                    .transpose()?
            } else {
                None
            };
            let (task_resets, task_subtractions) = match task_product_step.as_ref() {
                Some((transition, _)) => {
                    self.task_step_clock_updates(transition, &task_subtractions)?
                }
                None => (Vec::new(), Vec::new()),
            };
            let target = self.timed(self.timed.invariants(&post))?;
            stutters += usize::from(step.is_none());
            let position = match (&pieces, piece) {
                (Some(pieces), Some(piece)) => pieces
                    .get(piece)
                    .map(|(constraints, _)| constraints.clone())
                    .ok_or(ZoneCheckRefusal::Fault("edge-piece"))?,
                _ => Vec::new(),
            };
            stages.push(super::concretise::Stage {
                invariants: self.timed(self.timed.invariants(&concrete))?,
                elapses: self.elapses(&concrete)?,
                target,
                step,
                position,
                position_resets: task_resets,
                before: None,
                piece: None,
                silent: false,
                fixed: None,
                task_product_step,
                task_subtractions,
            });
            if !trivial {
                sigma = sigma.compose(&self.nodes[to].permutation.inverse());
            }
            concrete = post;
        }
        Ok(LiftedPath {
            initial,
            stages,
            stutters,
            sigma,
            state: concrete,
        })
    }

    /// `state` under `permutation`; under the trivial group, unchanged.
    fn permuted(
        &self,
        state: &ModelSimState,
        permutation: &Permutation,
    ) -> Result<ModelSimState, Halt> {
        if self.group.is_trivial() {
            return Ok(state.clone());
        }
        Ok(self
            .timed
            .system()
            .permute(state, permutation)
            .map_err(|_| ZoneCheckRefusal::Fault("permute"))?)
    }

    /// FR-241: the counterexample through the real edges to `node`.
    fn counterexample(
        &mut self,
        node: usize,
        badness: Badness,
        escape: Escape,
    ) -> Result<TemporalCounterexample, Halt> {
        let LiftedPath {
            initial,
            stages,
            stutters,
            sigma,
            state,
        } = self.lifted_to(node)?;
        // The violation and its divergent targets are read on the stored,
        // canonical state; σ carries them to the concrete one.
        let badness = match badness {
            Badness::False(instance) => Badness::False(lift_instance(instance, &sigma)),
            Badness::Undefined(instance, cause, locus) => {
                Badness::Undefined(lift_instance(instance, &sigma), cause, locus)
            }
            Badness::Deadlock(zone) => Badness::Deadlock(self.renamed(&zone, &sigma)?),
        };
        self.finite_counterexample((initial, stages, stutters), state, badness, escape, &sigma)
    }

    /// Build a task-derived counterexample only when the completed mixed
    /// source/task graph has a positive-time cycle reachable after the miss.
    /// Source release edges and their typed activation successors are retained
    /// as ordinary product edges in the lasso.
    fn task_miss_counterexample(
        &mut self,
        miss_edge: usize,
        job: &TaskTransitionIdentity,
        zone: &Dbm,
    ) -> Result<Option<TemporalCounterexample<TaskSearchTransition>>, Halt> {
        let edge = self
            .edges
            .get(miss_edge)
            .cloned()
            .ok_or(ZoneCheckRefusal::Fault("task-miss-edge"))?;
        if edge.task_transition.as_ref() != Some(job) || job.action != TaskAction::DeadlineMiss {
            return Err(ZoneCheckRefusal::Fault("task-miss-witness-endpoint").into());
        }
        if !self.group.is_trivial() {
            // Concrete task identities are not lifted through symmetry yet.
            return Ok(None);
        }
        let LiftedPath {
            initial,
            mut stages,
            state: _,
            stutters: _,
            sigma: _,
        } = self.lifted_to(edge.from)?;
        stages.extend(self.task_product_stages(&[miss_edge])?);
        let Some(last_stage) = stages.last_mut() else {
            return Ok(None);
        };
        if !matches!(
            last_stage.task_product_step.as_ref().map(|(transition, _)| transition),
            Some(TaskSearchTransition::Task(identity))
                if identity.action == TaskAction::DeadlineMiss
                    && identity.taskset == job.taskset
                    && identity.task == job.task
                    && identity.slot == job.slot
                    && identity.generation == job.generation
                    && identity.release == job.release
        ) {
            return Err(ZoneCheckRefusal::Fault("task-miss-witness-endpoint").into());
        }
        // Keep the concrete stem inside the exact missed valuation region.
        last_stage.piece = Some(zone.clone());
        let places = self.dim() - 1;
        let ceiling = self.timed.max_constant();
        for (stem_edges, cycle_edges) in self.task_cycle_candidates(edge.to) {
            let mut complete_stem = stages.clone();
            complete_stem.extend(self.task_product_stages(&stem_edges)?);
            let cycle_stages = self.task_product_stages(&cycle_edges)?;
            let Some((_, task_prefix, entry)) =
                self.timed_lasso(places, &ceiling, complete_stem, &cycle_stages)?
            else {
                continue;
            };
            let Some((miss_index, TaskSearchTransition::Task(last))) =
                task_prefix.iter().enumerate().find_map(|(index, step)| {
                    matches!(&step.transition, TaskSearchTransition::Task(identity)
                        if identity.action == TaskAction::DeadlineMiss
                            && identity.taskset == job.taskset
                            && identity.task == job.task
                            && identity.slot == job.slot
                            && identity.generation == job.generation
                            && identity.release == job.release)
                    .then_some((index, &step.transition))
                })
            else {
                continue;
            };
            if last.taskset == job.taskset
                && last.task == job.task
                && last.slot == job.slot
                && last.generation == job.generation
                && last.release == job.release
                && entry > miss_index
            {
                return Ok(Some(TemporalCounterexample {
                    initial_state: initial,
                    prefix: task_prefix,
                    over_binding: None,
                    fairness: Vec::new(),
                    kind: CounterexampleKind::Formula,
                    lasso: Some(Lasso {
                        entry,
                        terminal_stutter: false,
                    }),
                    blocked: Vec::new(),
                    refinement: None,
                    final_delay: None,
                }));
            }
        }
        Ok(None)
    }

    /// Candidate positive-time cycles reachable from `start`, each paired
    /// with a shortest path from `start` to the cycle's entry.
    fn task_cycle_candidates(&self, start: usize) -> Vec<(Vec<usize>, Vec<usize>)> {
        let mut parent = vec![None; self.nodes.len()];
        let mut reached = vec![false; self.nodes.len()];
        let mut queue = VecDeque::from([start]);
        reached[start] = true;
        while let Some(node) = queue.pop_front() {
            for &edge in &self.out[node] {
                let target = self.edges[edge].to;
                if !reached[target] {
                    reached[target] = true;
                    parent[target] = Some((node, edge));
                    queue.push_back(target);
                }
            }
        }
        let path_to = |mut target: usize| {
            let mut path = Vec::new();
            while target != start {
                let (from, edge) = parent[target]?;
                path.push(edge);
                target = from;
            }
            path.reverse();
            Some(path)
        };
        let adjacency: Vec<Vec<usize>> = self
            .out
            .iter()
            .map(|outgoing| outgoing.iter().map(|&edge| self.edges[edge].to).collect())
            .collect();
        let mut component_of = vec![usize::MAX; self.nodes.len()];
        let mut components = Vec::new();
        for component in super::divergence::sccs(&adjacency) {
            let id = components.len();
            for &node in &component {
                component_of[node] = id;
            }
            components.push(component);
        }
        let mut candidates = Vec::new();
        for (component_id, component) in components.iter().enumerate() {
            let Some(&first_node) = component.iter().find(|&&node| reached[node]) else {
                continue;
            };
            let cyclic = component.len() > 1
                || self.out[first_node]
                    .iter()
                    .any(|&edge| self.edges[edge].to == first_node);
            if !cyclic {
                continue;
            }
            let members: Vec<bool> = (0..self.nodes.len())
                .map(|node| component_of[node] == component_id)
                .collect();
            let mut found_for_component = false;
            for &node in component {
                let Some(stem) = path_to(node) else {
                    continue;
                };
                for &first in &self.out[node] {
                    let target = self.edges[first].to;
                    if !members.get(target).copied().unwrap_or(false) {
                        continue;
                    }
                    let back = if target == node {
                        Some(Vec::new())
                    } else {
                        graph_path_within(&self.out, &self.edges, target, node, &members)
                    };
                    if let Some(back) = back {
                        let mut cycle = vec![first];
                        cycle.extend(back);
                        candidates.push((stem, cycle));
                        found_for_component = true;
                        // Keep one canonical candidate first; further internal
                        // edges provide independent alternatives only if its
                        // exact timed concretisation fails.
                        break;
                    }
                }
                if found_for_component {
                    break;
                }
            }
            if found_for_component {
                for &node in component {
                    let Some(stem) = path_to(node) else {
                        continue;
                    };
                    for &first in &self.out[node] {
                        let target = self.edges[first].to;
                        if target == node || !members.get(target).copied().unwrap_or(false) {
                            continue;
                        }
                        let Some(back) =
                            graph_path_within(&self.out, &self.edges, target, node, &members)
                        else {
                            continue;
                        };
                        let mut cycle = vec![first];
                        cycle.extend(back);
                        let candidate = (stem.clone(), cycle);
                        if !candidates.iter().any(|(_, prior)| *prior == candidate.1) {
                            candidates.push(candidate);
                        }
                    }
                }
            }
        }
        candidates
    }

    fn task_step_clock_updates(
        &self,
        transition: &TaskSearchTransition,
        subtractions: &[TaskSubtraction],
    ) -> Result<TaskClockUpdates, Halt> {
        let TaskSearchTransition::Task(identity) = transition else {
            return Ok((Vec::new(), Vec::new()));
        };
        let layout = self
            .task_clock_layout
            .as_ref()
            .ok_or(ZoneCheckRefusal::Fault("task-clock-layout"))?;
        let index = |clock: usize| {
            clock
                .checked_sub(1)
                .ok_or(ZoneCheckRefusal::Fault("task-clock-index"))
        };
        let mut resets = Vec::new();
        match identity.action {
            TaskAction::Activate => {
                let arrival = layout
                    .arrival_index(&identity.taskset, &identity.task)
                    .ok_or(ZoneCheckRefusal::Fault("task-arrival-clock"))?;
                let service = layout
                    .service_clock(identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                let deadline = layout
                    .deadline_index(identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-deadline-clock"))?;
                resets.extend([index(arrival)?, index(service.index)?, index(deadline)?]);
                if let Some(jitter) = layout.jitter_index(identity) {
                    resets.push(index(jitter)?);
                }
            }
            TaskAction::Dispatch | TaskAction::Complete => {
                let service = layout
                    .service_clock(identity)
                    .ok_or(ZoneCheckRefusal::Fault("task-service-clock"))?;
                resets.push(index(service.index)?);
            }
            TaskAction::Release
            | TaskAction::Preempt
            | TaskAction::Resume
            | TaskAction::BudgetChange
            | TaskAction::DeadlineMiss => {}
        }
        let base_dim = self.timed.places().len() + 1 + usize::from(self.window.is_some());
        let subtraction_rows = subtractions
            .iter()
            .map(|subtraction| {
                let clock = layout
                    .clock_index(base_dim, &subtraction.clock)
                    .ok_or(ZoneCheckRefusal::Fault("task-subtraction-clock"))?;
                let amount = Rational::new(
                    subtraction.constant.clone(),
                    Integer::from_big(self.scale.clone()),
                )
                .map_err(|_| ZoneCheckRefusal::Fault("task-subtraction-scale"))?;
                Ok((index(clock)?, amount))
            })
            .collect::<Result<Vec<_>, Halt>>()?;
        Ok((resets, subtraction_rows))
    }

    /// Convert retained mixed-product edges into timed stages without
    /// erasing task actions or source operation identities.
    fn task_product_stages(
        &mut self,
        edge_ids: &[usize],
    ) -> Result<Vec<super::concretise::Stage>, Halt> {
        let mut stages = Vec::with_capacity(edge_ids.len());
        for &edge_id in edge_ids {
            let edge = self
                .edges
                .get(edge_id)
                .cloned()
                .ok_or(ZoneCheckRefusal::Fault("task-lasso-edge"))?;
            let source = self
                .nodes
                .get(edge.from)
                .ok_or(ZoneCheckRefusal::Fault("task-lasso-source"))?
                .state
                .clone();
            let target = self
                .nodes
                .get(edge.to)
                .ok_or(ZoneCheckRefusal::Fault("task-lasso-target"))?
                .clone();
            let step = match edge.step {
                Some((index, part)) => Some((
                    self.steps_for(&source)?
                        .get(index)
                        .cloned()
                        .ok_or(ZoneCheckRefusal::Fault("task-lasso-step"))?,
                    part,
                )),
                None => None,
            };
            let post = step
                .as_ref()
                .map_or_else(|| source.clone(), |(step, _)| step.post().clone());
            let transition = match (step.as_ref(), edge.task_transition) {
                (Some((step, _)), _) => TaskSearchTransition::Source(step.transition().clone()),
                (None, Some(task)) => TaskSearchTransition::Task(Box::new(task)),
                (None, None) => return Err(ZoneCheckRefusal::Fault("task-lasso-transition").into()),
            };
            let (_, source_key) = self
                .timed
                .system()
                .state_key(&target.state)
                .map_err(|_| ZoneCheckRefusal::Fault("task-lasso-state-key"))?;
            let scheduler = target
                .task_state
                .as_ref()
                .ok_or(ZoneCheckRefusal::Fault("task-lasso-scheduler"))?
                .canonical_control_key()
                .map_err(|_| ZoneCheckRefusal::Fault("task-lasso-state-key"))?;
            let state_key = TaskProductStateKey::new(source_key, scheduler)
                .into_state_key()
                .map_err(|_| ZoneCheckRefusal::Fault("task-lasso-state-key"))?;
            let (position_resets, task_subtractions) =
                self.task_step_clock_updates(&transition, &edge.task_subtractions)?;
            stages.push(super::concretise::Stage {
                invariants: self.timed(self.timed.invariants(&source))?,
                elapses: self.elapses(&source)?,
                target: self.timed(self.timed.invariants(&post))?,
                step,
                position: Vec::new(),
                position_resets,
                before: None,
                piece: Some(target.zone),
                silent: false,
                fixed: None,
                task_product_step: Some((transition, state_key)),
                task_subtractions,
            });
        }
        Ok(stages)
    }

    /// FR-241: the counterexample along `path` (its initial state's index,
    /// its stages and its number of idle-tail stutters) to a violation at
    /// `state`, whose divergence test from the path's last zone is
    /// `escape`.
    pub(super) fn finite_counterexample(
        &mut self,
        (initial, stages, stutters): (usize, Vec<super::concretise::Stage>, usize),
        state: ModelSimState,
        badness: Badness,
        escape: Escape,
        sigma: &Permutation,
    ) -> Result<TemporalCounterexample, Halt> {
        // FR-241: the last valuation is one with a time-divergent
        // continuation, never a time-locked one. A deadlocked valuation of a
        // quiescent state diverges by its idle tail; otherwise the target is
        // the valuations from which the model steps to a quiescent state are
        // taken, and failing that the least-delay point, checked.
        // Each divergent target in turn: a deadlock's own valuations, or the
        // predecessors of each quiescent state the divergence test reached,
        // in its breadth-first order.
        let targets = match &badness {
            Badness::Deadlock(zone) => vec![zone.clone()],
            Badness::False(_) | Badness::Undefined(..) => {
                let mut targets = Vec::with_capacity(escape.to_quiescent.len());
                for path in &escape.to_quiescent {
                    // The escape ran from the canonical state.
                    let region = self.pre_path(path)?;
                    targets.push(self.renamed(&region, sigma)?);
                }
                targets
            }
        };
        let within = targets.into_iter().find_map(|zone| {
            concretise(
                self.timed,
                self.dim() - 1,
                &stages,
                &Target::Within(zone),
                &self.scale,
            )
            .ok()
        });
        let concrete = match within {
            Some(concrete) => concrete,
            None => {
                let concrete = concretise(
                    self.timed,
                    self.dim() - 1,
                    &stages,
                    &Target::Any,
                    &self.scale,
                )
                .map_err(|_| ZoneCheckRefusal::Fault("concretisation"))?;
                if !self.point_diverges(&state, &concrete)? {
                    // FR-241 names no cause for a finite path with no
                    // divergent concretisation, only `LassoNotConcretized`
                    // for a lasso; it is reused here (spec gap STD-493).
                    return Err(self.undecided(ZoneUndecided::LassoNotConcretized));
                }
                concrete
            }
        };
        let (discrete_key, _) = self
            .timed
            .system()
            .state_key(&state)
            .map_err(|_| ZoneCheckRefusal::Fault("state-key"))?;
        let present: Vec<Rational> = self
            .timed
            .places()
            .iter()
            .zip(&concrete.last_clocks)
            .filter(|(place, _)| self.timed.live(place, &state))
            .map(|(_, value)| value.clone())
            .collect();
        let (kind, over_binding, fairness) = match badness {
            Badness::False(instance) => (
                CounterexampleKind::Formula,
                Some(instance),
                self.claim()
                    .map_or_else(Vec::new, |claim| claim.clause.fairness().to_vec()),
            ),
            Badness::Undefined(instance, cause, locus) => {
                let claim = self
                    .claim()
                    .ok_or(ZoneCheckRefusal::Fault("undefined-claim"))?;
                (
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site: UndefinedSite {
                            position: widen(concrete.prefix.len()),
                            state_key: qsl_eval::simulation::StateKey::from_canonical_bytes(
                                discrete_key,
                            ),
                            locus,
                            transition: None,
                            time: Some(Box::new(TimedPoint {
                                stamp: concrete.last_stamp.clone(),
                                clocks: present,
                            })),
                        },
                        cause,
                    }),
                    Some(instance),
                    claim.clause.fairness().to_vec(),
                )
            }
            Badness::Deadlock(_) => (CounterexampleKind::Deadlock, None, Vec::new()),
        };
        // A violation reached by idle-tail stutters is the terminal
        // stutter's loop; the delay before the violating stutter has no
        // field (spec gap STD-493).
        let lasso = (stutters > 0).then_some(Lasso {
            entry: concrete.prefix.len(),
            terminal_stutter: true,
        });
        Ok(TemporalCounterexample {
            initial_state: initial,
            prefix: concrete.prefix,
            over_binding,
            fairness,
            kind,
            // A violation read at an idle-tail position: the behaviour ends
            // in the stutter step repeated (FR-236 "idle tail"). The
            // stutter's delay has no field in the counterexample (spec gap
            // STD-493).
            lasso,
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        })
    }
}

/// A path to a node lifted to concrete states (QSpec FR-430).
pub(super) struct LiftedPath {
    /// The index of the initial state it starts from.
    pub(super) initial: usize,
    /// Its stages, over concrete states.
    pub(super) stages: Vec<super::concretise::Stage>,
    /// The idle-tail stutters among them.
    pub(super) stutters: usize,
    /// The permutation from the node's canonical state to the concrete one.
    pub(super) sigma: Permutation,
    /// The concrete state it ends at.
    pub(super) state: ModelSimState,
}

/// `instance` with its key mapped by `sigma`.
fn lift_instance(instance: ModelInstance, sigma: &Permutation) -> ModelInstance {
    let key = sigma.image(&instance.population, &instance.key).to_owned();
    ModelInstance {
        population: instance.population,
        key,
    }
}

/// One symbolic successor.
pub(super) struct Successor {
    /// The step's index among the source's guarded steps.
    pub step: usize,
    /// The convex part's index among the step's parts.
    pub part: usize,
    /// The model state it reaches.
    pub state: ModelSimState,
    /// The zone it reaches.
    pub zone: Dbm,
    /// Whether the step is a tick source's tick operation.
    pub tick: bool,
    /// The next task scheduler state, absent for an ordinary model subject.
    pub task_state: Option<TaskSchedulerState>,
    /// The derived scheduler action, absent for source-model steps.
    pub task_transition: Option<TaskTransitionIdentity>,
}

/// A derived zero-time task activation following its checked source release.
struct TaskActivationSuccessor {
    scheduler: TaskSchedulerState,
    zone: Dbm,
    transition: TaskTransitionIdentity,
    subtractions: Vec<TaskSubtraction>,
}

/// Apply one timed model step's simultaneous clock carries and resets.
pub(super) fn apply_step_clocks(
    zone: &mut Dbm,
    step: &GuardedStep,
    factor: &BigInt,
) -> Result<(), DbmError> {
    let carries: Vec<_> = step
        .carries()
        .iter()
        .map(|(target, source)| (target + 1, source + 1))
        .collect();
    let resets: Vec<_> = step
        .resets()
        .iter()
        .map(|(place, value)| (place + 1, scaled(value, factor)))
        .collect();
    zone.assign_clocks(&carries, &resets)
}

/// The exact predecessor zone through one timed model step's clock update.
pub(super) fn preimage_step_clocks(
    zone: &Dbm,
    step: &GuardedStep,
    factor: &BigInt,
) -> Result<Dbm, DbmError> {
    let carries: Vec<_> = step
        .carries()
        .iter()
        .map(|(target, source)| (target + 1, source + 1))
        .collect();
    let resets: Vec<_> = step
        .resets()
        .iter()
        .map(|(place, value)| (place + 1, scaled(value, factor)))
        .collect();
    zone.preimage_clocks(&carries, &resets)
}

/// Intersect `zone` with `constraints`, bounds in units of `1 / factor`.
pub(super) fn constrain_at(
    zone: &mut Dbm,
    constraints: &[PlaceConstraint],
    factor: &BigInt,
) -> Result<(), DbmError> {
    for constraint in constraints {
        let clock = constraint.place() + 1;
        let bound = scaled(constraint.bound(), factor);
        match constraint.op() {
            ClockOp::Less => zone.constrain(clock, 0, Bound::lt(bound))?,
            ClockOp::LessOrEqual => zone.constrain(clock, 0, Bound::le(bound))?,
            ClockOp::Equal => {
                zone.constrain(clock, 0, Bound::le(bound.clone()))?;
                zone.constrain(0, clock, Bound::le(-bound))?;
            }
            ClockOp::GreaterOrEqual => zone.constrain(0, clock, Bound::le(-bound))?,
            ClockOp::Greater => zone.constrain(0, clock, Bound::lt(-bound))?,
        }
    }
    Ok(())
}

/// `zone` intersected with `other`.
pub(super) fn intersect(zone: &mut Dbm, other: &Dbm) -> Result<(), DbmError> {
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            if i != j {
                zone.constrain(i, j, other.bound(i, j)?.clone())?;
            }
        }
    }
    Ok(())
}

/// The parts of `zone` no zone of `covers` holds, as zones whose union is
/// `zone` minus the union of `covers`. Equal covers count once: many steps
/// share one guard (a granted collection's candidates each give a step),
/// and the subtraction splits once per cover.
pub(super) fn uncovered(zone: &Dbm, covers: &[Dbm]) -> Result<Vec<Dbm>, DbmError> {
    let mut distinct: Vec<Dbm> = Vec::with_capacity(covers.len());
    for cover in covers {
        if !distinct.contains(cover) {
            distinct.push(cover.clone());
        }
    }
    subtract(zone, &distinct)
}

/// [`uncovered`] over covers already distinct: a cover that holds all of
/// `zone` leaves nothing, and one disjoint from it is skipped.
fn subtract(zone: &Dbm, covers: &[Dbm]) -> Result<Vec<Dbm>, DbmError> {
    if zone.is_empty() {
        return Ok(Vec::new());
    }
    let Some((first, rest)) = covers.split_first() else {
        return Ok(vec![zone.clone()]);
    };
    if first.includes(zone)? {
        return Ok(Vec::new());
    }
    let mut meet = zone.clone();
    intersect(&mut meet, first)?;
    if meet.is_empty() {
        return subtract(zone, rest);
    }
    let mut parts = Vec::new();
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            if i == j {
                continue;
            }
            let Bound::Finite { value, strict } = first.bound(i, j)? else {
                continue;
            };
            if zone.bound(i, j)? <= first.bound(i, j)? {
                continue;
            }
            // Outside `x_i - x_j ≺ value`: `x_j - x_i ≺' -value`.
            let mut outside = zone.clone();
            let negation = if *strict {
                Bound::le(-value)
            } else {
                Bound::lt(-value)
            };
            outside.constrain(j, i, negation)?;
            parts.extend(subtract(&outside, rest)?);
        }
    }
    Ok(parts)
}

/// Each place's `L` and `U` and the scale factor: every constant of the
/// timed model, with 1, scaled to integers.
fn scaled_bounds(
    timed: &TimedSystem<'_, '_>,
    window: Option<&WindowClaim>,
    automaton_constants: &[Rational],
) -> (BigInt, Vec<Option<BigInt>>, Vec<Option<BigInt>>) {
    let model = timed.model();
    let mut constants = vec![Rational::from_integer(quire_exact::Integer::one())];
    // Per clock field: (lower, upper) constants compared with it.
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
    // Every guard part, and every part of a quantifier family below one
    // (FR-230 "Quantified reads"): a family's template constrains each slot
    // place of its clock.
    let mut parts: Vec<&qsl_semantics::check::ConvexPart> = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .collect();
    while let Some(part) = parts.pop() {
        for constraint in part.clocks() {
            record(
                constraint.clock().index(),
                constraint.op(),
                constraint.bound(),
            );
        }
        parts.extend(part.families().iter().flat_map(|family| family.parts()));
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
    constants.extend(lower.iter().flatten().cloned());
    constants.extend(upper.iter().flatten().cloned());
    let horizon: Vec<Rational> = window.map_or_else(Vec::new, |window| window.constants().to_vec());
    constants.extend(horizon.iter().cloned());
    // A timed claim automaton's interval ends are exact at the scale too.
    constants.extend(automaton_constants.iter().cloned());
    let factor = scale_constants(&constants).factor().clone();
    let max = |values: &[Rational]| values.iter().max().map(|value| scaled(value, &factor));
    let mut lower: Vec<Option<BigInt>> = timed
        .places()
        .iter()
        .map(|place| max(&lower[place.clock().index()]))
        .collect();
    let mut upper: Vec<Option<BigInt>> = timed
        .places()
        .iter()
        .map(|place| max(&upper[place.clock().index()]))
        .collect();
    // The horizon clock is compared with every interval end, from both
    // sides.
    if window.is_some() {
        lower.push(max(&horizon));
        upper.push(max(&horizon));
    }
    (factor, lower, upper)
}

/// `value` in units of `1 / scale`; exact when `scale` is a multiple of its
/// denominator.
pub(super) fn scaled(value: &Rational, scale: &BigInt) -> BigInt {
    value.numerator().as_big() * scale / value.denominator().as_big()
}

/// The bytes a stored zone takes, by its bounds' magnitudes.
pub(super) fn zone_bytes(zone: &Dbm) -> u64 {
    let mut bytes = 0_u64;
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            let size = match zone.bound(i, j) {
                Ok(Bound::Finite { value, .. }) => 16 + value.bits().div_ceil(8),
                _ => 16,
            };
            bytes = bytes.saturating_add(size);
        }
    }
    bytes
}

pub(super) fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// A shortest retained-edge path from `start` to `target`, if one exists.
fn graph_path_within(
    out: &[Vec<usize>],
    edges: &[SymbolicEdge],
    start: usize,
    target: usize,
    allowed: &[bool],
) -> Option<Vec<usize>> {
    if start == target {
        return Some(Vec::new());
    }
    let mut parent = vec![None; out.len()];
    let mut seen = vec![false; out.len()];
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    while let Some(node) = queue.pop_front() {
        for &edge in out.get(node)? {
            let next = edges.get(edge)?.to;
            if !allowed.get(next).copied().unwrap_or(false) || seen[next] {
                continue;
            }
            seen[next] = true;
            parent[next] = Some((node, edge));
            if next == target {
                let mut path = Vec::new();
                let mut at = target;
                while at != start {
                    let (previous, edge) = parent[at]?;
                    path.push(edge);
                    at = previous;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(next);
        }
    }
    None
}

/// Time 0.
fn zero_stamp() -> Rational {
    Rational::from_integer(quire_exact::Integer::zero())
}

/// FR-126's refusals before any state is explored: `RequiresBound`, or an
/// extent that could not be classified.
pub(super) fn not_simulated(refusal: NotSimulated) -> ZoneCheckRefusal {
    match refusal {
        NotSimulated::RequiresBound(roots) => ZoneCheckRefusal::RequiresBound(roots),
        NotSimulated::Extent(failure) => ZoneCheckRefusal::Extent(Box::new(failure)),
        _ => ZoneCheckRefusal::Fault("bounds-precheck-returns-only-requires-bound-or-extent"),
    }
}

/// FR-242's TT-2 automaton of a `BoundedWindow` clause over `instances`
/// instances that nests no interval operator under another; `None` for any
/// other clause, which the liveness product reads.
fn flat_window(
    clause: &qsl_semantics::check::CheckedTemporalClause,
    instances: usize,
    period: Option<Rational>,
    max_states: u64,
) -> Result<Option<WindowClaim>, ZoneCheckRefusal> {
    if !matches!(
        clause.timed_form(),
        Some(TimedPropertyForm::BoundedWindow { .. })
    ) {
        return Ok(None);
    }
    match WindowClaim::new(clause.formula(), instances, period, max_states) {
        Ok(claim) => Ok(Some(claim)),
        Err(WindowRefusal::NestedWindow) => Ok(None),
        Err(refusal) => Err(refusal.into()),
    }
}
