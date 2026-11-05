// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202's witness production on a built product (EN-5's side).
//!
//! From an evaluator's attaining policy, [`produce_witness`] emits a path
//! set, most probable path first, whose exact probabilities sum past a bound,
//! with the action the policy takes at each state the paths pass through.
//! The policy already applies FR-198's witness rules (the MEC exit rule of a
//! maximum, then canonical transition order), so this module reads it back:
//! at a `FreeDelays` state the whole action, one row choice per row
//! ([`super::free_delays_choice`]).
//!
//! A step through a `FreeDelays` action is a merged step (FR-196, FR-202): it
//! names the action and the post-state, carries no given vector or draw, and
//! its probability is the sum over the action's rows of the row's
//! given-vector probability times the mass the row's race gives that
//! post-state with those rewards. The product folds `P(g)` into each
//! row-choice edge, so the merged probability is the sum of the matching
//! edges of the action's row choices.
//!
//! A witness scheduler has an entry at every state the induced chain reaches
//! from the initial state before it accepts (FR-204 "Evidence": every
//! digital evidence and continuation state), not only at the states the
//! emitted paths leave. [`witness_scheduler`] reads a randomized policy the
//! same way and names an exact-rational randomized entry ([`EntryChoice`]).
//!
//! Entries are keyed by product state; the canonical FR-202 scheduler keys
//! each by its complete canonical product key, which the checker encodes.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::rc::Rc;

use qsl_eval::simulation::model::same_rewards;
use quire_exact::{Identifier, Integer, Rational, Value};

use super::finite_horizon::rational_fits;
use super::rows::{free_delays_choice, free_delays_distribution, scheduler_distribution};
use super::unbounded::same_action;
use super::{
    ExactProbLimits, ProbProduct, ProbStateId, ProductAction, ProductKind, SchedulerChoice,
};

/// Which choices a witness scheduler resolves (FR-202, FR-204 "Evidence").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessScope {
    /// A claim `under every scheduler`: every choice.
    EveryScheduler,
    /// A claim under a workload: only the free-delay and observation
    /// choices; fixed race weights and distributions stay the workload's.
    Workload,
}

/// One step of a produced path.
#[derive(Clone, Debug)]
pub struct ProducedStep {
    /// The action taken: the whole `FreeDelays` action on a merged step,
    /// otherwise the edge's choice.
    pub action: ProductAction,
    /// The drawn random values; none on a merged step.
    pub draw: Option<Vec<Value>>,
    /// The step's exact probability, merged over the action's rows on a
    /// `FreeDelays` step.
    pub probability: Rational,
    /// The post-state.
    pub to: ProbStateId,
    /// The step's rewards, in ascending name order.
    pub rewards: Vec<(Identifier, Value)>,
}

/// One produced path from an initial state.
#[derive(Clone, Debug)]
pub struct ProducedPath {
    /// The steps, in order.
    pub steps: Vec<ProducedStep>,
    /// The product of the steps' probabilities.
    pub probability: Rational,
}

/// A produced witness: the policy's actions on the evidence's states and the
/// path set.
#[derive(Clone, Debug)]
pub struct ProducedWitness {
    /// The action at each state the policy's chain reaches before accepting
    /// that the scope resolves, in state order.
    pub entries: Vec<(ProbStateId, ProductAction)>,
    /// The paths, most probable first; their sum exceeds the bound.
    pub paths: Vec<ProducedPath>,
}

/// Why no witness could be read from the product and policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WitnessError {
    /// The initial state or the accepting vector does not fit the product.
    #[error("the witness inputs do not fit the product")]
    InvalidState,
    /// The policy selects no action at a state the paths reach.
    #[error("the policy selects no action at state {state}")]
    NoChoice {
        /// The state's discovery index.
        state: usize,
    },
    /// The selected action's edges do not form one distribution.
    #[error("the selected action at state {state} is malformed")]
    MalformedAction {
        /// The state's discovery index.
        state: usize,
    },
}

/// The action `policy` takes at `state`: the whole `FreeDelays` action at a
/// `FreeDelays` state, otherwise the `policy[state]`-th distinct choice in
/// edge order; the one choice of a DTMC state. `None` when the state has no
/// edges or the policy selects none.
pub fn policy_action(
    product: &ProbProduct,
    state: ProbStateId,
    policy: &[Option<usize>],
) -> Option<ProductAction> {
    let edges = product.outgoing(state);
    let first = edges.first()?;
    if matches!(first.choice, ProductAction::FreeDelays(_)) {
        return free_delays_choice(product, state, policy);
    }
    if product.kind == ProductKind::Dtmc {
        return Some(first.choice.clone());
    }
    choices_at(product, state)
        .get((*policy.get(state.index())?)?)
        .map(|choice| (*choice).clone())
}

/// The distinct choices of `state` in edge order.
fn choices_at(product: &ProbProduct, state: ProbStateId) -> Vec<&ProductAction> {
    let mut choices: Vec<&ProductAction> = Vec::new();
    for edge in product.outgoing(state) {
        if !choices
            .iter()
            .any(|choice| same_action(choice, &edge.choice))
        {
            choices.push(&edge.choice);
        }
    }
    choices
}

/// One entry of a witness scheduler (FR-202, FR-204 "Evidence").
#[derive(Clone, Debug)]
pub enum EntryChoice {
    /// One action.
    Action(ProductAction),
    /// An exact-rational randomization over actions, each with positive
    /// probability; the probabilities sum to 1. At a `FreeDelays` state the
    /// alternatives are whole actions and their measure is the product of
    /// the rows' distributions.
    Randomized(Vec<(ProductAction, Rational)>),
}

impl EntryChoice {
    /// The actions the entry can take.
    fn actions(&self) -> Vec<&ProductAction> {
        match self {
            Self::Action(action) => vec![action],
            Self::Randomized(alternatives) => {
                alternatives.iter().map(|(action, _)| action).collect()
            }
        }
    }
}

/// The entry a randomized `policy` takes at `state`; the one choice of a
/// DTMC state. `None` when the state has no edges, the policy selects none,
/// or a distribution does not sum to 1.
pub fn entry_choice(
    product: &ProbProduct,
    state: ProbStateId,
    policy: &[Option<SchedulerChoice>],
    limits: &ExactProbLimits,
) -> Option<EntryChoice> {
    let first = product.outgoing(state).first()?;
    let alternatives: Vec<(ProductAction, Rational)> =
        if matches!(first.choice, ProductAction::FreeDelays(_)) {
            free_delays_distribution(product, state, policy, limits)?
        } else if product.kind == ProductKind::Dtmc {
            vec![(first.choice.clone(), rational(1))]
        } else {
            let choices = choices_at(product, state);
            let selected = scheduler_distribution(policy.get(state.index())?.as_ref()?)?;
            let mut alternatives = Vec::with_capacity(selected.len());
            for (index, probability) in selected {
                alternatives.push(((*choices.get(index)?).clone(), probability));
            }
            alternatives
        };
    match alternatives.as_slice() {
        [(action, _)] => Some(EntryChoice::Action(action.clone())),
        _ => Some(EntryChoice::Randomized(alternatives)),
    }
}

/// The witness scheduler of a randomized `policy` (FR-202, FR-204
/// "Evidence"): the entry `scope` resolves at every state reachable from
/// `initial` through the support of the entries, up to the `accepting`
/// states, in state order. Evidence and continuation states alike carry an
/// entry, whether or not an emitted path leaves them.
///
/// # Errors
///
/// [`WitnessError`] when the inputs do not fit the product, or the policy
/// selects no well-formed entry at a reached state.
pub fn witness_scheduler(
    product: &ProbProduct,
    policy: &[Option<SchedulerChoice>],
    scope: WitnessScope,
    initial: ProbStateId,
    accepting: &[bool],
    limits: &ExactProbLimits,
) -> Result<Vec<(ProbStateId, EntryChoice)>, WitnessError> {
    if accepting.len() != product.states.len() || initial.index() >= product.states.len() {
        return Err(WitnessError::InvalidState);
    }
    let mut scheduler: BTreeMap<ProbStateId, EntryChoice> = BTreeMap::new();
    let mut queue = VecDeque::from([initial]);
    while let Some(state) = queue.pop_front() {
        let accepts = accepting
            .get(state.index())
            .copied()
            .ok_or(WitnessError::InvalidState)?;
        if accepts || scheduler.contains_key(&state) || product.outgoing(state).is_empty() {
            continue;
        }
        let index = state.index();
        let entry = entry_choice(product, state, policy, limits)
            .ok_or(WitnessError::NoChoice { state: index })?;
        for action in entry.actions() {
            let steps = action_steps(product, state, action)
                .ok_or(WitnessError::MalformedAction { state: index })?;
            queue.extend(steps.iter().map(|step| step.to));
        }
        scheduler.insert(state, entry);
    }
    Ok(scheduler
        .into_iter()
        .filter(|(_, entry)| {
            entry
                .actions()
                .into_iter()
                .all(|action| resolves(scope, action))
        })
        .collect())
}

/// The steps `action` takes from `state`, with positive probability: a
/// `FreeDelays` action's row-choice edges merged by post-state and rewards;
/// every edge of a DTMC state; otherwise the action's edges. `None` when the
/// action is not offered at the state or its steps do not sum to 1.
pub fn action_steps(
    product: &ProbProduct,
    state: ProbStateId,
    action: &ProductAction,
) -> Option<Vec<ProducedStep>> {
    let edges = product.outgoing(state);
    let mut steps: Vec<ProducedStep> = Vec::new();
    if let ProductAction::FreeDelays(rows) = action {
        for row in rows {
            let mut offered = false;
            for edge in edges.iter().filter(|edge| {
                matches!(&edge.choice, ProductAction::FreeDelays(body)
                    if matches!(body.as_slice(), [only] if only == row))
            }) {
                offered = true;
                let merged = steps
                    .iter_mut()
                    .find(|step| step.to == edge.to && same_rewards(&step.rewards, &edge.rewards));
                match merged {
                    Some(step) => step.probability = step.probability.add(&edge.probability),
                    None => steps.push(ProducedStep {
                        action: action.clone(),
                        draw: None,
                        probability: edge.probability.clone(),
                        to: edge.to,
                        rewards: edge.rewards.clone(),
                    }),
                }
            }
            if !offered {
                return None;
            }
        }
    } else {
        let dtmc = product.kind == ProductKind::Dtmc;
        steps.extend(
            edges
                .iter()
                .filter(|edge| dtmc || same_action(&edge.choice, action))
                .map(|edge| ProducedStep {
                    action: edge.choice.clone(),
                    draw: edge.draw.clone(),
                    probability: edge.probability.clone(),
                    to: edge.to,
                    rewards: edge.rewards.clone(),
                }),
        );
    }
    let mass = steps
        .iter()
        .fold(rational(0), |sum, step| sum.add(&step.probability));
    if mass != rational(1) {
        return None;
    }
    steps.retain(|step| step.probability > rational(0));
    Some(steps)
}

/// One step of a partial path: which step of its source state's induced
/// steps it took, linked to the steps before it.
struct Link {
    from: ProbStateId,
    index: usize,
    parent: Option<Rc<Link>>,
}

/// A partial path in the best-first search.
struct Partial {
    probability: Rational,
    sequence: u64,
    state: ProbStateId,
    last: Option<Rc<Link>>,
}

impl Partial {
    /// The path's steps, cloned from the induced chain; only accepted paths
    /// pay for this.
    fn steps(&self, induced: &Induced) -> Vec<ProducedStep> {
        let mut steps = Vec::new();
        let mut link = self.last.as_deref();
        while let Some(current) = link {
            if let Some((_, taken)) = induced.get(&current.from) {
                steps.extend(taken.get(current.index).cloned());
            }
            link = current.parent.as_deref();
        }
        steps.reverse();
        steps
    }
}

impl PartialEq for Partial {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Partial {}

impl PartialOrd for Partial {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Partial {
    /// The more probable partial first; among equals, the earlier found.
    fn cmp(&self, other: &Self) -> Ordering {
        self.probability
            .cmp(&other.probability)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

/// The chain `policy` induces from `initial`, up to the accepting states:
/// each reached state's action and steps.
type Induced = BTreeMap<ProbStateId, (ProductAction, Vec<ProducedStep>)>;

fn induce(
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: ProbStateId,
    accepting: &[bool],
) -> Result<Induced, WitnessError> {
    let mut induced = Induced::new();
    let mut queue = VecDeque::from([initial]);
    while let Some(state) = queue.pop_front() {
        let accepts = accepting
            .get(state.index())
            .copied()
            .ok_or(WitnessError::InvalidState)?;
        if accepts || induced.contains_key(&state) || product.outgoing(state).is_empty() {
            continue;
        }
        let index = state.index();
        let action =
            policy_action(product, state, policy).ok_or(WitnessError::NoChoice { state: index })?;
        let steps = action_steps(product, state, &action)
            .ok_or(WitnessError::MalformedAction { state: index })?;
        queue.extend(steps.iter().map(|step| step.to));
        induced.insert(state, (action, steps));
    }
    Ok(induced)
}

/// The states of the induced chain from which an accepting state is
/// reachable: one reverse breadth-first search from the accepting states.
fn reaching(induced: &Induced, accepting: &[bool]) -> BTreeSet<ProbStateId> {
    let mut predecessors: BTreeMap<ProbStateId, Vec<ProbStateId>> = BTreeMap::new();
    let mut queue = VecDeque::new();
    let mut reaching: BTreeSet<ProbStateId> = BTreeSet::new();
    for (state, (_, steps)) in induced {
        for step in steps {
            predecessors.entry(step.to).or_default().push(*state);
            if accepting.get(step.to.index()).copied().unwrap_or(false) && reaching.insert(*state) {
                queue.push_back(*state);
            }
        }
    }
    while let Some(state) = queue.pop_front() {
        for predecessor in predecessors.get(&state).into_iter().flatten() {
            if reaching.insert(*predecessor) {
                queue.push_back(*predecessor);
            }
        }
    }
    reaching
}

/// Produce FR-202's path-set witness on the chain `policy` induces from
/// `initial`: the paths that first reach an `accepting` state, most probable
/// first, until their exact sum exceeds `past`, and the policy's action at
/// each state they leave that `scope` resolves. A partial path at a state
/// from which no accepting state is reachable is not extended.
///
/// `Ok(None)` when no such path set exists within `limits.max_witness_paths`
/// paths, `max_transitions` expansions and frontier pushes, and
/// `max_rational_bits` per path probability: the claim then needs subsystem
/// evidence instead.
///
/// # Errors
///
/// [`WitnessError`] when the inputs do not fit the product, or the policy
/// selects no well-formed action at a reached state.
pub fn produce_witness(
    product: &ProbProduct,
    policy: &[Option<usize>],
    scope: WitnessScope,
    initial: ProbStateId,
    accepting: &[bool],
    past: &Rational,
    limits: &ExactProbLimits,
) -> Result<Option<ProducedWitness>, WitnessError> {
    if accepting.len() != product.states.len() || initial.index() >= product.states.len() {
        return Err(WitnessError::InvalidState);
    }
    let induced = induce(product, policy, initial, accepting)?;
    let reaching = reaching(&induced, accepting);
    let accepts = |state: ProbStateId| accepting.get(state.index()).copied().unwrap_or(false);
    let mut frontier = BinaryHeap::from([Partial {
        probability: rational(1),
        sequence: 0,
        state: initial,
        last: None,
    }]);
    let mut sequence = 0_u64;
    let mut expansions = 0_usize;
    let mut pushes = 1_usize;
    let mut paths: Vec<ProducedPath> = Vec::new();
    let mut sum = rational(0);
    while let Some(partial) = frontier.pop() {
        if accepts(partial.state) {
            if u64::try_from(paths.len()).map_or(true, |count| count >= limits.max_witness_paths) {
                return Ok(None);
            }
            sum = sum.add(&partial.probability);
            paths.push(ProducedPath {
                steps: partial.steps(&induced),
                probability: partial.probability,
            });
            if sum > *past {
                return Ok(Some(ProducedWitness {
                    entries: entries(&induced, scope),
                    paths,
                }));
            }
            continue;
        }
        let Some((_, steps)) = induced.get(&partial.state) else {
            continue;
        };
        expansions += 1;
        if expansions > limits.model.limits.max_transitions {
            return Ok(None);
        }
        for (index, step) in steps.iter().enumerate() {
            if !accepts(step.to) && !reaching.contains(&step.to) {
                continue;
            }
            pushes += 1;
            if pushes > limits.model.limits.max_transitions {
                return Ok(None);
            }
            let probability = partial.probability.mul(&step.probability);
            if !rational_fits(&probability, limits.max_rational_bits) {
                return Ok(None);
            }
            sequence += 1;
            frontier.push(Partial {
                probability,
                sequence,
                state: step.to,
                last: Some(Rc::new(Link {
                    from: partial.state,
                    index,
                    parent: partial.last.clone(),
                })),
            });
        }
    }
    Ok(None)
}

/// The scope's entries on every state the induced chain reaches before it
/// accepts: the evidence states the paths leave and the continuation states
/// beside them.
fn entries(induced: &Induced, scope: WitnessScope) -> Vec<(ProbStateId, ProductAction)> {
    induced
        .iter()
        .filter(|(_, (action, _))| resolves(scope, action))
        .map(|(state, (action, _))| (*state, action.clone()))
        .collect()
}

/// Whether `scope`'s witness scheduler names `action`'s choice.
fn resolves(scope: WitnessScope, action: &ProductAction) -> bool {
    match scope {
        WitnessScope::EveryScheduler => true,
        WitnessScope::Workload => match action {
            ProductAction::UnitDelay
            | ProductAction::EnterIdleDelay
            | ProductAction::IdleDelay
            | ProductAction::IdleObserve
            | ProductAction::WorkloadResolve { .. }
            | ProductAction::FreeDelays(_) => true,
            ProductAction::Identity(_)
            | ProductAction::PostState(_)
            | ProductAction::TerminalStutter => false,
        },
    }
}

fn rational(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}
