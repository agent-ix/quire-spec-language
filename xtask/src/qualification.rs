// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-350..FR-353: execute the QSpec complete-V1 corpus against QSL and
//! settle/report capability qualification.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use qsl_foundation::diagnostic::Code;
use qsl_foundation::SettingLimits;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::{Error, Result};
use crate::Outcome;

const USAGE: &str = "usage: cargo xtask qualify --corpus <dir> [--capability <id>]... [--limit <setting>=<value>]... [--text]";
const CORPUS_FILE: &str = "complete-v1.json";
const SCHEMA_FILE: &str = "proposals/conformance-corpus-v1/schema.json";
const QSL_VERSION: &str = "0.2.0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QualificationOption {
    Text,
    Corpus,
    Capability,
    Limit,
}

impl QualificationOption {
    fn parse(value: &str) -> Option<Self> {
        match value.as_bytes() {
            b"--text" => Some(Self::Text),
            b"--corpus" => Some(Self::Corpus),
            b"--capability" => Some(Self::Capability),
            b"--limit" => Some(Self::Limit),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum CorpusVersion {
    #[serde(rename = "quire.conformance-corpus/v1")]
    CompleteV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum InputKind {
    Source,
    Model,
    Input,
    Call,
    ObjectContext,
    Population,
    ReplayRequest,
    Bytes,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ConsumerType {
    Language,
    Runtime,
    Tool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Compile,
    Run,
    Replay,
    Format,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    contract_version: CorpusVersion,
    vectors: Vec<Vector>,
    results: Vec<CorpusResult>,
    coverage: Coverage,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverage {
    primary: Vec<Capability>,
    criteria_rollups: Vec<CriterionRollup>,
    consumer_rollups: Vec<ConsumerRollup>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusResult {
    #[allow(dead_code)]
    subject: Subject,
    #[allow(dead_code)]
    run_id: String,
    vector_id: String,
    #[allow(dead_code)]
    outcome: String,
    #[allow(dead_code)]
    artifacts: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CriterionRollup {
    criterion_id: String,
    capability_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumerRollup {
    consumer_type: ConsumerType,
    capability_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    capability_id: String,
    applicability: CapabilityApplicability,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityApplicability {
    consumer_types: Vec<ConsumerType>,
    semantic_profiles: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Vector {
    id: String,
    capability_id: String,
    #[allow(dead_code)]
    class: String,
    mutation_of: Option<String>,
    subject: Subject,
    applicability: VectorApplicability,
    input: Input,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Subject {
    consumer_type: ConsumerType,
    operation: Operation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorApplicability {
    consumer_types: Vec<ConsumerType>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    files: Vec<InputFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFile {
    kind: InputKind,
    path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
enum Expected {
    Value {
        value: Value,
    },
    Refusal {
        code: String,
        cause: String,
        #[serde(default)]
        details: Map<String, Value>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum VectorOutcome {
    Passed,
    Failed,
    ToolFailure,
    Unsupported,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CapabilityOutcome {
    Passed,
    Failed,
    ToolFailure,
    Unsupported,
    Incomplete,
    Uncovered,
}

#[derive(Debug, Serialize)]
struct VectorReport {
    id: String,
    outcome: VectorOutcome,
    expected: Expected,
    actual: Value,
}

#[derive(Debug, Serialize)]
struct CapabilityReport {
    capability_id: String,
    outcome: CapabilityOutcome,
    deciding_vector_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    vectors: Vec<VectorReport>,
}

#[derive(Debug, Serialize)]
struct Counts {
    passed: usize,
    failed: usize,
    #[serde(rename = "tool-failure")]
    tool_failure: usize,
    unsupported: usize,
    incomplete: usize,
    uncovered: usize,
}

#[derive(Debug, Serialize)]
struct CoverageReport {
    passed: usize,
    in_scope: usize,
    percent: f64,
}

#[derive(Debug, Serialize)]
struct Report {
    qsl_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    qualification: Option<&'static str>,
    scope: Vec<String>,
    verdict: &'static str,
    #[serde(skip)]
    default_scope: bool,
    counts: Counts,
    coverage: CoverageReport,
    capabilities: Vec<CapabilityReport>,
}

/// Run `cargo xtask qualify` after a full preflight. All corpus reads and
/// QSL calls occur sequentially on this thread (FR-350-AC-7).
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<Outcome> {
    let args = parse_args(operands)?;
    let limits = qsl_replay::CallerLimits::from_operands(args.limits.iter().map(String::as_str))
        .map_err(|error| Error::QualificationRefusal(error.to_string()))?;
    let corpus_root = fs::canonicalize(&args.corpus).map_err(|error| {
        Error::QualificationRefusal(format!("{}: {error}", args.corpus.display()))
    })?;
    if !corpus_root.is_dir() {
        return Err(Error::QualificationRefusal(format!(
            "{} is not a corpus directory",
            corpus_root.display()
        )));
    }
    let qspec = qspec_dir(workspace_root)?;
    let schema_path = qspec.join(SCHEMA_FILE);
    let schema_bytes = fs::read(&schema_path).map_err(|error| {
        Error::QualificationRefusal(format!("{}: {error}", schema_path.display()))
    })?;
    let schema: Value = serde_json::from_slice(&schema_bytes).map_err(|error| {
        Error::QualificationRefusal(format!(
            "{}: invalid schema JSON: {error}",
            schema_path.display()
        ))
    })?;
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .map_err(|error| Error::QualificationRefusal(format!("invalid FR-336 schema: {error}")))?;
    let corpus_path = corpus_root.join(CORPUS_FILE);
    let corpus_bytes = fs::read(&corpus_path).map_err(|error| {
        Error::QualificationRefusal(format!("{}: {error}", corpus_path.display()))
    })?;
    let value = parse_unique_json(&corpus_bytes).map_err(|reason| {
        Error::QualificationRefusal(format!("{}: {reason}", corpus_path.display()))
    })?;
    let canonical = serde_json_canonicalizer::to_vec(&value).map_err(|error| {
        Error::QualificationRefusal(format!("{}: canonicalize: {error}", corpus_path.display()))
    })?;
    if canonical != corpus_bytes {
        return Err(Error::QualificationRefusal(format!(
            "{} is not canonical JSON",
            corpus_path.display()
        )));
    }
    if let Err(errors) = schema.validate(&value) {
        let reasons = errors
            .map(|error| format!("{}: {}", error.instance_path, error))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Error::QualificationRefusal(reasons));
    }
    let corpus: Corpus = serde_json::from_value(value).map_err(|error| {
        Error::QualificationRefusal(format!("{}: {error}", corpus_path.display()))
    })?;
    validate_semantics(&corpus)?;

    let profiles = qsl_profiles();
    let all_capabilities = corpus
        .coverage
        .primary
        .iter()
        .map(|cap| (cap.capability_id.as_str(), cap))
        .collect::<BTreeMap<_, _>>();
    let default_scope = args.capabilities.is_empty();
    let mut scope = if default_scope {
        corpus
            .coverage
            .primary
            .iter()
            .filter(|cap| {
                cap.applicability
                    .consumer_types
                    .iter()
                    .any(|kind| qsl_consumer(*kind))
                    && cap
                        .applicability
                        .semantic_profiles
                        .iter()
                        .any(|profile| profiles.contains(profile))
            })
            .collect::<Vec<_>>()
    } else {
        let ids = args.capabilities.iter().cloned().collect::<BTreeSet<_>>();
        ids.into_iter()
            .map(|id| {
                all_capabilities.get(id.as_str()).copied().ok_or_else(|| {
                    Error::QualificationRefusal(format!(
                        "unknown_required_feature/unknown-feature: unknown capability id {id}"
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?
    };
    if scope.is_empty() {
        return Err(Error::QualificationRefusal(
            "empty-scope: no capabilities in the resolved qualification scope".to_owned(),
        ));
    }
    scope.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    let scope_ids = scope
        .iter()
        .map(|cap| cap.capability_id.clone())
        .collect::<Vec<_>>();
    let mut vector_reports = BTreeMap::<String, Vec<VectorReport>>::new();
    let mut vector_outcomes = BTreeMap::<String, Vec<(String, VectorOutcome)>>::new();
    for vector in &corpus.vectors {
        if !scope_ids.iter().any(|id| id == &vector.capability_id)
            || !vector
                .applicability
                .consumer_types
                .contains(&vector.subject.consumer_type)
            || !is_qsl_subject(&vector.subject)
        {
            continue;
        }
        let report = execute_vector(vector, &corpus_root, &limits);
        vector_outcomes
            .entry(vector.capability_id.clone())
            .or_default()
            .push((vector.id.clone(), report.outcome));
        vector_reports
            .entry(vector.capability_id.clone())
            .or_default()
            .push(report);
    }
    let mut capability_reports = Vec::new();
    for cap in &scope {
        let outcomes = vector_outcomes
            .remove(&cap.capability_id)
            .unwrap_or_default();
        let outcome = settle(&outcomes);
        let deciding_vector_ids =
            if outcome == CapabilityOutcome::Passed || outcome == CapabilityOutcome::Uncovered {
                Vec::new()
            } else {
                outcomes
                    .iter()
                    .filter(|(_, vector_outcome)| matches_outcome(outcome, *vector_outcome))
                    .map(|(id, _)| id.clone())
                    .collect()
            };
        let mut details = vector_reports
            .remove(&cap.capability_id)
            .unwrap_or_default();
        details.sort_by(|left, right| left.id.cmp(&right.id));
        details.retain(|detail| detail.outcome != VectorOutcome::Passed);
        capability_reports.push(CapabilityReport {
            capability_id: cap.capability_id.clone(),
            outcome,
            deciding_vector_ids,
            vectors: details,
        });
    }
    capability_reports.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    let counts = count_outcomes(&capability_reports);
    let qualified = counts.passed == scope_ids.len();
    let report = Report {
        qsl_version: QSL_VERSION,
        qualification: default_scope.then_some("complete-v1"),
        scope: scope_ids.clone(),
        verdict: if qualified {
            "qualified"
        } else {
            "not-qualified"
        },
        default_scope,
        coverage: CoverageReport {
            passed: counts.passed,
            in_scope: scope_ids.len(),
            percent: (counts.passed as f64 * 100.0) / scope_ids.len() as f64,
        },
        counts,
        capabilities: capability_reports,
    };
    let stdout = if args.text {
        render_text(&report)
    } else {
        serde_json::to_string(&report).map_err(|error| {
            Error::QualificationRefusal(format!("cannot encode report: {error}"))
        })? + "\n"
    };
    Ok(Outcome {
        stdout,
        code: if qualified { 0 } else { 1 },
    })
}

struct Args {
    corpus: PathBuf,
    capabilities: Vec<String>,
    limits: Vec<String>,
    text: bool,
}

fn parse_args(operands: &[OsString]) -> Result<Args> {
    let mut corpus = None;
    let mut capabilities = Vec::new();
    let mut limits = Vec::new();
    let mut text = false;
    let mut index = 0;
    while index < operands.len() {
        let option = operands[index].to_str().ok_or(Error::Usage(USAGE))?;
        match QualificationOption::parse(option) {
            Some(QualificationOption::Text) if !text => text = true,
            Some(QualificationOption::Corpus) if corpus.is_none() => {
                index += 1;
                let value = operands.get(index).ok_or(Error::Usage(USAGE))?;
                corpus = Some(PathBuf::from(value));
            }
            Some(QualificationOption::Capability) => {
                index += 1;
                capabilities.push(
                    operands
                        .get(index)
                        .and_then(|value| value.to_str())
                        .ok_or(Error::Usage(USAGE))?
                        .to_owned(),
                );
            }
            Some(QualificationOption::Limit) => {
                index += 1;
                limits.push(
                    operands
                        .get(index)
                        .and_then(|value| value.to_str())
                        .ok_or(Error::Usage(USAGE))?
                        .to_owned(),
                );
            }
            _ => return Err(Error::Usage(USAGE)),
        }
        index += 1;
    }
    Ok(Args {
        corpus: corpus.ok_or(Error::Usage(USAGE))?,
        capabilities,
        limits,
        text,
    })
}

fn qspec_dir(_workspace_root: &Path) -> Result<PathBuf> {
    let path = std::env::var_os("QSPEC_DIR")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            Error::QualificationRefusal(
                "QSPEC_DIR must point to an available QSpec checkout; no default is used"
                    .to_owned(),
            )
        })?;
    fs::canonicalize(&path).map_err(|error| {
        Error::QualificationRefusal(format!("QSpec checkout {}: {error}", path.display()))
    })
}

fn validate_semantics(corpus: &Corpus) -> Result<()> {
    if corpus.contract_version != CorpusVersion::CompleteV1 {
        return Err(Error::QualificationRefusal(
            "/contract_version: expected quire.conformance-corpus/v1".to_owned(),
        ));
    }
    let mut capabilities = BTreeSet::new();
    for capability in &corpus.coverage.primary {
        if !capabilities.insert(&capability.capability_id) {
            return Err(Error::QualificationRefusal(format!(
                "duplicate primary capability id {}",
                capability.capability_id
            )));
        }
    }
    let mut ids = BTreeSet::new();
    for vector in &corpus.vectors {
        if !ids.insert(&vector.id) {
            return Err(Error::QualificationRefusal(format!(
                "duplicate vector id {}",
                vector.id
            )));
        }
        if !capabilities.contains(&vector.capability_id) {
            return Err(Error::QualificationRefusal(format!(
                "vector {} references unknown capability {}",
                vector.id, vector.capability_id
            )));
        }
        for file in &vector.input.files {
            let path = Path::new(&file.path);
            if path.is_absolute()
                || path
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(Error::QualificationRefusal(format!(
                    "vector {} has invalid input path {}",
                    vector.id, file.path
                )));
            }
            if !matches!(
                file.kind,
                InputKind::Source | InputKind::Model | InputKind::Input
            ) {
                return Err(Error::QualificationRefusal(format!(
                    "vector {} has unsupported input kind {:?}",
                    vector.id, file.kind
                )));
            }
        }
    }
    for vector in &corpus.vectors {
        if let Some(parent) = &vector.mutation_of {
            if !ids.contains(parent) {
                return Err(Error::QualificationRefusal(format!(
                    "vector {} mutation_of references unknown vector {}",
                    vector.id, parent
                )));
            }
        }
    }
    for result in &corpus.results {
        if !ids.contains(&result.vector_id) {
            return Err(Error::QualificationRefusal(format!(
                "recorded result {} references unknown vector {}",
                result.run_id, result.vector_id
            )));
        }
    }
    for rollup in &corpus.coverage.criteria_rollups {
        for capability_id in &rollup.capability_ids {
            if !capabilities.contains(capability_id) {
                return Err(Error::QualificationRefusal(format!(
                    "criterion rollup {} references unknown capability {}",
                    rollup.criterion_id, capability_id
                )));
            }
        }
    }
    for rollup in &corpus.coverage.consumer_rollups {
        for capability_id in &rollup.capability_ids {
            if !capabilities.contains(capability_id) {
                return Err(Error::QualificationRefusal(format!(
                    "consumer rollup {:?} references unknown capability {}",
                    rollup.consumer_type, capability_id
                )));
            }
        }
    }
    Ok(())
}

fn qsl_profiles() -> BTreeSet<String> {
    let lock = qsl_semantics::value::DefinitionLock::pinned();
    let mut profiles = lock
        .catalog()
        .iter()
        .map(|entry| entry.identity.to_owned())
        .collect::<BTreeSet<_>>();
    profiles.extend(
        lock.model_catalog()
            .iter()
            .map(|entry| entry.identity.to_owned()),
    );
    profiles.extend(
        lock.temporal_catalog()
            .iter()
            .map(|entry| entry.identity.to_owned()),
    );
    profiles.extend(
        qsl_semantics::value::HeaderLayer::ALL
            .into_iter()
            .map(|layer| layer.identity().to_owned()),
    );
    profiles.extend(
        qsl_semantics::check::TemporalProfile::ALL
            .into_iter()
            .map(|profile| profile.identity().to_owned()),
    );
    profiles
}

fn qsl_consumer(value: ConsumerType) -> bool {
    matches!(
        value,
        ConsumerType::Language | ConsumerType::Runtime | ConsumerType::Tool
    )
}
fn is_qsl_subject(subject: &Subject) -> bool {
    matches!(
        (subject.consumer_type, subject.operation),
        (ConsumerType::Language, Operation::Compile)
            | (ConsumerType::Runtime, Operation::Run | Operation::Replay)
            | (ConsumerType::Tool, Operation::Format)
    )
}

fn execute_vector(vector: &Vector, root: &Path, limits: &qsl_replay::CallerLimits) -> VectorReport {
    let expected = vector.expected.clone();
    let mut inputs = Vec::with_capacity(vector.input.files.len());
    for input in &vector.input.files {
        let path = root.join(&input.path);
        let resolved = match fs::canonicalize(&path) {
            Ok(resolved) if resolved.starts_with(root) => resolved,
            Ok(_) => {
                return vector_failure(
                    vector,
                    expected,
                    VectorOutcome::ToolFailure,
                    json!({"code":"input-io","message":"input resolves outside the corpus root","path":input.path}),
                )
            }
            Err(error) => {
                return vector_failure(
                    vector,
                    expected,
                    VectorOutcome::ToolFailure,
                    json!({"code":"input-io","message":error.to_string(),"path":input.path}),
                )
            }
        };
        let bytes = match fs::read(&resolved) {
            Ok(bytes) => bytes,
            Err(error) => {
                return vector_failure(
                    vector,
                    expected,
                    VectorOutcome::ToolFailure,
                    json!({"code":"input-io","message":error.to_string(),"path":input.path}),
                )
            }
        };
        inputs.push((input, bytes));
    }
    let source = vector
        .input
        .files
        .iter()
        .find(|file| file.kind == InputKind::Source);
    let Some(source) = source else {
        return vector_failure(
            vector,
            expected,
            VectorOutcome::ToolFailure,
            json!({"code":"missing-input","message":"subject requires a source input"}),
        );
    };
    let bytes = inputs
        .iter()
        .find(|(input, _)| input.path == source.path)
        .map(|(_, bytes)| bytes.as_slice())
        .expect("source input was read before execution");
    let identity =
        qsl_foundation::SourceIdentity::new("agent-ix", &source.path, "qualification", "1");
    let result = match (vector.subject.consumer_type, vector.subject.operation) {
        (ConsumerType::Language, Operation::Compile) => {
            let stage_limits = qsl_replay::StageLimits::from_iter(limits.spine.bounds());
            match qsl_replay::compile_package(
                identity,
                &source.path,
                bytes,
                std::iter::empty::<&[u8]>(),
                &qsl_replay::DependencyInput::default(),
                stage_limits,
                limits.replay,
            ) {
                Ok(compiled) => {
                    let package_id = compiled.package_id();
                    (
                        json!({"domain": package_id.domain().as_str(), "hex": package_id.hex()}),
                        None,
                        None,
                    )
                }
                Err(refusal) => (
                    json!({"code": refusal.code().as_str(), "cause": refusal.to_string()}),
                    None,
                    None,
                ),
            }
        }
        (ConsumerType::Tool, Operation::Format) => {
            match qsl_cst::parse(identity, &source.path, bytes, limits.spine.source) {
                Ok(parsed) => match qsl_cst::format::format_with_limits(&parsed, limits.format) {
                    Ok(formatted) => (json!(formatted), Some(formatted), None),
                    Err(refusal) => {
                        let diagnostic = refusal.diagnostic();
                        (
                            json!({"code": diagnostic.code.as_str(), "cause": diagnostic.cause.as_str(), "message": diagnostic.message}),
                            None,
                            None,
                        )
                    }
                },
                Err(diagnostic) => (
                    json!({"code": diagnostic.code.as_str(), "cause": diagnostic.cause.as_str()}),
                    None,
                    None,
                ),
            }
        }
        _ => {
            return vector_failure(
                vector,
                expected,
                VectorOutcome::ToolFailure,
                json!({"code":"qualification_adapter_unavailable","cause":"the corpus-v1 input kind needed by this operation is not yet admitted"}),
            )
        }
    };
    let (actual, _output, fault) = result;
    let outcome = if let Some(fault) = fault {
        return vector_failure(vector, expected, VectorOutcome::ToolFailure, fault);
    } else {
        settle_vector(&expected, &actual)
    };
    VectorReport {
        id: vector.id.clone(),
        outcome,
        expected,
        actual,
    }
}

fn vector_failure(
    vector: &Vector,
    expected: Expected,
    outcome: VectorOutcome,
    actual: Value,
) -> VectorReport {
    VectorReport {
        id: vector.id.clone(),
        outcome,
        expected,
        actual,
    }
}

fn settle_vector(expected: &Expected, actual: &Value) -> VectorOutcome {
    match expected {
        Expected::Value { value } if value == actual => VectorOutcome::Passed,
        Expected::Refusal { code, cause, .. }
            if actual.get("code").and_then(Value::as_str) == Some(code)
                && actual.get("cause").and_then(Value::as_str) == Some(cause) =>
        {
            VectorOutcome::Passed
        }
        _ if matches!(
            actual
                .get("code")
                .and_then(Value::as_str)
                .and_then(Code::from_code),
            Some(Code::UnknownRequiredFeature | Code::UnsupportedProjection)
        ) =>
        {
            VectorOutcome::Unsupported
        }
        _ if actual
            .get("code")
            .and_then(Value::as_str)
            .and_then(Code::from_code)
            .is_some_and(|code| {
                matches!(code, Code::ResourceExhausted | Code::StageLimitExceeded)
            }) =>
        {
            VectorOutcome::Incomplete
        }
        _ => VectorOutcome::Failed,
    }
}

fn settle(outcomes: &[(String, VectorOutcome)]) -> CapabilityOutcome {
    if outcomes.is_empty() {
        return CapabilityOutcome::Uncovered;
    }
    for (outcome, capability) in [
        (VectorOutcome::Failed, CapabilityOutcome::Failed),
        (VectorOutcome::ToolFailure, CapabilityOutcome::ToolFailure),
        (VectorOutcome::Unsupported, CapabilityOutcome::Unsupported),
        (VectorOutcome::Incomplete, CapabilityOutcome::Incomplete),
    ] {
        if outcomes.iter().any(|(_, got)| *got == outcome) {
            return capability;
        }
    }
    CapabilityOutcome::Passed
}

fn matches_outcome(capability: CapabilityOutcome, vector: VectorOutcome) -> bool {
    match capability {
        CapabilityOutcome::Failed => vector == VectorOutcome::Failed,
        CapabilityOutcome::ToolFailure => vector == VectorOutcome::ToolFailure,
        CapabilityOutcome::Unsupported => vector == VectorOutcome::Unsupported,
        CapabilityOutcome::Incomplete => vector == VectorOutcome::Incomplete,
        _ => false,
    }
}

fn count_outcomes(capabilities: &[CapabilityReport]) -> Counts {
    let mut counts = Counts {
        passed: 0,
        failed: 0,
        tool_failure: 0,
        unsupported: 0,
        incomplete: 0,
        uncovered: 0,
    };
    for capability in capabilities {
        match capability.outcome {
            CapabilityOutcome::Passed => counts.passed += 1,
            CapabilityOutcome::Failed => counts.failed += 1,
            CapabilityOutcome::ToolFailure => counts.tool_failure += 1,
            CapabilityOutcome::Unsupported => counts.unsupported += 1,
            CapabilityOutcome::Incomplete => counts.incomplete += 1,
            CapabilityOutcome::Uncovered => counts.uncovered += 1,
        }
    }
    counts
}

fn render_text(report: &Report) -> String {
    let label = if report.default_scope {
        "complete-v1"
    } else {
        "qualification"
    };
    let mut output = format!("{label}: {}\n", report.verdict);
    output.push_str(&format!(
        "scope: {}\ncoverage: {}/{} ({:.0}%)\n",
        report.scope.join(", "),
        report.coverage.passed,
        report.coverage.in_scope,
        report.coverage.percent
    ));
    output.push_str(&format!(
        "counts: passed={} failed={} tool-failure={} unsupported={} incomplete={} uncovered={}\n",
        report.counts.passed,
        report.counts.failed,
        report.counts.tool_failure,
        report.counts.unsupported,
        report.counts.incomplete,
        report.counts.uncovered
    ));
    for capability in &report.capabilities {
        output.push_str(&format!(
            "{}: {:?} [{}]\n",
            capability.capability_id,
            capability.outcome,
            capability.deciding_vector_ids.join(", ")
        ));
    }
    output
}

fn parse_unique_json(bytes: &[u8]) -> std::result::Result<Value, String> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let parsed = UniqueValue::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    Ok(parsed.0)
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value with unique object keys")
            }
            fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::Bool(value)))
            }
            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::from(value)))
            }
            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::from(value)))
            }
            fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::from(value)))
            }
            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::String(value.to_owned())))
            }
            fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::String(value)))
            }
            fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut values = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate object key {key}"
                        )));
                    }
                    let value = map.next_value::<UniqueValue>()?;
                    values.insert(key, value.0);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_unique_json;
    #[test]
    fn rejects_duplicate_object_members() {
        assert!(parse_unique_json(br#"{"a":1,"a":2}"#)
            .unwrap_err()
            .contains("duplicate object key"));
    }
}
