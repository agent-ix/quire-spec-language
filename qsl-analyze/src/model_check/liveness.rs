// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1) for a liveness (TP-4) clause: the product of the
//! subject's state graph with the generalized Büchi automaton of the
//! clause's negation, retained edge by edge, and the second phase over it.
//!
//! One binding of the clause's `over` parameter is one product. Each is a
//! FR-101 [`TransitionSystem`] explored by EN-1's own [`retain`] in FR-101's
//! canonical order: its state is (model state, automaton state), its key is
//! (FR-101 state key, automaton state index), and under infinite-trace a
//! terminal model state continues along its stutter edge (ADR-018 SM-4).
//! A position's letter is read when the product state for it is created,
//! before the automaton steps on it (UE-4).
//!
//! The first phase is a scan of the retained states in discovery order for
//! the first whose letter is undefined, whose expansion evaluated a contract
//! conjunction undefined (UE-6), or whose expansion left one refused (V-6).
//! A property automaton with no move on a letter goes to a dead state, so a
//! behaviour that cannot violate the clause is still explored and still has
//! its letters and contracts examined, and no dead state is ever accepting.
//!
//! The second phase runs only when no binding's first phase ended the run.
//! It decomposes the retained graph into strongly connected components in
//! `qsl-eval`'s fairness kernels, passes a component that is non-trivial,
//! holds a state of every acceptance set and satisfies every weak
//! constraint (FA-4), and builds the canonical lasso (CX-5): the stem is the
//! breadth-first tree path to the first passing component's first state and
//! the loop is the greedy walk inside it. Enabledness is read from each
//! model state's own successor relation, never from product edges (FA-2).
//!
//! A product graph whose second phase finds no passing component proves the
//! item only with FR-339's component certificate, which is not built yet:
//! that run is refused ([`LivenessRefusal::ComponentCertificateNotBuilt`])
//! rather than returned as a closure proof, which does not check acceptance.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::automaton::{
    translate, Atom, AutomatonError, AutomatonKind, Letter, PropertyAutomaton, StateId,
};
use qsl_eval::fairness::{
    analyze, Analysis, Constraint, Edge, EdgeLabel, FairnessError, Graph, Identity, Vertex,
};
use qsl_eval::model_subject::ModelLetters;
use qsl_eval::simulation::model::{
    check_model_bounds, ModelFinding, ModelSimState, ModelSystem, ModelSystem as System,
};
use qsl_eval::simulation::{
    EncodingRefusal, Expansion, ExpansionLimit, ExpansionStop, Limit, NotSimulated, Outcome,
    TransitionSystem,
};
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::{CatalogCode, IdentityLimits, InternalFault};
use qsl_replay::{
    CounterexampleKind, IncompleteCause, Lasso, ModelCheckItem, ModelCheckLimit, ModelCheckOutcome,
    ModelStep, ReachedLimit, RunStats, TemporalCounterexample, UndecidedCause, UndefinedEvaluation,
    UndefinedLocus, UndefinedSite,
};
use qsl_semantics::check::{Activation, CheckedTemporalClause, FormulaNodeId};
use quire_canonical::{Encode, Sink, Writer};
use quire_exact::{Cancel, CancelCause, Meter, ObjectReference, ScalarLimits};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{ModelCheckRefusal, ModelCheckRequest};
use super::graph::{retain, ProductStateId, RetainedEdge, RetainedGraph};
use super::product::{ProductKey, ProductTransition};

/// The type positions the requires-bound pre-check may visit.
const BOUND_POSITIONS: u64 = 1 << 20;

/// `resource_exhausted`/`insufficient-next-charge`, the code of an expansion
/// that stopped; [`Halt`] records which budget it was.
const STOPPED: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");

/// Why EN-1 refused a liveness clause, or could not settle one it explored.
#[derive(Debug, thiserror::Error)]
pub enum LivenessRefusal {
    /// The fairness set holds a strong constraint, whose filter (ADR-019
    /// FS-2) and counterexample obligations this engine does not decide yet.
    #[error("EN-1 decides weak fairness only; the clause holds a strong fairness constraint")]
    StrongFairness,
    /// A guarded `on each` activation reads a `when` condition no letter
    /// seam supplies yet.
    #[error("EN-1 does not read the `when` condition of a guarded `on each` activation yet")]
    GuardedActivation,
    /// The clause has no property automaton.
    #[error(transparent)]
    Automaton(#[from] AutomatonError),
    /// A fairness constraint names an operation the subject does not admit.
    #[error("the fairness constraint on `{name}` names an operation the subject does not declare")]
    UnknownOperation {
        /// The operation's resolved name.
        name: String,
    },
    /// No fair accepting cycle exists in the fully explored product, which
    /// proves the item only with FR-339's component certificate, and that
    /// certificate and its core checker are not built yet.
    #[error("the item holds over {} product states, but FR-339's component certificate is not built yet", .stats.product_states)]
    ComponentCertificateNotBuilt {
        /// The run's counts.
        stats: RunStats,
    },
}

/// Decide a TP-4 clause (FR-126 first and second phase).
///
/// # Errors
///
/// [`ModelCheckRefusal`] as [`check_model`](super::check_model) returns it,
/// and [`LivenessRefusal`] for a clause this engine does not decide yet.
pub(super) fn check(
    request: ModelCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let ModelCheckItem::Clause { clause, letters } = request.item else {
        return Err(fault("a-liveness-item-is-a-clause"));
    };
    if clause
        .fairness()
        .iter()
        .any(|constraint| constraint.kind() == FairnessKind::Strong)
    {
        return Err(LivenessRefusal::StrongFairness.into());
    }
    if matches!(clause.activation(), Activation::Each { guarded: true }) {
        return Err(LivenessRefusal::GuardedActivation.into());
    }
    let bindings = letters.bindings();
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(|refusal| match refusal {
        NotSimulated::RequiresBound(roots) => ModelCheckRefusal::RequiresBound(roots),
        NotSimulated::Extent(failure) => ModelCheckRefusal::Extent(Box::new(failure)),
        _ => fault("bounds-precheck-returns-only-requires-bound-or-extent"),
    })?;
    let totals = Totals::default();
    if cancel.poll() {
        if let Some(source) = cancel.cause() {
            return Ok(stopped(
                IncompleteCause::Cancelled { source },
                None,
                totals.stats(),
            ));
        }
    }
    if no_initial {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats: totals.stats(),
        });
    }
    if bindings.is_empty() {
        return Ok(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            stats: totals.stats(),
        });
    }
    let run = Run {
        system: &system,
        clause,
        letters,
        bindings: &bindings,
        request: RunRequest {
            max_depth: request.max_depth,
            limits: request.limits,
            meter,
            max_candidates,
        },
        cancel,
    };
    run.decide(totals)
}

fn fault(invariant: &'static str) -> ModelCheckRefusal {
    ModelCheckRefusal::Fault(InternalFault::new("model-check", invariant))
}

fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn stopped(
    cause: IncompleteCause,
    limit: Option<ReachedLimit>,
    stats: RunStats,
) -> ModelCheckOutcome {
    ModelCheckOutcome::Stopped {
        cause,
        limit,
        stats,
    }
}

fn exhausted(limit: ModelCheckLimit, value: u64, stats: RunStats) -> ModelCheckOutcome {
    stopped(
        IncompleteCause::ResourceExhausted,
        Some(ReachedLimit { limit, value }),
        stats,
    )
}

/// The counts of the products explored so far.
#[derive(Clone, Copy, Debug, Default)]
struct Totals {
    states: u64,
    edges: u64,
    automaton_states: u64,
    depth: u64,
}

impl Totals {
    fn stats(&self) -> RunStats {
        RunStats {
            product_states: self.states,
            product_edges: self.edges,
            automaton_states: self.automaton_states,
            depth: self.depth,
        }
    }

    fn add(&mut self, graph: &RetainedGraph<LState, PT, Finding>, edges: usize, automaton: u64) {
        self.states = self.states.saturating_add(widen(graph.states().len()));
        self.edges = self.edges.saturating_add(widen(edges));
        self.automaton_states = self.automaton_states.saturating_add(automaton);
        let depth = graph
            .states()
            .iter()
            .map(|state| state.depth)
            .max()
            .unwrap_or(0);
        self.depth = self.depth.max(widen(depth));
    }
}

/// What a run needs of the request beyond the subject and the item.
struct RunRequest {
    max_depth: Option<usize>,
    limits: qsl_replay::ModelCheckLimits,
    meter: ScalarLimits,
    max_candidates: u64,
}

/// One position's letter for one binding, or the first atom that is undefined
/// there.
#[derive(Clone, Debug)]
enum Reading {
    Defined(Letter),
    Undefined {
        atom: FormulaNodeId,
        cause: UndefinedCause,
    },
}

/// The property component of a product state: where the automaton is after
/// reading this position's letter.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Property {
    /// A state of the negation automaton.
    After(StateId),
    /// The automaton has no move: no violating run goes through here.
    Dead,
    /// The letter at this position is undefined.
    Undefined {
        atom: FormulaNodeId,
        cause: UndefinedCause,
    },
}

/// A product state's canonical property key.
enum PropertyKey {
    After(usize),
    Dead,
    Undefined,
}

impl Encode for PropertyKey {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        match self {
            Self::After(index) => {
                writer.string("after")?;
                writer.integer(i128::try_from(*index).unwrap_or(i128::MAX))?;
            }
            Self::Dead => writer.string("dead")?,
            Self::Undefined => writer.string("undefined")?,
        }
        writer.end_array()
    }
}

/// A product state: a model state and the property state it was reached in,
/// with the depth it was created at.
#[derive(Clone)]
struct LState {
    model: ModelSimState,
    property: Property,
    depth: usize,
}

type PT = ProductTransition<ModelTransition>;

/// What an expansion found, beside its successors.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Finding {
    /// A contract conjunction the model's expansion evaluated undefined or
    /// refused.
    Contract(ModelFinding),
    /// The transition identities the model state enables (FA-2).
    Enabled(Vec<ModelTransition>),
    /// The state is at the search horizon and has an edge that was not taken.
    HorizonCut,
}

/// Why a product's exploration ended before its frontier emptied.
enum Halt {
    /// A model expansion stopped.
    Expansion(ExpansionStop),
    /// An atom's evaluation exhausted the meter.
    Meter,
    /// The caller cancelled the run.
    Cancelled(CancelCause),
    /// The automaton reached `max_automaton_states`.
    Automaton(u64),
    /// An identity has no canonical encoding, or an invariant broke.
    Refusal(ModelCheckRefusal),
}

/// Where the automaton is before a position is read.
#[derive(Clone, Copy)]
enum Before {
    Start,
    At(StateId),
    Dead,
}

/// One binding's product as a `TransitionSystem`.
struct LassoSystem<'r, 'a> {
    system: &'r System<'a>,
    letters: &'r dyn ModelLetters,
    binding: &'r ObjectReference,
    atoms: Vec<FormulaNodeId>,
    automaton: RefCell<PropertyAutomaton>,
    start: StateId,
    readings: RefCell<BTreeMap<DigestRecord, Reading>>,
    initial: Vec<LState>,
    meter: ScalarLimits,
    cancel: &'r Cancel,
    max_depth: Option<usize>,
    halt: RefCell<Option<Halt>>,
}

impl LassoSystem<'_, '_> {
    /// Record why the run stops, keeping the first reason, and return the
    /// stop the exploration engine takes.
    fn record(&self, halt: Halt) -> ExpansionStop {
        let mut slot = self.halt.borrow_mut();
        if slot.is_none() {
            *slot = Some(halt);
        }
        ExpansionStop {
            cause: STOPPED,
            limit: None,
        }
    }

    fn poll(&self) -> Result<(), ExpansionStop> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(self.record(Halt::Cancelled(source)));
            }
        }
        Ok(())
    }

    /// The letter of `state` for this binding, read once per model state.
    fn reading(&self, state: &ModelSimState) -> Result<Reading, ExpansionStop> {
        let (_, digest) = self
            .system
            .state_key(state)
            .map_err(|refusal| self.record(Halt::Refusal(ModelCheckRefusal::Encoding(refusal))))?;
        if let Some(known) = self.readings.borrow().get(&digest) {
            return Ok(known.clone());
        }
        let mut letter = Letter::default();
        let mut reading = None;
        for (index, atom) in self.atoms.iter().enumerate() {
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let value = self
                .letters
                .atom(*atom, self.binding, state, &mut meter)
                .map_err(|_| self.record(Halt::Meter))?;
            match value {
                AtomReading::True => {
                    let atom = Atom::new(index)
                        .map_err(|_| self.record(Halt::Refusal(fault("atom-in-range"))))?;
                    letter = letter.with(atom);
                }
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    reading = Some(Reading::Undefined { atom: *atom, cause });
                    break;
                }
            }
        }
        let reading = reading.unwrap_or(Reading::Defined(letter));
        self.readings.borrow_mut().insert(digest, reading.clone());
        Ok(reading)
    }

    /// The property states a position reads into, from `before`.
    fn step(&self, before: Before, reading: &Reading) -> Result<Vec<Property>, ExpansionStop> {
        let letter = match reading {
            Reading::Undefined { atom, cause } => {
                return Ok(vec![Property::Undefined {
                    atom: *atom,
                    cause: cause.clone(),
                }])
            }
            Reading::Defined(letter) => *letter,
        };
        let from = match before {
            Before::Dead => return Ok(vec![Property::Dead]),
            Before::Start => self.start,
            Before::At(state) => state,
        };
        let targets = self
            .automaton
            .borrow_mut()
            .successors(from, letter)
            .map_err(|error| match error {
                AutomatonError::StateLimit { limit } => self.record(Halt::Automaton(limit)),
                other => self.record(Halt::Refusal(LivenessRefusal::Automaton(other).into())),
            })?;
        Ok(if targets.is_empty() {
            vec![Property::Dead]
        } else {
            targets.into_iter().map(Property::After).collect()
        })
    }

    /// The initial product states: each initial model state paired with each
    /// property state its letter reads into.
    fn start_states(&mut self) -> Result<(), ExpansionStop> {
        let mut initial = Vec::new();
        for model in self.system.initial() {
            let reading = self.reading(&model)?;
            for property in self.step(Before::Start, &reading)? {
                initial.push(LState {
                    model: model.clone(),
                    property,
                    depth: 0,
                });
            }
        }
        self.initial = initial;
        Ok(())
    }
}

impl<'a> TransitionSystem for LassoSystem<'_, 'a> {
    type State = LState;
    type TransitionId = PT;
    type Key = ProductKey<<System<'a> as TransitionSystem>::Key, PropertyKey>;
    type Finding = Finding;

    fn initial(&self) -> Vec<LState> {
        self.initial.clone()
    }

    fn key(&self, state: &LState) -> Self::Key {
        ProductKey {
            model: self.system.key(&state.model),
            property: match &state.property {
                Property::After(state) => PropertyKey::After(state.index()),
                Property::Dead => PropertyKey::Dead,
                Property::Undefined { .. } => PropertyKey::Undefined,
            },
        }
    }

    fn successors(&self, state: &LState) -> Result<Expansion<PT, LState, Finding>, ExpansionStop> {
        self.poll()?;
        let before = match &state.property {
            // The first phase ends at this state; nothing follows it.
            Property::Undefined { .. } => {
                return Ok(Expansion {
                    successors: Vec::new(),
                    findings: Vec::new(),
                })
            }
            Property::Dead => Before::Dead,
            Property::After(at) => Before::At(*at),
        };
        let expansion = self.system.successors(&state.model).inspect_err(|&stop| {
            self.record(Halt::Expansion(stop));
        })?;
        let mut findings: Vec<Finding> = expansion
            .findings
            .into_iter()
            .filter(|finding| {
                finding.undefined_contract().is_some() || finding.is_refused_contract()
            })
            .map(Finding::Contract)
            .collect();
        let mut enabled: Vec<ModelTransition> = expansion
            .successors
            .iter()
            .map(|(transition, _)| transition.clone())
            .collect();
        enabled.sort();
        enabled.dedup();
        findings.push(Finding::Enabled(enabled));
        if self.max_depth.is_some_and(|horizon| state.depth >= horizon) {
            // Every product state has an edge under infinite-trace: a
            // terminal model state steps by its stutter edge.
            findings.push(Finding::HorizonCut);
            return Ok(Expansion {
                successors: Vec::new(),
                findings,
            });
        }
        let steps: Vec<(PT, ModelSimState)> = if expansion.successors.is_empty() {
            vec![(ProductTransition::TerminalStutter, state.model.clone())]
        } else {
            expansion
                .successors
                .into_iter()
                .map(|(transition, model)| (ProductTransition::Model(transition), model))
                .collect()
        };
        let mut successors = Vec::new();
        for (transition, model) in steps {
            let reading = self.reading(&model)?;
            for property in self.step(before, &reading)? {
                successors.push((
                    transition.clone(),
                    LState {
                        model: model.clone(),
                        property,
                        depth: state.depth.saturating_add(1),
                    },
                ));
            }
        }
        Ok(Expansion {
            successors,
            findings,
        })
    }
}

/// What one product's exploration ended with, beyond its graph.
enum Ending {
    /// The frontier emptied.
    Complete,
    /// A limit or a cancellation stopped the run.
    Stopped(IncompleteCause, Option<ReachedLimit>),
}

/// One explored binding.
struct Explored<'r, 'a> {
    product: LassoSystem<'r, 'a>,
    graph: RetainedGraph<LState, PT, Finding>,
    binding: &'r ObjectReference,
    ending: Ending,
}

/// What a scan of one graph's states found.
enum FirstPhase {
    Clear,
    Violated(Box<Violation>),
    Undecided,
}

struct Violation {
    state: ProductStateId,
    kind: CounterexampleKind,
}

/// What the second phase found in one graph.
enum Second {
    Lasso(Box<TemporalCounterexample>),
    Cancelled(CancelCause),
    NoneFair,
}

/// The run: the subject's system, the item and the request.
struct Run<'r, 'a> {
    system: &'r System<'a>,
    clause: &'r CheckedTemporalClause,
    letters: &'r dyn ModelLetters,
    bindings: &'r [ObjectReference],
    request: RunRequest,
    cancel: &'r Cancel,
}

impl<'r, 'a> Run<'r, 'a> {
    fn decide(&self, mut totals: Totals) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let mut explored = Vec::with_capacity(self.bindings.len());
        for binding in self.bindings {
            let run = match self.explore(binding, &mut totals)? {
                Ok(run) => run,
                Err(outcome) => return Ok(outcome),
            };
            match self.first_phase(&run)? {
                FirstPhase::Clear => {}
                FirstPhase::Undecided => {
                    return Ok(ModelCheckOutcome::Undecided {
                        cause: UndecidedCause::UndecidedSuccessor,
                        stats: totals.stats(),
                    })
                }
                FirstPhase::Violated(violation) => {
                    let Violation { state, kind } = *violation;
                    let counterexample = self.prefix_counterexample(&run, state, kind)?;
                    return Ok(ModelCheckOutcome::Violated {
                        counterexample: Box::new(counterexample),
                        stats: totals.stats(),
                    });
                }
            }
            if let Ending::Stopped(cause, limit) = run.ending {
                return Ok(stopped(cause, limit, totals.stats()));
            }
            explored.push(run);
        }
        let mut cut = false;
        for run in &explored {
            cut |= run
                .graph
                .states()
                .iter()
                .any(|state| state.findings.contains(&Finding::HorizonCut));
            match self.second_phase(run)? {
                Second::Lasso(counterexample) => {
                    return Ok(ModelCheckOutcome::Violated {
                        counterexample,
                        stats: totals.stats(),
                    })
                }
                Second::Cancelled(source) => {
                    return Ok(stopped(
                        IncompleteCause::Cancelled { source },
                        None,
                        totals.stats(),
                    ))
                }
                Second::NoneFair => {}
            }
        }
        match (cut, self.request.max_depth) {
            (true, Some(depth)) => Ok(ModelCheckOutcome::BoundReached {
                depth: widen(depth),
                stats: totals.stats(),
            }),
            _ => Err(LivenessRefusal::ComponentCertificateNotBuilt {
                stats: totals.stats(),
            }
            .into()),
        }
    }

    /// Explore `binding`'s product. `Err(outcome)` is a run that ended
    /// before it had a graph to examine.
    fn explore(
        &self,
        binding: &'r ObjectReference,
        totals: &mut Totals,
    ) -> Result<Result<Explored<'r, 'a>, ModelCheckOutcome>, ModelCheckRefusal> {
        let mut automaton = translate(self.clause, self.request.limits.max_automaton_states)
            .map_err(LivenessRefusal::from)?;
        if automaton.kind() != AutomatonKind::Liveness {
            return Err(fault(
                "a-liveness-clause-translates-to-a-liveness-automaton",
            ));
        }
        let start = match automaton.initial() {
            Ok(start) => start,
            Err(AutomatonError::StateLimit { limit }) => {
                return Ok(Err(exhausted(
                    ModelCheckLimit::MaxAutomatonStates,
                    limit,
                    totals.stats(),
                )))
            }
            Err(other) => return Err(LivenessRefusal::from(other).into()),
        };
        let atoms = automaton.atoms().to_vec();
        let mut product = LassoSystem {
            system: self.system,
            letters: self.letters,
            binding,
            atoms,
            automaton: RefCell::new(automaton),
            start,
            readings: RefCell::new(BTreeMap::new()),
            initial: Vec::new(),
            meter: self.request.meter,
            cancel: self.cancel,
            max_depth: self.request.max_depth,
            halt: RefCell::new(None),
        };
        if product.start_states().is_err() {
            return self.halted(&product, totals).map(Err);
        }
        let exploration = retain(
            &product,
            self.request.limits.limits,
            IdentityLimits::default(),
            None,
            || self.cancel.poll() && self.cancel.cause().is_some(),
        )
        .map_err(|error| {
            ModelCheckRefusal::Encoding(EncodingRefusal::of(&error, IdentityLimits::default()))
        })?;
        let transitions = match &exploration.outcome {
            Outcome::Exhaustive(stats) => stats.transitions,
            Outcome::BoundReached { stats, .. }
            | Outcome::Bounded { stats, .. }
            | Outcome::Cancelled { stats, .. }
            | Outcome::Stopped { stats, .. } => stats.transitions,
        };
        let automaton_states = widen(product.automaton.borrow().reached_states());
        totals.add(&exploration.graph, transitions, automaton_states);
        let ending = match &exploration.outcome {
            Outcome::Exhaustive(_) => Ending::Complete,
            Outcome::Bounded { limit, .. } => {
                let (limit, value) = match limit {
                    Limit::States(value) => (ModelCheckLimit::MaxStates, widen(*value)),
                    Limit::Transitions(value) => (ModelCheckLimit::MaxTransitions, widen(*value)),
                };
                Ending::Stopped(
                    IncompleteCause::ResourceExhausted,
                    Some(ReachedLimit { limit, value }),
                )
            }
            Outcome::Cancelled { .. } => match self.cancel.cause() {
                Some(source) => Ending::Stopped(IncompleteCause::Cancelled { source }, None),
                None => return Err(fault("a-cancelled-run-has-a-cause")),
            },
            Outcome::Stopped { .. } => match self.halt_ending(&product)? {
                Some(ending) => ending,
                None => return Err(fault("a-stopped-expansion-recorded-its-halt")),
            },
            Outcome::BoundReached { .. } => {
                return Err(fault("the-horizon-is-the-product-s-not-the-exploration-s"))
            }
        };
        Ok(Ok(Explored {
            product,
            graph: exploration.graph,
            binding,
            ending,
        }))
    }

    /// The outcome of a product that halted before it had a graph.
    fn halted(
        &self,
        product: &LassoSystem<'_, '_>,
        totals: &Totals,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        match self.halt_ending(product)? {
            Some(Ending::Stopped(cause, limit)) => Ok(stopped(cause, limit, totals.stats())),
            Some(Ending::Complete) | None => Err(fault("a-halted-start-recorded-its-halt")),
        }
    }

    /// What the product recorded when it stopped the exploration.
    fn halt_ending(
        &self,
        product: &LassoSystem<'_, '_>,
    ) -> Result<Option<Ending>, ModelCheckRefusal> {
        let Some(halt) = product.halt.borrow_mut().take() else {
            return Ok(None);
        };
        let reached = |limit, value| {
            Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit { limit, value }),
            )
        };
        Ok(Some(match halt {
            Halt::Meter => reached(
                ModelCheckLimit::EvaluationMeter,
                self.request.meter.work_units,
            ),
            Halt::Cancelled(source) => Ending::Stopped(IncompleteCause::Cancelled { source }, None),
            Halt::Automaton(limit) => reached(ModelCheckLimit::MaxAutomatonStates, limit),
            Halt::Refusal(refusal) => return Err(refusal),
            Halt::Expansion(stop) => match stop.limit {
                Some(ExpansionLimit::EvaluationMeter) => reached(
                    ModelCheckLimit::EvaluationMeter,
                    self.request.meter.work_units,
                ),
                Some(ExpansionLimit::MaxCandidates) => {
                    reached(ModelCheckLimit::MaxCandidates, self.request.max_candidates)
                }
                None if stop.is_internal_failure() => {
                    return Err(fault("model-expansion-invariant"))
                }
                None => Ending::Stopped(IncompleteCause::ResourceExhausted, None),
            },
        }))
    }

    /// The first phase over one graph: the first state in discovery order
    /// whose letter is undefined or whose expansion evaluated a contract
    /// conjunction undefined ends the run; failing that, a refused one
    /// leaves it undecided.
    fn first_phase(&self, run: &Explored<'_, '_>) -> Result<FirstPhase, ModelCheckRefusal> {
        let mut refused = false;
        for (index, state) in run.graph.states().iter().enumerate() {
            let id = ProductStateId(index);
            if let Property::Undefined { atom, cause } = &state.state.property {
                let site = self.site(&state.state, UndefinedLocus::Atom(*atom), None)?;
                return Ok(FirstPhase::Violated(Box::new(Violation {
                    state: id,
                    kind: CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site,
                        cause: cause.clone(),
                    }),
                })));
            }
            for finding in &state.findings {
                let Finding::Contract(contract) = finding else {
                    continue;
                };
                if let Some(undefined) = contract.undefined_contract() {
                    let site = self.site(
                        &state.state,
                        UndefinedLocus::Contract(undefined.clause),
                        undefined.guard_of,
                    )?;
                    return Ok(FirstPhase::Violated(Box::new(Violation {
                        state: id,
                        kind: CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                            site,
                            cause: undefined.cause,
                        }),
                    })));
                }
                refused |= contract.is_refused_contract();
            }
            if refused {
                return Ok(FirstPhase::Undecided);
            }
        }
        Ok(FirstPhase::Clear)
    }

    fn site(
        &self,
        state: &LState,
        locus: UndefinedLocus,
        transition: Option<ModelTransition>,
    ) -> Result<UndefinedSite, ModelCheckRefusal> {
        Ok(UndefinedSite {
            position: widen(state.depth),
            state_key: self.digest(&state.model)?,
            locus,
            transition,
        })
    }

    fn digest(&self, state: &ModelSimState) -> Result<DigestRecord, ModelCheckRefusal> {
        Ok(self.system.state_key(state)?.1)
    }

    /// The finite-prefix counterexample to product state `at`: its
    /// breadth-first tree path.
    fn prefix_counterexample(
        &self,
        run: &Explored<'_, '_>,
        at: ProductStateId,
        kind: CounterexampleKind,
    ) -> Result<TemporalCounterexample, ModelCheckRefusal> {
        let (root, path) = tree_path(&run.graph, at);
        let prefix = self.steps(
            &run.graph,
            path.iter().map(|edge| (&edge.transition, edge.target)),
        )?;
        Ok(TemporalCounterexample {
            initial_state: self.initial_index(&run.graph, root)?,
            prefix,
            over_binding: Some(run.binding.clone()),
            fairness: self.clause.fairness().to_vec(),
            kind,
            lasso: None,
        })
    }

    /// The model steps of product edges; a terminal stutter edge has none.
    fn steps<'e>(
        &self,
        graph: &RetainedGraph<LState, PT, Finding>,
        edges: impl Iterator<Item = (&'e PT, ProductStateId)>,
    ) -> Result<Vec<ModelStep>, ModelCheckRefusal> {
        let mut steps = Vec::new();
        for (transition, target) in edges {
            let ProductTransition::Model(transition) = transition else {
                continue;
            };
            let state = graph
                .state(target)
                .ok_or_else(|| fault("an-edge-targets-a-retained-state"))?;
            steps.push(ModelStep {
                transition: transition.clone(),
                post_state: self.digest(&state.state.model)?,
            });
        }
        Ok(steps)
    }

    /// The index of the subject's initial state the product root `root`
    /// starts from: the first with its state key.
    fn initial_index(
        &self,
        graph: &RetainedGraph<LState, PT, Finding>,
        root: ProductStateId,
    ) -> Result<usize, ModelCheckRefusal> {
        let state = graph
            .state(root)
            .ok_or_else(|| fault("a-path-starts-at-a-retained-state"))?;
        let wanted = self.digest(&state.state.model)?;
        for (index, initial) in self.system.initial().iter().enumerate() {
            if self.digest(initial)? == wanted {
                return Ok(index);
            }
        }
        Err(fault("a-root-is-an-initial-state"))
    }

    /// The second phase over one graph: the canonical lasso of its first
    /// passing component, if some component passes.
    fn second_phase(&self, run: &Explored<'_, '_>) -> Result<Second, ModelCheckRefusal> {
        let model = FairModel::build(self.system, self.clause, run)?;
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses = match analyze(&model.graph, &all, &model.constraints, self.cancel) {
            Ok(analyses) => analyses,
            Err(FairnessError::Cancelled) => {
                return match self.cancel.cause() {
                    Some(source) => Ok(Second::Cancelled(source)),
                    None => Err(fault("a-cancelled-run-has-a-cause")),
                }
            }
            Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                return Err(fault("the-fairness-graph-is-built-from-retained-states"))
            }
        };
        let Some(passing) = Analysis::first_fair_in(&analyses) else {
            return Ok(Second::NoneFair);
        };
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let Some(entry) = passing.vertices().first().copied() else {
            return Err(fault("a-component-has-a-vertex"));
        };
        let loop_edges = model.greedy_loop(&component, entry)?;
        let (root, stem) = tree_path(&run.graph, ProductStateId(entry.index()));
        let counterexample = self.lasso(run, &model, root, &stem, &loop_edges)?;
        Ok(Second::Lasso(Box::new(counterexample)))
    }

    /// The lasso over `stem` (retained edges from `root`) and the walk's
    /// `loop_edges`, as a model behaviour.
    fn lasso(
        &self,
        run: &Explored<'_, '_>,
        model: &FairModel,
        root: ProductStateId,
        stem: &[&RetainedEdge<PT>],
        loop_edges: &[(Vertex, Edge)],
    ) -> Result<TemporalCounterexample, ModelCheckRefusal> {
        let mut prefix = self.steps(
            &run.graph,
            stem.iter().map(|edge| (&edge.transition, edge.target)),
        )?;
        let entry = prefix.len();
        let mut stutters = 0_usize;
        for (_, edge) in loop_edges {
            match edge.label {
                EdgeLabel::Stutter => stutters += 1,
                EdgeLabel::Transition(identity) => {
                    let transition = model
                        .transitions
                        .get(usize::try_from(identity.value()).unwrap_or(usize::MAX))
                        .ok_or_else(|| fault("a-label-names-a-numbered-transition"))?;
                    let target = run
                        .graph
                        .state(ProductStateId(edge.target.index()))
                        .ok_or_else(|| fault("an-edge-targets-a-retained-state"))?;
                    prefix.push(ModelStep {
                        transition: transition.clone(),
                        post_state: self.digest(&target.state.model)?,
                    });
                }
            }
        }
        // A terminal model state has only its stutter edge, so a loop holds
        // stutter edges alone or model edges alone, and a stem's stutter
        // edges are the infinite repetition's own prefix.
        let terminal_stutter = stutters > 0;
        if terminal_stutter && prefix.len() != entry {
            return Err(fault("a-loop-is-all-stutter-or-all-model-steps"));
        }
        Ok(TemporalCounterexample {
            initial_state: self.initial_index(&run.graph, root)?,
            prefix,
            over_binding: Some(run.binding.clone()),
            fairness: self.clause.fairness().to_vec(),
            kind: CounterexampleKind::Formula,
            lasso: Some(Lasso {
                entry,
                terminal_stutter,
            }),
        })
    }
}

/// The breadth-first tree path to `at`: the initial state it starts from and
/// the edges that first discovered each state on the way.
fn tree_path(
    graph: &RetainedGraph<LState, PT, Finding>,
    at: ProductStateId,
) -> (ProductStateId, Vec<&RetainedEdge<PT>>) {
    let mut discovered_by: BTreeMap<ProductStateId, &RetainedEdge<PT>> = BTreeMap::new();
    let initial: BTreeSet<ProductStateId> = graph.initial().iter().copied().collect();
    for edge in graph.edges() {
        if !initial.contains(&edge.target) {
            discovered_by.entry(edge.target).or_insert(edge);
        }
    }
    let mut path = Vec::new();
    let mut cursor = at;
    while let Some(edge) = discovered_by.get(&cursor) {
        path.push(*edge);
        cursor = edge.source;
    }
    path.reverse();
    (cursor, path)
}

/// One graph as the fairness kernels read it: vertices and edges of the
/// retained states, the model states' enabled identities, the automaton's
/// acceptance sets and the clause's constraints.
struct FairModel {
    graph: Graph,
    constraints: Vec<Constraint>,
    /// The transition numbered `n` by [`Identity::value`], ascending.
    transitions: Vec<ModelTransition>,
}

impl FairModel {
    fn build(
        system: &System<'_>,
        clause: &CheckedTemporalClause,
        run: &Explored<'_, '_>,
    ) -> Result<Self, ModelCheckRefusal> {
        let graph = &run.graph;
        let mut seen: BTreeSet<&ModelTransition> = BTreeSet::new();
        for edge in graph.edges() {
            if let ProductTransition::Model(transition) = &edge.transition {
                seen.insert(transition);
            }
        }
        for state in graph.states() {
            for finding in &state.findings {
                if let Finding::Enabled(enabled) = finding {
                    seen.extend(enabled);
                }
            }
        }
        let transitions: Vec<ModelTransition> = seen.into_iter().cloned().collect();
        let numbered: BTreeMap<&ModelTransition, Identity> = transitions
            .iter()
            .enumerate()
            .map(|(index, transition)| (transition, Identity::new(widen(index))))
            .collect();
        let identity = |transition: &ModelTransition| {
            numbered
                .get(transition)
                .copied()
                .ok_or_else(|| fault("every-seen-transition-is-numbered"))
        };

        let mut fair = Graph::new();
        let mut vertices = Vec::with_capacity(graph.states().len());
        for state in graph.states() {
            let mut enabled = Vec::new();
            for finding in &state.findings {
                if let Finding::Enabled(transitions) = finding {
                    for transition in transitions {
                        enabled.push(identity(transition)?);
                    }
                }
            }
            vertices.push(fair.add_vertex(enabled));
        }
        let vertex = |id: ProductStateId| {
            vertices
                .get(id.index())
                .copied()
                .ok_or_else(|| fault("an-edge-names-a-retained-state"))
        };
        for (index, _) in graph.states().iter().enumerate() {
            let source = ProductStateId(index);
            for edge in graph.outgoing(source) {
                let label = match &edge.transition {
                    ProductTransition::Model(transition) => {
                        EdgeLabel::Transition(identity(transition)?)
                    }
                    ProductTransition::TerminalStutter => EdgeLabel::Stutter,
                };
                fair.add_edge(vertex(source)?, vertex(edge.target)?, label)
                    .map_err(|_| fault("an-edge-joins-two-vertices"))?;
            }
        }
        let automaton = run.product.automaton.borrow();
        for set in 0..automaton.acceptance_set_count() {
            let members: Vec<Vertex> = graph
                .states()
                .iter()
                .enumerate()
                .filter(|(_, state)| {
                    matches!(&state.state.property,
                        Property::After(at) if automaton.is_accepting_in(set, *at))
                })
                .map(|(index, _)| vertices.get(index).copied())
                .collect::<Option<_>>()
                .ok_or_else(|| fault("a-member-is-a-vertex"))?;
            fair.add_acceptance_set(members)
                .map_err(|_| fault("an-acceptance-member-is-a-vertex"))?;
        }
        // No run through a dead state violates the clause: a set of the
        // states that still can keeps a dead state out of every candidate,
        // including when the negation has no acceptance set of its own.
        let live: Vec<Vertex> = graph
            .states()
            .iter()
            .enumerate()
            .filter(|(_, state)| matches!(&state.state.property, Property::After(_)))
            .filter_map(|(index, _)| vertices.get(index).copied())
            .collect();
        fair.add_acceptance_set(live)
            .map_err(|_| fault("an-acceptance-member-is-a-vertex"))?;

        let mut constraints = Vec::new();
        for constraint in clause.fairness() {
            let operation = constraint.operation();
            let text = system
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or_else(|| LivenessRefusal::UnknownOperation {
                    name: operation.name().to_owned(),
                })?;
            let covered = transitions
                .iter()
                .filter(|transition| transition.operation() == text);
            match constraint.granularity() {
                FairnessGranularity::Whole => {
                    let members = covered
                        .map(&identity)
                        .collect::<Result<Vec<_>, ModelCheckRefusal>>()?;
                    constraints.push(Constraint::weak(members));
                }
                FairnessGranularity::Each => {
                    for transition in covered {
                        constraints.push(Constraint::weak([identity(transition)?]));
                    }
                }
            }
        }
        Ok(Self {
            graph: fair,
            constraints,
            transitions,
        })
    }

    /// ADR-018 CX-5's greedy walk inside `component` from its entry `entry`:
    /// the loop's edges, each with the vertex it leaves.
    fn greedy_loop(
        &self,
        component: &BTreeSet<Vertex>,
        entry: Vertex,
    ) -> Result<Vec<(Vertex, Edge)>, ModelCheckRefusal> {
        let mut open = Open::of(self);
        open.discharge_vertex(self, entry);
        let mut walk = Vec::new();
        let mut current = entry;
        while !open.is_empty() {
            let path = self
                .nearest(component, current, &open)
                .ok_or_else(|| fault("a-passing-component-discharges-every-obligation"))?;
            for (from, edge) in &path {
                open.discharge_edge(self, edge.label);
                open.discharge_vertex(self, edge.target);
                current = edge.target;
                walk.push((*from, *edge));
            }
        }
        if current != entry || walk.is_empty() {
            let back = self
                .shortest_return(component, current, entry)
                .ok_or_else(|| fault("a-component-vertex-lies-on-a-cycle-through-the-entry"))?;
            walk.extend(back);
        }
        Ok(walk)
    }

    /// The shortest path inside `component` from `from` to the nearest
    /// vertex or edge that discharges an open obligation, ties broken by the
    /// canonical order of the path's transitions (breadth-first order over
    /// edges in canonical successor order).
    fn nearest(
        &self,
        component: &BTreeSet<Vertex>,
        from: Vertex,
        open: &Open,
    ) -> Option<Vec<(Vertex, Edge)>> {
        let mut parent: BTreeMap<Vertex, (Vertex, usize)> = BTreeMap::new();
        let mut distance: BTreeMap<Vertex, usize> = BTreeMap::from([(from, 0)]);
        let mut queue = VecDeque::from([from]);
        // Candidates of the least length so far: the vertex the path reaches
        // and, for an edge obligation, the position of its final edge.
        let mut best: Option<usize> = None;
        let mut candidates: Vec<(Vertex, Option<usize>)> = Vec::new();
        while let Some(vertex) = queue.pop_front() {
            let reached = *distance.get(&vertex)?;
            if best.is_some_and(|length| reached >= length) {
                break;
            }
            let mut consider = |length: usize, candidate: (Vertex, Option<usize>)| {
                match best {
                    Some(least) if length > least => return,
                    Some(least) if length < least => candidates.clear(),
                    _ => {}
                }
                best = Some(length);
                candidates.push(candidate);
            };
            if vertex != from && open.discharged_by_vertex(self, vertex) {
                consider(reached, (vertex, None));
            }
            for (position, edge) in self.graph.edges_from(vertex).iter().enumerate() {
                if !component.contains(&edge.target) {
                    continue;
                }
                if open.discharged_by_edge(self, edge.label) {
                    consider(reached + 1, (vertex, Some(position)));
                }
                if let std::collections::btree_map::Entry::Vacant(slot) =
                    distance.entry(edge.target)
                {
                    slot.insert(reached + 1);
                    parent.insert(edge.target, (vertex, position));
                    queue.push_back(edge.target);
                }
            }
        }
        let mut chosen: Option<Vec<(Vertex, usize)>> = None;
        for (end, last) in candidates {
            let mut path = self.climb(&parent, from, end)?;
            if let Some(position) = last {
                path.push((end, position));
            }
            let ordinals = |path: &[(Vertex, usize)]| -> Vec<usize> {
                path.iter().map(|(_, position)| *position).collect()
            };
            if chosen
                .as_deref()
                .is_none_or(|kept| ordinals(&path) < ordinals(kept))
            {
                chosen = Some(path);
            }
        }
        self.edges_of(&chosen?)
    }

    /// The tree path from `from` to `end` as (vertex, edge position) pairs.
    fn climb(
        &self,
        parent: &BTreeMap<Vertex, (Vertex, usize)>,
        from: Vertex,
        end: Vertex,
    ) -> Option<Vec<(Vertex, usize)>> {
        let mut path = Vec::new();
        let mut cursor = end;
        while cursor != from {
            let (before, position) = *parent.get(&cursor)?;
            path.push((before, position));
            cursor = before;
        }
        path.reverse();
        Some(path)
    }

    fn edges_of(&self, path: &[(Vertex, usize)]) -> Option<Vec<(Vertex, Edge)>> {
        path.iter()
            .map(|(vertex, position)| {
                self.graph
                    .edges_from(*vertex)
                    .get(*position)
                    .map(|edge| (*vertex, *edge))
            })
            .collect()
    }

    /// The shortest path inside `component` that leaves `from` along at
    /// least one edge and ends at `entry`: the walk's way back, or the
    /// shortest cycle through `entry` when `from` is `entry`.
    fn shortest_return(
        &self,
        component: &BTreeSet<Vertex>,
        from: Vertex,
        entry: Vertex,
    ) -> Option<Vec<(Vertex, Edge)>> {
        let mut parent: BTreeMap<Vertex, (Vertex, usize)> = BTreeMap::new();
        let mut seen: BTreeSet<Vertex> = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(vertex) = queue.pop_front() {
            for (position, edge) in self.graph.edges_from(vertex).iter().enumerate() {
                if !component.contains(&edge.target) {
                    continue;
                }
                if edge.target == entry {
                    let mut path = self.climb(&parent, from, vertex)?;
                    path.push((vertex, position));
                    return self.edges_of(&path);
                }
                if seen.insert(edge.target) {
                    parent.insert(edge.target, (vertex, position));
                    queue.push_back(edge.target);
                }
            }
        }
        None
    }
}

/// The obligations of a component not yet discharged (CX-5): one state of
/// each acceptance set, and for each weak constraint an edge that takes it
/// or a state that disables it.
struct Open {
    sets: BTreeSet<usize>,
    constraints: BTreeSet<usize>,
}

impl Open {
    fn of(model: &FairModel) -> Self {
        Self {
            sets: (0..model.graph.acceptance_sets().len()).collect(),
            constraints: (0..model.constraints.len()).collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.sets.is_empty() && self.constraints.is_empty()
    }

    fn disabled_at(model: &FairModel, constraint: usize, vertex: Vertex) -> bool {
        match (
            model.constraints.get(constraint),
            model.graph.enabled(vertex),
        ) {
            (Some(constraint), Some(enabled)) => !constraint.is_enabled_in(enabled),
            _ => false,
        }
    }

    fn discharged_by_vertex(&self, model: &FairModel, vertex: Vertex) -> bool {
        self.sets.iter().any(|set| {
            model
                .graph
                .acceptance_sets()
                .get(*set)
                .is_some_and(|members| members.contains(&vertex))
        }) || self
            .constraints
            .iter()
            .any(|constraint| Self::disabled_at(model, *constraint, vertex))
    }

    fn discharged_by_edge(&self, model: &FairModel, label: EdgeLabel) -> bool {
        self.constraints.iter().any(|constraint| {
            model
                .constraints
                .get(*constraint)
                .is_some_and(|constraint| constraint.is_taken_by(label))
        })
    }

    fn discharge_vertex(&mut self, model: &FairModel, vertex: Vertex) {
        self.sets.retain(|set| {
            !model
                .graph
                .acceptance_sets()
                .get(*set)
                .is_some_and(|members| members.contains(&vertex))
        });
        self.constraints
            .retain(|constraint| !Self::disabled_at(model, *constraint, vertex));
    }

    fn discharge_edge(&mut self, model: &FairModel, label: EdgeLabel) {
        self.constraints.retain(|constraint| {
            !model
                .constraints
                .get(*constraint)
                .is_some_and(|constraint| constraint.is_taken_by(label))
        });
    }
}
