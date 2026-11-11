// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-298: deterministic renderings of typed inspect values.

use alloc::{format, string::String, vec::Vec};
use core::fmt;
use qsl_foundation::{
    diagnostic::{LimitExceeded, StageFailure, Staged},
    setting::Setting,
    source::{
        provenance::{RawSourceRef, SourceRegion},
        Source,
    },
};
use qsl_replay::{Certification, OutcomeDocument, RenderLimits};
use quire_exact::Cancel;
use quire_semantic_value::catalog::{CatalogCode, CatalogCoded};
use serde::Serialize;

use crate::{
    CounterexampleView, ModelCheckStateGraphView, OutcomeView, StateGraphView, TraceView, View,
};

const VIEW_FORMAT: &str = "quire.inspect-view/v1";
const LARGE_VALUE_BYTES: usize = 4096;
const VALUE_TEXT_FAULT: &str = "a counterexample value has no value text";

/// The requested output representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderForm {
    /// The outcome document or typed view's JSON representation.
    Json,
    /// A deterministic human-readable representation.
    Text,
    /// A Graphviz directed graph for trace and state-graph views.
    Dot,
}

impl RenderForm {
    fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Text => "text",
            Self::Dot => "dot",
        }
    }
}

/// Why a render request cannot produce bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderCause {
    /// DOT is defined only for trace and state-graph views.
    UnsupportedForm {
        /// The typed view kind.
        view: &'static str,
        /// The requested form.
        form: &'static str,
    },
}

impl CatalogCoded for RenderCause {
    fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::UnsupportedForm { .. } => {
                CatalogCode::new("invalid_quire_context", "unsupported-render-form")
            }
        }
    }

    fn catalog_fields(&self) -> Option<alloc::collections::BTreeMap<&'static str, String>> {
        let Self::UnsupportedForm { view, form } = self;
        Some(alloc::collections::BTreeMap::from([
            ("view", String::from(*view)),
            ("form", String::from(*form)),
        ]))
    }
}

impl fmt::Display for RenderCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedForm { view, form } => {
                write!(f, "{form} rendering is unsupported for the {view} view")
            }
        }
    }
}

/// Admitted source documents keyed by their own source reference.
///
/// The source collection can be empty:
///
/// ```
/// let sources = qsl_inspect::RenderSources::from_sources(Vec::new());
/// let _ = sources;
/// ```
///
/// Callers cannot choose a key:
///
/// ```compile_fail
/// let sources = qsl_inspect::RenderSources { sources: Vec::new() };
/// ```
///
/// Nor can a caller build one from a map of keys to sources:
///
/// ```compile_fail
/// let map = std::collections::BTreeMap::<String, qsl_foundation::source::Source>::new();
/// let sources = qsl_inspect::RenderSources::from(map);
/// ```
///
/// Nor insert a source under a key of their choosing:
///
/// ```compile_fail
/// fn insert(sources: &mut qsl_inspect::RenderSources, source: qsl_foundation::source::Source) {
///     sources.insert(String::from("key"), source);
/// }
/// ```
#[derive(Clone)]
pub struct RenderSources {
    sources: Vec<Source>,
}

impl fmt::Debug for RenderSources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderSources")
            .field("source_count", &self.sources.len())
            .finish()
    }
}

impl RenderSources {
    /// Build the source provision from admitted sources, retaining the first
    /// source for each source reference in input order.
    pub fn from_sources(sources: impl IntoIterator<Item = Source>) -> Self {
        let mut retained = Vec::new();
        for source in sources {
            if !retained
                .iter()
                .any(|item: &Source| item.reference() == source.reference())
            {
                retained.push(source);
            }
        }
        Self { sources: retained }
    }

    fn get(&self, region: &SourceRegion) -> Option<&Source> {
        self.sources
            .iter()
            .find(|source| source.reference() == region.source())
    }
}

/// Complete rendered output. Failed renders return no partial value.
#[derive(Clone, Eq, PartialEq)]
pub struct Rendered {
    text: String,
}

impl fmt::Debug for Rendered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rendered")
            .field("byte_len", &self.text.len())
            .finish()
    }
}

impl Rendered {
    /// The complete UTF-8 output bytes.
    pub fn bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    /// The rendered UTF-8 text.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

type Failure = StageFailure<RenderCause>;
type Step = Result<(), Failure>;

fn fault(message: &'static str) -> Failure {
    StageFailure::Fault(quire_semantic_value::catalog::InternalFault::new(
        "P", message,
    ))
}

/// The one place output is produced (FR-298 "Limits and cancellation"). A
/// unit is one line of text, one DOT statement or a whole JSON document. For
/// each unit the writer polls `Cancel`, formats the unit, charges its UTF-8
/// byte length against `render.output_bytes`, and only then appends it, so a
/// refused, limited or cancelled render stops before the next unit's work.
struct Writer<'a> {
    out: String,
    used: u128,
    bound: u64,
    charges: usize,
    cancel: &'a Cancel,
    observe: &'a mut dyn FnMut(usize),
}

impl<'a> Writer<'a> {
    fn new(limits: RenderLimits, cancel: &'a Cancel, observe: &'a mut dyn FnMut(usize)) -> Self {
        Self {
            out: String::new(),
            used: 0,
            bound: u64::try_from(limits.output_bytes).unwrap_or(u64::MAX),
            charges: 0,
            cancel,
            observe,
        }
    }

    fn try_unit(&mut self, make: impl FnOnce() -> Result<String, Failure>) -> Step {
        if let Some(cause) = self.cancel.cause() {
            return Err(StageFailure::Cancelled(cause));
        }
        let unit = make()?;
        let attempted = self.used + unit.len() as u128;
        if attempted > u128::from(self.bound) {
            return Err(StageFailure::Limit(LimitExceeded::new(
                Setting::RenderOutputBytes,
                self.bound,
                attempted,
            )));
        }
        self.used = attempted;
        self.charges += 1;
        (self.observe)(self.charges);
        self.out.push_str(&unit);
        Ok(())
    }

    fn unit(&mut self, make: impl FnOnce() -> String) -> Step {
        self.try_unit(|| Ok(make()))
    }

    fn finish(self) -> Staged<Rendered> {
        Staged::new(Rendered { text: self.out })
    }
}

fn unsupported(view: &'static str, form: RenderForm) -> Failure {
    StageFailure::Refused(RenderCause::UnsupportedForm {
        view,
        form: form.as_str(),
    })
}

/// Render an outcome document as its authoritative FR-286 JSON or text.
pub fn render_outcome(
    document: &OutcomeDocument,
    form: RenderForm,
    sources: &RenderSources,
    limits: RenderLimits,
    cancel: &Cancel,
) -> Result<Staged<Rendered>, Failure> {
    render_outcome_observed(document, form, sources, limits, cancel, &mut |_| {})
}

/// [`render_outcome`] reporting each output charge to `observe`.
fn render_outcome_observed(
    document: &OutcomeDocument,
    form: RenderForm,
    sources: &RenderSources,
    limits: RenderLimits,
    cancel: &Cancel,
    observe: &mut dyn FnMut(usize),
) -> Result<Staged<Rendered>, Failure> {
    let mut writer = Writer::new(limits, cancel, observe);
    match form {
        RenderForm::Json => writer.try_unit(|| {
            let bytes = document
                .to_bytes()
                .map_err(|_| fault("outcome JSON serialization failed"))?;
            String::from_utf8(bytes).map_err(|_| fault("outcome JSON is not UTF-8"))
        })?,
        RenderForm::Text => outcome_text(&mut writer, document, None, sources)?,
        RenderForm::Dot => return Err(unsupported("outcome", form)),
    }
    Ok(writer.finish())
}

/// Render a typed inspection view without parsing or feeding back rendered data.
///
/// `T` must be canonically encodable so that the JSON form names a transition
/// by its canonical identity rather than by its `Debug` text.
pub fn render_view<S, T, F>(
    view: &View<'_, S, T, F>,
    form: RenderForm,
    sources: &RenderSources,
    limits: RenderLimits,
    cancel: &Cancel,
) -> Result<Staged<Rendered>, Failure>
where
    S: fmt::Debug,
    T: fmt::Debug + quire_canonical::Encode,
    F: fmt::Debug,
{
    render_view_observed(view, form, sources, limits, cancel, &mut |_| {})
}

/// [`render_view`] reporting each output charge to `observe`.
fn render_view_observed<S, T, F>(
    view: &View<'_, S, T, F>,
    form: RenderForm,
    sources: &RenderSources,
    limits: RenderLimits,
    cancel: &Cancel,
    observe: &mut dyn FnMut(usize),
) -> Result<Staged<Rendered>, Failure>
where
    S: fmt::Debug,
    T: fmt::Debug + quire_canonical::Encode,
    F: fmt::Debug,
{
    let mut writer = Writer::new(limits, cancel, observe);
    match (form, view) {
        (RenderForm::Json, _) => writer.try_unit(|| view_json(view, limits.output_bytes))?,
        (RenderForm::Text, _) => view_text(&mut writer, view, sources)?,
        (RenderForm::Dot, View::Trace(trace)) => trace_dot(&mut writer, trace)?,
        (RenderForm::Dot, View::StateGraph(graph)) => retained_graph_dot(&mut writer, graph)?,
        (RenderForm::Dot, View::ModelCheckStateGraph(graph)) => {
            model_check_graph_dot(&mut writer, graph)?
        }
        (RenderForm::Dot, View::Package(_)) => return Err(unsupported("package", form)),
        (RenderForm::Dot, View::Node(_)) => return Err(unsupported("node", form)),
        (RenderForm::Dot, View::Outcome(_)) => return Err(unsupported("outcome", form)),
        (RenderForm::Dot, View::Counterexample(_)) => {
            return Err(unsupported("counterexample", form))
        }
    }
    Ok(writer.finish())
}

/// A serde wire label (`snake_case` or `lowercase`) of a closed enum.
fn wire_label<T: Serialize>(value: &T) -> Result<String, Failure> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(label)) => Ok(label),
        _ => Err(fault("a label did not serialize as a string")),
    }
}

fn outcome_text(
    writer: &mut Writer<'_>,
    document: &OutcomeDocument,
    view: Option<&OutcomeView<'_>>,
    sources: &RenderSources,
) -> Step {
    if let Some(view) = view {
        // `inspect_outcome` yields one locus entry per diagnostic, in order.
        if view.loci.len() != document.diagnostics().len() {
            return Err(fault("outcome loci do not align with its diagnostics"));
        }
    }
    if document.diagnostics().is_empty() {
        writer.unit(|| format!("Outcome: {}\n", document.category().as_str()))?;
    }
    for (index, diagnostic) in document.diagnostics().iter().enumerate() {
        let region = view
            .and_then(|view| view.loci.get(index))
            .and_then(|(_, regions)| regions.first())
            .cloned()
            .or_else(|| {
                diagnostic.locus().and_then(|locus| match locus {
                    qsl_replay::OutcomeLocus::Region { source, start, end } => {
                        SourceRegion::new(source.clone(), *start, *end).ok()
                    }
                    _ => None,
                })
            });
        writer.unit(|| {
            let prefix = region.map_or_else(
                || String::from("<no-source>"),
                |region| match sources
                    .get(&region)
                    .and_then(|source| source.render(&region).map(|span| (source, span)))
                {
                    Some((source, span)) => format!(
                        "{}:{}:{}",
                        source.path(),
                        span.start.line,
                        span.start.column
                    ),
                    None => format!(
                        "{}:{}-{}",
                        source_label(region.source()),
                        region.start(),
                        region.end()
                    ),
                },
            );
            format!(
                "{prefix}: {}: {}\n",
                diagnostic.code(),
                diagnostic.message()
            )
        })?;
    }
    for item in document.items() {
        writer.try_unit(|| {
            let mut line = format!(
                "Item {}: {}",
                item.request_index().get(),
                wire_label(&item.result())?
            );
            if let Some(certification) = item.certification() {
                line.push(' ');
                line.push_str(&wire_label(&certification)?);
            }
            line.push('\n');
            Ok(line)
        })?;
    }
    Ok(())
}

// ---- JSON ---------------------------------------------------------------

#[derive(Serialize)]
struct JsonView<'a> {
    format: &'static str,
    view: JsonViewBody<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum JsonViewBody<'a> {
    Package(PackageJson<'a>),
    Node(NodeJson<'a>),
    Outcome(OutcomeJson<'a>),
    Counterexample(CounterexampleJson<'a>),
    Trace(TraceJson<'a>),
    StateGraph(SimulationGraphJson),
    ModelCheckStateGraph(ModelCheckGraphJson<'a>),
}

#[derive(Serialize)]
struct PackageJson<'a> {
    dependencies: Vec<&'a str>,
    model_selections: Vec<&'a str>,
    exports: Vec<NodeSummary<'a>>,
    requirements: Vec<RequirementJson<'a>>,
    capability_report: &'a [quire_contract_model::CheckedCapability],
}

#[derive(Serialize)]
struct NodeJson<'a> {
    node: NodeSummary<'a>,
    occurrences: Vec<OccurrenceJson<'a>>,
    requirements: Vec<RequirementJson<'a>>,
}

#[derive(Serialize)]
struct NodeSummary<'a> {
    key: String,
    tag: serde_json::Value,
    semantic_form: &'static str,
    semantic_type: Option<String>,
    declaration: Option<Vec<&'a str>>,
    owner: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct OccurrenceKeyJson<'a> {
    node: String,
    role: &'a str,
    ordinal: u64,
}

#[derive(Serialize)]
struct RequirementJson<'a> {
    occurrence: OccurrenceKeyJson<'a>,
    owner: Option<&'a str>,
    extent_roots: Vec<String>,
}

#[derive(Serialize)]
struct OccurrenceJson<'a> {
    key: OccurrenceKeyJson<'a>,
    regions: Vec<RegionJson<'a>>,
}

#[derive(Serialize)]
struct RegionJson<'a> {
    source_authority: &'a str,
    source_identity: &'a str,
    start: u64,
    end: u64,
}

#[derive(Serialize)]
struct OutcomeJson<'a> {
    document: &'a OutcomeDocument,
    loci: Vec<LocusJson<'a>>,
}

#[derive(Serialize)]
struct LocusJson<'a> {
    locus: &'a Option<qsl_replay::OutcomeLocus>,
    regions: Vec<RegionJson<'a>>,
}

#[derive(Serialize)]
struct CounterexampleJson<'a> {
    function: String,
    assignments: Vec<AssignmentJson<'a>>,
    certification: Option<&'static str>,
}

#[derive(Serialize)]
struct AssignmentJson<'a> {
    parameter: &'a str,
    declaration_order: usize,
    identity: String,
    value_text: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ShapeJson {
    Finite { len: u64 },
    Lasso { prefix: u64, loop_len: u64 },
}

#[derive(Serialize)]
struct TraceJson<'a> {
    shape: ShapeJson,
    positions: Vec<TracePositionJson<'a>>,
    loop_start: Option<usize>,
    deciding_position: Option<usize>,
}

#[derive(Serialize)]
struct TracePositionJson<'a> {
    position: usize,
    authority: &'a str,
    identity: &'a str,
}

#[derive(Serialize)]
struct StatsJson {
    states: usize,
    transitions: usize,
    depth: usize,
}

#[derive(Serialize)]
struct CauseJson {
    code: &'static str,
    cause: &'static str,
}

#[derive(Serialize)]
struct SettingValueJson {
    setting: &'static str,
    value: usize,
}

#[derive(Serialize)]
struct CompletionJson {
    kind: &'static str,
    stats: StatsJson,
    horizon: Option<usize>,
    limit: Option<SettingValueJson>,
    cause: Option<CauseJson>,
}

#[derive(Serialize)]
struct SimulationStateJson {
    index: usize,
    key_hex: String,
    depth: usize,
    expanded: bool,
    findings: usize,
}

#[derive(Serialize)]
struct SimulationEdgeJson {
    source: usize,
    target: usize,
    transition_hex: String,
}

#[derive(Serialize)]
struct SimulationGraphJson {
    states: Vec<SimulationStateJson>,
    transitions: Vec<SimulationEdgeJson>,
    completion: CompletionJson,
    partial: bool,
}

#[derive(Serialize)]
struct ModelCheckNodeJson {
    index: usize,
    key_hex: String,
    depth: u64,
}

#[derive(Serialize)]
struct ModelCheckEdgeJson<'a> {
    source: usize,
    target: usize,
    operation: &'a str,
    receiver: Option<&'a str>,
    arguments: &'a [String],
}

#[derive(Serialize)]
struct ModelCheckLimitJson {
    limit: &'static str,
    value: u64,
}

#[derive(Serialize)]
struct RunEndJson {
    kind: &'static str,
    cause: Option<&'static str>,
    limit: Option<ModelCheckLimitJson>,
}

#[derive(Serialize)]
struct OpenNodeJson {
    node: usize,
    cause: &'static str,
}

#[derive(Serialize)]
struct ModelCheckGraphJson<'a> {
    nodes: Vec<ModelCheckNodeJson>,
    edges: Vec<ModelCheckEdgeJson<'a>>,
    end: RunEndJson,
    open: Vec<OpenNodeJson>,
    partial: bool,
}

fn view_json<S, T, F>(view: &View<'_, S, T, F>, encode_limit: usize) -> Result<String, Failure>
where
    T: quire_canonical::Encode,
{
    let body = match view {
        View::Package(package) => JsonViewBody::Package(PackageJson {
            dependencies: package
                .dependencies
                .iter()
                .map(|(id, _)| id.as_str())
                .collect(),
            model_selections: package
                .model_selections
                .iter()
                .map(|entry| entry.identity.as_str())
                .collect(),
            exports: package
                .exports
                .iter()
                .map(|node| node_summary(node))
                .collect::<Result<_, _>>()?,
            requirements: package
                .requirements
                .iter()
                .map(|(key, record)| requirement_json(key, record))
                .collect(),
            capability_report: package.capability_report,
        }),
        View::Node(node) => JsonViewBody::Node(NodeJson {
            node: node_summary(node.node)?,
            occurrences: node
                .occurrences
                .iter()
                .map(|occurrence| OccurrenceJson {
                    key: occurrence_key_json(&occurrence.key),
                    regions: occurrence.regions.iter().map(region_json).collect(),
                })
                .collect(),
            requirements: node
                .requirements
                .iter()
                .map(|(key, record)| requirement_json(key, record))
                .collect(),
        }),
        View::Outcome(outcome) => JsonViewBody::Outcome(OutcomeJson {
            document: outcome.document,
            loci: outcome
                .loci
                .iter()
                .map(|(locus, regions)| LocusJson {
                    locus,
                    regions: regions.iter().map(region_json).collect(),
                })
                .collect(),
        }),
        View::Counterexample(counterexample) => {
            JsonViewBody::Counterexample(counterexample_json(counterexample)?)
        }
        View::Trace(trace) => JsonViewBody::Trace(trace_json(trace)),
        View::StateGraph(graph) => {
            JsonViewBody::StateGraph(simulation_graph_json(graph, encode_limit)?)
        }
        View::ModelCheckStateGraph(graph) => {
            JsonViewBody::ModelCheckStateGraph(model_check_graph_json(graph))
        }
    };
    serde_json::to_string(&JsonView {
        format: VIEW_FORMAT,
        view: body,
    })
    .map_err(|_| fault("typed view JSON serialization failed"))
}

fn node_summary(node: &qsl_semantics::check::SemanticNode) -> Result<NodeSummary<'_>, Failure> {
    let value = |result: Result<serde_json::Value, serde_json::Error>| {
        result.map_err(|_| fault("a node field did not serialize"))
    };
    Ok(NodeSummary {
        key: node.key().to_string(),
        tag: value(serde_json::to_value(node.node_tag()))?,
        semantic_form: node.semantic_form(),
        semantic_type: node.semantic_type().map(|key| key.to_string()),
        declaration: node
            .declaration()
            .map(|path| path.iter().map(|part| part.as_str()).collect()),
        owner: node
            .owner()
            .map(|owner| value(serde_json::to_value(owner)))
            .transpose()?,
    })
}

fn occurrence_key_json(
    key: &qsl_foundation::source::provenance::OccurrenceKey,
) -> OccurrenceKeyJson<'_> {
    OccurrenceKeyJson {
        node: key.node().to_string(),
        role: key.origin().role().as_str(),
        ordinal: key.origin().ordinal(),
    }
}

fn requirement_json<'a>(
    key: &'a qsl_foundation::source::provenance::OccurrenceKey,
    record: &'a qsl_semantics::check::RequirementRecord,
) -> RequirementJson<'a> {
    RequirementJson {
        occurrence: occurrence_key_json(key),
        owner: record.owner(),
        extent_roots: record
            .extent_roots()
            .iter()
            .map(ToString::to_string)
            .collect(),
    }
}

fn region_json(region: &SourceRegion) -> RegionJson<'_> {
    RegionJson {
        source_authority: region.source().authority(),
        source_identity: region.source().identity(),
        start: region.start(),
        end: region.end(),
    }
}

fn counterexample_json<'a>(
    view: &'a CounterexampleView<'_>,
) -> Result<CounterexampleJson<'a>, Failure> {
    Ok(CounterexampleJson {
        function: view.function.to_string(),
        assignments: view
            .assignments
            .iter()
            .map(|assignment| {
                Ok(AssignmentJson {
                    parameter: assignment.parameter,
                    declaration_order: assignment.declaration_order,
                    identity: assignment.identity.to_string(),
                    value_text: assignment
                        .value
                        .to_value_text()
                        .map_err(|_| fault(VALUE_TEXT_FAULT))?,
                })
            })
            .collect::<Result<_, Failure>>()?,
        certification: view.certification.map(Certification::as_str),
    })
}

fn trace_json<'a>(view: &'a TraceView<'_>) -> TraceJson<'a> {
    TraceJson {
        shape: match view.shape {
            qsl_eval::temporal::TemporalTrace::Finite { len } => ShapeJson::Finite { len },
            qsl_eval::temporal::TemporalTrace::Lasso { prefix, loop_len } => {
                ShapeJson::Lasso { prefix, loop_len }
            }
        },
        positions: view
            .positions
            .iter()
            .map(|position| TracePositionJson {
                position: position.position,
                authority: &position.state.authority,
                identity: &position.state.identity,
            })
            .collect(),
        loop_start: view.loop_start,
        deciding_position: view.deciding_position,
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    text
}

fn completion_json(outcome: &qsl_eval::simulation::Outcome) -> CompletionJson {
    use qsl_eval::simulation::Outcome;
    let stats = |stats: &qsl_eval::simulation::Stats| StatsJson {
        states: stats.states,
        transitions: stats.transitions,
        depth: stats.depth,
    };
    let cause = |cause: &CatalogCode| CauseJson {
        code: cause.code(),
        cause: cause.cause(),
    };
    let mut json = CompletionJson {
        kind: "",
        stats: StatsJson {
            states: 0,
            transitions: 0,
            depth: 0,
        },
        horizon: None,
        limit: None,
        cause: None,
    };
    match outcome {
        Outcome::Exhaustive(counts) => {
            json.kind = "exhaustive";
            json.stats = stats(counts);
        }
        Outcome::BoundReached {
            stats: counts,
            depth,
            ..
        } => {
            json.kind = "bound_reached";
            json.stats = stats(counts);
            json.horizon = Some(*depth);
        }
        Outcome::Bounded {
            stats: counts,
            limit,
            ..
        } => {
            json.kind = "bounded";
            json.stats = stats(counts);
            let value = match limit {
                qsl_eval::simulation::Limit::States(value)
                | qsl_eval::simulation::Limit::Transitions(value) => *value,
            };
            json.limit = Some(SettingValueJson {
                setting: limit.setting().name(),
                value,
            });
        }
        Outcome::Cancelled {
            stats: counts,
            cause: why,
            ..
        } => {
            json.kind = "cancelled";
            json.stats = stats(counts);
            json.cause = Some(cause(why));
        }
        Outcome::Stopped {
            stats: counts,
            cause: why,
            ..
        } => {
            json.kind = "stopped";
            json.stats = stats(counts);
            json.cause = Some(cause(why));
        }
    }
    json
}

fn simulation_graph_json<S, T, F>(
    view: &StateGraphView<'_, S, T, F>,
    encode_limit: usize,
) -> Result<SimulationGraphJson, Failure>
where
    T: quire_canonical::Encode,
{
    let limits = quire_canonical::Limits::new(u64::try_from(encode_limit).unwrap_or(u64::MAX));
    let mut transitions = Vec::with_capacity(view.transitions.len());
    for edge in &view.transitions {
        let bytes = quire_canonical::to_vec(&edge.transition, limits)
            .map_err(|_| fault("a transition identity has no canonical encoding"))?;
        transitions.push(SimulationEdgeJson {
            source: edge.source.index(),
            target: edge.target.index(),
            transition_hex: hex_bytes(&bytes),
        });
    }
    Ok(SimulationGraphJson {
        states: view
            .states
            .iter()
            .enumerate()
            .map(|(index, state)| SimulationStateJson {
                index,
                key_hex: hex_bytes(&state.key),
                depth: state.depth,
                expanded: state.expanded,
                findings: state.findings.len(),
            })
            .collect(),
        transitions,
        completion: completion_json(view.outcome),
        partial: view.partial,
    })
}

fn open_cause_label(cause: qsl_replay::OpenCause) -> &'static str {
    match cause {
        qsl_replay::OpenCause::MaxDepth => "max_depth",
        qsl_replay::OpenCause::UndecidedSuccessor => "undecided_successor",
        qsl_replay::OpenCause::NotExpanded => "not_expanded",
    }
}

fn model_check_graph_json<'a>(view: &'a ModelCheckStateGraphView<'_>) -> ModelCheckGraphJson<'a> {
    let end = match view.end {
        qsl_replay::RunEnd::Completed => RunEndJson {
            kind: "completed",
            cause: None,
            limit: None,
        },
        qsl_replay::RunEnd::Stopped { cause, limit } => RunEndJson {
            kind: "stopped",
            cause: Some(cause.as_str()),
            limit: limit.as_ref().map(|reached| ModelCheckLimitJson {
                limit: reached.limit.as_str(),
                value: reached.value,
            }),
        },
    };
    ModelCheckGraphJson {
        nodes: view
            .states
            .iter()
            .enumerate()
            .map(|(index, node)| ModelCheckNodeJson {
                index,
                key_hex: hex_bytes(node.key.as_bytes()),
                depth: node.depth,
            })
            .collect(),
        edges: view
            .transitions
            .iter()
            .map(|edge| ModelCheckEdgeJson {
                source: edge.source.index(),
                target: edge.target.index(),
                operation: edge.transition.operation(),
                receiver: edge.transition.receiver(),
                arguments: edge.transition.arguments(),
            })
            .collect(),
        end,
        open: view
            .open
            .iter()
            .map(|(node, cause)| OpenNodeJson {
                node: node.index(),
                cause: open_cause_label(*cause),
            })
            .collect(),
        partial: view.partial,
    }
}

// ---- text ---------------------------------------------------------------

fn view_text<S, T, F>(
    writer: &mut Writer<'_>,
    view: &View<'_, S, T, F>,
    sources: &RenderSources,
) -> Step
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    match view {
        View::Package(package) => {
            for (identity, _) in &package.dependencies {
                writer.unit(|| format!("Dependency: {}\n", identity.as_str()))?;
            }
            for selection in package.model_selections {
                writer.unit(|| format!("Model selection: {}\n", selection.identity))?;
            }
            for node in &package.exports {
                writer.unit(|| {
                    format!(
                        "Export: {} {} {:?}\n",
                        node.key(),
                        node.semantic_form(),
                        node.declaration()
                    )
                })?;
            }
            for (occurrence, record) in &package.requirements {
                writer.unit(|| format!("Requirement {occurrence:?}: {record:?}\n"))?;
            }
            for capability in package.capability_report {
                writer.unit(|| format!("Capability: {capability:?}\n"))?;
            }
            Ok(())
        }
        View::Node(node) => {
            writer.unit(|| {
                format!(
                    "Node {} ({:?}, {})\n",
                    node.node.key(),
                    node.node.node_tag(),
                    node.node.semantic_form()
                )
            })?;
            writer.unit(|| format!("Semantic type: {:?}\n", node.node.semantic_type()))?;
            writer.unit(|| format!("Declaration: {:?}\n", node.node.declaration()))?;
            for occurrence in &node.occurrences {
                writer.unit(|| format!("Occurrence: {:?}\n", occurrence.key))?;
                for region in occurrence.regions {
                    writer.unit(|| {
                        format!(
                            "  Region: {}:{}-{}\n",
                            source_label(region.source()),
                            region.start(),
                            region.end()
                        )
                    })?;
                }
            }
            for (occurrence, record) in &node.requirements {
                writer.unit(|| format!("Requirement {occurrence:?}: {record:?}\n"))?;
            }
            Ok(())
        }
        View::Outcome(outcome) => outcome_text(writer, outcome.document, Some(outcome), sources),
        View::Counterexample(counterexample) => counterexample_text(writer, counterexample),
        View::Trace(trace) => trace_text(writer, trace),
        View::StateGraph(graph) => {
            writer.unit(|| format!("State graph (partial: {})\n", graph.partial))?;
            for state in &graph.states {
                writer.unit(|| format!("State: {state:?}\n"))?;
            }
            for transition in &graph.transitions {
                writer.unit(|| format!("Transition: {transition:?}\n"))?;
            }
            writer.unit(|| format!("Completion: {:?}\n", graph.outcome))
        }
        View::ModelCheckStateGraph(graph) => {
            writer.unit(|| format!("State graph (partial: {})\n", graph.partial))?;
            for state in &graph.states {
                writer.unit(|| format!("State: {state:?}\n"))?;
            }
            for transition in &graph.transitions {
                writer.unit(|| format!("Transition: {transition:?}\n"))?;
            }
            writer.unit(|| format!("Open frontier: {:?}\n", graph.open))?;
            writer.unit(|| format!("Completion: {:?}\n", graph.end))
        }
    }
}

fn counterexample_text(writer: &mut Writer<'_>, view: &CounterexampleView<'_>) -> Step {
    writer.unit(|| format!("Counterexample for {}\n", view.function))?;
    writer.unit(|| String::from("Parameter | Value\n"))?;
    let mut assignments = view.assignments.iter().collect::<Vec<_>>();
    assignments.sort_by_key(|assignment| assignment.declaration_order);
    for assignment in assignments {
        writer.try_unit(|| {
            // Decide the descriptor from the length first, so a large witness
            // is never materialized. A value that does not encode counts as
            // `usize::MAX`: it is a fault, never a plausible descriptor.
            let length = assignment.value.value_text_len();
            let value = if length == usize::MAX {
                return Err(fault(VALUE_TEXT_FAULT));
            } else if length > LARGE_VALUE_BYTES {
                format!("[value: {length} bytes]")
            } else {
                assignment
                    .value
                    .to_value_text()
                    .map_err(|_| fault(VALUE_TEXT_FAULT))?
            };
            Ok(format!("{} | {}\n", assignment.parameter, value))
        })?;
    }
    if let Some(certification) = view.certification {
        writer.unit(|| format!("Certification: {}\n", certification.as_str()))?;
    }
    Ok(())
}

fn trace_text(writer: &mut Writer<'_>, view: &TraceView<'_>) -> Step {
    for position in &view.positions {
        writer.unit(|| {
            let mut markers = Vec::new();
            if view.loop_start == Some(position.position) {
                markers.push("loop start");
            }
            if view.deciding_position == Some(position.position) {
                markers.push("deciding");
            }
            let suffix = if markers.is_empty() {
                String::new()
            } else {
                format!(" [{}]", markers.join(", "))
            };
            format!("{}: {:?}{suffix}\n", position.position, position.state)
        })?;
    }
    Ok(())
}

// ---- DOT ----------------------------------------------------------------

fn trace_dot(writer: &mut Writer<'_>, view: &TraceView<'_>) -> Step {
    writer.unit(|| String::from("digraph view {\n"))?;
    for position in &view.positions {
        writer.unit(|| {
            let label = format!("{}: {:?}", position.position, position.state);
            let shape = if view.deciding_position == Some(position.position) {
                ", shape=doublecircle"
            } else {
                ""
            };
            format!(
                "  n{} [label=\"{}\"{}];\n",
                position.position,
                dot_escape(&label),
                shape
            )
        })?;
        if position.position + 1 < view.positions.len() {
            writer.unit(|| format!("  n{} -> n{};\n", position.position, position.position + 1))?;
        }
    }
    if let (Some(last), Some(loop_start)) = (view.positions.last(), view.loop_start) {
        writer.unit(|| format!("  n{} -> n{};\n", last.position, loop_start))?;
    }
    writer.unit(|| String::from("}\n"))
}

fn retained_graph_dot<S, T, F>(writer: &mut Writer<'_>, view: &StateGraphView<'_, S, T, F>) -> Step
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    writer.unit(|| String::from("digraph view {\n"))?;
    for (index, state) in view.states.iter().enumerate() {
        writer.unit(|| {
            format!(
                "  n{index} [label=\"{}\"];\n",
                dot_escape(&format!("{state:?}"))
            )
        })?;
    }
    for edge in &view.transitions {
        writer.unit(|| {
            format!(
                "  n{} -> n{} [label=\"{}\"];\n",
                edge.source.index(),
                edge.target.index(),
                dot_escape(&format!("{:?}", edge.transition))
            )
        })?;
    }
    writer.unit(|| String::from("}\n"))
}

fn model_check_graph_dot(writer: &mut Writer<'_>, view: &ModelCheckStateGraphView<'_>) -> Step {
    writer.unit(|| String::from("digraph view {\n"))?;
    for (index, state) in view.states.iter().enumerate() {
        writer.unit(|| {
            format!(
                "  n{index} [label=\"{}\"];\n",
                dot_escape(&format!("{:?}", state.key))
            )
        })?;
    }
    for edge in &view.transitions {
        writer.unit(|| {
            format!(
                "  n{} -> n{} [label=\"{}\"];\n",
                edge.source.index(),
                edge.target.index(),
                dot_escape(&format!("{:?}", edge.transition))
            )
        })?;
    }
    writer.unit(|| String::from("}\n"))
}

fn dot_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn source_label(source: &RawSourceRef) -> String {
    format!("{}/{}", source.authority(), source.identity())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        inspect_node, inspect_package, inspect_state_graph, tests::emitted_spine_package,
        AssignmentView, InspectLimits, NodeSelector, OutcomeView, TracePositionView,
    };
    use ix_trace_rs::trace;
    use qsl_foundation::{diagnostic::LimitKind, source::SourceIdentity, Span};
    use quire_exact::CancelCause;
    use quire_semantic_value::catalog::Category;

    const EXPECTED_JSON: &[&str] = &[
        r#"refusal: {"format":"quire.inspect-view/v1","view":{"kind":"outcome","data":{"document":{"format":"quire-outcome/1","operation":"analyze","last_stage":null,"category":"refusal","items":[],"diagnostics":[{"cause":null,"code":"ill_typed","locus":{"kind":"region","source":{"authority":"tests","identity":"ref-inv"},"start":4,"end":5},"message":"bad value"}],"artifacts":[],"result":null},"loci":[{"locus":{"kind":"region","source":{"authority":"tests","identity":"ref-inv"},"start":4,"end":5},"regions":[{"source_authority":"tests","source_identity":"ref-inv","start":4,"end":5}]}]}}}"#,
        r#"proved: {"format":"quire.inspect-view/v1","view":{"kind":"outcome","data":{"document":{"format":"quire-outcome/1","operation":"prove","last_stage":null,"category":"success","items":[{"request_index":0,"result":"proved","cause":null,"basis":{"type":"inductive","depth":0},"certification":"trusted","category":"success"},{"request_index":1,"result":"proved","cause":null,"basis":{"type":"bounded-proof","checks":1},"certification":"certified","category":"success"},{"request_index":2,"result":"proved","cause":null,"basis":{"type":"exhaustive"},"certification":"uncertified","category":"success"},{"request_index":3,"result":"failed","cause":null,"category":"internal-failure"}],"diagnostics":[],"artifacts":[],"result":null},"loci":[]}}}"#,
        r#"counterexample: {"format":"quire.inspect-view/v1","view":{"kind":"counterexample","data":{"function":"check","assignments":[{"parameter":"p","declaration_order":0,"identity":"fixture:[\"x\",]:0","value_text":"{\"type\":\"boolean\",\"value\":true}"}],"certification":"trusted"}}}"#,
        r#"lasso: {"format":"quire.inspect-view/v1","view":{"kind":"trace","data":{"shape":{"kind":"lasso","prefix":2,"loop_len":2},"positions":[{"position":0,"authority":"tests","identity":"s0"},{"position":1,"authority":"tests","identity":"s1"},{"position":2,"authority":"tests","identity":"s2"},{"position":3,"authority":"tests","identity":"s3"}],"loop_start":2,"deciding_position":1}}}"#,
        r#"graph: {"format":"quire.inspect-view/v1","view":{"kind":"state_graph","data":{"states":[{"index":0,"key_hex":"30","depth":0,"expanded":true,"findings":0},{"index":1,"key_hex":"31","depth":1,"expanded":true,"findings":0},{"index":2,"key_hex":"32","depth":2,"expanded":true,"findings":0}],"transitions":[{"source":0,"target":1,"transition_hex":"227374657022"},{"source":1,"target":2,"transition_hex":"227374657022"}],"completion":{"kind":"exhaustive","stats":{"states":3,"transitions":2,"depth":2},"horizon":null,"limit":null,"cause":null},"partial":false}}}"#,
    ];

    type V<'a> = View<'a, usize, String, usize>;
    type Doc = qsl_semantics::model::observation::DocumentRef;

    fn source_under(identity: &str, path: &str, text: &str) -> Source {
        Source::read_typed(
            SourceIdentity::new("tests", identity),
            path,
            text.as_bytes(),
            text.len(),
        )
        .unwrap_or_else(|_| panic!("fixture source is admitted"))
    }

    fn snapshot(identity: &str) -> Doc {
        Doc {
            authority: String::from("tests"),
            identity: String::from(identity),
        }
    }

    fn refusal_over(region: &SourceRegion, end: u64, message: &str) -> OutcomeDocument {
        OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Refusal)
            .with_diagnostics(vec![qsl_replay::OutcomeDiagnostic::new(
                None,
                "ill_typed",
                Some(qsl_replay::OutcomeLocus::Region {
                    source: region.source().clone(),
                    start: region.start(),
                    end,
                }),
                message,
            )])
    }

    fn text_of(document: &OutcomeDocument, sources: &RenderSources) -> String {
        render_outcome(
            document,
            RenderForm::Text,
            sources,
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("diagnostic renders")
        .value()
        .as_str()
        .to_owned()
    }

    fn render_default<S, T, F>(view: &View<'_, S, T, F>, form: RenderForm) -> Rendered
    where
        S: fmt::Debug,
        T: fmt::Debug + quire_canonical::Encode,
        F: fmt::Debug,
    {
        render_view(
            view,
            form,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("view renders")
        .into_value()
    }

    fn trace_view<'a>(
        states: &'a [Doc],
        shape: qsl_eval::temporal::TemporalTrace,
        loop_start: Option<usize>,
        deciding_position: Option<usize>,
    ) -> V<'a> {
        View::Trace(TraceView {
            shape,
            positions: states
                .iter()
                .enumerate()
                .map(|(position, state)| TracePositionView { position, state })
                .collect(),
            loop_start,
            deciding_position,
        })
    }

    fn lasso(states: &[Doc]) -> V<'_> {
        trace_view(
            states,
            qsl_eval::temporal::TemporalTrace::Lasso {
                prefix: 2,
                loop_len: 2,
            },
            Some(2),
            Some(1),
        )
    }

    fn four_states() -> [Doc; 4] {
        [
            snapshot("s0"),
            snapshot("s1"),
            snapshot("s2"),
            snapshot("s3"),
        ]
    }

    /// A chain `0 -> 1 -> ... -> n - 1`.
    struct Chain(usize);

    impl qsl_eval::simulation::TransitionSystem for Chain {
        type State = usize;
        type TransitionId = String;
        type Key = usize;
        type Finding = usize;

        fn initial(&self) -> Vec<usize> {
            vec![0]
        }
        fn key(&self, state: &usize) -> usize {
            *state
        }
        fn successors(
            &self,
            state: &usize,
        ) -> Result<
            qsl_eval::simulation::Expansion<String, usize, usize>,
            qsl_eval::simulation::ExpansionStop,
        > {
            let successors = if state + 1 < self.0 {
                vec![(String::from("step"), state + 1)]
            } else {
                Vec::new()
            };
            Ok(qsl_eval::simulation::Expansion {
                successors,
                findings: Vec::new(),
            })
        }
    }

    fn explore_chain(
        states: usize,
    ) -> qsl_analyze::model_check::GraphExploration<usize, String, usize> {
        qsl_analyze::model_check::retain(
            &Chain(states),
            qsl_eval::simulation::Limits::default(),
            qsl_foundation::IdentityLimits::default(),
            None,
            || false,
        )
        .expect("a chain has canonical identities")
    }

    fn all_entries() -> InspectLimits {
        InspectLimits {
            max_view_entries: 1_000_000,
        }
    }

    fn graph_view(
        exploration: &qsl_analyze::model_check::GraphExploration<usize, String, usize>,
    ) -> V<'_> {
        inspect_state_graph(Some(exploration), all_entries(), &Cancel::new())
            .expect("the retained graph is inspectable")
            .into_value()
    }

    fn proved_document() -> OutcomeDocument {
        use qsl_replay::{OutcomeItem, RequestIndex, TerminalRecord};
        let backend = || qsl_foundation::backend::BackendId::new("plugin");
        let item = |record: TerminalRecord| OutcomeItem::from_terminal(&record);
        OutcomeDocument::new(qsl_replay::Operation::Prove, None, Category::Success).with_items(
            vec![
                item(TerminalRecord::proved_by_provider(
                    RequestIndex::new(0),
                    backend(),
                    qsl_replay::Certification::Trusted,
                )),
                item(TerminalRecord::proved(RequestIndex::new(1), backend(), 1)),
                item(TerminalRecord::exhaustive(RequestIndex::new(2), backend())),
                item(TerminalRecord::failed(RequestIndex::new(3), backend())),
            ],
        )
    }

    #[trace("TC-783", "FR-298-AC-1", "FR-298-AC-6")]
    #[test]
    fn diagnostic_text_resolves_path_and_scalar_column_or_falls_back_to_the_reference() {
        let text_source = "a\néx\n";
        let input = source_under("ref-inv", "src/inv.qsl", text_source);
        let region = input
            .region(Span { start: 4, end: 5 })
            .expect("region starts after multibyte scalar");
        let document = refusal_over(&region, region.end(), "bad value");
        // Column 2: the scalar column, not the byte-based 3.
        assert_eq!(
            text_of(&document, &RenderSources::from_sources([input.clone()])),
            "src/inv.qsl:2:2: ill_typed: bad value\n"
        );

        let fallback_label = format!(
            "{}:{}-{}: ill_typed:",
            source_label(region.source()),
            region.start(),
            region.end()
        );
        let unrelated =
            RenderSources::from_sources([source_under("ref-other", "src/other.qsl", "zzz\n")]);
        for sources in [RenderSources::from_sources([]), unrelated] {
            let text = text_of(&document, &sources);
            assert!(text.starts_with(&fallback_label), "{text}");
            assert!(!text.contains("src/"), "{text}");
        }

        // The region's end exactly one byte past the source: unresolved.
        let past_end_by_one = u64::try_from(text_source.len()).unwrap() + 1;
        let past_end = refusal_over(&region, past_end_by_one, "bad range");
        let text = text_of(&past_end, &RenderSources::from_sources([input.clone()]));
        assert_eq!(
            text,
            format!(
                "{}:4-{past_end_by_one}: ill_typed: bad range\n",
                source_label(region.source())
            )
        );
        // The region's end exactly at the source's length still resolves.
        let at_end = refusal_over(&region, u64::try_from(text_source.len()).unwrap(), "edge");
        assert!(
            text_of(&at_end, &RenderSources::from_sources([input])).starts_with("src/inv.qsl:2:2:")
        );
    }

    #[trace("TC-783", "FR-298-AC-11")]
    #[test]
    fn render_sources_hold_each_source_under_its_own_reference_and_keep_the_first() {
        let first = source_under("ref-a", "a.qsl", "alpha\n");
        let second = source_under("ref-b", "b.qsl", "beta\n");
        let region_a = first.region(Span { start: 0, end: 1 }).unwrap();
        let region_b = second.region(Span { start: 0, end: 1 }).unwrap();
        let sources = RenderSources::from_sources([first, second]);
        assert_eq!(
            text_of(&refusal_over(&region_a, 1, "m"), &sources),
            "a.qsl:1:1: ill_typed: m\n"
        );
        assert_eq!(
            text_of(&refusal_over(&region_b, 1, "m"), &sources),
            "b.qsl:1:1: ill_typed: m\n"
        );

        let under_a = source_under("ref-same", "a.qsl", "same\n");
        let under_b = source_under("ref-same", "b.qsl", "same\n");
        let region = under_a.region(Span { start: 0, end: 1 }).unwrap();
        let sources = RenderSources::from_sources([under_a, under_b]);
        assert_eq!(
            text_of(&refusal_over(&region, 1, "m"), &sources),
            "a.qsl:1:1: ill_typed: m\n"
        );
    }

    #[trace("TC-783", "FR-298-AC-4")]
    #[test]
    fn outcome_json_is_the_existing_authoritative_document_bytes() {
        let document =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Success);
        let rendered = render_outcome(
            &document,
            RenderForm::Json,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("outcome JSON renders");
        assert_eq!(
            rendered.value().bytes(),
            document.to_bytes().expect("document serializes")
        );
    }

    /// FR-298-AC-4 by variation: the same outcome rendered as text under two
    /// source sets reads differently (the path label changes), while its JSON
    /// is the same bytes both times and the document's own.
    #[trace("TC-783", "FR-298-AC-4")]
    #[test]
    fn outcome_json_does_not_change_when_the_text_rendering_does() {
        let under_a = source_under("ref-same", "a.qsl", "same\n");
        let under_b = source_under("ref-same", "b.qsl", "same\n");
        let region = under_a.region(Span { start: 0, end: 1 }).unwrap();
        let document = refusal_over(&region, 1, "m");
        let with_a = RenderSources::from_sources([under_a]);
        let with_b = RenderSources::from_sources([under_b]);
        assert_ne!(text_of(&document, &with_a), text_of(&document, &with_b));
        let json = |sources: &RenderSources| {
            render_outcome(
                &document,
                RenderForm::Json,
                sources,
                RenderLimits::default(),
                &Cancel::new(),
            )
            .expect("outcome JSON renders")
            .value()
            .bytes()
            .to_vec()
        };
        assert_eq!(json(&with_a), json(&with_b));
        assert_eq!(
            json(&with_a),
            document.to_bytes().expect("document serializes")
        );
    }

    /// The JSON of every AC-1 to AC-3 outcome and view is pinned to exact
    /// bytes. Text and DOT are rendered from the same values first, so a JSON
    /// that began to depend on either renderer (or on `Debug` text) would
    /// change these bytes.
    #[trace("TC-783", "FR-298-AC-4")]
    #[test]
    fn json_bytes_of_every_outcome_and_view_are_pinned_independent_of_text_and_dot() {
        let input = source_under("ref-inv", "src/inv.qsl", "a\néx\n");
        let region = input.region(Span { start: 4, end: 5 }).unwrap();
        let refusal = refusal_over(&region, region.end(), "bad value");
        let proved = proved_document();
        let function =
            qsl_replay::QualifiedName::new(vec![quire_exact::Identifier::new("check").unwrap()])
                .unwrap();
        let node = quire_exact::NodeKey::new(
            "fixture",
            vec![quire_exact::PathSegment::Text(String::from("x"))],
            0,
        )
        .unwrap();
        let value = qsl_replay::WitnessValue::Boolean(true);
        let states = four_states();
        let exploration = explore_chain(3);
        let views: Vec<(&str, V<'_>)> = vec![
            (
                "refusal",
                View::Outcome(OutcomeView {
                    document: &refusal,
                    loci: vec![(
                        refusal.diagnostics()[0].locus().cloned(),
                        vec![region.clone()],
                    )],
                }),
            ),
            (
                "proved",
                View::Outcome(OutcomeView {
                    document: &proved,
                    loci: Vec::new(),
                }),
            ),
            (
                "counterexample",
                View::Counterexample(CounterexampleView {
                    function: &function,
                    assignments: vec![AssignmentView {
                        parameter: "p",
                        declaration_order: 0,
                        identity: &node,
                        value: &value,
                    }],
                    certification: Some(qsl_replay::Certification::Trusted),
                }),
            ),
            ("lasso", lasso(&states)),
            ("graph", graph_view(&exploration)),
        ];
        let mut json = Vec::new();
        for (name, view) in &views {
            let before = render_default(view, RenderForm::Json);
            // Render the display forms between the two JSON renders.
            let text = render_default(view, RenderForm::Text);
            assert!(!text.as_str().is_empty());
            if matches!(view, View::Trace(_) | View::StateGraph(_)) {
                assert!(!render_default(view, RenderForm::Dot).as_str().is_empty());
            }
            let after = render_default(view, RenderForm::Json);
            assert_eq!(before, after, "{name}");
            json.push(format!("{name}: {}", before.as_str()));
        }
        let expected = EXPECTED_JSON;
        assert_eq!(json.len(), expected.len());
        for (actual, expected) in json.iter().zip(expected) {
            assert_eq!(actual, expected);
        }
    }

    #[trace("TC-783", "FR-298-AC-2", "FR-298-AC-4")]
    #[test]
    fn certification_reads_the_same_in_json_and_text() {
        let function =
            qsl_replay::QualifiedName::new(vec![quire_exact::Identifier::new("check").unwrap()])
                .unwrap();
        let view: V<'_> = View::Counterexample(CounterexampleView {
            function: &function,
            assignments: Vec::new(),
            certification: Some(qsl_replay::Certification::Uncertified),
        });
        assert!(render_default(&view, RenderForm::Json)
            .as_str()
            .contains(r#""certification":"uncertified""#));
        assert!(render_default(&view, RenderForm::Text)
            .as_str()
            .contains("Certification: uncertified\n"));
    }

    #[trace("TC-783", "FR-298-AC-3", "FR-298-AC-5")]
    #[test]
    fn dot_refuses_package_node_outcome_and_counterexample_views_but_text_and_json_render() {
        let (package, emission) = emitted_spine_package();
        let package_view = inspect_package(
            &package,
            &emission,
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("the package is inspectable")
        .into_value();
        let seven = package
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.declaration()
                    .is_some_and(|name| name.last().is_some_and(|part| part.as_str() == "seven"))
            })
            .expect("fixture declares seven")
            .key();
        let node_view = inspect_node(
            &package,
            &emission,
            &NodeSelector::Node(seven),
            InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("the node is inspectable")
        .into_value();
        let document =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Success);
        let function =
            qsl_replay::QualifiedName::new(vec![quire_exact::Identifier::new("check").unwrap()])
                .unwrap();
        let outcome_view: View<'_> = View::Outcome(OutcomeView {
            document: &document,
            loci: Vec::new(),
        });
        let counterexample_view: View<'_> = View::Counterexample(CounterexampleView {
            function: &function,
            assignments: Vec::new(),
            certification: None,
        });
        for (view, kind) in [
            (&package_view, "package"),
            (&node_view, "node"),
            (&outcome_view, "outcome"),
            (&counterexample_view, "counterexample"),
        ] {
            let result = render_view(
                view,
                RenderForm::Dot,
                &RenderSources::from_sources([]),
                RenderLimits::default(),
                &Cancel::new(),
            );
            let Err(failure) = result else {
                panic!("DOT for {kind} is refused with no bytes");
            };
            assert_eq!(failure.category(), Category::Refusal);
            assert_eq!(failure.category().exit_code(), 20);
            let StageFailure::Refused(cause) = failure else {
                panic!("DOT for {kind} is a typed refusal");
            };
            assert_eq!(
                cause,
                RenderCause::UnsupportedForm {
                    view: kind,
                    form: "dot",
                }
            );
            let code = cause.catalog_code();
            assert_eq!(
                code,
                CatalogCode::new("invalid_quire_context", "unsupported-render-form")
            );
            let fields = cause.catalog_fields().expect("the cause carries payload");
            assert_eq!(fields["view"], kind);
            assert_eq!(fields["form"], "dot");
            for form in [RenderForm::Text, RenderForm::Json] {
                let rendered = render_view(
                    view,
                    form,
                    &RenderSources::from_sources([]),
                    RenderLimits::default(),
                    &Cancel::new(),
                )
                .expect("text and JSON are not refused");
                assert!(!rendered.value().as_str().is_empty());
            }
        }
        // The package JSON is explicit: a formatted `Debug` string appears nowhere.
        let json = render_default(&package_view, RenderForm::Json);
        let parsed: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(parsed["format"], VIEW_FORMAT);
        assert_eq!(parsed["view"]["kind"], "package");
        assert!(parsed["view"]["data"]["model_selections"].is_array());
        assert!(parsed["view"]["data"]["capability_report"].is_array());
        assert!(!json.as_str().contains("RequirementRecord"));
        assert!(!json.as_str().contains("OccurrenceKey"));
        let node_json = render_default(&node_view, RenderForm::Json);
        assert!(!node_json.as_str().contains("OccurrenceKey"));
        assert!(!node_json.as_str().contains("Application"));
    }

    #[trace("TC-783", "FR-298-AC-3", "FR-298-AC-10")]
    #[test]
    fn state_graph_dot_has_one_node_per_state_and_one_edge_per_transition() {
        let exploration = explore_chain(4);
        let view = graph_view(&exploration);
        let first = render_default(&view, RenderForm::Dot);
        let text = first.as_str();
        assert_eq!(text.matches(" -> ").count(), 3);
        assert_eq!(
            text.matches(" [label=").count() - text.matches(" -> ").count(),
            4
        );
        assert!(text.contains("n0 -> n1") && text.contains("n2 -> n3"));
        assert_eq!(first, render_default(&view, RenderForm::Dot));
    }

    #[trace("TC-783", "FR-298-AC-7", "FR-298-AC-10")]
    #[test]
    fn trace_dot_has_exact_lasso_edges_and_marks_the_deciding_position() {
        let states = four_states();
        let output = render_default(&lasso(&states), RenderForm::Dot);
        let text = output.as_str();
        assert_eq!(text.matches(" [label=").count(), 4);
        assert_eq!(text.matches(" -> ").count(), 4);
        for edge in ["n0 -> n1;", "n1 -> n2;", "n2 -> n3;", "n3 -> n2;"] {
            assert!(text.contains(edge), "{text}");
        }
        assert!(text.contains("n1 [label=\"1:") && text.contains("shape=doublecircle"));
        assert_eq!(text.matches("doublecircle").count(), 1);
        assert_eq!(output, render_default(&lasso(&states), RenderForm::Dot));

        let finite = trace_view(
            &states[..3],
            qsl_eval::temporal::TemporalTrace::Finite { len: 3 },
            None,
            None,
        );
        let output = render_default(&finite, RenderForm::Dot);
        assert_eq!(output.as_str().matches(" -> ").count(), 2);
        assert!(!output.as_str().contains("n2 ->"));
    }

    #[trace("TC-783", "FR-298-AC-2", "FR-298-AC-10")]
    #[test]
    fn trace_text_marks_loop_and_deciding_positions_in_order() {
        let states = [snapshot("s0"), snapshot("s1"), snapshot("s2")];
        let view = trace_view(
            &states,
            qsl_eval::temporal::TemporalTrace::Lasso {
                prefix: 1,
                loop_len: 2,
            },
            Some(1),
            Some(2),
        );
        let rendered = render_default(&view, RenderForm::Text);
        let lines: Vec<&str> = rendered.as_str().lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("0: ") && !lines[0].contains('['));
        assert!(lines[1].starts_with("1: ") && lines[1].contains("[loop start]"));
        assert!(lines[2].starts_with("2: ") && lines[2].contains("[deciding]"));
        assert_eq!(rendered, render_default(&view, RenderForm::Text));
    }

    #[trace("TC-783", "FR-298-AC-2", "FR-298-AC-10")]
    #[test]
    fn counterexample_text_uses_parameter_declaration_order_and_bounds_bulk_values() {
        let function = qsl_replay::QualifiedName::new(vec![
            quire_exact::Identifier::new("module").unwrap(),
            quire_exact::Identifier::new("check").unwrap(),
        ])
        .unwrap();
        let node = |name: &str| {
            quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Text(String::from(name))],
                0,
            )
            .expect("fixture node is admissible")
        };
        let (first, second) = (node("first"), node("second"));
        let big = qsl_replay::WitnessValue::Text("x".repeat(1_048_576));
        let small = qsl_replay::WitnessValue::Boolean(true);
        // Supplied in reverse of declaration order.
        let view: V<'_> = View::Counterexample(CounterexampleView {
            function: &function,
            assignments: vec![
                AssignmentView {
                    parameter: "second",
                    declaration_order: 1,
                    identity: &second,
                    value: &small,
                },
                AssignmentView {
                    parameter: "first",
                    declaration_order: 0,
                    identity: &first,
                    value: &big,
                },
            ],
            certification: Some(qsl_replay::Certification::Trusted),
        });
        let rendered = render_default(&view, RenderForm::Text);
        let text = rendered.as_str();
        assert!(text.find("first | [value: ").unwrap() < text.find("second | ").unwrap());
        assert!(text.contains("Certification: trusted"));
        assert!(text.len() < 1_024);
        assert_eq!(rendered, render_default(&view, RenderForm::Text));
    }

    fn two_node_model_check_graph() -> (
        Vec<qsl_analyze::model_check::GraphNode>,
        std::collections::BTreeMap<qsl_analyze::model_check::NodeIndex, qsl_replay::OpenCause>,
        qsl_replay::RunEnd,
    ) {
        let node = |key: &[u8], depth| qsl_analyze::model_check::GraphNode {
            key: qsl_eval::simulation::StateKey::from_canonical_bytes(key.to_vec()),
            depth,
            labels: Vec::new(),
        };
        (
            vec![node(b"{\"a\":1}", 0), node(b"{\"a\":2}", 1)],
            std::collections::BTreeMap::new(),
            qsl_replay::RunEnd::Stopped {
                cause: qsl_replay::IncompleteCause::ResourceExhausted,
                limit: Some(qsl_replay::ReachedLimit {
                    limit: qsl_replay::ModelCheckLimit::MaxStates,
                    value: 2,
                }),
            },
        )
    }

    // A `NodeIndex` is only minted by the explorer, so these graphs carry
    // nodes and a stopped end but no edges or open frontier.
    #[trace("TC-783", "FR-298-AC-3", "FR-298-AC-10")]
    #[test]
    fn model_check_graph_json_and_dot_list_each_retained_node() {
        let (nodes, open, end) = two_node_model_check_graph();
        let view: V<'_> = View::ModelCheckStateGraph(ModelCheckStateGraphView {
            states: nodes.iter().collect(),
            transitions: Vec::new(),
            open: &open,
            end: &end,
            partial: true,
        });
        let json = render_default(&view, RenderForm::Json);
        let parsed: serde_json::Value = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(parsed["view"]["kind"], "model_check_state_graph");
        let data = &parsed["view"]["data"];
        assert_eq!(data["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(data["nodes"][1]["index"], 1);
        assert_eq!(data["nodes"][1]["depth"], 1);
        assert_eq!(data["nodes"][0]["key_hex"], hex_bytes(b"{\"a\":1}"));
        assert_eq!(data["edges"].as_array().unwrap().len(), 0);
        assert_eq!(data["end"]["kind"], "stopped");
        assert_eq!(data["end"]["limit"]["value"], 2);
        assert_eq!(data["partial"], true);
        assert_eq!(json, render_default(&view, RenderForm::Json));

        let dot = render_default(&view, RenderForm::Dot);
        let text = dot.as_str();
        assert!(text.starts_with("digraph view {\n") && text.ends_with("}\n"));
        assert_eq!(text.matches(" [label=").count(), 2);
        assert!(text.contains("  n0 [label=") && text.contains("  n1 [label="));
        assert!(!text.contains(" -> "));
        assert_eq!(dot, render_default(&view, RenderForm::Dot));
    }

    #[trace("TC-783", "FR-298-AC-1", "FR-298-AC-6")]
    #[test]
    fn outcome_view_text_takes_its_region_from_the_aligned_locus_entry() {
        let input = source_under("ref-inv", "src/inv.qsl", "ab\ncd\n");
        let region = input.region(Span { start: 4, end: 5 }).unwrap();
        // The diagnostic's own locus is an occurrence; only the locus entry
        // carries the resolved region.
        let locus = qsl_replay::OutcomeLocus::Occurrence {
            occurrence: String::from("occ"),
        };
        let document =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Refusal)
                .with_diagnostics(vec![qsl_replay::OutcomeDiagnostic::new(
                    None,
                    "ill_typed",
                    Some(locus.clone()),
                    "bad",
                )]);
        let sources = RenderSources::from_sources([input]);
        let aligned: V<'_> = View::Outcome(OutcomeView {
            document: &document,
            loci: vec![(Some(locus.clone()), vec![region])],
        });
        let rendered = render_view(
            &aligned,
            RenderForm::Text,
            &sources,
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("an aligned outcome view renders");
        assert_eq!(
            rendered.value().as_str(),
            "src/inv.qsl:2:2: ill_typed: bad\n"
        );

        let misaligned: V<'_> = View::Outcome(OutcomeView {
            document: &document,
            loci: Vec::new(),
        });
        let failure = render_view(
            &misaligned,
            RenderForm::Text,
            &sources,
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect_err("loci that do not align with the diagnostics are a fault");
        assert!(matches!(failure, StageFailure::Fault(_)));
    }

    #[trace("TC-783", "FR-298-AC-2")]
    #[test]
    fn proved_item_text_shows_its_certification_beside_the_verdict_even_with_diagnostics() {
        let text = text_of(&proved_document(), &RenderSources::from_sources([]));
        assert!(text.contains("Item 0: proved trusted\n"), "{text}");
        assert!(text.contains("Item 1: proved certified\n"), "{text}");
        assert!(text.contains("Item 2: proved uncertified\n"), "{text}");
        assert!(text.contains("Item 3: failed\n"), "{text}");

        let with_diagnostic =
            proved_document().with_diagnostics(vec![qsl_replay::OutcomeDiagnostic::new(
                None,
                "ill_typed",
                None,
                "bad",
            )]);
        let text = text_of(&with_diagnostic, &RenderSources::from_sources([]));
        assert!(text.starts_with("<no-source>: ill_typed: bad\n"), "{text}");
        assert!(text.contains("Item 0: proved trusted\n"), "{text}");
    }

    #[trace("TC-783", "FR-298-AC-8")]
    #[test]
    fn output_bytes_bound_is_inclusive_and_refusal_returns_no_bytes_after_the_last_fitting_unit() {
        let states = four_states();
        let view = lasso(&states);
        let full = render_default(&view, RenderForm::Dot);
        let sources = RenderSources::from_sources([]);
        let exact = render_view(
            &view,
            RenderForm::Dot,
            &sources,
            RenderLimits {
                output_bytes: full.bytes().len(),
            },
            &Cancel::new(),
        )
        .expect("the exact bound admits every byte")
        .into_value();
        assert_eq!(exact, full);

        let total_units = full.as_str().lines().count();
        let mut charged = 0;
        let refusal = render_view_observed(
            &view,
            RenderForm::Dot,
            &sources,
            RenderLimits {
                output_bytes: full.bytes().len() - 1,
            },
            &Cancel::new(),
            &mut |charges| charged = charges,
        );
        let Err(failure) = refusal else {
            panic!("one byte short refuses with no bytes");
        };
        assert_eq!(failure.category(), Category::Incomplete);
        assert_eq!(failure.category().exit_code(), 22);
        let StageFailure::Limit(limit) = failure else {
            panic!("the refusal is a limit");
        };
        assert_eq!(limit.kind(), LimitKind::InputBytes);
        assert_eq!(
            limit.catalog_code(),
            CatalogCode::new("stage_limit_exceeded", "input-bytes-exceeded")
        );
        assert_eq!(limit.setting(), Setting::RenderOutputBytes);
        assert_eq!(limit.setting().name(), "render.output_bytes");
        assert_eq!(limit.configured_bound(), full.bytes().len() as u64 - 1);
        assert_eq!(limit.actual(), full.bytes().len() as u128);
        // Every unit but the refused last one was charged; none after it.
        assert_eq!(charged, total_units - 1);
    }

    #[trace("TC-783", "FR-298-AC-9")]
    #[test]
    fn cancelling_from_a_second_thread_stops_render_view_at_its_next_charge() {
        let exploration = explore_chain(100_000);
        let view = graph_view(&exploration);
        let cancel = Cancel::new();
        let (cancel_tx, cancel_rx) = std::sync::mpsc::channel::<()>();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
        let mut observed = 0;
        let mut after_cancel = 0;
        let canceller = &cancel;
        let result = std::thread::scope(|scope| {
            scope.spawn(move || {
                cancel_rx
                    .recv()
                    .expect("the observer asks for cancellation");
                canceller.cancel(CancelCause::Deadline);
                done_tx.send(()).expect("the observer waits for it");
            });
            render_view_observed(
                &view,
                RenderForm::Dot,
                &RenderSources::from_sources([]),
                RenderLimits::default(),
                &cancel,
                &mut |charges| {
                    observed = charges;
                    if cancel.cause().is_some() {
                        after_cancel += 1;
                    }
                    if charges == 1_000 {
                        cancel_tx.send(()).expect("the canceller is waiting");
                        done_rx.recv().expect("the canceller finished");
                    }
                },
            )
        });
        let Err(failure) = result else {
            panic!("a cancelled render returns no bytes");
        };
        assert!(matches!(
            failure,
            StageFailure::Cancelled(CancelCause::Deadline)
        ));
        assert_eq!(failure.category().exit_code(), 22);
        assert_eq!(observed, 1_000);
        assert!(after_cancel <= 1);
    }

    #[trace("TC-783", "FR-298-AC-9")]
    #[test]
    fn a_handle_cancelled_before_render_returns_requested_before_the_first_charge() {
        let states = four_states();
        let exploration = explore_chain(3);
        let views = [lasso(&states), graph_view(&exploration)];
        for view in &views {
            for form in [RenderForm::Dot, RenderForm::Text, RenderForm::Json] {
                let cancel = Cancel::new();
                cancel.cancel(CancelCause::Requested);
                let mut charges = 0;
                let result = render_view_observed(
                    view,
                    form,
                    &RenderSources::from_sources([]),
                    RenderLimits::default(),
                    &cancel,
                    &mut |_| charges += 1,
                );
                assert!(matches!(
                    result,
                    Err(StageFailure::Cancelled(CancelCause::Requested))
                ));
                assert_eq!(charges, 0);
            }
        }
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        let document =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Success);
        for form in [RenderForm::Json, RenderForm::Text] {
            assert!(matches!(
                render_outcome(
                    &document,
                    form,
                    &RenderSources::from_sources([]),
                    RenderLimits::default(),
                    &cancel,
                ),
                Err(StageFailure::Cancelled(CancelCause::Requested))
            ));
        }
    }
}
