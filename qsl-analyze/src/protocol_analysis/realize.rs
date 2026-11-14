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
//! (PA-19). A forced refutation settles `refuted` only after the
//! counterexample replay arm re-walks it over the arena (PA-20).

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::model_subject::ProtocolSubject;
use qsl_eval::simulation::model::{ModelFinding, ModelSystem, TerminalClass};
use qsl_eval::simulation::protocol::{
    ProtocolRefusal, ProtocolState, ProtocolSystem, ProtocolTransition, Sc,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use qsl_package::protocol_view::ViewRefusal;
use qsl_replay::protocol_analysis::arena::{
    self, Arena, ArenaError, ArenaNames, FindingValue, RoleStep, Seen,
};
use qsl_replay::protocol_analysis::{
    replay_realizability_counterexample, AnalysisOutcome, AnalyzerIdentity, AssessedRoot,
    AssessedRootDraft, BoundsDraft, BranchDefeat, Closure, DeliveryPremise, HistoryEnd,
    InconclusiveCause, LostAction, MissingMember, ObservationClass, Play, PlayStep, PremiseRefusal,
    ProtocolAnalysisKey, ProtocolAnalysisResult, ProtocolPremises, RealizabilityCounterexample,
    RealizabilityDisagreement, RealizabilityKeyDraft, RealizabilityWitness, RoleStrategy,
    RunBudget, RunLimit, StrategyAction, UnsupportedCause, UnsupportedSlug,
};
use qsl_replay::ModelCheckLimit;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_contract_model::{CheckedNodeId, CheckedPackageV2};
use quire_exact::ScalarLimits;

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
}

impl From<ExpansionStop> for RealizabilityRefusal {
    fn from(stop: ExpansionStop) -> Self {
        Self::Stopped(stop)
    }
}

impl From<ArenaError> for RealizabilityRefusal {
    fn from(error: ArenaError) -> Self {
        match error {
            ArenaError::View(refusal) => Self::View(refusal),
            ArenaError::Stopped(stop) => Self::Stopped(stop),
            ArenaError::UnnamedNode(path) => Self::UnnamedNode(path),
        }
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
    /// The index of the play's initial state.
    initial: usize,
    observations: Vec<ObservationClass>,
    moves: Vec<Move>,
    /// For a lost leaf, the obligation the play violates.
    lost: Option<CheckedNodeId>,
    leaf: bool,
    value: Value,
}

/// The game: the arena's system, the names and the bounds.
struct Game<'s, 'a> {
    arena: Arena<'s, 'a>,
    max_depth: u64,
    max_history: u64,
    budget: RunBudget,
}

fn exceeds(count: usize, bound: u64) -> bool {
    u64::try_from(count).unwrap_or(u64::MAX) > bound
}

fn charge(count: usize, limit: ModelCheckLimit, value: u64) -> Result<(), RunLimit> {
    if exceeds(count, value) {
        Err(RunLimit { limit, value })
    } else {
        Ok(())
    }
}

/// The value a state's findings give a play.
fn finding_value(findings: &[ModelFinding]) -> Option<Value> {
    arena::finding_value(findings).map(|value| match value {
        FindingValue::Lose => Value::Lose,
        FindingValue::Undecided => Value::Cut(Open::Undecided),
    })
}

impl Game<'_, '_> {
    fn too_long(&self, seen: &Seen) -> bool {
        seen.exceeds(self.max_history)
    }

    /// Explore the arena to the bounds and evaluate every node (PA-17).
    fn explore(&self) -> Result<Result<Vec<Node>, RunLimit>, RealizabilityRefusal> {
        let mut nodes: Vec<Node> = Vec::new();
        let mut edges = 0usize;
        let roles = self.arena.names.roles().len();
        // A pending node: its state, the parent move it hangs off, its
        // play's initial state, its depth and what its play showed the roles.
        type Pending = (
            ProtocolState<Sc>,
            Option<(usize, usize)>,
            usize,
            usize,
            Seen,
        );
        let mut stack: Vec<Pending> = self
            .arena
            .system
            .initial()
            .into_iter()
            .enumerate()
            .rev()
            .map(|(initial, state)| (state, None, initial, 0, Seen::new(roles)))
            .collect();
        // Depth first, children in the system's canonical successor order;
        // a child's id is larger than its parent's.
        while let Some((state, parent, initial, depth, seen)) = stack.pop() {
            let id = nodes.len();
            if let Err(limit) = charge(id + 1, ModelCheckLimit::MaxStates, self.budget.max_states) {
                return Ok(Err(limit));
            }
            if let Some((from, at)) = parent {
                nodes[from].moves[at].child = id;
            }
            let seen = self.arena.bind(&state, seen)?;
            let (expansion, terminal) = self.arena.system.successors_and_terminal(&state)?;
            let mut node = Node {
                parent,
                initial,
                moves: Vec::new(),
                lost: None,
                leaf: true,
                value: Value::Cut(Open::Horizon),
                observations: seen.observations(),
            };
            if let Some(value) = finding_value(&expansion.findings) {
                node.value = value;
                if value == Value::Lose {
                    node.lost = Some(self.arena.names.clause().clone());
                }
            } else if expansion.successors.is_empty() {
                node.value = match terminal {
                    TerminalClass::Intended => Value::Win,
                    TerminalClass::Deadlocked => {
                        node.lost = Some(self.arena.names.finish().clone());
                        Value::Lose
                    }
                    TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. } => {
                        Value::Cut(Open::Undecided)
                    }
                    // No successor though a step is enabled: a method bound
                    // of `qsl-eval` cut it.
                    TerminalClass::NotTerminal => Value::Cut(Open::Horizon),
                };
            } else if exceeds(depth + 1, self.max_depth) || self.too_long(&seen) {
                node.value = Value::Cut(Open::Horizon);
            } else {
                node.leaf = false;
                edges += expansion.successors.len();
                if let Err(limit) = charge(
                    edges,
                    ModelCheckLimit::MaxTransitions,
                    self.budget.max_transitions,
                ) {
                    return Ok(Err(limit));
                }
                for (at, (transition, next)) in expansion.successors.into_iter().enumerate().rev() {
                    let step = self.arena.classify(&transition)?;
                    stack.push((
                        next,
                        Some((id, at)),
                        initial,
                        depth + 1,
                        seen.extended(step.as_ref()),
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
fn history(nodes: &[Node], mut id: usize) -> Play {
    let initial = nodes[id].initial;
    let mut steps = Vec::new();
    while let Some((parent, at)) = nodes[id].parent {
        steps.push(PlayStep {
            successor: at,
            transition: nodes[parent].moves[at].transition.clone(),
        });
        id = parent;
    }
    steps.reverse();
    Play { initial, steps }
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
    names: ArenaNames,
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
    let names = qsl_replay::spine::arena_names(package, &request.protocol, &premises)?;
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
                    node: names.clause().clone(),
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
    decide(package, request, &|_| {})
}

/// [`realize`] with each forced refutation passed through `edit` before the
/// replay arm settles it, so a test can show an edited counterexample
/// settles `inconclusive`.
///
/// # Errors
///
/// As [`realize`].
#[cfg(feature = "test-support")]
pub fn realize_tampered(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'_>,
    edit: &dyn Fn(&mut RealizabilityCounterexample),
) -> Result<RealizabilityOutcome, RealizabilityRefusal> {
    decide(package, request, edit)
}

/// `settled`, with an evaluation-meter stop settled `failed`, naming the
/// counter and the value in force, as every run limit is (PA-14).
fn metered(
    meter: &ScalarLimits,
    settled: Result<Settled, RealizabilityRefusal>,
) -> Result<Settled, RealizabilityRefusal> {
    match settled {
        Err(RealizabilityRefusal::Stopped(stop)) => match stop.limit {
            Some(ExpansionLimit::EvaluationMeter(counter)) => Ok(Settled::Failed(RunLimit {
                limit: ModelCheckLimit::EvaluationMeter { counter },
                value: qsl_replay::meter_bound(meter, counter),
            })),
            _ => Err(RealizabilityRefusal::Stopped(stop)),
        },
        other => other,
    }
}

fn decide(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'_>,
    edit: &dyn Fn(&mut RealizabilityCounterexample),
) -> Result<RealizabilityOutcome, RealizabilityRefusal> {
    let budget = request.budget;
    let meter = request.subject.model.simulation.meter;
    let admitted = admit(package, request)?;
    let key = admitted.key.clone();
    Ok(result(
        &key,
        metered(&meter, search(admitted, budget, edit))?,
    ))
}

fn search(
    admitted: Admitted<'_>,
    budget: RunBudget,
    edit: &dyn Fn(&mut RealizabilityCounterexample),
) -> Result<Settled, RealizabilityRefusal> {
    let key = &admitted.key;
    let root = &admitted.root;
    let system = match admitted.system {
        Ok(system) => system,
        Err(outcome) => return Ok(outcome),
    };
    let game = Game {
        arena: Arena {
            system: &system,
            names: admitted.names,
        },
        max_depth: admitted.max_depth,
        max_history: admitted.max_history,
        budget,
    };
    let nodes = match game.explore()? {
        Ok(nodes) => nodes,
        Err(limit) => return Ok(Settled::Failed(limit)),
    };
    let roots: Vec<usize> = (0..nodes.len())
        .filter(|id| nodes[*id].parent.is_none())
        .collect();
    let root_value = every(roots.iter().map(|root| nodes[*root].value));
    if root_value == Value::Lose {
        let mut counterexample = environment_defeat(&nodes, &roots, &game, key, root)?;
        edit(&mut counterexample);
        return game.replayed(key, root, counterexample);
    }
    let mut strategies = Vec::new();
    let mut unforced = false;
    for (role, classes) in decision_points(&nodes, game.arena.names.roles().len())
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
                let mut counterexample = indistinguishable(&nodes, &game, key, root, role, a, b)?;
                edit(&mut counterexample);
                return game.replayed(key, root, counterexample);
            } else {
                unforced = true;
            }
        }
        strategies.push(RoleStrategy {
            role: game.arena.names.roles()[role].clone(),
            table,
        });
    }
    if let Value::Cut(open) = root_value {
        return Ok(Settled::Inconclusive(open.cause()));
    }
    if unforced {
        return Ok(Settled::Inconclusive(InconclusiveCause::Unforced));
    }
    game.gate(key, strategies, admitted.open_range)
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
    let meter = request.subject.model.simulation.meter;
    let admitted = admit(package, request)?;
    let key = admitted.key.clone();
    let settled = match admitted.system {
        Err(outcome) => Ok(outcome),
        Ok(system) => {
            let game = Game {
                arena: Arena {
                    system: &system,
                    names: admitted.names,
                },
                max_depth: admitted.max_depth,
                max_history: admitted.max_history,
                budget,
            };
            game.gate(&key, witness.strategies.clone(), admitted.open_range)
        }
    };
    Ok(result(&key, metered(&meter, settled)?))
}

/// Replay `counterexample` over the request's arena by the arm [`realize`]
/// settles its own refutations with (PA-20): `None` when the request reaches
/// no arena (it settles before one, as [`realize`] settles it), else the
/// walk's agreement or the disagreement it found.
///
/// # Errors
///
/// [`RealizabilityRefusal`] as [`realize`] refuses.
pub fn replay_counterexample(
    package: &CheckedPackageV2,
    request: RealizabilityRequest<'_>,
    counterexample: &RealizabilityCounterexample,
) -> Result<Option<Result<(), RealizabilityDisagreement>>, RealizabilityRefusal> {
    let admitted = admit(package, request)?;
    let Ok(system) = admitted.system else {
        return Ok(None);
    };
    let arena = Arena {
        system: &system,
        names: admitted.names,
    };
    Ok(Some(replay_realizability_counterexample(
        &arena,
        &admitted.key,
        &admitted.root,
        counterexample,
    )?))
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
    let obligation = game.arena.names.finish().clone();
    let position = history_a
        .steps
        .len()
        .max(history_b.steps.len())
        .saturating_sub(1);
    // Every action the role can take at both ends is lost at one of them
    // (the not-lost sets are disjoint): its evidence is the first move of
    // the action whose child is lost, then the first losing path below it.
    let mut lost = Vec::new();
    for action in actions(nodes, a.node, role).intersection(&actions(nodes, b.node, role)) {
        let found = [(HistoryEnd::A, a.node), (HistoryEnd::B, b.node)]
            .into_iter()
            .find_map(|(at, node)| {
                nodes[node]
                    .moves
                    .iter()
                    .enumerate()
                    .find_map(|(index, held)| {
                        let taken = held
                            .step
                            .as_ref()
                            .is_some_and(|step| step.role == role && &step.action == action);
                        (taken && nodes[held.child].value == Value::Lose).then(|| {
                            let leaf = history(nodes, defeat(nodes, held.child));
                            let from = history(nodes, node).steps.len();
                            debug_assert_eq!(leaf.steps[from].successor, index);
                            LostAction {
                                action: action.clone(),
                                at,
                                continuation: leaf.steps[from..].to_vec(),
                            }
                        })
                    })
            });
        lost.extend(found);
    }
    Ok(RealizabilityCounterexample::IndistinguishableHistories {
        key: key.clone(),
        role: game.arena.names.roles()[role].clone(),
        history_a,
        history_b,
        action_a: a.winning.first().cloned().unwrap_or(StrategyAction::End),
        action_b: b.winning.first().cloned().unwrap_or(StrategyAction::End),
        lost,
        separating_witness: root.record(&obligation, position)?,
        obligation,
    })
}

/// The actions `role` can take at `node`: its steps among the node's moves,
/// and `End` at a won leaf.
fn actions(nodes: &[Node], node: usize, role: usize) -> BTreeSet<StrategyAction> {
    let held = &nodes[node];
    let mut found: BTreeSet<StrategyAction> = held
        .moves
        .iter()
        .filter_map(|held| held.step.as_ref())
        .filter(|step| step.role == role)
        .map(|step| step.action.clone())
        .collect();
    if held.leaf && held.value == Value::Win {
        found.insert(StrategyAction::End);
    }
    found
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
        .unwrap_or_else(|| game.arena.names.finish().clone());
    let play = history(nodes, lost);
    let position = play.steps.len().saturating_sub(1);
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
        let role = &self.arena.names.roles()[role];
        strategies
            .iter()
            .find(|strategy| &strategy.role == role)
            .and_then(|strategy| strategy.table.get(class))
            .cloned()
    }

    /// A forced refutation settles `refuted` only after its counterexample
    /// replays over the arena; a disagreeing replay is `inconclusive`
    /// (PA-20, ADR-018 SM-7).
    fn replayed(
        &self,
        key: &ProtocolAnalysisKey,
        root: &AssessedRoot,
        counterexample: RealizabilityCounterexample,
    ) -> Result<Settled, RealizabilityRefusal> {
        Ok(
            match replay_realizability_counterexample(&self.arena, key, root, &counterexample)? {
                Ok(()) => Settled::Refuted(counterexample),
                Err(_) => Settled::Inconclusive(InconclusiveCause::ReplayDisagreed),
            },
        )
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
        let roles = self.arena.names.roles().len();
        let mut plays = 0u64;
        let mut visited = 0usize;
        // A pending state: the state, its depth and what its play showed
        // the roles.
        let mut stack: Vec<(ProtocolState<Sc>, usize, Seen)> = self
            .arena
            .system
            .initial()
            .into_iter()
            .rev()
            .map(|state| (state, 0, Seen::new(roles)))
            .collect();
        while let Some((state, depth, seen)) = stack.pop() {
            visited += 1;
            if let Err(limit) = charge(visited, ModelCheckLimit::MaxStates, self.budget.max_states)
            {
                return Ok(Err(WalkFailure::Limit(limit)));
            }
            let seen = self.arena.bind(&state, seen)?;
            let (expansion, terminal) = self.arena.system.successors_and_terminal(&state)?;
            if finding_value(&expansion.findings).is_some() || self.too_long(&seen) {
                return Ok(Err(WalkFailure::NotShown));
            }
            // A class the table lacks matters only where the role acts.
            let committed: Vec<Option<StrategyAction>> = seen
                .observations()
                .iter()
                .enumerate()
                .map(|(role, class)| self.committed(strategies, role, class))
                .collect();
            let mut allowed = Vec::new();
            for (transition, next) in expansion.successors {
                let step = self.arena.classify(&transition)?;
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
                stack.push((next, depth + 1, seen.extended(step.as_ref())));
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

    fn point<const W: usize, const N: usize>(
        winning: [StrategyAction; W],
        not_lost: [StrategyAction; N],
    ) -> Point {
        Point {
            node: 0,
            winning: winning.into(),
            not_lost: not_lost.into(),
        }
    }

    /// PA-14: an evaluation-meter stop of the arena is the result's own
    /// `failed` outcome, naming the counter and the value in force; any
    /// other stop stays a refusal.
    #[ix_trace_rs::trace("TC-1413", "FR-573-AC-5")]
    #[test]
    fn an_evaluation_meter_stop_fails_the_run_naming_the_counter() {
        let meter = ScalarLimits {
            integer_bits: 1,
            decimal_digits: 1,
            scale_expansion: 1,
            text_input_bytes: 1,
            text_scalars: 1,
            normalized_scalars: 1,
            unit_edges: 1,
            value_occurrences: 1,
            work_units: 7,
            result_units: 1,
        };
        let stop = |limit| {
            Err(RealizabilityRefusal::Stopped(ExpansionStop {
                cause: quire_semantic_value::catalog::CatalogCode::new(
                    "resource_exhausted",
                    "insufficient-next-charge",
                ),
                limit,
            }))
        };
        let counter = quire_exact::LimitKind::WorkUnits;
        assert_eq!(
            metered(&meter, stop(Some(ExpansionLimit::EvaluationMeter(counter)))).ok(),
            Some(Settled::Failed(RunLimit {
                limit: ModelCheckLimit::EvaluationMeter { counter },
                value: 7,
            }))
        );
        assert!(matches!(
            metered(&meter, stop(Some(ExpansionLimit::MaxCandidates))),
            Err(RealizabilityRefusal::Stopped(_))
        ));
    }

    /// PA-18 (i): disjoint winning sets force a refutation only when the
    /// disjointness rests on no action a cut left open. Two `parallel`
    /// sends of one role, one of them cut at the horizon at the first
    /// history and winning at the second, force nothing.
    #[test]
    fn a_disjointness_resting_on_a_cut_action_is_not_forced() {
        let (x, y) = (send("X"), send("Y"));
        let cut = [
            point([x.clone()], [x.clone(), y.clone()]),
            point([y.clone()], [y.clone()]),
        ];
        assert!(forced_pair(&cut).is_none());
        let decided = [point([x.clone()], [x]), point([y.clone()], [y])];
        assert!(forced_pair(&decided).is_some());
    }
}
