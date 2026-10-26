// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-239: EN-6's passed/waiting search of symbolic states `(s, Z)` over a
//! timed subject (ADR-026 EZ-1, EZ-3 to EZ-5), deciding a TT-1 timed
//! invariant or the deadlock-freedom item.
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
    GuardedStep, ModelSimState, PlaceConstraint, TerminalClass, TimedStop, TimedSystem,
};
use qsl_eval::temporal::AtomReading;
use qsl_replay::{
    CounterexampleKind, TemporalCounterexample, TimedPoint, UndefinedEvaluation, UndefinedLocus,
    UndefinedSite,
};
use qsl_semantics::check::{CheckedTemporalClause, ClockOp, FormulaNodeId, TemporalProfile};
use quire_exact::{Cancel, Meter, Rational};

use super::concretise::{concretise, Concrete, Target};
use super::divergence::Escape;
use super::{scale_constants, Bound, Dbm, DbmError};

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
    /// A TT-1 clause `always holds(A)` whose atom reads the model state.
    Invariant {
        /// The checked clause.
        clause: &'a CheckedTemporalClause,
        /// The seam to the one clause evaluator for its atom.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's deadlock-freedom item, read over time (FR-232).
    DeadlockFreedom,
}

/// A request to EN-6.
pub struct ZoneCheckRequest<'r, 's, 'a> {
    /// The timed subject (FR-231).
    pub subject: &'r TimedSystem<'s, 'a>,
    /// The item.
    pub item: ZoneCheckItem<'r>,
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
    /// concretisation of its path ends at a valuation that has one.
    LassoNotConcretized,
    /// A quiescent symbolic state has deadlocked valuations only after a
    /// delay, which no position of a counterexample names: FR-236 and QSpec
    /// FR-417 give `final_delay` to time-locks only (spec gap STD-493).
    DeadlockAfterDelay,
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
pub struct SymbolicNode {
    /// The model state.
    pub state: ModelSimState,
    /// The zone over clock places, in scaled units.
    pub zone: Dbm,
    /// The index of its initial state, for an initial node.
    pub initial: Option<usize>,
    /// Whether it violates the item only on time-locked valuations.
    pub locked: bool,
}

/// A retained edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymbolicEdge {
    /// A successor by a step's convex guard part, in canonical order.
    Step {
        /// The source node.
        from: usize,
        /// The target node.
        to: usize,
        /// The step's index among the source's guarded steps.
        step: usize,
        /// The convex part's index among the step's parts.
        part: usize,
    },
    /// A covered successor, to the node whose aLU abstraction covers it.
    Covering {
        /// The source node.
        from: usize,
        /// The covering node.
        to: usize,
        /// The step's index among the source's guarded steps.
        step: usize,
        /// The convex part's index.
        part: usize,
    },
}

impl SymbolicEdge {
    fn ends(&self) -> (usize, usize) {
        match self {
            Self::Step { from, to, .. } | Self::Covering { from, to, .. } => (*from, *to),
        }
    }
}

/// The retained symbolic graph, which FR-240 and FR-244 read.
#[derive(Clone, Debug)]
pub struct SymbolicGraph {
    /// The scale factor of every zone bound.
    pub scale: BigInt,
    /// Each clock place's `L` and `U`, in scaled units.
    pub lower: Vec<Option<BigInt>>,
    /// Each clock place's `U`.
    pub upper: Vec<Option<BigInt>>,
    /// The stored nodes, in creation order.
    pub nodes: Vec<SymbolicNode>,
    /// The retained edges.
    pub edges: Vec<SymbolicEdge>,
}

/// FR-239's outcome.
#[derive(Clone, Debug)]
pub enum ZoneCheckOutcome {
    /// No admitted violation is reachable, and an initial state has a
    /// time-divergent behaviour.
    Holds {
        /// The retained graph the certificate is built from (FR-244).
        graph: SymbolicGraph,
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
    /// The clause is not `always holds(A)` under infinite-trace or the
    /// timed profile.
    #[error("the clause is not a TT-1 `always holds(A)`")]
    NotTimedInvariant,
    /// The clause's `over` instances are not finite.
    #[error("the clause's instances are not finite")]
    Unbounded,
    /// An atom's evaluation could not give a reading.
    #[error("the atom's evaluation stopped: {0}")]
    Atom(String),
    /// An invariant of the engine broke.
    #[error("the zone engine broke an invariant: {0}")]
    Fault(&'static str),
}

impl From<DbmError> for ZoneCheckRefusal {
    fn from(_: DbmError) -> Self {
        Self::Fault("dbm-shape")
    }
}

/// What makes a node bad.
#[derive(Clone, Debug)]
enum Badness {
    /// The claim's atom is false for the instance.
    False(ModelInstance),
    /// The claim's atom is undefined for the instance.
    Undefined(ModelInstance, qsl_eval::temporal::UndefinedCause),
    /// The node holds deadlocked valuations at entry, within the zone.
    Deadlock(Dbm),
}

/// A halt of the search.
pub(super) enum Halt {
    Outcome(Box<ZoneCheckOutcome>),
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
/// [`ZoneCheckRefusal`] when the clause is not a TT-1 invariant, its
/// instances are not finite, an atom gives no reading, or the engine
/// breaks an invariant.
pub fn check_timed_model(
    request: ZoneCheckRequest<'_, '_, '_>,
    cancel: &Cancel,
) -> Result<ZoneCheckOutcome, ZoneCheckRefusal> {
    let claim = match request.item {
        ZoneCheckItem::Invariant { clause, letters } => {
            let atom = qsl_eval::automaton::invariant_atom(clause)
                .ok_or(ZoneCheckRefusal::NotTimedInvariant)?;
            if !matches!(
                clause.profile(),
                TemporalProfile::InfiniteTrace | TemporalProfile::Timed
            ) {
                return Err(ZoneCheckRefusal::NotTimedInvariant);
            }
            let instances = request
                .subject
                .system()
                .instances(&clause.over().1)
                .map_err(|_| ZoneCheckRefusal::Unbounded)?;
            Some(Claim {
                clause,
                letters,
                atom,
                instances,
            })
        }
        ZoneCheckItem::DeadlockFreedom => None,
    };
    let mut run = Run::new(request.subject, claim, request.limits, cancel);
    match run.search() {
        Ok(outcome) => Ok(outcome),
        Err(Halt::Outcome(outcome)) => Ok(*outcome),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

struct Claim<'r> {
    clause: &'r CheckedTemporalClause,
    letters: &'r dyn ModelLetters,
    atom: FormulaNodeId,
    instances: Vec<ModelInstance>,
}

pub(super) struct Run<'r, 's, 'a> {
    pub(super) timed: &'r TimedSystem<'s, 'a>,
    claim: Option<Claim<'r>>,
    pub(super) limits: ZoneCheckLimits,
    cancel: &'r Cancel,
    scale: BigInt,
    lower: Vec<Option<BigInt>>,
    upper: Vec<Option<BigInt>>,
    nodes: Vec<SymbolicNode>,
    keys: Vec<Vec<u8>>,
    expanded: Vec<bool>,
    /// The real edge that created each node, `None` for an initial one.
    parent: Vec<Option<usize>>,
    edges: Vec<SymbolicEdge>,
    /// The retained edges leaving each node, by index into `edges`.
    out: Vec<Vec<usize>>,
    passed: BTreeMap<Vec<u8>, Vec<usize>>,
    zone_bytes: u64,
    /// The guarded steps of each model state reached, by its key.
    steps: BTreeMap<Vec<u8>, Vec<GuardedStep>>,
}

impl<'r, 's, 'a> Run<'r, 's, 'a> {
    fn new(
        timed: &'r TimedSystem<'s, 'a>,
        claim: Option<Claim<'r>>,
        limits: ZoneCheckLimits,
        cancel: &'r Cancel,
    ) -> Self {
        let (scale, lower, upper) = scaled_bounds(timed);
        Self {
            timed,
            claim,
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
            steps: BTreeMap::new(),
        }
    }

    fn stats(&self) -> ZoneStats {
        ZoneStats {
            symbolic_states: widen(self.nodes.len()),
            retained_edges: widen(self.edges.len()),
            automaton_states: u64::from(self.claim.is_some()),
            zone_bytes: self.zone_bytes,
        }
    }

    pub(super) fn stop(&self, cause: ZoneStop) -> Halt {
        Halt::Outcome(Box::new(ZoneCheckOutcome::Stopped {
            cause,
            stats: self.stats(),
        }))
    }

    pub(super) fn undecided(&self, cause: ZoneUndecided) -> Halt {
        Halt::Outcome(Box::new(ZoneCheckOutcome::Undecided {
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

    fn scaled(&self, value: &Rational) -> BigInt {
        scaled(value, &self.scale)
    }

    /// Intersect `zone` with `constraints`.
    fn constrain(&self, zone: &mut Dbm, constraints: &[PlaceConstraint]) -> Result<(), DbmError> {
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
                self.settle_at(step.post(), &mut next, factor)?;
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

    fn search(&mut self) -> Result<ZoneCheckOutcome, Halt> {
        let initial = self.timed.initial();
        if initial.is_empty() {
            return Err(self.undecided(ZoneUndecided::NoInitialState));
        }
        if self.claim.is_some() && self.limits.max_automaton_states == 0 {
            return Err(
                self.stop(ZoneStop::LimitReached(ZoneCheckLimit::MaxAutomatonStates(
                    0,
                ))),
            );
        }
        let dim = self.timed.places().len() + 1;
        let mut queue = VecDeque::new();
        let mut roots = Vec::new();
        let mut delayed_deadlock = false;
        for (index, state) in initial.iter().enumerate() {
            let discrete = state.discrete().clone();
            let mut zone = Dbm::zero(dim);
            self.settle(&discrete, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            if let Some(node) = self.insert(discrete, zone, Some(index), None)? {
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
        // Vacuity (ADR-026 TD-6): an initial state with a time-divergent
        // behaviour.
        let mut admitted = false;
        for root in roots {
            if self.node_escape(root)?.diverges {
                admitted = true;
                break;
            }
        }
        match admitted {
            false => Err(self.undecided(ZoneUndecided::NoAdmittedBehaviour)),
            // STD-493: a deadlock reached only by waiting at a quiescent
            // state has no counterexample form (FR-236 gives `final_delay`
            // to time-locks only), so it is neither proved away nor refuted.
            true if delayed_deadlock => Err(self.undecided(ZoneUndecided::DeadlockAfterDelay)),
            true => Ok(ZoneCheckOutcome::Holds {
                stats: self.stats(),
                graph: SymbolicGraph {
                    scale: self.scale.clone(),
                    lower: self.lower.clone(),
                    upper: self.upper.clone(),
                    nodes: std::mem::take(&mut self.nodes),
                    edges: std::mem::take(&mut self.edges),
                },
            }),
        }
    }

    /// Store `(state, zone)` unless a stored state covers it: the new node,
    /// or `None` with the covering node recorded through `edge`.
    fn insert(
        &mut self,
        state: ModelSimState,
        zone: Dbm,
        initial: Option<usize>,
        edge: Option<(usize, usize, usize)>,
    ) -> Result<Option<usize>, Halt> {
        let (key, _) = self
            .timed
            .system()
            .state_key(&state)
            .map_err(|_| ZoneCheckRefusal::Fault("state-key"))?;
        if let Some(stored) = self.passed.get(&key) {
            for &candidate in stored {
                if zone.alu_covered_by(&self.nodes[candidate].zone, &self.lower, &self.upper)? {
                    if let Some((from, step, part)) = edge {
                        self.retain(SymbolicEdge::Covering {
                            from,
                            to: candidate,
                            step,
                            part,
                        })?;
                    }
                    return Ok(None);
                }
            }
        }
        if widen(self.nodes.len()) >= self.limits.max_symbolic_states {
            return Err(
                self.stop(ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(
                    self.limits.max_symbolic_states,
                ))),
            );
        }
        let bytes = zone_bytes(&zone);
        if self.zone_bytes.saturating_add(bytes) > self.limits.max_zone_bytes {
            return Err(
                self.stop(ZoneStop::LimitReached(ZoneCheckLimit::MaxZoneBytes(
                    self.limits.max_zone_bytes,
                ))),
            );
        }
        self.zone_bytes += bytes;
        let index = self.nodes.len();
        self.nodes.push(SymbolicNode {
            state,
            zone,
            initial,
            locked: false,
        });
        self.keys.push(key.clone());
        self.expanded.push(false);
        self.out.push(Vec::new());
        self.passed.entry(key).or_default().push(index);
        let parent = match edge {
            Some((from, step, part)) => {
                self.retain(SymbolicEdge::Step {
                    from,
                    to: index,
                    step,
                    part,
                })?;
                Some(self.edges.len() - 1)
            }
            None => None,
        };
        self.parent.push(parent);
        Ok(Some(index))
    }

    fn retain(&mut self, edge: SymbolicEdge) -> Result<(), Halt> {
        if widen(self.edges.len()) >= self.limits.max_symbolic_transitions {
            return Err(self.stop(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxSymbolicTransitions(self.limits.max_symbolic_transitions),
            )));
        }
        let (from, _) = edge.ends();
        if let Some(out) = self.out.get_mut(from) {
            out.push(self.edges.len());
        }
        self.edges.push(edge);
        Ok(())
    }

    /// The guarded steps of `node`'s model state, computed once.
    fn steps_of(&mut self, node: usize) -> Result<Vec<GuardedStep>, Halt> {
        let state = self.nodes[node].state.clone();
        self.steps_for(&state)
    }

    /// The guarded steps of `state`, computed once per model state.
    fn steps_for(&mut self, state: &ModelSimState) -> Result<Vec<GuardedStep>, Halt> {
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
        let scale = self.scale.clone();
        let mut created = Vec::new();
        for successor in self.successors(&state, &zone, &scale)? {
            if let Some(new) = self.insert(
                successor.state,
                successor.zone,
                None,
                Some((node, successor.step, successor.part)),
            )? {
                created.push(new);
            }
        }
        Ok(created)
    }

    /// Judge a new node: end the run on an admitted violation, mark a locked
    /// one. `true` when the node is a deadlock reached only after a delay.
    fn judge(&mut self, node: usize) -> Result<bool, Halt> {
        let badness = match self.badness(node)? {
            Some(Ok(badness)) => badness,
            Some(Err(())) => return Ok(true),
            None => return Ok(false),
        };
        let escape = self.node_escape(node)?;
        if !escape.diverges {
            // Every behaviour through it is Zeno: the violation holds
            // vacuously and the time-lock-freedom item reports it.
            self.nodes[node].locked = true;
            return Ok(false);
        }
        let counterexample = self.counterexample(node, badness, escape)?;
        Err(Halt::Outcome(Box::new(ZoneCheckOutcome::Violated {
            counterexample: Box::new(counterexample),
            stats: self.stats(),
        })))
    }

    /// FR-240's test from `node`.
    fn node_escape(&mut self, node: usize) -> Result<Escape, Halt> {
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
        let Some(claim) = &self.claim else {
            return self.deadlock(node);
        };
        let state = &self.nodes[node].state;
        for instance in &claim.instances {
            let mut meter = Meter::new(self.timed.system().meter_limits());
            let reading = claim
                .letters
                .atom(claim.atom, instance, state, &mut meter)
                .map_err(|stop| ZoneCheckRefusal::Atom(format!("{stop:?}")))?;
            match reading {
                AtomReading::True => {}
                AtomReading::False => return Ok(Some(Ok(Badness::False(instance.clone())))),
                AtomReading::Undefined(cause) => {
                    return Ok(Some(Ok(Badness::Undefined(instance.clone(), cause))))
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
        let mut enabling = Vec::new();
        for step in self.steps_of(node)? {
            let target = self.timed(self.timed.invariants(step.post()))?;
            // A reset clock's bound is constant over delays; a false one
            // disables the step.
            let reset_breaks = target.iter().any(|bound| {
                step.resets()
                    .iter()
                    .rev()
                    .find(|(place, _)| *place == bound.place())
                    .is_some_and(|(_, value)| !bound.op().holds(value, bound.bound()))
            });
            if reset_breaks {
                continue;
            }
            let reset: Vec<usize> = step.resets().iter().map(|(place, _)| *place).collect();
            let unreset: Vec<PlaceConstraint> = target
                .into_iter()
                .filter(|bound| !reset.contains(&bound.place()))
                .collect();
            for part in step.parts() {
                let mut zone = Dbm::universe(dim);
                self.constrain(&mut zone, part)?;
                self.constrain(&mut zone, &unreset)?;
                if !zone.is_empty() {
                    zone.down();
                    enabling.push(zone);
                }
            }
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

    /// The valuations `node` is entered at, before time elapses.
    fn entry_zone(&mut self, node: usize) -> Result<Dbm, Halt> {
        let dim = self.nodes[node].zone.dim();
        let state = self.nodes[node].state.clone();
        let invariants = self.timed(self.timed.invariants(&state))?;
        let mut zone = match self.parent[node].map(|edge| self.edges[edge].clone()) {
            None => Dbm::zero(dim),
            Some(SymbolicEdge::Step {
                from, step, part, ..
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
            Some(SymbolicEdge::Covering { .. }) => {
                return Err(ZoneCheckRefusal::Fault("covering-parent").into())
            }
        };
        self.constrain(&mut zone, &invariants)?;
        Ok(zone)
    }

    /// The valuations, at the entry of the first source of `path`, from
    /// which its model steps are taken, each after an admissible delay: the
    /// exact predecessors of every valuation of the quiescent state it ends
    /// at, all of which diverge.
    fn pre_path(&mut self, path: &[(ModelSimState, usize, usize)]) -> Result<Dbm, Halt> {
        let mut region = Dbm::universe(self.timed.places().len() + 1);
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
    fn point_diverges(&mut self, state: &ModelSimState, concrete: &Concrete) -> Result<bool, Halt> {
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
        let mut point = Dbm::universe(self.timed.places().len() + 1);
        for (place, value) in concrete.last_clocks.iter().enumerate() {
            let value = scaled(value, &factor);
            point.constrain(place + 1, 0, Bound::le(value.clone()))?;
            point.constrain(0, place + 1, Bound::le(-value))?;
        }
        Ok(self
            .escape(state, &point, &factor, &lower, &upper)?
            .diverges)
    }

    /// FR-241: the counterexample through the real edges to `node`.
    fn counterexample(
        &mut self,
        node: usize,
        badness: Badness,
        escape: Escape,
    ) -> Result<TemporalCounterexample, Halt> {
        let mut path = Vec::new();
        let mut at = node;
        while let Some(edge) = self.parent[at] {
            let SymbolicEdge::Step {
                from, step, part, ..
            } = self.edges[edge].clone()
            else {
                return Err(ZoneCheckRefusal::Fault("covering-parent").into());
            };
            let steps = self.steps_of(from)?;
            let step = steps
                .get(step)
                .cloned()
                .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
            path.push((from, step, part));
            at = from;
        }
        path.reverse();
        let initial = self.nodes[at]
            .initial
            .ok_or(ZoneCheckRefusal::Fault("path-root"))?;
        let mut stages = Vec::with_capacity(path.len());
        for (from, step, part) in path {
            let state = self.nodes[from].state.clone();
            stages.push(super::concretise::Stage {
                invariants: self.timed(self.timed.invariants(&state))?,
                elapses: self.elapses(&state)?,
                target: self.timed(self.timed.invariants(step.post()))?,
                part,
                step,
            });
        }
        // FR-241: the last valuation is one with a time-divergent
        // continuation, never a time-locked one. A deadlocked valuation of a
        // quiescent state diverges by its idle tail; otherwise the target is
        // the valuations from which the model steps to a quiescent state are
        // taken, and failing that the least-delay point, checked.
        let target = match &badness {
            Badness::Deadlock(zone) => Some(zone.clone()),
            Badness::False(_) | Badness::Undefined(..) => match &escape.to_quiescent {
                Some(path) => Some(self.pre_path(path)?),
                None => None,
            },
        };
        let within = target.and_then(|zone| {
            concretise(self.timed, &stages, &Target::Within(zone), &self.scale).ok()
        });
        let state = self.nodes[node].state.clone();
        let concrete = match within {
            Some(concrete) => concrete,
            None => {
                let concrete = concretise(self.timed, &stages, &Target::Any, &self.scale)
                    .map_err(|_| ZoneCheckRefusal::Fault("concretisation"))?;
                if !self.point_diverges(&state, &concrete)? {
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
                self.claim
                    .as_ref()
                    .map_or_else(Vec::new, |claim| claim.clause.fairness().to_vec()),
            ),
            Badness::Undefined(instance, cause) => {
                let claim = self
                    .claim
                    .as_ref()
                    .ok_or(ZoneCheckRefusal::Fault("undefined-claim"))?;
                (
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site: UndefinedSite {
                            position: widen(concrete.prefix.len()),
                            state_key: discrete_key,
                            locus: UndefinedLocus::Atom(claim.atom),
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
        Ok(TemporalCounterexample {
            initial_state: initial,
            prefix: concrete.prefix,
            over_binding,
            fairness,
            kind,
            lasso: None,
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
fn uncovered(zone: &Dbm, covers: &[Dbm]) -> Result<Vec<Dbm>, DbmError> {
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
    let factor = scale_constants(&constants).factor().clone();
    let max = |values: &[Rational]| values.iter().max().map(|value| scaled(value, &factor));
    let lower = timed
        .places()
        .iter()
        .map(|place| max(&lower[place.clock().index()]))
        .collect();
    let upper = timed
        .places()
        .iter()
        .map(|place| max(&upper[place.clock().index()]))
        .collect();
    (factor, lower, upper)
}

/// `value` in units of `1 / scale`; exact when `scale` is a multiple of its
/// denominator.
pub(super) fn scaled(value: &Rational, scale: &BigInt) -> BigInt {
    value.numerator().as_big() * scale / value.denominator().as_big()
}

/// The bytes a stored zone takes, by its bounds' magnitudes.
fn zone_bytes(zone: &Dbm) -> u64 {
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
