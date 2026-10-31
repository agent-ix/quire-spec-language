// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-170's HP-5 evidence arms, routed through FR-181's product semantics.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::fairness::{analyze, Constraint, EdgeLabel, Graph, Identity, Strength, Vertex};
use qsl_eval::hyper::{
    BodySense, CheckedHyperLetters, ComponentStep, HyperFinding, HyperInputs, HyperLetters,
    HyperProduct, HyperState, HyperTransition, ParameterBinding, Property,
};
use qsl_eval::model_subject::AtomStop;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::StateKey;
use qsl_eval::simulation::{ExpansionLimit, TransitionSystem};
use qsl_eval::temporal::{
    evaluate_tuple, AtomReading, ComponentPosition, ComponentTrace, LetterStop, TemporalOutcome,
    TemporalPosition, TupleLetters,
};
use qsl_semantics::check::{CheckedHyperClause, FormulaNode, HyperForm, PredicateRole};
use quire_exact::{Cancel, LimitKind, Meter, ScalarLimits};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::FairnessGranularity;
use quire_semantic_value::model_transition::ModelTransition;

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::Lasso;
use crate::model_check::graph_replay::{inconclusive, replay_path, result, Position};
use crate::model_check::graph_settle::{GraphReplayResult, GraphReplaySettlement};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::model_check::replay::{
    close_loop, fault, refused, unfair_constraint, LoopState, ModelTraceRefusal,
    Position as ReplayPosition,
};
use crate::model_check::state_graph::{
    GraphEvidence, ModelLasso, ModelPath, TrapClosure, WitnessPath,
};

/// Replay HP-5's lasso witnesses or product trap using the body automaton and
/// fairness filter that FR-181 assigns to those evidence arms.
#[allow(clippy::too_many_arguments)]
pub(super) fn replay_evidence(
    system: &ModelSystem<'_>,
    package: &qsl_package::CheckedPackage,
    clause: &CheckedHyperClause,
    evidence: &GraphEvidence,
    max_states: usize,
    max_transitions: usize,
    max_automaton_states: u64,
    max_candidates: u64,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<GraphReplayResult, ReplayRefusal> {
    if clause.form() != Some(&HyperForm::SingleExistential) {
        return Err(refused(ModelTraceRefusal::HyperShape {
            reason: "names a hyper clause that is not HP-5",
        }));
    }
    let Some(quantifier) = clause.quantifiers().first() else {
        return Err(fault("an-HP-5-clause-has-one-quantifier"));
    };
    let over = match evidence {
        GraphEvidence::Witness { over, .. }
        | GraphEvidence::Trap { over, .. }
        | GraphEvidence::PathPair { over, .. }
        | GraphEvidence::Undefined { over, .. } => over.as_ref(),
    };
    let bindings = hyper_bindings(system, clause, over)?;
    let letters = CheckedHyperLetters::new(package, clause, vec![system])
        .map_err(|_| fault("the-HP-5-letters-match-the-subject"))?;
    match evidence {
        GraphEvidence::Witness { paths, .. } => {
            let initial_count = system.initial().len();
            if paths.len() != initial_count {
                return Err(refused(ModelTraceRefusal::InitialState {
                    index: paths.len(),
                    count: initial_count,
                }));
            }
            for (index, (path, _source)) in paths.iter().enumerate() {
                let WitnessPath::Lasso(lasso) = path else {
                    return Err(refused(ModelTraceRefusal::HyperShape {
                        reason: "an HP-5 witness is not a lasso",
                    }));
                };
                if path_initial(path) != index {
                    return Err(refused(ModelTraceRefusal::InitialState {
                        index: path_initial(path),
                        count: initial_count,
                    }));
                }
                match replay_lasso(
                    system,
                    clause,
                    &letters,
                    &bindings,
                    lasso,
                    &quantifier.variable,
                    accounting,
                    cancel,
                )? {
                    Some(true) => {}
                    Some(false) | None => {
                        return Ok(result(
                            inconclusive(Category::Success, Category::Violation),
                            Vec::new(),
                            None,
                            None,
                        ));
                    }
                }
            }
            Ok(result(
                GraphReplaySettlement::Reproduced,
                Vec::new(),
                None,
                None,
            ))
        }
        GraphEvidence::Trap {
            stem,
            closure: TrapClosure::Product,
            ..
        } => {
            if !stem.steps.is_empty() {
                return Err(refused(ModelTraceRefusal::HyperShape {
                    reason: "an HP-5 product trap has a non-empty stem",
                }));
            }
            replay_product_trap(
                system,
                clause,
                &letters,
                &bindings,
                stem,
                max_states,
                max_transitions,
                max_automaton_states,
                max_candidates,
                accounting,
                cancel,
            )
        }
        GraphEvidence::Trap { .. } => Err(refused(ModelTraceRefusal::HyperShape {
            reason: "an HP-5 trap does not name product closure",
        })),
        GraphEvidence::Undefined {
            stem, undefined, ..
        } => {
            let positions = replay_path(system, stem)?;
            let reproduced = replay_undefined(
                clause, &letters, &bindings, &positions, undefined, accounting, cancel,
            )?;
            Ok(if reproduced {
                result(GraphReplaySettlement::Reproduced, Vec::new(), None, None)
            } else {
                result(
                    inconclusive(Category::Violation, Category::Success),
                    Vec::new(),
                    None,
                    None,
                )
            })
        }
        GraphEvidence::PathPair { .. } => Err(refused(ModelTraceRefusal::HyperShape {
            reason: "an HP-5 path-pair arm is not admitted by FR-181",
        })),
    }
}

fn hyper_bindings(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    over: Option<&qsl_eval::model_subject::ModelInstance>,
) -> Result<Vec<ParameterBinding>, ReplayRefusal> {
    match (clause.parameters(), over) {
        ([], None) => Ok(Vec::new()),
        ([parameter], Some(instance)) => {
            let value_type = quire_exact::ValueType::Reference(parameter.population.object());
            let admitted = system
                .instances(&value_type)
                .map_err(|_| refused(ModelTraceRefusal::OverBinding))?;
            if !admitted.iter().any(|candidate| {
                candidate.population == instance.population && candidate.key == instance.key
            }) {
                return Err(refused(ModelTraceRefusal::OverBinding));
            }
            Ok(vec![ParameterBinding {
                parameter: parameter.name.clone(),
                instance: instance.clone(),
            }])
        }
        _ => Err(refused(ModelTraceRefusal::OverBinding)),
    }
}

fn path_initial(path: &WitnessPath) -> usize {
    match path {
        WitnessPath::Path(path) => path.initial as usize,
        WitnessPath::Lasso(lasso) => lasso.path.initial as usize,
    }
}

struct OneTraceLetters<'a> {
    letters: &'a dyn qsl_eval::hyper::HyperLetters,
    instance: &'a [ParameterBinding],
    positions: &'a [Position],
}

impl TupleLetters for OneTraceLetters<'_> {
    fn atom(
        &mut self,
        atom: qsl_semantics::check::FormulaNodeId,
        _joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let [component] = components else {
            return Err(LetterStop::Fault(
                quire_semantic_value::catalog::InternalFault::new(
                    "replay-model-graph-hyper",
                    "an-HP-5-letter-has-one-component",
                ),
            ));
        };
        let index = usize::try_from(component.position.index()).map_err(|_| {
            LetterStop::Fault(quire_semantic_value::catalog::InternalFault::new(
                "replay-model-graph-hyper",
                "an-HP-5-position-fits-usize",
            ))
        })?;
        let state = self
            .positions
            .get(index)
            .map(|position| &position.0)
            .ok_or_else(|| {
                LetterStop::Fault(quire_semantic_value::catalog::InternalFault::new(
                    "replay-model-graph-hyper",
                    "an-HP-5-position-names-a-state",
                ))
            })?;
        self.letters
            .atom(atom, self.instance, &[state], meter)
            .map_err(LetterStop::from)
    }
}

#[allow(clippy::too_many_arguments)]
fn replay_lasso(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    letters: &CheckedHyperLetters<'_, '_>,
    bindings: &[ParameterBinding],
    lasso: &ModelLasso,
    variable: &str,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<Option<bool>, ReplayRefusal> {
    let positions = replay_path(system, &lasso.path)?;
    let entry =
        usize::try_from(lasso.loop_entry).map_err(|_| fault("HP-5-loop-entry-fits-usize"))?;
    let steps = lasso.path.steps.len();
    let terminal_stutter = entry == steps;
    let trace_lasso = Lasso {
        entry,
        terminal_stutter,
    };
    let replay_positions: Vec<ReplayPosition> = positions
        .iter()
        .map(|(state, state_key)| ReplayPosition {
            state: state.clone(),
            state_key: state_key.clone(),
        })
        .collect();
    close_loop(system, &replay_positions, trace_lasso)?;
    let mut loop_states = Vec::new();
    if !terminal_stutter {
        for index in entry..steps {
            let state = &positions
                .get(index)
                .ok_or_else(|| fault("HP-5-loop-position-exists"))?
                .0;
            let expansion = match system.successors(state) {
                Ok(expansion) => expansion,
                Err(_) => return Ok(None),
            };
            let step = lasso
                .path
                .steps
                .get(index)
                .ok_or_else(|| fault("HP-5-loop-step-exists"))?;
            loop_states.push(LoopState {
                position: index - entry,
                enabled: expansion
                    .successors
                    .into_iter()
                    .map(|(transition, _)| transition)
                    .collect(),
                taken: step.transition.clone(),
            });
        }
    }
    if unfair_constraint(
        system,
        &loop_states,
        trace_lasso,
        &clause.quantifiers()[0].fairness,
    )?
    .is_some()
    {
        return Err(refused(ModelTraceRefusal::HyperUnfair {
            variable: variable.to_owned(),
        }));
    }
    let component = if terminal_stutter {
        ComponentTrace::Terminal {
            len: u64::try_from(positions.len()).map_err(|_| fault("HP-5-trace-length-fits-u64"))?,
        }
    } else {
        ComponentTrace::Lasso {
            prefix: u64::try_from(entry).map_err(|_| fault("HP-5-loop-entry-fits-u64"))?,
            loop_len: u64::try_from(steps - entry)
                .map_err(|_| fault("HP-5-loop-length-fits-u64"))?,
        }
    };
    let positions = replay_positions_as_positions(&replay_positions);
    let mut tuple_letters = OneTraceLetters {
        letters,
        instance: bindings,
        positions: &positions,
    };
    let mut meter = Meter::new(accounting).with_cancel(cancel.clone());
    match evaluate_tuple(clause, &[component], &mut tuple_letters, &mut meter)
        .map_err(|_| fault("the-HP-5-body-evaluates-on-its-lasso"))?
    {
        TemporalOutcome::Holds => Ok(Some(true)),
        TemporalOutcome::Violated { .. } => Ok(Some(false)),
        TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => Ok(None),
        TemporalOutcome::Undefined(_) => Ok(Some(false)),
        _ => Err(fault("the-HP-5-infinite-body-has-a-lasso-outcome")),
    }
}

fn replay_positions_as_positions(positions: &[ReplayPosition]) -> Vec<Position> {
    positions
        .iter()
        .map(|position| (position.state.clone(), position.state_key.clone()))
        .collect()
}

fn replay_undefined(
    clause: &CheckedHyperClause,
    letters: &CheckedHyperLetters<'_, '_>,
    bindings: &[ParameterBinding],
    positions: &[Position],
    undefined: &crate::model_check::state_graph::GraphUndefined,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<bool, ReplayRefusal> {
    if undefined.predicate != Some(PredicateRole::Target) || undefined.transition.is_some() {
        return Ok(false);
    }
    let Some(expected_locus) = undefined.locus.as_ref() else {
        return Ok(false);
    };
    let Some((_, state_key)) = positions.last() else {
        return Err(fault("an-HP-5-undefined-path-has-an-initial-state"));
    };
    if *state_key != undefined.state_key {
        return Ok(false);
    }
    let mut meter = Meter::new(accounting).with_cancel(cancel.clone());
    for (position_index, (state, _)) in positions.iter().enumerate() {
        for (index, node) in clause.body().nodes().iter().enumerate() {
            if !matches!(node, FormulaNode::Holds { .. }) {
                continue;
            }
            let atom = clause
                .body()
                .node_id(index)
                .ok_or_else(|| fault("a-formula-node-has-an-id"))?;
            let Some((condition, _)) = clause.condition(atom) else {
                return Err(fault("an-HP-5-holds-node-has-a-condition"));
            };
            let reading = match letters.atom(atom, bindings, &[state], &mut meter) {
                Ok(reading) => reading,
                Err(AtomStop::Fault(fault)) => return Err(ReplayRefusal::Fault(fault)),
                Err(AtomStop::Incomplete(_) | AtomStop::Refused(_) | AtomStop::Unobservable(_)) => {
                    return Ok(false);
                }
            };
            if let AtomReading::Undefined(cause) = reading {
                return Ok(position_index + 1 == positions.len()
                    && cause == undefined.cause
                    && condition.location() == expected_locus);
            }
        }
    }
    Ok(false)
}

#[derive(Clone)]
struct ProductNode {
    state: HyperState,
    enabled: Vec<ModelTransition>,
    edges: Vec<(HyperTransition, usize)>,
}

#[allow(clippy::too_many_arguments)]
fn replay_product_trap(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    letters: &CheckedHyperLetters<'_, '_>,
    bindings: &[ParameterBinding],
    stem: &ModelPath,
    max_states: usize,
    max_transitions: usize,
    max_automaton_states: u64,
    max_candidates: u64,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<GraphReplayResult, ReplayRefusal> {
    let built = HyperProduct::new(HyperInputs {
        systems: vec![system],
        instance: bindings,
        body: clause.body(),
        profile: clause.profile(),
        sense: BodySense::Itself,
        letters,
        matcher: None,
        max_automaton_states,
        meter: accounting,
        cancel,
        copy_swap: None,
    });
    let mut product = match built {
        Ok(product) => product,
        Err(qsl_eval::automaton::AutomatonError::StateLimit { limit }) => {
            return Ok(stopped_result(ModelCheckLimit::MaxAutomatonStates, limit));
        }
        Err(_) => return Err(fault("the-HP-5-body-translates")),
    };
    if product.start().is_err() {
        return product_halt_result(&product, max_candidates, accounting);
    }
    let initial = system
        .initial()
        .get(stem.initial as usize)
        .cloned()
        .ok_or_else(|| {
            refused(ModelTraceRefusal::InitialState {
                index: stem.initial as usize,
                count: system.initial().len(),
            })
        })?;
    if !stem.steps.is_empty() {
        return Err(refused(ModelTraceRefusal::HyperShape {
            reason: "an HP-5 product trap names an initial state with an empty stem",
        }));
    }
    let wanted = system
        .state_key(&initial)
        .map_err(|_| fault("HP-5-initial-state-key"))?
        .1;
    let roots: Vec<HyperState> = product
        .initial()
        .into_iter()
        .filter(|state| {
            state.components.first().is_some_and(|component| {
                system
                    .state_key(component)
                    .is_ok_and(|(_, state_key)| state_key == wanted)
            })
        })
        .collect();
    if roots.is_empty() {
        return Err(fault("the-HP-5-product-has-an-initial-root"));
    }

    let mut nodes = Vec::<ProductNode>::new();
    let mut by_key = BTreeMap::<(StateKey, Option<usize>), usize>::new();
    let mut queue = VecDeque::<usize>::new();
    for root in roots {
        let key = product_key(system, &root)?;
        if by_key.contains_key(&key) {
            continue;
        }
        if nodes.len() >= max_states {
            return Ok(stopped_result(
                ModelCheckLimit::MaxStates,
                max_states as u64,
            ));
        }
        let id = nodes.len();
        by_key.insert(key, id);
        nodes.push(ProductNode {
            state: root,
            enabled: Vec::new(),
            edges: Vec::new(),
        });
        queue.push_back(id);
    }
    let mut transitions = 0usize;
    while let Some(id) = queue.pop_front() {
        if let Some(cause) = cancel.cause() {
            return Err(ReplayRefusal::Cancelled(cause));
        }
        let state = nodes
            .get(id)
            .map(|node| node.state.clone())
            .ok_or_else(|| fault("a-product-queue-entry-is-a-node"))?;
        let expansion = match product.successors(&state) {
            Ok(expansion) => expansion,
            Err(_) => return product_halt_result(&product, max_candidates, accounting),
        };
        if state_has_open_finding(&expansion.findings)
            || matches!(state.property, Property::Undefined { .. })
        {
            return Ok(result(
                inconclusive(Category::Violation, Category::Success),
                Vec::new(),
                Some(nodes.len() as u64),
                None,
            ));
        }
        let enabled = expansion
            .findings
            .iter()
            .filter_map(|finding| match finding {
                HyperFinding::Enabled {
                    component: 0,
                    transitions,
                } => Some(transitions.as_slice()),
                _ => None,
            })
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        nodes[id].enabled = enabled;
        for (transition, next) in expansion.successors {
            transitions = transitions.saturating_add(1);
            if transitions > max_transitions {
                return Ok(stopped_result(
                    ModelCheckLimit::MaxTransitions,
                    max_transitions as u64,
                ));
            }
            if matches!(next.property, Property::Undefined { .. }) {
                return Ok(result(
                    inconclusive(Category::Violation, Category::Success),
                    Vec::new(),
                    Some(nodes.len() as u64),
                    None,
                ));
            }
            let key = product_key(system, &next)?;
            let target = if let Some(target) = by_key.get(&key) {
                *target
            } else {
                if nodes.len() >= max_states {
                    return Ok(stopped_result(
                        ModelCheckLimit::MaxStates,
                        max_states as u64,
                    ));
                }
                let target = nodes.len();
                by_key.insert(key, target);
                nodes.push(ProductNode {
                    state: next,
                    enabled: Vec::new(),
                    edges: Vec::new(),
                });
                queue.push_back(target);
                target
            };
            nodes[id].edges.push((transition, target));
        }
    }
    let (fair, constraints, vertices) = fairness_graph(system, clause, &product, &nodes)?;
    let within: BTreeSet<Vertex> = vertices.iter().copied().collect();
    let analyses = analyze(&fair, &within, &constraints, cancel)
        .map_err(|_| fault("the-HP-5-fairness-filter-completes"))?;
    if analyses
        .iter()
        .any(|analysis| analysis.first_fair().is_some())
    {
        Ok(result(
            inconclusive(Category::Violation, Category::Success),
            Vec::new(),
            Some(nodes.len() as u64),
            None,
        ))
    } else {
        Ok(result(
            GraphReplaySettlement::Reproduced,
            Vec::new(),
            Some(nodes.len() as u64),
            None,
        ))
    }
}

fn product_key(
    system: &ModelSystem<'_>,
    state: &HyperState,
) -> Result<(StateKey, Option<usize>), ReplayRefusal> {
    let component = state
        .components
        .first()
        .ok_or_else(|| fault("an-HP-5-product-state-has-one-component"))?;
    let (bytes, _) = system
        .state_key(component)
        .map_err(|_| fault("HP-5-product-model-state-key"))?;
    let key = StateKey::from_canonical_bytes(bytes);
    let property = match &state.property {
        Property::After(state) => Some(state.index()),
        Property::Dead | Property::Undefined { .. } => None,
        Property::Witnesses(_) | Property::Rejected | Property::MatchUndefined { .. } => {
            return Err(fault("an-HP-5-product-has-an-ordinary-property-state"));
        }
    };
    Ok((key, property))
}

fn state_has_open_finding(findings: &[HyperFinding]) -> bool {
    findings
        .iter()
        .any(|finding| !matches!(finding, HyperFinding::Enabled { .. }))
}

fn fairness_graph(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    product: &HyperProduct<'_, '_>,
    nodes: &[ProductNode],
) -> Result<(Graph, Vec<Constraint>, Vec<Vertex>), ReplayRefusal> {
    let transitions: BTreeSet<ModelTransition> = nodes
        .iter()
        .flat_map(|node| {
            node.enabled
                .iter()
                .cloned()
                .chain(node.edges.iter().filter_map(
                    |(transition, _)| match transition.0.as_slice() {
                        [ComponentStep::Model(transition)] => Some(transition.clone()),
                        _ => None,
                    },
                ))
        })
        .collect();
    let identities: BTreeMap<ModelTransition, Identity> = transitions
        .iter()
        .enumerate()
        .map(|(index, transition)| {
            (
                transition.clone(),
                Identity::new(u64::try_from(index).unwrap_or(u64::MAX)),
            )
        })
        .collect();
    let mut graph = Graph::new();
    let mut vertices = Vec::with_capacity(nodes.len());
    for node in nodes {
        let enabled = node
            .enabled
            .iter()
            .filter_map(|transition| identities.get(transition).copied())
            .collect::<Vec<_>>();
        vertices.push(graph.add_vertex(enabled));
    }
    for (source, node) in nodes.iter().enumerate() {
        let source = *vertices
            .get(source)
            .ok_or_else(|| fault("HP-5-node-has-a-fairness-vertex"))?;
        for (transition, target) in &node.edges {
            let label = match transition.0.as_slice() {
                [ComponentStep::Model(model)] => EdgeLabel::Transition(
                    *identities
                        .get(model)
                        .ok_or_else(|| fault("HP-5-edge-transition-has-an-identity"))?,
                ),
                [ComponentStep::Stutter] => EdgeLabel::Stutter,
                _ => return Err(fault("HP-5-transition-has-one-component")),
            };
            let target = *vertices
                .get(*target)
                .ok_or_else(|| fault("HP-5-edge-target-has-a-fairness-vertex"))?;
            graph
                .add_edge(source, target, label)
                .map_err(|_| fault("HP-5-fairness-edge-joins-known-vertices"))?;
        }
    }
    let automaton = product.automaton();
    if automaton.acceptance_set_count() == 0 {
        graph
            .add_acceptance_set(nodes.iter().enumerate().filter_map(|(index, node)| {
                matches!(node.state.property, Property::After(_))
                    .then(|| vertices.get(index).copied())
                    .flatten()
            }))
            .map_err(|_| fault("HP-5-live-acceptance-members-are-vertices"))?;
    } else {
        for acceptance in 0..automaton.acceptance_set_count() {
            graph
                .add_acceptance_set(nodes.iter().enumerate().filter_map(|(index, node)| {
                    let Property::After(state) = &node.state.property else {
                        return None;
                    };
                    automaton
                        .is_accepting_in(acceptance, *state)
                        .then(|| vertices.get(index).copied())
                        .flatten()
                }))
                .map_err(|_| fault("HP-5-acceptance-members-are-vertices"))?;
        }
    }
    let constraints = fairness_constraints(system, clause, &identities)?;
    Ok((graph, constraints, vertices))
}

fn fairness_constraints(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    identities: &BTreeMap<ModelTransition, Identity>,
) -> Result<Vec<Constraint>, ReplayRefusal> {
    let fairness = clause
        .quantifiers()
        .first()
        .map_or(&[][..], |quantifier| quantifier.fairness.as_slice());
    let mut constraints = Vec::new();
    for constraint in fairness {
        let operation = constraint.operation();
        let text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or_else(|| {
                refused(ModelTraceRefusal::UnknownFairnessOperation {
                    name: operation.name().to_owned(),
                })
            })?;
        let covered: Vec<Identity> = identities
            .iter()
            .filter_map(|(transition, identity)| {
                (transition.operation() == text).then_some(*identity)
            })
            .collect();
        let strength = Strength::from(constraint.kind());
        match constraint.granularity() {
            FairnessGranularity::Whole => constraints.push(Constraint::new(strength, covered)),
            FairnessGranularity::Each => {
                for identity in covered {
                    constraints.push(Constraint::new(strength, [identity]));
                }
            }
        }
    }
    Ok(constraints)
}

fn product_halt_result(
    product: &HyperProduct<'_, '_>,
    max_candidates: u64,
    accounting: ScalarLimits,
) -> Result<GraphReplayResult, ReplayRefusal> {
    use qsl_eval::hyper::Halt;
    let halt = product
        .take_halt()
        .ok_or_else(|| fault("a-stopped-HP-5-product-records-its-halt"))?;
    let reached = match halt {
        Halt::Automaton(value) => Some((ModelCheckLimit::MaxAutomatonStates, value)),
        Halt::Expansion { stop, .. } => stop.limit.map(|limit| match limit {
            ExpansionLimit::MaxCandidates => (ModelCheckLimit::MaxCandidates, max_candidates),
            ExpansionLimit::EvaluationMeter(counter) => (
                ModelCheckLimit::EvaluationMeter { counter },
                accounting_value(accounting, counter),
            ),
        }),
        Halt::Meter(counter) => Some((
            ModelCheckLimit::EvaluationMeter { counter },
            accounting_value(accounting, counter),
        )),
        Halt::Cancelled(cause) => return Err(ReplayRefusal::Cancelled(cause)),
        Halt::AutomatonKey(error) => {
            return Err(refused(ModelTraceRefusal::Automaton(error)));
        }
        Halt::Encoding(_) | Halt::Fault(_) | Halt::Atom(_) | Halt::WitnessSet(_) => {
            return Err(fault("the-HP-5-product-stop-is-recoverable"));
        }
    };
    match reached {
        Some((limit, value)) => Ok(stopped_result(limit, value)),
        None => Ok(result(
            inconclusive(Category::Violation, Category::Success),
            Vec::new(),
            None,
            None,
        )),
    }
}

fn accounting_value(limits: ScalarLimits, counter: LimitKind) -> u64 {
    match counter {
        LimitKind::IntegerBits => limits.integer_bits,
        LimitKind::DecimalDigits => limits.decimal_digits,
        LimitKind::ScaleExpansion => limits.scale_expansion,
        LimitKind::TextInputBytes => limits.text_input_bytes,
        LimitKind::TextScalars => limits.text_scalars,
        LimitKind::NormalizedScalars => limits.normalized_scalars,
        LimitKind::UnitEdges => limits.unit_edges,
        LimitKind::ValueOccurrences => limits.value_occurrences,
        LimitKind::WorkUnits => limits.work_units,
        LimitKind::ResultUnits => limits.result_units,
    }
}

fn stopped_result(limit: ModelCheckLimit, value: u64) -> GraphReplayResult {
    result(
        GraphReplaySettlement::Stopped(ReachedLimit { limit, value }),
        Vec::new(),
        None,
        None,
    )
}
