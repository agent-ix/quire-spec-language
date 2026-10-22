// SPDX-License-Identifier: AGPL-3.0-or-later
//! The breadth-first construction of the probabilistic product (FR-196
//! "Behavior"): the product, its state numbering and its edge order are
//! functions of the subject, the claim and the limits.

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::automaton::AutomatonError;
use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::{
    same_rewards, Action, ModelSimState, ModelSystem, NotMarkov, PredicateReading,
    ScheduledIdentity, Unsteppable,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::ModelCheckLimit;
use qsl_semantics::check::{CheckedPredicate, Workload};
use quire_exact::{Cancel, Identifier, Integer, ObjectReference, Rational, Value, ValueType};
use quire_semantic_value::catalog::InternalFault;

use super::plan::{Plan, PlanFault, Reads};
use super::{
    Accumulator, PathStep, Pending, ProbBuild, ProbEdge, ProbKey, ProbProduct, ProbStateId,
    ProductAction, ProductKind, ProductRefusal, ProductRequest, ProductStop, UndefinedPath,
    UndefinedRead,
};
use super::{FormulaState, MonitorState};

/// Build the product of `request`'s claim over its subject.
///
/// # Errors
///
/// [`ProductRefusal`] when the subject does not admit, a state does not
/// expand or step, the event has no monitor, or a budget or the cancellation
/// ends the build.
pub fn build_product(
    request: ProductRequest<'_>,
    cancel: &Cancel,
) -> Result<ProbBuild, ProductRefusal> {
    let ProductRequest {
        subject,
        claim,
        limits,
    } = request;
    let package = subject.simulation.package;
    let system = ModelSystem::new(subject.simulation).map_err(ProductRefusal::Admission)?;
    let (kind, workload) = match &claim.scheduler {
        qsl_semantics::check::ProbScheduler::Workload(name) => {
            let workload = package
                .graph()
                .probabilistic_model()
                .workload(name)
                .ok_or_else(|| fault("claim-workload-is-declared"))?;
            (ProductKind::Dtmc, Some(workload))
        }
        qsl_semantics::check::ProbScheduler::Every => (ProductKind::Mdp, None),
    };
    let (plan, reads) = Plan::of(claim, limits.max_automaton_states).map_err(plan_refusal)?;
    let bindings = match &claim.over {
        None => vec![(None, None)],
        Some(over) => {
            let ValueType::Reference(object) = &over.value_type else {
                return Err(fault("over-is-an-object-type"));
            };
            system
                .instances(&over.value_type)
                .map_err(ProductRefusal::NotSimulated)?
                .into_iter()
                .map(|instance| {
                    let reference = system
                        .bound_reference(*object, &instance)
                        .map_err(ProductRefusal::Internal)?;
                    Ok((Some(instance), Some(reference)))
                })
                .collect::<Result<Vec<_>, ProductRefusal>>()?
        }
    };
    let builder = Builder {
        system: &system,
        workload,
        kind,
        plan,
        reads,
        bindings,
        max_states: limits.limits.max_states,
        max_transitions: limits.limits.max_transitions,
        cancel,
        nodes: Vec::new(),
        index: BTreeMap::new(),
        readings: BTreeMap::new(),
        edges: Vec::new(),
        queue: VecDeque::new(),
    };
    builder.run()
}

fn fault(invariant: &'static str) -> ProductRefusal {
    ProductRefusal::Internal(InternalFault::new("exact-probabilistic", invariant))
}

fn plan_refusal(fault_: PlanFault) -> ProductRefusal {
    match fault_ {
        PlanFault::Automaton(error) => automaton_refusal(error),
        PlanFault::Broken(invariant) => fault(invariant),
    }
}

/// An automaton that reached its state limit is a budget; any other refusal
/// is the event's.
fn automaton_refusal(error: AutomatonError) -> ProductRefusal {
    match error {
        AutomatonError::StateLimit { limit } => ProductRefusal::Stopped(ProductStop::Limit {
            limit: ModelCheckLimit::MaxAutomatonStates,
            value: limit,
        }),
        other => ProductRefusal::Translation(other),
    }
}

/// Why the exploration ended before it was exhausted.
enum Halt {
    Refusal(ProductRefusal),
    Undefined(Box<UndefinedPath>),
}

impl From<ProductRefusal> for Halt {
    fn from(refusal: ProductRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<PlanFault> for Halt {
    fn from(fault: PlanFault) -> Self {
        Self::Refusal(plan_refusal(fault))
    }
}

type Step<T> = Result<T, Halt>;

/// What a draw with several post-states awaits: the post-states in canonical
/// order, and the step's rewards.
struct Resolution {
    successors: Vec<ModelSimState>,
    rewards: Vec<(Identifier, Value)>,
}

/// One discovered product state.
struct Node {
    key: ProbKey,
    model: ModelSimState,
    /// The truth of the claim's predicates at the model state.
    truths: Vec<bool>,
    /// The state it was first reached from, and the model step that reached
    /// it: none for a resolution, which is no model step.
    parent: Option<(usize, Option<PathStep>)>,
    /// Set at an intermediate state.
    resolution: Option<Resolution>,
}

/// A target of a model step, before it is a product state.
struct Target {
    key: ProbKey,
    model: ModelSimState,
    truths: Vec<bool>,
    /// Set for an intermediate state.
    resolution: Option<Resolution>,
}

/// An edge of the state being expanded, before its target is numbered.
struct Planned {
    choice: ProductAction,
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    target: Target,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
    /// The model step the edge takes.
    step: Option<PathStep>,
}

struct Builder<'r, 'c> {
    system: &'r ModelSystem<'r>,
    workload: Option<&'r Workload>,
    kind: ProductKind,
    plan: Plan<'c>,
    reads: Reads<'c>,
    /// Each `over` binding, with the reference its predicates are read for.
    bindings: Vec<(Option<ModelInstance>, Option<ObjectReference>)>,
    max_states: usize,
    max_transitions: usize,
    cancel: &'r Cancel,
    nodes: Vec<Node>,
    index: BTreeMap<ProbKey, usize>,
    /// The predicates' truths at a model state, by binding and state key.
    readings: BTreeMap<(usize, Vec<u8>), Vec<bool>>,
    edges: Vec<ProbEdge>,
    queue: VecDeque<usize>,
}

/// A predicate that has no value at a model state.
struct Hit {
    read: UndefinedRead,
    cause: UndefinedCause,
    state_key: DigestRecord,
}

impl Builder<'_, '_> {
    fn run(mut self) -> Result<ProbBuild, ProductRefusal> {
        match self.explore() {
            Ok(initial) => Ok(ProbBuild::Built(self.finish(initial))),
            Err(Halt::Refusal(refusal)) => Err(refusal),
            Err(Halt::Undefined(path)) => Ok(ProbBuild::Undefined(*path)),
        }
    }

    /// Number the initial states, then every state they reach, breadth
    /// first. Returns the initial states' handles.
    fn explore(&mut self) -> Step<Vec<(u32, Option<ModelInstance>, ProbStateId)>> {
        self.poll()?;
        let mut starts = Vec::new();
        for (position, state) in self.system.initial().into_iter().enumerate() {
            let at = u32::try_from(position).map_err(|_| fault("initial-states-count"))?;
            for binding in 0..self.bindings.len() {
                let truths = match self.truths(&state, binding)? {
                    Ok(truths) => truths,
                    Err(hit) => return Err(self.undefined(None, None, None, hit)),
                };
                let (monitor, accumulator) = self.plan.start(&truths)?;
                let key = self.key_of(&state, binding, monitor, accumulator, None)?;
                let target = Target {
                    key,
                    model: state.clone(),
                    truths,
                    resolution: None,
                };
                let id = self.intern(target, None, None)?;
                starts.push((at, self.bindings[binding].0.clone(), ProbStateId(id)));
            }
        }
        while let Some(at) = self.queue.pop_front() {
            self.poll()?;
            self.expand(at)?;
        }
        Ok(starts)
    }

    fn finish(self, initial: Vec<(u32, Option<ModelInstance>, ProbStateId)>) -> ProbProduct {
        let decided = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| match node.key.monitor {
                Some(MonitorState::Formula(FormulaState::Decided(verdict))) => {
                    Some((ProbStateId(index), verdict))
                }
                _ => None,
            })
            .collect();
        let labels = if matches!(self.plan, Plan::Labels) {
            self.nodes.iter().map(|node| node.truths.clone()).collect()
        } else {
            Vec::new()
        };
        ProbProduct {
            states: self.nodes.into_iter().map(|node| node.key).collect(),
            kind: self.kind,
            edges: self.edges,
            initial,
            decided,
            labels,
        }
    }

    fn poll(&self) -> Step<()> {
        if self.cancel.poll() {
            if let Some(cause) = self.cancel.cause() {
                return Err(ProductRefusal::Stopped(ProductStop::Cancelled(cause)).into());
            }
        }
        Ok(())
    }

    fn stopped(limit: ModelCheckLimit, value: usize) -> Halt {
        Halt::Refusal(ProductRefusal::Stopped(ProductStop::Limit {
            limit,
            value: u64::try_from(value).unwrap_or(u64::MAX),
        }))
    }

    /// The truth of the claim's predicates at `state` under binding
    /// `binding`, or the first that has no value.
    fn truths(&mut self, state: &ModelSimState, binding: usize) -> Step<Result<Vec<bool>, Hit>> {
        let (model, digest) = self
            .system
            .state_key(state)
            .map_err(|_| fault("state-key-encodes"))?;
        if let Some(known) = self.readings.get(&(binding, model.clone())) {
            return Ok(Ok(known.clone()));
        }
        let over = self.bindings[binding].1.as_ref();
        let predicates: Vec<(&CheckedPredicate, Option<&ObjectReference>)> = self
            .reads
            .iter()
            .map(|(_, predicate)| (*predicate, over))
            .collect();
        let readings = self
            .system
            .state_predicates(state, &predicates)
            .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop)))?;
        let mut truths = Vec::with_capacity(readings.len());
        for ((read, _), reading) in self.reads.iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Undefined { cause, .. } => {
                    return Ok(Err(Hit {
                        read: *read,
                        cause,
                        state_key: digest,
                    }));
                }
                PredicateReading::Refused(outcome) => {
                    return Err(ProductRefusal::PredicateRefused(outcome).into());
                }
            }
        }
        self.readings.insert((binding, model), truths.clone());
        Ok(Ok(truths))
    }

    fn key_of(
        &self,
        state: &ModelSimState,
        binding: usize,
        monitor: Option<MonitorState>,
        accumulator: Option<Accumulator>,
        pending: Option<Pending>,
    ) -> Step<ProbKey> {
        let (model, _) = self
            .system
            .state_key(state)
            .map_err(|_| fault("state-key-encodes"))?;
        Ok(ProbKey {
            binding: self.bindings[binding].0.clone(),
            model,
            monitor,
            accumulator,
            pending,
        })
    }

    /// The state of `target`, numbered at its first discovery. A state with
    /// decided control is never expanded.
    fn intern(
        &mut self,
        target: Target,
        parent: Option<usize>,
        step: Option<PathStep>,
    ) -> Step<usize> {
        if let Some(known) = self.index.get(&target.key) {
            return Ok(*known);
        }
        if self.nodes.len() >= self.max_states {
            return Err(Self::stopped(ModelCheckLimit::MaxStates, self.max_states));
        }
        let id = self.nodes.len();
        let decided = target.key.monitor.is_some_and(MonitorState::is_decided);
        self.index.insert(target.key.clone(), id);
        self.nodes.push(Node {
            key: target.key,
            model: target.model,
            truths: target.truths,
            parent: parent.map(|parent| (parent, step)),
            resolution: target.resolution,
        });
        if !decided {
            self.queue.push_back(id);
        }
        Ok(id)
    }

    /// The path to the state `hit` names: the steps to node `parent`, then
    /// `step`. A resolution has no step of its own: `resolved` is the model
    /// state it chose, which the path records on the draw step before it.
    fn undefined(
        &self,
        parent: Option<usize>,
        step: Option<&PathStep>,
        resolved: Option<Vec<u8>>,
        hit: Hit,
    ) -> Halt {
        let mut steps: Vec<PathStep> = step.cloned().into_iter().collect();
        let mut chosen = resolved;
        let mut cursor = parent;
        while let Some(at) = cursor {
            let Some((before, taken)) = &self.nodes[at].parent else {
                break;
            };
            match taken {
                Some(taken) => steps.push(PathStep {
                    resolved: chosen.take(),
                    ..taken.clone()
                }),
                // The edge into `at` resolved a draw: `at` is the chosen
                // post-state.
                None => chosen = Some(self.nodes[at].key.model.clone()),
            }
            cursor = Some(*before);
        }
        steps.reverse();
        let probability = steps
            .iter()
            .fold(Rational::from_integer(Integer::one()), |total, step| {
                total.mul(&step.probability)
            });
        Halt::Undefined(Box::new(UndefinedPath {
            position: u64::try_from(steps.len()).unwrap_or(u64::MAX),
            steps,
            probability,
            state_key: hit.state_key,
            read: hit.read,
            cause: hit.cause,
        }))
    }

    fn binding_of(&self, at: usize) -> Step<usize> {
        self.bindings
            .iter()
            .position(|(instance, _)| *instance == self.nodes[at].key.binding)
            .ok_or_else(|| fault("node-binding-is-a-binding").into())
    }

    /// The state reached from node `at` by a model step to `post` with
    /// `rewards`, its control and amounts advanced once. `step` is the model
    /// step the path gains; a resolution gains none.
    fn advance(
        &mut self,
        at: usize,
        post: &ModelSimState,
        rewards: &[(Identifier, Value)],
        step: Option<&PathStep>,
    ) -> Step<Target> {
        let binding = self.binding_of(at)?;
        let truths = match self.truths(post, binding)? {
            Ok(truths) => truths,
            Err(hit) => {
                let resolved = match step {
                    Some(_) => None,
                    None => Some(
                        self.system
                            .state_key(post)
                            .map_err(|_| fault("state-key-encodes"))?
                            .0,
                    ),
                };
                return Err(self.undefined(Some(at), step, resolved, hit));
            }
        };
        let source = &self.nodes[at];
        let (monitor, accumulator) = self.plan.step(
            source.key.monitor,
            source.key.accumulator.as_ref(),
            &source.truths,
            &truths,
            rewards,
        )?;
        let key = self.key_of(post, binding, monitor, accumulator, None)?;
        Ok(Target {
            key,
            model: post.clone(),
            truths,
            resolution: None,
        })
    }

    fn expand(&mut self, at: usize) -> Step<()> {
        let planned = match self.nodes[at].resolution.take() {
            Some(resolution) => self.resolve(at, &resolution)?,
            None => {
                let model = self.nodes[at].model.clone();
                match self.kind {
                    ProductKind::Dtmc => self.weighted(at, &model)?,
                    ProductKind::Mdp => self.choices(at, &model)?,
                }
            }
        };
        for edge in planned {
            if self.edges.len() >= self.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::MaxTransitions,
                    self.max_transitions,
                ));
            }
            let to = self.intern(edge.target, Some(at), edge.step)?;
            self.edges.push(ProbEdge {
                from: ProbStateId(at),
                choice: edge.choice,
                action: edge.action,
                draw: edge.draw,
                to: ProbStateId(to),
                probability: edge.probability,
                rewards: edge.rewards,
            });
        }
        Ok(())
    }

    /// The edges out of an intermediate state: each post-state of the draw,
    /// with probability 1.
    fn resolve(&mut self, at: usize, resolution: &Resolution) -> Step<Vec<Planned>> {
        let mut planned = Vec::with_capacity(resolution.successors.len());
        for post in &resolution.successors {
            let target = self.advance(at, post, &resolution.rewards, None)?;
            planned.push(Planned {
                choice: ProductAction::PostState(Box::new(target.key.clone())),
                action: None,
                draw: None,
                target,
                probability: Rational::from_integer(Integer::one()),
                rewards: self.system.zero_rewards(),
                step: None,
            });
        }
        Ok(planned)
    }

    /// A DTMC state's edges: FR-187's weighted steps.
    fn weighted(&mut self, at: usize, model: &ModelSimState) -> Step<Vec<Planned>> {
        let workload = self
            .workload
            .ok_or_else(|| fault("a-dtmc-has-a-workload"))?;
        let steps = match self.system.weighted_steps(model, workload) {
            Ok(Ok(steps)) => steps,
            Ok(Err(Unsteppable::NotMarkov(not_markov))) => {
                return Err(ProductRefusal::NotMarkov(Box::new(not_markov)).into());
            }
            Ok(Err(Unsteppable::UndecidedSuccessor(undecided))) => {
                return Err(ProductRefusal::UndecidedSuccessor(undecided).into());
            }
            Err(stop) => return Err(ProductRefusal::Expansion(stop).into()),
        };
        let mut planned = Vec::with_capacity(steps.len());
        for step in steps {
            let Some(identity) = step.identity else {
                planned.push(self.terminal(at, model, step.rewards)?);
                continue;
            };
            let taken = PathStep {
                action: Some(identity.clone()),
                draw: Some(step.random.clone()),
                probability: step.probability.clone(),
                resolved: None,
            };
            let target = self.advance(at, &step.post, &step.rewards, Some(&taken))?;
            planned.push(Planned {
                choice: ProductAction::Identity(identity.clone()),
                action: Some(identity),
                draw: Some(step.random),
                target,
                probability: step.probability,
                rewards: step.rewards,
                step: Some(taken),
            });
        }
        Ok(planned)
    }

    /// An MDP state's edges: FR-187's actions, each draw an edge, or an
    /// intermediate state when it has several post-states (SCH-2).
    fn choices(&mut self, at: usize, model: &ModelSimState) -> Step<Vec<Planned>> {
        let actions = match self.system.actions(model) {
            Ok(Ok(actions)) => actions,
            Ok(Err(undecided)) => {
                return Err(ProductRefusal::UndecidedSuccessor(undecided).into());
            }
            Err(stop) => return Err(ProductRefusal::Expansion(stop).into()),
        };
        if actions.is_empty() {
            let rewards = self.system.zero_rewards();
            return Ok(vec![self.terminal(at, model, rewards)?]);
        }
        let mut planned = Vec::new();
        for action in actions {
            self.action(at, model, action, &mut planned)?;
        }
        Ok(planned)
    }

    /// The edges of one action of an MDP state.
    fn action(
        &mut self,
        at: usize,
        model: &ModelSimState,
        action: Action,
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        let first = planned.len();
        for draw in action.draws {
            let taken = PathStep {
                action: Some(action.identity.clone()),
                draw: Some(draw.random.clone()),
                probability: draw.probability.clone(),
                resolved: None,
            };
            let mut successors = draw.successors;
            match successors.len() {
                0 => {
                    let (_, state) = self
                        .system
                        .state_key(model)
                        .map_err(|_| fault("state-key-encodes"))?;
                    return Err(ProductRefusal::NotMarkov(Box::new(NotMarkov {
                        state,
                        transition: action.identity,
                        random: draw.random,
                        post_states: 0,
                    }))
                    .into());
                }
                1 => {
                    let (_, post) = successors.remove(0);
                    let target = self.advance(at, &post, &draw.rewards, Some(&taken))?;
                    // Draws of one action that reach the same post-state with
                    // the same rewards are one edge with their summed
                    // probability.
                    if let Some(merged) = planned[first..].iter_mut().find(|edge| {
                        edge.target.key == target.key && same_rewards(&edge.rewards, &draw.rewards)
                    }) {
                        merged.probability = merged.probability.add(&draw.probability);
                        merged.draw = None;
                        merged.step = Some(PathStep {
                            action: taken.action,
                            draw: None,
                            probability: merged.probability.clone(),
                            resolved: None,
                        });
                        continue;
                    }
                    planned.push(Planned {
                        choice: ProductAction::Identity(action.identity.clone()),
                        action: Some(action.identity.clone()),
                        draw: Some(draw.random),
                        target,
                        probability: draw.probability,
                        rewards: draw.rewards,
                        step: Some(taken),
                    });
                }
                _ => {
                    let identity_bytes = self
                        .system
                        .transition_key(&successors[0].0)
                        .map_err(|_| fault("transition-key-encodes"))?;
                    let (monitor, accumulator) = {
                        let source = &self.nodes[at].key;
                        (source.monitor, source.accumulator.clone())
                    };
                    let binding = self.binding_of(at)?;
                    let key = self.key_of(
                        model,
                        binding,
                        monitor,
                        accumulator,
                        Some(Pending {
                            identity: action.identity.clone(),
                            random: draw.random.clone(),
                            identity_bytes,
                        }),
                    )?;
                    // The intermediate keeps the source's model state, so its
                    // predicates read as the source's; the monitor and the
                    // accumulator advance at the resolution, once.
                    let target = Target {
                        key,
                        model: model.clone(),
                        truths: self.nodes[at].truths.clone(),
                        resolution: Some(Resolution {
                            successors: successors.into_iter().map(|(_, post)| post).collect(),
                            rewards: draw.rewards.clone(),
                        }),
                    };
                    planned.push(Planned {
                        choice: ProductAction::Identity(action.identity.clone()),
                        action: Some(action.identity.clone()),
                        draw: Some(draw.random),
                        target,
                        probability: draw.probability,
                        rewards: draw.rewards,
                        step: Some(taken),
                    });
                }
            }
        }
        Ok(())
    }

    /// The edge of a terminal model state. A bounded event closes under the
    /// profile's false extension into a decided state; anything else takes
    /// the stutter step, which repeats the state.
    fn terminal(
        &mut self,
        at: usize,
        model: &ModelSimState,
        rewards: Vec<(Identifier, Value)>,
    ) -> Step<Planned> {
        let probability = Rational::from_integer(Integer::one());
        let taken = PathStep {
            action: None,
            draw: None,
            probability: probability.clone(),
            resolved: None,
        };
        let target = if self.plan.closes_at_terminal() {
            let Some(MonitorState::Formula(FormulaState::Running(running))) =
                self.nodes[at].key.monitor
            else {
                return Err(fault("a-terminal-event-state-is-undecided").into());
            };
            let verdict = self.plan.close(running)?;
            let binding = self.binding_of(at)?;
            let key = self.key_of(
                model,
                binding,
                Some(MonitorState::Formula(FormulaState::Decided(verdict))),
                None,
                None,
            )?;
            Target {
                key,
                model: model.clone(),
                truths: self.nodes[at].truths.clone(),
                resolution: None,
            }
        } else {
            self.advance(at, model, &rewards, Some(&taken))?
        };
        Ok(Planned {
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            target,
            probability,
            rewards,
            step: Some(taken),
        })
    }
}
