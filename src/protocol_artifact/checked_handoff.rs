// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared FR-051 machinery for constructor-private checked handoffs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Serialize,
};

use super::{content_identity, v2, wire as w};
use qsl_foundation::ByteDigest;

pub const MAX_INPUT_BYTES: usize = 8 * 1_048_576;
pub const MAX_OUTPUT_BYTES: usize = 8 * 1_048_576;
pub const MAX_JSON_DEPTH: usize = 64;
pub const MAX_STRING_BYTES: usize = 1_048_576;
pub const MAX_POPULATION: usize = 10_000;
pub const MAX_EXPRESSION_DEPTH: usize = 256;
pub const MAX_VISITED_FIELDS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Caller-selected resource ceilings, clamped to the owner's hard limits.
pub struct Limits {
    /// Maximum accepted input document bytes.
    pub input_bytes: usize,
    /// Maximum derived canonical document bytes.
    pub output_bytes: usize,
    /// Maximum JSON container nesting depth.
    pub json_depth: usize,
    /// Maximum UTF-8 byte length of one JSON string or member name.
    pub string_bytes: usize,
    /// Maximum population of one collection or object.
    pub population: usize,
    /// Maximum reachable expression-graph depth.
    pub expression_depth: usize,
    /// Maximum total fields and graph nodes visited.
    pub visited_fields: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: MAX_INPUT_BYTES,
            output_bytes: MAX_OUTPUT_BYTES,
            json_depth: MAX_JSON_DEPTH,
            string_bytes: MAX_STRING_BYTES,
            population: MAX_POPULATION,
            expression_depth: MAX_EXPRESSION_DEPTH,
            visited_fields: MAX_VISITED_FIELDS,
        }
    }
}

impl Limits {
    /// Clamps every requested ceiling to the corresponding owner hard limit.
    pub fn bounded(mut self) -> Self {
        let hard = Self::default();
        self.input_bytes = self.input_bytes.min(hard.input_bytes);
        self.output_bytes = self.output_bytes.min(hard.output_bytes);
        self.json_depth = self.json_depth.min(hard.json_depth);
        self.string_bytes = self.string_bytes.min(hard.string_bytes);
        self.population = self.population.min(hard.population);
        self.expression_depth = self.expression_depth.min(hard.expression_depth);
        self.visited_fields = self.visited_fields.min(hard.visited_fields);
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
/// Stable refusal and incomplete-outcome codes for checked handoffs.
pub enum ErrorCode {
    /// The selected declaration or expression is not an admitted owner object.
    InvalidSelection,
    /// The supplied bytes are not a structurally valid document.
    InvalidDocument,
    /// The supplied bytes differ from the uniquely derived canonical document.
    NonCanonical,
    /// A declared resource ceiling was exhausted before admission completed.
    ResourceIncomplete,
    /// A required fallible allocation failed.
    Allocation,
}

impl ErrorCode {
    /// Returns the stable wire spelling of this code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidSelection => "invalid_selection",
            Self::InvalidDocument => "invalid_document",
            Self::NonCanonical => "noncanonical_document",
            Self::ResourceIncomplete => "resource_incomplete",
            Self::Allocation => "allocation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("checked handoff {code:?} at {path}")]
/// Located checked-handoff refusal or incomplete outcome.
pub struct Error {
    code: ErrorCode,
    path: &'static str,
}

impl Error {
    /// Returns the stable error category.
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    /// Returns the stable document or limit path responsible for the outcome.
    pub const fn path(&self) -> &'static str {
        self.path
    }
}

fn invalid_selection(path: &'static str) -> Error {
    Error {
        code: ErrorCode::InvalidSelection,
        path,
    }
}

fn invalid_document(path: &'static str) -> Error {
    Error {
        code: ErrorCode::InvalidDocument,
        path,
    }
}

fn exhausted(path: &'static str) -> Error {
    Error {
        code: ErrorCode::ResourceIncomplete,
        path,
    }
}

fn allocation(path: &'static str) -> Error {
    Error {
        code: ErrorCode::Allocation,
        path,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// Exact resource usage observed before success or refusal.
pub struct Usage {
    /// Input document bytes inspected.
    pub input_bytes: usize,
    /// Canonical output bytes derived.
    pub output_bytes: usize,
    /// Greatest JSON container nesting depth observed.
    pub json_depth: usize,
    /// Greatest UTF-8 byte length of a JSON string or member name observed.
    pub string_bytes: usize,
    /// Greatest collection or object population observed.
    pub population: usize,
    /// Greatest reachable expression-graph depth observed.
    pub expression_depth: usize,
    /// Total document fields and graph nodes visited.
    pub visited_fields: usize,
}

#[derive(Debug)]
/// A checked-handoff outcome with effective limits and measured resource usage.
pub struct Report<T> {
    result: Result<T, Error>,
    limits: Limits,
    usage: Usage,
}

impl<T> Report<T> {
    /// Borrows the successful value or located failure.
    pub fn result(&self) -> Result<&T, &Error> {
        self.result.as_ref()
    }

    /// Consumes the report and returns its value or failure.
    pub fn into_result(self) -> Result<T, Error> {
        self.result
    }

    /// Returns the effective, owner-clamped limits.
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// Returns measured usage, including partial usage on failure.
    pub const fn usage(&self) -> Usage {
        self.usage
    }

    pub(super) fn map<U>(self, map: impl FnOnce(T) -> U) -> Report<U> {
        Report {
            result: self.result.map(map),
            limits: self.limits,
            usage: self.usage,
        }
    }
}

#[derive(Clone)]
pub(super) enum Selection {
    Predicate {
        declaration: u32,
        expression: w::Handle,
    },
    Temporal {
        declaration: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum HistoryBoundary {
    ExecutionOrigin,
    HistoryCutoff,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PackageBinding {
    artifact: w::ArtifactRef,
    digest: String,
    native_identity: String,
    native_revision: String,
    package_definition: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SubjectBinding {
    PredicateLeaf {
        parent_kind: String,
        declaration: u32,
        leaf: w::Handle,
        parameters: Vec<w::Handle>,
    },
    TemporalSubject {
        parent_kind: String,
        declaration: u32,
        activation: w::Activation,
        clock: u32,
        clock_configuration: Box<v2::wire::ClockConfiguration>,
        captures: Vec<w::Handle>,
        root: w::Handle,
        history_boundary: HistoryBoundary,
        operators: Vec<String>,
        required_history: Vec<w::Interval>,
        predicate_leaves: Vec<w::Handle>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct SourceBinding {
    source: w::Source,
    span: w::Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ClauseBinding {
    requirement: w::Requirement,
    clause: String,
    execution: w::Execution,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct IndexedValue {
    index: u32,
    value: w::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct IndexedTemporal {
    index: u32,
    value: w::Temporal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ExpressionBinding {
    root: w::Handle,
    values: Vec<IndexedValue>,
    temporal: Vec<IndexedTemporal>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
struct IdentityBinding {
    kind: String,
    identity: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TypeBinding {
    kind: String,
    indices: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct DefinitionBinding {
    identity: String,
    revision_namespace: String,
    revision: String,
    artifact: w::ArtifactRef,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ProfileBinding {
    evaluation: DefinitionBinding,
    definedness: Vec<DefinitionBinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct LimitBinding {
    input_bytes: usize,
    output_bytes: usize,
    json_depth: usize,
    string_bytes: usize,
    population: usize,
    expression_depth: usize,
    visited_fields: usize,
}

impl From<Limits> for LimitBinding {
    fn from(value: Limits) -> Self {
        Self {
            input_bytes: value.input_bytes,
            output_bytes: value.output_bytes,
            json_depth: value.json_depth,
            string_bytes: value.string_bytes,
            population: value.population,
            expression_depth: value.expression_depth,
            visited_fields: value.visited_fields,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
struct MeasureBinding {
    output_bytes: usize,
    json_depth: usize,
    string_bytes: usize,
    population: usize,
    expression_depth: usize,
    visited_fields: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct LimitsBinding {
    ceilings: LimitBinding,
    measured: MeasureBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct WireDocument {
    contract: String,
    identity: String,
    package: PackageBinding,
    subject: SubjectBinding,
    source: SourceBinding,
    clause: ClauseBinding,
    expression: ExpressionBinding,
    bindings: Vec<IdentityBinding>,
    #[serde(rename = "type")]
    value_type: TypeBinding,
    profiles: ProfileBinding,
    limits: LimitsBinding,
}

#[derive(Serialize)]
struct IdentityPreimage<'a> {
    contract: &'a str,
    package: &'a PackageBinding,
    subject: &'a SubjectBinding,
    source: &'a SourceBinding,
    clause: &'a ClauseBinding,
    expression: &'a ExpressionBinding,
    bindings: &'a [IdentityBinding],
    #[serde(rename = "type")]
    value_type: &'a TypeBinding,
    profiles: &'a ProfileBinding,
    limits: &'a LimitsBinding,
}

#[derive(Debug)]
pub(super) struct Document {
    bytes: Vec<u8>,
    digest: ByteDigest,
    wire: WireDocument,
}

impl Document {
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(super) const fn digest(&self) -> ByteDigest {
        self.digest
    }

    pub(super) fn identity(&self) -> &str {
        &self.wire.identity
    }

    pub(super) fn package_digest(&self) -> ByteDigest {
        self.wire.package.artifact.digest
    }

    pub(super) fn package_artifact(&self) -> &w::ArtifactRef {
        &self.wire.package.artifact
    }

    pub(super) fn declaration(&self) -> u32 {
        match &self.wire.subject {
            SubjectBinding::PredicateLeaf { declaration, .. }
            | SubjectBinding::TemporalSubject { declaration, .. } => *declaration,
        }
    }

    pub(super) fn root(&self) -> &w::Handle {
        &self.wire.expression.root
    }

    pub(super) fn source(&self) -> (&w::Source, &w::Span) {
        (&self.wire.source.source, &self.wire.source.span)
    }

    pub(super) fn clause(&self) -> (&w::Requirement, &str, &w::Execution) {
        (
            &self.wire.clause.requirement,
            &self.wire.clause.clause,
            &self.wire.clause.execution,
        )
    }

    pub(super) fn values(&self) -> impl ExactSizeIterator<Item = (u32, &w::Value)> {
        self.wire
            .expression
            .values
            .iter()
            .map(|entry| (entry.index, &entry.value))
    }

    pub(super) fn temporal(&self) -> impl ExactSizeIterator<Item = (u32, &w::Temporal)> {
        self.wire
            .expression
            .temporal
            .iter()
            .map(|entry| (entry.index, &entry.value))
    }

    pub(super) fn bindings(&self) -> impl ExactSizeIterator<Item = (&str, &str)> {
        self.wire
            .bindings
            .iter()
            .map(|binding| (binding.kind.as_str(), binding.identity.as_str()))
    }

    pub(super) fn type_indices(&self) -> &[u32] {
        &self.wire.value_type.indices
    }

    pub(super) fn profile_identities(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.wire.profiles.evaluation.identity.as_str()).chain(
            self.wire
                .profiles
                .definedness
                .iter()
                .map(|profile| profile.identity.as_str()),
        )
    }

    pub(super) fn predicate_subject(&self) -> Option<(&str, &w::Handle, &[w::Handle])> {
        match &self.wire.subject {
            SubjectBinding::PredicateLeaf {
                parent_kind,
                leaf,
                parameters,
                ..
            } => Some((parent_kind, leaf, parameters)),
            SubjectBinding::TemporalSubject { .. } => None,
        }
    }

    #[allow(
        clippy::type_complexity,
        reason = "private projection keeps the two public owner modules on one canonical record"
    )]
    pub(super) fn temporal_subject(
        &self,
    ) -> Option<(
        &w::Activation,
        u32,
        &v2::wire::ClockConfiguration,
        &[w::Handle],
        &str,
        &[String],
        &[w::Interval],
        &[w::Handle],
    )> {
        match &self.wire.subject {
            SubjectBinding::TemporalSubject {
                activation,
                clock,
                clock_configuration,
                captures,
                history_boundary,
                operators,
                required_history,
                predicate_leaves,
                ..
            } => Some((
                activation,
                *clock,
                clock_configuration.as_ref(),
                captures,
                match history_boundary {
                    HistoryBoundary::ExecutionOrigin => "execution-origin",
                    HistoryBoundary::HistoryCutoff => "history-cutoff",
                },
                operators,
                required_history,
                predicate_leaves,
            )),
            SubjectBinding::PredicateLeaf { .. } => None,
        }
    }
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    failure: Option<Error>,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            failure: None,
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl std::io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|next| *next <= self.limit)
            .is_none()
        {
            self.failure = Some(exhausted("limits.output_bytes"));
            return Err(std::io::Error::other("checked handoff output limit"));
        }
        if self.bytes.try_reserve(bytes.len()).is_err() {
            self.failure = Some(allocation("document"));
            return Err(std::io::Error::other("checked handoff allocation"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encode(value: &impl Serialize, limit: usize) -> Result<Vec<u8>, Error> {
    let mut writer = BoundedWriter::new(limit);
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        return Err(writer.failure.unwrap_or_else(|| {
            if error.is_io() {
                invalid_document("document.io")
            } else {
                invalid_document("document")
            }
        }));
    }
    Ok(writer.finish())
}

/// The FR-051 content identity: [`content_identity::of`] over the
/// document's identity preimage under its contract label.
fn identity(
    contract: &str,
    preimage: &IdentityPreimage<'_>,
    limits: Limits,
) -> Result<String, Error> {
    content_identity::of(contract, preimage, limits.output_bytes, limits.json_depth).map_err(
        |refusal| match refusal {
            content_identity::Refusal::OutputBytes => exhausted("limits.output_bytes"),
            content_identity::Refusal::JsonDepth => exhausted("limits.json_depth"),
            content_identity::Refusal::NotEncodable => invalid_document("identity"),
        },
    )
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct JsonUsage {
    depth: usize,
    string_bytes: usize,
    population: usize,
    visited_fields: usize,
}

struct JsonMeter {
    usage: JsonUsage,
    limits: Limits,
    failure: Option<Error>,
}

impl JsonMeter {
    fn enter<E: de::Error>(&mut self) -> Result<(), E> {
        self.usage.visited_fields = self
            .usage
            .visited_fields
            .checked_add(1)
            .filter(|visited| *visited <= self.limits.visited_fields)
            .ok_or_else(|| {
                self.failure = Some(exhausted("limits.visited_fields"));
                E::custom("checked handoff visited-field limit")
            })?;
        Ok(())
    }

    fn string<E: de::Error>(&mut self, value: &str) -> Result<(), E> {
        self.usage.string_bytes = self.usage.string_bytes.max(value.len());
        if self.usage.string_bytes > self.limits.string_bytes {
            self.failure = Some(exhausted("limits.string_bytes"));
            return Err(E::custom("checked handoff string limit"));
        }
        Ok(())
    }

    fn population<E: de::Error>(&mut self, value: usize) -> Result<(), E> {
        self.usage.population = self.usage.population.max(value);
        if value > self.limits.population {
            self.failure = Some(exhausted("limits.population"));
            return Err(E::custom("checked handoff population limit"));
        }
        Ok(())
    }
}

struct JsonSeed<'a>(&'a mut JsonMeter);

impl<'de> DeserializeSeed<'de> for JsonSeed<'_> {
    type Value = ();

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(JsonVisitor(self.0))
    }
}

struct JsonVisitor<'a>(&'a mut JsonMeter);

impl<'de> Visitor<'de> for JsonVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one bounded JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
        self.0.enter()
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
        self.0.enter()
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
        self.0.enter()
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
        self.0.enter()
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.0.enter()
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.0.enter()
    }

    fn visit_some<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        JsonSeed(self.0).deserialize(deserializer)
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.0.enter()?;
        self.0.string(value)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.0.enter()?;
        self.0.string(value)
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.visit_str(&value)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        self.0.enter()?;
        let mut population = 0usize;
        while sequence
            .next_element_seed(JsonSeed(&mut *self.0))?
            .is_some()
        {
            population = population.checked_add(1).ok_or_else(|| {
                self.0.failure = Some(exhausted("limits.population"));
                de::Error::custom("checked handoff population overflow")
            })?;
            self.0.population(population)?;
        }
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        self.0.enter()?;
        let mut population = 0usize;
        while object.next_key_seed(JsonSeed(&mut *self.0))?.is_some() {
            population = population.checked_add(1).ok_or_else(|| {
                self.0.failure = Some(exhausted("limits.population"));
                de::Error::custom("checked handoff population overflow")
            })?;
            self.0.population(population)?;
            object.next_value_seed(JsonSeed(&mut *self.0))?;
        }
        Ok(())
    }
}

fn inspect_json(bytes: &[u8], limits: Limits) -> Result<JsonUsage, Error> {
    let (depth, raw_string_bytes) = json_preflight(bytes, limits)?;
    let mut meter = JsonMeter {
        usage: JsonUsage {
            depth,
            string_bytes: raw_string_bytes,
            ..JsonUsage::default()
        },
        limits,
        failure: None,
    };
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    if JsonSeed(&mut meter).deserialize(&mut deserializer).is_err() || deserializer.end().is_err() {
        return Err(meter
            .failure
            .unwrap_or_else(|| invalid_document("document")));
    }
    Ok(JsonUsage {
        depth,
        string_bytes: meter.usage.string_bytes,
        population: meter.usage.population,
        visited_fields: meter.usage.visited_fields,
    })
}

fn definition(
    package: &w::Package,
    index: u32,
    path: &'static str,
) -> Result<DefinitionBinding, Error> {
    let selected = package
        .definitions
        .get(usize::try_from(index).map_err(|_| invalid_selection(path))?)
        .ok_or_else(|| invalid_selection(path))?;
    let artifact = package
        .dependencies
        .get(usize::try_from(selected.artifact).map_err(|_| invalid_selection(path))?)
        .ok_or_else(|| invalid_selection(path))?;
    Ok(DefinitionBinding {
        identity: selected.identity.clone(),
        revision_namespace: selected.revision.namespace.clone(),
        revision: selected.revision.value.clone(),
        artifact: artifact.artifact.clone(),
    })
}

fn value_children(value: &w::ValueOperation, limits: Limits) -> Result<Vec<w::Handle>, Error> {
    use w::ValueOperation as V;
    let count = match value {
        V::Boolean { .. } | V::Number { .. } | V::Text { .. } | V::Enum { .. } | V::Read { .. } => {
            0
        }
        V::Group { .. }
        | V::Field { .. }
        | V::Unary { .. }
        | V::Pre { .. }
        | V::Size { .. }
        | V::Parent { .. } => 1,
        V::Binary { .. }
        | V::Let { .. }
        | V::Contains { .. }
        | V::Query { .. }
        | V::Reaches { .. } => 2,
        V::If { .. } => 3,
        V::Call { arguments, .. } => arguments.len(),
    };
    if count > limits.population {
        return Err(exhausted("limits.population"));
    }
    let mut output = Vec::new();
    output
        .try_reserve(count)
        .map_err(|_| allocation("expression.values"))?;
    match value {
        V::Boolean { .. } | V::Number { .. } | V::Text { .. } | V::Enum { .. } | V::Read { .. } => {
        }
        V::Group { value }
        | V::Field { base: value, .. }
        | V::Unary { value, .. }
        | V::Pre { value, .. }
        | V::Size {
            collection: value, ..
        }
        | V::Parent {
            reference: value, ..
        } => output.push(value.clone()),
        V::Binary { left, right, .. }
        | V::Contains {
            collection: left,
            member: right,
        }
        | V::Reaches {
            start: left,
            target: right,
            ..
        } => {
            output.push(left.clone());
            output.push(right.clone());
        }
        V::If {
            condition,
            then_value,
            else_value,
        } => {
            output.push(condition.clone());
            output.push(then_value.clone());
            output.push(else_value.clone());
        }
        V::Let {
            initializer, body, ..
        } => {
            output.push(initializer.clone());
            output.push(body.clone());
        }
        V::Call { arguments, .. } => output.extend(arguments.iter().cloned()),
        V::Query {
            collection, body, ..
        } => {
            output.push(collection.clone());
            output.push(body.clone());
        }
    }
    Ok(output)
}

fn collect_values(
    declaration_index: u32,
    declaration: &w::Declaration,
    roots: &[w::Handle],
    limits: Limits,
    usage: &mut Usage,
) -> Result<Vec<IndexedValue>, Error> {
    let mut stack = Vec::new();
    stack
        .try_reserve(roots.len())
        .map_err(|_| allocation("expression.values"))?;
    stack.extend(roots.iter().cloned().map(|root| (root, 1usize)));
    let mut depths = BTreeMap::<u32, usize>::new();
    while let Some((handle, depth)) = stack.pop() {
        usage.visited_fields = usage
            .visited_fields
            .checked_add(1)
            .filter(|visited| *visited <= limits.visited_fields)
            .ok_or_else(|| exhausted("limits.visited_fields"))?;
        if handle.declaration != declaration_index {
            return Err(invalid_selection("expression.declaration"));
        }
        if depth > limits.expression_depth {
            return Err(exhausted("limits.expression_depth"));
        }
        usage.expression_depth = usage.expression_depth.max(depth);
        let Some(value) = declaration
            .values
            .get(usize::try_from(handle.index).map_err(|_| invalid_selection("expression"))?)
        else {
            return Err(invalid_selection("expression"));
        };
        if depths
            .get(&handle.index)
            .is_some_and(|known| *known >= depth)
        {
            continue;
        }
        if !depths.contains_key(&handle.index) && depths.len() >= limits.population {
            return Err(exhausted("limits.population"));
        }
        depths.insert(handle.index, depth);
        let children = value_children(&value.operation, limits)?;
        stack
            .try_reserve(children.len())
            .map_err(|_| allocation("expression.values"))?;
        stack.extend(
            children
                .into_iter()
                .map(|child| (child, depth.saturating_add(1))),
        );
    }
    if depths.len() > limits.population {
        return Err(exhausted("limits.population"));
    }
    usage.population = usage.population.max(depths.len());
    depths
        .keys()
        .map(|index| {
            let value = declaration
                .values
                .get(usize::try_from(*index).map_err(|_| invalid_selection("expression"))?)
                .ok_or_else(|| invalid_selection("expression"))?;
            Ok(IndexedValue {
                index: *index,
                value: value.clone(),
            })
        })
        .collect()
}

fn temporal_children(
    value: &w::TemporalOperation,
    limits: Limits,
) -> Result<(Vec<w::Handle>, Vec<w::Handle>), Error> {
    use w::TemporalOperation as T;
    let (temporal_count, predicate_count): (usize, usize) = match value {
        T::Constant { .. } => (0, 0),
        T::Holds { .. } => (0, 1),
        T::Group { .. } | T::Unary { .. } => (1, 0),
        T::Binary { .. } => (2, 0),
    };
    if temporal_count.saturating_add(predicate_count) > limits.population {
        return Err(exhausted("limits.population"));
    }
    let mut temporal = Vec::new();
    let mut predicates = Vec::new();
    temporal
        .try_reserve(temporal_count)
        .map_err(|_| allocation("expression.temporal"))?;
    predicates
        .try_reserve(predicate_count)
        .map_err(|_| allocation("expression.values"))?;
    match value {
        T::Constant { .. } => {}
        T::Holds { value } => predicates.push(value.clone()),
        T::Group { value } | T::Unary { value, .. } => temporal.push(value.clone()),
        T::Binary { left, right, .. } => {
            temporal.push(left.clone());
            temporal.push(right.clone());
        }
    }
    Ok((temporal, predicates))
}

type TemporalCollection = (
    Vec<IndexedTemporal>,
    Vec<w::Handle>,
    Vec<String>,
    Vec<w::Interval>,
);

fn collect_temporal(
    declaration_index: u32,
    declaration: &w::Declaration,
    root: &w::Handle,
    limits: Limits,
    usage: &mut Usage,
) -> Result<TemporalCollection, Error> {
    let mut stack = vec![(root.clone(), 1usize)];
    let mut depths = BTreeMap::<u32, usize>::new();
    let mut predicates = BTreeSet::<(u32, u32)>::new();
    let mut operators = BTreeSet::<String>::new();
    let mut intervals = BTreeMap::<String, w::Interval>::new();
    while let Some((handle, depth)) = stack.pop() {
        usage.visited_fields = usage
            .visited_fields
            .checked_add(1)
            .filter(|visited| *visited <= limits.visited_fields)
            .ok_or_else(|| exhausted("limits.visited_fields"))?;
        if handle.declaration != declaration_index {
            return Err(invalid_selection("temporal.declaration"));
        }
        if depth > limits.expression_depth {
            return Err(exhausted("limits.expression_depth"));
        }
        usage.expression_depth = usage.expression_depth.max(depth);
        let value = declaration
            .temporal
            .get(usize::try_from(handle.index).map_err(|_| invalid_selection("temporal"))?)
            .ok_or_else(|| invalid_selection("temporal"))?;
        if depths
            .get(&handle.index)
            .is_some_and(|known| *known >= depth)
        {
            continue;
        }
        if !depths.contains_key(&handle.index)
            && depths.len().saturating_add(predicates.len()) >= limits.population
        {
            return Err(exhausted("limits.population"));
        }
        depths.insert(handle.index, depth);
        match &value.operation {
            w::TemporalOperation::Constant { .. } => {
                operators.insert("constant".into());
            }
            w::TemporalOperation::Holds { .. } => {
                operators.insert("holds".into());
            }
            w::TemporalOperation::Group { .. } => {
                operators.insert("group".into());
            }
            w::TemporalOperation::Unary {
                operator, interval, ..
            } => {
                operators.insert(operator.as_str().into());
                if let Some(interval) = &interval.0 {
                    let key = serde_json::to_string(interval)
                        .map_err(|_| invalid_document("subject.required_history"))?;
                    intervals.entry(key).or_insert_with(|| interval.clone());
                }
            }
            w::TemporalOperation::Binary {
                operator, interval, ..
            } => {
                operators.insert(operator.as_str().into());
                if let Some(interval) = &interval.0 {
                    let key = serde_json::to_string(interval)
                        .map_err(|_| invalid_document("subject.required_history"))?;
                    intervals.entry(key).or_insert_with(|| interval.clone());
                }
            }
        }
        let (children, leaves) = temporal_children(&value.operation, limits)?;
        for leaf in leaves {
            let key = (leaf.declaration, leaf.index);
            if !predicates.contains(&key)
                && depths.len().saturating_add(predicates.len()) >= limits.population
            {
                return Err(exhausted("limits.population"));
            }
            predicates.insert(key);
        }
        stack
            .try_reserve(children.len())
            .map_err(|_| allocation("expression.temporal"))?;
        stack.extend(
            children
                .into_iter()
                .map(|child| (child, depth.saturating_add(1))),
        );
    }
    let population = depths.len().saturating_add(predicates.len());
    if population > limits.population {
        return Err(exhausted("limits.population"));
    }
    usage.population = usage.population.max(population);
    let temporal = depths
        .keys()
        .map(|index| {
            let value = declaration
                .temporal
                .get(usize::try_from(*index).map_err(|_| invalid_selection("temporal"))?)
                .ok_or_else(|| invalid_selection("temporal"))?;
            Ok(IndexedTemporal {
                index: *index,
                value: value.clone(),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let predicates = predicates
        .into_iter()
        .map(|(declaration, index)| w::Handle { declaration, index })
        .collect();
    Ok((
        temporal,
        predicates,
        operators.into_iter().collect(),
        intervals.into_values().collect(),
    ))
}

fn boolean_type_indices(
    package: &w::Package,
    declaration: &w::Declaration,
    values: &[IndexedValue],
    selected: Option<&w::Handle>,
) -> Result<Vec<u32>, Error> {
    if let Some(selected) = selected {
        let selected_value = declaration
            .values
            .get(usize::try_from(selected.index).map_err(|_| invalid_selection("type"))?)
            .ok_or_else(|| invalid_selection("type"))?;
        let selected_type = package
            .types
            .get(
                usize::try_from(selected_value.value_type)
                    .map_err(|_| invalid_selection("type"))?,
            )
            .ok_or_else(|| invalid_selection("type"))?;
        if !matches!(selected_type, w::Type::Boolean {}) {
            return Err(invalid_selection("type.boolean"));
        }
    }
    let mut indices = BTreeSet::new();
    for value in values {
        if matches!(
            package.types.get(
                usize::try_from(value.value.value_type).map_err(|_| invalid_selection("type"))?
            ),
            Some(w::Type::Boolean {})
        ) {
            indices.insert(value.value.value_type);
        }
    }
    if indices.is_empty() {
        let index = package
            .types
            .iter()
            .position(|value| matches!(value, w::Type::Boolean {}))
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| invalid_selection("type.boolean"))?;
        indices.insert(index);
    }
    Ok(indices.into_iter().collect())
}

fn identity_bindings(
    package: &w::Package,
    declaration_index: u32,
    declaration: &w::Declaration,
    expression: &ExpressionBinding,
    captures: &[w::Handle],
    limits: Limits,
    usage: &mut Usage,
) -> Result<Vec<IdentityBinding>, Error> {
    let mut bindings = BTreeSet::new();
    {
        let mut insert = |binding: IdentityBinding| -> Result<(), Error> {
            if !bindings.contains(&binding) && bindings.len() >= limits.population {
                return Err(exhausted("limits.population"));
            }
            bindings.insert(binding);
            Ok(())
        };
        let source = package
            .sources
            .get(
                usize::try_from(declaration.locus.source)
                    .map_err(|_| invalid_selection("source"))?,
            )
            .ok_or_else(|| invalid_selection("source"))?;
        insert(IdentityBinding {
            kind: "declaration".into(),
            identity: format!(
                "{}:{}:{}:{}",
                source.artifact.identity,
                declaration.locus.span.start,
                declaration.locus.span.end,
                declaration.name
            ),
        })?;
        for model in &package.models {
            let artifact = package
                .dependencies
                .get(usize::try_from(model.artifact).map_err(|_| invalid_selection("model"))?)
                .ok_or_else(|| invalid_selection("model"))?;
            insert(IdentityBinding {
                kind: "model".into(),
                identity: format!(
                    "{}@{}:{}#{}",
                    artifact.artifact.identity,
                    artifact.artifact.revision.namespace,
                    artifact.artifact.revision.value,
                    artifact.artifact.digest
                ),
            })?;
        }
        for value in &expression.values {
            insert(IdentityBinding {
                kind: "expression".into(),
                identity: format!("{declaration_index}:value:{}", value.index),
            })?;
        }
        for value in &expression.temporal {
            insert(IdentityBinding {
                kind: "expression".into(),
                identity: format!("{declaration_index}:temporal:{}", value.index),
            })?;
        }
        for (index, binder) in declaration.binders.iter().enumerate() {
            insert(IdentityBinding {
                kind: "binder".into(),
                identity: format!("{declaration_index}:{index}:{}", binder.name),
            })?;
        }
        for (index, requirement) in declaration.bindings.iter().enumerate() {
            insert(IdentityBinding {
                kind: "runtime_requirement".into(),
                identity: format!("{declaration_index}:{index}:{}", requirement.name),
            })?;
        }
        for capture in captures {
            insert(IdentityBinding {
                kind: "capture".into(),
                identity: format!("{}:{}", capture.declaration, capture.index),
            })?;
        }
    }
    usage.population = usage.population.max(bindings.len());
    Ok(bindings.into_iter().collect())
}

fn protocol_control_children(
    operation: &w::ControlOperation,
    limits: Limits,
) -> Result<Vec<w::Handle>, Error> {
    use w::ControlOperation as C;
    let count = match operation {
        C::Sequence { children } => children.len(),
        C::Choice { cases, .. } => cases.len(),
        C::Parallel { branches, .. } => branches.len(),
        C::Repeat { .. } => 2,
        C::Await { .. } => 3,
        C::Event { .. } | C::Check { .. } | C::Commit { .. } => 0,
    };
    if count > limits.population {
        return Err(exhausted("limits.population"));
    }
    let mut output = Vec::new();
    output
        .try_reserve(count)
        .map_err(|_| allocation("subject.controls"))?;
    match operation {
        C::Sequence { children } => output.extend(children.iter().cloned()),
        C::Choice { cases, .. } => {
            output.extend(cases.iter().map(|case| case.body.clone()));
        }
        C::Parallel { branches, .. } => {
            output.extend(branches.iter().map(|branch| branch.body.clone()));
        }
        C::Repeat {
            body, exhausted, ..
        } => {
            output.push(body.clone());
            output.push(exhausted.clone());
        }
        C::Await {
            event,
            then_body,
            timeout,
            ..
        } => {
            output.push(event.clone());
            output.push(then_body.clone());
            output.push(timeout.clone());
        }
        C::Event { .. } | C::Check { .. } | C::Commit { .. } => {}
    }
    Ok(output)
}

fn protocol_predicate_roots(
    declaration_index: u32,
    body: &w::Body,
    limits: Limits,
    usage: &mut Usage,
) -> Result<Vec<w::Handle>, Error> {
    let w::Body::Protocol {
        activation,
        controls,
        compensations,
        run,
        finish,
        ..
    } = body
    else {
        return Err(invalid_selection("subject.kind"));
    };
    let initial_roots = 1usize
        .saturating_add(usize::from(matches!(
            activation,
            w::Activation::Each {
                guard: w::Nullable(Some(_)),
                ..
            }
        )))
        .saturating_add(compensations.len());
    if initial_roots > limits.population {
        return Err(exhausted("limits.population"));
    }
    let mut roots = BTreeSet::<(u32, u32)>::new();
    roots.insert((finish.constraint.declaration, finish.constraint.index));
    if let w::Activation::Each {
        guard: w::Nullable(Some(guard)),
        ..
    } = activation
    {
        roots.insert((guard.declaration, guard.index));
    }
    for compensation in compensations {
        roots.insert((compensation.guard.declaration, compensation.guard.index));
    }

    let mut stack = vec![run.clone()];
    let mut visited = BTreeSet::new();
    while let Some(handle) = stack.pop() {
        usage.visited_fields = usage
            .visited_fields
            .checked_add(1)
            .filter(|visited| *visited <= limits.visited_fields)
            .ok_or_else(|| exhausted("limits.visited_fields"))?;
        if handle.declaration != declaration_index {
            return Err(invalid_selection("subject.control.declaration"));
        }
        if !visited.insert(handle.index) {
            continue;
        }
        if visited.len() > limits.population {
            return Err(exhausted("limits.population"));
        }
        let control = usize::try_from(handle.index)
            .ok()
            .and_then(|index| controls.get(index))
            .ok_or_else(|| invalid_selection("subject.control"))?;
        match &control.operation {
            w::ControlOperation::Choice { cases, .. } => {
                roots.extend(
                    cases
                        .iter()
                        .map(|case| (case.guard.declaration, case.guard.index)),
                );
            }
            w::ControlOperation::Repeat { guard, .. } => {
                roots.insert((guard.declaration, guard.index));
            }
            w::ControlOperation::Event { constraint, .. }
            | w::ControlOperation::Commit { constraint, .. } => {
                roots.insert((constraint.declaration, constraint.index));
            }
            w::ControlOperation::Check { value, .. } => {
                roots.insert((value.declaration, value.index));
            }
            w::ControlOperation::Sequence { .. }
            | w::ControlOperation::Parallel { .. }
            | w::ControlOperation::Await { .. } => {}
        }
        let children = protocol_control_children(&control.operation, limits)?;
        stack
            .try_reserve(children.len())
            .map_err(|_| allocation("subject.controls"))?;
        stack.extend(children);
    }
    let total = roots.len().saturating_add(visited.len());
    if total > limits.population {
        return Err(exhausted("limits.population"));
    }
    usage.population = usage.population.max(total);
    Ok(roots
        .into_iter()
        .map(|(declaration, index)| w::Handle { declaration, index })
        .collect())
}

fn build(
    package: &v2::AdmittedPackage,
    selection: Selection,
    limits: Limits,
    usage: &mut Usage,
) -> Result<Document, Error> {
    let package_wire = package.inherited();
    let artifact = package
        .artifact()
        .ok_or_else(|| invalid_selection("package.artifact"))?;
    let (declaration_index, selected_expression) = match &selection {
        Selection::Predicate {
            declaration,
            expression,
        } => (*declaration, expression.clone()),
        Selection::Temporal { declaration } => {
            let declaration_value = package_wire
                .declarations
                .get(usize::try_from(*declaration).map_err(|_| invalid_selection("declaration"))?)
                .ok_or_else(|| invalid_selection("declaration"))?;
            match &declaration_value.body {
                w::Body::Temporal { root, .. } => (*declaration, root.clone()),
                _ => return Err(invalid_selection("subject.kind")),
            }
        }
    };
    let declaration = package_wire
        .declarations
        .get(
            usize::try_from(declaration_index)
                .map_err(|_| invalid_selection("subject.declaration"))?,
        )
        .ok_or_else(|| invalid_selection("subject.declaration"))?;
    let source = package_wire
        .sources
        .get(usize::try_from(declaration.locus.source).map_err(|_| invalid_selection("source"))?)
        .ok_or_else(|| invalid_selection("source"))?;
    let package_definition = package_wire
        .definitions
        .get(
            usize::try_from(package_wire.package_definition)
                .map_err(|_| invalid_selection("package.package_definition"))?,
        )
        .ok_or_else(|| invalid_selection("package.package_definition"))?;

    let (subject, temporal, predicate_roots, parameters, captures) = match &selection {
        Selection::Predicate { expression, .. } => {
            if expression.declaration != declaration_index {
                return Err(invalid_selection("expression.declaration"));
            }
            let (parent_kind, parameters, selected) = match &declaration.body {
                w::Body::Predicate {
                    parameters, root, ..
                } => ("predicate", parameters.clone(), root == expression),
                w::Body::State { root, .. } => ("state", Vec::new(), root == expression),
                w::Body::Temporal {
                    activation, root, ..
                } => {
                    let (_, leaves, _, _) =
                        collect_temporal(declaration_index, declaration, root, limits, usage)?;
                    let activation_guard = matches!(
                        activation,
                        w::Activation::Each {
                            guard: w::Nullable(Some(guard)),
                            ..
                        } if guard == expression
                    );
                    (
                        "temporal",
                        Vec::new(),
                        activation_guard || leaves.iter().any(|leaf| leaf == expression),
                    )
                }
                w::Body::Protocol { .. } => {
                    let leaves = protocol_predicate_roots(
                        declaration_index,
                        &declaration.body,
                        limits,
                        usage,
                    )?;
                    (
                        "protocol",
                        Vec::new(),
                        leaves.iter().any(|leaf| leaf == expression),
                    )
                }
            };
            if !selected {
                return Err(invalid_selection("subject.leaf"));
            };
            (
                SubjectBinding::PredicateLeaf {
                    parent_kind: parent_kind.into(),
                    declaration: declaration_index,
                    leaf: expression.clone(),
                    parameters: parameters.clone(),
                },
                Vec::new(),
                vec![expression.clone()],
                parameters,
                Vec::new(),
            )
        }
        Selection::Temporal { .. } => {
            let w::Body::Temporal {
                clock,
                activation,
                captures,
                root,
                ..
            } = &declaration.body
            else {
                return Err(invalid_selection("subject.kind"));
            };
            let binding = package
                .package()
                .temporal_bindings
                .iter()
                .find(|binding| binding.declaration == declaration_index)
                .ok_or_else(|| invalid_selection("subject.clock_configuration"))?;
            if binding.definition != declaration.profile {
                return Err(invalid_selection("profiles.evaluation"));
            }
            let (temporal, predicates, operators, required_history) =
                collect_temporal(declaration_index, declaration, root, limits, usage)?;
            let history_boundary = match activation {
                w::Activation::Origin { .. } => HistoryBoundary::ExecutionOrigin,
                w::Activation::Each { .. } => HistoryBoundary::HistoryCutoff,
            };
            (
                SubjectBinding::TemporalSubject {
                    parent_kind: "temporal".into(),
                    declaration: declaration_index,
                    activation: activation.clone(),
                    clock: *clock,
                    clock_configuration: Box::new(binding.clock.clone()),
                    captures: captures.clone(),
                    root: root.clone(),
                    history_boundary,
                    operators,
                    required_history,
                    predicate_leaves: predicates.clone(),
                },
                temporal,
                predicates,
                Vec::new(),
                captures.clone(),
            )
        }
    };
    let values = collect_values(
        declaration_index,
        declaration,
        &predicate_roots,
        limits,
        usage,
    )?;
    let selected_boolean = match &selection {
        Selection::Predicate { .. } => Some(&selected_expression),
        Selection::Temporal { .. } => predicate_roots.first(),
    };
    let type_indices = boolean_type_indices(package_wire, declaration, &values, selected_boolean)?;
    let expression = ExpressionBinding {
        root: selected_expression,
        values,
        temporal,
    };
    let bindings = identity_bindings(
        package_wire,
        declaration_index,
        declaration,
        &expression,
        &captures,
        limits,
        usage,
    )?;
    let mut definedness = BTreeMap::new();
    for value in &expression.values {
        let selected = definition(package_wire, value.value.profile, "profiles.definedness")?;
        let key = format!(
            "{}\0{}\0{}\0{}",
            selected.identity,
            selected.revision_namespace,
            selected.revision,
            selected.artifact.digest
        );
        definedness.insert(key, selected);
    }
    let evaluation = definition(package_wire, declaration.profile, "profiles.evaluation")?;
    usage.visited_fields = usage
        .visited_fields
        .checked_add(
            expression
                .values
                .len()
                .saturating_add(expression.temporal.len())
                .saturating_add(bindings.len())
                .saturating_add(parameters.len()),
        )
        .filter(|visited| *visited <= limits.visited_fields)
        .ok_or_else(|| exhausted("limits.visited_fields"))?;
    let contract = match selection {
        Selection::Predicate { .. } => "quire.checked-predicate/v1",
        Selection::Temporal { .. } => "quire.checked-temporal-subject/v1",
    };
    let mut wire = WireDocument {
        contract: contract.into(),
        identity: "0".repeat(64),
        package: PackageBinding {
            artifact: artifact.clone(),
            digest: format!("{:x}", package.digest()),
            native_identity: source.native.identity.clone(),
            native_revision: source.native.revision.clone(),
            package_definition: package_definition.identity.clone(),
        },
        subject,
        source: SourceBinding {
            source: source.clone(),
            span: declaration.locus.span.clone(),
        },
        clause: ClauseBinding {
            requirement: declaration.requirement.clone(),
            clause: declaration.clause.clone(),
            execution: declaration.execution.clone(),
        },
        expression,
        bindings,
        value_type: TypeBinding {
            kind: "boolean".into(),
            indices: type_indices,
        },
        profiles: ProfileBinding {
            evaluation,
            definedness: definedness.into_values().collect(),
        },
        limits: LimitsBinding {
            ceilings: limits.into(),
            measured: MeasureBinding {
                output_bytes: 0,
                json_depth: 0,
                string_bytes: 0,
                population: usage.population,
                expression_depth: usage.expression_depth,
                visited_fields: usage.visited_fields,
            },
        },
    };
    let semantic_usage = *usage;
    let mut measured_converged = false;
    for _ in 0..4 {
        let bytes = encode(&wire, limits.output_bytes)?;
        let json = inspect_json(&bytes, limits)?;
        let visited_fields = semantic_usage
            .visited_fields
            .checked_add(json.visited_fields)
            .filter(|visited| *visited <= limits.visited_fields)
            .ok_or_else(|| exhausted("limits.visited_fields"))?;
        let measured = MeasureBinding {
            output_bytes: bytes.len(),
            json_depth: json.depth,
            string_bytes: json.string_bytes,
            population: semantic_usage.population.max(json.population),
            expression_depth: semantic_usage.expression_depth,
            visited_fields,
        };
        if wire.limits.measured == measured {
            measured_converged = true;
            break;
        }
        wire.limits.measured = measured;
    }
    if !measured_converged {
        return Err(invalid_document("limits.measured"));
    }
    let preimage = IdentityPreimage {
        contract: &wire.contract,
        package: &wire.package,
        subject: &wire.subject,
        source: &wire.source,
        clause: &wire.clause,
        expression: &wire.expression,
        bindings: &wire.bindings,
        value_type: &wire.value_type,
        profiles: &wire.profiles,
        limits: &wire.limits,
    };
    wire.identity = identity(contract, &preimage, limits)?;
    let bytes = encode(&wire, limits.output_bytes)?;
    if bytes.len() != wire.limits.measured.output_bytes {
        return Err(invalid_document("limits.measured.output_bytes"));
    }
    usage.output_bytes = bytes.len();
    usage.json_depth = wire.limits.measured.json_depth;
    usage.string_bytes = wire.limits.measured.string_bytes;
    usage.population = wire.limits.measured.population;
    usage.visited_fields = wire.limits.measured.visited_fields;
    Ok(Document {
        digest: ByteDigest::of(&bytes),
        bytes,
        wire,
    })
}

pub(super) fn derive(
    package: &v2::AdmittedPackage,
    selection: Selection,
    limits: Limits,
) -> Report<Document> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = build(package, selection, limits, &mut usage);
    Report {
        result,
        limits,
        usage,
    }
}

fn json_preflight(bytes: &[u8], limits: Limits) -> Result<(usize, usize), Error> {
    let mut depth = 0usize;
    let mut maximum = 0usize;
    let mut string_bytes = 0usize;
    let mut maximum_string_bytes = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for byte in bytes {
        if quoted {
            if !escaped && *byte == b'"' {
                quoted = false;
                maximum_string_bytes = maximum_string_bytes.max(string_bytes);
                continue;
            }
            string_bytes = string_bytes
                .checked_add(1)
                .filter(|length| *length <= limits.string_bytes)
                .ok_or_else(|| exhausted("limits.string_bytes"))?;
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
                string_bytes = 0;
            }
            b'{' | b'[' => {
                depth = depth
                    .checked_add(1)
                    .filter(|depth| *depth <= limits.json_depth)
                    .ok_or_else(|| exhausted("limits.json_depth"))?;
                maximum = maximum.max(depth);
            }
            b'}' | b']' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid_document("document"))?;
            }
            _ => {}
        }
    }
    if quoted || depth != 0 {
        return Err(invalid_document("document"));
    }
    Ok((maximum, maximum_string_bytes))
}

pub(super) fn read(
    bytes: &[u8],
    package: &v2::AdmittedPackage,
    selection: Selection,
    limits: Limits,
) -> Report<Document> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        if bytes.len() > limits.input_bytes {
            return Err(exhausted("limits.input_bytes"));
        }
        usage.input_bytes = bytes.len();
        let input_usage = inspect_json(bytes, limits)?;
        usage.json_depth = input_usage.depth;
        usage.string_bytes = input_usage.string_bytes;
        usage.population = input_usage.population;
        usage.visited_fields = input_usage.visited_fields;
        let mut derived_usage = Usage::default();
        let expected = build(package, selection, limits, &mut derived_usage)?;
        usage.output_bytes = derived_usage.output_bytes;
        usage.string_bytes = usage.string_bytes.max(derived_usage.string_bytes);
        usage.population = usage.population.max(derived_usage.population);
        usage.expression_depth = derived_usage.expression_depth;
        usage.visited_fields = usage
            .visited_fields
            .checked_add(derived_usage.visited_fields)
            .filter(|visited| *visited <= limits.visited_fields)
            .ok_or_else(|| exhausted("limits.visited_fields"))?;
        if bytes != expected.bytes() {
            return Err(Error {
                code: ErrorCode::NonCanonical,
                path: "document",
            });
        }
        Ok(expected)
    })();
    Report {
        result,
        limits,
        usage,
    }
}
