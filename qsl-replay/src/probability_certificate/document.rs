// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed probability document profiles and strict JCS codec.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use super::*;

mod key;
mod long_run;
#[cfg(feature = "test-support")]
pub use long_run::{
    check_long_run_probability_certificate_observing,
    check_long_run_probability_certificate_with_identity, LongRunProbabilityPhase,
};
pub use long_run::{
    long_run_probability_certificate_document, read_long_run_probability_certificate,
    write_long_run_probability_certificate, LongRunProbabilityComponent,
    LongRunProbabilityDocument,
};
mod unit;
mod writer;
pub(super) use key::{model_product_key, model_product_key_with_budget};

const PROFILE: &str = "quire.probability-objective/reachability/v1";
const STATE_PROFILE: &str = "quire.probability-objective/state-predicate/v1";
const REWARD_PROFILE: &str = "quire.probability-objective/expected-reward/v1";
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// Caller-owned byte and working-storage ceilings for certificate transport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbabilityDocumentIoLimits {
    /// Maximum canonical certificate document bytes.
    pub bytes: u64,
    /// Maximum working bytes admitted by the canonical writer or reader.
    pub storage_bytes: u64,
}

impl Default for ProbabilityDocumentIoLimits {
    fn default() -> Self {
        let bytes = crate::CertificateCheckLimits::default().max_certificate_bytes;
        Self {
            bytes,
            storage_bytes: bytes,
        }
    }
}

/// A distinct exhausted ceiling in the closed certificate profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityDocumentLimit {
    /// Canonical input/output bytes.
    Bytes,
    /// Canonical writer or reader working-storage bytes.
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
    pub fn setting(self) -> &'static str {
        self.model_limit().setting()
    }
}

/// Direct infinite-trace event represented by state-predicate evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbabilityStateEvent {
    /// A holds until B, with B taking precedence.
    Until,
    /// P holds forever, including terminal stutter.
    Always,
}

/// Exact probability evidence, before independent semantic checking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityCertificateDocument {
    /// Structural claim identity.
    pub identity: ObligationIdentity,
    /// Complete, strictly ordered reachable target keys.
    pub targets: Vec<ProbabilityProductKey>,
    /// Direct state-predicate event; absent for reachability/v1.
    pub event: Option<ProbabilityStateEvent>,
    /// Absorbing failures for a direct state-predicate event.
    pub failures: Vec<ProbabilityProductKey>,
    /// Named reward and normalized comparison unit; absent for probabilities.
    pub reward: Option<ProbabilityRewardObjective>,
    /// Operator selected by the checked claim.
    pub optimization: ProbabilityObjective,
    /// Exact values, covering every reachable product state.
    pub values: Vec<ExactProbabilityValue>,
    /// Required positive-value progress ranks, covering every reachable state.
    pub ranking: Vec<(ProbabilityProductKey, u64)>,
}

/// FR-196's actual model-scoped reward identity and checked comparison unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityRewardObjective {
    /// Declaring model's structural node key.
    pub model: quire_exact::NodeKey,
    /// Checked reward identifier.
    pub name: quire_exact::Identifier,
    /// None for integer rewards; the admitted UnitId for quantities.
    pub unit: Option<quire_exact::UnitId>,
}

/// Caller ceilings for certificate admission and independent graph checking.
#[derive(Clone, Copy, Debug)]
pub struct ProbabilityDocumentLimits {
    /// Maximum document bytes, for both reading and writing.
    pub bytes: u64,
    /// Canonical JSON writer or reader working-storage ceiling in bytes.
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
    /// A structurally invalid evidence row group.
    #[error("invalid_runtime_input: malformed probability certificate member {0}")]
    MalformedMember(&'static str),
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
    /// Raw or full-product identity encoding retained its actual owner refusal.
    #[error(transparent)]
    KeyEncoding(qsl_eval::simulation::EncodingRefusal),
    /// The owner could not represent a requested construction size.
    #[error("resource_exhausted: model key construction size overflow")]
    KeySizeOverflow,
    /// A key budget denied admission without retaining its caller cause.
    #[error("runtime_invariant: model key denial has no caller cause")]
    KeyDenied,

    /// The checked state predicate is undefined, with its original cause.
    #[error("probability certificate predicate undefined: {0:?}")]
    UndefinedPredicate(qsl_eval::temporal::UndefinedCause),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event: Option<ProbabilityStateEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failures: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reward: Option<(Box<serde_json::value::RawValue>, String)>,
    #[serde(
        default,
        deserialize_with = "present_unit",
        skip_serializing_if = "Option::is_none"
    )]
    unit: Option<Box<serde_json::value::RawValue>>,
    optimization: String,
    fairness: Vec<Value>,
}

fn present_unit<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Box<serde_json::value::RawValue>>, D::Error> {
    Box::<serde_json::value::RawValue>::deserialize(deserializer).map(Some)
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
    let mut counter = EncodedSize {
        used: 0,
        bound: bytes,
        required: None,
    };
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
        let required = self
            .used
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        if required > self.bound {
            self.required = Some(required);
            return Err(std::io::Error::other("certificate byte ceiling"));
        }
        self.used = required;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn exceeded(limit: ProbabilityDocumentLimit, bound: u64, actual: u64) -> ProbabilityDocumentError {
    ProbabilityDocumentError::Limit {
        limit,
        bound,
        actual,
    }
}

fn decode_key(
    text: &str,
    limits: ProbabilityDocumentLimits,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != text {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let key = quire_canonical::read_bounded(&bytes, limits.bytes, limits.storage_bytes).map_err(
        |error| match error {
            quire_canonical::ReadError::Limit(limit) => exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                limit.bound,
                limit.required,
            ),
            _ => ProbabilityDocumentError::Malformed,
        },
    )?;
    if quire_canonical::to_vec(&key, quire_canonical::Limits::new(limits.bytes))
        .map_err(|_| ProbabilityDocumentError::Malformed)?
        != bytes
    {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let quire_canonical::Node::Array(parts) = key.root().node() else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    if parts.len() != 6 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let parts = parts.collect::<Vec<_>>();
    if parts.len() != 6
        || !parts.first().is_some_and(|part| {
            matches!(
                part.node(),
                quire_canonical::Node::String("quire.probability-product-key/v1")
            )
        })
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    key::admit_parts(&parts)?;
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

fn ratio(
    wire: WireRational,
    bits: u64,
    reward: bool,
    meter: &mut quire_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityDocumentError> {
    let value = ratio_checked(wire, bits, meter, cancel, !reward)?;
    if value < rational(0) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(value)
}

fn ratio_checked(
    wire: WireRational,
    bits: u64,
    meter: &mut quire_exact::Meter,
    cancel: &Cancel,
    probability: bool,
) -> Result<Rational, ProbabilityDocumentError> {
    for (index, text) in [&wire.numerator, &wire.denominator].into_iter().enumerate() {
        let text = if index == 0 && !probability {
            text.strip_prefix('-').unwrap_or(text)
        } else {
            text.as_str()
        };
        if text.is_empty()
            || !text.bytes().all(|byte| byte.is_ascii_digit())
            || text.len() > 1 && text.starts_with('0')
        {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let digits = u64::try_from(text.len()).unwrap_or(u64::MAX);
        // 10^(d-1) >= 8^(d-1), so d digits require at least 3(d-1)+1 bits.
        // Single digits use the kernel's inline parser without large storage.
        let minimum_bits = digits.saturating_sub(1).saturating_mul(3).saturating_add(1);
        if minimum_bits > bits && digits > 1 {
            return Err(ProbabilityDocumentError::IntegerText {
                bit_bound: bits,
                decimal_digits: digits,
            });
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
    let value = arithmetic(
        quire_exact::RationalArithmetic::Divide(
            &Rational::from_integer(numerator),
            &Rational::from_integer(denominator),
        ),
        meter,
        cancel,
    )?;
    if value.numerator().to_string() != wire.numerator
        || value.denominator().to_string() != wire.denominator
        || probability && (value < rational(0) || value > rational(1))
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
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
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
            quire_canonical::ReadError::Limit(limit) => exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                limit.bound,
                limit.required,
            ),
            _ => ProbabilityDocumentError::Malformed,
        },
    )?;
    let root = parsed.root();
    let objective = root.get("objective");
    let outside_profile = matches!(root.get("kind").map(|node| node.node()), Some(quire_canonical::Node::String(kind)) if kind != "exact")
        || matches!(objective.and_then(|node| node.get("profile")).map(|node| node.node()), Some(quire_canonical::Node::String(profile)) if profile != PROFILE && profile != STATE_PROFILE && profile != REWARD_PROFILE)
        || matches!(objective.and_then(|node| node.get("event")).map(|node| node.node()), Some(quire_canonical::Node::String(event)) if !matches!(event, "until" | "always"))
        || matches!(objective.and_then(|node| node.get("optimization")).map(|node| node.node()), Some(quire_canonical::Node::String(operator)) if !matches!(operator, "minimum" | "maximum" | "workload"))
        || match objective
            .and_then(|node| node.get("fairness"))
            .map(|node| node.node())
        {
            Some(quire_canonical::Node::Array(mut items)) => items.next().is_some(),
            _ => false,
        };
    if outside_profile {
        let canonical =
            quire_canonical::to_vec(&parsed, quire_canonical::Limits::new(limits.bytes))
                .map_err(|_| ProbabilityDocumentError::Malformed)?;
        return Err(if canonical == bytes {
            ProbabilityDocumentError::Unsupported
        } else {
            ProbabilityDocumentError::Noncanonical
        });
    }
    if parsed.root().get("ranking").is_none() {
        return Err(ProbabilityDocumentError::MissingMember("ranking"));
    }
    let root = parsed.root();
    let rows = [
        root.get("values"),
        root.get("ranking"),
        root.get("objective")
            .and_then(|objective| objective.get("failures")),
        root.get("objective")
            .and_then(|objective| objective.get("targets")),
    ]
    .into_iter()
    .filter_map(|node| match node?.node() {
        quire_canonical::Node::Array(items) => Some(items.count()),
        _ => None,
    })
    .max()
    .unwrap_or(0);
    if rows > limits.states {
        return Err(exceeded(
            ProbabilityDocumentLimit::States,
            u64::try_from(limits.states).unwrap_or(u64::MAX),
            u64::try_from(rows).unwrap_or(u64::MAX),
        ));
    }
    drop(parsed);
    let wire: WireDocument =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if canonical(&wire, limits.bytes)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    if wire.kind != "exact"
        || !matches!(
            wire.objective.profile.as_str(),
            PROFILE | STATE_PROFILE | REWARD_PROFILE
        )
        || !wire.objective.fairness.is_empty()
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let state_profile = wire.objective.profile == STATE_PROFILE;
    let reward_profile = wire.objective.profile == REWARD_PROFILE;
    if state_profile != wire.objective.event.is_some()
        || state_profile != wire.objective.failures.is_some()
        || reward_profile != wire.objective.reward.is_some()
        || reward_profile != wire.objective.unit.is_some()
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let reward = match (wire.objective.reward, wire.objective.unit) {
        (Some((model, name)), Some(unit)) => Some(ProbabilityRewardObjective {
            model: qsl_foundation::node_wire::node_key_from_text(model.get())
                .map_err(|_| ProbabilityDocumentError::Malformed)?,
            name: quire_exact::Identifier::new(name)
                .map_err(|_| ProbabilityDocumentError::Malformed)?,
            unit: unit::read(unit.get().as_bytes(), limits, cancel)?,
        }),
        (None, None) => None,
        _ => return Err(ProbabilityDocumentError::Malformed),
    };
    let failures = ordered(
        wire.objective
            .failures
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|key| decode_key(key, limits))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
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
            .map(|key| decode_key(key, limits))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
    if failures
        .iter()
        .any(|key| targets.binary_search(key).is_ok())
    {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let mut values = Vec::new();
    for row in wire.values {
        poll(cancel)?;
        values.push(ExactProbabilityValue {
            state: decode_key(&row.state, limits)?,
            value: ratio(row.value, limits.scalar_bits, reward_profile, meter, cancel)?,
        });
    }
    ordered(values.iter().map(|row| row.state.clone()))?;
    let mut ranking = Vec::new();
    for row in wire.ranking {
        poll(cancel)?;
        if row.rank > MAX_COUNT {
            return Err(ProbabilityDocumentError::Malformed);
        }
        ranking.push((decode_key(&row.state, limits)?, row.rank));
    }
    ordered(ranking.iter().map(|(state, _)| state.clone()))?;
    poll(cancel)?;
    Ok(ProbabilityCertificateDocument {
        identity,
        targets,
        event: wire.objective.event,
        failures,
        reward,
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
    writer::write(document, limits, cancel)
}

type SubjectGraph = (
    CheckerGraph,
    ProbabilityObjective,
    Bound,
    Option<BTreeMap<ProbabilityProductKey, u64>>,
    Option<ProbabilityStateEvent>,
    Vec<ProbabilityProductKey>,
    Option<super::reward::RewardGraph>,
);

fn subject_graph(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<SubjectGraph, ProbabilityDocumentError> {
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
    let binding = claim_binding(system, claim)?;
    let mut issue = None;
    let reference = binding.as_ref().map(|bound| &bound.reference);
    let workload = claim_workload(system, claim)?;
    let mut state_event = None;
    let mut failures = Vec::new();
    let mut reward = None;
    let (bound, enumerated) = match &claim.form {
        ProbForm::ExpectedReward {
            bound,
            reward: name,
            until,
        } => {
            if qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                != Some(qsl_semantics::check::TemporalProfile::InfiniteTrace)
            {
                return Err(ProbabilityDocumentError::Unsupported);
            }
            let result = super::reward::enumerate(
                system, claim, name, until, reference, workload, limits, cancel,
            )
            .map(|(graph, ranks, costs)| {
                reward = Some(costs);
                (graph, Some(ranks))
            });
            (bound, result)
        }
        ProbForm::Reach {
            bound,
            target: target @ (ReachTarget::Until(..) | ReachTarget::Always(_)),
        } => {
            if qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                != Some(qsl_semantics::check::TemporalProfile::InfiniteTrace)
            {
                return Err(ProbabilityDocumentError::Unsupported);
            }
            let enumerated = super::state_predicate::enumerate(
                system,
                target,
                reference,
                workload,
                limits,
                cancel,
                &mut |failure| {
                    issue = Some(failure);
                },
            )
            .map(|(event, graph, derived_failures)| {
                state_event = Some(event);
                failures = derived_failures;
                (graph, None)
            });
            (bound, enumerated)
        }
        ProbForm::Reach {
            bound,
            target: ReachTarget::Eventually(predicate),
        } => (
            bound,
            enumerate_model_graph(
                system,
                predicate,
                reference,
                workload,
                limits.states,
                limits.edges,
                cancel,
                true,
                &mut |_, _| {},
                &mut |failure| {
                    issue = Some(failure);
                },
            )
            .map(|graph| (graph, None))
            .map_err(ProbabilityDocumentError::from),
        ),
        ProbForm::Probability {
            bound,
            event:
                qsl_semantics::check::Event::Formula {
                    formula,
                    timed_horizon: None,
                    ..
                },
        } => {
            let context = probabilistic_formula_monitor_context(claim, reference)?;
            (
                bound,
                super::bounded::enumerate(
                    system,
                    formula,
                    &context,
                    reference,
                    workload,
                    limits,
                    cancel,
                    &mut |failure| {
                        issue = Some(failure);
                    },
                )
                .map(|(graph, ranks)| (graph, Some(ranks))),
            )
        }
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    let (graph, bounded_ranks) = enumerated.map_err(|error| match issue {
        Some(issue) => ProbabilityDocumentError::Build(Box::new(issue)),
        None => match error {
            ProbabilityDocumentError::Graph(ExactProbabilityGraphError::LimitReached) => {
                let bound = u64::try_from(limits.states).unwrap_or(u64::MAX);
                exceeded(
                    ProbabilityDocumentLimit::States,
                    bound,
                    bound.saturating_add(1),
                )
            }
            ProbabilityDocumentError::Graph(ExactProbabilityGraphError::EdgeLimitReached) => {
                let bound = u64::try_from(limits.edges).unwrap_or(u64::MAX);
                exceeded(
                    ProbabilityDocumentLimit::Edges,
                    bound,
                    bound.saturating_add(1),
                )
            }
            other => other,
        },
    })?;
    if reward.is_none() {
        topological_order(&graph, cancel)?;
    }
    Ok((
        graph,
        claim_objective(&claim.scheduler, bound),
        bound.clone(),
        bounded_ranks,
        state_event,
        failures,
        reward,
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

/// Admit an expected-reward subject before an engine invokes its general
/// solver. This independently checks the finite acyclic profile, actual
/// costs, units and terminal targets under caller controls. It grants no
/// certificate acceptance and computes no expected value.
pub fn admit_expected_reward_certificate_subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    if !matches!(
        &bind_claim(claims, item, cancel)?.form,
        ProbForm::ExpectedReward { .. }
    ) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    subject_graph(system, claims, item, limits, cancel)?;
    Ok(())
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
    let (graph, optimization, _, bounded_ranks, event, failures, reward) =
        subject_graph(system, claims, item, limits, cancel)?;
    let stopped = event.is_some() || reward.is_some();
    let bounded = bounded_ranks.is_some();
    let mut ranks = bounded_ranks.unwrap_or_default();
    if bounded {
        for rank in ranks.values() {
            poll(cancel)?;
            if *rank > MAX_COUNT {
                return Err(exceeded(ProbabilityDocumentLimit::Rank, MAX_COUNT, *rank));
            }
        }
    }
    if !bounded {
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
                            .ok_or(exceeded(
                                ProbabilityDocumentLimit::Rank,
                                MAX_COUNT,
                                MAX_COUNT + 1,
                            ))?,
                    );
                }
            }
            ranks.insert(key, rank);
        }
    }
    Ok(ProbabilityCertificateDocument {
        identity: item.clone(),
        targets: target_keys(&graph),
        event,
        failures,
        reward: reward.map(|reward| reward.objective),
        optimization,
        values: certificate
            .values
            .iter()
            .filter(|row| !stopped || graph.states.contains_key(&row.state))
            .cloned()
            .collect(),
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
    if long_run::has_profile(bytes, limits, cancel)? {
        return long_run::check(system, claims, item, bytes, limits, cancel);
    }
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = match read_document(bytes, limits, cancel, &mut meter) {
        Ok(document) => document,
        Err(error) => return document_admission_error(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let (graph, objective, bound, bounded_ranks, event, failures, reward) =
        subject_graph(system, claims, item, limits, cancel)?;
    if objective != document.optimization
        || target_keys(&graph) != document.targets
        || event != document.event
        || failures != document.failures
        || reward.as_ref().map(|reward| &reward.objective) != document.reward.as_ref()
    {
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
            return Ok(
                if state.terminal.is_none()
                    && values.get(key).is_some_and(|value| *value > rational(0))
                {
                    rejected(ProbabilityCertificateRule::RankMissing, key.clone())
                } else {
                    malformed_ranking()
                },
            );
        }
    }
    if ranks.len() != graph.states.len() {
        return Ok(malformed_ranking());
    }
    if let Some(expected) = bounded_ranks {
        for (key, actual) in &expected {
            poll(cancel)?;
            if ranks.get(key) != Some(actual) {
                return Ok(rejected(
                    ProbabilityCertificateRule::RankBroken,
                    key.clone(),
                ));
            }
        }
    }
    for (key, state) in &graph.states {
        poll(cancel)?;
        if reward.is_some()
            || state.terminal.is_some()
            || values.get(key).is_some_and(|value| *value == rational(0))
        {
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
            return Ok(rejected(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            ));
        }
    }
    let mut check = check_document_equalities(
        &graph,
        &values,
        objective,
        &bound,
        reward.as_ref(),
        limits,
        cancel,
        &mut meter,
    )?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut check {
        accepted.document = Some(bytes.into());
    }
    poll(cancel)?;
    Ok(check)
}

fn malformed_ranking() -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::Malformed,
        at: ProbabilityCertificateLocus::Member("ranking"),
    })
}

pub(super) fn document_admission_error(
    error: ProbabilityDocumentError,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    match error {
        ProbabilityDocumentError::MissingMember("ranking") => Ok(malformed_ranking()),
        ProbabilityDocumentError::MalformedMember(member) => Ok(
            ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member(member),
            }),
        ),
        ProbabilityDocumentError::MissingMember("components") => Ok(
            ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member("components"),
            }),
        ),
        other => Err(other),
    }
}

pub(super) fn check_scalar(
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
                .magnitude_bits()
                .max(value.denominator().magnitude_bits()),
        ))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn check_document_equalities(
    graph: &CheckerGraph,
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    objective: ProbabilityObjective,
    bound: &Bound,
    reward: Option<&super::reward::RewardGraph>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    for (key, state) in &graph.states {
        poll(cancel)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let expected = match state.terminal {
            Some(true) => rational(i64::from(reward.is_none())),
            Some(false) => rational(0),
            None => {
                if state.choices.is_empty()
                    || objective == ProbabilityObjective::Workload && state.choices.len() != 1
                {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let mut expected: Option<Rational> = None;
                for (choice_index, choice) in state.choices.iter().enumerate() {
                    let mut sum = rational(0);
                    let mut mass = rational(0);
                    for (edge_index, edge) in choice.iter().enumerate() {
                        poll(cancel)?;
                        if edge.probability < rational(0) {
                            return Err(ProbabilityDocumentError::Malformed);
                        }
                        check_scalar(&edge.probability, limits)?;
                        let next = values
                            .get(&edge.target)
                            .ok_or(ProbabilityDocumentError::Malformed)?;
                        let next = if let Some(reward) = reward {
                            let cost = reward
                                .costs
                                .get(key)
                                .and_then(|choices| choices.get(choice_index))
                                .and_then(|costs| costs.get(edge_index))
                                .ok_or(ProbabilityDocumentError::Malformed)?;
                            check_scalar(cost, limits)?;
                            arithmetic(
                                quire_exact::RationalArithmetic::Add(cost, next),
                                meter,
                                cancel,
                            )?
                        } else {
                            next.clone()
                        };
                        check_scalar(&next, limits)?;
                        let product = arithmetic(
                            quire_exact::RationalArithmetic::Multiply(&edge.probability, &next),
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
            certificate: ExactProbabilityCertificate {
                values: values
                    .iter()
                    .map(|(state, value)| ExactProbabilityValue {
                        state: state.clone(),
                        value: value.clone(),
                    })
                    .collect(),
            },
            document: None,
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

// Admission is shared by the source facade, before compilation or enumeration.
pub(super) fn certificate_identity(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ObligationIdentity, ProbabilityDocumentError> {
    if long_run::has_profile(bytes, limits, cancel)? {
        return Ok(read_long_run_probability_certificate(bytes, limits, cancel)?.identity);
    }
    Ok(read_probability_certificate(bytes, limits, cancel)?.identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("TC-636", "FR-201-AC-21", "FR-201-AC-22")]
    #[test]
    fn reward_codec_preserves_large_values_and_closed_objective_without_changing_probability_codec()
    {
        let cancel = Cancel::new();
        let original = document();
        let probability = write_probability_certificate(&original, limits(), &cancel).unwrap();
        let mut reward = original.clone();
        reward.reward = Some(ProbabilityRewardObjective {
            model: reward.identity.node().clone(),
            name: quire_exact::Identifier::new("cost").unwrap(),
            unit: None,
        });
        reward.values[0].value = rational(4);
        let bytes = write_probability_certificate(&reward, limits(), &cancel).unwrap();
        let read = read_probability_certificate(&bytes, limits(), &cancel).unwrap();
        assert_eq!(read, reward);
        assert_eq!(
            write_probability_certificate(&read, limits(), &cancel).unwrap(),
            bytes
        );
        let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(wire["objective"]["profile"], REWARD_PROFILE);
        assert_eq!(wire["objective"].as_object().unwrap().len(), 6);
        wire["objective"]["event"] = json!("until");
        let mixed = serde_json::to_vec(&wire).unwrap();
        assert_eq!(
            read_probability_certificate(&mixed, limits(), &cancel),
            Err(ProbabilityDocumentError::Malformed)
        );
        wire["objective"].as_object_mut().unwrap().remove("event");
        wire["objective"].as_object_mut().unwrap().remove("unit");
        let missing = serde_json::to_vec(&wire).unwrap();
        assert_eq!(
            read_probability_certificate(&missing, limits(), &cancel),
            Err(ProbabilityDocumentError::Malformed)
        );
        assert_eq!(
            write_probability_certificate(&original, limits(), &cancel).unwrap(),
            probability
        );
    }

    pub(super) fn limits() -> ProbabilityDocumentLimits {
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 100,
            edges: 100,
            scalar_bits: 128,
            accounting: crate::AccountingLimits::default().0,
        }
    }
    #[trace("TC-636", "FR-201-AC-21", "FR-201-AC-22")]
    #[test]
    fn reward_unit_descriptors_use_bounded_input_depth_parsing() {
        let cancel = Cancel::new();
        let mut reward = document();
        let mut node = reward.identity.node().clone();
        for _ in 0..256 {
            node = quire_exact::NodeKey::try_new(
                "unit",
                vec![quire_exact::PathSegment::Node(node)],
                0,
            )
            .unwrap();
        }
        for unit in [
            quire_exact::UnitId::declared(node.clone()),
            quire_exact::UnitId::compound(vec![(node, Integer::from(-1_i64))]),
            quire_exact::UnitId::compound(Vec::new()),
        ] {
            reward.reward = Some(ProbabilityRewardObjective {
                model: reward.identity.node().clone(),
                name: quire_exact::Identifier::new("cost").unwrap(),
                unit: Some(unit),
            });
            let bytes = write_probability_certificate(&reward, limits(), &cancel).unwrap();
            assert_eq!(
                read_probability_certificate(&bytes, limits(), &cancel).unwrap(),
                reward
            );
        }
    }

    pub(super) fn document() -> ProbabilityCertificateDocument {
        let key = fixture_key(0);
        ProbabilityCertificateDocument {
            reward: None,
            identity: ObligationIdentity::new(crate::fixture_node(1), None, 0),
            targets: vec![key.clone()],
            event: None,
            failures: Vec::new(),
            optimization: ProbabilityObjective::Minimum,
            values: vec![ExactProbabilityValue {
                state: key.clone(),
                value: rational(1),
            }],
            ranking: vec![(key, 0)],
        }
    }
    pub(super) fn model_bytes(index: usize) -> &'static [u8] {
        match index {
            0 => include_bytes!(
                "../../../tools/qsl-probability-check/tests/fixtures/link.model-key-0.json"
            ),
            1 => include_bytes!(
                "../../../tools/qsl-probability-check/tests/fixtures/link.model-key-1.json"
            ),
            _ => panic!("two production model-state vectors"),
        }
    }
    pub(super) fn fixture_key(index: usize) -> ProbabilityProductKey {
        ProbabilityProductKey::for_model_state(None, model_bytes(index)).unwrap()
    }
    pub(super) fn large_key(size: usize) -> ProbabilityProductKey {
        let mut model: Value = serde_json::from_slice(model_bytes(0)).unwrap();
        // Object identities are caller-provided text; vary that identity in
        // the captured owner's complete typed form, preserving every envelope.
        model["semantic"]["entries"][0]["value"]["entries"][0]["key"]["identity"] =
            json!("m".repeat(size));
        let bytes = canonical(&model, limits().bytes).unwrap();
        ProbabilityProductKey::for_model_state(None, &bytes).unwrap()
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
        let error = read_probability_certificate(
            &bytes,
            ProbabilityDocumentLimits {
                storage_bytes: bound,
                ..limits()
            },
            &Cancel::new(),
        )
        .unwrap_err();
        let ProbabilityDocumentError::Limit {
            limit,
            bound: actual_bound,
            actual,
        } = error
        else {
            panic!("the bounded canonical reader must refuse its working allocation");
        };
        assert_eq!(limit, ProbabilityDocumentLimit::StorageBytes);
        assert_eq!(actual_bound, bound);
        assert!(actual > bound);
        let text = format!("1{}", "0".repeat(100_000));
        let offered = mutate(|value| {
            value["values"][0]["value"]["numerator"] = json!(text);
        });
        assert_eq!(
            read_probability_certificate(
                &offered,
                ProbabilityDocumentLimits {
                    scalar_bits: 16,
                    ..limits()
                },
                &Cancel::new()
            ),
            Err(ProbabilityDocumentError::IntegerText {
                bit_bound: 16,
                decimal_digits: 100_001
            })
        );
        let mut admission = ProbabilityDocumentLimits {
            storage_bytes: 0,
            ..limits()
        };
        let large_key = large_key(375_000);
        let valid = mutate(|value| {
            value["values"][0]["state"] = json!(URL_SAFE_NO_PAD.encode(large_key.as_bytes()));
        });
        loop {
            match read_probability_certificate(&valid, admission, &Cancel::new()) {
                Ok(_) => break,
                Err(ProbabilityDocumentError::Limit {
                    limit: ProbabilityDocumentLimit::StorageBytes,
                    bound,
                    actual,
                }) => {
                    assert_eq!(bound, admission.storage_bytes);
                    assert!(
                        actual > bound && actual <= limits().storage_bytes,
                        "actual reservation must advance within this fixture's finite ceiling"
                    );
                    admission.storage_bytes = actual;
                }
                Err(error) => panic!("the valid flat-key control must admit: {error:?}"),
            }
        }
        let inner = serde_json::to_vec(&vec![Value::Null; 100_000]).unwrap();
        let offered = mutate(|value| {
            value["values"][0]["state"] = json!(URL_SAFE_NO_PAD.encode(inner));
        });
        assert!(
            matches!(
                read_probability_certificate(&offered, admission, &Cancel::new()),
                Err(ProbabilityDocumentError::Limit {
                    limit: ProbabilityDocumentLimit::StorageBytes,
                    ..
                })
            ),
            "decoded key JSON must pass the same bounded admission before its tree is retained"
        );
    }

    #[trace("FR-201-AC-9")]
    #[test]
    fn every_row_collection_requires_strict_complete_key_order() {
        let mut document = document();
        let second = fixture_key(1);
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
        let root = fixture_key(0);
        let target = fixture_key(1);
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
        let check = |limits: ProbabilityDocumentLimits| {
            let cancel = Cancel::new();
            let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
            check_document_equalities(
                &graph,
                &values,
                ProbabilityObjective::Minimum,
                &Bound::AtLeast(rational(1)),
                None,
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
