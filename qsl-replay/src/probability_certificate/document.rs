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
    /// Maximum checker graph states and certificate rows.
    pub states: usize,
    /// Maximum checker graph edges.
    pub edges: usize,
    /// Maximum numerator or denominator bits.
    pub scalar_bits: u64,
}

/// A document never becomes accepted evidence when one of these occurs.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProbabilityDocumentError {
    /// Input is malformed or violates the closed wire contract.
    #[error("invalid_runtime_input: malformed probability certificate")]
    Malformed,
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
    #[error("probability certificate limit reached: {0}")]
    Limit(&'static str),
    /// Cancellation retains the caller's cause.
    #[error("probability certificate cancelled: {0:?}")]
    Cancelled(CancelCause),
    /// Independent subject admission or graph traversal stopped.
    #[error("probability certificate graph stopped: {0:?}")]
    Graph(ExactProbabilityGraphError),
    /// Required ranks fail their independently checked progress condition.
    #[error("probability certificate rank broken at {0:?}")]
    RankBroken(ProbabilityProductKey),
}

impl From<ExactProbabilityGraphError> for ProbabilityDocumentError {
    fn from(error: ExactProbabilityGraphError) -> Self {
        match error {
            ExactProbabilityGraphError::Cancelled(cause) => Self::Cancelled(cause),
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
        .map_err(|_| ProbabilityDocumentError::Limit("certificate.bytes"))
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
        return Err(ProbabilityDocumentError::Limit("certificate.scalar_bits"));
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
        return Err(ProbabilityDocumentError::Limit("certificate.bytes"));
    }
    quire_canonical::read(bytes, limits.bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
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
        return Err(ProbabilityDocumentError::Limit("certificate.states"));
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
    if !values
        .iter()
        .map(|row| &row.state)
        .eq(ranking.iter().map(|(state, _)| state))
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
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
    let graph = enumerate_model_graph(
        system,
        predicate,
        binding.as_ref().map(|bound| &bound.reference),
        limits.states,
        limits.edges,
        cancel,
        true,
        &mut |_, _| {},
        &mut |_| {},
    )?;
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
                        .ok_or(ProbabilityDocumentError::Limit("certificate.rank"))?,
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
    if !ranks.keys().eq(graph.states.keys()) {
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
    check_exact_reachability(&graph, &certificate, objective, &bound, cancel).map_err(Into::into)
}
