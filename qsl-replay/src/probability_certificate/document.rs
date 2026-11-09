// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed exact reachability document profile and strict JCS codec.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use super::*;

const PROFILE: &str = "quire.probability-objective/reachability/v1";
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// Exact reachability evidence, before independent semantic checking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityCertificateDocument {
    /// Structural claim identity.
    pub identity: ObligationIdentity,
    /// Complete, strictly ordered reachable target keys.
    pub targets: Vec<ProbabilityProductKey>,
    /// Operator selected by the checked claim.
    pub optimization: ProbabilityObjective,
    /// Exact values, covering every reachable product state.
    pub values: Vec<ExactProbabilityValue>,
    /// Required positive-value progress ranks, covering every reachable state.
    pub ranking: Vec<(ProbabilityProductKey, u64)>,
}

/// Caller ceilings for certificate admission and independent graph checking.
#[derive(Clone, Copy, Debug)]
pub struct ProbabilityDocumentLimits {
    /// Maximum document bytes, for both reading and writing.
    pub bytes: u64,
    /// Canonical JSON reader working-storage ceiling in bytes.
    pub storage_bytes: u64,
    /// Maximum checker graph states and certificate rows.
    pub states: usize,
    /// Maximum checker graph edges.
    pub edges: usize,
    /// Maximum numerator or denominator bits.
    pub scalar_bits: u64,
    /// Shared exact-arithmetic accounting ceilings, including work and storage.
    pub accounting: quire_exact::ScalarLimits,
}

/// A document never becomes accepted evidence when one of these occurs.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProbabilityDocumentError {
    /// Input is malformed or violates the closed wire contract.
    #[error("invalid_runtime_input: malformed probability certificate")]
    Malformed,
    /// A required evidence member was omitted.
    #[error("invalid_runtime_input: missing probability certificate member {0}")]
    MissingMember(&'static str),
    /// Bytes are valid JSON but not their RFC 8785 encoding.
    #[error("invalid_runtime_input: noncanonical probability certificate")]
    Noncanonical,
    /// The document selects a profile, optimization or kind outside this slice.
    #[error("unsupported probability certificate profile")]
    Unsupported,
    /// Structural identity differs from the requested checked claim.
    #[error("conflicting_identity: probability certificate")]
    IdentityMismatch,
    /// Targets or optimization differ from the independently checked claim.
    #[error("invalid_runtime_input: probability certificate objective mismatch")]
    ObjectiveMismatch,
    /// A caller byte, row or arithmetic ceiling was reached.
    #[error("probability certificate limit reached: {setting} ({actual} > {bound})")]
    Limit {
        /// Exhausted setting.
        setting: &'static str,
        /// Caller ceiling.
        bound: u64,
        /// Actual required amount, including the refused growth.
        actual: u64,
    },
    /// Cancellation retains the caller's cause.
    #[error("probability certificate cancelled: {0:?}")]
    Cancelled(CancelCause),
    /// Shared exact arithmetic exhausted a named accounting charge.
    #[error("probability certificate arithmetic stopped: {0:?}")]
    Accounting(Box<quire_exact::Incomplete>),
    /// Independent subject admission or graph traversal stopped.
    #[error("probability certificate graph stopped: {0:?}")]
    Graph(ExactProbabilityGraphError),
    /// Subject expansion or predicate admission retained its actual stop cause.
    #[error("probability certificate subject stopped: {0:?}")]
    Build(Box<crate::ExactProbBuildIssue>),
    /// Required ranks fail their independently checked progress condition.
    #[error("probability certificate rank broken at {0:?}")]
    RankBroken(ProbabilityProductKey),
    /// A rebuilt state has no required rank row.
    #[error("probability certificate rank missing at {0:?}")]
    RankMissing(ProbabilityProductKey),
}

impl From<ExactProbabilityGraphError> for ProbabilityDocumentError {
    fn from(error: ExactProbabilityGraphError) -> Self {
        match error {
            ExactProbabilityGraphError::Cancelled(cause) => Self::Cancelled(cause),
            ExactProbabilityGraphError::Cycle
            | ExactProbabilityGraphError::UnsupportedDraw
            | ExactProbabilityGraphError::UnsupportedInitial
            | ExactProbabilityGraphError::UnsupportedBinding
            | ExactProbabilityGraphError::UnsupportedClaimForm => Self::Unsupported,
            other => Self::Graph(other),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireIdentity {
    node_id: Value,
    model: Option<String>,
    ordinal: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObjective {
    profile: String,
    targets: Vec<String>,
    optimization: String,
    fairness: Vec<Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRational {
    numerator: String,
    denominator: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireValue {
    state: String,
    value: WireRational,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRank {
    state: String,
    rank: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDocument {
    identity: WireIdentity,
    objective: WireObjective,
    kind: String,
    values: Vec<WireValue>,
    ranking: Vec<WireRank>,
}

fn poll(cancel: &Cancel) -> Result<(), ProbabilityDocumentError> {
    poll_cancel(cancel).map_err(Into::into)
}

fn canonical<T: Serialize>(value: &T, bytes: u64) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let value = serde_json::to_value(value).map_err(|_| ProbabilityDocumentError::Malformed)?;
    quire_canonical::to_vec(&value, quire_canonical::Limits::new(bytes))
        .map_err(|error| match error {
            quire_canonical::Error::Limit(limit) => exceeded("certificate.bytes", limit.bound, limit.required),
            _ => ProbabilityDocumentError::Malformed,
        })
}

fn exceeded(setting: &'static str, bound: u64, actual: u64) -> ProbabilityDocumentError {
    ProbabilityDocumentError::Limit { setting, bound, actual }
}

fn decode_key(text: &str) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != text {
        return Err(ProbabilityDocumentError::Malformed);
    }
    quire_canonical::read(&bytes, u64::MAX).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let key: Value =
        serde_json::from_slice(&bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if canonical(&key, u64::MAX)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let parts = key.as_array().ok_or(ProbabilityDocumentError::Malformed)?;
    if parts.len() != 6
        || parts.first().and_then(Value::as_str) != Some("quire.probability-product-key/v1")
        || !parts.get(2).is_some_and(Value::is_string)
        || parts.iter().skip(3).any(|part| !part.is_null())
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let model = parts
        .get(2)
        .and_then(Value::as_str)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let model_bytes = URL_SAFE_NO_PAD
        .decode(model)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if URL_SAFE_NO_PAD.encode(&model_bytes) != model {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
}

fn ordered(
    keys: impl IntoIterator<Item = ProbabilityProductKey>,
) -> Result<Vec<ProbabilityProductKey>, ProbabilityDocumentError> {
    let mut result = Vec::new();
    for key in keys {
        if result.last().is_some_and(|previous| previous >= &key) {
            return Err(ProbabilityDocumentError::Malformed);
        }
        result.push(key);
    }
    Ok(result)
}

fn ratio(wire: WireRational, bits: u64) -> Result<Rational, ProbabilityDocumentError> {
    let numerator: Integer = wire
        .numerator
        .parse()
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    let denominator: Integer = wire
        .denominator
        .parse()
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if numerator.to_string() != wire.numerator
        || denominator.to_string() != wire.denominator
        || denominator <= Integer::from(0_i64)
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
    if numerator.as_big().bits() > bits || denominator.as_big().bits() > bits {
        return Err(exceeded("exact_prob.max_rational_bits", bits, numerator.as_big().bits().max(denominator.as_big().bits())));
    }
    let value =
        Rational::new(numerator, denominator).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if value.numerator().to_string() != wire.numerator
        || value.denominator().to_string() != wire.denominator
        || value < rational(0)
        || value > rational(1)
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(value)
}

/// Read strictly canonical bytes without normalizing malformed evidence.
/// This admits representation only; acceptance requires checking a subject.
pub fn read_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityCertificateDocument, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded("certificate.bytes", limits.bytes, u64::try_from(bytes.len()).unwrap_or(u64::MAX)));
    }
    let parsed = quire_canonical::read_bounded(bytes, limits.bytes, limits.storage_bytes).map_err(|error| match error {
        quire_canonical::ReadError::Limit(limit) => exceeded("certificate.storage_bytes", limit.bound, limit.required),
        _ => ProbabilityDocumentError::Malformed,
    })?;
    if parsed.root().get("ranking").is_none() {
        return Err(ProbabilityDocumentError::MissingMember("ranking"));
    }
    let wire: WireDocument =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if canonical(&wire, limits.bytes)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    if wire.kind != "exact"
        || wire.objective.profile != PROFILE
        || !wire.objective.fairness.is_empty()
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let optimization = match wire.objective.optimization.as_str() {
        "minimum" => ProbabilityObjective::Minimum,
        "maximum" => ProbabilityObjective::Maximum,
        "workload" => ProbabilityObjective::Workload,
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    if wire.values.len() > limits.states
        || wire.ranking.len() > limits.states
        || wire.objective.targets.len() > limits.states
    {
        return Err(exceeded("exact_prob.max_states", u64::try_from(limits.states).unwrap_or(u64::MAX), u64::try_from(wire.values.len().max(wire.ranking.len()).max(wire.objective.targets.len())).unwrap_or(u64::MAX)));
    }
    let node = qsl_foundation::node_wire::node_key_from_json(&wire.identity.node_id)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    let identity = ObligationIdentity::new(
        node,
        wire.identity.model.map(crate::ModelIdentity::new),
        wire.identity.ordinal,
    );
    let targets = ordered(
        wire.objective
            .targets
            .iter()
            .map(|key| decode_key(key))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
    let mut values = Vec::new();
    for row in wire.values {
        poll(cancel)?;
        values.push(ExactProbabilityValue {
            state: decode_key(&row.state)?,
            value: ratio(row.value, limits.scalar_bits)?,
        });
    }
    ordered(values.iter().map(|row| row.state.clone()))?;
    let mut ranking = Vec::new();
    for row in wire.ranking {
        poll(cancel)?;
        if row.rank > MAX_COUNT {
            return Err(ProbabilityDocumentError::Malformed);
        }
        ranking.push((decode_key(&row.state)?, row.rank));
    }
    ordered(ranking.iter().map(|(state, _)| state.clone()))?;
    poll(cancel)?;
    Ok(ProbabilityCertificateDocument {
        identity,
        targets,
        optimization,
        values,
        ranking,
    })
}

/// Write the single RFC 8785 representation admitted by the profile reader.
pub fn write_probability_certificate(
    document: &ProbabilityCertificateDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    poll(cancel)?;
    let wire = WireDocument {
        identity: WireIdentity {
            node_id: qsl_foundation::node_wire::node_key_to_json(document.identity.node())
                .map_err(|_| ProbabilityDocumentError::Malformed)?,
            model: document
                .identity
                .model()
                .map(|model| model.as_str().to_owned()),
            ordinal: document.identity.ordinal(),
        },
        objective: WireObjective {
            profile: PROFILE.to_owned(),
            targets: document
                .targets
                .iter()
                .map(|key| URL_SAFE_NO_PAD.encode(key.as_bytes()))
                .collect(),
            optimization: match document.optimization {
                ProbabilityObjective::Minimum => "minimum",
                ProbabilityObjective::Maximum => "maximum",
                ProbabilityObjective::Workload => "workload",
            }
            .to_owned(),
            fairness: Vec::new(),
        },
        kind: "exact".to_owned(),
        values: document
            .values
            .iter()
            .map(|row| WireValue {
                state: URL_SAFE_NO_PAD.encode(row.state.as_bytes()),
                value: WireRational {
                    numerator: row.value.numerator().to_string(),
                    denominator: row.value.denominator().to_string(),
                },
            })
            .collect(),
        ranking: document
            .ranking
            .iter()
            .map(|(state, rank)| WireRank {
                state: URL_SAFE_NO_PAD.encode(state.as_bytes()),
                rank: *rank,
            })
            .collect(),
    };
    let bytes = canonical(&wire, limits.bytes)?;
    read_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

fn subject_graph(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(CheckerGraph, ProbabilityObjective, Bound), ProbabilityDocumentError> {
    use qsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    let claim = bind_claim(claims, item, cancel)?;
    if !claim.fairness.is_empty()
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(predicate),
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let binding = claim_binding(system, claim)?;
    let mut issue = None;
    let graph = enumerate_model_graph(
        system,
        predicate,
        binding.as_ref().map(|bound| &bound.reference),
        limits.states,
        limits.edges,
        cancel,
        true,
        &mut |_, _| {},
        &mut |failure| { issue = Some(failure); },
    ).map_err(|error| match issue {
        Some(issue) => ProbabilityDocumentError::Build(Box::new(issue)),
        None => match error {
            ExactProbabilityGraphError::LimitReached => {
                let bound = u64::try_from(limits.states).unwrap_or(u64::MAX);
                exceeded("exact_prob.max_states", bound, bound.saturating_add(1))
            },
            ExactProbabilityGraphError::EdgeLimitReached => {
                let bound = u64::try_from(limits.edges).unwrap_or(u64::MAX);
                exceeded("exact_prob.max_transitions", bound, bound.saturating_add(1))
            },
            other => other.into(),
        },
    })?;
    topological_order(&graph, cancel)?;
    Ok((
        graph,
        claim_objective(&claim.scheduler, bound),
        bound.clone(),
    ))
}

fn target_keys(graph: &CheckerGraph) -> Vec<ProbabilityProductKey> {
    graph
        .states
        .iter()
        .filter(|(_, state)| state.terminal == Some(true))
        .map(|(key, _)| key.clone())
        .collect()
}

/// Build a transport document from solver values and independently derived
/// target keys and acyclic height ranks. This does not grant acceptance.
pub fn probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityCertificateDocument, ProbabilityDocumentError> {
    let (graph, optimization, _) = subject_graph(system, claims, item, limits, cancel)?;
    let mut ranks = BTreeMap::<ProbabilityProductKey, u64>::new();
    for key in topological_order(&graph, cancel)?.into_iter().rev() {
        poll(cancel)?;
        let state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let mut rank = 0;
        if state.terminal.is_none() {
            for edge in state
                .choices
                .iter()
                .flatten()
                .filter(|edge| edge.probability > rational(0))
            {
                let successor = ranks
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                rank = rank.max(
                    successor
                        .checked_add(1)
                        .filter(|rank| *rank <= MAX_COUNT)
                        .ok_or(exceeded("certificate.rank", MAX_COUNT, MAX_COUNT + 1))?,
                );
            }
        }
        ranks.insert(key, rank);
    }
    Ok(ProbabilityCertificateDocument {
        identity: item.clone(),
        targets: target_keys(&graph),
        optimization,
        values: certificate.values.clone(),
        ranking: ranks.into_iter().collect(),
    })
}

/// Read and independently check document evidence against a checked subject.
/// Identity is compared before enumeration; targets and operator come solely
/// from the checked claim. The opaque acceptance token is minted only after
/// coverage, required rank progress, Bellman equality and threshold checks.
pub fn check_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let document = read_probability_certificate(bytes, limits, cancel)?;
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let (graph, objective, bound) = subject_graph(system, claims, item, limits, cancel)?;
    if objective != document.optimization || target_keys(&graph) != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let ranks: BTreeMap<_, _> = document.ranking.into_iter().collect();
    for key in graph.states.keys() {
        if !ranks.contains_key(key) { return Err(ProbabilityDocumentError::RankMissing(key.clone())); }
    }
    if ranks.len() != graph.states.len() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let certificate = ExactProbabilityCertificate {
        values: document.values,
    };
    let values = match certificate_values(&graph, &certificate, cancel)? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Rejected(rejection) => {
            return Ok(ExactProbabilityCertificateCheck::Rejected(rejection))
        }
    };
    for (key, state) in &graph.states {
        poll(cancel)?;
        if state.terminal.is_some() || values.get(key).is_some_and(|value| *value == rational(0)) {
            continue;
        }
        let rank = ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let progress = state.choices.iter().flatten().any(|edge| {
            edge.probability > rational(0)
                && values
                    .get(&edge.target)
                    .is_some_and(|value| *value > rational(0))
                && ranks
                    .get(&edge.target)
                    .is_some_and(|successor| successor < rank)
        });
        if !progress {
            return Err(ProbabilityDocumentError::RankBroken(key.clone()));
        }
    }
    check_document_equalities(&graph, &values, objective, &bound, limits, cancel)
}

fn check_scalar(
    value: &Rational,
    limits: ProbabilityDocumentLimits,
) -> Result<(), ProbabilityDocumentError> {
    if value.numerator().as_big().bits() > limits.scalar_bits
        || value.denominator().as_big().bits() > limits.scalar_bits
    {
        Err(exceeded("exact_prob.max_rational_bits", limits.scalar_bits, value.numerator().as_big().bits().max(value.denominator().as_big().bits())))
    } else {
        Ok(())
    }
}

fn check_document_equalities(
    graph: &CheckerGraph,
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    objective: ProbabilityObjective,
    bound: &Bound,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    for (key, state) in &graph.states {
        poll(cancel)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let expected = match state.terminal {
            Some(true) => rational(1),
            Some(false) => rational(0),
            None => {
                if state.choices.is_empty()
                    || objective == ProbabilityObjective::Workload && state.choices.len() != 1
                {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let mut expected: Option<Rational> = None;
                for choice in &state.choices {
                    let mut sum = rational(0);
                    let mut mass = rational(0);
                    for edge in choice {
                        poll(cancel)?;
                        if edge.probability < rational(0) {
                            return Err(ProbabilityDocumentError::Malformed);
                        }
                        check_scalar(&edge.probability, limits)?;
                        let next = values
                            .get(&edge.target)
                            .ok_or(ProbabilityDocumentError::Malformed)?;
                        let product = arithmetic(
                            quire_exact::RationalArithmetic::Multiply(&edge.probability, next),
                            &mut meter,
                            cancel,
                        )?;
                        check_scalar(&product, limits)?;
                        mass = arithmetic(
                            quire_exact::RationalArithmetic::Add(&mass, &edge.probability),
                            &mut meter,
                            cancel,
                        )?;
                        sum = arithmetic(
                            quire_exact::RationalArithmetic::Add(&sum, &product),
                            &mut meter,
                            cancel,
                        )?;
                        check_scalar(&mass, limits)?;
                        check_scalar(&sum, limits)?;
                    }
                    if mass != rational(1) {
                        return Err(ProbabilityDocumentError::Malformed);
                    }
                    expected = Some(match expected {
                        None => sum,
                        Some(previous) => match objective {
                            ProbabilityObjective::Minimum => previous.min(sum),
                            ProbabilityObjective::Maximum => previous.max(sum),
                            ProbabilityObjective::Workload => {
                                return Err(ProbabilityDocumentError::Unsupported)
                            }
                        },
                    });
                }
                expected.ok_or(ProbabilityDocumentError::Malformed)?
            }
        };
        if value != &expected {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                key.clone(),
            ));
        }
    }
    let initial = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let threshold = match bound {
        Bound::AtLeast(value) | Bound::AtMost(value) => value,
    };
    check_scalar(threshold, limits)?;
    let meets = match bound {
        Bound::AtLeast(threshold) => initial >= threshold,
        Bound::AtMost(threshold) => initial <= threshold,
    };
    if !meets {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    poll(cancel)?;
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            value: initial.clone(),
            side: match bound {
                Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
                Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
            },
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn limits() -> ProbabilityDocumentLimits {
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 100,
            edges: 100,
            scalar_bits: 128,
            accounting: crate::AccountingLimits::default().0,
        }
    }
    fn document() -> ProbabilityCertificateDocument {
        let key = ProbabilityProductKey::for_model_state(None, b"model").unwrap();
        ProbabilityCertificateDocument {
            identity: ObligationIdentity::new(crate::fixture_node(1), None, 0),
            targets: vec![key.clone()],
            optimization: ProbabilityObjective::Minimum,
            values: vec![ExactProbabilityValue {
                state: key.clone(),
                value: rational(1),
            }],
            ranking: vec![(key, 0)],
        }
    }
    fn bytes() -> Vec<u8> {
        write_probability_certificate(&document(), limits(), &Cancel::new()).unwrap()
    }
    fn mutate(change: impl FnOnce(&mut Value)) -> Vec<u8> {
        let mut value: Value = serde_json::from_slice(&bytes()).unwrap();
        change(&mut value);
        canonical(&value, limits().bytes).unwrap()
    }

    #[trace("FR-201-AC-9")]
    #[test]
    fn closed_exact_document_preserves_every_component_and_canonical_bytes() {
        let original = document();
        let bytes = write_probability_certificate(&original, limits(), &Cancel::new()).unwrap();
        let read = read_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(read, original);
        assert_eq!(
            write_probability_certificate(&read, limits(), &Cancel::new()).unwrap(),
            bytes
        );
    }

    #[trace("FR-201-AC-9")]
    #[test]
    fn every_row_collection_requires_strict_complete_key_order() {
        let mut document = document();
        let second = ProbabilityProductKey::for_model_state(None, b"other-model").unwrap();
        document.targets.push(second.clone());
        document.targets.sort();
        document.values.push(ExactProbabilityValue { state: second.clone(), value: rational(0) });
        document.values.sort_by(|left, right| left.state.cmp(&right.state));
        document.ranking.push((second, 0));
        document.ranking.sort();
        let bytes = write_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
        for member in ["targets", "values", "ranking"] {
            for duplicate in [false, true] {
                let mut value: Value = serde_json::from_slice(&bytes).unwrap();
                let rows = if member == "targets" { &mut value["objective"][member] } else { &mut value[member] };
                let rows = rows.as_array_mut().unwrap();
                if duplicate { let row = rows[0].clone(); rows.insert(0, row); } else { rows.reverse(); }
                let changed = canonical(&value, limits().bytes).unwrap();
                assert_eq!(read_probability_certificate(&changed, limits(), &Cancel::new()), Err(ProbabilityDocumentError::Malformed), "{member} duplicate={duplicate}");
            }
        }
    }

    #[trace("FR-201-AC-11")]
    #[test]
    fn bellman_arithmetic_charges_shared_work_and_result_storage() {
        let root = ProbabilityProductKey::for_model_state(None, b"root").unwrap();
        let target = ProbabilityProductKey::for_model_state(None, b"target").unwrap();
        let graph = CheckerGraph { initial: root.clone(), states: BTreeMap::from([
            (root.clone(), CheckerState { terminal: None, choices: vec![vec![ProbabilityEdge { target: target.clone(), probability: rational(1) }]] }),
            (target.clone(), CheckerState { terminal: Some(true), choices: Vec::new() }),
        ]) };
        let values = BTreeMap::from([(root, rational(1)), (target, rational(1))]);
        let check = |limits| check_document_equalities(&graph, &values, ProbabilityObjective::Minimum, &Bound::AtLeast(rational(1)), limits, &Cancel::new());
        assert!(matches!(check(limits()).unwrap(), ExactProbabilityCertificateCheck::Accepted(_)));
        let mut work = limits(); work.accounting.work_units = 0;
        let mut storage = limits(); storage.accounting.result_units = 0;
        for (limits, counter) in [(work, quire_exact::LimitKind::WorkUnits), (storage, quire_exact::LimitKind::ResultUnits)] {
            let Err(ProbabilityDocumentError::Accounting(incomplete)) = check(limits) else { panic!("the actual shared arithmetic charge must stop"); };
            assert_eq!(incomplete.limit_kind, counter);
            assert_eq!(incomplete.limit, 0);
        }
    }

    #[trace("FR-201-AC-9")]
    #[test]
    fn malformed_rows_rationals_fields_and_keys_are_not_normalized() {
        let mutations: Vec<Vec<u8>> = vec![
            mutate(|v| {
                let row = v["values"][0].clone();
                v["values"].as_array_mut().unwrap().push(row);
            }),
            mutate(|v| {
                v["values"][0]["value"] = json!({"numerator":"2", "denominator":"4"});
            }),
            mutate(|v| {
                v["values"][0]["value"]["denominator"] = json!("0");
            }),
            mutate(|v| {
                v["values"][0]["value"]["denominator"] = json!("-1");
            }),
            mutate(|v| {
                v["values"][0]["value"]["numerator"] = json!("01");
            }),
            mutate(|v| {
                v["values"][0]["value"]["numerator"] = json!("-0");
            }),
            mutate(|v| {
                v.as_object_mut().unwrap().remove("ranking");
            }),
            mutate(|v| {
                v["extra"] = json!(true);
            }),
            mutate(|v| {
                v["values"][0]["value"]["extra"] = json!(true);
            }),
            mutate(|v| {
                v["ranking"][0]["rank"] = json!(MAX_COUNT + 1);
            }),
            mutate(|v| {
                let key = v["values"][0]["state"].as_str().unwrap().to_owned();
                v["values"][0]["state"] = json!(format!("{key}="));
            }),
            mutate(|v| {
                v["values"][0]["state"] = json!(URL_SAFE_NO_PAD.encode(br#"["model-only"]"#));
            }),
        ];
        for bytes in mutations {
            assert!(
                read_probability_certificate(&bytes, limits(), &Cancel::new()).is_err(),
                "malformed evidence must refuse: {}",
                String::from_utf8_lossy(&bytes)
            );
        }
        let bytes = bytes();
        let duplicate = [b"{\"kind\":\"exact\",".as_slice(), bytes.get(1..).unwrap()].concat();
        assert!(read_probability_certificate(&duplicate, limits(), &Cancel::new()).is_err());
        let whitespace = [b" ".as_slice(), bytes.as_slice()].concat();
        assert_eq!(
            read_probability_certificate(&whitespace, limits(), &Cancel::new()),
            Err(ProbabilityDocumentError::Noncanonical)
        );
        assert!(read_probability_certificate(&[0xff], limits(), &Cancel::new()).is_err());
    }

    #[trace("FR-201-AC-9", "FR-201-AC-11")]
    #[test]
    fn unsupported_profiles_and_cancelled_or_exhausted_reads_have_no_evidence() {
        for bytes in [
            mutate(|v| {
                v["kind"] = json!("lower");
            }),
            mutate(|v| {
                v["objective"]["profile"] = json!("future/v2");
            }),
            mutate(|v| {
                v["objective"]["optimization"] = json!("guess");
            }),
        ] {
            assert_eq!(
                read_probability_certificate(&bytes, limits(), &Cancel::new()),
                Err(ProbabilityDocumentError::Unsupported)
            );
        }
        let bytes = bytes();
        assert_eq!(
            read_probability_certificate(
                &bytes,
                ProbabilityDocumentLimits {
                    bytes: u64::try_from(bytes.len()).unwrap() - 1,
                    ..limits()
                },
                &Cancel::new()
            ),
            Err(exceeded("certificate.bytes", u64::try_from(bytes.len()).unwrap() - 1, u64::try_from(bytes.len()).unwrap()))
        );
        assert_eq!(
            read_probability_certificate(
                &bytes,
                ProbabilityDocumentLimits {
                    states: 0,
                    ..limits()
                },
                &Cancel::new()
            ),
            Err(exceeded("exact_prob.max_states", 0, 1))
        );
        assert_eq!(
            read_probability_certificate(
                &bytes,
                ProbabilityDocumentLimits {
                    scalar_bits: 0,
                    ..limits()
                },
                &Cancel::new()
            ),
            Err(exceeded("exact_prob.max_rational_bits", 0, 1))
        );
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            cancel.cancel(cause);
            assert_eq!(
                read_probability_certificate(&bytes, limits(), &cancel),
                Err(ProbabilityDocumentError::Cancelled(cause))
            );
        }
    }
}

fn arithmetic(
    operation: quire_exact::RationalArithmetic<'_>,
    meter: &mut quire_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityDocumentError> {
    poll(cancel)?;
    match quire_exact::evaluate_rational_arithmetic(operation, None, meter) {
        quire_exact::Outcome::Completed(value) => Ok(value),
        quire_exact::Outcome::Incomplete(incomplete) => {
            poll(cancel)?;
            Err(ProbabilityDocumentError::Accounting(Box::new(incomplete)))
        }
        quire_exact::Outcome::Undefined(_) | quire_exact::Outcome::Refused(_) => {
            Err(ProbabilityDocumentError::Malformed)
        }
    }
}
