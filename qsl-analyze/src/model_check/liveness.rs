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
//! A model state reached where the negation automaton has no move starts no
//! violating run, but its letters and contracts are still examined (UE-1):
//! a model-only pass from those states, over the model states no product
//! state already covers, ranks its findings with the first phase's by depth.
//! Those states are not product states and count toward no limit or total.
//!
//! The bindings are ranked across (FR-126 "Bindings and ranking"): the first
//! phase runs over every binding in canonical instance order and the finding
//! that ranks first by depth wins, a tie going to the earlier binding. Once a
//! binding has a finding at depth `d`, each later binding is explored only
//! through depth `d - 1`, since it wins only with a strictly shallower one; a
//! later binding that stops before it completes that depth answers with its
//! stop.
//!
//! The second phase runs only when no binding's first phase ended the run; it
//! then runs over every binding and the shortest lasso, stem and loop together,
//! wins, a tie again going to the earlier binding. Each binding's second phase
//! decomposes the retained graph into strongly connected components in
//! `qsl-eval`'s fairness kernels, passes a component that is non-trivial,
//! holds a state of every acceptance set and satisfies every weak
//! constraint (FA-4), and builds the canonical lasso (CX-5): the stem is the
//! breadth-first tree path to the first passing component's first state and
//! the loop is the greedy walk inside it. A component that satisfies the
//! weak constraints but starves a strong one is refined recursively (ADR-019
//! FS-2): the kernels remove the states where a starved strong constraint is
//! enabled and decide the remainder again, depth first, so the passing
//! component can be a sub-component of an SCC, and the greedy walk owes a
//! strong constraint an edge only when the component enables it somewhere
//! (FS-7). Enabledness is read from each
//! model state's own successor relation, never from product edges (FA-2).
//!
//! A product graph whose second phase finds no passing component proves the
//! item only with FR-339's component certificate, which is not built yet:
//! that run is refused ([`LivenessRefusal::ComponentCertificateNotBuilt`])
//! rather than returned as a closure proof, which does not check acceptance.
//!
//! Under symmetry (ADR-021 SYM-6, EI-3) each orbit of the `over` binding is
//! one product, bound to its least key and reduced by that key's
//! stabiliser: every product state is canonicalised and every retained edge
//! keeps its permutation. The second phase then runs on the annotated
//! quotient (`quotient`, FR-153), which decides weak and strong fairness of
//! either granularity; a run with no fair accepting component settles
//! `HoldsReduced`, which no certificate covers (RV-1). Every counterexample
//! is concretised before it leaves (`concretise`, FR-161): its stem and, for
//! a lasso, its loop, repeated until the concrete state returns to the
//! concrete entry. A state constraint on a liveness clause is refused
//! ([`ModelCheckRefusal::ReducedLiveness`]).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::automaton::{
    translate, Atom, AutomatonError, AutomatonKind, Letter, PropertyAutomaton, StateId,
};
use qsl_eval::fairness::{
    analyze, Analysis, Constraint, Edge, EdgeLabel, FairnessError, Graph, Identity, Obligation,
    Strength, Vertex,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{check_model_bounds, ModelFinding, ModelSimState, ModelSystem};
use qsl_eval::simulation::{
    EncodingRefusal, Expansion, ExpansionLimit, ExpansionStop, Limit, Limits, OfferedReductions,
    Outcome, Permutation, SymmetryGroup, TransitionSystem,
};
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::{CatalogCode, IdentityLimits, InternalFault};
use qsl_replay::{
    AppliedReduction, ClauseIdentity, ClauseRootRefusal, CounterexampleKind, IncompleteCause,
    ItemRefusal, Lasso, ModelCheckItem, ModelCheckLimit, ModelCheckOutcome, ReachedLimit, RunStats,
    TemporalCounterexample, UndecidedCause, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use qsl_semantics::check::{Activation, CheckedTemporalClause, FormulaNodeId};
use quire_canonical::{Encode, Sink, Writer};
use quire_exact::{Cancel, CancelCause, Charge, ChargePoint, Meter, ScalarLimits};
use quire_semantic_value::fairness::FairnessGranularity;
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{
    not_simulated, precheck, Admitted, ModelCheckRefusal, ModelCheckRequest, BOUND_POSITIONS,
};
use super::concretise::{Lift, LiftStop};
use super::graph::{retain_reduced, ProductStateId, RetainedEdge, RetainedGraph};
use super::product::{ProductKey, ProductTransition};
use super::quotient::{
    Analysis as QuotientAnalysis, Analyzer, LoopEdge, Quotient, QuotientConstraint, QuotientEdge,
    QuotientStop,
};
use super::reduce;

/// `resource_exhausted`/`insufficient-next-charge`, the code of an expansion
/// that stopped; [`Halt`] records which budget it was.
const STOPPED: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");

/// Why EN-1 refused a liveness clause, or could not settle one it explored.
#[derive(Debug, thiserror::Error)]
pub enum LivenessRefusal {
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
    let symmetric = !request.reductions.symmetry.is_empty();
    if matches!(clause.activation(), Activation::Each { guarded: true }) {
        return Err(ModelCheckRefusal::GuardedActivation);
    }
    // The clause's identity is resolved from the package, as the safety path
    // does (FR-337-AC-5): a package that gives it no root refuses.
    let root_cancelled = match ClauseIdentity::resolve(
        request.subject.simulation.package,
        request.subject.simulation.packages,
        clause,
        cancel,
    ) {
        Ok(_) => None,
        Err(ClauseRootRefusal::Cancelled(cause)) => Some(cause),
        Err(refusal) => return Err(ModelCheckRefusal::Item(ItemRefusal::ClauseRoot(refusal))),
    };
    let meter = request.subject.simulation.meter;
    let max_candidates = request.subject.simulation.expansion.max_candidates;
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation.clone())
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(not_simulated)?;
    let group = if request.reductions.is_empty() {
        SymmetryGroup::default()
    } else {
        match precheck(&request.reductions, &request.item, &system)? {
            Admitted::Group(group) => group,
            Admitted::Settled(outcome) => return Ok(outcome),
        }
    };
    if request.reductions.constraint.is_some() || request.reductions.partial_order {
        return Err(ModelCheckRefusal::ReducedLiveness);
    }
    // The instances are the keys of the universes of the populations that
    // admit the clause's `over` type (FR-125 Instancing); under symmetry one
    // per orbit, run under its stabiliser (FR-151 "Instances").
    let instances = system.instances(&clause.over().1).map_err(not_simulated)?;
    let bindings: Vec<Binding> = if symmetric {
        reduce::instance_groups(Some(&instances), &group)
            .into_iter()
            .filter_map(|instance| {
                let representative = instance.representative?;
                Some(Binding {
                    applied: Some(AppliedReduction::Symmetry {
                        groups: group.classes.clone(),
                        orbit: instance.orbit,
                        stabiliser: instance.group.classes.clone(),
                    }),
                    instance: representative,
                    group: instance.group,
                })
            })
            .collect()
    } else {
        instances
            .into_iter()
            .map(|instance| Binding {
                instance,
                group: SymmetryGroup::default(),
                applied: None,
            })
            .collect()
    };
    let totals = Totals::default();
    if let Some(source) = root_cancelled {
        return Ok(stopped(
            IncompleteCause::Cancelled { source },
            None,
            totals.stats(),
        ));
    }
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
        meter: RefCell::new(Meter::new(meter)),
    };
    run.decide(totals)
}

fn fault(invariant: &'static str) -> ModelCheckRefusal {
    ModelCheckRefusal::Fault(InternalFault::new("model-check", invariant))
}

/// One product the run explores: the bound instance, the group its product
/// is reduced by (trivial with no symmetry), and the reduction it applies.
struct Binding {
    instance: ModelInstance,
    group: SymmetryGroup,
    applied: Option<AppliedReduction>,
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
    /// The states and edges the model-only passes were charged: budget used,
    /// but in no product total.
    charged_states: u64,
    charged_edges: u64,
}

impl Totals {
    /// What is left of `max_states` and `max_transitions`: the products' and
    /// the model-only passes' use subtracted.
    fn remaining(&self, limits: Limits) -> (usize, usize) {
        let used = |counted: u64, charged: u64| {
            usize::try_from(counted.saturating_add(charged)).unwrap_or(usize::MAX)
        };
        (
            limits
                .max_states
                .saturating_sub(used(self.states, self.charged_states)),
            limits
                .max_transitions
                .saturating_sub(used(self.edges, self.charged_edges)),
        )
    }

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
        // Every binding translates the same clause into the same automaton.
        self.automaton_states = self.automaton_states.max(automaton);
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
    /// The letter at this position is undefined.
    Undefined {
        atom: FormulaNodeId,
        cause: UndefinedCause,
    },
}

/// A product state's canonical property key.
enum PropertyKey {
    After(usize),
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
}

/// A model state reached where the negation automaton has no move.
struct DeadRoot {
    /// The product state it was reached from, by its model state's digest and
    /// automaton state, and the transition taken; `None` for an initial state.
    via: Option<(DigestRecord, StateId, PT)>,
    model: ModelSimState,
    depth: usize,
}

/// One binding's product as a `TransitionSystem`.
struct LassoSystem<'r, 'a> {
    system: &'r ModelSystem<'a>,
    letters: &'r dyn ModelLetters,
    binding: &'r ModelInstance,
    atoms: Vec<FormulaNodeId>,
    automaton: RefCell<PropertyAutomaton>,
    start: StateId,
    readings: RefCell<BTreeMap<DigestRecord, Reading>>,
    initial: Vec<LState>,
    dead: RefCell<Vec<DeadRoot>>,
    meter: ScalarLimits,
    cancel: &'r Cancel,
    max_depth: Option<usize>,
    /// The depth of the first-phase finding the exploration has met: a
    /// state that would sit there or deeper ranks after it, so none is
    /// created (FR-126 ends the first phase at its finding).
    found: Cell<Option<usize>>,
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

    /// Note a first-phase finding at `depth`.
    fn found_at(&self, depth: usize) {
        self.found.set(Some(
            self.found.get().map_or(depth, |known| known.min(depth)),
        ));
    }

    fn poll(&self) -> Result<(), ExpansionStop> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(self.record(Halt::Cancelled(source)));
            }
        }
        Ok(())
    }

    /// The model's expansion of `state`, recording why when it stops.
    fn expand(
        &self,
        state: &ModelSimState,
    ) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
        self.system.successors(state).inspect_err(|&stop| {
            self.record(Halt::Expansion(stop));
        })
    }

    fn digest_of(&self, state: &ModelSimState) -> Result<DigestRecord, ExpansionStop> {
        self.system
            .state_key(state)
            .map(|(_, digest)| digest)
            .map_err(|refusal| self.record(Halt::Refusal(ModelCheckRefusal::Encoding(refusal))))
    }

    /// The letter of `state` for this binding, read once per model state.
    fn reading(&self, state: &ModelSimState) -> Result<Reading, ExpansionStop> {
        let digest = self.digest_of(state)?;
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

    /// The property states a position reads into, from `before`; none when
    /// the automaton has no move on the letter.
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
        Ok(targets.into_iter().map(Property::After).collect())
    }

    /// The initial product states: each initial model state paired with each
    /// property state its letter reads into.
    fn start_states(&mut self) -> Result<(), ExpansionStop> {
        let mut initial = Vec::new();
        for model in self.system.initial() {
            let reading = self.reading(&model)?;
            let properties = self.step(Before::Start, &reading)?;
            if properties.is_empty() {
                self.dead.borrow_mut().push(DeadRoot {
                    via: None,
                    model: model.clone(),
                    depth: 0,
                });
            }
            for property in properties {
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
    type Key = ProductKey<<ModelSystem<'a> as TransitionSystem>::Key, PropertyKey>;
    type Finding = Finding;

    fn initial(&self) -> Vec<LState> {
        self.initial.clone()
    }

    fn key(&self, state: &LState) -> Self::Key {
        ProductKey {
            model: self.system.key(&state.model),
            property: match &state.property {
                Property::After(state) => PropertyKey::After(state.index()),
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
            Property::After(at) => Before::At(*at),
        };
        let expansion = self.expand(&state.model)?;
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
        if findings.iter().any(|finding| {
            matches!(finding, Finding::Contract(contract)
                if contract.undefined_contract().is_some() || contract.is_refused_contract())
        }) {
            self.found_at(state.depth);
        }
        let met_finding = self
            .found
            .get()
            .is_some_and(|found| state.depth.saturating_add(1) >= found);
        if met_finding || self.max_depth.is_some_and(|horizon| state.depth >= horizon) {
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
            let properties = self.step(before, &reading)?;
            if let (true, Before::At(at)) = (properties.is_empty(), before) {
                self.dead.borrow_mut().push(DeadRoot {
                    via: Some((self.digest_of(&state.model)?, at, transition.clone())),
                    model: model.clone(),
                    depth: state.depth.saturating_add(1),
                });
            }
            if properties
                .iter()
                .any(|property| matches!(property, Property::Undefined { .. }))
            {
                self.found_at(state.depth.saturating_add(1));
            }
            for property in properties {
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

    /// ADR-021 EI-4: the product offers what its model component offers.
    fn offered_reductions(&self) -> OfferedReductions {
        self.system.offered_reductions()
    }

    /// ADR-021 EI-4: the model component is canonicalised and the property
    /// component kept, so the automaton is never reduced.
    fn canonical(
        &self,
        state: &LState,
        group: &SymmetryGroup,
    ) -> Option<Result<(LState, Permutation), ExpansionStop>> {
        Some(
            self.system
                .canonicalise(&state.model, group)
                .map(|(model, permutation)| {
                    (
                        LState {
                            model,
                            property: state.property.clone(),
                            depth: state.depth,
                        },
                        permutation,
                    )
                })
                .inspect_err(|&stop| {
                    self.record(Halt::Expansion(stop));
                }),
        )
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
    binding: &'r Binding,
    /// What is left of the run's `max_transitions` for concrete
    /// counterexample steps (FR-161 "Length and limits").
    budget: usize,
    ending: Ending,
    /// Whether a model state reached where the automaton has no move was
    /// left unexpanded at the search horizon.
    dead_cut: bool,
}

/// A built counterexample, or the stop that ended its concretisation
/// (FR-161 "Length and limits").
enum Built {
    Counterexample(Box<TemporalCounterexample>),
    Stopped(IncompleteCause, Option<ReachedLimit>),
}

/// The first phase's finding: a counterexample, or a refused contract.
enum FirstHit {
    Violated(Box<TemporalCounterexample>),
    Undecided,
    /// Concretising the counterexample stopped the run.
    Stopped(IncompleteCause, Option<ReachedLimit>),
}

impl From<Built> for FirstHit {
    fn from(built: Built) -> Self {
        match built {
            Built::Counterexample(counterexample) => Self::Violated(counterexample),
            Built::Stopped(cause, limit) => Self::Stopped(cause, limit),
        }
    }
}

impl FirstHit {
    fn outcome(self, stats: RunStats) -> ModelCheckOutcome {
        match self {
            Self::Violated(counterexample) => ModelCheckOutcome::Violated {
                counterexample,
                stats,
            },
            Self::Undecided => ModelCheckOutcome::Undecided {
                cause: UndecidedCause::UndecidedSuccessor,
                stats,
            },
            Self::Stopped(cause, limit) => stopped(cause, limit, stats),
        }
    }
}

/// The shallowest depth each model state is known at, by its state-key
/// digest: a product state's, or the model-only pass's own.
#[derive(Default)]
struct Depths(BTreeMap<DigestRecord, usize>);

impl Depths {
    /// Record that `digest` is reached at `depth`; whether that is news: the
    /// state is new, or this is shallower than where it was known, so a
    /// finding there ranks first (UE-4).
    fn reach(&mut self, digest: DigestRecord, depth: usize) -> bool {
        match self.0.entry(digest) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(depth);
                true
            }
            std::collections::btree_map::Entry::Occupied(mut known) => {
                if depth < *known.get() {
                    known.insert(depth);
                    true
                } else {
                    false
                }
            }
        }
    }
}

/// A model state of the model-only pass over the states a dead automaton
/// leaves: how it was reached, from a root or from another such state.
struct DeadNode {
    model: ModelSimState,
    parent: Option<(usize, ModelTransition)>,
    root: usize,
}

/// What the model-only pass found.
#[derive(Default)]
struct DeadPhase {
    hit: Option<(usize, FirstHit)>,
    cut: bool,
    stopped: Option<Ending>,
}

/// The annotated quotient of one reduced product and its constraints.
type Annotated = (
    Quotient<ModelTransition>,
    Vec<QuotientConstraint<ModelTransition>>,
);

/// Why a lasso's loop was not built: its concretisation stopped, or the
/// annotated quotient's loop search did.
enum LoopStop {
    Lift(LiftStop),
    Quotient(QuotientStop),
}

impl From<LiftStop> for LoopStop {
    fn from(stop: LiftStop) -> Self {
        Self::Lift(stop)
    }
}

impl From<ModelCheckRefusal> for LoopStop {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Lift(LiftStop::Refusal(refusal))
    }
}

/// What the second phase found in one graph.
enum Second {
    Lasso(Box<TemporalCounterexample>),
    Cancelled(CancelCause),
    /// A limit stopped the annotated quotient's analysis or the lasso's
    /// concretisation.
    Stopped(IncompleteCause, Option<ReachedLimit>),
    NoneFair,
}

impl From<Built> for Second {
    fn from(built: Built) -> Self {
        match built {
            Built::Counterexample(counterexample) => Self::Lasso(counterexample),
            Built::Stopped(cause, limit) => Self::Stopped(cause, limit),
        }
    }
}

/// The run: the subject's system, the item and the request.
struct Run<'r, 'a> {
    system: &'r ModelSystem<'a>,
    clause: &'r CheckedTemporalClause,
    letters: &'r dyn ModelLetters,
    bindings: &'r [Binding],
    request: RunRequest,
    cancel: &'r Cancel,
    /// The run's meter, which the annotated quotient's work of every
    /// product charges (FR-153 "Limits").
    meter: RefCell<Meter>,
}

impl<'r, 'a> Run<'r, 'a> {
    fn decide(&self, mut totals: Totals) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let mut explored = Vec::with_capacity(self.bindings.len());
        // The finding that ranks first so far, with its depth: a later
        // binding replaces it only with a strictly shallower one (a tie goes
        // to the earlier binding), so it is explored through `depth - 1`
        // alone and not at all past a finding at depth 0.
        let mut best: Option<(usize, FirstHit)> = None;
        for (index, binding) in self.bindings.iter().enumerate() {
            let cap = match best {
                Some((0, _)) => break,
                Some((depth, _)) => Some(depth - 1),
                None => None,
            };
            let mut run = match self.explore(binding, cap, &mut totals)? {
                Ok(run) => run,
                Err(outcome) => return Ok(outcome),
            };
            let live = self.first_phase(&run)?;
            if let Ending::Stopped(cause, limit) = &run.ending {
                // A binding that stops answers with its stop, unless it holds
                // a finding that is canonical (every shallower state was
                // expanded) with no earlier finding to rank against: that
                // finding then bounds the later bindings, which stop in turn
                // when the budget is spent (FR-126 "Bindings and ranking").
                let canonical = live.as_ref().is_some_and(|(depth, _)| {
                    run.graph.states().iter().all(|state| {
                        state.state.depth >= *depth
                            || state
                                .findings
                                .iter()
                                .any(|finding| matches!(finding, Finding::Enabled(_)))
                    })
                });
                match live {
                    Some((depth, hit)) if best.is_none() && canonical => {
                        // The binding's model-only pass has not run: a dead
                        // root below the finding may hold a shallower one.
                        let dead = self.dead_phase(&run, Some(depth), &mut totals)?;
                        if let Some(Ending::Stopped(cause, limit)) = dead.stopped {
                            return Ok(stopped(cause, limit, totals.stats()));
                        }
                        let (depth, hit) = match dead.hit {
                            Some((dead_depth, dead_hit)) if dead_depth < depth => {
                                (dead_depth, dead_hit)
                            }
                            _ => (depth, hit),
                        };
                        let last = index + 1 == self.bindings.len();
                        if depth == 0 || last {
                            return Ok(hit.outcome(totals.stats()));
                        }
                        best = Some((depth, hit));
                        continue;
                    }
                    _ => return Ok(stopped(*cause, *limit, totals.stats())),
                }
            }
            let bound = [
                live.as_ref().map(|(depth, _)| *depth),
                cap.map(|cap| cap + 1),
            ]
            .into_iter()
            .flatten()
            .min();
            let dead = self.dead_phase(&run, bound, &mut totals)?;
            run.dead_cut = dead.cut;
            if let Some(Ending::Stopped(cause, limit)) = dead.stopped {
                // The pass did not finish, so a shallower finding may be
                // hidden: the stop is the answer.
                return Ok(stopped(cause, limit, totals.stats()));
            }
            match (live, dead.hit) {
                (Some((live_depth, _)), Some((dead_depth, hit))) if dead_depth < live_depth => {
                    best = Some((dead_depth, hit));
                }
                (Some(found), _) | (None, Some(found)) => best = Some(found),
                (None, None) => {}
            }
            explored.push(run);
        }
        if let Some((_, hit)) = best {
            return Ok(hit.outcome(totals.stats()));
        }
        // No binding's first phase ended: the shortest lasso across the
        // bindings, measured in transitions, the earlier binding winning a
        // tie.
        let mut cut = false;
        let mut shortest: Option<Box<TemporalCounterexample>> = None;
        for run in &explored {
            cut |= run.dead_cut
                || run
                    .graph
                    .states()
                    .iter()
                    .any(|state| state.findings.contains(&Finding::HorizonCut));
            match self.second_phase(run)? {
                Second::Lasso(counterexample) => {
                    if shortest
                        .as_ref()
                        .is_none_or(|known| counterexample.len() < known.len())
                    {
                        shortest = Some(counterexample);
                    }
                }
                Second::Cancelled(source) => {
                    return Ok(stopped(
                        IncompleteCause::Cancelled { source },
                        None,
                        totals.stats(),
                    ))
                }
                Second::Stopped(cause, limit) => return Ok(stopped(cause, limit, totals.stats())),
                Second::NoneFair => {}
            }
        }
        if let Some(counterexample) = shortest {
            return Ok(ModelCheckOutcome::Violated {
                counterexample,
                stats: totals.stats(),
            });
        }
        let applied: Option<Vec<AppliedReduction>> = self
            .bindings
            .iter()
            .map(|binding| binding.applied.clone())
            .collect();
        match (cut, self.request.max_depth, applied) {
            (true, Some(depth), _) => Ok(ModelCheckOutcome::BoundReached {
                depth: widen(depth),
                stats: totals.stats(),
            }),
            // ADR-021 RV-1: a reduced run that completes with no fair
            // accepting component is a reduced proof, which no certificate
            // covers.
            (_, _, Some(reductions)) if !reductions.is_empty() => {
                Ok(ModelCheckOutcome::HoldsReduced {
                    reductions,
                    stats: totals.stats(),
                })
            }
            _ => Err(LivenessRefusal::ComponentCertificateNotBuilt {
                stats: totals.stats(),
            }
            .into()),
        }
    }

    /// Explore `binding`'s product, through depth `cap` when there is one
    /// (and no deeper than the request's own horizon). `Err(outcome)` is a
    /// run that ended before it had a graph to examine.
    fn explore(
        &self,
        binding: &'r Binding,
        cap: Option<usize>,
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
            binding: &binding.instance,
            atoms,
            automaton: RefCell::new(automaton),
            start,
            readings: RefCell::new(BTreeMap::new()),
            initial: Vec::new(),
            dead: RefCell::new(Vec::new()),
            meter: self.request.meter,
            cancel: self.cancel,
            max_depth: [self.request.max_depth, cap].into_iter().flatten().min(),
            found: Cell::new(None),
            halt: RefCell::new(None),
        };
        if product.start_states().is_err() {
            return self.halted(&product, totals).map(Err);
        }
        // The budgets are the run's, shared by every binding.
        let spent = self.request.limits.limits;
        let (max_states, max_transitions) = totals.remaining(spent);
        let remaining = Limits {
            max_states,
            max_transitions,
        };
        let exploration = retain_reduced(
            &product,
            &binding.group,
            remaining,
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
        let budget = totals.remaining(spent).1;
        let ending = match &exploration.outcome {
            Outcome::Exhaustive(_) => Ending::Complete,
            Outcome::Bounded { limit, .. } => {
                let (limit, value) = match limit {
                    Limit::States(_) => (ModelCheckLimit::MaxStates, widen(spent.max_states)),
                    Limit::Transitions(_) => (
                        ModelCheckLimit::MaxTransitions,
                        widen(spent.max_transitions),
                    ),
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
            budget,
            ending,
            dead_cut: false,
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
        self.ending(halt).map(Some)
    }

    /// The ending `halt` settles.
    fn ending(&self, halt: Halt) -> Result<Ending, ModelCheckRefusal> {
        let reached = |limit, value| {
            Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit { limit, value }),
            )
        };
        Ok(match halt {
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
        })
    }

    /// The first phase over one graph: the first state in discovery order
    /// whose letter is undefined or whose expansion evaluated a contract
    /// conjunction undefined ends the run; failing that, a refused one
    /// leaves it undecided. The finding comes with the depth it was at.
    fn first_phase(
        &self,
        run: &Explored<'_, '_>,
    ) -> Result<Option<(usize, FirstHit)>, ModelCheckRefusal> {
        let mut refused = false;
        for (index, state) in run.graph.states().iter().enumerate() {
            let id = ProductStateId(index);
            let violated = |kind| -> Result<Option<(usize, FirstHit)>, ModelCheckRefusal> {
                let built = self.prefix_counterexample(run, id, kind)?;
                Ok(Some((state.state.depth, FirstHit::from(built))))
            };
            if let Property::Undefined { atom, cause } = &state.state.property {
                let site = self.site(&state.state, UndefinedLocus::Atom(*atom), None)?;
                return violated(CounterexampleKind::UndefinedEvaluation(
                    UndefinedEvaluation {
                        site,
                        cause: cause.clone(),
                    },
                ));
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
                    return violated(CounterexampleKind::UndefinedEvaluation(
                        UndefinedEvaluation {
                            site,
                            cause: undefined.cause,
                        },
                    ));
                }
                refused |= contract.is_refused_contract();
            }
            if refused {
                return Ok(Some((state.state.depth, FirstHit::Undecided)));
            }
        }
        Ok(None)
    }

    /// The model-only pass over the states reached where the automaton has
    /// no move (UE-1 reads every position of an admitted behaviour): in
    /// breadth-first order from those states, over the model states no
    /// product state covers, the first whose letter is undefined or whose
    /// expansion evaluates a contract conjunction undefined ends the first
    /// phase, and a refused one leaves it undecided. It stops at `bound`, the
    /// depth of a finding the product already has, which wins ties. Its
    /// states and edges are charged to what is left of the run's
    /// `max_states` and `max_transitions`, though they join no total: a pass
    /// that exhausts either stops the run naming it.
    fn dead_phase(
        &self,
        run: &Explored<'_, '_>,
        bound: Option<usize>,
        spent: &mut Totals,
    ) -> Result<DeadPhase, ModelCheckRefusal> {
        let (mut states_left, mut edges_left) = spent.remaining(self.request.limits.limits);
        let (states_before, edges_before) = (states_left, edges_left);
        let phase = self.dead_walk(run, bound, &mut states_left, &mut edges_left);
        spent.charged_states = spent
            .charged_states
            .saturating_add(widen(states_before - states_left));
        spent.charged_edges = spent
            .charged_edges
            .saturating_add(widen(edges_before - edges_left));
        phase
    }

    /// [`Self::dead_phase`]'s walk, decrementing the budget it is given.
    fn dead_walk(
        &self,
        run: &Explored<'_, '_>,
        bound: Option<usize>,
        states_left: &mut usize,
        edges_left: &mut usize,
    ) -> Result<DeadPhase, ModelCheckRefusal> {
        let roots = std::mem::take(&mut *run.product.dead.borrow_mut());
        let mut phase = DeadPhase::default();
        if roots.is_empty() {
            return Ok(phase);
        }
        let limits = self.request.limits.limits;
        let out_of = |limit, value: usize| {
            Some(Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit,
                    value: widen(value),
                }),
            ))
        };
        let mut examined = Depths::default();
        let mut live: BTreeMap<(DigestRecord, StateId), ProductStateId> = BTreeMap::new();
        for (index, state) in run.graph.states().iter().enumerate() {
            let digest = self.digest(&state.state.model)?;
            examined.reach(digest, state.depth);
            if let Property::After(at) = &state.state.property {
                live.insert((digest, *at), ProductStateId(index));
            }
        }
        let mut nodes: Vec<DeadNode> = Vec::new();
        let mut frontier: BTreeSet<(usize, usize)> = BTreeSet::new();
        for (root, entry) in roots.iter().enumerate() {
            if bound.is_some_and(|bound| entry.depth >= bound) {
                continue;
            }
            if examined.reach(self.digest(&entry.model)?, entry.depth) {
                if *states_left == 0 {
                    phase.stopped = out_of(ModelCheckLimit::MaxStates, limits.max_states);
                    return Ok(phase);
                }
                *states_left -= 1;
                frontier.insert((entry.depth, nodes.len()));
                nodes.push(DeadNode {
                    model: entry.model.clone(),
                    parent: None,
                    root,
                });
            }
        }
        let mut found: Option<usize> = None;
        let stop = |phase: &mut DeadPhase| -> Result<(), ModelCheckRefusal> {
            phase.stopped = self.halt_ending(&run.product)?;
            Ok(())
        };
        while let Some((depth, index)) = frontier.pop_first() {
            if bound.is_some_and(|live_depth| depth >= live_depth) {
                break;
            }
            let Some(model) = nodes.get(index).map(|node| node.model.clone()) else {
                return Err(fault("a-frontier-index-names-a-node"));
            };
            if run.product.poll().is_err() {
                stop(&mut phase)?;
                return Ok(phase);
            }
            let reading = match run.product.reading(&model) {
                Ok(reading) => reading,
                Err(_) => {
                    stop(&mut phase)?;
                    return Ok(phase);
                }
            };
            let expansion = match run.product.expand(&model) {
                Ok(expansion) => expansion,
                Err(_) => {
                    stop(&mut phase)?;
                    return Ok(phase);
                }
            };
            let kind = if let Reading::Undefined { atom, cause } = reading {
                Some((UndefinedLocus::Atom(atom), None, cause))
            } else {
                expansion
                    .findings
                    .iter()
                    .find_map(ModelFinding::undefined_contract)
                    .map(|undefined| {
                        (
                            UndefinedLocus::Contract(undefined.clause),
                            undefined.guard_of,
                            undefined.cause,
                        )
                    })
            };
            if let Some(found) = kind {
                let (locus, transition, cause) = found;
                let site = UndefinedSite {
                    position: widen(depth),
                    state_key: self.digest(&model)?,
                    locus,
                    transition,
                };
                let counterexample = self.dead_counterexample(
                    run,
                    &roots,
                    &nodes,
                    &live,
                    index,
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation { site, cause }),
                )?;
                phase.hit = Some((depth, FirstHit::from(counterexample)));
                return Ok(phase);
            }
            if expansion
                .findings
                .iter()
                .any(ModelFinding::is_refused_contract)
            {
                phase.hit = Some((depth, FirstHit::Undecided));
                return Ok(phase);
            }
            if self
                .request
                .max_depth
                .is_some_and(|horizon| depth >= horizon)
            {
                phase.cut = true;
                continue;
            }
            // A state at the bound ranks no better than the finding that set
            // it, so the successors that would sit there are neither created
            // nor charged.
            // The same holds below the depth of an undefined letter the pass
            // has met at a state's creation: the first phase ends at it.
            if [bound, found]
                .into_iter()
                .flatten()
                .any(|limit| depth.saturating_add(1) >= limit)
            {
                continue;
            }
            let root = nodes.get(index).map_or(0, |node| node.root);
            for (transition, next) in expansion.successors {
                if *edges_left == 0 {
                    phase.stopped = out_of(ModelCheckLimit::MaxTransitions, limits.max_transitions);
                    return Ok(phase);
                }
                *edges_left -= 1;
                if examined.reach(self.digest(&next)?, depth.saturating_add(1)) {
                    if *states_left == 0 {
                        phase.stopped = out_of(ModelCheckLimit::MaxStates, limits.max_states);
                        return Ok(phase);
                    }
                    *states_left -= 1;
                    let undefined = match run.product.reading(&next) {
                        Ok(reading) => matches!(reading, Reading::Undefined { .. }),
                        Err(_) => {
                            stop(&mut phase)?;
                            return Ok(phase);
                        }
                    };
                    frontier.insert((depth.saturating_add(1), nodes.len()));
                    nodes.push(DeadNode {
                        model: next,
                        parent: Some((index, transition)),
                        root,
                    });
                    if undefined {
                        found = Some(depth.saturating_add(1));
                        break;
                    }
                }
            }
        }
        Ok(phase)
    }

    /// The counterexample to dead-pass node `at`: the product's tree path to
    /// the state its root was reached from, the root's transition, and the
    /// model path from the root, concretised (FR-161).
    fn dead_counterexample(
        &self,
        run: &Explored<'_, '_>,
        roots: &[DeadRoot],
        nodes: &[DeadNode],
        live: &BTreeMap<(DigestRecord, StateId), ProductStateId>,
        at: usize,
        mut kind: CounterexampleKind,
    ) -> Result<Built, ModelCheckRefusal> {
        let mut chain = Vec::new();
        let mut cursor = at;
        while let Some(node) = nodes.get(cursor) {
            chain.push(cursor);
            match &node.parent {
                Some((parent, _)) => cursor = *parent,
                None => break,
            }
        }
        chain.reverse();
        let Some(root) = chain
            .first()
            .and_then(|first| nodes.get(*first))
            .and_then(|node| roots.get(node.root))
        else {
            return Err(fault("a-dead-node-has-a-root"));
        };
        let lifted = (|| -> Result<(usize, Lift<'_, '_>), LiftStop> {
            // A dead root and the model path from it were never
            // canonicalised: each is a step with the identity permutation.
            let identity = Permutation::identity();
            let (initial, mut lift) = match &root.via {
                Some((digest, at, transition)) => {
                    let Some(parent) = live.get(&(*digest, *at)) else {
                        return Err(fault("a-dead-root-was-reached-from-a-product-state").into());
                    };
                    let (start, path) = tree_path(&run.graph, *parent);
                    let (initial, mut lift) = self.lift_start(run, start)?;
                    self.lift_edges(run, &mut lift, &path)?;
                    if let ProductTransition::Model(transition) = transition {
                        lift.edge(transition, &root.model, &identity)?;
                    }
                    (initial, lift)
                }
                // An initial model state with no automaton move is concrete.
                None => Lift::start(
                    self.system,
                    &SymmetryGroup::default(),
                    &root.model,
                    run.budget,
                )?,
            };
            for index in chain.iter().skip(1) {
                let Some(node) = nodes.get(*index) else {
                    return Err(fault("a-chain-index-names-a-node").into());
                };
                if let Some((_, transition)) = &node.parent {
                    lift.edge(transition, &node.model, &identity)?;
                }
            }
            lift.site(&mut kind)?;
            Ok((initial, lift))
        })();
        self.built(lifted.map(|(initial_state, lift)| TemporalCounterexample {
            initial_state,
            prefix: lift.into_steps(),
            over_binding: Some(run.binding.instance.clone()),
            fairness: self.clause.fairness().to_vec(),
            kind,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
        }))
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
    /// breadth-first tree path, concretised (FR-161).
    fn prefix_counterexample(
        &self,
        run: &Explored<'_, '_>,
        at: ProductStateId,
        mut kind: CounterexampleKind,
    ) -> Result<Built, ModelCheckRefusal> {
        let (root, path) = tree_path(&run.graph, at);
        let lifted = (|| -> Result<(usize, Lift<'_, '_>), LiftStop> {
            let (initial, mut lift) = self.lift_start(run, root)?;
            self.lift_edges(run, &mut lift, &path)?;
            lift.site(&mut kind)?;
            Ok((initial, lift))
        })();
        self.built(lifted.map(|(initial_state, lift)| TemporalCounterexample {
            initial_state,
            prefix: lift.into_steps(),
            over_binding: Some(run.binding.instance.clone()),
            fairness: self.clause.fairness().to_vec(),
            kind,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
        }))
    }

    /// FR-161 "Start" at product root `root`, under the binding's group.
    fn lift_start(
        &self,
        run: &Explored<'_, '_>,
        root: ProductStateId,
    ) -> Result<(usize, Lift<'r, 'a>), LiftStop> {
        let state = run
            .graph
            .state(root)
            .ok_or_else(|| fault("a-path-starts-at-a-retained-state"))?;
        Lift::start(
            self.system,
            &run.binding.group,
            &state.state.model,
            run.budget,
        )
    }

    /// FR-161 "Steps" along retained product edges; a terminal stutter edge
    /// is no model step.
    fn lift_edges(
        &self,
        run: &Explored<'_, '_>,
        lift: &mut Lift<'_, '_>,
        edges: &[&RetainedEdge<PT>],
    ) -> Result<(), LiftStop> {
        for edge in edges {
            let ProductTransition::Model(transition) = &edge.transition else {
                continue;
            };
            let target = run
                .graph
                .state(edge.target)
                .ok_or_else(|| fault("an-edge-targets-a-retained-state"))?;
            lift.edge(transition, &target.state.model, &edge.permutation)?;
        }
        Ok(())
    }

    /// A counterexample, or the stop its concretisation settles: an
    /// expansion's limit as the exploration settles it, or
    /// `max_transitions` (FR-161 "Length and limits").
    fn built(
        &self,
        lifted: Result<TemporalCounterexample, LiftStop>,
    ) -> Result<Built, ModelCheckRefusal> {
        match lifted {
            Ok(counterexample) => Ok(Built::Counterexample(Box::new(counterexample))),
            Err(LiftStop::Expansion(stop)) => match self.ending(Halt::Expansion(stop))? {
                Ending::Stopped(cause, limit) => Ok(Built::Stopped(cause, limit)),
                Ending::Complete => Err(fault("an-expansion-stop-is-a-stop")),
            },
            Err(LiftStop::Transitions) => Ok(Built::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxTransitions,
                    value: widen(self.request.limits.limits.max_transitions),
                }),
            )),
            Err(LiftStop::Refusal(refusal)) => Err(refusal),
        }
    }

    /// The second phase over one graph: the canonical lasso of its first
    /// passing component, if some component passes. A reduced graph is read
    /// as the annotated quotient.
    fn second_phase(&self, run: &Explored<'_, '_>) -> Result<Second, ModelCheckRefusal> {
        // A reduced run reads its product as the annotated quotient even
        // under a trivial stabiliser: the quotient is what concretises a
        // reduced run's loop and what decides `each` fairness over orbits.
        if run.binding.applied.is_some() {
            return self.quotient_phase(run);
        }
        let model = FairModel::build(self.system, self.clause, run)?;
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses = match analyze(&model.graph, &all, &model.constraints, self.cancel) {
            Ok(analyses) => analyses,
            Err(FairnessError::Cancelled) => return self.cancelled(),
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
        let walk = model.greedy_loop(&component, passing.removed(), entry)?;
        let loop_edges = model.loop_edges(&walk)?;
        let entry = ProductStateId(entry.index());
        self.lasso(run, entry, |_| Ok(loop_edges))
    }

    fn cancelled(&self) -> Result<Second, ModelCheckRefusal> {
        match self.cancel.cause() {
            Some(source) => Ok(Second::Cancelled(source)),
            None => Err(fault("a-cancelled-run-has-a-cause")),
        }
    }

    /// FR-153: the second phase over a symmetry-reduced graph, on the
    /// annotated quotient. The loop of the first fair component is CX-5's
    /// greedy walk when every constraint is weak and `whole` (EI-6), and
    /// AQ-7's otherwise.
    fn quotient_phase(&self, run: &Explored<'_, '_>) -> Result<Second, ModelCheckRefusal> {
        let mut meter = self.meter.borrow_mut();
        let (quotient, constraints) = match self.quotient(run, &mut meter)? {
            Ok(built) => built,
            Err(stop) => return self.quotient_stop(stop),
        };
        let rename = |transition: &ModelTransition, permutation: &Permutation| {
            self.system.permute_transition(transition, permutation)
        };
        let mut analyzer =
            match Analyzer::new(&quotient, &constraints, &rename, &mut meter, self.cancel) {
                Ok(analyzer) => analyzer,
                Err(stop) => return self.quotient_stop(stop),
            };
        let all: BTreeSet<ProductStateId> =
            (0..quotient.enabled.len()).map(ProductStateId).collect();
        let analyses = match analyzer.analyze(&all) {
            Ok(analyses) => analyses,
            Err(stop) => return self.quotient_stop(stop),
        };
        let Some(fair) = QuotientAnalysis::first_fair_in(&analyses) else {
            return Ok(Second::NoneFair);
        };
        let entry = *fair
            .states
            .first()
            .ok_or_else(|| fault("a-component-has-a-state"))?;
        let greedy = constraints.iter().all(|constraint| {
            constraint.strength == Strength::Weak
                && constraint.granularity == FairnessGranularity::Whole
        });
        self.lasso(run, entry, |sigma| {
            if greedy {
                let model = FairModel::build(self.system, self.clause, run)?;
                let vertices: Vec<Vertex> = model.graph.vertices().collect();
                let vertex = |id: &ProductStateId| {
                    vertices
                        .get(id.index())
                        .copied()
                        .ok_or_else(|| fault("a-state-is-a-vertex"))
                };
                let component = fair
                    .states
                    .iter()
                    .map(vertex)
                    .collect::<Result<BTreeSet<Vertex>, _>>()?;
                // The quotient's own filter removes no edge.
                let walk = model.greedy_loop(&component, &BTreeSet::new(), vertex(&entry)?)?;
                return Ok(model.loop_edges(&walk)?);
            }
            analyzer.fair_loop(fair, sigma).map_err(LoopStop::Quotient)
        })
    }

    /// The annotated quotient of a reduced graph (FR-153 "Inputs") and its
    /// constraints: each operation's identities are those the product names,
    /// closed under the binding's group, which charges `meter`. An identity
    /// of the operation that no reachable state enables is disabled at every
    /// state, so it passes every test and leaves no verdict changed.
    fn quotient(
        &self,
        run: &Explored<'_, '_>,
        meter: &mut Meter,
    ) -> Result<Result<Annotated, QuotientStop>, ModelCheckRefusal> {
        let graph = &run.graph;
        let mut enabled = Vec::with_capacity(graph.states().len());
        let mut edges = Vec::with_capacity(graph.states().len());
        let mut seen: BTreeSet<ModelTransition> = BTreeSet::new();
        for (index, state) in graph.states().iter().enumerate() {
            let mut here = BTreeSet::new();
            for finding in &state.findings {
                if let Finding::Enabled(transitions) = finding {
                    here.extend(transitions.iter().cloned());
                }
            }
            seen.extend(here.iter().cloned());
            enabled.push(here);
            let mut outgoing = Vec::new();
            for edge in graph.outgoing(ProductStateId(index)) {
                let label = match &edge.transition {
                    ProductTransition::Model(transition) => {
                        seen.insert(transition.clone());
                        Some(transition.clone())
                    }
                    ProductTransition::TerminalStutter => None,
                };
                outgoing.push(QuotientEdge {
                    label,
                    target: edge.target,
                    permutation: edge.permutation.clone(),
                });
            }
            edges.push(outgoing);
        }
        let automaton = run.product.automaton.borrow();
        let acceptance = (0..automaton.acceptance_set_count())
            .map(|set| {
                graph
                    .states()
                    .iter()
                    .enumerate()
                    .filter(|(_, state)| {
                        matches!(&state.state.property,
                            Property::After(at) if automaton.is_accepting_in(set, *at))
                    })
                    .map(|(index, _)| ProductStateId(index))
                    .collect()
            })
            .collect();
        let generators = reduce::generators(&run.binding.group);
        let mut constraints = Vec::new();
        for constraint in self.clause.fairness() {
            let operation = constraint.operation();
            let text = self
                .system
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or_else(|| LivenessRefusal::UnknownOperation {
                    name: operation.name().to_owned(),
                })?;
            let mut identities: BTreeSet<ModelTransition> = seen
                .iter()
                .filter(|transition| transition.operation() == text)
                .cloned()
                .collect();
            let mut queue: VecDeque<ModelTransition> = identities.iter().cloned().collect();
            while let Some(identity) = queue.pop_front() {
                for (_, generator) in &generators {
                    if meter.charge(Charge::new(ChargePoint::GraphExpand)).is_err() {
                        return Ok(Err(QuotientStop::Meter));
                    }
                    let image = match self.system.permute_transition(&identity, generator) {
                        Ok(image) => image,
                        Err(stop) => return Ok(Err(QuotientStop::Rename(stop))),
                    };
                    if identities.insert(image.clone()) {
                        queue.push_back(image);
                    }
                }
            }
            constraints.push(QuotientConstraint {
                strength: Strength::from(constraint.kind()),
                granularity: constraint.granularity(),
                identities,
            });
        }
        Ok(Ok((
            Quotient {
                enabled,
                edges,
                acceptance,
            },
            constraints,
        )))
    }

    /// The outcome of an analysis that stopped: the meter settles V-7
    /// naming it (FR-153 "Limits").
    fn quotient_stop(&self, stop: QuotientStop) -> Result<Second, ModelCheckRefusal> {
        match stop {
            QuotientStop::Meter => Ok(Second::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::EvaluationMeter,
                    value: self.request.meter.work_units,
                }),
            )),
            QuotientStop::Cancelled => self.cancelled(),
            QuotientStop::Rename(stop) => match self.ending(Halt::Expansion(stop))? {
                Ending::Stopped(cause, limit) => Ok(Second::Stopped(cause, limit)),
                Ending::Complete => Err(fault("an-expansion-stop-is-a-stop")),
            },
            QuotientStop::Fault(invariant) => Err(fault(invariant)),
        }
    }

    /// The lasso to the component entry `entry`: the stem is the
    /// breadth-first tree path, and `pass` gives one pass of the loop from
    /// the entry, given `σ` there. Both are concretised, and the loop is
    /// repeated until the concrete state equals the concrete entry state,
    /// which it does within the order of the pass's word (FR-161 "Loop
    /// closure"); every pass polls the cancellation and every concrete step
    /// spends `max_transitions`, so no pass count is computed up front.
    fn lasso(
        &self,
        run: &Explored<'_, '_>,
        entry: ProductStateId,
        pass: impl FnOnce(&Permutation) -> Result<Vec<LoopEdge>, LoopStop>,
    ) -> Result<Second, ModelCheckRefusal> {
        let (root, stem) = tree_path(&run.graph, entry);
        let lifted = (|| -> Result<TemporalCounterexample, LoopStop> {
            let (initial_state, mut lift) = self.lift_start(run, root)?;
            self.lift_edges(run, &mut lift, &stem)?;
            let entry_index = lift.len();
            let entry_state = lift.digest();
            let loop_edges = pass(lift.sigma())?;
            let mut edges = Vec::with_capacity(loop_edges.len());
            for (source, position) in loop_edges {
                let edge = run
                    .graph
                    .outgoing(source)
                    .nth(position)
                    .ok_or_else(|| fault("a-loop-edge-is-retained"))?;
                edges.push(edge);
            }
            // A terminal model state has only its stutter edge, so a loop
            // holds stutter edges alone or model edges alone.
            let stutters = edges
                .iter()
                .filter(|edge| edge.transition == ProductTransition::TerminalStutter)
                .count();
            let terminal_stutter = stutters > 0;
            if terminal_stutter && stutters != edges.len() {
                return Err(
                    LiftStop::from(fault("a-loop-is-all-stutter-or-all-model-steps")).into(),
                );
            }
            // Each pass of a reduced loop with a model step spends the
            // concrete-step budget, so the repetition ends at the concrete
            // entry or at `max_transitions`; a concrete (trivial-group) loop
            // closes after one pass.
            loop {
                if self.cancel.poll() && self.cancel.cause().is_some() {
                    return Err(LoopStop::Quotient(QuotientStop::Cancelled));
                }
                self.lift_edges(run, &mut lift, &edges)?;
                if lift.digest() == entry_state {
                    break;
                }
                if run.binding.group.is_trivial() {
                    return Err(LiftStop::from(fault("a-concrete-loop-closes-in-one-pass")).into());
                }
            }
            Ok(TemporalCounterexample {
                initial_state,
                prefix: lift.into_steps(),
                over_binding: Some(run.binding.instance.clone()),
                fairness: self.clause.fairness().to_vec(),
                kind: CounterexampleKind::Formula,
                lasso: Some(Lasso {
                    entry: entry_index,
                    terminal_stutter,
                }),
                blocked: Vec::new(),
                refinement: None,
            })
        })();
        match lifted {
            Ok(counterexample) => self.built(Ok(counterexample)).map(Second::from),
            Err(LoopStop::Lift(stop)) => self.built(Err(stop)).map(Second::from),
            Err(LoopStop::Quotient(stop)) => self.quotient_stop(stop),
        }
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
}

impl FairModel {
    fn build(
        system: &ModelSystem<'_>,
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
        let mut constraints = Vec::new();
        for constraint in clause.fairness() {
            let strength = Strength::from(constraint.kind());
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
                    constraints.push(Constraint::new(strength, members));
                }
                FairnessGranularity::Each => {
                    for transition in covered {
                        constraints.push(Constraint::new(strength, [identity(transition)?]));
                    }
                }
            }
        }
        Ok(Self {
            graph: fair,
            constraints,
        })
    }

    /// The retained edges of a walk over the graph, each as its source and
    /// its position among the source's edges (the graph's edges are added in
    /// the retained graph's canonical successor order). A parallel edge with
    /// the same label and target is the first such.
    fn loop_edges(&self, walk: &[(Vertex, Edge)]) -> Result<Vec<LoopEdge>, ModelCheckRefusal> {
        walk.iter()
            .map(|(source, edge)| {
                self.graph
                    .edges_from(*source)
                    .iter()
                    .position(|held| held == edge)
                    .map(|position| (ProductStateId(source.index()), position))
                    .ok_or_else(|| fault("a-walk-edge-is-a-graph-edge"))
            })
            .collect()
    }

    /// ADR-018 CX-5's greedy walk inside `component` from its entry `entry`:
    /// the loop's edges, each with the vertex it leaves. The walk takes no
    /// edge whose identity is in `removed`, the edges the filter's
    /// visibility refinements left out of the passing component (ADR-025
    /// MF-2).
    fn greedy_loop(
        &self,
        component: &BTreeSet<Vertex>,
        removed: &BTreeSet<Identity>,
        entry: Vertex,
    ) -> Result<Vec<(Vertex, Edge)>, ModelCheckRefusal> {
        let inside = Inside { component, removed };
        let mut open = Open::of(self, &inside);
        open.discharge_vertex(self, entry);
        let mut walk = Vec::new();
        let mut current = entry;
        while !open.is_empty() {
            let path = self
                .nearest(&inside, current, &open)
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
                .shortest_return(&inside, current, entry)
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
        inside: &Inside<'_>,
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
            if best.is_some_and(|length| reached > length) {
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
                if !inside.keeps(edge) {
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
        inside: &Inside<'_>,
        from: Vertex,
        entry: Vertex,
    ) -> Option<Vec<(Vertex, Edge)>> {
        let mut parent: BTreeMap<Vertex, (Vertex, usize)> = BTreeMap::new();
        let mut seen: BTreeSet<Vertex> = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(vertex) = queue.pop_front() {
            for (position, edge) in self.graph.edges_from(vertex).iter().enumerate() {
                if !inside.keeps(edge) {
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

/// A passing component's subgraph: its vertices, and the edge identities
/// the filter removed from it.
struct Inside<'a> {
    component: &'a BTreeSet<Vertex>,
    removed: &'a BTreeSet<Identity>,
}

impl Inside<'_> {
    /// Whether the walk may take `edge` out of a vertex of the component.
    fn keeps(&self, edge: &Edge) -> bool {
        self.component.contains(&edge.target)
            && match edge.label {
                EdgeLabel::Transition(identity) => !self.removed.contains(&identity),
                EdgeLabel::Stutter => true,
            }
    }
}

/// The obligations of a component not yet discharged (CX-5, ADR-019 FS-7):
/// one state of each acceptance set, for each weak constraint an edge that
/// takes it or a state that disables it, for each strong constraint enabled
/// at some state of the component an edge that takes it, and for each
/// visibility constraint a kept edge of the component loads (ADR-025 MF-2)
/// an edge that takes it.
struct Open {
    sets: BTreeSet<usize>,
    constraints: BTreeSet<usize>,
}

impl Open {
    /// The obligations of the component `inside`: a strong constraint
    /// enabled at no state of it, and a visibility constraint whose loads no
    /// kept edge of it takes, have none.
    fn of(model: &FairModel, inside: &Inside<'_>) -> Self {
        let owed = |constraint: &Constraint| match constraint.obligation() {
            Obligation::Enablement(Strength::Weak) => true,
            Obligation::Enablement(Strength::Strong) => inside.component.iter().any(|vertex| {
                model
                    .graph
                    .enabled(*vertex)
                    .and_then(|enabled| constraint.is_enabled_in(enabled))
                    == Some(true)
            }),
            Obligation::Visibility { .. } => inside.component.iter().any(|vertex| {
                model
                    .graph
                    .edges_from(*vertex)
                    .iter()
                    .any(|edge| inside.keeps(edge) && constraint.is_owed_by(edge.label))
            }),
        };
        Self {
            sets: (0..model.graph.acceptance_sets().len()).collect(),
            constraints: model
                .constraints
                .iter()
                .enumerate()
                .filter(|(_, constraint)| owed(constraint))
                .map(|(index, _)| index)
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.sets.is_empty() && self.constraints.is_empty()
    }

    /// Whether `vertex` discharges the obligation of `constraint`: a weak
    /// constraint is disabled there. A strong constraint is never discharged
    /// by a state.
    fn disabled_at(model: &FairModel, constraint: usize, vertex: Vertex) -> bool {
        match (
            model.constraints.get(constraint),
            model.graph.enabled(vertex),
        ) {
            // Only an enablement weak constraint is discharged by a state.
            (Some(constraint), Some(enabled)) => {
                constraint.obligation() == &Obligation::Enablement(Strength::Weak)
                    && constraint.is_enabled_in(enabled) == Some(false)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// UE-4: a model state the product knows at depth 3 is news again when
    /// the model-only pass reaches it at depth 1, but not at depth 1 twice or
    /// at depth 2 after that.
    #[trace("TC-521")]
    #[test]
    fn a_shallower_arrival_at_a_known_model_state_is_news() {
        use qsl_foundation::digest::DigestDomain;
        let digest = DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [7; 32]);
        let mut depths = Depths::default();
        assert!(depths.reach(digest, 3), "a new state");
        assert!(!depths.reach(digest, 3), "no shallower");
        assert!(!depths.reach(digest, 4), "deeper");
        assert!(depths.reach(digest, 1), "shallower");
        assert!(!depths.reach(digest, 2), "deeper than the shallowest");
    }

    /// CX-5 tie: from the entry v0, edge 0 reaches v1 (an acceptance-set
    /// state) and edge 1 takes the weak constraint; both obligations are at
    /// distance 1 and edge 0 is canonically first, so the walk is
    /// v0 v1 v0 v2 v0, not the edge obligation first.
    #[trace("TC-521")]
    #[test]
    fn a_vertex_obligation_ties_an_edge_obligation_at_the_same_distance() {
        let id0 = Identity::new(0);
        let id1 = Identity::new(1);
        let mut graph = Graph::new();
        let v0 = graph.add_vertex([id0, id1]);
        let v1 = graph.add_vertex([id0, id1]);
        let v2 = graph.add_vertex([id0, id1]);
        for (from, to, label) in [(v0, v1, id0), (v0, v2, id1), (v1, v0, id0), (v2, v0, id0)] {
            graph
                .add_edge(from, to, EdgeLabel::Transition(label))
                .expect("both vertices exist");
        }
        graph.add_acceptance_set([v1]).expect("v1 exists");
        let model = FairModel {
            graph,
            constraints: vec![Constraint::weak([id1])],
        };
        let component: BTreeSet<Vertex> = [v0, v1, v2].into_iter().collect();
        let walk = model
            .greedy_loop(&component, &BTreeSet::new(), v0)
            .expect("a walk exists");
        let targets: Vec<usize> = walk.iter().map(|(_, edge)| edge.target.index()).collect();
        assert_eq!(targets, [1, 0, 2, 0]);
    }

    /// ADR-025 MF-2 in CX-5: thread `b`'s loads of `l` are identities 10
    /// (stale) and 11 (mo-last); 20 is another step. The accepting `v0` has a
    /// stale self-loop, canonically its first edge, and the cycle
    /// `v0 -> v1 -> v0` loads nothing. The filter removes the stale load and
    /// passes the rest, and the walk takes only the kept edges: never the
    /// stale self-loop, whose loop would be unfair.
    #[trace("TC-673", "FR-228-AC-6")]
    #[test]
    fn the_walk_takes_no_edge_a_visibility_refinement_removed() {
        let (stale, fresh, other) = (Identity::new(10), Identity::new(11), Identity::new(20));
        let mut graph = Graph::new();
        let v0 = graph.add_vertex([stale, fresh, other]);
        let v1 = graph.add_vertex([other]);
        for (from, to, label) in [(v0, v0, stale), (v0, v1, other), (v1, v0, other)] {
            graph
                .add_edge(from, to, EdgeLabel::Transition(label))
                .expect("both vertices exist");
        }
        graph.add_acceptance_set([v0]).expect("v0 exists");
        let model = FairModel {
            graph,
            constraints: vec![Constraint::visibility([stale, fresh], [fresh])],
        };
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses =
            analyze(&model.graph, &all, &model.constraints, &Cancel::new()).expect("the filter");
        let passing = Analysis::first_fair_in(&analyses).expect("a fair component");
        assert!(passing.removed().contains(&stale));
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let walk = model
            .greedy_loop(&component, passing.removed(), v0)
            .expect("a walk exists");
        let labels: Vec<EdgeLabel> = walk.iter().map(|(_, edge)| edge.label).collect();
        assert_eq!(
            labels,
            [EdgeLabel::Transition(other), EdgeLabel::Transition(other)]
        );
    }

    /// ADR-025 MF-2: a stale self-loop on an accepting state, with nothing
    /// else, holds no fair cycle, so the filter yields no counterexample.
    #[trace("TC-673", "FR-228-AC-3")]
    #[test]
    fn a_stale_self_loop_on_an_accepting_state_is_no_counterexample() {
        let (stale, fresh) = (Identity::new(10), Identity::new(11));
        let mut graph = Graph::new();
        let v0 = graph.add_vertex([stale, fresh]);
        graph
            .add_edge(v0, v0, EdgeLabel::Transition(stale))
            .expect("v0 exists");
        graph.add_acceptance_set([v0]).expect("v0 exists");
        let constraints = [Constraint::visibility([stale, fresh], [fresh])];
        let analyses = analyze(&graph, &BTreeSet::from([v0]), &constraints, &Cancel::new())
            .expect("the filter");
        assert!(Analysis::first_fair_in(&analyses).is_none());
    }

    /// FR-228-AC-8: a strong constraint (identity 9, enabled only at state
    /// 0) and a failing pair (stale `1 -> 2`) refine in one step; the second
    /// pair (stale `3 -> 3`, fresh `3 -> 4`) still loads `{3, 4}`, so the
    /// walk from the accepting state 3 discharges it by the fresh load and
    /// the loop is `3 -> 4 -> 3`.
    #[trace("TC-673", "FR-228-AC-8")]
    #[test]
    fn the_walk_discharges_a_pair_the_passing_component_still_loads() {
        let id = Identity::new;
        let mut graph = Graph::new();
        let vertices: Vec<Vertex> = (0..5)
            .map(|index| graph.add_vertex(if index == 0 { vec![id(9)] } else { vec![] }))
            .collect();
        for (from, to, label) in [
            (0, 1, 20),
            (1, 0, 21),
            (1, 2, 10),
            (2, 3, 22),
            (3, 1, 23),
            (3, 3, 12),
            (3, 4, 13),
            (4, 3, 24),
        ] {
            graph
                .add_edge(
                    vertices[from],
                    vertices[to],
                    EdgeLabel::Transition(id(label)),
                )
                .expect("both vertices exist");
        }
        graph
            .add_acceptance_set([vertices[3]])
            .expect("vertex 3 exists");
        let model = FairModel {
            graph,
            constraints: vec![
                Constraint::strong([id(9)]),
                Constraint::visibility([id(10)], [id(11)]),
                Constraint::visibility([id(12), id(13)], [id(13)]),
            ],
        };
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses =
            analyze(&model.graph, &all, &model.constraints, &Cancel::new()).expect("the filter");
        let passing = Analysis::first_fair_in(&analyses).expect("a fair component");
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        assert_eq!(component, BTreeSet::from([vertices[3], vertices[4]]));
        let walk = model
            .greedy_loop(&component, passing.removed(), vertices[3])
            .expect("a walk exists");
        let path: Vec<(usize, usize)> = walk
            .iter()
            .map(|(from, edge)| (from.index(), edge.target.index()))
            .collect();
        assert_eq!(path, [(3, 4), (4, 3)]);
    }
}
