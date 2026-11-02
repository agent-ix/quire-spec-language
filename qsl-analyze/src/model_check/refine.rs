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
use qsl_eval::model_subject::{ModelInstance, ModelSubject};
use qsl_eval::simulation::model::{check_model_bounds, ModelFinding, ModelSimState, ModelSystem};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop, TransitionSystem};
use qsl_package::CheckedPackage;
use qsl_replay::{
    abstract_classes, check_initial, check_protocol_initial, check_protocol_state_relations,
    check_protocol_step, check_step, open_protocol_systems, open_systems, AbstractClass,
    AbstractProtocolSystem, CertifiedEdge, CertifiedPosition, ConcreteStep, CounterexampleKind,
    IncompleteCause, Lasso, LivenessHalf, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ModelStep, ProtocolRelationError, ProtocolStateRelation, ReachedLimit, RefinementFailure,
    RefinementOutcome, RefinementPosition, RefinementSystems, RunStats, SafetyOutcome,
    SafetyUndecided, SimulationCertificate, StepResult, StepVerdict, SubjectRefusal, Taken,
    TemporalCounterexample, UnavailabilityCause, UndecidedCause, UndefinedEvaluation, Undetermined,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;
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
    /// Bindings for an abstract protocol subject. The clause and static
    /// roles remain owned by `abstract_package`; the request supplies only
    /// the model instance bound to its `over` parameter.
    pub abstract_protocol_binding: Option<AbstractProtocolBinding>,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
}

/// Request-owned binding for the selected abstract protocol subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractProtocolBinding {
    /// The object bound to the protocol's checked `over` input.
    pub over: ModelInstance,
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

/// One candidate-specific edge in the paired concrete/protocol graph. The
/// stored model product edge may admit several protocol candidates; this
/// edge keeps one coherent pre, transition and post triple for fairness.
#[derive(Clone, Eq, PartialEq)]
struct CandidateEdge {
    target: usize,
    concrete: Option<ModelTransition>,
    protocol: Option<qsl_eval::simulation::protocol::ProtocolTransition>,
}

/// Source identities carried by one paired transition in the protocol
/// assumption graph. The empty pair is the concrete terminal stutter.
type PairedIdentityKey = (Option<Vec<u8>>, Option<Vec<u8>>);

/// One vertex in the candidate-specific product, retaining the concrete
/// product node and exactly one abstract protocol candidate.
struct CandidateNode {
    product: usize,
    candidate: qsl_replay::ProtocolCandidate,
    edges: Vec<CandidateEdge>,
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
    let opened = match (
        refinement.abstract_side(),
        request.abstract_protocol_binding,
    ) {
        (qsl_semantics::check::AbstractSide::Model, _) => open_systems(
            refinement,
            &request.concrete.simulation,
            request.abstract_package,
            request.abstract_initial.clone(),
        ),
        (qsl_semantics::check::AbstractSide::Protocol { .. }, Some(binding)) => {
            open_protocol_systems(
                refinement,
                &request.concrete.simulation,
                request.abstract_package,
                request.abstract_initial.clone(),
                binding.over,
            )
        }
        (qsl_semantics::check::AbstractSide::Protocol { .. }, None) => {
            return Err(ModelCheckRefusal::ProtocolAbstractSide)
        }
    };
    let systems = opened.map_err(|refusal| match refusal {
        SubjectRefusal::Admission(failure) => ModelCheckRefusal::Admission(failure),
        SubjectRefusal::ProtocolSubject => ModelCheckRefusal::ProtocolRefinement,
        SubjectRefusal::NoModels | SubjectRefusal::NoUniverse => {
            fault("the-subjects-of-a-refinement-open")
        }
        SubjectRefusal::ProtocolAbstractSide => ModelCheckRefusal::ProtocolAbstractSide,
    })?;
    let protocol_system = systems.abstract_protocol.as_ref();
    let RefinementSystems {
        concrete,
        abstract_system,
        ..
    } = systems;
    let simulation = &request.concrete.simulation;
    check_model_bounds(&concrete, BOUND_POSITIONS).map_err(not_simulated)?;
    check_model_bounds(&abstract_system, BOUND_POSITIONS).map_err(not_simulated)?;
    let mut run = Run {
        refinement,
        concrete: &concrete,
        abstract_system: &abstract_system,
        abstract_protocol: protocol_system,
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
            Ok(()) if run.abstract_protocol.is_some() => match run.protocol_divergence() {
                Ok(Some(counterexample)) => SafetyOutcome::Violated {
                    counterexample: Box::new(counterexample),
                    stats: run.stats(),
                },
                Ok(None) => match run.protocol_assumption_strengthening() {
                    Ok(()) => SafetyOutcome::HoldsUncertified { stats: run.stats() },
                    Err(End::Stopped(cause, limit)) => SafetyOutcome::Stopped {
                        cause,
                        limit,
                        stats: run.stats(),
                    },
                    Err(End::Fault(fault)) => SafetyOutcome::Fault {
                        fault,
                        stats: run.stats(),
                    },
                    Err(End::Undecided(cause)) => SafetyOutcome::Undecided {
                        cause,
                        stats: run.stats(),
                    },
                    Err(End::Refusal(refusal)) => return Err(refusal),
                    Err(End::Violated(failure)) => SafetyOutcome::Violated {
                        counterexample: Box::new(run.counterexample(*failure)),
                        stats: run.stats(),
                    },
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
                Err(End::Undecided(cause)) => SafetyOutcome::Undecided {
                    cause,
                    stats: run.stats(),
                },
                Err(End::Refusal(refusal)) => return Err(refusal),
                Err(End::Violated(failure)) => SafetyOutcome::Violated {
                    counterexample: Box::new(run.counterexample(*failure)),
                    stats: run.stats(),
                },
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
    let liveness = if refinement.abstract_operation_fairness().next().is_none() {
        None
    } else if run.abstract_protocol.is_some()
        || refinement
            .objects()
            .iter()
            .any(|object| !object.hidden.is_empty())
    {
        Some(LivenessHalf::Unsupported(
            UnavailabilityCause::HiddenFieldLiveness,
        ))
    } else if refinement
        .abstract_operation_fairness()
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
    abstract_protocol: Option<&'r AbstractProtocolSystem<'a>>,
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
    /// Reconstruct the candidate-specific graph retained by the set-valued
    /// refinement product. `Taken::Protocol` is intentionally a union at the
    /// step API; pairing each transition with its admitted pre/post candidate
    /// here avoids combining incompatible abstract paths in fairness checks.
    fn candidate_graph(&mut self) -> Result<(Vec<CandidateNode>, Vec<usize>), End> {
        let Some(protocol) = self.abstract_protocol else {
            return Err(End::Fault(InternalFault::new(
                "refinement-check",
                "candidate-graph-requires-protocol",
            )));
        };
        let mut candidates = Vec::<(usize, qsl_replay::ProtocolCandidate, Vec<u8>, bool)>::new();
        let mut by_product_key = BTreeMap::<(usize, Vec<u8>), usize>::new();
        for (product, node) in self.nodes.iter().enumerate() {
            for candidate in &node.position.abstract_protocol_states {
                let (key, _) = protocol.state_key(candidate).map_err(|_| {
                    End::Fault(InternalFault::new(
                        "refinement-check",
                        "protocol-candidate-state-key",
                    ))
                })?;
                let pair = (product, key.clone());
                if by_product_key.contains_key(&pair) {
                    continue;
                }
                let index = candidates.len();
                by_product_key.insert(pair, index);
                candidates.push((product, candidate.clone(), key, true));
            }
        }
        // Retain the protocol-only internal closure candidates between a
        // paired concrete transition and its endpoint. These intermediate
        // states carry fairness classes such as the selected TSO flush.
        for (product, node) in self.nodes.iter().enumerate() {
            for edge in &node.edges {
                let Taken::ProtocolTraces(traces) = &edge.taken else {
                    continue;
                };
                let target_product = usize::try_from(edge.post).map_err(|_| {
                    End::Fault(InternalFault::new(
                        "refinement-check",
                        "candidate-product-edge-target",
                    ))
                })?;
                for trace in traces {
                    let start_pair = (product, trace.start_key.clone());
                    if !by_product_key.contains_key(&start_pair) {
                        return Err(End::Fault(InternalFault::new(
                            "refinement-check",
                            "protocol-trace-start-is-a-candidate",
                        )));
                    }
                    for (candidate, key) in trace.candidates.iter().zip(&trace.candidate_keys) {
                        let pair = (target_product, key.clone());
                        if let Some(index) = by_product_key.get(&pair).copied() {
                            candidates[index].3 = true;
                            continue;
                        }
                        let index = candidates.len();
                        by_product_key.insert(pair, index);
                        candidates.push((target_product, candidate.clone(), key.clone(), false));
                    }
                }
            }
        }
        let mut graph = candidates
            .iter()
            .map(|(product, candidate, _, _)| CandidateNode {
                product: *product,
                candidate: candidate.clone(),
                edges: Vec::new(),
            })
            .collect::<Vec<_>>();
        for (source, (product, candidate, _, endpoint)) in candidates.iter().enumerate() {
            self.poll()?;
            if !endpoint {
                continue;
            }
            let Some(node) = self.nodes.get(*product) else {
                return Err(End::Fault(InternalFault::new(
                    "refinement-check",
                    "candidate-product-node-exists",
                )));
            };
            for edge in &node.edges {
                let target_product = usize::try_from(edge.post).map_err(|_| {
                    End::Fault(InternalFault::new(
                        "refinement-check",
                        "candidate-product-edge-target",
                    ))
                })?;
                let target_keys = self
                    .nodes
                    .get(target_product)
                    .ok_or_else(|| {
                        End::Fault(InternalFault::new(
                            "refinement-check",
                            "candidate-product-edge-target-exists",
                        ))
                    })?
                    .position
                    .abstract_protocol_states
                    .iter()
                    .map(|candidate| {
                        protocol
                            .state_key(candidate)
                            .map(|(key, _)| key)
                            .map_err(|_| {
                                End::Fault(InternalFault::new(
                                    "refinement-check",
                                    "protocol-candidate-state-key",
                                ))
                            })
                    })
                    .collect::<Result<BTreeSet<_>, _>>()?;
                match &edge.taken {
                    Taken::Stutter => {
                        let (key, _) = protocol.state_key(candidate).map_err(|_| {
                            End::Fault(InternalFault::new(
                                "refinement-check",
                                "protocol-candidate-state-key",
                            ))
                        })?;
                        if target_keys.contains(&key) {
                            if let Some(target) = by_product_key.get(&(target_product, key)) {
                                graph[source].edges.push(CandidateEdge {
                                    target: *target,
                                    concrete: edge.transition.clone(),
                                    protocol: None,
                                });
                            }
                        }
                    }
                    Taken::Protocol(accepted) => {
                        let accepted = accepted
                            .iter()
                            .filter_map(|transition| protocol.transition_key(transition).ok())
                            .collect::<BTreeSet<_>>();
                        let expansion = protocol
                            .successors(candidate)
                            .map_err(|stop| self.expansion_stop(stop))?;
                        self.transitions =
                            self.transitions.saturating_add(expansion.successors.len());
                        if self.transitions > self.limits.limits.max_transitions {
                            return Err(Self::stopped(
                                ModelCheckLimit::MaxTransitions,
                                widen(self.limits.limits.max_transitions),
                            ));
                        }
                        for (transition, post_candidate) in expansion.successors {
                            let transition_key =
                                protocol.transition_key(&transition).map_err(|_| {
                                    End::Fault(InternalFault::new(
                                        "refinement-check",
                                        "protocol-transition-key",
                                    ))
                                })?;
                            if !accepted.contains(&transition_key) {
                                continue;
                            }
                            let (post_key, _) =
                                protocol.state_key(&post_candidate).map_err(|_| {
                                    End::Fault(InternalFault::new(
                                        "refinement-check",
                                        "protocol-candidate-state-key",
                                    ))
                                })?;
                            if !target_keys.contains(&post_key) {
                                continue;
                            }
                            let Some(target) = by_product_key.get(&(target_product, post_key))
                            else {
                                continue;
                            };
                            graph[source].edges.push(CandidateEdge {
                                target: *target,
                                concrete: edge.transition.clone(),
                                protocol: Some(transition),
                            });
                        }
                    }
                    Taken::ProtocolTraces(traces) => {
                        for trace in traces {
                            let source_key = protocol
                                .state_key(&trace.start)
                                .map_err(|_| {
                                    End::Fault(InternalFault::new(
                                        "refinement-check",
                                        "protocol-trace-start-key",
                                    ))
                                })?
                                .0;
                            if source_key
                                != protocol
                                    .state_key(candidate)
                                    .map_err(|_| {
                                        End::Fault(InternalFault::new(
                                            "refinement-check",
                                            "protocol-candidate-state-key",
                                        ))
                                    })?
                                    .0
                            {
                                continue;
                            }
                            let mut from = source;
                            for (index, (transition, post_candidate)) in
                                trace.transitions.iter().zip(&trace.candidates).enumerate()
                            {
                                let post_key =
                                    trace.candidate_keys.get(index).ok_or_else(|| {
                                        End::Fault(InternalFault::new(
                                            "refinement-check",
                                            "protocol-trace-state-key-exists",
                                        ))
                                    })?;
                                let Some(to) = by_product_key
                                    .get(&(target_product, post_key.clone()))
                                    .copied()
                                else {
                                    return Err(End::Fault(InternalFault::new(
                                        "refinement-check",
                                        "protocol-trace-candidate-exists",
                                    )));
                                };
                                let candidate_edge = CandidateEdge {
                                    target: to,
                                    concrete: (index == 0)
                                        .then(|| edge.transition.clone())
                                        .flatten(),
                                    protocol: Some(transition.clone()),
                                };
                                if !graph[from].edges.contains(&candidate_edge) {
                                    graph[from].edges.push(candidate_edge);
                                }
                                from = to;
                                let _ = post_candidate;
                            }
                        }
                    }
                    Taken::Abstract(_) => {
                        return Err(End::Fault(InternalFault::new(
                            "refinement-check",
                            "protocol-product-edge-has-protocol-taken",
                        )));
                    }
                }
            }
        }
        let initial = self
            .initial_positions
            .iter()
            .flat_map(|product| {
                self.nodes[*product]
                    .position
                    .abstract_protocol_states
                    .iter()
                    .filter_map(|candidate| {
                        protocol
                            .state_key(candidate)
                            .ok()
                            .and_then(|(key, _)| by_product_key.get(&(*product, key)).copied())
                    })
            })
            .collect();
        Ok((graph, initial))
    }

    /// FR-148's fairness-independent observable-divergence check over the
    /// retained safety product.
    fn protocol_divergence(&mut self) -> Result<Option<TemporalCounterexample>, End> {
        let Some(protocol) = self.abstract_protocol else {
            return Ok(None);
        };
        self.validate_candidate_graph()?;
        let internal: BTreeSet<&str> = self
            .refinement
            .abstract_internal()
            .iter()
            .map(|class| class.spelling())
            .collect();
        let mut common = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            self.poll()?;
            let mut each = None::<BTreeSet<String>>;
            for candidate in &node.position.abstract_protocol_states {
                let expansion = protocol
                    .successors(candidate)
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
                self.transitions = self.transitions.saturating_add(expansion.successors.len());
                for _ in &expansion.successors {
                    if self.transitions > self.limits.limits.max_transitions {
                        return Err(Self::stopped(
                            ModelCheckLimit::MaxTransitions,
                            widen(self.limits.limits.max_transitions),
                        ));
                    }
                }
                let classes: BTreeSet<String> = expansion
                    .successors
                    .iter()
                    .filter_map(|(transition, _)| {
                        qsl_replay::protocol_transition_class(protocol, transition)
                    })
                    .filter(|class| !internal.contains(class.as_str()))
                    .collect();
                each = Some(match each {
                    Some(previous) => previous.intersection(&classes).cloned().collect(),
                    None => classes,
                });
            }
            common.push(each.unwrap_or_default().into_iter().next());
        }
        for start in 0..self.nodes.len() {
            let Some(class) = common[start].clone() else {
                continue;
            };
            let mut queue = VecDeque::<(usize, Vec<(usize, usize)>)>::new();
            let mut seen = BTreeSet::from([start]);
            for (edge_index, edge) in self.nodes[start].edges.iter().enumerate() {
                if edge.taken == Taken::Stutter
                    && usize::try_from(edge.post)
                        .is_ok_and(|target| common.get(target).is_some_and(Option::is_some))
                {
                    let target = usize::try_from(edge.post).map_err(|_| {
                        End::Fault(InternalFault::new(
                            "refinement-check",
                            "protocol-cycle-target",
                        ))
                    })?;
                    if target == start {
                        return self.protocol_divergence_counterexample(start, class);
                    }
                    if seen.insert(target) {
                        queue.push_back((target, vec![(start, edge_index)]));
                    }
                }
            }
            while let Some((at, path)) = queue.pop_front() {
                for (edge_index, edge) in self.nodes[at].edges.iter().enumerate() {
                    let Ok(target) = usize::try_from(edge.post) else {
                        continue;
                    };
                    if edge.taken != Taken::Stutter
                        || common.get(target).is_none_or(Option::is_none)
                    {
                        continue;
                    }
                    let mut next = path.clone();
                    next.push((at, edge_index));
                    if target == start {
                        return self
                            .protocol_divergence_counterexample_with_loop(start, class, next);
                    }
                    if seen.insert(target) {
                        queue.push_back((target, next));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Validate the candidate-specific projection before running protocol
    /// liveness and environment-relation checks. Every retained set-valued
    /// concrete edge must have at least one coherent candidate witness.
    fn validate_candidate_graph(&mut self) -> Result<(), End> {
        let (graph, initial) = self.candidate_graph()?;
        let Some(protocol) = self.abstract_protocol else {
            return Err(End::Fault(InternalFault::new(
                "refinement-check",
                "candidate-graph-requires-protocol",
            )));
        };
        if graph.is_empty() || initial.is_empty() {
            return Err(End::Fault(InternalFault::new(
                "refinement-check",
                "protocol-candidate-graph-has-initials",
            )));
        }
        for candidate in &graph {
            protocol.state_key(&candidate.candidate).map_err(|_| {
                End::Fault(InternalFault::new(
                    "refinement-check",
                    "candidate-vertex-retains-checked-state",
                ))
            })?;
        }
        for (product, node) in self.nodes.iter().enumerate() {
            for retained in &node.edges {
                let target = usize::try_from(retained.post).map_err(|_| {
                    End::Fault(InternalFault::new(
                        "refinement-check",
                        "candidate-edge-target-index",
                    ))
                })?;
                let witnessed = graph
                    .iter()
                    .filter(|candidate| candidate.product == product)
                    .flat_map(|candidate| &candidate.edges)
                    .any(|candidate_edge| {
                        graph
                            .get(candidate_edge.target)
                            .is_some_and(|post| post.product == target)
                            && candidate_edge.concrete == retained.transition
                            && matches!(
                                (&retained.taken, &candidate_edge.protocol),
                                (Taken::Stutter, None)
                                    | (Taken::Protocol(_), Some(_))
                                    | (Taken::ProtocolTraces(_), Some(_))
                            )
                    });
                if !witnessed {
                    return Err(End::Fault(InternalFault::new(
                        "refinement-check",
                        "retained-product-edge-has-coherent-candidate-witness",
                    )));
                }
            }
        }
        Ok(())
    }

    /// FR-148's first assumption-relation slice: find a paired lasso that
    /// satisfies the protocol's environment assumptions and violates a
    /// concrete `assume` class. The paired graph retains one candidate per
    /// vertex, so incompatible protocol paths are never unioned.
    fn protocol_assumption_strengthening(&mut self) -> Result<(), End> {
        let Some(protocol) = self.abstract_protocol else {
            return Ok(());
        };
        let concrete_fairness: Vec<_> = self
            .refinement
            .concrete_operation_fairness()
            .cloned()
            .collect();
        if concrete_fairness.is_empty() {
            return Ok(());
        }
        let protocol_assumptions = protocol
            .assumption_fairness()
            .map_err(|_| fault("the-admitted-protocol-fairness-set-builds"))?;
        let protocol_fairness: Vec<_> = protocol_assumptions
            .iter()
            .map(|assumption| assumption.fairness.clone())
            .collect();

        let (candidates, initial) = self.candidate_graph()?;
        if candidates.is_empty() || initial.is_empty() {
            return Err(End::Fault(InternalFault::new(
                "refinement-check",
                "protocol-assumption-graph-has-initials",
            )));
        }

        let mut enabled_by_node = Vec::with_capacity(candidates.len());
        let mut pair_keys = BTreeSet::<PairedIdentityKey>::new();
        let mut concrete_by_key = BTreeMap::<Vec<u8>, ModelTransition>::new();
        let mut protocol_by_key =
            BTreeMap::<Vec<u8>, qsl_eval::simulation::protocol::ProtocolTransition>::new();
        for node in &candidates {
            let concrete = self.nodes[node.product]
                .edges
                .iter()
                .filter_map(|edge| edge.transition.as_ref())
                .map(|transition| {
                    self.concrete
                        .transition_key(transition)
                        .map(|key| (key, transition.clone()))
                        .map_err(|_| fault("a-concrete-assumption-transition-has-a-key"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let expansion = protocol
                .successors(&node.candidate)
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
            self.transitions = self.transitions.saturating_add(expansion.successors.len());
            if self.transitions > self.limits.limits.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                ));
            }
            let mut protocol_enabled = Vec::with_capacity(expansion.successors.len());
            for (transition, _) in expansion.successors {
                let key = protocol
                    .transition_key(&transition)
                    .map_err(|_| fault("a-protocol-assumption-transition-has-a-key"))?;
                protocol_by_key.insert(key.clone(), transition);
                protocol_enabled.push(key);
            }
            let mut concrete_enabled = Vec::with_capacity(concrete.len());
            for (key, transition) in concrete {
                concrete_by_key.insert(key.clone(), transition);
                concrete_enabled.push(key);
            }
            for key in &concrete_enabled {
                pair_keys.insert((Some(key.clone()), None));
            }
            for key in &protocol_enabled {
                pair_keys.insert((None, Some(key.clone())));
            }
            for edge in &node.edges {
                let concrete_key = edge
                    .concrete
                    .as_ref()
                    .map(|transition| {
                        self.concrete
                            .transition_key(transition)
                            .map_err(|_| fault("a-concrete-assumption-edge-has-a-key"))
                    })
                    .transpose()?;
                let protocol_key = edge
                    .protocol
                    .as_ref()
                    .map(|transition| {
                        protocol
                            .transition_key(transition)
                            .map_err(|_| fault("a-protocol-assumption-edge-has-a-key"))
                    })
                    .transpose()?;
                if concrete_key.is_some() || protocol_key.is_some() {
                    pair_keys.insert((concrete_key, protocol_key));
                }
            }
            enabled_by_node.push((concrete_enabled, protocol_enabled));
        }
        let identities: BTreeMap<_, _> = pair_keys
            .into_iter()
            .enumerate()
            .map(|(index, key)| {
                u64::try_from(index)
                    .map(|number| (key, Identity::new(number)))
                    .map_err(|_| fault("paired-assumption-identities-fit-u64"))
            })
            .collect::<Result<_, _>>()?;
        let source_ids = |concrete: &Option<Vec<u8>>, abstract_: &Option<Vec<u8>>| {
            identities
                .get(&(concrete.clone(), abstract_.clone()))
                .copied()
                .ok_or_else(|| fault("paired-assumption-identity-is-numbered"))
        };
        let mut graph = Graph::new();
        let mut vertices = Vec::with_capacity(candidates.len());
        for (index, node) in candidates.iter().enumerate() {
            let (concrete, abstract_) = &enabled_by_node[index];
            let mut ids = concrete
                .iter()
                .map(|key| source_ids(&Some(key.clone()), &None))
                .chain(
                    abstract_
                        .iter()
                        .map(|key| source_ids(&None, &Some(key.clone()))),
                )
                .collect::<Result<Vec<_>, _>>()?;
            for edge in &node.edges {
                let concrete_key = edge
                    .concrete
                    .as_ref()
                    .map(|transition| {
                        self.concrete
                            .transition_key(transition)
                            .map_err(|_| fault("a-concrete-assumption-edge-has-a-key"))
                    })
                    .transpose()?;
                let protocol_key = edge
                    .protocol
                    .as_ref()
                    .map(|transition| {
                        protocol
                            .transition_key(transition)
                            .map_err(|_| fault("a-protocol-assumption-edge-has-a-key"))
                    })
                    .transpose()?;
                if concrete_key.is_some() || protocol_key.is_some() {
                    ids.push(source_ids(&concrete_key, &protocol_key)?);
                }
            }
            ids.sort_unstable();
            ids.dedup();
            vertices.push(graph.add_vertex(ids));
        }
        for (source, node) in candidates.iter().enumerate() {
            for edge in &node.edges {
                let concrete_key = edge
                    .concrete
                    .as_ref()
                    .map(|transition| {
                        self.concrete
                            .transition_key(transition)
                            .map_err(|_| fault("a-concrete-assumption-edge-has-a-key"))
                    })
                    .transpose()?;
                let protocol_key = edge
                    .protocol
                    .as_ref()
                    .map(|transition| {
                        protocol
                            .transition_key(transition)
                            .map_err(|_| fault("a-protocol-assumption-edge-has-a-key"))
                    })
                    .transpose()?;
                let label = if concrete_key.is_none() && protocol_key.is_none() {
                    EdgeLabel::Stutter
                } else {
                    EdgeLabel::Transition(source_ids(&concrete_key, &protocol_key)?)
                };
                graph
                    .add_edge(vertices[source], vertices[edge.target], label)
                    .map_err(|_| fault("paired-assumption-edge-joins-existing-vertices"))?;
            }
        }

        let protocol_keys: Vec<_> = protocol_by_key.keys().cloned().collect();
        let protocol_transitions: Vec<_> = protocol_keys
            .iter()
            .map(|key| protocol_by_key[key].clone())
            .collect();
        let protocol_source_ids: BTreeMap<_, _> = protocol_keys
            .iter()
            .enumerate()
            .map(|(index, key)| {
                u64::try_from(index)
                    .map(|number| (key.clone(), Identity::new(number)))
                    .map_err(|_| fault("protocol-assumption-identities-fit-u64"))
            })
            .collect::<Result<_, _>>()?;
        let protocol_source_constraints = qsl_eval::simulation::protocol::protocol_constraints(
            &protocol_fairness,
            &protocol_transitions,
            |transition| {
                let key = protocol.transition_key(transition).map_err(|_| {
                    InternalFault::new(
                        "refinement-check",
                        "protocol-assumption-transition-has-a-key",
                    )
                })?;
                protocol_source_ids.get(&key).copied().ok_or_else(|| {
                    InternalFault::new(
                        "refinement-check",
                        "protocol-assumption-transition-is-numbered",
                    )
                })
            },
        )
        .map_err(|_| fault("protocol-assumption-classes-resolve"))?;
        let mut specification_constraints = Vec::new();
        let mut specification_identities = Vec::new();
        for (member_index, source_constraint) in protocol_source_constraints {
            let Some(source) = protocol_assumptions.get(member_index) else {
                return Err(fault("protocol-assumption-class-names-its-member").into());
            };
            let member = &source.fairness;
            let mut taken_members = Vec::new();
            let mut loaded_members = Vec::new();
            let mut selected_class = None;
            for (key, transition) in protocol_keys.iter().zip(&protocol_transitions) {
                let source_identity = protocol_source_ids[key];
                let source_label = EdgeLabel::Transition(source_identity);
                let takes = source_constraint.is_taken_by(source_label);
                let loads = source_constraint.is_owed_by(source_label);
                if !takes && !loads {
                    continue;
                }
                if selected_class.is_none() {
                    selected_class = member.class_of(transition);
                }
                for ((_, paired_protocol), paired_identity) in &identities {
                    if paired_protocol.as_ref() == Some(key) {
                        if takes {
                            taken_members.push(*paired_identity);
                        }
                        if loads {
                            loaded_members.push(*paired_identity);
                        }
                    }
                }
            }
            let resolved = match source_constraint.obligation() {
                qsl_eval::fairness::Obligation::Enablement(_) => {
                    qsl_eval::fairness::Constraint::new(
                        source_constraint.strength(),
                        taken_members.iter().copied(),
                    )
                }
                qsl_eval::fairness::Obligation::Visibility { .. } => {
                    qsl_eval::fairness::Constraint::visibility(
                        loaded_members,
                        taken_members.iter().copied(),
                    )
                }
            };
            specification_constraints.push(resolved);
            let origin = match member.origin() {
                qsl_eval::simulation::protocol::FairnessOrigin::Authored => {
                    qsl_replay::AssumptionOrigin::Authored
                }
                qsl_eval::simulation::protocol::FairnessOrigin::Scheduler => {
                    qsl_replay::AssumptionOrigin::Scheduler
                }
                qsl_eval::simulation::protocol::FairnessOrigin::Memory => {
                    qsl_replay::AssumptionOrigin::Memory
                }
            };
            let target = source.required_temporal.as_ref().map_or_else(
                || qsl_replay::AssumptionTarget::Protocol(member.clone()),
                |required| qsl_replay::AssumptionTarget::RequiredTemporal {
                    clause: required.clause.clone(),
                    constraint: required.constraint.clone(),
                    fairness: member.clone(),
                },
            );
            specification_identities.push(qsl_replay::AssumptionIdentity {
                side: qsl_replay::AssumptionSide::Specification,
                origin,
                kind: member.kind(),
                granularity: member.granularity(),
                target,
                selected: selected_class.map(qsl_replay::AssumptionSelectedMember::Protocol),
            });
        }

        let mut reachable = BTreeSet::new();
        let mut prefix_roots = BTreeMap::new();
        let mut prefix_parent = BTreeMap::<usize, (usize, usize)>::new();
        let mut queue = VecDeque::new();
        for root in &initial {
            prefix_roots.insert(*root, *root);
            reachable.insert(vertices[*root]);
            queue.push_back(*root);
        }
        while let Some(from) = queue.pop_front() {
            for (edge_index, edge) in candidates[from].edges.iter().enumerate() {
                let to = edge.target;
                if prefix_roots.contains_key(&to) {
                    continue;
                }
                prefix_roots.insert(to, prefix_roots[&from]);
                reachable.insert(vertices[to]);
                prefix_parent.insert(to, (from, edge_index));
                queue.push_back(to);
            }
        }
        for assumption in &concrete_fairness {
            let operation = assumption.operation();
            let text = self
                .concrete
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or_else(|| fault("a-checked-assume-names-a-concrete-operation"))?;
            let selected: Vec<_> = concrete_by_key
                .iter()
                .filter(|(_, transition)| transition.operation() == text)
                .map(|(key, transition)| (key.clone(), transition.clone()))
                .collect();
            let groups: Vec<Vec<(Vec<u8>, ModelTransition)>> = match assumption.granularity() {
                quire_semantic_value::fairness::FairnessGranularity::Whole => {
                    vec![selected.clone()]
                }
                quire_semantic_value::fairness::FairnessGranularity::Each => selected
                    .iter()
                    .cloned()
                    .map(|transition| vec![transition])
                    .collect(),
            };
            for group in groups {
                let group_keys: BTreeSet<_> = group.iter().map(|(key, _)| key.clone()).collect();
                let members: Vec<_> = identities
                    .iter()
                    .filter_map(|((concrete_key, _), identity)| {
                        concrete_key
                            .as_ref()
                            .filter(|key| group_keys.contains(*key))
                            .map(|_| *identity)
                    })
                    .collect();
                let constraint = qsl_eval::fairness::Constraint::new(
                    qsl_eval::fairness::Strength::from(assumption.kind()),
                    members,
                );
                let enabled: BTreeSet<_> = reachable
                    .iter()
                    .copied()
                    .filter(|vertex| {
                        constraint.is_enabled_in(graph.enabled(*vertex).unwrap_or(&BTreeSet::new()))
                            == Some(true)
                    })
                    .collect();
                let mut reduced = Graph::new();
                for vertex in graph.vertices() {
                    reduced.add_vertex(graph.enabled(vertex).into_iter().flatten().copied());
                }
                for from in graph.vertices() {
                    for edge in graph.edges_from(from) {
                        if constraint.is_taken_by(edge.label) {
                            continue;
                        }
                        if assumption.kind() == FairnessKind::Weak
                            && (!enabled.contains(&from) || !enabled.contains(&edge.target))
                        {
                            continue;
                        }
                        reduced
                            .add_edge(from, edge.target, edge.label)
                            .map_err(|_| {
                                fault("filtered-assumption-edge-joins-existing-vertices")
                            })?;
                    }
                }
                if assumption.kind() == FairnessKind::Strong {
                    reduced
                        .add_acceptance_set(enabled.iter().copied())
                        .map_err(|_| {
                            fault("strong-assumption-acceptance-uses-existing-vertices")
                        })?;
                }
                let analyses = match analyze(
                    &reduced,
                    &reachable,
                    &specification_constraints,
                    self.cancel,
                ) {
                    Ok(analyses) => analyses,
                    Err(FairnessError::Cancelled) => {
                        self.poll()?;
                        return Err(End::Fault(InternalFault::new(
                            "refinement-check",
                            "a-cancelled-run-has-a-cause",
                        )));
                    }
                    Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                        return Err(
                            fault("assumption-fairness-analysis-uses-the-paired-graph").into()
                        );
                    }
                };
                let Some(passing) = Analysis::first_fair_in(&analyses) else {
                    continue;
                };
                let component: BTreeSet<_> = passing.vertices().iter().copied().collect();
                let witness_at = *passing
                    .vertices()
                    .first()
                    .ok_or_else(|| fault("a-fair-assumption-component-has-a-vertex"))?;
                let fair_model = FairModel {
                    graph: reduced,
                    constraints: specification_constraints.clone(),
                };
                let loop_edges =
                    fair_model.greedy_loop(&component, passing.removed(), witness_at)?;
                let identity = qsl_replay::AssumptionIdentity {
                    side: qsl_replay::AssumptionSide::Implementation,
                    origin: qsl_replay::AssumptionOrigin::Authored,
                    kind: assumption.kind(),
                    granularity: assumption.granularity(),
                    target: qsl_replay::AssumptionTarget::Model(assumption.clone()),
                    selected: (assumption.granularity()
                        == quire_semantic_value::fairness::FairnessGranularity::Each)
                        .then(|| {
                            group.first().map(|(_, transition)| {
                                qsl_replay::AssumptionSelectedMember::Model(transition.clone())
                            })
                        })
                        .flatten(),
                };
                let root = *prefix_roots
                    .get(&witness_at.index())
                    .ok_or_else(|| fault("fair-assumption-component-is-reachable"))?;
                let mut prefix_edges = Vec::new();
                let mut cursor = witness_at.index();
                while cursor != root {
                    let (parent, edge_index) = *prefix_parent
                        .get(&cursor)
                        .ok_or_else(|| fault("assumption-prefix-has-a-parent-edge"))?;
                    prefix_edges.push((parent, edge_index));
                    cursor = parent;
                }
                prefix_edges.reverse();
                let loop_entry = prefix_edges.len();
                let mut paired_edges: Vec<_> = prefix_edges
                    .into_iter()
                    .map(|(source, edge)| (source, candidates[source].edges[edge].clone()))
                    .collect();
                for (source, graph_edge) in &loop_edges {
                    let source = source.index();
                    let target = graph_edge.target.index();
                    let edge = candidates[source]
                        .edges
                        .iter()
                        .find(|edge| {
                            if edge.target != target {
                                return false;
                            }
                            let concrete_key = edge.concrete.as_ref().and_then(|transition| {
                                self.concrete.transition_key(transition).ok()
                            });
                            let protocol_key = edge
                                .protocol
                                .as_ref()
                                .and_then(|transition| protocol.transition_key(transition).ok());
                            if concrete_key.is_none() && protocol_key.is_none() {
                                graph_edge.label == EdgeLabel::Stutter
                            } else {
                                identities.get(&(concrete_key, protocol_key)).is_some_and(
                                    |identity| graph_edge.label == EdgeLabel::Transition(*identity),
                                )
                            }
                        })
                        .cloned()
                        .ok_or_else(|| fault("assumption-lasso-edge-is-a-paired-product-edge"))?;
                    paired_edges.push((source, edge));
                }
                let path = self.paired_assumption_path(
                    &candidates,
                    root,
                    paired_edges,
                    Some(loop_entry),
                    protocol,
                )?;
                return Err(End::Undecided(
                    SafetyUndecided::ProtocolAssumptionNotEstablished(Box::new(
                        qsl_replay::ProtocolAssumptionWitness {
                            reason: qsl_replay::ProtocolAssumptionReason::Strengthened,
                            identities: std::iter::once(identity)
                                .chain(specification_identities.iter().cloned())
                                .collect(),
                            path,
                            undecided_locus: None,
                        },
                    )),
                ));
            }
        }
        Ok(())
    }

    fn paired_assumption_path(
        &self,
        candidates: &[CandidateNode],
        root: usize,
        edges: Vec<(usize, CandidateEdge)>,
        loop_entry: Option<usize>,
        protocol: &AbstractProtocolSystem<'_>,
    ) -> Result<qsl_replay::PairedAssumptionPath, End> {
        let Some(root_node) = candidates.get(root) else {
            return Err(fault("assumption-witness-root-is-a-candidate").into());
        };
        let (_, concrete_initial) = self
            .concrete
            .state_key(&self.nodes[root_node.product].state)
            .map_err(|_| fault("assumption-witness-concrete-initial-has-a-key"))?;
        let (protocol_initial, _) = protocol
            .state_key(&root_node.candidate)
            .map_err(|_| fault("assumption-witness-protocol-initial-has-a-key"))?;
        let mut steps = Vec::with_capacity(edges.len());
        for (_, edge) in edges {
            let Some(post) = candidates.get(edge.target) else {
                return Err(fault("assumption-witness-edge-target-is-a-candidate").into());
            };
            let (_, concrete_post) = self
                .concrete
                .state_key(&self.nodes[post.product].state)
                .map_err(|_| fault("assumption-witness-concrete-post-has-a-key"))?;
            let (protocol_post, _) = protocol
                .state_key(&post.candidate)
                .map_err(|_| fault("assumption-witness-protocol-post-has-a-key"))?;
            let protocol_transition = edge
                .protocol
                .as_ref()
                .map(|transition| protocol.transition_key(transition))
                .transpose()
                .map_err(|_| fault("assumption-witness-protocol-transition-has-a-key"))?;
            steps.push(qsl_replay::PairedAssumptionStep {
                concrete: edge.concrete,
                concrete_post,
                protocol: protocol_transition,
                protocol_post,
            });
        }
        Ok(qsl_replay::PairedAssumptionPath {
            concrete_initial,
            protocol_initial,
            steps,
            loop_entry,
        })
    }

    fn paired_assumption_prefix_to(
        &mut self,
        target_product: usize,
        protocol: &AbstractProtocolSystem<'_>,
    ) -> Result<qsl_replay::PairedAssumptionPath, End> {
        let (candidates, initial) = self.candidate_graph()?;
        let target = candidates
            .iter()
            .position(|candidate| candidate.product == target_product)
            .ok_or_else(|| fault("undetermined-assumption-state-has-a-protocol-candidate"))?;
        let mut parent = BTreeMap::<usize, (usize, usize)>::new();
        let mut roots = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        for root in initial {
            roots.insert(root, root);
            seen.insert(root);
            queue.push_back(root);
        }
        while let Some(from) = queue.pop_front() {
            if from == target {
                break;
            }
            for (edge_index, edge) in candidates[from].edges.iter().enumerate() {
                if seen.insert(edge.target) {
                    roots.insert(edge.target, roots[&from]);
                    parent.insert(edge.target, (from, edge_index));
                    queue.push_back(edge.target);
                }
            }
        }
        let root = *roots
            .get(&target)
            .ok_or_else(|| fault("undetermined-assumption-state-is-reachable"))?;
        let mut edges = Vec::new();
        let mut cursor = target;
        while cursor != root {
            let (from, edge_index) = *parent
                .get(&cursor)
                .ok_or_else(|| fault("undetermined-assumption-prefix-has-a-parent"))?;
            edges.push((from, candidates[from].edges[edge_index].clone()));
            cursor = from;
        }
        edges.reverse();
        self.paired_assumption_path(&candidates, root, edges, None, protocol)
    }

    fn protocol_divergence_counterexample(
        &self,
        start: usize,
        class: String,
    ) -> Result<Option<TemporalCounterexample>, End> {
        let Some(edge) = self.nodes[start]
            .edges
            .iter()
            .position(|edge| edge.taken == Taken::Stutter && edge.post == widen(start))
        else {
            return Ok(None);
        };
        self.protocol_divergence_counterexample_with_loop(start, class, vec![(start, edge)])
    }

    fn protocol_divergence_counterexample_with_loop(
        &self,
        start: usize,
        class: String,
        cycle: Vec<(usize, usize)>,
    ) -> Result<Option<TemporalCounterexample>, End> {
        let mut prefix = Vec::new();
        let mut cursor = start;
        while let Some((parent, step)) = &self.nodes[cursor].parent {
            prefix.push(step.clone());
            cursor = *parent;
        }
        prefix.reverse();
        let initial_state = self.nodes[cursor].initial_index;
        let entry = prefix.len();
        let mut terminal_stutter = false;
        for (from, edge_index) in cycle {
            let edge = self.nodes[from].edges.get(edge_index).ok_or_else(|| {
                End::Fault(InternalFault::new(
                    "refinement-check",
                    "protocol-cycle-edge",
                ))
            })?;
            let Some(transition) = edge.transition.clone() else {
                terminal_stutter = true;
                continue;
            };
            let target = usize::try_from(edge.post).map_err(|_| {
                End::Fault(InternalFault::new(
                    "refinement-check",
                    "protocol-cycle-target",
                ))
            })?;
            let (post_state, _) =
                Self::encoding(self.concrete.state_key(&self.nodes[target].state))?;
            prefix.push(ModelStep {
                transition,
                post_state: qsl_eval::simulation::StateKey::from_canonical_bytes(post_state),
                delay: None,
            });
        }
        Ok(Some(TemporalCounterexample {
            initial_state,
            prefix,
            over_binding: None,
            fairness: self
                .refinement
                .concrete_operation_fairness()
                .cloned()
                .collect(),
            kind: CounterexampleKind::Formula,
            lasso: Some(Lasso {
                entry,
                terminal_stutter,
            }),
            refinement: Some(RefinementFailure::Divergence {
                constraint: None,
                protocol_class: Some(class),
            }),
            blocked: Vec::new(),
            final_delay: None,
        }))
    }

    fn stats(&self) -> RunStats {
        RunStats {
            product_states: widen(self.nodes.len()),
            product_edges: widen(self.edges),
            automaton_states: 0,
            depth: widen(self.nodes.iter().map(|node| node.depth).max().unwrap_or(0)),
            bound_limited: 0,
            instance_limited: 0,
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
            Some(ExpansionLimit::EvaluationMeter(counter)) => Self::stopped(
                ModelCheckLimit::EvaluationMeter { counter },
                qsl_replay::meter_bound(&self.meter, counter),
            ),
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
            let verdict = match self.abstract_protocol {
                Some(protocol) => {
                    check_protocol_initial(
                        self.refinement,
                        self.concrete,
                        self.abstract_system,
                        protocol,
                        index,
                        state.state(),
                    )
                    .verdict
                }
                None => check_initial(
                    self.refinement,
                    self.concrete,
                    self.abstract_system,
                    index,
                    state.state(),
                ),
            };
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
        if let Some((transition, clause)) = expansion.findings.iter().find_map(|finding| {
            let ModelFinding::ContractUndetermined {
                transition, clause, ..
            } = finding
            else {
                return None;
            };
            self.refinement
                .concrete_operation_fairness()
                .find(|assumption| {
                    let operation = assumption.operation();
                    self.concrete
                        .operation_text(operation.declaring_type(), operation.name())
                        == Some(transition.operation())
                })
                .map(|_| (transition.clone(), clause.clone()))
        }) {
            if let Some(protocol) = self.abstract_protocol {
                let Some(assumption) =
                    self.refinement
                        .concrete_operation_fairness()
                        .find(|assumption| {
                            let operation = assumption.operation();
                            self.concrete
                                .operation_text(operation.declaring_type(), operation.name())
                                == Some(transition.operation())
                        })
                else {
                    return Err(End::Fault(InternalFault::new(
                        "refinement-check",
                        "undetermined-assumption-has-a-checked-class",
                    )));
                };
                let implementation = qsl_replay::AssumptionIdentity {
                    side: qsl_replay::AssumptionSide::Implementation,
                    origin: qsl_replay::AssumptionOrigin::Authored,
                    kind: assumption.kind(),
                    granularity: assumption.granularity(),
                    target: qsl_replay::AssumptionTarget::Model(assumption.clone()),
                    selected: (assumption.granularity()
                        == quire_semantic_value::fairness::FairnessGranularity::Each)
                        .then(|| qsl_replay::AssumptionSelectedMember::Model(transition.clone())),
                };
                let mut specification = Vec::new();
                for source in protocol
                    .assumption_fairness()
                    .map_err(|_| fault("the-admitted-protocol-fairness-set-builds"))?
                {
                    let assumption = source.fairness;
                    let origin = match assumption.origin() {
                        qsl_eval::simulation::protocol::FairnessOrigin::Authored => {
                            qsl_replay::AssumptionOrigin::Authored
                        }
                        qsl_eval::simulation::protocol::FairnessOrigin::Scheduler => {
                            qsl_replay::AssumptionOrigin::Scheduler
                        }
                        qsl_eval::simulation::protocol::FairnessOrigin::Memory => {
                            qsl_replay::AssumptionOrigin::Memory
                        }
                    };
                    let selected_classes =
                        if assumption.granularity()
                            == quire_semantic_value::fairness::FairnessGranularity::Whole
                        {
                            vec![None]
                        } else {
                            let mut classes = BTreeSet::new();
                            for candidate in &self.nodes[at].position.abstract_protocol_states {
                                let successors = protocol.successors(candidate).map_err(|_| {
                                    fault("undetermined-protocol-assumption-candidates-expand")
                                })?;
                                classes.extend(successors.successors.into_iter().filter_map(
                                    |(transition, _)| assumption.class_of(&transition),
                                ));
                            }
                            classes
                                .into_iter()
                                .map(|class| {
                                    Some(qsl_replay::AssumptionSelectedMember::Protocol(class))
                                })
                                .collect()
                        };
                    for selected in selected_classes {
                        let target = source.required_temporal.as_ref().map_or_else(
                            || qsl_replay::AssumptionTarget::Protocol(assumption.clone()),
                            |required| qsl_replay::AssumptionTarget::RequiredTemporal {
                                clause: required.clause.clone(),
                                constraint: required.constraint.clone(),
                                fairness: assumption.clone(),
                            },
                        );
                        specification.push(qsl_replay::AssumptionIdentity {
                            side: qsl_replay::AssumptionSide::Specification,
                            origin,
                            kind: assumption.kind(),
                            granularity: assumption.granularity(),
                            target,
                            selected,
                        });
                    }
                }
                let path = self.paired_assumption_prefix_to(at, protocol)?;
                return Err(End::Undecided(
                    SafetyUndecided::ProtocolAssumptionNotEstablished(Box::new(
                        qsl_replay::ProtocolAssumptionWitness {
                            reason: qsl_replay::ProtocolAssumptionReason::RelationUndetermined,
                            identities: std::iter::once(implementation)
                                .chain(specification)
                                .collect(),
                            path,
                            undecided_locus: Some(format!("ContractUndetermined:{clause}")),
                        },
                    )),
                ));
            }
        }
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
            let (key, state_key) = Self::encoding(self.concrete.state_key(&state))?;
            edges.push((order, key, state_key, Some(transition), state));
        }
        edges.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        if edges.is_empty() {
            // ADR-018 SM-4: a terminal state repeats forever, one stutter
            // step that is no model step.
            let state = self.nodes[at].state.clone();
            let (key, state_key) = Self::encoding(self.concrete.state_key(&state))?;
            edges.push((Vec::new(), key, state_key, None, state));
        }
        let mut listed = Vec::with_capacity(edges.len());
        for (_, key, _state_key, transition, state) in edges {
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
            } = match self.abstract_protocol {
                Some(protocol) => check_protocol_step(
                    self.refinement,
                    self.concrete,
                    self.abstract_system,
                    protocol,
                    &self.nodes[at].position,
                    &ConcreteStep {
                        position: widen(depth.saturating_add(1)),
                        transition: transition.as_ref(),
                        post: &state,
                    },
                ),
                None => check_step(
                    self.refinement,
                    self.concrete,
                    self.abstract_system,
                    &self.nodes[at].position,
                    &ConcreteStep {
                        position: widen(depth.saturating_add(1)),
                        transition: transition.as_ref(),
                        post: &state,
                    },
                ),
            };
            self.transitions = self
                .transitions
                .saturating_add(usize::try_from(abstract_successors).unwrap_or(usize::MAX));
            let step = transition.clone().map(|transition| ModelStep {
                transition,
                post_state: qsl_eval::simulation::StateKey::from_canonical_bytes(key.clone()),
                delay: None,
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
        if let Some(protocol) = self.abstract_protocol {
            let relation = check_protocol_state_relations(
                self.refinement,
                self.concrete,
                protocol,
                &self.nodes[at].position,
                &self.nodes[at].state,
                &self.nodes[at].edges,
                widen(self.nodes[at].depth),
            );
            let relation = match relation {
                Ok(relation) => relation,
                Err(ProtocolRelationError::Expansion(stop)) => {
                    return Err(self.expansion_stop(stop));
                }
                Err(ProtocolRelationError::Fault(fault)) => return Err(End::Fault(fault)),
            };
            let expanded = match &relation {
                ProtocolStateRelation::Decided { expanded, .. }
                | ProtocolStateRelation::Undetermined { expanded } => *expanded,
            };
            self.transitions = self
                .transitions
                .saturating_add(usize::try_from(expanded).unwrap_or(usize::MAX));
            if self.transitions > self.limits.limits.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                ));
            }
            match relation {
                ProtocolStateRelation::Decided {
                    failure: Some(failure),
                    ..
                } => {
                    return Err(End::Violated(Box::new(Failure {
                        from: Some(at),
                        edge: None,
                        initial_index: self.nodes[at].initial_index,
                        failure,
                        kind: CounterexampleKind::Formula,
                    })));
                }
                ProtocolStateRelation::Undetermined { .. } => {
                    return Err(End::Undecided(SafetyUndecided::Undetermined(
                        Undetermined::UndecidedSuccessor,
                    )));
                }
                ProtocolStateRelation::Decided { failure: None, .. } => {}
            }
        }
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
        let mut candidates = if let Some(protocol) = self.abstract_protocol {
            let mut candidates = Vec::with_capacity(node.position.abstract_protocol_states.len());
            for candidate in &node.position.abstract_protocol_states {
                candidates.push(Self::encoding(protocol.state_key(candidate))?.0);
            }
            candidates
        } else {
            let mut candidates = Vec::with_capacity(node.position.abstract_states.len());
            for candidate in &node.position.abstract_states {
                candidates.push(Self::encoding(self.abstract_system.state_key_of(candidate))?.0);
            }
            candidates
        };
        candidates.sort();
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
    /// as the state keys of its states, each initial state's position and each
    /// state's edges.
    fn certificate(&self) -> Result<SimulationCertificate, InternalFault> {
        let encodes = |_| InternalFault::new("refinement-check", "state-encodes");
        let mut positions = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let concrete_bytes = self
                .concrete
                .state_key_of(&node.position.concrete)
                .map_err(encodes)?
                .0;
            let concrete = qsl_eval::simulation::StateKey::from_canonical_bytes(concrete_bytes);
            let abstract_states = node
                .position
                .abstract_states
                .iter()
                .map(|state| {
                    self.abstract_system
                        .state_key_of(state)
                        .map(|(bytes, _)| {
                            qsl_eval::simulation::StateKey::from_canonical_bytes(bytes)
                        })
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
            final_delay: None,
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
        let abstract_fairness: Vec<_> = self
            .refinement
            .abstract_operation_fairness()
            .cloned()
            .collect();
        let classes = (self.classes_of)(self.abstract_system, &abstract_fairness, &identities)
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
        let concrete_fairness: Vec<_> = self
            .refinement
            .concrete_operation_fairness()
            .cloned()
            .collect();
        let constraints =
            fairness_constraints(self.concrete, &concrete_fairness, &transitions, identity)?;
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
                    let (post_state, _) =
                        Self::encoding(self.concrete.state_key(&self.nodes[target].state))?;
                    prefix.push(ModelStep {
                        transition,
                        post_state: qsl_eval::simulation::StateKey::from_canonical_bytes(
                            post_state,
                        ),
                        delay: None,
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
            .abstract_operation_fairness()
            .nth(class.constraint())
            .ok_or_else(|| fault("a-class-names-a-constraint-of-the-set"))?
            .clone();
        let failure = if taking {
            RefinementFailure::AbstractUnfair { constraint }
        } else {
            RefinementFailure::Divergence {
                constraint: Some(constraint),
                protocol_class: None,
            }
        };
        Ok(ModelCheckOutcome::Violated {
            counterexample: Box::new(TemporalCounterexample {
                initial_state,
                prefix,
                over_binding: None,
                fairness: self
                    .refinement
                    .concrete_operation_fairness()
                    .cloned()
                    .collect(),
                kind: CounterexampleKind::Formula,
                lasso: Some(Lasso {
                    entry: entry_position,
                    terminal_stutter: stutters > 0,
                }),
                refinement: Some(failure),
                blocked: Vec::new(),
                final_delay: None,
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
            abstract_protocol_binding: None,
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
