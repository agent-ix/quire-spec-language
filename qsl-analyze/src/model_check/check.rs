// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1, first phase): [`check_model`] decides a safety item
//! over every behaviour of a model subject: a reachable invariant (TP-1), a
//! bounded MLTL clause (TP-2), an infinite-trace safety clause (TP-3), or the
//! deadlock-freedom item.
//!
//! Each item gets the deterministic bad-prefix monitor of its formula
//! (ADR-018 SM-6), built lazily in `qsl-eval`'s automaton kernel over the
//! clause's translation (`always holds(not deadlocked)` for the
//! deadlock-freedom item), and the engine explores the product of the
//! subject's state graph with it breadth-first in FR-101's canonical order:
//! initial states by ascending state-key bytes, successors by
//! transition-identity bytes then post-state key bytes. A product state is
//! (model state, monitor state of each instance), keyed by the model state's
//! FR-101 key and the monitor states' graph-local numbers. The translation
//! materializes its states only as the monitor reaches them, and the run's
//! automaton-state count is those states; both they and the monitor's own
//! states are bounded by `max_automaton_states` (ADR-018 IV-6).
//!
//! A clause's letter is read when its product state is created, for each
//! instance of the clause's `over` binding: every atom of the clause at every
//! position the claim reads (ADR-018 UE-1, FR-125). A TP-2 `on origin` claim
//! reads positions 0 to its horizon `h`, and its later positions evaluate no
//! atom; every other claim reads every position. The first product state in breadth-first
//! order whose letter is undefined, whose monitor state rejects, whose
//! expansion evaluates a contract conjunction undefined, whose bounded
//! closure rejects, or (for the deadlock-freedom item) that is deadlocked
//! ends the run with the canonical path to it (ADR-018 UE-4, DL-4, DL-7).
//! Once a bad state is created no later state is, and the states before it
//! are still expanded in order, so an earlier expansion-time violation wins.
//!
//! At a terminal model state a TP-2 clause closes its monitor by the bounded
//! profile's closed-boundary rule: the following positions read every atom
//! false (QSpec FR-091), until the monitor rejects or repeats. A TP-1 or TP-3
//! clause instead takes one terminal stutter edge, re-reading the terminal
//! letter (FR-125); its counterexample is the model prefix to the terminal
//! state, whose behaviour is that prefix stuttered forever.
//!
//! A run proves an item only when its frontier empties with no state bad:
//! every reachable product state was examined. Its `Holds` carries the
//! closure certificate the core checks before the proof settles (ADR-018
//! PC-3, FR-338): for a TP-1 clause under infinite-trace and the
//! deadlock-freedom item, every explored model state with the canonical key,
//! under each instance's context, of the carried root. A run that reaches its
//! horizon with a state left unexpanded settles V-5; a run that reaches a
//! limit or is cancelled settles V-7; neither is ever `Holds`.
//!
//! Remaining work: QSL-878 gives the TP-2 and TP-3 translation states their
//! FR-338 canonical keys. Until it lands such a `Holds` carries an empty
//! closure certificate, which the core checker cannot admit, so it never
//! settles `proved`. A liveness clause (TP-4) is decided by `liveness`,
//! which adds the second phase; an `on each` activation with a `when`
//! condition is refused before any state is explored, as [`ModelLetters`]
//! does not read it. A clause with no instance is `Undecided(NoInstance)`
//! before any state is explored (FR-125).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::automaton::{
    translate, Atom, AutomatonError, AutomatonKind, Gba, GbaBuilder, Guard, KeyRefusal, Letter,
    Monitor, SafetyAutomaton, SafetyState, StateId,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters, ModelSubject, ProtocolSubject};
use qsl_eval::simulation::model::{
    check_model_bounds, ModelFinding, ModelSimState, ModelSystem, TerminalClass,
};
use qsl_eval::simulation::protocol::{
    check_protocol_bounds, MemoryModel, ProtocolRefusal, ProtocolSystem, ProtocolTransition, Sc,
    SubjectMemory,
};
use qsl_eval::simulation::{
    ConstraintValue, EncodingRefusal, Expansion, ExpansionLimit, ExpansionStop, NotSimulated,
    Permutation, RequiresBound, SubjectSystem, SymmetryGroup, TransitionSystem,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::InternalFault;
use qsl_replay::undefined_finding;
use qsl_replay::{
    AppliedReduction, ClauseRootRefusal, ClosureCertificate, CounterexampleKind, IncompleteCause,
    Instance, ItemRefusal, MemoryBound, MemoryBoundMember, MemoryCause, ModelCheckItem,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ModelStep, ProductStateRef, Proviso,
    ReachedLimit, ReductionCause, RunStats, TemporalCounterexample, UndecidedCause,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use qsl_semantics::check::{Activation, FormulaNodeId, TemporalProfile};
use qsl_semantics::family::ClassifyFailure;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{Cancel, Meter};
use quire_semantic_value::fairness::{FairnessConstraint, PropertyForm};
use quire_semantic_value::model_transition::ModelTransition;

use super::ample::{Atoms, Por};
use super::concretise::{Lift, LiftStop};
use super::false_extension::false_extension_rejects;
use super::reduce::{self, InstanceGroup, Reductions, SymmetryRefusal, TableForm};

pub(super) use qsl_eval::simulation::BOUND_POSITIONS;

/// One EN-1 run (FR-126 Inputs).
pub struct ModelCheckRequest<'a> {
    /// The model subject.
    pub subject: ModelSubject<'a>,
    /// The item.
    pub item: ModelCheckItem<'a>,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
    /// The reductions the request selects (FR-159).
    pub reductions: Reductions,
}

/// Why [`check_model`] explored nothing (FR-126).
#[derive(Debug, thiserror::Error)]
pub enum ModelCheckRefusal {
    /// FR-106's admission of the subject's `ModelSystem` failed.
    #[error("the subject's model system did not admit: {0:?}")]
    Admission(Box<AdmissionFailure>),
    /// FR-336: a root of the subject is unbounded under its universes. The
    /// refusal names each unbounded root's `WireNodeId` (as the key of
    /// [`RequiresBound::domains`]) and is returned before any state is
    /// explored. A `ProofBound` never makes a root finite here.
    #[error("the subject has an unbounded root and requires a bound")]
    RequiresBound(RequiresBound),
    /// FR-101's pre-check could not classify the subject's extent.
    #[error("the subject's extent could not be classified: {0:?}")]
    Extent(Box<ClassifyFailure>),
    /// The subject's protocol system does not build (FR-205).
    #[error(transparent)]
    Protocol(ProtocolRefusal),
    /// The clause's property form is not one this engine decides yet: a
    /// liveness clause (TP-4), or a clause with no property form.
    #[error("EN-1 does not decide the property form {form:?} yet")]
    FormNotDecided {
        /// The clause's property form.
        form: Option<PropertyForm>,
    },
    /// The clause's `on each` activation has a `when` condition, which the
    /// letter seam does not read.
    #[error("the `when` condition of an `on each` activation is not read yet")]
    GuardedActivation,
    /// The clause has no property automaton: a past operator over a future
    /// one, or more atoms than a letter holds.
    #[error("the clause has no property automaton: {0}")]
    Translation(AutomatonError),
    /// A state or transition identity has no canonical encoding.
    #[error(transparent)]
    Encoding(#[from] EncodingRefusal),
    /// An automaton state key has no canonical encoding within the identity
    /// byte budget.
    #[error(transparent)]
    Key(#[from] KeyRefusal),
    /// The item has no canonical translation.
    #[error(transparent)]
    Item(ItemRefusal),
    /// A liveness clause this engine does not decide yet, or whose proof it
    /// cannot certify yet.
    #[error(transparent)]
    Liveness(#[from] super::liveness::LivenessRefusal),
    /// A hyper clause EN-1 does not decide, or whose subjects the request
    /// does not supply (FR-176).
    #[error(transparent)]
    Hyper(#[from] super::hyper::HyperRefusal),
    /// FR-151: a symmetry declaration is malformed.
    #[error(transparent)]
    Symmetry(SymmetryRefusal),
    /// FR-158 "Resolution": the request's constraint names no checked `at
    /// current` state clause of the package
    /// (`invalid_runtime_input`/`invalid-value`).
    #[error("invalid_runtime_input/invalid-value: no state constraint {name}")]
    UnknownConstraint {
        /// The request's constraint name.
        name: String,
    },
    /// A liveness clause with a state constraint selected: the liveness
    /// engine runs reduced under symmetry and partial-order reduction only.
    #[error("EN-1 does not decide a liveness clause under a state constraint yet")]
    ReducedLiveness,
    /// FR-155: partial-order reduction needs a footprint of an atom or a
    /// state constraint that reads through a form QSpec FR-383 gives no
    /// rule for.
    #[error(transparent)]
    Footprint(#[from] qsl_eval::simulation::model::FootprintError),
    /// An invariant of the engine or the subject broke.
    #[error("model check broke invariant {0:?}")]
    Fault(InternalFault),
}

/// The item as the run reads it.
enum Item<'a> {
    Clause {
        /// Each instance of the `over` binding, in FR-125's canonical
        /// instance order.
        bindings: Vec<ModelInstance>,
        letters: &'a dyn ModelLetters,
        fairness: Vec<FairnessConstraint>,
        /// The `holds` atom each letter atom reads: atom `i` is `atoms[i]`.
        atoms: Vec<FormulaNodeId>,
        /// The atom true at every represented position of a bounded `on
        /// each` clause.
        live: Option<Atom>,
        /// Whether a terminal state closes the monitor by the bounded
        /// profile's closed-boundary rule (TP-2) rather than stuttering.
        bounded: bool,
        /// The last position the claim reads: the horizon `h` of a TP-2 `on
        /// origin` clause, none when it reads every position (UE-1).
        last_read: Option<u64>,
    },
    Deadlock,
}

impl Item<'_> {
    /// The number of instances, each with its own monitor state.
    fn instance_count(&self) -> usize {
        match self {
            Self::Clause { bindings, .. } => bindings.len(),
            Self::Deadlock => 1,
        }
    }
}

/// One product state: its model state, key, depth, the step that first
/// reached it (`None` for the terminal stutter step), and the monitor state
/// of each instance after reading this position. An instance whose atom is
/// undefined there keeps the state it read the position from.
struct Node<State, Transition> {
    state: State,
    digest: DigestRecord,
    depth: usize,
    parent: Option<(usize, Option<Transition>)>,
    initial_index: usize,
    monitor: Vec<StateId>,
    /// The permutation the canonicaliser mapped the reached state to
    /// `state` with (FR-152): for an initial state the state the subject
    /// lists, for any other the successor its parent's step reached. The
    /// identity under the trivial group.
    permutation: Permutation,
    /// Whether the state constraint is false at `state` (FR-158): a
    /// boundary state is stored and read, never expanded.
    boundary: bool,
}

/// Why the run ended before its frontier emptied.
enum Finish {
    Violated(Box<Violation>),
    Undecided(UndecidedCause),
    Stopped(IncompleteCause, Option<ReachedLimit>),
}

/// A violation: its kind, the node it is at, and the `over` binding the
/// counterexample names.
struct Violation {
    kind: CounterexampleKind,
    at: usize,
    binding: Option<ModelInstance>,
}

impl Finish {
    fn violated(kind: CounterexampleKind, at: usize, binding: Option<ModelInstance>) -> Self {
        Self::Violated(Box::new(Violation { kind, at, binding }))
    }
}

type Step<T> = Result<T, Halt>;

/// What reading a position found, and the instance that found it.
type Finding = (CounterexampleKind, usize);

/// Each instance's monitor state after reading a position, and what the read
/// found.
type Reading = (Vec<StateId>, Option<Finding>);

/// One successor of an expansion: the transition identity's order bytes, the
/// successor's state-key bytes and digest, the transition (none for the
/// terminal stutter), the canonical state and the permutation that
/// canonicalised it.
type Successor<S> = (
    Vec<u8>,
    Vec<u8>,
    DigestRecord,
    Option<<S as TransitionSystem>::TransitionId>,
    <S as TransitionSystem>::State,
    Permutation,
);

/// An ended run, or a refusal discovered after exploration started.
enum Halt {
    Finish(Finish),
    Refusal(ModelCheckRefusal),
}

impl From<Finish> for Halt {
    fn from(end: Finish) -> Self {
        Self::Finish(end)
    }
}

impl From<ModelCheckRefusal> for Halt {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<EncodingRefusal> for Halt {
    fn from(refusal: EncodingRefusal) -> Self {
        Self::Refusal(ModelCheckRefusal::Encoding(refusal))
    }
}

pub(super) fn fault(invariant: &'static str) -> ModelCheckRefusal {
    ModelCheckRefusal::Fault(InternalFault::new("model-check", invariant))
}

pub(super) fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// The item, the automaton its monitor reads, and whether the run's proof
/// has a canonical closure certificate.
type Prepared<'a> = (Item<'a>, Box<dyn SafetyAutomaton>, bool);

/// FR-126: decide `request`'s item over every behaviour of its subject,
/// polling `cancel` before every expansion and every atom evaluation.
///
/// # Errors
///
/// [`ModelCheckRefusal`] when the subject does not admit, a root is
/// unbounded, the item is a guarded `on each` clause or has no
/// property automaton, or an identity cannot be encoded; no outcome is
/// settled then.
pub fn check_model(
    request: ModelCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    if let ModelCheckItem::Clause { clause, .. } = request.item {
        if clause.property_form() == Some(PropertyForm::Liveness) {
            // FR-159 (a) reads the clause alone, so it settles before the
            // liveness engine reads its fairness set.
            if !request.reductions.is_empty() {
                let form =
                    TableForm::of(&request.item).ok_or(ModelCheckRefusal::FormNotDecided {
                        form: clause.property_form(),
                    })?;
                if let Some(reduction) = reduce::not_preserving(&request.reductions, form) {
                    return Ok(settled(ReductionCause::NotPreserving {
                        reduction,
                        form: PropertyForm::Liveness,
                    }));
                }
            }
            return super::liveness::check(request, cancel);
        }
    }
    if !request.reductions.is_empty() {
        return check_reduced(request, cancel);
    }
    let max_automaton_states = request.limits.max_automaton_states;
    // The form is refused before admission reads any snapshot.
    let (mut item, automaton, certified) = prepare(request.item, max_automaton_states)?;
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation.clone())
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(not_simulated)?;
    let (instances, root_cancelled) = if certified {
        match request.item.instances(&request.subject, &system, cancel) {
            Ok(instances) => (instances, None),
            Err(ItemRefusal::ClauseRoot(ClauseRootRefusal::Cancelled(cause))) => {
                (Vec::new(), Some(cause))
            }
            Err(ItemRefusal::Unbounded(refusal)) => return Err(not_simulated(refusal)),
            Err(refusal) => return Err(ModelCheckRefusal::Item(refusal)),
        }
    } else {
        (Vec::new(), None)
    };
    if let (Item::Clause { bindings, .. }, ModelCheckItem::Clause { clause, .. }) =
        (&mut item, request.item)
    {
        // FR-125: the instances come from the subject's universes; a
        // certified proof's are those its certificate's translations carry.
        *bindings = if certified {
            instances
                .iter()
                .filter_map(|instance| instance.binding.clone())
                .collect()
        } else {
            system.instances(&clause.over().1).map_err(not_simulated)?
        };
    }
    let mut run = Run {
        system: &system,
        reducer: Some(&system),
        item: &item,
        limits: request.limits,
        max_depth: request.max_depth,
        meter,
        max_candidates,
        cancel,
        nodes: Vec::new(),
        known: BTreeMap::new(),
        queue: VecDeque::new(),
        edges: 0,
        horizon_cut: false,
        first_bad: None,
        monitor: Monitor::bad_prefix(automaton, max_automaton_states)
            .map_err(|_| fault("a-safety-item-has-a-safety-automaton"))?,
        group: SymmetryGroup::default(),
        constraint: None,
        boundary: 0,
        chooser: None,
        memory_bound: None,
        bound_limited: BTreeSet::new(),
    };
    if let Some(source) = root_cancelled {
        return Ok(ModelCheckOutcome::Stopped {
            cause: IncompleteCause::Cancelled { source },
            limit: None,
            stats: run.stats(),
        });
    }
    if no_initial {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats: run.stats(),
        });
    }
    if item.instance_count() == 0 {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats: run.stats(),
        });
    }
    run.decide(&instances)
}

/// FR-215 "Model checking a protocol subject" (ADR-027 TS-6): one EN-1 run
/// over a protocol subject.
pub struct ProtocolCheckRequest<'a> {
    /// The protocol subject.
    pub subject: ProtocolSubject<'a>,
    /// The item: a safety clause over the subject's model, or the
    /// deadlock-freedom item, whose deadlocks are FR-211's.
    pub item: ModelCheckItem<'a>,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The method bound `max_store_buffer` (FR-225, ADR-025 MB-2): the
    /// entries per thread's buffer under `tso`; `None` runs with 4.
    pub max_store_buffer: Option<u64>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
}

/// FR-215 (ADR-027 TS-5, TS-6): decide `request`'s item over every
/// behaviour of its protocol subject with [`check_model`]'s engine, over
/// the subject's protocol system under its resolved memory models
/// (FR-219): `ProtocolSystem<Sc>`, or `ProtocolSystem<Tso>` bounded by
/// `max_store_buffer` when a `parallel` resolves to `tso` (FR-221, FR-225).
/// The requires-bound pre-check reads the model's roots and the protocol's
/// before any state is explored, and EN-1's access-shape pre-check settles
/// `WeakAccessShape` before any state is explored (FR-220). A run that left
/// a store unexpanded under the bound and found no counterexample settles
/// `MemoryBoundReached` (ADR-025 MB-4). A deadlock counterexample names the
/// waiting threads of its last state (ADR-027 PD-4).
///
/// # Errors
///
/// [`ModelCheckRefusal`] as [`check_model`] returns it,
/// [`ModelCheckRefusal::FormNotDecided`] for a liveness clause, whose
/// scheduler fairness (FR-212) this engine does not read yet, and
/// [`ModelCheckRefusal::Protocol`] when the protocol system does not build.
pub fn check_protocol(
    request: ProtocolCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
    let max_automaton_states = request.limits.max_automaton_states;
    let (mut item, automaton, certified) = prepare(request.item, max_automaton_states)?;
    let subject = &request.subject;
    let simulation = subject.model.simulation.clone();
    let meter = simulation.meter;
    let max_candidates = simulation.expansion.max_candidates;
    let no_initial = simulation.initial.is_empty();
    let model = ModelSystem::new(simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    let protocol = subject
        .model
        .simulation
        .package
        .graph()
        .protocol(&subject.protocol)
        .ok_or_else(|| {
            ModelCheckRefusal::Protocol(ProtocolRefusal::UnknownProtocol {
                protocol: subject.protocol.clone(),
            })
        })?;
    check_protocol_bounds(&model, protocol, BOUND_POSITIONS).map_err(not_simulated)?;
    let (instances, root_cancelled) = if certified {
        match request.item.instances(&subject.model, &model, cancel) {
            Ok(instances) => (instances, None),
            Err(ItemRefusal::ClauseRoot(ClauseRootRefusal::Cancelled(cause))) => {
                (Vec::new(), Some(cause))
            }
            Err(ItemRefusal::Unbounded(refusal)) => return Err(not_simulated(refusal)),
            Err(refusal) => return Err(ModelCheckRefusal::Item(refusal)),
        }
    } else {
        (Vec::new(), None)
    };
    if let (Item::Clause { bindings, .. }, ModelCheckItem::Clause { clause, .. }) =
        (&mut item, request.item)
    {
        *bindings = if certified {
            instances
                .iter()
                .filter_map(|instance| instance.binding.clone())
                .collect()
        } else {
            model.instances(&clause.over().1).map_err(not_simulated)?
        };
    }
    let protocol_run = ProtocolRun {
        item: &item,
        instances: &instances,
        root_cancelled,
        no_initial,
        limits: request.limits,
        max_depth: request.max_depth,
        meter,
        max_candidates,
        cancel,
        max_automaton_states,
    };
    let memory = SubjectMemory::of(&subject.memory, request.max_store_buffer);
    let built = match memory {
        SubjectMemory::Sc => ProtocolSystem::new(model, subject, Sc)
            .map(|system| protocol_run.decide(&system, automaton, None)),
        SubjectMemory::Tso(tso) => {
            let bound = MemoryBound {
                member: MemoryBoundMember::MaxStoreBuffer,
                value: tso.max_store_buffer(),
            };
            ProtocolSystem::new(model, subject, tso)
                .map(|system| protocol_run.decide(&system, automaton, Some(bound)))
        }
    };
    match built {
        Ok(outcome) => outcome,
        // FR-220 "Access shape": the pre-check settles V-8 before any state
        // is explored.
        Err(ProtocolRefusal::WeakAccessShape(shape)) => Ok(ModelCheckOutcome::Memory {
            cause: MemoryCause::WeakAccessShape(shape),
            stats: RunStats::default(),
        }),
        Err(refusal) => Err(ModelCheckRefusal::Protocol(refusal)),
    }
}

/// What [`check_protocol`] runs over its protocol system, whichever memory
/// model the subject's resolved models need.
struct ProtocolRun<'r> {
    item: &'r Item<'r>,
    instances: &'r [Instance],
    root_cancelled: Option<quire_exact::CancelCause>,
    no_initial: bool,
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    meter: quire_exact::ScalarLimits,
    max_candidates: u64,
    cancel: &'r Cancel,
    max_automaton_states: u64,
}

impl ProtocolRun<'_> {
    /// The unreduced run over `system`, under the memory bound `bound`
    /// when its memory model has one (ADR-025 MB-3, MB-4).
    fn decide<M: MemoryModel>(
        &self,
        system: &ProtocolSystem<'_, M>,
        automaton: Box<dyn SafetyAutomaton>,
        bound: Option<MemoryBound>,
    ) -> Result<ModelCheckOutcome<ProtocolTransition>, ModelCheckRefusal> {
        let mut run = Run {
            system,
            reducer: None,
            item: self.item,
            limits: self.limits,
            max_depth: self.max_depth,
            meter: self.meter,
            max_candidates: self.max_candidates,
            cancel: self.cancel,
            nodes: Vec::new(),
            known: BTreeMap::new(),
            queue: VecDeque::new(),
            edges: 0,
            horizon_cut: false,
            first_bad: None,
            monitor: Monitor::bad_prefix(automaton, self.max_automaton_states)
                .map_err(|_| fault("a-safety-item-has-a-safety-automaton"))?,
            group: SymmetryGroup::default(),
            constraint: None,
            boundary: 0,
            chooser: None,
            memory_bound: bound,
            bound_limited: BTreeSet::new(),
        };
        if let Some(source) = self.root_cancelled {
            return Ok(ModelCheckOutcome::Stopped {
                cause: IncompleteCause::Cancelled { source },
                limit: None,
                stats: run.stats(),
            });
        }
        if self.no_initial {
            return Ok(ModelCheckOutcome::Undecided {
                cause: UndecidedCause::NoInitialState,
                stats: run.stats(),
            });
        }
        if self.item.instance_count() == 0 {
            return Ok(ModelCheckOutcome::Undecided {
                cause: UndecidedCause::NoInstance,
                stats: run.stats(),
            });
        }
        run.decide(self.instances)
    }
}

/// An outcome the reduction pre-check settles, with no state explored.
fn settled(cause: ReductionCause) -> ModelCheckOutcome {
    ModelCheckOutcome::Reduction {
        cause,
        stats: RunStats::default(),
    }
}

/// One product a reduced item runs: the `over` bindings it reads, the group
/// it is reduced by, and the keys its one bound instance stands for.
struct Plan {
    bindings: Vec<ModelInstance>,
    group: SymmetryGroup,
    orbit: Vec<String>,
}

/// The counts of `left` and `right` together.
fn sum(left: RunStats, right: RunStats) -> RunStats {
    RunStats {
        product_states: left.product_states.saturating_add(right.product_states),
        product_edges: left.product_edges.saturating_add(right.product_edges),
        automaton_states: left.automaton_states.saturating_add(right.automaton_states),
        depth: left.depth.max(right.depth),
    }
}

/// FR-159, FR-151, FR-158 and FR-162 over a safety item: the pre-check,
/// then one product per [`Plan`]: per orbit of the `over` binding under
/// symmetry, else one. The first run that does not complete its method
/// settles the item; otherwise boundary states settle `ConstraintReached`
/// (before `BoundReached`), a horizon cut `BoundReached`, and a completed
/// search `HoldsReduced` (ADR-021 RV-1, RV-4, RV-5).
fn check_reduced(
    request: ModelCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let reductions = &request.reductions;
    let max_automaton_states = request.limits.max_automaton_states;
    // The form is refused before admission reads any snapshot.
    prepare(request.item, max_automaton_states)?;
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation.clone())
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(not_simulated)?;
    let group = match precheck(reductions, &request.item, &system)? {
        Admitted::Group(group) => group,
        Admitted::Settled(outcome) => return Ok(outcome),
    };
    if no_initial {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats: RunStats::default(),
        });
    }
    let symmetric = !reductions.symmetry.is_empty();
    let plans = match request.item {
        ModelCheckItem::Clause { clause, .. } => {
            let bindings = system.instances(&clause.over().1).map_err(not_simulated)?;
            if bindings.is_empty() {
                return Ok(ModelCheckOutcome::Undecided {
                    cause: UndecidedCause::NoInstance,
                    stats: RunStats::default(),
                });
            }
            if symmetric {
                reduce::instance_groups(Some(&bindings), &group)
                    .into_iter()
                    .map(|instance: InstanceGroup| Plan {
                        bindings: instance.representative.into_iter().collect(),
                        group: instance.group,
                        orbit: instance.orbit,
                    })
                    .collect()
            } else {
                vec![Plan {
                    bindings,
                    group: SymmetryGroup::default(),
                    orbit: Vec::new(),
                }]
            }
        }
        ModelCheckItem::DeadlockFreedom => vec![Plan {
            bindings: Vec::new(),
            group: group.clone(),
            orbit: Vec::new(),
        }],
    };
    let mut total = RunStats::default();
    let mut boundary: usize = 0;
    let mut horizon = false;
    let mut applied = Vec::new();
    for plan in plans {
        let (mut item, automaton, _) = prepare(request.item, max_automaton_states)?;
        if let Item::Clause { bindings, .. } = &mut item {
            *bindings = plan.bindings;
        }
        let ample = if reductions.partial_order {
            Some(ample_of(&system, &item, reductions.constraint.as_deref())?)
        } else {
            None
        };
        let mut run = Run {
            system: &system,
            reducer: Some(&system),
            item: &item,
            limits: request.limits,
            max_depth: request.max_depth,
            meter,
            max_candidates,
            cancel,
            nodes: Vec::new(),
            known: BTreeMap::new(),
            queue: VecDeque::new(),
            edges: 0,
            horizon_cut: false,
            first_bad: None,
            monitor: Monitor::bad_prefix(automaton, max_automaton_states)
                .map_err(|_| fault("a-safety-item-has-a-safety-automaton"))?,
            group: plan.group.clone(),
            constraint: reductions.constraint.as_deref(),
            boundary: 0,
            chooser: ample.as_ref().map(|ample| ample as &dyn Chooser<_, _>),
            memory_bound: None,
            bound_limited: BTreeSet::new(),
        };
        let explored = run.explore();
        total = sum(total, run.stats());
        match explored {
            Ok(()) => {
                boundary = boundary.saturating_add(run.boundary);
                horizon |= run.horizon_cut;
                if symmetric {
                    applied.push(AppliedReduction::Symmetry {
                        groups: group.classes.clone(),
                        orbit: plan.orbit,
                        stabiliser: plan.group.classes,
                    });
                }
            }
            Err(Halt::Refusal(refusal)) => return Err(refusal),
            Err(Halt::Finish(Finish::Undecided(cause))) => {
                return Ok(ModelCheckOutcome::Undecided {
                    cause,
                    stats: total,
                })
            }
            Err(Halt::Finish(Finish::Stopped(cause, limit))) => {
                return Ok(ModelCheckOutcome::Stopped {
                    cause,
                    limit,
                    stats: total,
                })
            }
            Err(Halt::Finish(Finish::Violated(violation))) => {
                let Violation { kind, at, binding } = *violation;
                // FR-161 "Length and limits": a stop while concretising
                // settles V-7.
                return match run.model_counterexample(kind, at, binding) {
                    Ok(counterexample) => Ok(ModelCheckOutcome::Violated {
                        counterexample: Box::new(counterexample),
                        stats: total,
                    }),
                    Err(Halt::Finish(Finish::Stopped(cause, limit))) => {
                        Ok(ModelCheckOutcome::Stopped {
                            cause,
                            limit,
                            stats: total,
                        })
                    }
                    Err(Halt::Finish(Finish::Undecided(cause))) => {
                        Ok(ModelCheckOutcome::Undecided {
                            cause,
                            stats: total,
                        })
                    }
                    Err(Halt::Finish(Finish::Violated(_))) => {
                        Err(fault("a-concretised-path-ends-once"))
                    }
                    Err(Halt::Refusal(refusal)) => Err(refusal),
                };
            }
        }
    }
    if reductions.partial_order {
        applied.push(AppliedReduction::PartialOrder {
            proviso: Proviso::BreadthFirstRevisit,
        });
    }
    let max_depth = widen(request.max_depth.unwrap_or(0));
    Ok(if boundary > 0 {
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached {
                boundary_states: widen(boundary),
            },
            stats: total,
        }
    } else if horizon && reductions.partial_order {
        // ADR-021 RV-5: a reduced graph does not keep short counterexamples
        // short, so a partial-order run to the horizon proves no bound.
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::Horizon { max_depth },
            stats: total,
        }
    } else if horizon {
        ModelCheckOutcome::BoundReached {
            depth: max_depth,
            stats: total,
        }
    } else {
        ModelCheckOutcome::HoldsReduced {
            reductions: applied,
            stats: total,
        }
    })
}

/// The ample-set choice and the enforcing expansion of a partial-order run
/// (FR-156, FR-155 "Read enforcement"), read by [`Run`] through [`Chooser`].
trait Chooser<State, Transition> {
    /// `state`'s expansion, holding every step to its footprint.
    fn expand(
        &self,
        state: &State,
    ) -> Result<Expansion<Transition, State, ModelFinding>, ExpansionStop>;

    /// The ample set among `enabled` at `state`, where `fresh` says whether
    /// every successor through a transition is a product state not yet
    /// discovered (C3).
    fn choose(
        &self,
        state: &State,
        enabled: &[Transition],
        fresh: &dyn Fn(&Transition) -> bool,
    ) -> Result<Vec<Transition>, ExpansionStop>;
}

/// A model subject's ample sets: [`Por`] over its `ModelSystem`.
struct ModelAmple<'r, 'a> {
    system: &'r ModelSystem<'a>,
    por: Por<'r, 'a>,
}

impl Chooser<ModelSimState, ModelTransition> for ModelAmple<'_, '_> {
    fn expand(
        &self,
        state: &ModelSimState,
    ) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
        self.system.successors_enforced(state)
    }

    fn choose(
        &self,
        state: &ModelSimState,
        enabled: &[ModelTransition],
        fresh: &dyn Fn(&ModelTransition) -> bool,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let mut enabled = enabled.to_vec();
        enabled.sort();
        enabled.dedup();
        self.por.ample(state, &enabled, fresh)
    }
}

/// The inputs of ample-set choice for `item` over `system` under the state
/// constraint `constraint` (FR-156 "Atom footprints", "Visibility").
fn ample_of<'r, 'a>(
    system: &'r ModelSystem<'a>,
    item: &'r Item<'r>,
    constraint: Option<&str>,
) -> Result<ModelAmple<'r, 'a>, ModelCheckRefusal> {
    let por = match item {
        Item::Clause {
            bindings,
            letters,
            fairness,
            atoms,
            ..
        } => Por::new(
            system,
            Some(Atoms {
                letters: *letters,
                nodes: atoms,
            }),
            bindings,
            fairness,
            constraint,
        ),
        Item::Deadlock => Por::new(system, None, &[], &[], constraint),
    }?;
    Ok(ModelAmple { system, por })
}

/// What FR-159's pre-check admits: the run's group, or an outcome it
/// settles before any state is explored.
pub(super) enum Admitted {
    /// Every selected reduction admits; the group is trivial with no
    /// symmetry.
    Group(SymmetryGroup),
    /// The pre-check settled the item.
    Settled(ModelCheckOutcome),
}

/// FR-159's pre-check over `system`, in its order: (a) the preservation
/// table, (b) admission of the symmetry declarations and closure of the
/// initial states under their group, (c) the reductions the system offers,
/// (d) resolution of the state constraint.
///
/// # Errors
///
/// [`ModelCheckRefusal`] for a malformed declaration, an unknown
/// constraint, or an identity with no canonical encoding.
pub(super) fn precheck(
    reductions: &Reductions,
    item: &ModelCheckItem<'_>,
    system: &ModelSystem<'_>,
) -> Result<Admitted, ModelCheckRefusal> {
    let form = TableForm::of(item).ok_or_else(|| fault("a-decided-item-has-a-table-row"))?;
    if let Some(reduction) = reduce::not_preserving(reductions, form) {
        let form = match item {
            ModelCheckItem::Clause { clause, .. } => clause
                .property_form()
                .ok_or_else(|| fault("a-decided-clause-has-a-form"))?,
            ModelCheckItem::DeadlockFreedom => PropertyForm::ReachableInvariant,
        };
        return Ok(Admitted::Settled(settled(ReductionCause::NotPreserving {
            reduction,
            form,
        })));
    }
    let group = reduce::admit(system, &reductions.symmetry).map_err(ModelCheckRefusal::Symmetry)?;
    let closed = reduce::closure(system, &group).map_err(|refusal| match refusal {
        reduce::ClosureRefusal::Encoding(refusal) => ModelCheckRefusal::Encoding(refusal),
        reduce::ClosureRefusal::Unordered => fault("a-permuted-collection-is-ordered"),
    })?;
    if let Some(cause) = closed {
        return Ok(Admitted::Settled(settled(cause)));
    }
    if let Some(reduction) = reduce::unoffered(reductions, system) {
        return Ok(Admitted::Settled(settled(ReductionCause::Unsupported {
            reduction,
        })));
    }
    if let Some(name) = &reductions.constraint {
        if !system.has_constraint(name) {
            return Err(ModelCheckRefusal::UnknownConstraint { name: name.clone() });
        }
    }
    Ok(Admitted::Group(group))
}

/// FR-336 and FR-125: an unbounded root, or an admitting population with no
/// universe, is `RequiresBound`; a stopped classification is `Extent`.
pub(super) fn not_simulated(refusal: NotSimulated) -> ModelCheckRefusal {
    match refusal {
        NotSimulated::RequiresBound(roots) => ModelCheckRefusal::RequiresBound(roots),
        NotSimulated::Extent(failure) => ModelCheckRefusal::Extent(Box::new(failure)),
        _ => fault("bounds-precheck-returns-only-requires-bound-or-extent"),
    }
}

/// Read the item: a TP-1, TP-2 or TP-3 clause's translation, or the
/// deadlock-freedom item's monitor of `always holds(not deadlocked)`.
fn prepare(
    item: ModelCheckItem<'_>,
    max_automaton_states: u64,
) -> Result<Prepared<'_>, ModelCheckRefusal> {
    let ModelCheckItem::Clause { clause, letters } = item else {
        return Ok((Item::Deadlock, Box::new(deadlock_automaton()?), true));
    };
    let form = clause.property_form();
    let bounded = match form {
        Some(PropertyForm::ReachableInvariant | PropertyForm::Safety) => false,
        Some(PropertyForm::BoundedMltl) => true,
        Some(PropertyForm::Liveness) | None => {
            return Err(ModelCheckRefusal::FormNotDecided { form })
        }
    };
    if matches!(clause.activation(), Activation::Each { guarded: true }) {
        return Err(ModelCheckRefusal::GuardedActivation);
    }
    let automaton =
        translate(clause, max_automaton_states).map_err(ModelCheckRefusal::Translation)?;
    if automaton.kind() == AutomatonKind::Liveness {
        return Err(fault("a-safety-form-translates-to-a-safety-automaton"));
    }
    let certified = form == Some(PropertyForm::ReachableInvariant)
        && clause.profile() == TemporalProfile::InfiniteTrace;
    let last_read = match clause.activation() {
        Activation::Origin if bounded => clause.horizon(),
        Activation::Origin | Activation::Each { .. } => None,
    };
    let prepared = Item::Clause {
        bindings: Vec::new(),
        letters,
        fairness: clause.fairness().to_vec(),
        atoms: automaton.atoms().to_vec(),
        live: automaton.live(),
        bounded,
        last_read,
    };
    Ok((prepared, Box::new(automaton), certified))
}

/// The automaton of `always a`, with `a` the letter's atom 0 (`not
/// deadlocked`): one state that reads `a` at every position, the initial
/// state included.
fn deadlock_automaton() -> Result<Gba, ModelCheckRefusal> {
    let holding = Guard::always().requiring(Atom::new(0).map_err(|_| fault("atom-0-exists"))?);
    let mut builder = GbaBuilder::new();
    builder
        .add_transition(Gba::INITIAL, holding, Gba::INITIAL)
        .map_err(|_| fault("the-initial-state-exists"))?;
    Ok(builder.build())
}

/// FR-152 and FR-158 over a subject's states: what a reduced run reads of a
/// model subject. A protocol subject offers no reduction, so its run has no
/// reducer and a trivial group.
trait Reducer<State> {
    fn canonicalise(
        &self,
        state: &State,
        group: &SymmetryGroup,
    ) -> Result<(State, Permutation), ExpansionStop>;

    fn constraint_value(&self, state: &State, name: &str)
        -> Result<ConstraintValue, ExpansionStop>;
}

impl Reducer<ModelSimState> for ModelSystem<'_> {
    fn canonicalise(
        &self,
        state: &ModelSimState,
        group: &SymmetryGroup,
    ) -> Result<(ModelSimState, Permutation), ExpansionStop> {
        ModelSystem::canonicalise(self, state, group)
    }

    fn constraint_value(
        &self,
        state: &ModelSimState,
        name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        ModelSystem::constraint_value(self, state, name)
    }
}

struct Run<'r, S: SubjectSystem> {
    system: &'r S,
    /// The system's reductions; `None` for a subject that offers none.
    reducer: Option<&'r dyn Reducer<S::State>>,
    item: &'r Item<'r>,
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    meter: quire_exact::ScalarLimits,
    max_candidates: u64,
    cancel: &'r Cancel,
    nodes: Vec<Node<S::State, S::TransitionId>>,
    /// Each product state's index by its key: the model state's key bytes
    /// and the monitor state of each instance.
    known: BTreeMap<(Vec<u8>, Vec<StateId>), usize>,
    queue: VecDeque<usize>,
    edges: usize,
    horizon_cut: bool,
    /// The first bad product state created and its violation; no state is
    /// created after it.
    first_bad: Option<(usize, Box<Violation>)>,
    monitor: Monitor<Box<dyn SafetyAutomaton>>,
    /// The group states are canonicalised under (FR-152); trivial with no
    /// symmetry.
    group: SymmetryGroup,
    /// The state constraint's clause name (FR-158).
    constraint: Option<&'r str>,
    /// The boundary states stored.
    boundary: usize,
    /// The ample-set choice when partial-order reduction is selected
    /// (FR-156); `None` for a subject that offers none.
    chooser: Option<&'r dyn Chooser<S::State, S::TransitionId>>,
    /// The memory bound the subject's memory model runs under (ADR-025
    /// MB-2); `None` for one with no bound.
    memory_bound: Option<MemoryBound>,
    /// The distinct model states, by digest, whose expansion cut a step at
    /// the memory bound (ADR-025 MB-3); a model state read under several
    /// monitor states counts once.
    bound_limited: BTreeSet<DigestRecord>,
}

impl<S: SubjectSystem> Run<'_, S> {
    fn stats(&self) -> RunStats {
        RunStats {
            product_states: widen(self.nodes.len()),
            product_edges: widen(self.edges),
            automaton_states: widen(self.monitor.automaton().state_count()),
            depth: widen(self.nodes.iter().map(|node| node.depth).max().unwrap_or(0)),
        }
    }

    /// The closure certificate of a run that proved its item: every explored
    /// model state under each instance's carried root, sorted by model state
    /// then key. With no instance translation (a TP-2 or TP-3 clause) it is
    /// empty; Remaining work: QSL-878.
    fn certificate(&self, instances: &[Instance]) -> Result<ClosureCertificate, ModelCheckRefusal> {
        let mut states = Vec::with_capacity(self.nodes.len().saturating_mul(instances.len()));
        for instance in instances {
            let key = instance.translation.key(SafetyState::Carried)?;
            states.extend(self.nodes.iter().map(|node| ProductStateRef {
                model_state: node.digest,
                automaton_state: key.clone(),
            }));
        }
        states.sort();
        states.dedup();
        Ok(ClosureCertificate { states })
    }

    fn poll(&self) -> Result<(), Finish> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(Finish::Stopped(IncompleteCause::Cancelled { source }, None));
            }
        }
        Ok(())
    }

    fn stopped(limit: ModelCheckLimit, value: u64) -> Finish {
        Finish::Stopped(
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    }

    fn automaton(error: AutomatonError) -> Halt {
        match error {
            AutomatonError::StateLimit { limit } => {
                Self::stopped(ModelCheckLimit::MaxAutomatonStates, limit).into()
            }
            AutomatonError::AtomOutOfRange { .. }
            | AutomatonError::UnknownState { .. }
            | AutomatonError::NotSafetyAutomaton { .. }
            | AutomatonError::InvertedInterval { .. }
            | AutomatonError::PastOverFuture
            | AutomatonError::ProfileNotTranslated { .. }
            | AutomatonError::Invariant { .. } => fault("monitor-steps").into(),
        }
    }

    fn rejects(&self, state: StateId) -> Step<bool> {
        self.monitor.is_rejecting(state).map_err(Self::automaton)
    }

    /// An unreduced run's outcome: explore, then settle what it found. Under
    /// the trivial group the path to a violation is already concrete, so no
    /// expansion runs that could stop.
    fn decide(
        &mut self,
        instances: &[Instance],
    ) -> Result<ModelCheckOutcome<S::TransitionId>, ModelCheckRefusal> {
        match self.explore() {
            // ADR-025 MB-4: a bound-limited state settles before the horizon.
            Ok(()) => Ok(
                if let (Some(bound), true) = (self.memory_bound, !self.bound_limited.is_empty()) {
                    ModelCheckOutcome::Memory {
                        cause: MemoryCause::BoundReached {
                            bound,
                            states: widen(self.bound_limited.len()),
                        },
                        stats: self.stats(),
                    }
                } else if self.horizon_cut {
                    ModelCheckOutcome::BoundReached {
                        depth: widen(self.max_depth.unwrap_or(0)),
                        stats: self.stats(),
                    }
                } else {
                    ModelCheckOutcome::Holds {
                        certificate: self.certificate(instances)?,
                        stats: self.stats(),
                    }
                },
            ),
            Err(Halt::Refusal(refusal)) => Err(refusal),
            Err(Halt::Finish(Finish::Undecided(cause))) => Ok(ModelCheckOutcome::Undecided {
                cause,
                stats: self.stats(),
            }),
            Err(Halt::Finish(Finish::Stopped(cause, limit))) => Ok(ModelCheckOutcome::Stopped {
                cause,
                limit,
                stats: self.stats(),
            }),
            Err(Halt::Finish(Finish::Violated(violation))) => {
                let Violation { kind, at, binding } = *violation;
                let counterexample = match self.counterexample(kind, at, binding) {
                    Ok(counterexample) => counterexample,
                    Err(Halt::Refusal(refusal)) => return Err(refusal),
                    Err(Halt::Finish(_)) => return Err(fault("an-unreduced-path-is-concrete")),
                };
                Ok(ModelCheckOutcome::Violated {
                    counterexample: Box::new(counterexample),
                    stats: self.stats(),
                })
            }
        }
    }

    /// The breadth-first first phase.
    fn explore(&mut self) -> Step<()> {
        self.poll()?;
        let start = self.monitor.initial().map_err(Self::automaton)?;
        let mut initial = Vec::new();
        for (index, state) in self.system.initial().into_iter().enumerate() {
            let (state, permutation) = self.canonical(state)?;
            let (key, digest) = self.system.state_key(&state)?;
            initial.push((key, index, digest, state, permutation));
        }
        // Equal keys coalesce into the first initial state that has the key.
        initial.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        initial.dedup_by(|later, earlier| later.0 == earlier.0);
        if initial.len() > self.limits.limits.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            )
            .into());
        }
        let before = vec![start; self.item.instance_count()];
        for (key, index, digest, state, permutation) in initial {
            if self.first_bad.is_some() {
                break;
            }
            let node = Node {
                state,
                digest,
                depth: 0,
                parent: None,
                initial_index: index,
                monitor: Vec::new(),
                permutation,
                boundary: false,
            };
            self.create(key, node, &before, None)?;
        }
        while let Some(at) = self.queue.pop_front() {
            self.poll()?;
            if self.first_bad.as_ref().is_some_and(|(bad, _)| *bad == at) {
                if let Some((_, violation)) = self.first_bad.take() {
                    return Err(Finish::Violated(violation).into());
                }
            }
            // FR-158: a boundary state is never expanded, stutter-extended
            // or classified as a deadlock.
            if self.nodes[at].boundary {
                continue;
            }
            let expanded = match self.chooser {
                Some(chooser) => chooser
                    .expand(&self.nodes[at].state)
                    .map(|expansion| (expansion, false)),
                None => self.system.successors_bounded(&self.nodes[at].state),
            };
            let (expansion, cut) = match expanded {
                Ok(expanded) => expanded,
                Err(stop) => return Err(self.expansion_stop(stop)),
            };
            self.contracts(at, &expansion.findings)?;
            if cut {
                self.bound_limited.insert(self.nodes[at].digest);
                // ADR-025 MB-3: a state whose only enabled steps are cut
                // stores is a boundary state: never stutter-extended and
                // never a deadlock.
                if expansion.successors.is_empty() {
                    continue;
                }
            }
            if matches!(self.item, Item::Deadlock) {
                let after = self.deadlock_letter(at, !expansion.successors.is_empty())?;
                self.nodes[at].monitor = vec![after];
            }
            let terminal = expansion.successors.is_empty();
            if terminal {
                self.close(at)?;
            }
            if self.first_bad.is_some() {
                continue;
            }
            let mut steps = Vec::with_capacity(expansion.successors.len());
            for (transition, state) in expansion.successors {
                let order = self.system.transition_key(&transition)?;
                let (state, permutation) = self.canonical(state)?;
                let (key, digest) = self.system.state_key(&state)?;
                steps.push((order, key, digest, Some(transition), state, permutation));
            }
            steps.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
            if terminal && self.stutters() {
                let (key, digest) = self.system.state_key(&self.nodes[at].state)?;
                steps.push((
                    Vec::new(),
                    key,
                    digest,
                    None,
                    self.nodes[at].state.clone(),
                    Permutation::identity(),
                ));
            }
            let depth = self.nodes[at].depth;
            let before = self.nodes[at].monitor.clone();
            if self.max_depth.is_some_and(|horizon| depth >= horizon) {
                // A terminal stutter that coalesces with an explored product
                // state leaves nothing unexplored.
                let unexplored = match steps.pop() {
                    Some((_, key, digest, None, state, permutation)) => {
                        let node = Node {
                            state,
                            digest,
                            depth: depth.saturating_add(1),
                            parent: Some((at, None)),
                            initial_index: self.nodes[at].initial_index,
                            monitor: Vec::new(),
                            permutation,
                            boundary: false,
                        };
                        let (after, bad) = self.advance(&node, &before)?;
                        bad.is_some() || !self.known.contains_key(&(key, after))
                    }
                    Some(_) => true,
                    None => false,
                };
                self.horizon_cut |= unexplored;
                continue;
            }
            let mut reads = self.ample(at, &before, &mut steps)?.into_iter();
            for (_, key, digest, transition, state, permutation) in steps {
                if self.edges >= self.limits.limits.max_transitions {
                    return Err(Self::stopped(
                        ModelCheckLimit::MaxTransitions,
                        widen(self.limits.limits.max_transitions),
                    )
                    .into());
                }
                self.edges += 1;
                let node = Node {
                    state,
                    digest,
                    depth: depth.saturating_add(1),
                    parent: Some((at, transition)),
                    initial_index: self.nodes[at].initial_index,
                    monitor: Vec::new(),
                    permutation,
                    boundary: false,
                };
                self.create(key, node, &before, reads.next().flatten())?;
                if self.first_bad.is_some() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// FR-156: keep of `steps`, the canonical successors of node `at`, those
    /// through its ample set. A step through the terminal stutter edge or a
    /// state with a single enabled transition is kept; so is every step when
    /// no candidate satisfies C2 and C3.
    ///
    /// Judging C3 reads each successor's letter, so the readings come back,
    /// one per kept step, for `create` to use instead of reading the letter
    /// again; they are all `None` when nothing was judged.
    fn ample(
        &mut self,
        at: usize,
        before: &[StateId],
        steps: &mut Vec<Successor<S>>,
    ) -> Step<Vec<Option<Reading>>> {
        let Some(chooser) = self.chooser else {
            return Ok(vec![None; steps.len()]);
        };
        let mut enabled: Vec<S::TransitionId> = Vec::new();
        for transition in steps.iter().filter_map(|step| step.3.as_ref()) {
            if !enabled.contains(transition) {
                enabled.push(transition.clone());
            }
        }
        if enabled.len() <= 1 {
            return Ok(vec![None; steps.len()]);
        }
        // C3 on product states (POR-8): a step is fresh when its product
        // state is bad, which is always created, or not yet discovered.
        let depth = self.nodes[at].depth;
        let mut fresh: Vec<bool> = Vec::with_capacity(steps.len());
        let mut readings: Vec<Reading> = Vec::with_capacity(steps.len());
        for (_, key, digest, _, state, permutation) in steps.iter() {
            let node = Node {
                state: state.clone(),
                digest: *digest,
                depth: depth.saturating_add(1),
                parent: Some((at, None)),
                initial_index: self.nodes[at].initial_index,
                monitor: Vec::new(),
                permutation: permutation.clone(),
                boundary: false,
            };
            let (after, bad) = self.advance(&node, before)?;
            fresh.push(bad.is_some() || !self.known.contains_key(&(key.clone(), after.clone())));
            readings.push((after, bad));
        }
        let reaches_known = |transition: &S::TransitionId| {
            steps
                .iter()
                .zip(&fresh)
                .any(|(step, fresh)| step.3.as_ref() == Some(transition) && !fresh)
        };
        let chosen = chooser
            .choose(&self.nodes[at].state, &enabled, &|transition| {
                !reaches_known(transition)
            })
            .map_err(|stop| self.expansion_stop(stop))?;
        let kept: Vec<bool> = steps
            .iter()
            .map(|step| {
                step.3
                    .as_ref()
                    .is_none_or(|transition| chosen.contains(transition))
            })
            .collect();
        let mut kept_flags = kept.iter();
        steps.retain(|_| *kept_flags.next().unwrap_or(&true));
        Ok(readings
            .into_iter()
            .zip(kept)
            .filter_map(|(reading, kept)| kept.then_some(Some(reading)))
            .collect())
    }

    /// `state`'s canonical form under the run's group and the permutation
    /// that maps `state` to it (FR-152); `state` itself and the identity
    /// under the trivial group.
    fn canonical(&self, state: S::State) -> Step<(S::State, Permutation)> {
        if self.group.is_trivial() {
            return Ok((state, Permutation::identity()));
        }
        self.reducer
            .ok_or_else(|| fault("a-reduced-run-has-a-reducer"))?
            .canonicalise(&state, &self.group)
            .map_err(|stop| self.expansion_stop(stop))
    }

    /// Whether a terminal state takes the terminal stutter edge: a TP-1 or
    /// TP-3 clause under infinite-trace semantics (FR-125). The
    /// deadlock-freedom item's letter at a terminal state is decided by its
    /// classification alone, so a stutter adds nothing to it.
    fn stutters(&self) -> bool {
        matches!(self.item, Item::Clause { bounded: false, .. })
    }

    /// Create the product state of `node`'s model state reached with the
    /// monitor states `before`: read its letter for each instance and step
    /// the monitor, then coalesce it with an existing state of the same key
    /// or add it to the frontier. A state whose letter is undefined or whose
    /// monitor rejects is added as the run's first bad state.
    fn create(
        &mut self,
        key: Vec<u8>,
        mut node: Node<S::State, S::TransitionId>,
        before: &[StateId],
        read: Option<Reading>,
    ) -> Step<()> {
        let (after, mut bad) = match read {
            Some(read) => read,
            None => self.advance(&node, before)?,
        };
        node.monitor = after;
        let product = (key, node.monitor.clone());
        if bad.is_none() && self.known.contains_key(&product) {
            return Ok(());
        }
        if self.nodes.len() >= self.limits.limits.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            )
            .into());
        }
        if bad.is_none() {
            if let Some(name) = self.constraint {
                let reducer = self
                    .reducer
                    .ok_or_else(|| fault("a-constrained-run-has-a-reducer"))?;
                match reducer.constraint_value(&node.state, name) {
                    Ok(ConstraintValue::Holds) => {}
                    Ok(ConstraintValue::Boundary) => {
                        node.boundary = true;
                        self.boundary = self.boundary.saturating_add(1);
                    }
                    // ADR-021 SC-2: an undefined constraint refutes the item
                    // at this state, which is not a boundary state.
                    Ok(ConstraintValue::Undefined { cause, .. }) => {
                        let site =
                            Self::site(&node, UndefinedLocus::Constraint(name.to_owned()), None);
                        bad = Some((
                            CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                                site,
                                cause,
                            }),
                            0,
                        ));
                    }
                    Err(stop) => return Err(self.expansion_stop(stop)),
                }
            }
        }
        let at = self.nodes.len();
        self.nodes.push(node);
        self.queue.push_back(at);
        match bad {
            Some((kind, instance)) => {
                let binding = self.binding_of(instance);
                self.first_bad = Some((at, Box::new(Violation { kind, at, binding })));
            }
            None => {
                self.known.insert(product, at);
            }
        }
        Ok(())
    }

    /// Each instance's monitor state after reading `node`'s position from
    /// `before`, and the first finding with its instance: an undefined atom,
    /// which ends the read with the remaining instances left at `before`, or
    /// else the first rejecting monitor state.
    fn advance(
        &mut self,
        node: &Node<S::State, S::TransitionId>,
        before: &[StateId],
    ) -> Step<(Vec<StateId>, Option<Finding>)> {
        if matches!(self.item, Item::Deadlock) {
            return Ok((before.to_vec(), None));
        }
        let mut after = Vec::with_capacity(before.len());
        let mut bad = None;
        for (instance, from) in before.iter().enumerate() {
            let (state, finding) = self.read(node, instance, *from)?;
            after.push(state);
            match finding {
                Some(undefined @ CounterexampleKind::UndefinedEvaluation(_)) => {
                    bad = Some((undefined, instance));
                    break;
                }
                Some(kind) => {
                    bad.get_or_insert((kind, instance));
                }
                None => {}
            }
        }
        after.extend_from_slice(before.get(after.len()..).unwrap_or_default());
        Ok((after, bad))
    }

    /// Instance `instance`'s monitor state after reading `node`'s letter
    /// from `from`, evaluating every atom of the clause when the claim reads
    /// `node`'s position and none otherwise, and what the read found: an
    /// undefined atom, or a rejecting monitor state.
    fn read(
        &mut self,
        node: &Node<S::State, S::TransitionId>,
        instance: usize,
        from: StateId,
    ) -> Step<(StateId, Option<CounterexampleKind>)> {
        let Item::Clause {
            bindings,
            letters,
            atoms,
            live,
            last_read,
            ..
        } = self.item
        else {
            return Err(fault("only-a-clause-reads-letters").into());
        };
        let binding = bindings
            .get(instance)
            .ok_or_else(|| fault("instance-has-a-binding"))?;
        let mut letter = live.map_or_else(Letter::default, |live| Letter::default().with(live));
        let reads_position = last_read.is_none_or(|last| widen(node.depth) <= last);
        let read: &[FormulaNodeId] = if reads_position { atoms } else { &[] };
        for (index, formula_atom) in read.iter().enumerate() {
            let atom = Atom::new(index).map_err(|_| fault("clause-atom-in-range"))?;
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let reading = letters
                .atom(
                    *formula_atom,
                    binding,
                    &self.system.atom_state(&node.state),
                    &mut meter,
                )
                .map_err(|_| {
                    Self::stopped(ModelCheckLimit::EvaluationMeter, self.meter.work_units)
                })?;
            match reading {
                AtomReading::True => letter = letter.with(atom),
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    let site = Self::site(node, UndefinedLocus::Atom(*formula_atom), None);
                    let kind = CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site,
                        cause,
                    });
                    return Ok((from, Some(kind)));
                }
            }
        }
        let after = self.monitor.step(from, letter).map_err(Self::automaton)?;
        let finding = self.rejects(after)?.then_some(CounterexampleKind::Formula);
        Ok((after, finding))
    }

    /// QSpec FR-091's closed-boundary rule at the terminal node `at` of a
    /// TP-2 clause: each instance's monitor reads every following position
    /// with every atom false until it rejects, which violates the item at
    /// `at`, or repeats a state. No product state is created.
    fn close(&mut self, at: usize) -> Step<()> {
        if !matches!(self.item, Item::Clause { bounded: true, .. }) {
            return Ok(());
        }
        let reached = self.nodes[at].monitor.clone();
        for (instance, state) in reached.into_iter().enumerate() {
            if false_extension_rejects(&mut self.monitor, state).map_err(Self::automaton)? {
                let binding = self.binding_of(instance);
                return Err(Finish::violated(CounterexampleKind::Formula, at, binding).into());
            }
        }
        Ok(())
    }

    fn site(
        node: &Node<S::State, S::TransitionId>,
        locus: UndefinedLocus,
        transition: Option<ModelTransition>,
    ) -> UndefinedSite {
        UndefinedSite {
            position: widen(node.depth),
            state_key: node.digest,
            locus,
            transition,
        }
    }

    /// ADR-018 UE-6 and V-6 (ADR-027 TS-4): the first contract conjunction
    /// or protocol constraint the expansion evaluated undefined ends the run
    /// with a violation; otherwise a refused one leaves it undecided.
    fn contracts(&self, at: usize, findings: &[ModelFinding]) -> Step<()> {
        if let Some((locus, guard_of, cause)) = undefined_finding(findings) {
            let site = Self::site(&self.nodes[at], locus, guard_of);
            return Err(Finish::violated(
                CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause }),
                at,
                self.binding_of(0),
            )
            .into());
        }
        if findings.iter().any(ModelFinding::is_refused_contract) {
            return Err(Finish::Undecided(UndecidedCause::UndecidedSuccessor).into());
        }
        Ok(())
    }

    /// The `over` binding of a clause's instance `instance`, none for the
    /// deadlock-freedom item. A violation that is not a clause's own names
    /// the first instance.
    fn binding_of(&self, instance: usize) -> Option<ModelInstance> {
        match self.item {
            Item::Clause { bindings, .. } => bindings.get(instance).cloned(),
            Item::Deadlock => None,
        }
    }

    /// FR-124 and ADR-018 DL-7: the deadlock-freedom item's letter at node
    /// `at`, `not deadlocked`, read from its expansion: the monitor rejects
    /// a terminal state no declaration intends, which ends the run with a
    /// deadlock. An undefined `terminal when` ends it with an undefined
    /// evaluation, before the monitor steps; a refused one leaves it
    /// undecided.
    fn deadlock_letter(&mut self, at: usize, has_successor: bool) -> Step<StateId> {
        let class = match self
            .system
            .terminal_class(&self.nodes[at].state, has_successor)
        {
            Ok(class) => class,
            Err(stop) => return Err(self.expansion_stop(stop)),
        };
        let deadlocked = match class {
            TerminalClass::NotTerminal | TerminalClass::Intended => false,
            TerminalClass::Deadlocked => true,
            TerminalClass::Undefined { model, cause } => {
                let site = Self::site(&self.nodes[at], UndefinedLocus::Terminal(model), None);
                return Err(Finish::violated(
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause }),
                    at,
                    None,
                )
                .into());
            }
            TerminalClass::Undecided { .. } => {
                return Err(Finish::Undecided(UndecidedCause::UndecidedSuccessor).into())
            }
        };
        let letter = if deadlocked {
            Letter::default()
        } else {
            Letter::from_bits(1)
        };
        let before = *self.nodes[at]
            .monitor
            .first()
            .ok_or_else(|| fault("the-deadlock-item-has-one-instance"))?;
        let after = self.monitor.step(before, letter).map_err(Self::automaton)?;
        if self.rejects(after)? {
            return Err(Finish::violated(CounterexampleKind::Deadlock, at, None).into());
        }
        Ok(after)
    }

    /// An expansion stop: a named budget settles V-7 naming it, a broken
    /// invariant refuses, any other stop settles V-7 with no limit.
    fn expansion_stop(&self, stop: ExpansionStop) -> Halt {
        match stop.limit {
            Some(ExpansionLimit::EvaluationMeter) => {
                Self::stopped(ModelCheckLimit::EvaluationMeter, self.meter.work_units).into()
            }
            Some(ExpansionLimit::MaxCandidates) => {
                Self::stopped(ModelCheckLimit::MaxCandidates, self.max_candidates).into()
            }
            None if stop.is_internal_failure() => fault("model-expansion-invariant").into(),
            None => Finish::Stopped(IncompleteCause::ResourceExhausted, None).into(),
        }
    }

    /// The canonical path to node `at`: its breadth-first tree path,
    /// concretised under symmetry (FR-161). A terminal stutter step is no
    /// model step: the behaviour is the model prefix to the terminal state,
    /// stuttered forever.
    fn path(&self, at: usize) -> Vec<usize> {
        let mut path = vec![at];
        let mut cursor = at;
        while let Some((parent, _)) = &self.nodes[cursor].parent {
            cursor = *parent;
            path.push(cursor);
        }
        path.reverse();
        path
    }

    /// The canonical path to node `at` of an unreduced run, its steps the
    /// tree path's. A terminal stutter step is no step: the behaviour is the
    /// prefix to the terminal state, stuttered forever. A deadlock names the
    /// waiting threads of its last state (ADR-027 PD-4).
    fn counterexample(
        &self,
        kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ModelInstance>,
    ) -> Step<TemporalCounterexample<S::TransitionId>> {
        if !self.group.is_trivial() {
            return Err(fault("an-unreduced-path-is-concrete").into());
        }
        let path = self.path(at);
        let prefix = path
            .iter()
            .filter_map(|index| {
                let node = &self.nodes[*index];
                let (_, transition) = node.parent.as_ref()?;
                Some(ModelStep {
                    transition: transition.clone()?,
                    post_state: node.digest,
                })
            })
            .collect();
        let initial_state = path
            .first()
            .map_or(0, |root| self.nodes[*root].initial_index);
        self.finished(kind, at, over_binding, initial_state, prefix)
    }

    fn finished(
        &self,
        kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ModelInstance>,
        initial_state: usize,
        prefix: Vec<ModelStep<S::TransitionId>>,
    ) -> Step<TemporalCounterexample<S::TransitionId>> {
        let fairness = match self.item {
            Item::Clause { fairness, .. } => fairness.clone(),
            Item::Deadlock => Vec::new(),
        };
        let blocked = if matches!(kind, CounterexampleKind::Deadlock) {
            self.system
                .blocked(&self.nodes[at].state)
                .map_err(|stop| self.expansion_stop(stop))?
        } else {
            Vec::new()
        };
        Ok(TemporalCounterexample {
            initial_state,
            prefix,
            over_binding,
            fairness,
            kind,
            lasso: None,
            blocked,
            refinement: None,
        })
    }
}

impl Run<'_, ModelSystem<'_>> {
    /// The canonical path to node `at`: its breadth-first tree path,
    /// concretised under symmetry (FR-161).
    fn model_counterexample(
        &self,
        mut kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ModelInstance>,
    ) -> Step<TemporalCounterexample> {
        if self.group.is_trivial() {
            return self.counterexample(kind, at, over_binding);
        }
        let path = self.path(at);
        let (initial_state, prefix) = self.concretise(&path, &mut kind)?;
        self.finished(kind, at, over_binding, initial_state, prefix)
    }

    /// ADR-021 EI-6 (FR-161 "Start", "Steps"): the subject's initial state
    /// and the concrete steps of the canonical tree path `path`, each edge
    /// lifted by [`Lift`] and counted against what is left of
    /// `max_transitions`. An undefined evaluation's site is concretised at
    /// the path's end.
    fn concretise(
        &self,
        path: &[usize],
        kind: &mut CounterexampleKind,
    ) -> Step<(usize, Vec<ModelStep>)> {
        let first = &self.nodes[*path.first().ok_or_else(|| fault("a-path-has-a-root"))?];
        let budget = self
            .limits
            .limits
            .max_transitions
            .saturating_sub(self.edges);
        let lifted = (|| {
            let (initial, mut lift) = Lift::start(self.system, &self.group, &first.state, budget)?;
            for index in path.iter().skip(1) {
                let node = &self.nodes[*index];
                // The terminal stutter edge is no model step.
                if let Some((_, Some(transition))) = &node.parent {
                    lift.edge(transition, &node.state, &node.permutation)?;
                }
            }
            lift.site(kind)?;
            Ok((initial, lift.into_steps()))
        })();
        lifted.map_err(|stop| match stop {
            LiftStop::Expansion(stop) => self.expansion_stop(stop),
            LiftStop::Transitions => Self::stopped(
                ModelCheckLimit::MaxTransitions,
                widen(self.limits.limits.max_transitions),
            )
            .into(),
            LiftStop::Refusal(refusal) => refusal.into(),
        })
    }
}
