// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201 exact-value certificate types and the independent Bellman checker
//! core. The bounded graph adapter below enumerates a checker-owned model
//! graph through `qsl-eval::ModelSystem`; it does not consume analyzer output.

use qsl_semantics::check::Bound;
use quire_exact::{Integer, Rational};
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_semantics::check::{
    CheckedProbabilisticClaim, ProbForm, ProbScheduler, ReachTarget, StatePredicate,
};
use serde_json::{Value, json};

use crate::identity::ObligationIdentity;

/// A complete canonical product-state key, encoded by the checker.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProbabilityProductKey(Vec<u8>);

impl ProbabilityProductKey {
    /// Construct a key from its canonical encoding.
    pub fn from_canonical_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The canonical product-state encoding.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The timed-tail control included in an FR-204 model key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigitalTail {
    /// A state that still admits ordinary discrete operations.
    Running,
    /// A selected idle tail before its first positive delay.
    IdleReady,
    /// A selected idle tail after a positive delay.
    IdleDelayed,
}

impl DigitalTail {
    fn tag(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::IdleReady => "idle-ready",
            Self::IdleDelayed => "idle-delayed",
        }
    }
}

/// One admitted digital clock and its checker-derived cap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigitalClockKey {
    /// The checked declaring type, field and receiver tuple.
    pub identity: Value,
    /// The clock value in digital units.
    pub value: u64,
    /// The checker-derived saturation cap in digital units.
    pub cap: u64,
}

/// A timed deadline clock and its scaled closed bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DigitalDeadlineKey {
    /// The scaled closed deadline bound.
    pub limit: u64,
    /// The current non-resetting deadline clock value.
    pub elapsed: u64,
}

/// Encode the FR-196 canonical ProductKey for one admitted digital state.
///
/// `discrete` is the complete non-clock simulation-state form and `unit` is
/// FR-196's closed time-unit form. Clock identities are canonical three-part
/// tuples; the encoder orders them by their complete JCS bytes. The numeric
/// fields use decimal strings in the key so values are not rounded through
/// JSON numbers.
///
/// The JSON forms supplied for `binding`, `discrete`, `unit`, `monitor`,
/// `accumulator` and `pending` must already have been derived from the checked
/// subject and claim. This routine owns only their canonical representation;
/// contextual admission and derivation remain the caller's responsibility.
pub fn digital_probability_product_key(
    binding: Option<Value>,
    discrete: Value,
    unit: Value,
    scale: u64,
    clocks: Vec<DigitalClockKey>,
    deadline: Option<DigitalDeadlineKey>,
    tail: DigitalTail,
    monitor: Option<Value>,
    accumulator: Option<Value>,
    pending: Option<Value>,
) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
    if scale == 0 {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }

    let mut keyed_clocks = clocks
        .into_iter()
        .map(|clock| {
            if clock
                .identity
                .as_array()
                .is_none_or(|parts| parts.len() != 3)
                || clock.value > clock.cap
            {
                return Err(ExactProbabilityGraphError::InvalidDigitalKey);
            }
            let identity_bytes = canonical_json(&clock.identity)?;
            Ok((identity_bytes, clock.identity, clock.value, clock.cap))
        })
        .collect::<Result<Vec<_>, _>>()?;
    keyed_clocks.sort_by(|left, right| left.0.cmp(&right.0));
    if keyed_clocks.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }
    let clocks = keyed_clocks
        .into_iter()
        .map(|(_, identity, value, cap)| json!([identity, value.to_string(), cap.to_string()]))
        .collect::<Vec<_>>();

    let deadline = match deadline {
        Some(DigitalDeadlineKey { limit, elapsed })
            if limit.checked_add(1).is_some_and(|cap| elapsed <= cap) =>
        {
            json!(["deadline", limit.to_string(), elapsed.to_string()])
        }
        Some(_) => return Err(ExactProbabilityGraphError::InvalidDigitalKey),
        None => Value::Null,
    };
    let model = json!([
        "quire.simulation.state-key/v1",
        [
            "digital",
            discrete,
            unit,
            scale.to_string(),
            clocks,
            deadline,
            tail.tag()
        ]
    ]);
    let model_bytes = canonical_json(&model)?;
    let encoded_model = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        model_bytes,
    );
    let product = json!([
        "quire.probability-product-key/v1",
        binding,
        encoded_model,
        monitor,
        accumulator,
        pending
    ]);
    canonical_json(&product).map(ProbabilityProductKey)
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, ExactProbabilityGraphError> {
    quire_canonical::to_vec(value, quire_canonical::Limits::new(u64::MAX))
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)
}

/// The exact rational value certified for one probabilistic product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityValue {
    /// The complete canonical product-state key.
    pub state: ProbabilityProductKey,
    /// The exact value at that state.
    pub value: Rational,
}

/// An exact-value certificate's state values (FR-201).
///
/// This certificate form does not itself establish graph completeness. The
/// replay facade must re-enumerate the subject and pass that checker-owned
/// graph to the independent checker before accepting the certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityCertificate {
    /// One value for every state in the checker-owned graph, in key order.
    pub values: Vec<ExactProbabilityValue>,
}

/// The scheduler objective for an exact reachability certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityObjective {
    /// The minimum over all enabled actions.
    Minimum,
    /// The maximum over all enabled actions.
    Maximum,
    /// A workload with exactly one enabled probability distribution.
    Workload,
}

/// Whether the certified initial value satisfies the claim's probability bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityBoundSide {
    /// The value satisfies an authored `>=` bound.
    AtLeast,
    /// The value satisfies an authored `<=` bound.
    AtMost,
}

/// An exact certificate rejection and the first canonical state at fault.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityCertificateRejection {
    /// The failed FR-201 condition.
    pub rule: ProbabilityCertificateRule,
    /// The state or member at which it failed.
    pub at: ProbabilityCertificateLocus,
}

/// The exact-certificate rejection rules implemented by this checker core.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificateRule {
    /// A certificate value does not equal the recomputed Bellman value.
    InequalityFails,
    /// A structurally valid certificate's initial value misses the claim bound.
    ThresholdMissed,
    /// A certificate omits, duplicates, or adds a state value.
    Malformed,
}

/// The locus of an exact-certificate rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificateLocus {
    /// A full probabilistic product state key.
    ProductState(ProbabilityProductKey),
    /// A malformed certificate member.
    Member(&'static str),
}

/// The result of checking an exact-value certificate against a graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactProbabilityCertificateCheck {
    /// Every value matches the independently evaluated graph and the initial
    /// value satisfies the claim bound.
    Accepted {
        /// The checked initial value.
        value: Rational,
        /// The authored bound side that was checked.
        side: ProbabilityBoundSide,
    },
    /// The certificate fails a checked condition.
    Rejected(ProbabilityCertificateRejection),
}

/// Why the exact Bellman-check core could not check a graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactProbabilityGraphError {
    /// A listed successor has no state in the checker graph.
    MissingSuccessor,
    /// The graph's edge count exceeds the representable in-memory state count.
    TooManyEdges,
    /// The graph is cyclic; cyclic certificates require the later
    /// unbounded-certificate checking path.
    Cycle,
    /// The checker-side model expansion was undecided or refused.
    ExpansionRefused,
    /// The checker-side graph exceeded its configured state or edge bound.
    LimitReached,
    /// The model API exposed a draw that was not a single-state probability.
    UnsupportedDraw,
    /// A digital key contains an invalid or inconsistent timed component.
    InvalidDigitalKey,
    /// The model has no single initial state; this slice checks exactly one.
    UnsupportedInitial,
    /// No checked claim has the requested obligation's node as its checked
    /// package identity, or more than one does.
    ClaimNotBound,
    /// The bound claim is not an unbounded `eventually` reachability claim,
    /// the only form this bounded slice checks.
    UnsupportedClaimForm,
}

/// Enumerate an unbound reachability graph from the checked model itself.
/// Product keys use the FR-196 v1 array with only the model component set;
/// monitor, accumulator, and pending controls are null in this bounded slice.
///
/// This adapter intentionally refuses draws with multiple post-states and
/// stops at explicit limits. It is a bounded exact-check foundation, not a
/// complete probabilistic product implementation.
fn enumerate_model_graph<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: &P,
    max_states: usize,
    max_edges: usize,
) -> Result<CheckerGraph, ExactProbabilityGraphError> {
    let initial = system.initial();
    if initial.len() != 1 {
        return Err(ExactProbabilityGraphError::UnsupportedInitial);
    }
    let key_of = |state: &ModelSimState| {
        let (model_bytes, _) = system
            .state_key(state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        product_key(&model_bytes)
    };
    let first = initial
        .into_iter()
        .next()
        .ok_or(ExactProbabilityGraphError::UnsupportedInitial)?;
    let root = key_of(&first)?;
    let mut pending = VecDeque::from([(root.clone(), first)]);
    let mut states = BTreeMap::new();
    let mut seen = BTreeSet::from([root.clone()]);
    let mut edge_count = 0usize;
    while let Some((key, state)) = pending.pop_front() {
        if states.len() == max_states {
            return Err(ExactProbabilityGraphError::LimitReached);
        }
        let reading = system
            .state_predicates(&state, &[(target, None)])
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
            .into_iter()
            .next()
            .ok_or(ExactProbabilityGraphError::ExpansionRefused)?;
        let terminal = match reading {
            qsl_eval::simulation::model::PredicateReading::True => Some(true),
            qsl_eval::simulation::model::PredicateReading::False => None,
            _ => return Err(ExactProbabilityGraphError::ExpansionRefused),
        };
        let choices = if terminal.is_some() {
            Vec::new()
        } else {
            let actions = system
                .actions(&state)
                .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
                .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
            if actions.is_empty() {
                states.insert(
                    key,
                    CheckerState {
                        terminal: Some(false),
                        choices: Vec::new(),
                    },
                );
                continue;
            }
            let mut choices = Vec::with_capacity(actions.len());
            for action in actions {
                let mut choice = Vec::new();
                for draw in action.draws {
                    if draw.probability == rational(0) {
                        continue;
                    }
                    if draw.successors.len() != 1 {
                        return Err(ExactProbabilityGraphError::UnsupportedDraw);
                    }
                    let (_, successor) = draw
                        .successors
                        .into_iter()
                        .next()
                        .ok_or(ExactProbabilityGraphError::UnsupportedDraw)?;
                    let target_key = key_of(&successor)?;
                    edge_count = edge_count
                        .checked_add(1)
                        .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
                    if edge_count > max_edges {
                        return Err(ExactProbabilityGraphError::LimitReached);
                    }
                    choice.push(ProbabilityEdge {
                        target: target_key.clone(),
                        probability: draw.probability,
                    });
                    if seen.insert(target_key.clone()) {
                        pending.push_back((target_key, successor));
                    }
                }
                choices.push(choice);
            }
            choices
        };
        states.insert(key, CheckerState { terminal, choices });
    }
    Ok(CheckerGraph {
        initial: root,
        states,
    })
}

fn product_key(model_bytes: &[u8]) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
    let encoded_model = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        model_bytes,
    );
    let bytes = serde_json::to_vec(&serde_json::json!([
        "quire.probability-product-key/v1",
        null,
        encoded_model,
        null,
        null,
        null
    ]))
    .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    Ok(ProbabilityProductKey(bytes))
}

/// Independently enumerate and check a bounded unbound exact reachability
/// certificate against the supplied checked model system.
///
/// The caller constructs `system` from replay/recompiled subject inputs and
/// supplies the checked claim predicate. The certificate cannot supply graph
/// states or transitions. This slice handles one initial state and exact
/// draws with one successor each; unsupported shapes return an error.
pub(crate) fn check_model_exact_reachability<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: &P,
    certificate: &ExactProbabilityCertificate,
    objective: ProbabilityObjective,
    bound: &Bound,
    max_states: usize,
    max_edges: usize,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    let graph = enumerate_model_graph(system, target, max_states, max_edges)?;
    check_exact_reachability(&graph, certificate, objective, bound)
}

/// Bind `item` to its checked claim by checked node identity, then check the
/// certificate over that claim's target predicate and bound (FR-498).
///
/// The claim is the one whose `occurrence().node()` equals the obligation's
/// node; name, span and source order never select it. A claim without a
/// package identity is never bound.
pub fn check_claim_exact_reachability(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    max_states: usize,
    max_edges: usize,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    let mut bound = claims.iter().filter(|claim| {
        claim
            .occurrence()
            .is_some_and(|key| key.node() == item.node())
    });
    let claim = bound
        .next()
        .ok_or(ExactProbabilityGraphError::ClaimNotBound)?;
    if bound.next().is_some() {
        return Err(ExactProbabilityGraphError::ClaimNotBound);
    }
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(predicate),
    } = &claim.form
    else {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm);
    };
    let objective = claim_objective(&claim.scheduler, bound);
    check_model_exact_reachability(
        system,
        predicate,
        certificate,
        objective,
        bound,
        max_states,
        max_edges,
    )
}

/// The scheduler objective the claim itself fixes (FR-198): a claim `under
/// every scheduler` must hold at the worst scheduler, the minimum for a `>=`
/// bound and the maximum for a `<=` bound; a workload has one distribution.
fn claim_objective(scheduler: &ProbScheduler, bound: &Bound) -> ProbabilityObjective {
    match (scheduler, bound) {
        (ProbScheduler::Workload(_), _) => ProbabilityObjective::Workload,
        (ProbScheduler::Every, Bound::AtLeast(_)) => ProbabilityObjective::Minimum,
        (ProbScheduler::Every, Bound::AtMost(_)) => ProbabilityObjective::Maximum,
    }
}

/// One successor and its exact probability mass.
#[derive(Clone, Debug)]
struct ProbabilityEdge {
    target: ProbabilityProductKey,
    probability: Rational,
}

/// One action's probabilistic successors.
type ProbabilityChoice = Vec<ProbabilityEdge>;

/// One checker-enumerated state. This type is private so an eventual replay
/// facade owns graph construction instead of trusting solver data.
#[derive(Clone, Debug)]
struct CheckerState {
    terminal: Option<bool>,
    choices: Vec<ProbabilityChoice>,
}

/// The graph produced by a checker-side enumerator.
#[derive(Clone, Debug)]
struct CheckerGraph {
    initial: ProbabilityProductKey,
    states: BTreeMap<ProbabilityProductKey, CheckerState>,
}

/// Check exact reachability values on a checker-owned acyclic graph.
///
/// The graph representation and this evaluator are independent of
/// `qsl-analyze`; this core performs no policy iteration or linear algebra.
/// Cyclic graphs are rejected until FR-201's unbounded graph checks are
/// implemented.
fn check_exact_reachability(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    objective: ProbabilityObjective,
    bound: &Bound,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    let values = match certificate_values(graph, certificate) {
        Ok(values) => values,
        Err(rejection) => return Ok(ExactProbabilityCertificateCheck::Rejected(rejection)),
    };
    let order = topological_order(graph)?;

    let zero = rational(0);
    let one = rational(1);
    let mut computed = BTreeMap::new();
    for key in order.into_iter().rev() {
        let Some(state) = graph.states.get(&key) else {
            return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
        };
        let actual = if let Some(target) = state.terminal {
            if target {
                one.clone()
            } else {
                zero.clone()
            }
        } else {
            if state.choices.is_empty()
                || (objective == ProbabilityObjective::Workload && state.choices.len() != 1)
            {
                return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
            }
            let mut choice_values = Vec::with_capacity(state.choices.len());
            for choice in &state.choices {
                let mut mass = zero.clone();
                let mut value = zero.clone();
                for edge in choice {
                    if edge.probability < zero {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    }
                    if edge.probability == zero {
                        continue;
                    }
                    let Some(successor) = computed.get(&edge.target) else {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    };
                    mass = mass.add(&edge.probability);
                    value = value.add(&edge.probability.mul(successor));
                }
                if mass != one {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    ));
                }
                choice_values.push(value);
            }
            match objective {
                ProbabilityObjective::Minimum => choice_values.into_iter().min(),
                ProbabilityObjective::Maximum => choice_values.into_iter().max(),
                ProbabilityObjective::Workload => choice_values.into_iter().next(),
            }
            .unwrap_or_else(|| zero.clone())
        };
        let Some(certified) = values.get(&key) else {
            return Ok(rejected(ProbabilityCertificateRule::Malformed, key));
        };
        if actual != *certified {
            return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
        }
        computed.insert(key, actual);
    }

    let Some(initial) = computed.get(&graph.initial) else {
        return Ok(rejected(
            ProbabilityCertificateRule::InequalityFails,
            graph.initial.clone(),
        ));
    };
    let meets_bound = match bound {
        Bound::AtLeast(threshold) => initial >= threshold,
        Bound::AtMost(threshold) => initial <= threshold,
    };
    if !meets_bound {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    Ok(ExactProbabilityCertificateCheck::Accepted {
        value: initial.clone(),
        side: match bound {
            Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
            Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
        },
    })
}

fn certificate_values(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
) -> Result<BTreeMap<ProbabilityProductKey, Rational>, ProbabilityCertificateRejection> {
    let mut values = BTreeMap::new();
    let mut previous = None;
    for entry in &certificate.values {
        if previous.as_ref().is_some_and(|key| key >= &entry.state) {
            return Err(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::ProductState(entry.state.clone()),
            });
        }
        previous = Some(entry.state.clone());
        if values
            .insert(entry.state.clone(), entry.value.clone())
            .is_some()
            || !graph.states.contains_key(&entry.state)
        {
            return Err(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::ProductState(entry.state.clone()),
            });
        }
    }
    if values.len() != graph.states.len() {
        let missing = graph
            .states
            .keys()
            .find(|key| !values.contains_key(*key))
            .cloned()
            .unwrap_or_else(|| graph.initial.clone());
        return Err(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::ProductState(missing),
        });
    }
    Ok(values)
}

fn topological_order(
    graph: &CheckerGraph,
) -> Result<Vec<ProbabilityProductKey>, ExactProbabilityGraphError> {
    let zero = rational(0);
    let mut indegree = graph
        .states
        .keys()
        .cloned()
        .map(|key| (key, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for state in graph.states.values() {
        for edge in state.choices.iter().flatten() {
            if edge.probability == zero {
                continue;
            }
            let Some(count) = indegree.get_mut(&edge.target) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            *count = count
                .checked_add(1)
                .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(key, degree)| (*degree == 0).then_some(key.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(graph.states.len());
    while let Some(key) = ready.pop_first() {
        order.push(key.clone());
        let Some(state) = graph.states.get(&key) else {
            return Err(ExactProbabilityGraphError::MissingSuccessor);
        };
        for edge in state.choices.iter().flatten() {
            if edge.probability == zero {
                continue;
            }
            let Some(count) = indegree.get_mut(&edge.target) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            let Some(remaining) = count.checked_sub(1) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            *count = remaining;
            if *count == 0 {
                ready.insert(edge.target.clone());
            }
        }
    }
    if order.len() == graph.states.len() {
        Ok(order)
    } else {
        Err(ExactProbabilityGraphError::Cycle)
    }
}

fn rejected(
    rule: ProbabilityCertificateRule,
    state: ProbabilityProductKey,
) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule,
        at: ProbabilityCertificateLocus::ProductState(state),
    })
}

fn rational(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use ix_trace_rs::trace;

    fn key(value: u8) -> ProbabilityProductKey {
        ProbabilityProductKey(vec![value])
    }

    fn edge(target: u8, numerator: i64, denominator: i64) -> ProbabilityEdge {
        ProbabilityEdge {
            target: key(target),
            probability: Rational::new(Integer::from(numerator), Integer::from(denominator))
                .expect("nonzero denominator"),
        }
    }

    fn graph() -> CheckerGraph {
        let initial = key(0);
        let mut states = BTreeMap::new();
        states.insert(
            initial.clone(),
            CheckerState {
                terminal: None,
                choices: vec![
                    vec![edge(1, 4, 5), edge(2, 1, 5)],
                    vec![edge(1, 9, 10), edge(2, 1, 10)],
                ],
            },
        );
        states.insert(
            key(1),
            CheckerState {
                terminal: Some(true),
                choices: Vec::new(),
            },
        );
        states.insert(
            key(2),
            CheckerState {
                terminal: Some(false),
                choices: Vec::new(),
            },
        );
        CheckerGraph { initial, states }
    }

    fn certificate(initial_value: Rational) -> ExactProbabilityCertificate {
        ExactProbabilityCertificate {
            values: vec![
                ExactProbabilityValue {
                    state: key(0),
                    value: initial_value,
                },
                ExactProbabilityValue {
                    state: key(1),
                    value: rational(1),
                },
                ExactProbabilityValue {
                    state: key(2),
                    value: rational(0),
                },
            ],
        }
    }

    #[trace("TC-636")]
    #[test]
    fn the_claim_fixes_the_scheduler_objective() {
        let half = || Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap();
        let every = ProbScheduler::Every;
        let workload = ProbScheduler::Workload("W".to_owned());
        assert_eq!(
            claim_objective(&every, &Bound::AtLeast(half())),
            ProbabilityObjective::Minimum
        );
        assert_eq!(
            claim_objective(&every, &Bound::AtMost(half())),
            ProbabilityObjective::Maximum
        );
        for bound in [Bound::AtLeast(half()), Bound::AtMost(half())] {
            assert_eq!(
                claim_objective(&workload, &bound),
                ProbabilityObjective::Workload
            );
        }
    }

    #[trace("TC-636")]
    #[test]
    fn bounded_model_product_key_uses_the_complete_fr196_v1_tuple() {
        let key = product_key(b"model-state-key").expect("the fixed key encodes");
        let actual: serde_json::Value =
            serde_json::from_slice(key.as_bytes()).expect("the key is JSON");
        assert_eq!(
            actual,
            serde_json::json!([
                "quire.probability-product-key/v1",
                null,
                "bW9kZWwtc3RhdGUta2V5",
                null,
                null,
                null
            ])
        );
    }

    #[trace("TC-636")]
    #[test]
    fn exact_certificate_checks_each_state_and_reports_first_bad_key() {
        let graph = graph();
        let result = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            result,
            ExactProbabilityCertificateCheck::Accepted {
                value: Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap(),
                side: ProbabilityBoundSide::AtLeast,
            }
        );

        let rejected_result = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            rejected_result,
            rejected(ProbabilityCertificateRule::InequalityFails, key(0))
        );
    }

    #[trace("TC-636")]
    #[test]
    fn exact_certificate_rejects_threshold_miss_and_cycles() {
        let graph = graph();
        let missed = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            missed,
            rejected(ProbabilityCertificateRule::ThresholdMissed, key(0))
        );

        let mut cyclic = graph;
        let state = cyclic.states.get_mut(&key(2)).unwrap();
        state.terminal = None;
        state.choices = vec![vec![edge(2, 1, 1)]];
        assert_eq!(
            check_exact_reachability(
                &cyclic,
                &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
                ProbabilityObjective::Workload,
                &Bound::AtLeast(rational(0)),
            ),
            Err(ExactProbabilityGraphError::Cycle)
        );
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn digital_product_key_encodes_the_complete_timed_model_and_orders_clocks() {
        let key = digital_probability_product_key(
            None,
            json!(["state", {"z": 1, "a": true}]),
            json!(["unit", "second"]),
            10,
            vec![
                DigitalClockKey {
                    identity: json!(["Type", "zeta", ["u", "object-z"]]),
                    value: 1,
                    cap: 3,
                },
                DigitalClockKey {
                    identity: json!(["Type", "alpha", ["u", "object-a"]]),
                    value: 0,
                    cap: 3,
                },
            ],
            Some(DigitalDeadlineKey {
                limit: 4,
                elapsed: 1,
            }),
            DigitalTail::IdleDelayed,
            None,
            None,
            None,
        )
        .expect("the complete timed key is representable");
        let outer: Value = serde_json::from_slice(key.as_bytes()).expect("the key is JSON");
        let model_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(outer[2].as_str().expect("the model key is base64url"))
            .expect("the model key decodes");
        assert_eq!(
            model_bytes,
            br#"["quire.simulation.state-key/v1",["digital",["state",{"a":true,"z":1}],["unit","second"],"10",[[["Type","alpha",["u","object-a"]],"0","3"],[["Type","zeta",["u","object-z"]],"1","3"]],["deadline","4","1"],"idle-delayed"]]"#
        );
        assert_eq!(
            outer,
            json!([
                "quire.probability-product-key/v1",
                null,
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(model_bytes),
                null,
                null,
                null
            ])
        );
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn digital_product_key_rejects_invalid_scale_clock_and_deadline_values() {
        let clock = || DigitalClockKey {
            identity: json!(["Type", "clock", ["u", "object"]]),
            value: 1,
            cap: 1,
        };
        let key = |scale, clocks, deadline| {
            digital_probability_product_key(
                None,
                json!([]),
                json!(["unit", "second"]),
                scale,
                clocks,
                deadline,
                DigitalTail::Running,
                None,
                None,
                None,
            )
        };
        assert_eq!(
            key(0, vec![], None),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
        assert_eq!(
            key(
                1,
                vec![DigitalClockKey {
                    value: 2,
                    ..clock()
                }],
                None
            ),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
        assert_eq!(
            key(1, vec![clock(), clock()], None),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
        assert_eq!(
            key(
                1,
                vec![clock()],
                Some(DigitalDeadlineKey {
                    limit: 1,
                    elapsed: 3,
                })
            ),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
    }
}
