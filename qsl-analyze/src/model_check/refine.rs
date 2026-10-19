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
//! A concrete state's abstract state is a function of the concrete state
//! alone until history fields and hidden fields exist, so a product state is
//! keyed by the concrete state's key and its mapped state's key. Hidden
//! fields need the AX-2 candidate set and are refused before any state is
//! explored. The liveness half (FR-143) is not built.

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{check_model_bounds, ModelFinding, ModelSimState, ModelSystem};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop, TransitionSystem};
use qsl_foundation::InternalFault;
use qsl_package::CheckedPackage;
use qsl_replay::{
    check_initial, check_step, open_systems, ConcreteStep, CounterexampleKind, IncompleteCause,
    ModelCheckLimit, ModelCheckLimits, ModelStep, ReachedLimit, RefinementFailure,
    RefinementOutcome, RefinementPosition, RefinementSystems, RunStats, SafetyOutcome,
    SafetyUndecided, StepVerdict, SubjectRefusal, TemporalCounterexample, UndefinedEvaluation,
    Undetermined,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;

use super::check::{fault, not_simulated, widen, ModelCheckRefusal, BOUND_POSITIONS};

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

/// One product state: the concrete state, the position `check_step` gave it,
/// the step that first reached it (`None` for an initial state) and its
/// breadth-first depth.
struct Node {
    state: ModelSimState,
    position: RefinementPosition,
    depth: usize,
    parent: Option<(usize, ModelStep)>,
    initial_index: usize,
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

/// FR-142: decide the safety half of `request`'s refinement, polling
/// `cancel` before every expansion and every edge.
///
/// # Errors
///
/// [`ModelCheckRefusal`] when a subject does not admit, a root of either is
/// unbounded, or the refinement has a hidden field; nothing is explored then.
pub fn check_refinement(
    request: RefinementCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<RefinementOutcome, ModelCheckRefusal> {
    let refinement = request.refinement;
    if refinement
        .objects()
        .iter()
        .any(|object| !object.hidden.is_empty())
    {
        return Err(ModelCheckRefusal::HiddenFields);
    }
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
        SubjectRefusal::NoModels | SubjectRefusal::NoUniverse => {
            fault("the-subjects-of-a-refinement-open")
        }
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
        transitions: 0,
        edges: 0,
        horizon_cut: false,
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
            Ok(()) => SafetyOutcome::Holds { stats: run.stats() },
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
    Ok(RefinementOutcome { safety })
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
    /// bytes and the mapped state's.
    known: BTreeMap<(Vec<u8>, Vec<u8>), usize>,
    queue: VecDeque<usize>,
    /// FR-101's `max_transitions` count: every concrete edge explored and
    /// every abstract transition `check_step` judged or expanded.
    transitions: usize,
    /// The concrete edges explored.
    edges: usize,
    horizon_cut: bool,
}

impl Run<'_, '_> {
    fn stats(&self) -> RunStats {
        RunStats {
            product_states: widen(self.nodes.len()),
            product_edges: widen(self.edges),
            automaton_states: 0,
            depth: widen(self.nodes.iter().map(|node| node.depth).max().unwrap_or(0)),
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
            let position = self.judge(verdict, None, None, index)?;
            self.create(
                key,
                Node {
                    state: state.clone(),
                    position,
                    depth: 0,
                    parent: None,
                    initial_index: index,
                },
            )?;
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
        if self.max_depth.is_some_and(|horizon| depth >= horizon) {
            // A terminal state's stutter coalesces with the state itself, so
            // only a real successor leaves anything unexplored.
            self.horizon_cut |= !expansion.successors.is_empty();
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
        for (_, key, digest, transition, state) in edges {
            if self.transitions >= self.limits.limits.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                ));
            }
            self.transitions += 1;
            self.edges += 1;
            let mut abstract_successors = 0;
            let verdict = check_step(
                self.refinement,
                self.concrete,
                self.abstract_system,
                &self.nodes[at].position,
                &ConcreteStep {
                    position: widen(depth.saturating_add(1)),
                    transition: transition.as_ref(),
                    post: &state,
                },
                &mut abstract_successors,
            );
            self.transitions = self
                .transitions
                .saturating_add(usize::try_from(abstract_successors).unwrap_or(usize::MAX));
            let edge = transition.map(|transition| ModelStep {
                transition,
                post_state: digest,
            });
            let initial_index = self.nodes[at].initial_index;
            let position = self.judge(verdict, Some(at), edge.clone(), initial_index)?;
            self.create(
                key,
                Node {
                    state,
                    position,
                    depth: depth.saturating_add(1),
                    parent: edge.map(|edge| (at, edge)),
                    initial_index,
                },
            )?;
        }
        Ok(())
    }

    /// The position a passing verdict gives, or the end the verdict is.
    fn judge(
        &self,
        verdict: StepVerdict,
        from: Option<usize>,
        edge: Option<ModelStep>,
        initial_index: usize,
    ) -> Result<RefinementPosition, End> {
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
            StepVerdict::Passes { post, .. } => Ok(post),
            StepVerdict::Fails(failure) => Err(failed(failure, CounterexampleKind::Formula)),
            StepVerdict::Undefined(evaluation) => {
                let row = match &evaluation.site.locus {
                    qsl_replay::UndefinedLocus::Mapping { row, .. } => row.clone(),
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
            StepVerdict::Fault(fault) => Err(End::Fault(fault)),
        }
    }

    /// Add the product state of `node` unless one with its key exists.
    fn create(&mut self, key: Vec<u8>, node: Node) -> Result<(), End> {
        let [mapped] = node.position.abstract_states.as_slice() else {
            return Err(End::Fault(InternalFault::new(
                "refinement-check",
                "one-abstract-state-without-hidden-fields",
            )));
        };
        let (mapped_key, _) = Self::encoding(self.abstract_system.state_key_of(mapped))?;
        let product = (key, mapped_key);
        if self.known.contains_key(&product) {
            return Ok(());
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
        Ok(())
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
        }
    }
}
