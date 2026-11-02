// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (ADR-028 TA-1 to TA-7): the digital MDP of a closed timed subject.
//!
//! A digital timed state is the discrete model state with an integer value
//! per clock, capped at one plus the clock's largest compared constant after
//! every constant is scaled to an integer, the optional deadline clock of a
//! `[0, D]` event, and TS-5's tail mode. Its moves are the unit delay, the
//! discrete steps with integer clocks, and, in a quiescent state, the idle
//! tail's enter-delay, delay and observation.
//!
//! Over every scheduler the scheduler chooses each delay and each scheduled
//! identity. Under a workload, a `discrete` delay distribution turns the race
//! of ADR-026 SD-3 into one probabilistic edge per outcome; a delay no
//! distribution gives stays the scheduler's choice between a unit delay and
//! the workload's resolution of the identity.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive, Zero};
use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::{
    same_rewards, Action, ClockValuation, GuardedStep, ModelSimState, ModelSystem, NotMarkov,
    PlaceConstraint, PredicateReading, ScheduledIdentity, TimedStop,
    TimedSystem,
};
use qsl_eval::simulation::StateKey;
use qsl_replay::ModelCheckLimit;
use qsl_semantics::check::{
    BinaryTemporal, CheckedInterval, CheckedPredicate, CheckedProbabilisticClaim, ClockOp,
    ConvexPart, DelayDistribution, Event, FormulaNode, ProbForm, ProbScheduler, UnaryTemporal,
    Workload,
};
use quire_exact::{Cancel, Identifier, Integer, ObjectReference, Rational, Value, ValueType};
use quire_semantic_value::catalog::InternalFault;

use super::{
    DeadlineKind, DeadlinePhase, ExactProbLimits, MonitorState, MonitorVerdict, PathStep, Pending,
    ProbBuild, ProbEdge, ProbKey, ProbProduct, ProbStateId, ProductAction, ProductKind,
    ProductRefusal, ProductStop, UndefinedPath, UndefinedRead,
};

/// Build the digital product of a claim over a timed subject (FR-204).
///
/// # Errors
///
/// [`ProductRefusal`]: the admission causes of FR-204, a model that is not a
/// Markov decision process or chain at some digital state, a budget or the
/// cancellation.
pub(super) fn build<'s, 'a>(
    system: &'s ModelSystem<'a>,
    timed: TimedSystem<'s, 'a>,
    claim: &CheckedProbabilisticClaim,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbBuild, ProductRefusal> {
    let package = system.package();
    let workload = match &claim.scheduler {
        ProbScheduler::Workload(name) => Some(
            package
                .graph()
                .probabilistic_model()
                .workload(name)
                .ok_or_else(|| fault("claim-workload-is-declared"))?,
        ),
        ProbScheduler::Every => None,
    };
    let form = Form::of(claim)?;
    admit(system, &timed, claim, workload, &form)?;
    let scale = scale_of(system, &timed, &form)?;
    let caps = caps_of(&timed, &scale)?;
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
                        .bound_reference(object.clone(), &instance)
                        .map_err(ProductRefusal::Internal)?;
                    Ok((Some(instance), Some(reference)))
                })
                .collect::<Result<Vec<_>, ProductRefusal>>()?
        }
    };
    let builder = Builder {
        system,
        timed,
        workload,
        form,
        scale,
        caps,
        bindings,
        max_states: limits.model.limits.max_states,
        max_transitions: limits.model.limits.max_transitions,
        cancel,
        locals: BTreeMap::new(),
        nodes: Vec::new(),
        index: BTreeMap::new(),
        readings: BTreeMap::new(),
        edges: Vec::new(),
        zero_delay: Vec::new(),
        queue: VecDeque::new(),
        decisions: false,
        time_locked: Vec::new(),
    };
    builder.run()
}

fn fault(invariant: &'static str) -> ProductRefusal {
    ProductRefusal::Internal(InternalFault::new("exact-probabilistic-digital", invariant))
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// The claim forms the digital route admits (ADR-028 TA-4).
enum Form<'c> {
    /// `eventually[0, D] holds(P)` or `holds(A) until[0, D] holds(B)`.
    Deadline {
        kind: DeadlineKind,
        /// `D` in the model's time unit.
        bound: Rational,
        reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
    },
    /// An unbounded reachability form: its predicates label the states.
    Labels(Vec<(UndefinedRead, &'c CheckedPredicate)>),
}

impl<'c> Form<'c> {
    fn of(claim: &'c CheckedProbabilisticClaim) -> Result<Self, ProductRefusal> {
        let shape = || ProductRefusal::TimedFormShape { span: claim.span };
        match &claim.form {
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            } => {
                let checked = formula.formula();
                let nodes = checked.nodes();
                let root = nodes.get(checked.root().index()).ok_or_else(shape)?;
                let holds = |id: super::FormulaNodeId| {
                    matches!(nodes.get(id.index()), Some(FormulaNode::Holds { .. }))
                        .then(|| formula.condition(id.index()).map(|p| (id, p)))
                        .flatten()
                };
                let (kind, interval, atoms) = match root {
                    FormulaNode::Unary {
                        operator: UnaryTemporal::Eventually,
                        interval: Some(CheckedInterval::Timed(interval)),
                        operand,
                    } => (
                        DeadlineKind::Eventually,
                        interval,
                        vec![holds(*operand).ok_or_else(shape)?],
                    ),
                    FormulaNode::Binary {
                        operator: BinaryTemporal::Until,
                        interval: Some(CheckedInterval::Timed(interval)),
                        left,
                        right,
                    } => (
                        DeadlineKind::Until,
                        interval,
                        vec![
                            holds(*left).ok_or_else(shape)?,
                            holds(*right).ok_or_else(shape)?,
                        ],
                    ),
                    _ => return Err(shape()),
                };
                if !(interval.lower().is_zero() && interval.lower_closed()) {
                    return Err(shape());
                }
                Ok(Self::Deadline {
                    kind,
                    bound: interval.upper().clone(),
                    reads: atoms
                        .into_iter()
                        .map(|(id, predicate)| (UndefinedRead::Atom(id), predicate))
                        .collect(),
                })
            }
            ProbForm::Reach { .. } | ProbForm::ExpectedReward { .. } => {
                let (plan, reads) = super::plan::Plan::of(claim, 1).map_err(|_| shape())?;
                drop(plan);
                Ok(Self::Labels(reads))
            }
            _ => Err(shape()),
        }
    }

    fn reads(&self) -> &[(UndefinedRead, &'c CheckedPredicate)] {
        match self {
            Self::Deadline { reads, .. } | Self::Labels(reads) => reads,
        }
    }
}

/// Every atomic clock constraint of a convex part, families included.
fn constraints_of<'m>(part: &'m ConvexPart, found: &mut Vec<&'m qsl_semantics::check::ClockConstraint>) {
    found.extend(part.clocks());
    for family in part.families() {
        for inner in family.parts() {
            constraints_of(inner, found);
        }
    }
}

/// The model's clock constraints in guard-then-invariant declaration order.
fn model_constraints<'m>(
    timed: &'m TimedSystem<'_, '_>,
) -> Vec<&'m qsl_semantics::check::ClockConstraint> {
    let mut found = Vec::new();
    for guard in timed.model().guards() {
        for part in guard.parts() {
            constraints_of(part, &mut found);
        }
    }
    for invariant in timed.model().invariants() {
        found.extend(invariant.bounds());
    }
    found
}

/// FR-204 "Admission": the closed condition, the delay distributions and the
/// form shape.
fn admit(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    workload: Option<&Workload>,
    form: &Form<'_>,
) -> Result<(), ProductRefusal> {
    let strict = model_constraints(timed)
        .into_iter()
        .filter(|constraint| matches!(constraint.op(), ClockOp::Less | ClockOp::Greater))
        .map(|constraint| constraint.span())
        .min_by_key(|span| span.start);
    if let Some(span) = strict {
        return Err(ProductRefusal::StrictClockConstraint { span });
    }
    if let ProbForm::Probability {
        event: Event::Formula { formula, .. },
        ..
    } = &claim.form
    {
        let open = formula.formula().nodes().iter().any(|node| match node {
            FormulaNode::Unary {
                interval: Some(CheckedInterval::Timed(interval)),
                ..
            }
            | FormulaNode::Binary {
                interval: Some(CheckedInterval::Timed(interval)),
                ..
            } => !interval.upper_closed() || !interval.lower_closed(),
            _ => false,
        });
        if open && matches!(form, Form::Deadline { .. }) {
            return Err(ProductRefusal::StrictClockConstraint { span: claim.span });
        }
    }
    let delays = system.package().graph().probabilistic_model().delays();
    for delay in delays {
        let admitted = workload.is_some() && matches!(delay.distribution(), DelayDistribution::Discrete(_));
        if !admitted {
            let operation = delay.operation();
            return Err(ProductRefusal::DelayDistribution {
                operation: system
                    .operation_text(operation.declaring(), operation.name())
                    .unwrap_or_else(|| operation.name())
                    .to_owned(),
            });
        }
    }
    Ok(())
}

/// The least common denominator of every time constant of the model and the
/// claim (ADR-028 TA-3), as one positive integer.
fn scale_of(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    form: &Form<'_>,
) -> Result<u64, ProductRefusal> {
    let mut denominators: Vec<BigInt> = Vec::new();
    let mut note = |value: &Rational| denominators.push(value.denominator().as_big().clone());
    for constraint in model_constraints(timed) {
        note(constraint.bound());
    }
    for resets in timed.model().resets() {
        for reset in resets.resets() {
            note(reset.value());
        }
    }
    if let Form::Deadline { bound, .. } = form {
        note(bound);
    }
    for delay in system.package().graph().probabilistic_model().delays() {
        if let DelayDistribution::Discrete(values) = delay.distribution() {
            for (delay, _) in values {
                note(delay);
            }
        }
    }
    let scale = denominators
        .iter()
        .fold(BigInt::one(), |factor, denominator| factor.lcm(denominator));
    scale
        .to_u64()
        .filter(|scale| *scale > 0)
        .ok_or_else(|| fault("digital-scale-fits-u64"))
}

/// A rational time constant as a count of digital units.
fn units(value: &Rational, scale: u64) -> Option<u64> {
    let scaled = value.numerator().as_big() * BigInt::from(scale);
    let (quotient, remainder) = scaled.div_rem(value.denominator().as_big());
    remainder.is_zero().then(|| quotient.to_u64()).flatten()
}

/// Each clock place's cap: one plus its clock's largest scaled compared
/// constant (FR-196 "Canonical probability product key").
fn caps_of(timed: &TimedSystem<'_, '_>, scale: &u64) -> Result<Vec<u64>, ProductRefusal> {
    let mut largest: BTreeMap<qsl_semantics::check::ClockId, u64> = BTreeMap::new();
    for constraint in model_constraints(timed) {
        let constant = units(constraint.bound(), *scale).ok_or_else(|| fault("constant-scales"))?;
        let entry = largest.entry(constraint.clock()).or_insert(0);
        *entry = (*entry).max(constant);
    }
    timed
        .places()
        .iter()
        .map(|place| {
            largest
                .get(&place.clock())
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| fault("cap-fits-u64"))
        })
        .collect()
}

/// TS-5's tail mode (FR-196 canonical key).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Tail {
    Running,
    IdleReady,
    IdleDelayed,
}

impl Tail {
    fn tag(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::IdleReady => "idle-ready",
            Self::IdleDelayed => "idle-delayed",
        }
    }
}

/// A digital timed state.
#[derive(Clone)]
struct DState {
    model: ModelSimState,
    clocks: Vec<u64>,
    deadline: Option<u64>,
    tail: Tail,
}

/// What a discrete model state offers, computed once per state.
struct Local {
    actions: Vec<Action>,
    steps: Vec<GuardedStep>,
    invariants: Vec<PlaceConstraint>,
    urgent: bool,
    quiescent: bool,
}

/// A draw of an enabled identity: its enabled post-states.
struct EnabledDraw {
    random: Vec<Value>,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
    posts: Vec<(ModelSimState, Vec<u64>)>,
}

/// A scheduled identity enabled at a digital state.
struct Enabled {
    identity: ScheduledIdentity,
    draws: Vec<EnabledDraw>,
}

struct Resolution {
    successors: Vec<DState>,
    rewards: Vec<(Identifier, Value)>,
}

struct Node {
    key: ProbKey,
    state: DState,
    truths: Vec<bool>,
    parent: Option<(usize, Option<TraceStep>)>,
    initial: u32,
    resolution: Option<Resolution>,
}

struct Target {
    key: ProbKey,
    state: DState,
    truths: Vec<bool>,
    resolution: Option<Resolution>,
}

#[derive(Clone)]
struct TraceStep {
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    probability: Rational,
}

struct Planned {
    choice: ProductAction,
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    target: Target,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
    step: Option<TraceStep>,
    zero_delay: bool,
}

enum Halt {
    Refusal(ProductRefusal),
    Undefined(Box<UndefinedPath>),
}

impl From<ProductRefusal> for Halt {
    fn from(refusal: ProductRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

type Step<T> = Result<T, Halt>;

struct Hit {
    read: UndefinedRead,
    cause: qsl_eval::temporal::UndefinedCause,
    state_key: StateKey,
    model: Vec<u8>,
}

/// The window of one scheduled identity: the delays, in digital units, after
/// which it is enabled, and whether the window continues forever.
struct Window {
    identity: ScheduledIdentity,
    delays: BTreeSet<u64>,
    forever: bool,
    given: Option<Vec<(u64, Rational)>>,
}

impl Window {
    fn contains(&self, delay: u64, reached: u64) -> bool {
        self.delays.contains(&delay.min(reached)) && (delay <= reached || self.forever)
    }
}

/// The digital states reached by successive unit delays from a state: index
/// `k` is the state after `k` units, and `forever` holds when the last one
/// repeats under another unit.
struct Trajectory {
    states: Vec<DState>,
    forever: bool,
}

struct Builder<'r, 'a, 'c> {
    system: &'r ModelSystem<'a>,
    timed: TimedSystem<'r, 'a>,
    workload: Option<&'r Workload>,
    form: Form<'c>,
    scale: u64,
    caps: Vec<u64>,
    bindings: Vec<(Option<ModelInstance>, Option<ObjectReference>)>,
    max_states: usize,
    max_transitions: usize,
    cancel: &'r Cancel,
    locals: BTreeMap<Vec<u8>, Rc<Local>>,
    nodes: Vec<Node>,
    index: BTreeMap<ProbKey, usize>,
    readings: BTreeMap<(usize, Vec<u8>), Vec<bool>>,
    edges: Vec<ProbEdge>,
    zero_delay: Vec<bool>,
    queue: VecDeque<usize>,
    decisions: bool,
    time_locked: Vec<usize>,
}

fn timed_refusal(stop: TimedStop) -> Halt {
    match stop {
        TimedStop::Expansion(stop) => Halt::Refusal(ProductRefusal::Expansion(stop)),
        _ => Halt::Refusal(fault("timed-moves-decide")),
    }
}

impl<'c> Builder<'_, '_, 'c> {
    fn run(mut self) -> Result<ProbBuild, ProductRefusal> {
        match self.explore() {
            Ok(initial) => self.finish(initial).map(ProbBuild::Built),
            Err(Halt::Refusal(refusal)) => Err(refusal),
            Err(Halt::Undefined(path)) => Ok(ProbBuild::Undefined(*path)),
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

    // -- digital arithmetic ------------------------------------------------

    fn valuation(&self, clocks: &[u64]) -> Step<ClockValuation> {
        let denominator = Integer::from_big(BigInt::from(self.scale));
        let values = clocks
            .iter()
            .map(|value| {
                Rational::new(Integer::from_big(BigInt::from(*value)), denominator.clone())
                    .map_err(|_| fault("scale-is-positive").into())
            })
            .collect::<Step<Vec<_>>>()?;
        Ok(ClockValuation::new(values))
    }

    fn digital(&self, valuation: &ClockValuation) -> Step<Vec<u64>> {
        valuation
            .values()
            .iter()
            .zip(&self.caps)
            .map(|(value, cap)| {
                let units = units(value, self.scale).ok_or_else(|| fault("clock-on-grid"))?;
                Ok(units.min(*cap))
            })
            .collect()
    }

    fn bound_units(&self, bound: &Rational) -> Step<u64> {
        units(bound, self.scale).ok_or_else(|| fault("constant-scales").into())
    }

    fn deadline_bound(&self) -> Step<Option<u64>> {
        match &self.form {
            Form::Deadline { bound, .. } => self.bound_units(bound).map(Some),
            Form::Labels(_) => Ok(None),
        }
    }

    fn local(&mut self, model: &ModelSimState) -> Step<Rc<Local>> {
        let (key, _) = self
            .system
            .state_key(model)
            .map_err(|_| fault("state-key-encodes"))?;
        if let Some(known) = self.locals.get(&key) {
            return Ok(Rc::clone(known));
        }
        let actions = match self.system.actions(model) {
            Ok(Ok(actions)) => actions,
            Ok(Err(undecided)) => {
                return Err(ProductRefusal::UndecidedSuccessor(undecided).into());
            }
            Err(stop) => return Err(ProductRefusal::Expansion(stop).into()),
        };
        let steps = self.timed.guarded_steps(model).map_err(timed_refusal)?;
        let invariants = self.timed.invariants(model).map_err(timed_refusal)?;
        let urgent = self.timed.is_urgent(model).map_err(timed_refusal)?;
        let local = Rc::new(Local {
            actions,
            steps,
            quiescent: invariants.is_empty() && !urgent,
            invariants,
            urgent,
        });
        self.locals.insert(key, Rc::clone(&local));
        Ok(local)
    }

    /// The state after one more digital unit, or `None` when the delay is
    /// not admissible (urgent, or a time invariant fails after it).
    fn unit_delay(&self, local: &Local, state: &DState) -> Step<Option<(Vec<u64>, Option<u64>)>> {
        if local.urgent {
            return Ok(None);
        }
        let clocks: Vec<u64> = state
            .clocks
            .iter()
            .zip(&self.caps)
            .map(|(value, cap)| if value < cap { value + 1 } else { *value })
            .collect();
        let valuation = self.valuation(&clocks)?;
        if !local.invariants.iter().all(|bound| bound.holds(&valuation)) {
            return Ok(None);
        }
        let deadline = match (state.deadline, self.deadline_bound()?) {
            (Some(t), Some(bound)) => Some((t + 1).min(bound + 1)),
            _ => None,
        };
        Ok(Some((clocks, deadline)))
    }

    /// The scheduled identities enabled at `clocks` with their enabled
    /// post-states (ADR-026 TS-2 with integer clocks).
    fn enabled(&mut self, local: &Local, model: &ModelSimState, clocks: &[u64]) -> Step<Vec<Enabled>> {
        let valuation = self.valuation(clocks)?;
        let mut found = Vec::new();
        for action in &local.actions {
            let mut draws = Vec::with_capacity(action.draws.len());
            for draw in &action.draws {
                let mut posts = Vec::new();
                for (transition, post) in &draw.successors {
                    let Some(step) = local.steps.iter().find(|step| step.transition() == transition)
                    else {
                        continue;
                    };
                    if !step
                        .parts()
                        .iter()
                        .any(|part| part.iter().all(|constraint| constraint.holds(&valuation)))
                    {
                        continue;
                    }
                    let reset = step.reset(&valuation);
                    let after = self.local(post)?;
                    if !after.invariants.iter().all(|bound| bound.holds(&reset)) {
                        continue;
                    }
                    posts.push((post.clone(), self.digital(&reset)?));
                }
                draws.push(EnabledDraw {
                    random: draw.random.clone(),
                    probability: draw.probability.clone(),
                    rewards: draw.rewards.clone(),
                    posts,
                });
            }
            if draws.iter().all(|draw| draw.posts.is_empty()) {
                continue;
            }
            if let Some(blocked) = draws.iter().find(|draw| draw.posts.is_empty()) {
                let (_, state) = self
                    .system
                    .state_key(model)
                    .map_err(|_| fault("state-key-encodes"))?;
                return Err(ProductRefusal::NotMarkov(Box::new(NotMarkov {
                    state,
                    transition: action.identity.clone(),
                    random: blocked.random.clone(),
                    post_states: 0,
                }))
                .into());
            }
            found.push(Enabled {
                identity: action.identity.clone(),
                draws,
            });
        }
        Ok(found)
    }

    // -- keys and truths ---------------------------------------------------

    fn encode(&self, state: &DState) -> Step<Vec<u8>> {
        let (model, _) = self
            .system
            .state_key(&state.model)
            .map_err(|_| fault("state-key-encodes"))?;
        let mut bytes = b"quire.simulation.state-key/v1/digital\n".to_vec();
        bytes.extend((model.len() as u64).to_be_bytes());
        bytes.extend(model);
        bytes.extend(self.scale.to_be_bytes());
        bytes.extend((state.clocks.len() as u64).to_be_bytes());
        for (value, cap) in state.clocks.iter().zip(&self.caps) {
            bytes.extend(value.to_be_bytes());
            bytes.extend(cap.to_be_bytes());
        }
        match state.deadline {
            Some(deadline) => {
                bytes.push(1);
                bytes.extend(deadline.to_be_bytes());
            }
            None => bytes.push(0),
        }
        bytes.extend(state.tail.tag().as_bytes());
        Ok(bytes)
    }

    fn key_of(
        &self,
        state: &DState,
        binding: usize,
        monitor: Option<MonitorState>,
        pending: Option<Pending>,
    ) -> Step<ProbKey> {
        Ok(ProbKey {
            binding: self.bindings[binding].0.clone(),
            model: self.encode(state)?,
            monitor,
            accumulator: None,
            pending,
        })
    }

    fn truths(&mut self, state: &ModelSimState, binding: usize) -> Step<Result<Vec<bool>, Hit>> {
        let (model, state_key) = self
            .system
            .state_key(state)
            .map_err(|_| fault("state-key-encodes"))?;
        if let Some(known) = self.readings.get(&(binding, model.clone())) {
            return Ok(Ok(known.clone()));
        }
        let over = self.bindings[binding].1.as_ref();
        let predicates: Vec<(&CheckedPredicate, Option<&ObjectReference>)> = self
            .form
            .reads()
            .iter()
            .map(|(_, predicate)| (*predicate, over))
            .collect();
        let readings = self
            .system
            .state_predicates(state, &predicates)
            .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop)))?;
        let mut truths = Vec::with_capacity(readings.len());
        for ((read, _), reading) in self.form.reads().iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Undefined { cause, .. } => {
                    return Ok(Err(Hit {
                        read: *read,
                        cause,
                        state_key,
                        model,
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

    fn binding_of(&self, at: usize) -> Step<usize> {
        self.bindings
            .iter()
            .position(|(instance, _)| *instance == self.nodes[at].key.binding)
            .ok_or_else(|| fault("node-binding-is-a-binding").into())
    }

    // -- the deadline event --------------------------------------------------

    /// The event's phase after reading one letter.
    fn read_letter(kind: DeadlineKind, phase: DeadlinePhase, truths: &[bool]) -> DeadlinePhase {
        if phase != DeadlinePhase::Pending {
            return phase;
        }
        match (kind, truths) {
            (DeadlineKind::Eventually, [target]) if *target => DeadlinePhase::Accept,
            (DeadlineKind::Until, [from, until]) => {
                if *until {
                    DeadlinePhase::Accept
                } else if !*from {
                    DeadlinePhase::Reject
                } else {
                    DeadlinePhase::Pending
                }
            }
            _ => DeadlinePhase::Pending,
        }
    }

    fn deadline_start(&self, truths: &[bool]) -> Option<MonitorState> {
        match &self.form {
            Form::Deadline { kind, .. } => Some(MonitorState::Deadline {
                kind: *kind,
                phase: Self::read_letter(*kind, DeadlinePhase::Pending, truths),
            }),
            Form::Labels(_) => None,
        }
    }

    /// The control after the source's delay and the letters of `posts` (the
    /// discrete steps that follow the delay, none for a pure delay).
    fn control_after(
        &mut self,
        source: Option<MonitorState>,
        reached: &DState,
        letters: &[Vec<bool>],
    ) -> Step<Option<MonitorState>> {
        let Some(MonitorState::Deadline { kind, phase }) = source else {
            return Ok(None);
        };
        let bound = self.deadline_bound()?;
        let mut phase = phase;
        if phase == DeadlinePhase::Pending {
            if let (Some(t), Some(bound)) = (reached.deadline, bound) {
                if t > bound {
                    phase = DeadlinePhase::Reject;
                }
            }
        }
        for truths in letters {
            phase = Self::read_letter(kind, phase, truths);
        }
        Ok(Some(MonitorState::Deadline { kind, phase }))
    }

    // -- exploration ---------------------------------------------------------

    fn explore(&mut self) -> Step<Vec<(u32, Option<ModelInstance>, ProbStateId)>> {
        self.poll()?;
        let deadline = self.deadline_bound()?.map(|_| 0);
        let mut starts = Vec::new();
        for (position, initial) in self.timed.initial().into_iter().enumerate() {
            let at = u32::try_from(position).map_err(|_| fault("initial-states-count"))?;
            let state = DState {
                model: initial.discrete().clone(),
                clocks: self.digital(initial.clocks())?,
                deadline,
                tail: Tail::Running,
            };
            for binding in 0..self.bindings.len() {
                let truths = match self.truths(&state.model, binding)? {
                    Ok(truths) => truths,
                    Err(hit) => {
                        let monitor = match &self.form {
                            Form::Deadline { kind, .. } => Some(MonitorState::Deadline {
                                kind: *kind,
                                phase: DeadlinePhase::Pending,
                            }),
                            Form::Labels(_) => None,
                        };
                        let key = self.key_of(&state, binding, monitor, None)?;
                        return Err(self.undefined(None, None, None, at, key, hit));
                    }
                };
                let monitor = self.deadline_start(&truths);
                let key = self.key_of(&state, binding, monitor, None)?;
                let target = Target {
                    key,
                    state: state.clone(),
                    truths,
                    resolution: None,
                };
                let id = self.intern(target, None, None, at)?;
                starts.push((at, self.bindings[binding].0.clone(), ProbStateId(id)));
            }
        }
        while let Some(at) = self.queue.pop_front() {
            self.poll()?;
            self.expand(at)?;
        }
        Ok(starts)
    }

    fn intern(
        &mut self,
        target: Target,
        parent: Option<usize>,
        step: Option<TraceStep>,
        initial: u32,
    ) -> Step<usize> {
        if let Some(known) = self.index.get(&target.key) {
            return Ok(*known);
        }
        if self.nodes.len() >= self.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::ExactProbMaxStates,
                self.max_states,
            ));
        }
        let id = self.nodes.len();
        let initial = parent
            .map(|parent| self.nodes[parent].initial)
            .unwrap_or(initial);
        let decided = target.key.monitor.is_some_and(MonitorState::is_decided);
        self.index.insert(target.key.clone(), id);
        self.nodes.push(Node {
            key: target.key,
            state: target.state,
            truths: target.truths,
            parent: parent.map(|parent| (parent, step)),
            initial,
            resolution: target.resolution,
        });
        if !decided {
            self.queue.push_back(id);
        }
        Ok(id)
    }

    fn undefined(
        &self,
        parent: Option<usize>,
        step: Option<&TraceStep>,
        resolved: Option<ProbKey>,
        initial: u32,
        key: ProbKey,
        hit: Hit,
    ) -> Halt {
        let mut steps: Vec<PathStep> = step
            .map(|step| PathStep {
                action: step.action.clone(),
                draw: step.draw.clone(),
                probability: step.probability.clone(),
                target: Box::new(key.clone()),
                resolved: None,
            })
            .into_iter()
            .collect();
        let mut chosen = resolved;
        let mut cursor = parent;
        let mut initial = initial;
        while let Some(at) = cursor {
            initial = self.nodes[at].initial;
            let Some((before, taken)) = &self.nodes[at].parent else {
                break;
            };
            match taken {
                Some(taken) => {
                    let resolution = chosen.take();
                    steps.push(PathStep {
                        action: taken.action.clone(),
                        draw: taken.draw.clone(),
                        probability: taken.probability.clone(),
                        target: Box::new(
                            resolution
                                .clone()
                                .unwrap_or_else(|| self.nodes[at].key.clone()),
                        ),
                        resolved: resolution.map(Box::new),
                    });
                }
                None => chosen = Some(self.nodes[at].key.clone()),
            }
            cursor = Some(*before);
        }
        steps.reverse();
        let probability = steps
            .iter()
            .fold(one(), |total, step| total.mul(&step.probability));
        Halt::Undefined(Box::new(UndefinedPath {
            initial,
            binding: key.binding.clone(),
            position: u64::try_from(steps.len()).unwrap_or(u64::MAX),
            steps,
            probability,
            state_key: hit.state_key,
            key: Box::new(key),
            read: hit.read,
            cause: hit.cause,
        }))
    }

    /// The target reached from node `at` by `reached` after the letters of
    /// the discrete steps listed in `landed` (model states, in order).
    fn arrive(
        &mut self,
        at: usize,
        reached: DState,
        landed: &[ModelSimState],
        taken: Option<&TraceStep>,
    ) -> Step<Target> {
        let binding = self.binding_of(at)?;
        let mut letters = Vec::with_capacity(landed.len());
        for model in landed {
            match self.truths(model, binding)? {
                Ok(truths) => letters.push(truths),
                Err(hit) => {
                    let source = self.nodes[at].key.monitor;
                    let key = self.key_of(&reached, binding, source, None)?;
                    let resolved = taken.is_none().then(|| key.clone());
                    return Err(self.undefined(
                        Some(at),
                        taken,
                        resolved,
                        self.nodes[at].initial,
                        key,
                        hit,
                    ));
                }
            }
        }
        let truths = match letters.last() {
            Some(last) => last.clone(),
            None => match self.truths(&reached.model, binding)? {
                Ok(truths) => truths,
                Err(_) => self.nodes[at].truths.clone(),
            },
        };
        let source = self.nodes[at].key.monitor;
        let monitor = self.control_after(source, &reached, &letters)?;
        let key = self.key_of(&reached, binding, monitor, None)?;
        Ok(Target {
            key,
            state: reached,
            truths,
            resolution: None,
        })
    }

    fn expand(&mut self, at: usize) -> Step<()> {
        let planned = match self.nodes[at].resolution.take() {
            Some(resolution) => self.resolve(at, &resolution)?,
            None => self.moves(at)?,
        };
        for edge in planned {
            if self.edges.len() >= self.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::ExactProbMaxTransitions,
                    self.max_transitions,
                ));
            }
            let zero_delay = edge.zero_delay;
            let to = self.intern(edge.target, Some(at), edge.step, self.nodes[at].initial)?;
            self.edges.push(ProbEdge {
                from: ProbStateId(at),
                choice: edge.choice,
                action: edge.action,
                draw: edge.draw,
                to: ProbStateId(to),
                probability: edge.probability,
                rewards: edge.rewards,
            });
            self.zero_delay.push(zero_delay);
        }
        Ok(())
    }

    fn resolve(&mut self, at: usize, resolution: &Resolution) -> Step<Vec<Planned>> {
        let mut planned = Vec::with_capacity(resolution.successors.len());
        for post in &resolution.successors {
            let model = post.model.clone();
            let target = self.arrive(at, post.clone(), &[model], None)?;
            planned.push(Planned {
                choice: ProductAction::PostState(Box::new(target.key.clone())),
                action: None,
                draw: None,
                target,
                probability: one(),
                rewards: self.system.zero_rewards(),
                step: None,
                zero_delay: true,
            });
        }
        Ok(planned)
    }

    fn moves(&mut self, at: usize) -> Step<Vec<Planned>> {
        let state = self.nodes[at].state.clone();
        let local = self.local(&state.model)?;
        match state.tail {
            Tail::Running => match self.workload {
                None => self.scheduler_moves(at, &state, &local),
                Some(workload) => self.workload_moves(at, &state, &local, workload),
            },
            Tail::IdleReady | Tail::IdleDelayed => self.idle_moves(at, &state),
        }
    }

    /// A pure delay move to `clocks`/`deadline` with the tail `tail`.
    fn delay_move(
        &mut self,
        at: usize,
        state: &DState,
        choice: ProductAction,
        delayed: (Vec<u64>, Option<u64>),
        tail: Tail,
    ) -> Step<Planned> {
        let reached = DState {
            model: state.model.clone(),
            clocks: delayed.0,
            deadline: delayed.1,
            tail,
        };
        let taken = TraceStep {
            action: None,
            draw: None,
            probability: one(),
        };
        let target = self.arrive(at, reached, &[], Some(&taken))?;
        Ok(Planned {
            choice,
            action: None,
            draw: None,
            target,
            probability: one(),
            rewards: self.system.zero_rewards(),
            step: Some(taken),
            zero_delay: false,
        })
    }

    /// The delay and observation moves every running state offers.
    fn running_delays(
        &mut self,
        at: usize,
        state: &DState,
        local: &Local,
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        if let Some(delayed) = self.unit_delay(local, state)? {
            planned.push(self.delay_move(
                at,
                state,
                ProductAction::UnitDelay,
                delayed.clone(),
                Tail::Running,
            )?);
            if local.quiescent {
                planned.push(self.delay_move(
                    at,
                    state,
                    ProductAction::EnterIdleDelay,
                    delayed,
                    Tail::IdleDelayed,
                )?);
            }
        }
        Ok(())
    }

    fn idle_moves(&mut self, at: usize, state: &DState) -> Step<Vec<Planned>> {
        let local = self.local(&state.model)?;
        let mut planned = Vec::new();
        if let Some(delayed) = self.unit_delay(&local, state)? {
            planned.push(self.delay_move(
                at,
                state,
                ProductAction::IdleDelay,
                delayed,
                Tail::IdleDelayed,
            )?);
        }
        if state.tail == Tail::IdleDelayed {
            let reached = DState {
                tail: Tail::IdleReady,
                ..state.clone()
            };
            let taken = TraceStep {
                action: None,
                draw: None,
                probability: one(),
            };
            let landed = [state.model.clone()];
            let target = self.arrive(at, reached, &landed, Some(&taken))?;
            planned.push(Planned {
                choice: ProductAction::IdleObserve,
                action: None,
                draw: None,
                target,
                probability: one(),
                rewards: self.system.zero_rewards(),
                step: Some(taken),
                zero_delay: false,
            });
        }
        self.decisions = true;
        Ok(planned)
    }

    /// Over every scheduler: the delays and each enabled identity's draws.
    fn scheduler_moves(&mut self, at: usize, state: &DState, local: &Rc<Local>) -> Step<Vec<Planned>> {
        let mut planned = Vec::new();
        self.running_delays(at, state, local, &mut planned)?;
        let enabled = self.enabled(local, &state.model, &state.clocks)?;
        for action in enabled {
            self.identity_edges(at, state, &action, one(), &mut planned)?;
        }
        if planned.is_empty() {
            self.time_locked.push(at);
        }
        self.decisions = true;
        Ok(planned)
    }

    /// The edges of one enabled identity at `state`, scaled by `share`.
    fn identity_edges(
        &mut self,
        at: usize,
        state: &DState,
        action: &Enabled,
        share: Rational,
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        let first = planned.len();
        for draw in &action.draws {
            let taken = TraceStep {
                action: Some(action.identity.clone()),
                draw: Some(draw.random.clone()),
                probability: share.mul(&draw.probability),
            };
            let probability = share.mul(&draw.probability);
            let post = |model: &ModelSimState, clocks: &[u64]| DState {
                model: model.clone(),
                clocks: clocks.to_vec(),
                deadline: state.deadline,
                tail: Tail::Running,
            };
            if let [(model, clocks)] = draw.posts.as_slice() {
                let reached = post(model, clocks);
                let target = self.arrive(at, reached, std::slice::from_ref(model), Some(&taken))?;
                if let Some(merged) = planned[first..].iter_mut().find(|edge| {
                    edge.target.key == target.key && same_rewards(&edge.rewards, &draw.rewards)
                }) {
                    merged.probability = merged.probability.add(&probability);
                    merged.draw = None;
                    merged.step = Some(TraceStep {
                        action: taken.action,
                        draw: None,
                        probability: merged.probability.clone(),
                    });
                    continue;
                }
                planned.push(Planned {
                    choice: ProductAction::Identity(action.identity.clone()),
                    action: Some(action.identity.clone()),
                    draw: Some(draw.random.clone()),
                    target,
                    probability,
                    rewards: draw.rewards.clone(),
                    step: Some(taken),
                    zero_delay: true,
                });
                continue;
            }
            // Several post-states: the scheduler resolves them at an
            // intermediate state keyed by the draw (SCH-2).
            let identity_bytes = self
                .system
                .transition_key(
                    &self
                        .local(&state.model)?
                        .actions
                        .iter()
                        .find(|candidate| candidate.identity == action.identity)
                        .and_then(|candidate| candidate.draws.first())
                        .and_then(|draw| draw.successors.first())
                        .map(|(transition, _)| transition.clone())
                        .ok_or_else(|| fault("pending-draw-has-a-transition"))?,
                )
                .map_err(|_| fault("transition-key-encodes"))?;
            let binding = self.binding_of(at)?;
            let monitor = self.nodes[at].key.monitor;
            let key = self.key_of(
                state,
                binding,
                monitor,
                Some(Pending {
                    identity: action.identity.clone(),
                    random: draw.random.clone(),
                    identity_bytes,
                }),
            )?;
            let target = Target {
                key,
                state: state.clone(),
                truths: self.nodes[at].truths.clone(),
                resolution: Some(Resolution {
                    successors: draw
                        .posts
                        .iter()
                        .map(|(model, clocks)| post(model, clocks))
                        .collect(),
                    rewards: draw.rewards.clone(),
                }),
            };
            planned.push(Planned {
                choice: ProductAction::Identity(action.identity.clone()),
                action: Some(action.identity.clone()),
                draw: Some(draw.random.clone()),
                target,
                probability,
                rewards: draw.rewards.clone(),
                step: Some(taken),
                zero_delay: true,
            });
        }
        Ok(())
    }

    // -- under a workload ----------------------------------------------------

    fn weight_of(&self, workload: &Workload, operation: &str) -> Step<Rational> {
        workload
            .weights()
            .iter()
            .find(|(weighted, _)| {
                self.system.operation_text(weighted.declaring(), weighted.name()) == Some(operation)
            })
            .map(|(_, weight)| weight.clone())
            .ok_or_else(|| fault("workload-weighs-an-operation").into())
    }

    /// The delay distribution of `operation`, when it has a `discrete` one.
    fn given(&self, operation: &str) -> Option<Vec<(Rational, Rational)>> {
        self.system
            .package()
            .graph()
            .probabilistic_model()
            .delays()
            .iter()
            .find(|delay| {
                let named = delay.operation();
                self.system.operation_text(named.declaring(), named.name()) == Some(operation)
            })
            .and_then(|delay| match delay.distribution() {
                DelayDistribution::Discrete(values) => Some(values.clone()),
                _ => None,
            })
    }

    fn trajectory(&mut self, state: &DState) -> Step<Trajectory> {
        let mut states = vec![state.clone()];
        let mut forever = false;
        let bound = self.max_states.max(1);
        while states.len() <= bound {
            let last = states.last().cloned().ok_or_else(|| fault("trajectory-starts"))?;
            let local = self.local(&last.model)?;
            let Some((clocks, deadline)) = self.unit_delay(&local, &last)? else {
                break;
            };
            if clocks == last.clocks && deadline == last.deadline {
                forever = true;
                break;
            }
            states.push(DState {
                clocks,
                deadline,
                ..last
            });
        }
        Ok(Trajectory { states, forever })
    }

    fn windows(&mut self, trajectory: &Trajectory) -> Step<Vec<Window>> {
        let mut windows: Vec<Window> = Vec::new();
        for (k, state) in trajectory.states.iter().enumerate() {
            let local = self.local(&state.model)?;
            for action in self.enabled(&local, &state.model, &state.clocks)? {
                let delay = u64::try_from(k).map_err(|_| fault("delay-count"))?;
                match windows.iter_mut().find(|w| w.identity == action.identity) {
                    Some(window) => {
                        window.delays.insert(delay);
                    }
                    None => {
                        let given = self.given(&action.identity.operation).map(|values| {
                            values
                                .into_iter()
                                .map(|(delay, weight)| (delay, weight))
                                .collect::<Vec<_>>()
                        });
                        let given = match given {
                            Some(values) => Some(
                                values
                                    .into_iter()
                                    .map(|(delay, weight)| Ok((self.bound_units(&delay)?, weight)))
                                    .collect::<Step<Vec<_>>>()?,
                            ),
                            None => None,
                        };
                        windows.push(Window {
                            identity: action.identity.clone(),
                            delays: BTreeSet::from([delay]),
                            forever: false,
                            given,
                        });
                    }
                }
            }
        }
        if trajectory.forever {
            if let Some(last) = trajectory.states.len().checked_sub(1) {
                let last = u64::try_from(last).map_err(|_| fault("delay-count"))?;
                for window in &mut windows {
                    window.forever = window.delays.contains(&last);
                }
            }
        }
        Ok(windows)
    }

    fn workload_moves(
        &mut self,
        at: usize,
        state: &DState,
        local: &Rc<Local>,
        workload: &Workload,
    ) -> Step<Vec<Planned>> {
        let trajectory = self.trajectory(state)?;
        let windows = self.windows(&trajectory)?;
        let reached = u64::try_from(trajectory.states.len().saturating_sub(1))
            .map_err(|_| fault("delay-count"))?;
        let free = windows.iter().any(|window| window.given.is_none());
        let given = windows.iter().any(|window| window.given.is_some());
        if free && given {
            return Err(ProductRefusal::MixedDelays {
                state: Box::new(self.nodes[at].key.clone()),
            }
            .into());
        }
        if free {
            return self.free_moves(at, state, local, workload);
        }
        // Every racing identity has a given delay: draw each, conditioned on
        // its window, and take the smallest (ADR-026 SD-3).
        let mut racers: Vec<(&Window, Vec<(u64, Rational)>)> = Vec::new();
        for window in &windows {
            let Some(support) = &window.given else {
                continue;
            };
            let inside: Vec<(u64, Rational)> = support
                .iter()
                .filter(|(delay, _)| window.contains(*delay, reached))
                .cloned()
                .collect();
            let total = inside
                .iter()
                .fold(zero(), |sum, (_, weight)| sum.add(weight));
            if inside.is_empty() {
                continue;
            }
            let conditioned = inside
                .into_iter()
                .map(|(delay, weight)| {
                    let share = weight
                        .div(&total)
                        .ok_or_else(|| Halt::from(fault("conditioned-weights-sum")))?;
                    Ok((delay, share))
                })
                .collect::<Step<Vec<_>>>()?;
            racers.push((window, conditioned));
        }
        if racers.is_empty() {
            let mut planned = Vec::new();
            self.running_delays(at, state, local, &mut planned)?;
            if planned.is_empty() {
                self.time_locked.push(at);
            } else if !local.quiescent {
                return Err(ProductRefusal::NotStochastic {
                    state: Box::new(self.nodes[at].key.clone()),
                }
                .into());
            }
            self.decisions = true;
            return Ok(planned);
        }
        let mut outcomes: Vec<(u64, Vec<usize>, Rational)> = Vec::new();
        let mut draw = vec![0_usize; racers.len()];
        loop {
            let mut probability = one();
            let mut least = u64::MAX;
            for (index, (_, support)) in racers.iter().enumerate() {
                let (delay, share) = &support[draw[index]];
                probability = probability.mul(share);
                least = least.min(*delay);
            }
            let tied: Vec<usize> = racers
                .iter()
                .enumerate()
                .filter(|(index, (_, support))| support[draw[*index]].0 == least)
                .map(|(index, _)| index)
                .collect();
            outcomes.push((least, tied, probability));
            let mut position = racers.len();
            loop {
                if position == 0 {
                    break;
                }
                position -= 1;
                draw[position] += 1;
                if draw[position] < racers[position].1.len() {
                    position = usize::MAX;
                    break;
                }
                draw[position] = 0;
            }
            if position != usize::MAX {
                break;
            }
        }
        let representative = racers
            .first()
            .map(|(window, _)| window.identity.clone())
            .ok_or_else(|| fault("a-race-has-a-racer"))?;
        let mut planned: Vec<Planned> = Vec::new();
        for (delay, tied, probability) in outcomes {
            let index = usize::try_from(delay).map_err(|_| fault("delay-count"))?;
            let start = trajectory
                .states
                .get(index.min(trajectory.states.len() - 1))
                .ok_or_else(|| fault("race-delay-is-reachable"))?
                .clone();
            let identities: Vec<ScheduledIdentity> = tied
                .iter()
                .map(|racer| racers[*racer].0.identity.clone())
                .collect();
            for (order, ordered) in self.orderings(workload, &identities)? {
                self.chain_edges(
                    at,
                    &start,
                    &ordered,
                    (probability.mul(&order), delay == 0, &representative),
                    &mut planned,
                )?;
            }
        }
        Ok(planned)
    }

    /// Every ordering of the tied identities with its probability: drawing
    /// without replacement by the workload's operation weights, and
    /// uniformly within an operation (ADR-026 SD-3).
    fn orderings(
        &self,
        workload: &Workload,
        tied: &[ScheduledIdentity],
    ) -> Step<Vec<(Rational, Vec<ScheduledIdentity>)>> {
        if tied.is_empty() {
            return Ok(vec![(one(), Vec::new())]);
        }
        let mut operations: Vec<(String, Rational)> = Vec::new();
        for identity in tied {
            if !operations.iter().any(|(name, _)| *name == identity.operation) {
                operations.push((
                    identity.operation.clone(),
                    self.weight_of(workload, &identity.operation)?,
                ));
            }
        }
        let total = operations
            .iter()
            .fold(zero(), |sum, (_, weight)| sum.add(weight));
        let mut found = Vec::new();
        for (name, weight) in &operations {
            let same: Vec<usize> = tied
                .iter()
                .enumerate()
                .filter(|(_, identity)| identity.operation == *name)
                .map(|(index, _)| index)
                .collect();
            let operation_share = weight
                .div(&total)
                .ok_or_else(|| fault("workload-weights-sum"))?;
            let each = operation_share
                .div(&Rational::from_integer(Integer::from_big(BigInt::from(
                    same.len() as u64,
                ))))
                .ok_or_else(|| fault("operation-has-identities"))?;
            for index in same {
                let rest: Vec<ScheduledIdentity> = tied
                    .iter()
                    .enumerate()
                    .filter(|(position, _)| *position != index)
                    .map(|(_, identity)| identity.clone())
                    .collect();
                for (probability, mut tail) in self.orderings(workload, &rest)? {
                    let mut ordered = vec![tied[index].clone()];
                    ordered.append(&mut tail);
                    found.push((each.mul(&probability), ordered));
                }
            }
        }
        Ok(found)
    }

    /// The edges of one race outcome: the state after the winning delay, then
    /// each tied identity in order while it is still enabled.
    fn chain_edges(
        &mut self,
        at: usize,
        start: &DState,
        ordered: &[ScheduledIdentity],
        outcome: (Rational, bool, &ScheduledIdentity),
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        let (probability, zero_delay, representative) = outcome;
        // (probability, states reached after each step, first draw, first rewards)
        type Chain = (Rational, Vec<DState>, Option<Vec<Value>>, Vec<(Identifier, Value)>);
        let mut chains: Vec<Chain> = vec![(
            probability,
            vec![start.clone()],
            None,
            self.system.zero_rewards(),
        )];
        for (position, identity) in ordered.iter().enumerate() {
            let mut next: Vec<Chain> = Vec::new();
            for (chain_probability, states, first_draw, first_rewards) in chains {
                let current = states.last().cloned().ok_or_else(|| fault("chain-has-a-state"))?;
                let local = self.local(&current.model)?;
                let available = self.enabled(&local, &current.model, &current.clocks)?;
                let Some(action) = available.iter().find(|candidate| candidate.identity == *identity)
                else {
                    next.push((chain_probability, states, first_draw, first_rewards));
                    continue;
                };
                for draw in &action.draws {
                    let [(model, clocks)] = draw.posts.as_slice() else {
                        let (_, state) = self
                            .system
                            .state_key(&current.model)
                            .map_err(|_| fault("state-key-encodes"))?;
                        return Err(ProductRefusal::NotMarkov(Box::new(NotMarkov {
                            state,
                            transition: identity.clone(),
                            random: draw.random.clone(),
                            post_states: u32::try_from(draw.posts.len()).unwrap_or(u32::MAX),
                        }))
                        .into());
                    };
                    let mut reached = states.clone();
                    reached.push(DState {
                        model: model.clone(),
                        clocks: clocks.clone(),
                        deadline: current.deadline,
                        tail: Tail::Running,
                    });
                    next.push((
                        chain_probability.mul(&draw.probability),
                        reached,
                        first_draw.clone().or_else(|| Some(draw.random.clone())),
                        if position == 0 || first_draw.is_none() {
                            draw.rewards.clone()
                        } else {
                            first_rewards.clone()
                        },
                    ));
                }
            }
            chains = next;
        }
        let first = ordered.first().cloned().ok_or_else(|| fault("race-takes-an-identity"))?;
        for (chain_probability, states, draw, rewards) in chains {
            let reached = states.last().cloned().ok_or_else(|| fault("chain-has-a-state"))?;
            let landed: Vec<ModelSimState> =
                states.iter().skip(1).map(|state| state.model.clone()).collect();
            let taken = TraceStep {
                action: Some(first.clone()),
                draw: draw.clone(),
                probability: chain_probability.clone(),
            };
            let target = self.arrive(at, reached, &landed, Some(&taken))?;
            if let Some(merged) = planned.iter_mut().find(|edge| {
                edge.target.key == target.key
                    && edge.action.as_ref() == Some(&first)
                    && same_rewards(&edge.rewards, &rewards)
            }) {
                merged.probability = merged.probability.add(&chain_probability);
                merged.draw = None;
                continue;
            }
            planned.push(Planned {
                choice: ProductAction::Identity(representative.clone()),
                action: Some(first.clone()),
                draw,
                target,
                probability: chain_probability,
                rewards,
                step: Some(taken),
                zero_delay,
            });
        }
        Ok(())
    }

    /// Under a workload whose racing identities all have free delays: the
    /// scheduler chooses between a unit delay and letting the workload resolve
    /// the identity now (ADR-028 TA-1, RU-6).
    fn free_moves(
        &mut self,
        at: usize,
        state: &DState,
        local: &Rc<Local>,
        workload: &Workload,
    ) -> Step<Vec<Planned>> {
        let mut planned = Vec::new();
        self.running_delays(at, state, local, &mut planned)?;
        let enabled = self.enabled(local, &state.model, &state.clocks)?;
        let mut operations: BTreeMap<String, (Rational, u64)> = BTreeMap::new();
        for action in &enabled {
            let operation = action.identity.operation.clone();
            if !operations.contains_key(&operation) {
                let weight = self.weight_of(workload, &operation)?;
                operations.insert(operation.clone(), (weight, 0));
            }
            if let Some(entry) = operations.get_mut(&operation) {
                entry.1 += 1;
            }
        }
        let total = operations
            .values()
            .fold(zero(), |sum, (weight, _)| sum.add(weight));
        let representative = enabled.first().map(|action| action.identity.clone());
        for action in &enabled {
            let (weight, identities) = operations
                .get(action.identity.operation.as_str())
                .ok_or_else(|| fault("operation-is-counted"))?;
            let share = weight
                .div(&total)
                .and_then(|share| {
                    share.div(&Rational::from_integer(Integer::from_big(BigInt::from(*identities))))
                })
                .ok_or_else(|| fault("workload-share"))?;
            let first = planned.len();
            self.identity_edges(at, state, action, share, &mut planned)?;
            if let Some(representative) = &representative {
                for edge in &mut planned[first..] {
                    edge.choice = ProductAction::Identity(representative.clone());
                }
            }
        }
        if planned.is_empty() {
            self.time_locked.push(at);
        }
        self.decisions = true;
        Ok(planned)
    }

    // -- the finished product --------------------------------------------------

    /// FR-204 TA-5: a reachable cycle of discrete moves is Zeno.
    fn zero_delay_cycle(&self) -> Option<Vec<usize>> {
        let count = self.nodes.len();
        let mut successors: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (edge, zero) in self.edges.iter().zip(&self.zero_delay) {
            if *zero {
                successors[edge.from.index()].push(edge.to.index());
            }
        }
        // Iterative Tarjan.
        let mut index_of = vec![usize::MAX; count];
        let mut low = vec![0_usize; count];
        let mut on_stack = vec![false; count];
        let mut stack: Vec<usize> = Vec::new();
        let mut next = 0_usize;
        for root in 0..count {
            if index_of[root] != usize::MAX {
                continue;
            }
            let mut work: Vec<(usize, usize)> = vec![(root, 0)];
            while let Some((node, cursor)) = work.pop() {
                if cursor == 0 {
                    index_of[node] = next;
                    low[node] = next;
                    next += 1;
                    stack.push(node);
                    on_stack[node] = true;
                }
                if let Some(&to) = successors[node].get(cursor) {
                    work.push((node, cursor + 1));
                    if index_of[to] == usize::MAX {
                        work.push((to, 0));
                    } else if on_stack[to] {
                        low[node] = low[node].min(index_of[to]);
                    }
                    continue;
                }
                if low[node] == index_of[node] {
                    let mut component = Vec::new();
                    while let Some(member) = stack.pop() {
                        on_stack[member] = false;
                        component.push(member);
                        if member == node {
                            break;
                        }
                    }
                    let cyclic = component.len() > 1
                        || component
                            .first()
                            .is_some_and(|member| successors[*member].contains(member));
                    if cyclic {
                        component.sort_unstable();
                        return Some(component);
                    }
                }
                if let Some((parent, _)) = work.last() {
                    low[*parent] = low[*parent].min(low[node]);
                }
            }
        }
        None
    }

    fn finish(
        self,
        initial: Vec<(u32, Option<ModelInstance>, ProbStateId)>,
    ) -> Result<ProbProduct, ProductRefusal> {
        if let Some(cycle) = self.zero_delay_cycle() {
            return Err(ProductRefusal::ZeroDelayCycle {
                states: cycle
                    .into_iter()
                    .map(|member| self.nodes[member].key.clone())
                    .collect(),
            });
        }
        let mut decided: Vec<(ProbStateId, MonitorVerdict)> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| match node.key.monitor {
                Some(MonitorState::Deadline { phase, .. }) => match phase {
                    DeadlinePhase::Accept => Some((ProbStateId(index), MonitorVerdict::Accept)),
                    DeadlinePhase::Reject => Some((ProbStateId(index), MonitorVerdict::Reject)),
                    DeadlinePhase::Pending => None,
                },
                _ => None,
            })
            .collect();
        // A pending event at a state with no move at all (a time-lock) can
        // never be met: ADR-026 TD-2 reports the lock, and the event closes
        // as a rejection (FR-196 "Canonical probability product key").
        for state in &self.time_locked {
            if matches!(self.form, Form::Deadline { .. }) {
                decided.push((ProbStateId(*state), MonitorVerdict::Reject));
            }
        }
        decided.sort_by_key(|(state, _)| state.index());
        decided.dedup_by_key(|(state, _)| state.index());
        let labels = if matches!(self.form, Form::Labels(_)) {
            self.nodes.iter().map(|node| node.truths.clone()).collect()
        } else {
            Vec::new()
        };
        Ok(ProbProduct {
            states: self.nodes.into_iter().map(|node| node.key).collect(),
            kind: if self.decisions || self.workload.is_none() {
                ProductKind::Mdp
            } else {
                ProductKind::Dtmc
            },
            edges: self.edges,
            initial,
            decided,
            labels,
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        })
    }
}
