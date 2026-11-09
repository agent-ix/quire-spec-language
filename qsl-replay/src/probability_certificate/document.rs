// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed exact reachability document profile and strict JCS codec.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use super::*;

const PROFILE: &str = "quire.probability-objective/reachability/v1";
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// Caller-owned byte and working-storage ceilings for certificate transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbabilityDocumentIoLimits {
    /// Maximum canonical certificate document bytes.
    pub bytes: u64,
    /// Maximum working bytes reserved by the canonical JSON reader.
    pub storage_bytes: u64,
}

impl Default for ProbabilityDocumentIoLimits {
    fn default() -> Self {
        let bytes = crate::CertificateCheckLimits::default().max_certificate_bytes;
        Self { bytes, storage_bytes: bytes }
    }
}

/// A distinct exhausted ceiling in the closed certificate profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityDocumentLimit {
    /// Canonical input/output bytes.
    Bytes,
    /// JSON reader working-storage bytes.
    StorageBytes,
    /// Reachable product states or supplied state rows.
    States,
    /// Positive-probability product edges.
    Edges,
    /// Exact rational numerator or denominator bits.
    RationalBits,
    /// The profile's maximum representable progress rank.
    Rank,
}

impl ProbabilityDocumentLimit {
    /// Shared incomplete-result vocabulary for this actual exhausted ceiling.
    pub fn model_limit(self) -> crate::ModelCheckLimit {
        match self {
            Self::Bytes => crate::ModelCheckLimit::CertificateMaxBytes,
            Self::StorageBytes => crate::ModelCheckLimit::CertificateMaxWorkingBytes,
            Self::States => crate::ModelCheckLimit::ExactProbMaxStates,
            Self::Edges => crate::ModelCheckLimit::ExactProbMaxTransitions,
            Self::RationalBits => crate::ModelCheckLimit::ExactProbMaxRationalBits,
            Self::Rank => crate::ModelCheckLimit::CertificateMaxRank,
        }
    }

    /// The configured setting named by a stopped check.
    pub fn setting(self) -> &'static str { self.model_limit().setting() }
}

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
    #[error("probability certificate limit reached: {} ({actual} > {bound})", limit.setting())]
    Limit {
        /// Exhausted setting.
        limit: ProbabilityDocumentLimit,
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
    /// Decimal text proves that an integer exceeds the bit ceiling before parsing it.
    #[error("exact_prob.max_rational_bits exhausted: {decimal_digits} decimal digits exceed the {bit_bound}-bit ceiling")]
    IntegerText {
        /// Configured numerator/denominator bit ceiling.
        bit_bound: u64,
        /// Offered canonical decimal digit count; no large integer was constructed.
        decimal_digits: u64,
    },
    /// Independent subject admission or graph traversal stopped.
    #[error("probability certificate graph stopped: {0:?}")]
    Graph(ExactProbabilityGraphError),
    /// Subject expansion or predicate admission retained its actual stop cause.
    #[error("probability certificate subject stopped: {0:?}")]
    Build(Box<crate::ExactProbBuildIssue>),
}

impl From<ExactProbabilityGraphError> for ProbabilityDocumentError {
    fn from(error: ExactProbabilityGraphError) -> Self {
        match error {
            ExactProbabilityGraphError::Cancelled(cause) => Self::Cancelled(cause),
            ExactProbabilityGraphError::ClaimNotBound => Self::IdentityMismatch,
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
    node_id: Box<serde_json::value::RawValue>,
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
    let mut counter = EncodedSize { used: 0, bound: bytes, required: None };
    if serde_json::to_writer(&mut counter, value).is_err() {
        return Err(match counter.required {
            Some(actual) => exceeded(ProbabilityDocumentLimit::Bytes, bytes, actual),
            None => ProbabilityDocumentError::Malformed,
        });
    }
    let encoded = serde_json::to_vec(value).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let parsed = quire_canonical::read(&encoded, u64::MAX)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    quire_canonical::to_vec(&parsed, quire_canonical::Limits::new(bytes)).map_err(|error| {
        match error {
            quire_canonical::Error::Limit(limit) => {
                exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
            }
            _ => ProbabilityDocumentError::Malformed,
        }
    })
}

struct EncodedSize {
    used: u64,
    bound: u64,
    required: Option<u64>,
}

impl std::io::Write for EncodedSize {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let required = self.used.saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        if required > self.bound {
            self.required = Some(required);
            return Err(std::io::Error::other("certificate byte ceiling"));
        }
        self.used = required;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

fn exceeded(limit: ProbabilityDocumentLimit, bound: u64, actual: u64) -> ProbabilityDocumentError {
    ProbabilityDocumentError::Limit {
        limit,
        bound,
        actual,
    }
}

fn decode_key(text: &str) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != text {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let key =
        quire_canonical::read(&bytes, u64::MAX).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if quire_canonical::to_vec(&key, quire_canonical::Limits::new(u64::MAX))
        .map_err(|_| ProbabilityDocumentError::Malformed)?
        != bytes
    {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let quire_canonical::Node::Array(parts) = key.root().node() else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    let parts = parts.collect::<Vec<_>>();
    if parts.len() != 6
        || !parts.first().is_some_and(|part| {
            matches!(
                part.node(),
                quire_canonical::Node::String("quire.probability-product-key/v1")
            )
        })
        || parts
            .iter()
            .skip(3)
            .any(|part| !matches!(part.node(), quire_canonical::Node::Null))
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let Some(quire_canonical::Node::String(model)) = parts.get(2).map(|part| part.node()) else {
        return Err(ProbabilityDocumentError::Malformed);
    };
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

fn ratio(wire: WireRational, bits: u64, meter: &mut quire_exact::Meter, cancel: &Cancel) -> Result<Rational, ProbabilityDocumentError> {
    for text in [&wire.numerator, &wire.denominator] {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit())
            || text.len() > 1 && text.starts_with('0') {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let digits = u64::try_from(text.len()).unwrap_or(u64::MAX);
        // 10^(d-1) >= 8^(d-1), so d digits require at least 3(d-1)+1 bits.
        // Single digits use the kernel's inline parser without large storage.
        let minimum_bits = digits.saturating_sub(1).saturating_mul(3).saturating_add(1);
        if minimum_bits > bits && digits > 1 {
            return Err(ProbabilityDocumentError::IntegerText { bit_bound: bits, decimal_digits: digits });
        }
    }
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
    if numerator.magnitude_bits() > bits || denominator.magnitude_bits() > bits {
        return Err(exceeded(
            ProbabilityDocumentLimit::RationalBits,
            bits,
            numerator.magnitude_bits().max(denominator.magnitude_bits()),
        ));
    }
    let value = arithmetic(quire_exact::RationalArithmetic::Divide(
        &Rational::from_integer(numerator), &Rational::from_integer(denominator)), meter, cancel)?;
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
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    read_document(bytes, limits, cancel, &mut meter)
}

fn read_document(
    bytes: &[u8], limits: ProbabilityDocumentLimits, cancel: &Cancel, meter: &mut quire_exact::Meter,
) -> Result<ProbabilityCertificateDocument, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    let parsed = quire_canonical::read_bounded(bytes, limits.bytes, limits.storage_bytes).map_err(
        |error| match error {
            quire_canonical::ReadError::Limit(limit) => {
                exceeded(ProbabilityDocumentLimit::StorageBytes, limit.bound, limit.required)
            }
            _ => ProbabilityDocumentError::Malformed,
        },
    )?;
    if parsed.root().get("ranking").is_none() {
        return Err(ProbabilityDocumentError::MissingMember("ranking"));
    }
    let root = parsed.root();
    let rows = [root.get("values"), root.get("ranking"), root.get("objective").and_then(|objective| objective.get("targets"))]
        .into_iter().filter_map(|node| match node?.node() {
            quire_canonical::Node::Array(items) => Some(items.count()),
            _ => None,
        }).max().unwrap_or(0);
    if rows > limits.states {
        return Err(exceeded(ProbabilityDocumentLimit::States, u64::try_from(limits.states).unwrap_or(u64::MAX), u64::try_from(rows).unwrap_or(u64::MAX)));
    }
    drop(parsed);
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
    let node = qsl_foundation::node_wire::node_key_from_text(wire.identity.node_id.get())
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
            value: ratio(row.value, limits.scalar_bits, meter, cancel)?,
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
            node_id: serde_json::value::RawValue::from_string(
                qsl_foundation::node_wire::node_key_to_text(document.identity.node())
                    .map_err(|_| ProbabilityDocumentError::Malformed)?,
            )
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
    if !std::ptr::eq(
        system.package().graph().probabilistic_model().claims(),
        claims,
    ) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
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
        claim_workload(system, claim)?,
        limits.states,
        limits.edges,
        cancel,
        true,
        &mut |_, _| {},
        &mut |failure| {
            issue = Some(failure);
        },
    )
    .map_err(|error| match issue {
        Some(issue) => ProbabilityDocumentError::Build(Box::new(issue)),
        None => match error {
            ExactProbabilityGraphError::LimitReached => {
                let bound = u64::try_from(limits.states).unwrap_or(u64::MAX);
                exceeded(ProbabilityDocumentLimit::States, bound, bound.saturating_add(1))
            }
            ExactProbabilityGraphError::EdgeLimitReached => {
                let bound = u64::try_from(limits.edges).unwrap_or(u64::MAX);
                exceeded(ProbabilityDocumentLimit::Edges, bound, bound.saturating_add(1))
            }
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
                        .ok_or(exceeded(ProbabilityDocumentLimit::Rank, MAX_COUNT, MAX_COUNT + 1))?,
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
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = match read_document(bytes, limits, cancel, &mut meter) {
        Ok(document) => document,
        Err(error) => return document_admission_error(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let (graph, objective, bound) = subject_graph(system, claims, item, limits, cancel)?;
    if objective != document.optimization || target_keys(&graph) != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let ranks: BTreeMap<_, _> = document.ranking.into_iter().collect();
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
        if !ranks.contains_key(key) {
            return Ok(if state.terminal.is_none() && values.get(key).is_some_and(|value| *value > rational(0)) {
                rejected(ProbabilityCertificateRule::RankMissing, key.clone())
            } else { malformed_ranking() });
        }
    }
    if ranks.len() != graph.states.len() { return Ok(malformed_ranking()); }
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
            return Ok(rejected(ProbabilityCertificateRule::RankBroken, key.clone()));
        }
    }
    check_document_equalities(&graph, &values, objective, &bound, limits, cancel, &mut meter)
}

fn malformed_ranking() -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::Malformed,
        at: ProbabilityCertificateLocus::Member("ranking"),
    })
}

pub(super) fn document_admission_error(error: ProbabilityDocumentError) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    match error {
        ProbabilityDocumentError::MissingMember("ranking") => Ok(malformed_ranking()),
        other => Err(other),
    }
}

fn check_scalar(
    value: &Rational,
    limits: ProbabilityDocumentLimits,
) -> Result<(), ProbabilityDocumentError> {
    if value.numerator().magnitude_bits() > limits.scalar_bits
        || value.denominator().magnitude_bits() > limits.scalar_bits
    {
        Err(exceeded(
            ProbabilityDocumentLimit::RationalBits,
            limits.scalar_bits,
            value
                .numerator()
                .as_big()
                .bits()
                .max(value.denominator().magnitude_bits()),
        ))
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
    meter: &mut quire_exact::Meter,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
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
                            meter,
                            cancel,
                        )?;
                        check_scalar(&product, limits)?;
                        mass = arithmetic(
                            quire_exact::RationalArithmetic::Add(&mass, &edge.probability),
                            meter,
                            cancel,
                        )?;
                        sum = arithmetic(
                            quire_exact::RationalArithmetic::Add(&sum, &product),
                            meter,
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

fn arithmetic(
    operation: quire_exact::RationalArithmetic<'_>,
    meter: &mut quire_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityDocumentError> {
    poll(cancel)?;
    match quire_exact::evaluate_rational_arithmetic(operation, None, meter) {
        quire_exact::Outcome::Completed(value) => Ok(value),
        quire_exact::Outcome::Incomplete(incomplete) => match cancel.tripped() {
            Some(cause) => Err(ProbabilityDocumentError::Cancelled(cause)),
            None => Err(ProbabilityDocumentError::Accounting(Box::new(incomplete))),
        },
        quire_exact::Outcome::Undefined(_) | quire_exact::Outcome::Refused(_) => {
            Err(ProbabilityDocumentError::Malformed)
        }
    }
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
    fn structural_identity_nesting_uses_the_shared_iterative_codec() {
        let mut node = crate::fixture_node(1);
        for ordinal in 1..=200 {
            node = quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Node(node)],
                ordinal,
            )
            .unwrap();
        }
        let mut document = document();
        document.identity = ObligationIdentity::new(node, None, 0);
        let bytes = write_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
        assert_eq!(
            read_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap(),
            document
        );
    }

    #[trace("FR-201-AC-11", "FR-282-AC-12")]
    #[test]
    fn storage_and_rational_text_are_admitted_before_rows_or_large_integers() {
        let bytes = bytes();
        let bound = u64::try_from(bytes.len()).unwrap();
        let error = read_probability_certificate(&bytes, ProbabilityDocumentLimits { storage_bytes: bound, ..limits() }, &Cancel::new()).unwrap_err();
        let ProbabilityDocumentError::Limit { limit, bound: actual_bound, actual } = error else { panic!("the bounded canonical reader must refuse its working allocation"); };
        assert_eq!(limit, ProbabilityDocumentLimit::StorageBytes);
        assert_eq!(actual_bound, bound);
        assert!(actual > bound);
        let text = format!("1{}", "0".repeat(100_000));
        let offered = mutate(|value| { value["values"][0]["value"]["numerator"] = json!(text); });
        assert_eq!(read_probability_certificate(&offered, ProbabilityDocumentLimits { scalar_bits: 16, ..limits() }, &Cancel::new()), Err(ProbabilityDocumentError::IntegerText { bit_bound: 16, decimal_digits: 100_001 }));
    }

    #[trace("FR-201-AC-9")]
    #[test]
    fn every_row_collection_requires_strict_complete_key_order() {
        let mut document = document();
        let second = ProbabilityProductKey::for_model_state(None, b"other-model").unwrap();
        document.targets.push(second.clone());
        document.targets.sort();
        document.values.push(ExactProbabilityValue {
            state: second.clone(),
            value: rational(0),
        });
        document
            .values
            .sort_by(|left, right| left.state.cmp(&right.state));
        document.ranking.push((second, 0));
        document.ranking.sort();
        let bytes = write_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
        for member in ["targets", "values", "ranking"] {
            for duplicate in [false, true] {
                let mut value: Value = serde_json::from_slice(&bytes).unwrap();
                let rows = if member == "targets" {
                    &mut value["objective"][member]
                } else {
                    &mut value[member]
                };
                let rows = rows.as_array_mut().unwrap();
                if duplicate {
                    let row = rows[0].clone();
                    rows.insert(0, row);
                } else {
                    rows.reverse();
                }
                let changed = canonical(&value, limits().bytes).unwrap();
                assert_eq!(
                    read_probability_certificate(&changed, limits(), &Cancel::new()),
                    Err(ProbabilityDocumentError::Malformed),
                    "{member} duplicate={duplicate}"
                );
            }
        }
    }

    #[trace("FR-201-AC-11")]
    #[test]
    fn bellman_arithmetic_charges_shared_work_and_result_storage() {
        let root = ProbabilityProductKey::for_model_state(None, b"root").unwrap();
        let target = ProbabilityProductKey::for_model_state(None, b"target").unwrap();
        let graph = CheckerGraph {
            initial: root.clone(),
            states: BTreeMap::from([
                (
                    root.clone(),
                    CheckerState {
                        terminal: None,
                        choices: vec![vec![ProbabilityEdge {
                            target: target.clone(),
                            probability: rational(1),
                        }]],
                    },
                ),
                (
                    target.clone(),
                    CheckerState {
                        terminal: Some(true),
                        choices: Vec::new(),
                    },
                ),
            ]),
        };
        let values = BTreeMap::from([(root, rational(1)), (target, rational(1))]);
        let check = |limits| {
            let cancel = Cancel::new();
            let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
            check_document_equalities(
                &graph,
                &values,
                ProbabilityObjective::Minimum,
                &Bound::AtLeast(rational(1)),
                limits,
                &cancel,
                &mut meter,
            )
        };
        assert!(matches!(
            check(limits()).unwrap(),
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        let mut work = limits();
        work.accounting.work_units = 0;
        let mut storage = limits();
        storage.accounting.result_units = 0;
        for (limits, counter) in [
            (work, quire_exact::LimitKind::WorkUnits),
            (storage, quire_exact::LimitKind::ResultUnits),
        ] {
            let Err(ProbabilityDocumentError::Accounting(incomplete)) = check(limits) else {
                panic!("the actual shared arithmetic charge must stop");
            };
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
            Err(exceeded(
                ProbabilityDocumentLimit::Bytes,
                u64::try_from(bytes.len()).unwrap() - 1,
                u64::try_from(bytes.len()).unwrap()
            ))
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
            Err(exceeded(ProbabilityDocumentLimit::States, 0, 1))
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
            Err(exceeded(ProbabilityDocumentLimit::RationalBits, 0, 1))
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
