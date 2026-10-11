// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-267: canonical clause-run result production from the FR-109 report.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::Code;
use qsl_foundation::diagnostic::Locus;
use qsl_foundation::wire_format::WireFormat;
use qsl_foundation::LocatedSpan;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::call::{CallOutcome, CallRefusal, CallValue};
use super::{ClauseDisposition, ClauseRunReport};
use crate::result::RecordWire;

/// Why a clause-run report could not be written as a canonical document.
#[derive(Debug, thiserror::Error)]
pub enum NativeRunResultError {
    /// The report's settlement basis and witness disagree.
    #[error("basis and witness presence disagree")]
    BasisWitness,
    /// A verdict has no decisive or closed settlement, or a non-verdict has one.
    #[error("basis and disposition disagree")]
    BasisDisposition,
    /// A separating witness did not have the record its family assigns.
    #[error("invalid separating witness: {0}")]
    Witness(#[from] crate::result::CauseCodecError),
    /// The typed report could not be represented as JSON.
    #[error("cannot serialize clause-run result: {0}")]
    Json(#[from] serde_json::Error),
    /// The canonical reader refused a producer document.
    #[error("cannot read clause-run result for canonical encoding: {0}")]
    Read(#[from] quire_canonical::ReadError),
    /// The canonical encoder could not write the report.
    #[error("cannot encode clause-run result canonically: {0}")]
    Canonical(#[from] quire_canonical::Error),
}

/// A refused `native-run-result/2` read, with the failing wire member named.
#[derive(Debug, thiserror::Error)]
pub enum NativeRunReadError {
    /// The identifier is absent or is not the one supported version.
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
    /// The document or a closed member is malformed.
    #[error("invalid native run result: {0}")]
    Malformed(#[from] serde_json::Error),
    /// A closed member has a wrong type or shape at this exact path.
    #[error("invalid member {member}: {source}")]
    MalformedMember {
        /// The structured member path.
        member: String,
        /// The original JSON decoder error with its line and column.
        #[source]
        source: serde_json::Error,
    },
    /// The canonical JSON reader refused the document.
    #[error("invalid native run result: {0}")]
    Read(#[from] quire_canonical::ReadError),
    /// A report member contradicts another report member.
    #[error("invalid member {member}: {reason}")]
    Invalid {
        /// The member at fault.
        member: &'static str,
        /// The failed constraint.
        reason: &'static str,
    },
    /// The typed separating record is malformed.
    #[error("invalid member witness: {0}")]
    Witness(#[from] crate::result::CauseCodecError),
}

/// The two closed result variants of the same `/2` reader.
pub enum NativeRunResult {
    /// A clause run reached `run_clause` and returned its report.
    Clause(Box<ClauseRunRecord>),
    /// Request intake or command handling failed before a report existed.
    CommandError(CommandErrorRecord),
}

/// The typed clause-run document returned by the strict reader.
pub struct ClauseRunRecord(ClauseRead);

impl ClauseRunRecord {
    /// The compiled package identity, absent before compile completed.
    pub fn package_id(&self) -> Option<&str> {
        self.0.package_id.as_deref()
    }

    /// The settlement basis label.
    pub fn basis(&self) -> &'static str {
        self.0.basis.as_str()
    }

    /// Whether the document carries a separating witness.
    pub fn has_witness(&self) -> bool {
        self.0.witness.is_some()
    }

    /// The reported disposition stage.
    pub fn stage(&self) -> &'static str {
        self.0.disposition.stage.as_str()
    }

    /// The reported outcome category.
    pub fn category(&self) -> &'static str {
        self.0.disposition.category.as_str()
    }

    /// The truth of a completed Boolean claim, when one exists.
    pub fn truth(&self) -> Option<bool> {
        self.0.disposition.truth
    }

    /// The catalog code of a non-verdict disposition, when it has one.
    pub fn code(&self) -> Option<&str> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .map(|r| r.code.as_str())
            .or_else(|| match self.0.disposition.outcome.as_ref() {
                Some(OutcomeRead::Refused { code, .. }) => Some(code.as_str()),
                _ => None,
            })
    }

    /// The catalog cause of a non-verdict disposition, when it has one.
    pub fn cause(&self) -> Option<&str> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .and_then(|r| r.cause.as_deref())
            .or_else(|| match self.0.disposition.outcome.as_ref() {
                Some(OutcomeRead::Refused { cause, .. }) => Some(cause.as_str()),
                _ => None,
            })
    }

    /// Structured refusal fields supplied by the underlying stage.
    pub fn fields(&self) -> Option<&BTreeMap<String, String>> {
        self.0
            .disposition
            .refusal
            .as_ref()
            .and_then(|r| r.fields.as_ref())
            .or_else(|| match self.0.disposition.outcome.as_ref() {
                Some(OutcomeRead::Refused { fields, .. }) => fields.as_ref(),
                _ => None,
            })
    }

    /// Whether a refused S6a outcome retains its located source region.
    pub fn has_outcome_locus(&self) -> bool {
        matches!(
            self.0.disposition.outcome.as_ref(),
            Some(OutcomeRead::Refused { locus: Some(_), .. })
        )
    }

    /// Whether a refused S6a outcome retains its typed expression location.
    pub fn has_outcome_location(&self) -> bool {
        matches!(
            self.0.disposition.outcome.as_ref(),
            Some(OutcomeRead::Refused {
                location: Some(_),
                ..
            })
        )
    }

    /// The stable identifiers of an internal failure, when present.
    pub fn internal_fault(&self) -> Option<(&str, &str)> {
        self.0
            .disposition
            .internal_fault
            .as_ref()
            .map(|fault| (fault.stage.as_str(), fault.invariant.as_str()))
    }

    /// The S6a outcome, when evaluation returned one.
    pub fn outcome(&self) -> Option<NativeRunOutcome<'_>> {
        self.0
            .disposition
            .outcome
            .as_ref()
            .map(|outcome| match outcome {
                OutcomeRead::Completed {
                    value: ValueRead::Boolean { value },
                } => NativeRunOutcome::Boolean(*value),
                OutcomeRead::Completed {
                    value: ValueRead::Integer { decimal },
                } => NativeRunOutcome::Integer(decimal),
                OutcomeRead::Refused { .. } => NativeRunOutcome::Refused,
                OutcomeRead::Undefined { reason } => NativeRunOutcome::Undefined(reason),
                OutcomeRead::Incomplete { limit } => NativeRunOutcome::Incomplete(limit),
            })
    }

    /// Admission bytes, objects and values consumed, in that order.
    pub fn admission_usage(&self) -> (u64, u64, u64) {
        let usage = &self.0.usage.admission;
        (usage.document_bytes, usage.objects, usage.values)
    }

    /// The number of evaluation meter charges admitted.
    pub fn evaluation_admissions(&self) -> u64 {
        self.0.usage.evaluation_admissions
    }

    /// Evaluation counter names and consumed amounts, in catalog order.
    pub fn evaluation_consumed(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0
            .usage
            .evaluation_consumed
            .iter()
            .map(|counter| (counter.kind.as_str(), counter.consumed))
    }
}

/// The typed S6a outcome retained by a clause-run document.
pub enum NativeRunOutcome<'a> {
    /// A Boolean claim completed with this truth.
    Boolean(bool),
    /// An integer value completed with this canonical decimal spelling.
    Integer(&'a str),
    /// Evaluation refused at its catalogued cause.
    Refused,
    /// The claim was undefined for this reason.
    Undefined(&'a str),
    /// Evaluation exhausted this counter.
    Incomplete(&'a str),
}

/// The typed command-error document returned by the strict reader.
pub struct CommandErrorRecord(CommandErrorRead);

impl CommandErrorRecord {
    /// The stage that failed.
    pub fn stage(&self) -> &str {
        &self.0.stage
    }

    /// The catalog code of the command failure.
    pub fn code(&self) -> &str {
        &self.0.code
    }

    /// The catalog cause, where the code has one.
    pub fn cause(&self) -> Option<&str> {
        self.0.cause.as_deref()
    }

    /// The human-readable diagnostic.
    pub fn message(&self) -> &str {
        &self.0.message
    }

    /// The code-specific closed detail object as its original JSON value.
    pub fn details(&self) -> &RawValue {
        &self.0.details
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Basis {
    ClosedScope,
    DecisiveWitness,
    DecisiveCounterexample,
    Unsettled,
    Unavailable,
}

impl Basis {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ClosedScope => "closed-scope",
            Self::DecisiveWitness => "decisive-witness",
            Self::DecisiveCounterexample => "decisive-counterexample",
            Self::Unsettled => "unsettled",
            Self::Unavailable => "unavailable",
        }
    }

    fn is_decisive(&self) -> bool {
        matches!(self, Self::DecisiveWitness | Self::DecisiveCounterexample)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Compile,
    Select,
    Admit,
    Evaluate,
}

impl Stage {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Select => "select",
            Self::Admit => "admit",
            Self::Evaluate => "evaluate",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Category {
    Success,
    Violation,
    Undefined,
    Refusal,
    Unsupported,
    Incomplete,
    Inconclusive,
    InternalFailure,
}

impl Category {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Violation => "violation",
            Self::Undefined => "undefined",
            Self::Refusal => "refusal",
            Self::Unsupported => "unsupported",
            Self::Incomplete => "incomplete",
            Self::Inconclusive => "inconclusive",
            Self::InternalFailure => "internal-failure",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CounterRead {
    kind: String,
    consumed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionRead {
    document_bytes: u64,
    objects: u64,
    values: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UsageRead {
    admission: AdmissionRead,
    evaluation_admissions: u64,
    evaluation_consumed: Vec<CounterRead>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum OutcomeRead {
    Completed {
        value: ValueRead,
    },
    Refused {
        code: String,
        cause: String,
        #[serde(default, deserialize_with = "present")]
        fields: Option<BTreeMap<String, String>>,
        #[serde(default, deserialize_with = "present")]
        locus: Option<CallLocusRead>,
        #[serde(default, deserialize_with = "present")]
        location: Option<CallLocationRead>,
    },
    Undefined {
        reason: String,
    },
    Incomplete {
        limit: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ValueRead {
    Boolean { value: bool },
    Integer { decimal: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct CallLocusRead {
    source_digest: String,
    span: SpanRead,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct SpanRead {
    start: PositionRead,
    end: PositionRead,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct PositionRead {
    byte: usize,
    line: usize,
    column: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct CallLocationRead {
    origin: CallOriginRead,
    path: Vec<usize>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
enum CallOriginRead {
    Body { function: String, index: usize },
    Measure { function: String, index: usize },
    Expression,
    TypeDeclaration { name: String },
    StateClause { clause: String, index: usize },
    ProtocolAttempt { protocol: usize, attempt: usize },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InternalFaultRead {
    stage: String,
    invariant: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct RefusalRead {
    code: String,
    #[serde(default, deserialize_with = "present")]
    cause: Option<String>,
    #[serde(default, deserialize_with = "present")]
    fields: Option<BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "present")]
    locus: Option<RefusalLocusRead>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
enum RefusalLocusRead {
    Region {
        source: SourceRefRead,
        start: u64,
        end: u64,
    },
    Located {
        source_digest: String,
        span: SpanRead,
    },
    Occurrence {
        occurrence: String,
    },
    Artifact {
        domain: String,
        digest: String,
        pointer: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
struct SourceRefRead {
    authority: String,
    identity: String,
    digest_domain: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DispositionRead {
    stage: Stage,
    category: Category,
    #[serde(default, deserialize_with = "present")]
    truth: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    outcome: Option<OutcomeRead>,
    #[serde(default, deserialize_with = "present")]
    internal_fault: Option<InternalFaultRead>,
    #[serde(default, deserialize_with = "present")]
    refusal: Option<RefusalRead>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClauseRead {
    format: String,
    #[serde(default, deserialize_with = "present")]
    package_id: Option<String>,
    disposition: DispositionRead,
    usage: UsageRead,
    basis: Basis,
    #[serde(default, deserialize_with = "present")]
    witness: Option<RecordWire<Box<RawValue>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandErrorRead {
    format: String,
    stage: String,
    code: String,
    #[serde(default, deserialize_with = "present")]
    cause: Option<String>,
    message: String,
    details: Box<RawValue>,
    basis: Basis,
}

fn present<'de, D, T>(decoder: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(decoder).map(Some)
}

fn read_typed<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, NativeRunReadError> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    serde_path_to_error::deserialize(&mut decoder).map_err(|error| {
        NativeRunReadError::MalformedMember {
            member: error.path().to_string(),
            source: error.into_inner(),
        }
    })
}

fn validate_disposition(value: &DispositionRead) -> Result<(), NativeRunReadError> {
    let invalid = |member, reason| NativeRunReadError::Invalid { member, reason };
    match (&value.stage, &value.category) {
        (Stage::Compile | Stage::Select | Stage::Admit, Category::InternalFailure) => {
            if value.internal_fault.is_none()
                || value.refusal.is_some()
                || value.outcome.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.internal_fault",
                    "internal failure needs only its typed fault",
                ));
            }
        }
        (Stage::Compile | Stage::Select | Stage::Admit, _) => {
            if value.refusal.is_none()
                || value.internal_fault.is_some()
                || value.outcome.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.refusal",
                    "pre-evaluation result needs one refusal record",
                ));
            }
            if matches!(value.stage, Stage::Select) {
                let refusal = value.refusal.as_ref().expect("checked above");
                if refusal.cause.is_none() {
                    return Err(invalid(
                        "disposition.refusal.cause",
                        "selection refusal needs its cause",
                    ));
                }
                if refusal.fields.is_none() {
                    return Err(invalid(
                        "disposition.refusal.fields",
                        "selection refusal needs its fields",
                    ));
                }
            }
        }
        (Stage::Evaluate, Category::InternalFailure) => {
            if value.internal_fault.is_none()
                || value.refusal.is_some()
                || value.outcome.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.internal_fault",
                    "evaluate internal failure needs only its typed fault",
                ));
            }
        }
        (Stage::Evaluate, Category::Success | Category::Violation) => {
            let completed = match &value.outcome {
                Some(OutcomeRead::Completed {
                    value: ValueRead::Boolean { value },
                }) => Some(*value),
                _ => None,
            };
            let is_frame = matches!(value.category, Category::Violation)
                && value.refusal.is_some()
                && value.outcome.is_none();
            if value.internal_fault.is_some()
                || (!is_frame && (completed.is_none() || completed != value.truth))
                || (is_frame && value.truth != Some(false))
            {
                return Err(invalid(
                    "disposition.outcome",
                    "verdict needs its Boolean completion or frame record",
                ));
            }
        }
        (Stage::Evaluate, Category::Undefined) => {
            if !matches!(value.outcome.as_ref(), Some(OutcomeRead::Undefined { .. }))
                || value.internal_fault.is_some()
                || value.refusal.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.outcome",
                    "undefined needs its FR-100 reason",
                ));
            }
        }
        (Stage::Evaluate, Category::Refusal | Category::Unsupported) => {
            if !matches!(value.outcome.as_ref(), Some(OutcomeRead::Refused { .. }))
                || value.internal_fault.is_some()
                || value.refusal.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.outcome",
                    "refusal needs its FR-100 record",
                ));
            }
        }
        (Stage::Evaluate, Category::Incomplete) => {
            if !matches!(
                value.outcome.as_ref(),
                Some(OutcomeRead::Incomplete { .. } | OutcomeRead::Refused { .. })
            ) || value.internal_fault.is_some()
                || value.refusal.is_some()
                || value.truth.is_some()
            {
                return Err(invalid(
                    "disposition.outcome",
                    "incomplete needs its FR-100 outcome",
                ));
            }
        }
        (Stage::Evaluate, Category::Inconclusive) => {
            return Err(invalid(
                "disposition.category",
                "clause run has no inconclusive category",
            ))
        }
    }
    Ok(())
}

/// FR-100's command refusal table fixes these rows' cause and detail shapes.
#[qsl_attrs::string_edge]
fn validate_command_details(
    document: &CommandErrorRead,
    details: &serde_json::Map<String, serde_json::Value>,
    code: Code,
) -> Result<(), NativeRunReadError> {
    let invalid = |member, reason| NativeRunReadError::Invalid { member, reason };
    match code {
        Code::UnknownEdition => {
            if document.stage != "profile" || document.cause.is_some() {
                return Err(invalid("cause", "unknown_edition has no cause at profile"));
            }
            if details.get("phase").and_then(serde_json::Value::as_str) != Some("profile")
                || details.get("code").and_then(serde_json::Value::as_str)
                    != Some("unknown_edition")
                || details
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
            {
                return Err(invalid(
                    "details",
                    "unknown_edition needs its profile diagnostic",
                ));
            }
        }
        Code::MissingDeclaration | Code::UnsupportedConstruct if document.stage == "call" => {
            if document.cause.is_some() {
                return Err(invalid("cause", "call selection refusal has no cause"));
            }
            if details.len() != 1
                || details
                    .get("function")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
            {
                return Err(invalid("details", "call selection needs function"));
            }
        }
        Code::InvalidRuntimeInput if document.stage == "call" => {
            if document.cause.is_some() {
                return Err(invalid("cause", "call argument refusal has no cause"));
            }
            let parameter = details.get("parameter").and_then(serde_json::Value::as_str);
            let position = details.get("position").and_then(serde_json::Value::as_u64);
            if details.len() != 1 || (parameter.is_some() == position.is_some()) {
                return Err(invalid(
                    "details",
                    "call argument needs parameter or position",
                ));
            }
        }
        Code::RuntimeInvariant if document.stage == "call" => {
            if document.cause.as_deref() != Some("established-invariant-broken") {
                return Err(invalid(
                    "cause",
                    "runtime_invariant needs its catalog cause",
                ));
            }
            if details.len() != 2
                || details
                    .get("stage")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
                || details
                    .get("invariant")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
            {
                return Err(invalid(
                    "details",
                    "runtime_invariant needs stage and invariant",
                ));
            }
        }
        Code::InvalidRequest if document.stage == "request" && document.cause.is_some() => {
            return Err(invalid("cause", "invalid-request has no cause"));
        }
        _ => {}
    }
    Ok(())
}

/// Read the exact `/2` identifier before choosing the closed clause-run or
/// command-error shape. No other format is inferred from its payload.
#[qsl_attrs::string_edge]
pub fn read_native_run_result(bytes: &[u8]) -> Result<NativeRunResult, NativeRunReadError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let format = value
        .get("format")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<missing>");
    if format != WireFormat::RunResult.as_str() {
        return Err(NativeRunReadError::UnsupportedFormat(format.to_owned()));
    }
    quire_canonical::read(bytes, u64::MAX)?;
    if value.get("disposition").is_some() {
        let document: ClauseRead = read_typed(bytes)?;
        if document.format != format {
            return Err(NativeRunReadError::UnsupportedFormat(document.format));
        }
        if document.package_id.as_ref().is_some_and(|id| {
            id.len() != 64
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            return Err(NativeRunReadError::Invalid {
                member: "package_id",
                reason: "expected 64 lowercase hex digits",
            });
        }
        if document.basis.is_decisive() != document.witness.is_some() {
            return Err(NativeRunReadError::Invalid {
                member: "witness",
                reason: "presence disagrees with basis",
            });
        }
        if let Some(witness) = &document.witness {
            witness.validate_state()?;
        }
        validate_disposition(&document.disposition)?;
        if matches!(
            document.disposition.category,
            Category::Success | Category::Violation
        ) != document.disposition.truth.is_some()
        {
            return Err(NativeRunReadError::Invalid {
                member: "truth",
                reason: "presence disagrees with category",
            });
        }
        let verdict = matches!(
            document.disposition.category,
            Category::Success | Category::Violation
        );
        if (verdict
            && !matches!(
                document.basis,
                Basis::ClosedScope | Basis::DecisiveWitness | Basis::DecisiveCounterexample
            ))
            || (!verdict && !matches!(document.basis, Basis::Unavailable))
        {
            return Err(NativeRunReadError::Invalid {
                member: "basis",
                reason: "basis disagrees with disposition",
            });
        }
        Ok(NativeRunResult::Clause(Box::new(ClauseRunRecord(document))))
    } else {
        let document: CommandErrorRead = read_typed(bytes)?;
        if document.format != format {
            return Err(NativeRunReadError::UnsupportedFormat(document.format));
        }
        if !matches!(document.basis, Basis::Unavailable) {
            return Err(NativeRunReadError::Invalid {
                member: "basis",
                reason: "command errors use unavailable",
            });
        }
        let details: serde_json::Value = serde_json::from_str(document.details.get())?;
        let Some(details) = details.as_object() else {
            return Err(NativeRunReadError::Invalid {
                member: "details",
                reason: "expected an object",
            });
        };
        let Some(code) = Code::from_code(&document.code) else {
            return Err(NativeRunReadError::Invalid {
                member: "code",
                reason: "unknown catalog code",
            });
        };
        validate_command_details(&document, details, code)?;
        Ok(NativeRunResult::CommandError(CommandErrorRecord(document)))
    }
}

#[derive(Serialize)]
struct CounterUsage {
    kind: &'static str,
    consumed: u64,
}

#[derive(Serialize)]
struct Usage {
    admission: AdmissionUsage,
    evaluation_admissions: u64,
    evaluation_consumed: Vec<CounterUsage>,
}

#[derive(Serialize)]
struct AdmissionUsage {
    document_bytes: u64,
    objects: u64,
    values: u64,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Outcome {
    Completed {
        value: ValueWire,
    },
    Refused {
        code: &'static str,
        cause: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<BTreeMap<&'static str, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        locus: Option<CallLocusWire>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<CallLocationWire>,
    },
    Undefined {
        reason: &'static str,
    },
    Incomplete {
        limit: &'static str,
    },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ValueWire {
    Boolean { value: bool },
    Integer { decimal: String },
}

#[derive(Serialize)]
struct CallLocusWire {
    source_digest: String,
    span: LocatedSpan,
}

#[derive(Serialize)]
struct CallLocationWire {
    origin: CallOriginWire,
    path: Vec<usize>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum CallOriginWire {
    Body { function: String, index: usize },
    Measure { function: String, index: usize },
    Expression,
    TypeDeclaration { name: String },
    StateClause { clause: String, index: usize },
    ProtocolAttempt { protocol: usize, attempt: usize },
}

impl From<&quire_semantic_value::location::Location> for CallLocationWire {
    fn from(location: &quire_semantic_value::location::Location) -> Self {
        use quire_semantic_value::location::Origin;
        let origin = match &location.origin {
            Origin::Body { function, index } => CallOriginWire::Body {
                function: function.clone(),
                index: *index,
            },
            Origin::Measure { function, index } => CallOriginWire::Measure {
                function: function.clone(),
                index: *index,
            },
            Origin::Expression => CallOriginWire::Expression,
            Origin::TypeDeclaration { name } => {
                CallOriginWire::TypeDeclaration { name: name.clone() }
            }
            Origin::StateClause { clause, index } => CallOriginWire::StateClause {
                clause: clause.clone(),
                index: *index,
            },
            Origin::ProtocolAttempt { protocol, attempt } => CallOriginWire::ProtocolAttempt {
                protocol: *protocol,
                attempt: *attempt,
            },
        };
        Self {
            origin,
            path: location.path(),
        }
    }
}

#[derive(Serialize)]
struct InternalFaultWire {
    stage: &'static str,
    invariant: &'static str,
}

#[derive(Serialize)]
struct RefusalWire {
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<BTreeMap<&'static str, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locus: Option<crate::outcome::OutcomeLocus>,
}

impl RefusalWire {
    fn new(code: &'static str, cause: Option<&'static str>) -> Self {
        Self {
            code,
            cause,
            fields: None,
            locus: None,
        }
    }
    fn fields(mut self, fields: BTreeMap<&'static str, String>) -> Self {
        self.fields = Some(fields);
        self
    }
}

#[derive(Serialize)]
struct Disposition {
    stage: &'static str,
    category: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    truth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    internal_fault: Option<InternalFaultWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<RefusalWire>,
}

#[derive(Serialize)]
struct ClauseRunDocument {
    format: WireFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_id: Option<String>,
    disposition: Disposition,
    usage: Usage,
    basis: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    witness: Option<RecordWire<Box<RawValue>>>,
}

impl Disposition {
    fn of(value: &ClauseDisposition) -> Self {
        let mut result = Self {
            stage: match value.stage() {
                super::ClauseRunStage::Compile => "compile",
                super::ClauseRunStage::Select => "select",
                super::ClauseRunStage::Admit => "admit",
                super::ClauseRunStage::Evaluate => "evaluate",
            },
            category: value.category().as_str(),
            truth: value.truth(),
            outcome: None,
            internal_fault: None,
            refusal: None,
        };
        match value {
            ClauseDisposition::Compile(refusal) => {
                let mut record = RefusalWire::new(refusal.code().as_str(), refusal.cause());
                record.locus = refusal.region().map(|region| {
                    crate::outcome::OutcomeLocus::from(&Locus::Region(region.clone()))
                });
                result.refusal = Some(record);
            }
            ClauseDisposition::UnknownLanguage { language } => {
                result.refusal = Some(
                    RefusalWire::new(Code::UnknownLanguage.as_str(), None)
                        .fields(BTreeMap::from([("language", language.clone())])),
                );
            }
            ClauseDisposition::StalePackage { expected, actual } => {
                result.refusal = Some(
                    RefusalWire::new(Code::StaleDependency.as_str(), Some("stale-package")).fields(
                        BTreeMap::from([("expected", expected.clone()), ("actual", actual.hex())]),
                    ),
                );
            }
            ClauseDisposition::MissingName { name } => {
                result.refusal = Some(
                    RefusalWire::new(Code::MissingDeclaration.as_str(), Some("missing-name"))
                        .fields(BTreeMap::from([("name", name.clone())])),
                );
            }
            ClauseDisposition::NotAPredicate { name } => {
                result.refusal = Some(
                    RefusalWire::new(Code::IllTyped.as_str(), Some("type-mismatch"))
                        .fields(BTreeMap::from([("name", name.clone())])),
                );
            }
            ClauseDisposition::Admit(failure) => match failure {
                qsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | qsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                    result.refusal = Some(
                        RefusalWire::new(record.code.as_str(), Some(record.cause))
                            .fields(record.fields.clone()),
                    );
                }
                qsl_semantics::model::observation::AdmissionFailure::Fault(fault) => {
                    result.internal_fault = Some(InternalFaultWire {
                        stage: fault.stage(),
                        invariant: fault.invariant(),
                    });
                }
            },
            ClauseDisposition::ArgumentRefusal(refusal) => {
                if let super::RunRefusal::Fault(fault) = refusal.as_ref() {
                    result.internal_fault = Some(InternalFaultWire {
                        stage: fault.stage(),
                        invariant: fault.invariant(),
                    });
                    return result;
                }
                let cause = match refusal.as_ref() {
                    super::RunRefusal::Compile(compile) => compile.cause(),
                    super::RunRefusal::Fault(_) => None,
                    super::RunRefusal::MissingDeclaration { .. }
                    | super::RunRefusal::UnsupportedResult { .. }
                    | super::RunRefusal::UnknownParameter { .. }
                    | super::RunRefusal::DuplicateArgument { .. }
                    | super::RunRefusal::UnboundParameter { .. }
                    | super::RunRefusal::WrongValueKind { .. }
                    | super::RunRefusal::Cancelled(_) => None,
                };
                let mut record = RefusalWire::new(refusal.code().as_str(), cause);
                match refusal.as_ref() {
                    super::RunRefusal::Compile(compile) => {
                        record.locus = compile.region().map(|region| {
                            crate::outcome::OutcomeLocus::from(&Locus::Region(region.clone()))
                        });
                    }
                    super::RunRefusal::MissingDeclaration { function }
                    | super::RunRefusal::UnsupportedResult { function } => {
                        record = record.fields(BTreeMap::from([("function", function.clone())]));
                    }
                    super::RunRefusal::UnknownParameter { parameter }
                    | super::RunRefusal::DuplicateArgument { parameter }
                    | super::RunRefusal::UnboundParameter { parameter } => {
                        record = record.fields(BTreeMap::from([("parameter", parameter.clone())]));
                    }
                    super::RunRefusal::WrongValueKind { position } => {
                        record =
                            record.fields(BTreeMap::from([("position", position.to_string())]));
                    }
                    super::RunRefusal::Fault(_) | super::RunRefusal::Cancelled(_) => {}
                }
                result.refusal = Some(record);
            }
            ClauseDisposition::Evaluate(outcome) => match outcome {
                CallOutcome::Completed(CallValue::Boolean(value)) => {
                    result.outcome = Some(Outcome::Completed {
                        value: ValueWire::Boolean { value: *value },
                    });
                }
                CallOutcome::Completed(CallValue::Integer(value)) => {
                    result.outcome = Some(Outcome::Completed {
                        value: ValueWire::Integer {
                            decimal: value.to_string(),
                        },
                    });
                }
                CallOutcome::Refused(refusal) => {
                    result.outcome = Some(match refusal {
                        CallRefusal::Record {
                            code,
                            fields,
                            locus,
                            location,
                        } => Outcome::Refused {
                            code: code.code(),
                            cause: code.cause(),
                            fields: Some(fields.clone()),
                            locus: locus.as_ref().map(|locus| CallLocusWire {
                                source_digest: locus.source_digest.clone(),
                                span: locus.span,
                            }),
                            location: location.as_ref().map(CallLocationWire::from),
                        },
                        CallRefusal::Family { code, location } => Outcome::Refused {
                            code: code.code(),
                            cause: code.cause(),
                            fields: None,
                            locus: None,
                            location: location.as_ref().map(CallLocationWire::from),
                        },
                    });
                }
                CallOutcome::Undefined { reason } => {
                    result.outcome = Some(Outcome::Undefined { reason });
                }
                CallOutcome::Incomplete(incomplete) => {
                    result.outcome = Some(Outcome::Incomplete {
                        limit: incomplete.limits_field(),
                    });
                }
            },
            ClauseDisposition::FrameViolation(witness) => {
                result.refusal = Some(
                    RefusalWire::new(witness.code.as_str(), Some(witness.cause))
                        .fields(BTreeMap::from([("population", witness.population.clone())])),
                );
            }
            ClauseDisposition::EvaluateFault(fault) => {
                result.internal_fault = Some(InternalFaultWire {
                    stage: fault.stage(),
                    invariant: fault.invariant(),
                });
            }
        }
        result
    }
}

/// Serialize one FR-109 report as RFC 8785 `native-run-result/2` bytes.
pub fn write_clause_run_result(report: &ClauseRunReport) -> Result<Vec<u8>, NativeRunResultError> {
    if report.basis.is_decisive() != report.witness.is_some() {
        return Err(NativeRunResultError::BasisWitness);
    }
    let verdict = matches!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Success
            | qsl_foundation::diagnostic::Category::Violation
    );
    if (verdict
        && !matches!(
            report.basis,
            crate::proof_result::SettlementBasis::ClosedScope
                | crate::proof_result::SettlementBasis::DecisiveWitness
                | crate::proof_result::SettlementBasis::DecisiveCounterexample
        ))
        || (!verdict
            && !matches!(
                report.basis,
                crate::proof_result::SettlementBasis::Unavailable
            ))
    {
        return Err(NativeRunResultError::BasisDisposition);
    }
    let witness = report.witness.as_ref().map(RecordWire::of).transpose()?;
    let document = ClauseRunDocument {
        format: WireFormat::RunResult,
        package_id: report.package_id.as_ref().map(|id| id.hex()),
        disposition: Disposition::of(&report.disposition),
        usage: Usage {
            admission: AdmissionUsage {
                document_bytes: report.usage.admission_consumed.document_bytes,
                objects: report.usage.admission_consumed.objects,
                values: report.usage.admission_consumed.values,
            },
            evaluation_admissions: report.usage.evaluation_admissions,
            evaluation_consumed: report
                .usage
                .evaluation_consumed
                .iter()
                .map(|(kind, consumed)| CounterUsage {
                    kind: kind.as_str(),
                    consumed: *consumed,
                })
                .collect(),
        },
        basis: report.basis.as_str(),
        witness,
    };
    let json = serde_json::to_vec(&document)?;
    let parsed = quire_canonical::read(&json, u64::MAX)?;
    Ok(quire_canonical::to_vec(
        &parsed.root(),
        quire_canonical::Limits::new(u64::MAX),
    )?)
}
