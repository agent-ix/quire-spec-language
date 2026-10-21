// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 (ADR-020 RE-1, ADR-018 EN-1): [`check_refinement`], the safety
//! phase of a refinement on the explicit-state product of the concrete
//! subject with the mapped abstract state.
//!
//! The phase runs [`check_initial`] on each initial state and [`check_step`]
//! on each product edge it explores, in FR-101's canonical breadth-first
//! order: initial states by ascending state-key bytes, successors by
//! transition-identity bytes then post-state key bytes. It stops at the first
//! edge or initial state whose check fails or is undefined, so the failing
//! prefix it returns is the canonical path to that edge followed by the edge.
//! Both checks are given the refinement's own concrete and abstract
//! `ModelSystem`s, the systems whose states are explored and mapped, and the
//! abstract system is built with the derived universes: each abstract
//! population's universe is the concrete universe of its source population
//! (ADR-020 RM-2).
//!
//! A product state is the concrete state, its history values (FR-138) and,
//! when the refinement has a hidden field, the AX-2 set of abstract states
//! consistent with the behaviour so far (FR-142). Its key is the concrete
//! state's key, the history values' key and the candidates' sorted state
//! keys, so two paths that reach one concrete state with different history or
//! candidates are two states. A run that holds hands over the simulation relation it found
//! (FR-149): every stored product state, the position of each initial state
//! and every edge with the `Taken` that `check_step` returned, for the
//! core's checker to recompute.
//!
//! The liveness half (FR-143) runs on that retained graph when `F_A` is
//! non-empty and the safety half holds. For each class of `F_A`, in source
//! order and then canonical identity order, it decomposes the subgraph of the
//! product states that enable the class and the edges that do not take it,
//! and the first component that passes the `F_C` filter violates the class:
//! the stem is the breadth-first tree path to it and the loop is FR-126's
//! greedy walk inside it, so the lasso is the one a liveness clause gets.
//! A class is enabled at a product state when the abstract subject has a
//! successor of the class from the state's mapped abstract state, and taken by
//! an edge when the edge's `check_step` result lists an identity of it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::fairness::{analyze, Analysis, EdgeLabel, FairnessError, Graph, Identity, Vertex};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{check_model_bounds, ModelFinding, ModelSimState, ModelSystem};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop, TransitionSystem};
use qsl_foundation::InternalFault;
use qsl_package::CheckedPackage;
use qsl_replay::{
    abstract_classes, check_initial, check_step, open_systems, AbstractClass, CertifiedEdge,
    CertifiedPosition, ConcreteStep, CounterexampleKind, IncompleteCause, Lasso, LivenessHalf,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ModelStep, ReachedLimit,
    RefinementFailure, RefinementOutcome, RefinementPosition, RefinementSystems, RunStats,
    SafetyOutcome, SafetyUndecided, SimulationCertificate, StepResult, StepVerdict, SubjectRefusal,
    Taken, TemporalCounterexample, UnavailabilityCause, UndecidedCause, UndefinedEvaluation,
    Undetermined,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessKind};
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{fault, not_simulated, widen, ModelCheckRefusal, BOUND_POSITIONS};
use super::fair_walk::{fairness_constraints, FairModel};

/// One refinement safety run (FR-142 Inputs).
pub struct RefinementCheckRequest<'a> {
    /// The checked refinement (FR-135).
    pub refinement: &'a CheckedRefinement,
    /// The concrete subject (FR-125), a model subject.
    pub concrete: ModelSubject<'a>,
    /// The package that holds the abstract model: the declaring package, or
    /// a dependency (ADR-020 RM-1).
    pub abstract_package: &'a CheckedPackage,
    /// The abstract subject's FR-106 initial snapshots.
    pub abstract_initial: Vec<DocumentRef>,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
}

/// A product state's key: the concrete state's key bytes, its history
/// values' key and its candidates' sorted state keys.
type ProductKey = (Vec<u8>, Vec<u8>, Vec<Vec<u8>>);

/// One product state: the concrete state, the position `check_step` gave it,
/// the step that first reached it (`None` for an initial state) and its
/// breadth-first depth.
struct Node {
    state: ModelSimState,
    position: RefinementPosition,
    depth: usize,
    parent: Option<(usize, ModelStep)>,
    initial_index: usize,
    /// The edges out of the state, in canonical order, once it is expanded.
    edges: Vec<CertifiedEdge>,
}

/// The first failing initial state or edge: the node it leaves, the step it
/// takes, and what failed.
struct Failure {
    from: Option<usize>,
    edge: Option<ModelStep>,
    initial_index: usize,
    failure: RefinementFailure,
    kind: CounterexampleKind,
}

/// Why the run ended before its frontier emptied.
enum End {
    Violated(Box<Failure>),
    Undecided(SafetyUndecided),
    Stopped(IncompleteCause, Option<ReachedLimit>),
    Fault(InternalFault),
    Refusal(ModelCheckRefusal),
}

impl From<ModelCheckRefusal> for End {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

/// FR-142 and FR-143: decide the safety half of `request`'s refinement and,
/// when `F_A` is non-empty and the safety half holds, its liveness half,
/// polling `cancel` before every expansion and every edge.
///
/// # Errors
///
/// [`ModelCheckRefusal`] when a subject does not admit or a root of either is
/// unbounded, nothing being explored then, and when the liveness half meets
/// an internal fault. A fault of the liveness half is a refusal, not a
/// `Fault` outcome, as FR-126's liveness phase returns it; the safety half
/// and its certificate are discarded with it.
pub fn check_refinement(
    request: RefinementCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<RefinementOutcome, ModelCheckRefusal> {
    check_with(request, cancel, abstract_classes)
}

/// How the second phase reads the classes of `F_A`: [`abstract_classes`] in
/// production, so a unit test can have the phase meet an internal fault.
type ClassesOf = fn(
    &ModelSystem<'_>,
    &[FairnessConstraint],
    &[ModelTransition],
) -> Result<Vec<AbstractClass>, InternalFault>;

fn check_with(
    request: RefinementCheckRequest<'_>,
    cancel: &Cancel,
    classes_of: ClassesOf,
) -> Result<RefinementOutcome, ModelCheckRefusal> {
    let refinement = request.refinement;
    let RefinementSystems {
        concrete,
        abstract_system,
    } = open_systems(
        refinement,
        &request.concrete.simulation,
        request.abstract_package,
        request.abstract_initial.clone(),
    )
    .map_err(|refusal| match refusal {
        SubjectRefusal::Admission(failure) => ModelCheckRefusal::Admission(failure),
        SubjectRefusal::NoModels
        | SubjectRefusal::NoUniverse
        | SubjectRefusal::ProtocolAbstractSide => fault("the-subjects-of-a-refinement-open"),
    })?;
    let simulation = &request.concrete.simulation;
    check_model_bounds(&concrete, BOUND_POSITIONS).map_err(not_simulated)?;
    check_model_bounds(&abstract_system, BOUND_POSITIONS).map_err(not_simulated)?;
    let mut run = Run {
        refinement,
        concrete: &concrete,
        abstract_system: &abstract_system,
        limits: request.limits,
        max_depth: request.max_depth,
        meter: simulation.meter,
        max_candidates: simulation.expansion.max_candidates,
        cancel,
        nodes: Vec::new(),
        known: BTreeMap::new(),
        queue: VecDeque::new(),
        initial_positions: Vec::new(),
        transitions: 0,
        edges: 0,
        horizon_cut: false,
        classes_of,
    };
    let safety = if concrete.initial_states().is_empty() {
        SafetyOutcome::Undecided {
            cause: SafetyUndecided::NoInitialState,
            stats: run.stats(),
        }
    } else {
        match run.explore() {
            Ok(()) if run.horizon_cut => SafetyOutcome::BoundReached {
                depth: widen(run.max_depth.unwrap_or(0)),
                stats: run.stats(),
            },
            Ok(()) => match run.certificate() {
                Ok(certificate) => SafetyOutcome::Holds {
                    certificate: Box::new(certificate),
                    stats: run.stats(),
                },
                Err(fault) => SafetyOutcome::Fault {
                    fault,
                    stats: run.stats(),
                },
            },
            Err(End::Refusal(refusal)) => return Err(refusal),
            Err(End::Undecided(cause)) => SafetyOutcome::Undecided {
                cause,
                stats: run.stats(),
            },
            Err(End::Stopped(cause, limit)) => SafetyOutcome::Stopped {
                cause,
                limit,
                stats: run.stats(),
            },
            Err(End::Fault(fault)) => SafetyOutcome::Fault {
                fault,
                stats: run.stats(),
            },
            Err(End::Violated(failure)) => SafetyOutcome::Violated {
                counterexample: Box::new(run.counterexample(*failure)),
                stats: run.stats(),
            },
        }
    };
    let liveness = if refinement.abstract_fairness().is_empty() {
        None
    } else if refinement
        .objects()
        .iter()
        .any(|object| !object.hidden.is_empty())
    {
        Some(LivenessHalf::Unsupported(
            UnavailabilityCause::HiddenFieldLiveness,
        ))
    } else if refinement
        .abstract_fairness()
        .iter()
        .any(|constraint| constraint.kind() == FairnessKind::Strong)
    {
        // A hidden field is reported first (above), so a refinement with
        // both gets `HiddenFieldLiveness`. The second phase reads a class as weak (enabled at every state of
        // the loop); a strong class needs another reading the spec has not
        // given, so the half is not decided.
        Some(LivenessHalf::Unsupported(
            UnavailabilityCause::StrongAbstractFairness,
        ))
    } else if matches!(safety, SafetyOutcome::Holds { .. }) {
        Some(LivenessHalf::Checked(run.liveness()?))
    } else {
        None
    };
    Ok(RefinementOutcome { safety, liveness })
}

struct Run<'r, 'a> {
    refinement: &'r CheckedRefinement,
    concrete: &'r ModelSystem<'a>,
    abstract_system: &'r ModelSystem<'a>,
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    meter: quire_exact::ScalarLimits,
    max_candidates: u64,
    cancel: &'r Cancel,
    nodes: Vec<Node>,
    /// Each product state's index by its key: the concrete state's key
    /// bytes, its history values' key and its candidates' sorted state
    /// keys.
    known: BTreeMap<ProductKey, usize>,
    queue: VecDeque<usize>,
    /// The position of each concrete initial state, in the subject's order.
    initial_positions: Vec<usize>,
    /// FR-101's `max_transitions` count: every concrete edge explored and
    /// every abstract transition `check_step` judged or expanded.
    transitions: usize,
    /// The concrete edges explored.
    edges: usize,
    horizon_cut: bool,
    classes_of: ClassesOf,
}

impl Run<'_, '_> {
    fn stats(&self) -> RunStats {
        RunStats {
            product_states: widen(self.nodes.len()),
            product_edges: widen(self.edges),
            automaton_states: 0,
            depth: widen(self.nodes.iter().map(|node| node.depth).max().unwrap_or(0)),
            bound_limited: 0,
        }
    }

    fn poll(&self) -> Result<(), End> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(End::Stopped(IncompleteCause::Cancelled { source }, None));
            }
        }
        Ok(())
    }

    fn stopped(limit: ModelCheckLimit, value: u64) -> End {
        End::Stopped(
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    }

    /// An expansion stop: a named budget settles V-7 naming it, a broken
    /// invariant is a fault, any other stop settles V-7 with no limit.
    fn expansion_stop(&self, stop: ExpansionStop) -> End {
        match stop.limit {
            Some(ExpansionLimit::EvaluationMeter) => {
                Self::stopped(ModelCheckLimit::EvaluationMeter, self.meter.work_units)
            }
            Some(ExpansionLimit::MaxCandidates) => {
                Self::stopped(ModelCheckLimit::MaxCandidates, self.max_candidates)
            }
            None if stop.is_internal_failure() => {
                End::Fault(InternalFault::new("refinement-check", "model-expansion"))
            }
            None => End::Stopped(IncompleteCause::ResourceExhausted, None),
        }
    }

    fn encoding<T>(result: Result<T, qsl_eval::simulation::EncodingRefusal>) -> Result<T, End> {
        result.map_err(|_| End::Fault(InternalFault::new("refinement-check", "state-encodes")))
    }

    /// The breadth-first safety phase.
    fn explore(&mut self) -> Result<(), End> {
        self.poll()?;
        let mut initial = Vec::new();
        for (index, state) in self.concrete.initial_states().iter().enumerate() {
            let (key, _) = Self::encoding(self.concrete.state_key(state))?;
            initial.push((key, index, state));
        }
        let keys: Vec<Vec<u8>> = initial.iter().map(|(key, _, _)| key.clone()).collect();
        let mut at_key = BTreeMap::new();
        // Equal keys coalesce into the first initial state that has the key.
        initial.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        initial.dedup_by(|later, earlier| later.0 == earlier.0);
        if initial.len() > self.limits.limits.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        for (key, index, state) in initial {
            let verdict = check_initial(
                self.refinement,
                self.concrete,
                self.abstract_system,
                index,
                state.state(),
            );
            let (position, _) = self.judge(verdict, None, None, index)?;
            let at = self.create(
                key.clone(),
                Node {
                    state: state.clone(),
                    position,
                    depth: 0,
                    parent: None,
                    initial_index: index,
                    edges: Vec::new(),
                },
            )?;
            at_key.insert(key, at);
        }
        for key in keys {
            let Some(&at) = at_key.get(&key) else {
                return Err(End::Fault(InternalFault::new(
                    "refinement-check",
                    "every-initial-state-has-a-position",
                )));
            };
            self.initial_positions.push(at);
        }
        while let Some(at) = self.queue.pop_front() {
            self.poll()?;
            self.expand(at)?;
        }
        Ok(())
    }

    /// Check every edge out of node `at`, in canonical order.
    fn expand(&mut self, at: usize) -> Result<(), End> {
        let expansion = self
            .concrete
            .successors(&self.nodes[at].state)
            .map_err(|stop| self.expansion_stop(stop))?;
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            return Err(End::Undecided(SafetyUndecided::Undetermined(
                Undetermined::UndecidedSuccessor,
            )));
        }
        let depth = self.nodes[at].depth;
        if self.max_depth.is_some_and(|horizon| depth >= horizon)
            && !expansion.successors.is_empty()
        {
            // A real successor beyond the horizon is left unexplored. A
            // terminal state's stutter coalesces with the state itself, so
            // it leaves nothing, and its edge is listed below like any other.
            self.horizon_cut = true;
            return Ok(());
        }
        let mut edges = Vec::with_capacity(expansion.successors.len());
        for (transition, state) in expansion.successors {
            let order = Self::encoding(self.concrete.transition_key(&transition))?;
            let (key, digest) = Self::encoding(self.concrete.state_key(&state))?;
            edges.push((order, key, digest, Some(transition), state));
        }
        edges.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        if edges.is_empty() {
            // ADR-018 SM-4: a terminal state repeats forever, one stutter
            // step that is no model step.
            let state = self.nodes[at].state.clone();
            let (key, digest) = Self::encoding(self.concrete.state_key(&state))?;
            edges.push((Vec::new(), key, digest, None, state));
        }
        let mut listed = Vec::with_capacity(edges.len());
        for (_, key, digest, transition, state) in edges {
            if self.transitions >= self.limits.limits.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                ));
            }
            self.transitions += 1;
            self.edges += 1;
            let StepResult {
                verdict,
                abstract_successors,
            } = check_step(
                self.refinement,
                self.concrete,
                self.abstract_system,
                &self.nodes[at].position,
                &ConcreteStep {
                    position: widen(depth.saturating_add(1)),
                    transition: transition.as_ref(),
                    post: &state,
                },
            );
            self.transitions = self
                .transitions
                .saturating_add(usize::try_from(abstract_successors).unwrap_or(usize::MAX));
            let step = transition.clone().map(|transition| ModelStep {
                transition,
                post_state: digest,
            });
            let initial_index = self.nodes[at].initial_index;
            let (position, taken) = self.judge(verdict, Some(at), step.clone(), initial_index)?;
            let post = self.create(
                key,
                Node {
                    state,
                    position,
                    depth: depth.saturating_add(1),
                    parent: step.map(|step| (at, step)),
                    initial_index,
                    edges: Vec::new(),
                },
            )?;
            listed.push(CertifiedEdge {
                transition,
                post: widen(post),
                taken,
            });
        }
        self.nodes[at].edges = listed;
        Ok(())
    }

    /// The position and `Taken` a passing verdict gives, or the end the
    /// verdict is.
    fn judge(
        &self,
        verdict: StepVerdict,
        from: Option<usize>,
        edge: Option<ModelStep>,
        initial_index: usize,
    ) -> Result<(RefinementPosition, Taken), End> {
        let failed = |failure, kind| {
            End::Violated(Box::new(Failure {
                from,
                edge: edge.clone(),
                initial_index,
                failure,
                kind,
            }))
        };
        match verdict {
            StepVerdict::Passes { post, taken } => Ok((post, taken)),
            StepVerdict::Fails(failure) => Err(failed(failure, CounterexampleKind::Formula)),
            StepVerdict::Undefined(evaluation) => {
                let row = match &evaluation.site.locus {
                    qsl_replay::UndefinedLocus::Mapping(site) => site.row.clone(),
                    _ => {
                        return Err(End::Fault(InternalFault::new(
                            "refinement-check",
                            "an-undefined-mapping-names-its-row",
                        )))
                    }
                };
                let failure = RefinementFailure::Undefined {
                    position: evaluation.site.position,
                    row,
                };
                Err(failed(
                    failure,
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site: evaluation.site,
                        cause: evaluation.cause,
                    }),
                ))
            }
            StepVerdict::Undetermined(undetermined) => {
                Err(End::Undecided(SafetyUndecided::Undetermined(undetermined)))
            }
            StepVerdict::Stopped(reached) => Err(End::Stopped(
                IncompleteCause::ResourceExhausted,
                Some(reached),
            )),
            StepVerdict::Fault(fault) => Err(End::Fault(fault)),
        }
    }

    /// The index of the product state of `node`: the one with its key if it
    /// exists, else the new one.
    fn create(&mut self, key: Vec<u8>, node: Node) -> Result<usize, End> {
        let history = Self::encoding(node.position.history.key(self.concrete))?;
        let mut candidates = Vec::with_capacity(node.position.abstract_states.len());
        for candidate in &node.position.abstract_states {
            candidates.push(Self::encoding(self.abstract_system.state_key_of(candidate))?.0);
        }
        let product = (key, history, candidates);
        if let Some(&existing) = self.known.get(&product) {
            return Ok(existing);
        }
        if self.nodes.len() >= self.limits.limits.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        let at = self.nodes.len();
        self.nodes.push(node);
        self.queue.push_back(at);
        self.known.insert(product, at);
        Ok(at)
    }

    /// FR-149's certificate of the finished run: every stored product state
    /// as the digests of its states, each initial state's position and each
    /// state's edges.
    fn certificate(&self) -> Result<SimulationCertificate, InternalFault> {
        let encodes = |_| InternalFault::new("refinement-check", "state-encodes");
        let mut positions = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let concrete = self
                .concrete
                .state_key_of(&node.position.concrete)
                .map_err(encodes)?
                .1;
            let abstract_states = node
                .position
                .abstract_states
                .iter()
                .map(|state| {
                    self.abstract_system
                        .state_key_of(state)
                        .map(|(_, digest)| digest)
                        .map_err(encodes)
                })
                .collect::<Result<_, _>>()?;
            let history = node.position.history.key(self.concrete).map_err(encodes)?;
            positions.push(CertifiedPosition {
                concrete,
                history,
                abstract_states,
            });
        }
        Ok(SimulationCertificate {
            refinement: self.refinement.name().to_owned(),
            positions,
            initial: self.initial_positions.iter().map(|at| widen(*at)).collect(),
            successors: self.nodes.iter().map(|node| node.edges.clone()).collect(),
        })
    }

    /// The canonical counterexample: the breadth-first tree path to the
    /// node the failing edge leaves, then the edge.
    fn counterexample(&self, failure: Failure) -> TemporalCounterexample {
        let mut prefix = Vec::new();
        let mut cursor = failure.from;
        while let Some(index) = cursor {
            match &self.nodes[index].parent {
                Some((parent, step)) => {
                    prefix.push(step.clone());
                    cursor = Some(*parent);
                }
                None => cursor = None,
            }
        }
        prefix.reverse();
        prefix.extend(failure.edge);
        TemporalCounterexample {
            initial_state: failure.initial_index,
            prefix,
            over_binding: None,
            fairness: Vec::new(),
            kind: failure.kind,
            lasso: None,
            refinement: Some(failure.failure),
            blocked: Vec::new(),
        }
    }
}

/// FR-143: the liveness half.
impl Run<'_, '_> {
    /// Decide the liveness half over the retained graph of a safety half
    /// that holds.
    fn liveness(&mut self) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        match self.second_phase() {
            Ok(outcome) => Ok(outcome),
            // The phase asks only the abstract model's successors, so the one
            // thing it cannot decide is a contract conjunction.
            Err(End::Undecided(_)) => Ok(ModelCheckOutcome::Undecided {
                cause: UndecidedCause::UndecidedSuccessor,
                stats: self.stats(),
            }),
            Err(End::Stopped(cause, limit)) => Ok(ModelCheckOutcome::Stopped {
                cause,
                limit,
                stats: self.stats(),
            }),
            Err(End::Fault(fault)) => Err(ModelCheckRefusal::Fault(fault)),
            Err(End::Refusal(refusal)) => Err(refusal),
            Err(End::Violated(_)) => Err(fault("the-second-phase-returns-its-lasso")),
        }
    }

    /// The transition identities the abstract subject enables from each
    /// product state's mapped state (CO-2), in the order of `nodes`.
    fn abstract_enabled(&mut self) -> Result<Vec<Vec<ModelTransition>>, End> {
        let mut enabled = Vec::with_capacity(self.nodes.len());
        let mut transitions = self.transitions;
        for node in &self.nodes {
            self.poll()?;
            let mut here = Vec::new();
            for candidate in &node.position.abstract_states {
                if transitions >= self.limits.limits.max_transitions {
                    return Err(Self::stopped(
                        ModelCheckLimit::MaxTransitions,
                        widen(self.limits.limits.max_transitions),
                    ));
                }
                let expansion = self
                    .abstract_system
                    .expand_state(candidate)
                    .map_err(|stop| self.expansion_stop(stop))?;
                transitions = transitions.saturating_add(expansion.successors.len());
                if expansion
                    .findings
                    .iter()
                    .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
                {
                    return Err(End::Undecided(SafetyUndecided::Undetermined(
                        Undetermined::UndecidedSuccessor,
                    )));
                }
                here.extend(
                    expansion
                        .successors
                        .into_iter()
                        .map(|(transition, _)| transition),
                );
            }
            enabled.push(here);
        }
        self.transitions = transitions;
        Ok(enabled)
    }

    /// The second phase: the first violated class of `F_A`, as its lasso, or
    /// `Holds` when none is violated.
    fn second_phase(&mut self) -> Result<ModelCheckOutcome, End> {
        self.poll()?;
        let enabled = self.abstract_enabled()?;
        let mut identities = BTreeMap::new();
        for transition in enabled.iter().flatten() {
            let order = Self::encoding(self.abstract_system.transition_key(transition))?;
            identities
                .entry(order)
                .or_insert_with(|| transition.clone());
        }
        let identities: Vec<ModelTransition> = identities.into_values().collect();
        let classes = (self.classes_of)(
            self.abstract_system,
            self.refinement.abstract_fairness(),
            &identities,
        )
        .map_err(End::Fault)?;
        let transitions: Vec<ModelTransition> = self
            .nodes
            .iter()
            .flat_map(|node| &node.edges)
            .filter_map(|edge| edge.transition.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let numbered: BTreeMap<&ModelTransition, Identity> = transitions
            .iter()
            .enumerate()
            .map(|(index, transition)| (transition, Identity::new(widen(index))))
            .collect();
        let identity = |transition: &ModelTransition| {
            numbered
                .get(transition)
                .copied()
                .ok_or_else(|| fault("every-edge-transition-is-numbered"))
        };
        let constraints = fairness_constraints(
            self.concrete,
            self.refinement.concrete_fairness(),
            &transitions,
            identity,
        )?;
        for class in &classes {
            let (graph, vertices) = self.fair_graph(class, &identity)?;
            let within: BTreeSet<Vertex> = vertices
                .iter()
                .zip(&enabled)
                .filter(|(_, here)| class.is_enabled_in(here))
                .map(|(vertex, _)| *vertex)
                .collect();
            let analyses = match analyze(&graph, &within, &constraints, self.cancel) {
                Ok(analyses) => analyses,
                Err(FairnessError::Cancelled) => {
                    self.poll()?;
                    return Err(End::Fault(InternalFault::new(
                        "refinement-check",
                        "a-cancelled-run-has-a-cause",
                    )));
                }
                Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                    return Err(fault("the-fairness-graph-is-built-from-the-product").into())
                }
            };
            if let Some(passing) = Analysis::first_fair_in(&analyses) {
                let model = FairModel { graph, constraints };
                return self.lasso(class, &model, passing, &transitions);
            }
        }
        Ok(ModelCheckOutcome::HoldsReduced {
            reductions: Vec::new(),
            stats: self.stats(),
        })
    }

    /// The retained graph as the fairness kernels read it for `class`: a
    /// vertex per product state holding the concrete identities enabled
    /// there, and an edge per retained edge that does not take `class`.
    fn fair_graph(
        &self,
        class: &AbstractClass,
        identity: &impl Fn(&ModelTransition) -> Result<Identity, ModelCheckRefusal>,
    ) -> Result<(Graph, Vec<Vertex>), ModelCheckRefusal> {
        let mut graph = Graph::new();
        let mut vertices = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let enabled = node
                .edges
                .iter()
                .filter_map(|edge| edge.transition.as_ref())
                .map(identity)
                .collect::<Result<Vec<_>, _>>()?;
            vertices.push(graph.add_vertex(enabled));
        }
        let vertex = |at: usize| {
            vertices
                .get(at)
                .copied()
                .ok_or_else(|| fault("an-edge-names-a-product-state"))
        };
        for (source, node) in self.nodes.iter().enumerate() {
            for edge in node
                .edges
                .iter()
                .filter(|edge| !class.is_taken_by(&edge.taken))
            {
                let label = match &edge.transition {
                    Some(transition) => EdgeLabel::Transition(identity(transition)?),
                    None => EdgeLabel::Stutter,
                };
                let target = usize::try_from(edge.post)
                    .map_err(|_| fault("an-edge-names-a-product-state"))?;
                graph
                    .add_edge(vertex(source)?, vertex(target)?, label)
                    .map_err(|_| fault("an-edge-joins-two-vertices"))?;
            }
        }
        Ok((graph, vertices))
    }

    /// The canonical lasso into the passing component `passing`, which
    /// violates `class`: the tree path to its first state, then CX-5's greedy
    /// walk inside it. `Divergence` when every loop edge is a `stutter`,
    /// the concrete terminal stutter included, `AbstractUnfair` otherwise.
    fn lasso(
        &self,
        class: &AbstractClass,
        model: &FairModel,
        passing: &Analysis,
        transitions: &[ModelTransition],
    ) -> Result<ModelCheckOutcome, End> {
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let entry = *passing
            .vertices()
            .first()
            .ok_or_else(|| fault("a-component-has-a-vertex"))?;
        let walk = model.greedy_loop(&component, passing.removed(), entry)?;
        let mut prefix = Vec::new();
        let mut cursor = entry.index();
        while let Some((parent, step)) = &self.nodes[cursor].parent {
            prefix.push(step.clone());
            cursor = *parent;
        }
        prefix.reverse();
        let initial_state = self.nodes[cursor].initial_index;
        let entry_position = prefix.len();
        let mut stutters = 0;
        let mut taking = false;
        for (from, edge) in &walk {
            let transition = match edge.label {
                EdgeLabel::Transition(identity) => Some(
                    usize::try_from(identity.value())
                        .ok()
                        .and_then(|index| transitions.get(index))
                        .ok_or_else(|| fault("a-loop-edge-is-a-numbered-transition"))?
                        .clone(),
                ),
                EdgeLabel::Stutter => None,
            };
            let target = edge.target.index();
            let certified = self.nodes[from.index()]
                .edges
                .iter()
                .find(|certified| {
                    certified.post == widen(target) && certified.transition == transition
                })
                .ok_or_else(|| fault("a-loop-edge-is-a-retained-edge"))?;
            taking |= certified.taken != Taken::Stutter;
            match transition {
                Some(transition) => {
                    let (_, post_state) =
                        Self::encoding(self.concrete.state_key(&self.nodes[target].state))?;
                    prefix.push(ModelStep {
                        transition,
                        post_state,
                    });
                }
                None => stutters += 1,
            }
        }
        // A terminal state has only its stutter edge, so a loop holds
        // stutter edges alone or model edges alone.
        if stutters > 0 && stutters != walk.len() {
            return Err(fault("a-loop-is-all-stutter-or-all-model-steps").into());
        }
        let constraint = self
            .refinement
            .abstract_fairness()
            .get(class.constraint())
            .ok_or_else(|| fault("a-class-names-a-constraint-of-the-set"))?
            .clone();
        let failure = if taking {
            RefinementFailure::AbstractUnfair { constraint }
        } else {
            RefinementFailure::Divergence { constraint }
        };
        Ok(ModelCheckOutcome::Violated {
            counterexample: Box::new(TemporalCounterexample {
                initial_state,
                prefix,
                over_binding: None,
                fairness: self.refinement.concrete_fairness().to_vec(),
                kind: CounterexampleKind::Formula,
                lasso: Some(Lasso {
                    entry: entry_position,
                    terminal_stutter: stutters > 0,
                }),
                refinement: Some(failure),
                blocked: Vec::new(),
            }),
            stats: self.stats(),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/support/refinement_cas.rs"]
mod cas_fixture;

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::cas_fixture::{cas_refines_counter, World, UNLIMITED};
    use super::*;

    fn request(world: &World) -> RefinementCheckRequest<'_> {
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        }
    }

    fn failing(
        _: &ModelSystem<'_>,
        _: &[FairnessConstraint],
        _: &[ModelTransition],
    ) -> Result<Vec<AbstractClass>, InternalFault> {
        Err(InternalFault::new("refinement-liveness", "injected"))
    }

    /// FR-143-AC-7 (TC-548 step 7): an internal fault of the second phase,
    /// injected into a refinement whose safety half holds, makes the check
    /// return a refusal carrying the fault and no outcome; with the real
    /// classes the same request returns its outcome.
    #[trace("TC-548", "FR-143-AC-7")]
    #[test]
    fn a_second_phase_fault_is_a_refusal_carrying_the_fault_and_no_outcome() {
        let world = World::new(&cas_refines_counter());
        let cancel = Cancel::new();
        let outcome = check_with(request(&world), &cancel, abstract_classes)
            .expect("the refinement is checked");
        assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
        assert!(outcome.liveness.is_some());

        let Err(ModelCheckRefusal::Fault(fault)) = check_with(request(&world), &cancel, failing)
        else {
            panic!("the second phase's fault is a refusal");
        };
        assert_eq!(fault, InternalFault::new("refinement-liveness", "injected"));
    }
}
