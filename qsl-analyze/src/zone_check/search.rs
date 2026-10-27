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

use std::collections::{BTreeMap, VecDeque};

use num_bigint::BigInt;
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{
    check_model_bounds, GuardedStep, ModelSimState, PlaceConstraint, TerminalClass, TimedStop,
    TimedSystem,
};
use qsl_eval::simulation::{NotSimulated, RequiresBound, BOUND_POSITIONS};
use qsl_eval::temporal::AtomReading;
use qsl_replay::{
    CounterexampleKind, Lasso, TemporalCounterexample, TimedPoint, UndefinedEvaluation,
    UndefinedLocus, UndefinedSite,
};
use qsl_semantics::check::{
    CheckedTemporalClause, ClockOp, FormulaNodeId, TemporalProfile, TimedPropertyForm,
};
use qsl_semantics::family::ClassifyFailure;
use quire_exact::{Cancel, Meter, Rational};

use super::certificate::Proof;
use super::concretise::{concretise, Concrete, Target};
use super::divergence::Escape;
use super::liveness::Liveness;
use super::{scale_constants, Bound, Dbm, DbmError};
use qsl_eval::automaton::{
    position_automaton, AutomatonError, PositionAutomaton, Read, Stamp, WindowClaim, WindowFault,
    WindowRefusal,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};

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
}

impl Default for ZoneCheckLimits {
    fn default() -> Self {
        Self {
            max_symbolic_states: 1 << 20,
            max_symbolic_transitions: 1 << 24,
            max_automaton_states: 1 << 20,
            max_zone_bytes: 1 << 30,
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
}

/// Why a run stopped with no verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZoneStop {
    /// A budget was reached.
    LimitReached(ZoneCheckLimit),
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

/// A stored symbolic state.
#[derive(Clone, Debug)]
pub(super) struct SymbolicNode {
    /// The model state.
    pub state: ModelSimState,
    /// The zone over clock places, in scaled units.
    pub zone: Dbm,
    /// The index of its initial state, for an initial node.
    pub initial: Option<usize>,
    /// Whether it violates the item only on time-locked valuations.
    pub locked: bool,
    /// The claim-automaton state; 0 with no automaton.
    pub q: usize,
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
}

/// How a new node is reached: an edge from a stored node, with its step
/// and piece.
#[derive(Clone, Copy, Debug)]
struct Arrival {
    from: usize,
    step: Option<(usize, usize)>,
    piece: Option<usize>,
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
    /// The first admitted violation in breadth-first order, concretised
    /// with exact delays (FR-241), to be replayed before it counts.
    Violated {
        /// The counterexample.
        counterexample: Box<TemporalCounterexample>,
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
    /// An invariant of the engine broke.
    #[error("the zone engine broke an invariant: {0}")]
    Fault(&'static str),
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
    };
    if !matches {
        return Err(ZoneCheckRefusal::ObligationMismatch);
    }
    // FR-239: an unbounded population root refuses before any state is
    // explored, as FR-126 states.
    check_model_bounds(request.subject.system(), BOUND_POSITIONS).map_err(not_simulated)?;
    let mut window = None;
    let mut live = None;
    let judged = match request.item {
        ZoneCheckItem::Clause { clause, letters } => {
            let instances = || {
                request
                    .subject
                    .system()
                    .instances(&clause.over().1)
                    .map_err(not_simulated)
            };
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
                    instances: instances()?,
                })
            } else if let Some(claim) = flat_window(
                clause,
                instances()?.len(),
                request.subject.tick_period().cloned(),
                request.limits.max_automaton_states,
            )? {
                let instances = instances()?;
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
                    instances: instances()?,
                })
            } else {
                return Err(ZoneCheckRefusal::UnsupportedForm);
            }
        }
        ZoneCheckItem::DeadlockFreedom => Judged::DeadlockFreedom,
        ZoneCheckItem::TimeLockFreedom => Judged::TimeLockFreedom,
    };
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
    );
    run.live = live;
    let searched = match run.search() {
        Ok(searched) => searched,
        Err(Halt::Outcome(searched)) => *searched,
        Err(Halt::Refusal(refusal)) => return Err(refusal),
    };
    Ok(match searched {
        Searched::Holds { proof, stats } => ZoneCheckOutcome::Holds {
            certificate: Box::new(proof.proving(request.obligation)),
            stats,
        },
        Searched::Violated {
            counterexample,
            stats,
        } => ZoneCheckOutcome::Violated {
            counterexample,
            stats,
        },
        Searched::Undecided { cause, stats } => ZoneCheckOutcome::Undecided { cause, stats },
        Searched::Stopped { cause, stats } => ZoneCheckOutcome::Stopped { cause, stats },
    })
}

/// A run's outcome before its certificate names the item's obligation.
pub(super) enum Searched {
    Holds {
        proof: Box<Proof>,
        stats: ZoneStats,
    },
    Violated {
        counterexample: Box<TemporalCounterexample>,
        stats: ZoneStats,
    },
    Undecided {
        cause: ZoneUndecided,
        stats: ZoneStats,
    },
    Stopped {
        cause: ZoneStop,
        stats: ZoneStats,
    },
}

/// What a run judges its nodes by.
enum Judged<'r> {
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
}

impl<'r, 's, 'a> Run<'r, 's, 'a> {
    fn new(
        timed: &'r TimedSystem<'s, 'a>,
        judged: Judged<'r>,
        window: Option<WindowClaim>,
        automaton_constants: &[Rational],
        limits: ZoneCheckLimits,
        cancel: &'r Cancel,
    ) -> Self {
        let (scale, mut lower, mut upper) =
            scaled_bounds(timed, window.as_ref(), automaton_constants);
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
        }
    }

    /// The authored clause the run judges, if any.
    pub(super) fn claim(&self) -> Option<&Claim<'r>> {
        match &self.judged {
            Judged::Claim(claim) | Judged::Liveness(claim) => Some(claim),
            Judged::DeadlockFreedom | Judged::TimeLockFreedom => None,
        }
    }

    fn stats(&self) -> ZoneStats {
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
        Halt::Outcome(Box::new(Searched::Stopped {
            cause,
            stats: self.stats(),
        }))
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
        result.map_err(|stop| self.undecided(ZoneUndecided::UndecidedSuccessor(stop)))
    }

    pub(super) fn scaled(&self, value: &Rational) -> BigInt {
        scaled(value, &self.scale)
    }

    /// The dimension of every zone: the reference clock, the clock places
    /// and, for a TT-2 clause, the horizon clock.
    pub(super) fn dim(&self) -> usize {
        self.timed.places().len() + 1 + usize::from(self.window.is_some())
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
    fn settle(&self, state: &ModelSimState, zone: &mut Dbm) -> Result<(), Halt> {
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
                for (place, value) in step.resets() {
                    next.reset(place + 1, &scaled(value, factor))?;
                }
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
            // Position 0 is read at time 0, before time elapses.
            let q = match self.window {
                Some(_) => {
                    self.window_position(None, &discrete, Stamp::At(&zero_stamp()), false)?
                }
                None => 0,
            };
            self.settle(&discrete, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            if let Some(node) = self.insert(discrete, q, zone, Some(index), None)? {
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
            for created in self.expand(node)? {
                queue.push_back(created);
                delayed_deadlock |= self.judge(created)?;
            }
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

    /// Store `(state, q, zone)` unless a stored state with equal `state`
    /// and `q` covers it: the new node, or `None` with the covering node
    /// recorded through `arrival`.
    fn insert(
        &mut self,
        state: ModelSimState,
        q: usize,
        zone: Dbm,
        initial: Option<usize>,
        arrival: Option<Arrival>,
    ) -> Result<Option<usize>, Halt> {
        let (mut key, _) = self
            .timed
            .system()
            .state_key(&state)
            .map_err(|_| ZoneCheckRefusal::Fault("state-key"))?;
        key.extend_from_slice(&widen(q).to_be_bytes());
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
                        })?;
                    }
                    return Ok(None);
                }
            }
        }
        self.charge_state(&zone)?;
        let index = self.nodes.len();
        self.nodes.push(SymbolicNode {
            state,
            zone,
            initial,
            locked: false,
            q,
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
                })?;
                Some(self.edges.len() - 1)
            }
            None => None,
        };
        self.parent.push(parent);
        Ok(Some(index))
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
        self.expanded[node] = true;
        let state = self.nodes[node].state.clone();
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
            for mut successor in self.successors(&state, &zone, &scale)? {
                if let Some(h) = horizon {
                    successor.zone.free(h)?;
                }
                let arrival = Arrival {
                    from: node,
                    step: Some((successor.step, successor.part)),
                    piece: None,
                };
                if let Some(new) =
                    self.insert(successor.state, q, successor.zone, None, Some(arrival))?
                {
                    created.push(new);
                }
            }
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
                };
                if let Some(new) =
                    self.insert(successor.state.clone(), read, at, None, Some(arrival))?
                {
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
                };
                if let Some(new) = self.insert(state.clone(), read, at, None, Some(arrival))? {
                    created.push(new);
                }
            }
        }
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
        let claim = match &self.judged {
            Judged::Claim(claim) => claim,
            Judged::DeadlockFreedom => return self.deadlock(node),
            Judged::TimeLockFreedom | Judged::Liveness(_) => return Ok(None),
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
        let reset_breaks = target.iter().any(|bound| {
            step.resets()
                .iter()
                .rev()
                .find(|(place, _)| *place == bound.place())
                .is_some_and(|(_, value)| !bound.op().holds(value, bound.bound()))
        });
        if reset_breaks {
            return Ok(Vec::new());
        }
        let reset: Vec<usize> = step.resets().iter().map(|(place, _)| *place).collect();
        let unreset: Vec<PlaceConstraint> = target
            .into_iter()
            .filter(|bound| !reset.contains(&bound.place()))
            .collect();
        Ok(step
            .parts()
            .iter()
            .map(|part| part.iter().chain(&unreset).cloned().collect())
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
                for (place, value) in step.resets() {
                    zone.reset(place + 1, &self.scaled(value))?;
                }
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
    fn pre_path(&mut self, path: &[(ModelSimState, usize, usize)]) -> Result<Dbm, Halt> {
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
            for (place, value) in step.resets().iter().rev() {
                let value = self.scaled(value);
                region.constrain(place + 1, 0, Bound::le(value.clone()))?;
                region.constrain(0, place + 1, Bound::le(-value))?;
                region.free(place + 1)?;
            }
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
        let mut path = Vec::new();
        let mut at = node;
        while let Some(edge) = self.parent[at] {
            let edge = self.edges[edge].clone();
            if edge.covering {
                return Err(ZoneCheckRefusal::Fault("covering-parent").into());
            }
            let step = match edge.step {
                Some((step, part)) => {
                    let steps = self.steps_of(edge.from)?;
                    let step = steps
                        .get(step)
                        .cloned()
                        .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                    Some((step, part))
                }
                None => None,
            };
            path.push((edge.from, step, edge.piece));
            at = edge.from;
        }
        path.reverse();
        let initial = self.nodes[at]
            .initial
            .ok_or(ZoneCheckRefusal::Fault("path-root"))?;
        let pieces = self.window.as_ref().map(|_| self.pieces.clone());
        let mut stages = Vec::with_capacity(path.len());
        let mut stutters = 0_usize;
        for (from, step, piece) in path {
            let state = self.nodes[from].state.clone();
            let post = step.as_ref().map_or(&state, |(step, _)| step.post());
            let target = self.timed(self.timed.invariants(post))?;
            stutters += usize::from(step.is_none());
            let position = match (&pieces, piece) {
                (Some(pieces), Some(piece)) => pieces
                    .get(piece)
                    .map(|(constraints, _)| constraints.clone())
                    .ok_or(ZoneCheckRefusal::Fault("edge-piece"))?,
                _ => Vec::new(),
            };
            stages.push(super::concretise::Stage {
                invariants: self.timed(self.timed.invariants(&state))?,
                elapses: self.elapses(&state)?,
                target,
                step,
                position,
                position_resets: Vec::new(),
                before: None,
                piece: None,
                silent: false,
                fixed: None,
            });
        }
        Ok((initial, stages, stutters))
    }

    /// FR-241: the counterexample through the real edges to `node`.
    fn counterexample(
        &mut self,
        node: usize,
        badness: Badness,
        escape: Escape,
    ) -> Result<TemporalCounterexample, Halt> {
        let path = self.stages_to(node)?;
        let state = self.nodes[node].state.clone();
        self.finite_counterexample(path, state, badness, escape)
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
                    targets.push(self.pre_path(path)?);
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
        let (_, discrete_key) = self
            .timed
            .system()
            .state_key(&state)
            .map_err(|_| ZoneCheckRefusal::Fault("state-key"))?;
        let present: Vec<Rational> = self
            .timed
            .places()
            .iter()
            .zip(&concrete.last_clocks)
            .filter(|(place, _)| place.present(&state))
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
                            state_key: discrete_key,
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
/// `zone` minus the union of `covers`.
pub(super) fn uncovered(zone: &Dbm, covers: &[Dbm]) -> Result<Vec<Dbm>, DbmError> {
    if zone.is_empty() {
        return Ok(Vec::new());
    }
    let Some((first, rest)) = covers.split_first() else {
        return Ok(vec![zone.clone()]);
    };
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
            parts.extend(uncovered(&outside, rest)?);
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

fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Time 0.
fn zero_stamp() -> Rational {
    Rational::from_integer(quire_exact::Integer::zero())
}

/// FR-126's refusals before any state is explored: `RequiresBound`, or an
/// extent that could not be classified.
fn not_simulated(refusal: NotSimulated) -> ZoneCheckRefusal {
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
