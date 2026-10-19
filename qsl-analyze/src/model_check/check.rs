// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1, first phase): [`check_model`] decides a reachable
//! invariant (TP-1) or the deadlock-freedom item over every behaviour of a
//! model subject.
//!
//! Both items are safety forms: `always holds(I)` and `always holds(not
//! deadlocked)`. Each gets the deterministic bad-prefix monitor of its
//! formula (ADR-018 SM-6), built in `qsl-eval`'s automaton kernel, and the
//! engine explores the product of the subject's state graph with it
//! breadth-first in FR-101's canonical order: initial states by ascending
//! state-key bytes, successors by transition-identity bytes then post-state
//! key bytes. The first product state in that order whose letter is
//! undefined, whose monitor state rejects, whose expansion evaluates a
//! contract conjunction undefined, or (for the deadlock-freedom item) that
//! is deadlocked ends the run with the canonical path to it (ADR-018 UE-4,
//! DL-4, DL-7). A product state's letter is read when the state is taken
//! from the frontier, which is breadth-first discovery order.
//!
//! A run proves an item only when its frontier empties with no state bad:
//! every reachable product state was examined. A run that reaches its
//! horizon with a state left unexpanded settles V-5; a run that reaches a
//! limit or is cancelled settles V-7; neither is ever `Holds`.
//!
//! The liveness phase (TP-4), the bounded and interval monitors (TP-2,
//! TP-3) and proof certificates are not part of this module yet: a clause
//! of another property form is refused before any state is explored.

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::automaton::{Atom, AutomatonError, GbaBuilder, Guard, Letter, Monitor, StateId};
use qsl_eval::model_subject::{ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    check_model_bounds, ModelFinding, ModelSimState, ModelSystem, TerminalClass,
};
use qsl_eval::simulation::{
    EncodingRefusal, ExpansionLimit, ExpansionStop, NotSimulated, RequiresBound, TransitionSystem,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::InternalFault;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ModelStep, ReachedLimit, RunStats, TemporalCounterexample, UndecidedCause, UndefinedEvaluation,
    UndefinedLocus, UndefinedSite,
};
use qsl_semantics::check::{CheckedTemporalClause, FormulaNode, FormulaNodeId};
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{Cancel, Meter, ObjectReference};
use quire_semantic_value::fairness::{FairnessConstraint, PropertyForm};
use quire_semantic_value::model_transition::ModelTransition;

/// The type positions the requires-bound pre-check may visit.
const BOUND_POSITIONS: u64 = 1 << 20;

/// What an EN-1 run decides (FR-126 Inputs).
pub enum ModelCheckItem<'a> {
    /// An authored clause with its property form, its atoms read through
    /// `letters`.
    Clause {
        /// The checked clause.
        clause: &'a CheckedTemporalClause,
        /// The seam to the one clause evaluator for its `holds` atoms.
        letters: &'a dyn ModelLetters,
    },
    /// FR-124's derived deadlock-freedom item.
    DeadlockFreedom,
}

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
    #[error(transparent)]
    NotSimulated(Box<NotSimulated>),
    /// The clause's property form is not one this engine decides yet.
    #[error("EN-1 does not decide the property form {form:?} yet")]
    FormNotDecided {
        /// The clause's property form.
        form: Option<PropertyForm>,
    },
    /// The clause binds more instances than one letter holds atoms.
    #[error("the clause binds {count} instances; a letter holds at most {max}")]
    TooManyBindings {
        /// The number of instances.
        count: usize,
        /// The atoms one letter holds.
        max: usize,
    },
    /// A state or transition identity has no canonical encoding.
    #[error(transparent)]
    Encoding(#[from] EncodingRefusal),
    /// An invariant of the engine or the subject broke.
    #[error("model check broke invariant {0:?}")]
    Fault(InternalFault),
}

/// The item as the run reads it.
enum Item<'a> {
    Invariant {
        atom: FormulaNodeId,
        bindings: Vec<ObjectReference>,
        letters: &'a dyn ModelLetters,
        fairness: Vec<FairnessConstraint>,
    },
    Deadlock,
}

/// One product state: its model state, key, depth, the step that first
/// reached it and the monitor state it was reached from.
struct Node {
    state: ModelSimState,
    digest: DigestRecord,
    depth: usize,
    parent: Option<(usize, ModelTransition)>,
    initial_index: usize,
    before: StateId,
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
    binding: Option<ObjectReference>,
}

impl Finish {
    fn violated(kind: CounterexampleKind, at: usize, binding: Option<ObjectReference>) -> Self {
        Self::Violated(Box::new(Violation { kind, at, binding }))
    }
}

type Step<T> = Result<T, Halt>;

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

fn fault(invariant: &'static str) -> ModelCheckRefusal {
    ModelCheckRefusal::Fault(InternalFault::new("model-check", invariant))
}

fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// FR-126: decide `request`'s item over every behaviour of its subject,
/// polling `cancel` before every expansion and every atom evaluation.
///
/// # Errors
///
/// [`ModelCheckRefusal`] when the subject does not admit, a root is
/// unbounded, the item is not a TP-1 clause or the deadlock-freedom item,
/// or an identity cannot be encoded; no outcome is settled then.
pub fn check_model(
    request: ModelCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let item = match request.item {
        ModelCheckItem::Clause { clause, letters } => {
            let form = clause.property_form();
            let atom = reachable_invariant_atom(clause)
                .ok_or(ModelCheckRefusal::FormNotDecided { form })?;
            let bindings = letters.bindings();
            if bindings.len() > qsl_eval::automaton::MAX_ATOMS {
                return Err(ModelCheckRefusal::TooManyBindings {
                    count: bindings.len(),
                    max: qsl_eval::automaton::MAX_ATOMS,
                });
            }
            Item::Invariant {
                atom,
                bindings,
                letters,
                fairness: clause.fairness().to_vec(),
            }
        }
        ModelCheckItem::DeadlockFreedom => Item::Deadlock,
    };
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(|refusal| match refusal {
        NotSimulated::RequiresBound(roots) => ModelCheckRefusal::RequiresBound(roots),
        other => ModelCheckRefusal::NotSimulated(Box::new(other)),
    })?;
    let mut run = Run {
        system: &system,
        item: &item,
        limits: request.limits,
        max_depth: request.max_depth,
        meter,
        max_candidates,
        cancel,
        nodes: Vec::new(),
        edges: 0,
        horizon_cut: false,
        monitor: monitor(&item, request.limits.max_automaton_states)
            .map_err(|_| fault("monitor-builds"))?,
    };
    if no_initial {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats: run.stats(),
        });
    }
    match run.explore() {
        Ok(()) => Ok(if run.horizon_cut {
            ModelCheckOutcome::BoundReached {
                depth: widen(run.max_depth.unwrap_or(0)),
                stats: run.stats(),
            }
        } else {
            ModelCheckOutcome::Holds { stats: run.stats() }
        }),
        Err(Halt::Refusal(refusal)) => Err(refusal),
        Err(Halt::Finish(Finish::Undecided(cause))) => Ok(ModelCheckOutcome::Undecided {
            cause,
            stats: run.stats(),
        }),
        Err(Halt::Finish(Finish::Stopped(cause, limit))) => Ok(ModelCheckOutcome::Stopped {
            cause,
            limit,
            stats: run.stats(),
        }),
        Err(Halt::Finish(Finish::Violated(violation))) => {
            let Violation { kind, at, binding } = *violation;
            let counterexample = run.counterexample(kind, at, binding);
            Ok(ModelCheckOutcome::Violated {
                counterexample: Box::new(counterexample),
                stats: run.stats(),
            })
        }
    }
}

/// The `holds` atom of a TP-1 clause, `always holds(I)`.
fn reachable_invariant_atom(clause: &CheckedTemporalClause) -> Option<FormulaNodeId> {
    if clause.property_form() != Some(PropertyForm::ReachableInvariant) {
        return None;
    }
    let formula = clause.formula();
    let FormulaNode::Unary { operand, .. } = formula.nodes().get(formula.root().index())? else {
        return None;
    };
    matches!(
        formula.nodes().get(operand.index()),
        Some(FormulaNode::Holds { .. })
    )
    .then_some(*operand)
}

/// The bad-prefix monitor of `always (a_0 and ... and a_{n-1})`: one atom
/// per binding of a TP-1 clause, one (`not deadlocked`) for the
/// deadlock-freedom item. Its initial state reads position 0's letter.
fn monitor(item: &Item<'_>, max_states: u64) -> Result<Monitor, AutomatonError> {
    let atoms = match item {
        Item::Invariant { bindings, .. } => bindings.len(),
        Item::Deadlock => 1,
    };
    let mut guard = Guard::always();
    for index in 0..atoms {
        guard = guard.requiring(Atom::new(index)?);
    }
    let mut builder = GbaBuilder::new();
    let holding = builder.add_state();
    builder.add_transition(qsl_eval::automaton::Gba::INITIAL, guard, holding)?;
    builder.add_transition(holding, guard, holding)?;
    Monitor::bad_prefix(&builder.build(), max_states)
}

struct Run<'r, 'a> {
    system: &'r ModelSystem<'a>,
    item: &'r Item<'r>,
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    meter: quire_exact::ScalarLimits,
    max_candidates: u64,
    cancel: &'r Cancel,
    nodes: Vec<Node>,
    edges: usize,
    horizon_cut: bool,
    monitor: Monitor,
}

impl Run<'_, '_> {
    fn stats(&self) -> RunStats {
        RunStats {
            product_states: widen(self.nodes.len()),
            product_edges: widen(self.edges),
            automaton_states: widen(self.monitor.state_count()),
            depth: widen(self.nodes.iter().map(|node| node.depth).max().unwrap_or(0)),
        }
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

    fn automaton(&self, error: AutomatonError) -> Halt {
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

    /// The breadth-first first phase.
    fn explore(&mut self) -> Step<()> {
        self.poll()?;
        let before = self
            .monitor
            .initial()
            .map_err(|error| self.automaton(error))?;
        let mut initial = Vec::new();
        for (index, state) in self.system.initial().into_iter().enumerate() {
            let (key, digest) = self.system.state_key(&state)?;
            initial.push((key, index, digest, state));
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
        let mut known = BTreeMap::new();
        let mut queue = VecDeque::new();
        for (key, index, digest, state) in initial {
            known.insert(key, self.nodes.len());
            queue.push_back(self.nodes.len());
            self.nodes.push(Node {
                state,
                digest,
                depth: 0,
                parent: None,
                initial_index: index,
                before,
            });
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            let mut after = self.invariant_letter(at)?;
            let expansion = match self.system.successors(&self.nodes[at].state) {
                Ok(expansion) => expansion,
                Err(stop) => return Err(self.expansion_stop(stop)),
            };
            self.contracts(at, &expansion.findings)?;
            if matches!(self.item, Item::Deadlock) {
                after = self.deadlock_letter(at, !expansion.successors.is_empty())?;
            }
            let depth = self.nodes[at].depth;
            if self.max_depth.is_some_and(|horizon| depth >= horizon) {
                self.horizon_cut |= !expansion.successors.is_empty();
                continue;
            }
            let mut successors = Vec::with_capacity(expansion.successors.len());
            for (transition, state) in expansion.successors {
                let order = self.system.transition_key(&transition)?;
                let (key, digest) = self.system.state_key(&state)?;
                successors.push((order, key, digest, transition, state));
            }
            successors.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
            for (_, key, digest, transition, state) in successors {
                if self.edges >= self.limits.limits.max_transitions {
                    return Err(Self::stopped(
                        ModelCheckLimit::MaxTransitions,
                        widen(self.limits.limits.max_transitions),
                    )
                    .into());
                }
                self.edges += 1;
                if known.contains_key(&key) {
                    continue;
                }
                if self.nodes.len() >= self.limits.limits.max_states {
                    return Err(Self::stopped(
                        ModelCheckLimit::MaxStates,
                        widen(self.limits.limits.max_states),
                    )
                    .into());
                }
                known.insert(key, self.nodes.len());
                queue.push_back(self.nodes.len());
                let initial_index = self.nodes[at].initial_index;
                self.nodes.push(Node {
                    state,
                    digest,
                    depth: depth.saturating_add(1),
                    parent: Some((at, transition)),
                    initial_index,
                    before: after,
                });
            }
        }
        Ok(())
    }

    /// Step the monitor from node `at`'s predecessor state on `letter`; a
    /// rejecting state ends the run with `kind`.
    fn step(
        &mut self,
        at: usize,
        letter: Letter,
        kind: CounterexampleKind,
        binding: Option<ObjectReference>,
    ) -> Step<StateId> {
        let after = self
            .monitor
            .step(self.nodes[at].before, letter)
            .map_err(|error| self.automaton(error))?;
        if self
            .monitor
            .is_rejecting(after)
            .map_err(|error| self.automaton(error))?
        {
            return Err(Finish::violated(kind, at, binding).into());
        }
        Ok(after)
    }

    /// A TP-1 item's letter at node `at`: each binding's atom, in binding
    /// order, read before the state is expanded; the monitor state the
    /// node's successors are reached from. The deadlock-freedom item reads
    /// its letter from the expansion ([`Self::deadlock_letter`]), so here
    /// it leaves the predecessor state unchanged.
    fn invariant_letter(&mut self, at: usize) -> Step<StateId> {
        let Item::Invariant {
            atom,
            bindings,
            letters,
            ..
        } = self.item
        else {
            return Ok(self.nodes[at].before);
        };
        let mut letter = Letter::default();
        let mut first_false = None;
        for (index, binding) in bindings.iter().enumerate() {
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let reading = letters
                .atom(*atom, binding, &self.nodes[at].state, &mut meter)
                .map_err(|_| {
                    Self::stopped(ModelCheckLimit::EvaluationMeter, self.meter.work_units)
                })?;
            match reading {
                AtomReading::True => {
                    let atom = Atom::new(index).map_err(|_| fault("binding-atom-in-range"))?;
                    letter = letter.with(atom);
                }
                AtomReading::False => {
                    first_false.get_or_insert(binding.clone());
                }
                AtomReading::Undefined(cause) => {
                    let site = self.site(at, UndefinedLocus::Atom(*atom), None);
                    return Err(Finish::violated(
                        CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                            site,
                            cause,
                        }),
                        at,
                        Some(binding.clone()),
                    )
                    .into());
                }
            }
        }
        self.step(at, letter, CounterexampleKind::Formula, first_false)
    }

    fn site(
        &self,
        at: usize,
        locus: UndefinedLocus,
        transition: Option<ModelTransition>,
    ) -> UndefinedSite {
        UndefinedSite {
            position: widen(self.nodes[at].depth),
            state_key: self.nodes[at].digest,
            locus,
            transition,
        }
    }

    /// ADR-018 UE-6 and V-6: the first contract conjunction the expansion
    /// evaluated undefined ends the run with a violation; otherwise a
    /// refused one leaves it undecided.
    fn contracts(&self, at: usize, findings: &[ModelFinding]) -> Step<()> {
        if let Some(contract) = findings.iter().find_map(ModelFinding::undefined_contract) {
            let site = self.site(
                at,
                UndefinedLocus::Contract(contract.clause),
                contract.guard_of,
            );
            return Err(Finish::violated(
                CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                    site,
                    cause: contract.cause,
                }),
                at,
                self.binding(),
            )
            .into());
        }
        if findings.iter().any(ModelFinding::is_refused_contract) {
            return Err(Finish::Undecided(UndecidedCause::UndecidedSuccessor).into());
        }
        Ok(())
    }

    /// The `over` binding a counterexample that is not a formula violation
    /// names: the first instance of a TP-1 clause, none for the
    /// deadlock-freedom item.
    fn binding(&self) -> Option<ObjectReference> {
        match self.item {
            Item::Invariant { bindings, .. } => bindings.first().cloned(),
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
                let site = self.site(at, UndefinedLocus::Terminal(model), None);
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
        self.step(at, letter, CounterexampleKind::Deadlock, None)
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

    /// The canonical path to node `at`: its breadth-first tree path.
    fn counterexample(
        &self,
        kind: CounterexampleKind,
        at: usize,
        over_binding: Option<ObjectReference>,
    ) -> TemporalCounterexample {
        let mut prefix = Vec::new();
        let mut cursor = at;
        while let Some((parent, transition)) = &self.nodes[cursor].parent {
            prefix.push(ModelStep {
                transition: transition.clone(),
                post_state: self.nodes[cursor].digest,
            });
            cursor = *parent;
        }
        prefix.reverse();
        let fairness = match self.item {
            Item::Invariant { fairness, .. } => fairness.clone(),
            Item::Deadlock => Vec::new(),
        };
        TemporalCounterexample {
            initial_state: self.nodes[cursor].initial_index,
            prefix,
            over_binding,
            fairness,
            kind,
        }
    }
}
