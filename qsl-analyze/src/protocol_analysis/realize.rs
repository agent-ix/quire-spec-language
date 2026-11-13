// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-571: decide whether local strategies realize a checked global
//! protocol (ADR-036 PA-16 to PA-19).
//!
//! The arena is the protocol's state graph under the one `qsl-eval`
//! protocol transition system, explored as a tree of plays to the bounds;
//! nothing here steps the protocol (PA-22). A `send` whose binder a guard of
//! a `choice` its sender owns reads is that owner's branch selection; every
//! other step, the scheduling of threads, sent payloads and delivery
//! alternatives included, is the environment's. Backward induction gives
//! each node a value; each role's histories are grouped by the one
//! [`observation`] function and every class commits to a common winning
//! action, which the replay walk then re-executes before `proved`.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::model_subject::ProtocolSubject;
use qsl_eval::simulation::expansion::ExpansionStop;
use qsl_eval::simulation::model::{ModelFinding, ModelSystem, TerminalClass};
use qsl_eval::simulation::protocol::{
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, Sc, StepKind,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_package::protocol_view::{protocol_view, ViewRefusal};
use qsl_replay::protocol_analysis::{
    AnalysisOutcome, AnalyzerIdentity, AssessedRoot, AssessedRootDraft, BoundsDraft, BranchDefeat,
    Closure, DeliveryPremise, InconclusiveCause, MissingMember, ObservationClass, Observed,
    PremiseRefusal, ProtocolAnalysisKey, ProtocolAnalysisResult, ProtocolPremises,
    RealizabilityCounterexample, RealizabilityKeyDraft, RealizabilityWitness, RoleStrategy,
    RunBudget, RunLimit, StrategyAction, UnsupportedCause, UnsupportedSlug,
};
use qsl_semantics::model::observation::AdmissionFailure;
use quire_contract_model::{CheckedNodeId, CheckedPackageV2};

use super::profile::unsupported_construct;
use super::PROFILE;

/// The analyzer identity a realizability key carries (PA-15).
pub const ANALYZER: &str = "qsl-analyze::protocol_analysis";

/// A realizability request (PA-5). Key members are optional so that the key
/// builder names the first absent one (PA-13).
pub struct RealizabilityRequest<'a> {
    /// The `protocol_clause` node.
    pub protocol: CheckedNodeId,
    /// The protocol subject whose `qsl-eval` system is the arena.
    pub subject: ProtocolSubject<'a>,
    /// The FR-288 qualified definition key.
    pub definition_key: Option<String>,
    /// The strategy profile.
    pub strategy_profile: Option<String>,
    /// Premise group (c): the member, then the value (`None` is "not
    /// stated").
    pub role_knowledge: Option<Option<BTreeSet<String>>>,
    /// Premise group (d), as `role_knowledge`.
    pub environment: Option<Option<BTreeSet<String>>>,
    /// The bounds.
    pub bounds: Option<BoundsDraft>,
    /// The claim's assessed-subject root (FR-574); not a key member.
    pub assessed_root: Option<AssessedRootDraft>,
    /// The run limits (PA-14); not key members.
    pub budget: RunBudget,
}

/// A request the engine refuses before any analysis.
#[derive(Debug, thiserror::Error)]
pub enum RealizabilityRefusal {
    /// A key member or the assessed-subject root is absent.
    #[error(transparent)]
    Missing(#[from] MissingMember),
    /// The strategy profile is not `bounded-static`.
    #[error(
        "invalid_runtime_input/invalid-value: the member `strategy_profile` is `{0}`, not `bounded-static`"
    )]
    InvalidProfile(String),
    /// A protocol read refused.
    #[error(transparent)]
    View(#[from] ViewRefusal),
    /// Premises (a) and (b) were not built.
    #[error(transparent)]
    Premises(#[from] PremiseRefusal),
    /// The subject's model did not admit.
    #[error("the protocol subject's model does not admit: {0:?}")]
    Admission(Box<AdmissionFailure>),
    /// `qsl-eval` refused the subject for a reason other than a construct.
    #[error(transparent)]
    Protocol(ProtocolRefusal),
    /// An expansion stopped: an evaluation broke.
    #[error("the arena expansion stopped: {0:?}")]
    Stopped(ExpansionStop),
    /// A step names a node the linked package does not hold.
    #[error("the step at `{0}` names no node of the linked package")]
    UnnamedNode(String),
}

impl From<ExpansionStop> for RealizabilityRefusal {
    fn from(stop: ExpansionStop) -> Self {
        Self::Stopped(stop)
    }
}

/// The settlement of one realizability analysis (PA-20).
pub type RealizabilityOutcome =
    ProtocolAnalysisResult<RealizabilityWitness, RealizabilityCounterexample>;

/// Why a strategy table failed the replay walk (PA-19).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StrategyReplayFailure {
    /// A role reached a class the table does not hold.
    Unclassified {
        /// The role.
        role: CheckedNodeId,
    },
    /// A play under the table violated an obligation, stopped with a
    /// committed action not enabled, or ended undecided.
    Lost {
        /// The play, in step order.
        play: Vec<ProtocolTransition>,
    },
    /// A play reached the horizon or `max_history`: the walk did not close.
    NotClosed,
    /// A run limit stopped the walk.
    Limit(RunLimit),
}

/// A node's value under backward induction. `Cut` is a value a bound or an
/// undecided evaluation leaves open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Value {
    Win,
    Lose,
    Cut,
}

/// The environment's join: every move must win.
fn every(values: impl IntoIterator<Item = Value>) -> Value {
    let mut joined = Value::Win;
    for value in values {
        match value {
            Value::Lose => return Value::Lose,
            Value::Cut => joined = Value::Cut,
            Value::Win => {}
        }
    }
    joined
}

/// An owner's join: some branch must win.
fn some(values: impl IntoIterator<Item = Value>) -> Value {
    let mut joined = Value::Lose;
    for value in values {
        match value {
            Value::Win => return Value::Win,
            Value::Cut => joined = Value::Cut,
            Value::Lose => {}
        }
    }
    joined
}

/// Who moves a step and what it is to the mover.
#[derive(Clone, Debug)]
enum Mover {
    Environment,
    Role {
        role: usize,
        action: StrategyAction,
        observed: Observed,
        owner: bool,
    },
}

/// One explored move: its step, its mover and the child node.
struct Move {
    transition: ProtocolTransition,
    mover: Mover,
    child: usize,
}

/// One arena node: a history, its moves and its value.
struct Node {
    parent: Option<(usize, usize)>,
    observations: Vec<ObservationClass>,
    moves: Vec<Move>,
    /// For a leaf, the obligation a lost play violates.
    lost: Option<CheckedNodeId>,
    leaf: bool,
    value: Value,
}

/// The names the arena's steps resolve to in the linked package.
struct Names {
    /// Each control and channel by its `qsl-eval` declaration path.
    nodes: BTreeMap<String, CheckedNodeId>,
    /// Each channel's (sender, receiver) roles, by channel node.
    ends: BTreeMap<CheckedNodeId, (usize, usize)>,
    /// Each choice's owning role, by choice node.
    owners: BTreeMap<CheckedNodeId, usize>,
    /// The roles, ascending.
    roles: Vec<CheckedNodeId>,
    /// The progress and terminal obligation: the `finish` member.
    finish: CheckedNodeId,
    /// The safety obligations' holder: the clause.
    clause: CheckedNodeId,
}

impl Names {
    fn new(
        package: &CheckedPackageV2,
        protocol: &CheckedNodeId,
        premises: &ProtocolPremises,
    ) -> Result<Self, RealizabilityRefusal> {
        let view = protocol_view(package, protocol)?;
        let graph = package.resolved_graph();
        let path_of = |id: &CheckedNodeId| {
            graph
                .nodes
                .iter()
                .find(|node| &node.node_id == id)
                .and_then(|node| node.declaration.as_ref())
                .map(|declaration| {
                    declaration
                        .qualified_name
                        .iter()
                        .map(|segment| segment.to_string())
                        .collect::<Vec<_>>()
                })
        };
        let prefix = path_of(view.clause()).unwrap_or_default().len();
        let mut nodes = BTreeMap::new();
        for id in view.controls().iter().chain(view.channels()) {
            if let Some(path) = path_of(id) {
                nodes.insert(path.get(prefix..).unwrap_or_default().join("/"), (*id).clone());
            }
        }
        let mut roles: Vec<CheckedNodeId> = view.roles().iter().map(|id| (*id).clone()).collect();
        roles.sort();
        let index = |id: &CheckedNodeId| roles.binary_search(id).ok();
        let mut ends = BTreeMap::new();
        for channel in &premises.channels {
            if let (Some(sender), Some(receiver)) = (index(&channel.sender), index(&channel.receiver))
            {
                ends.insert(channel.channel.clone(), (sender, receiver));
            }
        }
        let mut owners = BTreeMap::new();
        for choice in &premises.choices {
            if let Some(owner) = index(&choice.owner) {
                owners.insert(choice.choice.clone(), owner);
            }
        }
        Ok(Self {
            nodes,
            ends,
            owners,
            finish: view.finish().clone(),
            clause: view.clause().clone(),
            roles,
        })
    }

    fn node(&self, path: &str) -> Result<&CheckedNodeId, RealizabilityRefusal> {
        self.nodes
            .get(path)
            .ok_or_else(|| RealizabilityRefusal::UnnamedNode(path.to_owned()))
    }

    fn channel(&self, path: &str) -> Result<(usize, usize), RealizabilityRefusal> {
        self.ends
            .get(self.node(path)?)
            .copied()
            .ok_or_else(|| RealizabilityRefusal::UnnamedNode(path.to_owned()))
    }
}

/// The game: the arena's system, the names and the bounds.
struct Game<'s, 'a> {
    system: &'s ProtocolSystem<'a, Sc>,
    names: Names,
    max_depth: u64,
    max_history: u64,
    budget: RunBudget,
}

/// The observation of `role` after `history`: its own sends and receives,
/// with their records, in step order (PA-16). An owner's branch selection
/// is the record of the send that fixes it, so its choices are in its own
/// sends; an outcome is visible to a role exactly through the records it
/// sent or received.
fn observation(
    role: usize,
    history: &[ProtocolTransition],
    classify: impl Fn(&ProtocolTransition) -> Result<Mover, RealizabilityRefusal>,
) -> Result<ObservationClass, RealizabilityRefusal> {
    let mut class = Vec::new();
    for step in history {
        if let Mover::Role {
            role: mover,
            observed,
            ..
        } = classify(step)?
        {
            if mover == role {
                class.push(observed);
            }
        }
    }
    Ok(class)
}

impl Game<'_, '_> {
    /// Who moves `transition`, with the mover's action and observation.
    fn classify(&self, transition: &ProtocolTransition) -> Result<Mover, RealizabilityRefusal> {
        let (role, peer, sending) = match transition.kind() {
            StepKind::Send => {
                let channel = transition
                    .send_record()
                    .map(|record| record.channel())
                    .ok_or_else(|| RealizabilityRefusal::UnnamedNode("send".to_owned()))?;
                let (sender, receiver) = self.names.channel(channel)?;
                (sender, receiver, true)
            }
            StepKind::Receive => {
                let (channel, _, _) = transition
                    .received()
                    .ok_or_else(|| RealizabilityRefusal::UnnamedNode("receive".to_owned()))?;
                let (sender, receiver) = self.names.channel(channel)?;
                (receiver, sender, false)
            }
            _ => return Ok(Mover::Environment),
        };
        let path = transition.node().unwrap_or_default();
        let node = self.names.node(path)?.clone();
        let payload = match self.system.binder_record_json(transition)? {
            Some(record) => serde_json::to_vec(&record)
                .map_err(|_| RealizabilityRefusal::UnnamedNode(path.to_owned()))?,
            None => Vec::new(),
        };
        if !sending {
            return Ok(Mover::Role {
                role,
                action: StrategyAction::Receive {
                    peer: self.names.roles[peer].clone(),
                },
                observed: Observed::Received { node, payload },
                owner: false,
            });
        }
        let mut owner = false;
        for choice in self.system.deciding_choices(transition) {
            owner |= self.names.owners.get(self.names.node(choice)?) == Some(&role);
        }
        Ok(Mover::Role {
            role,
            action: StrategyAction::Send {
                node: node.clone(),
                payload: owner.then(|| payload.clone()),
            },
            observed: Observed::Sent { node, payload },
            owner,
        })
    }

    /// A finding's value: undecided evaluations leave the play open.
    fn finding_value(findings: &[ModelFinding]) -> Option<Value> {
        let mut value = None;
        for finding in findings {
            match finding {
                ModelFinding::InvariantUndetermined { .. }
                | ModelFinding::ProtocolExpressionUndetermined { .. }
                | ModelFinding::ContractUndetermined { .. } => {
                    value.get_or_insert(Value::Cut);
                }
                _ => return Some(Value::Lose),
            }
        }
        value
    }

    fn charge(count: usize, limit: &'static str, value: u64) -> Result<(), RunLimit> {
        if u64::try_from(count).unwrap_or(u64::MAX) > value {
            Err(RunLimit { limit, value })
        } else {
            Ok(())
        }
    }

    /// Explore the arena to the bounds and evaluate every node (PA-17).
    fn explore(&self) -> Result<Result<Vec<Node>, RunLimit>, RealizabilityRefusal> {
        let mut nodes = Vec::new();
        let mut edges = 0usize;
        let roles = self.names.roles.len();
        let mut stack: Vec<(ProtocolState<Sc>, Option<(usize, usize)>, u64)> = self
            .system
            .initial()
            .into_iter()
            .rev()
            .map(|state| (state, None, 0))
            .collect();
        // Depth-first, children in the system's canonical successor order.
        while let Some((state, parent, depth)) = stack.pop() {
            let id = nodes.len();
            if let Err(limit) = Self::charge(id + 1, "max_states", self.budget.max_states) {
                return Ok(Err(limit));
            }
            let observations = match parent {
                None => vec![Vec::new(); roles],
                Some((from, at)) => {
                    let parent: &Node = &nodes[from];
                    let mut observations = parent.observations.clone();
                    if let Mover::Role { role, observed, .. } = &parent.moves[at].mover {
                        observations[*role].push(observed.clone());
                    }
                    observations
                }
            };
            if let Some((from, at)) = parent {
                let held: &mut Node = &mut nodes[from];
                held.moves[at].child = id;
            }
            let (expansion, terminal) = self.system.successors_and_terminal(&state)?;
            let mut node = Node {
                parent,
                observations,
                moves: Vec::new(),
                lost: None,
                leaf: true,
                value: Value::Cut,
            };
            let too_long = node
                .observations
                .iter()
                .any(|class| u64::try_from(class.len()).unwrap_or(u64::MAX) > self.max_history);
            if let Some(value) = Self::finding_value(&expansion.findings) {
                node.value = value;
                node.lost = Some(self.names.clause.clone());
            } else if expansion.successors.is_empty() {
                node.value = match terminal {
                    TerminalClass::Intended => Value::Win,
                    TerminalClass::Deadlocked => {
                        node.lost = Some(self.names.finish.clone());
                        Value::Lose
                    }
                    _ => Value::Cut,
                };
            } else if depth >= self.max_depth || too_long {
                node.value = Value::Cut;
            } else {
                node.leaf = false;
                edges += expansion.successors.len();
                if let Err(limit) =
                    Self::charge(edges, "max_transitions", self.budget.max_transitions)
                {
                    return Ok(Err(limit));
                }
                for (at, (transition, next)) in expansion.successors.into_iter().enumerate().rev()
                {
                    stack.push((next, Some((id, at)), depth + 1));
                    node.moves.push(Move {
                        mover: self.classify(&transition)?,
                        transition,
                        child: usize::MAX,
                    });
                }
                node.moves.reverse();
            }
            nodes.push(node);
        }
        // Children have larger ids than their parents: evaluate in reverse.
        for id in (0..nodes.len()).rev() {
            if nodes[id].leaf {
                continue;
            }
            let value = {
                let node = &nodes[id];
                let mut environment = Vec::new();
                let mut owned: BTreeMap<(usize, &StrategyAction), Vec<Value>> = BTreeMap::new();
                let mut groups: BTreeMap<usize, BTreeSet<&StrategyAction>> = BTreeMap::new();
                for held in &node.moves {
                    let child = nodes[held.child].value;
                    match &held.mover {
                        Mover::Role {
                            role,
                            action,
                            owner: true,
                            ..
                        } => {
                            owned.entry((*role, action)).or_default().push(child);
                            groups.entry(*role).or_default().insert(action);
                        }
                        _ => environment.push(child),
                    }
                }
                let branches = |role: usize| {
                    groups[&role]
                        .iter()
                        .map(|action| every(owned[&(role, *action)].iter().copied()))
                        .collect::<Vec<_>>()
                };
                every(
                    environment
                        .into_iter()
                        .chain(groups.keys().map(|role| some(branches(*role)))),
                )
            };
            nodes[id].value = value;
        }
        Ok(Ok(nodes))
    }
}

/// One decision point of one role: its history's node, the winning
/// actions there, and whether an earlier owner choice could avoid it.
struct Point {
    node: usize,
    winning: BTreeSet<StrategyAction>,
    avoidable: bool,
}

/// The value of the owner branch `action` of `role` at `node`.
fn branch_value(nodes: &[Node], node: &Node, role: usize, action: &StrategyAction) -> Value {
    every(node.moves.iter().filter_map(|held| match &held.mover {
        Mover::Role {
            role: mover,
            action: taken,
            owner: true,
            ..
        } if *mover == role && taken == action => Some(nodes[held.child].value),
        _ => None,
    }))
}

/// Every decision point of every role on the plays an owner does not
/// abandon, by role then observation class (PA-17).
fn decision_points(
    nodes: &[Node],
    roots: &[usize],
    roles: usize,
) -> Vec<BTreeMap<ObservationClass, Vec<Point>>> {
    let mut points: Vec<BTreeMap<ObservationClass, Vec<Point>>> =
        (0..roles).map(|_| BTreeMap::new()).collect();
    let mut stack: Vec<(usize, bool)> = roots.iter().rev().map(|root| (*root, false)).collect();
    while let Some((id, avoidable)) = stack.pop() {
        let node = &nodes[id];
        if node.value == Value::Lose {
            continue;
        }
        let mut winning: Vec<Option<BTreeSet<StrategyAction>>> = vec![None; roles];
        if node.leaf && node.value == Value::Win {
            for held in &mut winning {
                *held = Some(BTreeSet::from([StrategyAction::End]));
            }
        }
        let mut next = Vec::new();
        for held in &node.moves {
            let child = nodes[held.child].value;
            match &held.mover {
                Mover::Environment => next.push((held.child, avoidable)),
                Mover::Role {
                    role,
                    action,
                    owner,
                    ..
                } => {
                    let set = winning[*role].get_or_insert_with(BTreeSet::new);
                    let value = if *owner {
                        branch_value(nodes, node, *role, action)
                    } else {
                        child
                    };
                    if value == Value::Win {
                        set.insert(action.clone());
                    }
                    if *owner {
                        if value != Value::Lose {
                            let other = node.moves.iter().any(|sibling| match &sibling.mover {
                                Mover::Role {
                                    role: mover,
                                    action: taken,
                                    owner: true,
                                    ..
                                } => {
                                    mover == role
                                        && taken != action
                                        && branch_value(nodes, node, *role, taken) != Value::Lose
                                }
                                Mover::Environment | Mover::Role { .. } => false,
                            });
                            next.push((held.child, avoidable || other));
                        }
                    } else {
                        next.push((held.child, avoidable));
                    }
                }
            }
        }
        // A non-owner role's action is lost when any of its moves loses.
        for held in &node.moves {
            if let Mover::Role {
                role,
                action,
                owner: false,
                ..
            } = &held.mover
            {
                if nodes[held.child].value != Value::Win {
                    if let Some(set) = winning[*role].as_mut() {
                        set.remove(action);
                    }
                }
            }
        }
        for (role, held) in winning.into_iter().enumerate() {
            if let Some(winning) = held {
                points[role]
                    .entry(node.observations[role].clone())
                    .or_default()
                    .push(Point {
                        node: id,
                        winning,
                        avoidable,
                    });
            }
        }
        stack.extend(next.into_iter().rev());
    }
    points
}

/// The steps from an initial state to `id`.
fn history(nodes: &[Node], mut id: usize) -> Vec<ProtocolTransition> {
    let mut steps = Vec::new();
    while let Some((parent, at)) = nodes[id].parent {
        steps.push(nodes[parent].moves[at].transition.clone());
        id = parent;
    }
    steps.reverse();
    steps
}

/// From the lost node `id`, the first losing move, down to a lost leaf.
fn defeat(nodes: &[Node], mut id: usize) -> usize {
    while !nodes[id].leaf {
        let Some(held) = nodes[id]
            .moves
            .iter()
            .find(|held| nodes[held.child].value == Value::Lose)
        else {
            break;
        };
        id = held.child;
    }
    id
}

/// The first lost owner node from the lost roots, as (node, owner role),
/// or the lost leaf when no owner move lies on the defeat.
fn first_lost_owner(nodes: &[Node], roots: &[usize]) -> Result<(usize, usize), usize> {
    let mut id = roots
        .iter()
        .copied()
        .find(|root| nodes[*root].value == Value::Lose)
        .unwrap_or_default();
    loop {
        let node = &nodes[id];
        if node.leaf {
            return Err(id);
        }
        let mut found = None;
        for held in &node.moves {
            match &held.mover {
                Mover::Role {
                    role,
                    action,
                    owner: true,
                    ..
                } => {
                    let lost = node.moves.iter().all(|sibling| match &sibling.mover {
                        Mover::Role {
                            role: mover,
                            action: taken,
                            owner: true,
                            ..
                        } if mover == role => {
                            branch_value(nodes, node, *role, taken) == Value::Lose
                        }
                        _ => true,
                    });
                    let _ = action;
                    if lost {
                        return Ok((id, *role));
                    }
                }
                Mover::Environment | Mover::Role { .. } => {
                    if nodes[held.child].value == Value::Lose {
                        found = Some(held.child);
                        break;
                    }
                }
            }
        }
        match found {
            Some(child) => id = child,
            None => return Err(defeat(nodes, id)),
        }
    }
}

fn result(
    key: &ProtocolAnalysisKey,
    outcome: AnalysisOutcome<RealizabilityWitness, RealizabilityCounterexample>,
) -> RealizabilityOutcome {
    ProtocolAnalysisResult {
        key: key.clone(),
        outcome,
    }
}

/// The admitted pieces of a request: its key, root and arena system.
struct Admitted<'a> {
    key: ProtocolAnalysisKey,
    root: AssessedRoot,
    names: Names,
    system: Result<ProtocolSystem<'a, Sc>, AnalysisOutcome<RealizabilityWitness, RealizabilityCounterexample>>,
    max_depth: u64,
    max_history: u64,
    open_range: bool,
}

fn admit<'a>(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'a>,
) -> Result<Admitted<'a>, RealizabilityRefusal> {
    let premises = ProtocolPremises::from_view(package, &request.protocol)?;
    let open_range = premises.channels.iter().any(|channel| {
        matches!(
            channel.delivery,
            DeliveryPremise::AtLeastOnce | DeliveryPremise::Unknown
        )
    });
    let key = ProtocolAnalysisKey::realizability(RealizabilityKeyDraft {
        definition_key: request.definition_key,
        strategy_profile: request.strategy_profile.clone(),
        role_knowledge: request.role_knowledge.clone(),
        channels: Some(premises.channels.clone()),
        choices: Some(premises.choices.clone()),
        environment: request.environment.clone(),
        bounds: request.bounds,
        analyzer: Some(AnalyzerIdentity {
            identity: ANALYZER.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }),
    })?;
    let ProtocolAnalysisKey::Realizability(held) = &key else {
        unreachable!("the realizability builder returns a realizability key");
    };
    let (max_depth, max_history) = (
        held.bounds.max_depth,
        held.bounds.max_history.unwrap_or_default(),
    );
    if request.strategy_profile.as_deref() != Some(PROFILE) {
        return Err(RealizabilityRefusal::InvalidProfile(
            request.strategy_profile.unwrap_or_default(),
        ));
    }
    let root = AssessedRoot::new(request.assessed_root)?;
    let names = Names::new(package, &request.protocol, &premises)?;
    let stated = |member: &'static str, value: &Option<Option<BTreeSet<String>>>| {
        value
            .as_ref()
            .is_some_and(Option::is_none)
            .then_some(AnalysisOutcome::Inconclusive(InconclusiveCause::PremiseNotStated { member }))
    };
    let system = if let Some(cause) = unsupported_construct(package, &request.protocol)? {
        Err(AnalysisOutcome::Unsupported(cause))
    } else if let Some(outcome) = stated("role_knowledge", &request.role_knowledge)
        .or_else(|| stated("environment", &request.environment))
    {
        Err(outcome)
    } else {
        let model = ModelSystem::new(request.subject.model.simulation.clone())
            .map_err(|failure| RealizabilityRefusal::Admission(Box::new(failure)))?;
        match ProtocolSystem::new(model, &request.subject, Sc) {
            Ok(system) => Ok(system),
            Err(ProtocolRefusal::Unsupported(construct)) => {
                Err(AnalysisOutcome::Unsupported(UnsupportedCause {
                    slug: UnsupportedSlug::EvaluatorUnsupported,
                    node: names.clause.clone(),
                    detail: Some(construct.construct().to_owned()),
                }))
            }
            Err(other) => return Err(RealizabilityRefusal::Protocol(other)),
        }
    };
    Ok(Admitted {
        key,
        root,
        names,
        system,
        max_depth,
        max_history,
        open_range,
    })
}

/// Decide whether local strategies realize the protocol (FR-571).
///
/// # Errors
///
/// [`RealizabilityRefusal`] for a request the engine refuses at admission.
pub fn realize(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'_>,
) -> Result<RealizabilityOutcome, RealizabilityRefusal> {
    let budget = request.budget;
    let admitted = admit(package, request)?;
    let key = &admitted.key;
    let system = match admitted.system {
        Ok(system) => system,
        Err(outcome) => return Ok(result(key, outcome)),
    };
    let game = Game {
        system: &system,
        names: admitted.names,
        max_depth: admitted.max_depth,
        max_history: admitted.max_history,
        budget,
    };
    let nodes = match game.explore()? {
        Ok(nodes) => nodes,
        Err(limit) => return Ok(result(key, AnalysisOutcome::Failed(limit))),
    };
    let roots: Vec<usize> = (0..nodes.len())
        .filter(|id| nodes[*id].parent.is_none())
        .collect();
    let root_value = every(roots.iter().map(|root| nodes[*root].value));
    let roles = &game.names.roles;

    if root_value == Value::Lose {
        let counterexample = environment_defeat(&nodes, &roots, &game, key, &admitted.root)?;
        return Ok(result(key, AnalysisOutcome::Refuted(counterexample)));
    }
    let points = decision_points(&nodes, &roots, roles.len());
    let mut strategies = Vec::new();
    let mut unforced = false;
    for (role, classes) in points.iter().enumerate() {
        let mut table = BTreeMap::new();
        for (class, held) in classes {
            let mut common = held.first().map(|point| point.winning.clone()).unwrap_or_default();
            for point in held {
                common.retain(|action| point.winning.contains(action));
            }
            if let Some(action) = common.into_iter().next() {
                table.insert(class.clone(), action);
                continue;
            }
            if let Some((a, b)) = forced_pair(held) {
                let counterexample =
                    indistinguishable(&nodes, &game, key, &admitted.root, role, a, b)?;
                return Ok(result(key, AnalysisOutcome::Refuted(counterexample)));
            }
            unforced = true;
        }
        strategies.push(RoleStrategy {
            role: roles[role].clone(),
            table,
        });
    }
    if root_value == Value::Cut {
        return Ok(result(
            key,
            AnalysisOutcome::Inconclusive(InconclusiveCause::Horizon),
        ));
    }
    if unforced {
        return Ok(result(
            key,
            AnalysisOutcome::Inconclusive(InconclusiveCause::Unforced),
        ));
    }
    let closure = match game.walk(&strategies)? {
        Ok(closure) => closure,
        Err(StrategyReplayFailure::Limit(limit)) => {
            return Ok(result(key, AnalysisOutcome::Failed(limit)));
        }
        Err(_) => {
            return Ok(result(
                key,
                AnalysisOutcome::Inconclusive(InconclusiveCause::StrategyReplayFailed),
            ));
        }
    };
    if admitted.open_range {
        return Ok(result(
            key,
            AnalysisOutcome::Inconclusive(InconclusiveCause::CutCount),
        ));
    }
    Ok(result(
        key,
        AnalysisOutcome::Proved(RealizabilityWitness {
            key: key.clone(),
            strategies,
            closure,
        }),
    ))
}

/// Re-run the replay walk of PA-19 for `witness` over the request's arena.
///
/// # Errors
///
/// [`RealizabilityRefusal`] as [`realize`] refuses; the walk's own result is
/// the inner `Result`. A request that does not reach the arena (unsupported,
/// a premise not stated) has no walk and returns
/// [`StrategyReplayFailure::NotClosed`].
pub fn replay_strategy(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'_>,
    witness: &RealizabilityWitness,
) -> Result<Result<Closure, StrategyReplayFailure>, RealizabilityRefusal> {
    let budget = request.budget;
    let admitted = admit(package, request)?;
    let Ok(system) = admitted.system else {
        return Ok(Err(StrategyReplayFailure::NotClosed));
    };
    let game = Game {
        system: &system,
        names: admitted.names,
        max_depth: admitted.max_depth,
        max_history: admitted.max_history,
        budget,
    };
    game.walk(&witness.strategies)
}

/// The first pair of points of one class whose winning sets are disjoint
/// and non-empty and neither of which an earlier owner choice avoids.
fn forced_pair(points: &[Point]) -> Option<(&Point, &Point)> {
    for (at, a) in points.iter().enumerate() {
        for b in &points[at + 1..] {
            if !a.avoidable
                && !b.avoidable
                && !a.winning.is_empty()
                && !b.winning.is_empty()
                && a.winning.is_disjoint(&b.winning)
            {
                return Some((a, b));
            }
        }
    }
    None
}

fn indistinguishable(
    nodes: &[Node],
    game: &Game<'_, '_>,
    key: &ProtocolAnalysisKey,
    root: &AssessedRoot,
    role: usize,
    a: &Point,
    b: &Point,
) -> Result<RealizabilityCounterexample, RealizabilityRefusal> {
    let (history_a, history_b) = (history(nodes, a.node), history(nodes, b.node));
    let obligation = game.names.finish.clone();
    let position = history_a.len().max(history_b.len()).saturating_sub(1);
    Ok(RealizabilityCounterexample::IndistinguishableHistories {
        key: key.clone(),
        role: game.names.roles[role].clone(),
        history_a,
        history_b,
        action_a: a.winning.first().cloned().unwrap_or(StrategyAction::End),
        action_b: b.winning.first().cloned().unwrap_or(StrategyAction::End),
        separating_witness: root.record(&obligation, position)?,
        obligation,
    })
}

fn environment_defeat(
    nodes: &[Node],
    roots: &[usize],
    game: &Game<'_, '_>,
    key: &ProtocolAnalysisKey,
    root: &AssessedRoot,
) -> Result<RealizabilityCounterexample, RealizabilityRefusal> {
    let mut defeats = Vec::new();
    match first_lost_owner(nodes, roots) {
        Err(leaf) => defeats.push((leaf, None, None)),
        Ok((id, owner)) => {
            let node = &nodes[id];
            let mut branches = BTreeSet::new();
            for held in &node.moves {
                if let Mover::Role {
                    role,
                    action,
                    owner: true,
                    ..
                } = &held.mover
                {
                    if *role == owner && branches.insert(action.clone()) {
                        let lost = node
                            .moves
                            .iter()
                            .find(|sibling| {
                                matches!(&sibling.mover, Mover::Role { role: mover, action: taken, owner: true, .. }
                                    if *mover == owner && taken == action)
                                    && nodes[sibling.child].value == Value::Lose
                            })
                            .map_or(held.child, |sibling| sibling.child);
                        defeats.push((
                            defeat(nodes, lost),
                            Some(game.names.roles[owner].clone()),
                            Some(action.clone()),
                        ));
                    }
                }
            }
        }
    }
    let first = defeats.first().map_or(0, |(leaf, _, _)| *leaf);
    let obligation = nodes[first]
        .lost
        .clone()
        .unwrap_or_else(|| game.names.finish.clone());
    let defeats_per_branch: Vec<BranchDefeat> = defeats
        .into_iter()
        .map(|(leaf, owner, branch)| BranchDefeat {
            owner,
            branch,
            play: history(nodes, leaf),
        })
        .collect();
    let position = defeats_per_branch
        .first()
        .map_or(0, |held| held.play.len().saturating_sub(1));
    Ok(RealizabilityCounterexample::EnvironmentDefeat {
        key: key.clone(),
        defeats_per_branch,
        separating_witness: root.record(&obligation, position)?,
        obligation,
    })
}

impl Game<'_, '_> {
    /// PA-19: walk every environment play against the finished table,
    /// selecting each role's action by its recomputed class, and check that
    /// every play wins with every committed action enabled.
    fn walk(
        &self,
        strategies: &[RoleStrategy],
    ) -> Result<Result<Closure, StrategyReplayFailure>, RealizabilityRefusal> {
        let classify = |transition: &ProtocolTransition| self.classify(transition);
        let committed = |role: usize, history: &[ProtocolTransition]| {
            let class = observation(role, history, &classify)?;
            Ok::<_, RealizabilityRefusal>(
                strategies
                    .iter()
                    .find(|strategy| strategy.role == self.names.roles[role])
                    .and_then(|strategy| strategy.table.get(&class))
                    .cloned(),
            )
        };
        let mut plays = 0u64;
        let mut visited = 0usize;
        let mut stack: Vec<(ProtocolState<Sc>, Vec<ProtocolTransition>)> = self
            .system
            .initial()
            .into_iter()
            .rev()
            .map(|state| (state, Vec::new()))
            .collect();
        while let Some((state, played)) = stack.pop() {
            visited += 1;
            if let Err(limit) = Self::charge(visited, "max_states", self.budget.max_states) {
                return Ok(Err(StrategyReplayFailure::Limit(limit)));
            }
            let lost = |play: &[ProtocolTransition]| {
                Ok(Err(StrategyReplayFailure::Lost {
                    play: play.to_vec(),
                }))
            };
            let (expansion, terminal) = self.system.successors_and_terminal(&state)?;
            if !expansion.findings.is_empty() {
                return lost(&played);
            }
            for role in 0..self.names.roles.len() {
                let length = observation(role, &played, &classify)?.len();
                if u64::try_from(length).unwrap_or(u64::MAX) > self.max_history {
                    return Ok(Err(StrategyReplayFailure::NotClosed));
                }
            }
            let mut allowed = Vec::new();
            for (transition, next) in expansion.successors {
                let take = match self.classify(&transition)? {
                    Mover::Environment => true,
                    Mover::Role { role, action, .. } => match committed(role, &played)? {
                        Some(held) => held == action,
                        None => {
                            return Ok(Err(StrategyReplayFailure::Unclassified {
                                role: self.names.roles[role].clone(),
                            }))
                        }
                    },
                };
                if take {
                    allowed.push((transition, next));
                }
            }
            if allowed.is_empty() {
                if terminal != TerminalClass::Intended {
                    return lost(&played);
                }
                for role in 0..self.names.roles.len() {
                    match committed(role, &played)? {
                        Some(StrategyAction::End) => {}
                        Some(_) => return lost(&played),
                        None => {
                            return Ok(Err(StrategyReplayFailure::Unclassified {
                                role: self.names.roles[role].clone(),
                            }))
                        }
                    }
                }
                plays += 1;
                continue;
            }
            if u64::try_from(played.len()).unwrap_or(u64::MAX) >= self.max_depth {
                return Ok(Err(StrategyReplayFailure::NotClosed));
            }
            for (transition, next) in allowed.into_iter().rev() {
                let mut extended = played.clone();
                extended.push(transition);
                stack.push((next, extended));
            }
        }
        Ok(Ok(Closure {
            plays,
            horizon: self.max_depth,
            closed: true,
        }))
    }
}
