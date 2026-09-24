// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::{self, Write};

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};

use crate::{protocol_artifact::content_identity, temporal};
use qsl_foundation::ByteDigest;

/// Contract identifier for the native-temporal request document.
pub const REQUEST_CONTRACT: &str = "quire.native-temporal-request/v1";
/// Contract identifier for the native-temporal result document.
pub const RESULT_CONTRACT: &str = "quire.native-temporal-result/v1";

const MAX_BYTES: usize = 8 * 1_048_576;
const MAX_DEPTH: usize = 64;
const MAX_STRING_BYTES: usize = 1_048_576;
const MAX_FORMULA_NODES: usize = 100_000;
const MAX_FORMULA_DEPTH: usize = 64;
const MAX_POSITIONS: usize = 1_000_000;
const MAX_VALUATIONS: usize = 1_000_000;
const MAX_CAPTURES: usize = 100_000;
const MAX_SUPPORT: usize = 1_000_000;
/// The largest integer an I-JSON (RFC 7493) reader holds exactly as a JSON
/// number: 2^53 - 1, or `usize::MAX` where that is smaller. A document
/// carrying a larger `limits.history_span` refuses as `invalid("limits")`.
const MAX_HISTORY_SPAN: usize = usize::MAX >> usize::BITS.saturating_sub(53);
const MAX_EVALUATION_STEPS: usize = 1_000_000;
const MAX_LINEAGE: usize = 1_024;
const MAX_VISITED: usize = 2_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Caller-selected ceilings, independently clamped to QSL owner maxima.
pub struct Limits {
    /// Maximum size in bytes of the encoded input document.
    pub input_bytes: usize,
    /// Maximum size in bytes of the encoded output document.
    pub output_bytes: usize,
    /// Maximum nesting depth allowed while parsing JSON.
    pub json_depth: usize,
    /// Maximum size in bytes of any single string value.
    pub string_bytes: usize,
    /// Maximum number of nodes in the temporal formula.
    pub formula_nodes: usize,
    /// Maximum depth of the temporal formula tree.
    pub formula_depth: usize,
    /// Maximum number of observed trace positions.
    pub positions: usize,
    /// Maximum number of formula-leaf valuations across all positions.
    pub valuations: usize,
    /// Maximum number of trigger captures.
    pub captures: usize,
    /// Maximum retained evidence support set size.
    pub support: usize,
    /// Maximum required history span, in clock units.
    pub history_span: usize,
    /// Maximum number of evaluation steps performed.
    pub evaluation_steps: usize,
    /// Maximum lineage chain length retained.
    pub lineage: usize,
    /// Maximum total number of nodes visited across validation and evaluation.
    pub visited: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: MAX_BYTES,
            output_bytes: MAX_BYTES,
            json_depth: MAX_DEPTH,
            string_bytes: MAX_STRING_BYTES,
            formula_nodes: MAX_FORMULA_NODES,
            formula_depth: MAX_FORMULA_DEPTH,
            positions: MAX_POSITIONS,
            valuations: MAX_VALUATIONS,
            captures: MAX_CAPTURES,
            support: MAX_SUPPORT,
            history_span: MAX_HISTORY_SPAN,
            evaluation_steps: MAX_EVALUATION_STEPS,
            lineage: MAX_LINEAGE,
            visited: MAX_VISITED,
        }
    }
}

impl Limits {
    pub(crate) fn bounded(mut self) -> Self {
        let hard = Self::default();
        macro_rules! clamp {
            ($($field:ident),* $(,)?) => {
                $(self.$field = self.$field.min(hard.$field);)*
            };
        }
        clamp!(
            input_bytes,
            output_bytes,
            json_depth,
            string_bytes,
            formula_nodes,
            formula_depth,
            positions,
            valuations,
            captures,
            support,
            history_span,
            evaluation_steps,
            lineage,
            visited,
        );
        self
    }

    pub(crate) fn evaluation(self) -> temporal::Limits {
        temporal::Limits {
            positions: self.positions,
            valuations: self.valuations,
            instances: 1,
            captures: self.captures,
            retention: self.support,
            visits: self.evaluation_steps,
            depth: self.formula_depth,
            horizon: self.history_span,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WireLimits {
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub json_depth: u64,
    pub string_bytes: u64,
    pub formula_nodes: u64,
    pub formula_depth: u64,
    pub positions: u64,
    pub valuations: u64,
    pub captures: u64,
    pub support: u64,
    pub history_span: u64,
    pub evaluation_steps: u64,
    pub lineage: u64,
    pub visited: u64,
}

impl TryFrom<Limits> for WireLimits {
    type Error = Error;

    fn try_from(value: Limits) -> Result<Self, Self::Error> {
        let value = value.bounded();
        macro_rules! fixed {
            ($field:ident) => {
                u64::try_from(value.$field)
                    .map_err(|_| invalid(concat!("limits.", stringify!($field))))?
            };
        }
        Ok(Self {
            input_bytes: fixed!(input_bytes),
            output_bytes: fixed!(output_bytes),
            json_depth: fixed!(json_depth),
            string_bytes: fixed!(string_bytes),
            formula_nodes: fixed!(formula_nodes),
            formula_depth: fixed!(formula_depth),
            positions: fixed!(positions),
            valuations: fixed!(valuations),
            captures: fixed!(captures),
            support: fixed!(support),
            history_span: fixed!(history_span),
            evaluation_steps: fixed!(evaluation_steps),
            lineage: fixed!(lineage),
            visited: fixed!(visited),
        })
    }
}

impl TryFrom<WireLimits> for Limits {
    type Error = Error;

    fn try_from(value: WireLimits) -> Result<Self, Self::Error> {
        macro_rules! width {
            ($field:ident) => {
                usize::try_from(value.$field)
                    .map_err(|_| invalid(concat!("limits.", stringify!($field))))?
            };
        }
        let limits = Self {
            input_bytes: width!(input_bytes),
            output_bytes: width!(output_bytes),
            json_depth: width!(json_depth),
            string_bytes: width!(string_bytes),
            formula_nodes: width!(formula_nodes),
            formula_depth: width!(formula_depth),
            positions: width!(positions),
            valuations: width!(valuations),
            captures: width!(captures),
            support: width!(support),
            history_span: width!(history_span),
            evaluation_steps: width!(evaluation_steps),
            lineage: width!(lineage),
            visited: width!(visited),
        };
        if limits != limits.bounded() {
            return Err(invalid("limits"));
        }
        Ok(limits)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// Measured work retained for either success or failure.
pub struct Usage {
    /// Size in bytes of the encoded input document actually consumed.
    pub input_bytes: usize,
    /// Size in bytes of the encoded output document actually produced.
    pub output_bytes: usize,
    /// Maximum JSON nesting depth actually observed.
    pub json_depth: usize,
    /// Length in bytes of the longest string value actually observed.
    pub string_bytes: usize,
    /// Number of formula nodes actually visited.
    pub formula_nodes: usize,
    /// Number of observed trace positions actually processed.
    pub positions: usize,
    /// Number of formula-leaf valuations actually processed.
    pub valuations: usize,
    /// Number of trigger captures actually processed.
    pub captures: usize,
    /// Size of the evidence support set actually retained.
    pub support: usize,
    /// Length of the lineage chain actually retained.
    pub lineage: usize,
    /// Total number of nodes actually visited across validation and evaluation.
    pub visited: usize,
    /// Resource usage recorded by the underlying temporal evaluation.
    pub evaluation: temporal::Usage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
/// Stable refusal categories for the FR-052 owner contracts.
pub enum ErrorCode {
    /// The caller-supplied input failed validation.
    InvalidInput,
    /// The document bytes were malformed or failed to decode.
    InvalidDocument,
    /// The document decoded successfully but was not in canonical form.
    NonCanonical,
    /// The document requested behavior this owner contract does not support.
    Unsupported,
    /// The document's cross-references did not hold the required relationship.
    InvalidRelation,
    /// A resource limit was exceeded before the operation could complete.
    ResourceIncomplete,
    /// Memory allocation failed while producing or reading the document.
    Allocation,
}

impl ErrorCode {
    /// Returns the stable string form of this error code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::InvalidDocument => "invalid_document",
            Self::NonCanonical => "noncanonical_document",
            Self::Unsupported => "unsupported",
            Self::InvalidRelation => "invalid_relation",
            Self::ResourceIncomplete => "resource_incomplete",
            Self::Allocation => "allocation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("native temporal owner {code:?} at {path}")]
/// Located contract refusal. No error contains a partial Boolean result.
pub struct Error {
    code: ErrorCode,
    path: &'static str,
}

impl Error {
    /// Returns the error's stable category code.
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Returns the field path the error refers to.
    pub const fn path(&self) -> &'static str {
        self.path
    }
}

pub(crate) fn error(code: ErrorCode, path: &'static str) -> Error {
    Error { code, path }
}

pub(crate) fn invalid(path: &'static str) -> Error {
    error(ErrorCode::InvalidInput, path)
}

pub(crate) fn exhausted(path: &'static str) -> Error {
    error(ErrorCode::ResourceIncomplete, path)
}

#[derive(Debug)]
/// One bounded owner-contract operation.
pub struct Report<T> {
    result: Result<T, Error>,
    limits: Limits,
    usage: Usage,
}

impl<T> Report<T> {
    /// Returns the operation's result by reference.
    pub fn result(&self) -> Result<&T, &Error> {
        self.result.as_ref()
    }

    /// Consumes the report and returns its result.
    pub fn into_result(self) -> Result<T, Error> {
        self.result
    }

    /// Returns the resource limits the operation ran under.
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// Returns the resource usage measured during the operation.
    pub const fn usage(&self) -> Usage {
        self.usage
    }
}

pub(crate) fn report<T>(limits: Limits, usage: Usage, result: Result<T, Error>) -> Report<T> {
    Report {
        result,
        limits,
        usage,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Opaque external owner evidence retained without authenticating its bytes.
pub struct EvidenceRef {
    contract: String,
    schema_digest: String,
    identity: String,
    digest: String,
    authority_identity: String,
    authority_revision: String,
    authority_digest: String,
    scope: String,
    population: u64,
}

impl EvidenceRef {
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner evidence tuple is intentionally explicit"
    )]
    /// Constructs a new evidence reference from its owner-supplied fields.
    pub fn new(
        contract: impl Into<String>,
        schema_digest: impl Into<String>,
        identity: impl Into<String>,
        digest: impl Into<String>,
        authority_identity: impl Into<String>,
        authority_revision: impl Into<String>,
        authority_digest: impl Into<String>,
        scope: impl Into<String>,
        population: u64,
    ) -> Self {
        Self {
            contract: contract.into(),
            schema_digest: schema_digest.into(),
            identity: identity.into(),
            digest: digest.into(),
            authority_identity: authority_identity.into(),
            authority_revision: authority_revision.into(),
            authority_digest: authority_digest.into(),
            scope: scope.into(),
            population,
        }
    }

    /// Returns the contract identifier of the referenced evidence.
    pub fn contract(&self) -> &str {
        &self.contract
    }
    /// Returns the digest of the schema the referenced evidence conforms to.
    pub fn schema_digest(&self) -> &str {
        &self.schema_digest
    }
    /// Returns the identity of the referenced evidence.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// Returns the digest of the referenced evidence's bytes.
    pub fn digest(&self) -> &str {
        &self.digest
    }
    /// Returns the identity of the authority that produced the referenced evidence.
    pub fn authority_identity(&self) -> &str {
        &self.authority_identity
    }
    /// Returns the revision of the authority that produced the referenced evidence.
    pub fn authority_revision(&self) -> &str {
        &self.authority_revision
    }
    /// Returns the digest of the authority that produced the referenced evidence.
    pub fn authority_digest(&self) -> &str {
        &self.authority_digest
    }
    /// Returns the scope this evidence reference is bound to.
    pub fn scope(&self) -> &str {
        &self.scope
    }
    /// Returns the exact population of facts the referenced evidence covers.
    pub const fn population(&self) -> u64 {
        self.population
    }

    pub(crate) fn validate(
        &self,
        scope: &'static str,
        limits: Limits,
        usage: &mut Usage,
    ) -> Result<(), Error> {
        for value in [
            self.contract.as_str(),
            self.identity.as_str(),
            self.authority_identity.as_str(),
            self.authority_revision.as_str(),
        ] {
            if value.is_empty() {
                return Err(invalid("evidence.string"));
            }
            if value.len() > limits.string_bytes {
                return Err(exhausted("limits.string_bytes"));
            }
            usage.string_bytes = usage.string_bytes.max(value.len());
        }
        for value in [
            self.schema_digest.as_str(),
            self.digest.as_str(),
            self.authority_digest.as_str(),
        ] {
            validate_digest(value, "evidence.digest")?;
        }
        if self.scope != scope {
            return Err(invalid("evidence.scope"));
        }
        usage.visited = usage
            .visited
            .checked_add(9)
            .ok_or_else(|| exhausted("limits.visited"))?;
        if usage.visited > limits.visited {
            return Err(exhausted("limits.visited"));
        }
        Ok(())
    }
}

pub(crate) fn validate_digest(value: &str, path: &'static str) -> Result<(), Error> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(path));
    }
    Ok(())
}

pub(crate) fn validate_string(
    value: &str,
    allow_empty: bool,
    limits: Limits,
    usage: &mut Usage,
    path: &'static str,
) -> Result<(), Error> {
    if !allow_empty && value.is_empty() {
        return Err(invalid(path));
    }
    if value.len() > limits.string_bytes {
        return Err(exhausted("limits.string_bytes"));
    }
    usage.string_bytes = usage.string_bytes.max(value.len());
    Ok(())
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    failure: Option<ErrorCode>,
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self.bytes.len().checked_add(bytes.len());
        if next.is_none_or(|next| next > self.limit) {
            self.failure = Some(ErrorCode::ResourceIncomplete);
            return Err(io::Error::other("native temporal output limit"));
        }
        self.bytes.try_reserve(bytes.len()).map_err(|_| {
            self.failure = Some(ErrorCode::Allocation);
            io::Error::other("native temporal output allocation")
        })?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn encode(value: &impl Serialize, limits: Limits) -> Result<Vec<u8>, Error> {
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        limit: limits.output_bytes,
        failure: None,
    };
    if serde_json::to_writer(&mut writer, value).is_err() {
        return Err(error(
            writer.failure.unwrap_or(ErrorCode::InvalidDocument),
            "document",
        ));
    }
    Ok(writer.bytes)
}

/// The FR-052 content identity: [`content_identity::of`] over a document's
/// identity preimage under its contract label.
pub(crate) fn identity(
    contract: &str,
    preimage: &impl Serialize,
    limits: Limits,
) -> Result<String, Error> {
    content_identity::of(contract, preimage, limits.output_bytes, limits.json_depth).map_err(
        |refusal| match refusal {
            content_identity::Refusal::OutputBytes => exhausted("limits.output_bytes"),
            content_identity::Refusal::JsonDepth => exhausted("limits.json_depth"),
            content_identity::Refusal::NotEncodable => {
                error(ErrorCode::InvalidDocument, "identity")
            }
        },
    )
}

pub(crate) fn raw_digest(bytes: &[u8]) -> ByteDigest {
    ByteDigest::of(bytes)
}

pub(crate) fn preflight(bytes: &[u8], limits: Limits, usage: &mut Usage) -> Result<(), Error> {
    if bytes.len() > limits.input_bytes {
        return Err(exhausted("limits.input_bytes"));
    }
    usage.input_bytes = bytes.len();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut string = 0usize;
    for byte in bytes {
        if quoted {
            if !escaped && *byte == b'"' {
                quoted = false;
                usage.string_bytes = usage.string_bytes.max(string);
                continue;
            }
            string = string
                .checked_add(1)
                .ok_or_else(|| exhausted("limits.string_bytes"))?;
            if string > limits.string_bytes {
                return Err(exhausted("limits.string_bytes"));
            }
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            }
            continue;
        }
        match *byte {
            b'"' => {
                quoted = true;
                string = 0;
            }
            b'{' | b'[' => {
                depth = depth
                    .checked_add(1)
                    .ok_or_else(|| exhausted("limits.json_depth"))?;
                if depth > limits.json_depth {
                    return Err(exhausted("limits.json_depth"));
                }
                usage.json_depth = usage.json_depth.max(depth);
            }
            b'}' | b']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| error(ErrorCode::InvalidDocument, "document"))?
            }
            _ => {}
        }
    }
    if quoted || depth != 0 {
        return Err(error(ErrorCode::InvalidDocument, "document"));
    }
    Ok(())
}

struct JsonMeter {
    limits: Limits,
    usage: Usage,
    failure: Option<Error>,
}

impl JsonMeter {
    fn enter<E: de::Error>(&mut self) -> Result<(), E> {
        self.usage.visited = self
            .usage
            .visited
            .checked_add(1)
            .filter(|visited| *visited <= self.limits.visited)
            .ok_or_else(|| {
                self.failure = Some(exhausted("limits.visited"));
                E::custom("native temporal visited limit")
            })?;
        Ok(())
    }

    fn string<E: de::Error>(&mut self, value: &str) -> Result<(), E> {
        self.usage.string_bytes = self.usage.string_bytes.max(value.len());
        if value.len() > self.limits.string_bytes {
            self.failure = Some(exhausted("limits.string_bytes"));
            return Err(E::custom("native temporal string limit"));
        }
        Ok(())
    }

    fn population<E: de::Error>(&mut self, value: usize) -> Result<(), E> {
        let maximum = self
            .limits
            .positions
            .max(self.limits.valuations)
            .max(self.limits.captures)
            .max(self.limits.support)
            .max(self.limits.lineage);
        if value > maximum {
            self.failure = Some(exhausted("limits.population"));
            return Err(E::custom("native temporal population limit"));
        }
        Ok(())
    }
}

struct JsonSeed<'a>(&'a mut JsonMeter);

impl<'de> DeserializeSeed<'de> for JsonSeed<'_> {
    type Value = ();

    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        decoder.deserialize_any(JsonVisitor(self.0))
    }
}

struct JsonVisitor<'a>(&'a mut JsonMeter);

impl<'de> Visitor<'de> for JsonVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one bounded JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        self.0.enter()
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        self.0.enter()
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        self.0.enter()
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<(), E> {
        self.0.enter()
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        self.0.enter()
    }
    fn visit_none<E: de::Error>(self) -> Result<(), E> {
        self.0.enter()
    }
    fn visit_some<D: de::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        JsonSeed(self.0).deserialize(decoder)
    }
    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<(), E> {
        self.0.enter()?;
        self.0.string(value)
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<(), E> {
        self.0.enter()?;
        self.0.string(value)
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<(), E> {
        self.visit_str(&value)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        self.0.enter()?;
        let mut population = 0usize;
        while sequence
            .next_element_seed(JsonSeed(&mut *self.0))?
            .is_some()
        {
            population = population
                .checked_add(1)
                .ok_or_else(|| de::Error::custom("native temporal population overflow"))?;
            self.0.population(population)?;
        }
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<(), A::Error> {
        self.0.enter()?;
        let mut population = 0usize;
        while object.next_key_seed(JsonSeed(&mut *self.0))?.is_some() {
            population = population
                .checked_add(1)
                .ok_or_else(|| de::Error::custom("native temporal population overflow"))?;
            self.0.population(population)?;
            object.next_value_seed(JsonSeed(&mut *self.0))?;
        }
        Ok(())
    }
}

fn inspect_json(bytes: &[u8], limits: Limits, usage: &mut Usage) -> Result<(), Error> {
    let mut meter = JsonMeter {
        limits,
        usage: *usage,
        failure: None,
    };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    if JsonSeed(&mut meter).deserialize(&mut decoder).is_err() {
        return Err(meter
            .failure
            .unwrap_or_else(|| error(ErrorCode::InvalidDocument, "document")));
    }
    decoder
        .end()
        .map_err(|_| error(ErrorCode::InvalidDocument, "document.trailing"))?;
    *usage = meter.usage;
    Ok(())
}

pub(crate) fn decode<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    limits: Limits,
    usage: &mut Usage,
) -> Result<T, Error> {
    preflight(bytes, limits, usage)?;
    inspect_json(bytes, limits, usage)?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value =
        T::deserialize(&mut decoder).map_err(|_| error(ErrorCode::InvalidDocument, "document"))?;
    decoder
        .end()
        .map_err(|_| error(ErrorCode::InvalidDocument, "document.trailing"))?;
    Ok(value)
}
