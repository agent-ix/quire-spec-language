// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-298: deterministic renderings of typed inspect values.

use alloc::{format, string::String, vec::Vec};
use core::fmt;
use qsl_foundation::{
    diagnostic::{LimitExceeded, StageFailure, Staged},
    setting::Setting,
    source::{provenance::SourceRegion, Source},
};
use qsl_replay::{OutcomeDocument, RenderLimits};
use quire_exact::Cancel;
use quire_semantic_value::catalog::{CatalogCode, CatalogCoded};
use serde::Serialize;

use crate::{
    CounterexampleView, ModelCheckStateGraphView, OutcomeView, StateGraphView, TraceView, View,
};

const VIEW_FORMAT: &str = "quire.inspect-view/v1";
const LARGE_VALUE_BYTES: usize = 4096;

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

/// Admitted source documents keyed by their own byte digest.
///
/// The source collection can be empty:
///
/// ```
/// let sources = qsl_inspect::RenderSources::from_sources(Vec::new());
/// let _ = sources;
/// ```
///
/// Callers cannot choose a digest key:
///
/// ```compile_fail
/// let sources = qsl_inspect::RenderSources { sources: Vec::new() };
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
    /// source for each digest in input order.
    pub fn from_sources(sources: impl IntoIterator<Item = Source>) -> Self {
        let mut retained = Vec::new();
        for source in sources {
            if !retained
                .iter()
                .any(|item: &Source| item.reference().digest() == source.reference().digest())
            {
                retained.push(source);
            }
        }
        Self { sources: retained }
    }

    fn get(&self, region: &SourceRegion) -> Option<&Source> {
        self.sources
            .iter()
            .find(|source| source.reference().digest() == region.source().digest())
    }
}

/// Complete rendered bytes. Failed renders return no partial value.
#[derive(Clone, Eq, PartialEq)]
pub struct Rendered {
    bytes: Vec<u8>,
}

impl fmt::Debug for Rendered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rendered")
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl Rendered {
    /// The complete UTF-8 output bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The rendered UTF-8 text.
    pub fn as_str(&self) -> &str {
        // All render forms are constructed from UTF-8 strings or JSON bytes.
        core::str::from_utf8(&self.bytes).expect("render output is UTF-8")
    }
}

/// Render an outcome document as its authoritative FR-286 JSON or text.
pub fn render_outcome(
    document: &OutcomeDocument,
    form: RenderForm,
    sources: &RenderSources,
    limits: RenderLimits,
    cancel: &Cancel,
) -> Result<Staged<Rendered>, StageFailure<RenderCause>> {
    let bytes = match form {
        RenderForm::Json => document.to_bytes().map_err(|_| {
            StageFailure::Fault(quire_semantic_value::catalog::InternalFault::new(
                "P",
                "outcome JSON serialization failed",
            ))
        })?,
        RenderForm::Text => render_diagnostic_lines(document, None, sources),
        RenderForm::Dot => {
            return Err(StageFailure::Refused(RenderCause::UnsupportedForm {
                view: "outcome",
                form: form.as_str(),
            }))
        }
    };
    finish_output(bytes, form, limits, cancel)
}

/// Render a typed inspection view without parsing or feeding back rendered data.
pub fn render_view<S, T, F>(
    view: &View<'_, S, T, F>,
    form: RenderForm,
    sources: &RenderSources,
    limits: RenderLimits,
    cancel: &Cancel,
) -> Result<Staged<Rendered>, StageFailure<RenderCause>>
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    if form == RenderForm::Dot
        && !matches!(
            view,
            View::Trace(_) | View::StateGraph(_) | View::ModelCheckStateGraph(_)
        )
    {
        return Err(StageFailure::Refused(RenderCause::UnsupportedForm {
            view: view_kind(view),
            form: form.as_str(),
        }));
    }
    let bytes = match form {
        RenderForm::Json => view_json(view).map_err(|_| {
            StageFailure::Fault(quire_semantic_value::catalog::InternalFault::new(
                "P",
                "typed view JSON serialization failed",
            ))
        })?,
        RenderForm::Text => render_view_text(view, sources),
        RenderForm::Dot => render_view_dot(view),
    };
    finish_output(bytes, form, limits, cancel)
}

fn finish_output(
    bytes: Vec<u8>,
    form: RenderForm,
    limits: RenderLimits,
    cancel: &Cancel,
) -> Result<Staged<Rendered>, StageFailure<RenderCause>> {
    finish_units(bytes, form, limits, cancel, |_| {})
}

fn finish_units(
    bytes: Vec<u8>,
    form: RenderForm,
    limits: RenderLimits,
    cancel: &Cancel,
    mut charged: impl FnMut(usize),
) -> Result<Staged<Rendered>, StageFailure<RenderCause>> {
    let bound = u64::try_from(limits.output_bytes).unwrap_or(u64::MAX);
    let mut output = Vec::new();
    let units: Vec<&[u8]> = if form == RenderForm::Json {
        vec![bytes.as_slice()]
    } else {
        bytes.split_inclusive(|byte| *byte == b'\n').collect()
    };
    let mut used = 0_u128;
    for (index, unit) in units.iter().enumerate() {
        if let Some(cause) = cancel.cause() {
            return Err(StageFailure::Cancelled(cause));
        }
        let attempted = used + unit.len() as u128;
        if attempted > u128::from(bound) {
            return Err(StageFailure::Limit(LimitExceeded::new(
                Setting::RenderOutputBytes,
                bound,
                attempted,
            )));
        }
        used = attempted;
        charged(index + 1);
        output.extend_from_slice(unit);
    }
    Ok(Staged::new(Rendered { bytes: output }))
}

fn view_kind<S, T, F>(view: &View<'_, S, T, F>) -> &'static str {
    match view {
        View::Package(_) => "package",
        View::Node(_) => "node",
        View::Outcome(_) => "outcome",
        View::Counterexample(_) => "counterexample",
        View::Trace(_) => "trace",
        View::StateGraph(_) | View::ModelCheckStateGraph(_) => "state-graph",
    }
}

fn render_diagnostic_lines(
    document: &OutcomeDocument,
    view: Option<&OutcomeView<'_>>,
    sources: &RenderSources,
) -> Vec<u8> {
    let mut output = String::new();
    if document.diagnostics().is_empty() {
        output.push_str(&format!("Outcome: {}\n", document.category().as_str()));
        for item in document.items() {
            output.push_str(&format!("Item: {}\n", format!("{item:?}").to_lowercase()));
        }
        return output.into_bytes();
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
                    region.source().digest().hex(),
                    region.start(),
                    region.end()
                ),
            },
        );
        output.push_str(&format!(
            "{prefix}: {}: {}\n",
            diagnostic.code(),
            diagnostic.message()
        ));
    }
    output.into_bytes()
}

#[derive(Serialize)]
struct JsonView<'a> {
    format: &'static str,
    view: JsonViewBody<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum JsonViewBody<'a> {
    Package(PackageJson),
    Node(NodeJson),
    Outcome(OutcomeJson<'a>),
    Counterexample(CounterexampleJson<'a>),
    Trace(TraceJson),
    StateGraph(StateGraphJson),
}

#[derive(Serialize)]
struct PackageJson {
    package_id: String,
    dependencies: Vec<String>,
    model_selections: Vec<String>,
    exports: Vec<NodeSummary>,
    requirements: Vec<String>,
    capability_report: Vec<String>,
}

#[derive(Serialize)]
struct NodeJson {
    node: NodeSummary,
    occurrences: Vec<OccurrenceJson>,
    requirements: Vec<String>,
}

#[derive(Serialize)]
struct NodeSummary {
    key: String,
    tag: String,
    semantic_form: &'static str,
    semantic_type: Option<String>,
    declaration: Option<Vec<String>>,
    body: String,
    owner: String,
}

#[derive(Serialize)]
struct OccurrenceJson {
    key: String,
    regions: Vec<RegionJson>,
}

#[derive(Serialize)]
struct RegionJson {
    source_digest: String,
    start: u64,
    end: u64,
}

#[derive(Serialize)]
struct OutcomeJson<'a> {
    document: &'a OutcomeDocument,
    package_id: Option<String>,
    loci: Vec<LocusJson>,
}

#[derive(Serialize)]
struct LocusJson {
    locus: Option<qsl_replay::OutcomeLocus>,
    regions: Vec<RegionJson>,
}

#[derive(Serialize)]
struct CounterexampleJson<'a> {
    function: String,
    assignments: Vec<AssignmentJson<'a>>,
    certification: Option<String>,
}

#[derive(Serialize)]
struct AssignmentJson<'a> {
    parameter: &'a str,
    declaration_order: usize,
    identity: String,
    value_text: String,
}

#[derive(Serialize)]
struct TraceJson {
    shape: String,
    positions: Vec<TracePositionJson>,
    loop_start: Option<usize>,
    deciding_position: Option<usize>,
}

#[derive(Serialize)]
struct TracePositionJson {
    position: usize,
    state: DocumentJson,
}

#[derive(Serialize)]
struct DocumentJson {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
    digest: String,
}

#[derive(Serialize)]
struct StateGraphJson {
    graph_kind: &'static str,
    states: Vec<String>,
    transitions: Vec<String>,
    completion: String,
    partial: bool,
}

fn view_json<S, T, F>(view: &View<'_, S, T, F>) -> Result<Vec<u8>, serde_json::Error>
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    let body = match view {
        View::Package(package) => JsonViewBody::Package(PackageJson {
            package_id: package.package_id.hex(),
            dependencies: package
                .dependencies
                .iter()
                .map(|(id, _)| id.hex())
                .collect(),
            model_selections: package
                .model_selections
                .iter()
                .map(|entry| format!("{entry:?}"))
                .collect(),
            exports: package
                .exports
                .iter()
                .map(|node| node_summary(node))
                .collect(),
            requirements: package
                .requirements
                .iter()
                .map(|(key, record)| format!("{key:?}: {record:?}"))
                .collect(),
            capability_report: package
                .capability_report
                .iter()
                .map(|row| format!("{row:?}"))
                .collect(),
        }),
        View::Node(node) => JsonViewBody::Node(NodeJson {
            node: node_summary(node.node),
            occurrences: node
                .occurrences
                .iter()
                .map(|occurrence| OccurrenceJson {
                    key: format!("{:?}", occurrence.key),
                    regions: occurrence.regions.iter().map(region_json).collect(),
                })
                .collect(),
            requirements: node
                .requirements
                .iter()
                .map(|(key, record)| format!("{key:?}: {record:?}"))
                .collect(),
        }),
        View::Outcome(outcome) => JsonViewBody::Outcome(OutcomeJson {
            document: outcome.document,
            package_id: outcome.package_id.map(|id| id.hex()),
            loci: outcome
                .loci
                .iter()
                .map(|(locus, regions)| LocusJson {
                    locus: locus.clone(),
                    regions: regions.iter().map(region_json).collect(),
                })
                .collect(),
        }),
        View::Counterexample(counterexample) => {
            JsonViewBody::Counterexample(counterexample_json(counterexample))
        }
        View::Trace(trace) => JsonViewBody::Trace(trace_json(trace)),
        View::StateGraph(graph) => JsonViewBody::StateGraph(StateGraphJson {
            graph_kind: "simulation",
            states: graph
                .states
                .iter()
                .map(|state| format!("{state:?}"))
                .collect(),
            transitions: graph
                .transitions
                .iter()
                .map(|edge| format!("{edge:?}"))
                .collect(),
            completion: format!("{:?}", graph.outcome),
            partial: graph.partial,
        }),
        View::ModelCheckStateGraph(graph) => {
            JsonViewBody::StateGraph(model_check_graph_json(graph))
        }
    };
    serde_json::to_vec(&JsonView {
        format: VIEW_FORMAT,
        view: body,
    })
}

fn node_summary(node: &qsl_semantics::check::SemanticNode) -> NodeSummary {
    NodeSummary {
        key: node.key().to_string(),
        tag: format!("{:?}", node.node_tag()),
        semantic_form: node.semantic_form(),
        semantic_type: node.semantic_type().map(|key| key.to_string()),
        declaration: node.declaration().map(|path| {
            path.iter()
                .map(|part| String::from(part.as_str()))
                .collect()
        }),
        body: format!("{:?}", node.body()),
        owner: node
            .owner()
            .map_or_else(|| String::from("<none>"), |owner| format!("{owner:?}")),
    }
}

fn region_json(region: &SourceRegion) -> RegionJson {
    RegionJson {
        source_digest: region.source().digest().hex(),
        start: region.start(),
        end: region.end(),
    }
}

fn counterexample_json<'a>(view: &'a CounterexampleView<'_>) -> CounterexampleJson<'a> {
    CounterexampleJson {
        function: view.function.to_string(),
        assignments: view
            .assignments
            .iter()
            .map(|assignment| AssignmentJson {
                parameter: assignment.parameter,
                declaration_order: assignment.declaration_order,
                identity: assignment.identity.to_string(),
                value_text: assignment
                    .value
                    .to_value_text()
                    .unwrap_or_else(|_| String::from("<unavailable>")),
            })
            .collect(),
        certification: view
            .certification
            .map(|certification| format!("{certification:?}")),
    }
}

fn trace_json(view: &TraceView<'_>) -> TraceJson {
    TraceJson {
        shape: format!("{:?}", view.shape),
        positions: view
            .positions
            .iter()
            .map(|position| TracePositionJson {
                position: position.position,
                state: DocumentJson {
                    authority: position.state.authority.clone(),
                    identity: position.state.identity.clone(),
                    revision_namespace: position.state.revision_namespace.clone(),
                    revision: position.state.revision.clone(),
                    digest: hex_bytes(&position.state.digest),
                },
            })
            .collect(),
        loop_start: view.loop_start,
        deciding_position: view.deciding_position,
    }
}

fn model_check_graph_json(view: &ModelCheckStateGraphView<'_>) -> StateGraphJson {
    StateGraphJson {
        graph_kind: "model_check",
        states: view.states.iter().map(|node| format!("{node:?}")).collect(),
        transitions: view
            .transitions
            .iter()
            .map(|edge| format!("{edge:?}"))
            .collect(),
        completion: format!("{:?}; open={:?}", view.end, view.open),
        partial: view.partial,
    }
}

fn render_view_text<S, T, F>(view: &View<'_, S, T, F>, sources: &RenderSources) -> Vec<u8>
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    let text = match view {
        View::Package(package) => {
            let mut text = format!("Package {}\n", package.package_id.hex());
            for (identity, _) in &package.dependencies {
                text.push_str(&format!("Dependency: {}\n", identity.hex()));
            }
            for selection in package.model_selections {
                text.push_str(&format!("Model selection: {selection:?}\n"));
            }
            for node in &package.exports {
                text.push_str(&format!(
                    "Export: {} {} {:?}\n",
                    node.key(),
                    node.semantic_form(),
                    node.declaration()
                ));
            }
            for (occurrence, record) in &package.requirements {
                text.push_str(&format!("Requirement {occurrence:?}: {record:?}\n"));
            }
            for capability in package.capability_report {
                text.push_str(&format!("Capability: {capability:?}\n"));
            }
            text
        }
        View::Node(node) => {
            let mut text = format!(
                "Node {} ({:?}, {})\nSemantic type: {:?}\nDeclaration: {:?}\n",
                node.node.key(),
                node.node.node_tag(),
                node.node.semantic_form(),
                node.node.semantic_type(),
                node.node.declaration()
            );
            for occurrence in &node.occurrences {
                text.push_str(&format!("Occurrence: {:?}\n", occurrence.key));
                for region in occurrence.regions {
                    text.push_str(&format!(
                        "  Region: {}:{}-{}\n",
                        region.source().digest().hex(),
                        region.start(),
                        region.end()
                    ));
                }
            }
            for (occurrence, record) in &node.requirements {
                text.push_str(&format!("Requirement {occurrence:?}: {record:?}\n"));
            }
            text
        }
        View::Outcome(outcome) => String::from_utf8(render_diagnostic_lines(
            outcome.document,
            Some(outcome),
            sources,
        ))
        .expect("diagnostic rendering constructs UTF-8"),
        View::Counterexample(counterexample) => render_counterexample_text(counterexample),
        View::Trace(trace) => render_trace_text(trace),
        View::StateGraph(graph) => {
            let mut text = format!("State graph (partial: {})\n", graph.partial);
            for state in &graph.states {
                text.push_str(&format!("State: {state:?}\n"));
            }
            for transition in &graph.transitions {
                text.push_str(&format!("Transition: {transition:?}\n"));
            }
            text.push_str(&format!("Completion: {:?}\n", graph.outcome));
            text
        }
        View::ModelCheckStateGraph(graph) => {
            let mut text = format!("State graph (partial: {})\n", graph.partial);
            for state in &graph.states {
                text.push_str(&format!("State: {state:?}\n"));
            }
            for transition in &graph.transitions {
                text.push_str(&format!("Transition: {transition:?}\n"));
            }
            text.push_str(&format!("Open frontier: {:?}\n", graph.open));
            text.push_str(&format!("Completion: {:?}\n", graph.end));
            text
        }
    };
    text.into_bytes()
}

fn render_counterexample_text(view: &CounterexampleView<'_>) -> String {
    let mut output = format!("Counterexample for {}\nParameter | Value\n", view.function);
    let mut assignments = view.assignments.iter().collect::<Vec<_>>();
    assignments.sort_by_key(|assignment| assignment.declaration_order);
    for assignment in assignments {
        let value = assignment
            .value
            .to_value_text()
            .unwrap_or_else(|_| String::from("<unavailable>"));
        let value = if value.len() > LARGE_VALUE_BYTES {
            format!("[value: {} bytes]", assignment.value.value_text_len())
        } else {
            value
        };
        output.push_str(&format!("{} | {}\n", assignment.parameter, value));
    }
    if let Some(certification) = view.certification {
        output.push_str(&format!("Certification: {certification:?}\n"));
    }
    output
}

fn render_trace_text(view: &TraceView<'_>) -> String {
    let mut output = String::new();
    for position in &view.positions {
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
        output.push_str(&format!(
            "{}: {:?}{suffix}\n",
            position.position, position.state
        ));
    }
    output
}

fn render_view_dot<S, T, F>(view: &View<'_, S, T, F>) -> Vec<u8>
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    let statements = match view {
        View::Trace(trace) => trace_dot_statements(trace),
        View::StateGraph(graph) => retained_graph_dot_statements(graph),
        View::ModelCheckStateGraph(graph) => model_check_dot_statements(graph),
        _ => Vec::new(),
    };
    let mut output = String::from("digraph view {\n");
    for statement in statements {
        output.push_str(&statement);
        output.push('\n');
    }
    output.push_str("}\n");
    output.into_bytes()
}

fn trace_dot_statements(view: &TraceView<'_>) -> Vec<String> {
    let mut statements = Vec::new();
    for position in &view.positions {
        let label = format!("{}: {:?}", position.position, position.state);
        let shape = if view.deciding_position == Some(position.position) {
            ", shape=doublecircle"
        } else {
            ""
        };
        statements.push(format!(
            "  n{} [label=\"{}\"{}];",
            position.position,
            dot_escape(&label),
            shape
        ));
        if position.position + 1 < view.positions.len() {
            statements.push(format!(
                "  n{} -> n{};",
                position.position,
                position.position + 1
            ));
        }
    }
    if let (Some(last), Some(loop_start)) = (view.positions.last(), view.loop_start) {
        statements.push(format!("  n{} -> n{};", last.position, loop_start));
    }
    statements
}

fn retained_graph_dot_statements<S, T, F>(view: &StateGraphView<'_, S, T, F>) -> Vec<String>
where
    S: fmt::Debug,
    T: fmt::Debug,
    F: fmt::Debug,
{
    let mut statements = Vec::new();
    for (index, state) in view.states.iter().enumerate() {
        statements.push(format!(
            "  n{index} [label=\"{}\"];",
            dot_escape(&format!("{state:?}"))
        ));
    }
    for edge in &view.transitions {
        statements.push(format!(
            "  n{} -> n{} [label=\"{}\"];",
            edge.source.index(),
            edge.target.index(),
            dot_escape(&format!("{:?}", edge.transition))
        ));
    }
    statements
}

fn model_check_dot_statements(view: &ModelCheckStateGraphView<'_>) -> Vec<String> {
    let mut statements = Vec::new();
    for (index, state) in view.states.iter().enumerate() {
        statements.push(format!(
            "  n{index} [label=\"{}\"];",
            dot_escape(&format!("{:?}", state.key))
        ));
    }
    for edge in &view.transitions {
        statements.push(format!(
            "  n{} -> n{} [label=\"{}\"];",
            edge.source.index(),
            edge.target.index(),
            dot_escape(&format!("{:?}", edge.transition))
        ));
    }
    statements
}

fn dot_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX[(byte >> 4) as usize]));
        text.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_foundation::{source::SourceIdentity, Span};
    use quire_exact::CancelCause;
    use quire_semantic_value::catalog::Category;

    fn source(path: &str, text: &str) -> Source {
        Source::read_typed(
            SourceIdentity::new("tests", path, "test", "1"),
            path,
            text.as_bytes(),
            text.len(),
        )
        .unwrap_or_else(|_| panic!("fixture source is admitted"))
    }

    fn snapshot(identity: &str, digest: u8) -> qsl_semantics::model::observation::DocumentRef {
        qsl_semantics::model::observation::DocumentRef {
            authority: String::from("tests"),
            identity: String::from(identity),
            revision_namespace: String::from("test"),
            revision: String::from("1"),
            digest: [digest; 32],
        }
    }

    #[trace("TC-783", "FR-298-AC-1", "FR-298-AC-6")]
    #[test]
    fn diagnostic_text_resolves_path_and_scalar_column_from_its_admitted_source() {
        let input = source("src/inv.qsl", "a\néx\n");
        let region = input
            .region(Span { start: 4, end: 5 })
            .expect("region starts after multibyte scalar");
        let document =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Violation)
                .with_diagnostics(vec![qsl_replay::OutcomeDiagnostic::new(
                    None,
                    "ill_typed",
                    Some(qsl_replay::OutcomeLocus::Region {
                        source: region.source().clone(),
                        start: region.start(),
                        end: region.end(),
                    }),
                    "bad value",
                )]);
        let duplicate_bytes = source("src/other.qsl", "a\néx\n");
        let sources = RenderSources::from_sources([input.clone(), duplicate_bytes]);
        let rendered = render_outcome(
            &document,
            RenderForm::Text,
            &sources,
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("diagnostic renders");
        assert_eq!(
            rendered.value().as_str(),
            "src/inv.qsl:2:2: ill_typed: bad value\n"
        );

        let out_of_source =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Refusal)
                .with_diagnostics(vec![qsl_replay::OutcomeDiagnostic::new(
                    None,
                    "ill_typed",
                    Some(qsl_replay::OutcomeLocus::Region {
                        source: region.source().clone(),
                        start: region.start(),
                        end: 99,
                    }),
                    "bad range",
                )]);
        let fallback = render_outcome(
            &out_of_source,
            RenderForm::Text,
            &sources,
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("an unresolved source range uses its bounded identity");
        assert!(fallback.value().as_str().contains(&format!(
            "{}:4-99: ill_typed: bad range",
            region.source().digest().hex()
        )));
        assert!(!fallback.value().as_str().contains("src/inv.qsl"));
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

    #[trace("TC-783", "FR-298-AC-5")]
    #[test]
    fn dot_for_outcome_view_refuses_with_the_catalogued_view_and_form() {
        let document =
            OutcomeDocument::new(qsl_replay::Operation::Analyze, None, Category::Success);
        let view = View::<(), (), ()>::Outcome(OutcomeView {
            document: &document,
            package_id: None,
            loci: Vec::new(),
        });
        let result = render_view(
            &view,
            RenderForm::Dot,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        );
        let Err(StageFailure::Refused(cause)) = result else {
            panic!("DOT for an outcome is refused");
        };
        assert_eq!(
            cause,
            RenderCause::UnsupportedForm {
                view: "outcome",
                form: "dot",
            }
        );
        assert_eq!(
            cause.catalog_code(),
            CatalogCode::new("invalid_quire_context", "unsupported-render-form")
        );
        assert_eq!(cause.catalog_fields().unwrap()["view"], "outcome");
        assert_eq!(cause.catalog_fields().unwrap()["form"], "dot");
    }

    #[trace("TC-783", "FR-298-AC-7")]
    #[test]
    fn trace_dot_has_exact_lasso_edges_and_marks_the_deciding_position() {
        let states = [
            snapshot("s0", 0),
            snapshot("s1", 1),
            snapshot("s2", 2),
            snapshot("s3", 3),
        ];
        let view = View::Trace(TraceView {
            shape: qsl_eval::temporal::TemporalTrace::Lasso {
                prefix: 2,
                loop_len: 2,
            },
            positions: states
                .iter()
                .enumerate()
                .map(|(position, state)| crate::TracePositionView { position, state })
                .collect(),
            loop_start: Some(2),
            deciding_position: Some(1),
        });
        let output = render_view::<(), (), ()>(
            &view,
            RenderForm::Dot,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("trace DOT renders");
        let text = output.value().as_str();
        assert_eq!(text.matches(" [label=").count(), 4);
        assert_eq!(text.matches(" -> ").count(), 4);
        assert!(text.contains("n0 -> n1;"));
        assert!(text.contains("n1 -> n2;"));
        assert!(text.contains("n2 -> n3;"));
        assert!(text.contains("n3 -> n2;"));
        assert!(text.contains("n1 [label=\"1:") && text.contains("shape=doublecircle"));

        let finite = View::Trace(TraceView {
            shape: qsl_eval::temporal::TemporalTrace::Finite { len: 3 },
            positions: states[..3]
                .iter()
                .enumerate()
                .map(|(position, state)| crate::TracePositionView { position, state })
                .collect(),
            loop_start: None,
            deciding_position: None,
        });
        let output = render_view::<(), (), ()>(
            &finite,
            RenderForm::Dot,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("finite trace DOT renders");
        assert_eq!(output.value().as_str().matches(" -> ").count(), 2);
        assert!(!output.value().as_str().contains("n2 ->"));
    }

    #[trace("TC-783", "FR-298-AC-2", "FR-298-AC-10")]
    #[test]
    fn trace_text_marks_loop_and_deciding_positions_in_order() {
        let states = [snapshot("s0", 0), snapshot("s1", 1), snapshot("s2", 2)];
        let view = View::Trace(TraceView {
            shape: qsl_eval::temporal::TemporalTrace::Lasso {
                prefix: 1,
                loop_len: 2,
            },
            positions: states
                .iter()
                .enumerate()
                .map(|(position, state)| crate::TracePositionView { position, state })
                .collect(),
            loop_start: Some(1),
            deciding_position: Some(2),
        });
        let rendered = render_view::<(), (), ()>(
            &view,
            RenderForm::Text,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("trace text renders");
        let text = rendered.value().as_str();
        assert!(text.find("0: ").unwrap() < text.find("1: ").unwrap());
        assert!(text.find("1: ").unwrap() < text.find("2: ").unwrap());
        assert!(text.contains("1: ") && text.contains("[loop start]"));
        assert!(text.contains("2: ") && text.contains("[deciding]"));
    }

    #[trace("TC-783", "FR-298-AC-2", "FR-298-AC-10")]
    #[test]
    fn counterexample_text_uses_parameter_declaration_order_and_bounds_bulk_values() {
        let function = qsl_replay::QualifiedName::new(vec![
            quire_exact::Identifier::new("module").unwrap(),
            quire_exact::Identifier::new("check").unwrap(),
        ])
        .unwrap();
        let first = qsl_foundation::digest::WireNodeId::from_digest([1; 32]);
        let second = qsl_foundation::digest::WireNodeId::from_digest([2; 32]);
        let site = qsl_replay::FunctionSite {
            function: qsl_foundation::digest::WireNodeId::from_digest([3; 32]),
            declaration: qsl_foundation::source::provenance::OccurrenceKey::new(
                qsl_foundation::digest::WireNodeId::from_digest([3; 32]),
                quire_exact::Origin::new(quire_exact::Role::new("declaration"), 0),
            ),
            parameters: vec![
                (quire_exact::Identifier::new("first").unwrap(), first),
                (quire_exact::Identifier::new("second").unwrap(), second),
            ],
            value_parity: None,
        };
        let assignments = [
            qsl_replay::CanonicalAssignment {
                parameter: second,
                value: qsl_replay::WitnessValue::Boolean(true),
            },
            qsl_replay::CanonicalAssignment {
                parameter: first,
                value: qsl_replay::WitnessValue::Text("x".repeat(5_000)),
            },
        ];
        let view = crate::inspect_counterexample(
            &function,
            &site,
            &assignments,
            Some(qsl_replay::Certification::Trusted),
            crate::InspectLimits::default(),
            &Cancel::new(),
        )
        .expect("the function signature names both assignments");
        let rendered = render_view(
            view.value(),
            RenderForm::Text,
            &RenderSources::from_sources([]),
            RenderLimits::default(),
            &Cancel::new(),
        )
        .expect("counterexample text renders");
        let text = rendered.value().as_str();
        assert!(text.find("first | [value: ").unwrap() < text.find("second | ").unwrap());
        assert!(text.contains("Certification: Trusted"));
        assert!(!text.contains(&"x".repeat(5_000)));
    }

    #[trace("TC-783", "FR-298-AC-8")]
    #[test]
    fn byte_limit_is_inclusive_and_refusal_returns_no_partial_output() {
        let bytes = b"digraph view {\n  n0 [label=\"x\"];\n}\n".to_vec();
        let exact = finish_units(
            bytes.clone(),
            RenderForm::Dot,
            RenderLimits {
                output_bytes: bytes.len(),
            },
            &Cancel::new(),
            |_| {},
        )
        .expect("exact byte bound admits all bytes");
        assert_eq!(exact.value().bytes(), bytes);
        let refusal = finish_units(
            bytes,
            RenderForm::Dot,
            RenderLimits {
                output_bytes: exact.value().bytes().len() - 1,
            },
            &Cancel::new(),
            |_| {},
        )
        .expect_err("one-short output bound refuses");
        assert!(matches!(
            refusal,
            StageFailure::Limit(limit)
                if limit.setting() == Setting::RenderOutputBytes
                    && u128::from(limit.configured_bound()) + 1 == limit.actual()
        ));
    }

    #[trace("TC-783", "FR-298-AC-9")]
    #[test]
    fn cancellation_after_a_charge_stops_before_the_next_unit() {
        let cancel = Cancel::new();
        let mut observed = 0;
        let result = finish_units(
            (0..2_000).flat_map(|_| b"unit\n".iter().copied()).collect(),
            RenderForm::Text,
            RenderLimits::default(),
            &cancel,
            |charges| {
                observed = charges;
                if charges == 1_000 {
                    cancel.cancel(CancelCause::Deadline);
                }
            },
        );
        assert!(matches!(
            result,
            Err(StageFailure::Cancelled(CancelCause::Deadline))
        ));
        assert_eq!(observed, 1_000);
    }
}
