// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-571: decide whether local strategies realize a checked global
//! protocol (ADR-036 PA-16 to PA-19).
//!
//! The arena is the protocol's state graph under the one `qsl-eval`
//! protocol transition system, explored as a tree of plays to the bounds;
//! nothing here steps the protocol (PA-22).
//!
//! In the `bounded-static` profile no step is an owner's free branch
//! selection. A `choice`'s decision may read only what its owner received,
//! the protocol's captures and constants (FR-218: a `send` binder is never
//! an observable fact of a decision), and `qsl-eval` steps the one case
//! whose guard holds. Each branch is so fixed by records other roles sent,
//! which the environment picks, or by the initial state through the
//! captures. Every node is therefore an environment node, an owner node of
//! PA-17 has one branch, no earlier owner choice can avoid a history
//! (PA-18 (i)), and an environment defeat has one defeating play. The
//! roles' actions are their sends and receives; each role's histories are
//! grouped by the one `observations` function (the captures, then the
//! role's own sends and receives), every class commits to a common winning
//! action, and the replay walk re-executes the table before `proved`
//! (PA-19).

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::model_subject::ProtocolSubject;
use qsl_eval::simulation::model::{ModelFinding, ModelSystem, TerminalClass};
use qsl_eval::simulation::protocol::{
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, Sc, StepKind,
};
use qsl_eval::simulation::ExpansionStop;
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

use super::profile::{unsupported_construct, Profile};

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
    /// A role-knowledge or environment premise holds a fact. V1 defines no
    /// fact for groups (c) and (d), so only a declared-empty set is decided
    /// under.
    #[error(
        "invalid_runtime_input/invalid-value: the member `{0}` holds a fact; V1 decides only under a declared-empty set"
    )]
    PremiseFact(&'static str),
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
    /// A step's binder record has no RFC 8785 encoding.
    #[error("the record of the step at `{0}` has no canonical encoding")]
    Unencodable(String),
}

impl From<ExpansionStop> for RealizabilityRefusal {
    fn from(stop: ExpansionStop) -> Self {
        Self::Stopped(stop)
    }
}

/// The settlement of one realizability analysis (PA-20).
pub type RealizabilityOutcome =
    ProtocolAnalysisResult<RealizabilityWitness, RealizabilityCounterexample>;

/// Why a strategy table did not pass the replay walk (PA-19).
enum WalkFailure {
    /// A role reached a class the table does not hold; a play under the
    /// table violated an obligation, stopped with a committed action not
    /// enabled or ended undecided; or a play reached `max_depth` or
    /// `max_history`, so the walk did not close.
    NotShown,
    /// A run limit stopped the walk.
    Limit(RunLimit),
}

/// Why a node's value is left open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Open {
    /// `max_depth`, `max_history` or a method bound of `qsl-eval`.
    Horizon,
    /// An undetermined finding, or a terminal predicate undefined or
    /// refused.
    Undecided,
}

impl Open {
    fn cause(self) -> InconclusiveCause {
        match self {
            Self::Horizon => InconclusiveCause::Horizon,
            Self::Undecided => InconclusiveCause::Undecided,
        }
    }
}

/// A node's value under backward induction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Value {
    Win,
    Lose,
    Cut(Open),
}

/// The environment's join: every move must win. An open value keeps the
/// first cause in move order.
fn every(values: impl IntoIterator<Item = Value>) -> Value {
    let mut joined = Value::Win;
    for value in values {
        match value {
            Value::Lose => return Value::Lose,
            Value::Cut(open) if joined == Value::Win => joined = Value::Cut(open),
            Value::Cut(_) | Value::Win => {}
        }
    }
    joined
}

/// A role's step: the role, its action and what it observes.
#[derive(Clone, Debug)]
struct RoleStep {
    role: usize,
    action: StrategyAction,
    observed: Observed,
}

/// One explored move: its step, the role that takes it (none for an
/// environment step) and the child node.
struct Move {
    transition: ProtocolTransition,
    step: Option<RoleStep>,
    child: usize,
}

/// One arena node: a history, its moves and its value.
struct Node {
    parent: Option<(usize, usize)>,
    observations: Vec<ObservationClass>,
    moves: Vec<Move>,
    /// For a lost leaf, the obligation the play violates.
    lost: Option<CheckedNodeId>,
    leaf: bool,
    value: Value,
}

/// The names the arena's steps resolve to in the linked package.
struct Names {
    /// Each control by its declared name; `None` for a shared name.
    controls: BTreeMap<String, Option<CheckedNodeId>>,
    /// Each channel by its declared name; `None` for a shared name.
    channels: BTreeMap<String, Option<CheckedNodeId>>,
    /// Each channel's (sender, receiver) roles, by channel node.
    ends: BTreeMap<CheckedNodeId, (usize, usize)>,
    /// The roles, ascending.
    roles: Vec<CheckedNodeId>,
    /// The progress and terminal obligation: the `finish` member.
    finish: CheckedNodeId,
    /// The holder of the safety obligations: the clause.
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
        // `qsl-eval` names a control by its path through the `run` tree with
        // case labels (`Main/Decide/yes/Ship/PickIt`) and a channel by its
        // qualified name (`Flow::Pick`); the package declares each under the
        // clause by its own name. A name two nodes share resolves to neither.
        let name_of = |id: &CheckedNodeId| {
            graph
                .nodes
                .iter()
                .find(|node| &node.node_id == id)
                .and_then(|node| node.declaration.as_ref())
                .and_then(|declaration| declaration.qualified_name.last())
                .map(|segment| segment.to_string())
        };
        let mut controls = BTreeMap::new();
        let mut channels = BTreeMap::new();
        for (held, ids) in [
            (&mut controls, view.controls()),
            (&mut channels, view.channels()),
        ] {
            for id in ids {
                if let Some(name) = name_of(id) {
                    held.entry(name)
                        .and_modify(|shared: &mut Option<CheckedNodeId>| *shared = None)
                        .or_insert_with(|| Some((*id).clone()));
                }
            }
        }
        let mut roles: Vec<CheckedNodeId> = view.roles().iter().map(|id| (*id).clone()).collect();
        roles.sort();
        let index = |id: &CheckedNodeId| roles.binary_search(id).ok();
        let mut ends = BTreeMap::new();
        for channel in &premises.channels {
            if let (Some(sender), Some(receiver)) =
                (index(&channel.sender), index(&channel.receiver))
            {
                ends.insert(channel.channel.clone(), (sender, receiver));
            }
        }
        Ok(Self {
            controls,
            channels,
            ends,
            finish: view.finish().clone(),
            clause: view.clause().clone(),
            roles,
        })
    }

    /// The control `qsl-eval` names by the path `path`: `qsl-eval` exposes
    /// a step's control only as this string.
    #[ix_trace_rs::string_edge]
    fn node(&self, path: &str) -> Result<&CheckedNodeId, RealizabilityRefusal> {
        let name = path.rsplit('/').next().unwrap_or(path);
        self.controls
            .get(name)
            .and_then(Option::as_ref)
            .ok_or_else(|| RealizabilityRefusal::UnnamedNode(path.to_owned()))
    }

    /// The (sender, receiver) roles of the channel `qsl-eval` names
    /// `qualified`: `qsl-eval` exposes a step's channel only as this string.
    #[ix_trace_rs::string_edge]
    fn channel(&self, qualified: &str) -> Result<(usize, usize), RealizabilityRefusal> {
        let name = qualified.rsplit("::").next().unwrap_or(qualified);
        self.channels
            .get(name)
            .and_then(Option::as_ref)
            .and_then(|channel| self.ends.get(channel))
            .copied()
            .ok_or_else(|| RealizabilityRefusal::UnnamedNode(qualified.to_owned()))
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

fn exceeds(count: usize, bound: u64) -> bool {
    u64::try_from(count).unwrap_or(u64::MAX) > bound
}

fn charge(count: usize, limit: &'static str, value: u64) -> Result<(), RunLimit> {
    if exceeds(count, value) {
        Err(RunLimit { limit, value })
    } else {
        Ok(())
    }
}

/// The findings' value: a violated invariant loses the play, an undecided
/// evaluation leaves it open, and the FR-220 weak-access records classify
/// accesses without violating an obligation.
fn finding_value(findings: &[ModelFinding]) -> Option<Value> {
    let mut value = None;
    for finding in findings {
        match finding {
            ModelFinding::InvariantViolated { .. } => return Some(Value::Lose),
            ModelFinding::InvariantUndetermined { .. }
            | ModelFinding::ProtocolExpressionUndetermined { .. }
            | ModelFinding::ContractUndetermined { .. } => {
                value.get_or_insert(Value::Cut(Open::Undecided));
            }
            ModelFinding::WeakAccessShape(_)
            | ModelFinding::WeakAccessDeferred { .. }
            | ModelFinding::WeakAccessShared { .. } => {}
        }
    }
    value
}

impl Game<'_, '_> {
    /// The role that takes `transition`, with its action and observation;
    /// `None` for an environment step (PA-16).
    fn classify(
        &self,
        transition: &ProtocolTransition,
    ) -> Result<Option<RoleStep>, RealizabilityRefusal> {
        let unnamed = |what: &str| RealizabilityRefusal::UnnamedNode(what.to_owned());
        let path = transition.node().unwrap_or_default();
        let (role, receive_from) = match transition.kind() {
            StepKind::Send => {
                let channel = transition
                    .send_record()
                    .map(|record| record.channel())
                    .ok_or_else(|| unnamed(path))?;
                (self.names.channel(channel)?.0, None)
            }
            StepKind::Receive => {
                let (channel, _, _) = transition.received().ok_or_else(|| unnamed(path))?;
                let (sender, receiver) = self.names.channel(channel)?;
                (receiver, Some(sender))
            }
            _ => return Ok(None),
        };
        let node = self.names.node(path)?.clone();
        let payload = match self.system.binder_record_json(transition)? {
            Some(record) => {
                quire_canonical::to_vec(&record, quire_canonical::Limits::new(u64::MAX))
                    .map_err(|_| RealizabilityRefusal::Unencodable(path.to_owned()))?
            }
            None => Vec::new(),
        };
        Ok(Some(match receive_from {
            Some(sender) => RoleStep {
                role,
                action: StrategyAction::Receive {
                    peer: self.names.roles[sender].clone(),
                },
                observed: Observed::Received { node, payload },
            },
            None => RoleStep {
                role,
                action: StrategyAction::Send { node: node.clone() },
                observed: Observed::Sent { node, payload },
            },
        }))
    }

    /// The observation of every role at `state`, whose own sends and
    /// receives with their records are `steps[role]` in step order: the
    /// protocol captures every role holds (FR-218), then the role's steps
    /// (PA-16). The one function both the search and the replay walk
    /// compute classes with; each extends its parent's `steps` by the one
    /// step taken.
    fn observations(
        &self,
        state: &ProtocolState<Sc>,
        steps: &[ObservationClass],
    ) -> Result<Vec<ObservationClass>, RealizabilityRefusal> {
        let captures = self.system.capture_bytes(state)?;
        Ok(steps
            .iter()
            .map(|own| {
                captures
                    .iter()
                    .map(|payload| Observed::Captures {
                        payload: payload.clone(),
                    })
                    .chain(own.iter().cloned())
                    .collect()
            })
            .collect())
    }

    /// Whether some role's own steps exceed `max_history`.
    fn too_long(&self, steps: &[ObservationClass]) -> bool {
        steps.iter().any(|own| exceeds(own.len(), self.max_history))
    }

    /// `steps` extended by `step`, when a role takes it.
    fn extended(steps: &[ObservationClass], step: Option<&RoleStep>) -> Vec<ObservationClass> {
        let mut next = steps.to_vec();
        if let Some(step) = step {
            next[step.role].push(step.observed.clone());
        }
        next
    }

    /// Explore the arena to the bounds and evaluate every node (PA-17).
    fn explore(&self) -> Result<Result<Vec<Node>, RunLimit>, RealizabilityRefusal> {
        let mut nodes: Vec<Node> = Vec::new();
        let mut edges = 0usize;
        let roles = self.names.roles.len();
        // A pending node: its state, the parent move it hangs off, its
        // depth and each role's own steps.
        type Pending = (
            ProtocolState<Sc>,
            Option<(usize, usize)>,
            usize,
            Vec<ObservationClass>,
        );
        let mut stack: Vec<Pending> = self
            .system
            .initial()
            .into_iter()
            .rev()
            .map(|state| (state, None, 0, vec![Vec::new(); roles]))
            .collect();
        // Depth first, children in the system's canonical successor order;
        // a child's id is larger than its parent's.
        while let Some((state, parent, depth, steps)) = stack.pop() {
            let id = nodes.len();
            if let Err(limit) = charge(id + 1, "max_states", self.budget.max_states) {
                return Ok(Err(limit));
            }
            if let Some((from, at)) = parent {
                nodes[from].moves[at].child = id;
            }
            let (expansion, terminal) = self.system.successors_and_terminal(&state)?;
            let mut node = Node {
                parent,
                moves: Vec::new(),
                lost: None,
                leaf: true,
                value: Value::Cut(Open::Horizon),
                observations: self.observations(&state, &steps)?,
            };
            if let Some(value) = finding_value(&expansion.findings) {
                node.value = value;
                if value == Value::Lose {
                    node.lost = Some(self.names.clause.clone());
                }
            } else if expansion.successors.is_empty() {
                node.value = match terminal {
                    TerminalClass::Intended => Value::Win,
                    TerminalClass::Deadlocked => {
                        node.lost = Some(self.names.finish.clone());
                        Value::Lose
                    }
                    TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. } => {
                        Value::Cut(Open::Undecided)
                    }
                    // No successor though a step is enabled: a method bound
                    // of `qsl-eval` cut it.
                    TerminalClass::NotTerminal => Value::Cut(Open::Horizon),
                };
            } else if exceeds(depth + 1, self.max_depth) || self.too_long(&steps) {
                node.value = Value::Cut(Open::Horizon);
            } else {
                node.leaf = false;
                edges += expansion.successors.len();
                if let Err(limit) = charge(edges, "max_transitions", self.budget.max_transitions) {
                    return Ok(Err(limit));
                }
                for (at, (transition, next)) in expansion.successors.into_iter().enumerate().rev() {
                    let step = self.classify(&transition)?;
                    stack.push((
                        next,
                        Some((id, at)),
                        depth + 1,
                        Self::extended(&steps, step.as_ref()),
                    ));
                    node.moves.push(Move {
                        step,
                        transition,
                        child: usize::MAX,
                    });
                }
                node.moves.reverse();
            }
            nodes.push(node);
        }
        for id in (0..nodes.len()).rev() {
            if !nodes[id].leaf {
                nodes[id].value = every(nodes[id].moves.iter().map(|held| nodes[held.child].value));
            }
        }
        Ok(Ok(nodes))
    }
}

/// One decision point of one role: its history's node, the actions whose
/// every move wins there, and the actions no move of which loses (the
/// winning ones and those a cut leaves open).
struct Point {
    node: usize,
    winning: BTreeSet<StrategyAction>,
    not_lost: BTreeSet<StrategyAction>,
}

/// Every decision point of every role at the nodes not lost, by role then
/// observation class (PA-17): a node where the role can step, with each
/// action's value over its moves, and a won terminal node, where every role
/// ends.
fn decision_points(nodes: &[Node], roles: usize) -> Vec<BTreeMap<ObservationClass, Vec<Point>>> {
    let mut points: Vec<BTreeMap<ObservationClass, Vec<Point>>> =
        (0..roles).map(|_| BTreeMap::new()).collect();
    for (id, node) in nodes.iter().enumerate() {
        if node.value == Value::Lose {
            continue;
        }
        let mut stepping: Vec<Option<BTreeMap<&StrategyAction, Value>>> = vec![None; roles];
        if node.leaf && node.value == Value::Win {
            for held in &mut stepping {
                *held = Some(BTreeMap::from([(&StrategyAction::End, Value::Win)]));
            }
        }
        for held in &node.moves {
            if let Some(step) = &held.step {
                let child = nodes[held.child].value;
                stepping[step.role]
                    .get_or_insert_with(BTreeMap::new)
                    .entry(&step.action)
                    .and_modify(|value: &mut Value| *value = every([*value, child]))
                    .or_insert(child);
            }
        }
        for (role, held) in stepping.into_iter().enumerate() {
            if let Some(actions) = held {
                let with = |keep: fn(Value) -> bool| {
                    actions
                        .iter()
                        .filter(|(_, value)| keep(**value))
                        .map(|(action, _)| (*action).clone())
                        .collect()
                };
                points[role]
                    .entry(node.observations[role].clone())
                    .or_default()
                    .push(Point {
                        node: id,
                        winning: with(|value| value == Value::Win),
                        not_lost: with(|value| value != Value::Lose),
                    });
            }
        }
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

/// The first lost leaf below the lost node `id`, by the first losing move.
fn defeat(nodes: &[Node], mut id: usize) -> usize {
    while let Some(held) = nodes[id]
        .moves
        .iter()
        .find(|held| nodes[held.child].value == Value::Lose)
    {
        id = held.child;
    }
    id
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

type Settled = AnalysisOutcome<RealizabilityWitness, RealizabilityCounterexample>;

/// The admitted pieces of a request: its key, root, names and arena system,
/// or the outcome a request settles before any arena.
struct Admitted<'a> {
    key: ProtocolAnalysisKey,
    root: AssessedRoot,
    names: Names,
    system: Result<ProtocolSystem<'a, Sc>, Settled>,
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
    if request
        .strategy_profile
        .as_deref()
        .and_then(Profile::from_wire)
        .is_none()
    {
        return Err(RealizabilityRefusal::InvalidProfile(
            request.strategy_profile.unwrap_or_default(),
        ));
    }
    for (member, value) in [
        ("role_knowledge", &request.role_knowledge),
        ("environment", &request.environment),
    ] {
        if matches!(value, Some(Some(facts)) if !facts.is_empty()) {
            return Err(RealizabilityRefusal::PremiseFact(member));
        }
    }
    let root = AssessedRoot::new(request.assessed_root)?;
    let names = Names::new(package, &request.protocol, &premises)?;
    let not_stated = |member: &'static str, value: &Option<Option<BTreeSet<String>>>| {
        matches!(value, Some(None)).then_some(Settled::Inconclusive(
            InconclusiveCause::PremiseNotStated { member },
        ))
    };
    let system = if let Some(cause) = unsupported_construct(package, &request.protocol)? {
        Err(Settled::Unsupported(cause))
    } else if let Some(outcome) = not_stated("role_knowledge", &request.role_knowledge)
        .or_else(|| not_stated("environment", &request.environment))
    {
        Err(outcome)
    } else {
        let model = ModelSystem::new(request.subject.model.simulation.clone())
            .map_err(|failure| RealizabilityRefusal::Admission(Box::new(failure)))?;
        match ProtocolSystem::new(model, &request.subject, Sc) {
            Ok(system) => Ok(system),
            Err(ProtocolRefusal::Unsupported(construct)) => {
                Err(Settled::Unsupported(UnsupportedCause {
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
        Err(limit) => return Ok(result(key, Settled::Failed(limit))),
    };
    let roots: Vec<usize> = (0..nodes.len())
        .filter(|id| nodes[*id].parent.is_none())
        .collect();
    let root_value = every(roots.iter().map(|root| nodes[*root].value));
    if root_value == Value::Lose {
        let counterexample = environment_defeat(&nodes, &roots, &game, key, &admitted.root)?;
        return Ok(result(key, Settled::Refuted(counterexample)));
    }
    let mut strategies = Vec::new();
    let mut unforced = false;
    for (role, classes) in decision_points(&nodes, game.names.roles.len())
        .into_iter()
        .enumerate()
    {
        let mut table = BTreeMap::new();
        for (class, held) in classes {
            let mut common = held
                .first()
                .map(|point| point.winning.clone())
                .unwrap_or_default();
            for point in &held {
                common.retain(|action| point.winning.contains(action));
            }
            if let Some(action) = common.into_iter().next() {
                table.insert(class, action);
            } else if let Some((a, b)) = forced_pair(&held) {
                let counterexample =
                    indistinguishable(&nodes, &game, key, &admitted.root, role, a, b)?;
                return Ok(result(key, Settled::Refuted(counterexample)));
            } else {
                unforced = true;
            }
        }
        strategies.push(RoleStrategy {
            role: game.names.roles[role].clone(),
            table,
        });
    }
    if let Value::Cut(open) = root_value {
        return Ok(result(key, Settled::Inconclusive(open.cause())));
    }
    if unforced {
        return Ok(result(
            key,
            Settled::Inconclusive(InconclusiveCause::Unforced),
        ));
    }
    Ok(result(
        key,
        game.gate(key, strategies, admitted.open_range)?,
    ))
}

/// Settle the strategy table of `witness` over the request's arena by the
/// gate [`realize`] settles its own table with (PA-19): `proved`, with the
/// walk's closure, only after the replay walk passes; `inconclusive` for a
/// table that fails it or under an open delivery range; `failed` when a
/// run limit stops it. A request that reaches no arena settles as
/// [`realize`] settles it.
///
/// # Errors
///
/// [`RealizabilityRefusal`] as [`realize`] refuses.
pub fn replay_strategy(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'_>,
    witness: &RealizabilityWitness,
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
    Ok(result(
        key,
        game.gate(key, witness.strategies.clone(), admitted.open_range)?,
    ))
}

/// The first pair of points of one class whose winning sets are non-empty
/// and disjoint, with a disjointness that rests on no cut action: no action
/// is left unlost at both (PA-18 (i)). No owner move exists in the profile
/// (see the module), so no earlier owner choice can avoid either history.
fn forced_pair(points: &[Point]) -> Option<(&Point, &Point)> {
    points.iter().enumerate().find_map(|(at, a)| {
        points[at + 1..]
            .iter()
            .find(|b| {
                !a.winning.is_empty()
                    && !b.winning.is_empty()
                    && a.not_lost.is_disjoint(&b.not_lost)
            })
            .map(|b| (a, b))
    })
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

/// PA-18 (ii). No owner move exists in the profile, so the first lost
/// owner node has one branch: the defeating play from the first lost
/// initial state.
fn environment_defeat(
    nodes: &[Node],
    roots: &[usize],
    game: &Game<'_, '_>,
    key: &ProtocolAnalysisKey,
    root: &AssessedRoot,
) -> Result<RealizabilityCounterexample, RealizabilityRefusal> {
    let lost = roots
        .iter()
        .copied()
        .find(|root| nodes[*root].value == Value::Lose)
        .map_or(0, |root| defeat(nodes, root));
    let obligation = nodes[lost]
        .lost
        .clone()
        .unwrap_or_else(|| game.names.finish.clone());
    let play = history(nodes, lost);
    let position = play.len().saturating_sub(1);
    Ok(RealizabilityCounterexample::EnvironmentDefeat {
        key: key.clone(),
        defeats_per_branch: vec![BranchDefeat { play }],
        separating_witness: root.record(&obligation, position)?,
        obligation,
    })
}

impl Game<'_, '_> {
    /// The action `strategies` commit `role` to in the recomputed `class`.
    fn committed(
        &self,
        strategies: &[RoleStrategy],
        role: usize,
        class: &ObservationClass,
    ) -> Option<StrategyAction> {
        let role = &self.names.roles[role];
        strategies
            .iter()
            .find(|strategy| &strategy.role == role)
            .and_then(|strategy| strategy.table.get(class))
            .cloned()
    }

    /// PA-19's gate over the finished table `strategies`.
    fn gate(
        &self,
        key: &ProtocolAnalysisKey,
        strategies: Vec<RoleStrategy>,
        open_range: bool,
    ) -> Result<Settled, RealizabilityRefusal> {
        let closure = match self.walk(&strategies)? {
            Ok(closure) => closure,
            Err(WalkFailure::Limit(limit)) => return Ok(Settled::Failed(limit)),
            Err(WalkFailure::NotShown) => {
                return Ok(Settled::Inconclusive(
                    InconclusiveCause::StrategyReplayFailed,
                ))
            }
        };
        if open_range {
            return Ok(Settled::Inconclusive(InconclusiveCause::CutCount));
        }
        Ok(Settled::Proved(RealizabilityWitness {
            key: key.clone(),
            strategies,
            closure,
        }))
    }

    /// PA-19: walk every environment play against the finished table,
    /// selecting each role's action by its recomputed class, and check that
    /// every play wins with every committed action enabled.
    fn walk(
        &self,
        strategies: &[RoleStrategy],
    ) -> Result<Result<Closure, WalkFailure>, RealizabilityRefusal> {
        let roles = self.names.roles.len();
        let mut plays = 0u64;
        let mut visited = 0usize;
        // A pending state: the state, its depth and each role's own steps.
        let mut stack: Vec<(ProtocolState<Sc>, usize, Vec<ObservationClass>)> = self
            .system
            .initial()
            .into_iter()
            .rev()
            .map(|state| (state, 0, vec![Vec::new(); roles]))
            .collect();
        while let Some((state, depth, steps)) = stack.pop() {
            visited += 1;
            if let Err(limit) = charge(visited, "max_states", self.budget.max_states) {
                return Ok(Err(WalkFailure::Limit(limit)));
            }
            let (expansion, terminal) = self.system.successors_and_terminal(&state)?;
            if finding_value(&expansion.findings).is_some() || self.too_long(&steps) {
                return Ok(Err(WalkFailure::NotShown));
            }
            // A class the table lacks matters only where the role acts.
            let committed: Vec<Option<StrategyAction>> = self
                .observations(&state, &steps)?
                .iter()
                .enumerate()
                .map(|(role, class)| self.committed(strategies, role, class))
                .collect();
            let mut allowed = Vec::new();
            for (transition, next) in expansion.successors {
                let step = self.classify(&transition)?;
                if let Some(step) = &step {
                    match &committed[step.role] {
                        Some(action) if *action == step.action => {}
                        Some(_) => continue,
                        None => return Ok(Err(WalkFailure::NotShown)),
                    }
                }
                allowed.push((next, step));
            }
            if allowed.is_empty() {
                let ended = committed
                    .iter()
                    .all(|action| action.as_ref() == Some(&StrategyAction::End));
                if terminal != TerminalClass::Intended || !ended {
                    return Ok(Err(WalkFailure::NotShown));
                }
                plays += 1;
                continue;
            }
            if exceeds(depth + 1, self.max_depth) {
                return Ok(Err(WalkFailure::NotShown));
            }
            for (next, step) in allowed.into_iter().rev() {
                stack.push((next, depth + 1, Self::extended(&steps, step.as_ref())));
            }
        }
        Ok(Ok(Closure {
            plays,
            horizon: self.max_depth,
            closed: true,
        }))
    }
}

#[cfg(test)]
mod tests {
    use quire_contract_model::CheckedNodePathSegment;

    use super::*;

    fn send(name: &str) -> StrategyAction {
        StrategyAction::Send {
            node: CheckedNodeId {
                package: "pkg".into(),
                path: vec![CheckedNodePathSegment::Name(name.into())],
                ordinal: 0,
            },
        }
    }

    fn point(winning: &[StrategyAction], not_lost: &[StrategyAction]) -> Point {
        Point {
            node: 0,
            winning: winning.iter().cloned().collect(),
            not_lost: not_lost.iter().cloned().collect(),
        }
    }

    /// PA-18 (i): disjoint winning sets force a refutation only when the
    /// disjointness rests on no action a cut left open. Two `parallel`
    /// sends of one role, one of them cut at the horizon at the first
    /// history and winning at the second, force nothing.
    #[test]
    fn a_disjointness_resting_on_a_cut_action_is_not_forced() {
        let (x, y) = (send("X"), send("Y"));
        let cut = [
            point(&[x.clone()], &[x.clone(), y.clone()]),
            point(&[y.clone()], &[y.clone()]),
        ];
        assert!(forced_pair(&cut).is_none());
        let decided = [point(&[x.clone()], &[x]), point(&[y.clone()], &[y])];
        assert!(forced_pair(&decided).is_some());
    }
}
