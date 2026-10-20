// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176, FR-177 (ADR-023 HC-1, HC-2, HC-4, HC-8, HM-7; ADR-018 EN-1): the
//! explicit-state check of a universal hyperproperty (HP-2) by
//! self-composition and of a forall-exists safety hyperproperty (HP-3) by
//! witness sets.
//!
//! [`check_hyper`] decides a `hyper` clause over `behaviours` whose
//! quantifiers are all `forall` (HP-2), or `forall`s then `exists`s over a
//! safety body (HP-3). Before any product is built it classifies
//! every root of every subject and explores each distinct subject's reachable
//! graph once under the request's limits, so a subject that alone reaches a
//! limit stops the item there (HC-8). It then explores, for each instance of
//! the clause's object parameters, the [`HyperProduct`] of its subjects with
//! the generalized Büchi automaton of the negated body, retaining every edge
//! (HC-1), in FR-101's canonical order.
//!
//! The first phase is a scan of the retained states in discovery order for the
//! first whose letter is undefined, whose expansion left a joint step on which
//! `μ_U` has no value, or whose component expansion evaluated a contract
//! conjunction undefined (ADR-023 HV-8): the counterexample ends there, one
//! path per trace variable. A refused contract or `match` there leaves the
//! item undecided (`UndecidedSuccessor`, `MatchUndetermined`). Instances are
//! ranked across as FR-126 ranks bindings: the shallowest finding wins, a tie
//! going to the earlier instance.
//!
//! The second phase runs for every body, a safety body included (HC-4): the
//! fairness kernels decompose the retained graph into strongly connected
//! components and pass one that is non-trivial, holds a state of every
//! acceptance set and satisfies every constraint, and the canonical lasso
//! (ADR-018 CX-5) is split into one trace per variable. A constraint of
//! variable `k` is enabled where component `k` enables its operation and is
//! taken by a joint step whose component `k` step is one of its operation
//! (HC-4), so the kernels are the single-model ones, unchanged. A passing
//! component of the shortest lasso across instances is the counterexample.
//!
//! A run whose product has no fair accepting component holds, unless the
//! clause's `μ_U` is non-empty and the product has no fair cycle at all, with
//! the acceptance sets set aside: no tuple of fair behaviours is matched at
//! every joint step, and the item is vacuous (HM-7), never proved.
//!
//! An HP-3 clause is explored as a [`WitnessProduct`] instead: a state is the
//! universal tuple with the set `X` of existential tuples still matched, and
//! the second phase is a scan, over the states whose `X` is empty, for a
//! cycle that is fair for every universal variable (HC-2). The first state in
//! discovery order from which one is reachable ends the run: the universal
//! lasso through it is the counterexample, `position` the first joint
//! position at which `X` is empty. An undefined evaluation, a refused `match`
//! and that violation are ranked by the joint position they are at, in that
//! order on a tie: a refused `match` at or before the empty `X` may have
//! dropped a member of it. An undefined evaluation ends at one path for every
//! universal and existential variable, an existential variable's path being
//! its member's [`Partner`].
//!
//! Not built here, and refused or absent rather than approximated: the
//! product-closure certificate (FR-163), so a `Holds` carries none and settles
//! no `proved`; a search horizon `max_depth` (HV-3); the copy-swap reduction
//! (FR-174); and the facade entry that replays a counterexample from a
//! recompiled package, which waits for S4 to emit a hyper clause. The core of
//! that replay is `qsl_replay::replay_hyper_trace_tuple`.

use std::cell::{Cell, Ref};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use qsl_eval::automaton::{AutomatonError, PropertyAutomaton};
use qsl_eval::fairness::{
    analyze, Analysis, Constraint, EdgeLabel, FairnessError, Graph, Identity,
    Outcome as FairnessOutcome, Strength, Vertex,
};
use qsl_eval::hyper::{
    cartesian, ComponentStep, Halt, HyperFinding, HyperInputs, HyperLetters, HyperProduct,
    HyperState, HyperTransition, ParameterBinding, Partner, PartnerStep, Property, StepMatch,
    WitnessInputs, WitnessProduct,
};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{
    check_model_bounds, ModelFinding, ModelKey, ModelSimState, ModelSystem,
};
use qsl_eval::simulation::{
    Expansion, ExpansionLimit, ExpansionStop, Limit, Limits, Outcome, TransitionSystem,
};
use qsl_eval::temporal::{
    evaluate_tuple, AtomReading, ComponentPosition, ComponentTrace, LetterStop, TemporalOutcome,
    TemporalPosition, TupleLetters,
};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::{IdentityLimits, InternalFault};
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep, HyperTrace,
    HyperUndefined, IncompleteCause, ModelCheckLimit, ModelCheckLimits, ModelStep, ReachedLimit,
    RunStats, UndecidedCause,
};
use qsl_semantics::check::{CheckedHyperClause, FormulaNodeId, HyperForm};
use quire_exact::{Cancel, Meter, ScalarLimits, ValueType};
use quire_semantic_value::fairness::FairnessGranularity;
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{fault, not_simulated, widen, ModelCheckRefusal, BOUND_POSITIONS};
use super::fair_walk::{tree_path, FairModel};
use super::graph::{retain, GraphExploration, ProductStateId, RetainedEdge, RetainedGraph};

type HyperGraph = RetainedGraph<HyperState, HyperTransition, HyperFinding>;

/// One EN-1 run over an HP-2 or HP-3 item (FR-176, FR-177 Inputs).
pub struct HyperCheckRequest<'a> {
    /// The checked clause: lockstep, every quantifier `forall` (HP-2) or
    /// `forall`s then `exists`s over a safety body (HP-3).
    pub clause: &'a CheckedHyperClause,
    /// One model subject per distinct alias, in the order of
    /// [`CheckedHyperClause::aliases`]. Two variables of one alias range over
    /// the same subject.
    pub subjects: Vec<ModelSubject<'a>>,
    /// The seam to the body's `holds` atoms.
    pub letters: &'a dyn HyperLetters,
    /// The seam to the clause's `match` conjuncts.
    pub matcher: &'a dyn StepMatch,
    /// The run's caller-raisable budgets (FR-184).
    pub limits: ModelCheckLimits,
    /// The evaluation meter budget of each atom and each `μ_U` evaluation.
    pub meter: ScalarLimits,
}

/// Why [`check_hyper`] explored nothing, or built no counterexample
/// (FR-176, FR-177).
#[derive(Debug, thiserror::Error)]
pub enum HyperRefusal {
    /// The clause is not an HP-2 or HP-3 clause.
    #[error("EN-1 decides a universal (HP-2) or forall-exists safety (HP-3) hyperproperty here, not {form:?}")]
    FormNotDecided {
        /// The clause's form.
        form: Option<HyperForm>,
    },
    /// The request does not supply one subject per alias of the clause.
    #[error("the clause names {expected} model aliases and the request supplies {found} subjects")]
    SubjectCount {
        /// The clause's distinct aliases.
        expected: usize,
        /// The subjects supplied.
        found: usize,
    },
    /// The negated body has no property automaton.
    #[error(transparent)]
    Automaton(#[from] AutomatonError),
    /// An undefined evaluation of an HP-3 clause's universal side, where
    /// FR-177 and QSpec FR-400 give the counterexample no path for the
    /// existential variables.
    #[error(
        "an undefined evaluation on the universal side of an HP-3 clause names no existential path"
    )]
    NoPartnerPath,
    /// A fairness constraint names an operation its variable's subject does
    /// not declare.
    #[error("the fairness constraint on `{name}` names an operation the subject does not declare")]
    UnknownOperation {
        /// The operation's resolved name.
        name: String,
    },
}

/// Decide an HP-2 item (FR-176).
///
/// # Errors
///
/// [`ModelCheckRefusal`] as [`check_model`](super::check_model) returns it
/// for a subject that does not admit or has an unbounded root, and
/// [`HyperRefusal`] for a clause this engine does not decide.
pub fn check_hyper(
    request: HyperCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<HyperOutcome, ModelCheckRefusal> {
    let clause = request.clause;
    if !matches!(
        clause.form(),
        Some(HyperForm::Universal | HyperForm::ForallExistsSafety)
    ) {
        return Err(HyperRefusal::FormNotDecided {
            form: clause.form().cloned(),
        }
        .into());
    }
    if request.subjects.len() != clause.aliases().len() {
        return Err(HyperRefusal::SubjectCount {
            expected: clause.aliases().len(),
            found: request.subjects.len(),
        }
        .into());
    }
    let systems = request
        .subjects
        .iter()
        .map(|subject| {
            ModelSystem::new(subject.simulation.clone())
                .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for system in &systems {
        check_model_bounds(system, BOUND_POSITIONS).map_err(not_simulated)?;
    }
    let slot_of = |alias: &str| {
        clause
            .aliases()
            .iter()
            .position(|known| known.as_str() == alias)
            .ok_or_else(|| fault("a-quantifier-names-an-alias-of-the-clause"))
    };
    let slots = clause
        .quantifiers()
        .iter()
        .map(|quantifier| slot_of(quantifier.alias.as_str()))
        .collect::<Result<Vec<_>, _>>()?;
    let instances = instances(clause, &systems, &slot_of)?;
    let run = Run {
        request: &request,
        systems,
        slots,
        witness: matches!(clause.form(), Some(HyperForm::ForallExistsSafety)),
        cancel,
    };
    run.decide(&instances)
}

/// The instances of the clause's object parameters: one binding of each
/// parameter to a key of its population's universe, over the product of the
/// universes in parameter order (ADR-023 HS-3).
fn instances(
    clause: &CheckedHyperClause,
    systems: &[ModelSystem<'_>],
    slot_of: &impl Fn(&str) -> Result<usize, ModelCheckRefusal>,
) -> Result<Vec<Vec<ParameterBinding>>, ModelCheckRefusal> {
    let mut lists = Vec::with_capacity(clause.parameters().len());
    for parameter in clause.parameters() {
        let system = systems
            .get(slot_of(parameter.alias.as_str())?)
            .ok_or_else(|| fault("a-parameter-names-an-alias-of-the-clause"))?;
        let keys = system
            .instances(&ValueType::Reference(parameter.population.object()))
            .map_err(not_simulated)?;
        lists.push(
            keys.into_iter()
                .map(|instance| ParameterBinding {
                    parameter: parameter.name.clone(),
                    instance,
                })
                .collect::<Vec<_>>(),
        );
    }
    Ok(cartesian(&lists))
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
    /// What is left of `max_states` and `max_transitions`.
    fn remaining(&self, limits: Limits) -> Limits {
        let left = |limit: usize, used: u64| {
            limit.saturating_sub(usize::try_from(used).unwrap_or(usize::MAX))
        };
        Limits {
            max_states: left(limits.max_states, self.states),
            max_transitions: left(limits.max_transitions, self.edges),
        }
    }

    fn stats(&self) -> RunStats {
        RunStats {
            product_states: self.states,
            product_edges: self.edges,
            automaton_states: self.automaton_states,
            depth: self.depth,
        }
    }

    fn add(&mut self, graph: &HyperGraph, edges: usize, automaton: u64) {
        self.states = self.states.saturating_add(widen(graph.states().len()));
        self.edges = self.edges.saturating_add(widen(edges));
        // Every instance translates the same body into the same automaton.
        self.automaton_states = self.automaton_states.max(automaton);
        let depth = graph.states().iter().map(|s| s.depth).max().unwrap_or(0);
        self.depth = self.depth.max(widen(depth));
    }
}

fn stopped(
    cause: IncompleteCause,
    limit: Option<ReachedLimit>,
    count: Option<u64>,
    totals: &Totals,
) -> HyperOutcome {
    HyperOutcome::Stopped {
        cause,
        limit,
        count,
        stats: totals.stats(),
    }
}

fn exhausted(limit: ModelCheckLimit, value: u64, totals: &Totals) -> HyperOutcome {
    stopped(
        IncompleteCause::ResourceExhausted,
        Some(ReachedLimit { limit, value }),
        None,
        totals,
    )
}

/// A subject's own system, recording why an expansion stopped: the engine
/// reports a stop by the limit it ran out of, which an `ExpansionStop`
/// carries and an exploration `Outcome` does not.
struct Watched<'r, 's> {
    system: &'r ModelSystem<'s>,
    stop: Cell<Option<ExpansionStop>>,
}

impl TransitionSystem for Watched<'_, '_> {
    type State = ModelSimState;
    type TransitionId = ModelTransition;
    type Key = ModelKey;
    type Finding = ModelFinding;

    fn initial(&self) -> Vec<Self::State> {
        self.system.initial()
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        self.system.key(state)
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        self.system.successors(state).inspect_err(|&stop| {
            self.stop.set(Some(stop));
        })
    }
}

/// How a product's exploration ended.
enum Ending {
    /// The frontier emptied.
    Complete,
    /// A limit or a cancellation stopped the run; the count is the one a
    /// counted budget reached (FR-184).
    Stopped(IncompleteCause, Option<ReachedLimit>, Option<u64>),
}

/// The product of one instance: the self-composition of an HP-2 clause or the
/// witness-set product of an HP-3 clause.
enum Built<'r, 's> {
    Universal(HyperProduct<'r, 's>),
    Witness(WitnessProduct<'r, 's>),
}

impl Built<'_, '_> {
    fn take_halt(&self) -> Option<Halt> {
        match self {
            Self::Universal(product) => product.take_halt(),
            Self::Witness(product) => product.take_halt(),
        }
    }

    fn automaton(&self) -> Ref<'_, PropertyAutomaton> {
        match self {
            Self::Universal(product) => product.automaton(),
            Self::Witness(product) => product.automaton(),
        }
    }
}

/// One explored instance.
struct Explored<'r, 's> {
    instance: &'r [ParameterBinding],
    product: Built<'r, 's>,
    graph: HyperGraph,
}

/// What the first phase found at a state.
enum Found {
    /// The state's own letter has an undefined atom; for an HP-3 clause, at
    /// the joint position of the existential tuple that `partner` reached.
    Atom {
        atom: FormulaNodeId,
        cause: qsl_eval::temporal::UndefinedCause,
        partner: Option<Rc<Partner>>,
    },
    /// `μ_E` has no value on a joint step of an HP-3 member of the state.
    Existential {
        step: HyperTransition,
        posts: Vec<DigestRecord>,
        conjunct: usize,
        cause: qsl_eval::temporal::UndefinedCause,
        partner: Rc<Partner>,
    },
    /// `μ_U` has no value on a joint step out of the state.
    Match {
        step: HyperTransition,
        posts: Vec<DigestRecord>,
        conjunct: usize,
        cause: qsl_eval::temporal::UndefinedCause,
    },
    /// A component's contract conjunction is undefined at the state.
    Contract {
        component: usize,
        clause: String,
        guard_of: Option<ModelTransition>,
        cause: qsl_eval::temporal::UndefinedCause,
    },
}

/// A first-phase finding before its counterexample is built.
enum Candidate {
    /// An undefined evaluation at a state.
    Found(ProductStateId, Found),
    /// A refused contract or `match`.
    Refused(UndecidedCause),
}

/// The first phase's finding.
enum FirstHit {
    Violated(Box<HyperCounterexample>),
    Undecided(UndecidedCause),
}

/// The second phase's result for one instance.
enum Second {
    /// The shortest passing lasso, with its length in joint steps.
    Lasso(Box<HyperCounterexample>, usize),
    NoneFair,
    Stopped(HyperOutcome),
}

/// The second phase's result for one HP-3 instance.
enum Exhausted {
    /// The universal lasso through the first state whose witness set is
    /// empty and from which a fair cycle is reachable, with the joint
    /// position at which the set is first empty.
    Lasso(Box<HyperCounterexample>, usize),
    NoneFair,
    Stopped(HyperOutcome),
}

/// What decided an HP-3 item.
enum Verdict {
    Violated(Box<HyperCounterexample>),
    Undecided(UndecidedCause),
}

/// The existential half of an undefined-evaluation counterexample's paths: the
/// partner's path, then the existential components of the last joint step.
struct PartnerPath<'p> {
    partner: &'p Partner,
    last: Option<PartnerStep<'p>>,
}

struct Run<'r, 'a> {
    request: &'r HyperCheckRequest<'a>,
    systems: Vec<ModelSystem<'a>>,
    /// The index in `systems` of each trace variable's subject.
    slots: Vec<usize>,
    /// Whether the clause is HP-3: its leading `universals` variables are
    /// universal and the rest existential.
    witness: bool,
    cancel: &'r Cancel,
}

impl<'a> Run<'_, 'a> {
    fn decide(
        &self,
        instances: &[Vec<ParameterBinding>],
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        let mut totals = Totals::default();
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Ok(stopped(
                    IncompleteCause::Cancelled { source },
                    None,
                    None,
                    &totals,
                ));
            }
        }
        if self
            .request
            .subjects
            .iter()
            .any(|subject| subject.simulation.initial.is_empty())
        {
            return Ok(HyperOutcome::Undecided {
                cause: UndecidedCause::NoInitialState,
                stats: totals.stats(),
            });
        }
        if instances.is_empty() {
            return Ok(HyperOutcome::Undecided {
                cause: UndecidedCause::NoInstance,
                stats: totals.stats(),
            });
        }
        if let Some(outcome) = self.precheck(&totals)? {
            return Ok(outcome);
        }
        if self.witness {
            return self.decide_witness(instances, totals);
        }
        let mut explored = Vec::with_capacity(instances.len());
        let mut best: Option<(usize, FirstHit)> = None;
        for instance in instances {
            // FR-126 "Bindings and ranking": a later instance wins only with
            // a strictly shallower finding, so it is explored only through
            // the depth that finding would need (every state shallower than
            // the best is expanded) and not at all past a finding at depth 0.
            let cap = match best {
                Some((0, _)) => break,
                Some((depth, _)) => Some(depth),
                None => None,
            };
            let run = match self.explore(instance, cap, &mut totals)? {
                Ok(run) => run,
                Err(outcome) => return Ok(outcome),
            };
            if let Some((depth, hit)) = self.first_phase(&run)? {
                if best.as_ref().is_none_or(|(known, _)| depth < *known) {
                    best = Some((depth, hit));
                }
            }
            explored.push(run);
        }
        if let Some((_, hit)) = best {
            return Ok(match hit {
                FirstHit::Violated(counterexample) => HyperOutcome::Violated {
                    counterexample,
                    stats: totals.stats(),
                },
                FirstHit::Undecided(cause) => HyperOutcome::Undecided {
                    cause,
                    stats: totals.stats(),
                },
            });
        }
        let mut shortest: Option<(Box<HyperCounterexample>, usize)> = None;
        for run in &explored {
            match self.second_phase(run, &totals)? {
                Second::Lasso(counterexample, length) => {
                    if shortest.as_ref().is_none_or(|(_, known)| length < *known) {
                        shortest = Some((counterexample, length));
                    }
                }
                Second::NoneFair => {}
                Second::Stopped(outcome) => return Ok(outcome),
            }
        }
        if let Some((counterexample, _)) = shortest {
            return Ok(HyperOutcome::Violated {
                counterexample,
                stats: totals.stats(),
            });
        }
        self.clean(&explored, &totals)
    }

    /// Decide an HP-3 item (FR-177): for each instance, the first phase's
    /// undefined evaluations and refused `match`es, and the second phase's
    /// empty witness sets with a fair universal cycle, ranked together by the
    /// joint position they are at (ADR-023 HV-8). A later instance wins only
    /// with a strictly earlier finding.
    fn decide_witness(
        &self,
        instances: &[Vec<ParameterBinding>],
        mut totals: Totals,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        let mut explored = Vec::with_capacity(instances.len());
        // The least (position, rank) so far: an undefined evaluation ranks
        // before a refused `match` at one position, and a refused `match`
        // before an empty witness set, which it may have dropped a member of.
        let mut best: Option<(usize, u8, Verdict)> = None;
        let mut offer = |position: usize, rank: u8, verdict: Verdict| {
            if best
                .as_ref()
                .is_none_or(|(known, known_rank, _)| (position, rank) < (*known, *known_rank))
            {
                best = Some((position, rank, verdict));
            }
        };
        for instance in instances {
            let run = match self.explore(instance, None, &mut totals)? {
                Ok(run) => run,
                Err(outcome) => return Ok(outcome),
            };
            match self.first_phase(&run)? {
                Some((depth, FirstHit::Violated(counterexample))) => {
                    offer(depth, 0, Verdict::Violated(counterexample));
                }
                Some((depth, FirstHit::Undecided(cause))) => {
                    offer(depth, 1, Verdict::Undecided(cause));
                }
                None => {}
            }
            match self.exhaustion(&run, &totals)? {
                Exhausted::Lasso(counterexample, position) => {
                    offer(position, 2, Verdict::Violated(counterexample));
                }
                Exhausted::NoneFair => {}
                Exhausted::Stopped(outcome) => return Ok(outcome),
            }
            explored.push(run);
        }
        match best {
            Some((_, _, Verdict::Violated(counterexample))) => Ok(HyperOutcome::Violated {
                counterexample,
                stats: totals.stats(),
            }),
            Some((_, _, Verdict::Undecided(cause))) => Ok(HyperOutcome::Undecided {
                cause,
                stats: totals.stats(),
            }),
            None => self.clean(&explored, &totals),
        }
    }

    /// The second phase over one HP-3 instance (ADR-023 HC-2): among the
    /// states whose witness set is empty, which is absorbing, the first in
    /// discovery order from which a cycle that is fair for every universal
    /// variable is reachable. The universal lasso is the tree path to it, the
    /// shortest path on to the nearest state of a fair component and that
    /// component's canonical loop (ADR-018 CX-5), with its entry moved back
    /// while the lasso it describes does not change.
    fn exhaustion(
        &self,
        run: &Explored<'_, '_>,
        totals: &Totals,
    ) -> Result<Exhausted, ModelCheckRefusal> {
        let graph = &run.graph;
        let is_empty = |id: ProductStateId| {
            graph.state(id).is_some_and(
                |state| matches!(&state.state.property, Property::Witnesses(set) if set.is_empty()),
            )
        };
        let empty: BTreeSet<ProductStateId> = (0..graph.states().len())
            .map(ProductStateId)
            .filter(|id| is_empty(*id))
            .collect();
        if empty.is_empty() {
            return Ok(Exhausted::NoneFair);
        }
        let (base, constraints) = self.fair_graph(run)?;
        let model = FairModel {
            graph: base,
            constraints,
        };
        let within: BTreeSet<Vertex> = model
            .graph
            .vertices()
            .filter(|vertex| empty.contains(&ProductStateId(vertex.index())))
            .collect();
        let analyses = match analyze(&model.graph, &within, &model.constraints, self.cancel) {
            Ok(analyses) => analyses,
            Err(FairnessError::Cancelled) => {
                return self.cancelled(totals).map(Exhausted::Stopped);
            }
            Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                return Err(fault("the-fairness-graph-is-built-from-retained-states"));
            }
        };
        let mut fair = Vec::new();
        fair_components(&analyses, &mut fair);
        let fair_states: BTreeSet<ProductStateId> = fair
            .iter()
            .flat_map(|analysis| analysis.vertices())
            .map(|vertex| ProductStateId(vertex.index()))
            .collect();
        // The empty-`X` states from which a fair component is reachable, by
        // walking the empty part of the graph backwards.
        let mut before: BTreeMap<ProductStateId, Vec<ProductStateId>> = BTreeMap::new();
        for edge in graph.edges() {
            if empty.contains(&edge.source) {
                before.entry(edge.target).or_default().push(edge.source);
            }
        }
        let mut reaches = fair_states.clone();
        let mut queue: VecDeque<ProductStateId> = fair_states.iter().copied().collect();
        while let Some(state) = queue.pop_front() {
            for source in before.get(&state).into_iter().flatten() {
                if reaches.insert(*source) {
                    queue.push_back(*source);
                }
            }
        }
        let Some(first) = reaches.first().copied() else {
            return Ok(Exhausted::NoneFair);
        };
        // The nearest state of a fair component from `first`, by the
        // breadth-first order of the retained edges.
        let mut found_by: BTreeMap<ProductStateId, &RetainedEdge<HyperTransition>> =
            BTreeMap::new();
        let mut seen = BTreeSet::from([first]);
        let mut queue = VecDeque::from([first]);
        let mut entry = None;
        while let Some(state) = queue.pop_front() {
            if fair_states.contains(&state) {
                entry = Some(state);
                break;
            }
            for edge in graph.outgoing(state) {
                if seen.insert(edge.target) {
                    found_by.insert(edge.target, edge);
                    queue.push_back(edge.target);
                }
            }
        }
        let entry = entry.ok_or_else(|| fault("a-reaching-state-reaches-a-fair-component"))?;
        let mut connect = Vec::new();
        let mut cursor = entry;
        while let Some(edge) = found_by.get(&cursor) {
            connect.push(*edge);
            cursor = edge.source;
        }
        connect.reverse();
        let passing = fair
            .iter()
            .find(|analysis| {
                analysis
                    .vertices()
                    .iter()
                    .any(|vertex| ProductStateId(vertex.index()) == entry)
            })
            .ok_or_else(|| fault("an-entry-lies-in-a-fair-component"))?;
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let vertex = passing
            .vertices()
            .iter()
            .copied()
            .find(|vertex| ProductStateId(vertex.index()) == entry)
            .ok_or_else(|| fault("an-entry-is-a-vertex"))?;
        let walk = model.greedy_loop(&component, passing.removed(), vertex)?;
        let mut loop_path = Vec::with_capacity(walk.len());
        for (source, position) in model.loop_edges(&walk)? {
            loop_path.push(
                graph
                    .outgoing(source)
                    .nth(position)
                    .ok_or_else(|| fault("a-loop-edge-is-retained"))?,
            );
        }
        let (root, mut stem) = tree_path(graph, first);
        stem.extend(connect);
        let mut path = stem.clone();
        path.extend(loop_path.iter().copied());
        // The joint position of the first state whose `X` is empty.
        let mut positions = vec![root];
        positions.extend(path.iter().map(|edge| edge.target));
        let position = positions
            .iter()
            .position(|id| is_empty(*id))
            .ok_or_else(|| fault("a-lasso-through-an-empty-set-has-one"))?;
        let mut entry_at = stem.len();
        let mut length = path.len();
        // The lasso is the same word when its last step repeats the stem's
        // last step from a universal state that equals the one that step left.
        while entry_at > 0 && entry_at < length {
            let (Some(last), Some(left)) = (path.get(length - 1), path.get(entry_at - 1)) else {
                return Err(fault("a-lasso-step-is-a-path-edge"));
            };
            let (from_last, from_left) = (graph.state(last.source), graph.state(left.source));
            let same = last.transition == left.transition
                && match (from_last, from_left) {
                    (Some(a), Some(b)) => self.digests(&a.state)? == self.digests(&b.state)?,
                    _ => return Err(fault("a-lasso-step-leaves-a-retained-state")),
                };
            if !same {
                break;
            }
            entry_at -= 1;
            length -= 1;
        }
        let traces = self.traces(run, root, &path[..length], None, None)?;
        let counterexample = HyperCounterexample {
            kind: HyperCounterexampleKind::WitnessExhausted {
                loop_entry: widen(entry_at),
                position: widen(position),
            },
            instance: Self::instance_of(run.instance),
            traces,
            trace_position: widen(position),
        };
        Ok(Exhausted::Lasso(Box::new(counterexample), position))
    }

    /// The outcome of an item no phase refuted or left undecided: vacuous
    /// when its non-empty `μ_U` is matched by no fair tuple of universal
    /// behaviours (ADR-023 HM-7), otherwise holding.
    fn clean(
        &self,
        explored: &[Explored<'_, '_>],
        totals: &Totals,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        if !self.request.clause.mu_universal().is_empty() {
            let mut matched = false;
            for run in explored {
                match self.fair_cycle(run)? {
                    Some(true) => {
                        matched = true;
                        break;
                    }
                    Some(false) => {}
                    None => return self.cancelled(totals),
                }
            }
            if !matched {
                return Ok(HyperOutcome::Undecided {
                    cause: UndecidedCause::VacuousMatch,
                    stats: totals.stats(),
                });
            }
        }
        Ok(HyperOutcome::Holds {
            stats: totals.stats(),
        })
    }

    fn cancelled(&self, totals: &Totals) -> Result<HyperOutcome, ModelCheckRefusal> {
        match self.cancel.cause() {
            Some(source) => Ok(stopped(
                IncompleteCause::Cancelled { source },
                None,
                None,
                totals,
            )),
            None => Err(fault("a-cancelled-run-has-a-cause")),
        }
    }

    /// HC-8: explore each distinct subject's reachable graph once under the
    /// request's limits. A subject that alone reaches a limit stops the item
    /// before any product is built.
    fn precheck(&self, totals: &Totals) -> Result<Option<HyperOutcome>, ModelCheckRefusal> {
        let limits = self.request.limits.limits;
        for system in &self.systems {
            let watched = Watched {
                system,
                stop: Cell::new(None),
            };
            let exploration = retain(&watched, limits, IdentityLimits::default(), None, || {
                self.cancel.poll() && self.cancel.cause().is_some()
            })
            .map_err(|error| {
                ModelCheckRefusal::Encoding(qsl_eval::simulation::EncodingRefusal::of(
                    &error,
                    IdentityLimits::default(),
                ))
            })?;
            match exploration.outcome {
                Outcome::Exhaustive(_) => {}
                Outcome::Bounded { limit, .. } => {
                    let (limit, value) = match limit {
                        Limit::States(_) => (ModelCheckLimit::MaxStates, widen(limits.max_states)),
                        Limit::Transitions(_) => (
                            ModelCheckLimit::MaxTransitions,
                            widen(limits.max_transitions),
                        ),
                    };
                    return Ok(Some(exhausted(limit, value, totals)));
                }
                Outcome::Cancelled { .. } => return self.cancelled(totals).map(Some),
                Outcome::Stopped { .. } => {
                    let ending = match watched.stop.take() {
                        Some(stop) => self.expansion_ending(None, stop)?,
                        None => {
                            return Err(fault("a-stopped-expansion-recorded-its-stop"));
                        }
                    };
                    let Ending::Stopped(cause, limit, count) = ending else {
                        return Err(fault("a-stop-is-an-ending-that-stops"));
                    };
                    return Ok(Some(stopped(cause, limit, count, totals)));
                }
                Outcome::BoundReached { .. } => {
                    return Err(fault("the-precheck-sets-no-horizon"));
                }
            }
        }
        Ok(None)
    }

    /// The ending a component's expansion stop settles; `component` is the
    /// stopped component, `None` when it is a subject's own exploration.
    fn expansion_ending(
        &self,
        component: Option<usize>,
        stop: ExpansionStop,
    ) -> Result<Ending, ModelCheckRefusal> {
        let reached = |limit, value| {
            Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit { limit, value }),
                None,
            )
        };
        Ok(match stop.limit {
            Some(ExpansionLimit::EvaluationMeter) => reached(
                ModelCheckLimit::EvaluationMeter,
                self.request.meter.work_units,
            ),
            Some(ExpansionLimit::MaxCandidates) => {
                let subject = component
                    .and_then(|component| self.slots.get(component).copied())
                    .unwrap_or(0);
                reached(
                    ModelCheckLimit::MaxCandidates,
                    self.request
                        .subjects
                        .get(subject)
                        .map_or(0, |subject| subject.simulation.expansion.max_candidates),
                )
            }
            None if stop.is_internal_failure() => return Err(fault("model-expansion-invariant")),
            None => Ending::Stopped(IncompleteCause::ResourceExhausted, None, None),
        })
    }

    /// The ending a product's recorded halt settles.
    fn halt_ending(&self, halt: Halt) -> Result<Ending, ModelCheckRefusal> {
        Ok(match halt {
            Halt::Expansion { component, stop } => self.expansion_ending(Some(component), stop)?,
            Halt::Meter => Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::EvaluationMeter,
                    value: self.request.meter.work_units,
                }),
                None,
            ),
            Halt::WitnessSet(count) => Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxWitnessSet,
                    value: self.request.limits.max_witness_set,
                }),
                Some(count),
            ),
            Halt::Cancelled(source) => {
                Ending::Stopped(IncompleteCause::Cancelled { source }, None, None)
            }
            Halt::Automaton(limit) => Ending::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxAutomatonStates,
                    value: limit,
                }),
                None,
            ),
            Halt::Encoding(refusal) => return Err(ModelCheckRefusal::Encoding(refusal)),
            Halt::Fault(internal) => return Err(ModelCheckRefusal::Fault(internal)),
        })
    }

    /// The product of `instance`, with its initial states read. `Err(outcome)`
    /// is a run that ended before it had a graph to explore.
    fn build<'x>(
        &'x self,
        instance: &'x [ParameterBinding],
        totals: &Totals,
    ) -> Result<Result<Built<'x, 'a>, HyperOutcome>, ModelCheckRefusal> {
        let clause = self.request.clause;
        let limits = &self.request.limits;
        let systems = self.slots.iter().map(|slot| &self.systems[*slot]).collect();
        let built = if self.witness {
            WitnessProduct::new(WitnessInputs {
                systems,
                universal: clause.universals(),
                instance,
                body: clause.body(),
                profile: clause.profile(),
                letters: self.request.letters,
                universal_match: (!clause.mu_universal().is_empty())
                    .then_some(self.request.matcher),
                existential_match: (!clause.mu_existential().is_empty())
                    .then_some(self.request.matcher),
                max_automaton_states: limits.max_automaton_states,
                max_witness_set: limits.max_witness_set,
                meter: self.request.meter,
                cancel: self.cancel,
            })
            .map(Built::Witness)
        } else {
            HyperProduct::new(HyperInputs {
                systems,
                instance,
                body: clause.body(),
                profile: clause.profile(),
                letters: self.request.letters,
                matcher: (!clause.mu_universal().is_empty()).then_some(self.request.matcher),
                max_automaton_states: limits.max_automaton_states,
                meter: self.request.meter,
                cancel: self.cancel,
            })
            .map(Built::Universal)
        };
        let mut built = match built {
            Ok(built) => built,
            Err(AutomatonError::StateLimit { limit }) => {
                return Ok(Err(exhausted(
                    ModelCheckLimit::MaxAutomatonStates,
                    limit,
                    totals,
                )))
            }
            Err(other) => return Err(HyperRefusal::Automaton(other).into()),
        };
        let started = match &mut built {
            Built::Universal(product) => product.start(),
            Built::Witness(product) => product.start(),
        };
        if started.is_err() {
            return self.halted(&built, totals).map(Err);
        }
        Ok(Ok(built))
    }

    /// The retained graph of `product` under the limits `totals` leave.
    fn retained<P>(
        &self,
        product: &P,
        cap: Option<usize>,
        totals: &Totals,
    ) -> Result<GraphExploration<HyperState, HyperTransition, HyperFinding>, ModelCheckRefusal>
    where
        P: TransitionSystem<
            State = HyperState,
            TransitionId = HyperTransition,
            Finding = HyperFinding,
        >,
    {
        retain(
            product,
            totals.remaining(self.request.limits.limits),
            IdentityLimits::default(),
            cap,
            || self.cancel.poll() && self.cancel.cause().is_some(),
        )
        .map_err(|error| {
            ModelCheckRefusal::Encoding(qsl_eval::simulation::EncodingRefusal::of(
                &error,
                IdentityLimits::default(),
            ))
        })
    }

    /// Explore `instance`'s product. `Err(outcome)` is a run that ended
    /// before it had a graph to examine.
    fn explore<'x>(
        &'x self,
        instance: &'x [ParameterBinding],
        cap: Option<usize>,
        totals: &mut Totals,
    ) -> Result<Result<Explored<'x, 'a>, HyperOutcome>, ModelCheckRefusal> {
        let product = match self.build(instance, totals)? {
            Ok(product) => product,
            Err(outcome) => return Ok(Err(outcome)),
        };
        let spent = self.request.limits.limits;
        let exploration = match &product {
            Built::Universal(product) => self.retained(product, cap, totals)?,
            Built::Witness(product) => self.retained(product, cap, totals)?,
        };
        let transitions = match &exploration.outcome {
            Outcome::Exhaustive(stats) => stats.transitions,
            Outcome::BoundReached { stats, .. }
            | Outcome::Bounded { stats, .. }
            | Outcome::Cancelled { stats, .. }
            | Outcome::Stopped { stats, .. } => stats.transitions,
        };
        totals.add(
            &exploration.graph,
            transitions,
            widen(product.automaton().reached_states()),
        );
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
                    None,
                )
            }
            Outcome::Cancelled { .. } => match self.cancel.cause() {
                Some(source) => Ending::Stopped(IncompleteCause::Cancelled { source }, None, None),
                None => return Err(fault("a-cancelled-run-has-a-cause")),
            },
            Outcome::Stopped { .. } => match product.take_halt() {
                Some(halt) => self.halt_ending(halt)?,
                None => return Err(fault("a-stopped-expansion-recorded-its-halt")),
            },
            // The horizon is the ranking cap's: the instance is explored as
            // far as it can still win.
            Outcome::BoundReached { .. } if cap.is_some() => Ending::Complete,
            Outcome::BoundReached { .. } => {
                return Err(fault("the-product-sets-no-horizon-without-a-cap"));
            }
        };
        if let Ending::Stopped(cause, limit, count) = ending {
            return Ok(Err(stopped(cause, limit, count, totals)));
        }
        Ok(Ok(Explored {
            instance,
            product,
            graph: exploration.graph,
        }))
    }

    /// The outcome of a product that halted before it had a graph.
    fn halted(
        &self,
        product: &Built<'_, '_>,
        totals: &Totals,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        match product.take_halt() {
            Some(halt) => match self.halt_ending(halt)? {
                Ending::Stopped(cause, limit, count) => Ok(stopped(cause, limit, count, totals)),
                Ending::Complete => Err(fault("a-halted-start-is-a-stop")),
            },
            None => Err(fault("a-halted-start-recorded-its-halt")),
        }
    }

    /// The first phase over one instance: the first state in discovery order
    /// whose letter is undefined, or whose expansion found an undefined
    /// `μ_U` or contract, ends the run; failing that, a refused one leaves it
    /// undecided. The finding comes with the depth it was at.
    fn first_phase(
        &self,
        run: &Explored<'_, '_>,
    ) -> Result<Option<(usize, FirstHit)>, ModelCheckRefusal> {
        // The candidates rank by depth, an undefined evaluation before a
        // refused one at one depth, then by discovery order: the first state
        // in discovery order, and within it the first finding, wins a tie.
        let mut best: Option<(usize, u8, Candidate)> = None;
        let mut offer = |depth: usize, rank: u8, candidate: Candidate| {
            if best
                .as_ref()
                .is_none_or(|(known, known_rank, _)| (depth, rank) < (*known, *known_rank))
            {
                best = Some((depth, rank, candidate));
            }
        };
        for (index, state) in run.graph.states().iter().enumerate() {
            let id = ProductStateId(index);
            if let Property::Undefined {
                atom,
                cause,
                partner,
            } = &state.state.property
            {
                offer(
                    state.depth,
                    0,
                    Candidate::Found(
                        id,
                        Found::Atom {
                            atom: *atom,
                            cause: cause.clone(),
                            partner: partner.clone(),
                        },
                    ),
                );
            }
            let mut refused = None;
            for finding in &state.findings {
                match finding {
                    HyperFinding::Contract { component, finding } => {
                        if let Some(undefined) = finding.undefined_contract() {
                            offer(
                                state.depth,
                                0,
                                Candidate::Found(
                                    id,
                                    Found::Contract {
                                        component: *component,
                                        clause: undefined.clause,
                                        guard_of: undefined.guard_of,
                                        cause: undefined.cause,
                                    },
                                ),
                            );
                        } else if finding.is_refused_contract() {
                            refused
                                .get_or_insert((state.depth, UndecidedCause::UndecidedSuccessor));
                        }
                    }
                    HyperFinding::MatchUndefined {
                        step,
                        posts,
                        conjunct,
                        cause,
                    } => offer(
                        state.depth.saturating_add(1),
                        0,
                        Candidate::Found(
                            id,
                            Found::Match {
                                step: step.clone(),
                                posts: posts.clone(),
                                conjunct: *conjunct,
                                cause: cause.clone(),
                            },
                        ),
                    ),
                    HyperFinding::ExistentialMatchUndefined {
                        step,
                        posts,
                        conjunct,
                        cause,
                        partner,
                    } => offer(
                        state.depth.saturating_add(1),
                        0,
                        Candidate::Found(
                            id,
                            Found::Existential {
                                step: step.clone(),
                                posts: posts.clone(),
                                conjunct: *conjunct,
                                cause: cause.clone(),
                                partner: partner.clone(),
                            },
                        ),
                    ),
                    HyperFinding::MatchRefused => {
                        refused.get_or_insert((
                            state.depth.saturating_add(1),
                            UndecidedCause::MatchUndetermined,
                        ));
                    }
                    HyperFinding::MemberContract => {
                        refused.get_or_insert((state.depth, UndecidedCause::UndecidedSuccessor));
                    }
                    HyperFinding::Enabled { .. } => {}
                }
            }
            if let Some((depth, cause)) = refused {
                offer(depth, 1, Candidate::Refused(cause));
            }
        }
        Ok(match best {
            None => None,
            Some((depth, _, Candidate::Refused(cause))) => {
                Some((depth, FirstHit::Undecided(cause)))
            }
            Some((depth, _, Candidate::Found(id, found))) => {
                let counterexample = self.undefined_counterexample(run, id, found)?;
                Some((depth, FirstHit::Violated(Box::new(counterexample))))
            }
        })
    }

    /// The digest of each component of `state`, in quantifier order.
    fn digests(&self, state: &HyperState) -> Result<Vec<DigestRecord>, ModelCheckRefusal> {
        self.slots
            .iter()
            .zip(&state.components)
            .map(|(slot, component)| {
                self.systems[*slot]
                    .state_key(component)
                    .map(|(_, digest)| digest)
                    .map_err(ModelCheckRefusal::Encoding)
            })
            .collect()
    }

    /// The trace variables with their aliases, in quantifier order.
    fn variables(&self) -> Vec<(String, String)> {
        self.request
            .clause
            .quantifiers()
            .iter()
            .map(|quantifier| {
                (
                    quantifier.variable.clone(),
                    quantifier.alias.as_str().to_owned(),
                )
            })
            .collect()
    }

    /// The instance a counterexample names: each parameter and its key.
    fn instance_of(instance: &[ParameterBinding]) -> Vec<(String, String)> {
        instance
            .iter()
            .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
            .collect()
    }

    /// One trace per variable: the path from the root through `edges`, then
    /// the joint step `last` when there is one. An HP-3 clause's existential
    /// variables take theirs from `partner`; without one the traces are the
    /// universal variables' alone.
    fn traces(
        &self,
        run: &Explored<'_, '_>,
        root: ProductStateId,
        edges: &[&RetainedEdge<HyperTransition>],
        last: Option<(&HyperTransition, &[DigestRecord])>,
        partner: Option<&PartnerPath<'_>>,
    ) -> Result<Vec<HyperTrace>, ModelCheckRefusal> {
        let root_state = run
            .graph
            .state(root)
            .ok_or_else(|| fault("a-path-starts-at-a-retained-state"))?;
        let universals = self.request.clause.universals();
        let count = if partner.is_some() {
            self.slots.len()
        } else {
            universals
        };
        let mut traces = Vec::with_capacity(count);
        for (component, (variable, alias)) in self.variables().into_iter().take(count).enumerate() {
            let (initial, steps) = if component < universals {
                let system = &self.systems[self.slots[component]];
                let start = system
                    .state_key(&root_state.state.components[component])
                    .map_err(ModelCheckRefusal::Encoding)?
                    .1;
                let initial = system
                    .initial()
                    .iter()
                    .position(|state| {
                        system
                            .state_key(state)
                            .is_ok_and(|(_, digest)| digest == start)
                    })
                    .and_then(|index| u32::try_from(index).ok())
                    .ok_or_else(|| fault("a-root-component-is-an-initial-state"))?;
                let mut steps = Vec::with_capacity(edges.len() + 1);
                for edge in edges {
                    let target = run
                        .graph
                        .state(edge.target)
                        .ok_or_else(|| fault("an-edge-names-a-retained-state"))?;
                    let post = self.digests(&target.state)?;
                    steps.push(hyper_step(&edge.transition, &post, component)?);
                }
                if let Some((transition, posts)) = last {
                    steps.push(hyper_step(transition, posts, component)?);
                }
                (initial, steps)
            } else {
                let path = partner.ok_or_else(|| fault("an-existential-trace-has-a-partner"))?;
                self.partner_trace(path, component - universals)?
            };
            traces.push(HyperTrace {
                variable,
                alias,
                initial,
                steps,
            });
        }
        Ok(traces)
    }

    /// The existential variable `offset`'s initial index and steps along
    /// `path`.
    fn partner_trace(
        &self,
        path: &PartnerPath<'_>,
        offset: usize,
    ) -> Result<(u32, Vec<HyperStep>), ModelCheckRefusal> {
        let (indices, moves) = path.partner.path();
        let initial = indices
            .get(offset)
            .copied()
            .ok_or_else(|| fault("a-partner-has-an-initial-index-per-variable"))?;
        let mut steps = Vec::with_capacity(moves.len() + 1);
        for (taken, posts) in moves.into_iter().chain(path.last) {
            let (step, post) = taken
                .get(offset)
                .zip(posts.get(offset))
                .ok_or_else(|| fault("a-partner-step-has-a-component-per-variable"))?;
            steps.push(component_step(step, *post));
        }
        Ok((initial, steps))
    }

    /// The undefined-evaluation counterexample for `found` at `at` (ADR-023
    /// HV-8): the tree path to `at` for every trace variable, then the joint
    /// step a `μ` finding names. An HP-3 clause's existential variables follow
    /// the partner of the member the evaluation was made on.
    fn undefined_counterexample(
        &self,
        run: &Explored<'_, '_>,
        at: ProductStateId,
        found: Found,
    ) -> Result<HyperCounterexample, ModelCheckRefusal> {
        let (root, path) = tree_path(&run.graph, at);
        let state = run
            .graph
            .state(at)
            .ok_or_else(|| fault("a-finding-is-at-a-retained-state"))?;
        let stem = widen(path.len());
        let variables = self.variables();
        let universals = self.request.clause.universals();
        let mut partner: Option<Rc<Partner>> = None;
        let mut partner_last = None;
        let (position, state_keys, named, locus, transition, cause, last) = match found {
            Found::Atom {
                atom,
                cause,
                partner: read,
            } => {
                let mut keys = self.digests(&state.state)?;
                if let Some(read) = &read {
                    keys.extend_from_slice(read.digests());
                }
                partner = read;
                (
                    stem,
                    keys,
                    variables,
                    HyperLocus::Atom(atom),
                    None,
                    cause,
                    None,
                )
            }
            Found::Match {
                step,
                posts,
                conjunct,
                cause,
            } => {
                if self.witness {
                    return Err(HyperRefusal::NoPartnerPath.into());
                }
                (
                    stem.saturating_add(1),
                    posts.clone(),
                    variables,
                    HyperLocus::Match { conjunct },
                    None,
                    cause,
                    Some((step, posts)),
                )
            }
            Found::Existential {
                step,
                posts,
                conjunct,
                cause,
                partner: read,
            } => {
                let (universal, existential) = step.0.split_at(universals.min(step.0.len()));
                let (universal_posts, existential_posts) =
                    posts.split_at(universals.min(posts.len()));
                partner_last = Some((existential.to_vec(), existential_posts.to_vec()));
                partner = Some(read);
                let last = Some((
                    HyperTransition(universal.to_vec()),
                    universal_posts.to_vec(),
                ));
                (
                    stem.saturating_add(1),
                    posts,
                    variables,
                    HyperLocus::ExistentialMatch { conjunct },
                    None,
                    cause,
                    last,
                )
            }
            Found::Contract {
                component,
                clause,
                guard_of,
                cause,
            } => {
                if self.witness {
                    partner = match &state.state.property {
                        Property::Witnesses(set) => set
                            .members()
                            .first()
                            .map(|member| Rc::clone(member.partner())),
                        _ => None,
                    };
                    if partner.is_none() {
                        return Err(HyperRefusal::NoPartnerPath.into());
                    }
                }
                let digests = self.digests(&state.state)?;
                let digest = digests
                    .get(component)
                    .copied()
                    .ok_or_else(|| fault("a-contract-finding-names-a-component"))?;
                let named = variables
                    .get(component)
                    .cloned()
                    .ok_or_else(|| fault("a-contract-finding-names-a-variable"))?;
                (
                    stem,
                    vec![digest],
                    vec![named],
                    HyperLocus::Contract(clause),
                    guard_of,
                    cause,
                    None,
                )
            }
        };
        let path_of = partner.as_deref().map(|partner| PartnerPath {
            partner,
            last: partner_last.as_ref().map(
                |(steps, posts): &(Vec<ComponentStep>, Vec<DigestRecord>)| {
                    (steps.as_slice(), posts.as_slice())
                },
            ),
        });
        let traces = self.traces(
            run,
            root,
            &path,
            last.as_ref().map(|(step, posts)| (step, posts.as_slice())),
            path_of.as_ref(),
        )?;
        Ok(HyperCounterexample {
            kind: HyperCounterexampleKind::Undefined(HyperUndefined {
                position,
                state_keys,
                variables: named,
                locus,
                transition,
                cause,
            }),
            instance: Self::instance_of(run.instance),
            traces,
            trace_position: position,
        })
    }

    /// The fairness graph of `run` with no acceptance set, and the clause's
    /// constraints scoped to their own variables (HC-4).
    fn fair_graph(
        &self,
        run: &Explored<'_, '_>,
    ) -> Result<(Graph, Vec<Constraint>), ModelCheckRefusal> {
        let graph = &run.graph;
        let joints: BTreeSet<&HyperTransition> =
            graph.edges().iter().map(|edge| &edge.transition).collect();
        let joint_ids: BTreeMap<&HyperTransition, Identity> = joints
            .iter()
            .enumerate()
            .map(|(index, joint)| (*joint, Identity::new(widen(index))))
            .collect();
        let mut enabled_at: BTreeSet<(usize, &ModelTransition)> = BTreeSet::new();
        for state in graph.states() {
            for finding in &state.findings {
                if let HyperFinding::Enabled {
                    component,
                    transitions,
                } = finding
                {
                    enabled_at.extend(transitions.iter().map(|t| (*component, t)));
                }
            }
        }
        // The identities a vertex enables are a space of their own beside the
        // joint steps': a constraint covers the joint steps whose component
        // takes its operation and the (component, transition) pairs that
        // enable it.
        let enabled_ids: BTreeMap<(usize, &ModelTransition), Identity> = enabled_at
            .iter()
            .enumerate()
            .map(|(index, key)| (*key, Identity::new(widen(joint_ids.len() + index))))
            .collect();
        let mut fair = Graph::new();
        let mut vertices = Vec::with_capacity(graph.states().len());
        for state in graph.states() {
            let mut enabled = Vec::new();
            for finding in &state.findings {
                if let HyperFinding::Enabled {
                    component,
                    transitions,
                } = finding
                {
                    for transition in transitions {
                        enabled.push(
                            enabled_ids
                                .get(&(*component, transition))
                                .copied()
                                .ok_or_else(|| fault("every-enabled-transition-is-numbered"))?,
                        );
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
        for index in 0..graph.states().len() {
            let source = ProductStateId(index);
            for edge in graph.outgoing(source) {
                let label = if !edge.transition.0.is_empty()
                    && edge
                        .transition
                        .0
                        .iter()
                        .all(|step| matches!(step, ComponentStep::Stutter))
                {
                    EdgeLabel::Stutter
                } else {
                    EdgeLabel::Transition(
                        joint_ids
                            .get(&edge.transition)
                            .copied()
                            .ok_or_else(|| fault("every-joint-step-is-numbered"))?,
                    )
                };
                fair.add_edge(vertex(source)?, vertex(edge.target)?, label)
                    .map_err(|_| fault("an-edge-joins-two-vertices"))?;
            }
        }
        let mut constraints = Vec::new();
        for (component, quantifier) in self.request.clause.quantifiers().iter().enumerate() {
            let system = &self.systems[self.slots[component]];
            for constraint in &quantifier.fairness {
                let strength = Strength::from(constraint.kind());
                let operation = constraint.operation();
                let text = system
                    .operation_text(operation.declaring_type(), operation.name())
                    .ok_or_else(|| HyperRefusal::UnknownOperation {
                        name: operation.name().to_owned(),
                    })?;
                let covers = |transition: &ModelTransition| transition.operation() == text;
                // The transitions of this component's operation, each with
                // the identities that enable it and the joint steps that take
                // it.
                let mut covered: Vec<(&ModelTransition, Vec<Identity>)> = Vec::new();
                for ((owner, transition), enabled_id) in &enabled_ids {
                    if *owner != component || !covers(transition) {
                        continue;
                    }
                    let mut members = vec![*enabled_id];
                    members.extend(joint_ids.iter().filter_map(|(joint, id)| {
                        matches!(joint.0.get(component),
                            Some(ComponentStep::Model(taken)) if taken == *transition)
                        .then_some(*id)
                    }));
                    covered.push((transition, members));
                }
                match constraint.granularity() {
                    FairnessGranularity::Whole => constraints.push(Constraint::new(
                        strength,
                        covered.into_iter().flat_map(|(_, members)| members),
                    )),
                    FairnessGranularity::Each => {
                        for (_, members) in covered {
                            constraints.push(Constraint::new(strength, members));
                        }
                    }
                }
            }
        }
        Ok((fair, constraints))
    }

    /// `base` with the automaton's acceptance sets added. A negated body with
    /// none (a co-safety body, `eventually p`) accepts every run that has no
    /// bad prefix: the states the automaton still has a run in, so the
    /// `Dead` states the product keeps for UE-1 accept nothing.
    fn accepting(&self, run: &Explored<'_, '_>, base: &Graph) -> Result<Graph, ModelCheckRefusal> {
        let mut graph = base.clone();
        let automaton = run.product.automaton();
        let vertices: Vec<Vertex> = graph.vertices().collect();
        if automaton.acceptance_set_count() == 0 {
            let live = run
                .graph
                .states()
                .iter()
                .enumerate()
                .filter(|(_, state)| matches!(state.state.property, Property::After(_)))
                .map(|(index, _)| vertices.get(index).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| fault("a-member-is-a-vertex"))?;
            graph
                .add_acceptance_set(live)
                .map_err(|_| fault("an-acceptance-member-is-a-vertex"))?;
        }
        for set in 0..automaton.acceptance_set_count() {
            let members: Vec<Vertex> = run
                .graph
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
            graph
                .add_acceptance_set(members)
                .map_err(|_| fault("an-acceptance-member-is-a-vertex"))?;
        }
        Ok(graph)
    }

    /// Whether the product has a fair cycle with the acceptance sets set
    /// aside: a tuple of fair behaviours matched at every joint step (HM-7).
    /// `None` when the run was cancelled.
    fn fair_cycle(&self, run: &Explored<'_, '_>) -> Result<Option<bool>, ModelCheckRefusal> {
        let (graph, constraints) = self.fair_graph(run)?;
        let all: BTreeSet<Vertex> = graph.vertices().collect();
        match analyze(&graph, &all, &constraints, self.cancel) {
            Ok(analyses) => Ok(Some(Analysis::first_fair_in(&analyses).is_some())),
            Err(FairnessError::Cancelled) => Ok(None),
            Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                Err(fault("the-fairness-graph-is-built-from-retained-states"))
            }
        }
    }

    /// The second phase over one instance: the canonical lasso of the first
    /// fair accepting component (ADR-018 CX-5), split into one trace per
    /// variable.
    fn second_phase(
        &self,
        run: &Explored<'_, '_>,
        totals: &Totals,
    ) -> Result<Second, ModelCheckRefusal> {
        let (base, constraints) = self.fair_graph(run)?;
        let model = FairModel {
            graph: self.accepting(run, &base)?,
            constraints,
        };
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses = match analyze(&model.graph, &all, &model.constraints, self.cancel) {
            Ok(analyses) => analyses,
            Err(FairnessError::Cancelled) => {
                return self.cancelled(totals).map(Second::Stopped);
            }
            Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                return Err(fault("the-fairness-graph-is-built-from-retained-states"));
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
        let (root, stem) = tree_path(&run.graph, entry);
        let mut loop_path = Vec::with_capacity(loop_edges.len());
        for (source, position) in loop_edges {
            loop_path.push(
                run.graph
                    .outgoing(source)
                    .nth(position)
                    .ok_or_else(|| fault("a-loop-edge-is-retained"))?,
            );
        }
        let mut path = stem.clone();
        path.extend(loop_path.iter().copied());
        let traces = self.traces(run, root, &path, None, None)?;
        // The positions of the lasso: the stem's states, then the loop's but
        // the last, which is the entry again.
        let mut positions = vec![root];
        positions.extend(stem.iter().map(|edge| edge.target));
        positions.extend(
            loop_path
                .iter()
                .take(loop_path.len().saturating_sub(1))
                .map(|edge| edge.target),
        );
        let at = match self.false_position(run, &positions, stem.len(), loop_path.len(), totals)? {
            Ok(at) => at,
            Err(outcome) => return Ok(Second::Stopped(outcome)),
        };
        let counterexample = HyperCounterexample {
            kind: HyperCounterexampleKind::Lockstep {
                loop_entry: widen(stem.len()),
            },
            instance: Self::instance_of(run.instance),
            traces,
            trace_position: at,
        };
        Ok(Second::Lasso(
            Box::new(counterexample),
            stem.len().saturating_add(loop_path.len()),
        ))
    }

    /// The joint position at which the body's `false` is fixed on the lasso
    /// whose positions are `positions` (FR-175): the counterexample's
    /// `trace_position`, read by the evaluator replay uses, which also
    /// confirms the automaton's verdict.
    fn false_position(
        &self,
        run: &Explored<'_, '_>,
        positions: &[ProductStateId],
        prefix: usize,
        loop_len: usize,
        totals: &Totals,
    ) -> Result<Result<u64, HyperOutcome>, ModelCheckRefusal> {
        let sequence = positions
            .iter()
            .map(|id| run.graph.state(*id).map(|state| &state.state))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| fault("a-lasso-position-is-a-retained-state"))?;
        let component = ComponentTrace::Lasso {
            prefix: widen(prefix),
            loop_len: widen(loop_len),
        };
        let components = vec![component; self.slots.len()];
        let mut letters = Positions {
            letters: self.request.letters,
            instance: run.instance,
            sequence,
        };
        let mut meter = Meter::new(self.request.meter).with_cancel(self.cancel.clone());
        let outcome = evaluate_tuple(
            self.request.clause,
            &components,
            &mut letters,
            &mut meter,
            Some(self.cancel),
        )
        .map_err(|_| fault("the-evaluator-reads-the-lasso"))?;
        Ok(match outcome {
            TemporalOutcome::Violated { at } => Ok(at.index()),
            TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => {
                Err(exhausted(
                    ModelCheckLimit::EvaluationMeter,
                    self.request.meter.work_units,
                    totals,
                ))
            }
            _ => return Err(fault("the-evaluator-confirms-the-automaton")),
        })
    }
}

/// The fair components among `analyses`, depth first through each refinement
/// (ADR-019 FS-6), in order.
fn fair_components<'a>(analyses: &'a [Analysis], found: &mut Vec<&'a Analysis>) {
    for analysis in analyses {
        match analysis.outcome() {
            FairnessOutcome::Fair => found.push(analysis),
            FairnessOutcome::UnfairStrong { sub, .. }
            | FairnessOutcome::UnfairVisibility { sub, .. } => fair_components(sub, found),
            FairnessOutcome::Trivial
            | FairnessOutcome::MissingAcceptance { .. }
            | FairnessOutcome::UnfairWeak { .. } => {}
        }
    }
}

/// A trace's step of `component` in the joint step `transition`, with the
/// digest of its post-state.
fn hyper_step(
    transition: &HyperTransition,
    posts: &[DigestRecord],
    component: usize,
) -> Result<HyperStep, ModelCheckRefusal> {
    let step = transition
        .0
        .get(component)
        .ok_or_else(|| fault("a-joint-step-has-a-step-per-component"))?;
    let post = posts
        .get(component)
        .ok_or_else(|| fault("a-joint-step-has-a-post-state-per-component"))?;
    Ok(component_step(step, *post))
}

/// A component's `step` as a trace's step, with the digest of its post-state.
fn component_step(step: &ComponentStep, post: DigestRecord) -> HyperStep {
    match step {
        ComponentStep::Model(taken) => HyperStep::Model(ModelStep {
            transition: taken.clone(),
            post_state: post,
        }),
        ComponentStep::Stutter => HyperStep::TerminalStutter,
    }
}

/// The body's atoms on a lasso of retained states: component `c` at joint
/// position `p` reads its own position's state (FR-175 "Indexed atoms").
struct Positions<'r> {
    letters: &'r dyn HyperLetters,
    instance: &'r [ParameterBinding],
    sequence: Vec<&'r HyperState>,
}

impl TupleLetters for Positions<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        _joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let mut states: Vec<&ModelSimState> = Vec::with_capacity(components.len());
        for (component, read) in components.iter().enumerate() {
            let index = usize::try_from(read.position.index()).map_err(|_| {
                LetterStop::Fault(InternalFault::new("hyper-check", "a-position-fits-usize"))
            })?;
            let state = self
                .sequence
                .get(index)
                .and_then(|state| state.components.get(component))
                .ok_or_else(|| {
                    LetterStop::Fault(InternalFault::new("hyper-check", "a-position-is-a-state"))
                })?;
            states.push(state);
        }
        self.letters
            .atom(atom, self.instance, &states, meter)
            .map_err(LetterStop::Incomplete)
    }
}
