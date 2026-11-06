// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-350..FR-353: execute the QSpec complete-V1 corpus against QSL and
//! settle/report capability qualification.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use qsl_foundation::diagnostic::Code;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::Outcome;

mod adapters;

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
    Monitor,
    #[serde(rename = "analyzer/backend")]
    AnalyzerBackend,
    #[serde(rename = "consuming-system")]
    ConsumingSystem,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Operation {
    Compile,
    Run,
    Replay,
    Format,
    Other(String),
}

impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "compile" => Self::Compile,
            "run" => Self::Run,
            "replay" => Self::Replay,
            "format" => Self::Format,
            operation => Self::Other(operation.to_owned()),
        })
    }
}

impl Serialize for Operation {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Compile => "compile",
            Self::Run => "run",
            Self::Replay => "replay",
            Self::Format => "format",
            Self::Other(operation) => operation,
        })
    }
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

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
enum Expected {
    Value {
        value: DeepValue,
    },
    Refusal {
        code: String,
        cause: String,
        details: DeepValue,
    },
}

impl<'de> Deserialize<'de> for Expected {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Expected;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a closed expected disposition")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut disposition = None;
                let mut value = None;
                let mut code = None;
                let mut cause = None;
                let mut details = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "disposition" if disposition.is_none() => {
                            disposition = Some(map.next_value::<String>()?)
                        }
                        "value" if value.is_none() => value = Some(map.next_value::<DeepValue>()?),
                        "code" if code.is_none() => code = Some(map.next_value::<String>()?),
                        "cause" if cause.is_none() => cause = Some(map.next_value::<String>()?),
                        "details" if details.is_none() => {
                            let value = map.next_value::<DeepValue>()?;
                            if !value.is_object() {
                                return Err(serde::de::Error::custom(
                                    "refusal details is an object",
                                ));
                            }
                            details = Some(value);
                        }
                        "disposition" | "value" | "code" | "cause" | "details" => {
                            return Err(serde::de::Error::custom(
                                "duplicate expected-disposition member",
                            ))
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                &key,
                                &["disposition", "value", "code", "cause", "details"],
                            ))
                        }
                    }
                }
                match disposition.as_deref() {
                    Some("value") if code.is_none() && cause.is_none() && details.is_none() => {
                        Ok(Expected::Value {
                            value: value.ok_or_else(|| serde::de::Error::missing_field("value"))?,
                        })
                    }
                    Some("refusal") if value.is_none() => Ok(Expected::Refusal {
                        code: code.ok_or_else(|| serde::de::Error::missing_field("code"))?,
                        cause: cause.ok_or_else(|| serde::de::Error::missing_field("cause"))?,
                        details: details.unwrap_or_else(|| DeepValue::new(serde_json::json!({}))),
                    }),
                    None => Err(serde::de::Error::missing_field("disposition")),
                    _ => Err(serde::de::Error::custom(
                        "invalid expected disposition members",
                    )),
                }
            }
        }
        deserializer.deserialize_map(Visitor)
    }
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
    actual: DeepValue,
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
    execute(workspace_root, operands).map(|execution| execution.outcome)
}

/// A qualification report and the exact public-operation bytes of its passed
/// vectors. Operation bytes are deliberately separate from the FR-352 report.
pub struct Execution {
    /// The command's report and exit status.
    pub outcome: Outcome,
    /// Passed vector ids in bytewise order, paired with their operation bytes.
    pub outputs: BTreeMap<String, Vec<u8>>,
}

/// Execute qualification, retaining successful operation bytes for callers
/// that compare determinism. The CLI writes only the report.
pub fn execute(workspace_root: &Path, operands: &[OsString]) -> Result<Execution> {
    let qspec = qspec_dir(workspace_root)?;
    execute_with_qspec(&qspec, operands)
}

/// Execute against an explicitly selected QSpec checkout, without changing
/// process-global environment variables.
pub fn execute_with_qspec(qspec: &Path, operands: &[OsString]) -> Result<Execution> {
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
    let adapters = adapters::Schemas::new(qspec, &schema)?;
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .map_err(|error| Error::QualificationRefusal(format!("invalid FR-336 schema: {error}")))?;
    let corpus_path = corpus_root.join(CORPUS_FILE);
    let corpus_bytes = fs::read(&corpus_path).map_err(|error| {
        Error::QualificationRefusal(format!("{}: {error}", corpus_path.display()))
    })?;
    let value = DeepValue::new(parse_unique_json(&corpus_bytes).map_err(|reason| {
        Error::QualificationRefusal(format!("{}: {reason}", corpus_path.display()))
    })?);
    let document = quire_canonical::read(
        &corpus_bytes,
        u64::try_from(corpus_bytes.len()).unwrap_or(u64::MAX),
    )
    .map_err(|error| {
        Error::QualificationRefusal(format!("{}: canonicalize: {error}", corpus_path.display()))
    })?;
    let canonical = quire_canonical::to_vec(&document, quire_canonical::Limits::new(u64::MAX))
        .map_err(|error| {
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
            .map(|error| {
                format!(
                    "{}: fails schema {}",
                    error.instance_path, error.schema_path
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(Error::QualificationRefusal(reasons));
    }
    // Fixed-shape corpus fields decode directly from the bytes. Expected
    // payloads use RawValue below, so serde never builds an internally-tagged
    // recursive Content tree for an arbitrary operation oracle.
    let corpus: Corpus = serde_json::from_slice(&corpus_bytes).map_err(|error| {
        Error::QualificationRefusal(format!("{}: {error}", corpus_path.display()))
    })?;
    drop(value);
    validate_semantics(&corpus)?;

    let all_capabilities = corpus
        .coverage
        .primary
        .iter()
        .map(|cap| (cap.capability_id.as_str(), cap))
        .collect::<BTreeMap<_, _>>();
    let default_scope = args.capabilities.is_empty();
    let profiles = qsl_profiles();
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
    let mut outputs = BTreeMap::new();
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
        let (report, bytes) =
            execute_vector(vector, &corpus_root, &limits, &args.limits, &adapters);
        if report.outcome == VectorOutcome::Passed {
            if let Some(bytes) = bytes {
                outputs.insert(vector.id.clone(), bytes);
            }
        }
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
    Ok(Execution {
        outcome: Outcome {
            stdout,
            code: if qualified { 0 } else { 1 },
        },
        outputs,
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

fn qspec_dir(workspace_root: &Path) -> Result<PathBuf> {
    let path = std::env::var_os("QSPEC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            workspace_root
                .parent()
                .unwrap_or(workspace_root)
                .join("quire-specification")
        });
    fs::canonicalize(&path).map_err(|error| {
        Error::QualificationRefusal(format!("QSPEC_DIR {}: {error}", path.display()))
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
    let mut profiles = qsl_semantics::value::DefinitionRole::ALL
        .into_iter()
        .map(|role| role.identity().to_owned())
        .collect::<BTreeSet<_>>();
    profiles.extend(
        qsl_semantics::value::ModelRole::ALL
            .into_iter()
            .map(|role| role.identity().to_owned()),
    );
    profiles.extend(
        qsl_semantics::value::TemporalRole::ALL
            .into_iter()
            .map(|role| role.identity().to_owned()),
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
        (subject.consumer_type, &subject.operation),
        (ConsumerType::Language, Operation::Compile)
            | (ConsumerType::Runtime, Operation::Run | Operation::Replay)
            | (ConsumerType::Tool, Operation::Format)
    )
}

fn execute_vector(
    vector: &Vector,
    root: &Path,
    limits: &qsl_replay::CallerLimits,
    overrides: &[String],
    schemas: &adapters::Schemas,
) -> (VectorReport, Option<Vec<u8>>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        adapters::execute(vector, root, limits, overrides, schemas)
    }))
    .unwrap_or_else(|payload| Err(adapters::Failure::panic(payload)));
    match result {
        Ok(result) => {
            let outcome = match result.partition {
                adapters::Partition::Result => settle_vector(&vector.expected, &result.actual),
                adapters::Partition::Unsupported => {
                    if matches_expected(&vector.expected, &result.actual) {
                        VectorOutcome::Passed
                    } else {
                        VectorOutcome::Unsupported
                    }
                }
                adapters::Partition::Incomplete => {
                    if matches_expected(&vector.expected, &result.actual) {
                        VectorOutcome::Passed
                    } else {
                        VectorOutcome::Incomplete
                    }
                }
                adapters::Partition::AdapterUnavailable => VectorOutcome::Unsupported,
                adapters::Partition::ToolFailure => VectorOutcome::ToolFailure,
            };
            (
                VectorReport {
                    id: vector.id.clone(),
                    outcome,
                    expected: vector.expected.clone(),
                    actual: DeepValue::new(result.actual),
                },
                result.bytes,
            )
        }
        Err(failure) => (
            vector_failure(
                vector,
                vector.expected.clone(),
                VectorOutcome::ToolFailure,
                serde_json::to_value(failure).expect("adapter failures contain only JSON values"),
            ),
            None,
        ),
    }
}

fn matches_expected(expected: &Expected, actual: &Value) -> bool {
    match expected {
        Expected::Value { value } => json_equal(value, actual),
        Expected::Refusal {
            code,
            cause,
            details,
        } => {
            actual.get("code").and_then(Value::as_str) == Some(code)
                && actual.get("cause").and_then(Value::as_str) == Some(cause)
                && details.as_object().is_some_and(|details| {
                    details.iter().all(|(name, expected)| {
                        actual
                            .get(name)
                            .is_some_and(|value| json_equal(expected, value))
                    })
                })
        }
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
        actual: DeepValue::new(actual),
    }
}

fn settle_vector(expected: &Expected, actual: &Value) -> VectorOutcome {
    match expected {
        Expected::Value { value } if json_equal(value, actual) => VectorOutcome::Passed,
        Expected::Refusal { .. } if matches_expected(expected, actual) => VectorOutcome::Passed,
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
    use quire_canonical::{Malformed, Node, NodeRef, ReadError};

    // The file was admitted under the caller's byte ceiling. The shared
    // reader's arena and every conversion stack are bounded by those bytes;
    // neither introduces a second, hidden nesting ceiling.
    let byte_bound = u64::try_from(bytes.len()).map_err(|error| error.to_string())?;
    let document = quire_canonical::read(bytes, byte_bound).map_err(|error| match error {
        ReadError::Malformed {
            kind: Malformed::DuplicateName,
            offset,
        } => format!("duplicate object key at byte {offset}"),
        other => other.to_string(),
    })?;
    enum Task<'a> {
        Read(NodeRef<'a>),
        Array(usize),
        Object(Vec<&'a str>),
    }
    let mut tasks = vec![Task::Read(document.root())];
    let mut values = Vec::<DeepValue>::new();
    while let Some(task) = tasks.pop() {
        let value = match task {
            Task::Read(node) => match node.node() {
                Node::Null => Value::Null,
                Node::Bool(value) => Value::Bool(value),
                // The shared reader also exposes an f64 for canonical
                // writing. Decode the original scalar lexeme instead: a
                // qualification adapter must not round a supplied integer.
                Node::Number(number) => {
                    serde_json::from_str(number.text()).map_err(|error| error.to_string())?
                }
                Node::String(value) => Value::String(value.to_owned()),
                Node::Array(items) => {
                    let items = items.collect::<Vec<_>>();
                    tasks.push(Task::Array(items.len()));
                    tasks.extend(items.into_iter().rev().map(Task::Read));
                    continue;
                }
                Node::Object(members) => {
                    let members = members.collect::<Vec<_>>();
                    tasks.push(Task::Object(
                        members.iter().map(|(name, _)| *name).collect(),
                    ));
                    tasks.extend(
                        members
                            .into_iter()
                            .rev()
                            .map(|(_, value)| Task::Read(value)),
                    );
                    continue;
                }
            },
            Task::Array(length) => {
                let start = values
                    .len()
                    .checked_sub(length)
                    .ok_or("JSON array children")?;
                Value::Array(
                    values
                        .split_off(start)
                        .into_iter()
                        .map(DeepValue::into_inner)
                        .collect(),
                )
            }
            Task::Object(names) => {
                let start = values
                    .len()
                    .checked_sub(names.len())
                    .ok_or("JSON object children")?;
                Value::Object(
                    names
                        .into_iter()
                        .zip(values.split_off(start))
                        .map(|(name, value)| (name.to_owned(), value.into_inner()))
                        .collect(),
                )
            }
        };
        values.push(DeepValue::new(value));
    }
    values
        .pop()
        .map(DeepValue::into_inner)
        .ok_or_else(|| "missing JSON value".to_owned())
}

/// An untrusted JSON tree whose release never uses the native call stack.
struct DeepValue(Value);

impl DeepValue {
    fn new(value: Value) -> Self {
        Self(value)
    }

    fn into_inner(mut self) -> Value {
        std::mem::take(&mut self.0)
    }
}

impl std::ops::Deref for DeepValue {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for DeepValue {
    fn clone(&self) -> Self {
        enum Task<'a> {
            Read(&'a Value),
            Array(usize),
            Object(Vec<&'a str>),
        }
        let mut tasks = vec![Task::Read(&self.0)];
        let mut values = Vec::<DeepValue>::new();
        while let Some(task) = tasks.pop() {
            let value = match task {
                Task::Read(Value::Array(items)) => {
                    tasks.push(Task::Array(items.len()));
                    tasks.extend(items.iter().rev().map(Task::Read));
                    continue;
                }
                Task::Read(Value::Object(members)) => {
                    tasks.push(Task::Object(members.keys().map(String::as_str).collect()));
                    tasks.extend(members.values().rev().map(Task::Read));
                    continue;
                }
                Task::Read(scalar) => scalar.clone(),
                Task::Array(count) => {
                    // Every close task follows exactly these immutable
                    // source children; caller data cannot change the count.
                    let start = values
                        .len()
                        .checked_sub(count)
                        .expect("cloned array children");
                    Value::Array(
                        values
                            .split_off(start)
                            .into_iter()
                            .map(Self::into_inner)
                            .collect(),
                    )
                }
                Task::Object(names) => {
                    let start = values
                        .len()
                        .checked_sub(names.len())
                        .expect("cloned object children");
                    Value::Object(
                        names
                            .into_iter()
                            .zip(values.split_off(start))
                            .map(|(name, value)| (name.to_owned(), value.into_inner()))
                            .collect(),
                    )
                }
            };
            values.push(Self::new(value));
        }
        values
            .pop()
            .expect("a source JSON value produces one clone")
    }
}

impl std::fmt::Debug for DeepValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Value::Array(items) => formatter
                .debug_struct("JsonArray")
                .field("length", &items.len())
                .finish(),
            Value::Object(members) => formatter
                .debug_struct("JsonObject")
                .field("length", &members.len())
                .finish(),
            scalar => std::fmt::Debug::fmt(scalar, formatter),
        }
    }
}

impl Serialize for DeepValue {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let bytes = json_bytes(&self.0).map_err(serde::ser::Error::custom)?;
        let text = String::from_utf8(bytes).map_err(serde::ser::Error::custom)?;
        let raw =
            serde_json::value::RawValue::from_string(text).map_err(serde::ser::Error::custom)?;
        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DeepValue {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        // RawValue skips the arbitrary JSON subtree iteratively. The shared
        // reader then verifies uniqueness and constructs the guarded tree.
        let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        parse_unique_json(raw.get().as_bytes())
            .map(Self::new)
            .map_err(serde::de::Error::custom)
    }
}

fn json_equal(left: &Value, right: &Value) -> bool {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        match (left, right) {
            (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
                pending.extend(left.iter().zip(right))
            }
            (Value::Object(left), Value::Object(right)) if left.len() == right.len() => {
                for (name, left) in left {
                    let Some(right) = right.get(name) else {
                        return false;
                    };
                    pending.push((left, right));
                }
            }
            (Value::Array(_) | Value::Object(_), _) | (_, Value::Array(_) | Value::Object(_)) => {
                return false
            }
            (left, right) if left != right => return false,
            _ => {}
        }
    }
    true
}

/// Serialize JSON framing iteratively, delegating each scalar (including
/// its exact serde number representation) to the existing JSON encoder.
fn json_bytes(value: &Value) -> serde_json::Result<Vec<u8>> {
    enum Task<'a> {
        Value(&'a Value),
        Name(&'a str),
        Byte(u8),
    }
    let mut bytes = Vec::new();
    let mut pending = vec![Task::Value(value)];
    while let Some(task) = pending.pop() {
        match task {
            Task::Byte(byte) => bytes.push(byte),
            Task::Name(name) => serde_json::to_writer(&mut bytes, name)?,
            Task::Value(Value::Array(items)) => {
                bytes.push(b'[');
                pending.push(Task::Byte(b']'));
                for (index, value) in items.iter().enumerate().rev() {
                    pending.push(Task::Value(value));
                    if index != 0 {
                        pending.push(Task::Byte(b','));
                    }
                }
            }
            Task::Value(Value::Object(members)) => {
                bytes.push(b'{');
                pending.push(Task::Byte(b'}'));
                for (index, (name, value)) in members.iter().enumerate().rev() {
                    pending.push(Task::Value(value));
                    pending.push(Task::Byte(b':'));
                    pending.push(Task::Name(name));
                    if index != 0 {
                        pending.push(Task::Byte(b','));
                    }
                }
            }
            Task::Value(scalar) => serde_json::to_writer(&mut bytes, scalar)?,
        }
    }
    Ok(bytes)
}

impl Drop for DeepValue {
    fn drop(&mut self) {
        qsl_foundation::node_wire::discard_node_json(std::mem::take(&mut self.0));
    }
}

#[cfg(test)]
mod tests {
    use super::{json_bytes, json_equal, parse_unique_json, DeepValue};
    use ix_trace_rs::trace;
    #[test]
    fn rejects_duplicate_object_members() {
        assert!(parse_unique_json(br#"{"a":1,"a":2}"#)
            .unwrap_err()
            .contains("duplicate object key"));
    }

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn deep_json_keeps_numeric_facts_and_rejects_escaped_duplicate_members() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let depth = 1_000;
                let mut bytes = b"[".repeat(depth);
                bytes.extend_from_slice(
                    br#"{"wide":18446744073709551615,"negative_zero":-0.0,"exponent":1e2}"#,
                );
                bytes.extend_from_slice(&b"]".repeat(depth));
                let value = DeepValue::new(
                    parse_unique_json(&bytes)
                        .expect("nesting costs input bytes, not a fixed depth cap"),
                );
                let mut leaf = &*value;
                for _ in 0..depth {
                    leaf = &leaf.as_array().expect("one retained array per layer")[0];
                }
                assert_eq!(leaf["wide"].as_u64(), Some(u64::MAX));
                assert!(leaf["negative_zero"]
                    .as_f64()
                    .expect("a supplied JSON float")
                    .is_sign_negative());
                assert_eq!(leaf["exponent"], serde_json::json!(100.0));

                let mut duplicate = b"[".repeat(depth);
                duplicate.extend_from_slice(br#"{"a":1,"\u0061":2}"#);
                duplicate.extend_from_slice(&b"]".repeat(depth));
                let error = parse_unique_json(&duplicate)
                    .expect_err("decoded duplicate names remain refused at depth");
                assert!(error.contains("duplicate object key"), "{error}");
            })
            .expect("spawn the bounded-stack JSON control")
            .join()
            .expect("deep parsing, refusal and release do not overflow");
    }

    #[trace("TC-875", "FR-350-AC-1", "FR-350-AC-5", "FR-350-AC-11")]
    #[test]
    fn deep_value_serialization_preserves_the_existing_json_scalar_contract() {
        let value = serde_json::json!({
            "wide": u64::MAX,
            "negative_zero": -0.0,
            "exponent": 1e100,
            "nested": [null, true, {"escaped": "\"\\\n😀"}]
        });
        let bytes = json_bytes(&value).expect("existing scalar facts encode");
        assert_eq!(bytes, serde_json::to_vec(&value).unwrap());
        let document = quire_canonical::read(&bytes, u64::MAX).unwrap();
        assert_eq!(
            quire_canonical::to_vec(&document, quire_canonical::Limits::new(u64::MAX)).unwrap(),
            serde_json_canonicalizer::to_vec(&value).unwrap(),
        );
        let read = DeepValue::new(parse_unique_json(&bytes).unwrap());
        assert!(json_equal(&value, &read));
        assert_eq!(serde_json::to_vec(&read).unwrap(), bytes);
    }

    #[trace("TC-875", "FR-350-AC-1", "FR-350-AC-5", "FR-350-AC-11")]
    #[test]
    fn a_fifty_thousand_deep_oracle_returns_failed_and_releases_its_report() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let qspec = std::env::var_os("QSPEC_DIR")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| {
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .ancestors()
                            .map(|parent| parent.join("quire-specification"))
                            .find(|path| path.is_dir())
                            .expect("a QSpec checkout or QSPEC_DIR")
                    });
                let mut nested = b"[".repeat(50_000);
                nested.push(b'0');
                nested.extend_from_slice(&b"]".repeat(50_000));
                let oracle = DeepValue::new(parse_unique_json(&nested).unwrap());
                let copy = oracle.clone();
                assert!(json_equal(&oracle, &copy));
                drop(copy);

                for refusal in [false, true] {
                    let directory = tempfile::tempdir().unwrap();
                    let corpus_root = directory.path();
                    std::fs::write(corpus_root.join("unit.native"),
                        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction f using v(x: Int[0,9]): Int[0,9] pure { x }\n").unwrap();
                    let expected = if refusal {
                        serde_json::json!({"disposition":"refusal", "code":"ill_typed", "cause":"type-mismatch", "details":{"deep":null}})
                    } else {
                        serde_json::json!({"disposition":"value", "value":null})
                    };
                    let mut corpus = DeepValue::new(serde_json::json!({
                        "contract_version":"quire.conformance-corpus/v1",
                        "vectors":[{"id":"V1-TOOL-004-depth", "capability_id":"V1-TOOL-004", "class":"positive", "subject":{"consumer_type":"tool", "operation":"format"}, "applicability":{"consumer_types":["tool"]}, "input":{"files":[{"kind":"source", "path":"unit.native"}]}, "expected":expected}],
                        "results":[],
                        "coverage":{"primary":[{"capability_id":"V1-TOOL-004", "applicability":{"consumer_types":["tool"], "semantic_profiles":["quire.value.complete/v1"]}}], "criteria_rollups":[], "consumer_rollups":[]}
                    }));
                    let expected = &mut corpus.0["vectors"][0]["expected"];
                    if refusal {
                        expected["details"]["deep"] = oracle.clone().into_inner();
                    } else {
                        expected["value"] = oracle.clone().into_inner();
                    }
                    std::fs::write(corpus_root.join("complete-v1.json"), json_bytes(&corpus).unwrap()).unwrap();
                    let arguments = [
                        "--corpus".into(), corpus_root.as_os_str().to_owned(),
                    ];
                    let execution = super::execute_with_qspec(&qspec, &arguments)
                        .expect("a mismatching deep oracle settles rather than aborting");
                    assert_eq!(execution.outcome.code, 1);
                    let report = DeepValue::new(parse_unique_json(execution.outcome.stdout.as_bytes()).unwrap());
                    assert_eq!(report["capabilities"][0]["vectors"][0]["outcome"], "failed");
                    assert!(report["capabilities"][0]["vectors"][0]["actual"]
                        .as_str()
                        .expect("the real formatter succeeded before oracle comparison")
                        .contains("function f"));
                    assert_eq!(report["verdict"], "not-qualified");
                    let expected = &report["capabilities"][0]["vectors"][0]["expected"];
                    let retained = if refusal { &expected["details"]["deep"] } else { &expected["value"] };
                    assert!(json_equal(&oracle, retained), "the complete oracle is retained in the report");
                    let repeated = super::execute_with_qspec(&qspec, &arguments)
                        .expect("the repeated complete qualification remains admitted");
                    assert_eq!(execution.outcome.stdout, repeated.outcome.stdout);
                }
            })
            .expect("spawn the bounded-stack complete qualification control")
            .join()
            .expect("corpus admission, comparison, report serialization and cleanup do not overflow");
    }
}
